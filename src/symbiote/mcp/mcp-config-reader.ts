/**
 * MCP Configuration Reader
 * 
 * Reads and parses .vscode/mcp.json files to discover MCP server definitions
 */

import * as fs from 'fs';
import * as path from 'path';
import { workspace, Uri, ConfigurationTarget } from 'vscode';

export interface McpServerDefinition {
  /**
   * Command to execute to start the MCP server
   */
  command: string;
  
  /**
   * Arguments to pass to the command
   */
  args?: string[];
  
  /**
   * Environment variables for the server process
   */
  env?: Record<string, string>;
  
  /**
   * Working directory for the server process
   */
  cwd?: string;
  
  /**
   * Whether to auto-start this server
   */
  autoStart?: boolean;
  
  /**
   * Timeout for server startup in milliseconds
   */
  startupTimeout?: number;
  
  /**
   * Maximum number of restart attempts
   */
  maxRestarts?: number;
}

export interface McpConfiguration {
  /**
   * Map of server ID to server definition
   */
  mcpServers: Record<string, McpServerDefinition>;
}

export class McpConfigReader {
  private static readonly CONFIG_FILENAME = 'mcp.json';
  private static readonly VSCODE_DIR = '.vscode';
  
  /**
   * Read MCP configuration from workspace
   */
  async readConfiguration(workspaceUri?: Uri): Promise<McpConfiguration | null> {
    try {
      // Try multiple locations in order of precedence
      const configs = await Promise.all([
        this.readWorkspaceConfig(workspaceUri),
        this.readUserConfig(),
        this.readProjectConfig(workspaceUri)
      ]);
      
      // Merge configurations (workspace > user > project)
      return this.mergeConfigurations(...configs.filter(Boolean) as McpConfiguration[]);
    } catch (error) {
      console.error('Failed to read MCP configuration:', error);
      return null;
    }
  }
  
  /**
   * Read configuration from workspace settings
   */
  private async readWorkspaceConfig(workspaceUri?: Uri): Promise<McpConfiguration | null> {
    try {
      const config = workspace.getConfiguration('symbioteIDE.mcp', workspaceUri);
      const servers = config.get<Record<string, McpServerDefinition>>('servers');
      
      if (servers && Object.keys(servers).length > 0) {
        return { mcpServers: servers };
      }
      
      return null;
    } catch (error) {
      console.error('Failed to read workspace MCP config:', error);
      return null;
    }
  }
  
  /**
   * Read configuration from user settings
   */
  private async readUserConfig(): Promise<McpConfiguration | null> {
    try {
      const config = workspace.getConfiguration('symbioteIDE.mcp');
      const servers = config.inspect<Record<string, McpServerDefinition>>('servers');
      
      if (servers?.globalValue && Object.keys(servers.globalValue).length > 0) {
        return { mcpServers: servers.globalValue };
      }
      
      return null;
    } catch (error) {
      console.error('Failed to read user MCP config:', error);
      return null;
    }
  }
  
  /**
   * Read configuration from .vscode/mcp.json
   */
  private async readProjectConfig(workspaceUri?: Uri): Promise<McpConfiguration | null> {
    try {
      // Get workspace folders
      const folders = workspace.workspaceFolders;
      if (!folders || folders.length === 0) {
        return null;
      }
      
      // Use provided workspace URI or first workspace folder
      const folder = workspaceUri ? 
        folders.find(f => f.uri.toString() === workspaceUri.toString()) || folders[0] :
        folders[0];
      
      const configPath = path.join(folder.uri.fsPath, McpConfigReader.VSCODE_DIR, McpConfigReader.CONFIG_FILENAME);
      
      if (!fs.existsSync(configPath)) {
        return null;
      }
      
      const content = await fs.promises.readFile(configPath, 'utf-8');
      const config = JSON.parse(content) as McpConfiguration;
      
      // Validate configuration
      if (!config.mcpServers || typeof config.mcpServers !== 'object') {
        console.warn('Invalid MCP configuration: missing or invalid mcpServers');
        return null;
      }
      
      return config;
    } catch (error) {
      console.error('Failed to read project MCP config:', error);
      return null;
    }
  }
  
  /**
   * Merge multiple configurations
   */
  private mergeConfigurations(...configs: McpConfiguration[]): McpConfiguration {
    const merged: McpConfiguration = { mcpServers: {} };
    
    // Merge in reverse order so first config takes precedence
    for (const config of configs.reverse()) {
      Object.assign(merged.mcpServers, config.mcpServers);
    }
    
    return merged;
  }
  
  /**
   * Save configuration to workspace
   */
  async saveConfiguration(
    config: McpConfiguration,
    target: ConfigurationTarget = ConfigurationTarget.Workspace,
    workspaceUri?: Uri
  ): Promise<void> {
    try {
      const vsConfig = workspace.getConfiguration('symbioteIDE.mcp', workspaceUri);
      await vsConfig.update('servers', config.mcpServers, target);
    } catch (error) {
      console.error('Failed to save MCP configuration:', error);
      throw error;
    }
  }
  
  /**
   * Watch for configuration changes
   */
  watchConfiguration(callback: (config: McpConfiguration | null) => void): { dispose: () => void } {
    // Watch workspace configuration changes
    const configWatcher = workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration('symbioteIDE.mcp')) {
        const config = await this.readConfiguration();
        callback(config);
      }
    });
    
    // Watch file system changes for .vscode/mcp.json
    const fileWatcher = workspace.createFileSystemWatcher('**/.vscode/mcp.json');
    
    const handleFileChange = async () => {
      const config = await this.readConfiguration();
      callback(config);
    };
    
    fileWatcher.onDidCreate(handleFileChange);
    fileWatcher.onDidChange(handleFileChange);
    fileWatcher.onDidDelete(handleFileChange);
    
    return {
      dispose: () => {
        configWatcher.dispose();
        fileWatcher.dispose();
      }
    };
  }
  
  /**
   * Validate server definition
   */
  validateServerDefinition(serverId: string, definition: McpServerDefinition): string[] {
    const errors: string[] = [];
    
    if (!definition.command) {
      errors.push(`Server '${serverId}': missing required field 'command'`);
    }
    
    if (definition.args && !Array.isArray(definition.args)) {
      errors.push(`Server '${serverId}': 'args' must be an array`);
    }
    
    if (definition.env && typeof definition.env !== 'object') {
      errors.push(`Server '${serverId}': 'env' must be an object`);
    }
    
    if (definition.startupTimeout !== undefined && 
        (typeof definition.startupTimeout !== 'number' || definition.startupTimeout <= 0)) {
      errors.push(`Server '${serverId}': 'startupTimeout' must be a positive number`);
    }
    
    if (definition.maxRestarts !== undefined && 
        (typeof definition.maxRestarts !== 'number' || definition.maxRestarts < 0)) {
      errors.push(`Server '${serverId}': 'maxRestarts' must be a non-negative number`);
    }
    
    return errors;
  }
  
  /**
   * Validate entire configuration
   */
  validateConfiguration(config: McpConfiguration): { valid: boolean; errors: string[] } {
    const errors: string[] = [];
    
    if (!config.mcpServers || typeof config.mcpServers !== 'object') {
      errors.push('Configuration must have an mcpServers object');
      return { valid: false, errors };
    }
    
    for (const [serverId, definition] of Object.entries(config.mcpServers)) {
      errors.push(...this.validateServerDefinition(serverId, definition));
    }
    
    return {
      valid: errors.length === 0,
      errors
    };
  }
  
  /**
   * Get effective environment variables for a server
   */
  getEffectiveEnvironment(definition: McpServerDefinition): Record<string, string> {
    // Start with process environment
    const env = { ...process.env };
    
    // Override with server-specific environment
    if (definition.env) {
      Object.assign(env, definition.env);
    }
    
    // Ensure PATH is included
    if (!env.PATH) {
      env.PATH = process.env.PATH || '';
    }
    
    return env;
  }
  
  /**
   * Resolve command path
   */
  async resolveCommand(command: string, cwd?: string): Promise<string | null> {
    // If it's an absolute path, check if it exists
    if (path.isAbsolute(command)) {
      return fs.existsSync(command) ? command : null;
    }
    
    // Try to resolve relative to cwd
    if (cwd) {
      const cwdPath = path.join(cwd, command);
      if (fs.existsSync(cwdPath)) {
        return cwdPath;
      }
    }
    
    // Try to resolve relative to workspace folders
    const folders = workspace.workspaceFolders;
    if (folders) {
      for (const folder of folders) {
        const folderPath = path.join(folder.uri.fsPath, command);
        if (fs.existsSync(folderPath)) {
          return folderPath;
        }
      }
    }
    
    // Return as-is and let the system PATH resolve it
    return command;
  }
}