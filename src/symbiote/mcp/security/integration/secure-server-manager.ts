/**
 * Secure Server Manager
 * 
 * Integrates security features with MCP server management
 */

import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import { MCPServerManager } from '../../server-manager';
import { SecurityManager } from '../security-manager';
import { 
  SandboxConfig, 
  TrustLevel,
  PermissionType,
  SecurityIncident
} from '../types';

export interface SecureServerOptions {
  serverId: string;
  serverName: string;
  command: string;
  args: string[];
  env?: Record<string, string>;
  trustLevel?: TrustLevel;
  autoScan?: boolean;
}

export class SecureMCPServerManager extends EventEmitter {
  private serverManager: MCPServerManager;
  private securityManager: SecurityManager;
  private secureServers: Map<string, SecureServerInfo> = new Map();
  
  constructor(
    serverManager: MCPServerManager,
    securityManager: SecurityManager
  ) {
    super();
    this.serverManager = serverManager;
    this.securityManager = securityManager;
    
    this.setupEventHandlers();
  }
  
  /**
   * Initialize secure server manager
   */
  async initialize(): Promise<void> {
    await this.securityManager.initialize();
    
    // Scan existing servers
    const servers = this.serverManager.getServers();
    for (const [serverId, server] of servers) {
      await this.secureExistingServer(serverId, server);
    }
  }
  
  /**
   * Start secure MCP server
   */
  async startSecureServer(options: SecureServerOptions): Promise<void> {
    const { serverId, serverName, command, args, env, trustLevel, autoScan } = options;
    
    // Initialize server profile
    await this.securityManager.initializeServerProfile(
      serverId,
      serverName,
      trustLevel
    );
    
    // Scan server if requested
    if (autoScan && command) {
      const scanResult = await this.securityManager.scanServer(serverId, command);
      
      if (!scanResult.passed && scanResult.findings.some(f => f.severity === 'critical')) {
        const action = await vscode.window.showWarningMessage(
          `Security scan failed for ${serverName} with critical issues. Continue anyway?`,
          'View Report',
          'Continue',
          'Cancel'
        );
        
        if (action === 'View Report') {
          // Show scan results
          await this.showScanResults(scanResult);
          return;
        } else if (action !== 'Continue') {
          return;
        }
      }
    }
    
    // Get sandbox configuration based on trust level
    const sandboxConfig = await this.securityManager.getSandboxConfig(serverId);
    
    // Create sandboxed process
    const sandboxedProcess = await this.securityManager.createSandbox(
      serverId,
      command,
      args,
      sandboxConfig,
      env
    );
    
    // Store secure server info
    this.secureServers.set(serverId, {
      serverId,
      serverName,
      trustLevel: this.securityManager.getTrustLevel(serverId),
      sandboxConfig,
      process: sandboxedProcess,
      startTime: new Date(),
      status: 'running'
    });
    
    // Setup process monitoring
    this.monitorProcess(serverId, sandboxedProcess);
    
    // Start the actual MCP server
    await this.serverManager.startServer(serverId);
    
    this.emit('server-started', { serverId, serverName });
  }
  
  /**
   * Stop secure server
   */
  async stopSecureServer(serverId: string): Promise<void> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo) return;
    
    // Stop MCP server
    await this.serverManager.stopServer(serverId);
    
    // Terminate sandbox
    await this.securityManager.terminateSandbox(serverId);
    
    // Update status
    serverInfo.status = 'stopped';
    serverInfo.stopTime = new Date();
    
    // Generate session summary
    await this.generateSessionSummary(serverId);
    
    this.emit('server-stopped', { serverId });
  }
  
  /**
   * Request permission for operation
   */
  async requestPermission(
    serverId: string,
    type: PermissionType,
    resource: string,
    metadata?: any
  ): Promise<boolean> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo) return false;
    
    return await this.securityManager.checkPermission(
      serverId,
      type,
      resource,
      metadata
    );
  }
  
  /**
   * Handle tool execution request
   */
  async handleToolExecution(
    serverId: string,
    toolName: string,
    args: any
  ): Promise<{ allowed: boolean; result?: any }> {
    // Check permission
    const allowed = await this.requestPermission(
      serverId,
      PermissionType.ToolExecution,
      toolName,
      { args }
    );
    
    if (!allowed) {
      return { allowed: false };
    }
    
    // Execute tool with monitoring
    const startTime = Date.now();
    let result: any;
    let error: any;
    
    try {
      // This would integrate with the actual tool execution
      result = await this.serverManager.executeTool(serverId, toolName, args);
    } catch (e) {
      error = e;
    }
    
    // Log execution
    await this.securityManager.logToolExecution(serverId, {
      toolName,
      args,
      result: error ? 'error' : 'success',
      duration: Date.now() - startTime,
      error
    });
    
    if (error) {
      throw error;
    }
    
    return { allowed: true, result };
  }
  
  /**
   * Handle resource access request
   */
  async handleResourceAccess(
    serverId: string,
    resourceUri: string
  ): Promise<{ allowed: boolean; content?: any }> {
    // Check permission
    const allowed = await this.requestPermission(
      serverId,
      PermissionType.ResourceAccess,
      resourceUri
    );
    
    if (!allowed) {
      return { allowed: false };
    }
    
    // Access resource with monitoring
    const content = await this.serverManager.accessResource(serverId, resourceUri);
    
    return { allowed: true, content };
  }
  
  /**
   * Update server trust level
   */
  async updateTrustLevel(
    serverId: string,
    trustLevel: TrustLevel,
    reason: string
  ): Promise<void> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo) return;
    
    await this.securityManager.setServerTrustLevel(
      serverId,
      trustLevel,
      reason,
      'user'
    );
    
    // Update sandbox restrictions if server is running
    if (serverInfo.status === 'running') {
      const newConfig = await this.securityManager.getSandboxConfig(serverId);
      await this.securityManager.updateSandboxRestrictions(serverId, newConfig);
      
      serverInfo.sandboxConfig = newConfig;
      serverInfo.trustLevel = trustLevel;
    }
    
    this.emit('trust-updated', { serverId, trustLevel });
  }
  
  /**
   * Get secure server info
   */
  getSecureServerInfo(serverId?: string): SecureServerInfo[] {
    if (serverId) {
      const info = this.secureServers.get(serverId);
      return info ? [info] : [];
    }
    
    return Array.from(this.secureServers.values());
  }
  
  /**
   * Monitor sandboxed process
   */
  private monitorProcess(serverId: string, process: any): void {
    // Monitor resource usage
    process.on('resource-limit', (event: any) => {
      this.handleResourceLimit(serverId, event);
    });
    
    // Monitor suspicious behavior
    process.on('suspicious-activity', (event: any) => {
      this.handleSuspiciousActivity(serverId, event);
    });
    
    // Monitor network access
    process.on('network-blocked', (event: any) => {
      this.securityManager.logNetworkBlock(serverId, event);
    });
    
    // Monitor file access
    process.on('file-access', (event: any) => {
      this.securityManager.logFileAccess(serverId, event);
    });
    
    // Handle process exit
    process.on('exit', (code: number, signal: string) => {
      this.handleProcessExit(serverId, code, signal);
    });
  }
  
  /**
   * Handle resource limit exceeded
   */
  private async handleResourceLimit(serverId: string, event: any): Promise<void> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo) return;
    
    // Log incident
    const incident: SecurityIncident = {
      id: crypto.randomUUID(),
      timestamp: new Date(),
      serverId,
      serverName: serverInfo.serverName,
      type: 'resource_limit_exceeded',
      severity: event.type === 'memory' ? 'high' : 'medium',
      description: `Resource limit exceeded: ${event.type}`,
      details: event,
      resolved: false
    };
    
    await this.securityManager.logIncident(incident);
    
    // Take action based on severity
    if (event.type === 'memory' && event.duration > 60000) {
      // Sustained memory violation - stop server
      await this.stopSecureServer(serverId);
      
      vscode.window.showErrorMessage(
        `Server ${serverInfo.serverName} terminated due to sustained memory limit violation`
      );
    } else {
      // Show warning
      vscode.window.showWarningMessage(
        `Server ${serverInfo.serverName} exceeded ${event.type} limit`
      );
    }
  }
  
  /**
   * Handle suspicious activity
   */
  private async handleSuspiciousActivity(serverId: string, event: any): Promise<void> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo) return;
    
    // Log incident
    const incident: SecurityIncident = {
      id: crypto.randomUUID(),
      timestamp: new Date(),
      serverId,
      serverName: serverInfo.serverName,
      type: 'suspicious_activity',
      severity: 'high',
      description: event.description,
      details: event,
      resolved: false
    };
    
    await this.securityManager.logIncident(incident);
    
    // Notify user
    const action = await vscode.window.showErrorMessage(
      `Suspicious activity detected in ${serverInfo.serverName}: ${event.description}`,
      'Stop Server',
      'View Details',
      'Ignore'
    );
    
    if (action === 'Stop Server') {
      await this.stopSecureServer(serverId);
    } else if (action === 'View Details') {
      await vscode.commands.executeCommand('symbiote.mcpSecurity.showIncident', incident.id);
    }
  }
  
  /**
   * Handle process exit
   */
  private async handleProcessExit(
    serverId: string,
    code: number,
    signal: string
  ): Promise<void> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo) return;
    
    serverInfo.status = 'exited';
    serverInfo.exitCode = code;
    serverInfo.exitSignal = signal;
    
    // Log abnormal exit
    if (code !== 0) {
      await this.securityManager.logAuditEntry({
        id: crypto.randomUUID(),
        timestamp: new Date(),
        serverId,
        serverName: serverInfo.serverName,
        category: 'process',
        action: 'abnormal_exit',
        resource: '',
        result: 'failed',
        metadata: { code, signal }
      });
    }
  }
  
  /**
   * Secure existing server
   */
  private async secureExistingServer(serverId: string, server: any): Promise<void> {
    // Initialize security profile for existing server
    await this.securityManager.initializeServerProfile(
      serverId,
      server.name || serverId,
      TrustLevel.Unknown
    );
    
    // Add to secure servers
    this.secureServers.set(serverId, {
      serverId,
      serverName: server.name || serverId,
      trustLevel: TrustLevel.Unknown,
      sandboxConfig: await this.securityManager.getSandboxConfig(serverId),
      process: null,
      startTime: new Date(),
      status: 'unknown'
    });
  }
  
  /**
   * Generate session summary
   */
  private async generateSessionSummary(serverId: string): Promise<void> {
    const serverInfo = this.secureServers.get(serverId);
    if (!serverInfo || !serverInfo.startTime) return;
    
    const duration = (serverInfo.stopTime || new Date()).getTime() - serverInfo.startTime.getTime();
    
    // Get session metrics
    const metrics = await this.securityManager.getSessionMetrics(serverId);
    
    // Generate summary
    const summary = {
      serverId,
      serverName: serverInfo.serverName,
      duration,
      trustLevel: serverInfo.trustLevel,
      metrics,
      exitCode: serverInfo.exitCode,
      exitSignal: serverInfo.exitSignal
    };
    
    // Log summary
    await this.securityManager.logAuditEntry({
      id: crypto.randomUUID(),
      timestamp: new Date(),
      serverId,
      serverName: serverInfo.serverName,
      category: 'session',
      action: 'session_summary',
      resource: '',
      result: 'info',
      metadata: summary
    });
  }
  
  /**
   * Show scan results
   */
  private async showScanResults(scanResult: any): Promise<void> {
    const doc = await vscode.workspace.openTextDocument({
      content: JSON.stringify(scanResult, null, 2),
      language: 'json'
    });
    
    await vscode.window.showTextDocument(doc);
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Forward security events
    this.securityManager.on('permission-request', (request) => {
      this.emit('permission-request', request);
    });
    
    this.securityManager.on('security-incident', (incident) => {
      this.emit('security-incident', incident);
    });
    
    this.securityManager.on('trust-updated', (event) => {
      this.emit('trust-updated', event);
    });
  }
}

interface SecureServerInfo {
  serverId: string;
  serverName: string;
  trustLevel: TrustLevel;
  sandboxConfig: SandboxConfig;
  process: any;
  startTime: Date;
  stopTime?: Date;
  status: 'running' | 'stopped' | 'exited' | 'unknown';
  exitCode?: number;
  exitSignal?: string;
}