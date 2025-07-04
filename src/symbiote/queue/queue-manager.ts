/**
 * Queue Manager - Centralized management of Bull queues
 */

import { Queue, Worker, QueueEvents, Job, ConnectionOptions } from 'bullmq';
import { Redis } from 'ioredis';
import { EventEmitter } from 'events';
import { 
  QueueName, 
  QueueConfig, 
  JobData, 
  JobResult, 
  JobProgress,
  QueueMetrics,
  QueueEventHandlers,
  BulkJobOperation,
  JobFilter,
  SymbioteJobOptions
} from './types';
import { Logger } from '../utils/logger';
import { RedisManager } from '../redis/redis-manager';

export class QueueManager extends EventEmitter {
  private queues: Map<QueueName, Queue> = new Map();
  private workers: Map<QueueName, Worker> = new Map();
  private queueEvents: Map<QueueName, QueueEvents> = new Map();
  private logger: Logger;
  private redisManager: RedisManager;
  private connection: ConnectionOptions;
  private isShutdown: boolean = false;
  private metricsInterval?: NodeJS.Timeout;

  constructor(redisManager: RedisManager) {
    super();
    this.logger = new Logger('QueueManager');
    this.redisManager = redisManager;
    
    // Use Redis connection from RedisManager
    this.connection = {
      host: process.env.REDIS_HOST || 'localhost',
      port: parseInt(process.env.REDIS_PORT || '6379'),
      password: process.env.REDIS_PASSWORD,
      db: parseInt(process.env.REDIS_QUEUE_DB || '2'), // Separate DB for queues
      maxRetriesPerRequest: 3,
      enableReadyCheck: true,
      lazyConnect: true
    };
  }

  /**
   * Initialize queue manager
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing queue manager');
    
    // Initialize default queues
    await this.createQueue(QueueName.CodeIndexing, {
      name: QueueName.CodeIndexing,
      defaultJobOptions: {
        attempts: 3,
        backoff: { type: 'exponential', delay: 1000 },
        removeOnComplete: 100,
        removeOnFail: 1000
      },
      workerConfig: {
        concurrency: 5,
        maxStalledCount: 3
      }
    });

    await this.createQueue(QueueName.EmbeddingGeneration, {
      name: QueueName.EmbeddingGeneration,
      defaultJobOptions: {
        attempts: 3,
        backoff: { type: 'exponential', delay: 2000 },
        removeOnComplete: 50
      },
      workerConfig: {
        concurrency: 3, // Limited due to API rate limits
        maxStalledCount: 2
      },
      rateLimiter: {
        max: 100,
        duration: 60000 // 100 requests per minute
      }
    });

    await this.createQueue(QueueName.MemoryConsolidation, {
      name: QueueName.MemoryConsolidation,
      defaultJobOptions: {
        attempts: 2,
        removeOnComplete: 10
      },
      workerConfig: {
        concurrency: 2,
        lockDuration: 300000 // 5 minutes
      }
    });

    await this.createQueue(QueueName.GraphSync, {
      name: QueueName.GraphSync,
      defaultJobOptions: {
        attempts: 3,
        backoff: { type: 'fixed', delay: 1000 }
      },
      workerConfig: {
        concurrency: 4
      }
    });

    await this.createQueue(QueueName.AgentTasks, {
      name: QueueName.AgentTasks,
      defaultJobOptions: {
        attempts: 1, // Agent tasks typically shouldn't retry automatically
        removeOnComplete: 20
      },
      workerConfig: {
        concurrency: 10
      }
    });

    // Start metrics collection
    this.startMetricsCollection();
    
    this.logger.info('Queue manager initialized');
  }

  /**
   * Create a new queue with worker
   */
  async createQueue(name: QueueName, config: QueueConfig): Promise<void> {
    if (this.queues.has(name)) {
      this.logger.warn(`Queue ${name} already exists`);
      return;
    }

    // Create queue
    const queue = new Queue(name, {
      connection: this.connection,
      defaultJobOptions: config.defaultJobOptions
    });

    // Create worker
    const worker = new Worker(
      name,
      async (job: Job) => this.processJob(name, job),
      {
        connection: this.connection,
        ...config.workerConfig
      }
    );

    // Create queue events
    const queueEvents = new QueueEvents(name, {
      connection: this.connection
    });

    // Set up event handlers
    this.setupQueueEvents(name, queueEvents);
    this.setupWorkerEvents(name, worker);

    // Apply rate limiter if configured
    if (config.rateLimiter) {
      await queue.setRateLimiter(config.rateLimiter);
    }

    // Store references
    this.queues.set(name, queue);
    this.workers.set(name, worker);
    this.queueEvents.set(name, queueEvents);

    this.logger.info(`Queue ${name} created with worker`);
  }

  /**
   * Add a job to queue
   */
  async addJob<T extends JobData>(
    queueName: QueueName,
    jobName: string,
    data: T,
    options?: SymbioteJobOptions
  ): Promise<Job<T, JobResult>> {
    const queue = this.queues.get(queueName);
    if (!queue) {
      throw new Error(`Queue ${queueName} not found`);
    }

    const job = await queue.add(jobName, data, {
      ...options,
      // Add metadata
      timestamp: Date.now(),
      attemptsMade: 0
    });

    this.logger.debug(`Job ${job.id} added to queue ${queueName}`, {
      jobName,
      priority: options?.priority
    });

    return job as Job<T, JobResult>;
  }

  /**
   * Add multiple jobs
   */
  async addBulkJobs(
    queueName: QueueName,
    jobs: BulkJobOperation[]
  ): Promise<Job[]> {
    const queue = this.queues.get(queueName);
    if (!queue) {
      throw new Error(`Queue ${queueName} not found`);
    }

    const bulkJobs = await queue.addBulk(jobs);
    
    this.logger.info(`Added ${bulkJobs.length} jobs to queue ${queueName}`);
    
    return bulkJobs;
  }

  /**
   * Process job (delegates to specific processors)
   */
  private async processJob(
    queueName: QueueName,
    job: Job<JobData>
  ): Promise<JobResult> {
    const startTime = Date.now();
    
    try {
      this.logger.debug(`Processing job ${job.id} in queue ${queueName}`, {
        type: job.data.type,
        attemptsMade: job.attemptsMade
      });

      // Import and execute the appropriate processor
      const processor = await this.getProcessor(queueName);
      const result = await processor.process(job);

      const duration = Date.now() - startTime;
      
      // Add metrics to result
      result.metrics = {
        duration,
        ...result.metrics
      };

      this.logger.debug(`Job ${job.id} completed`, { duration });
      
      return result;
      
    } catch (error: any) {
      const duration = Date.now() - startTime;
      
      this.logger.error(`Job ${job.id} failed`, {
        error: error.message,
        duration,
        stack: error.stack
      });

      throw error;
    }
  }

  /**
   * Get processor for queue
   */
  private async getProcessor(queueName: QueueName): Promise<any> {
    switch (queueName) {
      case QueueName.CodeIndexing:
        const { CodeIndexingProcessor } = await import('./processors/code-indexing-processor');
        return new CodeIndexingProcessor(this);
        
      case QueueName.EmbeddingGeneration:
        const { EmbeddingProcessor } = await import('./processors/embedding-processor');
        return new EmbeddingProcessor(this);
        
      case QueueName.MemoryConsolidation:
        const { MemoryProcessor } = await import('./processors/memory-processor');
        return new MemoryProcessor(this);
        
      case QueueName.GraphSync:
        const { GraphSyncProcessor } = await import('./processors/graph-sync-processor');
        return new GraphSyncProcessor(this);
        
      case QueueName.AgentTasks:
        const { AgentTaskProcessor } = await import('./processors/agent-task-processor');
        return new AgentTaskProcessor(this);
        
      default:
        throw new Error(`No processor found for queue ${queueName}`);
    }
  }

  /**
   * Get job by ID
   */
  async getJob(
    queueName: QueueName,
    jobId: string
  ): Promise<Job | undefined> {
    const queue = this.queues.get(queueName);
    if (!queue) {
      throw new Error(`Queue ${queueName} not found`);
    }

    return queue.getJob(jobId);
  }

  /**
   * Get jobs by filter
   */
  async getJobs(
    queueName: QueueName,
    filter: JobFilter
  ): Promise<Job[]> {
    const queue = this.queues.get(queueName);
    if (!queue) {
      throw new Error(`Queue ${queueName} not found`);
    }

    let jobs: Job[] = [];

    // Get jobs by status
    if (filter.status) {
      for (const status of filter.status) {
        const statusJobs = await queue.getJobs(
          [status],
          filter.offset || 0,
          filter.limit || 100
        );
        jobs.push(...statusJobs);
      }
    } else {
      // Get all jobs
      jobs = await queue.getJobs(
        ['waiting', 'active', 'completed', 'failed', 'delayed'],
        filter.offset || 0,
        filter.limit || 100
      );
    }

    // Filter by type
    if (filter.types) {
      jobs = jobs.filter(job => 
        filter.types!.includes(job.data.type)
      );
    }

    // Filter by date range
    if (filter.dateRange) {
      const startTime = filter.dateRange.start.getTime();
      const endTime = filter.dateRange.end.getTime();
      
      jobs = jobs.filter(job => {
        const timestamp = job.timestamp || 0;
        return timestamp >= startTime && timestamp <= endTime;
      });
    }

    return jobs;
  }

  /**
   * Update job progress
   */
  async updateJobProgress(
    queueName: QueueName,
    jobId: string,
    progress: JobProgress
  ): Promise<void> {
    const job = await this.getJob(queueName, jobId);
    if (job) {
      await job.updateProgress(progress);
    }
  }

  /**
   * Pause/resume queue
   */
  async pauseQueue(queueName: QueueName): Promise<void> {
    const queue = this.queues.get(queueName);
    if (queue) {
      await queue.pause();
      this.logger.info(`Queue ${queueName} paused`);
    }
  }

  async resumeQueue(queueName: QueueName): Promise<void> {
    const queue = this.queues.get(queueName);
    if (queue) {
      await queue.resume();
      this.logger.info(`Queue ${queueName} resumed`);
    }
  }

  /**
   * Clean queue
   */
  async cleanQueue(
    queueName: QueueName,
    grace: number = 0,
    limit: number = 100,
    status: 'completed' | 'failed' = 'completed'
  ): Promise<string[]> {
    const queue = this.queues.get(queueName);
    if (!queue) {
      throw new Error(`Queue ${queueName} not found`);
    }

    const removed = await queue.clean(grace, limit, status);
    this.logger.info(`Cleaned ${removed.length} ${status} jobs from queue ${queueName}`);
    
    return removed;
  }

  /**
   * Get queue metrics
   */
  async getQueueMetrics(queueName: QueueName): Promise<QueueMetrics> {
    const queue = this.queues.get(queueName);
    if (!queue) {
      throw new Error(`Queue ${queueName} not found`);
    }

    const counts = await queue.getJobCounts();
    const jobs = await queue.getJobs(['completed', 'failed'], 0, 100);
    
    // Calculate performance metrics
    let totalProcessingTime = 0;
    let successCount = 0;
    const jobTypeCounts: { [key: string]: number } = {};

    for (const job of jobs) {
      if (job.finishedOn && job.processedOn) {
        totalProcessingTime += job.finishedOn - job.processedOn;
      }
      
      if (job.failedReason === undefined) {
        successCount++;
      }

      const jobType = job.data.type || 'unknown';
      jobTypeCounts[jobType] = (jobTypeCounts[jobType] || 0) + 1;
    }

    const totalJobs = jobs.length;
    const averageProcessingTime = totalJobs > 0 ? totalProcessingTime / totalJobs : 0;
    const successRate = totalJobs > 0 ? successCount / totalJobs : 0;

    // Calculate throughput (jobs per minute)
    const recentJobs = jobs.filter(job => {
      const timestamp = job.timestamp || 0;
      return Date.now() - timestamp < 60000; // Last minute
    });
    const throughput = recentJobs.length;

    return {
      waiting: counts.waiting || 0,
      active: counts.active || 0,
      completed: counts.completed || 0,
      failed: counts.failed || 0,
      delayed: counts.delayed || 0,
      paused: await queue.isPaused(),
      jobCounts: jobTypeCounts,
      performance: {
        averageProcessingTime,
        successRate,
        throughput
      }
    };
  }

  /**
   * Get all queues metrics
   */
  async getAllMetrics(): Promise<Map<QueueName, QueueMetrics>> {
    const metrics = new Map<QueueName, QueueMetrics>();
    
    for (const queueName of this.queues.keys()) {
      metrics.set(queueName, await this.getQueueMetrics(queueName));
    }
    
    return metrics;
  }

  /**
   * Setup queue events
   */
  private setupQueueEvents(name: QueueName, queueEvents: QueueEvents): void {
    queueEvents.on('completed', ({ jobId, returnvalue }) => {
      this.emit('job:completed', { queue: name, jobId, result: returnvalue });
    });

    queueEvents.on('failed', ({ jobId, failedReason }) => {
      this.emit('job:failed', { queue: name, jobId, reason: failedReason });
    });

    queueEvents.on('progress', ({ jobId, data }) => {
      this.emit('job:progress', { queue: name, jobId, progress: data });
    });

    queueEvents.on('stalled', ({ jobId }) => {
      this.emit('job:stalled', { queue: name, jobId });
    });
  }

  /**
   * Setup worker events
   */
  private setupWorkerEvents(name: QueueName, worker: Worker): void {
    worker.on('error', (error) => {
      this.logger.error(`Worker error in queue ${name}`, error);
      this.emit('worker:error', { queue: name, error });
    });

    worker.on('closed', () => {
      this.logger.warn(`Worker closed in queue ${name}`);
      this.emit('worker:closed', { queue: name });
    });
  }

  /**
   * Start metrics collection
   */
  private startMetricsCollection(): void {
    this.metricsInterval = setInterval(async () => {
      try {
        const metrics = await this.getAllMetrics();
        
        // Store in Redis for monitoring
        await this.redisManager.set(
          'queue:metrics:snapshot',
          metrics,
          300 // 5 minutes TTL
        );
        
        // Emit metrics event
        this.emit('metrics:updated', metrics);
      } catch (error) {
        this.logger.error('Failed to collect metrics', error);
      }
    }, 60000); // Every minute
  }

  /**
   * Shutdown queue manager
   */
  async shutdown(): Promise<void> {
    if (this.isShutdown) {
      return;
    }

    this.logger.info('Shutting down queue manager');
    this.isShutdown = true;

    // Stop metrics collection
    if (this.metricsInterval) {
      clearInterval(this.metricsInterval);
    }

    // Close all workers
    const workerClosePromises = Array.from(this.workers.values()).map(
      worker => worker.close()
    );
    await Promise.all(workerClosePromises);

    // Close all queue events
    const queueEventsClosePromises = Array.from(this.queueEvents.values()).map(
      qe => qe.close()
    );
    await Promise.all(queueEventsClosePromises);

    // Close all queues
    const queueClosePromises = Array.from(this.queues.values()).map(
      queue => queue.close()
    );
    await Promise.all(queueClosePromises);

    this.logger.info('Queue manager shutdown complete');
  }

  /**
   * Register event handlers
   */
  registerHandlers(queueName: QueueName, handlers: QueueEventHandlers): void {
    if (handlers.onCompleted) {
      this.on('job:completed', async (event) => {
        if (event.queue === queueName) {
          const job = await this.getJob(queueName, event.jobId);
          if (job) {
            await handlers.onCompleted!(job, event.result);
          }
        }
      });
    }

    if (handlers.onFailed) {
      this.on('job:failed', async (event) => {
        if (event.queue === queueName) {
          const job = await this.getJob(queueName, event.jobId);
          if (job) {
            await handlers.onFailed!(job, new Error(event.reason));
          }
        }
      });
    }

    if (handlers.onProgress) {
      this.on('job:progress', async (event) => {
        if (event.queue === queueName) {
          const job = await this.getJob(queueName, event.jobId);
          if (job) {
            await handlers.onProgress!(job, event.progress);
          }
        }
      });
    }
  }
}