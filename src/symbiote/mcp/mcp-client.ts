/**
 * MCP Client
 * 
 * High-level client for interacting with MCP servers
 */

import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import { McpServerManager } from './mcp-server-manager';
import { McpConfigReader } from './mcp-config-reader';
import { SecurityManager } from './security/security-manager';
import { PermissionType } from './security/types';
import {
  McpServerInfo,
  McpServerStatus,
  McpTool,
  McpResource,
  McpPrompt,
  McpToolExecutionRequest,
  McpToolExecutionResult,
  McpResourceReadRequest,
  McpResourceContent,
  McpPromptExecutionRequest,
  McpPromptResult,
  McpServerDefinition,
  McpCapabilities
} from './mcp-types';

export interface McpClientOptions {
  autoStart?: boolean;
  maxConcurrentServers?: number;
  defaultTimeout?: number;
  healthCheckInterval?: number;
}

export class McpClient extends EventEmitter {
  private serverManager: McpServerManager;
  private configReader: McpConfigReader;
  private securityManager?: SecurityManager;
  private initialized: boolean = false;
  
  constructor(
    private context: vscode.ExtensionContext,
    private options: McpClientOptions = {}
  ) {
    super();
    
    this.serverManager = new McpServerManager(context, {
      autoStart: options.autoStart,
      maxConcurrentServers: options.maxConcurrentServers,
      defaultTimeout: options.defaultTimeout,
      healthCheckInterval: options.healthCheckInterval
    });
    
    this.configReader = new McpConfigReader();
    
    // Forward server events
    this.serverManager.on('server-event', (event) => {
      this.emit('server-event', event);
    });
  }
  
  /**
   * Initialize the MCP client
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }
    
    await this.serverManager.initialize();
    
    // Get security manager from server manager
    this.securityManager = (this.serverManager as any).securityManager;
    
    // Watch for configuration changes
    this.configReader.watchConfiguration(async (config) => {
      if (config) {
        await this.handleConfigurationChange(config);
      }
    });
    
    this.initialized = true;
  }
  
  /**
   * List all available tools across all servers
   */
  async listTools(serverId?: string): Promise<McpTool[]> {
    const servers = serverId ? 
      [this.serverManager.getServer(serverId)].filter(Boolean) as McpServerInfo[] :
      this.serverManager.getAllServers();
    
    const tools: McpTool[] = [];
    
    for (const server of servers) {
      if (server.status === McpServerStatus.Running && server.capabilities?.tools) {
        tools.push(...server.capabilities.tools);
      }
    }
    
    return tools;
  }
  
  /**
   * Execute a tool
   */
  async executeTool(request: McpToolExecutionRequest): Promise<McpToolExecutionResult> {
    const startTime = Date.now();
    
    try {
      // Check security permission for tool execution
      if (this.securityManager) {
        const hasPermission = await this.securityManager.checkPermission({
          serverName: request.serverId,
          permission: PermissionType.ExecuteTool,
          resource: request.toolName,
          reason: `Execute tool '${request.toolName}' with arguments`
        });
        
        if (!hasPermission) {
          throw new Error(`Permission denied to execute tool '${request.toolName}'`);
        }
        
        // Scan tool arguments for security issues
        const scanResult = await this.securityManager.scanToolArguments(
          request.serverId,
          request.toolName,
          request.arguments
        );
        
        if (scanResult.blocked) {
          throw new Error(`Security scan blocked tool execution: ${scanResult.reason}`);
        }
        
        // Log audit entry
        await this.securityManager.auditToolExecution(request.serverId, {
          toolName: request.toolName,
          arguments: request.arguments,
          timestamp: new Date()
        });
      }
      
      const result = await this.serverManager.sendRequest(
        request.serverId,
        'tools/call',
        {
          name: request.toolName,
          arguments: request.arguments
        }
      );
      
      // Validate response for security threats
      if (this.securityManager) {
        const responseCheck = await this.securityManager.validateToolResponse(
          request.serverId,
          request.toolName,
          result
        );
        
        if (!responseCheck.safe) {
          throw new Error(`Tool response failed security validation: ${responseCheck.reason}`);
        }
      }
      
      return {
        serverId: request.serverId,
        toolName: request.toolName,
        success: true,
        result,
        duration: Date.now() - startTime
      };
    } catch (error) {
      // Log security failures
      if (this.securityManager && error instanceof Error && error.message.includes('Permission denied')) {
        await this.securityManager.reportViolation(
          request.serverId,
          'permission_denied',
          {
            tool: request.toolName,
            error: error.message
          }
        );
      }
      
      return {
        serverId: request.serverId,
        toolName: request.toolName,
        success: false,
        error: error instanceof Error ? error.message : String(error),
        duration: Date.now() - startTime
      };
    }
  }
  
  /**
   * List all available resources
   */
  async listResources(serverId?: string): Promise<McpResource[]> {
    const servers = serverId ? 
      [this.serverManager.getServer(serverId)].filter(Boolean) as McpServerInfo[] :
      this.serverManager.getAllServers();
    
    const resources: McpResource[] = [];
    
    for (const server of servers) {
      if (server.status === McpServerStatus.Running && server.capabilities?.resources) {
        resources.push(...server.capabilities.resources);
      }
    }
    
    return resources;
  }
  
  /**
   * Read a resource
   */
  async readResource(request: McpResourceReadRequest): Promise<McpResourceContent> {
    // Check security permission for resource access
    if (this.securityManager) {
      const hasPermission = await this.securityManager.checkPermission({
        serverName: request.serverId,
        permission: PermissionType.ReadResource,
        resource: request.uri,
        reason: `Read resource '${request.uri}'`
      });
      
      if (!hasPermission) {
        throw new Error(`Permission denied to read resource '${request.uri}'`);
      }
      
      // Audit resource access
      await this.securityManager.auditResourceAccess(request.serverId, {
        uri: request.uri,
        operation: 'read',
        timestamp: new Date()
      });
    }
    
    const result = await this.serverManager.sendRequest(
      request.serverId,
      'resources/read',
      {
        uri: request.uri
      }
    );
    
    return {
      uri: request.uri,
      content: result.content,
      mimeType: result.mimeType,
      encoding: result.encoding
    };
  }
  
  /**
   * List all available prompts
   */
  async listPrompts(serverId?: string): Promise<McpPrompt[]> {
    const servers = serverId ? 
      [this.serverManager.getServer(serverId)].filter(Boolean) as McpServerInfo[] :
      this.serverManager.getAllServers();
    
    const prompts: McpPrompt[] = [];
    
    for (const server of servers) {
      if (server.status === McpServerStatus.Running && server.capabilities?.prompts) {
        prompts.push(...server.capabilities.prompts);
      }
    }
    
    return prompts;
  }
  
  /**
   * Execute a prompt
   */
  async executePrompt(request: McpPromptExecutionRequest): Promise<McpPromptResult> {
    const result = await this.serverManager.sendRequest(
      request.serverId,
      'prompts/run',
      {
        name: request.promptName,
        arguments: request.arguments
      }
    );
    
    return {
      serverId: request.serverId,
      promptName: request.promptName,
      result: result.prompt
    };
  }
  
  /**
   * Get server status
   */
  getServerStatus(serverId: string): McpServerStatus | undefined {
    const server = this.serverManager.getServer(serverId);
    return server?.status;
  }
  
  /**
   * Get all servers
   */
  getAllServers(): McpServerInfo[] {
    return this.serverManager.getAllServers();
  }
  
  /**
   * Start a server
   */
  async startServer(serverId: string): Promise<void> {
    await this.serverManager.startServer(serverId);
  }
  
  /**
   * Stop a server
   */
  async stopServer(serverId: string, force?: boolean): Promise<void> {
    await this.serverManager.stopServer(serverId, force);
  }
  
  /**
   * Restart a server
   */
  async restartServer(serverId: string): Promise<void> {
    await this.serverManager.restartServer(serverId);
  }
  
  /**
   * Register a new server
   */
  async registerServer(serverId: string, definition: McpServerDefinition): Promise<void> {
    await this.serverManager.registerServer(serverId, definition);
  }
  
  /**
   * Get server capabilities
   */
  getServerCapabilities(serverId: string): McpCapabilities | undefined {
    const server = this.serverManager.getServer(serverId);
    return server?.capabilities;
  }
  
  /**
   * Send raw request to server
   */
  async sendRequest(serverId: string, method: string, params?: any): Promise<any> {
    return await this.serverManager.sendRequest(serverId, method, params);
  }
  
  /**
   * Send raw notification to server
   */
  sendNotification(serverId: string, method: string, params?: any): void {
    this.serverManager.sendNotification(serverId, method, params);
  }
  
  /**
   * Shutdown the client
   */
  async shutdown(): Promise<void> {
    await this.serverManager.shutdown();
  }
  
  /**
   * Handle configuration changes
   */
  private async handleConfigurationChange(config: { mcpServers: Record<string, McpServerDefinition> }): Promise<void> {
    const currentServers = new Set(this.serverManager.getAllServers().map(s => s.id));
    const newServers = new Set(Object.keys(config.mcpServers));
    
    // Stop removed servers
    for (const serverId of currentServers) {
      if (!newServers.has(serverId)) {
        await this.serverManager.stopServer(serverId).catch(console.error);
      }
    }
    
    // Register and start new servers
    for (const [serverId, definition] of Object.entries(config.mcpServers)) {
      if (!currentServers.has(serverId)) {
        await this.serverManager.registerServer(serverId, definition);
        if (definition.autoStart !== false && this.options.autoStart !== false) {
          await this.serverManager.startServer(serverId).catch(console.error);
        }
      }
    }
  }
  
  /**
   * Get tool by name across all servers
   */
  async findTool(toolName: string): Promise<{ serverId: string; tool: McpTool } | undefined> {
    const servers = this.serverManager.getAllServers();
    
    for (const server of servers) {
      if (server.status === McpServerStatus.Running && server.capabilities?.tools) {
        const tool = server.capabilities.tools.find(t => t.name === toolName);
        if (tool) {
          return { serverId: server.id, tool };
        }
      }
    }
    
    return undefined;
  }
  
  /**
   * Execute tool by name (finds server automatically)
   */
  async executeToolByName(toolName: string, args: any, timeout?: number): Promise<McpToolExecutionResult> {
    const toolInfo = await this.findTool(toolName);
    
    if (!toolInfo) {
      return {
        serverId: 'unknown',
        toolName,
        success: false,
        error: `Tool '${toolName}' not found in any server`,
        duration: 0
      };
    }
    
    return this.executeTool({
      serverId: toolInfo.serverId,
      toolName,
      arguments: args,
      timeout
    });
  }
}