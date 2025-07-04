/**
 * Credential Types and Interfaces
 */

export enum ProviderType {
  Anthropic = 'anthropic',
  OpenAI = 'openai',
  Google = 'google',
  Mistral = 'mistral',
  Azure = 'azure',
  AWS = 'aws',
  Custom = 'custom'
}

export interface Credential {
  id: string;
  provider: ProviderType;
  name: string;
  apiKey: string;
  createdAt: Date;
  lastUsed?: Date;
  lastRotated?: Date;
  expiresAt?: Date;
  metadata?: CredentialMetadata;
}

export interface CredentialMetadata {
  environment?: string;
  project?: string;
  tags?: string[];
  customFields?: Record<string, any>;
}

export interface SecureCredential extends Omit<Credential, 'apiKey'> {
  maskedKey: string;
  keyId: string; // Reference to key in OS keychain
}

export interface CredentialCreateRequest {
  provider: ProviderType;
  name: string;
  apiKey: string;
  metadata?: CredentialMetadata;
  expiresAt?: Date;
}

export interface CredentialUpdateRequest {
  name?: string;
  metadata?: CredentialMetadata;
  expiresAt?: Date;
}

export interface CredentialValidationResult {
  valid: boolean;
  error?: string;
  providerInfo?: {
    accountId?: string;
    accountName?: string;
    quotaRemaining?: number;
    capabilities?: string[];
  };
}

export interface CredentialProvider {
  /**
   * Store a credential securely
   */
  store(credential: Credential): Promise<string>;
  
  /**
   * Retrieve a credential by ID
   */
  retrieve(id: string): Promise<Credential | null>;
  
  /**
   * Delete a credential
   */
  delete(id: string): Promise<boolean>;
  
  /**
   * List all credential IDs
   */
  list(): Promise<string[]>;
  
  /**
   * Check if provider is available on this platform
   */
  isAvailable(): Promise<boolean>;
}

export interface CredentialValidator {
  /**
   * Validate API key format
   */
  validateFormat(apiKey: string): boolean;
  
  /**
   * Validate API key with provider (test call)
   */
  validateWithProvider(apiKey: string): Promise<CredentialValidationResult>;
  
  /**
   * Get provider type from key format
   */
  detectProvider(apiKey: string): ProviderType | null;
}

export interface EncryptionService {
  /**
   * Encrypt data
   */
  encrypt(data: string): Promise<{ encrypted: string; iv: string }>;
  
  /**
   * Decrypt data
   */
  decrypt(encrypted: string, iv: string): Promise<string>;
  
  /**
   * Generate secure random key
   */
  generateKey(): Promise<string>;
  
  /**
   * Securely clear sensitive data from memory
   */
  secureClear(data: string): void;
}

export interface CredentialEventType {
  CREDENTIAL_ADDED: 'credential:added';
  CREDENTIAL_UPDATED: 'credential:updated';
  CREDENTIAL_DELETED: 'credential:deleted';
  CREDENTIAL_ACCESSED: 'credential:accessed';
  CREDENTIAL_VALIDATED: 'credential:validated';
  CREDENTIAL_ROTATION_NEEDED: 'credential:rotation_needed';
  CREDENTIAL_EXPIRED: 'credential:expired';
}

export const CredentialEvents: CredentialEventType = {
  CREDENTIAL_ADDED: 'credential:added',
  CREDENTIAL_UPDATED: 'credential:updated',
  CREDENTIAL_DELETED: 'credential:deleted',
  CREDENTIAL_ACCESSED: 'credential:accessed',
  CREDENTIAL_VALIDATED: 'credential:validated',
  CREDENTIAL_ROTATION_NEEDED: 'credential:rotation_needed',
  CREDENTIAL_EXPIRED: 'credential:expired'
};

export interface CredentialEvent {
  type: keyof CredentialEventType;
  credentialId: string;
  timestamp: Date;
  metadata?: Record<string, any>;
}

export interface CredentialManagerOptions {
  /**
   * Enable automatic key rotation reminders
   */
  enableRotationReminders?: boolean;
  
  /**
   * Key rotation interval in days
   */
  rotationIntervalDays?: number;
  
  /**
   * Enable audit logging
   */
  enableAuditLogging?: boolean;
  
  /**
   * Session timeout for cached credentials (ms)
   */
  sessionTimeout?: number;
  
  /**
   * Maximum number of credentials to cache
   */
  maxCachedCredentials?: number;
  
  /**
   * Enable environment variable fallback
   */
  enableEnvFallback?: boolean;
  
  /**
   * Enable VS Code settings fallback (deprecated)
   */
  enableSettingsFallback?: boolean;
}