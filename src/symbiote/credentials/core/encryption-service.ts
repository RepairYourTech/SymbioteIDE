/**
 * Encryption Service
 * 
 * Provides additional encryption layer for sensitive data
 */

import * as crypto from 'crypto';
import { EncryptionService } from './credential-types';

export class AESEncryptionService implements EncryptionService {
  private readonly algorithm = 'aes-256-gcm';
  private readonly keyLength = 32; // 256 bits
  private readonly ivLength = 16; // 128 bits
  private readonly tagLength = 16; // 128 bits
  private readonly saltLength = 32; // 256 bits
  private readonly iterations = 100000; // PBKDF2 iterations
  
  private masterKey?: Buffer;

  constructor(private readonly masterPassword?: string) {
    if (masterPassword) {
      // Derive master key from password
      const salt = crypto.randomBytes(this.saltLength);
      this.masterKey = crypto.pbkdf2Sync(
        masterPassword,
        salt,
        this.iterations,
        this.keyLength,
        'sha256'
      );
    }
  }

  /**
   * Encrypt data using AES-256-GCM
   */
  async encrypt(data: string): Promise<{ encrypted: string; iv: string }> {
    try {
      // Generate random IV
      const iv = crypto.randomBytes(this.ivLength);
      
      // Get encryption key
      const key = await this.getEncryptionKey();
      
      // Create cipher
      const cipher = crypto.createCipheriv(this.algorithm, key, iv);
      
      // Encrypt data
      const encrypted = Buffer.concat([
        cipher.update(data, 'utf8'),
        cipher.final()
      ]);
      
      // Get authentication tag
      const tag = cipher.getAuthTag();
      
      // Combine encrypted data and tag
      const combined = Buffer.concat([encrypted, tag]);
      
      // Clear sensitive data from memory
      this.secureClear(data);
      
      return {
        encrypted: combined.toString('base64'),
        iv: iv.toString('base64')
      };
    } catch (error) {
      throw new Error(`Encryption failed: ${error.message}`);
    }
  }

  /**
   * Decrypt data using AES-256-GCM
   */
  async decrypt(encrypted: string, iv: string): Promise<string> {
    try {
      // Decode from base64
      const combined = Buffer.from(encrypted, 'base64');
      const ivBuffer = Buffer.from(iv, 'base64');
      
      // Split encrypted data and tag
      const encryptedData = combined.slice(0, -this.tagLength);
      const tag = combined.slice(-this.tagLength);
      
      // Get decryption key
      const key = await this.getEncryptionKey();
      
      // Create decipher
      const decipher = crypto.createDecipheriv(this.algorithm, key, ivBuffer);
      decipher.setAuthTag(tag);
      
      // Decrypt data
      const decrypted = Buffer.concat([
        decipher.update(encryptedData),
        decipher.final()
      ]);
      
      return decrypted.toString('utf8');
    } catch (error) {
      throw new Error(`Decryption failed: ${error.message}`);
    }
  }

  /**
   * Generate a secure random key
   */
  async generateKey(): Promise<string> {
    const key = crypto.randomBytes(this.keyLength);
    return key.toString('base64');
  }

  /**
   * Securely clear sensitive data from memory
   * Note: This is best-effort in JavaScript/Node.js
   */
  secureClear(data: string): void {
    if (!data) return;
    
    // Overwrite string content (best effort in JS)
    try {
      // For Buffer objects
      if (Buffer.isBuffer(data)) {
        data.fill(0);
      }
      // For strings, we can't directly modify them
      // This is a limitation of JavaScript
      // The garbage collector will eventually clear the memory
    } catch {
      // Ignore errors in secure clear
    }
  }

  /**
   * Get or generate encryption key
   */
  private async getEncryptionKey(): Promise<Buffer> {
    if (this.masterKey) {
      return this.masterKey;
    }
    
    // Generate a new key if no master key
    // In production, this should be derived from a secure source
    const key = crypto.randomBytes(this.keyLength);
    return key;
  }

  /**
   * Derive key from password (for future use)
   */
  static deriveKeyFromPassword(
    password: string,
    salt: Buffer,
    iterations: number = 100000
  ): Buffer {
    return crypto.pbkdf2Sync(password, salt, iterations, 32, 'sha256');
  }

  /**
   * Generate random salt
   */
  static generateSalt(): Buffer {
    return crypto.randomBytes(32);
  }

  /**
   * Constant-time string comparison
   */
  static constantTimeCompare(a: string, b: string): boolean {
    if (a.length !== b.length) {
      return false;
    }
    
    const bufferA = Buffer.from(a);
    const bufferB = Buffer.from(b);
    
    return crypto.timingSafeEqual(bufferA, bufferB);
  }

  /**
   * Hash API key for comparison/storage
   */
  static hashApiKey(apiKey: string): string {
    const hash = crypto.createHash('sha256');
    hash.update(apiKey);
    return hash.digest('base64');
  }

  /**
   * Generate secure random string
   */
  static generateSecureRandom(length: number = 32): string {
    return crypto.randomBytes(length).toString('base64url');
  }
}

/**
 * Memory-safe string handling utilities
 */
export class SecureString {
  private buffer: Buffer;
  
  constructor(data: string) {
    this.buffer = Buffer.from(data, 'utf8');
  }
  
  /**
   * Get the string value
   */
  toString(): string {
    return this.buffer.toString('utf8');
  }
  
  /**
   * Clear the string from memory
   */
  clear(): void {
    this.buffer.fill(0);
  }
  
  /**
   * Get length
   */
  get length(): number {
    return this.buffer.length;
  }
}

/**
 * Secure memory pool for temporary sensitive data
 */
export class SecureMemoryPool {
  private pool: Map<string, SecureString> = new Map();
  
  /**
   * Store data in secure pool
   */
  store(key: string, data: string): void {
    // Clear existing data if present
    this.clear(key);
    
    // Store new data
    this.pool.set(key, new SecureString(data));
  }
  
  /**
   * Retrieve data from pool
   */
  retrieve(key: string): string | null {
    const secure = this.pool.get(key);
    return secure ? secure.toString() : null;
  }
  
  /**
   * Clear specific data
   */
  clear(key: string): void {
    const secure = this.pool.get(key);
    if (secure) {
      secure.clear();
      this.pool.delete(key);
    }
  }
  
  /**
   * Clear all data
   */
  clearAll(): void {
    for (const [key, secure] of this.pool) {
      secure.clear();
    }
    this.pool.clear();
  }
}