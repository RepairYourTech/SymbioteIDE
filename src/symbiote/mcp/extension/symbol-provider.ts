/**
 * MCP Symbol Provider
 * 
 * Provides workspace and document symbols from MCP servers
 */

import * as vscode from 'vscode';
import { MCPServerManager } from '../server-manager';

export class MCPDocumentSymbolProvider implements vscode.DocumentSymbolProvider {
  private serverManager: MCPServerManager;
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
  }
  
  async provideDocumentSymbols(
    document: vscode.TextDocument,
    token: vscode.CancellationToken
  ): Promise<vscode.SymbolInformation[] | vscode.DocumentSymbol[]> {
    const symbols: vscode.DocumentSymbol[] = [];
    const servers = this.serverManager.getServers();
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      if (!client || !client.isConnected()) continue;
      
      try {
        // Check if server supports document symbols
        const serverInfo = await client.getServerInfo();
        if (!serverInfo.capabilities?.documentSymbol) continue;
        
        // Request symbols from server
        const serverSymbols = await client.getDocumentSymbols({
          uri: document.uri.toString()
        });
        
        if (!serverSymbols || serverSymbols.length === 0) continue;
        
        // Convert to VS Code document symbols
        for (const symbol of serverSymbols) {
          const docSymbol = this.convertToDocumentSymbol(symbol);
          if (docSymbol) {
            symbols.push(docSymbol);
          }
        }
      } catch (error) {
        console.error(`Failed to get document symbols from ${serverId}:`, error);
      }
    }
    
    return symbols;
  }
  
  private convertToDocumentSymbol(symbol: any): vscode.DocumentSymbol | undefined {
    try {
      const range = new vscode.Range(
        symbol.range.start.line,
        symbol.range.start.character,
        symbol.range.end.line,
        symbol.range.end.character
      );
      
      const selectionRange = symbol.selectionRange
        ? new vscode.Range(
            symbol.selectionRange.start.line,
            symbol.selectionRange.start.character,
            symbol.selectionRange.end.line,
            symbol.selectionRange.end.character
          )
        : range;
      
      const docSymbol = new vscode.DocumentSymbol(
        symbol.name,
        symbol.detail || '',
        this.getSymbolKind(symbol.kind),
        range,
        selectionRange
      );
      
      // Add children if present
      if (symbol.children && Array.isArray(symbol.children)) {
        for (const child of symbol.children) {
          const childSymbol = this.convertToDocumentSymbol(child);
          if (childSymbol) {
            docSymbol.children.push(childSymbol);
          }
        }
      }
      
      return docSymbol;
    } catch (error) {
      console.error('Failed to convert symbol:', error);
      return undefined;
    }
  }
  
  private getSymbolKind(kind?: string): vscode.SymbolKind {
    switch (kind) {
      case 'file': return vscode.SymbolKind.File;
      case 'module': return vscode.SymbolKind.Module;
      case 'namespace': return vscode.SymbolKind.Namespace;
      case 'package': return vscode.SymbolKind.Package;
      case 'class': return vscode.SymbolKind.Class;
      case 'method': return vscode.SymbolKind.Method;
      case 'property': return vscode.SymbolKind.Property;
      case 'field': return vscode.SymbolKind.Field;
      case 'constructor': return vscode.SymbolKind.Constructor;
      case 'enum': return vscode.SymbolKind.Enum;
      case 'interface': return vscode.SymbolKind.Interface;
      case 'function': return vscode.SymbolKind.Function;
      case 'variable': return vscode.SymbolKind.Variable;
      case 'constant': return vscode.SymbolKind.Constant;
      case 'string': return vscode.SymbolKind.String;
      case 'number': return vscode.SymbolKind.Number;
      case 'boolean': return vscode.SymbolKind.Boolean;
      case 'array': return vscode.SymbolKind.Array;
      case 'object': return vscode.SymbolKind.Object;
      case 'key': return vscode.SymbolKind.Key;
      case 'null': return vscode.SymbolKind.Null;
      case 'enummember': return vscode.SymbolKind.EnumMember;
      case 'struct': return vscode.SymbolKind.Struct;
      case 'event': return vscode.SymbolKind.Event;
      case 'operator': return vscode.SymbolKind.Operator;
      case 'typeparameter': return vscode.SymbolKind.TypeParameter;
      default: return vscode.SymbolKind.Variable;
    }
  }
}

export class MCPWorkspaceSymbolProvider implements vscode.WorkspaceSymbolProvider {
  private serverManager: MCPServerManager;
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
  }
  
  async provideWorkspaceSymbols(
    query: string,
    token: vscode.CancellationToken
  ): Promise<vscode.SymbolInformation[]> {
    const symbols: vscode.SymbolInformation[] = [];
    const servers = this.serverManager.getServers();
    
    for (const [serverId, server] of servers) {
      const client = this.serverManager.getClient(serverId);
      if (!client || !client.isConnected()) continue;
      
      try {
        // Check if server supports workspace symbols
        const serverInfo = await client.getServerInfo();
        if (!serverInfo.capabilities?.workspaceSymbol) continue;
        
        // Request symbols from server
        const serverSymbols = await client.getWorkspaceSymbols({
          query
        });
        
        if (!serverSymbols || serverSymbols.length === 0) continue;
        
        // Convert to VS Code symbol information
        for (const symbol of serverSymbols) {
          const symInfo = this.convertToSymbolInformation(symbol);
          if (symInfo) {
            symbols.push(symInfo);
          }
        }
      } catch (error) {
        console.error(`Failed to get workspace symbols from ${serverId}:`, error);
      }
    }
    
    return symbols;
  }
  
  private convertToSymbolInformation(symbol: any): vscode.SymbolInformation | undefined {
    try {
      const location = new vscode.Location(
        vscode.Uri.parse(symbol.location.uri),
        new vscode.Range(
          symbol.location.range.start.line,
          symbol.location.range.start.character,
          symbol.location.range.end.line,
          symbol.location.range.end.character
        )
      );
      
      return new vscode.SymbolInformation(
        symbol.name,
        this.getSymbolKind(symbol.kind),
        symbol.containerName || '',
        location
      );
    } catch (error) {
      console.error('Failed to convert symbol information:', error);
      return undefined;
    }
  }
  
  private getSymbolKind(kind?: string): vscode.SymbolKind {
    // Same implementation as in DocumentSymbolProvider
    switch (kind) {
      case 'file': return vscode.SymbolKind.File;
      case 'module': return vscode.SymbolKind.Module;
      case 'namespace': return vscode.SymbolKind.Namespace;
      case 'package': return vscode.SymbolKind.Package;
      case 'class': return vscode.SymbolKind.Class;
      case 'method': return vscode.SymbolKind.Method;
      case 'property': return vscode.SymbolKind.Property;
      case 'field': return vscode.SymbolKind.Field;
      case 'constructor': return vscode.SymbolKind.Constructor;
      case 'enum': return vscode.SymbolKind.Enum;
      case 'interface': return vscode.SymbolKind.Interface;
      case 'function': return vscode.SymbolKind.Function;
      case 'variable': return vscode.SymbolKind.Variable;
      case 'constant': return vscode.SymbolKind.Constant;
      case 'string': return vscode.SymbolKind.String;
      case 'number': return vscode.SymbolKind.Number;
      case 'boolean': return vscode.SymbolKind.Boolean;
      case 'array': return vscode.SymbolKind.Array;
      case 'object': return vscode.SymbolKind.Object;
      case 'key': return vscode.SymbolKind.Key;
      case 'null': return vscode.SymbolKind.Null;
      case 'enummember': return vscode.SymbolKind.EnumMember;
      case 'struct': return vscode.SymbolKind.Struct;
      case 'event': return vscode.SymbolKind.Event;
      case 'operator': return vscode.SymbolKind.Operator;
      case 'typeparameter': return vscode.SymbolKind.TypeParameter;
      default: return vscode.SymbolKind.Variable;
    }
  }
}