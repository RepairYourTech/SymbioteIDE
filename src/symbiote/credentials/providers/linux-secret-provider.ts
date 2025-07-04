/**
 * Linux Secret Service Provider
 * 
 * Uses Secret Service API (GNOME Keyring, KWallet, etc.) for secure storage
 */

import { Credential } from '../core/credential-types';
import { BaseCredentialProvider } from './base-provider';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export class LinuxSecretProvider extends BaseCredentialProvider {
  private readonly collection = 'default';
  
  /**
   * Store credential using secret-tool
   */
  async store(credential: Credential): Promise<string> {
    try {
      const attributes = this.buildAttributes(credential.id);
      const metadata = this.serializeMetadata(credential);
      
      // Store API key with secret-tool
      const command = [
        'echo', '-n', `'${credential.apiKey}'`, '|',
        'secret-tool', 'store',
        '--label', `'SymbioteIDE: ${credential.name}'`,
        ...this.attributesToArgs(attributes)
      ].join(' ');
      
      await execAsync(command, { shell: '/bin/bash' });
      
      // Store metadata as a separate secret
      const metadataCommand = [
        'echo', '-n', `'${metadata}'`, '|',
        'secret-tool', 'store',
        '--label', `'SymbioteIDE Metadata: ${credential.name}'`,
        ...this.attributesToArgs({ ...attributes, type: 'metadata' })
      ].join(' ');
      
      await execAsync(metadataCommand, { shell: '/bin/bash' });
      
      this.logSecurityEvent('credential_stored', credential.id);
      return credential.id;
    } catch (error) {
      throw this.handleError(error, 'store');
    }
  }

  /**
   * Retrieve credential using secret-tool
   */
  async retrieve(id: string): Promise<Credential | null> {
    try {
      const attributes = this.buildAttributes(id);
      
      // Retrieve API key
      const keyCommand = [
        'secret-tool', 'lookup',
        ...this.attributesToArgs(attributes)
      ].join(' ');
      
      let apiKey: string;
      try {
        const { stdout } = await execAsync(keyCommand);
        apiKey = stdout.trim();
        
        if (!apiKey) {
          return null;
        }
      } catch (error) {
        // Secret not found
        return null;
      }
      
      // Retrieve metadata
      const metadataCommand = [
        'secret-tool', 'lookup',
        ...this.attributesToArgs({ ...attributes, type: 'metadata' })
      ].join(' ');
      
      let metadata: string;
      try {
        const { stdout } = await execAsync(metadataCommand);
        metadata = stdout.trim();
      } catch {
        // Metadata not found, return null
        return null;
      }
      
      const credential = this.deserializeMetadata(metadata, apiKey);
      this.logSecurityEvent('credential_retrieved', id);
      
      return credential;
    } catch (error) {
      throw this.handleError(error, 'retrieve');
    }
  }

  /**
   * Delete credential using secret-tool
   */
  async delete(id: string): Promise<boolean> {
    try {
      const attributes = this.buildAttributes(id);
      
      // Delete API key
      const keyCommand = [
        'secret-tool', 'clear',
        ...this.attributesToArgs(attributes)
      ].join(' ');
      
      await execAsync(keyCommand);
      
      // Delete metadata
      const metadataCommand = [
        'secret-tool', 'clear',
        ...this.attributesToArgs({ ...attributes, type: 'metadata' })
      ].join(' ');
      
      await execAsync(metadataCommand);
      
      this.logSecurityEvent('credential_deleted', id);
      return true;
    } catch (error) {
      // Check if not found
      if (error.code === 1) {
        return false;
      }
      throw this.handleError(error, 'delete');
    }
  }

  /**
   * List all credential IDs
   */
  async list(): Promise<string[]> {
    try {
      // Search for all secrets with our service attribute
      const command = [
        'secret-tool', 'search',
        'service', this.serviceName,
        'type', 'credential'
      ].join(' ');
      
      const { stdout } = await execAsync(command);
      const lines = stdout.split('\n');
      const credentialIds: string[] = [];
      
      for (const line of lines) {
        // Parse attribute lines
        const match = line.match(/attribute\.id\s*=\s*(.+)/);
        if (match) {
          credentialIds.push(match[1].trim());
        }
      }
      
      return [...new Set(credentialIds)]; // Remove duplicates
    } catch (error) {
      // If no secrets found, secret-tool returns error code 1
      if (error.code === 1) {
        return [];
      }
      throw this.handleError(error, 'list');
    }
  }

  /**
   * Check if Secret Service is available
   */
  async isAvailable(): Promise<boolean> {
    if (process.platform !== 'linux') {
      return false;
    }
    
    try {
      // Check if secret-tool is available
      await execAsync('which secret-tool');
      
      // Test if we can access the secret service
      await execAsync('secret-tool search --all service test-service 2>/dev/null || true');
      
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get platform name
   */
  getPlatformName(): string {
    return 'Linux Secret Service';
  }

  /**
   * Build attributes for secret-tool
   */
  private buildAttributes(id: string): Record<string, string> {
    return {
      service: this.serviceName,
      account: this.generateAccountName(id),
      id: id,
      type: 'credential'
    };
  }

  /**
   * Convert attributes to command line arguments
   */
  private attributesToArgs(attributes: Record<string, string>): string[] {
    const args: string[] = [];
    
    for (const [key, value] of Object.entries(attributes)) {
      args.push(key, `'${value}'`);
    }
    
    return args;
  }
}

/**
 * Alternative implementation using libsecret via keytar
 */
export class LinuxSecretProviderNative extends BaseCredentialProvider {
  private keytar: any;

  constructor() {
    super();
    try {
      this.keytar = require('keytar');
    } catch {
      // Keytar not available
    }
  }

  async store(credential: Credential): Promise<string> {
    if (!this.keytar) {
      throw new Error('Native secret storage not available');
    }

    try {
      const metadata = this.serializeMetadata(credential);
      
      // Store API key
      await this.keytar.setPassword(
        this.serviceName,
        this.generateAccountName(credential.id),
        credential.apiKey
      );
      
      // Store metadata
      await this.keytar.setPassword(
        this.serviceName + '_metadata',
        this.generateAccountName(credential.id),
        metadata
      );
      
      this.logSecurityEvent('credential_stored', credential.id);
      return credential.id;
    } catch (error) {
      throw this.handleError(error, 'store');
    }
  }

  async retrieve(id: string): Promise<Credential | null> {
    if (!this.keytar) {
      throw new Error('Native secret storage not available');
    }

    try {
      const accountName = this.generateAccountName(id);
      
      // Retrieve API key
      const apiKey = await this.keytar.getPassword(this.serviceName, accountName);
      if (!apiKey) {
        return null;
      }
      
      // Retrieve metadata
      const metadata = await this.keytar.getPassword(
        this.serviceName + '_metadata',
        accountName
      );
      
      if (!metadata) {
        return null;
      }
      
      const credential = this.deserializeMetadata(metadata, apiKey);
      this.logSecurityEvent('credential_retrieved', id);
      
      return credential;
    } catch (error) {
      throw this.handleError(error, 'retrieve');
    }
  }

  async delete(id: string): Promise<boolean> {
    if (!this.keytar) {
      throw new Error('Native secret storage not available');
    }

    try {
      const accountName = this.generateAccountName(id);
      
      // Delete API key
      const deleted1 = await this.keytar.deletePassword(
        this.serviceName,
        accountName
      );
      
      // Delete metadata
      const deleted2 = await this.keytar.deletePassword(
        this.serviceName + '_metadata',
        accountName
      );
      
      if (deleted1 || deleted2) {
        this.logSecurityEvent('credential_deleted', id);
        return true;
      }
      
      return false;
    } catch (error) {
      throw this.handleError(error, 'delete');
    }
  }

  async list(): Promise<string[]> {
    if (!this.keytar) {
      throw new Error('Native secret storage not available');
    }

    try {
      const credentials = await this.keytar.findCredentials(this.serviceName);
      const ids: string[] = [];
      
      for (const cred of credentials) {
        const id = this.parseAccountName(cred.account);
        if (id) {
          ids.push(id);
        }
      }
      
      return ids;
    } catch (error) {
      throw this.handleError(error, 'list');
    }
  }

  async isAvailable(): Promise<boolean> {
    return process.platform === 'linux' && !!this.keytar;
  }

  getPlatformName(): string {
    return 'Linux Secret Service (Native)';
  }
}

/**
 * Fallback provider using encrypted file storage
 * Only used when Secret Service is not available
 */
export class LinuxFileFallbackProvider extends BaseCredentialProvider {
  private readonly storageDir: string;
  private readonly crypto: any;

  constructor() {
    super();
    const os = require('os');
    const path = require('path');
    this.storageDir = path.join(os.homedir(), '.config', 'symbiote-ide', 'credentials');
    this.crypto = require('crypto');
  }

  async store(credential: Credential): Promise<string> {
    throw new Error('File-based storage is not secure. Please install gnome-keyring or similar.');
  }

  async retrieve(id: string): Promise<Credential | null> {
    throw new Error('File-based storage is not secure. Please install gnome-keyring or similar.');
  }

  async delete(id: string): Promise<boolean> {
    throw new Error('File-based storage is not secure. Please install gnome-keyring or similar.');
  }

  async list(): Promise<string[]> {
    return [];
  }

  async isAvailable(): Promise<boolean> {
    return false; // Never use file fallback for security reasons
  }

  getPlatformName(): string {
    return 'Linux File Fallback (Disabled)';
  }
}