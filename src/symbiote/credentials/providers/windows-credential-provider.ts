/**
 * Windows Credential Provider
 * 
 * Uses Windows Credential Manager for secure storage
 */

import { Credential } from '../core/credential-types';
import { BaseCredentialProvider } from './base-provider';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export class WindowsCredentialProvider extends BaseCredentialProvider {
  private readonly targetPrefix = 'SymbioteIDE:';

  /**
   * Store credential in Windows Credential Manager
   */
  async store(credential: Credential): Promise<string> {
    try {
      const targetName = this.targetPrefix + credential.id;
      const metadata = this.serializeMetadata(credential);
      
      // Use cmdkey to store credential
      // Store API key as password and metadata as username
      const command = `cmdkey /generic:"${targetName}" /user:"${Buffer.from(metadata).toString('base64')}" /pass:"${credential.apiKey}"`;
      
      await execAsync(command);
      
      this.logSecurityEvent('credential_stored', credential.id);
      return credential.id;
    } catch (error) {
      throw this.handleError(error, 'store');
    }
  }

  /**
   * Retrieve credential from Windows Credential Manager
   */
  async retrieve(id: string): Promise<Credential | null> {
    try {
      const targetName = this.targetPrefix + id;
      
      // Use PowerShell to retrieve credential
      const script = `
        $cred = Get-StoredCredential -Target "${targetName}" -Type Generic -AsCredentialObject
        if ($cred) {
          $username = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($cred.UserName))
          $password = $cred.Password
          Write-Output "USERNAME:$username"
          Write-Output "PASSWORD:$password"
        }
      `;
      
      const command = `powershell -Command "${script.replace(/\n/g, ' ')}"`;
      const { stdout } = await execAsync(command);
      
      if (!stdout.trim()) {
        return null;
      }
      
      const lines = stdout.trim().split('\n');
      let metadata = '';
      let apiKey = '';
      
      for (const line of lines) {
        if (line.startsWith('USERNAME:')) {
          metadata = Buffer.from(line.substring(9), 'base64').toString('utf8');
        } else if (line.startsWith('PASSWORD:')) {
          apiKey = line.substring(9);
        }
      }
      
      if (!metadata || !apiKey) {
        return null;
      }
      
      const credential = this.deserializeMetadata(metadata, apiKey);
      this.logSecurityEvent('credential_retrieved', id);
      
      return credential;
    } catch (error) {
      // Check if error is due to credential not found
      if (error.message?.includes('not found') || error.code === 1) {
        return null;
      }
      throw this.handleError(error, 'retrieve');
    }
  }

  /**
   * Delete credential from Windows Credential Manager
   */
  async delete(id: string): Promise<boolean> {
    try {
      const targetName = this.targetPrefix + id;
      const command = `cmdkey /delete:"${targetName}"`;
      
      await execAsync(command);
      
      this.logSecurityEvent('credential_deleted', id);
      return true;
    } catch (error) {
      // Check if error is due to credential not found
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
      // Use cmdkey to list all credentials
      const { stdout } = await execAsync('cmdkey /list');
      
      const lines = stdout.split('\n');
      const credentialIds: string[] = [];
      
      for (const line of lines) {
        if (line.includes(this.targetPrefix)) {
          const match = line.match(/Target: (.+)/);
          if (match) {
            const target = match[1].trim();
            if (target.startsWith(this.targetPrefix)) {
              const id = target.substring(this.targetPrefix.length);
              credentialIds.push(id);
            }
          }
        }
      }
      
      return credentialIds;
    } catch (error) {
      throw this.handleError(error, 'list');
    }
  }

  /**
   * Check if Windows Credential Manager is available
   */
  async isAvailable(): Promise<boolean> {
    if (process.platform !== 'win32') {
      return false;
    }
    
    try {
      // Test if cmdkey is available
      await execAsync('cmdkey /?');
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get platform name
   */
  getPlatformName(): string {
    return 'Windows Credential Manager';
  }
}

/**
 * Alternative implementation using node-windows-credentials if available
 * This would be more reliable but requires native module compilation
 */
export class WindowsCredentialProviderNative extends BaseCredentialProvider {
  private keytar: any;

  constructor() {
    super();
    // Try to load keytar if available
    try {
      this.keytar = require('keytar');
    } catch {
      // Keytar not available, will use command line fallback
    }
  }

  async store(credential: Credential): Promise<string> {
    if (!this.keytar) {
      throw new Error('Native credential storage not available');
    }

    try {
      const metadata = this.serializeMetadata(credential);
      
      // Store API key
      await this.keytar.setPassword(
        this.serviceName,
        this.generateAccountName(credential.id),
        credential.apiKey
      );
      
      // Store metadata separately
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
      throw new Error('Native credential storage not available');
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
      throw new Error('Native credential storage not available');
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
      throw new Error('Native credential storage not available');
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
    return process.platform === 'win32' && !!this.keytar;
  }

  getPlatformName(): string {
    return 'Windows Credential Manager (Native)';
  }
}