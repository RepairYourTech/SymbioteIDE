/**
 * MCP Configuration Manager
 * 
 * Manages MCP extension configuration and settings
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

export interface MCPConfiguration {
  servers: MCPServerConfig[];
  autoStart: string[];
  defaultTimeout: number;
  enableSecurity: boolean;
  enableDiagnostics: boolean;
  enableTelemetry: boolean;
  logLevel: 'error' | 'warning' | 'info' | 'debug';
  experimental: {
    enableLanguageClient: boolean;
    enableDebugAdapter: boolean;
    enableCustomEditors: boolean;
  };
}

export interface MCPServerConfig {
  id: string;
  name: string;
  description?: string;
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  workingDirectory?: string;
  transport?: 'stdio' | 'http' | 'websocket';
  autoStart?: boolean;
  trustLevel?: string;
  capabilities?: string[];
  metadata?: Record<string, any>;
}

export class MCPConfigurationManager {
  private context: vscode.ExtensionContext;
  private configuration: MCPConfiguration;
  private configWatcher: vscode.FileSystemWatcher | null = null;
  private globalConfigPath: string;
  private workspaceConfigPath: string | null = null;
  
  constructor(context: vscode.ExtensionContext) {
    this.context = context;
    this.globalConfigPath = path.join(context.globalStorageUri.fsPath, 'mcp-config.json');
    
    // Initialize with defaults
    this.configuration = this.getDefaultConfiguration();
  }
  
  /**
   * Initialize configuration manager
   */
  async initialize(): Promise<void> {
    // Ensure storage directory exists
    await vscode.workspace.fs.createDirectory(this.context.globalStorageUri);
    
    // Load configuration
    await this.loadConfiguration();
    
    // Setup file watcher for workspace config
    this.setupConfigWatcher();
    
    // Migrate old configurations if needed
    await this.migrateOldConfigurations();
  }
  
  /**
   * Get current configuration
   */
  getConfiguration(): MCPConfiguration {
    return { ...this.configuration };
  }
  
  /**
   * Get servers
   */
  getServers(): MCPServerConfig[] {
    return [...this.configuration.servers];
  }
  
  /**
   * Get server by ID
   */
  getServer(serverId: string): MCPServerConfig | undefined {
    return this.configuration.servers.find(s => s.id === serverId);
  }
  
  /**
   * Add or update server
   */
  async addOrUpdateServer(server: MCPServerConfig): Promise<void> {
    const index = this.configuration.servers.findIndex(s => s.id === server.id);
    
    if (index >= 0) {
      this.configuration.servers[index] = server;
    } else {
      this.configuration.servers.push(server);
    }
    
    await this.saveConfiguration();
  }
  
  /**
   * Remove server
   */
  async removeServer(serverId: string): Promise<void> {
    this.configuration.servers = this.configuration.servers.filter(s => s.id !== serverId);
    
    // Remove from auto-start if present
    this.configuration.autoStart = this.configuration.autoStart.filter(id => id !== serverId);
    
    await this.saveConfiguration();
  }
  
  /**
   * Get auto-start servers
   */
  getAutoStartServers(): string[] {
    return [...this.configuration.autoStart];
  }
  
  /**
   * Set auto-start servers
   */
  async setAutoStartServers(serverIds: string[]): Promise<void> {
    this.configuration.autoStart = serverIds;
    await this.saveConfiguration();
  }
  
  /**
   * Get VS Code configuration value
   */
  getVSCodeConfig<T>(key: string, defaultValue?: T): T | undefined {
    return vscode.workspace.getConfiguration('symbiote.mcp').get<T>(key, defaultValue);
  }
  
  /**
   * Update VS Code configuration
   */
  async updateVSCodeConfig(key: string, value: any, global: boolean = false): Promise<void> {
    const target = global ? vscode.ConfigurationTarget.Global : vscode.ConfigurationTarget.Workspace;
    await vscode.workspace.getConfiguration('symbiote.mcp').update(key, value, target);
  }
  
  /**
   * Check if first activation
   */
  isFirstActivation(): boolean {
    return this.context.globalState.get('mcp.firstActivation', true);
  }
  
  /**
   * Mark as activated
   */
  async markAsActivated(): Promise<void> {
    await this.context.globalState.update('mcp.firstActivation', false);
  }
  
  /**
   * Get workspace MCP config path
   */
  getWorkspaceMCPConfigPath(): string | null {
    if (!vscode.workspace.workspaceFolders || vscode.workspace.workspaceFolders.length === 0) {
      return null;
    }
    
    return path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, '.mcp.json');
  }
  
  /**
   * Load workspace MCP config
   */
  async loadWorkspaceMCPConfig(): Promise<any | null> {
    const configPath = this.getWorkspaceMCPConfigPath();
    if (!configPath) return null;
    
    try {
      const content = await fs.promises.readFile(configPath, 'utf8');
      return JSON.parse(content);
    } catch {
      return null;
    }
  }
  
  /**
   * Save workspace MCP config
   */
  async saveWorkspaceMCPConfig(config: any): Promise<void> {
    const configPath = this.getWorkspaceMCPConfigPath();
    if (!configPath) return;
    
    await fs.promises.writeFile(configPath, JSON.stringify(config, null, 2));
  }
  
  /**
   * Import configuration
   */
  async importConfiguration(filePath: string): Promise<void> {
    try {
      const content = await fs.promises.readFile(filePath, 'utf8');
      const imported = JSON.parse(content);
      
      // Validate configuration
      if (!this.validateConfiguration(imported)) {
        throw new Error('Invalid configuration format');
      }
      
      // Merge with existing configuration
      this.configuration = {
        ...this.configuration,
        ...imported,
        servers: [...this.configuration.servers, ...(imported.servers || [])]
      };
      
      await this.saveConfiguration();
      
      vscode.window.showInformationMessage('Configuration imported successfully');
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to import configuration: ${error.message}`);
    }
  }
  
  /**
   * Export configuration
   */
  async exportConfiguration(filePath: string): Promise<void> {
    try {
      await fs.promises.writeFile(
        filePath,
        JSON.stringify(this.configuration, null, 2)
      );
      
      vscode.window.showInformationMessage('Configuration exported successfully');
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to export configuration: ${error.message}`);
    }
  }
  
  /**
   * Reset configuration
   */
  async resetConfiguration(): Promise<void> {
    const confirm = await vscode.window.showWarningMessage(
      'This will reset all MCP configuration to defaults. Continue?',
      'Reset',
      'Cancel'
    );
    
    if (confirm !== 'Reset') return;
    
    this.configuration = this.getDefaultConfiguration();
    await this.saveConfiguration();
    
    vscode.window.showInformationMessage('Configuration reset to defaults');
  }
  
  /**
   * Reload configuration
   */
  async reload(): Promise<void> {
    await this.loadConfiguration();
  }
  
  /**
   * Get server validation errors
   */
  validateServer(server: MCPServerConfig): string[] {
    const errors: string[] = [];
    
    if (!server.id) {
      errors.push('Server ID is required');
    }
    
    if (!server.name) {
      errors.push('Server name is required');
    }
    
    if (!server.command && !server.url) {
      errors.push('Either command or URL is required');
    }
    
    if (server.command && !Array.isArray(server.args)) {
      errors.push('Args must be an array when using command');
    }
    
    if (server.transport && !['stdio', 'http', 'websocket'].includes(server.transport)) {
      errors.push('Invalid transport type');
    }
    
    return errors;
  }
  
  /**
   * Load configuration
   */
  private async loadConfiguration(): Promise<void> {
    // Load VS Code settings
    const vscodeConfig = vscode.workspace.getConfiguration('symbiote.mcp');
    
    // Load global configuration file
    let fileConfig: Partial<MCPConfiguration> = {};
    try {
      const content = await fs.promises.readFile(this.globalConfigPath, 'utf8');
      fileConfig = JSON.parse(content);
    } catch {
      // File doesn't exist or is invalid
    }
    
    // Load workspace configuration
    const workspaceConfig = await this.loadWorkspaceMCPConfig();
    
    // Merge configurations (VS Code settings > workspace config > global config > defaults)
    this.configuration = {
      ...this.getDefaultConfiguration(),
      ...fileConfig,
      servers: [
        ...(fileConfig.servers || []),
        ...(workspaceConfig?.servers || [])
      ],
      autoStart: vscodeConfig.get('autoStart', fileConfig.autoStart || []),
      defaultTimeout: vscodeConfig.get('defaultTimeout', fileConfig.defaultTimeout || 30000),
      enableSecurity: vscodeConfig.get('enableSecurity', fileConfig.enableSecurity ?? true),
      enableDiagnostics: vscodeConfig.get('enableDiagnostics', fileConfig.enableDiagnostics ?? true),
      enableTelemetry: vscodeConfig.get('enableTelemetry', fileConfig.enableTelemetry ?? false),
      logLevel: vscodeConfig.get('logLevel', fileConfig.logLevel || 'info'),
      experimental: {
        enableLanguageClient: vscodeConfig.get('experimental.enableLanguageClient', false),
        enableDebugAdapter: vscodeConfig.get('experimental.enableDebugAdapter', false),
        enableCustomEditors: vscodeConfig.get('experimental.enableCustomEditors', false)
      }
    };
    
    // Remove duplicate servers
    const seen = new Set<string>();
    this.configuration.servers = this.configuration.servers.filter(server => {
      if (seen.has(server.id)) return false;
      seen.add(server.id);
      return true;
    });
  }
  
  /**
   * Save configuration
   */
  private async saveConfiguration(): Promise<void> {
    // Save to global configuration file
    await fs.promises.writeFile(
      this.globalConfigPath,
      JSON.stringify(this.configuration, null, 2)
    );
    
    // Trigger configuration change event
    vscode.workspace.getConfiguration('symbiote.mcp').update(
      'configurationVersion',
      Date.now(),
      vscode.ConfigurationTarget.Global
    );
  }
  
  /**
   * Get default configuration
   */
  private getDefaultConfiguration(): MCPConfiguration {
    return {
      servers: [],
      autoStart: [],
      defaultTimeout: 30000,
      enableSecurity: true,
      enableDiagnostics: true,
      enableTelemetry: false,
      logLevel: 'info',
      experimental: {
        enableLanguageClient: false,
        enableDebugAdapter: false,
        enableCustomEditors: false
      }
    };
  }
  
  /**
   * Validate configuration
   */
  private validateConfiguration(config: any): boolean {
    if (typeof config !== 'object') return false;
    
    if (config.servers && !Array.isArray(config.servers)) return false;
    
    if (config.autoStart && !Array.isArray(config.autoStart)) return false;
    
    if (config.logLevel && !['error', 'warning', 'info', 'debug'].includes(config.logLevel)) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Setup configuration file watcher
   */
  private setupConfigWatcher(): void {
    if (!vscode.workspace.workspaceFolders) return;
    
    this.configWatcher = vscode.workspace.createFileSystemWatcher('**/.mcp.json');
    
    this.configWatcher.onDidChange(async () => {
      await this.loadConfiguration();
    });
    
    this.configWatcher.onDidCreate(async () => {
      await this.loadConfiguration();
    });
    
    this.configWatcher.onDidDelete(async () => {
      await this.loadConfiguration();
    });
  }
  
  /**
   * Migrate old configurations
   */
  private async migrateOldConfigurations(): Promise<void> {
    // Check for old configuration format
    const oldConfigPath = path.join(this.context.globalStorageUri.fsPath, 'servers.json');
    
    try {
      const oldContent = await fs.promises.readFile(oldConfigPath, 'utf8');
      const oldServers = JSON.parse(oldContent);
      
      // Migrate to new format
      if (Array.isArray(oldServers)) {
        this.configuration.servers = oldServers.map((server: any) => ({
          id: server.id || server.name.toLowerCase().replace(/\s+/g, '-'),
          name: server.name,
          description: server.description,
          command: server.command,
          args: server.args,
          env: server.env,
          transport: server.transport || 'stdio',
          autoStart: server.autoStart || false
        }));
        
        await this.saveConfiguration();
        
        // Remove old config file
        await fs.promises.unlink(oldConfigPath);
        
        vscode.window.showInformationMessage('MCP configuration migrated to new format');
      }
    } catch {
      // Old config doesn't exist or migration failed
    }
  }
  
  /**
   * Dispose resources
   */
  dispose(): void {
    if (this.configWatcher) {
      this.configWatcher.dispose();
    }
  }
}