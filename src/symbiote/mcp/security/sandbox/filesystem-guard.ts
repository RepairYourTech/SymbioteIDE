/**
 * Filesystem Guard
 * 
 * Enforces filesystem access restrictions for sandboxed processes
 */

import { EventEmitter } from 'events';
import * as path from 'path';
import * as fs from 'fs';
import * as crypto from 'crypto';
import { FilesystemRestrictions } from '../types';

export interface FileAccessEvent {
  serverId: string;
  operation: 'read' | 'write' | 'delete' | 'execute' | 'stat';
  path: string;
  allowed: boolean;
  reason?: string;
  timestamp: Date;
}

export interface FilesystemMetrics {
  totalReads: number;
  totalWrites: number;
  blockedReads: number;
  blockedWrites: number;
  bytesRead: number;
  bytesWritten: number;
  openFiles: number;
}

interface FileHandle {
  path: string;
  mode: 'r' | 'w' | 'rw';
  opened: Date;
  lastAccess: Date;
  bytesRead: number;
  bytesWritten: number;
}

export class FilesystemGuard extends EventEmitter {
  private handles: Map<string, Map<number, FileHandle>> = new Map();
  private metrics: Map<string, FilesystemMetrics> = new Map();
  private tempFiles: Map<string, Set<string>> = new Map();
  private watchedPaths: Map<string, fs.FSWatcher> = new Map();
  
  /**
   * Check file access permission
   */
  async checkAccess(
    serverId: string,
    filePath: string,
    operation: FileAccessEvent['operation'],
    restrictions: FilesystemRestrictions
  ): Promise<{ allowed: boolean; reason?: string }> {
    const normalizedPath = path.normalize(path.resolve(filePath));
    
    // Track access attempt
    this.recordAccess(serverId, normalizedPath, operation);
    
    // Check if filesystem access is enabled
    if (!restrictions.enabled) {
      return { allowed: false, reason: 'Filesystem access disabled' };
    }
    
    // Check against blocked paths
    for (const blockedPath of restrictions.blockedPaths) {
      if (this.matchesPath(normalizedPath, blockedPath)) {
        this.emitAccessEvent(serverId, operation, normalizedPath, false, 'Path is blocked');
        return { allowed: false, reason: `Path matches blocked pattern: ${blockedPath}` };
      }
    }
    
    // Check against allowed paths based on operation
    const allowedPaths = operation === 'write' || operation === 'delete' ?
      restrictions.allowedWritePaths :
      restrictions.allowedReadPaths;
    
    let allowed = false;
    for (const allowedPath of allowedPaths) {
      if (this.matchesPath(normalizedPath, allowedPath)) {
        allowed = true;
        break;
      }
    }
    
    if (!allowed && allowedPaths.length > 0) {
      this.emitAccessEvent(serverId, operation, normalizedPath, false, 'Path not in allowed list');
      return { allowed: false, reason: 'Path not in allowed list' };
    }
    
    // Check file size limits for read operations
    if (operation === 'read' && restrictions.maxFileSizeMB) {
      try {
        const stats = await fs.promises.stat(normalizedPath);
        const sizeMB = stats.size / (1024 * 1024);
        
        if (sizeMB > restrictions.maxFileSizeMB) {
          this.emitAccessEvent(serverId, operation, normalizedPath, false, 'File too large');
          return { allowed: false, reason: `File size ${sizeMB.toFixed(2)}MB exceeds limit of ${restrictions.maxFileSizeMB}MB` };
        }
      } catch {
        // File doesn't exist, allow the operation to fail naturally
      }
    }
    
    // Check temp directory restrictions
    if (restrictions.tempDirOnly && operation === 'write') {
      const tempDir = await this.getServerTempDir(serverId);
      if (!normalizedPath.startsWith(tempDir)) {
        this.emitAccessEvent(serverId, operation, normalizedPath, false, 'Write outside temp directory');
        return { allowed: false, reason: 'Writes are restricted to temp directory only' };
      }
    }
    
    // Additional security checks
    if (this.isPotentiallyDangerous(normalizedPath)) {
      this.emitAccessEvent(serverId, operation, normalizedPath, false, 'Potentially dangerous path');
      return { allowed: false, reason: 'Access to system files is restricted' };
    }
    
    this.emitAccessEvent(serverId, operation, normalizedPath, true);
    return { allowed: true };
  }
  
  /**
   * Create monitored file handle
   */
  async createFileHandle(
    serverId: string,
    filePath: string,
    mode: 'r' | 'w' | 'rw',
    restrictions: FilesystemRestrictions
  ): Promise<number> {
    // Check access first
    const operation = mode.includes('w') ? 'write' : 'read';
    const { allowed, reason } = await this.checkAccess(serverId, filePath, operation, restrictions);
    
    if (!allowed) {
      throw new Error(`Access denied: ${reason}`);
    }
    
    // Generate handle ID
    const handleId = Date.now() + Math.floor(Math.random() * 1000);
    
    // Store handle
    const serverHandles = this.handles.get(serverId) || new Map();
    serverHandles.set(handleId, {
      path: filePath,
      mode,
      opened: new Date(),
      lastAccess: new Date(),
      bytesRead: 0,
      bytesWritten: 0
    });
    
    this.handles.set(serverId, serverHandles);
    
    // Update metrics
    const metrics = this.getMetrics(serverId);
    metrics.openFiles++;
    
    return handleId;
  }
  
  /**
   * Close file handle
   */
  async closeFileHandle(serverId: string, handleId: number): Promise<void> {
    const serverHandles = this.handles.get(serverId);
    if (!serverHandles) return;
    
    const handle = serverHandles.get(handleId);
    if (!handle) return;
    
    serverHandles.delete(handleId);
    
    // Update metrics
    const metrics = this.getMetrics(serverId);
    metrics.openFiles = Math.max(0, metrics.openFiles - 1);
    
    // Emit close event
    this.emit('file-closed', {
      serverId,
      path: handle.path,
      bytesRead: handle.bytesRead,
      bytesWritten: handle.bytesWritten,
      duration: Date.now() - handle.opened.getTime()
    });
  }
  
  /**
   * Track bytes read
   */
  trackBytesRead(serverId: string, handleId: number, bytes: number): void {
    const serverHandles = this.handles.get(serverId);
    if (!serverHandles) return;
    
    const handle = serverHandles.get(handleId);
    if (!handle) return;
    
    handle.bytesRead += bytes;
    handle.lastAccess = new Date();
    
    const metrics = this.getMetrics(serverId);
    metrics.bytesRead += bytes;
  }
  
  /**
   * Track bytes written
   */
  trackBytesWritten(serverId: string, handleId: number, bytes: number): void {
    const serverHandles = this.handles.get(serverId);
    if (!serverHandles) return;
    
    const handle = serverHandles.get(handleId);
    if (!handle) return;
    
    handle.bytesWritten += bytes;
    handle.lastAccess = new Date();
    
    const metrics = this.getMetrics(serverId);
    metrics.bytesWritten += bytes;
  }
  
  /**
   * Get server temp directory
   */
  async getServerTempDir(serverId: string): Promise<string> {
    const baseTemp = process.env.TEMP || process.env.TMP || '/tmp';
    const serverTemp = path.join(baseTemp, 'symbiote-mcp', serverId);
    
    // Create if doesn't exist
    await fs.promises.mkdir(serverTemp, { recursive: true });
    
    // Track temp files
    if (!this.tempFiles.has(serverId)) {
      this.tempFiles.set(serverId, new Set());
    }
    
    return serverTemp;
  }
  
  /**
   * Create temp file
   */
  async createTempFile(
    serverId: string,
    prefix: string = 'tmp',
    extension: string = ''
  ): Promise<string> {
    const tempDir = await this.getServerTempDir(serverId);
    const filename = `${prefix}-${Date.now()}-${crypto.randomBytes(4).toString('hex')}${extension}`;
    const filepath = path.join(tempDir, filename);
    
    // Track temp file
    const serverTempFiles = this.tempFiles.get(serverId) || new Set();
    serverTempFiles.add(filepath);
    this.tempFiles.set(serverId, serverTempFiles);
    
    return filepath;
  }
  
  /**
   * Clean up server resources
   */
  async cleanup(serverId: string): Promise<void> {
    // Close all file handles
    const serverHandles = this.handles.get(serverId);
    if (serverHandles) {
      for (const handleId of serverHandles.keys()) {
        await this.closeFileHandle(serverId, handleId);
      }
      this.handles.delete(serverId);
    }
    
    // Clean up temp files
    const serverTempFiles = this.tempFiles.get(serverId);
    if (serverTempFiles) {
      for (const filepath of serverTempFiles) {
        try {
          await fs.promises.unlink(filepath);
        } catch {
          // File might already be deleted
        }
      }
      this.tempFiles.delete(serverId);
    }
    
    // Remove temp directory
    try {
      const tempDir = await this.getServerTempDir(serverId);
      await fs.promises.rmdir(tempDir);
    } catch {
      // Directory might not be empty or already deleted
    }
    
    // Clear metrics
    this.metrics.delete(serverId);
  }
  
  /**
   * Watch path for changes
   */
  async watchPath(
    serverId: string,
    watchPath: string,
    callback: (event: string, filename: string) => void
  ): Promise<void> {
    const key = `${serverId}:${watchPath}`;
    
    // Check if already watching
    if (this.watchedPaths.has(key)) {
      return;
    }
    
    try {
      const watcher = fs.watch(watchPath, (event, filename) => {
        // Security check - ensure callback can't escape sandbox
        if (filename && this.isPotentiallyDangerous(filename)) {
          return;
        }
        
        callback(event, filename);
      });
      
      this.watchedPaths.set(key, watcher);
      
      // Auto-cleanup after timeout
      setTimeout(() => {
        this.unwatchPath(serverId, watchPath);
      }, 300000); // 5 minutes
      
    } catch (error) {
      throw new Error(`Failed to watch path: ${error.message}`);
    }
  }
  
  /**
   * Stop watching path
   */
  unwatchPath(serverId: string, watchPath: string): void {
    const key = `${serverId}:${watchPath}`;
    const watcher = this.watchedPaths.get(key);
    
    if (watcher) {
      watcher.close();
      this.watchedPaths.delete(key);
    }
  }
  
  /**
   * Get filesystem metrics
   */
  getMetrics(serverId: string): FilesystemMetrics {
    let metrics = this.metrics.get(serverId);
    
    if (!metrics) {
      metrics = {
        totalReads: 0,
        totalWrites: 0,
        blockedReads: 0,
        blockedWrites: 0,
        bytesRead: 0,
        bytesWritten: 0,
        openFiles: 0
      };
      this.metrics.set(serverId, metrics);
    }
    
    return metrics;
  }
  
  /**
   * Check if path matches pattern
   */
  private matchesPath(filePath: string, pattern: string): boolean {
    const normalizedPath = path.normalize(filePath);
    const normalizedPattern = path.normalize(pattern);
    
    // Handle glob patterns
    if (pattern.includes('*')) {
      const regex = this.globToRegex(normalizedPattern);
      return regex.test(normalizedPath);
    }
    
    // Handle directory patterns (ending with /)
    if (pattern.endsWith('/') || pattern.endsWith(path.sep)) {
      return normalizedPath.startsWith(normalizedPattern);
    }
    
    // Exact match or subdirectory
    return normalizedPath === normalizedPattern || 
           normalizedPath.startsWith(normalizedPattern + path.sep);
  }
  
  /**
   * Convert glob to regex
   */
  private globToRegex(glob: string): RegExp {
    const escaped = glob
      .replace(/[.+^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '.*')
      .replace(/\?/g, '.');
    
    return new RegExp(`^${escaped}$`);
  }
  
  /**
   * Check if path is potentially dangerous
   */
  private isPotentiallyDangerous(filePath: string): boolean {
    const dangerous = [
      '/etc/passwd',
      '/etc/shadow',
      '/etc/sudoers',
      '/.ssh/',
      '/.aws/',
      '/.git-credentials',
      '/Windows/System32/',
      '/System/Library/',
      'id_rsa',
      'id_dsa',
      'id_ecdsa',
      'id_ed25519',
      '.pem',
      '.key',
      '.pfx'
    ];
    
    const normalized = path.normalize(filePath).toLowerCase();
    
    return dangerous.some(pattern => 
      normalized.includes(pattern.toLowerCase())
    );
  }
  
  /**
   * Record access attempt
   */
  private recordAccess(
    serverId: string,
    filePath: string,
    operation: FileAccessEvent['operation']
  ): void {
    const metrics = this.getMetrics(serverId);
    
    if (operation === 'read') {
      metrics.totalReads++;
    } else if (operation === 'write') {
      metrics.totalWrites++;
    }
  }
  
  /**
   * Emit access event
   */
  private emitAccessEvent(
    serverId: string,
    operation: FileAccessEvent['operation'],
    filePath: string,
    allowed: boolean,
    reason?: string
  ): void {
    const event: FileAccessEvent = {
      serverId,
      operation,
      path: filePath,
      allowed,
      reason,
      timestamp: new Date()
    };
    
    this.emit('file-access', event);
    
    // Update blocked metrics
    if (!allowed) {
      const metrics = this.getMetrics(serverId);
      if (operation === 'read') {
        metrics.blockedReads++;
      } else if (operation === 'write') {
        metrics.blockedWrites++;
      }
    }
  }
}