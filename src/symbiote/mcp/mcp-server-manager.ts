/**
 * MCP Server Manager
 * 
 * Manages lifecycle of MCP server processes
 */

import { EventEmitter } from 'events';
import { ChildProcess, spawn } from 'child_process';
import * as path from 'path';
import { 
  McpServerInfo, 
  McpServerStatus, 
  McpServerDefinition,
  McpServerEvent,
  McpServerEventType,
  McpConnection,
  McpCapabilities,
  McpRequest,
  McpResponse,
  McpNotification,
  McpMessage
} from './mcp-types';
import { McpConfigReader } from './mcp-config-reader';
import { SecurityManager, SecurityManagerOptions } from './security/security-manager';
import { TrustLevel, SecurityPolicy, PermissionType } from './security/types';
import * as vscode from 'vscode';

export class McpServerManager extends EventEmitter {
  private servers: Map<string, McpServerInfo> = new Map();
  private processes: Map<string, ChildProcess> = new Map();
  private configReader: McpConfigReader;
  private securityManager: SecurityManager;
  private healthCheckInterval: NodeJS.Timer | null = null;
  private messageBuffers: Map<string, string> = new Map();
  
  constructor(
    private context: vscode.ExtensionContext,
    private options: {
      autoStart?: boolean;
      maxConcurrentServers?: number;
      defaultTimeout?: number;
      healthCheckInterval?: number;
      security?: SecurityManagerOptions;
    } = {}
  ) {
    super();
    this.configReader = new McpConfigReader();
    
    // Initialize security manager with default options
    const securityOptions: SecurityManagerOptions = {
      enableScanning: true,
      enableAuditing: true,
      dataDirectory: path.join(context.globalStorageUri.fsPath, 'security'),
      defaultPolicy: SecurityPolicy.Moderate,
      userInteractionHandler: async (request) => {
        // Show VS Code prompt for permission requests
        const result = await vscode.window.showWarningMessage(
          `MCP Server '${request.serverName}' requests permission to ${request.permission}`,
          { modal: true, detail: request.reason },
          'Allow', 'Deny', 'Always Allow'
        );
        
        if (result === 'Always Allow') {
          // Grant permanent permission
          await this.securityManager.grantPermanentPermission(
            request.serverName,
            request.permission
          );
          return { allow: true, remember: true };
        }
        
        return { allow: result === 'Allow', remember: false };
      },
      ...options.security
    };
    
    this.securityManager = new SecurityManager(securityOptions);
    
    // Start health checks
    if (options.healthCheckInterval !== 0) {
      this.startHealthChecks();
    }
  }
  
  /**
   * Initialize and start configured servers
   */
  async initialize(): Promise<void> {
    // Initialize security manager
    await this.securityManager.initialize();
    
    // Listen to security events
    this.securityManager.on('securityAlert', (alert) => {
      vscode.window.showErrorMessage(`Security Alert: ${alert.message}`, 'View Details').then(action => {
        if (action === 'View Details') {
          this.showSecurityDetails(alert);
        }
      });
    });
    
    this.securityManager.on('permissionDenied', (event) => {
      vscode.window.showWarningMessage(
        `Permission denied for ${event.serverName}: ${event.permission}`
      );
    });
    
    const config = await this.configReader.readConfiguration();
    
    if (!config || !config.mcpServers) {
      return;
    }
    
    // Register all servers with security checks
    for (const [serverId, definition] of Object.entries(config.mcpServers)) {
      // Scan server for security issues before registering
      const scanResult = await this.securityManager.scanServer(definition);
      
      if (scanResult.severity === 'critical') {
        vscode.window.showErrorMessage(
          `Server '${serverId}' failed security scan: ${scanResult.issues[0]?.description}`,
          'Skip', 'Trust Anyway'
        ).then(async (action) => {
          if (action === 'Trust Anyway') {
            await this.securityManager.setServerTrust(serverId, TrustLevel.UserTrusted);
            await this.registerServer(serverId, definition);
          }
        });
      } else {
        await this.registerServer(serverId, definition);
      }
    }
    
    // Auto-start servers if enabled
    if (this.options.autoStart !== false) {
      for (const [serverId, definition] of Object.entries(config.mcpServers)) {
        if (definition.autoStart !== false) {
          await this.startServer(serverId).catch(error => {
            console.error(`Failed to auto-start server ${serverId}:`, error);
          });
        }
      }
    }
  }
  
  /**
   * Register a server
   */
  async registerServer(serverId: string, definition: McpServerDefinition): Promise<void> {
    if (this.servers.has(serverId)) {
      throw new Error(`Server '${serverId}' is already registered`);
    }
    
    const serverInfo: McpServerInfo = {
      id: serverId,
      definition,
      status: McpServerStatus.Stopped,
      restartCount: 0
    };
    
    this.servers.set(serverId, serverInfo);
    this.emitServerEvent(serverId, McpServerEventType.Stopped);
  }
  
  /**
   * Start a server
   */
  async startServer(serverId: string): Promise<void> {
    const serverInfo = this.servers.get(serverId);
    if (!serverInfo) {
      throw new Error(`Server '${serverId}' not found`);
    }
    
    if (serverInfo.status !== McpServerStatus.Stopped && 
        serverInfo.status !== McpServerStatus.Error &&
        serverInfo.status !== McpServerStatus.Crashed) {
      throw new Error(`Server '${serverId}' is already running or starting`);
    }
    
    // Check concurrent server limit
    if (this.options.maxConcurrentServers) {
      const runningCount = Array.from(this.servers.values())
        .filter(s => s.status === McpServerStatus.Running).length;
      
      if (runningCount >= this.options.maxConcurrentServers) {
        throw new Error(`Maximum concurrent servers limit reached (${this.options.maxConcurrentServers})`);
      }
    }
    
    serverInfo.status = McpServerStatus.Starting;
    this.emitServerEvent(serverId, McpServerEventType.Started);
    
    try {
      // Check if server has permission to start
      const canStart = await this.securityManager.checkPermission({
        serverName: serverId,
        permission: PermissionType.Execute,
        resource: serverInfo.definition.command,
        reason: 'Starting MCP server process'
      });
      
      if (!canStart) {
        throw new Error(`Permission denied to start server '${serverId}'`);
      }
      
      // Resolve command
      const command = await this.configReader.resolveCommand(
        serverInfo.definition.command,
        serverInfo.definition.cwd
      );
      
      if (!command) {
        throw new Error(`Command '${serverInfo.definition.command}' not found`);
      }
      
      // Get effective environment
      const env = this.configReader.getEffectiveEnvironment(serverInfo.definition);
      const args = serverInfo.definition.args || [];
      const cwd = serverInfo.definition.cwd || process.cwd();
      
      // Get sandbox configuration based on server trust level
      const trustLevel = await this.securityManager.getServerTrust(serverId);
      const sandboxConfig = await this.securityManager.getSandboxConfig(trustLevel);
      
      // Start server in sandbox
      const sandboxedProcess = await this.securityManager.startSandboxedServer(
        serverId,
        {
          command,
          args,
          cwd,
          env,
          ...sandboxConfig
        }
      );
      
      const childProcess = sandboxedProcess.process;
      this.processes.set(serverId, childProcess);
      
      // Set up process handlers with security monitoring
      this.setupSecureProcessHandlers(serverId, childProcess);
      
      // Wait for initialization
      await this.waitForInitialization(serverId, serverInfo.definition.startupTimeout);
      
      serverInfo.status = McpServerStatus.Running;
      serverInfo.process = {
        pid: childProcess.pid!,
        startTime: new Date()
      };
      
      // Initialize connection
      serverInfo.connection = {
        transport: 'stdio',
        connected: true,
        lastPing: new Date()
      };
      
      // Request capabilities
      await this.requestCapabilities(serverId);
      
      this.emitServerEvent(serverId, McpServerEventType.Connected);
      
      // Log audit entry
      await this.securityManager.auditServerStart(serverId, {
        command,
        args,
        trustLevel,
        sandboxed: true
      });
      
    } catch (error) {
      serverInfo.status = McpServerStatus.Error;
      serverInfo.lastError = error instanceof Error ? error.message : String(error);
      this.emitServerEvent(serverId, McpServerEventType.Error, error);
      throw error;
    }
  }
  
  /**
   * Stop a server
   */
  async stopServer(serverId: string, force: boolean = false): Promise<void> {
    const serverInfo = this.servers.get(serverId);
    if (!serverInfo) {
      throw new Error(`Server '${serverId}' not found`);
    }
    
    if (serverInfo.status === McpServerStatus.Stopped) {
      return;
    }
    
    serverInfo.status = McpServerStatus.Stopping;
    
    const process = this.processes.get(serverId);
    if (!process) {
      serverInfo.status = McpServerStatus.Stopped;
      return;
    }
    
    try {
      if (force) {
        process.kill('SIGKILL');
      } else {
        // Try graceful shutdown first
        process.kill('SIGTERM');
        
        // Wait for process to exit
        await new Promise<void>((resolve, reject) => {
          const timeout = setTimeout(() => {
            process.kill('SIGKILL');
          }, 5000);
          
          process.once('exit', () => {
            clearTimeout(timeout);
            resolve();
          });
        });
      }
      
      this.processes.delete(serverId);
      serverInfo.status = McpServerStatus.Stopped;
      serverInfo.process = undefined;
      serverInfo.connection = undefined;
      serverInfo.capabilities = undefined;
      
      this.emitServerEvent(serverId, McpServerEventType.Stopped);
      
    } catch (error) {
      serverInfo.status = McpServerStatus.Error;
      serverInfo.lastError = error instanceof Error ? error.message : String(error);
      throw error;
    }
  }
  
  /**
   * Restart a server
   */
  async restartServer(serverId: string): Promise<void> {
    await this.stopServer(serverId);
    await this.startServer(serverId);
  }
  
  /**
   * Get server info
   */
  getServer(serverId: string): McpServerInfo | undefined {
    return this.servers.get(serverId);
  }
  
  /**
   * Get all servers
   */
  getAllServers(): McpServerInfo[] {
    return Array.from(this.servers.values());
  }
  
  /**
   * Send request to server
   */
  async sendRequest(serverId: string, method: string, params?: any): Promise<any> {
    const serverInfo = this.servers.get(serverId);
    if (!serverInfo || serverInfo.status !== McpServerStatus.Running) {
      throw new Error(`Server '${serverId}' is not running`);
    }
    
    const process = this.processes.get(serverId);
    if (!process || !process.stdin) {
      throw new Error(`No process found for server '${serverId}'`);
    }
    
    const id = this.generateRequestId();
    const request: McpRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params
    };
    
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`Request timeout for ${method}`));
      }, this.options.defaultTimeout || 30000);
      
      // Set up response handler
      const responseHandler = (message: McpMessage) => {
        if (message.type === 'response' && 
            message.data.id === id) {
          clearTimeout(timeout);
          this.removeListener(`message-${serverId}`, responseHandler);
          
          const response = message.data as McpResponse;
          if (response.error) {
            reject(new Error(`${response.error.message} (${response.error.code})`));
          } else {
            resolve(response.result);
          }
        }
      };
      
      this.on(`message-${serverId}`, responseHandler);
      
      // Send request
      const requestStr = JSON.stringify(request) + '\n';
      process.stdin.write(requestStr);
      
      // Track message
      this.trackMessage(serverId, 'outgoing', request);
    });
  }
  
  /**
   * Send notification to server
   */
  sendNotification(serverId: string, method: string, params?: any): void {
    const serverInfo = this.servers.get(serverId);
    if (!serverInfo || serverInfo.status !== McpServerStatus.Running) {
      throw new Error(`Server '${serverId}' is not running`);
    }
    
    const process = this.processes.get(serverId);
    if (!process || !process.stdin) {
      throw new Error(`No process found for server '${serverId}'`);
    }
    
    const notification: McpNotification = {
      jsonrpc: '2.0',
      method,
      params
    };
    
    const notificationStr = JSON.stringify(notification) + '\n';
    process.stdin.write(notificationStr);
    
    // Track message
    this.trackMessage(serverId, 'outgoing', notification);
  }
  
  /**
   * Shutdown all servers
   */
  async shutdown(): Promise<void> {
    // Stop health checks
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
      this.healthCheckInterval = null;
    }
    
    // Stop all servers
    const stopPromises = Array.from(this.servers.keys())
      .map(serverId => this.stopServer(serverId).catch(console.error));
    
    await Promise.all(stopPromises);
    
    this.servers.clear();
    this.processes.clear();
    this.messageBuffers.clear();
  }
  
  // Private methods
  
  private setupProcessHandlers(serverId: string, process: ChildProcess): void {
    const serverInfo = this.servers.get(serverId)!;
    
    // Handle stdout (JSON-RPC messages)
    process.stdout?.on('data', (data: Buffer) => {
      this.handleStdoutData(serverId, data);
    });
    
    // Handle stderr (logging)
    process.stderr?.on('data', (data: Buffer) => {
      console.error(`[${serverId}]`, data.toString());
    });
    
    // Handle process exit
    process.on('exit', (code, signal) => {
      this.handleProcessExit(serverId, code, signal);
    });
    
    // Handle process error
    process.on('error', (error) => {
      serverInfo.status = McpServerStatus.Error;
      serverInfo.lastError = error.message;
      this.emitServerEvent(serverId, McpServerEventType.Error, error);
    });
  }
  
  private handleStdoutData(serverId: string, data: Buffer): void {
    // Buffer data as it might come in chunks
    let buffer = this.messageBuffers.get(serverId) || '';
    buffer += data.toString();
    
    // Process complete messages
    let newlineIndex: number;
    while ((newlineIndex = buffer.indexOf('\n')) !== -1) {
      const line = buffer.substring(0, newlineIndex);
      buffer = buffer.substring(newlineIndex + 1);
      
      if (line.trim()) {
        try {
          const message = JSON.parse(line);
          this.handleMessage(serverId, message);
        } catch (error) {
          console.error(`Failed to parse message from ${serverId}:`, error, line);
        }
      }
    }
    
    // Store remaining buffer
    this.messageBuffers.set(serverId, buffer);
  }
  
  private handleMessage(serverId: string, data: any): void {
    let type: 'request' | 'response' | 'notification';
    
    if ('id' in data && 'method' in data) {
      type = 'request';
    } else if ('id' in data) {
      type = 'response';
    } else {
      type = 'notification';
    }
    
    const message: McpMessage = {
      type,
      data,
      timestamp: new Date(),
      direction: 'incoming'
    };
    
    // Track message
    this.trackMessage(serverId, 'incoming', data);
    
    // Emit for specific handlers
    this.emit(`message-${serverId}`, message);
    
    // Handle specific notifications
    if (type === 'notification') {
      this.handleNotification(serverId, data as McpNotification);
    }
  }
  
  private handleNotification(serverId: string, notification: McpNotification): void {
    // Handle standard notifications
    switch (notification.method) {
      case 'capabilities/update':
        this.handleCapabilitiesUpdate(serverId, notification.params);
        break;
      case 'error':
        console.error(`Error from ${serverId}:`, notification.params);
        break;
    }
  }
  
  private handleCapabilitiesUpdate(serverId: string, capabilities: McpCapabilities): void {
    const serverInfo = this.servers.get(serverId);
    if (serverInfo) {
      serverInfo.capabilities = capabilities;
      this.emitServerEvent(serverId, McpServerEventType.CapabilitiesUpdated, capabilities);
    }
  }
  
  private handleProcessExit(serverId: string, code: number | null, signal: string | null): void {
    const serverInfo = this.servers.get(serverId);
    if (!serverInfo) return;
    
    this.processes.delete(serverId);
    this.messageBuffers.delete(serverId);
    
    if (serverInfo.status === McpServerStatus.Stopping) {
      // Expected exit
      serverInfo.status = McpServerStatus.Stopped;
      serverInfo.process = undefined;
      serverInfo.connection = undefined;
    } else {
      // Unexpected exit
      serverInfo.status = McpServerStatus.Crashed;
      serverInfo.lastError = `Process exited with code ${code} signal ${signal}`;
      this.emitServerEvent(serverId, McpServerEventType.Disconnected);
      
      // Attempt restart if configured
      if (serverInfo.definition.maxRestarts && 
          serverInfo.restartCount < serverInfo.definition.maxRestarts) {
        serverInfo.restartCount++;
        console.log(`Attempting to restart ${serverId} (attempt ${serverInfo.restartCount})`);
        
        setTimeout(() => {
          this.startServer(serverId).catch(error => {
            console.error(`Failed to restart ${serverId}:`, error);
          });
        }, 1000 * serverInfo.restartCount); // Exponential backoff
      }
    }
  }
  
  private async waitForInitialization(serverId: string, timeout?: number): Promise<void> {
    const startTime = Date.now();
    const maxWait = timeout || this.options.defaultTimeout || 30000;
    
    return new Promise((resolve, reject) => {
      const checkInterval = setInterval(() => {
        const serverInfo = this.servers.get(serverId);
        const process = this.processes.get(serverId);
        
        if (!serverInfo || !process) {
          clearInterval(checkInterval);
          reject(new Error('Server or process not found'));
          return;
        }
        
        // Check if process is still running
        try {
          process.kill(0); // Check if process exists
        } catch {
          clearInterval(checkInterval);
          reject(new Error('Process died during initialization'));
          return;
        }
        
        // For now, we'll just wait a bit and assume it's ready
        // In a real implementation, we'd wait for an initialization message
        if (Date.now() - startTime > 1000) {
          clearInterval(checkInterval);
          resolve();
        }
        
        if (Date.now() - startTime > maxWait) {
          clearInterval(checkInterval);
          reject(new Error('Initialization timeout'));
        }
      }, 100);
    });
  }
  
  private async requestCapabilities(serverId: string): Promise<void> {
    try {
      const capabilities = await this.sendRequest(serverId, 'initialize', {
        clientInfo: {
          name: 'SymbioteIDE',
          version: '1.0.0'
        }
      });
      
      const serverInfo = this.servers.get(serverId);
      if (serverInfo) {
        serverInfo.capabilities = capabilities;
        this.emitServerEvent(serverId, McpServerEventType.CapabilitiesUpdated, capabilities);
      }
    } catch (error) {
      console.error(`Failed to get capabilities for ${serverId}:`, error);
    }
  }
  
  private startHealthChecks(): void {
    const interval = this.options.healthCheckInterval || 30000;
    
    this.healthCheckInterval = setInterval(() => {
      for (const [serverId, serverInfo] of this.servers) {
        if (serverInfo.status === McpServerStatus.Running) {
          this.performHealthCheck(serverId).catch(console.error);
        }
      }
    }, interval);
  }
  
  private async performHealthCheck(serverId: string): Promise<void> {
    try {
      const startTime = Date.now();
      await this.sendRequest(serverId, 'ping', {});
      const latency = Date.now() - startTime;
      
      const serverInfo = this.servers.get(serverId);
      if (serverInfo && serverInfo.connection) {
        serverInfo.connection.lastPing = new Date();
        serverInfo.connection.latency = latency;
      }
    } catch (error) {
      console.error(`Health check failed for ${serverId}:`, error);
      // Could trigger reconnection logic here
    }
  }
  
  private trackMessage(serverId: string, direction: 'incoming' | 'outgoing', data: any): void {
    this.emitServerEvent(
      serverId,
      direction === 'incoming' ? McpServerEventType.MessageReceived : McpServerEventType.MessageSent,
      data
    );
  }
  
  private emitServerEvent(serverId: string, type: McpServerEventType, data?: any): void {
    const event: McpServerEvent = {
      serverId,
      type,
      data,
      timestamp: new Date()
    };
    
    this.emit('server-event', event);
    this.emit(`server-event-${serverId}`, event);
  }
  
  private generateRequestId(): string {
    return `req-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
  
  /**
   * Set up secure process handlers with monitoring
   */
  private setupSecureProcessHandlers(serverId: string, process: ChildProcess): void {
    // Use existing setupProcessHandlers but add security monitoring
    this.setupProcessHandlers(serverId, process);
    
    // Add security monitoring
    const serverInfo = this.servers.get(serverId);
    if (!serverInfo) return;
    
    // Monitor resource usage
    const monitorInterval = setInterval(async () => {
      if (serverInfo.status !== McpServerStatus.Running) {
        clearInterval(monitorInterval);
        return;
      }
      
      const metrics = await this.securityManager.getServerMetrics(serverId);
      
      // Check for violations
      if (metrics.cpuUsage > 80) {
        await this.securityManager.reportViolation(serverId, 'high_cpu_usage', {
          usage: metrics.cpuUsage
        });
      }
      
      if (metrics.memoryUsage > 500 * 1024 * 1024) { // 500MB
        await this.securityManager.reportViolation(serverId, 'high_memory_usage', {
          usage: metrics.memoryUsage
        });
      }
    }, 5000); // Check every 5 seconds
    
    // Clean up on process exit
    process.once('exit', () => {
      clearInterval(monitorInterval);
    });
  }
  
  /**
   * Show security details dialog
   */
  private async showSecurityDetails(alert: any): Promise<void> {
    const panel = vscode.window.createWebviewPanel(
      'mcpSecurityAlert',
      'MCP Security Alert',
      vscode.ViewColumn.One,
      {
        enableScripts: true
      }
    );
    
    panel.webview.html = this.getSecurityAlertHtml(alert);
  }
  
  /**
   * Get security alert HTML
   */
  private getSecurityAlertHtml(alert: any): string {
    return `<!DOCTYPE html>
    <html>
    <head>
      <style>
        body {
          padding: 20px;
          font-family: var(--vscode-font-family);
          color: var(--vscode-foreground);
        }
        .alert {
          background: var(--vscode-inputValidation-errorBackground);
          border: 1px solid var(--vscode-inputValidation-errorBorder);
          padding: 15px;
          border-radius: 4px;
          margin-bottom: 20px;
        }
        .details {
          margin-top: 20px;
        }
        .actions {
          margin-top: 20px;
        }
        button {
          background: var(--vscode-button-background);
          color: var(--vscode-button-foreground);
          border: none;
          padding: 8px 16px;
          margin-right: 10px;
          cursor: pointer;
        }
        button:hover {
          background: var(--vscode-button-hoverBackground);
        }
      </style>
    </head>
    <body>
      <div class="alert">
        <h2>Security Alert</h2>
        <p><strong>Server:</strong> ${alert.serverName}</p>
        <p><strong>Type:</strong> ${alert.type}</p>
        <p><strong>Message:</strong> ${alert.message}</p>
      </div>
      
      <div class="details">
        <h3>Details</h3>
        <pre>${JSON.stringify(alert.details, null, 2)}</pre>
      </div>
      
      <div class="actions">
        <button onclick="trustServer()">Trust Server</button>
        <button onclick="blockServer()">Block Server</button>
        <button onclick="viewLogs()">View Logs</button>
      </div>
      
      <script>
        const vscode = acquireVsCodeApi();
        
        function trustServer() {
          vscode.postMessage({ command: 'trustServer', serverId: '${alert.serverName}' });
        }
        
        function blockServer() {
          vscode.postMessage({ command: 'blockServer', serverId: '${alert.serverName}' });
        }
        
        function viewLogs() {
          vscode.postMessage({ command: 'viewLogs', serverId: '${alert.serverName}' });
        }
      </script>
    </body>
    </html>`;
  }
  
  /**
   * Get security status for all servers
   */
  async getSecurityStatus(): Promise<Map<string, any>> {
    const status = new Map();
    
    for (const [serverId, serverInfo] of this.servers) {
      const trustLevel = await this.securityManager.getServerTrust(serverId);
      const metrics = await this.securityManager.getServerMetrics(serverId);
      const violations = await this.securityManager.getServerViolations(serverId);
      
      status.set(serverId, {
        trustLevel,
        metrics,
        violations,
        status: serverInfo.status
      });
    }
    
    return status;
  }
  
  /**
   * Dispose resources
   */
  async dispose(): Promise<void> {
    // Stop all servers
    for (const serverId of this.servers.keys()) {
      await this.stopServer(serverId, true).catch(console.error);
    }
    
    // Clean up security manager
    await this.securityManager.dispose();
    
    // Clear intervals
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
    }
  }
}