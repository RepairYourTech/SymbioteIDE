/**
 * MCP Extension Entry Point
 * 
 * Main entry point for VS Code extension activation
 */

import * as vscode from 'vscode';
import { MCPExtension } from './mcp-extension';

let mcpExtension: MCPExtension | undefined;

/**
 * Activate the MCP extension
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
  console.log('Activating MCP Extension...');
  
  try {
    // Create and activate the extension
    mcpExtension = new MCPExtension(context);
    await mcpExtension.activate();
    
    // Register extension API
    const api = {
      getServerManager: () => mcpExtension?.getServerManager(),
      getSecurityExtension: () => mcpExtension?.getSecurityExtension(),
      getConfigurationManager: () => mcpExtension?.getConfigurationManager(),
      executeTool: async (serverId: string, toolName: string, args: any) => {
        return mcpExtension?.executeTool(serverId, toolName, args);
      },
      readResource: async (serverId: string, uri: string) => {
        return mcpExtension?.readResource(serverId, uri);
      }
    };
    
    // Return API for other extensions
    return api as any;
    
  } catch (error) {
    console.error('Failed to activate MCP Extension:', error);
    vscode.window.showErrorMessage(`Failed to activate MCP Extension: ${error.message}`);
    throw error;
  }
}

/**
 * Deactivate the MCP extension
 */
export async function deactivate(): Promise<void> {
  console.log('Deactivating MCP Extension...');
  
  if (mcpExtension) {
    try {
      await mcpExtension.deactivate();
    } catch (error) {
      console.error('Error during MCP deactivation:', error);
    }
    
    mcpExtension = undefined;
  }
}