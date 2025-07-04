/**
 * Setup Wizard - Interactive setup for Docker services
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';
import { platformDetector, SystemInfo } from './platform-detector';
import { dockerManager } from '../infrastructure/docker-manager';
import { Logger } from '../utils/logger';

export interface SetupOptions {
  includeOllama: boolean;
  localLLMType?: 'ollama' | 'lm-studio' | 'llama-cpp' | 'custom' | 'none';
  localLLMEndpoint?: string;
  apiKeys?: {
    openai?: string;
    anthropic?: string;
    google?: string;
    perplexity?: string;
  };
}

export class SetupWizard {
  private logger = new Logger('SetupWizard');
  private outputChannel: vscode.OutputChannel;
  
  constructor() {
    this.outputChannel = vscode.window.createOutputChannel('SymbioteIDE Setup');
  }
  
  /**
   * Run the setup wizard
   */
  async runSetup(): Promise<void> {
    try {
      this.outputChannel.show();
      this.log('Starting SymbioteIDE Setup...');
      
      // Step 1: Check system requirements
      const systemInfo = await this.checkSystemRequirements();
      if (!systemInfo) {
        return;
      }
      
      // Step 2: Check Docker installation
      const dockerReady = await this.checkDocker(systemInfo);
      if (!dockerReady) {
        return;
      }
      
      // Step 3: Configure options
      const options = await this.configureOptions(systemInfo);
      if (!options) {
        return;
      }
      
      // Step 4: Setup API keys
      await this.setupAPIKeys(options);
      
      // Step 5: Deploy services
      await this.deployServices(options);
      
      // Step 6: Verify deployment
      await this.verifyDeployment();
      
      // Step 7: Show success
      await this.showSuccess();
      
    } catch (error) {
      this.logger.error('Setup failed', error);
      vscode.window.showErrorMessage(
        `Setup failed: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }
  
  /**
   * Check system requirements
   */
  private async checkSystemRequirements(): Promise<SystemInfo | null> {
    this.log('Checking system requirements...');
    
    const systemInfo = await platformDetector.detectSystem();
    const requirements = platformDetector.checkRequirements(systemInfo);
    
    // Show system info
    this.log(`Platform: ${systemInfo.platform} ${systemInfo.platformVersion}`);
    this.log(`Architecture: ${systemInfo.architecture}`);
    this.log(`CPU: ${systemInfo.cpuCount} cores`);
    this.log(`Memory: ${systemInfo.totalMemoryGB}GB total, ${systemInfo.freeMemoryGB}GB free`);
    this.log(`Disk: ${systemInfo.availableDiskGB}GB available`);
    
    if (systemInfo.hasGPU) {
      this.log(`GPU: ${systemInfo.gpuInfo}`);
    }
    
    // Show errors
    if (requirements.errors.length > 0) {
      this.log('\n❌ System Requirements Not Met:');
      for (const error of requirements.errors) {
        this.log(`  - ${error}`);
      }
      
      const proceed = await vscode.window.showErrorMessage(
        'System requirements not met. Some features may not work properly.',
        'Continue Anyway',
        'Cancel'
      );
      
      if (proceed !== 'Continue Anyway') {
        return null;
      }
    }
    
    // Show warnings
    if (requirements.warnings.length > 0) {
      this.log('\n⚠️ Warnings:');
      for (const warning of requirements.warnings) {
        this.log(`  - ${warning}`);
      }
    }
    
    return systemInfo;
  }
  
  /**
   * Check Docker installation
   */
  private async checkDocker(systemInfo: SystemInfo): Promise<boolean> {
    this.log('\nChecking Docker installation...');
    
    if (systemInfo.dockerType === 'none') {
      this.log('❌ Docker not found!');
      
      const action = await vscode.window.showErrorMessage(
        'Docker is not installed. Please install Docker Desktop or Docker CE.',
        'Download Docker',
        'I\'ll Install Manually',
        'Cancel'
      );
      
      if (action === 'Download Docker') {
        const url = systemInfo.platform === 'windows' ?
          'https://desktop.docker.com/win/stable/Docker%20Desktop%20Installer.exe' :
          systemInfo.platform === 'macos' ?
            'https://desktop.docker.com/mac/stable/Docker.dmg' :
            'https://docs.docker.com/engine/install/';
        
        vscode.env.openExternal(vscode.Uri.parse(url));
        
        await vscode.window.showInformationMessage(
          'Please install Docker and restart VS Code when done.',
          'OK'
        );
      }
      
      return false;
    }
    
    this.log(`✅ Docker ${systemInfo.dockerType} v${systemInfo.dockerVersion} found`);
    
    if (systemInfo.dockerComposeVersion) {
      this.log(`✅ Docker Compose v${systemInfo.dockerComposeVersion} found`);
    } else {
      this.log('❌ Docker Compose not found');
      
      const proceed = await vscode.window.showWarningMessage(
        'Docker Compose is required. It should be included with Docker Desktop.',
        'Continue',
        'Cancel'
      );
      
      return proceed === 'Continue';
    }
    
    // Initialize Docker manager
    await dockerManager.initialize();
    
    return true;
  }
  
  /**
   * Configure setup options
   */
  private async configureOptions(systemInfo: SystemInfo): Promise<SetupOptions | null> {
    const options: SetupOptions = {
      includeOllama: false
    };
    
    // Check for existing local LLM
    if (systemInfo.localLLM && systemInfo.localLLM.type !== 'none') {
      this.log(`\n✅ Found ${systemInfo.localLLM.type} at ${systemInfo.localLLM.endpoint}`);
      
      const useExisting = await vscode.window.showInformationMessage(
        `Found ${systemInfo.localLLM.type} running. Use this for local AI features?`,
        'Yes',
        'No, Use Ollama',
        'No Local AI'
      );
      
      if (useExisting === 'Yes') {
        options.localLLMType = systemInfo.localLLM.type;
        options.localLLMEndpoint = systemInfo.localLLM.endpoint;
      } else if (useExisting === 'No, Use Ollama') {
        options.includeOllama = true;
        options.localLLMType = 'ollama';
      } else {
        options.localLLMType = 'none';
      }
    } else {
      // No local LLM found
      const choice = await vscode.window.showInformationMessage(
        'No local LLM found. Would you like to install Ollama for local AI features?',
        'Install Ollama',
        'Use My Own',
        'Skip Local AI'
      );
      
      if (choice === 'Install Ollama') {
        options.includeOllama = true;
        options.localLLMType = 'ollama';
      } else if (choice === 'Use My Own') {
        // Get custom endpoint
        const endpoint = await vscode.window.showInputBox({
          prompt: 'Enter your local LLM endpoint',
          value: 'http://localhost:8080',
          validateInput: (value) => {
            try {
              new URL(value);
              return null;
            } catch {
              return 'Invalid URL';
            }
          }
        });
        
        if (endpoint) {
          options.localLLMType = 'custom';
          options.localLLMEndpoint = endpoint;
        } else {
          return null;
        }
      } else {
        options.localLLMType = 'none';
      }
    }
    
    return options;
  }
  
  /**
   * Setup API keys
   */
  private async setupAPIKeys(options: SetupOptions): Promise<void> {
    this.log('\nConfiguring API keys...');
    
    const setupKeys = await vscode.window.showInformationMessage(
      'Would you like to configure API keys for cloud AI providers?',
      'Configure Now',
      'Skip'
    );
    
    if (setupKeys !== 'Configure Now') {
      return;
    }
    
    options.apiKeys = {};
    
    // OpenAI
    const openaiKey = await vscode.window.showInputBox({
      prompt: 'Enter OpenAI API key (optional)',
      password: true,
      placeHolder: 'sk-...'
    });
    if (openaiKey) {
      options.apiKeys.openai = openaiKey;
    }
    
    // Anthropic
    const anthropicKey = await vscode.window.showInputBox({
      prompt: 'Enter Anthropic API key (optional)',
      password: true,
      placeHolder: 'sk-ant-...'
    });
    if (anthropicKey) {
      options.apiKeys.anthropic = anthropicKey;
    }
    
    // Save API keys securely
    if (options.apiKeys) {
      await this.saveAPIKeys(options.apiKeys);
    }
  }
  
  /**
   * Deploy Docker services
   */
  private async deployServices(options: SetupOptions): Promise<void> {
    this.log('\n🚀 Deploying Docker services...');
    this.log('This may take several minutes on first run.\n');
    
    // Subscribe to progress events
    dockerManager.on('progress', (progress) => {
      this.log(`[${progress.service}] ${progress.status} ${progress.message || ''}`);
    });
    
    try {
      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: 'Deploying SymbioteIDE Services',
        cancellable: false
      }, async (progress) => {
        progress.report({ increment: 0, message: 'Starting services...' });
        
        // Track progress
        let servicesStarted = 0;
        const totalServices = 12 + (options.includeOllama ? 1 : 0);
        
        dockerManager.on('progress', (event) => {
          if (event.status === 'running' || event.status === 'healthy') {
            servicesStarted++;
            const percent = Math.round((servicesStarted / totalServices) * 100);
            progress.report({
              increment: percent / totalServices,
              message: `${event.service} ${event.status}`
            });
          }
        });
        
        // Start services
        await dockerManager.startServices({
          includeOllama: options.includeOllama,
          detached: true
        });
        
        progress.report({ increment: 100, message: 'All services started!' });
      });
      
      this.log('\n✅ All services deployed successfully!');
      
    } catch (error) {
      this.log(`\n❌ Deployment failed: ${error}`);
      throw error;
    }
  }
  
  /**
   * Verify deployment
   */
  private async verifyDeployment(): Promise<void> {
    this.log('\n🔍 Verifying deployment...');
    
    const services = await dockerManager.checkServicesHealth();
    
    let allHealthy = true;
    
    for (const [name, service] of services) {
      const status = service.status === 'healthy' ? '✅' :
                     service.status === 'running' ? '🟡' : '❌';
      
      this.log(`${status} ${name}: ${service.status}`);
      
      if (service.ports && service.ports.length > 0) {
        this.log(`   Ports: ${service.ports.join(', ')}`);
      }
      
      if (service.status !== 'healthy' && service.status !== 'running') {
        allHealthy = false;
      }
    }
    
    if (!allHealthy) {
      const retry = await vscode.window.showWarningMessage(
        'Some services are not healthy. This may be normal during startup.',
        'Check Again',
        'View Logs',
        'Continue'
      );
      
      if (retry === 'Check Again') {
        await this.verifyDeployment();
      } else if (retry === 'View Logs') {
        await this.showServiceLogs();
      }
    }
  }
  
  /**
   * Show service logs
   */
  private async showServiceLogs(): Promise<void> {
    const services = await dockerManager.checkServicesHealth();
    const serviceNames = Array.from(services.keys());
    
    const selected = await vscode.window.showQuickPick(serviceNames, {
      placeHolder: 'Select service to view logs'
    });
    
    if (selected) {
      const logs = await dockerManager.getServiceLogs(selected);
      
      // Create new output channel for logs
      const logsChannel = vscode.window.createOutputChannel(`Logs: ${selected}`);
      logsChannel.append(logs);
      logsChannel.show();
    }
  }
  
  /**
   * Show success message
   */
  private async showSuccess(): Promise<void> {
    this.log('\n🎉 Setup Complete!');
    this.log('\nService URLs:');
    this.log('  - SymbioteIDE: http://localhost:8080');
    this.log('  - Neo4j Browser: http://localhost:7474');
    this.log('  - RabbitMQ Management: http://localhost:15672');
    this.log('  - Langfuse: http://localhost:3030');
    this.log('  - Prometheus: http://localhost:9090');
    this.log('  - Grafana: http://localhost:3031');
    
    const actions = await vscode.window.showInformationMessage(
      'SymbioteIDE setup complete! All services are running.',
      'Open IDE',
      'View Documentation',
      'Close'
    );
    
    if (actions === 'Open IDE') {
      vscode.env.openExternal(vscode.Uri.parse('http://localhost:8080'));
    } else if (actions === 'View Documentation') {
      vscode.env.openExternal(vscode.Uri.parse('https://symbiote-ide.com/docs'));
    }
  }
  
  /**
   * Save API keys
   */
  private async saveAPIKeys(apiKeys: any): Promise<void> {
    const configPath = path.join(
      process.env.HOME || process.env.USERPROFILE || '',
      '.symbiote',
      'api-keys.json'
    );
    
    await fs.mkdir(path.dirname(configPath), { recursive: true });
    await fs.writeFile(configPath, JSON.stringify(apiKeys, null, 2), {
      mode: 0o600 // Read/write for owner only
    });
  }
  
  /**
   * Log message
   */
  private log(message: string): void {
    this.outputChannel.appendLine(message);
  }
  
  /**
   * Run quick setup (non-interactive)
   */
  async runQuickSetup(): Promise<void> {
    this.log('Running quick setup...');
    
    const systemInfo = await platformDetector.detectSystem();
    await dockerManager.initialize();
    
    // Use defaults
    const options: SetupOptions = {
      includeOllama: systemInfo.localLLM?.type === 'none',
      localLLMType: systemInfo.localLLM?.type || 'none',
      localLLMEndpoint: systemInfo.localLLM?.endpoint
    };
    
    await dockerManager.startServices({
      includeOllama: options.includeOllama,
      detached: true
    });
    
    this.log('Quick setup complete!');
  }
}

// Export singleton
export const setupWizard = new SetupWizard();