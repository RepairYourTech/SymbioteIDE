/**
 * Key Sanitizer Unit Tests
 */

import { KeySanitizer } from '../../security/key-sanitizer';

describe('KeySanitizer', () => {
  describe('sanitize', () => {
    it('should sanitize OpenAI API keys', () => {
      const input = 'My API key is sk-1234567890abcdef1234567890abcdef1234567890abcdef';
      const sanitized = KeySanitizer.sanitize(input);
      expect(sanitized).toBe('My API key is sk-***[REDACTED]');
    });

    it('should sanitize Anthropic API keys', () => {
      const input = 'Using key: sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyz123456';
      const sanitized = KeySanitizer.sanitize(input);
      expect(sanitized).toBe('Using key: sk-ant-***[REDACTED]');
    });

    it('should sanitize Bearer tokens', () => {
      const input = 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...';
      const sanitized = KeySanitizer.sanitize(input);
      expect(sanitized).toBe('Authorization: Bearer ***[REDACTED]');
    });

    it('should sanitize multiple keys in one string', () => {
      const input = 'Keys: sk-1234567890abcdef1234567890abcdef1234567890abcdef and sk-ant-api03-xyz123';
      const sanitized = KeySanitizer.sanitize(input);
      expect(sanitized).toBe('Keys: sk-***[REDACTED] and sk-ant-***[REDACTED]');
    });

    it('should handle null and undefined', () => {
      expect(KeySanitizer.sanitize(null as any)).toBe(null);
      expect(KeySanitizer.sanitize(undefined as any)).toBe(undefined);
    });
  });

  describe('sanitizeObject', () => {
    it('should sanitize nested objects', () => {
      const obj = {
        config: {
          apiKey: 'sk-1234567890abcdef1234567890abcdef1234567890abcdef',
          endpoint: 'https://api.openai.com'
        },
        data: 'Some data'
      };

      const sanitized = KeySanitizer.sanitizeObject(obj);
      expect(sanitized.config.apiKey).toBe('***[REDACTED]');
      expect(sanitized.config.endpoint).toBe('https://api.openai.com');
      expect(sanitized.data).toBe('Some data');
    });

    it('should sanitize arrays', () => {
      const arr = [
        'sk-1234567890abcdef1234567890abcdef1234567890abcdef',
        'normal string',
        { apiKey: 'sk-ant-api03-xyz123' }
      ];

      const sanitized = KeySanitizer.sanitizeObject(arr);
      expect(sanitized[0]).toBe('sk-***[REDACTED]');
      expect(sanitized[1]).toBe('normal string');
      expect(sanitized[2].apiKey).toBe('***[REDACTED]');
    });

    it('should detect sensitive keys', () => {
      const obj = {
        api_key: 'any-value',
        apiKey: 'another-value',
        password: 'secret',
        normal: 'visible'
      };

      const sanitized = KeySanitizer.sanitizeObject(obj);
      expect(sanitized.api_key).toBe('***[REDACTED]');
      expect(sanitized.apiKey).toBe('***[REDACTED]');
      expect(sanitized.password).toBe('***[REDACTED]');
      expect(sanitized.normal).toBe('visible');
    });
  });

  describe('sanitizeError', () => {
    it('should sanitize error messages', () => {
      const error = new Error('Failed with key sk-1234567890abcdef1234567890abcdef1234567890abcdef');
      const sanitized = KeySanitizer.sanitizeError(error);
      expect(sanitized.message).toBe('Failed with key sk-***[REDACTED]');
    });

    it('should sanitize error stack traces', () => {
      const error = new Error('Test error');
      error.stack = 'Error at sk-1234567890abcdef1234567890abcdef1234567890abcdef';
      const sanitized = KeySanitizer.sanitizeError(error);
      expect(sanitized.stack).toBe('Error at sk-***[REDACTED]');
    });

    it('should preserve error properties', () => {
      const error: any = new Error('Test');
      error.code = 'TEST_ERROR';
      error.statusCode = 401;
      
      const sanitized = KeySanitizer.sanitizeError(error);
      expect(sanitized.name).toBe(error.name);
      expect((sanitized as any).code).toBe('TEST_ERROR');
      expect((sanitized as any).statusCode).toBe(401);
    });
  });

  describe('containsApiKey', () => {
    it('should detect API keys in strings', () => {
      expect(KeySanitizer.containsApiKey('sk-1234567890abcdef1234567890abcdef1234567890abcdef')).toBe(true);
      expect(KeySanitizer.containsApiKey('sk-ant-api03-xyz123')).toBe(true);
      expect(KeySanitizer.containsApiKey('normal string')).toBe(false);
    });

    it('should detect API keys in objects', () => {
      expect(KeySanitizer.containsApiKey({ key: 'sk-1234567890abcdef1234567890abcdef1234567890abcdef' })).toBe(true);
      expect(KeySanitizer.containsApiKey({ data: 'normal' })).toBe(false);
    });
  });

  describe('createSafeLogger', () => {
    it('should create logger that sanitizes output', () => {
      const logger = KeySanitizer.createSafeLogger();
      const consoleSpy = jest.spyOn(console, 'log').mockImplementation();

      logger.log('Key:', 'sk-1234567890abcdef1234567890abcdef1234567890abcdef');
      
      expect(consoleSpy).toHaveBeenCalledWith('Key:', 'sk-***[REDACTED]');
      consoleSpy.mockRestore();
    });

    it('should sanitize errors in logger', () => {
      const logger = KeySanitizer.createSafeLogger();
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation();

      const error = new Error('Failed with sk-1234567890abcdef1234567890abcdef1234567890abcdef');
      logger.error(error);
      
      expect(consoleSpy).toHaveBeenCalled();
      const sanitizedError = consoleSpy.mock.calls[0][0];
      expect(sanitizedError.message).toBe('Failed with sk-***[REDACTED]');
      
      consoleSpy.mockRestore();
    });
  });

  describe('safeStringify', () => {
    it('should sanitize during JSON stringification', () => {
      const obj = {
        config: {
          apiKey: 'sk-1234567890abcdef1234567890abcdef1234567890abcdef'
        }
      };

      const json = KeySanitizer.safeStringify(obj);
      expect(json).not.toContain('sk-1234567890abcdef');
      expect(json).toContain('***[REDACTED]');
    });
  });
});