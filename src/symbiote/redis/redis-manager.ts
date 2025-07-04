/**
 * Redis Manager
 * 
 * Manages Redis connections for distributed caching, pub/sub, and session management
 */

import { EventEmitter } from 'events';
import Redis, { Redis as RedisClient, Cluster } from 'ioredis';
import { Logger } from '../utils/logger';
import {
  RedisConfig,
  CacheEntryMeta,
  CacheStats,
  PubSubChannel,
  PubSubMessage,
  RateLimitConfig,
  RateLimitResult,
  SessionData,
  LockOptions,
  ConnectionStatus,
  CachePattern,
  InvalidationStrategy,
  SerializationFormat
} from './types';

export class RedisManager extends EventEmitter {
  private client: RedisClient | Cluster;
  private pubClient?: RedisClient;
  private subClient?: RedisClient;
  private config: RedisConfig;
  private logger = new Logger('RedisManager');
  private isConnected = false;
  private cacheStats: CacheStats;
  private subscribedChannels = new Set<string>();
  private locks = new Map<string, NodeJS.Timeout>();
  
  constructor(config: RedisConfig = {}) {
    super();
    
    this.config = {
      host: config.host || process.env.REDIS_HOST || 'localhost',
      port: config.port || parseInt(process.env.REDIS_PORT || '6379'),
      password: config.password || process.env.REDIS_PASSWORD,
      db: config.db || 0,
      keyPrefix: config.keyPrefix || 'symbiote:',
      connectionName: config.connectionName || 'symbiote-main',
      connectTimeout: config.connectTimeout || 20000,
      commandTimeout: config.commandTimeout || 5000,
      maxRetriesPerRequest: config.maxRetriesPerRequest || 3,
      enableOfflineQueue: config.enableOfflineQueue !== false,
      ...config
    };
    
    this.cacheStats = {
      totalEntries: 0,
      totalSize: 0,
      hitRate: 0,
      missRate: 0,
      evictionCount: 0,
      connectionStatus: 'disconnected'
    };
    
    // Create main client
    this.client = this.createClient();
    
    // Set up event handlers
    this.setupEventHandlers();
  }
  
  /**
   * Create Redis client
   */
  private createClient(): RedisClient | Cluster {
    if (this.config.cluster && this.config.clusterNodes) {
      return new Redis.Cluster(this.config.clusterNodes, {
        redisOptions: {
          password: this.config.password,
          tls: this.config.tls
        }
      });
    }
    
    return new Redis({
      host: this.config.host,
      port: this.config.port,
      password: this.config.password,
      db: this.config.db,
      keyPrefix: this.config.keyPrefix,
      connectionName: this.config.connectionName,
      connectTimeout: this.config.connectTimeout,
      commandTimeout: this.config.commandTimeout,
      maxRetriesPerRequest: this.config.maxRetriesPerRequest,
      enableOfflineQueue: this.config.enableOfflineQueue,
      tls: this.config.tls
    });
  }
  
  /**
   * Set up event handlers
   */
  private setupEventHandlers(): void {
    this.client.on('connect', () => {
      this.logger.info('Redis connected');
      this.cacheStats.connectionStatus = 'connected';
      this.isConnected = true;
      this.emit('connected');
    });
    
    this.client.on('ready', () => {
      this.logger.info('Redis ready');
      this.updateStats();
    });
    
    this.client.on('error', (error) => {
      this.logger.error('Redis error', error);
      this.cacheStats.connectionStatus = 'error';
      this.cacheStats.lastError = error.message;
      this.emit('error', error);
    });
    
    this.client.on('close', () => {
      this.logger.info('Redis connection closed');
      this.cacheStats.connectionStatus = 'disconnected';
      this.isConnected = false;
      this.emit('disconnected');
    });
  }
  
  /**
   * Connect to Redis
   */
  async connect(): Promise<void> {
    if (this.isConnected) {
      return;
    }
    
    try {
      await this.client.connect();
      await this.initializePubSub();
      this.logger.info('Redis manager initialized');
    } catch (error) {
      this.logger.error('Failed to connect to Redis', error);
      throw error;
    }
  }
  
  /**
   * Initialize pub/sub clients
   */
  private async initializePubSub(): Promise<void> {
    this.pubClient = this.createClient();
    this.subClient = this.createClient();
    
    // Set up message handler
    this.subClient.on('message', (channel, message) => {
      try {
        const parsed: PubSubMessage = JSON.parse(message);
        this.emit('message', parsed);
        this.emit(`channel:${channel}`, parsed);
      } catch (error) {
        this.logger.error('Failed to parse pub/sub message', error);
      }
    });
    
    await Promise.all([
      this.pubClient.connect(),
      this.subClient.connect()
    ]);
  }
  
  /**
   * Disconnect from Redis
   */
  async disconnect(): Promise<void> {
    this.isConnected = false;
    
    // Clear all locks
    for (const [key, timeout] of this.locks) {
      clearTimeout(timeout);
      await this.unlock(key);
    }
    this.locks.clear();
    
    // Disconnect all clients
    await Promise.all([
      this.client.quit(),
      this.pubClient?.quit(),
      this.subClient?.quit()
    ].filter(Boolean));
    
    this.logger.info('Redis manager disconnected');
  }
  
  /**
   * Get connection status
   */
  getStatus(): ConnectionStatus {
    return {
      connected: this.isConnected,
      ready: this.client.status === 'ready',
      status: this.client.status as any,
      lastActivity: new Date(),
      commandQueueLength: (this.client as any).commandQueue?.length || 0
    };
  }
  
  // Cache Operations
  
  /**
   * Get value from cache
   */
  async get<T = any>(key: string): Promise<T | null> {
    try {
      const value = await this.client.get(key);
      
      if (value === null) {
        this.emit('cache-miss', key);
        this.updateCacheStats('miss');
        return null;
      }
      
      this.emit('cache-hit', key);
      this.updateCacheStats('hit');
      
      // Update access metadata
      await this.updateAccessMetadata(key);
      
      return this.deserialize<T>(value);
      
    } catch (error) {
      this.logger.error('Cache get error', error);
      throw error;
    }
  }
  
  /**
   * Set value in cache
   */
  async set<T = any>(
    key: string,
    value: T,
    ttl?: number,
    tags?: string[]
  ): Promise<void> {
    try {
      const serialized = this.serialize(value);
      
      if (ttl) {
        await this.client.setex(key, ttl, serialized);
      } else {
        await this.client.set(key, serialized);
      }
      
      // Store metadata
      await this.setMetadata(key, {
        size: serialized.length,
        ttl,
        tags
      });
      
      // Update tags index
      if (tags && tags.length > 0) {
        await this.addToTagIndex(key, tags);
      }
      
    } catch (error) {
      this.logger.error('Cache set error', error);
      throw error;
    }
  }
  
  /**
   * Delete from cache
   */
  async delete(key: string): Promise<void> {
    try {
      const meta = await this.getMetadata(key);
      
      await this.client.del(key);
      await this.client.del(`${key}:meta`);
      
      // Remove from tag indexes
      if (meta?.tags) {
        await this.removeFromTagIndex(key, meta.tags);
      }
      
      this.emit('cache-evict', key, 'manual');
      
    } catch (error) {
      this.logger.error('Cache delete error', error);
      throw error;
    }
  }
  
  /**
   * Delete by pattern
   */
  async deleteByPattern(pattern: string): Promise<number> {
    const keys = await this.keys(pattern);
    
    if (keys.length === 0) {
      return 0;
    }
    
    await Promise.all(keys.map(key => this.delete(key)));
    return keys.length;
  }
  
  /**
   * Delete by tags
   */
  async deleteByTags(tags: string[]): Promise<number> {
    const keys = await this.getKeysByTags(tags);
    
    if (keys.length === 0) {
      return 0;
    }
    
    await Promise.all(keys.map(key => this.delete(key)));
    return keys.length;
  }
  
  /**
   * Get keys by pattern
   */
  async keys(pattern: string): Promise<string[]> {
    if (this.client instanceof Redis.Cluster) {
      // For cluster, we need to scan all nodes
      const nodes = this.client.nodes('master');
      const allKeys: string[] = [];
      
      for (const node of nodes) {
        const keys = await this.scanKeys(node, pattern);
        allKeys.push(...keys);
      }
      
      return [...new Set(allKeys)];
    }
    
    return this.scanKeys(this.client, pattern);
  }
  
  /**
   * Scan keys using SCAN command
   */
  private async scanKeys(client: RedisClient, pattern: string): Promise<string[]> {
    const keys: string[] = [];
    let cursor = '0';
    
    do {
      const [nextCursor, batch] = await client.scan(
        cursor,
        'MATCH',
        `${this.config.keyPrefix}${pattern}`,
        'COUNT',
        '100'
      );
      
      keys.push(...batch);
      cursor = nextCursor;
    } while (cursor !== '0');
    
    return keys;
  }
  
  // Pub/Sub Operations
  
  /**
   * Publish message
   */
  async publish<T = any>(
    channel: PubSubChannel | string,
    type: string,
    data: T,
    metadata?: Record<string, any>
  ): Promise<void> {
    if (!this.pubClient) {
      throw new Error('Pub/sub not initialized');
    }
    
    const message: PubSubMessage<T> = {
      id: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      channel,
      type,
      source: this.config.connectionName || 'unknown',
      timestamp: Date.now(),
      data,
      metadata
    };
    
    await this.pubClient.publish(channel, JSON.stringify(message));
  }
  
  /**
   * Subscribe to channel
   */
  async subscribe(
    channel: PubSubChannel | string,
    handler?: (message: PubSubMessage) => void
  ): Promise<void> {
    if (!this.subClient) {
      throw new Error('Pub/sub not initialized');
    }
    
    await this.subClient.subscribe(channel);
    this.subscribedChannels.add(channel);
    
    if (handler) {
      this.on(`channel:${channel}`, handler);
    }
  }
  
  /**
   * Unsubscribe from channel
   */
  async unsubscribe(channel: PubSubChannel | string): Promise<void> {
    if (!this.subClient) {
      throw new Error('Pub/sub not initialized');
    }
    
    await this.subClient.unsubscribe(channel);
    this.subscribedChannels.delete(channel);
    this.removeAllListeners(`channel:${channel}`);
  }
  
  // Rate Limiting
  
  /**
   * Check rate limit
   */
  async checkRateLimit(
    key: string,
    config: RateLimitConfig
  ): Promise<RateLimitResult> {
    const limitKey = `${config.keyPrefix || 'ratelimit:'}${key}`;
    const now = Date.now();
    const windowStart = now - config.windowMs;
    
    // Remove old entries
    await this.client.zremrangebyscore(limitKey, '-inf', windowStart);
    
    // Count requests in window
    const count = await this.client.zcard(limitKey);
    
    if (count >= config.maxRequests) {
      // Get oldest entry to calculate retry time
      const oldest = await this.client.zrange(limitKey, 0, 0, 'WITHSCORES');
      const oldestTime = oldest.length > 1 ? parseInt(oldest[1]) : now;
      const resetAt = oldestTime + config.windowMs;
      
      return {
        allowed: false,
        remaining: 0,
        resetAt,
        retryAfter: resetAt - now
      };
    }
    
    // Add current request
    await this.client.zadd(limitKey, now, `${now}-${Math.random()}`);
    await this.client.expire(limitKey, Math.ceil(config.windowMs / 1000));
    
    return {
      allowed: true,
      remaining: config.maxRequests - count - 1,
      resetAt: now + config.windowMs
    };
  }
  
  // Session Management
  
  /**
   * Get session
   */
  async getSession(sessionId: string): Promise<SessionData | null> {
    const key = `session:${sessionId}`;
    const data = await this.get<SessionData>(key);
    
    if (data && data.expiresAt && data.expiresAt < Date.now()) {
      await this.delete(key);
      return null;
    }
    
    return data;
  }
  
  /**
   * Set session
   */
  async setSession(
    sessionId: string,
    data: Partial<SessionData>,
    ttl?: number
  ): Promise<void> {
    const key = `session:${sessionId}`;
    const existing = await this.getSession(sessionId);
    
    const session: SessionData = {
      id: sessionId,
      created: existing?.created || Date.now(),
      updated: Date.now(),
      ...existing,
      ...data,
      expiresAt: ttl ? Date.now() + ttl * 1000 : undefined
    };
    
    await this.set(key, session, ttl);
  }
  
  /**
   * Delete session
   */
  async deleteSession(sessionId: string): Promise<void> {
    await this.delete(`session:${sessionId}`);
  }
  
  // Distributed Locking
  
  /**
   * Acquire lock
   */
  async lock(
    key: string,
    options: LockOptions = {}
  ): Promise<boolean> {
    const lockKey = `lock:${key}`;
    const lockId = `${this.config.connectionName}:${Date.now()}`;
    const ttl = options.ttl || 30000;
    const retries = options.retries || 3;
    const retryDelay = options.retryDelay || 100;
    
    for (let i = 0; i < retries; i++) {
      const result = await this.client.set(
        lockKey,
        lockId,
        'PX',
        ttl,
        'NX'
      );
      
      if (result === 'OK') {
        // Set up auto-renewal
        const timeout = setInterval(async () => {
          const current = await this.client.get(lockKey);
          if (current === lockId) {
            await this.client.pexpire(lockKey, ttl);
          } else {
            clearInterval(timeout);
            this.locks.delete(key);
          }
        }, ttl / 2);
        
        this.locks.set(key, timeout);
        return true;
      }
      
      if (i < retries - 1) {
        await new Promise(resolve => setTimeout(resolve, retryDelay));
      }
    }
    
    return false;
  }
  
  /**
   * Release lock
   */
  async unlock(key: string): Promise<void> {
    const lockKey = `lock:${key}`;
    const timeout = this.locks.get(key);
    
    if (timeout) {
      clearInterval(timeout);
      this.locks.delete(key);
    }
    
    await this.client.del(lockKey);
  }
  
  // Helper Methods
  
  /**
   * Serialize value
   */
  private serialize<T>(value: T): string {
    return JSON.stringify(value);
  }
  
  /**
   * Deserialize value
   */
  private deserialize<T>(value: string): T {
    try {
      return JSON.parse(value);
    } catch {
      return value as any;
    }
  }
  
  /**
   * Get metadata for key
   */
  private async getMetadata(key: string): Promise<CacheEntryMeta | null> {
    return this.get<CacheEntryMeta>(`${key}:meta`);
  }
  
  /**
   * Set metadata for key
   */
  private async setMetadata(
    key: string,
    updates: Partial<CacheEntryMeta>
  ): Promise<void> {
    const existing = await this.getMetadata(key);
    
    const meta: CacheEntryMeta = {
      key,
      created: existing?.created || Date.now(),
      updated: Date.now(),
      accessed: Date.now(),
      accessCount: (existing?.accessCount || 0) + 1,
      size: 0,
      ...existing,
      ...updates
    };
    
    await this.client.set(`${key}:meta`, JSON.stringify(meta));
  }
  
  /**
   * Update access metadata
   */
  private async updateAccessMetadata(key: string): Promise<void> {
    const meta = await this.getMetadata(key);
    if (meta) {
      meta.accessed = Date.now();
      meta.accessCount++;
      await this.client.set(`${key}:meta`, JSON.stringify(meta));
    }
  }
  
  /**
   * Add key to tag index
   */
  private async addToTagIndex(key: string, tags: string[]): Promise<void> {
    const pipeline = this.client.pipeline();
    
    for (const tag of tags) {
      pipeline.sadd(`tag:${tag}`, key);
    }
    
    await pipeline.exec();
  }
  
  /**
   * Remove key from tag index
   */
  private async removeFromTagIndex(key: string, tags: string[]): Promise<void> {
    const pipeline = this.client.pipeline();
    
    for (const tag of tags) {
      pipeline.srem(`tag:${tag}`, key);
    }
    
    await pipeline.exec();
  }
  
  /**
   * Get keys by tags
   */
  private async getKeysByTags(tags: string[]): Promise<string[]> {
    if (tags.length === 0) {
      return [];
    }
    
    if (tags.length === 1) {
      return Array.from(await this.client.smembers(`tag:${tags[0]}`));
    }
    
    // Intersection of multiple tags
    const tagKeys = tags.map(tag => `tag:${tag}`);
    return Array.from(await this.client.sinter(...tagKeys));
  }
  
  /**
   * Update cache statistics
   */
  private updateCacheStats(type: 'hit' | 'miss'): void {
    const total = this.cacheStats.hitRate + this.cacheStats.missRate + 1;
    
    if (type === 'hit') {
      this.cacheStats.hitRate = (this.cacheStats.hitRate * (total - 1) + 1) / total;
      this.cacheStats.missRate = (this.cacheStats.missRate * (total - 1)) / total;
    } else {
      this.cacheStats.missRate = (this.cacheStats.missRate * (total - 1) + 1) / total;
      this.cacheStats.hitRate = (this.cacheStats.hitRate * (total - 1)) / total;
    }
  }
  
  /**
   * Update statistics
   */
  private async updateStats(): Promise<void> {
    try {
      const info = await this.client.info('memory');
      const keys = await this.client.dbsize();
      
      this.cacheStats.totalEntries = keys;
      
      // Parse memory usage from info
      const memMatch = info.match(/used_memory:(\d+)/);
      if (memMatch) {
        this.cacheStats.totalSize = parseInt(memMatch[1]);
      }
    } catch (error) {
      this.logger.error('Failed to update stats', error);
    }
  }
  
  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    return { ...this.cacheStats };
  }
}