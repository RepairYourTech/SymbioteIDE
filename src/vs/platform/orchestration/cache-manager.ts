/**
 * Cache Manager - Manages caching of AI model responses
 */

import * as crypto from 'crypto';
import { AITask, TaskResult, CachePolicy } from './interfaces';

export interface CacheEntry {
  key: string;
  task: AITask;
  result: TaskResult;
  timestamp: number;
  accessCount: number;
  lastAccessed: number;
  ttl: number;
  metadata: CacheMetadata;
}

export interface CacheMetadata {
  model: string;
  taskType: string;
  similarity?: number;
  tags?: string[];
}

export interface CacheStats {
  totalEntries: number;
  totalSize: number;
  hitRate: number;
  missRate: number;
  evictionCount: number;
  averageAccessCount: number;
}

export class CacheManager {
  private memoryCache: Map<string, CacheEntry> = new Map();
  private accessLog: Array<{ key: string; hit: boolean; timestamp: number }> = [];
  private evictionCount: number = 0;
  private maxMemorySize: number = 100 * 1024 * 1024; // 100MB default
  private maxEntries: number = 10000;

  constructor(
    private defaultTTL: number = 3600000, // 1 hour default
    maxMemorySize?: number,
    maxEntries?: number
  ) {
    if (maxMemorySize) this.maxMemorySize = maxMemorySize;
    if (maxEntries) this.maxEntries = maxEntries;
  }

  /**
   * Get cached result if available
   */
  get(task: AITask, policy?: CachePolicy): TaskResult | null {
    if (policy && !policy.enabled) {
      return null;
    }

    const key = this.generateCacheKey(task);
    const entry = this.memoryCache.get(key);

    // Log access attempt
    this.logAccess(key, !!entry);

    if (!entry) {
      return null;
    }

    // Check TTL
    const now = Date.now();
    const age = now - entry.timestamp;
    const ttl = policy?.ttl || entry.ttl;

    if (age > ttl) {
      this.memoryCache.delete(key);
      return null;
    }

    // Update access metadata
    entry.accessCount++;
    entry.lastAccessed = now;

    return {
      ...entry.result,
      metadata: {
        ...entry.result.metadata,
        cacheHit: true
      }
    };
  }

  /**
   * Store result in cache
   */
  set(
    task: AITask,
    result: TaskResult,
    policy?: CachePolicy,
    metadata?: Partial<CacheMetadata>
  ): void {
    if (policy && !policy.enabled) {
      return;
    }

    const key = this.generateCacheKey(task);
    const now = Date.now();

    const entry: CacheEntry = {
      key,
      task: this.sanitizeTask(task),
      result: this.sanitizeResult(result),
      timestamp: now,
      accessCount: 0,
      lastAccessed: now,
      ttl: policy?.ttl || this.defaultTTL,
      metadata: {
        model: result.model,
        taskType: task.type,
        ...metadata
      }
    };

    // Check size limits
    this.enforceMemoryLimit();

    this.memoryCache.set(key, entry);
  }

  /**
   * Find similar cached results
   */
  findSimilar(
    task: AITask,
    threshold: number = 0.8,
    limit: number = 5
  ): Array<{ entry: CacheEntry; similarity: number }> {
    const results: Array<{ entry: CacheEntry; similarity: number }> = [];

    for (const entry of this.memoryCache.values()) {
      // Skip expired entries
      if (Date.now() - entry.timestamp > entry.ttl) {
        continue;
      }

      const similarity = this.calculateSimilarity(task, entry.task);
      if (similarity >= threshold) {
        results.push({ entry, similarity });
      }
    }

    // Sort by similarity and limit results
    return results
      .sort((a, b) => b.similarity - a.similarity)
      .slice(0, limit);
  }

  /**
   * Invalidate cache entries
   */
  invalidate(filter?: {
    model?: string;
    taskType?: string;
    olderThan?: number;
    tags?: string[];
  }): number {
    let invalidated = 0;
    const now = Date.now();

    for (const [key, entry] of this.memoryCache.entries()) {
      let shouldInvalidate = false;

      if (filter) {
        if (filter.model && entry.metadata.model !== filter.model) continue;
        if (filter.taskType && entry.metadata.taskType !== filter.taskType) continue;
        if (filter.olderThan && now - entry.timestamp < filter.olderThan) continue;
        if (filter.tags && !filter.tags.some(tag => entry.metadata.tags?.includes(tag))) continue;
      }

      this.memoryCache.delete(key);
      invalidated++;
    }

    return invalidated;
  }

  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    const totalEntries = this.memoryCache.size;
    let totalSize = 0;
    let totalAccessCount = 0;

    for (const entry of this.memoryCache.values()) {
      totalSize += this.estimateEntrySize(entry);
      totalAccessCount += entry.accessCount;
    }

    const recentLogs = this.accessLog.slice(-1000);
    const hits = recentLogs.filter(log => log.hit).length;
    const hitRate = recentLogs.length > 0 ? hits / recentLogs.length : 0;

    return {
      totalEntries,
      totalSize,
      hitRate,
      missRate: 1 - hitRate,
      evictionCount: this.evictionCount,
      averageAccessCount: totalEntries > 0 ? totalAccessCount / totalEntries : 0
    };
  }

  /**
   * Clear all cache entries
   */
  clear(): void {
    this.memoryCache.clear();
    this.accessLog = [];
  }

  /**
   * Export cache data
   */
  exportCache(): string {
    const entries = Array.from(this.memoryCache.values()).map(entry => ({
      ...entry,
      task: this.sanitizeTask(entry.task),
      result: this.sanitizeResult(entry.result)
    }));

    return JSON.stringify({
      exportTime: Date.now(),
      entries,
      stats: this.getStats()
    }, null, 2);
  }

  /**
   * Import cache data
   */
  importCache(json: string): void {
    const data = JSON.parse(json);
    
    if (data.entries && Array.isArray(data.entries)) {
      for (const entry of data.entries) {
        this.memoryCache.set(entry.key, entry);
      }
    }
  }

  // Private methods

  private generateCacheKey(task: AITask): string {
    const normalized = {
      type: task.type,
      prompt: task.prompt.trim().toLowerCase(),
      context: task.context ? this.normalizeContext(task.context) : null,
      constraints: task.constraints ? {
        temperature: task.constraints.temperature,
        maxTokens: task.constraints.maxTokens,
        topP: task.constraints.topP
      } : null
    };

    const hash = crypto.createHash('sha256');
    hash.update(JSON.stringify(normalized));
    return hash.digest('hex');
  }

  private normalizeContext(context: any): any {
    // Normalize context for consistent hashing
    if (context.files) {
      return {
        files: context.files.map((f: any) => ({
          path: f.path,
          content: f.content.trim(),
          language: f.language
        })).sort((a: any, b: any) => a.path.localeCompare(b.path))
      };
    }
    return context;
  }

  private calculateSimilarity(task1: AITask, task2: AITask): number {
    let similarity = 0;
    let weights = 0;

    // Type similarity (30% weight)
    if (task1.type === task2.type) {
      similarity += 0.3;
    }
    weights += 0.3;

    // Prompt similarity (50% weight)
    const promptSim = this.textSimilarity(task1.prompt, task2.prompt);
    similarity += promptSim * 0.5;
    weights += 0.5;

    // Context similarity (20% weight)
    if (task1.context && task2.context) {
      const contextSim = this.contextSimilarity(task1.context, task2.context);
      similarity += contextSim * 0.2;
      weights += 0.2;
    } else if (!task1.context && !task2.context) {
      similarity += 0.2;
      weights += 0.2;
    }

    return weights > 0 ? similarity / weights : 0;
  }

  private textSimilarity(text1: string, text2: string): number {
    // Simple Jaccard similarity for now
    const words1 = new Set(text1.toLowerCase().split(/\s+/));
    const words2 = new Set(text2.toLowerCase().split(/\s+/));
    
    const intersection = new Set([...words1].filter(x => words2.has(x)));
    const union = new Set([...words1, ...words2]);
    
    return union.size > 0 ? intersection.size / union.size : 0;
  }

  private contextSimilarity(context1: any, context2: any): number {
    // Simplified context comparison
    if (context1.files && context2.files) {
      const paths1 = new Set(context1.files.map((f: any) => f.path));
      const paths2 = new Set(context2.files.map((f: any) => f.path));
      
      const intersection = new Set([...paths1].filter(x => paths2.has(x)));
      const union = new Set([...paths1, ...paths2]);
      
      return union.size > 0 ? intersection.size / union.size : 0;
    }
    return 0;
  }

  private sanitizeTask(task: AITask): AITask {
    // Remove sensitive information
    return {
      ...task,
      metadata: task.metadata ? {
        ...task.metadata,
        userId: undefined,
        tracking: undefined
      } : undefined
    };
  }

  private sanitizeResult(result: TaskResult): TaskResult {
    // Remove sensitive information
    return {
      ...result,
      metadata: result.metadata ? {
        ...result.metadata,
        processingTime: undefined
      } : undefined
    };
  }

  private enforceMemoryLimit(): void {
    // Check number of entries
    if (this.memoryCache.size >= this.maxEntries) {
      this.evictLRU();
    }

    // Check memory size
    let totalSize = 0;
    for (const entry of this.memoryCache.values()) {
      totalSize += this.estimateEntrySize(entry);
    }

    while (totalSize > this.maxMemorySize && this.memoryCache.size > 0) {
      this.evictLRU();
      
      totalSize = 0;
      for (const entry of this.memoryCache.values()) {
        totalSize += this.estimateEntrySize(entry);
      }
    }
  }

  private evictLRU(): void {
    let oldest: CacheEntry | null = null;
    let oldestKey: string | null = null;

    for (const [key, entry] of this.memoryCache.entries()) {
      if (!oldest || entry.lastAccessed < oldest.lastAccessed) {
        oldest = entry;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      this.memoryCache.delete(oldestKey);
      this.evictionCount++;
    }
  }

  private estimateEntrySize(entry: CacheEntry): number {
    // Rough estimation of memory usage
    const taskSize = JSON.stringify(entry.task).length;
    const resultSize = JSON.stringify(entry.result).length;
    const overhead = 1024; // Metadata and structure overhead
    
    return taskSize + resultSize + overhead;
  }

  private logAccess(key: string, hit: boolean): void {
    this.accessLog.push({ key, hit, timestamp: Date.now() });
    
    // Keep only recent logs
    if (this.accessLog.length > 10000) {
      this.accessLog = this.accessLog.slice(-5000);
    }
  }
}