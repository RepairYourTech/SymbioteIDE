/**
 * Orchestration Integration
 * 
 * Integrates credential management with the orchestration engine
 */

import { CredentialManager } from '../core/credential-manager';
import { SecureTransmission } from '../security/secure-transmission';
import { ProviderType } from '../core/credential-types';
import { ProviderAdapter } from '../../orchestration/providers/provider-adapter';

export class CredentialOrchestrationIntegration {
  private credentialManager: CredentialManager;
  private secureTransmission: SecureTransmission;
  private credentialCache: Map<string, { apiKey: string; expires: number }> = new Map();
  private readonly cacheTimeout = 300000; // 5 minutes

  constructor(credentialManager: CredentialManager) {
    this.credentialManager = credentialManager;
    this.secureTransmission = new SecureTransmission();
    
    // Clean up cache periodically
    setInterval(() => this.cleanupCache(), 60000);
  }

  /**
   * Get API key for provider
   */
  async getApiKeyForProvider(provider: ProviderType): Promise<string | null> {
    // Check cache first
    const cached = this.credentialCache.get(provider);
    if (cached && cached.expires > Date.now()) {
      return cached.apiKey;
    }

    // Get credentials for provider
    const credentials = await this.credentialManager.getCredentialsByProvider(provider);
    
    if (credentials.length === 0) {
      // Fall back to environment variables
      return this.getApiKeyFromEnvironment(provider);
    }

    // Use the first available credential
    const credential = await this.credentialManager.useCredential(credentials[0].id);
    
    if (!credential) {
      return null;
    }

    // Cache the API key
    this.credentialCache.set(provider, {
      apiKey: credential.apiKey,
      expires: Date.now() + this.cacheTimeout
    });

    return credential.apiKey;
  }

  /**
   * Create provider adapter with credentials
   */
  async createProviderAdapter(
    provider: ProviderType,
    AdapterClass: typeof ProviderAdapter
  ): Promise<ProviderAdapter> {
    const apiKey = await this.getApiKeyForProvider(provider);
    
    if (!apiKey) {
      throw new Error(`No API key found for provider: ${provider}`);
    }

    // Create adapter with secure configuration
    return new (AdapterClass as any)({
      apiKey,
      // Use secure transmission for API calls
      fetch: this.createSecureFetch(provider, apiKey)
    });
  }

  /**
   * Update provider adapter configuration
   */
  updateProviderAdapterConfig(adapter: ProviderAdapter): void {
    // Override the getApiKeyFromEnv method to use credential manager
    const originalGetApiKey = adapter['getApiKeyFromEnv'].bind(adapter);
    
    adapter['getApiKeyFromEnv'] = () => {
      // This will be called when adapter needs API key
      // We'll return empty string and handle it in execute method
      return '';
    };

    // Override execute method to inject credentials
    const originalExecute = adapter.execute.bind(adapter);
    
    adapter.execute = async (task: any, model: any, options: any) => {
      // Get provider type from model
      const provider = model.provider as ProviderType;
      
      // Get API key
      const apiKey = await this.getApiKeyForProvider(provider);
      
      if (!apiKey) {
        throw new Error(`No API key found for provider: ${provider}`);
      }

      // Update adapter configuration
      adapter['apiKey'] = apiKey;
      
      // Create secure options
      const secureOptions = {
        ...options,
        apiKey,
        fetch: this.createSecureFetch(provider, apiKey)
      };

      // Execute with secure options
      return originalExecute(task, model, secureOptions);
    };
  }

  /**
   * Create secure fetch function
   */
  private createSecureFetch(provider: ProviderType, apiKey: string): typeof fetch {
    const credential = {
      id: `temp-${provider}`,
      provider,
      name: 'Temporary',
      apiKey,
      createdAt: new Date()
    };

    const secureProxy = new (require('../security/secure-transmission').SecureProxy)();
    return secureProxy.createSecureFetch(credential);
  }

  /**
   * Get API key from environment
   */
  private getApiKeyFromEnvironment(provider: ProviderType): string | null {
    const envMap: Record<ProviderType, string> = {
      [ProviderType.Anthropic]: 'ANTHROPIC_API_KEY',
      [ProviderType.OpenAI]: 'OPENAI_API_KEY',
      [ProviderType.Google]: 'GOOGLE_API_KEY',
      [ProviderType.Mistral]: 'MISTRAL_API_KEY',
      [ProviderType.Azure]: 'AZURE_OPENAI_API_KEY',
      [ProviderType.AWS]: 'AWS_ACCESS_KEY_ID',
      [ProviderType.Custom]: 'CUSTOM_API_KEY'
    };

    const envVar = envMap[provider];
    return envVar ? process.env[envVar] || null : null;
  }

  /**
   * Clean up expired cache entries
   */
  private cleanupCache(): void {
    const now = Date.now();
    
    for (const [provider, entry] of this.credentialCache) {
      if (entry.expires <= now) {
        // Clear from memory
        entry.apiKey = '';
        this.credentialCache.delete(provider);
      }
    }
  }

  /**
   * Clear all cached credentials
   */
  clearCache(): void {
    // Clear API keys from memory
    for (const [_, entry] of this.credentialCache) {
      entry.apiKey = '';
    }
    this.credentialCache.clear();
  }
}

/**
 * Monkey patch for existing provider adapters
 */
export function patchProviderAdapters(credentialManager: CredentialManager): void {
  const integration = new CredentialOrchestrationIntegration(credentialManager);
  
  // Import provider adapters
  const adapters = [
    require('../../orchestration/providers/anthropic-adapter').AnthropicAdapter,
    require('../../orchestration/providers/openai-adapter').OpenAIAdapter,
    require('../../orchestration/providers/google-adapter').GoogleAdapter,
    require('../../orchestration/providers/mistral-adapter').MistralAdapter
  ];

  // Patch each adapter's prototype
  for (const AdapterClass of adapters) {
    const originalConstructor = AdapterClass.prototype.constructor;
    
    AdapterClass.prototype.constructor = function(...args: any[]) {
      originalConstructor.apply(this, args);
      integration.updateProviderAdapterConfig(this);
    };
  }
}