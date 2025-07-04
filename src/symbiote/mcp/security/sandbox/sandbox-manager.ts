/**
 * Sandbox Manager
 * 
 * Manages sandboxed execution environments for MCP servers
 */

import { EventEmitter } from 'events';
import { ChildProcess, spawn } from 'child_process';
import * as os from 'os';
import * as path from 'path';
import { Worker } from 'worker_threads';
import {
  SandboxConfig,
  ResourceLimits,
  FilesystemRestrictions,
  NetworkRestrictions,
  ProcessRestrictions,
  SecurityMetrics
} from '../types';
import { ProcessSandbox } from './process-sandbox';
import { ResourceMonitor } from './resource-monitor';
import { NetworkProxy } from './network-proxy';
import { FilesystemGuard } from './filesystem-guard';

export interface SandboxedProcess {
  id: string;
  serverId: string;
  process: ChildProcess | Worker;
  sandbox: ProcessSandbox;
  monitor: ResourceMonitor;
  config: SandboxConfig;
  startTime: Date;
  violations: number;
}

export class SandboxManager extends EventEmitter {
  private sandboxes: Map<string, SandboxedProcess> = new Map();
  private networkProxy: NetworkProxy;
  private filesystemGuard: FilesystemGuard;
  private monitoringInterval: NodeJS.Timer | null = null;
  
  constructor() {
    super();
    
    this.networkProxy = new NetworkProxy();
    this.filesystemGuard = new FilesystemGuard();
    
    this.startMonitoring();
  }
  
  /**
   * Create sandboxed process
   */
  async createSandbox(
    serverId: string,
    command: string,
    args: string[],
    config: SandboxConfig,
    env?: Record<string, string>
  ): Promise<SandboxedProcess> {
    // Create process sandbox
    const sandbox = new ProcessSandbox(config);
    
    // Apply filesystem restrictions
    await this.filesystemGuard.applyRestrictions(serverId, config.filesystem);
    
    // Setup network proxy if needed
    let proxiedEnv = env || {};
    if (config.network.enabled) {
      const proxyConfig = await this.networkProxy.createProxy(serverId, config.network);
      proxiedEnv = {
        ...proxiedEnv,
        ...proxyConfig.env
      };
    }
    
    // Create sandboxed process
    const sandboxedProcess = await sandbox.spawn(
      command,
      args,
      {
        env: this.filterEnvironment(proxiedEnv, config.process.envVarRestrictions),
        cwd: this.restrictWorkingDirectory(process.cwd(), config.filesystem),
        uid: this.getRestrictedUid(config),
        gid: this.getRestrictedGid(config),
        detached: false
      }
    );
    
    // Create resource monitor
    const monitor = new ResourceMonitor(
      sandboxedProcess.pid!,
      config.resourceLimits
    );
    
    // Setup monitoring
    monitor.on('limit-exceeded', (metric) => {
      this.handleLimitExceeded(serverId, metric);
    });
    
    await monitor.start();
    
    // Store sandbox
    const sandboxInfo: SandboxedProcess = {
      id: this.generateId(),
      serverId,
      process: sandboxedProcess,
      sandbox,
      monitor,
      config,
      startTime: new Date(),
      violations: 0
    };
    
    this.sandboxes.set(serverId, sandboxInfo);
    
    // Setup process handlers
    this.setupProcessHandlers(sandboxInfo);
    
    this.emit('sandbox-created', { serverId, config });
    
    return sandboxInfo;
  }
  
  /**
   * Update sandbox configuration
   */
  async updateConfig(serverId: string, config: SandboxConfig): Promise<void> {
    const sandbox = this.sandboxes.get(serverId);
    if (!sandbox) {
      throw new Error(`Sandbox for server ${serverId} not found`);
    }
    
    // Update config
    sandbox.config = config;
    
    // Update resource limits
    await sandbox.monitor.updateLimits(config.resourceLimits);
    
    // Update filesystem restrictions
    await this.filesystemGuard.updateRestrictions(serverId, config.filesystem);
    
    // Update network restrictions
    if (config.network.enabled) {
      await this.networkProxy.updateRestrictions(serverId, config.network);
    }
    
    this.emit('config-updated', { serverId, config });
  }
  
  /**
   * Terminate sandbox
   */
  async terminateSandbox(serverId: string, force: boolean = false): Promise<void> {
    const sandbox = this.sandboxes.get(serverId);
    if (!sandbox) {
      return;
    }
    
    try {
      // Stop monitoring
      await sandbox.monitor.stop();
      
      // Terminate process
      await sandbox.sandbox.terminate(force);
      
      // Cleanup network proxy
      await this.networkProxy.removeProxy(serverId);
      
      // Cleanup filesystem restrictions
      await this.filesystemGuard.removeRestrictions(serverId);
      
      // Remove from map
      this.sandboxes.delete(serverId);
      
      this.emit('sandbox-terminated', { serverId });
      
    } catch (error) {
      this.emit('error', {
        serverId,
        error,
        operation: 'terminate'
      });
    }
  }
  
  /**
   * Get metrics for sandbox
   */
  async getMetrics(serverId: string): Promise<SecurityMetrics> {
    const sandbox = this.sandboxes.get(serverId);
    if (!sandbox) {
      throw new Error(`Sandbox for server ${serverId} not found`);
    }
    
    const resourceMetrics = await sandbox.monitor.getMetrics();
    const networkMetrics = await this.networkProxy.getMetrics(serverId);
    
    return {
      serverId,
      timestamp: new Date(),
      cpu: {
        usage: resourceMetrics.cpu.percentage,
        limit: sandbox.config.resourceLimits.maxCpuPercent
      },
      memory: {
        used: resourceMetrics.memory.usedMB,
        limit: sandbox.config.resourceLimits.maxMemoryMB
      },
      disk: {
        readMBps: resourceMetrics.disk.readMBps,
        writeMBps: resourceMetrics.disk.writeMBps,
        limit: sandbox.config.resourceLimits.maxDiskIOMBps
      },
      network: {
        requestsPerSecond: networkMetrics.requestsPerSecond,
        bytesPerSecond: networkMetrics.bytesPerSecond,
        activeConnections: networkMetrics.activeConnections
      },
      violations: sandbox.violations,
      deniedOperations: 0 // TODO: Track denied operations
    };
  }
  
  /**
   * Check if operation is allowed
   */
  async checkOperation(
    serverId: string,
    operation: {
      type: 'file' | 'network' | 'process';
      action: string;
      resource: string;
    }
  ): Promise<boolean> {
    const sandbox = this.sandboxes.get(serverId);
    if (!sandbox) {
      return false;
    }
    
    switch (operation.type) {
      case 'file':
        return this.filesystemGuard.checkAccess(
          serverId,
          operation.resource,
          operation.action as any
        );
        
      case 'network':
        return this.networkProxy.checkAccess(
          serverId,
          operation.resource
        );
        
      case 'process':
        return this.checkProcessOperation(
          sandbox.config.process,
          operation.action,
          operation.resource
        );
        
      default:
        return false;
    }
  }
  
  /**
   * Report violation
   */
  reportViolation(serverId: string, violation: any): void {
    const sandbox = this.sandboxes.get(serverId);
    if (sandbox) {
      sandbox.violations++;
      
      // Check if we should terminate
      if (sandbox.violations > 10) {
        this.terminateSandbox(serverId, true);
      }
    }
    
    this.emit('violation', { serverId, violation });
  }
  
  /**
   * Setup process handlers
   */
  private setupProcessHandlers(sandbox: SandboxedProcess): void {
    const process = sandbox.process;
    
    if ('on' in process) {
      process.on('exit', (code, signal) => {
        this.handleProcessExit(sandbox.serverId, code, signal);
      });
      
      process.on('error', (error) => {
        this.handleProcessError(sandbox.serverId, error);
      });
    }
  }
  
  /**
   * Handle process exit
   */
  private handleProcessExit(
    serverId: string,
    code: number | null,
    signal: NodeJS.Signals | null
  ): void {
    this.emit('process-exit', { serverId, code, signal });
    
    // Cleanup
    this.terminateSandbox(serverId);
  }
  
  /**
   * Handle process error
   */
  private handleProcessError(serverId: string, error: Error): void {
    this.emit('process-error', { serverId, error });
    
    // Report as violation
    this.reportViolation(serverId, {
      type: 'process_error',
      error: error.message
    });
  }
  
  /**
   * Handle resource limit exceeded
   */
  private handleLimitExceeded(serverId: string, metric: any): void {
    this.emit('limit-exceeded', { serverId, metric });
    
    // Report as violation
    this.reportViolation(serverId, {
      type: 'resource_limit',
      metric
    });
    
    const sandbox = this.sandboxes.get(serverId);
    if (sandbox) {
      // Take action based on metric
      if (metric.type === 'memory' && metric.value > metric.limit * 1.5) {
        // Terminate if way over memory limit
        this.terminateSandbox(serverId, true);
      } else if (metric.type === 'cpu' && metric.duration > 60000) {
        // Terminate if CPU limit exceeded for over a minute
        this.terminateSandbox(serverId, true);
      }
    }
  }
  
  /**
   * Start monitoring all sandboxes
   */
  private startMonitoring(): void {
    this.monitoringInterval = setInterval(() => {
      for (const [serverId, sandbox] of this.sandboxes) {
        this.monitorSandbox(serverId, sandbox);
      }
    }, 1000);
  }
  
  /**
   * Monitor individual sandbox
   */
  private async monitorSandbox(
    serverId: string,
    sandbox: SandboxedProcess
  ): Promise<void> {
    try {
      // Check if process is still alive
      if ('exitCode' in sandbox.process && sandbox.process.exitCode !== null) {
        await this.terminateSandbox(serverId);
        return;
      }
      
      // Check resource usage
      const metrics = await sandbox.monitor.getMetrics();
      
      // Check for anomalies
      if (this.detectAnomalies(metrics, sandbox.config.resourceLimits)) {
        this.emit('anomaly-detected', { serverId, metrics });
      }
      
    } catch (error) {
      // Process might have died
      await this.terminateSandbox(serverId);
    }
  }
  
  /**
   * Detect anomalies in resource usage
   */
  private detectAnomalies(
    metrics: any,
    limits: ResourceLimits
  ): boolean {
    // Check for suspicious patterns
    if (metrics.cpu.percentage > limits.maxCpuPercent * 0.9) {
      return true; // Near CPU limit
    }
    
    if (metrics.memory.usedMB > limits.maxMemoryMB * 0.9) {
      return true; // Near memory limit
    }
    
    if (metrics.disk.readMBps + metrics.disk.writeMBps > limits.maxDiskIOMBps * 0.9) {
      return true; // Near I/O limit
    }
    
    return false;
  }
  
  /**
   * Filter environment variables
   */
  private filterEnvironment(
    env: Record<string, string>,
    restrictions: ProcessRestrictions['envVarRestrictions']
  ): Record<string, string | undefined> {
    const filtered: Record<string, string | undefined> = {};
    
    if (restrictions.blockAll) {
      // Only allow explicitly allowed vars
      if (restrictions.allowed) {
        for (const key of restrictions.allowed) {
          if (env[key]) {
            filtered[key] = env[key];
          }
        }
      }
    } else {
      // Copy all except blocked
      for (const [key, value] of Object.entries(env)) {
        let blocked = false;
        
        if (restrictions.blocked) {
          for (const pattern of restrictions.blocked) {
            if (pattern.includes('*')) {
              // Handle wildcards
              const regex = new RegExp(pattern.replace('*', '.*'));
              if (regex.test(key)) {
                blocked = true;
                break;
              }
            } else if (key === pattern) {
              blocked = true;
              break;
            }
          }
        }
        
        if (!blocked) {
          filtered[key] = value;
        }
      }
    }
    
    return filtered;
  }
  
  /**
   * Restrict working directory
   */
  private restrictWorkingDirectory(
    requestedCwd: string,
    restrictions: FilesystemRestrictions
  ): string {
    // Ensure CWD is within allowed paths
    const normalizedCwd = path.normalize(requestedCwd);
    
    for (const allowedPath of restrictions.allowedReadPaths) {
      const normalizedAllowed = path.normalize(allowedPath);
      if (normalizedCwd.startsWith(normalizedAllowed)) {
        return normalizedCwd;
      }
    }
    
    // Default to first allowed path
    return path.normalize(restrictions.allowedReadPaths[0] || process.cwd());
  }
  
  /**
   * Get restricted UID (Unix only)
   */
  private getRestrictedUid(config: SandboxConfig): number | undefined {
    if (process.platform === 'win32') {
      return undefined;
    }
    
    // Use nobody user for strict isolation
    if (config.isolationLevel === 'strict') {
      try {
        const { uid } = os.userInfo();
        // Don't change if already non-root
        if (uid !== 0) {
          return undefined;
        }
        // TODO: Get 'nobody' user ID
        return 65534; // Common UID for nobody
      } catch {
        return undefined;
      }
    }
    
    return undefined;
  }
  
  /**
   * Get restricted GID (Unix only)
   */
  private getRestrictedGid(config: SandboxConfig): number | undefined {
    if (process.platform === 'win32') {
      return undefined;
    }
    
    // Use nogroup for strict isolation
    if (config.isolationLevel === 'strict') {
      try {
        const { gid } = os.userInfo();
        // Don't change if already non-root
        if (gid !== 0) {
          return undefined;
        }
        // TODO: Get 'nogroup' ID
        return 65534; // Common GID for nogroup
      } catch {
        return undefined;
      }
    }
    
    return undefined;
  }
  
  /**
   * Check process operation
   */
  private checkProcessOperation(
    restrictions: ProcessRestrictions,
    action: string,
    resource: string
  ): boolean {
    if (action === 'spawn') {
      if (!restrictions.allowSpawn) {
        return false;
      }
      
      if (restrictions.allowedExecutables) {
        return restrictions.allowedExecutables.includes(resource);
      }
    }
    
    return true;
  }
  
  /**
   * Generate unique ID
   */
  private generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }
  
  /**
   * Cleanup on shutdown
   */
  async shutdown(): Promise<void> {
    if (this.monitoringInterval) {
      clearInterval(this.monitoringInterval);
    }
    
    // Terminate all sandboxes
    const promises = Array.from(this.sandboxes.keys()).map(serverId =>
      this.terminateSandbox(serverId, true)
    );
    
    await Promise.all(promises);
  }
}