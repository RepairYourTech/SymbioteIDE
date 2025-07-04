/**
 * MCP Security Settings
 * 
 * Configuration settings for MCP security
 */

import * as vscode from 'vscode';
import { SecurityPolicy, TrustLevel } from '../types';

export interface SecuritySettings {
  // General settings
  enableSecurity: boolean;
  defaultPolicy: SecurityPolicy;
  enableAuditing: boolean;
  enableScanning: boolean;
  
  // Trust settings
  defaultTrustLevel: TrustLevel;
  requireUserApproval: boolean;
  rememberPermissions: boolean;
  
  // Sandbox settings
  enableSandbox: boolean;
  sandboxFilesystemAccess: 'none' | 'readonly' | 'restricted' | 'full';
  sandboxNetworkAccess: 'none' | 'localhost' | 'restricted' | 'full';
  sandboxProcessLimits: {
    maxCpu: number;      // Percentage
    maxMemory: number;   // MB
    maxDiskIO: number;   // MB/s
  };
  
  // Alert settings
  showSecurityAlerts: boolean;
  alertOnViolations: boolean;
  alertOnHighResourceUsage: boolean;
  
  // Advanced settings
  scanDepth: 'basic' | 'moderate' | 'deep';
  auditRetentionDays: number;
  maxConcurrentServers: number;
}

export class SecuritySettingsManager {
  private static readonly SECTION = 'symbiote.mcp.security';
  
  /**
   * Get all security settings
   */
  static getSettings(): SecuritySettings {
    const config = vscode.workspace.getConfiguration(this.SECTION);
    
    return {
      // General settings
      enableSecurity: config.get<boolean>('enabled', true),
      defaultPolicy: config.get<SecurityPolicy>('defaultPolicy', SecurityPolicy.Moderate),
      enableAuditing: config.get<boolean>('enableAuditing', true),
      enableScanning: config.get<boolean>('enableScanning', true),
      
      // Trust settings
      defaultTrustLevel: config.get<TrustLevel>('defaultTrustLevel', TrustLevel.Unknown),
      requireUserApproval: config.get<boolean>('requireUserApproval', true),
      rememberPermissions: config.get<boolean>('rememberPermissions', true),
      
      // Sandbox settings
      enableSandbox: config.get<boolean>('sandbox.enabled', true),
      sandboxFilesystemAccess: config.get<'none' | 'readonly' | 'restricted' | 'full'>('sandbox.filesystemAccess', 'restricted'),
      sandboxNetworkAccess: config.get<'none' | 'localhost' | 'restricted' | 'full'>('sandbox.networkAccess', 'localhost'),
      sandboxProcessLimits: {
        maxCpu: config.get<number>('sandbox.limits.maxCpu', 50),
        maxMemory: config.get<number>('sandbox.limits.maxMemory', 512),
        maxDiskIO: config.get<number>('sandbox.limits.maxDiskIO', 10)
      },
      
      // Alert settings
      showSecurityAlerts: config.get<boolean>('alerts.show', true),
      alertOnViolations: config.get<boolean>('alerts.violations', true),
      alertOnHighResourceUsage: config.get<boolean>('alerts.highResourceUsage', true),
      
      // Advanced settings
      scanDepth: config.get<'basic' | 'moderate' | 'deep'>('advanced.scanDepth', 'moderate'),
      auditRetentionDays: config.get<number>('advanced.auditRetentionDays', 30),
      maxConcurrentServers: config.get<number>('advanced.maxConcurrentServers', 10)
    };
  }
  
  /**
   * Update a setting
   */
  static async updateSetting<T>(key: string, value: T, global: boolean = false): Promise<void> {
    const config = vscode.workspace.getConfiguration(this.SECTION);
    await config.update(key, value, global ? vscode.ConfigurationTarget.Global : vscode.ConfigurationTarget.Workspace);
  }
  
  /**
   * Register configuration schema
   */
  static registerConfiguration(context: vscode.ExtensionContext): void {
    // VS Code will automatically use package.json configuration contribution
    // This method can be used for dynamic configuration if needed
  }
  
  /**
   * Show security settings
   */
  static showSettings(): void {
    vscode.commands.executeCommand('workbench.action.openSettings', '@ext:symbiote.mcp security');
  }
}