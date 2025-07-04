/**
 * Redis Cache Manager
 * 
 * Distributed cache manager using Redis for AI model response caching
 */

import * as crypto from 'crypto';
import { EventEmitter } from 'events';
import { AITask, TaskResult, CachePolicy } from './interfaces';
import { CacheEntry, CacheMetadata, CacheStats } from './cache-manager';
import { RedisManager, PubSubChannel } from '../redis';
import { Logger } from '../utils/logger';

export interface RedisCacheConfig {
  redisManager: RedisManager;
  keyPrefix?: string;
  defaultTTL?: number;
  maxEntries?: number;
  enableBroadcast?: boolean;
  compressionThreshold?: number;
}

export class RedisCacheManager extends EventEmitter {
  private redis: RedisManager;
  private config: Required<RedisCacheConfig>;
  private logger = new Logger('RedisCacheManager');
  private localStats = {
    hits: 0,
    misses: 0,
    writes: 0,
    evictions: 0
  };
  
  constructor(config: RedisCacheConfig) {
    super();
    
    this.redis = config.redisManager;
    this.config = {
      redisManager: config.redisManager,
      keyPrefix: config.keyPrefix || 'ai-cache:',
      defaultTTL: config.defaultTTL || 3600, // 1 hour in seconds
      maxEntries: config.maxEntries || 10000,
      enableBroadcast: config.enableBroadcast ?? true,
      compressionThreshold: config.compressionThreshold || 1024 // 1KB
    };
    
    // Subscribe to cache invalidation events
    if (this.config.enableBroadcast) {
      this.setupBroadcastHandlers();
    }
  }
  
  /**
   * Set up broadcast handlers for cache invalidation
   */
  private async setupBroadcastHandlers(): Promise<void> {
    await this.redis.subscribe(PubSubChannel.SystemStatus, (message) => {
      if (message.type === 'cache-invalidate') {
        this.handleRemoteInvalidation(message.data);
      }
    });
  }
  
  /**
   * Get cached result
   */
  async get(task: AITask, policy?: CachePolicy): Promise<TaskResult | null> {
    if (policy && !policy.enabled) {
      return null;
    }
    
    const key = this.generateCacheKey(task);
    const fullKey = `${this.config.keyPrefix}${key}`;
    
    try {
      const entry = await this.redis.get<CacheEntry>(fullKey);
      
      if (!entry) {
        this.localStats.misses++;
        this.emit('cache-miss', { task, key });
        return null;
      }
      
      // Check if entry is still valid
      const now = Date.now();
      const age = now - entry.timestamp;
      const ttl = policy?.ttl || entry.ttl;
      
      if (age > ttl) {
        await this.redis.delete(fullKey);
        this.localStats.misses++;
        this.emit('cache-expired', { task, key, age });
        return null;
      }
      
      // Update access count (async, don't wait)
      this.updateAccessCount(fullKey, entry).catch(err => 
        this.logger.error('Failed to update access count', err)
      );
      
      this.localStats.hits++;
      this.emit('cache-hit', { task, key, age });
      
      return {
        ...entry.result,
        metadata: {
          ...entry.result.metadata,
          cacheHit: true,
          cacheAge: age,
          accessCount: entry.accessCount + 1
        }
      };
      
    } catch (error) {
      this.logger.error('Cache get error', error);
      return null;
    }
  }
  
  /**
   * Store result in cache
   */
  async set(
    task: AITask,
    result: TaskResult,
    policy?: CachePolicy,
    metadata?: Partial<CacheMetadata>
  ): Promise<void> {
    if (policy && !policy.enabled) {
      return;
    }
    
    const key = this.generateCacheKey(task);
    const fullKey = `${this.config.keyPrefix}${key}`;
    const now = Date.now();
    const ttl = policy?.ttl || this.config.defaultTTL;
    
    try {
      const entry: CacheEntry = {
        key,
        task: this.sanitizeTask(task),
        result: this.sanitizeResult(result),
        timestamp: now,
        accessCount: 0,
        lastAccessed: now,
        ttl: ttl * 1000, // Convert to milliseconds for consistency
        metadata: {
          model: result.model,
          taskType: task.type,
          ...metadata
        }
      };
      
      // Check cache size limit
      await this.enforceMemoryLimit();
      
      // Store with tags for efficient invalidation
      const tags = [
        `model:${result.model}`,
        `type:${task.type}`,
        ...(metadata?.tags || [])
      ];
      
      await this.redis.set(fullKey, entry, ttl, tags);
      
      this.localStats.writes++;
      this.emit('cache-write', { task, key, ttl });
      
      // Broadcast cache update if enabled
      if (this.config.enableBroadcast) {
        await this.broadcastCacheUpdate(key, 'set');
      }
      
    } catch (error) {
      this.logger.error('Cache set error', error);
      throw error;
    }
  }
  
  /**
   * Find similar cached results using semantic search
   */
  async findSimilar(
    task: AITask,
    threshold: number = 0.8,
    limit: number = 5
  ): Promise<Array<{ entry: CacheEntry; similarity: number }>> {
    // This would integrate with Qdrant for semantic similarity
    // For now, we'll do a pattern-based search
    
    const pattern = `${this.config.keyPrefix}*${task.type}*`;
    const keys = await this.redis.keys(pattern);
    const results: Array<{ entry: CacheEntry; similarity: number }> = [];
    
    // Fetch entries in parallel
    const entries = await Promise.all(
      keys.slice(0, 50).map(key => this.redis.get<CacheEntry>(key))
    );
    
    for (const entry of entries) {
      if (!entry) continue;
      
      // Check if not expired
      const age = Date.now() - entry.timestamp;
      if (age > entry.ttl) continue;
      
      const similarity = this.calculateSimilarity(task, entry.task);
      if (similarity >= threshold) {
        results.push({ entry, similarity });
      }
    }
    
    return results
      .sort((a, b) => b.similarity - a.similarity)
      .slice(0, limit);
  }
  
  /**
   * Invalidate cache entries
   */
  async invalidate(filter?: {
    model?: string;
    taskType?: string;
    olderThan?: number;
    tags?: string[];
  }): Promise<number> {
    let invalidated = 0;
    
    try {
      if (filter?.tags) {
        // Delete by tags
        invalidated = await this.redis.deleteByTags(filter.tags);
      } else if (filter?.model) {
        // Delete by model tag
        invalidated = await this.redis.deleteByTags([`model:${filter.model}`]);
      } else if (filter?.taskType) {
        // Delete by task type tag
        invalidated = await this.redis.deleteByTags([`type:${filter.taskType}`]);
      } else {
        // Delete all cache entries
        invalidated = await this.redis.deleteByPattern(`${this.config.keyPrefix}*`);
      }
      
      this.localStats.evictions += invalidated;
      this.emit('cache-invalidate', { filter, count: invalidated });
      
      // Broadcast invalidation
      if (this.config.enableBroadcast && invalidated > 0) {
        await this.broadcastInvalidation(filter);
      }
      
      return invalidated;
      
    } catch (error) {
      this.logger.error('Cache invalidation error', error);
      throw error;
    }
  }
  
  /**
   * Get cache statistics
   */
  async getStats(): Promise<CacheStats> {
    const redisStats = await this.redis.getStats();
    const pattern = `${this.config.keyPrefix}*`;
    const keys = await this.redis.keys(pattern);
    
    // Sample some entries to calculate average access count
    const sampleSize = Math.min(100, keys.length);
    const sampleKeys = keys.slice(0, sampleSize);
    const entries = await Promise.all(
      sampleKeys.map(key => this.redis.get<CacheEntry>(key))
    );
    
    let totalAccessCount = 0;
    let validEntries = 0;
    
    for (const entry of entries) {
      if (entry) {
        totalAccessCount += entry.accessCount;
        validEntries++;
      }
    }
    
    const total = this.localStats.hits + this.localStats.misses;
    const hitRate = total > 0 ? this.localStats.hits / total : 0;
    const missRate = total > 0 ? this.localStats.misses / total : 0;
    
    return {
      totalEntries: keys.length,
      totalSize: redisStats.totalSize,
      hitRate,
      missRate,
      evictionCount: this.localStats.evictions,
      averageAccessCount: validEntries > 0 ? totalAccessCount / validEntries : 0
    };
  }
  
  /**
   * Clear all cache entries
   */
  async clear(): Promise<void> {
    const count = await this.redis.deleteByPattern(`${this.config.keyPrefix}*`);
    this.localStats.evictions += count;
    this.emit('cache-clear', { count });
    
    if (this.config.enableBroadcast) {
      await this.broadcastCacheUpdate('*', 'clear');
    }
  }
  
  /**
   * Warm up cache with predefined entries
   */
  async warmUp(entries: Array<{
    task: AITask;
    result: TaskResult;
    policy?: CachePolicy;
  }>): Promise<void> {
    this.logger.info(`Warming up cache with ${entries.length} entries`);
    
    await Promise.all(
      entries.map(({ task, result, policy }) => 
        this.set(task, result, policy).catch(err => 
          this.logger.error('Failed to warm up entry', err)
        )
      )
    );
    
    this.emit('cache-warmup', { count: entries.length });
  }
  
  // Private helper methods
  
  private generateCacheKey(task: AITask): string {
    const normalized = {
      type: task.type,
      prompt: task.prompt?.trim(),
      model: task.model,
      temperature: task.parameters?.temperature,
      maxTokens: task.parameters?.maxTokens
    };
    
    const data = JSON.stringify(normalized);
    return crypto.createHash('sha256').update(data).digest('hex');
  }
  
  private sanitizeTask(task: AITask): AITask {
    return {
      ...task,
      prompt: task.prompt.substring(0, 1000) // Limit prompt length
    };
  }
  
  private sanitizeResult(result: TaskResult): TaskResult {
    return {
      ...result,
      response: result.response.substring(0, 5000) // Limit response length
    };
  }
  
  private calculateSimilarity(task1: AITask, task2: AITask): number {
    // Simple similarity based on type and model
    let score = 0;
    
    if (task1.type === task2.type) score += 0.4;
    if (task1.model === task2.model) score += 0.3;
    if (task1.parameters?.temperature === task2.parameters?.temperature) score += 0.1;
    
    // Prompt similarity (simplified)
    if (task1.prompt && task2.prompt) {
      const words1 = new Set(task1.prompt.toLowerCase().split(/\s+/));
      const words2 = new Set(task2.prompt.toLowerCase().split(/\s+/));
      const intersection = new Set([...words1].filter(x => words2.has(x)));
      const union = new Set([...words1, ...words2]);
      score += 0.2 * (intersection.size / union.size);
    }
    
    return score;
  }
  
  private async updateAccessCount(key: string, entry: CacheEntry): Promise<void> {
    entry.accessCount++;
    entry.lastAccessed = Date.now();
    await this.redis.set(key, entry, Math.ceil(entry.ttl / 1000));
  }
  
  private async enforceMemoryLimit(): Promise<void> {
    const pattern = `${this.config.keyPrefix}*`;
    const keys = await this.redis.keys(pattern);
    
    if (keys.length >= this.config.maxEntries) {
      // Get oldest entries
      const entries = await Promise.all(
        keys.map(async key => ({
          key,
          entry: await this.redis.get<CacheEntry>(key)
        }))
      );
      
      // Sort by last accessed time and remove oldest
      const sorted = entries
        .filter(e => e.entry !== null)
        .sort((a, b) => a.entry!.lastAccessed - b.entry!.lastAccessed);
      
      const toRemove = sorted.slice(0, Math.ceil(keys.length * 0.1)); // Remove 10%
      
      await Promise.all(
        toRemove.map(({ key }) => this.redis.delete(key))
      );
      
      this.localStats.evictions += toRemove.length;
      this.emit('cache-eviction', { count: toRemove.length, reason: 'limit' });
    }
  }
  
  private async broadcastCacheUpdate(key: string, action: string): Promise<void> {
    await this.redis.publish(
      PubSubChannel.SystemStatus,
      'cache-update',
      { key, action },
      { source: 'cache-manager' }
    );
  }
  
  private async broadcastInvalidation(filter?: any): Promise<void> {
    await this.redis.publish(
      PubSubChannel.SystemStatus,
      'cache-invalidate',
      { filter },
      { source: 'cache-manager' }
    );
  }
  
  private handleRemoteInvalidation(data: any): void {
    // Handle invalidation from other instances
    this.emit('remote-invalidation', data);
  }
}