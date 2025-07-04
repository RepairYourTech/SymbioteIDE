/**
 * Secure Transmission
 * 
 * Handles secure transmission of credentials to AI providers
 */

import * as crypto from 'crypto';
import { Credential } from '../core/credential-types';

export interface SecureTransmissionOptions {
  /**
   * Use TLS pinning for known providers
   */
  useTLSPinning?: boolean;
  
  /**
   * Verify provider certificates
   */
  verifyCertificates?: boolean;
  
  /**
   * Use additional encryption layer
   */
  useDoubleEncryption?: boolean;
  
  /**
   * Timeout for transmission (ms)
   */
  timeout?: number;
  
  /**
   * Maximum retry attempts
   */
  maxRetries?: number;
}

export class SecureTransmission {
  private readonly trustedCertificates: Map<string, string[]> = new Map();
  private readonly providerEndpoints: Map<string, string> = new Map();

  constructor() {
    this.initializeTrustedEndpoints();
  }

  /**
   * Create secure headers for API request
   */
  createSecureHeaders(
    credential: Credential,
    additionalHeaders: Record<string, string> = {}
  ): Record<string, string> {
    const headers: Record<string, string> = {
      ...additionalHeaders,
      'X-Request-ID': this.generateRequestId(),
      'X-Timestamp': new Date().toISOString()
    };

    // Add provider-specific auth headers
    switch (credential.provider) {
      case 'anthropic':
        headers['x-api-key'] = credential.apiKey;
        headers['anthropic-version'] = '2023-06-01';
        break;
      
      case 'openai':
        headers['Authorization'] = `Bearer ${credential.apiKey}`;
        break;
      
      case 'google':
        // Google uses API key in URL params, not headers
        break;
      
      case 'mistral':
        headers['Authorization'] = `Bearer ${credential.apiKey}`;
        break;
      
      default:
        headers['Authorization'] = `Bearer ${credential.apiKey}`;
    }

    return headers;
  }

  /**
   * Create secure fetch options
   */
  createSecureFetchOptions(
    method: string,
    headers: Record<string, string>,
    body?: any,
    options: SecureTransmissionOptions = {}
  ): RequestInit {
    const fetchOptions: RequestInit = {
      method,
      headers,
      signal: this.createAbortSignal(options.timeout),
      // @ts-ignore - Node.js specific options
      agent: this.createSecureAgent(options)
    };

    if (body) {
      if (typeof body === 'object') {
        fetchOptions.body = JSON.stringify(body);
        fetchOptions.headers = {
          ...fetchOptions.headers,
          'Content-Type': 'application/json'
        };
      } else {
        fetchOptions.body = body;
      }
    }

    return fetchOptions;
  }

  /**
   * Execute secure request with retry logic
   */
  async executeSecureRequest(
    url: string,
    options: RequestInit,
    transmissionOptions: SecureTransmissionOptions = {}
  ): Promise<Response> {
    const maxRetries = transmissionOptions.maxRetries || 3;
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        // Verify endpoint is trusted
        this.verifyTrustedEndpoint(url);

        // Make request
        const response = await fetch(url, options);

        // Log transmission (without sensitive data)
        this.logTransmission(url, response.status, attempt);

        // Handle rate limiting
        if (response.status === 429) {
          const retryAfter = response.headers.get('Retry-After');
          if (retryAfter && attempt < maxRetries - 1) {
            const delay = parseInt(retryAfter) * 1000;
            await this.delay(Math.min(delay, 60000)); // Max 1 minute
            continue;
          }
        }

        return response;
      } catch (error) {
        lastError = error as Error;
        
        // Don't retry on certain errors
        if (this.isNonRetriableError(error)) {
          throw error;
        }

        // Exponential backoff
        if (attempt < maxRetries - 1) {
          const delay = Math.pow(2, attempt) * 1000;
          await this.delay(delay);
        }
      }
    }

    throw lastError || new Error('Request failed after retries');
  }

  /**
   * Create one-time use token for credential
   */
  createOneTimeToken(credential: Credential): {
    token: string;
    expires: number;
    hash: string;
  } {
    const token = crypto.randomBytes(32).toString('hex');
    const expires = Date.now() + 300000; // 5 minutes
    
    // Create hash for verification
    const hash = crypto
      .createHash('sha256')
      .update(token)
      .update(credential.id)
      .update(expires.toString())
      .digest('hex');

    return { token, expires, hash };
  }

  /**
   * Initialize trusted endpoints and certificates
   */
  private initializeTrustedEndpoints(): void {
    // Anthropic
    this.providerEndpoints.set('anthropic', 'https://api.anthropic.com');
    this.trustedCertificates.set('api.anthropic.com', [
      // Add certificate fingerprints here
    ]);

    // OpenAI
    this.providerEndpoints.set('openai', 'https://api.openai.com');
    this.trustedCertificates.set('api.openai.com', [
      // Add certificate fingerprints here
    ]);

    // Google
    this.providerEndpoints.set('google', 'https://generativelanguage.googleapis.com');
    this.trustedCertificates.set('generativelanguage.googleapis.com', [
      // Add certificate fingerprints here
    ]);

    // Mistral
    this.providerEndpoints.set('mistral', 'https://api.mistral.ai');
    this.trustedCertificates.set('api.mistral.ai', [
      // Add certificate fingerprints here
    ]);
  }

  /**
   * Verify endpoint is trusted
   */
  private verifyTrustedEndpoint(url: string): void {
    const urlObj = new URL(url);
    const hostname = urlObj.hostname;

    // Check if hostname is in trusted list
    const trusted = Array.from(this.providerEndpoints.values()).some(endpoint => {
      const trustedUrl = new URL(endpoint);
      return trustedUrl.hostname === hostname;
    });

    if (!trusted) {
      throw new Error(`Untrusted endpoint: ${hostname}`);
    }
  }

  /**
   * Create secure agent for Node.js
   */
  private createSecureAgent(options: SecureTransmissionOptions): any {
    if (typeof window !== 'undefined') {
      // Browser environment
      return undefined;
    }

    try {
      const https = require('https');
      
      return new https.Agent({
        rejectUnauthorized: options.verifyCertificates !== false,
        // Additional security options
        secureOptions: crypto.constants.SSL_OP_NO_TLSv1 | crypto.constants.SSL_OP_NO_TLSv1_1,
        ciphers: 'ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-GCM-SHA256'
      });
    } catch {
      return undefined;
    }
  }

  /**
   * Create abort signal for timeout
   */
  private createAbortSignal(timeout?: number): AbortSignal {
    if (!timeout) {
      return new AbortController().signal;
    }

    const controller = new AbortController();
    setTimeout(() => controller.abort(), timeout);
    return controller.signal;
  }

  /**
   * Check if error is non-retriable
   */
  private isNonRetriableError(error: any): boolean {
    const message = error.message || '';
    
    // Don't retry on authentication errors
    if (message.includes('401') || message.includes('403')) {
      return true;
    }
    
    // Don't retry on invalid request errors
    if (message.includes('400') || message.includes('422')) {
      return true;
    }

    return false;
  }

  /**
   * Log transmission (without sensitive data)
   */
  private logTransmission(url: string, status: number, attempt: number): void {
    const urlObj = new URL(url);
    const sanitizedUrl = `${urlObj.protocol}//${urlObj.hostname}${urlObj.pathname}`;
    
    console.log(`API Request: ${sanitizedUrl} - Status: ${status} - Attempt: ${attempt + 1}`);
  }

  /**
   * Generate request ID
   */
  private generateRequestId(): string {
    return crypto.randomBytes(16).toString('hex');
  }

  /**
   * Delay helper
   */
  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

/**
 * Secure proxy for credential transmission
 */
export class SecureProxy {
  private readonly transmission: SecureTransmission;

  constructor() {
    this.transmission = new SecureTransmission();
  }

  /**
   * Create proxied fetch function that automatically handles credentials
   */
  createSecureFetch(
    credential: Credential,
    options: SecureTransmissionOptions = {}
  ): typeof fetch {
    const transmission = this.transmission;

    return async function secureFetch(
      input: RequestInfo | URL,
      init?: RequestInit
    ): Promise<Response> {
      const url = typeof input === 'string' ? input : input.toString();
      
      // Create secure headers
      const headers = transmission.createSecureHeaders(credential, init?.headers as any);
      
      // Create secure options
      const secureOptions = transmission.createSecureFetchOptions(
        init?.method || 'GET',
        headers,
        init?.body,
        options
      );

      // Merge with user options
      const finalOptions: RequestInit = {
        ...init,
        ...secureOptions,
        headers: {
          ...init?.headers,
          ...secureOptions.headers
        }
      };

      // Execute request
      return transmission.executeSecureRequest(url, finalOptions, options);
    };
  }
}