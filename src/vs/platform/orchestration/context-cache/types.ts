/**
 * Context Cache Types
 * 
 * Type definitions for the context caching system
 */

export interface CacheConfig {
  providers: {
    anthropic?: ProviderCacheConfig;
    gemini?: ProviderCacheConfig;
    openai?: ProviderCacheConfig;
    [key: string]: ProviderCacheConfig | undefined;
  };
  semantic?: SemanticCacheConfig;
  storage: StorageConfig;
  monitoring?: MonitoringConfig;
}

export interface ProviderCacheConfig {
  enabled: boolean;
  ttl?: number; // Time to live in seconds
  maxSize?: number; // Max cache size in MB
  maxEntries?: number; // Max number of cache entries
  costThreshold?: number; // Min cost to cache (in dollars)
}

export interface SemanticCacheConfig {
  enabled: boolean;
  threshold: number; // Similarity threshold (0-1)
  embeddingModel?: string;
  maxDimensions?: number;
  indexType?: 'flat' | 'hnsw' | 'ivf';
}

export interface StorageConfig {
  type: 'redis' | 'memory' | 'hybrid';
  redis?: {
    host: string;
    port: number;
    password?: string;
    db?: number;
    keyPrefix?: string;
  };
  memory?: {
    maxSize: number; // MB
    evictionPolicy: 'lru' | 'lfu' | 'fifo';
  };
}

export interface MonitoringConfig {
  enabled: boolean;
  metricsInterval?: number; // seconds
  alertThresholds?: {
    hitRate?: number;
    evictionRate?: number;
    errorRate?: number;
  };
}

export interface CacheEntry {
  key: string;
  provider: string;
  model: string;
  content: CachedContent;
  metadata: CacheMetadata;
  ttl: number;
  createdAt: Date;
  lastAccessedAt: Date;
  accessCount: number;
}

export interface CachedContent {
  type: 'context' | 'response' | 'embedding' | 'function';
  data: any;
  tokenCount?: number;
  hash?: string;
}

export interface CacheMetadata {
  cost?: number;
  latency?: number;
  quality?: number;
  tags?: string[];
  version?: string;
}

export interface CacheKey {
  provider: string;
  model: string;
  hash: string;
  semantic?: boolean;
}

export interface CacheRequest {
  provider: string;
  model: string;
  messages?: any[];
  systemPrompt?: string;
  functions?: any[];
  tools?: any[];
  temperature?: number;
  maxTokens?: number;
  metadata?: Record<string, any>;
}

export interface CacheResponse {
  hit: boolean;
  key?: string;
  content?: any;
  metadata?: CacheMetadata;
  similarity?: number;
}

export interface CacheStats {
  provider: string;
  hits: number;
  misses: number;
  evictions: number;
  errors: number;
  totalRequests: number;
  hitRate: number;
  avgLatency: number;
  costSaved: number;
  storageUsed: number;
  lastReset: Date;
}

export interface ProviderCacheAdapter {
  provider: string;
  
  /**
   * Check if caching is supported for this request
   */
  isSupported(request: CacheRequest): boolean;
  
  /**
   * Generate cache key for the request
   */
  generateKey(request: CacheRequest): string;
  
  /**
   * Prepare request with cache information
   */
  prepareCachedRequest(request: CacheRequest, cacheKey: string): any;
  
  /**
   * Extract cacheable content from response
   */
  extractCacheable(response: any): CachedContent | null;
  
  /**
   * Calculate cost saved by cache hit
   */
  calculateCostSaving(cached: CachedContent): number;
}

export interface SemanticCache {
  /**
   * Generate embedding for content
   */
  generateEmbedding(content: string): Promise<number[]>;
  
  /**
   * Find similar cached entries
   */
  findSimilar(
    embedding: number[], 
    threshold: number,
    limit?: number
  ): Promise<Array<{
    entry: CacheEntry;
    similarity: number;
  }>>;
  
  /**
   * Index new cache entry
   */
  indexEntry(entry: CacheEntry, embedding: number[]): Promise<void>;
  
  /**
   * Remove entry from index
   */
  removeFromIndex(key: string): Promise<void>;
}

export interface CacheStorage {
  /**
   * Get cache entry by key
   */
  get(key: string): Promise<CacheEntry | null>;
  
  /**
   * Set cache entry
   */
  set(key: string, entry: CacheEntry, ttl?: number): Promise<void>;
  
  /**
   * Delete cache entry
   */
  delete(key: string): Promise<boolean>;
  
  /**
   * Check if key exists
   */
  exists(key: string): Promise<boolean>;
  
  /**
   * Get all keys matching pattern
   */
  keys(pattern: string): Promise<string[]>;
  
  /**
   * Clear all cache entries
   */
  clear(): Promise<void>;
  
  /**
   * Get storage statistics
   */
  getStats(): Promise<{
    size: number;
    entries: number;
    oldest: Date | null;
    newest: Date | null;
  }>;
}

export interface CacheEvents {
  hit: (key: string, provider: string) => void;
  miss: (key: string, provider: string) => void;
  set: (key: string, provider: string, size: number) => void;
  evict: (key: string, reason: string) => void;
  error: (error: Error, operation: string) => void;
}

// Provider-specific types

export interface AnthropicCacheOptions {
  cacheControl?: {
    type: 'ephemeral';
    ttl?: number;
  };
  includeSystemPrompt?: boolean;
  includeTools?: boolean;
}

export interface GeminiCacheOptions {
  cachedContentName?: string;
  model?: string;
  expireTime?: string; // ISO timestamp
  displayName?: string;
}

export interface OpenAICacheOptions {
  cacheableFields?: string[];
  responseCache?: boolean;
  functionResultCache?: boolean;
}