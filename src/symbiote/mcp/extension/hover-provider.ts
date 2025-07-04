/**
 * MCP Hover Provider
 * 
 * Provides hover information from MCP servers
 */

import * as vscode from 'vscode';
import { MCPServerManager } from '../server-manager';

export class MCPHoverProvider implements vscode.HoverProvider {
  private serverManager: MCPServerManager;
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
  }
  
  async provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken
  ): Promise<vscode.Hover | undefined> {
    // Get word at position
    const wordRange = document.getWordRangeAtPosition(position);
    if (!wordRange) return undefined;
    
    const word = document.getText(wordRange);
    
    // Query all connected MCP servers for hover info
    const servers = this.serverManager.getServers();
    const contents: vscode.MarkdownString[] = [];
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      if (!client || !client.isConnected()) continue;
      
      try {
        // Check if server supports hover
        const serverInfo = await client.getServerInfo();
        if (!serverInfo.capabilities?.hover) continue;
        
        // Request hover info from server
        const hover = await client.getHover({
          uri: document.uri.toString(),
          position: {
            line: position.line,
            character: position.character
          }
        });
        
        if (hover && hover.contents) {
          const content = new vscode.MarkdownString();
          content.supportHtml = true;
          content.isTrusted = true;
          
          // Add server name as header
          content.appendMarkdown(`**[${server.name}]**\n\n`);
          
          // Add hover contents
          if (typeof hover.contents === 'string') {
            content.appendMarkdown(hover.contents);
          } else if (Array.isArray(hover.contents)) {
            for (const item of hover.contents) {
              if (typeof item === 'string') {
                content.appendMarkdown(item + '\n\n');
              } else if (item.language && item.value) {
                content.appendCodeblock(item.value, item.language);
              }
            }
          }
          
          contents.push(content);
        }
      } catch (error) {
        console.error(`Failed to get hover from ${serverId}:`, error);
      }
    }
    
    if (contents.length === 0) return undefined;
    
    return new vscode.Hover(contents, wordRange);
  }
}