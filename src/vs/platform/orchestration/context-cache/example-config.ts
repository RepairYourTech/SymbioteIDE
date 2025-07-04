/**
 * Example Configuration for Context Caching System
 * 
 * Shows how to configure and use the context caching with the orchestration engine
 */

import { OrchestrationEngine } from '../orchestration-engine';
import { CacheConfig } from './types';

// Example 1: Basic context caching configuration
const basicConfig = {
  contextCacheConfig: {
    providers: {
      anthropic: {
        enabled: true,
        ttl: 300, // 5 minutes
        costThreshold: 0.01 // Only cache if cost > $0.01
      },
      gemini: {
        enabled: true,
        ttl: 3600 // 1 hour (Gemini default)
      },
      openai: {
        enabled: true,
        ttl: 600 // 10 minutes
      }
    },
    storage: {
      type: 'memory' as const,
      memory: {
        maxSize: 100, // 100MB
        evictionPolicy: 'lru' as const
      }
    }
  }
};

// Example 2: Advanced configuration with Redis and semantic caching
const advancedConfig = {
  contextCacheConfig: {
    providers: {
      anthropic: {
        enabled: true,
        ttl: 300,
        maxEntries: 1000,
        costThreshold: 0.005
      },
      gemini: {
        enabled: true,
        ttl: 3600,
        maxSize: 200 // MB
      }
    },
    semantic: {
      enabled: true,
      threshold: 0.95, // 95% similarity threshold
      embeddingModel: 'text-embedding-ada-002'
    },
    storage: {
      type: 'redis' as const,
      redis: {
        host: 'localhost',
        port: 6379,
        keyPrefix: 'context_cache:'
      }
    },
    monitoring: {
      enabled: true,
      metricsInterval: 60, // seconds
      alertThresholds: {
        hitRate: 0.3, // Alert if hit rate < 30%
        evictionRate: 0.5, // Alert if eviction rate > 50%
        errorRate: 0.05 // Alert if error rate > 5%
      }
    }
  } as CacheConfig
};

// Example 3: Using the orchestration engine with context caching
async function exampleUsage() {
  // Initialize orchestration engine with context caching
  const engine = new OrchestrationEngine(advancedConfig);
  
  // Execute a task - will automatically use context caching
  const result = await engine.executeTask({
    id: 'task-1',
    type: 'completion',
    prompt: 'Explain quantum computing',
    messages: [
      { role: 'user', content: 'What is quantum computing?' }
    ],
    requirements: {
      maxTokens: 500
    }
  });
  
  console.log('Result:', result);
  console.log('Cache hit:', result.metadata?.cacheHit);
  
  // Get cache statistics
  const stats = engine.getContextCacheStats();
  console.log('Cache stats:', stats);
  
  // Warm cache with common contexts
  const commonContexts = [
    {
      provider: 'anthropic',
      model: 'claude-3-sonnet',
      messages: [
        { role: 'system', content: 'You are a helpful assistant.' },
        { role: 'user', content: 'Hello!' }
      ]
    },
    {
      provider: 'openai',
      model: 'gpt-4o',
      systemPrompt: 'You are an expert programmer.',
      messages: []
    }
  ];
  
  const warmed = await engine.warmContextCache(commonContexts);
  console.log(`Warmed ${warmed} cache entries`);
  
  // Generate cache report
  const report = engine.getContextCacheReport();
  console.log('Cache report:\n', report);
  
  // Listen to cache events
  engine.on('cache:hit', ({ key, provider }) => {
    console.log(`Cache hit for ${provider}: ${key}`);
  });
  
  engine.on('cache:alert', (alert) => {
    console.warn('Cache alert:', alert);
  });
  
  // Shutdown when done
  await engine.shutdown();
}

// Example 4: Provider-specific cache options
async function providerSpecificExample() {
  const engine = new OrchestrationEngine(basicConfig);
  
  // Anthropic with cache control
  await engine.executeTask({
    id: 'anthropic-task',
    type: 'completion',
    prompt: 'Write a story',
    requirements: {
      provider: 'anthropic'
    },
    metadata: {
      cacheOptions: {
        cacheControl: {
          type: 'ephemeral',
          ttl: 300
        },
        includeSystemPrompt: true,
        includeTools: true
      }
    }
  });
  
  // Gemini with named cache
  await engine.executeTask({
    id: 'gemini-task',
    type: 'completion',
    prompt: 'Analyze this code',
    requirements: {
      provider: 'gemini'
    },
    metadata: {
      cacheOptions: {
        cachedContentName: 'code-analysis-context',
        displayName: 'Code Analysis Cache',
        expireTime: new Date(Date.now() + 3600000).toISOString()
      }
    }
  });
  
  // OpenAI with response caching
  await engine.executeTask({
    id: 'openai-task',
    type: 'completion',
    prompt: 'Generate test cases',
    parameters: {
      temperature: 0, // Deterministic for better caching
      seed: 12345
    },
    requirements: {
      provider: 'openai'
    },
    metadata: {
      cacheOptions: {
        responseCache: true,
        functionResultCache: true,
        cacheableFields: ['messages', 'functions', 'temperature']
      }
    }
  });
}

// Example 5: Cache management
async function cacheManagementExample() {
  const engine = new OrchestrationEngine(advancedConfig);
  
  // Invalidate specific cache entries
  const invalidated = await engine.invalidateContextCache('anthropic:*');
  console.log(`Invalidated ${invalidated} cache entries`);
  
  // Clear all cache
  await engine.invalidateContextCache();
  
  // Monitor cache performance
  setInterval(() => {
    const stats = engine.getContextCacheStats();
    
    // Check hit rate for each provider
    if (stats instanceof Map) {
      stats.forEach((providerStats, provider) => {
        console.log(`${provider} hit rate: ${(providerStats.hitRate * 100).toFixed(2)}%`);
        console.log(`${provider} cost saved: $${providerStats.costSaved.toFixed(4)}`);
      });
    }
  }, 60000); // Every minute
}

export {
  basicConfig,
  advancedConfig,
  exampleUsage,
  providerSpecificExample,
  cacheManagementExample
};