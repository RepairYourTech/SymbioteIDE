/**
 * Context Cache System
 * 
 * Advanced caching system supporting provider-specific context caching APIs
 */

export * from './types';
export * from './context-cache-manager';
export * from './semantic-cache-layer';
export * from './cache-monitor';

// Storage implementations
export * from './storage/cache-storage';
export * from './storage/memory-storage';
export * from './storage/redis-storage';

// Provider adapters
export * from './adapters/anthropic-adapter';
export * from './adapters/gemini-adapter';
export * from './adapters/openai-adapter';

// Re-export main class for convenience
export { ContextCacheManager } from './context-cache-manager';
export { CacheMonitor } from './cache-monitor';

// Default configuration
export const defaultCacheConfig = {
  providers: {
    anthropic: {
      enabled: true,
      ttl: 300, // 5 minutes
      maxSize: 100, // MB
      maxEntries: 1000,
      costThreshold: 0.01 // $0.01
    },
    gemini: {
      enabled: true,
      ttl: 3600, // 1 hour
      maxSize: 200, // MB
      maxEntries: 500,
      costThreshold: 0.005 // $0.005
    },
    openai: {
      enabled: true,
      ttl: 600, // 10 minutes
      maxSize: 150, // MB
      maxEntries: 1000,
      costThreshold: 0.01 // $0.01
    }
  },
  semantic: {
    enabled: true,
    threshold: 0.95,
    embeddingModel: 'text-embedding-ada-002',
    maxDimensions: 1536,
    indexType: 'flat' as const
  },
  storage: {
    type: 'memory' as const,
    memory: {
      maxSize: 500, // MB total
      evictionPolicy: 'lru' as const
    }
  },
  monitoring: {
    enabled: true,
    metricsInterval: 60, // seconds
    alertThresholds: {
      hitRate: 0.3, // Alert if hit rate drops below 30%
      evictionRate: 0.5, // Alert if eviction rate exceeds 50%
      errorRate: 0.05 // Alert if error rate exceeds 5%
    }
  }
};