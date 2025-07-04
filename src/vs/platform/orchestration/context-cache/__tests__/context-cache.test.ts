/**
 * Tests for Context Cache System
 */

import { ContextCacheManager } from '../context-cache-manager';
import { MemoryCacheStorage } from '../storage/memory-storage';
import { AnthropicCacheAdapter } from '../adapters/anthropic-adapter';
import { defaultCacheConfig, CacheRequest } from '../types';

describe('ContextCacheManager', () => {
  let cacheManager: ContextCacheManager;
  
  beforeEach(() => {
    cacheManager = new ContextCacheManager({
      ...defaultCacheConfig,
      storage: {
        type: 'memory',
        memory: {
          maxSize: 10, // 10MB for testing
          evictionPolicy: 'lru'
        }
      }
    });
  });
  
  afterEach(async () => {
    await cacheManager.dispose();
  });
  
  describe('Basic Caching', () => {
    it('should cache a context request', async () => {
      const request: CacheRequest = {
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [
          { role: 'user', content: 'Hello, world!' }
        ],
        systemPrompt: 'You are a helpful assistant'
      };
      
      const key = await cacheManager.cacheContext(request);
      expect(key).toBeTruthy();
      expect(key).toContain('anthropic:claude-3-sonnet:');
    });
    
    it('should return same key for identical requests', async () => {
      const request: CacheRequest = {
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [
          { role: 'user', content: 'Test message' }
        ]
      };
      
      const key1 = await cacheManager.cacheContext(request);
      const key2 = await cacheManager.cacheContext(request);
      
      expect(key1).toBe(key2);
    });
    
    it('should execute with cache hit', async () => {
      const request: CacheRequest = {
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [
          { role: 'user', content: 'Test query' }
        ]
      };
      
      let executionCount = 0;
      const executor = jest.fn(async () => {
        executionCount++;
        return { response: 'Test response' };
      });
      
      // First execution - cache miss
      const result1 = await cacheManager.executeWithCache(request, executor);
      expect(result1.hit).toBe(false);
      expect(executionCount).toBe(1);
      
      // Second execution - cache hit
      const result2 = await cacheManager.executeWithCache(request, executor);
      expect(result2.hit).toBe(true);
      expect(executionCount).toBe(1); // Should not execute again
    });
  });
  
  describe('Provider Adapters', () => {
    it('should check if caching is supported', () => {
      const adapter = new AnthropicCacheAdapter({ enabled: true });
      
      const supported = adapter.isSupported({
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [{ role: 'user', content: 'Test' }]
      });
      
      expect(supported).toBe(true);
      
      const notSupported = adapter.isSupported({
        provider: 'anthropic',
        model: 'unsupported-model',
        messages: []
      });
      
      expect(notSupported).toBe(false);
    });
    
    it('should calculate cost savings', () => {
      const adapter = new AnthropicCacheAdapter({ enabled: true });
      
      const savings = adapter.calculateCostSaving({
        type: 'response',
        data: {
          usage: {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_input_tokens: 800
          },
          model: 'claude-3-sonnet'
        }
      });
      
      expect(savings).toBeGreaterThan(0);
    });
  });
  
  describe('Cache Invalidation', () => {
    it('should invalidate cache entries by pattern', async () => {
      // Add some entries
      await cacheManager.cacheContext({
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [{ role: 'user', content: 'Test 1' }]
      });
      
      await cacheManager.cacheContext({
        provider: 'openai',
        model: 'gpt-4',
        messages: [{ role: 'user', content: 'Test 2' }]
      });
      
      // Invalidate anthropic entries
      const count = await cacheManager.invalidate('anthropic:*');
      expect(count).toBe(1);
      
      // Check stats
      const stats = cacheManager.getStats('anthropic');
      expect(stats).toBeTruthy();
    });
    
    it('should clear all cache entries', async () => {
      // Add entries
      await cacheManager.cacheContext({
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [{ role: 'user', content: 'Test' }]
      });
      
      // Clear all
      const count = await cacheManager.invalidate();
      expect(count).toBe(-1); // Indicates all cleared
    });
  });
  
  describe('Statistics', () => {
    it('should track hit rate', async () => {
      const request: CacheRequest = {
        provider: 'anthropic',
        model: 'claude-3-sonnet',
        messages: [{ role: 'user', content: 'Test' }]
      };
      
      const executor = jest.fn(async () => ({ response: 'Test' }));
      
      // Execute multiple times
      await cacheManager.executeWithCache(request, executor); // miss
      await cacheManager.executeWithCache(request, executor); // hit
      await cacheManager.executeWithCache(request, executor); // hit
      
      const stats = cacheManager.getStats('anthropic');
      expect(stats.totalRequests).toBe(3);
      expect(stats.hits).toBe(2);
      expect(stats.misses).toBe(1);
      expect(stats.hitRate).toBeCloseTo(0.67, 2);
    });
  });
});

describe('MemoryCacheStorage', () => {
  let storage: MemoryCacheStorage;
  
  beforeEach(() => {
    storage = new MemoryCacheStorage({
      maxSize: 1, // 1MB
      evictionPolicy: 'lru'
    });
  });
  
  it('should store and retrieve entries', async () => {
    const entry = {
      key: 'test-key',
      provider: 'test',
      model: 'test-model',
      content: { type: 'context' as const, data: 'test' },
      metadata: {},
      ttl: 300,
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0
    };
    
    await storage.set('test-key', entry);
    const retrieved = await storage.get('test-key');
    
    expect(retrieved).toEqual(entry);
  });
  
  it('should evict entries when size limit reached', async () => {
    // Fill cache with large entries
    for (let i = 0; i < 10; i++) {
      const largeData = 'x'.repeat(100000); // ~100KB
      const entry = {
        key: `key-${i}`,
        provider: 'test',
        model: 'test',
        content: { 
          type: 'context' as const, 
          data: largeData 
        },
        metadata: {},
        ttl: 300,
        createdAt: new Date(),
        lastAccessedAt: new Date(),
        accessCount: 0
      };
      
      await storage.set(`key-${i}`, entry);
    }
    
    // Check that early entries were evicted
    const firstEntry = await storage.get('key-0');
    expect(firstEntry).toBeNull();
    
    // Recent entries should still exist
    const lastEntry = await storage.get('key-9');
    expect(lastEntry).toBeTruthy();
  });
  
  it('should handle TTL expiration', async () => {
    const entry = {
      key: 'expiring-key',
      provider: 'test',
      model: 'test',
      content: { type: 'context' as const, data: 'test' },
      metadata: {},
      ttl: 0.1, // 0.1 second
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0
    };
    
    await storage.set('expiring-key', entry);
    
    // Wait for expiration
    await new Promise(resolve => setTimeout(resolve, 200));
    
    const retrieved = await storage.get('expiring-key');
    expect(retrieved).toBeNull();
  });
});