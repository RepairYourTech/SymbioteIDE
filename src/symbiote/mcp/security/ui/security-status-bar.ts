/**
 * MCP Security Status Bar
 * 
 * Shows security status and alerts in VS Code status bar
 */

import * as vscode from 'vscode';
import { SecurityManager } from '../security-manager';
import { TrustLevel } from '../types';

export class SecurityStatusBar {
  private statusBarItem: vscode.StatusBarItem;
  private securityManager: SecurityManager;
  
  constructor(securityManager: SecurityManager) {
    this.securityManager = securityManager;
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      90
    );
    
    this.setupStatusBar();
    this.setupEventListeners();
  }
  
  private setupStatusBar(): void {
    this.statusBarItem.command = 'symbiote.mcp.showSecurityPanel';
    this.updateStatus();
    this.statusBarItem.show();
  }
  
  private setupEventListeners(): void {
    // Update on security events
    this.securityManager.on('securityAlert', () => {
      this.updateStatus(true);
    });
    
    this.securityManager.on('trustLevelChanged', () => {
      this.updateStatus();
    });
    
    this.securityManager.on('violationReported', () => {
      this.updateStatus(true);
    });
  }
  
  private async updateStatus(hasAlert: boolean = false): Promise<void> {
    const stats = await this.getSecurityStats();
    
    if (hasAlert) {
      this.statusBarItem.text = '$(shield) MCP Security: $(alert) Alert';
      this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
      this.statusBarItem.tooltip = 'Click to view security alerts';
      
      // Clear alert after 5 seconds
      setTimeout(() => this.updateStatus(), 5000);
    } else if (stats.violations > 0) {
      this.statusBarItem.text = `$(shield) MCP Security: ${stats.violations} Issues`;
      this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
      this.statusBarItem.tooltip = `${stats.violations} security violations detected`;
    } else {
      this.statusBarItem.text = '$(shield) MCP Security';
      this.statusBarItem.backgroundColor = undefined;
      this.statusBarItem.tooltip = this.getTooltip(stats);
    }
  }
  
  private async getSecurityStats(): Promise<{
    servers: number;
    trusted: number;
    violations: number;
  }> {
    const servers = await this.securityManager.getAllServerTrust();
    let trusted = 0;
    let violations = 0;
    
    for (const [serverId, trustLevel] of servers) {
      if (trustLevel === TrustLevel.Trusted || trustLevel === TrustLevel.UserTrusted) {
        trusted++;
      }
      
      const serverViolations = await this.securityManager.getServerViolations(serverId);
      violations += serverViolations.length;
    }
    
    return {
      servers: servers.size,
      trusted,
      violations
    };
  }
  
  private getTooltip(stats: { servers: number; trusted: number; violations: number }): string {
    const lines = [
      'MCP Security Status',
      '',
      `Servers: ${stats.servers}`,
      `Trusted: ${stats.trusted}`,
      `Violations: ${stats.violations}`,
      '',
      'Click to open security panel'
    ];
    
    return lines.join('\n');
  }
  
  public dispose(): void {
    this.statusBarItem.dispose();
  }
}