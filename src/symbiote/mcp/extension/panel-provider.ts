/**
 * MCP Panel Provider
 * 
 * Provides webview panel for MCP management
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { MCPServerManager } from '../server-manager';

export class MCPPanelProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'symbiote.mcpPanel';
  
  private _view?: vscode.WebviewView;
  private _extensionUri: vscode.Uri;
  private serverManager: MCPServerManager;
  
  constructor(
    context: vscode.ExtensionContext,
    serverManager: MCPServerManager
  ) {
    this._extensionUri = context.extensionUri;
    this.serverManager = serverManager;
    
    // Setup event listeners
    this.setupEventHandlers();
  }
  
  /**
   * Show panel
   */
  show(): void {
    vscode.commands.executeCommand('symbiote.mcpPanel.focus');
  }
  
  /**
   * Resolve webview view
   */
  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;
    
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this._extensionUri, 'media'),
        vscode.Uri.joinPath(this._extensionUri, 'out')
      ]
    };
    
    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);
    
    // Handle messages from webview
    webviewView.webview.onDidReceiveMessage(
      message => this.handleWebviewMessage(message)
    );
    
    // Update webview when it becomes visible
    webviewView.onDidChangeVisibility(() => {
      if (webviewView.visible) {
        this.updateWebview();
      }
    });
    
    // Initial update
    this.updateWebview();
  }
  
  /**
   * Handle webview messages
   */
  private async handleWebviewMessage(message: any) {
    switch (message.type) {
      case 'ready':
        await this.updateWebview();
        break;
        
      case 'start-server':
        await this.serverManager.startServer(message.serverId);
        break;
        
      case 'stop-server':
        await this.serverManager.stopServer(message.serverId);
        break;
        
      case 'restart-server':
        await vscode.commands.executeCommand('symbiote.mcp.restartServer', message.serverId);
        break;
        
      case 'execute-tool':
        await vscode.commands.executeCommand('symbiote.mcp.executeTool', message.serverId, message.toolName);
        break;
        
      case 'read-resource':
        await vscode.commands.executeCommand('symbiote.mcp.readResource', message.serverId, message.uri);
        break;
        
      case 'show-logs':
        await vscode.commands.executeCommand('symbiote.mcp.showLogs', message.serverId);
        break;
        
      case 'edit-server':
        await vscode.commands.executeCommand('symbiote.mcp.editServer', message.serverId);
        break;
        
      case 'remove-server':
        await vscode.commands.executeCommand('symbiote.mcp.removeServer', message.serverId);
        break;
        
      case 'add-server':
        await vscode.commands.executeCommand('symbiote.mcp.addServer');
        break;
        
      case 'refresh':
        await this.updateWebview();
        break;
        
      case 'open-settings':
        await vscode.commands.executeCommand('symbiote.mcp.openSettings');
        break;
        
      case 'copy-to-clipboard':
        await vscode.env.clipboard.writeText(message.text);
        vscode.window.showInformationMessage('Copied to clipboard');
        break;
    }
  }
  
  /**
   * Update webview content
   */
  private async updateWebview() {
    if (!this._view) return;
    
    const servers = await this.getServersData();
    
    this._view.webview.postMessage({
      type: 'update',
      servers
    });
  }
  
  /**
   * Get servers data for webview
   */
  private async getServersData() {
    const servers = this.serverManager.getServers();
    const data = [];
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      const isConnected = client?.isConnected() || false;
      
      let serverInfo = null;
      let tools = [];
      let resources = [];
      let prompts = [];
      
      if (isConnected && client) {
        try {
          serverInfo = await client.getServerInfo();
          
          if (serverInfo.capabilities?.tools) {
            tools = await client.listTools();
          }
          
          if (serverInfo.capabilities?.resources) {
            resources = await client.listResources();
          }
          
          if (serverInfo.capabilities?.prompts) {
            prompts = await client.listPrompts();
          }
        } catch (error) {
          console.error(`Failed to get server info for ${serverId}:`, error);
        }
      }
      
      data.push({
        id: serverId,
        name: server.name || serverId,
        description: server.description,
        status: isConnected ? 'connected' : 'disconnected',
        transport: server.transport || 'stdio',
        command: server.command,
        url: server.url,
        serverInfo,
        tools,
        resources,
        prompts,
        metrics: this.serverManager.getServerMetrics(serverId)
      });
    }
    
    return data;
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers() {
    // Listen for server changes
    this.serverManager.on('server-started', () => {
      this.updateWebview();
    });
    
    this.serverManager.on('server-stopped', () => {
      this.updateWebview();
    });
    
    this.serverManager.on('server-error', () => {
      this.updateWebview();
    });
    
    this.serverManager.on('servers-changed', () => {
      this.updateWebview();
    });
  }
  
  /**
   * Get HTML for webview
   */
  private _getHtmlForWebview(webview: vscode.Webview) {
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'mcp-panel.js')
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'mcp-panel.css')
    );
    
    const codiconUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'codicon.css')
    );
    
    const nonce = getNonce();
    
    return `<!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'; font-src ${webview.cspSource};">
        <link href="${styleUri}" rel="stylesheet">
        <link href="${codiconUri}" rel="stylesheet">
        <title>MCP Servers</title>
      </head>
      <body>
        <div class="container">
          <div class="header">
            <h2>MCP Servers</h2>
            <div class="actions">
              <button class="icon-button" id="add-server" title="Add Server">
                <i class="codicon codicon-add"></i>
              </button>
              <button class="icon-button" id="refresh" title="Refresh">
                <i class="codicon codicon-refresh"></i>
              </button>
              <button class="icon-button" id="settings" title="Settings">
                <i class="codicon codicon-settings-gear"></i>
              </button>
            </div>
          </div>
          
          <div id="servers-list" class="servers-list">
            <div class="loading">
              <i class="codicon codicon-loading codicon-modifier-spin"></i>
              Loading servers...
            </div>
          </div>
          
          <div class="empty-state" id="empty-state" style="display: none;">
            <i class="codicon codicon-server-environment"></i>
            <p>No MCP servers configured</p>
            <button class="button primary" id="add-first-server">
              Add Server
            </button>
          </div>
        </div>
        
        <script nonce="${nonce}">
          const vscode = acquireVsCodeApi();
          
          // Initialize
          window.addEventListener('DOMContentLoaded', () => {
            vscode.postMessage({ type: 'ready' });
            
            // Setup event handlers
            document.getElementById('add-server').addEventListener('click', () => {
              vscode.postMessage({ type: 'add-server' });
            });
            
            document.getElementById('add-first-server').addEventListener('click', () => {
              vscode.postMessage({ type: 'add-server' });
            });
            
            document.getElementById('refresh').addEventListener('click', () => {
              vscode.postMessage({ type: 'refresh' });
            });
            
            document.getElementById('settings').addEventListener('click', () => {
              vscode.postMessage({ type: 'open-settings' });
            });
          });
          
          // Handle messages from extension
          window.addEventListener('message', event => {
            const message = event.data;
            
            switch (message.type) {
              case 'update':
                updateServers(message.servers);
                break;
            }
          });
          
          // Update servers display
          function updateServers(servers) {
            const container = document.getElementById('servers-list');
            const emptyState = document.getElementById('empty-state');
            
            if (servers.length === 0) {
              container.style.display = 'none';
              emptyState.style.display = 'flex';
              return;
            }
            
            container.style.display = 'block';
            emptyState.style.display = 'none';
            
            container.innerHTML = servers.map(server => createServerCard(server)).join('');
            
            // Setup server card event handlers
            servers.forEach(server => {
              const card = document.getElementById('server-' + server.id);
              if (!card) return;
              
              // Toggle expand/collapse
              card.querySelector('.server-header').addEventListener('click', () => {
                card.classList.toggle('expanded');
              });
              
              // Server actions
              card.querySelector('.start-server')?.addEventListener('click', (e) => {
                e.stopPropagation();
                vscode.postMessage({ type: 'start-server', serverId: server.id });
              });
              
              card.querySelector('.stop-server')?.addEventListener('click', (e) => {
                e.stopPropagation();
                vscode.postMessage({ type: 'stop-server', serverId: server.id });
              });
              
              card.querySelector('.restart-server')?.addEventListener('click', (e) => {
                e.stopPropagation();
                vscode.postMessage({ type: 'restart-server', serverId: server.id });
              });
              
              card.querySelector('.show-logs')?.addEventListener('click', (e) => {
                e.stopPropagation();
                vscode.postMessage({ type: 'show-logs', serverId: server.id });
              });
              
              card.querySelector('.edit-server')?.addEventListener('click', (e) => {
                e.stopPropagation();
                vscode.postMessage({ type: 'edit-server', serverId: server.id });
              });
              
              card.querySelector('.remove-server')?.addEventListener('click', (e) => {
                e.stopPropagation();
                vscode.postMessage({ type: 'remove-server', serverId: server.id });
              });
              
              // Tool actions
              card.querySelectorAll('.execute-tool').forEach(btn => {
                btn.addEventListener('click', () => {
                  const toolName = btn.getAttribute('data-tool');
                  vscode.postMessage({ 
                    type: 'execute-tool', 
                    serverId: server.id,
                    toolName 
                  });
                });
              });
              
              // Resource actions
              card.querySelectorAll('.read-resource').forEach(btn => {
                btn.addEventListener('click', () => {
                  const uri = btn.getAttribute('data-uri');
                  vscode.postMessage({ 
                    type: 'read-resource', 
                    serverId: server.id,
                    uri 
                  });
                });
              });
            });
          }
          
          // Create server card HTML
          function createServerCard(server) {
            const isConnected = server.status === 'connected';
            const statusIcon = isConnected ? 'circle-filled' : 'circle-outline';
            const statusClass = isConnected ? 'connected' : 'disconnected';
            
            return \`
              <div class="server-card" id="server-\${server.id}">
                <div class="server-header">
                  <div class="server-info">
                    <i class="codicon codicon-\${statusIcon} status-icon \${statusClass}"></i>
                    <span class="server-name">\${server.name}</span>
                    <span class="server-type">\${server.transport}</span>
                  </div>
                  <div class="server-actions">
                    \${isConnected ? \`
                      <button class="icon-button stop-server" title="Stop Server">
                        <i class="codicon codicon-debug-stop"></i>
                      </button>
                      <button class="icon-button restart-server" title="Restart Server">
                        <i class="codicon codicon-debug-restart"></i>
                      </button>
                    \` : \`
                      <button class="icon-button start-server" title="Start Server">
                        <i class="codicon codicon-play"></i>
                      </button>
                    \`}
                    <button class="icon-button show-logs" title="Show Logs">
                      <i class="codicon codicon-output"></i>
                    </button>
                    <button class="icon-button edit-server" title="Edit Server">
                      <i class="codicon codicon-edit"></i>
                    </button>
                    <button class="icon-button remove-server" title="Remove Server">
                      <i class="codicon codicon-trash"></i>
                    </button>
                  </div>
                </div>
                
                <div class="server-details">
                  \${server.description ? \`<p class="description">\${server.description}</p>\` : ''}
                  
                  \${server.serverInfo ? \`
                    <div class="info-section">
                      <h4>Server Information</h4>
                      <div class="info-grid">
                        <span>Name:</span><span>\${server.serverInfo.name}</span>
                        <span>Version:</span><span>\${server.serverInfo.version}</span>
                        <span>Protocol:</span><span>\${server.serverInfo.protocolVersion}</span>
                      </div>
                    </div>
                  \` : ''}
                  
                  \${server.tools.length > 0 ? \`
                    <div class="tools-section">
                      <h4>Tools (\${server.tools.length})</h4>
                      <div class="tool-list">
                        \${server.tools.map(tool => \`
                          <div class="tool-item">
                            <span class="tool-name">\${tool.name}</span>
                            <button class="icon-button execute-tool" data-tool="\${tool.name}" title="Execute">
                              <i class="codicon codicon-play"></i>
                            </button>
                          </div>
                        \`).join('')}
                      </div>
                    </div>
                  \` : ''}
                  
                  \${server.resources.length > 0 ? \`
                    <div class="resources-section">
                      <h4>Resources (\${server.resources.length})</h4>
                      <div class="resource-list">
                        \${server.resources.slice(0, 10).map(resource => \`
                          <div class="resource-item">
                            <span class="resource-name">\${resource.name}</span>
                            <button class="icon-button read-resource" data-uri="\${resource.uri}" title="Read">
                              <i class="codicon codicon-eye"></i>
                            </button>
                          </div>
                        \`).join('')}
                        \${server.resources.length > 10 ? \`
                          <div class="more-items">... and \${server.resources.length - 10} more</div>
                        \` : ''}
                      </div>
                    </div>
                  \` : ''}
                  
                  \${server.metrics ? \`
                    <div class="metrics-section">
                      <h4>Metrics</h4>
                      <div class="metrics-grid">
                        <span>Requests:</span><span>\${server.metrics.requestCount}</span>
                        <span>Errors:</span><span>\${server.metrics.errorCount}</span>
                        <span>Uptime:</span><span>\${formatUptime(server.metrics.uptime)}</span>
                      </div>
                    </div>
                  \` : ''}
                </div>
              </div>
            \`;
          }
          
          // Format uptime
          function formatUptime(ms) {
            if (!ms) return 'N/A';
            
            const seconds = Math.floor(ms / 1000);
            const minutes = Math.floor(seconds / 60);
            const hours = Math.floor(minutes / 60);
            const days = Math.floor(hours / 24);
            
            if (days > 0) return days + 'd';
            if (hours > 0) return hours + 'h';
            if (minutes > 0) return minutes + 'm';
            return seconds + 's';
          }
        </script>
      </body>
      </html>`;
  }
}

/**
 * Generate nonce for webview security
 */
function getNonce() {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}