/**
 * Redis Cache Storage
 * 
 * Redis-based distributed cache storage implementation
 */

import Redis from 'ioredis';
import { CacheEntry } from '../types';
import { BaseCacheStorage } from './cache-storage';

interface RedisStorageConfig {
  host: string;
  port: number;
  password?: string;
  db?: number;
  keyPrefix?: string;
}

export class RedisCacheStorage extends BaseCacheStorage {
  private redis: Redis;
  private keyPrefix: string;
  
  constructor(config: RedisStorageConfig) {
    super(config);
    
    this.redis = new Redis({
      host: config.host,
      port: config.port,
      password: config.password,
      db: config.db || 0,
      retryStrategy: (times) => Math.min(times * 50, 2000)
    });
    
    this.keyPrefix = config.keyPrefix || 'cache:';
    
    // Handle connection events
    this.redis.on('error', (error) => {
      console.error('Redis cache connection error:', error);
    });
    
    this.redis.on('connect', () => {
      console.log('Redis cache connected');
    });
  }
  
  async get(key: string): Promise<CacheEntry | null> {
    try {
      const fullKey = this.getFullKey(key);
      const data = await this.redis.get(fullKey);
      
      if (!data) {
        return null;
      }
      
      const entry = this.deserialize(data);
      
      // Check if expired (Redis TTL should handle this, but double-check)
      if (this.isExpired(entry)) {
        await this.delete(key);
        return null;
      }
      
      return entry;
      
    } catch (error) {
      console.error('Redis get error:', error);
      return null;
    }
  }
  
  async set(key: string, entry: CacheEntry, ttl?: number): Promise<void> {
    try {
      const fullKey = this.getFullKey(key);
      const data = this.serialize(entry);
      
      // Use provided TTL or entry TTL
      const effectiveTTL = ttl !== undefined ? ttl : entry.ttl;
      
      if (effectiveTTL && effectiveTTL > 0) {
        await this.redis.setex(fullKey, effectiveTTL, data);
      } else {
        await this.redis.set(fullKey, data);
      }
      
    } catch (error) {
      console.error('Redis set error:', error);
      throw error;
    }
  }
  
  async delete(key: string): Promise<boolean> {
    try {
      const fullKey = this.getFullKey(key);
      const result = await this.redis.del(fullKey);
      return result > 0;
      
    } catch (error) {
      console.error('Redis delete error:', error);
      return false;
    }
  }
  
  async exists(key: string): Promise<boolean> {
    try {
      const fullKey = this.getFullKey(key);
      const result = await this.redis.exists(fullKey);
      return result > 0;
      
    } catch (error) {
      console.error('Redis exists error:', error);
      return false;
    }
  }
  
  async keys(pattern: string): Promise<string[]> {
    try {
      const fullPattern = this.getFullKey(pattern);
      const fullKeys = await this.redis.keys(fullPattern);
      
      // Remove prefix from keys
      return fullKeys.map(fullKey => 
        fullKey.substring(this.keyPrefix.length)
      );
      
    } catch (error) {
      console.error('Redis keys error:', error);
      return [];
    }
  }
  
  async clear(): Promise<void> {
    try {
      // Get all cache keys
      const pattern = this.getFullKey('*');
      const keys = await this.redis.keys(pattern);
      
      if (keys.length > 0) {
        // Delete in batches
        const batchSize = 1000;
        for (let i = 0; i < keys.length; i += batchSize) {
          const batch = keys.slice(i, i + batchSize);
          await this.redis.del(...batch);
        }
      }
      
    } catch (error) {
      console.error('Redis clear error:', error);
      throw error;
    }
  }
  
  async getStats(): Promise<{
    size: number;
    entries: number;
    oldest: Date | null;
    newest: Date | null;
  }> {
    try {
      // Get all cache keys
      const pattern = this.getFullKey('*');
      const keys = await this.redis.keys(pattern);
      
      if (keys.length === 0) {
        return {
          size: 0,
          entries: 0,
          oldest: null,
          newest: null
        };
      }
      
      let totalSize = 0;
      let oldest: Date | null = null;
      let newest: Date | null = null;
      
      // Sample keys for statistics (avoid loading all entries)
      const sampleSize = Math.min(100, keys.length);
      const sampleIndices = this.getSampleIndices(keys.length, sampleSize);
      
      for (const index of sampleIndices) {
        const key = keys[index];
        const data = await this.redis.get(key);
        
        if (data) {
          totalSize += Buffer.byteLength(data, 'utf8');
          
          try {
            const entry = this.deserialize(data);
            if (!oldest || entry.createdAt < oldest) {
              oldest = entry.createdAt;
            }
            if (!newest || entry.createdAt > newest) {
              newest = entry.createdAt;
            }
          } catch (e) {
            // Skip invalid entries
          }
        }
      }
      
      // Estimate total size based on sample
      const estimatedTotalSize = keys.length > sampleSize
        ? Math.round(totalSize * keys.length / sampleSize)
        : totalSize;
      
      return {
        size: estimatedTotalSize,
        entries: keys.length,
        oldest,
        newest
      };
      
    } catch (error) {
      console.error('Redis getStats error:', error);
      return {
        size: 0,
        entries: 0,
        oldest: null,
        newest: null
      };
    }
  }
  
  /**
   * Close Redis connection
   */
  async dispose(): Promise<void> {
    await this.redis.quit();
  }
  
  // Private methods
  
  private getFullKey(key: string): string {
    return this.keyPrefix + key;
  }
  
  private getSampleIndices(total: number, sampleSize: number): number[] {
    const indices: number[] = [];
    const step = Math.floor(total / sampleSize);
    
    for (let i = 0; i < sampleSize && i * step < total; i++) {
      indices.push(i * step);
    }
    
    return indices;
  }
}