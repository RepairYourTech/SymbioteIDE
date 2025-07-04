/**
 * Credential Validation Service
 * 
 * Validates API key formats and tests them with providers
 */

import { ProviderType, CredentialValidator, CredentialValidationResult } from './credential-types';

export class ValidationService {
  private validators: Map<ProviderType, CredentialValidator>;

  constructor() {
    this.validators = new Map();
    this.registerDefaultValidators();
  }

  /**
   * Register a validator for a provider
   */
  registerValidator(provider: ProviderType, validator: CredentialValidator): void {
    this.validators.set(provider, validator);
  }

  /**
   * Validate credential format
   */
  validateFormat(provider: ProviderType, apiKey: string): boolean {
    const validator = this.validators.get(provider);
    if (!validator) {
      return true; // Allow unknown providers
    }
    return validator.validateFormat(apiKey);
  }

  /**
   * Validate credential with provider
   */
  async validateWithProvider(
    provider: ProviderType, 
    apiKey: string
  ): Promise<CredentialValidationResult> {
    const validator = this.validators.get(provider);
    if (!validator) {
      return {
        valid: true,
        error: 'No validator available for this provider'
      };
    }
    return validator.validateWithProvider(apiKey);
  }

  /**
   * Detect provider from API key format
   */
  detectProvider(apiKey: string): ProviderType | null {
    for (const [provider, validator] of this.validators) {
      const detected = validator.detectProvider(apiKey);
      if (detected) {
        return detected;
      }
    }
    return null;
  }

  /**
   * Register default validators
   */
  private registerDefaultValidators(): void {
    // Anthropic validator
    this.registerValidator(ProviderType.Anthropic, {
      validateFormat: (apiKey: string) => {
        // Anthropic keys start with 'sk-ant-'
        return /^sk-ant-[a-zA-Z0-9-_]{40,}$/.test(apiKey);
      },
      validateWithProvider: async (apiKey: string) => {
        try {
          // Make a minimal API call to validate
          const response = await fetch('https://api.anthropic.com/v1/messages', {
            method: 'POST',
            headers: {
              'x-api-key': apiKey,
              'anthropic-version': '2023-06-01',
              'content-type': 'application/json'
            },
            body: JSON.stringify({
              model: 'claude-3-haiku-20240307',
              messages: [{ role: 'user', content: 'Hi' }],
              max_tokens: 1
            })
          });

          if (response.status === 401) {
            return { valid: false, error: 'Invalid API key' };
          }

          if (response.status === 200) {
            return { 
              valid: true,
              providerInfo: {
                capabilities: ['chat', 'code', 'vision']
              }
            };
          }

          return { valid: false, error: `Unexpected status: ${response.status}` };
        } catch (error) {
          return { valid: false, error: `Connection error: ${error.message}` };
        }
      },
      detectProvider: (apiKey: string) => {
        return apiKey.startsWith('sk-ant-') ? ProviderType.Anthropic : null;
      }
    });

    // OpenAI validator
    this.registerValidator(ProviderType.OpenAI, {
      validateFormat: (apiKey: string) => {
        // OpenAI keys start with 'sk-'
        return /^sk-[a-zA-Z0-9]{48}$/.test(apiKey);
      },
      validateWithProvider: async (apiKey: string) => {
        try {
          const response = await fetch('https://api.openai.com/v1/models', {
            headers: {
              'Authorization': `Bearer ${apiKey}`
            }
          });

          if (response.status === 401) {
            return { valid: false, error: 'Invalid API key' };
          }

          if (response.status === 200) {
            const data = await response.json();
            return { 
              valid: true,
              providerInfo: {
                capabilities: ['chat', 'code', 'function', 'vision']
              }
            };
          }

          return { valid: false, error: `Unexpected status: ${response.status}` };
        } catch (error) {
          return { valid: false, error: `Connection error: ${error.message}` };
        }
      },
      detectProvider: (apiKey: string) => {
        return /^sk-[a-zA-Z0-9]{48}$/.test(apiKey) ? ProviderType.OpenAI : null;
      }
    });

    // Google validator
    this.registerValidator(ProviderType.Google, {
      validateFormat: (apiKey: string) => {
        // Google API keys are typically 39 characters
        return /^[a-zA-Z0-9_-]{39}$/.test(apiKey);
      },
      validateWithProvider: async (apiKey: string) => {
        try {
          const response = await fetch(
            `https://generativelanguage.googleapis.com/v1beta/models?key=${apiKey}`
          );

          if (response.status === 403 || response.status === 401) {
            return { valid: false, error: 'Invalid API key' };
          }

          if (response.status === 200) {
            return { 
              valid: true,
              providerInfo: {
                capabilities: ['chat', 'code']
              }
            };
          }

          return { valid: false, error: `Unexpected status: ${response.status}` };
        } catch (error) {
          return { valid: false, error: `Connection error: ${error.message}` };
        }
      },
      detectProvider: (apiKey: string) => {
        return /^[a-zA-Z0-9_-]{39}$/.test(apiKey) ? ProviderType.Google : null;
      }
    });

    // Mistral validator
    this.registerValidator(ProviderType.Mistral, {
      validateFormat: (apiKey: string) => {
        // Mistral keys are 32 characters
        return /^[a-zA-Z0-9]{32}$/.test(apiKey);
      },
      validateWithProvider: async (apiKey: string) => {
        try {
          const response = await fetch('https://api.mistral.ai/v1/models', {
            headers: {
              'Authorization': `Bearer ${apiKey}`
            }
          });

          if (response.status === 401) {
            return { valid: false, error: 'Invalid API key' };
          }

          if (response.status === 200) {
            return { 
              valid: true,
              providerInfo: {
                capabilities: ['chat', 'code']
              }
            };
          }

          return { valid: false, error: `Unexpected status: ${response.status}` };
        } catch (error) {
          return { valid: false, error: `Connection error: ${error.message}` };
        }
      },
      detectProvider: (apiKey: string) => {
        return /^[a-zA-Z0-9]{32}$/.test(apiKey) ? ProviderType.Mistral : null;
      }
    });

    // Azure OpenAI validator
    this.registerValidator(ProviderType.Azure, {
      validateFormat: (apiKey: string) => {
        // Azure keys are typically 32 characters hex
        return /^[a-f0-9]{32}$/.test(apiKey);
      },
      validateWithProvider: async (apiKey: string) => {
        // Azure validation requires endpoint URL as well
        return {
          valid: true,
          error: 'Azure validation requires endpoint URL'
        };
      },
      detectProvider: (apiKey: string) => {
        return /^[a-f0-9]{32}$/.test(apiKey) ? ProviderType.Azure : null;
      }
    });

    // AWS Bedrock validator
    this.registerValidator(ProviderType.AWS, {
      validateFormat: (apiKey: string) => {
        // AWS access keys start with AKIA and are 20 characters
        return /^AKIA[A-Z0-9]{16}$/.test(apiKey);
      },
      validateWithProvider: async (apiKey: string) => {
        // AWS validation requires secret key and region as well
        return {
          valid: true,
          error: 'AWS validation requires secret key and region'
        };
      },
      detectProvider: (apiKey: string) => {
        return apiKey.startsWith('AKIA') ? ProviderType.AWS : null;
      }
    });

    // Custom provider validator (always passes)
    this.registerValidator(ProviderType.Custom, {
      validateFormat: (apiKey: string) => {
        return apiKey.length > 0;
      },
      validateWithProvider: async (apiKey: string) => {
        return { valid: true };
      },
      detectProvider: (apiKey: string) => {
        return null; // Never auto-detect as custom
      }
    });
  }
}

/**
 * Rate limiter for validation requests
 */
export class ValidationRateLimiter {
  private attempts: Map<string, number[]> = new Map();
  private readonly maxAttempts = 5;
  private readonly windowMs = 60000; // 1 minute

  /**
   * Check if validation is allowed
   */
  canValidate(apiKey: string): boolean {
    const now = Date.now();
    const hash = this.hashKey(apiKey);
    const attempts = this.attempts.get(hash) || [];
    
    // Remove old attempts
    const recentAttempts = attempts.filter(time => now - time < this.windowMs);
    
    if (recentAttempts.length >= this.maxAttempts) {
      return false;
    }
    
    // Add current attempt
    recentAttempts.push(now);
    this.attempts.set(hash, recentAttempts);
    
    return true;
  }

  /**
   * Hash API key for rate limiting
   */
  private hashKey(apiKey: string): string {
    // Use first 8 chars as identifier (don't store full key)
    return apiKey.substring(0, 8);
  }
}