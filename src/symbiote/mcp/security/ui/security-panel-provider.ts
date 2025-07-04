/**
 * Security Panel Provider
 * 
 * VS Code webview provider for MCP security management
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { SecurityManager } from '../security-manager';
import { PermissionRequest, PermissionDecision } from '../types';

export class SecurityPanelProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'symbiote.mcpSecurity';
  
  private _view?: vscode.WebviewView;
  private _extensionUri: vscode.Uri;
  private _securityManager: SecurityManager;
  
  constructor(
    extensionUri: vscode.Uri,
    securityManager: SecurityManager
  ) {
    this._extensionUri = extensionUri;
    this._securityManager = securityManager;
    
    // Setup event listeners
    this.setupSecurityEventHandlers();
  }
  
  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;
    
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [this._extensionUri]
    };
    
    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);
    
    // Handle messages from webview
    webviewView.webview.onDidReceiveMessage(
      message => this.handleWebviewMessage(message),
      null,
      []
    );
    
    // Update webview when it becomes visible
    webviewView.onDidChangeVisibility(() => {
      if (webviewView.visible) {
        this.updateWebview();
      }
    });
  }
  
  /**
   * Handle messages from webview
   */
  private async handleWebviewMessage(message: any) {
    switch (message.type) {
      case 'ready':
        await this.updateWebview();
        break;
        
      case 'permission-decision':
        await this.handlePermissionDecision(
          message.requestId,
          message.decision
        );
        break;
        
      case 'update-trust-level':
        await this.handleTrustUpdate(
          message.serverId,
          message.trustLevel,
          message.reason
        );
        break;
        
      case 'create-policy':
        await this.showPolicyCreator();
        break;
        
      case 'update-policy':
        await this.handlePolicyUpdate(message.policy);
        break;
        
      case 'delete-policy':
        await this.handlePolicyDelete(message.policyId);
        break;
        
      case 'resolve-incident':
        await this.handleIncidentResolution(
          message.incidentId,
          message.resolution
        );
        break;
        
      case 'export-audit-logs':
        await this.exportAuditLogs();
        break;
        
      case 'generate-audit-report':
        await this.generateAuditReport();
        break;
        
      case 'scan-server':
        await this.scanServer(message.serverId);
        break;
        
      case 'block-server':
        await this.blockServer(message.serverId);
        break;
        
      case 'unblock-server':
        await this.unblockServer(message.serverId);
        break;
    }
  }
  
  /**
   * Update webview with current data
   */
  private async updateWebview() {
    if (!this._view) return;
    
    // Get current security state
    const servers = await this.getServerInfo();
    const pendingRequests = this._securityManager.getPendingPermissionRequests();
    const incidents = await this._securityManager.getSecurityIncidents();
    const policies = this._securityManager.getSecurityPolicies();
    
    // Send to webview
    this._view.webview.postMessage({
      type: 'update-servers',
      servers
    });
    
    this._view.webview.postMessage({
      type: 'update-permission-requests',
      requests: pendingRequests
    });
    
    this._view.webview.postMessage({
      type: 'update-incidents',
      incidents
    });
    
    this._view.webview.postMessage({
      type: 'update-policies',
      policies
    });
  }
  
  /**
   * Show permission request notification
   */
  public async showPermissionRequest(request: PermissionRequest) {
    // Show notification
    const action = await vscode.window.showInformationMessage(
      `MCP Server "${request.serverName}" requests ${request.type} permission for: ${request.resource}`,
      'View Details',
      'Allow',
      'Deny'
    );
    
    if (action === 'View Details') {
      // Focus security panel
      await vscode.commands.executeCommand('symbiote.mcpSecurity.focus');
      
      // Send request to webview
      if (this._view) {
        this._view.webview.postMessage({
          type: 'new-permission-request',
          request
        });
      }
    } else if (action === 'Allow') {
      await this.handlePermissionDecision(request.id, {
        allowed: true,
        remember: false,
        scope: 'once',
        timestamp: new Date()
      });
    } else if (action === 'Deny') {
      await this.handlePermissionDecision(request.id, {
        allowed: false,
        remember: false,
        scope: 'once',
        timestamp: new Date()
      });
    }
  }
  
  /**
   * Handle permission decision
   */
  private async handlePermissionDecision(
    requestId: string,
    decision: PermissionDecision
  ) {
    await this._securityManager.handlePermissionDecision(requestId, decision);
    await this.updateWebview();
  }
  
  /**
   * Handle trust level update
   */
  private async handleTrustUpdate(
    serverId: string,
    trustLevel: string,
    reason: string
  ) {
    const username = vscode.workspace.getConfiguration('symbiote').get<string>('username') || 'user';
    
    await this._securityManager.setServerTrustLevel(
      serverId,
      trustLevel as any,
      reason,
      username
    );
    
    await this.updateWebview();
    
    vscode.window.showInformationMessage(
      `Trust level updated for server ${serverId}`
    );
  }
  
  /**
   * Show policy creator
   */
  private async showPolicyCreator() {
    // Create new policy using quick input
    const name = await vscode.window.showInputBox({
      prompt: 'Policy name',
      placeHolder: 'e.g., Strict Filesystem Access'
    });
    
    if (!name) return;
    
    const description = await vscode.window.showInputBox({
      prompt: 'Policy description',
      placeHolder: 'Describe what this policy does'
    });
    
    const defaultAction = await vscode.window.showQuickPick(
      ['allow', 'deny', 'prompt'],
      { placeHolder: 'Default action for unmatched requests' }
    );
    
    if (!defaultAction) return;
    
    // Create policy
    const policy = {
      id: `policy-${Date.now()}`,
      name,
      description: description || '',
      enabled: true,
      priority: 50,
      rules: [],
      defaultAction: defaultAction as any,
      appliesTo: []
    };
    
    await this._securityManager.addSecurityPolicy(policy);
    await this.updateWebview();
    
    vscode.window.showInformationMessage('Security policy created');
  }
  
  /**
   * Handle policy update
   */
  private async handlePolicyUpdate(policy: any) {
    await this._securityManager.updateSecurityPolicy(policy);
    await this.updateWebview();
  }
  
  /**
   * Handle policy deletion
   */
  private async handlePolicyDelete(policyId: string) {
    const confirm = await vscode.window.showWarningMessage(
      'Are you sure you want to delete this policy?',
      'Delete',
      'Cancel'
    );
    
    if (confirm === 'Delete') {
      await this._securityManager.removeSecurityPolicy(policyId);
      await this.updateWebview();
    }
  }
  
  /**
   * Handle incident resolution
   */
  private async handleIncidentResolution(
    incidentId: string,
    resolution: string
  ) {
    const username = vscode.workspace.getConfiguration('symbiote').get<string>('username') || 'user';
    
    await this._securityManager.resolveIncident(
      incidentId,
      resolution,
      username
    );
    
    await this.updateWebview();
    
    vscode.window.showInformationMessage('Incident resolved');
  }
  
  /**
   * Export audit logs
   */
  private async exportAuditLogs() {
    const uri = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file('mcp-audit-logs.json'),
      filters: {
        'JSON': ['json'],
        'CSV': ['csv']
      }
    });
    
    if (!uri) return;
    
    const format = uri.fsPath.endsWith('.csv') ? 'csv' : 'json';
    
    // Get server selection
    const servers = await this.getServerInfo();
    const serverName = await vscode.window.showQuickPick(
      ['All servers', ...servers.map(s => s.name)],
      { placeHolder: 'Select server to export logs for' }
    );
    
    if (!serverName) return;
    
    const serverId = serverName === 'All servers' ? 
      undefined : 
      servers.find(s => s.name === serverName)?.id;
    
    await this._securityManager.exportAuditLogs(
      serverId || '*',
      format,
      uri.fsPath
    );
    
    vscode.window.showInformationMessage(`Audit logs exported to ${uri.fsPath}`);
  }
  
  /**
   * Generate audit report
   */
  private async generateAuditReport() {
    const servers = await this.getServerInfo();
    const serverName = await vscode.window.showQuickPick(
      servers.map(s => s.name),
      { placeHolder: 'Select server for audit report' }
    );
    
    if (!serverName) return;
    
    const server = servers.find(s => s.name === serverName);
    if (!server) return;
    
    // Get date range
    const days = await vscode.window.showQuickPick(
      ['Last 24 hours', 'Last 7 days', 'Last 30 days', 'Custom'],
      { placeHolder: 'Select time period' }
    );
    
    if (!days) return;
    
    let startDate = new Date();
    let endDate = new Date();
    
    switch (days) {
      case 'Last 24 hours':
        startDate.setDate(startDate.getDate() - 1);
        break;
      case 'Last 7 days':
        startDate.setDate(startDate.getDate() - 7);
        break;
      case 'Last 30 days':
        startDate.setDate(startDate.getDate() - 30);
        break;
      case 'Custom':
        // TODO: Implement custom date picker
        return;
    }
    
    const summary = await this._securityManager.generateAuditSummary(
      server.id,
      startDate,
      endDate
    );
    
    // Show summary in new editor
    const doc = await vscode.workspace.openTextDocument({
      content: JSON.stringify(summary, null, 2),
      language: 'json'
    });
    
    await vscode.window.showTextDocument(doc);
  }
  
  /**
   * Scan server
   */
  private async scanServer(serverId: string) {
    vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: `Scanning MCP server ${serverId}...`,
      cancellable: false
    }, async () => {
      const result = await this._securityManager.scanServer(serverId);
      
      if (!result.passed) {
        vscode.window.showWarningMessage(
          `Security scan failed for ${serverId}: ${result.findings.length} issues found`
        );
      } else {
        vscode.window.showInformationMessage(
          `Security scan passed for ${serverId} (score: ${result.score})`
        );
      }
      
      await this.updateWebview();
    });
  }
  
  /**
   * Block server
   */
  private async blockServer(serverId: string) {
    const reason = await vscode.window.showInputBox({
      prompt: 'Reason for blocking server',
      placeHolder: 'Security violation, malicious behavior, etc.'
    });
    
    if (!reason) return;
    
    await this._securityManager.blockServer(serverId, reason);
    await this.updateWebview();
    
    vscode.window.showWarningMessage(`Server ${serverId} has been blocked`);
  }
  
  /**
   * Unblock server
   */
  private async unblockServer(serverId: string) {
    await this._securityManager.unblockServer(serverId);
    await this.updateWebview();
    
    vscode.window.showInformationMessage(`Server ${serverId} has been unblocked`);
  }
  
  /**
   * Get server info
   */
  private async getServerInfo() {
    // This would integrate with the MCP server manager
    // For now, return mock data
    return [];
  }
  
  /**
   * Setup security event handlers
   */
  private setupSecurityEventHandlers() {
    // Listen for permission requests
    this._securityManager.on('permission-request', (request: PermissionRequest) => {
      this.showPermissionRequest(request);
    });
    
    // Listen for security incidents
    this._securityManager.on('security-incident', (incident: any) => {
      vscode.window.showErrorMessage(
        `Security incident: ${incident.type} on server ${incident.serverName}`,
        'View Details'
      ).then(action => {
        if (action === 'View Details') {
          vscode.commands.executeCommand('symbiote.mcpSecurity.focus');
        }
      });
      
      if (this._view) {
        this.updateWebview();
      }
    });
    
    // Listen for critical findings
    this._securityManager.on('critical-findings', (event: any) => {
      vscode.window.showErrorMessage(
        `Critical security vulnerabilities found in ${event.serverId}`,
        'View Details'
      ).then(action => {
        if (action === 'View Details') {
          vscode.commands.executeCommand('symbiote.mcpSecurity.focus');
        }
      });
    });
  }
  
  /**
   * Get HTML for webview
   */
  private _getHtmlForWebview(webview: vscode.Webview) {
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'out', 'security-panel.js')
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'out', 'security-panel.css')
    );
    
    const nonce = getNonce();
    
    return `<!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
        <link href="${styleUri}" rel="stylesheet">
        <title>MCP Security</title>
      </head>
      <body>
        <div id="root"></div>
        <script nonce="${nonce}">
          const vscode = acquireVsCodeApi();
        </script>
        <script nonce="${nonce}" src="${scriptUri}"></script>
      </body>
      </html>`;
  }
}

function getNonce() {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}