/**
 * MCP Definition Provider
 * 
 * Provides go-to-definition functionality from MCP servers
 */

import * as vscode from 'vscode';
import { MCPServerManager } from '../server-manager';

export class MCPDefinitionProvider implements vscode.DefinitionProvider {
  private serverManager: MCPServerManager;
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
  }
  
  async provideDefinition(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken
  ): Promise<vscode.Definition | vscode.LocationLink[] | undefined> {
    // Get word at position
    const wordRange = document.getWordRangeAtPosition(position);
    if (!wordRange) return undefined;
    
    const word = document.getText(wordRange);
    const locations: vscode.LocationLink[] = [];
    
    // Query all connected MCP servers for definitions
    const servers = this.serverManager.getServers();
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      if (!client || !client.isConnected()) continue;
      
      try {
        // Check if server supports definitions
        const serverInfo = await client.getServerInfo();
        if (!serverInfo.capabilities?.definition) continue;
        
        // Request definition from server
        const definitions = await client.getDefinition({
          uri: document.uri.toString(),
          position: {
            line: position.line,
            character: position.character
          }
        });
        
        if (!definitions || definitions.length === 0) continue;
        
        // Convert to VS Code location links
        for (const definition of definitions) {
          const targetUri = vscode.Uri.parse(definition.uri);
          const targetRange = new vscode.Range(
            definition.range.start.line,
            definition.range.start.character,
            definition.range.end.line,
            definition.range.end.character
          );
          
          // Create location link
          const locationLink: vscode.LocationLink = {
            targetUri,
            targetRange,
            targetSelectionRange: definition.selectionRange
              ? new vscode.Range(
                  definition.selectionRange.start.line,
                  definition.selectionRange.start.character,
                  definition.selectionRange.end.line,
                  definition.selectionRange.end.character
                )
              : targetRange,
            originSelectionRange: wordRange
          };
          
          locations.push(locationLink);
        }
      } catch (error) {
        console.error(`Failed to get definitions from ${serverId}:`, error);
      }
    }
    
    return locations.length > 0 ? locations : undefined;
  }
}