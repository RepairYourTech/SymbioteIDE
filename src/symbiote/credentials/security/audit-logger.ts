/**
 * Audit Logger
 * 
 * Secure audit logging for credential operations without exposing sensitive data
 */

import { EventEmitter } from 'events';
import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';
import { CredentialEvent, ProviderType } from '../core/credential-types';

export interface AuditEvent {
  id: string;
  timestamp: Date;
  type: string;
  userId?: string;
  credentialId?: string;
  provider?: ProviderType;
  action: string;
  result: 'success' | 'failure';
  metadata?: Record<string, any>;
  hash: string;
}

export interface AuditLoggerOptions {
  /**
   * Directory to store audit logs
   */
  logDirectory?: string;
  
  /**
   * Maximum log file size in MB
   */
  maxFileSize?: number;
  
  /**
   * Maximum number of log files to retain
   */
  maxFiles?: number;
  
  /**
   * Enable console output
   */
  consoleOutput?: boolean;
  
  /**
   * Enable file output
   */
  fileOutput?: boolean;
  
  /**
   * Hash algorithm for integrity
   */
  hashAlgorithm?: string;
}

export class AuditLogger extends EventEmitter {
  private options: Required<AuditLoggerOptions>;
  private currentLogFile?: string;
  private logStream?: fs.WriteStream;
  private previousHash: string = '';

  constructor(options: AuditLoggerOptions = {}) {
    super();
    
    this.options = {
      logDirectory: path.join(process.cwd(), '.symbiote', 'audit'),
      maxFileSize: 10, // 10MB
      maxFiles: 100,
      consoleOutput: false,
      fileOutput: true,
      hashAlgorithm: 'sha256',
      ...options
    };

    if (this.options.fileOutput) {
      this.initializeFileLogging();
    }
  }

  /**
   * Log credential event
   */
  async logCredentialEvent(event: CredentialEvent, userId?: string): Promise<void> {
    const auditEvent: AuditEvent = {
      id: this.generateEventId(),
      timestamp: event.timestamp,
      type: 'credential',
      userId,
      credentialId: this.hashCredentialId(event.credentialId),
      action: event.type,
      result: 'success',
      metadata: this.sanitizeMetadata(event.metadata),
      hash: ''
    };

    // Add integrity hash
    auditEvent.hash = this.generateEventHash(auditEvent);

    await this.writeAuditEvent(auditEvent);
  }

  /**
   * Log security event
   */
  async logSecurityEvent(
    action: string,
    result: 'success' | 'failure',
    metadata?: Record<string, any>
  ): Promise<void> {
    const auditEvent: AuditEvent = {
      id: this.generateEventId(),
      timestamp: new Date(),
      type: 'security',
      action,
      result,
      metadata: this.sanitizeMetadata(metadata),
      hash: ''
    };

    auditEvent.hash = this.generateEventHash(auditEvent);

    await this.writeAuditEvent(auditEvent);
  }

  /**
   * Log access event
   */
  async logAccessEvent(
    credentialId: string,
    provider: ProviderType,
    userId?: string,
    success: boolean = true
  ): Promise<void> {
    const auditEvent: AuditEvent = {
      id: this.generateEventId(),
      timestamp: new Date(),
      type: 'access',
      userId,
      credentialId: this.hashCredentialId(credentialId),
      provider,
      action: 'credential_access',
      result: success ? 'success' : 'failure',
      hash: ''
    };

    auditEvent.hash = this.generateEventHash(auditEvent);

    await this.writeAuditEvent(auditEvent);
  }

  /**
   * Log validation event
   */
  async logValidationEvent(
    provider: ProviderType,
    success: boolean,
    error?: string
  ): Promise<void> {
    const auditEvent: AuditEvent = {
      id: this.generateEventId(),
      timestamp: new Date(),
      type: 'validation',
      provider,
      action: 'credential_validation',
      result: success ? 'success' : 'failure',
      metadata: error ? { error: this.sanitizeError(error) } : undefined,
      hash: ''
    };

    auditEvent.hash = this.generateEventHash(auditEvent);

    await this.writeAuditEvent(auditEvent);
  }

  /**
   * Search audit logs
   */
  async searchLogs(criteria: {
    startDate?: Date;
    endDate?: Date;
    type?: string;
    action?: string;
    credentialId?: string;
    userId?: string;
    result?: 'success' | 'failure';
  }): Promise<AuditEvent[]> {
    if (!this.options.fileOutput) {
      return [];
    }

    const logs: AuditEvent[] = [];
    const files = await this.getLogFiles();

    for (const file of files) {
      const content = await fs.promises.readFile(file, 'utf8');
      const lines = content.split('\n').filter(line => line.trim());

      for (const line of lines) {
        try {
          const event = JSON.parse(line) as AuditEvent;
          
          if (this.matchesCriteria(event, criteria)) {
            logs.push(event);
          }
        } catch {
          // Skip invalid lines
        }
      }
    }

    return logs;
  }

  /**
   * Verify audit log integrity
   */
  async verifyIntegrity(): Promise<{
    valid: boolean;
    errors: string[];
  }> {
    const errors: string[] = [];
    const files = await this.getLogFiles();
    let previousHash = '';

    for (const file of files) {
      const content = await fs.promises.readFile(file, 'utf8');
      const lines = content.split('\n').filter(line => line.trim());

      for (let i = 0; i < lines.length; i++) {
        try {
          const event = JSON.parse(lines[i]) as AuditEvent;
          const expectedHash = this.generateEventHash(event, previousHash);

          if (event.hash !== expectedHash) {
            errors.push(`Invalid hash in ${file} at line ${i + 1}`);
          }

          previousHash = event.hash;
        } catch (error) {
          errors.push(`Invalid JSON in ${file} at line ${i + 1}`);
        }
      }
    }

    return {
      valid: errors.length === 0,
      errors
    };
  }

  /**
   * Initialize file logging
   */
  private async initializeFileLogging(): Promise<void> {
    // Create log directory
    await fs.promises.mkdir(this.options.logDirectory, { recursive: true });

    // Get current log file
    this.currentLogFile = await this.getCurrentLogFile();

    // Create write stream
    this.logStream = fs.createWriteStream(this.currentLogFile, { flags: 'a' });
  }

  /**
   * Get current log file
   */
  private async getCurrentLogFile(): Promise<string> {
    const files = await this.getLogFiles();
    
    if (files.length === 0) {
      return this.createNewLogFile();
    }

    const latestFile = files[files.length - 1];
    const stats = await fs.promises.stat(latestFile);

    // Check if we need a new file
    if (stats.size >= this.options.maxFileSize * 1024 * 1024) {
      return this.createNewLogFile();
    }

    return latestFile;
  }

  /**
   * Create new log file
   */
  private createNewLogFile(): string {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return path.join(this.options.logDirectory, `audit-${timestamp}.log`);
  }

  /**
   * Get all log files
   */
  private async getLogFiles(): Promise<string[]> {
    try {
      const files = await fs.promises.readdir(this.options.logDirectory);
      return files
        .filter(file => file.startsWith('audit-') && file.endsWith('.log'))
        .map(file => path.join(this.options.logDirectory, file))
        .sort();
    } catch {
      return [];
    }
  }

  /**
   * Write audit event
   */
  private async writeAuditEvent(event: AuditEvent): Promise<void> {
    const line = JSON.stringify(event) + '\n';

    // Console output
    if (this.options.consoleOutput) {
      console.log('[AUDIT]', this.formatEventForConsole(event));
    }

    // File output
    if (this.options.fileOutput && this.logStream) {
      this.logStream.write(line);
      
      // Check rotation
      await this.checkLogRotation();
    }

    // Emit event
    this.emit('audit', event);

    // Update previous hash for chaining
    this.previousHash = event.hash;
  }

  /**
   * Check if log rotation is needed
   */
  private async checkLogRotation(): Promise<void> {
    if (!this.currentLogFile) return;

    const stats = await fs.promises.stat(this.currentLogFile);
    
    if (stats.size >= this.options.maxFileSize * 1024 * 1024) {
      // Close current stream
      if (this.logStream) {
        this.logStream.end();
      }

      // Create new log file
      this.currentLogFile = this.createNewLogFile();
      this.logStream = fs.createWriteStream(this.currentLogFile, { flags: 'a' });

      // Clean up old files
      await this.cleanupOldLogs();
    }
  }

  /**
   * Clean up old log files
   */
  private async cleanupOldLogs(): Promise<void> {
    const files = await this.getLogFiles();
    
    if (files.length > this.options.maxFiles) {
      const filesToDelete = files.slice(0, files.length - this.options.maxFiles);
      
      for (const file of filesToDelete) {
        await fs.promises.unlink(file);
      }
    }
  }

  /**
   * Generate event ID
   */
  private generateEventId(): string {
    return crypto.randomBytes(16).toString('hex');
  }

  /**
   * Hash credential ID
   */
  private hashCredentialId(credentialId: string): string {
    const hash = crypto.createHash('sha256');
    hash.update(credentialId);
    return hash.digest('hex').substring(0, 16);
  }

  /**
   * Generate event hash for integrity
   */
  private generateEventHash(event: AuditEvent, previousHash?: string): string {
    const hash = crypto.createHash(this.options.hashAlgorithm);
    
    // Include previous hash for chaining
    if (previousHash || this.previousHash) {
      hash.update(previousHash || this.previousHash);
    }

    // Hash event data (excluding the hash field itself)
    const { hash: _, ...eventData } = event;
    hash.update(JSON.stringify(eventData));

    return hash.digest('hex');
  }

  /**
   * Sanitize metadata for logging
   */
  private sanitizeMetadata(metadata?: Record<string, any>): Record<string, any> | undefined {
    if (!metadata) return undefined;

    const sanitized: Record<string, any> = {};

    for (const [key, value] of Object.entries(metadata)) {
      // Skip sensitive keys
      if (this.isSensitiveKey(key)) {
        continue;
      }

      // Recursively sanitize objects
      if (typeof value === 'object' && value !== null) {
        sanitized[key] = this.sanitizeMetadata(value);
      } else if (typeof value === 'string') {
        // Mask potential sensitive values
        sanitized[key] = this.maskSensitiveValue(value);
      } else {
        sanitized[key] = value;
      }
    }

    return sanitized;
  }

  /**
   * Check if key is sensitive
   */
  private isSensitiveKey(key: string): boolean {
    const sensitiveKeys = [
      'apikey', 'api_key', 'apiKey',
      'password', 'secret', 'token',
      'credential', 'auth', 'authorization'
    ];

    const lowerKey = key.toLowerCase();
    return sensitiveKeys.some(sensitive => lowerKey.includes(sensitive));
  }

  /**
   * Mask sensitive value
   */
  private maskSensitiveValue(value: string): string {
    // Check for API key patterns
    if (/^sk-[a-zA-Z0-9]{48}$/.test(value) ||
        /^sk-ant-[a-zA-Z0-9-_]{40,}$/.test(value)) {
      return '***REDACTED***';
    }

    return value;
  }

  /**
   * Sanitize error message
   */
  private sanitizeError(error: string): string {
    // Remove any potential API keys from error messages
    return error
      .replace(/sk-[a-zA-Z0-9]{48}/g, '***REDACTED***')
      .replace(/sk-ant-[a-zA-Z0-9-_]{40,}/g, '***REDACTED***')
      .replace(/Bearer [a-zA-Z0-9-_]+/g, 'Bearer ***REDACTED***');
  }

  /**
   * Format event for console output
   */
  private formatEventForConsole(event: AuditEvent): string {
    return `${event.timestamp.toISOString()} [${event.type}] ${event.action} - ${event.result}`;
  }

  /**
   * Check if event matches criteria
   */
  private matchesCriteria(
    event: AuditEvent,
    criteria: any
  ): boolean {
    if (criteria.startDate && event.timestamp < criteria.startDate) {
      return false;
    }

    if (criteria.endDate && event.timestamp > criteria.endDate) {
      return false;
    }

    if (criteria.type && event.type !== criteria.type) {
      return false;
    }

    if (criteria.action && event.action !== criteria.action) {
      return false;
    }

    if (criteria.credentialId && event.credentialId !== this.hashCredentialId(criteria.credentialId)) {
      return false;
    }

    if (criteria.userId && event.userId !== criteria.userId) {
      return false;
    }

    if (criteria.result && event.result !== criteria.result) {
      return false;
    }

    return true;
  }

  /**
   * Close audit logger
   */
  async close(): Promise<void> {
    if (this.logStream) {
      await new Promise<void>(resolve => {
        this.logStream!.end(() => resolve());
      });
    }
  }
}