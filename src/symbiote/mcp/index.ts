/**
 * MCP Integration Module
 * 
 * Main entry point for MCP functionality in SymbioteIDE
 */

export * from './mcp-types';
export * from './mcp-config-reader';
export * from './mcp-server-manager';
export * from './mcp-client';
export * from './mcp-orchestration-adapter';
export { MCPExtension } from './extension/mcp-extension';

import * as vscode from 'vscode';
import { McpClient, McpClientOptions } from './mcp-client';
import { McpOrchestrationAdapter } from './mcp-orchestration-adapter';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { McpIntegrationConfig, McpServerInfo, McpTool, McpResource, McpPrompt } from './mcp-types';

/**
 * Main MCP integration class
 */
export class McpIntegration {
  private client: McpClient;
  private adapters: Map<string, McpOrchestrationAdapter> = new Map();
  
  constructor(
    private context: vscode.ExtensionContext,
    private orchestrationEngine: OrchestrationEngine,
    options?: McpClientOptions
  ) {
    this.client = new McpClient(context, options);
  }
  
  /**
   * Initialize MCP integration
   */
  async initialize(): Promise<void> {
    await this.client.initialize();
    
    // Register existing servers with orchestration engine
    const servers = this.client.getAllServers();
    for (const server of servers) {
      if (server.status === 'running') {
        await this.registerServerWithOrchestration(server.id);
      }
    }
    
    // Listen for server events
    this.client.on('server-event', async (event) => {
      if (event.type === 'connected') {
        await this.registerServerWithOrchestration(event.serverId);
      } else if (event.type === 'disconnected' || event.type === 'stopped') {
        await this.unregisterServerFromOrchestration(event.serverId);
      }
    });
  }
  
  /**
   * Register MCP server with orchestration engine
   */
  async registerServerWithOrchestration(
    serverId: string,
    config?: Partial<McpIntegrationConfig>
  ): Promise<void> {
    const integrationConfig: McpIntegrationConfig = {
      serverId,
      ...config
    };
    
    // Create adapter
    const adapter = new McpOrchestrationAdapter(this.client, integrationConfig);
    this.adapters.set(serverId, adapter);
    
    // Register with orchestration engine
    await this.orchestrationEngine.providers.set(`mcp-${serverId}`, adapter);
    
    // Register available models
    const models = await adapter.getAvailableModels();
    for (const model of models) {
      await this.orchestrationEngine.modelRegistry.registerModel(model);
    }
  }
  
  /**
   * Unregister MCP server from orchestration engine
   */
  async unregisterServerFromOrchestration(serverId: string): Promise<void> {
    const adapter = this.adapters.get(serverId);
    if (adapter) {
      await adapter.shutdown();
      this.adapters.delete(serverId);
    }
    
    this.orchestrationEngine.providers.delete(`mcp-${serverId}`);
    
    // TODO: Remove models from registry (would need to implement in ModelRegistry)
  }
  
  /**
   * Get MCP client
   */
  getClient(): McpClient {
    return this.client;
  }
  
  /**
   * List all servers
   */
  getAllServers(): McpServerInfo[] {
    return this.client.getAllServers();
  }
  
  /**
   * List all available tools
   */
  async listTools(serverId?: string): Promise<McpTool[]> {
    return this.client.listTools(serverId);
  }
  
  /**
   * List all available resources
   */
  async listResources(serverId?: string): Promise<McpResource[]> {
    return this.client.listResources(serverId);
  }
  
  /**
   * List all available prompts
   */
  async listPrompts(serverId?: string): Promise<McpPrompt[]> {
    return this.client.listPrompts(serverId);
  }
  
  /**
   * Start a server
   */
  async startServer(serverId: string): Promise<void> {
    await this.client.startServer(serverId);
  }
  
  /**
   * Stop a server
   */
  async stopServer(serverId: string, force?: boolean): Promise<void> {
    await this.client.stopServer(serverId, force);
  }
  
  /**
   * Restart a server
   */
  async restartServer(serverId: string): Promise<void> {
    await this.client.restartServer(serverId);
  }
  
  /**
   * Execute tool directly
   */
  async executeTool(serverId: string, toolName: string, args: any): Promise<any> {
    const result = await this.client.executeTool({
      serverId,
      toolName,
      arguments: args
    });
    
    if (!result.success) {
      throw new Error(result.error || 'Tool execution failed');
    }
    
    return result.result;
  }
  
  /**
   * Shutdown MCP integration
   */
  async shutdown(): Promise<void> {
    // Shutdown all adapters
    for (const adapter of this.adapters.values()) {
      await adapter.shutdown();
    }
    this.adapters.clear();
    
    // Shutdown client
    await this.client.shutdown();
  }
}