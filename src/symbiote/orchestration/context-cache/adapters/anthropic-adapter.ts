/**
 * Anthropic Cache Adapter
 * 
 * Provider-specific adapter for Anthropic Claude context caching
 */

import {
  ProviderCacheAdapter,
  CacheRequest,
  CachedContent,
  ProviderCacheConfig,
  AnthropicCacheOptions
} from '../types';
import * as crypto from 'crypto';

export class AnthropicCacheAdapter implements ProviderCacheAdapter {
  provider = 'anthropic';
  private config: ProviderCacheConfig;
  
  constructor(config: ProviderCacheConfig) {
    this.config = config;
  }
  
  /**
   * Check if caching is supported for this request
   */
  isSupported(request: CacheRequest): boolean {
    // Anthropic supports caching for Claude models
    const supportedModels = [
      'claude-3-opus',
      'claude-3-sonnet',
      'claude-3-haiku',
      'claude-2.1',
      'claude-2.0'
    ];
    
    const modelBase = request.model.split('-20')[0]; // Remove date suffix
    if (!supportedModels.some(m => modelBase.includes(m))) {
      return false;
    }
    
    // Check if request has sufficient content to cache
    if (!request.messages || request.messages.length === 0) {
      return false;
    }
    
    // Check cost threshold if configured
    if (this.config.costThreshold) {
      const estimatedCost = this.estimateCost(request);
      if (estimatedCost < this.config.costThreshold) {
        return false;
      }
    }
    
    return true;
  }
  
  /**
   * Generate cache key for the request
   */
  generateKey(request: CacheRequest): string {
    const keyData: any = {
      model: request.model,
      messages: request.messages,
      temperature: request.temperature || 0
    };
    
    // Include system prompt if caching is enabled for it
    const options = request.metadata?.cacheOptions as AnthropicCacheOptions;
    if (options?.includeSystemPrompt && request.systemPrompt) {
      keyData.systemPrompt = request.systemPrompt;
    }
    
    // Include tools if caching is enabled for them
    if (options?.includeTools && request.tools) {
      keyData.tools = request.tools;
    }
    
    const hash = crypto
      .createHash('sha256')
      .update(JSON.stringify(keyData))
      .digest('hex');
    
    return `anthropic:${request.model}:${hash}`;
  }
  
  /**
   * Prepare request with cache information
   */
  prepareCachedRequest(request: CacheRequest, cacheKey: string): any {
    const options = request.metadata?.cacheOptions as AnthropicCacheOptions;
    
    // Anthropic's beta caching API format
    const cachedRequest: any = {
      model: request.model,
      messages: request.messages,
      max_tokens: request.maxTokens,
      temperature: request.temperature,
      system: request.systemPrompt
    };
    
    // Add cache control headers
    if (options?.cacheControl) {
      cachedRequest.cache_control = {
        type: options.cacheControl.type || 'ephemeral',
        ttl: options.cacheControl.ttl || this.config.ttl || 300
      };
    }
    
    // Add cache key as metadata
    cachedRequest.metadata = {
      ...request.metadata,
      cache_key: cacheKey,
      cached_at: new Date().toISOString()
    };
    
    // If we have tools, include them
    if (request.tools) {
      cachedRequest.tools = request.tools;
    }
    
    return cachedRequest;
  }
  
  /**
   * Extract cacheable content from response
   */
  extractCacheable(response: any): CachedContent | null {
    // Check if response has content
    if (!response || !response.content) {
      return null;
    }
    
    // Check if response indicates it was cached
    const wasCached = response.metadata?.cached || 
                     response.usage?.cache_creation_input_tokens > 0 ||
                     response.usage?.cache_read_input_tokens > 0;
    
    return {
      type: 'response',
      data: {
        content: response.content,
        model: response.model,
        stop_reason: response.stop_reason,
        usage: response.usage
      },
      tokenCount: response.usage?.output_tokens || 0,
      hash: this.generateResponseHash(response)
    };
  }
  
  /**
   * Calculate cost saved by cache hit
   */
  calculateCostSaving(cached: CachedContent): number {
    if (!cached.data?.usage) {
      return 0;
    }
    
    const usage = cached.data.usage;
    const model = cached.data.model || 'claude-3-sonnet';
    
    // Anthropic pricing (as of 2024)
    const pricing: Record<string, { input: number; output: number; cached: number }> = {
      'claude-3-opus': { input: 0.015, output: 0.075, cached: 0.0075 },
      'claude-3-sonnet': { input: 0.003, output: 0.015, cached: 0.0015 },
      'claude-3-haiku': { input: 0.00025, output: 0.00125, cached: 0.000125 }
    };
    
    const modelBase = model.split('-20')[0];
    const price = Object.entries(pricing).find(([key]) => 
      modelBase.includes(key)
    )?.[1] || pricing['claude-3-sonnet'];
    
    // Calculate savings
    const regularInputCost = (usage.input_tokens / 1000) * price.input;
    const cachedInputCost = (usage.cache_read_input_tokens / 1000) * price.cached;
    const outputCost = (usage.output_tokens / 1000) * price.output;
    
    const regularTotalCost = regularInputCost + outputCost;
    const cachedTotalCost = cachedInputCost + outputCost;
    
    return regularTotalCost - cachedTotalCost;
  }
  
  // Private methods
  
  private estimateCost(request: CacheRequest): number {
    // Estimate token count
    const content = JSON.stringify({
      messages: request.messages,
      systemPrompt: request.systemPrompt,
      tools: request.tools
    });
    
    const estimatedTokens = Math.ceil(content.length / 4);
    const outputTokens = request.maxTokens || 1000;
    
    // Use Sonnet pricing as default
    const inputCost = (estimatedTokens / 1000) * 0.003;
    const outputCost = (outputTokens / 1000) * 0.015;
    
    return inputCost + outputCost;
  }
  
  private generateResponseHash(response: any): string {
    const hashData = {
      content: response.content,
      model: response.model,
      stop_reason: response.stop_reason
    };
    
    return crypto
      .createHash('sha256')
      .update(JSON.stringify(hashData))
      .digest('hex')
      .substring(0, 16);
  }
}