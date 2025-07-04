/**
 * Cache Storage Interface
 * 
 * Abstract interface for cache storage implementations
 */

import { CacheEntry, CacheStorage } from '../types';

export abstract class BaseCacheStorage implements CacheStorage {
  protected config: any;
  
  constructor(config: any) {
    this.config = config;
  }
  
  abstract get(key: string): Promise<CacheEntry | null>;
  abstract set(key: string, entry: CacheEntry, ttl?: number): Promise<void>;
  abstract delete(key: string): Promise<boolean>;
  abstract exists(key: string): Promise<boolean>;
  abstract keys(pattern: string): Promise<string[]>;
  abstract clear(): Promise<void>;
  abstract getStats(): Promise<{
    size: number;
    entries: number;
    oldest: Date | null;
    newest: Date | null;
  }>;
  
  /**
   * Helper to serialize cache entries
   */
  protected serialize(entry: CacheEntry): string {
    return JSON.stringify(entry, (key, value) => {
      if (value instanceof Date) {
        return { __type: 'Date', value: value.toISOString() };
      }
      return value;
    });
  }
  
  /**
   * Helper to deserialize cache entries
   */
  protected deserialize(data: string): CacheEntry {
    return JSON.parse(data, (key, value) => {
      if (value && value.__type === 'Date') {
        return new Date(value.value);
      }
      return value;
    });
  }
  
  /**
   * Check if entry has expired
   */
  protected isExpired(entry: CacheEntry): boolean {
    if (!entry.ttl || entry.ttl <= 0) {
      return false;
    }
    
    const expiresAt = new Date(entry.createdAt.getTime() + entry.ttl * 1000);
    return new Date() > expiresAt;
  }
  
  /**
   * Calculate entry size in bytes
   */
  protected getEntrySize(entry: CacheEntry): number {
    return Buffer.byteLength(this.serialize(entry), 'utf8');
  }
}