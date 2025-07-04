/**
 * Docker Manager - Cross-platform Docker operations
 */

import * as path from 'path';
import * as fs from 'fs/promises';
import { spawn, ChildProcess } from 'child_process';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { platformDetector, Platform, DockerType, SystemInfo } from '../setup/platform-detector';

export interface DockerService {
  name: string;
  container?: string;
  status: 'running' | 'stopped' | 'starting' | 'error' | 'healthy' | 'unhealthy';
  health?: string;
  ports?: string[];
  error?: string;
}

export interface DockerProgress {
  service: string;
  status: string;
  progress?: number;
  message?: string;
}

export class DockerManager extends EventEmitter {
  private logger = new Logger('DockerManager');
  private systemInfo?: SystemInfo;
  private dockerProcess?: ChildProcess;
  private services: Map<string, DockerService> = new Map();
  
  // Service definitions
  private readonly REQUIRED_SERVICES = [
    'symbiote-ide',
    'orchestrator',
    'agent-runtime',
    'neo4j',
    'qdrant',
    'redis',
    'mem0',
    'rabbitmq',
    'langfuse',
    'postgres',
    'prometheus',
    'grafana'
  ];
  
  private readonly OPTIONAL_SERVICES = ['ollama'];
  
  constructor() {
    super();
  }
  
  /**
   * Initialize Docker manager
   */
  async initialize(): Promise<void> {
    this.systemInfo = await platformDetector.detectSystem();
    this.logger.info('Docker manager initialized', this.systemInfo);
  }
  
  /**
   * Get Docker Compose files for platform
   */
  private getComposeFiles(): string[] {
    if (!this.systemInfo) {
      throw new Error('DockerManager not initialized');
    }
    
    const baseFile = path.join(process.cwd(), 'docker', 'docker-compose.yml');
    const files = [baseFile];
    
    // Add platform-specific overrides
    switch (this.systemInfo.platform) {
      case Platform.Windows:
      case Platform.WindowsWSL:
        files.push(path.join(process.cwd(), 'docker', 'docker-compose.windows.yml'));
        break;
      case Platform.MacOS:
        files.push(path.join(process.cwd(), 'docker', 'docker-compose.macos.yml'));
        break;
    }
    
    return files;
  }
  
  /**
   * Prepare environment variables
   */
  private async prepareEnvironment(): Promise<Record<string, string>> {
    const env: Record<string, string> = { ...process.env };
    
    // Set workspace path
    if (this.systemInfo?.platform === Platform.Windows) {
      env.WORKSPACE_PATH = env.WORKSPACE_PATH || 'C:\\SymbioteWorkspace';
      env.CONFIG_PATH = env.CONFIG_PATH || `C:\\Users\\${process.env.USERNAME}\\.symbiote`;
    } else {
      env.WORKSPACE_PATH = env.WORKSPACE_PATH || path.join(process.env.HOME!, 'SymbioteWorkspace');
      env.CONFIG_PATH = env.CONFIG_PATH || path.join(process.env.HOME!, '.symbiote');
    }
    
    // Create directories if they don't exist
    await fs.mkdir(env.WORKSPACE_PATH, { recursive: true });
    await fs.mkdir(env.CONFIG_PATH, { recursive: true });
    
    // WSL-specific adjustments
    if (this.systemInfo?.isWSL) {
      // Convert Windows paths to WSL paths if needed
      if (env.WORKSPACE_PATH.startsWith('C:\\')) {
        env.WORKSPACE_PATH = env.WORKSPACE_PATH.replace('C:\\', '/mnt/c/').replace(/\\/g, '/');
      }
      if (env.CONFIG_PATH.startsWith('C:\\')) {
        env.CONFIG_PATH = env.CONFIG_PATH.replace('C:\\', '/mnt/c/').replace(/\\/g, '/');
      }
    }
    
    return env;
  }
  
  /**
   * Start Docker services
   */
  async startServices(options?: {
    includeOllama?: boolean;
    detached?: boolean;
  }): Promise<void> {
    this.logger.info('Starting Docker services...');
    
    const env = await this.prepareEnvironment();
    const composeFiles = this.getComposeFiles();
    
    // Build Docker Compose command
    const args: string[] = [];
    
    // Add compose files
    for (const file of composeFiles) {
      args.push('-f', file);
    }
    
    // Add command
    args.push('up');
    
    if (options?.detached !== false) {
      args.push('-d');
    }
    
    // Add services
    const services = [...this.REQUIRED_SERVICES];
    if (options?.includeOllama) {
      services.push('ollama');
    }
    args.push(...services);
    
    // Execute Docker Compose
    await this.executeDockerCompose(args, env);
    
    // Wait for services to be healthy
    await this.waitForServices();
  }
  
  /**
   * Stop Docker services
   */
  async stopServices(): Promise<void> {
    this.logger.info('Stopping Docker services...');
    
    const composeFiles = this.getComposeFiles();
    const args: string[] = [];
    
    // Add compose files
    for (const file of composeFiles) {
      args.push('-f', file);
    }
    
    args.push('down');
    
    await this.executeDockerCompose(args);
  }
  
  /**
   * Execute Docker Compose command
   */
  private async executeDockerCompose(
    args: string[],
    env?: Record<string, string>
  ): Promise<void> {
    return new Promise((resolve, reject) => {
      const command = this.systemInfo?.dockerType === DockerType.DockerDesktop ? 
        'docker' : 'docker';
      
      const fullArgs = ['compose', ...args];
      
      this.logger.info(`Executing: ${command} ${fullArgs.join(' ')}`);
      
      this.dockerProcess = spawn(command, fullArgs, {
        cwd: process.cwd(),
        env: env || process.env,
        shell: this.systemInfo?.platform === Platform.Windows
      });
      
      // Handle stdout
      this.dockerProcess.stdout?.on('data', (data) => {
        const output = data.toString();
        this.logger.debug(output);
        this.parseDockerOutput(output);
      });
      
      // Handle stderr
      this.dockerProcess.stderr?.on('data', (data) => {
        const output = data.toString();
        
        // Docker Compose often writes normal output to stderr
        if (output.includes('Creating') || output.includes('Starting') || output.includes('Pulling')) {
          this.logger.debug(output);
          this.parseDockerOutput(output);
        } else {
          this.logger.warn(output);
        }
      });
      
      // Handle completion
      this.dockerProcess.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`Docker Compose exited with code ${code}`));
        }
      });
      
      // Handle errors
      this.dockerProcess.on('error', (error) => {
        this.logger.error('Docker Compose error', error);
        reject(error);
      });
    });
  }
  
  /**
   * Parse Docker output for progress
   */
  private parseDockerOutput(output: string): void {
    const lines = output.split('\n').filter(line => line.trim());
    
    for (const line of lines) {
      // Parse service creation/start
      const createMatch = line.match(/Creating (.+)\.\.\./);
      if (createMatch) {
        this.updateServiceStatus(createMatch[1], 'starting');
      }
      
      const startMatch = line.match(/Starting (.+)\.\.\./);
      if (startMatch) {
        this.updateServiceStatus(startMatch[1], 'starting');
      }
      
      const doneMatch = line.match(/(.+)\s+done/);
      if (doneMatch) {
        this.updateServiceStatus(doneMatch[1].trim(), 'running');
      }
      
      // Parse pull progress
      const pullMatch = line.match(/Pulling (.+) \((\d+)\/(\d+)\)/);
      if (pullMatch) {
        const progress = Math.round((parseInt(pullMatch[2]) / parseInt(pullMatch[3])) * 100);
        this.emitProgress({
          service: pullMatch[1],
          status: 'pulling',
          progress,
          message: `Pulling image layers`
        });
      }
    }
  }
  
  /**
   * Update service status
   */
  private updateServiceStatus(serviceName: string, status: DockerService['status']): void {
    const service = this.services.get(serviceName) || { name: serviceName, status: 'stopped' };
    service.status = status;
    this.services.set(serviceName, service);
    
    this.emitProgress({
      service: serviceName,
      status,
      message: `Service ${status}`
    });
  }
  
  /**
   * Emit progress event
   */
  private emitProgress(progress: DockerProgress): void {
    this.emit('progress', progress);
  }
  
  /**
   * Wait for services to be healthy
   */
  private async waitForServices(timeout: number = 300000): Promise<void> {
    this.logger.info('Waiting for services to be healthy...');
    
    const startTime = Date.now();
    const checkInterval = 5000;
    
    while (Date.now() - startTime < timeout) {
      const statuses = await this.checkServicesHealth();
      
      // Check if all required services are healthy
      const allHealthy = this.REQUIRED_SERVICES.every(service => {
        const status = statuses.get(service);
        return status?.status === 'healthy' || status?.status === 'running';
      });
      
      if (allHealthy) {
        this.logger.info('All services are healthy!');
        return;
      }
      
      // Log current status
      for (const [name, service] of statuses) {
        if (service.status !== 'healthy' && service.status !== 'running') {
          this.logger.debug(`${name}: ${service.status} ${service.health || ''}`);
        }
      }
      
      await new Promise(resolve => setTimeout(resolve, checkInterval));
    }
    
    throw new Error('Timeout waiting for services to be healthy');
  }
  
  /**
   * Check health of all services
   */
  async checkServicesHealth(): Promise<Map<string, DockerService>> {
    const services = new Map<string, DockerService>();
    
    try {
      // Get container statuses
      const { stdout } = await this.execCommand([
        'docker', 'ps', '--format', 
        '{{.Names}}\\t{{.Status}}\\t{{.Ports}}'
      ]);
      
      const lines = stdout.split('\n').filter(line => line.trim());
      
      for (const line of lines) {
        const [name, status, ports] = line.split('\t');
        
        if (this.REQUIRED_SERVICES.includes(name) || this.OPTIONAL_SERVICES.includes(name)) {
          const service: DockerService = {
            name,
            container: name,
            status: this.parseContainerStatus(status),
            health: status,
            ports: ports ? ports.split(', ') : []
          };
          
          services.set(name, service);
        }
      }
      
      // Check specific service health endpoints
      await this.checkServiceEndpoints(services);
      
    } catch (error) {
      this.logger.error('Failed to check service health', error);
    }
    
    return services;
  }
  
  /**
   * Parse container status
   */
  private parseContainerStatus(status: string): DockerService['status'] {
    if (status.includes('healthy')) return 'healthy';
    if (status.includes('unhealthy')) return 'unhealthy';
    if (status.includes('starting')) return 'starting';
    if (status.includes('Up')) return 'running';
    return 'stopped';
  }
  
  /**
   * Check service endpoints
   */
  private async checkServiceEndpoints(services: Map<string, DockerService>): Promise<void> {
    const healthChecks = [
      { name: 'symbiote-ide', url: 'http://localhost:8080/health' },
      { name: 'orchestrator', url: 'http://localhost:9000/health' },
      { name: 'neo4j', url: 'http://localhost:7474' },
      { name: 'qdrant', url: 'http://localhost:6333/health' },
      { name: 'mem0', url: 'http://localhost:8000/health' },
      { name: 'langfuse', url: 'http://localhost:3030/api/public/health' },
      { name: 'prometheus', url: 'http://localhost:9090/-/healthy' },
      { name: 'grafana', url: 'http://localhost:3031/api/health' }
    ];
    
    for (const check of healthChecks) {
      const service = services.get(check.name);
      if (service && service.status === 'running') {
        try {
          const response = await fetch(check.url, { 
            method: 'GET',
            signal: AbortSignal.timeout(5000)
          });
          
          if (response.ok) {
            service.status = 'healthy';
          }
        } catch (error) {
          // Service not responding yet
          service.status = 'unhealthy';
        }
      }
    }
  }
  
  /**
   * Execute command
   */
  private execCommand(command: string[]): Promise<{ stdout: string; stderr: string }> {
    return new Promise((resolve, reject) => {
      const proc = spawn(command[0], command.slice(1), {
        shell: this.systemInfo?.platform === Platform.Windows
      });
      
      let stdout = '';
      let stderr = '';
      
      proc.stdout.on('data', (data) => stdout += data.toString());
      proc.stderr.on('data', (data) => stderr += data.toString());
      
      proc.on('close', (code) => {
        if (code === 0) {
          resolve({ stdout, stderr });
        } else {
          reject(new Error(`Command failed with code ${code}: ${stderr}`));
        }
      });
    });
  }
  
  /**
   * Configure local LLM
   */
  async configureLocalLLM(config: {
    type: 'ollama' | 'lm-studio' | 'llama-cpp' | 'custom';
    endpoint: string;
    apiKey?: string;
  }): Promise<void> {
    this.logger.info(`Configuring local LLM: ${config.type}`);
    
    // Save configuration
    const configPath = path.join(
      this.systemInfo?.platform === Platform.Windows ? 
        `C:\\Users\\${process.env.USERNAME}\\.symbiote` :
        path.join(process.env.HOME!, '.symbiote'),
      'local-llm.json'
    );
    
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    
    // Update orchestrator environment
    if (config.type === 'ollama') {
      // Enable Ollama service if not already running
      await this.startOllamaService();
    }
    
    // Notify orchestrator of new LLM endpoint
    try {
      await fetch('http://localhost:9000/api/config/local-llm', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      });
    } catch (error) {
      this.logger.warn('Failed to update orchestrator with local LLM config', error);
    }
  }
  
  /**
   * Start Ollama service
   */
  private async startOllamaService(): Promise<void> {
    const composeFiles = this.getComposeFiles();
    const args: string[] = [];
    
    for (const file of composeFiles) {
      args.push('-f', file);
    }
    
    args.push('up', '-d', 'ollama');
    
    await this.executeDockerCompose(args);
  }
  
  /**
   * Get service logs
   */
  async getServiceLogs(serviceName: string, lines: number = 100): Promise<string> {
    const { stdout } = await this.execCommand([
      'docker', 'logs', '--tail', lines.toString(), serviceName
    ]);
    
    return stdout;
  }
  
  /**
   * Restart service
   */
  async restartService(serviceName: string): Promise<void> {
    this.logger.info(`Restarting service: ${serviceName}`);
    
    await this.execCommand(['docker', 'restart', serviceName]);
    
    // Wait for service to be healthy again
    const startTime = Date.now();
    const timeout = 60000;
    
    while (Date.now() - startTime < timeout) {
      const statuses = await this.checkServicesHealth();
      const service = statuses.get(serviceName);
      
      if (service?.status === 'healthy' || service?.status === 'running') {
        this.logger.info(`Service ${serviceName} restarted successfully`);
        return;
      }
      
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    
    throw new Error(`Timeout waiting for ${serviceName} to restart`);
  }
  
  /**
   * Get service stats
   */
  async getServiceStats(): Promise<Map<string, any>> {
    const stats = new Map();
    
    try {
      const { stdout } = await this.execCommand([
        'docker', 'stats', '--no-stream', '--format',
        '{{.Container}}\\t{{.CPUPerc}}\\t{{.MemUsage}}\\t{{.NetIO}}'
      ]);
      
      const lines = stdout.split('\n').filter(line => line.trim());
      
      for (const line of lines) {
        const [container, cpu, memory, network] = line.split('\t');
        
        if (this.REQUIRED_SERVICES.includes(container) || this.OPTIONAL_SERVICES.includes(container)) {
          stats.set(container, {
            cpu,
            memory,
            network
          });
        }
      }
    } catch (error) {
      this.logger.error('Failed to get service stats', error);
    }
    
    return stats;
  }
  
  /**
   * Cleanup
   */
  async cleanup(): Promise<void> {
    if (this.dockerProcess) {
      this.dockerProcess.kill();
    }
    
    this.services.clear();
    this.removeAllListeners();
  }
}

// Export singleton
export const dockerManager = new DockerManager();