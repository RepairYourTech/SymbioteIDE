# Context Caching System

Advanced caching system supporting provider-specific context caching APIs for cost optimization and performance improvement.

## Features

- **Provider-Specific Caching**
  - Anthropic Claude context caching (beta)
  - Google Gemini context caching (1-hour persistence)
  - OpenAI partial response caching
  
- **Semantic Caching**
  - Embedding-based similarity matching
  - Cross-provider cache sharing
  - Intelligent cache key generation
  
- **Cache Management**
  - TTL-based expiration
  - LRU eviction policies
  - Manual invalidation support
  - Cache warming strategies
  
- **Performance & Cost Optimization**
  - Hit rate tracking
  - Cost reduction metrics
  - Latency improvements
  - Memory usage monitoring

## Architecture

```
Context Cache System
├── CacheManager (orchestrator)
├── Provider Adapters
│   ├── AnthropicCacheAdapter
│   ├── GeminiCacheAdapter
│   └── OpenAICacheAdapter
├── Semantic Cache Layer
│   ├── EmbeddingGenerator
│   └── SimilarityMatcher
├── Storage Backend
│   ├── Redis (distributed)
│   └── In-Memory (local)
└── Monitoring & Analytics
```

## Usage

```typescript
// Initialize cache manager
const cacheManager = new ContextCacheManager({
  providers: {
    anthropic: { enabled: true, ttl: 300 },
    gemini: { enabled: true, ttl: 3600 },
    openai: { enabled: true, ttl: 600 }
  },
  semantic: {
    enabled: true,
    threshold: 0.95
  },
  storage: {
    type: 'redis',
    config: { /* redis config */ }
  }
});

// Cache a context
const cacheKey = await cacheManager.cacheContext({
  provider: 'anthropic',
  model: 'claude-3-sonnet',
  messages: [...],
  systemPrompt: '...'
});

// Use cached context
const response = await cacheManager.executeWithCache({
  provider: 'anthropic',
  model: 'claude-3-sonnet',
  messages: [...],
  cacheKey: cacheKey
});
```

## Provider-Specific Features

### Anthropic Claude
- Beta API support for context caching
- 5-minute cache TTL
- Significant cost reduction for repeated contexts
- Automatic cache key management

### Google Gemini
- 1-hour context persistence
- Cached content pricing model
- Cross-request context sharing
- Automatic expiration handling

### OpenAI
- Partial response caching
- Function call result caching
- Embedding cache integration
- Token-level caching optimization