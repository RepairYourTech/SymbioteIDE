/**
 * Base Credential Provider
 * 
 * Abstract base class for platform-specific credential storage
 */

import { Credential, CredentialProvider, SecureCredential } from '../core/credential-types';
import { EventEmitter } from 'events';

export abstract class BaseCredentialProvider extends EventEmitter implements CredentialProvider {
  protected readonly serviceName: string = 'SymbioteIDE';
  protected readonly accountPrefix: string = 'symbiote_credential_';

  constructor() {
    super();
  }

  /**
   * Store a credential securely in the OS keychain
   */
  abstract store(credential: Credential): Promise<string>;

  /**
   * Retrieve a credential by ID from the OS keychain
   */
  abstract retrieve(id: string): Promise<Credential | null>;

  /**
   * Delete a credential from the OS keychain
   */
  abstract delete(id: string): Promise<boolean>;

  /**
   * List all credential IDs
   */
  abstract list(): Promise<string[]>;

  /**
   * Check if this provider is available on the current platform
   */
  abstract isAvailable(): Promise<boolean>;

  /**
   * Get platform name
   */
  abstract getPlatformName(): string;

  /**
   * Generate a unique account name for keychain storage
   */
  protected generateAccountName(id: string): string {
    return `${this.accountPrefix}${id}`;
  }

  /**
   * Parse account name to extract credential ID
   */
  protected parseAccountName(accountName: string): string | null {
    if (accountName.startsWith(this.accountPrefix)) {
      return accountName.substring(this.accountPrefix.length);
    }
    return null;
  }

  /**
   * Mask an API key for display
   */
  protected maskApiKey(apiKey: string): string {
    if (!apiKey || apiKey.length < 8) {
      return '****';
    }
    const visibleChars = 4;
    const maskedSection = '*'.repeat(apiKey.length - visibleChars);
    return maskedSection + apiKey.slice(-visibleChars);
  }

  /**
   * Convert credential to secure credential (without exposing API key)
   */
  protected toSecureCredential(credential: Credential, keyId: string): SecureCredential {
    const { apiKey, ...rest } = credential;
    return {
      ...rest,
      maskedKey: this.maskApiKey(apiKey),
      keyId
    };
  }

  /**
   * Serialize credential metadata for storage
   */
  protected serializeMetadata(credential: Credential): string {
    const metadata = {
      id: credential.id,
      provider: credential.provider,
      name: credential.name,
      createdAt: credential.createdAt.toISOString(),
      lastUsed: credential.lastUsed?.toISOString(),
      lastRotated: credential.lastRotated?.toISOString(),
      expiresAt: credential.expiresAt?.toISOString(),
      metadata: credential.metadata
    };
    return JSON.stringify(metadata);
  }

  /**
   * Deserialize credential metadata from storage
   */
  protected deserializeMetadata(data: string, apiKey: string): Credential {
    const parsed = JSON.parse(data);
    return {
      id: parsed.id,
      provider: parsed.provider,
      name: parsed.name,
      apiKey,
      createdAt: new Date(parsed.createdAt),
      lastUsed: parsed.lastUsed ? new Date(parsed.lastUsed) : undefined,
      lastRotated: parsed.lastRotated ? new Date(parsed.lastRotated) : undefined,
      expiresAt: parsed.expiresAt ? new Date(parsed.expiresAt) : undefined,
      metadata: parsed.metadata
    };
  }

  /**
   * Log security event (without exposing sensitive data)
   */
  protected logSecurityEvent(event: string, credentialId: string, details?: any): void {
    this.emit('security-event', {
      event,
      credentialId,
      timestamp: new Date(),
      platform: this.getPlatformName(),
      details
    });
  }

  /**
   * Handle provider errors
   */
  protected handleError(error: any, operation: string): Error {
    const message = `Credential provider error during ${operation}: ${error.message || error}`;
    const providerError = new Error(message);
    (providerError as any).originalError = error;
    (providerError as any).provider = this.getPlatformName();
    (providerError as any).operation = operation;
    return providerError;
  }
}