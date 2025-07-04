/**
 * MCP Tool Adapter
 * 
 * Adapts MCP tools to work with Google AI ADK agents
 */

import { ADKTool } from './types';
import { MCPClient } from '../mcp/client/mcp-client';
import { MCPManager } from '../mcp/manager';
import { Logger } from '../utils/logger';

export class MCPToolAdapter {
  private logger = new Logger('MCPToolAdapter');
  private mcpManager: MCPManager;
  private toolCache = new Map<string, ADKTool>();

  constructor() {
    this.mcpManager = MCPManager.getInstance();
  }

  /**
   * Initialize the adapter
   */
  async initialize(servers?: any[]): Promise<void> {
    if (servers) {
      await this.mcpManager.initialize(servers);
    }
  }

  /**
   * Adapt an MCP tool to ADK Tool format
   */
  async adaptTool(serverName: string, toolName: string): Promise<ADKTool> {
    const cacheKey = `${serverName}:${toolName}`;
    
    // Check cache
    if (this.toolCache.has(cacheKey)) {
      return this.toolCache.get(cacheKey)!;
    }

    try {
      // Get MCP client for server
      const client = await this.mcpManager.getClient(serverName);
      if (!client) {
        throw new Error(`MCP server ${serverName} not found`);
      }

      // Get tool definition from server
      const tools = await client.listTools();
      const mcpTool = tools.find((t: any) => t.name === toolName);
      
      if (!mcpTool) {
        throw new Error(`Tool ${toolName} not found on server ${serverName}`);
      }

      // Create ADK tool wrapper
      const adkTool: ADKTool = {
        name: mcpTool.name,
        description: mcpTool.description || '',
        parameters: this.convertParameters(mcpTool.inputSchema),
        execute: async (params: any) => {
          // Execute MCP tool
          const result = await client.callTool(toolName, params);

          // Convert result format
          return this.convertResult(result);
        }
      };

      // Cache the adapted tool
      this.toolCache.set(cacheKey, adkTool);
      
      this.logger.info(`Adapted MCP tool ${serverName}:${toolName} for ADK`);
      
      return adkTool;
    } catch (error) {
      this.logger.error(`Failed to adapt MCP tool ${serverName}:${toolName}`, error);
      throw error;
    }
  }

  /**
   * Adapt all tools from an MCP server
   */
  async adaptAllTools(serverName: string): Promise<ADKTool[]> {
    try {
      const client = await this.mcpManager.getClient(serverName);
      if (!client) {
        throw new Error(`MCP server ${serverName} not found`);
      }

      const tools = await client.listTools();
      const adaptedTools: ADKTool[] = [];

      for (const mcpTool of tools) {
        const adkTool = await this.adaptTool(serverName, mcpTool.name);
        adaptedTools.push(adkTool);
      }

      return adaptedTools;
    } catch (error) {
      this.logger.error(`Failed to adapt tools from server ${serverName}`, error);
      throw error;
    }
  }

  /**
   * Convert MCP parameter schema to ADK format
   */
  private convertParameters(inputSchema?: any): any[] {
    if (!inputSchema || inputSchema.type !== 'object') {
      return [];
    }

    const parameters: any[] = [];
    const properties = inputSchema.properties || {};
    const required = inputSchema.required || [];

    for (const [name, schema] of Object.entries(properties)) {
      parameters.push({
        name,
        type: this.convertType((schema as any).type),
        description: (schema as any).description,
        required: required.includes(name),
        default: (schema as any).default
      });
    }

    return parameters;
  }

  /**
   * Convert JSON schema type to ADK type
   */
  private convertType(jsonType: string): string {
    const typeMap: Record<string, string> = {
      'string': 'string',
      'number': 'number',
      'integer': 'number',
      'boolean': 'boolean',
      'object': 'object',
      'array': 'array'
    };

    return typeMap[jsonType] || 'string';
  }

  /**
   * Convert MCP result to ADK format
   */
  private convertResult(mcpResult: any): any {
    // MCP results typically have content array
    if (mcpResult.content && Array.isArray(mcpResult.content)) {
      // Extract text content
      const textContent = mcpResult.content
        .filter((item: any) => item.type === 'text')
        .map((item: any) => item.text)
        .join('\n');

      // Include any other content types
      const otherContent = mcpResult.content
        .filter((item: any) => item.type !== 'text');

      if (otherContent.length === 0) {
        return textContent;
      }

      return {
        text: textContent,
        content: otherContent
      };
    }

    return mcpResult;
  }

  /**
   * Create a dynamic tool that discovers MCP tools at runtime
   */
  createDynamicMCPTool(): ADKTool {
    return {
      name: 'use_mcp_tool',
      description: 'Use any available MCP tool by specifying server and tool name',
      parameters: [
        {
          name: 'server',
          type: 'string',
          description: 'MCP server name',
          required: true
        },
        {
          name: 'tool',
          type: 'string', 
          description: 'Tool name on the server',
          required: true
        },
        {
          name: 'parameters',
          type: 'object',
          description: 'Parameters to pass to the tool',
          required: false
        }
      ],
      execute: async (params: any) => {
        const { server, tool, parameters = {} } = params;
        
        // Get or create adapted tool
        const adkTool = await this.adaptTool(server, tool);
        
        // Execute with provided parameters
        return adkTool.execute ? adkTool.execute(parameters) : null;
      }
    };
  }

  /**
   * Get list of available MCP servers and their tools
   */
  async getAvailableTools(): Promise<Record<string, string[]>> {
    const servers = await this.mcpManager.listServers();
    const available: Record<string, string[]> = {};

    for (const server of servers) {
      try {
        const client = await this.mcpManager.getClient(server.name);
        if (client && server.status === 'running') {
          const tools = await client.listTools();
          available[server.name] = tools.map((t: any) => t.name);
        }
      } catch (error) {
        this.logger.warn(`Failed to get tools from ${server.name}`, error);
        available[server.name] = [];
      }
    }

    return available;
  }

  /**
   * Clear tool cache
   */
  clearCache(): void {
    this.toolCache.clear();
  }
}