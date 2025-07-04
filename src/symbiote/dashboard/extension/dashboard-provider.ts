/**
 * Dashboard Provider
 * 
 * VS Code WebView provider for the SymbioteIDE User Dashboard
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { 
  DashboardState, 
  DashboardMessage, 
  DashboardMessageType,
  SystemStatus,
  AgentStatus,
  MCPServerStatus,
  OrchestrationMetrics,
  ModelUsageStats,
  ActivityEntry,
  ExecutionRecord
} from '../types/dashboard-types';
import { DashboardDataCollector } from './dashboard-data-collector';
import { WebSocketManager } from '../../websocket/websocket-manager';
import { Logger } from '../../utils/logger';

export class DashboardProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'symbiote.dashboard';
  
  private _view?: vscode.WebviewView;
  private _disposables: vscode.Disposable[] = [];
  private dataCollector: DashboardDataCollector;
  private wsManager: WebSocketManager;
  private logger = new Logger('DashboardProvider');
  private refreshInterval?: NodeJS.Timeout;
  private state: DashboardState | null = null;
  
  constructor(
    private readonly context: vscode.ExtensionContext,
    dataCollector: DashboardDataCollector,
    wsManager: WebSocketManager
  ) {
    this.dataCollector = dataCollector;
    this.wsManager = wsManager;
    
    // Set up real-time updates
    this.setupRealtimeUpdates();
  }
  
  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;
    
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.context.extensionUri, 'src', 'symbiote', 'dashboard', 'webview', 'dist'),
        vscode.Uri.joinPath(this.context.extensionUri, 'media')
      ]
    };
    
    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);
    
    // Handle messages from webview
    webviewView.webview.onDidReceiveMessage(
      message => this.handleMessage(message),
      undefined,
      this._disposables
    );
    
    // Handle visibility changes
    webviewView.onDidChangeVisibility(
      () => {
        if (webviewView.visible) {
          this.startRefreshInterval();
        } else {
          this.stopRefreshInterval();
        }
      },
      undefined,
      this._disposables
    );
    
    // Start data collection if visible
    if (webviewView.visible) {
      this.startRefreshInterval();
    }
  }
  
  private _getHtmlForWebview(webview: vscode.Webview) {
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'src', 'symbiote', 'dashboard', 'webview', 'dist', 'main.js')
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'src', 'symbiote', 'dashboard', 'webview', 'dist', 'main.css')
    );
    
    const nonce = this.getNonce();
    
    return `<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'; font-src ${webview.cspSource}; img-src ${webview.cspSource} data: https:;">
      <link href="${styleUri}" rel="stylesheet">
      <title>SymbioteIDE Dashboard</title>
    </head>
    <body>
      <div id="root"></div>
      <script nonce="${nonce}" src="${scriptUri}"></script>
    </body>
    </html>`;
  }
  
  private getNonce() {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
      text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
  }
  
  private async handleMessage(message: DashboardMessage) {
    switch (message.type) {
      case DashboardMessageType.READY:
        await this.sendInitialState();
        break;
        
      case DashboardMessageType.REFRESH:
        await this.refreshData(message.payload?.sections);
        break;
        
      case DashboardMessageType.UPDATE_SETTINGS:
        await this.updateSettings(message.payload);
        break;
        
      case DashboardMessageType.EXECUTE_ACTION:
        await this.executeAction(message.payload);
        break;
        
      case DashboardMessageType.REQUEST_DATA:
        await this.sendRequestedData(message.payload);
        break;
    }
  }
  
  private async sendInitialState() {
    try {
      this.state = await this.dataCollector.collectFullState();
      this.postMessage({
        type: DashboardMessageType.STATE_UPDATE,
        payload: this.state
      });
    } catch (error) {
      this.logger.error('Failed to send initial state:', error);
      this.postMessage({
        type: DashboardMessageType.ERROR,
        payload: {
          message: 'Failed to load dashboard data',
          error: error instanceof Error ? error.message : 'Unknown error'
        }
      });
    }
  }
  
  private async refreshData(sections?: string[]) {
    try {
      if (sections && sections.length > 0) {
        // Partial refresh
        const updates = await this.dataCollector.collectPartialState(sections);
        this.postMessage({
          type: DashboardMessageType.PARTIAL_UPDATE,
          payload: updates
        });
      } else {
        // Full refresh
        await this.sendInitialState();
      }
    } catch (error) {
      this.logger.error('Failed to refresh data:', error);
    }
  }
  
  private async updateSettings(settings: any) {
    try {
      // Update VS Code configuration
      const config = vscode.workspace.getConfiguration('symbiote.dashboard');
      
      for (const [key, value] of Object.entries(settings)) {
        await config.update(key, value, vscode.ConfigurationTarget.Global);
      }
      
      this.postMessage({
        type: DashboardMessageType.ACTION_RESULT,
        payload: {
          action: 'updateSettings',
          success: true
        }
      });
      
      // Refresh if needed
      if (settings.refreshInterval) {
        this.restartRefreshInterval(settings.refreshInterval);
      }
    } catch (error) {
      this.logger.error('Failed to update settings:', error);
      this.postMessage({
        type: DashboardMessageType.ERROR,
        payload: {
          message: 'Failed to update settings',
          error: error instanceof Error ? error.message : 'Unknown error'
        }
      });
    }
  }
  
  private async executeAction(action: any) {
    try {
      let result: any;
      
      switch (action.type) {
        case 'toggleAgent':
          result = await this.dataCollector.toggleAgent(action.payload.agentId, action.payload.enabled);
          break;
          
        case 'clearCache':
          result = await this.dataCollector.clearCache(action.payload.cacheType);
          break;
          
        case 'restartMCPServer':
          result = await this.dataCollector.restartMCPServer(action.payload.serverId);
          break;
          
        case 'exportMetrics':
          result = await this.exportMetrics(action.payload.format);
          break;
          
        default:
          throw new Error(`Unknown action type: ${action.type}`);
      }
      
      this.postMessage({
        type: DashboardMessageType.ACTION_RESULT,
        payload: {
          action: action.type,
          success: true,
          result
        }
      });
      
      // Refresh relevant sections
      await this.refreshData(this.getAffectedSections(action.type));
      
    } catch (error) {
      this.logger.error('Failed to execute action:', error);
      this.postMessage({
        type: DashboardMessageType.ERROR,
        payload: {
          message: `Failed to execute action: ${action.type}`,
          error: error instanceof Error ? error.message : 'Unknown error'
        }
      });
    }
  }
  
  private async sendRequestedData(request: any) {
    try {
      const data = await this.dataCollector.getSpecificData(request.dataType, request.params);
      this.postMessage({
        type: DashboardMessageType.PARTIAL_UPDATE,
        payload: {
          [request.dataType]: data
        }
      });
    } catch (error) {
      this.logger.error('Failed to send requested data:', error);
    }
  }
  
  private setupRealtimeUpdates() {
    // WebSocket updates
    this.wsManager.on('metrics', (data) => {
      this.postMessage({
        type: DashboardMessageType.METRIC_UPDATE,
        payload: data
      });
    });
    
    this.wsManager.on('agent-status', (data) => {
      this.postMessage({
        type: DashboardMessageType.AGENT_UPDATE,
        payload: data
      });
    });
    
    this.wsManager.on('log', (data) => {
      this.postMessage({
        type: DashboardMessageType.LOG_ENTRY,
        payload: data
      });
    });
    
    this.wsManager.on('execution', (data) => {
      this.postMessage({
        type: DashboardMessageType.EXECUTION_UPDATE,
        payload: data
      });
    });
  }
  
  private startRefreshInterval() {
    const config = vscode.workspace.getConfiguration('symbiote.dashboard');
    const interval = config.get<number>('refreshInterval', 5000);
    
    this.refreshInterval = setInterval(() => {
      this.refreshData(['systemStatus', 'orchestrationMetrics']);
    }, interval);
  }
  
  private stopRefreshInterval() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
      this.refreshInterval = undefined;
    }
  }
  
  private restartRefreshInterval(newInterval: number) {
    this.stopRefreshInterval();
    this.refreshInterval = setInterval(() => {
      this.refreshData(['systemStatus', 'orchestrationMetrics']);
    }, newInterval);
  }
  
  private getAffectedSections(actionType: string): string[] {
    switch (actionType) {
      case 'toggleAgent':
        return ['agents', 'systemStatus'];
      case 'clearCache':
        return ['memoryUsage', 'orchestrationMetrics'];
      case 'restartMCPServer':
        return ['mcpServers'];
      default:
        return [];
    }
  }
  
  private async exportMetrics(format: 'json' | 'csv') {
    const data = await this.dataCollector.collectFullState();
    const timestamp = new Date().toISOString().replace(/:/g, '-');
    const filename = `symbiote-metrics-${timestamp}.${format}`;
    
    let content: string;
    if (format === 'json') {
      content = JSON.stringify(data, null, 2);
    } else {
      // Convert to CSV (simplified)
      content = this.convertToCSV(data);
    }
    
    const uri = vscode.Uri.file(path.join(vscode.workspace.rootPath || '', filename));
    await vscode.workspace.fs.writeFile(uri, Buffer.from(content, 'utf-8'));
    
    vscode.window.showInformationMessage(`Metrics exported to ${filename}`);
    return { filename, path: uri.fsPath };
  }
  
  private convertToCSV(data: any): string {
    // Simplified CSV conversion for metrics
    const lines: string[] = ['Metric,Value,Timestamp'];
    const timestamp = new Date().toISOString();
    
    // System metrics
    lines.push(`CPU Usage,${data.systemStatus.resources.cpu},${timestamp}`);
    lines.push(`Memory Usage,${data.systemStatus.resources.memory.percentage},${timestamp}`);
    lines.push(`Active Agents,${data.agents.filter((a: AgentStatus) => a.status === 'active').length},${timestamp}`);
    lines.push(`Queue Length,${data.orchestrationMetrics.queueLength},${timestamp}`);
    
    // Model usage
    data.modelUsage.forEach((model: ModelUsageStats) => {
      lines.push(`${model.model} Requests,${model.requests},${timestamp}`);
      lines.push(`${model.model} Cost,$${model.cost.toFixed(2)},${timestamp}`);
    });
    
    return lines.join('\n');
  }
  
  private postMessage(message: DashboardMessage) {
    if (this._view && this._view.visible) {
      this._view.webview.postMessage(message);
    }
  }
  
  public dispose() {
    this.stopRefreshInterval();
    while (this._disposables.length) {
      const disposable = this._disposables.pop();
      if (disposable) {
        disposable.dispose();
      }
    }
  }
}