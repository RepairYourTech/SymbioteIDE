/**
 * Redis Queue Manager - Queue management using Redis
 */

import { EventEmitter } from 'events';
import { RedisClient } from '../infrastructure/redis/redis-client';

export interface QueueJob<T = any> {
  id: string;
  queue: string;
  data: T;
  priority?: number;
  attempts?: number;
  createdAt: Date;
  processedAt?: Date;
  completedAt?: Date;
  failedAt?: Date;
  error?: string;
}

export interface QueueOptions {
  maxRetries?: number;
  retryDelay?: number;
  timeout?: number;
  priority?: number;
}

export class RedisQueueManager extends EventEmitter {
  private static instance: RedisQueueManager;
  private redisClient: RedisClient;
  private processing: Map<string, boolean> = new Map();

  private constructor() {
    super();
    this.redisClient = RedisClient.getInstance();
  }

  static getInstance(): RedisQueueManager {
    if (!RedisQueueManager.instance) {
      RedisQueueManager.instance = new RedisQueueManager();
    }
    return RedisQueueManager.instance;
  }

  async addJob<T>(queue: string, data: T, options?: QueueOptions): Promise<string> {
    const job: QueueJob<T> = {
      id: `job_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      queue,
      data,
      priority: options?.priority || 0,
      attempts: 0,
      createdAt: new Date()
    };

    const client = this.redisClient.getClient();
    const jobKey = `queue:${queue}:job:${job.id}`;
    
    // Store job data
    await client.hset(jobKey, 'data', JSON.stringify(job));
    
    // Add to queue
    if (job.priority) {
      await client.zadd(`queue:${queue}:priority`, job.priority, job.id);
    } else {
      await client.lpush(`queue:${queue}:pending`, job.id);
    }

    this.emit('job:added', job);
    return job.id;
  }

  async getJob<T>(queue: string): Promise<QueueJob<T> | null> {
    const client = this.redisClient.getClient();
    
    // Try priority queue first
    const priorityJobId = await client.zpopmax(`queue:${queue}:priority`, 1);
    let jobId: string | null = priorityJobId?.[0] || null;
    
    // If no priority job, get from regular queue
    if (!jobId) {
      jobId = await client.rpop(`queue:${queue}:pending`);
    }
    
    if (!jobId) {
      return null;
    }

    const jobKey = `queue:${queue}:job:${jobId}`;
    const jobData = await client.hget(jobKey, 'data');
    
    if (!jobData) {
      return null;
    }

    const job = JSON.parse(jobData) as QueueJob<T>;
    job.processedAt = new Date();
    
    // Move to processing
    await client.lpush(`queue:${queue}:processing`, jobId);
    await client.hset(jobKey, 'data', JSON.stringify(job));

    this.emit('job:processing', job);
    return job;
  }

  async completeJob(queue: string, jobId: string): Promise<void> {
    const client = this.redisClient.getClient();
    const jobKey = `queue:${queue}:job:${jobId}`;
    
    const jobData = await client.hget(jobKey, 'data');
    if (!jobData) {
      throw new Error(`Job ${jobId} not found`);
    }

    const job = JSON.parse(jobData) as QueueJob;
    job.completedAt = new Date();
    
    // Update job and move to completed
    await client.hset(jobKey, 'data', JSON.stringify(job));
    await client.lrem(`queue:${queue}:processing`, 1, jobId);
    await client.lpush(`queue:${queue}:completed`, jobId);

    this.emit('job:completed', job);
  }

  async failJob(queue: string, jobId: string, error: string, options?: QueueOptions): Promise<void> {
    const client = this.redisClient.getClient();
    const jobKey = `queue:${queue}:job:${jobId}`;
    
    const jobData = await client.hget(jobKey, 'data');
    if (!jobData) {
      throw new Error(`Job ${jobId} not found`);
    }

    const job = JSON.parse(jobData) as QueueJob;
    job.attempts = (job.attempts || 0) + 1;
    job.error = error;
    
    const maxRetries = options?.maxRetries || 3;
    
    if (job.attempts < maxRetries) {
      // Retry job
      await client.lrem(`queue:${queue}:processing`, 1, jobId);
      
      if (options?.retryDelay) {
        setTimeout(() => {
          client.lpush(`queue:${queue}:pending`, jobId);
        }, options.retryDelay);
      } else {
        await client.lpush(`queue:${queue}:pending`, jobId);
      }
      
      this.emit('job:retry', job);
    } else {
      // Move to failed
      job.failedAt = new Date();
      await client.lrem(`queue:${queue}:processing`, 1, jobId);
      await client.lpush(`queue:${queue}:failed`, jobId);
      
      this.emit('job:failed', job);
    }
    
    await client.hset(jobKey, 'data', JSON.stringify(job));
  }

  async getQueueStats(queue: string): Promise<{
    pending: number;
    processing: number;
    completed: number;
    failed: number;
  }> {
    const client = this.redisClient.getClient();
    
    const [pending, processing, completed, failed, priority] = await Promise.all([
      client.llen(`queue:${queue}:pending`),
      client.llen(`queue:${queue}:processing`),
      client.llen(`queue:${queue}:completed`),
      client.llen(`queue:${queue}:failed`),
      client.zcard(`queue:${queue}:priority`)
    ]);

    return {
      pending: pending + priority,
      processing,
      completed,
      failed
    };
  }

  async clearQueue(queue: string): Promise<void> {
    const client = this.redisClient.getClient();
    
    const keys = await client.keys(`queue:${queue}:*`);
    if (keys.length > 0) {
      await client.del(...keys);
    }
    
    this.emit('queue:cleared', queue);
  }

  startProcessing(queue: string, handler: (job: QueueJob) => Promise<void>, options?: QueueOptions): void {
    if (this.processing.get(queue)) {
      return;
    }

    this.processing.set(queue, true);
    
    const processNext = async () => {
      if (!this.processing.get(queue)) {
        return;
      }

      try {
        const job = await this.getJob(queue);
        
        if (job) {
          try {
            await handler(job);
            await this.completeJob(queue, job.id);
          } catch (error) {
            await this.failJob(queue, job.id, error.message, options);
          }
        }
      } catch (error) {
        this.emit('error', error);
      }

      // Continue processing
      if (this.processing.get(queue)) {
        setImmediate(processNext);
      }
    };

    processNext();
    this.emit('processing:started', queue);
  }

  stopProcessing(queue: string): void {
    this.processing.set(queue, false);
    this.emit('processing:stopped', queue);
  }

  isProcessing(queue: string): boolean {
    return this.processing.get(queue) || false;
  }
}

export default RedisQueueManager;