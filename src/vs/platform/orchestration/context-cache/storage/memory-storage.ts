/**
 * Memory Cache Storage
 * 
 * In-memory cache storage implementation with LRU eviction
 */

import { CacheEntry } from '../types';
import { BaseCacheStorage } from './cache-storage';

interface MemoryStorageConfig {
  maxSize: number; // MB
  evictionPolicy: 'lru' | 'lfu' | 'fifo';
}

export class MemoryCacheStorage extends BaseCacheStorage {
  private cache: Map<string, CacheEntry>;
  private accessOrder: string[]; // For LRU
  private accessCounts: Map<string, number>; // For LFU
  private currentSize: number;
  private maxSizeBytes: number;
  
  constructor(config: MemoryStorageConfig) {
    super(config);
    this.cache = new Map();
    this.accessOrder = [];
    this.accessCounts = new Map();
    this.currentSize = 0;
    this.maxSizeBytes = config.maxSize * 1024 * 1024;
  }
  
  async get(key: string): Promise<CacheEntry | null> {
    const entry = this.cache.get(key);
    
    if (!entry) {
      return null;
    }
    
    // Check if expired
    if (this.isExpired(entry)) {
      await this.delete(key);
      return null;
    }
    
    // Update access tracking
    this.updateAccessTracking(key);
    
    return entry;
  }
  
  async set(key: string, entry: CacheEntry, ttl?: number): Promise<void> {
    // Calculate entry size
    const entrySize = this.getEntrySize(entry);
    
    // Check if we need to evict entries
    while (this.currentSize + entrySize > this.maxSizeBytes && this.cache.size > 0) {
      await this.evictOne();
    }
    
    // Remove old entry if exists
    if (this.cache.has(key)) {
      const oldEntry = this.cache.get(key)!;
      this.currentSize -= this.getEntrySize(oldEntry);
    }
    
    // Set TTL if provided
    if (ttl !== undefined) {
      entry.ttl = ttl;
    }
    
    // Store entry
    this.cache.set(key, entry);
    this.currentSize += entrySize;
    
    // Update access tracking
    this.updateAccessTracking(key);
  }
  
  async delete(key: string): Promise<boolean> {
    const entry = this.cache.get(key);
    if (!entry) {
      return false;
    }
    
    this.cache.delete(key);
    this.currentSize -= this.getEntrySize(entry);
    
    // Clean up access tracking
    const index = this.accessOrder.indexOf(key);
    if (index > -1) {
      this.accessOrder.splice(index, 1);
    }
    this.accessCounts.delete(key);
    
    return true;
  }
  
  async exists(key: string): Promise<boolean> {
    const entry = this.cache.get(key);
    if (!entry) {
      return false;
    }
    
    // Check if expired
    if (this.isExpired(entry)) {
      await this.delete(key);
      return false;
    }
    
    return true;
  }
  
  async keys(pattern: string): Promise<string[]> {
    const regex = this.patternToRegex(pattern);
    const validKeys: string[] = [];
    
    for (const [key, entry] of this.cache.entries()) {
      if (regex.test(key) && !this.isExpired(entry)) {
        validKeys.push(key);
      }
    }
    
    return validKeys;
  }
  
  async clear(): Promise<void> {
    this.cache.clear();
    this.accessOrder = [];
    this.accessCounts.clear();
    this.currentSize = 0;
  }
  
  async getStats(): Promise<{
    size: number;
    entries: number;
    oldest: Date | null;
    newest: Date | null;
  }> {
    let oldest: Date | null = null;
    let newest: Date | null = null;
    
    for (const entry of this.cache.values()) {
      if (!this.isExpired(entry)) {
        if (!oldest || entry.createdAt < oldest) {
          oldest = entry.createdAt;
        }
        if (!newest || entry.createdAt > newest) {
          newest = entry.createdAt;
        }
      }
    }
    
    return {
      size: this.currentSize,
      entries: this.cache.size,
      oldest,
      newest
    };
  }
  
  // Private methods
  
  private updateAccessTracking(key: string): void {
    switch (this.config.evictionPolicy) {
      case 'lru':
        // Remove from current position
        const index = this.accessOrder.indexOf(key);
        if (index > -1) {
          this.accessOrder.splice(index, 1);
        }
        // Add to end (most recently used)
        this.accessOrder.push(key);
        break;
        
      case 'lfu':
        // Increment access count
        const count = this.accessCounts.get(key) || 0;
        this.accessCounts.set(key, count + 1);
        break;
        
      case 'fifo':
        // Only track on first insert
        if (!this.accessOrder.includes(key)) {
          this.accessOrder.push(key);
        }
        break;
    }
  }
  
  private async evictOne(): Promise<void> {
    let keyToEvict: string | undefined;
    
    switch (this.config.evictionPolicy) {
      case 'lru':
        // Evict least recently used (first in accessOrder)
        keyToEvict = this.accessOrder.shift();
        break;
        
      case 'lfu':
        // Evict least frequently used
        let minCount = Infinity;
        for (const [key, count] of this.accessCounts.entries()) {
          if (count < minCount) {
            minCount = count;
            keyToEvict = key;
          }
        }
        break;
        
      case 'fifo':
        // Evict oldest (first in accessOrder)
        keyToEvict = this.accessOrder.shift();
        break;
    }
    
    if (keyToEvict) {
      await this.delete(keyToEvict);
    }
  }
  
  private patternToRegex(pattern: string): RegExp {
    // Convert simple wildcard pattern to regex
    const escaped = pattern
      .replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      .replace(/\\\*/g, '.*')
      .replace(/\\\?/g, '.');
    
    return new RegExp(`^${escaped}$`);
  }
}