/**
 * MCP Status Bar Manager
 * 
 * Manages status bar items for MCP extension
 */

import * as vscode from 'vscode';

export class MCPStatusBarManager {
  private mainItem: vscode.StatusBarItem;
  private serverItem: vscode.StatusBarItem;
  private status: 'loading' | 'ready' | 'error' = 'loading';
  private activeServers: Map<string, ServerStatus> = new Map();
  
  constructor() {
    this.mainItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      1000
    );
    
    this.serverItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      999
    );
  }
  
  /**
   * Create status bar items
   */
  createStatusBarItems(): void {
    // Main MCP item
    this.mainItem.text = '$(loading~spin) MCP';
    this.mainItem.tooltip = 'Model Context Protocol - Loading...';
    this.mainItem.command = 'symbiote.mcp.quickAction';
    this.mainItem.show();
    
    // Server status item
    this.updateServerStatusItem();
  }
  
  /**
   * Update main status
   */
  updateStatus(status: 'loading' | 'ready' | 'error', message?: string): void {
    this.status = status;
    
    switch (status) {
      case 'loading':
        this.mainItem.text = '$(loading~spin) MCP';
        this.mainItem.tooltip = message || 'Model Context Protocol - Loading...';
        this.mainItem.backgroundColor = undefined;
        break;
        
      case 'ready':
        this.mainItem.text = '$(plug) MCP';
        this.mainItem.tooltip = message || 'Model Context Protocol - Ready';
        this.mainItem.backgroundColor = undefined;
        break;
        
      case 'error':
        this.mainItem.text = '$(error) MCP';
        this.mainItem.tooltip = message || 'Model Context Protocol - Error';
        this.mainItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        break;
    }
  }
  
  /**
   * Update server status
   */
  updateServerStatus(serverId: string, status: ServerStatusType, name?: string): void {
    this.activeServers.set(serverId, {
      id: serverId,
      name: name || serverId,
      status,
      lastUpdate: new Date()
    });
    
    this.updateServerStatusItem();
  }
  
  /**
   * Remove server status
   */
  removeServerStatus(serverId: string): void {
    this.activeServers.delete(serverId);
    this.updateServerStatusItem();
  }
  
  /**
   * Refresh all status items
   */
  refresh(): void {
    this.updateServerStatusItem();
  }
  
  /**
   * Get main status bar item
   */
  getMainItem(): vscode.StatusBarItem {
    return this.mainItem;
  }
  
  /**
   * Get server status bar item
   */
  getServerItem(): vscode.StatusBarItem {
    return this.serverItem;
  }
  
  /**
   * Dispose status bar items
   */
  dispose(): void {
    this.mainItem.dispose();
    this.serverItem.dispose();
  }
  
  /**
   * Update server status item
   */
  private updateServerStatusItem(): void {
    const servers = Array.from(this.activeServers.values());
    const running = servers.filter(s => s.status === 'running').length;
    const error = servers.filter(s => s.status === 'error').length;
    const total = servers.length;
    
    if (total === 0) {
      this.serverItem.hide();
      return;
    }
    
    // Build status text
    let text = '';
    let tooltip = 'MCP Servers\n';
    
    if (running > 0) {
      text += `$(server) ${running}`;
    }
    
    if (error > 0) {
      text += ` $(error) ${error}`;
    }
    
    if (text === '') {
      text = `$(server) ${total}`;
    }
    
    // Build tooltip
    for (const server of servers) {
      const icon = this.getServerStatusIcon(server.status);
      tooltip += `\n${icon} ${server.name}`;
    }
    
    this.serverItem.text = text;
    this.serverItem.tooltip = tooltip;
    this.serverItem.command = 'symbiote.mcp.showPanel';
    
    // Set background color if there are errors
    if (error > 0) {
      this.serverItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else {
      this.serverItem.backgroundColor = undefined;
    }
    
    this.serverItem.show();
  }
  
  /**
   * Get server status icon
   */
  private getServerStatusIcon(status: ServerStatusType): string {
    switch (status) {
      case 'running':
        return '$(circle-filled)';
      case 'stopped':
        return '$(circle-outline)';
      case 'error':
        return '$(error)';
      case 'starting':
        return '$(loading~spin)';
      case 'stopping':
        return '$(loading~spin)';
      default:
        return '$(question)';
    }
  }
}

type ServerStatusType = 'running' | 'stopped' | 'error' | 'starting' | 'stopping';

interface ServerStatus {
  id: string;
  name: string;
  status: ServerStatusType;
  lastUpdate: Date;
}