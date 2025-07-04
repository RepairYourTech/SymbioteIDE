/**
 * MCP Command Registry
 * 
 * Handles all MCP extension commands
 */

import * as vscode from 'vscode';
import { MCPExtension } from './mcp-extension';
import { MCPServerConfig } from './configuration-manager';

export class MCPCommandRegistry {
  private extension: MCPExtension;
  
  constructor(extension: MCPExtension) {
    this.extension = extension;
  }
  
  /**
   * Start MCP server
   */
  async startServer(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectServer('Select server to start');
      if (!serverId) return;
    }
    
    try {
      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `Starting MCP server ${serverId}...`,
        cancellable: false
      }, async () => {
        await serverManager.startServer(serverId!);
      });
      
      vscode.window.showInformationMessage(`MCP server ${serverId} started`);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to start server: ${error.message}`);
    }
  }
  
  /**
   * Stop MCP server
   */
  async stopServer(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectRunningServer('Select server to stop');
      if (!serverId) return;
    }
    
    try {
      await serverManager.stopServer(serverId);
      vscode.window.showInformationMessage(`MCP server ${serverId} stopped`);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to stop server: ${error.message}`);
    }
  }
  
  /**
   * Restart MCP server
   */
  async restartServer(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectRunningServer('Select server to restart');
      if (!serverId) return;
    }
    
    try {
      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `Restarting MCP server ${serverId}...`,
        cancellable: false
      }, async () => {
        await serverManager.stopServer(serverId!);
        await new Promise(resolve => setTimeout(resolve, 1000)); // Brief pause
        await serverManager.startServer(serverId!);
      });
      
      vscode.window.showInformationMessage(`MCP server ${serverId} restarted`);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to restart server: ${error.message}`);
    }
  }
  
  /**
   * Add new MCP server
   */
  async addServer(): Promise<void> {
    const configManager = this.extension.getConfigurationManager();
    
    // Multi-step input
    const serverType = await vscode.window.showQuickPick([
      { label: 'Local Process', value: 'process', description: 'Run a local executable' },
      { label: 'HTTP Server', value: 'http', description: 'Connect to HTTP endpoint' },
      { label: 'WebSocket Server', value: 'websocket', description: 'Connect to WebSocket endpoint' },
      { label: 'From Template', value: 'template', description: 'Use a server template' }
    ], {
      placeHolder: 'Select server type'
    });
    
    if (!serverType) return;
    
    let server: MCPServerConfig;
    
    switch (serverType.value) {
      case 'process':
        server = await this.createProcessServer();
        break;
      case 'http':
        server = await this.createHttpServer();
        break;
      case 'websocket':
        server = await this.createWebSocketServer();
        break;
      case 'template':
        server = await this.createServerFromTemplate();
        break;
      default:
        return;
    }
    
    if (!server) return;
    
    // Validate server
    const errors = configManager.validateServer(server);
    if (errors.length > 0) {
      vscode.window.showErrorMessage(`Invalid server configuration: ${errors.join(', ')}`);
      return;
    }
    
    // Add server
    await configManager.addOrUpdateServer(server);
    
    // Ask if should start immediately
    const start = await vscode.window.showQuickPick(['Yes', 'No'], {
      placeHolder: 'Start server now?'
    });
    
    if (start === 'Yes') {
      await this.startServer(server.id);
    }
  }
  
  /**
   * Remove MCP server
   */
  async removeServer(serverId?: string): Promise<void> {
    const configManager = this.extension.getConfigurationManager();
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectServer('Select server to remove');
      if (!serverId) return;
    }
    
    const server = configManager.getServer(serverId);
    if (!server) return;
    
    const confirm = await vscode.window.showWarningMessage(
      `Remove MCP server '${server.name}'?`,
      'Remove',
      'Cancel'
    );
    
    if (confirm !== 'Remove') return;
    
    // Stop server if running
    const client = serverManager.getClient(serverId);
    if (client && client.isConnected()) {
      await serverManager.stopServer(serverId);
    }
    
    // Remove from configuration
    await configManager.removeServer(serverId);
    
    vscode.window.showInformationMessage(`MCP server '${server.name}' removed`);
  }
  
  /**
   * Edit MCP server
   */
  async editServer(serverId?: string): Promise<void> {
    const configManager = this.extension.getConfigurationManager();
    
    if (!serverId) {
      serverId = await this.selectServer('Select server to edit');
      if (!serverId) return;
    }
    
    const server = configManager.getServer(serverId);
    if (!server) return;
    
    // Show edit options
    const action = await vscode.window.showQuickPick([
      { label: 'Edit Name', value: 'name' },
      { label: 'Edit Command', value: 'command' },
      { label: 'Edit Arguments', value: 'args' },
      { label: 'Edit Environment', value: 'env' },
      { label: 'Edit Working Directory', value: 'cwd' },
      { label: 'Edit Trust Level', value: 'trust' },
      { label: 'Edit Auto-start', value: 'autostart' },
      { label: 'Open in Editor', value: 'editor' }
    ], {
      placeHolder: 'What would you like to edit?'
    });
    
    if (!action) return;
    
    switch (action.value) {
      case 'name':
        const name = await vscode.window.showInputBox({
          prompt: 'Server name',
          value: server.name
        });
        if (name) {
          server.name = name;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'command':
        const command = await vscode.window.showInputBox({
          prompt: 'Server command',
          value: server.command
        });
        if (command !== undefined) {
          server.command = command;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'args':
        const argsStr = await vscode.window.showInputBox({
          prompt: 'Server arguments (comma-separated)',
          value: server.args?.join(', ')
        });
        if (argsStr !== undefined) {
          server.args = argsStr.split(',').map(a => a.trim()).filter(a => a);
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'env':
        await this.editServerEnvironment(server);
        break;
        
      case 'cwd':
        const folders = await vscode.window.showOpenDialog({
          canSelectFiles: false,
          canSelectFolders: true,
          canSelectMany: false,
          defaultUri: server.workingDirectory ? vscode.Uri.file(server.workingDirectory) : undefined
        });
        if (folders && folders.length > 0) {
          server.workingDirectory = folders[0].fsPath;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'trust':
        const trustLevel = await vscode.window.showQuickPick([
          'trusted', 'verified', 'unknown', 'restricted', 'untrusted'
        ], {
          placeHolder: 'Select trust level',
          activeItems: server.trustLevel ? [server.trustLevel] : undefined
        });
        if (trustLevel) {
          server.trustLevel = trustLevel;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'autostart':
        server.autoStart = !server.autoStart;
        await configManager.addOrUpdateServer(server);
        vscode.window.showInformationMessage(
          `Auto-start ${server.autoStart ? 'enabled' : 'disabled'} for ${server.name}`
        );
        break;
        
      case 'editor':
        await this.openServerInEditor(server);
        break;
    }
  }
  
  /**
   * Execute MCP tool
   */
  async executeTool(serverId?: string, toolName?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectRunningServer('Select server');
      if (!serverId) return;
    }
    
    const client = serverManager.getClient(serverId);
    if (!client || !client.isConnected()) {
      vscode.window.showErrorMessage('Server is not running');
      return;
    }
    
    // Get available tools
    const tools = await client.listTools();
    
    if (!toolName) {
      const selected = await vscode.window.showQuickPick(
        tools.map(tool => ({
          label: tool.name,
          description: tool.description,
          tool
        })),
        {
          placeHolder: 'Select tool to execute'
        }
      );
      
      if (!selected) return;
      toolName = selected.tool.name;
    }
    
    const tool = tools.find(t => t.name === toolName);
    if (!tool) {
      vscode.window.showErrorMessage(`Tool '${toolName}' not found`);
      return;
    }
    
    // Get tool arguments
    const args = await this.getToolArguments(tool);
    if (args === undefined) return;
    
    try {
      const result = await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `Executing ${tool.name}...`,
        cancellable: false
      }, async () => {
        return await client.callTool(tool.name, args);
      });
      
      // Show result
      const doc = await vscode.workspace.openTextDocument({
        content: JSON.stringify(result, null, 2),
        language: 'json'
      });
      
      await vscode.window.showTextDocument(doc);
    } catch (error) {
      vscode.window.showErrorMessage(`Tool execution failed: ${error.message}`);
    }
  }
  
  /**
   * List MCP tools
   */
  async listTools(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectRunningServer('Select server');
      if (!serverId) return;
    }
    
    const client = serverManager.getClient(serverId);
    if (!client || !client.isConnected()) {
      vscode.window.showErrorMessage('Server is not running');
      return;
    }
    
    try {
      const tools = await client.listTools();
      
      const doc = await vscode.workspace.openTextDocument({
        content: JSON.stringify(tools, null, 2),
        language: 'json'
      });
      
      await vscode.window.showTextDocument(doc);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to list tools: ${error.message}`);
    }
  }
  
  /**
   * List MCP resources
   */
  async listResources(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectRunningServer('Select server');
      if (!serverId) return;
    }
    
    const client = serverManager.getClient(serverId);
    if (!client || !client.isConnected()) {
      vscode.window.showErrorMessage('Server is not running');
      return;
    }
    
    try {
      const resources = await client.listResources();
      
      const doc = await vscode.workspace.openTextDocument({
        content: JSON.stringify(resources, null, 2),
        language: 'json'
      });
      
      await vscode.window.showTextDocument(doc);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to list resources: ${error.message}`);
    }
  }
  
  /**
   * Read MCP resource
   */
  async readResource(serverId?: string, uri?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectRunningServer('Select server');
      if (!serverId) return;
    }
    
    const client = serverManager.getClient(serverId);
    if (!client || !client.isConnected()) {
      vscode.window.showErrorMessage('Server is not running');
      return;
    }
    
    if (!uri) {
      // List resources first
      const resources = await client.listResources();
      
      const selected = await vscode.window.showQuickPick(
        resources.map(resource => ({
          label: resource.name,
          description: resource.uri,
          detail: resource.description,
          resource
        })),
        {
          placeHolder: 'Select resource to read'
        }
      );
      
      if (!selected) return;
      uri = selected.resource.uri;
    }
    
    try {
      const content = await client.readResource(uri);
      
      // Determine language from MIME type
      let language = 'plaintext';
      if (content.mimeType) {
        if (content.mimeType.includes('json')) language = 'json';
        else if (content.mimeType.includes('javascript')) language = 'javascript';
        else if (content.mimeType.includes('typescript')) language = 'typescript';
        else if (content.mimeType.includes('python')) language = 'python';
        else if (content.mimeType.includes('markdown')) language = 'markdown';
      }
      
      const doc = await vscode.workspace.openTextDocument({
        content: typeof content.contents === 'string' ? 
          content.contents : 
          JSON.stringify(content.contents, null, 2),
        language
      });
      
      await vscode.window.showTextDocument(doc);
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to read resource: ${error.message}`);
    }
  }
  
  /**
   * Show server info
   */
  async showServerInfo(serverId?: string): Promise<void> {
    const configManager = this.extension.getConfigurationManager();
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectServer('Select server');
      if (!serverId) return;
    }
    
    const server = configManager.getServer(serverId);
    if (!server) return;
    
    const client = serverManager.getClient(serverId);
    const isRunning = client && client.isConnected();
    
    let info = `# MCP Server: ${server.name}\n\n`;
    info += `**ID**: ${server.id}\n`;
    info += `**Status**: ${isRunning ? '🟢 Running' : '🔴 Stopped'}\n`;
    info += `**Type**: ${server.transport || 'stdio'}\n`;
    
    if (server.description) {
      info += `**Description**: ${server.description}\n`;
    }
    
    if (server.command) {
      info += `\n## Command\n\`\`\`\n${server.command} ${server.args?.join(' ') || ''}\n\`\`\`\n`;
    } else if (server.url) {
      info += `\n## URL\n${server.url}\n`;
    }
    
    if (server.workingDirectory) {
      info += `\n## Working Directory\n${server.workingDirectory}\n`;
    }
    
    if (server.env && Object.keys(server.env).length > 0) {
      info += `\n## Environment Variables\n\`\`\`json\n${JSON.stringify(server.env, null, 2)}\n\`\`\`\n`;
    }
    
    if (isRunning && client) {
      try {
        const serverInfo = await client.getServerInfo();
        info += `\n## Server Information\n\`\`\`json\n${JSON.stringify(serverInfo, null, 2)}\n\`\`\`\n`;
      } catch {
        // Server doesn't support info
      }
    }
    
    const doc = await vscode.workspace.openTextDocument({
      content: info,
      language: 'markdown'
    });
    
    await vscode.window.showTextDocument(doc);
  }
  
  /**
   * Show server logs
   */
  async showLogs(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectServer('Select server');
      if (!serverId) return;
    }
    
    const outputChannel = serverManager.getOutputChannel(serverId);
    if (outputChannel) {
      outputChannel.show();
    } else {
      vscode.window.showInformationMessage('No logs available for this server');
    }
  }
  
  /**
   * Create server project
   */
  async createServerProject(): Promise<void> {
    const template = await vscode.window.showQuickPick([
      { label: 'Basic TypeScript Server', value: 'typescript' },
      { label: 'Basic JavaScript Server', value: 'javascript' },
      { label: 'Python Server', value: 'python' },
      { label: 'Go Server', value: 'go' },
      { label: 'From Example', value: 'example' }
    ], {
      placeHolder: 'Select server template'
    });
    
    if (!template) return;
    
    const folders = await vscode.window.showOpenDialog({
      canSelectFiles: false,
      canSelectFolders: true,
      canSelectMany: false,
      openLabel: 'Select Project Location'
    });
    
    if (!folders || folders.length === 0) return;
    
    const projectPath = folders[0].fsPath;
    const projectName = await vscode.window.showInputBox({
      prompt: 'Project name',
      value: 'my-mcp-server'
    });
    
    if (!projectName) return;
    
    // Create project
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Creating MCP server project...',
      cancellable: false
    }, async () => {
      // TODO: Implement project creation
      await new Promise(resolve => setTimeout(resolve, 2000));
    });
    
    // Open project
    const projectUri = vscode.Uri.file(projectPath);
    await vscode.commands.executeCommand('vscode.openFolder', projectUri);
  }
  
  /**
   * Test MCP server
   */
  async testServer(serverId?: string): Promise<void> {
    const serverManager = this.extension.getServerManager();
    
    if (!serverId) {
      serverId = await this.selectServer('Select server to test');
      if (!serverId) return;
    }
    
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Testing MCP server...',
      cancellable: false
    }, async (progress) => {
      try {
        // Start server if not running
        progress.report({ message: 'Starting server...' });
        const client = serverManager.getClient(serverId!);
        if (!client || !client.isConnected()) {
          await serverManager.startServer(serverId!);
        }
        
        // Test connection
        progress.report({ message: 'Testing connection...' });
        const info = await serverManager.getClient(serverId!)?.getServerInfo();
        
        // Test tools
        progress.report({ message: 'Testing tools...' });
        const tools = await serverManager.getClient(serverId!)?.listTools();
        
        // Test resources
        progress.report({ message: 'Testing resources...' });
        const resources = await serverManager.getClient(serverId!)?.listResources();
        
        // Show results
        const results = {
          connection: 'OK',
          serverInfo: info,
          toolsCount: tools?.length || 0,
          resourcesCount: resources?.length || 0
        };
        
        vscode.window.showInformationMessage(
          `Server test passed: ${results.toolsCount} tools, ${results.resourcesCount} resources`
        );
        
      } catch (error) {
        vscode.window.showErrorMessage(`Server test failed: ${error.message}`);
      }
    });
  }
  
  /**
   * Show quick actions
   */
  async showQuickActions(): Promise<void> {
    const serverManager = this.extension.getServerManager();
    const servers = this.extension.getConfigurationManager().getServers();
    
    const actions: any[] = [];
    
    // Add server actions
    for (const server of servers) {
      const client = serverManager.getClient(server.id);
      const isRunning = client && client.isConnected();
      
      actions.push({
        label: `${isRunning ? '🟢' : '🔴'} ${server.name}`,
        description: isRunning ? 'Stop server' : 'Start server',
        action: async () => {
          if (isRunning) {
            await this.stopServer(server.id);
          } else {
            await this.startServer(server.id);
          }
        }
      });
    }
    
    // Add global actions
    actions.push(
      { label: '➕ Add Server', action: () => this.addServer() },
      { label: '🔧 Open Settings', action: () => vscode.commands.executeCommand('symbiote.mcp.openSettings') },
      { label: '📊 Show Panel', action: () => vscode.commands.executeCommand('symbiote.mcp.showPanel') }
    );
    
    const selected = await vscode.window.showQuickPick(actions, {
      placeHolder: 'MCP Quick Actions'
    });
    
    if (selected) {
      await selected.action();
    }
  }
  
  // Helper methods
  
  /**
   * Select server from list
   */
  private async selectServer(placeHolder: string): Promise<string | undefined> {
    const servers = this.extension.getConfigurationManager().getServers();
    
    if (servers.length === 0) {
      vscode.window.showInformationMessage('No MCP servers configured');
      return undefined;
    }
    
    const selected = await vscode.window.showQuickPick(
      servers.map(server => ({
        label: server.name,
        description: server.id,
        server
      })),
      { placeHolder }
    );
    
    return selected?.server.id;
  }
  
  /**
   * Select running server
   */
  private async selectRunningServer(placeHolder: string): Promise<string | undefined> {
    const serverManager = this.extension.getServerManager();
    const servers = this.extension.getConfigurationManager().getServers();
    
    const runningServers = servers.filter(server => {
      const client = serverManager.getClient(server.id);
      return client && client.isConnected();
    });
    
    if (runningServers.length === 0) {
      vscode.window.showInformationMessage('No running MCP servers');
      return undefined;
    }
    
    const selected = await vscode.window.showQuickPick(
      runningServers.map(server => ({
        label: server.name,
        description: server.id,
        server
      })),
      { placeHolder }
    );
    
    return selected?.server.id;
  }
  
  /**
   * Create process server configuration
   */
  private async createProcessServer(): Promise<MCPServerConfig | undefined> {
    const id = await vscode.window.showInputBox({
      prompt: 'Server ID (unique identifier)',
      placeHolder: 'my-server',
      validateInput: (value) => {
        if (!value) return 'ID is required';
        if (!/^[a-z0-9-]+$/.test(value)) return 'ID must contain only lowercase letters, numbers, and hyphens';
        return null;
      }
    });
    
    if (!id) return undefined;
    
    const name = await vscode.window.showInputBox({
      prompt: 'Server name',
      placeHolder: 'My MCP Server'
    });
    
    if (!name) return undefined;
    
    const command = await vscode.window.showInputBox({
      prompt: 'Command to run',
      placeHolder: 'node, python, etc.'
    });
    
    if (!command) return undefined;
    
    const argsStr = await vscode.window.showInputBox({
      prompt: 'Arguments (optional, comma-separated)',
      placeHolder: 'server.js, --port, 3000'
    });
    
    const args = argsStr ? argsStr.split(',').map(a => a.trim()) : [];
    
    return {
      id,
      name,
      command,
      args,
      transport: 'stdio'
    };
  }
  
  /**
   * Create HTTP server configuration
   */
  private async createHttpServer(): Promise<MCPServerConfig | undefined> {
    const id = await vscode.window.showInputBox({
      prompt: 'Server ID (unique identifier)',
      placeHolder: 'my-http-server'
    });
    
    if (!id) return undefined;
    
    const name = await vscode.window.showInputBox({
      prompt: 'Server name',
      placeHolder: 'My HTTP Server'
    });
    
    if (!name) return undefined;
    
    const url = await vscode.window.showInputBox({
      prompt: 'Server URL',
      placeHolder: 'http://localhost:3000'
    });
    
    if (!url) return undefined;
    
    return {
      id,
      name,
      url,
      transport: 'http'
    };
  }
  
  /**
   * Create WebSocket server configuration
   */
  private async createWebSocketServer(): Promise<MCPServerConfig | undefined> {
    const id = await vscode.window.showInputBox({
      prompt: 'Server ID (unique identifier)',
      placeHolder: 'my-ws-server'
    });
    
    if (!id) return undefined;
    
    const name = await vscode.window.showInputBox({
      prompt: 'Server name',
      placeHolder: 'My WebSocket Server'
    });
    
    if (!name) return undefined;
    
    const url = await vscode.window.showInputBox({
      prompt: 'Server URL',
      placeHolder: 'ws://localhost:3000'
    });
    
    if (!url) return undefined;
    
    return {
      id,
      name,
      url,
      transport: 'websocket'
    };
  }
  
  /**
   * Create server from template
   */
  private async createServerFromTemplate(): Promise<MCPServerConfig | undefined> {
    // TODO: Implement template selection
    vscode.window.showInformationMessage('Server templates not yet implemented');
    return undefined;
  }
  
  /**
   * Get tool arguments
   */
  private async getToolArguments(tool: any): Promise<any> {
    if (!tool.inputSchema || tool.inputSchema.type !== 'object') {
      return {};
    }
    
    const args: any = {};
    const properties = tool.inputSchema.properties || {};
    const required = tool.inputSchema.required || [];
    
    for (const [key, schema] of Object.entries(properties)) {
      const isRequired = required.includes(key);
      const schemaObj = schema as any;
      
      let value: any;
      
      switch (schemaObj.type) {
        case 'string':
          value = await vscode.window.showInputBox({
            prompt: `${key}${isRequired ? ' (required)' : ''}`,
            placeHolder: schemaObj.description || schemaObj.default
          });
          break;
          
        case 'number':
        case 'integer':
          const numStr = await vscode.window.showInputBox({
            prompt: `${key}${isRequired ? ' (required)' : ''}`,
            placeHolder: schemaObj.description || schemaObj.default?.toString(),
            validateInput: (v) => {
              if (!v && !isRequired) return null;
              if (isNaN(Number(v))) return 'Must be a number';
              return null;
            }
          });
          value = numStr ? Number(numStr) : undefined;
          break;
          
        case 'boolean':
          const boolChoice = await vscode.window.showQuickPick(['true', 'false'], {
            placeHolder: `${key}${isRequired ? ' (required)' : ''}`
          });
          value = boolChoice === 'true';
          break;
          
        default:
          // Complex types - use JSON input
          const jsonStr = await vscode.window.showInputBox({
            prompt: `${key} (JSON)${isRequired ? ' (required)' : ''}`,
            placeHolder: 'Enter JSON value'
          });
          try {
            value = jsonStr ? JSON.parse(jsonStr) : undefined;
          } catch {
            vscode.window.showErrorMessage(`Invalid JSON for ${key}`);
            return undefined;
          }
      }
      
      if (value === undefined && isRequired) {
        return undefined; // User cancelled
      }
      
      if (value !== undefined) {
        args[key] = value;
      }
    }
    
    return args;
  }
  
  /**
   * Edit server environment variables
   */
  private async editServerEnvironment(server: MCPServerConfig): Promise<void> {
    const configManager = this.extension.getConfigurationManager();
    const env = server.env || {};
    
    const action = await vscode.window.showQuickPick([
      { label: 'Add Variable', value: 'add' },
      { label: 'Edit Variable', value: 'edit' },
      { label: 'Remove Variable', value: 'remove' },
      { label: 'Clear All', value: 'clear' }
    ], {
      placeHolder: 'Environment variable action'
    });
    
    if (!action) return;
    
    switch (action.value) {
      case 'add':
        const key = await vscode.window.showInputBox({
          prompt: 'Variable name',
          validateInput: (v) => {
            if (!v) return 'Name required';
            if (!/^[A-Z_][A-Z0-9_]*$/i.test(v)) return 'Invalid variable name';
            return null;
          }
        });
        
        if (!key) return;
        
        const value = await vscode.window.showInputBox({
          prompt: `Value for ${key}`
        });
        
        if (value !== undefined) {
          env[key] = value;
          server.env = env;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'edit':
        const vars = Object.keys(env);
        if (vars.length === 0) {
          vscode.window.showInformationMessage('No environment variables defined');
          return;
        }
        
        const varToEdit = await vscode.window.showQuickPick(vars, {
          placeHolder: 'Select variable to edit'
        });
        
        if (!varToEdit) return;
        
        const newValue = await vscode.window.showInputBox({
          prompt: `New value for ${varToEdit}`,
          value: env[varToEdit]
        });
        
        if (newValue !== undefined) {
          env[varToEdit] = newValue;
          server.env = env;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'remove':
        const varsToRemove = Object.keys(env);
        if (varsToRemove.length === 0) {
          vscode.window.showInformationMessage('No environment variables defined');
          return;
        }
        
        const varToRemove = await vscode.window.showQuickPick(varsToRemove, {
          placeHolder: 'Select variable to remove'
        });
        
        if (varToRemove) {
          delete env[varToRemove];
          server.env = env;
          await configManager.addOrUpdateServer(server);
        }
        break;
        
      case 'clear':
        const confirm = await vscode.window.showWarningMessage(
          'Clear all environment variables?',
          'Clear',
          'Cancel'
        );
        
        if (confirm === 'Clear') {
          server.env = {};
          await configManager.addOrUpdateServer(server);
        }
        break;
    }
  }
  
  /**
   * Open server configuration in editor
   */
  private async openServerInEditor(server: MCPServerConfig): Promise<void> {
    const doc = await vscode.workspace.openTextDocument({
      content: JSON.stringify(server, null, 2),
      language: 'json'
    });
    
    const editor = await vscode.window.showTextDocument(doc);
    
    // Listen for save
    const disposable = vscode.workspace.onDidSaveTextDocument(async (savedDoc) => {
      if (savedDoc === doc) {
        try {
          const updatedServer = JSON.parse(savedDoc.getText());
          
          // Validate
          const errors = this.extension.getConfigurationManager().validateServer(updatedServer);
          if (errors.length > 0) {
            vscode.window.showErrorMessage(`Invalid configuration: ${errors.join(', ')}`);
            return;
          }
          
          // Update
          await this.extension.getConfigurationManager().addOrUpdateServer(updatedServer);
          vscode.window.showInformationMessage('Server configuration updated');
          
          disposable.dispose();
        } catch (error) {
          vscode.window.showErrorMessage(`Invalid JSON: ${error.message}`);
        }
      }
    });
  }
}