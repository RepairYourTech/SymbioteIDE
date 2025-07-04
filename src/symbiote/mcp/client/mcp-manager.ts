/**
 * MCP Manager - Manages multiple MCP client connections
 */

import { MCPClient, MCPClientConfig } from './mcp-client';

export interface MCPServer {
  name: string;
  config: MCPClientConfig;
}

export class MCPManager {
  private static instance: MCPManager;
  private clients: Map<string, MCPClient> = new Map();
  private servers: Map<string, MCPServer> = new Map();

  private constructor() {}

  static getInstance(): MCPManager {
    if (!MCPManager.instance) {
      MCPManager.instance = new MCPManager();
    }
    return MCPManager.instance;
  }

  async initialize(servers: MCPServer[]): Promise<void> {
    for (const server of servers) {
      this.servers.set(server.name, server);
    }
  }

  async getClient(serverName: string): Promise<MCPClient> {
    let client = this.clients.get(serverName);
    
    if (!client) {
      const server = this.servers.get(serverName);
      if (!server) {
        throw new Error(`Server ${serverName} not found`);
      }
      
      client = new MCPClient(server.config);
      await client.connect();
      this.clients.set(serverName, client);
    }
    
    return client;
  }

  async disconnectAll(): Promise<void> {
    for (const [name, client] of this.clients) {
      await client.disconnect();
    }
    this.clients.clear();
  }

  getAvailableServers(): string[] {
    return Array.from(this.servers.keys());
  }

  listServers(): Array<{ name: string; status: string }> {
    return Array.from(this.servers.entries()).map(([name, server]) => ({
      name,
      status: this.clients.has(name) ? 'running' : 'configured'
    }));
  }

  isServerConfigured(serverName: string): boolean {
    return this.servers.has(serverName);
  }
}

export default MCPManager;