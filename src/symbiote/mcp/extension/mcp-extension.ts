/**
 * MCP Extension for VS Code
 * 
 * Main entry point for Model Context Protocol integration
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { MCPServerManager } from '../server-manager';
import { MCPClient } from '../client/mcp-client';
import { MCPSecurityExtension } from '../security/extension/security-extension';
import { MCPConfigurationManager } from './configuration-manager';
import { MCPCommandRegistry } from './command-registry';
import { MCPStatusBarManager } from './status-bar-manager';
import { MCPTreeDataProvider } from './tree-data-provider';
import { MCPPanelProvider } from './panel-provider';
import { MCPDiagnosticManager } from './diagnostic-manager';
import { MCPLanguageClient } from './language-client';
import { MCPFileSystemProvider } from './filesystem-provider';
import { MCPDebugAdapterFactory } from './debug-adapter';
import { MCPCompletionProvider } from './completion-provider';
import { MCPHoverProvider } from './hover-provider';
import { MCPDefinitionProvider } from './definition-provider';
import { MCPDocumentSymbolProvider, MCPWorkspaceSymbolProvider } from './symbol-provider';
import { MCPTaskProvider } from './task-provider';

export class MCPExtension {
  private context: vscode.ExtensionContext;
  private serverManager: MCPServerManager;
  private securityExtension: MCPSecurityExtension;
  private configManager: MCPConfigurationManager;
  private commandRegistry: MCPCommandRegistry;
  private statusBarManager: MCPStatusBarManager;
  private treeProvider: MCPTreeDataProvider;
  private panelProvider: MCPPanelProvider;
  private diagnosticManager: MCPDiagnosticManager;
  private languageClient: MCPLanguageClient;
  private isActivated: boolean = false;
  
  constructor(context: vscode.ExtensionContext) {
    this.context = context;
    
    // Initialize core components
    this.configManager = new MCPConfigurationManager(context);
    this.serverManager = new MCPServerManager();
    this.securityExtension = new MCPSecurityExtension(context, this.serverManager);
    this.commandRegistry = new MCPCommandRegistry(this);
    this.statusBarManager = new MCPStatusBarManager();
    this.treeProvider = new MCPTreeDataProvider(this.serverManager);
    this.panelProvider = new MCPPanelProvider(context, this.serverManager);
    this.diagnosticManager = new MCPDiagnosticManager();
    this.languageClient = new MCPLanguageClient(context, this.serverManager);
  }
  
  /**
   * Activate the MCP extension
   */
  async activate(): Promise<void> {
    if (this.isActivated) return;
    
    try {
      // Show activation progress
      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Window,
        title: 'Activating MCP Extension',
        cancellable: false
      }, async (progress) => {
        progress.report({ increment: 0, message: 'Loading configuration...' });
        await this.configManager.initialize();
        
        progress.report({ increment: 20, message: 'Initializing server manager...' });
        await this.serverManager.initialize();
        
        progress.report({ increment: 40, message: 'Setting up security...' });
        await this.securityExtension.activate();
        
        progress.report({ increment: 60, message: 'Registering commands...' });
        this.registerCommands();
        
        progress.report({ increment: 80, message: 'Setting up UI components...' });
        this.setupUI();
        
        progress.report({ increment: 90, message: 'Registering providers...' });
        await this.registerProviders();
        
        progress.report({ increment: 100, message: 'MCP activated' });
      });
      
      // Setup event handlers
      this.setupEventHandlers();
      
      // Auto-start servers if configured
      await this.autoStartServers();
      
      this.isActivated = true;
      
      // Show welcome message if first activation
      if (this.configManager.isFirstActivation()) {
        this.showWelcomeMessage();
      }
      
      // Update status
      this.statusBarManager.updateStatus('ready', 'MCP Extension Ready');
      
      // Log activation
      this.logActivation();
      
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to activate MCP Extension: ${error.message}`);
      throw error;
    }
  }
  
  /**
   * Deactivate the extension
   */
  async deactivate(): Promise<void> {
    if (!this.isActivated) return;
    
    try {
      // Stop all servers
      await this.serverManager.stopAllServers();
      
      // Cleanup components
      await this.securityExtension.deactivate();
      await this.languageClient.stop();
      this.diagnosticManager.dispose();
      this.statusBarManager.dispose();
      
      this.isActivated = false;
      
    } catch (error) {
      console.error('Error during MCP deactivation:', error);
    }
  }
  
  /**
   * Register all commands
   */
  private registerCommands(): void {
    // Server management commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.startServer', async (serverId?: string) => {
        await this.commandRegistry.startServer(serverId);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.stopServer', async (serverId?: string) => {
        await this.commandRegistry.stopServer(serverId);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.restartServer', async (serverId?: string) => {
        await this.commandRegistry.restartServer(serverId);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.addServer', async () => {
        await this.commandRegistry.addServer();
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.removeServer', async (serverId?: string) => {
        await this.commandRegistry.removeServer(serverId);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.editServer', async (serverId?: string) => {
        await this.commandRegistry.editServer(serverId);
      })
    );
    
    // Tool commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.executeTool', async (serverId?: string, toolName?: string) => {
        await this.commandRegistry.executeTool(serverId, toolName);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.listTools', async (serverId?: string) => {
        await this.commandRegistry.listTools(serverId);
      })
    );
    
    // Resource commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.listResources', async (serverId?: string) => {
        await this.commandRegistry.listResources(serverId);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.readResource', async (serverId?: string, uri?: string) => {
        await this.commandRegistry.readResource(serverId, uri);
      })
    );
    
    // UI commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.showPanel', () => {
        this.panelProvider.show();
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.showServerInfo', async (serverId?: string) => {
        await this.commandRegistry.showServerInfo(serverId);
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.showLogs', async (serverId?: string) => {
        await this.commandRegistry.showLogs(serverId);
      })
    );
    
    // Configuration commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.openSettings', () => {
        vscode.commands.executeCommand('workbench.action.openSettings', '@ext:symbiote.mcp');
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.reloadConfiguration', async () => {
        await this.configManager.reload();
        vscode.window.showInformationMessage('MCP configuration reloaded');
      })
    );
    
    // Development commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.createServer', async () => {
        await this.commandRegistry.createServerProject();
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.testServer', async (serverId?: string) => {
        await this.commandRegistry.testServer(serverId);
      })
    );
    
    // Quick actions
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.quickAction', async () => {
        await this.commandRegistry.showQuickActions();
      })
    );
  }
  
  /**
   * Setup UI components
   */
  private setupUI(): void {
    // Register tree view
    const treeView = vscode.window.createTreeView('mcpServers', {
      treeDataProvider: this.treeProvider,
      showCollapseAll: true
    });
    
    this.context.subscriptions.push(treeView);
    
    // Register webview panel provider
    this.context.subscriptions.push(
      vscode.window.registerWebviewViewProvider(
        MCPPanelProvider.viewType,
        this.panelProvider
      )
    );
    
    // Setup status bar
    this.statusBarManager.createStatusBarItems();
    this.context.subscriptions.push(
      this.statusBarManager.getMainItem(),
      this.statusBarManager.getServerItem()
    );
    
    // Register custom editors if needed
    this.registerCustomEditors();
  }
  
  /**
   * Register providers
   */
  private async registerProviders(): Promise<void> {
    // Language features
    const selector = { scheme: 'mcp', language: '*' };
    
    // Completion provider
    const completionProvider = new MCPCompletionProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.languages.registerCompletionItemProvider(
        selector,
        completionProvider,
        '.', '/', ':'
      )
    );
    
    // Hover provider
    const hoverProvider = new MCPHoverProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.languages.registerHoverProvider(selector, hoverProvider)
    );
    
    // Definition provider
    const definitionProvider = new MCPDefinitionProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.languages.registerDefinitionProvider(selector, definitionProvider)
    );
    
    // Document symbol provider
    const documentSymbolProvider = new MCPDocumentSymbolProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.languages.registerDocumentSymbolProvider(selector, documentSymbolProvider)
    );
    
    // Workspace symbol provider
    const workspaceSymbolProvider = new MCPWorkspaceSymbolProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.languages.registerWorkspaceSymbolProvider(workspaceSymbolProvider)
    );
    
    // File system provider
    const fsProvider = new MCPFileSystemProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.workspace.registerFileSystemProvider('mcp', fsProvider, {
        isCaseSensitive: true,
        isReadonly: false
      })
    );
    
    // Task provider
    const taskProvider = new MCPTaskProvider(this.serverManager);
    this.context.subscriptions.push(
      vscode.tasks.registerTaskProvider('mcp', taskProvider)
    );
    await taskProvider.registerCommands(this.context);
    
    // Debug adapter
    const debugAdapterFactory = new MCPDebugAdapterFactory(this.serverManager);
    this.context.subscriptions.push(
      vscode.debug.registerDebugAdapterDescriptorFactory('mcp', debugAdapterFactory)
    );
    
    // Diagnostics
    this.diagnosticManager.registerDiagnosticCollection(this.context);
  }
  
  /**
   * Register custom editors
   */
  private registerCustomEditors(): void {
    // MCP config editor
    this.context.subscriptions.push(
      vscode.window.registerCustomEditorProvider(
        'symbiote.mcpConfig',
        {
          async openCustomDocument(uri: vscode.Uri) {
            const content = await vscode.workspace.fs.readFile(uri);
            return {
              uri,
              dispose: () => {},
              content: content.toString()
            };
          },
          
          async resolveCustomEditor(document: any, webviewPanel: vscode.WebviewPanel) {
            webviewPanel.webview.options = {
              enableScripts: true
            };
            
            webviewPanel.webview.html = this.getMCPConfigEditorHtml(
              webviewPanel.webview,
              document
            );
          }
        },
        {
          webviewOptions: {
            retainContextWhenHidden: true
          },
          supportsMultipleEditorsPerDocument: false
        }
      )
    );
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Configuration changes
    this.context.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration(async (e) => {
        if (e.affectsConfiguration('symbiote.mcp')) {
          await this.handleConfigurationChange();
        }
      })
    );
    
    // Workspace changes
    this.context.subscriptions.push(
      vscode.workspace.onDidChangeWorkspaceFolders(async () => {
        await this.handleWorkspaceChange();
      })
    );
    
    // File system watchers
    const mcpConfigWatcher = vscode.workspace.createFileSystemWatcher('**/.mcp.json');
    
    this.context.subscriptions.push(
      mcpConfigWatcher.onDidCreate(async (uri) => {
        await this.handleMCPConfigCreated(uri);
      }),
      
      mcpConfigWatcher.onDidChange(async (uri) => {
        await this.handleMCPConfigChanged(uri);
      }),
      
      mcpConfigWatcher.onDidDelete(async (uri) => {
        await this.handleMCPConfigDeleted(uri);
      })
    );
    
    // Server events
    this.serverManager.on('server-started', (event) => {
      this.handleServerStarted(event);
    });
    
    this.serverManager.on('server-stopped', (event) => {
      this.handleServerStopped(event);
    });
    
    this.serverManager.on('server-error', (event) => {
      this.handleServerError(event);
    });
    
    this.serverManager.on('server-message', (event) => {
      this.handleServerMessage(event);
    });
    
    // Window state
    this.context.subscriptions.push(
      vscode.window.onDidChangeWindowState((state) => {
        if (state.focused) {
          this.handleWindowFocused();
        }
      })
    );
  }
  
  /**
   * Auto-start configured servers
   */
  private async autoStartServers(): Promise<void> {
    const autoStart = this.configManager.getAutoStartServers();
    
    for (const serverId of autoStart) {
      try {
        await this.serverManager.startServer(serverId);
      } catch (error) {
        console.error(`Failed to auto-start server ${serverId}:`, error);
      }
    }
  }
  
  /**
   * Show welcome message
   */
  private showWelcomeMessage(): void {
    const actions = ['View Documentation', 'Add Server', 'Open Settings'];
    
    vscode.window.showInformationMessage(
      'Welcome to MCP Extension! Get started by adding your first MCP server.',
      ...actions
    ).then(action => {
      switch (action) {
        case 'View Documentation':
          vscode.env.openExternal(vscode.Uri.parse('https://modelcontextprotocol.io'));
          break;
        case 'Add Server':
          vscode.commands.executeCommand('symbiote.mcp.addServer');
          break;
        case 'Open Settings':
          vscode.commands.executeCommand('symbiote.mcp.openSettings');
          break;
      }
    });
  }
  
  /**
   * Handle configuration change
   */
  private async handleConfigurationChange(): Promise<void> {
    await this.configManager.reload();
    
    // Update server configurations
    const servers = this.configManager.getServers();
    for (const serverConfig of servers) {
      await this.serverManager.updateServerConfig(serverConfig.id, serverConfig);
    }
    
    // Update UI
    this.treeProvider.refresh();
    this.statusBarManager.refresh();
  }
  
  /**
   * Handle workspace change
   */
  private async handleWorkspaceChange(): Promise<void> {
    // Scan for MCP config files in new workspace folders
    for (const folder of vscode.workspace.workspaceFolders || []) {
      const mcpConfig = vscode.Uri.joinPath(folder.uri, '.mcp.json');
      
      try {
        await vscode.workspace.fs.stat(mcpConfig);
        await this.handleMCPConfigCreated(mcpConfig);
      } catch {
        // File doesn't exist
      }
    }
  }
  
  /**
   * Handle MCP config created
   */
  private async handleMCPConfigCreated(uri: vscode.Uri): Promise<void> {
    const config = await this.loadMCPConfig(uri);
    if (!config) return;
    
    const action = await vscode.window.showInformationMessage(
      `Found MCP configuration in ${vscode.workspace.asRelativePath(uri)}`,
      'Load Servers',
      'Ignore'
    );
    
    if (action === 'Load Servers') {
      await this.loadServersFromConfig(config, uri);
    }
  }
  
  /**
   * Handle MCP config changed
   */
  private async handleMCPConfigChanged(uri: vscode.Uri): Promise<void> {
    const config = await this.loadMCPConfig(uri);
    if (!config) return;
    
    // Update existing servers or add new ones
    await this.loadServersFromConfig(config, uri);
  }
  
  /**
   * Handle MCP config deleted
   */
  private async handleMCPConfigDeleted(uri: vscode.Uri): Promise<void> {
    // Remove servers associated with this config
    const servers = this.serverManager.getServers();
    
    for (const [serverId, server] of servers) {
      if (server.configUri?.toString() === uri.toString()) {
        await this.serverManager.removeServer(serverId);
      }
    }
  }
  
  /**
   * Handle server started
   */
  private handleServerStarted(event: any): void {
    this.treeProvider.refresh();
    this.statusBarManager.updateServerStatus(event.serverId, 'running');
    
    vscode.window.showInformationMessage(
      `MCP Server '${event.serverName}' started`
    );
  }
  
  /**
   * Handle server stopped
   */
  private handleServerStopped(event: any): void {
    this.treeProvider.refresh();
    this.statusBarManager.updateServerStatus(event.serverId, 'stopped');
  }
  
  /**
   * Handle server error
   */
  private handleServerError(event: any): void {
    this.treeProvider.refresh();
    this.statusBarManager.updateServerStatus(event.serverId, 'error');
    
    vscode.window.showErrorMessage(
      `MCP Server '${event.serverName}' error: ${event.error.message}`,
      'View Logs'
    ).then(action => {
      if (action === 'View Logs') {
        vscode.commands.executeCommand('symbiote.mcp.showLogs', event.serverId);
      }
    });
  }
  
  /**
   * Handle server message
   */
  private handleServerMessage(event: any): void {
    if (event.message.method === 'notifications/message') {
      const params = event.message.params;
      
      switch (params.level) {
        case 'error':
          vscode.window.showErrorMessage(`[${event.serverName}] ${params.message}`);
          break;
        case 'warning':
          vscode.window.showWarningMessage(`[${event.serverName}] ${params.message}`);
          break;
        case 'info':
          vscode.window.showInformationMessage(`[${event.serverName}] ${params.message}`);
          break;
      }
    }
  }
  
  /**
   * Handle window focused
   */
  private handleWindowFocused(): void {
    // Refresh server status when window regains focus
    this.serverManager.refreshAllServers();
  }
  
  /**
   * Load MCP config file
   */
  private async loadMCPConfig(uri: vscode.Uri): Promise<any> {
    try {
      const content = await vscode.workspace.fs.readFile(uri);
      return JSON.parse(content.toString());
    } catch (error) {
      console.error('Failed to load MCP config:', error);
      return null;
    }
  }
  
  /**
   * Load servers from config
   */
  private async loadServersFromConfig(config: any, configUri: vscode.Uri): Promise<void> {
    if (!config.mcpServers) return;
    
    for (const [serverId, serverConfig] of Object.entries(config.mcpServers)) {
      const fullConfig = {
        id: serverId,
        ...serverConfig as any,
        configUri
      };
      
      await this.serverManager.addOrUpdateServer(fullConfig);
    }
    
    this.treeProvider.refresh();
  }
  
  /**
   * Get MCP config editor HTML
   */
  private getMCPConfigEditorHtml(webview: vscode.Webview, document: any): string {
    const nonce = getNonce();
    
    return `<!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
        <title>MCP Configuration</title>
      </head>
      <body>
        <h1>MCP Configuration Editor</h1>
        <div id="editor"></div>
        <script nonce="${nonce}">
          const vscode = acquireVsCodeApi();
          const config = ${JSON.stringify(JSON.parse(document.content), null, 2)};
          
          // TODO: Implement config editor UI
          document.getElementById('editor').textContent = JSON.stringify(config, null, 2);
        </script>
      </body>
      </html>`;
  }
  
  /**
   * Log activation
   */
  private logActivation(): void {
    const outputChannel = vscode.window.createOutputChannel('MCP Extension');
    outputChannel.appendLine(`MCP Extension activated at ${new Date().toISOString()}`);
    outputChannel.appendLine(`Version: ${this.context.extension.packageJSON.version}`);
    outputChannel.appendLine(`Servers: ${this.serverManager.getServers().size}`);
  }
  
  // Public API methods
  
  /**
   * Get server manager
   */
  getServerManager(): MCPServerManager {
    return this.serverManager;
  }
  
  /**
   * Get security extension
   */
  getSecurityExtension(): MCPSecurityExtension {
    return this.securityExtension.getSecureServerManager();
  }
  
  /**
   * Get configuration manager
   */
  getConfigurationManager(): MCPConfigurationManager {
    return this.configManager;
  }
  
  /**
   * Execute MCP tool
   */
  async executeTool(serverId: string, toolName: string, args: any): Promise<any> {
    return await this.serverManager.executeTool(serverId, toolName, args);
  }
  
  /**
   * Read MCP resource
   */
  async readResource(serverId: string, uri: string): Promise<any> {
    return await this.serverManager.readResource(serverId, uri);
  }
}

/**
 * Generate nonce for webview security
 */
function getNonce(): string {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}