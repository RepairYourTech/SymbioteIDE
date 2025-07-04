/**
 * MCP Security Extension
 * 
 * Registers security features with VS Code
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { SecurityManager } from '../security-manager';
import { SecurityPanelProvider } from '../ui/security-panel-provider';
import { SecureMCPServerManager } from '../integration/secure-server-manager';
import { MCPServerManager } from '../../server-manager';
import { TrustLevel } from '../types';

export class MCPSecurityExtension {
  private context: vscode.ExtensionContext;
  private securityManager: SecurityManager;
  private securityPanelProvider: SecurityPanelProvider;
  private secureServerManager: SecureMCPServerManager;
  private statusBarItem: vscode.StatusBarItem;
  
  constructor(
    context: vscode.ExtensionContext,
    serverManager: MCPServerManager
  ) {
    this.context = context;
    
    // Initialize security manager
    const dataDirectory = path.join(context.globalStorageUri.fsPath, 'mcp-security');
    this.securityManager = new SecurityManager({
      dataDirectory,
      defaultPolicy: this.getDefaultSecurityPolicy(),
      enableSandbox: true,
      enableScanning: true,
      enableAuditing: true
    });
    
    // Initialize secure server manager
    this.secureServerManager = new SecureMCPServerManager(
      serverManager,
      this.securityManager
    );
    
    // Initialize UI provider
    this.securityPanelProvider = new SecurityPanelProvider(
      context.extensionUri,
      this.securityManager
    );
    
    // Create status bar item
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );
  }
  
  /**
   * Activate security extension
   */
  async activate(): Promise<void> {
    // Initialize components
    await this.securityManager.initialize();
    await this.secureServerManager.initialize();
    
    // Register webview provider
    this.context.subscriptions.push(
      vscode.window.registerWebviewViewProvider(
        SecurityPanelProvider.viewType,
        this.securityPanelProvider
      )
    );
    
    // Register commands
    this.registerCommands();
    
    // Setup status bar
    this.updateStatusBar();
    this.statusBarItem.show();
    
    // Setup event handlers
    this.setupEventHandlers();
    
    // Show activation message
    vscode.window.showInformationMessage('MCP Security layer activated');
  }
  
  /**
   * Register commands
   */
  private registerCommands(): void {
    // Security panel commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.showPanel', () => {
        vscode.commands.executeCommand('symbiote.mcpSecurity.focus');
      })
    );
    
    // Server security commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.scanServer', async (serverId?: string) => {
        if (!serverId) {
          serverId = await this.selectServer('Select server to scan');
          if (!serverId) return;
        }
        
        await vscode.window.withProgress({
          location: vscode.ProgressLocation.Notification,
          title: 'Scanning MCP server...',
          cancellable: false
        }, async () => {
          const result = await this.securityManager.scanServer(serverId!);
          
          if (result.passed) {
            vscode.window.showInformationMessage(
              `Security scan passed (score: ${result.score})`
            );
          } else {
            const action = await vscode.window.showWarningMessage(
              `Security scan found ${result.findings.length} issues`,
              'View Report'
            );
            
            if (action === 'View Report') {
              await this.showScanReport(result);
            }
          }
        });
      })
    );
    
    // Trust management commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.setTrustLevel', async (serverId?: string) => {
        if (!serverId) {
          serverId = await this.selectServer('Select server to update trust level');
          if (!serverId) return;
        }
        
        const trustLevel = await vscode.window.showQuickPick([
          { label: 'Trusted', value: TrustLevel.Trusted },
          { label: 'Verified', value: TrustLevel.Verified },
          { label: 'Unknown', value: TrustLevel.Unknown },
          { label: 'Restricted', value: TrustLevel.Restricted },
          { label: 'Untrusted', value: TrustLevel.Untrusted }
        ], {
          placeHolder: 'Select trust level'
        });
        
        if (!trustLevel) return;
        
        const reason = await vscode.window.showInputBox({
          prompt: 'Reason for trust level change',
          placeHolder: 'e.g., Passed security review'
        });
        
        if (!reason) return;
        
        await this.secureServerManager.updateTrustLevel(
          serverId,
          trustLevel.value,
          reason
        );
        
        vscode.window.showInformationMessage('Trust level updated');
      })
    );
    
    // Policy management commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.createPolicy', async () => {
        await this.createSecurityPolicy();
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.importPolicy', async () => {
        const uri = await vscode.window.showOpenDialog({
          canSelectFiles: true,
          canSelectFolders: false,
          canSelectMany: false,
          filters: {
            'Security Policy': ['json']
          }
        });
        
        if (!uri || uri.length === 0) return;
        
        const content = await vscode.workspace.fs.readFile(uri[0]);
        const policy = JSON.parse(content.toString());
        
        await this.securityManager.addSecurityPolicy(policy);
        vscode.window.showInformationMessage('Security policy imported');
      })
    );
    
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.exportPolicy', async () => {
        const policies = this.securityManager.getSecurityPolicies();
        const policy = await vscode.window.showQuickPick(
          policies.map(p => ({ label: p.name, policy: p })),
          { placeHolder: 'Select policy to export' }
        );
        
        if (!policy) return;
        
        const uri = await vscode.window.showSaveDialog({
          defaultUri: vscode.Uri.file(`${policy.policy.name}.json`),
          filters: {
            'Security Policy': ['json']
          }
        });
        
        if (!uri) return;
        
        await vscode.workspace.fs.writeFile(
          uri,
          Buffer.from(JSON.stringify(policy.policy, null, 2))
        );
        
        vscode.window.showInformationMessage('Security policy exported');
      })
    );
    
    // Incident management commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.showIncident', async (incidentId: string) => {
        const incidents = await this.securityManager.getSecurityIncidents();
        const incident = incidents.find(i => i.id === incidentId);
        
        if (!incident) {
          vscode.window.showErrorMessage('Incident not found');
          return;
        }
        
        const doc = await vscode.workspace.openTextDocument({
          content: JSON.stringify(incident, null, 2),
          language: 'json'
        });
        
        await vscode.window.showTextDocument(doc);
      })
    );
    
    // Audit commands
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.viewAuditLog', async () => {
        const serverId = await this.selectServer('Select server to view audit log');
        if (!serverId) return;
        
        const events = await this.securityManager.queryAuditEvents(serverId);
        
        const doc = await vscode.workspace.openTextDocument({
          content: events.map(e => JSON.stringify(e)).join('\n'),
          language: 'jsonl'
        });
        
        await vscode.window.showTextDocument(doc);
      })
    );
    
    // Quick actions
    this.context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcpSecurity.blockServer', async (serverId?: string) => {
        if (!serverId) {
          serverId = await this.selectServer('Select server to block');
          if (!serverId) return;
        }
        
        const reason = await vscode.window.showInputBox({
          prompt: 'Reason for blocking server',
          placeHolder: 'e.g., Security violation'
        });
        
        if (!reason) return;
        
        await this.securityManager.blockServer(serverId, reason);
        vscode.window.showWarningMessage(`Server ${serverId} has been blocked`);
      })
    );
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Permission request notifications
    this.secureServerManager.on('permission-request', async (request) => {
      await this.securityPanelProvider.showPermissionRequest(request);
    });
    
    // Security incident alerts
    this.secureServerManager.on('security-incident', async (incident) => {
      const action = await vscode.window.showErrorMessage(
        `Security incident: ${incident.description}`,
        'View Details',
        'Block Server'
      );
      
      if (action === 'View Details') {
        await vscode.commands.executeCommand('symbiote.mcpSecurity.showIncident', incident.id);
      } else if (action === 'Block Server') {
        await vscode.commands.executeCommand('symbiote.mcpSecurity.blockServer', incident.serverId);
      }
    });
    
    // Trust level changes
    this.secureServerManager.on('trust-updated', () => {
      this.updateStatusBar();
    });
    
    // Update status bar periodically
    setInterval(() => {
      this.updateStatusBar();
    }, 10000);
  }
  
  /**
   * Update status bar
   */
  private updateStatusBar(): void {
    const metrics = this.securityManager.getSecurityMetrics();
    
    let icon = '🛡️';
    let tooltip = 'MCP Security\n';
    
    if (metrics.activeIncidents > 0) {
      icon = '🚨';
      tooltip += `${metrics.activeIncidents} active incidents\n`;
    } else if (metrics.pendingRequests > 0) {
      icon = '🔔';
      tooltip += `${metrics.pendingRequests} pending requests\n`;
    }
    
    tooltip += `${metrics.activeServers} active servers\n`;
    tooltip += `${metrics.trustedServers} trusted, ${metrics.untrustedServers} untrusted`;
    
    this.statusBarItem.text = `${icon} MCP Security`;
    this.statusBarItem.tooltip = tooltip;
    this.statusBarItem.command = 'symbiote.mcpSecurity.showPanel';
  }
  
  /**
   * Select server
   */
  private async selectServer(placeHolder: string): Promise<string | undefined> {
    const servers = this.secureServerManager.getSecureServerInfo();
    
    if (servers.length === 0) {
      vscode.window.showInformationMessage('No MCP servers available');
      return undefined;
    }
    
    const selected = await vscode.window.showQuickPick(
      servers.map(s => ({
        label: s.serverName,
        description: `${s.trustLevel} - ${s.status}`,
        serverId: s.serverId
      })),
      { placeHolder }
    );
    
    return selected?.serverId;
  }
  
  /**
   * Create security policy
   */
  private async createSecurityPolicy(): Promise<void> {
    // Use multi-step input to create policy
    const name = await vscode.window.showInputBox({
      prompt: 'Policy name',
      placeHolder: 'e.g., Development Environment Policy'
    });
    
    if (!name) return;
    
    const description = await vscode.window.showInputBox({
      prompt: 'Policy description',
      placeHolder: 'Describe what this policy controls'
    });
    
    const defaultAction = await vscode.window.showQuickPick(
      ['allow', 'deny', 'prompt'],
      { placeHolder: 'Default action for unmatched requests' }
    );
    
    if (!defaultAction) return;
    
    // Create basic policy
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
    
    // Add some default rules
    const addDefaultRules = await vscode.window.showQuickPick(
      ['Yes', 'No'],
      { placeHolder: 'Add default security rules?' }
    );
    
    if (addDefaultRules === 'Yes') {
      policy.rules = [
        {
          id: 'block-system-files',
          name: 'Block System Files',
          type: 'filesystem',
          resource: '/etc/*,/System/*,C:\\Windows\\*',
          action: 'deny',
          conditions: []
        },
        {
          id: 'block-ssh-keys',
          name: 'Block SSH Keys',
          type: 'filesystem',
          resource: '*/.ssh/*,*id_rsa*,*id_dsa*',
          action: 'deny',
          conditions: []
        },
        {
          id: 'restrict-network',
          name: 'Restrict Network Access',
          type: 'network',
          resource: 'localhost,127.0.0.1,::1',
          action: 'deny',
          conditions: []
        }
      ];
    }
    
    await this.securityManager.addSecurityPolicy(policy);
    vscode.window.showInformationMessage(`Security policy "${name}" created`);
  }
  
  /**
   * Show scan report
   */
  private async showScanReport(result: any): Promise<void> {
    const doc = await vscode.workspace.openTextDocument({
      content: this.formatScanReport(result),
      language: 'markdown'
    });
    
    await vscode.window.showTextDocument(doc);
  }
  
  /**
   * Format scan report as markdown
   */
  private formatScanReport(result: any): string {
    let markdown = `# Security Scan Report\n\n`;
    markdown += `**Server ID**: ${result.serverId}\n`;
    markdown += `**Scan Date**: ${result.timestamp}\n`;
    markdown += `**Status**: ${result.passed ? '✅ PASSED' : '❌ FAILED'}\n`;
    markdown += `**Score**: ${result.score}/100\n\n`;
    
    if (result.findings.length > 0) {
      markdown += `## Findings (${result.findings.length})\n\n`;
      
      const grouped = result.findings.reduce((acc: any, finding: any) => {
        if (!acc[finding.severity]) acc[finding.severity] = [];
        acc[finding.severity].push(finding);
        return acc;
      }, {});
      
      for (const [severity, findings] of Object.entries(grouped)) {
        markdown += `### ${severity.toUpperCase()} (${(findings as any[]).length})\n\n`;
        
        for (const finding of findings as any[]) {
          markdown += `- **${finding.title}**\n`;
          markdown += `  - ${finding.description}\n`;
          if (finding.location) {
            markdown += `  - Location: ${finding.location.file}:${finding.location.line}\n`;
          }
          if (finding.recommendation) {
            markdown += `  - Recommendation: ${finding.recommendation}\n`;
          }
          markdown += `\n`;
        }
      }
    }
    
    if (result.recommendations.length > 0) {
      markdown += `## Recommendations\n\n`;
      for (const rec of result.recommendations) {
        markdown += `- ${rec}\n`;
      }
    }
    
    return markdown;
  }
  
  /**
   * Get default security policy
   */
  private getDefaultSecurityPolicy(): any {
    return {
      id: 'default',
      name: 'Default Security Policy',
      description: 'Default security policy for MCP servers',
      enabled: true,
      priority: 0,
      rules: [
        {
          id: 'deny-system-access',
          name: 'Deny System Access',
          type: 'filesystem',
          resource: '/etc/*,/System/*,/Windows/System32/*',
          action: 'deny',
          conditions: []
        },
        {
          id: 'deny-credential-access',
          name: 'Deny Credential Access',
          type: 'filesystem',
          resource: '*/.ssh/*,*/.aws/*,*/.git-credentials',
          action: 'deny',
          conditions: []
        },
        {
          id: 'prompt-network-access',
          name: 'Prompt for Network Access',
          type: 'network',
          resource: '*',
          action: 'prompt',
          conditions: []
        },
        {
          id: 'limit-process-spawn',
          name: 'Limit Process Spawning',
          type: 'process',
          resource: '*',
          action: 'prompt',
          conditions: []
        }
      ],
      defaultAction: 'prompt',
      appliesTo: []
    };
  }
  
  /**
   * Get secure server manager
   */
  getSecureServerManager(): SecureMCPServerManager {
    return this.secureServerManager;
  }
  
  /**
   * Deactivate extension
   */
  async deactivate(): Promise<void> {
    this.statusBarItem.dispose();
    await this.securityManager.cleanup();
  }
}