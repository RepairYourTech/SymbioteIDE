/**
 * Queue Management Module Exports
 */

// Main queue manager
export { QueueManager, QueueManagerOptions } from './queue-manager';

// Core interfaces
export * from './queue-interfaces';

// Queue implementations
export { PriorityQueue } from './priority-queue';

// Rate limiting
export { 
  TokenBucketRateLimiter,
  SlidingWindowRateLimiter,
  CompositeRateLimiter
} from './rate-limiter';

// Backpressure handling
export { 
  BackpressureHandler,
  AdaptiveBackpressureHandler,
  BackpressureStatus
} from './backpressure-handler';

// Dead letter queue
export {
  DeadLetterQueueManager,
  DeadLetterConfig,
  DeadLetterStats,
  DeadLetterRecoveryService
} from './dead-letter-queue';

// Persistence
export {
  FileSystemQueuePersistence,
  InMemoryQueuePersistence
} from './queue-persistence';

// Re-export key types for convenience
export type {
  QueuedRequest,
  RequestPriority,
  RequestStatus,
  QueueConfig,
  QueueStats,
  RateLimitConfig,
  BackpressureConfig,
  BackpressureStrategy,
  QueueEvent,
  QueueEventType
} from './queue-interfaces';