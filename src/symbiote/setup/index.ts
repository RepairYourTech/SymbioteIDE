/**
 * Setup Module - Entry point for Docker infrastructure setup
 */

import * as vscode from 'vscode';
import { setupWizard } from './setup-wizard';
import { dockerManager } from '../infrastructure/docker-manager';
import { platformDetector } from './platform-detector';
import { Logger } from '../utils/logger';

export class SetupManager {
  private logger = new Logger('SetupManager');
  private statusBarItem: vscode.StatusBarItem;
  
  constructor() {
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );
  }
  
  /**
   * Initialize setup manager
   */
  async initialize(context: vscode.ExtensionContext): Promise<void> {
    // Register commands
    context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.setup', () => this.runSetup()),
      vscode.commands.registerCommand('symbiote.setup.quick', () => this.runQuickSetup()),
      vscode.commands.registerCommand('symbiote.docker.status', () => this.showDockerStatus()),
      vscode.commands.registerCommand('symbiote.docker.restart', () => this.restartServices()),
      vscode.commands.registerCommand('symbiote.docker.logs', () => this.showLogs())
    );
    
    // Check if first run
    const hasRunSetup = context.globalState.get('symbiote.setup.completed', false);
    
    if (!hasRunSetup) {
      // Show welcome message
      const runSetup = await vscode.window.showInformationMessage(
        'Welcome to SymbioteIDE! Let\'s set up the required Docker services.',
        'Run Setup',
        'Later'
      );
      
      if (runSetup === 'Run Setup') {
        await this.runSetup();
        context.globalState.update('symbiote.setup.completed', true);
      }
    } else {
      // Check service status in background
      this.checkServicesBackground();
    }
    
    // Setup status bar
    this.setupStatusBar();
  }
  
  /**
   * Run interactive setup
   */
  private async runSetup(): Promise<void> {
    try {
      await setupWizard.runSetup();
    } catch (error) {
      this.logger.error('Setup failed', error);
      vscode.window.showErrorMessage(
        `Setup failed: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }
  
  /**
   * Run quick setup
   */
  private async runQuickSetup(): Promise<void> {
    try {
      await setupWizard.runQuickSetup();
    } catch (error) {
      this.logger.error('Quick setup failed', error);
    }
  }
  
  /**
   * Check services in background
   */
  private async checkServicesBackground(): Promise<void> {
    try {
      await dockerManager.initialize();
      const services = await dockerManager.checkServicesHealth();
      
      let healthyCount = 0;
      let totalCount = 0;
      
      for (const [_, service] of services) {
        totalCount++;
        if (service.status === 'healthy' || service.status === 'running') {
          healthyCount++;
        }
      }
      
      if (totalCount === 0) {
        this.updateStatusBar('Docker Not Running', '$(error)', 'error');
      } else if (healthyCount === totalCount) {
        this.updateStatusBar('All Services Healthy', '$(check)', 'ready');
      } else {
        this.updateStatusBar(
          `${healthyCount}/${totalCount} Services Running`, 
          '$(warning)', 
          'warning'
        );
      }
    } catch (error) {
      this.updateStatusBar('Docker Error', '$(error)', 'error');
    }
  }
  
  /**
   * Setup status bar
   */
  private setupStatusBar(): void {
    this.statusBarItem.command = 'symbiote.docker.status';
    this.statusBarItem.show();
    
    // Update status periodically
    setInterval(() => {
      this.checkServicesBackground();
    }, 30000); // Every 30 seconds
  }
  
  /**
   * Update status bar
   */
  private updateStatusBar(text: string, icon: string, state: 'ready' | 'warning' | 'error'): void {
    this.statusBarItem.text = `${icon} Symbiote: ${text}`;
    
    switch (state) {
      case 'ready':
        this.statusBarItem.backgroundColor = undefined;
        break;
      case 'warning':
        this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        break;
      case 'error':
        this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        break;
    }
  }
  
  /**
   * Show Docker status
   */
  private async showDockerStatus(): Promise<void> {
    const services = await dockerManager.checkServicesHealth();
    
    const items: vscode.QuickPickItem[] = [];
    
    for (const [name, service] of services) {
      const icon = service.status === 'healthy' ? '$(check)' :
                   service.status === 'running' ? '$(sync~spin)' :
                   service.status === 'starting' ? '$(loading~spin)' : '$(error)';
      
      items.push({
        label: `${icon} ${name}`,
        description: service.status,
        detail: service.ports?.join(', ')
      });
    }
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: 'Docker Services Status',
      canPickMany: false
    });
    
    if (selected) {
      const serviceName = selected.label.split(' ')[1];
      const actions = ['View Logs', 'Restart', 'Cancel'];
      
      const action = await vscode.window.showQuickPick(actions, {
        placeHolder: `Actions for ${serviceName}`
      });
      
      if (action === 'View Logs') {
        await this.showServiceLogs(serviceName);
      } else if (action === 'Restart') {
        await dockerManager.restartService(serviceName);
      }
    }
  }
  
  /**
   * Restart all services
   */
  private async restartServices(): Promise<void> {
    const confirm = await vscode.window.showWarningMessage(
      'Restart all Docker services?',
      'Restart',
      'Cancel'
    );
    
    if (confirm === 'Restart') {
      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: 'Restarting services...'
      }, async () => {
        await dockerManager.stopServices();
        await dockerManager.startServices({ detached: true });
      });
    }
  }
  
  /**
   * Show logs
   */
  private async showLogs(): Promise<void> {
    const services = await dockerManager.checkServicesHealth();
    const serviceNames = Array.from(services.keys());
    
    const selected = await vscode.window.showQuickPick(serviceNames, {
      placeHolder: 'Select service to view logs'
    });
    
    if (selected) {
      await this.showServiceLogs(selected);
    }
  }
  
  /**
   * Show service logs
   */
  private async showServiceLogs(serviceName: string): Promise<void> {
    const logs = await dockerManager.getServiceLogs(serviceName, 200);
    
    const outputChannel = vscode.window.createOutputChannel(`Docker: ${serviceName}`);
    outputChannel.append(logs);
    outputChannel.show();
  }
}

// Export singleton
export const setupManager = new SetupManager();