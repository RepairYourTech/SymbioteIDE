/**
 * Redis Types and Interfaces
 * 
 * Core types for the distributed caching and pub/sub system
 */

import { Redis, RedisOptions } from 'ioredis';

/**
 * Redis connection configuration
 */
export interface RedisConfig {
  host?: string;
  port?: number;
  password?: string;
  db?: number;
  keyPrefix?: string;
  connectionName?: string;
  
  // Cluster configuration
  cluster?: boolean;
  clusterNodes?: Array<{ host: string; port: number }>;
  
  // Connection options
  connectTimeout?: number;
  commandTimeout?: number;
  maxRetriesPerRequest?: number;
  enableOfflineQueue?: boolean;
  
  // TLS options
  tls?: {
    rejectUnauthorized?: boolean;
    ca?: string;
    cert?: string;
    key?: string;
  };
}

/**
 * Cache entry metadata
 */
export interface CacheEntryMeta {
  key: string;
  created: number;
  updated: number;
  accessed: number;
  accessCount: number;
  size: number;
  ttl?: number;
  tags?: string[];
}

/**
 * Cache statistics
 */
export interface CacheStats {
  totalEntries: number;
  totalSize: number;
  hitRate: number;
  missRate: number;
  evictionCount: number;
  connectionStatus: 'connected' | 'connecting' | 'disconnected' | 'error';
  lastError?: string;
}

/**
 * Pub/Sub message types
 */
export enum PubSubChannel {
  // Memory system channels
  MemoryUpdates = 'memory:updates',
  MemorySync = 'memory:sync',
  MemoryDelete = 'memory:delete',
  
  // Code indexing channels
  IndexingProgress = 'indexing:progress',
  IndexingComplete = 'indexing:complete',
  
  // Agent collaboration channels
  AgentTask = 'agent:task',
  AgentResponse = 'agent:response',
  AgentStatus = 'agent:status',
  
  // System events
  SystemStatus = 'system:status',
  SystemError = 'system:error',
  
  // MCP events
  MCPServerStatus = 'mcp:server:status',
  MCPToolCall = 'mcp:tool:call',
  MCPToolResponse = 'mcp:tool:response'
}

/**
 * Pub/Sub message format
 */
export interface PubSubMessage<T = any> {
  id: string;
  channel: PubSubChannel | string;
  type: string;
  source: string;
  timestamp: number;
  data: T;
  metadata?: Record<string, any>;
}

/**
 * Rate limiting configuration
 */
export interface RateLimitConfig {
  windowMs: number;      // Time window in milliseconds
  maxRequests: number;   // Max requests per window
  keyPrefix?: string;    // Prefix for rate limit keys
  skipSuccessfulRequests?: boolean;
  skipFailedRequests?: boolean;
}

/**
 * Rate limit result
 */
export interface RateLimitResult {
  allowed: boolean;
  remaining: number;
  resetAt: number;
  retryAfter?: number;
}

/**
 * Session data
 */
export interface SessionData {
  id: string;
  userId?: string;
  agentId?: string;
  projectId?: string;
  created: number;
  updated: number;
  expiresAt?: number;
  data: Record<string, any>;
}

/**
 * Lock options
 */
export interface LockOptions {
  ttl?: number;          // Lock expiration time in ms
  retries?: number;      // Number of retries
  retryDelay?: number;   // Delay between retries in ms
}

/**
 * Job queue types
 */
export interface JobData<T = any> {
  id: string;
  type: string;
  data: T;
  priority?: number;
  attempts?: number;
  maxAttempts?: number;
  delay?: number;
  backoff?: {
    type: 'fixed' | 'exponential';
    delay: number;
  };
  removeOnComplete?: boolean;
  removeOnFail?: boolean;
}

/**
 * Job status
 */
export enum JobStatus {
  Waiting = 'waiting',
  Active = 'active',
  Completed = 'completed',
  Failed = 'failed',
  Delayed = 'delayed',
  Stalled = 'stalled'
}

/**
 * Cache invalidation strategies
 */
export enum InvalidationStrategy {
  TTL = 'ttl',              // Time-based expiration
  LRU = 'lru',              // Least recently used
  LFU = 'lfu',              // Least frequently used
  MANUAL = 'manual',        // Manual invalidation only
  BROADCAST = 'broadcast'   // Invalidate across all instances
}

/**
 * Redis manager events
 */
export interface RedisManagerEvents {
  'connected': () => void;
  'disconnected': (reason?: string) => void;
  'error': (error: Error) => void;
  'message': (message: PubSubMessage) => void;
  'cache-hit': (key: string) => void;
  'cache-miss': (key: string) => void;
  'cache-evict': (key: string, reason: string) => void;
}

/**
 * Redis connection status
 */
export interface ConnectionStatus {
  connected: boolean;
  ready: boolean;
  status: 'connecting' | 'connect' | 'ready' | 'error' | 'close' | 'reconnecting' | 'end';
  error?: Error;
  lastActivity?: Date;
  commandQueueLength?: number;
}

/**
 * Cache patterns
 */
export enum CachePattern {
  CacheAside = 'cache-aside',          // Load on miss
  WriteThrough = 'write-through',      // Write to cache and store
  WriteBehind = 'write-behind',        // Write to cache, async to store
  RefreshAhead = 'refresh-ahead'       // Proactive refresh before expiry
}

/**
 * Serialization format
 */
export enum SerializationFormat {
  JSON = 'json',
  MessagePack = 'msgpack',
  Protobuf = 'protobuf',
  Raw = 'raw'
}