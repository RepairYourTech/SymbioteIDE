/**
 * Gemini Cache Adapter
 * 
 * Provider-specific adapter for Google Gemini context caching
 */

import {
  ProviderCacheAdapter,
  CacheRequest,
  CachedContent,
  ProviderCacheConfig,
  GeminiCacheOptions
} from '../types';
import * as crypto from 'crypto';

export class GeminiCacheAdapter implements ProviderCacheAdapter {
  provider = 'gemini';
  private config: ProviderCacheConfig;
  
  constructor(config: ProviderCacheConfig) {
    this.config = config;
  }
  
  /**
   * Check if caching is supported for this request
   */
  isSupported(request: CacheRequest): boolean {
    // Gemini supports caching for 1.5 models
    const supportedModels = [
      'gemini-1.5-pro',
      'gemini-1.5-flash',
      'gemini-pro'
    ];
    
    if (!supportedModels.some(m => request.model.includes(m))) {
      return false;
    }
    
    // Check if request has sufficient content
    if (!request.messages || request.messages.length === 0) {
      return false;
    }
    
    // Gemini requires minimum token count for caching (32k tokens)
    const estimatedTokens = this.estimateTokenCount(request);
    if (estimatedTokens < 32768) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Generate cache key for the request
   */
  generateKey(request: CacheRequest): string {
    const keyData = {
      model: request.model,
      messages: request.messages,
      systemInstruction: request.systemPrompt,
      tools: request.functions
    };
    
    const hash = crypto
      .createHash('sha256')
      .update(JSON.stringify(keyData))
      .digest('hex');
    
    return `gemini:${request.model}:${hash}`;
  }
  
  /**
   * Prepare request with cache information
   */
  prepareCachedRequest(request: CacheRequest, cacheKey: string): any {
    const options = request.metadata?.cacheOptions as GeminiCacheOptions;
    
    // Gemini cached content format
    const cachedRequest: any = {
      model: options?.model || request.model,
      contents: this.convertMessagesToGeminiFormat(request.messages),
      systemInstruction: request.systemPrompt,
      generationConfig: {
        temperature: request.temperature,
        maxOutputTokens: request.maxTokens,
        topP: request.metadata?.topP,
        topK: request.metadata?.topK
      }
    };
    
    // Add cached content reference
    if (options?.cachedContentName) {
      cachedRequest.cachedContent = {
        name: options.cachedContentName,
        displayName: options.displayName || `Cache: ${cacheKey.substring(0, 8)}`
      };
    }
    
    // Add tools if present
    if (request.functions) {
      cachedRequest.tools = [{
        functionDeclarations: request.functions
      }];
    }
    
    // Add expiration time (1 hour from now by default)
    const expirationTime = new Date();
    expirationTime.setHours(expirationTime.getHours() + 1);
    cachedRequest.expireTime = options?.expireTime || expirationTime.toISOString();
    
    return cachedRequest;
  }
  
  /**
   * Extract cacheable content from response
   */
  extractCacheable(response: any): CachedContent | null {
    if (!response || !response.candidates?.[0]) {
      return null;
    }
    
    const candidate = response.candidates[0];
    
    return {
      type: 'response',
      data: {
        content: candidate.content,
        finishReason: candidate.finishReason,
        safetyRatings: candidate.safetyRatings,
        citationMetadata: candidate.citationMetadata,
        tokenCount: response.usageMetadata?.totalTokenCount
      },
      tokenCount: response.usageMetadata?.candidatesTokenCount || 0,
      hash: this.generateResponseHash(response)
    };
  }
  
  /**
   * Calculate cost saved by cache hit
   */
  calculateCostSaving(cached: CachedContent): number {
    if (!cached.data?.tokenCount) {
      return 0;
    }
    
    const tokenCount = cached.data.tokenCount;
    
    // Gemini pricing (as of 2024)
    // Cached content is 75% cheaper than regular input
    const pricing: Record<string, { input: number; output: number; cached: number }> = {
      'gemini-1.5-pro': { 
        input: 0.00125,    // $1.25 per 1M tokens
        output: 0.00375,   // $3.75 per 1M tokens  
        cached: 0.0003125  // $0.3125 per 1M tokens (75% discount)
      },
      'gemini-1.5-flash': { 
        input: 0.000075,   // $0.075 per 1M tokens
        output: 0.0003,    // $0.30 per 1M tokens
        cached: 0.00001875 // $0.01875 per 1M tokens (75% discount)
      }
    };
    
    const model = cached.data.model || 'gemini-1.5-pro';
    const price = Object.entries(pricing).find(([key]) => 
      model.includes(key)
    )?.[1] || pricing['gemini-1.5-pro'];
    
    // Calculate savings (input tokens only benefit from caching)
    const regularCost = (tokenCount / 1_000_000) * price.input;
    const cachedCost = (tokenCount / 1_000_000) * price.cached;
    
    return regularCost - cachedCost;
  }
  
  // Private methods
  
  private estimateTokenCount(request: CacheRequest): number {
    const content = JSON.stringify({
      messages: request.messages,
      systemPrompt: request.systemPrompt,
      functions: request.functions
    });
    
    // Gemini uses similar tokenization to GPT models
    return Math.ceil(content.length / 4);
  }
  
  private convertMessagesToGeminiFormat(messages: any[]): any[] {
    return messages.map(msg => {
      // Convert role names
      let role = msg.role;
      if (role === 'system') role = 'user';
      if (role === 'assistant') role = 'model';
      
      return {
        role,
        parts: typeof msg.content === 'string' 
          ? [{ text: msg.content }]
          : msg.content
      };
    });
  }
  
  private generateResponseHash(response: any): string {
    const candidate = response.candidates?.[0];
    if (!candidate) return '';
    
    const hashData = {
      content: candidate.content,
      finishReason: candidate.finishReason
    };
    
    return crypto
      .createHash('sha256')
      .update(JSON.stringify(hashData))
      .digest('hex')
      .substring(0, 16);
  }
}