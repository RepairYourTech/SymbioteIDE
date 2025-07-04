/**
 * MCP Completion Provider
 * 
 * Provides code completions from MCP servers
 */

import * as vscode from 'vscode';
import { MCPServerManager } from '../server-manager';

export class MCPCompletionProvider implements vscode.CompletionItemProvider {
  private serverManager: MCPServerManager;
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
  }
  
  async provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    context: vscode.CompletionContext
  ): Promise<vscode.CompletionItem[] | vscode.CompletionList> {
    const items: vscode.CompletionItem[] = [];
    
    // Get completions from all connected MCP servers
    const servers = this.serverManager.getServers();
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      if (!client || !client.isConnected()) continue;
      
      try {
        // Check if server supports completions
        const serverInfo = await client.getServerInfo();
        if (!serverInfo.capabilities?.completion) continue;
        
        // Request completions from server
        const completions = await client.getCompletion({
          uri: document.uri.toString(),
          position: {
            line: position.line,
            character: position.character
          },
          context: {
            triggerKind: context.triggerKind,
            triggerCharacter: context.triggerCharacter
          }
        });
        
        // Convert to VS Code completion items
        for (const completion of completions) {
          const item = new vscode.CompletionItem(
            completion.label,
            this.getCompletionItemKind(completion.kind)
          );
          
          item.detail = completion.detail;
          item.documentation = completion.documentation;
          item.insertText = completion.insertText || completion.label;
          item.filterText = completion.filterText;
          item.sortText = completion.sortText;
          
          if (completion.textEdit) {
            item.range = new vscode.Range(
              completion.textEdit.range.start.line,
              completion.textEdit.range.start.character,
              completion.textEdit.range.end.line,
              completion.textEdit.range.end.character
            );
          }
          
          items.push(item);
        }
      } catch (error) {
        console.error(`Failed to get completions from ${serverId}:`, error);
      }
    }
    
    return items;
  }
  
  private getCompletionItemKind(kind?: string): vscode.CompletionItemKind {
    switch (kind) {
      case 'method': return vscode.CompletionItemKind.Method;
      case 'function': return vscode.CompletionItemKind.Function;
      case 'variable': return vscode.CompletionItemKind.Variable;
      case 'class': return vscode.CompletionItemKind.Class;
      case 'interface': return vscode.CompletionItemKind.Interface;
      case 'module': return vscode.CompletionItemKind.Module;
      case 'property': return vscode.CompletionItemKind.Property;
      case 'constant': return vscode.CompletionItemKind.Constant;
      case 'enum': return vscode.CompletionItemKind.Enum;
      case 'keyword': return vscode.CompletionItemKind.Keyword;
      case 'snippet': return vscode.CompletionItemKind.Snippet;
      default: return vscode.CompletionItemKind.Text;
    }
  }
}