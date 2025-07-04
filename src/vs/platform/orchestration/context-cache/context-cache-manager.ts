/**
 * Context Cache Manager
 * 
 * Main orchestrator for the context caching system
 */

import { EventEmitter } from 'events';
import * as crypto from 'crypto';
import {
  CacheConfig,
  CacheEntry,
  CacheRequest,
  CacheResponse,
  CacheStats,
  CachedContent,
  CacheMetadata,
  CacheEvents
} from './types';
import { CacheStorage } from './storage/cache-storage';
import { RedisCacheStorage } from './storage/redis-storage';
import { MemoryCacheStorage } from './storage/memory-storage';
import { SemanticCacheLayer } from './semantic-cache-layer';
import { AnthropicCacheAdapter } from './adapters/anthropic-adapter';
import { GeminiCacheAdapter } from './adapters/gemini-adapter';
import { OpenAICacheAdapter } from './adapters/openai-adapter';
import { CacheMonitor } from './cache-monitor';

export class ContextCacheManager extends EventEmitter {
  private config: CacheConfig;
  private storage: CacheStorage;
  private semanticCache?: SemanticCacheLayer;
  private adapters: Map<string, any>;
  private monitor: CacheMonitor;
  private stats: Map<string, CacheStats>;
  
  constructor(config: CacheConfig) {
    super();
    this.config = config;
    this.adapters = new Map();
    this.stats = new Map();
    
    // Initialize storage
    this.storage = this.initializeStorage();
    
    // Initialize semantic cache if enabled
    if (config.semantic?.enabled) {
      this.semanticCache = new SemanticCacheLayer(config.semantic, this.storage);
    }
    
    // Initialize provider adapters
    this.initializeAdapters();
    
    // Initialize monitoring
    this.monitor = new CacheMonitor(this, config.monitoring);
    
    // Set up event handlers
    this.setupEventHandlers();
  }
  
  /**
   * Cache a context for future use
   */
  async cacheContext(request: CacheRequest): Promise<string | null> {
    try {
      const adapter = this.adapters.get(request.provider);
      if (!adapter || !adapter.isSupported(request)) {
        return null;
      }
      
      // Generate cache key
      const key = this.generateCacheKey(request);
      
      // Check if already cached
      const existing = await this.storage.get(key);
      if (existing) {
        existing.lastAccessedAt = new Date();
        existing.accessCount++;
        await this.storage.set(key, existing);
        this.emit('hit', key, request.provider);
        return key;
      }
      
      // Create cache entry
      const entry: CacheEntry = {
        key,
        provider: request.provider,
        model: request.model,
        content: {
          type: 'context',
          data: {
            messages: request.messages,
            systemPrompt: request.systemPrompt,
            functions: request.functions,
            tools: request.tools
          },
          tokenCount: this.estimateTokenCount(request)
        },
        metadata: {
          tags: request.metadata?.tags || [],
          version: '1.0'
        },
        ttl: this.getProviderTTL(request.provider),
        createdAt: new Date(),
        lastAccessedAt: new Date(),
        accessCount: 0
      };
      
      // Store in cache
      await this.storage.set(key, entry, entry.ttl);
      
      // Index in semantic cache if enabled
      if (this.semanticCache && this.config.semantic?.enabled) {
        const content = this.extractTextContent(request);
        const embedding = await this.semanticCache.generateEmbedding(content);
        await this.semanticCache.indexEntry(entry, embedding);
      }
      
      this.emit('set', key, request.provider, entry.content.tokenCount || 0);
      this.updateStats(request.provider, 'miss');
      
      return key;
      
    } catch (error) {
      this.emit('error', error as Error, 'cacheContext');
      return null;
    }
  }
  
  /**
   * Execute request with cache support
   */
  async executeWithCache(
    request: CacheRequest, 
    executor: (req: any) => Promise<any>
  ): Promise<CacheResponse> {
    try {
      const adapter = this.adapters.get(request.provider);
      if (!adapter || !adapter.isSupported(request)) {
        // Execute without cache
        const response = await executor(request);
        return {
          hit: false,
          content: response
        };
      }
      
      // Try to find cached content
      let cacheEntry: CacheEntry | null = null;
      let similarity = 1.0;
      
      // First try exact match
      const key = this.generateCacheKey(request);
      cacheEntry = await this.storage.get(key);
      
      // If no exact match and semantic cache is enabled, try similarity search
      if (!cacheEntry && this.semanticCache && this.config.semantic?.enabled) {
        const content = this.extractTextContent(request);
        const embedding = await this.semanticCache.generateEmbedding(content);
        const similar = await this.semanticCache.findSimilar(
          embedding,
          this.config.semantic.threshold
        );
        
        if (similar.length > 0) {
          cacheEntry = similar[0].entry;
          similarity = similar[0].similarity;
        }
      }
      
      if (cacheEntry) {
        // Cache hit
        cacheEntry.lastAccessedAt = new Date();
        cacheEntry.accessCount++;
        await this.storage.set(cacheEntry.key, cacheEntry);
        
        this.emit('hit', cacheEntry.key, request.provider);
        this.updateStats(request.provider, 'hit');
        
        // Calculate cost saved
        const costSaved = adapter.calculateCostSaving(cacheEntry.content);
        this.updateCostSaving(request.provider, costSaved);
        
        // Prepare cached request
        const cachedRequest = adapter.prepareCachedRequest(request, cacheEntry.key);
        const response = await executor(cachedRequest);
        
        return {
          hit: true,
          key: cacheEntry.key,
          content: response,
          metadata: cacheEntry.metadata,
          similarity
        };
      } else {
        // Cache miss - execute and cache result
        this.emit('miss', key, request.provider);
        this.updateStats(request.provider, 'miss');
        
        const response = await executor(request);
        
        // Try to cache the response
        const cacheable = adapter.extractCacheable(response);
        if (cacheable) {
          const responseEntry: CacheEntry = {
            key,
            provider: request.provider,
            model: request.model,
            content: cacheable,
            metadata: {
              latency: response.latency,
              cost: response.cost
            },
            ttl: this.getProviderTTL(request.provider),
            createdAt: new Date(),
            lastAccessedAt: new Date(),
            accessCount: 0
          };
          
          await this.storage.set(key, responseEntry, responseEntry.ttl);
          this.emit('set', key, request.provider, cacheable.tokenCount || 0);
        }
        
        return {
          hit: false,
          key,
          content: response
        };
      }
      
    } catch (error) {
      this.emit('error', error as Error, 'executeWithCache');
      throw error;
    }
  }
  
  /**
   * Invalidate cache entries
   */
  async invalidate(pattern?: string): Promise<number> {
    try {
      if (!pattern) {
        await this.storage.clear();
        if (this.semanticCache) {
          await this.semanticCache.clear();
        }
        return -1; // All entries cleared
      }
      
      const keys = await this.storage.keys(pattern);
      let count = 0;
      
      for (const key of keys) {
        const deleted = await this.storage.delete(key);
        if (deleted) {
          count++;
          if (this.semanticCache) {
            await this.semanticCache.removeFromIndex(key);
          }
          this.emit('evict', key, 'manual');
        }
      }
      
      return count;
      
    } catch (error) {
      this.emit('error', error as Error, 'invalidate');
      return 0;
    }
  }
  
  /**
   * Warm cache with predefined contexts
   */
  async warmCache(contexts: CacheRequest[]): Promise<number> {
    let warmed = 0;
    
    for (const context of contexts) {
      const key = await this.cacheContext(context);
      if (key) {
        warmed++;
      }
    }
    
    return warmed;
  }
  
  /**
   * Get cache statistics
   */
  getStats(provider?: string): CacheStats | Map<string, CacheStats> {
    if (provider) {
      return this.stats.get(provider) || this.createEmptyStats(provider);
    }
    return this.stats;
  }
  
  /**
   * Reset statistics
   */
  resetStats(provider?: string): void {
    if (provider) {
      this.stats.set(provider, this.createEmptyStats(provider));
    } else {
      this.stats.clear();
    }
  }
  
  // Private methods
  
  private initializeStorage(): CacheStorage {
    switch (this.config.storage.type) {
      case 'redis':
        return new RedisCacheStorage(this.config.storage.redis!);
      case 'memory':
        return new MemoryCacheStorage(this.config.storage.memory!);
      case 'hybrid':
        // TODO: Implement hybrid storage
        return new MemoryCacheStorage(this.config.storage.memory!);
      default:
        throw new Error(`Unknown storage type: ${this.config.storage.type}`);
    }
  }
  
  private initializeAdapters(): void {
    if (this.config.providers.anthropic?.enabled) {
      this.adapters.set('anthropic', new AnthropicCacheAdapter(
        this.config.providers.anthropic
      ));
    }
    
    if (this.config.providers.gemini?.enabled) {
      this.adapters.set('gemini', new GeminiCacheAdapter(
        this.config.providers.gemini
      ));
    }
    
    if (this.config.providers.openai?.enabled) {
      this.adapters.set('openai', new OpenAICacheAdapter(
        this.config.providers.openai
      ));
    }
  }
  
  private setupEventHandlers(): void {
    // Cast event names for type safety
    const events: CacheEvents = {
      hit: (key: string, provider: string) => {
        console.debug(`Cache hit: ${key} (${provider})`);
      },
      miss: (key: string, provider: string) => {
        console.debug(`Cache miss: ${key} (${provider})`);
      },
      set: (key: string, provider: string, size: number) => {
        console.debug(`Cache set: ${key} (${provider}, ${size} tokens)`);
      },
      evict: (key: string, reason: string) => {
        console.debug(`Cache evict: ${key} (${reason})`);
      },
      error: (error: Error, operation: string) => {
        console.error(`Cache error in ${operation}:`, error);
      }
    };
    
    // Register default handlers
    Object.entries(events).forEach(([event, handler]) => {
      this.on(event, handler);
    });
  }
  
  private generateCacheKey(request: CacheRequest): string {
    const keyData = {
      provider: request.provider,
      model: request.model,
      messages: request.messages,
      systemPrompt: request.systemPrompt,
      functions: request.functions,
      tools: request.tools,
      temperature: request.temperature,
      maxTokens: request.maxTokens
    };
    
    const hash = crypto
      .createHash('sha256')
      .update(JSON.stringify(keyData))
      .digest('hex');
    
    return `${request.provider}:${request.model}:${hash}`;
  }
  
  private extractTextContent(request: CacheRequest): string {
    const parts: string[] = [];
    
    if (request.systemPrompt) {
      parts.push(request.systemPrompt);
    }
    
    if (request.messages) {
      request.messages.forEach(msg => {
        if (typeof msg.content === 'string') {
          parts.push(msg.content);
        }
      });
    }
    
    if (request.functions) {
      parts.push(JSON.stringify(request.functions));
    }
    
    if (request.tools) {
      parts.push(JSON.stringify(request.tools));
    }
    
    return parts.join('\n');
  }
  
  private estimateTokenCount(request: CacheRequest): number {
    // Simple estimation - can be replaced with tiktoken
    const content = this.extractTextContent(request);
    return Math.ceil(content.length / 4);
  }
  
  private getProviderTTL(provider: string): number {
    const config = this.config.providers[provider];
    return config?.ttl || 300; // Default 5 minutes
  }
  
  private updateStats(provider: string, type: 'hit' | 'miss'): void {
    let stats = this.stats.get(provider);
    if (!stats) {
      stats = this.createEmptyStats(provider);
      this.stats.set(provider, stats);
    }
    
    stats.totalRequests++;
    if (type === 'hit') {
      stats.hits++;
    } else {
      stats.misses++;
    }
    
    stats.hitRate = stats.hits / stats.totalRequests;
  }
  
  private updateCostSaving(provider: string, amount: number): void {
    const stats = this.stats.get(provider);
    if (stats) {
      stats.costSaved += amount;
    }
  }
  
  private createEmptyStats(provider: string): CacheStats {
    return {
      provider,
      hits: 0,
      misses: 0,
      evictions: 0,
      errors: 0,
      totalRequests: 0,
      hitRate: 0,
      avgLatency: 0,
      costSaved: 0,
      storageUsed: 0,
      lastReset: new Date()
    };
  }
  
  /**
   * Cleanup resources
   */
  async dispose(): Promise<void> {
    this.removeAllListeners();
    this.monitor.stop();
    if (this.semanticCache) {
      await this.semanticCache.dispose();
    }
    // Storage cleanup handled by implementations
  }
}