/**
 * MCP Tree Data Provider
 * 
 * Provides tree view of MCP servers and their resources
 */

import * as vscode from 'vscode';
import { MCPServerManager } from '../server-manager';

export class MCPTreeDataProvider implements vscode.TreeDataProvider<MCPTreeItem> {
  private _onDidChangeTreeData: vscode.EventEmitter<MCPTreeItem | undefined | null | void> = 
    new vscode.EventEmitter<MCPTreeItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<MCPTreeItem | undefined | null | void> = 
    this._onDidChangeTreeData.event;
  
  private serverManager: MCPServerManager;
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
    
    // Listen for server changes
    this.serverManager.on('servers-changed', () => {
      this.refresh();
    });
  }
  
  /**
   * Refresh tree view
   */
  refresh(element?: MCPTreeItem): void {
    this._onDidChangeTreeData.fire(element);
  }
  
  /**
   * Get tree item
   */
  getTreeItem(element: MCPTreeItem): vscode.TreeItem {
    return element;
  }
  
  /**
   * Get children
   */
  async getChildren(element?: MCPTreeItem): Promise<MCPTreeItem[]> {
    if (!element) {
      // Root level - show servers
      return this.getServers();
    }
    
    if (element.type === 'server') {
      // Server level - show tools, resources, etc.
      return this.getServerChildren(element.serverId!);
    }
    
    if (element.type === 'tools-folder') {
      // Tools folder - show individual tools
      return this.getTools(element.serverId!);
    }
    
    if (element.type === 'resources-folder') {
      // Resources folder - show individual resources
      return this.getResources(element.serverId!);
    }
    
    if (element.type === 'prompts-folder') {
      // Prompts folder - show individual prompts
      return this.getPrompts(element.serverId!);
    }
    
    return [];
  }
  
  /**
   * Get parent
   */
  getParent(element: MCPTreeItem): vscode.ProviderResult<MCPTreeItem> {
    return element.parent;
  }
  
  /**
   * Get servers
   */
  private async getServers(): Promise<MCPTreeItem[]> {
    const servers = this.serverManager.getServers();
    const items: MCPTreeItem[] = [];
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      const isConnected = client?.isConnected() || false;
      
      const item = new MCPTreeItem(
        server.name || serverId,
        vscode.TreeItemCollapsibleState.Collapsed,
        'server',
        serverId
      );
      
      item.contextValue = isConnected ? 'server-running' : 'server-stopped';
      item.iconPath = new vscode.ThemeIcon(
        isConnected ? 'server' : 'server-environment',
        isConnected ? new vscode.ThemeColor('charts.green') : undefined
      );
      
      item.description = isConnected ? 'Connected' : 'Disconnected';
      
      // Add inline actions
      item.command = {
        command: 'symbiote.mcp.showServerInfo',
        title: 'Show Server Info',
        arguments: [serverId]
      };
      
      items.push(item);
    }
    
    // Add "Add Server" item if no servers
    if (items.length === 0) {
      const addItem = new MCPTreeItem(
        'No servers configured',
        vscode.TreeItemCollapsibleState.None,
        'empty'
      );
      
      addItem.command = {
        command: 'symbiote.mcp.addServer',
        title: 'Add Server'
      };
      
      addItem.contextValue = 'empty';
      items.push(addItem);
    }
    
    return items;
  }
  
  /**
   * Get server children
   */
  private async getServerChildren(serverId: string): Promise<MCPTreeItem[]> {
    const client = this.serverManager.getClient(serverId);
    const items: MCPTreeItem[] = [];
    
    if (!client || !client.isConnected()) {
      // Server not connected
      const item = new MCPTreeItem(
        'Server not connected',
        vscode.TreeItemCollapsibleState.None,
        'info'
      );
      
      item.contextValue = 'info';
      item.iconPath = new vscode.ThemeIcon('info');
      items.push(item);
      
      return items;
    }
    
    try {
      // Get server capabilities
      const serverInfo = await client.getServerInfo();
      const capabilities = serverInfo.capabilities || {};
      
      // Tools folder
      if (capabilities.tools) {
        const toolsFolder = new MCPTreeItem(
          'Tools',
          vscode.TreeItemCollapsibleState.Collapsed,
          'tools-folder',
          serverId
        );
        
        toolsFolder.iconPath = new vscode.ThemeIcon('tools');
        toolsFolder.contextValue = 'tools-folder';
        
        // Get tool count
        try {
          const tools = await client.listTools();
          toolsFolder.description = `${tools.length}`;
        } catch {
          // Ignore errors
        }
        
        items.push(toolsFolder);
      }
      
      // Resources folder
      if (capabilities.resources) {
        const resourcesFolder = new MCPTreeItem(
          'Resources',
          vscode.TreeItemCollapsibleState.Collapsed,
          'resources-folder',
          serverId
        );
        
        resourcesFolder.iconPath = new vscode.ThemeIcon('files');
        resourcesFolder.contextValue = 'resources-folder';
        
        // Get resource count
        try {
          const resources = await client.listResources();
          resourcesFolder.description = `${resources.length}`;
        } catch {
          // Ignore errors
        }
        
        items.push(resourcesFolder);
      }
      
      // Prompts folder
      if (capabilities.prompts) {
        const promptsFolder = new MCPTreeItem(
          'Prompts',
          vscode.TreeItemCollapsibleState.Collapsed,
          'prompts-folder',
          serverId
        );
        
        promptsFolder.iconPath = new vscode.ThemeIcon('comment');
        promptsFolder.contextValue = 'prompts-folder';
        
        // Get prompt count
        try {
          const prompts = await client.listPrompts();
          promptsFolder.description = `${prompts.length}`;
        } catch {
          // Ignore errors
        }
        
        items.push(promptsFolder);
      }
      
      // Server info
      const infoItem = new MCPTreeItem(
        'Server Information',
        vscode.TreeItemCollapsibleState.None,
        'info',
        serverId
      );
      
      infoItem.iconPath = new vscode.ThemeIcon('info');
      infoItem.contextValue = 'server-info';
      infoItem.command = {
        command: 'symbiote.mcp.showServerInfo',
        title: 'Show Info',
        arguments: [serverId]
      };
      
      infoItem.description = `${serverInfo.name} v${serverInfo.version}`;
      items.push(infoItem);
      
    } catch (error) {
      // Error getting server info
      const errorItem = new MCPTreeItem(
        'Error loading server info',
        vscode.TreeItemCollapsibleState.None,
        'error'
      );
      
      errorItem.iconPath = new vscode.ThemeIcon('error');
      errorItem.contextValue = 'error';
      errorItem.description = error.message;
      items.push(errorItem);
    }
    
    return items;
  }
  
  /**
   * Get tools
   */
  private async getTools(serverId: string): Promise<MCPTreeItem[]> {
    const client = this.serverManager.getClient(serverId);
    if (!client || !client.isConnected()) return [];
    
    try {
      const tools = await client.listTools();
      
      return tools.map(tool => {
        const item = new MCPTreeItem(
          tool.name,
          vscode.TreeItemCollapsibleState.None,
          'tool',
          serverId
        );
        
        item.iconPath = new vscode.ThemeIcon('symbol-function');
        item.contextValue = 'tool';
        item.description = tool.description;
        
        item.command = {
          command: 'symbiote.mcp.executeTool',
          title: 'Execute Tool',
          arguments: [serverId, tool.name]
        };
        
        // Store tool data
        item.data = tool;
        
        return item;
      });
    } catch (error) {
      return [this.createErrorItem('Failed to load tools', error.message)];
    }
  }
  
  /**
   * Get resources
   */
  private async getResources(serverId: string): Promise<MCPTreeItem[]> {
    const client = this.serverManager.getClient(serverId);
    if (!client || !client.isConnected()) return [];
    
    try {
      const resources = await client.listResources();
      
      // Group resources by type/path
      const grouped = this.groupResources(resources);
      const items: MCPTreeItem[] = [];
      
      for (const [group, groupResources] of grouped) {
        if (grouped.size > 1) {
          // Create folder for group
          const folder = new MCPTreeItem(
            group,
            vscode.TreeItemCollapsibleState.Collapsed,
            'resource-group',
            serverId
          );
          
          folder.iconPath = new vscode.ThemeIcon('folder');
          folder.contextValue = 'resource-group';
          folder.description = `${groupResources.length}`;
          
          // Add resources as children
          folder.children = groupResources.map(resource => {
            const item = new MCPTreeItem(
              resource.name,
              vscode.TreeItemCollapsibleState.None,
              'resource',
              serverId
            );
            
            item.iconPath = this.getResourceIcon(resource);
            item.contextValue = 'resource';
            item.description = resource.description;
            
            item.command = {
              command: 'symbiote.mcp.readResource',
              title: 'Read Resource',
              arguments: [serverId, resource.uri]
            };
            
            item.data = resource;
            item.parent = folder;
            
            return item;
          });
          
          items.push(folder);
        } else {
          // Add resources directly
          for (const resource of groupResources) {
            const item = new MCPTreeItem(
              resource.name,
              vscode.TreeItemCollapsibleState.None,
              'resource',
              serverId
            );
            
            item.iconPath = this.getResourceIcon(resource);
            item.contextValue = 'resource';
            item.description = resource.description;
            
            item.command = {
              command: 'symbiote.mcp.readResource',
              title: 'Read Resource',
              arguments: [serverId, resource.uri]
            };
            
            item.data = resource;
            
            items.push(item);
          }
        }
      }
      
      return items;
    } catch (error) {
      return [this.createErrorItem('Failed to load resources', error.message)];
    }
  }
  
  /**
   * Get prompts
   */
  private async getPrompts(serverId: string): Promise<MCPTreeItem[]> {
    const client = this.serverManager.getClient(serverId);
    if (!client || !client.isConnected()) return [];
    
    try {
      const prompts = await client.listPrompts();
      
      return prompts.map(prompt => {
        const item = new MCPTreeItem(
          prompt.name,
          vscode.TreeItemCollapsibleState.None,
          'prompt',
          serverId
        );
        
        item.iconPath = new vscode.ThemeIcon('comment-discussion');
        item.contextValue = 'prompt';
        item.description = prompt.description;
        
        item.command = {
          command: 'symbiote.mcp.getPrompt',
          title: 'Get Prompt',
          arguments: [serverId, prompt.name]
        };
        
        item.data = prompt;
        
        return item;
      });
    } catch (error) {
      return [this.createErrorItem('Failed to load prompts', error.message)];
    }
  }
  
  /**
   * Group resources by type or path
   */
  private groupResources(resources: any[]): Map<string, any[]> {
    const groups = new Map<string, any[]>();
    
    for (const resource of resources) {
      // Try to extract group from URI
      let group = 'Resources';
      
      if (resource.uri.includes('://')) {
        const parts = resource.uri.split('://');
        group = parts[0];
      } else if (resource.uri.includes('/')) {
        const parts = resource.uri.split('/');
        if (parts.length > 2) {
          group = parts[0];
        }
      }
      
      const existing = groups.get(group) || [];
      existing.push(resource);
      groups.set(group, existing);
    }
    
    return groups;
  }
  
  /**
   * Get resource icon based on type
   */
  private getResourceIcon(resource: any): vscode.ThemeIcon {
    const uri = resource.uri.toLowerCase();
    
    if (uri.endsWith('.json')) return new vscode.ThemeIcon('json');
    if (uri.endsWith('.js') || uri.endsWith('.ts')) return new vscode.ThemeIcon('symbol-file');
    if (uri.endsWith('.md')) return new vscode.ThemeIcon('markdown');
    if (uri.endsWith('.txt')) return new vscode.ThemeIcon('file-text');
    if (uri.endsWith('.yaml') || uri.endsWith('.yml')) return new vscode.ThemeIcon('file-code');
    if (uri.includes('http://') || uri.includes('https://')) return new vscode.ThemeIcon('globe');
    if (uri.includes('file://')) return new vscode.ThemeIcon('file');
    
    return new vscode.ThemeIcon('symbol-misc');
  }
  
  /**
   * Create error item
   */
  private createErrorItem(message: string, details?: string): MCPTreeItem {
    const item = new MCPTreeItem(
      message,
      vscode.TreeItemCollapsibleState.None,
      'error'
    );
    
    item.iconPath = new vscode.ThemeIcon('error');
    item.contextValue = 'error';
    item.description = details;
    
    return item;
  }
}

/**
 * MCP Tree Item
 */
export class MCPTreeItem extends vscode.TreeItem {
  public type: string;
  public serverId?: string;
  public parent?: MCPTreeItem;
  public children?: MCPTreeItem[];
  public data?: any;
  
  constructor(
    label: string,
    collapsibleState: vscode.TreeItemCollapsibleState,
    type: string,
    serverId?: string
  ) {
    super(label, collapsibleState);
    this.type = type;
    this.serverId = serverId;
  }
}