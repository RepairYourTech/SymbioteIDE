/**
 * MCP Diagnostic Manager
 * 
 * Manages diagnostics for MCP servers and configurations
 */

import * as vscode from 'vscode';
import { MCPServerManager } from '../server-manager';

export class MCPDiagnosticManager {
  private diagnosticCollection: vscode.DiagnosticCollection;
  private configDiagnostics: Map<string, vscode.Diagnostic[]> = new Map();
  
  constructor() {
    this.diagnosticCollection = vscode.languages.createDiagnosticCollection('mcp');
  }
  
  /**
   * Register diagnostic collection
   */
  registerDiagnosticCollection(context: vscode.ExtensionContext): void {
    context.subscriptions.push(this.diagnosticCollection);
  }
  
  /**
   * Add diagnostic
   */
  addDiagnostic(
    uri: vscode.Uri,
    diagnostic: vscode.Diagnostic
  ): void {
    const diagnostics = this.diagnosticCollection.get(uri) || [];
    diagnostics.push(diagnostic);
    this.diagnosticCollection.set(uri, diagnostics);
  }
  
  /**
   * Clear diagnostics for URI
   */
  clearDiagnostics(uri: vscode.Uri): void {
    this.diagnosticCollection.delete(uri);
  }
  
  /**
   * Clear all diagnostics
   */
  clearAllDiagnostics(): void {
    this.diagnosticCollection.clear();
  }
  
  /**
   * Validate MCP configuration file
   */
  async validateMCPConfig(uri: vscode.Uri): Promise<void> {
    const diagnostics: vscode.Diagnostic[] = [];
    
    try {
      const document = await vscode.workspace.openTextDocument(uri);
      const text = document.getText();
      
      // Parse JSON
      let config: any;
      try {
        config = JSON.parse(text);
      } catch (error) {
        const match = /at position (\d+)/.exec(error.message);
        const position = match ? parseInt(match[1], 10) : 0;
        const pos = document.positionAt(position);
        
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(pos, pos.translate(0, 1)),
          `JSON parse error: ${error.message}`,
          vscode.DiagnosticSeverity.Error
        ));
        
        this.diagnosticCollection.set(uri, diagnostics);
        return;
      }
      
      // Validate structure
      if (!config.mcpServers) {
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(0, 0, 0, 1),
          'Missing required property "mcpServers"',
          vscode.DiagnosticSeverity.Error
        ));
      } else {
        // Validate each server
        for (const [serverId, serverConfig] of Object.entries(config.mcpServers)) {
          await this.validateServerConfig(
            document,
            serverId,
            serverConfig as any,
            diagnostics
          );
        }
      }
      
    } catch (error) {
      console.error('Failed to validate MCP config:', error);
    }
    
    this.diagnosticCollection.set(uri, diagnostics);
  }
  
  /**
   * Validate server configuration
   */
  private async validateServerConfig(
    document: vscode.TextDocument,
    serverId: string,
    config: any,
    diagnostics: vscode.Diagnostic[]
  ): Promise<void> {
    // Find server position in document
    const serverMatch = new RegExp(`"${serverId}"\\s*:`).exec(document.getText());
    const serverPos = serverMatch ? document.positionAt(serverMatch.index) : new vscode.Position(0, 0);
    
    // Check required fields
    if (!config.command && !config.url) {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(serverPos, serverPos.translate(0, serverId.length + 2)),
        `Server "${serverId}": Either "command" or "url" is required`,
        vscode.DiagnosticSeverity.Error
      ));
    }
    
    // Check command exists
    if (config.command && typeof config.command === 'string') {
      const commandExists = await this.checkCommandExists(config.command);
      if (!commandExists) {
        const commandMatch = new RegExp(`"command"\\s*:\\s*"${config.command}"`).exec(document.getText());
        if (commandMatch) {
          const commandPos = document.positionAt(commandMatch.index);
          diagnostics.push(new vscode.Diagnostic(
            new vscode.Range(commandPos, commandPos.translate(0, commandMatch[0].length)),
            `Command "${config.command}" not found in PATH`,
            vscode.DiagnosticSeverity.Warning
          ));
        }
      }
    }
    
    // Check args type
    if (config.args && !Array.isArray(config.args)) {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(serverPos, serverPos.translate(1, 0)),
        `Server "${serverId}": "args" must be an array`,
        vscode.DiagnosticSeverity.Error
      ));
    }
    
    // Check env type
    if (config.env && typeof config.env !== 'object') {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(serverPos, serverPos.translate(1, 0)),
        `Server "${serverId}": "env" must be an object`,
        vscode.DiagnosticSeverity.Error
      ));
    }
    
    // Check transport
    if (config.transport && !['stdio', 'http', 'websocket'].includes(config.transport)) {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(serverPos, serverPos.translate(1, 0)),
        `Server "${serverId}": Invalid transport "${config.transport}"`,
        vscode.DiagnosticSeverity.Error
      ));
    }
    
    // Check URL format
    if (config.url && typeof config.url === 'string') {
      try {
        new URL(config.url);
      } catch {
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(serverPos, serverPos.translate(1, 0)),
          `Server "${serverId}": Invalid URL format`,
          vscode.DiagnosticSeverity.Error
        ));
      }
    }
  }
  
  /**
   * Check if command exists
   */
  private async checkCommandExists(command: string): Promise<boolean> {
    // Simple check - could be improved
    if (command.includes('/') || command.includes('\\')) {
      // Absolute path
      try {
        await vscode.workspace.fs.stat(vscode.Uri.file(command));
        return true;
      } catch {
        return false;
      }
    }
    
    // Command in PATH - harder to check, assume it exists
    return true;
  }
  
  /**
   * Check server health
   */
  async checkServerHealth(
    serverManager: MCPServerManager,
    serverId: string
  ): Promise<vscode.Diagnostic[]> {
    const diagnostics: vscode.Diagnostic[] = [];
    const client = serverManager.getClient(serverId);
    
    if (!client) {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 1),
        `Server "${serverId}" not found`,
        vscode.DiagnosticSeverity.Error
      ));
      return diagnostics;
    }
    
    if (!client.isConnected()) {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 1),
        `Server "${serverId}" is not connected`,
        vscode.DiagnosticSeverity.Warning
      ));
      return diagnostics;
    }
    
    // Check server capabilities
    try {
      const info = await client.getServerInfo();
      
      if (!info.capabilities || Object.keys(info.capabilities).length === 0) {
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(0, 0, 0, 1),
          `Server "${serverId}" reports no capabilities`,
          vscode.DiagnosticSeverity.Information
        ));
      }
      
      // Check protocol version
      if (info.protocolVersion !== '2024-11-05') {
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(0, 0, 0, 1),
          `Server "${serverId}" uses protocol version ${info.protocolVersion} (expected 2024-11-05)`,
          vscode.DiagnosticSeverity.Warning
        ));
      }
      
    } catch (error) {
      diagnostics.push(new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 1),
        `Failed to get server info: ${error.message}`,
        vscode.DiagnosticSeverity.Error
      ));
    }
    
    return diagnostics;
  }
  
  /**
   * Report server error
   */
  reportServerError(serverId: string, error: Error): void {
    // Create a virtual document URI for server errors
    const uri = vscode.Uri.parse(`mcp://server/${serverId}/errors`);
    
    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 1),
      error.message,
      vscode.DiagnosticSeverity.Error
    );
    
    diagnostic.source = 'MCP';
    diagnostic.code = error.name;
    
    this.addDiagnostic(uri, diagnostic);
  }
  
  /**
   * Dispose resources
   */
  dispose(): void {
    this.diagnosticCollection.dispose();
  }
}