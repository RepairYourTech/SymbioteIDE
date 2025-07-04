/**
 * OpenAI Cache Adapter
 * 
 * Provider-specific adapter for OpenAI response caching
 */

import {
  ProviderCacheAdapter,
  CacheRequest,
  CachedContent,
  ProviderCacheConfig,
  OpenAICacheOptions
} from '../types';
import * as crypto from 'crypto';

export class OpenAICacheAdapter implements ProviderCacheAdapter {
  provider = 'openai';
  private config: ProviderCacheConfig;
  
  constructor(config: ProviderCacheConfig) {
    this.config = config;
  }
  
  /**
   * Check if caching is supported for this request
   */
  isSupported(request: CacheRequest): boolean {
    // OpenAI doesn't have native context caching, but we can cache responses
    const supportedModels = [
      'gpt-4',
      'gpt-4-turbo',
      'gpt-4o',
      'gpt-4o-mini',
      'gpt-3.5-turbo'
    ];
    
    const modelBase = request.model.split('-202')[0]; // Remove date suffix
    if (!supportedModels.some(m => modelBase.includes(m))) {
      return false;
    }
    
    // Check if response caching is enabled
    const options = request.metadata?.cacheOptions as OpenAICacheOptions;
    if (!options?.responseCache && !options?.functionResultCache) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Generate cache key for the request
   */
  generateKey(request: CacheRequest): string {
    const options = request.metadata?.cacheOptions as OpenAICacheOptions;
    const keyData: any = {
      model: request.model,
      messages: request.messages
    };
    
    // Include specific fields if configured
    if (options?.cacheableFields) {
      options.cacheableFields.forEach(field => {
        if (request[field as keyof CacheRequest]) {
          keyData[field] = request[field as keyof CacheRequest];
        }
      });
    }
    
    // Include temperature for deterministic responses
    if (request.temperature === 0) {
      keyData.temperature = 0;
      keyData.seed = request.metadata?.seed;
    }
    
    // Include functions if caching function results
    if (options?.functionResultCache && request.functions) {
      keyData.functions = request.functions;
    }
    
    const hash = crypto
      .createHash('sha256')
      .update(JSON.stringify(keyData))
      .digest('hex');
    
    return `openai:${request.model}:${hash}`;
  }
  
  /**
   * Prepare request with cache information
   */
  prepareCachedRequest(request: CacheRequest, cacheKey: string): any {
    const options = request.metadata?.cacheOptions as OpenAICacheOptions;
    
    const cachedRequest: any = {
      model: request.model,
      messages: request.messages,
      temperature: request.temperature,
      max_tokens: request.maxTokens,
      top_p: request.metadata?.topP,
      frequency_penalty: request.metadata?.frequencyPenalty,
      presence_penalty: request.metadata?.presencePenalty
    };
    
    // Add deterministic parameters for better caching
    if (request.temperature === 0) {
      cachedRequest.seed = request.metadata?.seed || 12345;
      cachedRequest.temperature = 0;
    }
    
    // Add functions/tools if present
    if (request.functions) {
      cachedRequest.functions = request.functions;
    }
    
    if (request.tools) {
      cachedRequest.tools = request.tools;
    }
    
    // Add metadata for tracking
    cachedRequest.user = cacheKey.substring(0, 16);
    
    return cachedRequest;
  }
  
  /**
   * Extract cacheable content from response
   */
  extractCacheable(response: any): CachedContent | null {
    if (!response || !response.choices?.[0]) {
      return null;
    }
    
    const choice = response.choices[0];
    
    // Only cache complete responses
    if (choice.finish_reason !== 'stop' && 
        choice.finish_reason !== 'function_call' &&
        choice.finish_reason !== 'tool_calls') {
      return null;
    }
    
    return {
      type: 'response',
      data: {
        message: choice.message,
        finish_reason: choice.finish_reason,
        model: response.model,
        usage: response.usage,
        system_fingerprint: response.system_fingerprint
      },
      tokenCount: response.usage?.completion_tokens || 0,
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
    const model = cached.data.model || 'gpt-4';
    
    // OpenAI pricing (as of 2024)
    const pricing: Record<string, { input: number; output: number }> = {
      'gpt-4o': { input: 0.005, output: 0.015 },
      'gpt-4o-mini': { input: 0.00015, output: 0.0006 },
      'gpt-4-turbo': { input: 0.01, output: 0.03 },
      'gpt-4': { input: 0.03, output: 0.06 },
      'gpt-3.5-turbo': { input: 0.0005, output: 0.0015 }
    };
    
    const modelBase = model.split('-202')[0];
    const price = Object.entries(pricing).find(([key]) => 
      modelBase.includes(key)
    )?.[1] || pricing['gpt-4'];
    
    // Calculate total cost saved (both input and output for cached responses)
    const inputCost = (usage.prompt_tokens / 1000) * price.input;
    const outputCost = (usage.completion_tokens / 1000) * price.output;
    
    return inputCost + outputCost;
  }
  
  // Private methods
  
  private generateResponseHash(response: any): string {
    const choice = response.choices?.[0];
    if (!choice) return '';
    
    const hashData = {
      message: choice.message,
      finish_reason: choice.finish_reason,
      model: response.model
    };
    
    return crypto
      .createHash('sha256')
      .update(JSON.stringify(hashData))
      .digest('hex')
      .substring(0, 16);
  }
}