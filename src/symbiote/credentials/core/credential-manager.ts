/**
 * Credential Manager
 * 
 * Main service for managing API credentials with zero-knowledge architecture
 */

import { EventEmitter } from 'events';
import * as crypto from 'crypto';
import {
  Credential,
  SecureCredential,
  CredentialCreateRequest,
  CredentialUpdateRequest,
  CredentialProvider,
  ProviderType,
  CredentialManagerOptions,
  CredentialEvents,
  CredentialEvent
} from './credential-types';
import { ValidationService, ValidationRateLimiter } from './validation-service';
import { AESEncryptionService, SecureMemoryPool } from './encryption-service';
import { BaseCredentialProvider } from '../providers/base-provider';
import { WindowsCredentialProvider } from '../providers/windows-credential-provider';
import { MacOSKeychainProvider } from '../providers/macos-keychain-provider';
import { LinuxSecretProvider } from '../providers/linux-secret-provider';

export class CredentialManager extends EventEmitter {
  private provider: CredentialProvider;
  private validationService: ValidationService;
  private rateLimiter: ValidationRateLimiter;
  private encryptionService: AESEncryptionService;
  private memoryPool: SecureMemoryPool;
  private sessionCache: Map<string, { credential: Credential; expires: number }>;
  private options: Required<CredentialManagerOptions>;

  constructor(options: CredentialManagerOptions = {}) {
    super();
    
    this.options = {
      enableRotationReminders: true,
      rotationIntervalDays: 90,
      enableAuditLogging: true,
      sessionTimeout: 300000, // 5 minutes
      maxCachedCredentials: 10,
      enableEnvFallback: true,
      enableSettingsFallback: false,
      ...options
    };

    this.validationService = new ValidationService();
    this.rateLimiter = new ValidationRateLimiter();
    this.encryptionService = new AESEncryptionService();
    this.memoryPool = new SecureMemoryPool();
    this.sessionCache = new Map();

    // Initialize platform-specific provider
    this.provider = this.createProvider();
    
    // Start session cleanup interval
    setInterval(() => this.cleanupSessions(), 60000); // Every minute
    
    // Start rotation check interval
    if (this.options.enableRotationReminders) {
      setInterval(() => this.checkRotationReminders(), 86400000); // Daily
    }
  }

  /**
   * Create a new credential
   */
  async createCredential(request: CredentialCreateRequest): Promise<SecureCredential> {
    try {
      // Validate input
      if (!request.apiKey || !request.provider || !request.name) {
        throw new Error('Missing required fields');
      }

      // Validate API key format
      if (!this.validationService.validateFormat(request.provider, request.apiKey)) {
        throw new Error('Invalid API key format for provider');
      }

      // Rate limit validation requests
      if (!this.rateLimiter.canValidate(request.apiKey)) {
        throw new Error('Too many validation attempts. Please try again later.');
      }

      // Validate with provider if online
      const validation = await this.validationService.validateWithProvider(
        request.provider,
        request.apiKey
      );

      if (!validation.valid) {
        throw new Error(`API key validation failed: ${validation.error}`);
      }

      // Generate credential ID
      const id = this.generateCredentialId();

      // Create credential object
      const credential: Credential = {
        id,
        provider: request.provider,
        name: request.name,
        apiKey: request.apiKey,
        createdAt: new Date(),
        lastRotated: new Date(),
        expiresAt: request.expiresAt,
        metadata: {
          ...request.metadata,
          providerInfo: validation.providerInfo
        }
      };

      // Store in OS keychain
      await this.provider.store(credential);

      // Clear API key from memory
      this.memoryPool.clear(request.apiKey);

      // Emit event
      this.emitCredentialEvent(CredentialEvents.CREDENTIAL_ADDED, id, {
        provider: request.provider,
        name: request.name
      });

      // Return secure credential (without API key)
      return this.toSecureCredential(credential);
    } catch (error) {
      this.logError('createCredential', error);
      throw error;
    }
  }

  /**
   * Retrieve a credential by ID
   */
  async getCredential(id: string): Promise<SecureCredential | null> {
    try {
      // Check session cache first
      const cached = this.sessionCache.get(id);
      if (cached && cached.expires > Date.now()) {
        this.emitCredentialEvent(CredentialEvents.CREDENTIAL_ACCESSED, id, {
          source: 'cache'
        });
        return this.toSecureCredential(cached.credential);
      }

      // Retrieve from provider
      const credential = await this.provider.retrieve(id);
      if (!credential) {
        return null;
      }

      // Update last used
      credential.lastUsed = new Date();
      await this.provider.store(credential);

      // Cache in session
      this.cacheCredential(credential);

      // Emit event
      this.emitCredentialEvent(CredentialEvents.CREDENTIAL_ACCESSED, id, {
        source: 'keychain'
      });

      return this.toSecureCredential(credential);
    } catch (error) {
      this.logError('getCredential', error);
      throw error;
    }
  }

  /**
   * Get credential for use (returns actual API key)
   * This should be used sparingly and the key should be cleared after use
   */
  async useCredential(id: string): Promise<Credential | null> {
    try {
      // Check session cache first
      const cached = this.sessionCache.get(id);
      if (cached && cached.expires > Date.now()) {
        return cached.credential;
      }

      // Retrieve from provider
      const credential = await this.provider.retrieve(id);
      if (!credential) {
        // Try fallback sources
        return this.getCredentialFromFallback(id);
      }

      // Update last used
      credential.lastUsed = new Date();
      await this.provider.store(credential);

      // Cache in session
      this.cacheCredential(credential);

      // Check if rotation is needed
      this.checkRotationNeeded(credential);

      return credential;
    } catch (error) {
      this.logError('useCredential', error);
      throw error;
    }
  }

  /**
   * Update a credential
   */
  async updateCredential(
    id: string, 
    update: CredentialUpdateRequest
  ): Promise<SecureCredential | null> {
    try {
      const credential = await this.provider.retrieve(id);
      if (!credential) {
        return null;
      }

      // Update fields
      if (update.name !== undefined) {
        credential.name = update.name;
      }
      if (update.metadata !== undefined) {
        credential.metadata = { ...credential.metadata, ...update.metadata };
      }
      if (update.expiresAt !== undefined) {
        credential.expiresAt = update.expiresAt;
      }

      // Store updated credential
      await this.provider.store(credential);

      // Clear from cache
      this.sessionCache.delete(id);

      // Emit event
      this.emitCredentialEvent(CredentialEvents.CREDENTIAL_UPDATED, id, update);

      return this.toSecureCredential(credential);
    } catch (error) {
      this.logError('updateCredential', error);
      throw error;
    }
  }

  /**
   * Rotate a credential (update API key)
   */
  async rotateCredential(id: string, newApiKey: string): Promise<SecureCredential | null> {
    try {
      const credential = await this.provider.retrieve(id);
      if (!credential) {
        return null;
      }

      // Validate new API key
      if (!this.validationService.validateFormat(credential.provider, newApiKey)) {
        throw new Error('Invalid API key format for provider');
      }

      // Validate with provider
      const validation = await this.validationService.validateWithProvider(
        credential.provider,
        newApiKey
      );

      if (!validation.valid) {
        throw new Error(`API key validation failed: ${validation.error}`);
      }

      // Update credential
      credential.apiKey = newApiKey;
      credential.lastRotated = new Date();

      // Store updated credential
      await this.provider.store(credential);

      // Clear from cache
      this.sessionCache.delete(id);

      // Clear old key from memory
      this.memoryPool.clear(newApiKey);

      // Emit event
      this.emitCredentialEvent(CredentialEvents.CREDENTIAL_UPDATED, id, {
        rotated: true
      });

      return this.toSecureCredential(credential);
    } catch (error) {
      this.logError('rotateCredential', error);
      throw error;
    }
  }

  /**
   * Delete a credential
   */
  async deleteCredential(id: string): Promise<boolean> {
    try {
      // Clear from cache
      this.sessionCache.delete(id);

      // Delete from provider
      const deleted = await this.provider.delete(id);

      if (deleted) {
        // Emit event
        this.emitCredentialEvent(CredentialEvents.CREDENTIAL_DELETED, id);
      }

      return deleted;
    } catch (error) {
      this.logError('deleteCredential', error);
      throw error;
    }
  }

  /**
   * List all credentials
   */
  async listCredentials(): Promise<SecureCredential[]> {
    try {
      const ids = await this.provider.list();
      const credentials: SecureCredential[] = [];

      for (const id of ids) {
        const credential = await this.getCredential(id);
        if (credential) {
          credentials.push(credential);
        }
      }

      return credentials;
    } catch (error) {
      this.logError('listCredentials', error);
      throw error;
    }
  }

  /**
   * Get credentials by provider
   */
  async getCredentialsByProvider(provider: ProviderType): Promise<SecureCredential[]> {
    const allCredentials = await this.listCredentials();
    return allCredentials.filter(c => c.provider === provider);
  }

  /**
   * Clear session cache
   */
  clearCache(): void {
    // Clear sensitive data from memory
    for (const [id, cached] of this.sessionCache) {
      this.memoryPool.clear(cached.credential.apiKey);
    }
    this.sessionCache.clear();
  }

  /**
   * Create platform-specific provider
   */
  private createProvider(): CredentialProvider {
    const platform = process.platform;

    switch (platform) {
      case 'win32':
        return new WindowsCredentialProvider();
      case 'darwin':
        return new MacOSKeychainProvider();
      case 'linux':
        return new LinuxSecretProvider();
      default:
        throw new Error(`Unsupported platform: ${platform}`);
    }
  }

  /**
   * Generate unique credential ID
   */
  private generateCredentialId(): string {
    return crypto.randomBytes(16).toString('hex');
  }

  /**
   * Convert to secure credential
   */
  private toSecureCredential(credential: Credential): SecureCredential {
    const { apiKey, ...rest } = credential;
    return {
      ...rest,
      maskedKey: this.maskApiKey(apiKey),
      keyId: credential.id
    };
  }

  /**
   * Mask API key for display
   */
  private maskApiKey(apiKey: string): string {
    if (!apiKey || apiKey.length < 8) {
      return '****';
    }
    const visibleChars = 4;
    const maskedSection = '*'.repeat(apiKey.length - visibleChars);
    return maskedSection + apiKey.slice(-visibleChars);
  }

  /**
   * Cache credential in session
   */
  private cacheCredential(credential: Credential): void {
    // Limit cache size
    if (this.sessionCache.size >= this.options.maxCachedCredentials) {
      const firstKey = this.sessionCache.keys().next().value;
      if (firstKey) {
        const cached = this.sessionCache.get(firstKey);
        if (cached) {
          this.memoryPool.clear(cached.credential.apiKey);
        }
        this.sessionCache.delete(firstKey);
      }
    }

    // Store in secure memory pool
    this.memoryPool.store(credential.apiKey, credential.apiKey);

    // Add to cache
    this.sessionCache.set(credential.id, {
      credential,
      expires: Date.now() + this.options.sessionTimeout
    });
  }

  /**
   * Clean up expired sessions
   */
  private cleanupSessions(): void {
    const now = Date.now();
    const expired: string[] = [];

    for (const [id, cached] of this.sessionCache) {
      if (cached.expires <= now) {
        expired.push(id);
        this.memoryPool.clear(cached.credential.apiKey);
      }
    }

    for (const id of expired) {
      this.sessionCache.delete(id);
    }
  }

  /**
   * Check if rotation is needed
   */
  private checkRotationNeeded(credential: Credential): void {
    if (!this.options.enableRotationReminders) {
      return;
    }

    const rotationInterval = this.options.rotationIntervalDays * 24 * 60 * 60 * 1000;
    const timeSinceRotation = Date.now() - credential.lastRotated.getTime();

    if (timeSinceRotation > rotationInterval) {
      this.emitCredentialEvent(CredentialEvents.CREDENTIAL_ROTATION_NEEDED, credential.id, {
        lastRotated: credential.lastRotated,
        daysSinceRotation: Math.floor(timeSinceRotation / (24 * 60 * 60 * 1000))
      });
    }
  }

  /**
   * Check all credentials for rotation reminders
   */
  private async checkRotationReminders(): Promise<void> {
    try {
      const credentials = await this.listCredentials();
      
      for (const secureCredential of credentials) {
        const credential = await this.provider.retrieve(secureCredential.id);
        if (credential) {
          this.checkRotationNeeded(credential);
        }
      }
    } catch (error) {
      this.logError('checkRotationReminders', error);
    }
  }

  /**
   * Get credential from fallback sources
   */
  private async getCredentialFromFallback(id: string): Promise<Credential | null> {
    // Try environment variables
    if (this.options.enableEnvFallback) {
      const envKey = this.getEnvKeyForProvider(id);
      if (envKey && process.env[envKey]) {
        return {
          id,
          provider: this.detectProviderFromId(id),
          name: `${id} (from environment)`,
          apiKey: process.env[envKey]!,
          createdAt: new Date(),
          metadata: { source: 'environment' }
        };
      }
    }

    // Try VS Code settings (deprecated)
    if (this.options.enableSettingsFallback) {
      // This would integrate with VS Code settings API
      // Not implemented here as it requires VS Code extension context
    }

    return null;
  }

  /**
   * Get environment variable key for provider
   */
  private getEnvKeyForProvider(id: string): string | null {
    // This is a simplified mapping
    // In practice, you'd have a more sophisticated mapping
    const providerEnvMap: Record<string, string> = {
      anthropic: 'ANTHROPIC_API_KEY',
      openai: 'OPENAI_API_KEY',
      google: 'GOOGLE_API_KEY',
      mistral: 'MISTRAL_API_KEY'
    };

    for (const [provider, envKey] of Object.entries(providerEnvMap)) {
      if (id.includes(provider)) {
        return envKey;
      }
    }

    return null;
  }

  /**
   * Detect provider from ID
   */
  private detectProviderFromId(id: string): ProviderType {
    // Simple detection based on ID
    if (id.includes('anthropic')) return ProviderType.Anthropic;
    if (id.includes('openai')) return ProviderType.OpenAI;
    if (id.includes('google')) return ProviderType.Google;
    if (id.includes('mistral')) return ProviderType.Mistral;
    return ProviderType.Custom;
  }

  /**
   * Emit credential event
   */
  private emitCredentialEvent(
    type: keyof typeof CredentialEvents,
    credentialId: string,
    metadata?: any
  ): void {
    const event: CredentialEvent = {
      type,
      credentialId,
      timestamp: new Date(),
      metadata
    };

    this.emit(type, event);

    if (this.options.enableAuditLogging) {
      this.logAuditEvent(event);
    }
  }

  /**
   * Log audit event
   */
  private logAuditEvent(event: CredentialEvent): void {
    // In production, this would write to a secure audit log
    // For now, just emit an event
    this.emit('audit', {
      ...event,
      // Never log sensitive data
      metadata: this.sanitizeMetadata(event.metadata)
    });
  }

  /**
   * Sanitize metadata for logging
   */
  private sanitizeMetadata(metadata: any): any {
    if (!metadata) return metadata;

    const sanitized = { ...metadata };
    
    // Remove any potentially sensitive fields
    delete sanitized.apiKey;
    delete sanitized.password;
    delete sanitized.secret;
    delete sanitized.token;

    return sanitized;
  }

  /**
   * Log error
   */
  private logError(operation: string, error: any): void {
    this.emit('error', {
      operation,
      error: error.message || error,
      timestamp: new Date()
    });
  }
}