/**
 * Dead Letter Queue Manager
 */

import { 
  QueuedRequest, 
  DeadLetterEntry,
  QueuePersistence,
  QueueEvent,
  QueueEventType,
  RequestStatus
} from './queue-interfaces';
import { EventEmitter } from 'events';

export interface DeadLetterConfig {
  maxRetries: number;
  retryDelayMs: number;
  maxAge: number; // Maximum age in ms before permanent failure
  enableAutoRetry: boolean;
  retryBackoffMultiplier: number;
}

export interface DeadLetterStats {
  totalEntries: number;
  retriableEntries: number;
  permanentFailures: number;
  oldestEntry?: Date;
  mostCommonErrors: Array<{ error: string; count: number }>;
  entriesByReason: Record<string, number>;
}

export class DeadLetterQueueManager extends EventEmitter {
  private entries: Map<string, DeadLetterEntry>;
  private config: DeadLetterConfig;
  private persistence?: QueuePersistence;
  private retryTimers: Map<string, NodeJS.Timeout>;
  private errorCounts: Map<string, number>;

  constructor(config: DeadLetterConfig, persistence?: QueuePersistence) {
    super();
    this.config = config;
    this.persistence = persistence;
    this.entries = new Map();
    this.retryTimers = new Map();
    this.errorCounts = new Map();
  }

  /**
   * Initialize dead letter queue from persistence
   */
  async initialize(): Promise<void> {
    if (this.persistence) {
      const entries = await this.persistence.loadDeadLetter();
      for (const entry of entries) {
        this.entries.set(entry.request.id, entry);
        
        // Schedule auto-retry if enabled
        if (this.config.enableAutoRetry && this.isRetriable(entry)) {
          this.scheduleRetry(entry);
        }
      }
    }
  }

  /**
   * Add a failed request to dead letter queue
   */
  async addEntry(
    request: QueuedRequest, 
    reason: string, 
    error?: Error
  ): Promise<void> {
    const entry: DeadLetterEntry = {
      request: {
        ...request,
        status: RequestStatus.DeadLetter
      },
      reason,
      timestamp: new Date(),
      attempts: request.attempts,
      lastError: error
    };

    this.entries.set(request.id, entry);
    
    // Track error types
    const errorType = error?.message || reason;
    this.errorCounts.set(errorType, (this.errorCounts.get(errorType) || 0) + 1);

    // Persist to storage
    if (this.persistence) {
      await this.persistence.saveDeadLetter(entry);
    }

    // Emit event
    this.emit('event', {
      type: QueueEventType.RequestDeadLettered,
      timestamp: new Date(),
      requestId: request.id,
      data: { reason, error: error?.message }
    } as QueueEvent);

    // Schedule auto-retry if enabled and retriable
    if (this.config.enableAutoRetry && this.isRetriable(entry)) {
      this.scheduleRetry(entry);
    }
  }

  /**
   * Manually retry a dead letter entry
   */
  async retryEntry(requestId: string): Promise<QueuedRequest | null> {
    const entry = this.entries.get(requestId);
    if (!entry) {
      return null;
    }

    if (!this.isRetriable(entry)) {
      throw new Error('Entry has exceeded maximum retry attempts');
    }

    // Cancel any scheduled retry
    const timer = this.retryTimers.get(requestId);
    if (timer) {
      clearTimeout(timer);
      this.retryTimers.delete(requestId);
    }

    // Remove from dead letter queue
    this.entries.delete(requestId);

    // Update request for retry
    const retriedRequest: QueuedRequest = {
      ...entry.request,
      status: RequestStatus.Queued,
      attempts: entry.attempts + 1,
      metadata: {
        ...entry.request.metadata,
        retriedFromDeadLetter: true,
        previousError: entry.reason
      }
    };

    // Emit retry event
    this.emit('event', {
      type: QueueEventType.RequestRetried,
      timestamp: new Date(),
      requestId: requestId,
      data: { 
        attempt: retriedRequest.attempts,
        previousError: entry.reason 
      }
    } as QueueEvent);

    return retriedRequest;
  }

  /**
   * Retry all eligible entries
   */
  async retryAll(): Promise<QueuedRequest[]> {
    const retriedRequests: QueuedRequest[] = [];
    const entriesToRetry = Array.from(this.entries.values())
      .filter(entry => this.isRetriable(entry));

    for (const entry of entriesToRetry) {
      try {
        const retriedRequest = await this.retryEntry(entry.request.id);
        if (retriedRequest) {
          retriedRequests.push(retriedRequest);
        }
      } catch (error) {
        console.error(`Failed to retry entry ${entry.request.id}:`, error);
      }
    }

    return retriedRequests;
  }

  /**
   * Remove an entry from dead letter queue
   */
  async removeEntry(requestId: string): Promise<boolean> {
    const entry = this.entries.get(requestId);
    if (!entry) {
      return false;
    }

    // Cancel any scheduled retry
    const timer = this.retryTimers.get(requestId);
    if (timer) {
      clearTimeout(timer);
      this.retryTimers.delete(requestId);
    }

    this.entries.delete(requestId);
    return true;
  }

  /**
   * Get a specific entry
   */
  getEntry(requestId: string): DeadLetterEntry | undefined {
    return this.entries.get(requestId);
  }

  /**
   * Get all entries
   */
  getEntries(filter?: {
    retriable?: boolean;
    reason?: string;
    olderThan?: Date;
  }): DeadLetterEntry[] {
    let entries = Array.from(this.entries.values());

    if (filter) {
      if (filter.retriable !== undefined) {
        entries = entries.filter(e => this.isRetriable(e) === filter.retriable);
      }
      if (filter.reason) {
        entries = entries.filter(e => e.reason.includes(filter.reason!));
      }
      if (filter.olderThan) {
        entries = entries.filter(e => e.timestamp < filter.olderThan!);
      }
    }

    return entries;
  }

  /**
   * Get dead letter queue statistics
   */
  getStats(): DeadLetterStats {
    const entries = Array.from(this.entries.values());
    const retriableEntries = entries.filter(e => this.isRetriable(e));
    const permanentFailures = entries.length - retriableEntries.length;

    // Group entries by reason
    const entriesByReason: Record<string, number> = {};
    for (const entry of entries) {
      entriesByReason[entry.reason] = (entriesByReason[entry.reason] || 0) + 1;
    }

    // Get most common errors
    const mostCommonErrors = Array.from(this.errorCounts.entries())
      .map(([error, count]) => ({ error, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 10);

    // Find oldest entry
    let oldestEntry: Date | undefined;
    for (const entry of entries) {
      if (!oldestEntry || entry.timestamp < oldestEntry) {
        oldestEntry = entry.timestamp;
      }
    }

    return {
      totalEntries: entries.length,
      retriableEntries: retriableEntries.length,
      permanentFailures,
      oldestEntry,
      mostCommonErrors,
      entriesByReason
    };
  }

  /**
   * Clean up old entries
   */
  async cleanup(): Promise<number> {
    const cutoffTime = new Date(Date.now() - this.config.maxAge);
    const entriesToRemove: string[] = [];

    for (const [id, entry] of this.entries) {
      if (entry.timestamp < cutoffTime) {
        entriesToRemove.push(id);
      }
    }

    for (const id of entriesToRemove) {
      await this.removeEntry(id);
    }

    return entriesToRemove.length;
  }

  /**
   * Stop all retry timers
   */
  destroy(): void {
    for (const timer of this.retryTimers.values()) {
      clearTimeout(timer);
    }
    this.retryTimers.clear();
  }

  /**
   * Check if entry is retriable
   */
  private isRetriable(entry: DeadLetterEntry): boolean {
    // Check max retries
    if (entry.attempts >= this.config.maxRetries) {
      return false;
    }

    // Check age
    const age = Date.now() - entry.timestamp.getTime();
    if (age > this.config.maxAge) {
      return false;
    }

    // Check for permanent failure indicators
    const permanentErrors = [
      'Invalid API key',
      'Account suspended',
      'Model not found',
      'Invalid request format'
    ];

    if (entry.lastError) {
      for (const permanentError of permanentErrors) {
        if (entry.lastError.message.includes(permanentError)) {
          return false;
        }
      }
    }

    return true;
  }

  /**
   * Schedule automatic retry
   */
  private scheduleRetry(entry: DeadLetterEntry): void {
    // Calculate delay with exponential backoff
    const baseDelay = this.config.retryDelayMs;
    const multiplier = Math.pow(this.config.retryBackoffMultiplier, entry.attempts - 1);
    const delay = Math.min(baseDelay * multiplier, 300000); // Max 5 minutes

    const timer = setTimeout(async () => {
      try {
        const retriedRequest = await this.retryEntry(entry.request.id);
        if (retriedRequest) {
          this.emit('autoRetry', retriedRequest);
        }
      } catch (error) {
        console.error(`Auto-retry failed for ${entry.request.id}:`, error);
      }
    }, delay);

    this.retryTimers.set(entry.request.id, timer);
  }
}

/**
 * Dead Letter Queue Recovery Service
 * Provides advanced recovery strategies for dead letter entries
 */
export class DeadLetterRecoveryService {
  private dlq: DeadLetterQueueManager;

  constructor(dlq: DeadLetterQueueManager) {
    this.dlq = dlq;
  }

  /**
   * Analyze dead letter patterns and suggest recovery strategies
   */
  analyzePatterns(): {
    patterns: Array<{
      type: string;
      count: number;
      suggestion: string;
    }>;
    recommendations: string[];
  } {
    const stats = this.dlq.getStats();
    const patterns: Array<{ type: string; count: number; suggestion: string }> = [];
    const recommendations: string[] = [];

    // Analyze error patterns
    for (const { error, count } of stats.mostCommonErrors) {
      if (error.includes('timeout')) {
        patterns.push({
          type: 'Timeout Errors',
          count,
          suggestion: 'Consider increasing timeout limits or using faster models'
        });
      } else if (error.includes('rate limit')) {
        patterns.push({
          type: 'Rate Limit Errors',
          count,
          suggestion: 'Implement better rate limiting or request throttling'
        });
      } else if (error.includes('context')) {
        patterns.push({
          type: 'Context Errors',
          count,
          suggestion: 'Reduce prompt size or use models with larger context windows'
        });
      }
    }

    // Generate recommendations
    if (stats.retriableEntries > stats.totalEntries * 0.5) {
      recommendations.push('Many entries are still retriable - consider batch retry');
    }

    if (stats.permanentFailures > 10) {
      recommendations.push('High permanent failure count - review error handling logic');
    }

    const oldestAge = stats.oldestEntry ? 
      Date.now() - stats.oldestEntry.getTime() : 0;
    if (oldestAge > 24 * 60 * 60 * 1000) {
      recommendations.push('Old entries detected - consider cleanup or manual review');
    }

    return { patterns, recommendations };
  }

  /**
   * Attempt intelligent recovery based on error type
   */
  async intelligentRecover(requestId: string): Promise<QueuedRequest | null> {
    const entry = this.dlq.getEntry(requestId);
    if (!entry) {
      return null;
    }

    // Apply recovery strategies based on error type
    const modifiedRequest = { ...entry.request };

    if (entry.reason.includes('timeout')) {
      // Increase timeout for retry
      modifiedRequest.metadata = {
        ...modifiedRequest.metadata,
        timeout: (modifiedRequest.metadata?.timeout || 30000) * 2
      };
    } else if (entry.reason.includes('context too long')) {
      // Try to reduce context size
      if (modifiedRequest.task.context) {
        modifiedRequest.task.context = {
          ...modifiedRequest.task.context,
          truncated: true
        };
      }
    } else if (entry.reason.includes('rate limit')) {
      // Add delay before retry
      modifiedRequest.metadata = {
        ...modifiedRequest.metadata,
        delayBeforeExecution: 5000
      };
    }

    // Remove from DLQ and return modified request
    await this.dlq.removeEntry(requestId);
    return modifiedRequest;
  }
}