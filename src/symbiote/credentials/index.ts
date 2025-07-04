/**
 * Credential Management System
 * 
 * Zero-knowledge credential management for SymbioteIDE
 */

// Core exports
export * from './core/credential-types';
export { CredentialManager } from './core/credential-manager';
export { ValidationService, ValidationRateLimiter } from './core/validation-service';
export { AESEncryptionService, SecureString, SecureMemoryPool } from './core/encryption-service';

// Provider exports
export { BaseCredentialProvider } from './providers/base-provider';
export { WindowsCredentialProvider, WindowsCredentialProviderNative } from './providers/windows-credential-provider';
export { MacOSKeychainProvider, MacOSKeychainProviderNative } from './providers/macos-keychain-provider';
export { LinuxSecretProvider, LinuxSecretProviderNative } from './providers/linux-secret-provider';

// Security exports
export { ZeroKnowledgeVault, ZeroKnowledgeProof } from './security/zero-knowledge-vault';
export { SecureTransmission, SecureProxy } from './security/secure-transmission';
export { AuditLogger } from './security/audit-logger';
export { KeySanitizer, sanitizerMiddleware } from './security/key-sanitizer';

// Integration exports
export { CredentialOrchestrationIntegration, patchProviderAdapters } from './integration/orchestration-integration';

// Initialize global security features
import { KeySanitizer } from './security/key-sanitizer';

// Install global error handler to prevent credential leaks
if (process.env.NODE_ENV !== 'test') {
  KeySanitizer.installGlobalHandler();
}

/**
 * Create and configure credential manager with best practices
 */
export function createCredentialManager(options?: any): CredentialManager {
  const manager = new CredentialManager({
    enableRotationReminders: true,
    rotationIntervalDays: 90,
    enableAuditLogging: true,
    sessionTimeout: 300000, // 5 minutes
    maxCachedCredentials: 10,
    enableEnvFallback: true,
    enableSettingsFallback: false,
    ...options
  });

  // Add security monitoring
  const vault = new ZeroKnowledgeVault();
  vault.startLeakMonitoring();

  // Add audit logging
  const auditLogger = new AuditLogger({
    fileOutput: true,
    consoleOutput: false
  });

  // Connect events
  manager.on('credential:added', (event) => {
    auditLogger.logCredentialEvent(event);
  });

  manager.on('credential:updated', (event) => {
    auditLogger.logCredentialEvent(event);
  });

  manager.on('credential:deleted', (event) => {
    auditLogger.logCredentialEvent(event);
  });

  manager.on('credential:accessed', (event) => {
    auditLogger.logCredentialEvent(event);
  });

  return manager;
}

/**
 * Initialize credential system for orchestration engine
 */
export async function initializeCredentialSystem(): Promise<CredentialManager> {
  const manager = createCredentialManager();
  
  // Check platform availability
  const provider = manager['provider'];
  const isAvailable = await provider.isAvailable();
  
  if (!isAvailable) {
    console.warn(
      'Platform credential storage not available. ' +
      'Falling back to environment variables.'
    );
  }

  // Patch provider adapters to use credential manager
  patchProviderAdapters(manager);

  return manager;
}