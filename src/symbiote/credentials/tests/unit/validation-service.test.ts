/**
 * Validation Service Unit Tests
 */

import { ValidationService, ValidationRateLimiter } from '../../core/validation-service';
import { ProviderType } from '../../core/credential-types';

describe('ValidationService', () => {
  let validationService: ValidationService;

  beforeEach(() => {
    validationService = new ValidationService();
  });

  describe('validateFormat', () => {
    it('should validate Anthropic API key format', () => {
      const validKey = 'sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyzABCDEF';
      const invalidKey = 'sk-1234567890';
      
      expect(validationService.validateFormat(ProviderType.Anthropic, validKey)).toBe(true);
      expect(validationService.validateFormat(ProviderType.Anthropic, invalidKey)).toBe(false);
    });

    it('should validate OpenAI API key format', () => {
      const validKey = 'sk-' + 'a'.repeat(48);
      const invalidKey = 'sk-ant-' + 'a'.repeat(48);
      
      expect(validationService.validateFormat(ProviderType.OpenAI, validKey)).toBe(true);
      expect(validationService.validateFormat(ProviderType.OpenAI, invalidKey)).toBe(false);
    });

    it('should validate Google API key format', () => {
      const validKey = 'a'.repeat(39);
      const invalidKey = 'a'.repeat(38);
      
      expect(validationService.validateFormat(ProviderType.Google, validKey)).toBe(true);
      expect(validationService.validateFormat(ProviderType.Google, invalidKey)).toBe(false);
    });

    it('should validate Mistral API key format', () => {
      const validKey = 'a'.repeat(32);
      const invalidKey = 'a'.repeat(31);
      
      expect(validationService.validateFormat(ProviderType.Mistral, validKey)).toBe(true);
      expect(validationService.validateFormat(ProviderType.Mistral, invalidKey)).toBe(false);
    });

    it('should allow unknown providers', () => {
      expect(validationService.validateFormat('unknown' as ProviderType, 'any-key')).toBe(true);
    });
  });

  describe('detectProvider', () => {
    it('should detect Anthropic keys', () => {
      const key = 'sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyzABCDEF';
      expect(validationService.detectProvider(key)).toBe(ProviderType.Anthropic);
    });

    it('should detect OpenAI keys', () => {
      const key = 'sk-' + 'a'.repeat(48);
      expect(validationService.detectProvider(key)).toBe(ProviderType.OpenAI);
    });

    it('should detect AWS keys', () => {
      const key = 'AKIA' + 'A'.repeat(16);
      expect(validationService.detectProvider(key)).toBe(ProviderType.AWS);
    });

    it('should return null for unrecognized formats', () => {
      expect(validationService.detectProvider('random-key')).toBe(null);
    });
  });

  describe('validateWithProvider', () => {
    beforeEach(() => {
      // Mock fetch
      global.fetch = jest.fn();
    });

    it('should validate with Anthropic API', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        status: 200,
        json: async () => ({})
      });

      const result = await validationService.validateWithProvider(
        ProviderType.Anthropic,
        'sk-ant-test-key'
      );

      expect(result.valid).toBe(true);
      expect(result.providerInfo?.capabilities).toContain('chat');
      expect(global.fetch).toHaveBeenCalledWith(
        'https://api.anthropic.com/v1/messages',
        expect.objectContaining({
          headers: expect.objectContaining({
            'x-api-key': 'sk-ant-test-key'
          })
        })
      );
    });

    it('should handle invalid API key', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        status: 401
      });

      const result = await validationService.validateWithProvider(
        ProviderType.OpenAI,
        'invalid-key'
      );

      expect(result.valid).toBe(false);
      expect(result.error).toBe('Invalid API key');
    });

    it('should handle network errors', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('Network error'));

      const result = await validationService.validateWithProvider(
        ProviderType.OpenAI,
        'sk-test-key'
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Connection error');
    });
  });
});

describe('ValidationRateLimiter', () => {
  let rateLimiter: ValidationRateLimiter;

  beforeEach(() => {
    rateLimiter = new ValidationRateLimiter();
  });

  it('should allow initial validations', () => {
    const apiKey = 'sk-test-key';
    
    for (let i = 0; i < 5; i++) {
      expect(rateLimiter.canValidate(apiKey)).toBe(true);
    }
  });

  it('should block after rate limit exceeded', () => {
    const apiKey = 'sk-test-key';
    
    // Use up all attempts
    for (let i = 0; i < 5; i++) {
      rateLimiter.canValidate(apiKey);
    }
    
    // Should be blocked
    expect(rateLimiter.canValidate(apiKey)).toBe(false);
  });

  it('should use key prefix for rate limiting', () => {
    const key1 = 'sk-test-1234567890';
    const key2 = 'sk-test-0987654321';
    
    // Both keys share the same prefix
    for (let i = 0; i < 5; i++) {
      rateLimiter.canValidate(key1);
    }
    
    // Second key should also be blocked
    expect(rateLimiter.canValidate(key2)).toBe(false);
  });

  it('should reset after time window', () => {
    jest.useFakeTimers();
    
    const apiKey = 'sk-test-key';
    
    // Use up attempts
    for (let i = 0; i < 5; i++) {
      rateLimiter.canValidate(apiKey);
    }
    
    expect(rateLimiter.canValidate(apiKey)).toBe(false);
    
    // Advance time past window
    jest.advanceTimersByTime(61000); // 61 seconds
    
    // Should be allowed again
    expect(rateLimiter.canValidate(apiKey)).toBe(true);
    
    jest.useRealTimers();
  });
});