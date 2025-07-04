/**
 * Rate Limiter Implementation using Token Bucket Algorithm
 */

import { RateLimiter, RateLimitConfig } from './queue-interfaces';

interface TokenBucket {
  tokens: number;
  lastRefill: number;
  capacity: number;
  refillRate: number;
}

export class TokenBucketRateLimiter implements RateLimiter {
  private buckets: Map<string, TokenBucket>;
  private cleanupInterval: NodeJS.Timeout | null = null;
  private readonly cleanupIntervalMs = 60000; // Clean up every minute
  private readonly maxAge = 3600000; // Remove buckets older than 1 hour

  constructor() {
    this.buckets = new Map();
    this.startCleanup();
  }

  /**
   * Check if request is within rate limit
   */
  async checkLimit(key: string, limit: RateLimitConfig): Promise<boolean> {
    const bucketKey = this.getBucketKey(key, limit);
    const bucket = this.getOrCreateBucket(bucketKey, limit);
    
    this.refillTokens(bucket, limit);
    
    return bucket.tokens >= 1;
  }

  /**
   * Consume a token for the request
   */
  async consumeToken(key: string, limit: RateLimitConfig): Promise<void> {
    const bucketKey = this.getBucketKey(key, limit);
    const bucket = this.getOrCreateBucket(bucketKey, limit);
    
    this.refillTokens(bucket, limit);
    
    if (bucket.tokens < 1) {
      throw new Error(`Rate limit exceeded for ${key}`);
    }
    
    bucket.tokens -= 1;
  }

  /**
   * Get remaining tokens in the bucket
   */
  async getRemainingTokens(key: string, limit: RateLimitConfig): Promise<number> {
    const bucketKey = this.getBucketKey(key, limit);
    const bucket = this.getOrCreateBucket(bucketKey, limit);
    
    this.refillTokens(bucket, limit);
    
    return Math.floor(bucket.tokens);
  }

  /**
   * Reset rate limit for a specific key or all keys
   */
  async reset(key?: string): Promise<void> {
    if (key) {
      // Remove all buckets for this key
      const keysToDelete: string[] = [];
      for (const bucketKey of this.buckets.keys()) {
        if (bucketKey.startsWith(key + ':')) {
          keysToDelete.push(bucketKey);
        }
      }
      keysToDelete.forEach(k => this.buckets.delete(k));
    } else {
      // Reset all buckets
      this.buckets.clear();
    }
  }

  /**
   * Get time until next token refill
   */
  async getTimeUntilRefill(key: string, limit: RateLimitConfig): Promise<number> {
    const bucketKey = this.getBucketKey(key, limit);
    const bucket = this.getOrCreateBucket(bucketKey, limit);
    
    if (bucket.tokens >= 1) {
      return 0;
    }
    
    // Calculate time until at least 1 token is available
    const tokensNeeded = 1 - bucket.tokens;
    const refillTimeMs = (tokensNeeded / bucket.refillRate) * 1000;
    
    return Math.ceil(refillTimeMs);
  }

  /**
   * Get rate limit status for debugging
   */
  async getStatus(key: string, limit: RateLimitConfig): Promise<{
    limit: number;
    remaining: number;
    resetAt: Date;
    windowMs: number;
  }> {
    const bucketKey = this.getBucketKey(key, limit);
    const bucket = this.getOrCreateBucket(bucketKey, limit);
    
    this.refillTokens(bucket, limit);
    
    const timeUntilRefill = await this.getTimeUntilRefill(key, limit);
    
    return {
      limit: limit.limit,
      remaining: Math.floor(bucket.tokens),
      resetAt: new Date(Date.now() + timeUntilRefill),
      windowMs: limit.window
    };
  }

  /**
   * Stop the cleanup interval
   */
  destroy(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }

  /**
   * Generate bucket key based on scope
   */
  private getBucketKey(key: string, limit: RateLimitConfig): string {
    const scope = limit.scope || 'global';
    const customKey = limit.key || '';
    
    switch (scope) {
      case 'global':
        return `global:${limit.name}`;
      case 'user':
        return `user:${key}:${limit.name}`;
      case 'model':
        return `model:${key}:${limit.name}`;
      case 'task-type':
        return `task-type:${key}:${limit.name}`;
      default:
        return `${scope}:${key}:${customKey}:${limit.name}`;
    }
  }

  /**
   * Get or create a token bucket
   */
  private getOrCreateBucket(key: string, limit: RateLimitConfig): TokenBucket {
    let bucket = this.buckets.get(key);
    
    if (!bucket) {
      bucket = {
        tokens: limit.limit,
        lastRefill: Date.now(),
        capacity: limit.limit,
        refillRate: limit.limit / (limit.window / 1000) // tokens per second
      };
      this.buckets.set(key, bucket);
    }
    
    return bucket;
  }

  /**
   * Refill tokens based on elapsed time
   */
  private refillTokens(bucket: TokenBucket, limit: RateLimitConfig): void {
    const now = Date.now();
    const elapsedSeconds = (now - bucket.lastRefill) / 1000;
    
    const tokensToAdd = elapsedSeconds * bucket.refillRate;
    bucket.tokens = Math.min(bucket.capacity, bucket.tokens + tokensToAdd);
    bucket.lastRefill = now;
  }

  /**
   * Start periodic cleanup of old buckets
   */
  private startCleanup(): void {
    this.cleanupInterval = setInterval(() => {
      this.cleanupOldBuckets();
    }, this.cleanupIntervalMs);
  }

  /**
   * Remove old unused buckets
   */
  private cleanupOldBuckets(): void {
    const now = Date.now();
    const keysToDelete: string[] = [];
    
    for (const [key, bucket] of this.buckets) {
      if (now - bucket.lastRefill > this.maxAge) {
        keysToDelete.push(key);
      }
    }
    
    keysToDelete.forEach(key => this.buckets.delete(key));
  }
}

/**
 * Sliding Window Rate Limiter Implementation
 * More accurate but more memory intensive than token bucket
 */
export class SlidingWindowRateLimiter implements RateLimiter {
  private requests: Map<string, number[]>;
  private cleanupInterval: NodeJS.Timeout | null = null;

  constructor() {
    this.requests = new Map();
    this.startCleanup();
  }

  async checkLimit(key: string, limit: RateLimitConfig): Promise<boolean> {
    const bucketKey = this.getBucketKey(key, limit);
    const timestamps = this.getTimestamps(bucketKey);
    const cutoff = Date.now() - limit.window;
    
    const validRequests = timestamps.filter(ts => ts > cutoff);
    return validRequests.length < limit.limit;
  }

  async consumeToken(key: string, limit: RateLimitConfig): Promise<void> {
    const bucketKey = this.getBucketKey(key, limit);
    const timestamps = this.getTimestamps(bucketKey);
    const now = Date.now();
    const cutoff = now - limit.window;
    
    // Remove expired timestamps
    const validTimestamps = timestamps.filter(ts => ts > cutoff);
    
    if (validTimestamps.length >= limit.limit) {
      throw new Error(`Rate limit exceeded for ${key}`);
    }
    
    validTimestamps.push(now);
    this.requests.set(bucketKey, validTimestamps);
  }

  async getRemainingTokens(key: string, limit: RateLimitConfig): Promise<number> {
    const bucketKey = this.getBucketKey(key, limit);
    const timestamps = this.getTimestamps(bucketKey);
    const cutoff = Date.now() - limit.window;
    
    const validRequests = timestamps.filter(ts => ts > cutoff);
    return Math.max(0, limit.limit - validRequests.length);
  }

  async reset(key?: string): Promise<void> {
    if (key) {
      const keysToDelete: string[] = [];
      for (const bucketKey of this.requests.keys()) {
        if (bucketKey.includes(key)) {
          keysToDelete.push(bucketKey);
        }
      }
      keysToDelete.forEach(k => this.requests.delete(k));
    } else {
      this.requests.clear();
    }
  }

  destroy(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }

  private getBucketKey(key: string, limit: RateLimitConfig): string {
    const scope = limit.scope || 'global';
    return `${scope}:${key}:${limit.name}`;
  }

  private getTimestamps(key: string): number[] {
    return this.requests.get(key) || [];
  }

  private startCleanup(): void {
    // Clean up every minute
    this.cleanupInterval = setInterval(() => {
      const now = Date.now();
      for (const [key, timestamps] of this.requests) {
        // Keep only timestamps from last hour
        const validTimestamps = timestamps.filter(ts => now - ts < 3600000);
        if (validTimestamps.length === 0) {
          this.requests.delete(key);
        } else {
          this.requests.set(key, validTimestamps);
        }
      }
    }, 60000);
  }
}

/**
 * Composite Rate Limiter
 * Allows combining multiple rate limit strategies
 */
export class CompositeRateLimiter implements RateLimiter {
  private limiters: RateLimiter[];

  constructor(limiters: RateLimiter[]) {
    this.limiters = limiters;
  }

  async checkLimit(key: string, limit: RateLimitConfig): Promise<boolean> {
    for (const limiter of this.limiters) {
      if (!(await limiter.checkLimit(key, limit))) {
        return false;
      }
    }
    return true;
  }

  async consumeToken(key: string, limit: RateLimitConfig): Promise<void> {
    // Check all limiters first
    for (const limiter of this.limiters) {
      if (!(await limiter.checkLimit(key, limit))) {
        throw new Error(`Rate limit exceeded for ${key}`);
      }
    }
    
    // Consume from all limiters
    for (const limiter of this.limiters) {
      await limiter.consumeToken(key, limit);
    }
  }

  async getRemainingTokens(key: string, limit: RateLimitConfig): Promise<number> {
    let minRemaining = Infinity;
    
    for (const limiter of this.limiters) {
      const remaining = await limiter.getRemainingTokens(key, limit);
      minRemaining = Math.min(minRemaining, remaining);
    }
    
    return minRemaining;
  }

  async reset(key?: string): Promise<void> {
    await Promise.all(this.limiters.map(l => l.reset(key)));
  }
}