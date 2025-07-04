/**
 * Key Sanitizer
 * 
 * Ensures API keys are never logged or exposed in error messages
 */

export class KeySanitizer {
  private static readonly patterns: Array<{
    name: string;
    pattern: RegExp;
    replacement: string;
  }> = [
    {
      name: 'OpenAI',
      pattern: /sk-[a-zA-Z0-9]{48}/g,
      replacement: 'sk-***[REDACTED]'
    },
    {
      name: 'Anthropic',
      pattern: /sk-ant-[a-zA-Z0-9-_]{40,}/g,
      replacement: 'sk-ant-***[REDACTED]'
    },
    {
      name: 'Google',
      pattern: /AIza[a-zA-Z0-9_-]{35}/g,
      replacement: 'AIza***[REDACTED]'
    },
    {
      name: 'Mistral',
      pattern: /[a-f0-9]{32}/g,
      replacement: '***[REDACTED]'
    },
    {
      name: 'AWS',
      pattern: /AKIA[A-Z0-9]{16}/g,
      replacement: 'AKIA***[REDACTED]'
    },
    {
      name: 'Bearer Token',
      pattern: /Bearer\s+[a-zA-Z0-9\-._~+\/]+=*/g,
      replacement: 'Bearer ***[REDACTED]'
    },
    {
      name: 'Basic Auth',
      pattern: /Basic\s+[a-zA-Z0-9+\/]+=*/g,
      replacement: 'Basic ***[REDACTED]'
    },
    {
      name: 'API Key Header',
      pattern: /['"](api[-_]?key|apikey)['"]\s*[:=]\s*['"][^'"]+['"]/gi,
      replacement: '"api_key": "***[REDACTED]"'
    },
    {
      name: 'Authorization Header',
      pattern: /['"](authorization)['"]\s*[:=]\s*['"][^'"]+['"]/gi,
      replacement: '"authorization": "***[REDACTED]"'
    }
  ];

  /**
   * Sanitize a string by removing all potential API keys
   */
  static sanitize(input: string): string {
    if (!input || typeof input !== 'string') {
      return input;
    }

    let sanitized = input;

    // Apply all patterns
    for (const { pattern, replacement } of this.patterns) {
      sanitized = sanitized.replace(pattern, replacement);
    }

    return sanitized;
  }

  /**
   * Sanitize an object recursively
   */
  static sanitizeObject(obj: any): any {
    if (!obj) return obj;

    if (typeof obj === 'string') {
      return this.sanitize(obj);
    }

    if (Array.isArray(obj)) {
      return obj.map(item => this.sanitizeObject(item));
    }

    if (typeof obj === 'object') {
      const sanitized: any = {};
      
      for (const [key, value] of Object.entries(obj)) {
        // Check if key might contain sensitive data
        if (this.isSensitiveKey(key)) {
          sanitized[key] = '***[REDACTED]';
        } else {
          sanitized[key] = this.sanitizeObject(value);
        }
      }
      
      return sanitized;
    }

    return obj;
  }

  /**
   * Sanitize error object
   */
  static sanitizeError(error: Error): Error {
    const sanitizedError = new Error(this.sanitize(error.message));
    sanitizedError.name = error.name;
    
    if (error.stack) {
      sanitizedError.stack = this.sanitize(error.stack);
    }

    // Copy other properties
    for (const key of Object.keys(error)) {
      if (key !== 'message' && key !== 'stack') {
        (sanitizedError as any)[key] = this.sanitizeObject((error as any)[key]);
      }
    }

    return sanitizedError;
  }

  /**
   * Check if an object contains potential API keys
   */
  static containsApiKey(input: any): boolean {
    const str = typeof input === 'string' ? input : JSON.stringify(input);
    
    for (const { pattern } of this.patterns) {
      if (pattern.test(str)) {
        return true;
      }
    }

    return false;
  }

  /**
   * Create a safe logger that automatically sanitizes output
   */
  static createSafeLogger(): {
    log: (...args: any[]) => void;
    error: (...args: any[]) => void;
    warn: (...args: any[]) => void;
    info: (...args: any[]) => void;
    debug: (...args: any[]) => void;
  } {
    const sanitizeArgs = (args: any[]): any[] => {
      return args.map(arg => {
        if (typeof arg === 'string') {
          return this.sanitize(arg);
        } else if (arg instanceof Error) {
          return this.sanitizeError(arg);
        } else {
          return this.sanitizeObject(arg);
        }
      });
    };

    return {
      log: (...args: any[]) => console.log(...sanitizeArgs(args)),
      error: (...args: any[]) => console.error(...sanitizeArgs(args)),
      warn: (...args: any[]) => console.warn(...sanitizeArgs(args)),
      info: (...args: any[]) => console.info(...sanitizeArgs(args)),
      debug: (...args: any[]) => console.debug(...sanitizeArgs(args))
    };
  }

  /**
   * Install global error handler that sanitizes errors
   */
  static installGlobalHandler(): void {
    if (typeof process !== 'undefined') {
      // Node.js environment
      process.on('uncaughtException', (error: Error) => {
        console.error('Uncaught Exception:', this.sanitizeError(error));
        process.exit(1);
      });

      process.on('unhandledRejection', (reason: any) => {
        const error = reason instanceof Error ? reason : new Error(String(reason));
        console.error('Unhandled Rejection:', this.sanitizeError(error));
        process.exit(1);
      });
    }

    if (typeof window !== 'undefined') {
      // Browser environment
      window.addEventListener('error', (event) => {
        event.preventDefault();
        console.error('Global Error:', this.sanitizeError(event.error));
      });

      window.addEventListener('unhandledrejection', (event) => {
        event.preventDefault();
        const error = event.reason instanceof Error 
          ? event.reason 
          : new Error(String(event.reason));
        console.error('Unhandled Promise Rejection:', this.sanitizeError(error));
      });
    }
  }

  /**
   * Create a sanitized JSON.stringify
   */
  static safeStringify(obj: any, replacer?: any, space?: string | number): string {
    const sanitizedObj = this.sanitizeObject(obj);
    return JSON.stringify(sanitizedObj, replacer, space);
  }

  /**
   * Check if a key name suggests it contains sensitive data
   */
  private static isSensitiveKey(key: string): boolean {
    const sensitivePatterns = [
      /api[-_]?key/i,
      /apikey/i,
      /secret/i,
      /password/i,
      /token/i,
      /auth/i,
      /credential/i,
      /private[-_]?key/i,
      /access[-_]?key/i
    ];

    return sensitivePatterns.some(pattern => pattern.test(key));
  }
}

/**
 * Middleware for Express to sanitize request/response
 */
export function sanitizerMiddleware() {
  return (req: any, res: any, next: any) => {
    // Override console methods for this request
    const originalConsole = {
      log: console.log,
      error: console.error,
      warn: console.warn,
      info: console.info,
      debug: console.debug
    };

    const safeLogger = KeySanitizer.createSafeLogger();
    Object.assign(console, safeLogger);

    // Sanitize response
    const originalJson = res.json;
    res.json = function(data: any) {
      const sanitized = KeySanitizer.sanitizeObject(data);
      return originalJson.call(this, sanitized);
    };

    // Restore console on response end
    res.on('finish', () => {
      Object.assign(console, originalConsole);
    });

    next();
  };
}