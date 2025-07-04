/**
 * Priority Queue Implementation with Fair Scheduling
 */

import {
  QueuedRequest,
  RequestPriority,
  RequestScheduler,
  PriorityBoostConfig
} from './queue-interfaces';

export class PriorityQueue implements RequestScheduler {
  private queues: Map<RequestPriority, QueuedRequest[]>;
  private requestMap: Map<string, QueuedRequest>;
  private fairnessCounters: Map<RequestPriority, number>;
  private priorityBoostConfig?: PriorityBoostConfig;
  private lastScheduledPriority: RequestPriority = RequestPriority.Critical;

  constructor(priorityBoostConfig?: PriorityBoostConfig) {
    this.queues = new Map();
    this.requestMap = new Map();
    this.fairnessCounters = new Map();
    this.priorityBoostConfig = priorityBoostConfig;

    // Initialize queues for each priority level
    for (const priority of Object.values(RequestPriority)) {
      if (typeof priority === 'number') {
        this.queues.set(priority, []);
        this.fairnessCounters.set(priority, 0);
      }
    }
  }

  /**
   * Schedule a request in the appropriate priority queue
   */
  schedule(request: QueuedRequest): void {
    // Check for duplicates
    if (this.requestMap.has(request.id)) {
      throw new Error(`Request ${request.id} already in queue`);
    }

    // Apply priority boosting if configured
    if (this.priorityBoostConfig) {
      this.applyPriorityBoost(request);
    }

    // Add to appropriate queue
    const queue = this.queues.get(request.priority);
    if (!queue) {
      throw new Error(`Invalid priority: ${request.priority}`);
    }

    queue.push(request);
    this.requestMap.set(request.id, request);
  }

  /**
   * Cancel a scheduled request
   */
  cancel(requestId: string): boolean {
    const request = this.requestMap.get(requestId);
    if (!request) {
      return false;
    }

    const queue = this.queues.get(request.priority);
    if (queue) {
      const index = queue.findIndex(r => r.id === requestId);
      if (index !== -1) {
        queue.splice(index, 1);
      }
    }

    this.requestMap.delete(requestId);
    return true;
  }

  /**
   * Get the next request using fair scheduling
   */
  getNext(): QueuedRequest | null {
    // Apply priority boosting to waiting requests
    if (this.priorityBoostConfig) {
      this.boostWaitingRequests();
    }

    // Use weighted fair queuing
    const request = this.getNextFairScheduled();
    
    if (request) {
      // Remove from queue
      const queue = this.queues.get(request.priority);
      if (queue) {
        const index = queue.indexOf(request);
        if (index !== -1) {
          queue.splice(index, 1);
        }
      }
      this.requestMap.delete(request.id);
    }

    return request;
  }

  /**
   * Peek at the next request without removing it
   */
  peek(): QueuedRequest | null {
    // Find highest priority non-empty queue
    for (let priority = RequestPriority.Critical; priority <= RequestPriority.Background; priority++) {
      const queue = this.queues.get(priority);
      if (queue && queue.length > 0) {
        return queue[0];
      }
    }
    return null;
  }

  /**
   * Get total size across all queues
   */
  size(): number {
    return this.requestMap.size;
  }

  /**
   * Clear all queues
   */
  clear(): void {
    for (const queue of this.queues.values()) {
      queue.length = 0;
    }
    this.requestMap.clear();
    this.fairnessCounters.clear();
  }

  /**
   * Get queue statistics by priority
   */
  getQueueStats(): Map<RequestPriority, number> {
    const stats = new Map<RequestPriority, number>();
    for (const [priority, queue] of this.queues) {
      stats.set(priority, queue.length);
    }
    return stats;
  }

  /**
   * Get requests by priority
   */
  getRequestsByPriority(priority: RequestPriority): QueuedRequest[] {
    return [...(this.queues.get(priority) || [])];
  }

  /**
   * Apply priority boost based on wait time
   */
  private applyPriorityBoost(request: QueuedRequest): void {
    if (!this.priorityBoostConfig || request.priority === RequestPriority.Critical) {
      return;
    }

    const waitTime = Date.now() - request.enqueuedAt.getTime();
    const boosts = Math.floor(waitTime / this.priorityBoostConfig.waitTimeThreshold);
    
    if (boosts > 0) {
      const maxBoosts = Math.min(boosts, this.priorityBoostConfig.maxBoosts);
      const boostedPriority = Math.max(
        RequestPriority.Critical,
        request.priority - (maxBoosts * this.priorityBoostConfig.boostAmount)
      );
      
      request.priority = boostedPriority as RequestPriority;
      request.metadata = {
        ...request.metadata,
        priorityBoosted: true,
        originalPriority: request.metadata?.originalPriority || request.priority,
        boostCount: (request.metadata?.boostCount || 0) + maxBoosts
      };
    }
  }

  /**
   * Boost priority of waiting requests
   */
  private boostWaitingRequests(): void {
    if (!this.priorityBoostConfig) return;

    const now = Date.now();
    
    for (const [priority, queue] of this.queues) {
      if (priority === RequestPriority.Critical) continue;
      
      for (const request of queue) {
        const waitTime = now - request.enqueuedAt.getTime();
        const boostThreshold = this.priorityBoostConfig.waitTimeThreshold;
        
        if (waitTime > boostThreshold) {
          // Move to higher priority queue
          const newPriority = Math.max(
            RequestPriority.Critical,
            priority - this.priorityBoostConfig.boostAmount
          ) as RequestPriority;
          
          if (newPriority !== priority) {
            // Remove from current queue
            const index = queue.indexOf(request);
            if (index !== -1) {
              queue.splice(index, 1);
            }
            
            // Update priority and add to new queue
            request.priority = newPriority;
            request.metadata = {
              ...request.metadata,
              priorityBoosted: true,
              originalPriority: request.metadata?.originalPriority || priority,
              boostCount: (request.metadata?.boostCount || 0) + 1
            };
            
            const newQueue = this.queues.get(newPriority);
            if (newQueue) {
              newQueue.push(request);
            }
          }
        }
      }
    }
  }

  /**
   * Get next request using weighted fair scheduling
   */
  private getNextFairScheduled(): QueuedRequest | null {
    // Check each priority level with fairness
    const weights = {
      [RequestPriority.Critical]: 5,
      [RequestPriority.High]: 3,
      [RequestPriority.Normal]: 2,
      [RequestPriority.Low]: 1,
      [RequestPriority.Background]: 0.5
    };

    // Find the priority level that should be scheduled next
    let selectedPriority: RequestPriority | null = null;
    let minRatio = Infinity;

    for (let priority = RequestPriority.Critical; priority <= RequestPriority.Background; priority++) {
      const queue = this.queues.get(priority);
      if (!queue || queue.length === 0) continue;

      const counter = this.fairnessCounters.get(priority) || 0;
      const weight = weights[priority] || 1;
      const ratio = counter / weight;

      if (ratio < minRatio) {
        minRatio = ratio;
        selectedPriority = priority;
      }
    }

    if (selectedPriority === null) {
      return null;
    }

    // Get request from selected priority queue
    const queue = this.queues.get(selectedPriority);
    if (queue && queue.length > 0) {
      // Increment fairness counter
      this.fairnessCounters.set(
        selectedPriority,
        (this.fairnessCounters.get(selectedPriority) || 0) + 1
      );

      // Reset counters periodically to prevent overflow
      if (this.shouldResetCounters()) {
        this.resetFairnessCounters();
      }

      return queue[0];
    }

    return null;
  }

  /**
   * Check if fairness counters should be reset
   */
  private shouldResetCounters(): boolean {
    // Reset when any counter exceeds 1000
    for (const count of this.fairnessCounters.values()) {
      if (count > 1000) {
        return true;
      }
    }
    return false;
  }

  /**
   * Reset fairness counters proportionally
   */
  private resetFairnessCounters(): void {
    const minCount = Math.min(...Array.from(this.fairnessCounters.values()));
    
    for (const [priority, count] of this.fairnessCounters) {
      this.fairnessCounters.set(priority, count - minCount);
    }
  }
}