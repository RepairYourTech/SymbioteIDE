/**
 * Request Queue Management Interfaces
 */

import { AITask, TaskResult, ModelProfile } from '../interfaces';

export interface QueuedRequest {
  id: string;
  task: AITask;
  priority: RequestPriority;
  enqueuedAt: Date;
  startedAt?: Date;
  completedAt?: Date;
  attempts: number;
  status: RequestStatus;
  assignedModel?: ModelProfile;
  result?: TaskResult;
  error?: Error;
  metadata?: QueueMetadata;
  callback?: (result: TaskResult | Error) => void;
}

export enum RequestPriority {
  Critical = 0,  // Highest priority
  High = 1,
  Normal = 2,
  Low = 3,
  Background = 4 // Lowest priority
}

export enum RequestStatus {
  Queued = 'queued',
  Processing = 'processing',
  Completed = 'completed',
  Failed = 'failed',
  DeadLetter = 'dead-letter',
  Cancelled = 'cancelled'
}

export interface QueueMetadata {
  userId?: string;
  sessionId?: string;
  retryCount?: number;
  maxRetries?: number;
  timeout?: number;
  dedupKey?: string;
  tags?: string[];
  [key: string]: any;
}

export interface QueueConfig {
  maxConcurrency: number;
  maxQueueSize: number;
  defaultTimeout: number;
  maxRetries: number;
  enablePersistence: boolean;
  enableDeduplication: boolean;
  enableDeadLetter: boolean;
  rateLimits?: RateLimitConfig[];
  priorityBoost?: PriorityBoostConfig;
  backpressure?: BackpressureConfig;
}

export interface RateLimitConfig {
  name: string;
  limit: number;
  window: number; // milliseconds
  scope?: 'global' | 'user' | 'model' | 'task-type';
  key?: string; // For custom scoping
}

export interface PriorityBoostConfig {
  waitTimeThreshold: number; // ms - boost priority after waiting this long
  boostAmount: number; // how much to boost priority
  maxBoosts: number; // maximum number of boosts
}

export interface BackpressureConfig {
  highWaterMark: number; // Start applying backpressure
  lowWaterMark: number;  // Stop applying backpressure
  strategy: BackpressureStrategy;
  rejectOnFull: boolean;
}

export enum BackpressureStrategy {
  Pause = 'pause',        // Pause accepting new requests
  Throttle = 'throttle',  // Slow down request acceptance
  Reject = 'reject',      // Reject new requests
  Shed = 'shed'          // Drop lowest priority requests
}

export interface QueueStats {
  queueLength: number;
  activeRequests: number;
  completedRequests: number;
  failedRequests: number;
  averageWaitTime: number;
  averageProcessingTime: number;
  throughput: number;
  errorRate: number;
  utilizationRate: number;
  oldestRequestAge?: number;
  priorityDistribution: Record<RequestPriority, number>;
}

export interface DeadLetterEntry {
  request: QueuedRequest;
  reason: string;
  timestamp: Date;
  attempts: number;
  lastError?: Error;
}

export interface QueuePersistence {
  save(requests: QueuedRequest[]): Promise<void>;
  load(): Promise<QueuedRequest[]>;
  saveDeadLetter(entry: DeadLetterEntry): Promise<void>;
  loadDeadLetter(): Promise<DeadLetterEntry[]>;
  clear(): Promise<void>;
}

export interface RequestScheduler {
  schedule(request: QueuedRequest): void;
  cancel(requestId: string): boolean;
  getNext(): QueuedRequest | null;
  peek(): QueuedRequest | null;
  size(): number;
  clear(): void;
}

export interface RateLimiter {
  checkLimit(key: string, limit: RateLimitConfig): Promise<boolean>;
  consumeToken(key: string, limit: RateLimitConfig): Promise<void>;
  getRemainingTokens(key: string, limit: RateLimitConfig): Promise<number>;
  reset(key?: string): Promise<void>;
}

export interface QueueEvent {
  type: QueueEventType;
  timestamp: Date;
  requestId?: string;
  data?: any;
}

export enum QueueEventType {
  RequestEnqueued = 'request-enqueued',
  RequestStarted = 'request-started',
  RequestCompleted = 'request-completed',
  RequestFailed = 'request-failed',
  RequestRetried = 'request-retried',
  RequestDeadLettered = 'request-dead-lettered',
  RequestCancelled = 'request-cancelled',
  QueueFull = 'queue-full',
  BackpressureApplied = 'backpressure-applied',
  BackpressureReleased = 'backpressure-released',
  RateLimitExceeded = 'rate-limit-exceeded'
}