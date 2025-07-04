/**
 * Zero-Knowledge Vault
 * 
 * Ensures credentials never leave the user's machine unencrypted
 */

import { EventEmitter } from 'events';
import { Credential, SecureCredential } from '../core/credential-types';

export interface ZeroKnowledgePolicy {
  /**
   * Prevent credential export in plain text
   */
  allowPlainTextExport: false;
  
  /**
   * Require encryption for all network transmissions
   */
  requireEncryptedTransmission: true;
  
  /**
   * Clear credentials from memory after use
   */
  autoClearMemory: true;
  
  /**
   * Maximum time credentials can stay in memory (ms)
   */
  maxMemoryRetention: number;
  
  /**
   * Prevent credentials from being logged
   */
  preventLogging: true;
  
  /**
   * Require secure deletion of temporary files
   */
  secureFileDelete: true;
}

export class ZeroKnowledgeVault extends EventEmitter {
  private readonly policy: ZeroKnowledgePolicy = {
    allowPlainTextExport: false,
    requireEncryptedTransmission: true,
    autoClearMemory: true,
    maxMemoryRetention: 300000, // 5 minutes
    preventLogging: true,
    secureFileDelete: true
  };

  private memoryTimers: Map<string, NodeJS.Timeout> = new Map();

  /**
   * Wrap credential access with zero-knowledge protections
   */
  async accessCredential<T>(
    credentialId: string,
    accessor: (credential: Credential) => Promise<T>,
    getCredential: (id: string) => Promise<Credential | null>
  ): Promise<T> {
    let credential: Credential | null = null;
    
    try {
      // Retrieve credential
      credential = await getCredential(credentialId);
      
      if (!credential) {
        throw new Error('Credential not found');
      }

      // Set auto-clear timer
      this.setMemoryClearTimer(credentialId, credential);

      // Execute accessor function
      const result = await accessor(credential);

      return result;
    } finally {
      // Clear credential from memory
      if (credential) {
        this.clearFromMemory(credential);
      }
    }
  }

  /**
   * Prepare credential for network transmission
   */
  async prepareForTransmission(
    credential: Credential,
    encrypt: (data: string) => Promise<{ encrypted: string; key: string }>
  ): Promise<{ encryptedCredential: string; transmissionKey: string }> {
    if (!this.policy.requireEncryptedTransmission) {
      throw new Error('Encrypted transmission is required by policy');
    }

    // Never transmit plain text credentials
    const sanitized = this.sanitizeForTransmission(credential);
    const serialized = JSON.stringify(sanitized);
    
    // Encrypt for transmission
    const { encrypted, key } = await encrypt(serialized);

    return {
      encryptedCredential: encrypted,
      transmissionKey: key
    };
  }

  /**
   * Validate that a value doesn't contain credentials
   */
  validateNoCredentials(value: any, path: string = ''): void {
    if (typeof value === 'string') {
      // Check for common API key patterns
      const patterns = [
        /sk-[a-zA-Z0-9]{48}/, // OpenAI
        /sk-ant-[a-zA-Z0-9-_]{40,}/, // Anthropic
        /^[a-zA-Z0-9_-]{39}$/, // Google
        /^[a-zA-Z0-9]{32}$/, // Mistral
        /^AKIA[A-Z0-9]{16}$/, // AWS
      ];

      for (const pattern of patterns) {
        if (pattern.test(value)) {
          throw new Error(`Potential credential leak detected at ${path}`);
        }
      }
    } else if (typeof value === 'object' && value !== null) {
      // Recursively check object properties
      for (const [key, val] of Object.entries(value)) {
        this.validateNoCredentials(val, `${path}.${key}`);
      }
    } else if (Array.isArray(value)) {
      // Check array elements
      value.forEach((item, index) => {
        this.validateNoCredentials(item, `${path}[${index}]`);
      });
    }
  }

  /**
   * Create a secure context for credential operations
   */
  createSecureContext(): {
    log: (...args: any[]) => void;
    error: (...args: any[]) => void;
    stringify: (obj: any) => string;
  } {
    const self = this;
    
    return {
      log: (...args: any[]) => {
        // Validate no credentials in log
        args.forEach(arg => self.validateNoCredentials(arg, 'console.log'));
        console.log(...args);
      },
      error: (...args: any[]) => {
        // Validate no credentials in error
        args.forEach(arg => self.validateNoCredentials(arg, 'console.error'));
        console.error(...args);
      },
      stringify: (obj: any) => {
        // Validate no credentials in stringified object
        self.validateNoCredentials(obj, 'JSON.stringify');
        return JSON.stringify(obj);
      }
    };
  }

  /**
   * Monitor for credential leaks in global scope
   */
  startLeakMonitoring(): void {
    if (typeof global !== 'undefined') {
      // Override console methods
      const originalLog = console.log;
      const originalError = console.error;
      const originalWarn = console.warn;

      console.log = (...args: any[]) => {
        try {
          args.forEach(arg => this.validateNoCredentials(arg, 'console.log'));
        } catch (error) {
          this.emit('leak-detected', { method: 'console.log', error });
          return;
        }
        originalLog.apply(console, args);
      };

      console.error = (...args: any[]) => {
        try {
          args.forEach(arg => this.validateNoCredentials(arg, 'console.error'));
        } catch (error) {
          this.emit('leak-detected', { method: 'console.error', error });
          return;
        }
        originalError.apply(console, args);
      };

      console.warn = (...args: any[]) => {
        try {
          args.forEach(arg => this.validateNoCredentials(arg, 'console.warn'));
        } catch (error) {
          this.emit('leak-detected', { method: 'console.warn', error });
          return;
        }
        originalWarn.apply(console, args);
      };
    }
  }

  /**
   * Set timer to clear credential from memory
   */
  private setMemoryClearTimer(id: string, credential: Credential): void {
    // Clear existing timer
    const existingTimer = this.memoryTimers.get(id);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Set new timer
    const timer = setTimeout(() => {
      this.clearFromMemory(credential);
      this.memoryTimers.delete(id);
      this.emit('memory-cleared', { credentialId: id });
    }, this.policy.maxMemoryRetention);

    this.memoryTimers.set(id, timer);
  }

  /**
   * Clear credential from memory
   */
  private clearFromMemory(credential: Credential): void {
    // Best effort to clear string from memory
    if (credential.apiKey) {
      // Overwrite with random data
      const length = credential.apiKey.length;
      credential.apiKey = crypto.randomBytes(length).toString('hex').substring(0, length);
      credential.apiKey = '';
    }

    // Clear other sensitive fields
    if (credential.metadata) {
      credential.metadata = {};
    }
  }

  /**
   * Sanitize credential for transmission
   */
  private sanitizeForTransmission(credential: Credential): Partial<Credential> {
    const { apiKey, ...sanitized } = credential;
    return {
      ...sanitized,
      id: credential.id,
      provider: credential.provider,
      name: credential.name
    };
  }
}

/**
 * Zero-knowledge proof utilities
 */
export class ZeroKnowledgeProof {
  /**
   * Generate proof of credential possession without revealing the credential
   */
  static generateProof(
    credentialId: string,
    challenge: string,
    apiKey: string
  ): string {
    const crypto = require('crypto');
    
    // Create HMAC of challenge using API key
    const hmac = crypto.createHmac('sha256', apiKey);
    hmac.update(challenge);
    hmac.update(credentialId);
    
    return hmac.digest('hex');
  }

  /**
   * Verify proof of credential possession
   */
  static verifyProof(
    credentialId: string,
    challenge: string,
    proof: string,
    apiKey: string
  ): boolean {
    const expectedProof = this.generateProof(credentialId, challenge, apiKey);
    
    // Constant-time comparison
    const crypto = require('crypto');
    return crypto.timingSafeEqual(
      Buffer.from(proof),
      Buffer.from(expectedProof)
    );
  }

  /**
   * Generate challenge for proof
   */
  static generateChallenge(): string {
    const crypto = require('crypto');
    return crypto.randomBytes(32).toString('hex');
  }
}