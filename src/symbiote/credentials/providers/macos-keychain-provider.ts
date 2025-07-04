/**
 * macOS Keychain Provider
 * 
 * Uses macOS Keychain Services for secure storage
 */

import { Credential } from '../core/credential-types';
import { BaseCredentialProvider } from './base-provider';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export class MacOSKeychainProvider extends BaseCredentialProvider {
  /**
   * Store credential in macOS Keychain
   */
  async store(credential: Credential): Promise<string> {
    try {
      const accountName = this.generateAccountName(credential.id);
      const metadata = this.serializeMetadata(credential);
      
      // Delete existing credential if it exists (security add-generic-password doesn't update)
      await this.deleteFromKeychain(accountName);
      
      // Store credential using security command
      const command = [
        'security',
        'add-generic-password',
        '-a', accountName,
        '-s', this.serviceName,
        '-w', credential.apiKey,
        '-D', 'SymbioteIDE Credential',
        '-j', Buffer.from(metadata).toString('base64'),
        '-U', // Update if exists
        '-T', '' // Allow access by current application
      ].map(arg => this.escapeShellArg(arg)).join(' ');
      
      await execAsync(command);
      
      this.logSecurityEvent('credential_stored', credential.id);
      return credential.id;
    } catch (error) {
      throw this.handleError(error, 'store');
    }
  }

  /**
   * Retrieve credential from macOS Keychain
   */
  async retrieve(id: string): Promise<Credential | null> {
    try {
      const accountName = this.generateAccountName(id);
      
      // Retrieve password
      const passwordCommand = [
        'security',
        'find-generic-password',
        '-a', accountName,
        '-s', this.serviceName,
        '-w' // Output password only
      ].map(arg => this.escapeShellArg(arg)).join(' ');
      
      let apiKey: string;
      try {
        const { stdout: password } = await execAsync(passwordCommand);
        apiKey = password.trim();
      } catch (error) {
        if (error.code === 44) { // Item not found
          return null;
        }
        throw error;
      }
      
      // Retrieve metadata from comment field
      const metadataCommand = [
        'security',
        'find-generic-password',
        '-a', accountName,
        '-s', this.serviceName
      ].map(arg => this.escapeShellArg(arg)).join(' ');
      
      const { stdout: output } = await execAsync(metadataCommand);
      
      // Parse comment field containing metadata
      const commentMatch = output.match(/"icmt"<blob>="([^"]+)"/);
      if (!commentMatch) {
        return null;
      }
      
      const metadata = Buffer.from(commentMatch[1], 'base64').toString('utf8');
      const credential = this.deserializeMetadata(metadata, apiKey);
      
      this.logSecurityEvent('credential_retrieved', id);
      return credential;
    } catch (error) {
      if (error.code === 44) { // Item not found
        return null;
      }
      throw this.handleError(error, 'retrieve');
    }
  }

  /**
   * Delete credential from macOS Keychain
   */
  async delete(id: string): Promise<boolean> {
    try {
      const accountName = this.generateAccountName(id);
      const deleted = await this.deleteFromKeychain(accountName);
      
      if (deleted) {
        this.logSecurityEvent('credential_deleted', id);
      }
      
      return deleted;
    } catch (error) {
      throw this.handleError(error, 'delete');
    }
  }

  /**
   * List all credential IDs
   */
  async list(): Promise<string[]> {
    try {
      // Search for all passwords for this service
      const command = [
        'security',
        'dump-keychain'
      ].join(' ');
      
      const { stdout } = await execAsync(command);
      const lines = stdout.split('\n');
      const credentialIds: string[] = [];
      
      let currentAccount: string | null = null;
      let currentService: string | null = null;
      
      for (const line of lines) {
        // Parse account
        const accountMatch = line.match(/"acct"<blob>="([^"]+)"/);
        if (accountMatch) {
          currentAccount = accountMatch[1];
        }
        
        // Parse service
        const serviceMatch = line.match(/"svce"<blob>="([^"]+)"/);
        if (serviceMatch) {
          currentService = serviceMatch[1];
        }
        
        // Check if this is our credential
        if (currentAccount && currentService === this.serviceName) {
          const id = this.parseAccountName(currentAccount);
          if (id) {
            credentialIds.push(id);
          }
          currentAccount = null;
          currentService = null;
        }
      }
      
      return credentialIds;
    } catch (error) {
      throw this.handleError(error, 'list');
    }
  }

  /**
   * Check if macOS Keychain is available
   */
  async isAvailable(): Promise<boolean> {
    if (process.platform !== 'darwin') {
      return false;
    }
    
    try {
      // Test if security command is available
      await execAsync('security -h');
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get platform name
   */
  getPlatformName(): string {
    return 'macOS Keychain';
  }

  /**
   * Delete credential from keychain
   */
  private async deleteFromKeychain(accountName: string): Promise<boolean> {
    try {
      const command = [
        'security',
        'delete-generic-password',
        '-a', accountName,
        '-s', this.serviceName
      ].map(arg => this.escapeShellArg(arg)).join(' ');
      
      await execAsync(command);
      return true;
    } catch (error) {
      if (error.code === 44) { // Item not found
        return false;
      }
      throw error;
    }
  }

  /**
   * Escape shell arguments
   */
  private escapeShellArg(arg: string): string {
    return `'${arg.replace(/'/g, "'\\''")}'`;
  }
}

/**
 * Alternative implementation using keytar for better reliability
 */
export class MacOSKeychainProviderNative extends BaseCredentialProvider {
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
      throw new Error('Native keychain access not available');
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
      throw new Error('Native keychain access not available');
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
      throw new Error('Native keychain access not available');
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
      throw new Error('Native keychain access not available');
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
    return process.platform === 'darwin' && !!this.keytar;
  }

  getPlatformName(): string {
    return 'macOS Keychain (Native)';
  }
}