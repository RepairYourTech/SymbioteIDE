/**
 * Main Queue Manager - Orchestrates all queue components
 */

import { EventEmitter } from 'events';
import {
  QueuedRequest,
  RequestPriority,
  RequestStatus,
  QueueConfig,
  QueueStats,
  QueueEvent,
  QueueEventType,
  RequestScheduler,
  RateLimiter,
  QueuePersistence,
  QueueMetadata
} from './queue-interfaces';
import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { PriorityQueue } from './priority-queue';
import { TokenBucketRateLimiter } from './rate-limiter';
import { BackpressureHandler } from './backpressure-handler';
import { DeadLetterQueueManager } from './dead-letter-queue';
import { FileSystemQueuePersistence, InMemoryQueuePersistence } from './queue-persistence';

export interface QueueManagerOptions extends QueueConfig {
  persistencePath?: string;
  deadLetterConfig?: {
    maxRetries: number;
    retryDelayMs: number;
    maxAge: number;
    enableAutoRetry: boolean;
    retryBackoffMultiplier: number;
  };
}

export class QueueManager extends EventEmitter {
  private config: QueueConfig;
  private scheduler: RequestScheduler;
  private rateLimiter: RateLimiter;
  private backpressureHandler: BackpressureHandler;
  private deadLetterQueue: DeadLetterQueueManager;
  private persistence: QueuePersistence;
  private activeRequests: Map<string, QueuedRequest>;
  private completedCount: number = 0;
  private failedCount: number = 0;
  private totalWaitTime: number = 0;
  private totalProcessingTime: number = 0;
  private deduplicationMap: Map<string, string>;
  private persistenceInterval?: NodeJS.Timeout;

  constructor(options: QueueManagerOptions) {
    super();
    this.config = options;
    this.activeRequests = new Map();
    this.deduplicationMap = new Map();

    // Initialize components
    this.scheduler = new PriorityQueue(options.priorityBoost);
    this.rateLimiter = new TokenBucketRateLimiter();
    
    if (options.backpressure) {
      this.backpressureHandler = new BackpressureHandler(options.backpressure);
      this.backpressureHandler.on('event', (event) => this.emit('event', event));
    } else {
      // Default backpressure config
      this.backpressureHandler = new BackpressureHandler({
        highWaterMark: 80,
        lowWaterMark: 60,
        strategy: 'throttle' as any,
        rejectOnFull: false
      });
    }

    // Initialize persistence
    this.persistence = options.enablePersistence && options.persistencePath
      ? new FileSystemQueuePersistence(options.persistencePath)
      : new InMemoryQueuePersistence();

    // Initialize dead letter queue
    const dlqConfig = options.deadLetterConfig || {
      maxRetries: options.maxRetries,
      retryDelayMs: 5000,
      maxAge: 7 * 24 * 60 * 60 * 1000, // 7 days
      enableAutoRetry: true,
      retryBackoffMultiplier: 2
    };

    this.deadLetterQueue = new DeadLetterQueueManager(dlqConfig, this.persistence);
    this.deadLetterQueue.on('event', (event) => this.emit('event', event));
    this.deadLetterQueue.on('autoRetry', (request) => this.handleAutoRetry(request));

    // Start persistence interval
    if (options.enablePersistence) {
      this.startPersistence();
    }
  }

  /**
   * Initialize queue manager
   */
  async initialize(): Promise<void> {
    // Load persisted state
    if (this.config.enablePersistence) {
      const requests = await this.persistence.load();
      for (const request of requests) {
        if (request.status === RequestStatus.Queued) {
          this.scheduler.schedule(request);
        } else if (request.status === RequestStatus.Processing) {
          // Reset to queued state
          request.status = RequestStatus.Queued;
          this.scheduler.schedule(request);
        }
      }
    }

    // Initialize dead letter queue
    await this.deadLetterQueue.initialize();

    this.emit('initialized', {
      queuedRequests: this.scheduler.size(),
      deadLetterEntries: this.deadLetterQueue.getStats().totalEntries
    });
  }

  /**
   * Enqueue a new task
   */
  async enqueue(
    task: AITask,
    options?: {
      priority?: RequestPriority;
      metadata?: QueueMetadata;
      callback?: (result: TaskResult | Error) => void;
    }
  ): Promise<string> {
    // Check if queue is full
    if (this.scheduler.size() >= this.config.maxQueueSize) {
      const error = new Error('Queue is full');
      this.emit('event', {
        type: QueueEventType.QueueFull,
        timestamp: new Date(),
        data: { size: this.scheduler.size() }
      } as QueueEvent);
      throw error;
    }

    // Check deduplication
    if (this.config.enableDeduplication && options?.metadata?.dedupKey) {
      const existingId = this.deduplicationMap.get(options.metadata.dedupKey);
      if (existingId) {
        return existingId;
      }
    }

    // Apply backpressure
    const currentLoad = (this.scheduler.size() + this.activeRequests.size) / this.config.maxQueueSize * 100;
    this.backpressureHandler.checkBackpressure(currentLoad, this.config.maxQueueSize);
    
    const backpressureResult = await this.backpressureHandler.applyBackpressure({
      id: 'temp',
      task,
      priority: options?.priority || RequestPriority.Normal,
      enqueuedAt: new Date(),
      attempts: 0,
      status: RequestStatus.Queued
    });

    if (!backpressureResult.accepted) {
      throw new Error(backpressureResult.reason || 'Request rejected by backpressure');
    }

    // Create queued request
    const request: QueuedRequest = {
      id: `req-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      task,
      priority: options?.priority || RequestPriority.Normal,
      enqueuedAt: new Date(),
      attempts: 0,
      status: RequestStatus.Queued,
      metadata: {
        ...options?.metadata,
        maxRetries: options?.metadata?.maxRetries ?? this.config.maxRetries,
        timeout: options?.metadata?.timeout ?? this.config.defaultTimeout
      },
      callback: options?.callback
    };

    // Check rate limits
    if (this.config.rateLimits) {
      for (const limit of this.config.rateLimits) {
        const key = this.getRateLimitKey(request, limit);
        const canProceed = await this.rateLimiter.checkLimit(key, limit);
        
        if (!canProceed) {
          this.emit('event', {
            type: QueueEventType.RateLimitExceeded,
            timestamp: new Date(),
            requestId: request.id,
            data: { limit: limit.name }
          } as QueueEvent);
          throw new Error(`Rate limit exceeded: ${limit.name}`);
        }
      }
    }

    // Schedule request
    this.scheduler.schedule(request);

    // Track deduplication
    if (this.config.enableDeduplication && options?.metadata?.dedupKey) {
      this.deduplicationMap.set(options.metadata.dedupKey, request.id);
    }

    // Emit event
    this.emit('event', {
      type: QueueEventType.RequestEnqueued,
      timestamp: new Date(),
      requestId: request.id,
      data: { priority: request.priority }
    } as QueueEvent);

    // Trigger processing
    this.processNext();

    return request.id;
  }

  /**
   * Cancel a queued request
   */
  async cancel(requestId: string): Promise<boolean> {
    // Try to cancel from scheduler
    if (this.scheduler.cancel(requestId)) {
      this.emit('event', {
        type: QueueEventType.RequestCancelled,
        timestamp: new Date(),
        requestId,
        data: { location: 'queue' }
      } as QueueEvent);
      return true;
    }

    // Check if it's active
    const activeRequest = this.activeRequests.get(requestId);
    if (activeRequest) {
      activeRequest.status = RequestStatus.Cancelled;
      this.activeRequests.delete(requestId);
      
      this.emit('event', {
        type: QueueEventType.RequestCancelled,
        timestamp: new Date(),
        requestId,
        data: { location: 'active' }
      } as QueueEvent);
      return true;
    }

    return false;
  }

  /**
   * Get queue statistics
   */
  getStats(): QueueStats {
    const queueStats = this.scheduler.getQueueStats();
    const dlqStats = this.deadLetterQueue.getStats();
    const now = Date.now();

    // Calculate averages
    const totalCompleted = this.completedCount + this.failedCount;
    const avgWaitTime = totalCompleted > 0 ? this.totalWaitTime / totalCompleted : 0;
    const avgProcessingTime = this.completedCount > 0 ? this.totalProcessingTime / this.completedCount : 0;

    // Calculate throughput (requests per minute)
    const runtimeMs = now - (this.startTime || now);
    const throughput = runtimeMs > 0 ? (totalCompleted / runtimeMs) * 60000 : 0;

    // Calculate error rate
    const errorRate = totalCompleted > 0 ? this.failedCount / totalCompleted : 0;

    // Calculate utilization
    const utilizationRate = this.activeRequests.size / this.config.maxConcurrency;

    // Find oldest request
    let oldestRequestAge: number | undefined;
    const oldestRequest = this.scheduler.peek();
    if (oldestRequest) {
      oldestRequestAge = now - oldestRequest.enqueuedAt.getTime();
    }

    // Build priority distribution
    const priorityDistribution: Record<RequestPriority, number> = {
      [RequestPriority.Critical]: 0,
      [RequestPriority.High]: 0,
      [RequestPriority.Normal]: 0,
      [RequestPriority.Low]: 0,
      [RequestPriority.Background]: 0
    };

    for (const [priority, count] of queueStats) {
      priorityDistribution[priority] = count;
    }

    return {
      queueLength: this.scheduler.size(),
      activeRequests: this.activeRequests.size,
      completedRequests: this.completedCount,
      failedRequests: this.failedCount + dlqStats.totalEntries,
      averageWaitTime: avgWaitTime,
      averageProcessingTime: avgProcessingTime,
      throughput,
      errorRate,
      utilizationRate,
      oldestRequestAge,
      priorityDistribution
    };
  }

  /**
   * Get request status
   */
  getRequestStatus(requestId: string): {
    status: RequestStatus;
    position?: number;
    estimatedWaitTime?: number;
  } | null {
    // Check active requests
    const activeRequest = this.activeRequests.get(requestId);
    if (activeRequest) {
      return { status: activeRequest.status };
    }

    // Check queue position
    const queuedRequests = this.scheduler.getRequestsByPriority(RequestPriority.Critical)
      .concat(this.scheduler.getRequestsByPriority(RequestPriority.High))
      .concat(this.scheduler.getRequestsByPriority(RequestPriority.Normal))
      .concat(this.scheduler.getRequestsByPriority(RequestPriority.Low))
      .concat(this.scheduler.getRequestsByPriority(RequestPriority.Background));

    const position = queuedRequests.findIndex(r => r.id === requestId);
    if (position !== -1) {
      const avgProcessingTime = this.getStats().averageProcessingTime;
      const estimatedWaitTime = position * avgProcessingTime / this.config.maxConcurrency;
      
      return {
        status: RequestStatus.Queued,
        position: position + 1,
        estimatedWaitTime
      };
    }

    // Check dead letter queue
    const dlqEntry = this.deadLetterQueue.getEntry(requestId);
    if (dlqEntry) {
      return { status: RequestStatus.DeadLetter };
    }

    return null;
  }

  /**
   * Shutdown queue manager
   */
  async shutdown(): Promise<void> {
    // Stop accepting new requests
    this.emit('shuttingDown');

    // Wait for active requests to complete
    const timeout = 30000; // 30 seconds
    const startTime = Date.now();
    
    while (this.activeRequests.size > 0 && Date.now() - startTime < timeout) {
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    // Cancel remaining active requests
    for (const [id, request] of this.activeRequests) {
      await this.cancel(id);
    }

    // Stop components
    if (this.persistenceInterval) {
      clearInterval(this.persistenceInterval);
    }

    this.deadLetterQueue.destroy();
    
    if (this.rateLimiter && 'destroy' in this.rateLimiter) {
      (this.rateLimiter as any).destroy();
    }

    // Final persistence
    if (this.config.enablePersistence) {
      await this.persist();
    }

    this.emit('shutdown');
  }

  /**
   * Process next request in queue
   */
  private async processNext(): Promise<void> {
    // Check if we can process more requests
    if (this.activeRequests.size >= this.config.maxConcurrency) {
      return;
    }

    // Get next request
    const request = this.scheduler.getNext();
    if (!request) {
      return;
    }

    // Consume rate limit tokens
    if (this.config.rateLimits) {
      try {
        for (const limit of this.config.rateLimits) {
          const key = this.getRateLimitKey(request, limit);
          await this.rateLimiter.consumeToken(key, limit);
        }
      } catch (error) {
        // Return to queue if rate limited
        this.scheduler.schedule(request);
        return;
      }
    }

    // Mark as processing
    request.status = RequestStatus.Processing;
    request.startedAt = new Date();
    this.activeRequests.set(request.id, request);

    // Update wait time stats
    const waitTime = request.startedAt.getTime() - request.enqueuedAt.getTime();
    this.totalWaitTime += waitTime;

    // Emit event
    this.emit('event', {
      type: QueueEventType.RequestStarted,
      timestamp: new Date(),
      requestId: request.id,
      data: { waitTime }
    } as QueueEvent);

    // Process request
    this.emit('process', request);

    // Continue processing
    setImmediate(() => this.processNext());
  }

  /**
   * Handle request completion
   */
  async completeRequest(requestId: string, result: TaskResult): Promise<void> {
    const request = this.activeRequests.get(requestId);
    if (!request) {
      return;
    }

    request.status = RequestStatus.Completed;
    request.completedAt = new Date();
    request.result = result;

    // Update stats
    const processingTime = request.completedAt.getTime() - request.startedAt!.getTime();
    this.totalProcessingTime += processingTime;
    this.completedCount++;

    // Remove from active
    this.activeRequests.delete(requestId);

    // Clear deduplication
    if (request.metadata?.dedupKey) {
      this.deduplicationMap.delete(request.metadata.dedupKey);
    }

    // Invoke callback
    if (request.callback) {
      try {
        request.callback(result);
      } catch (error) {
        console.error('Callback error:', error);
      }
    }

    // Emit event
    this.emit('event', {
      type: QueueEventType.RequestCompleted,
      timestamp: new Date(),
      requestId,
      data: { processingTime }
    } as QueueEvent);

    // Process next
    this.processNext();
  }

  /**
   * Handle request failure
   */
  async failRequest(requestId: string, error: Error): Promise<void> {
    const request = this.activeRequests.get(requestId);
    if (!request) {
      return;
    }

    request.attempts++;
    request.error = error;

    // Check if should retry
    const maxRetries = request.metadata?.maxRetries ?? this.config.maxRetries;
    
    if (request.attempts < maxRetries) {
      // Return to queue for retry
      request.status = RequestStatus.Queued;
      request.startedAt = undefined;
      this.activeRequests.delete(requestId);
      this.scheduler.schedule(request);

      this.emit('event', {
        type: QueueEventType.RequestRetried,
        timestamp: new Date(),
        requestId,
        data: { 
          attempt: request.attempts,
          error: error.message 
        }
      } as QueueEvent);
    } else {
      // Move to dead letter queue
      request.status = RequestStatus.Failed;
      request.completedAt = new Date();
      this.failedCount++;
      this.activeRequests.delete(requestId);

      if (this.config.enableDeadLetter) {
        await this.deadLetterQueue.addEntry(
          request,
          `Failed after ${request.attempts} attempts`,
          error
        );
      }

      // Clear deduplication
      if (request.metadata?.dedupKey) {
        this.deduplicationMap.delete(request.metadata.dedupKey);
      }

      // Invoke callback with error
      if (request.callback) {
        try {
          request.callback(error);
        } catch (callbackError) {
          console.error('Callback error:', callbackError);
        }
      }

      this.emit('event', {
        type: QueueEventType.RequestFailed,
        timestamp: new Date(),
        requestId,
        data: { 
          attempts: request.attempts,
          error: error.message 
        }
      } as QueueEvent);
    }

    // Process next
    this.processNext();
  }

  /**
   * Handle auto-retry from dead letter queue
   */
  private async handleAutoRetry(request: QueuedRequest): Promise<void> {
    try {
      // Re-enqueue the request
      this.scheduler.schedule(request);
      this.processNext();
    } catch (error) {
      console.error('Failed to handle auto-retry:', error);
    }
  }

  /**
   * Get rate limit key for request
   */
  private getRateLimitKey(request: QueuedRequest, limit: any): string {
    switch (limit.scope) {
      case 'user':
        return request.metadata?.userId || 'anonymous';
      case 'model':
        return request.assignedModel?.id || 'unknown';
      case 'task-type':
        return request.task.type || 'general';
      default:
        return 'global';
    }
  }

  /**
   * Start persistence interval
   */
  private startPersistence(): void {
    this.persistenceInterval = setInterval(async () => {
      await this.persist();
    }, 30000); // Persist every 30 seconds
  }

  /**
   * Persist current state
   */
  private async persist(): Promise<void> {
    if (!this.config.enablePersistence) {
      return;
    }

    const allRequests: QueuedRequest[] = [];
    
    // Add queued requests
    for (let priority = RequestPriority.Critical; priority <= RequestPriority.Background; priority++) {
      allRequests.push(...this.scheduler.getRequestsByPriority(priority));
    }

    // Add active requests
    allRequests.push(...this.activeRequests.values());

    await this.persistence.save(allRequests);
  }

  private startTime = Date.now();
}