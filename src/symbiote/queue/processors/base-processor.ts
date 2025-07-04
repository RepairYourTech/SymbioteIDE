/**
 * Base Processor - Abstract base class for job processors
 */

import { Job } from 'bullmq';
import { JobData, JobResult } from '../types';
import { Logger } from '../../utils/logger';

export abstract class BaseProcessor<T extends JobData = JobData> {
  protected logger: Logger;
  protected queueManager: any;

  constructor(queueManager: any, loggerName: string) {
    this.queueManager = queueManager;
    this.logger = new Logger(loggerName);
  }

  /**
   * Process a job
   */
  abstract process(job: Job<T>): Promise<JobResult>;

  /**
   * Update job progress
   */
  protected async updateProgress(
    job: Job,
    percentage: number,
    message?: string,
    details?: any
  ): Promise<void> {
    await job.updateProgress({
      percentage,
      message,
      currentStep: details?.currentStep,
      totalSteps: details?.totalSteps,
      details
    });
  }

  /**
   * Create success result
   */
  protected createSuccessResult(data: any, metrics?: any): JobResult {
    return {
      success: true,
      data,
      metrics: {
        duration: 0, // Will be set by queue manager
        ...metrics
      }
    };
  }

  /**
   * Create error result
   */
  protected createErrorResult(
    code: string,
    message: string,
    details?: any
  ): JobResult {
    return {
      success: false,
      error: {
        code,
        message,
        details
      }
    };
  }

  /**
   * Validate job data
   */
  protected abstract validateJobData(data: T): void;

  /**
   * Handle job failure
   */
  protected async handleFailure(
    job: Job<T>,
    error: Error
  ): Promise<JobResult> {
    this.logger.error(`Job ${job.id} failed`, {
      error: error.message,
      stack: error.stack,
      attemptsMade: job.attemptsMade,
      data: job.data
    });

    // Check if should retry
    const shouldRetry = this.shouldRetry(error, job);
    
    if (!shouldRetry && job.opts.attempts && job.attemptsMade >= job.opts.attempts) {
      // Final failure - could trigger notifications or cleanup
      await this.onFinalFailure(job, error);
    }

    return this.createErrorResult(
      'JOB_FAILED',
      error.message,
      {
        stack: error.stack,
        attemptsMade: job.attemptsMade,
        willRetry: shouldRetry
      }
    );
  }

  /**
   * Determine if job should be retried
   */
  protected shouldRetry(error: Error, job: Job<T>): boolean {
    // Override in subclasses for custom retry logic
    // Default: retry on transient errors
    const transientErrors = [
      'ECONNREFUSED',
      'ETIMEDOUT',
      'ENOTFOUND',
      'RATE_LIMIT'
    ];

    return transientErrors.some(code => 
      error.message.includes(code)
    );
  }

  /**
   * Called when job has finally failed after all retries
   */
  protected async onFinalFailure(
    job: Job<T>,
    error: Error
  ): Promise<void> {
    // Override in subclasses for cleanup or notifications
    this.logger.error(`Job ${job.id} finally failed after ${job.attemptsMade} attempts`, {
      error: error.message,
      data: job.data
    });
  }

  /**
   * Get execution context
   */
  protected getContext(job: Job<T>): any {
    return {
      jobId: job.id,
      queueName: job.queueName,
      attemptsMade: job.attemptsMade,
      timestamp: job.timestamp,
      userId: job.data.userId,
      metadata: job.data.metadata
    };
  }
}