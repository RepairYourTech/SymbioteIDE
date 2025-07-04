/**
 * Kernel Manager
 * 
 * Manages notebook kernels for different languages and execution environments
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import {
  KernelInfo,
  KernelStatus,
  KernelSpec,
  CellLanguage,
  ExecuteRequest,
  ExecuteResult,
  NotebookEvents
} from './types';
import { JavaScriptKernel } from './kernels/javascript-kernel';
import { TypeScriptKernel } from './kernels/typescript-kernel';
import { PythonKernel } from './kernels/python-kernel';

export interface KernelManagerConfig {
  maxKernels?: number;
  defaultTimeout?: number;
  enableRemoteKernels?: boolean;
  kernelSpecs?: KernelSpec[];
}

export abstract class Kernel extends EventEmitter {
  abstract id: string;
  abstract name: string;
  abstract language: CellLanguage;
  abstract status: KernelStatus;
  
  abstract initialize(): Promise<void>;
  abstract execute(request: ExecuteRequest): Promise<ExecuteResult>;
  abstract interrupt(): Promise<void>;
  abstract restart(): Promise<void>;
  abstract shutdown(): Promise<void>;
  abstract getInfo(): KernelInfo;
}

export class KernelManager extends EventEmitter {
  private kernels = new Map<string, Kernel>();
  private activeKernel?: Kernel;
  private config: Required<KernelManagerConfig>;
  private logger = new Logger('KernelManager');
  
  constructor(config: KernelManagerConfig = {}) {
    super();
    
    this.config = {
      maxKernels: config.maxKernels || 10,
      defaultTimeout: config.defaultTimeout || 30000,
      enableRemoteKernels: config.enableRemoteKernels || false,
      kernelSpecs: config.kernelSpecs || this.getDefaultKernelSpecs()
    };
  }
  
  /**
   * Initialize the kernel manager
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing kernel manager...');
    
    // Register built-in kernels
    await this.registerBuiltinKernels();
    
    this.logger.info('Kernel manager initialized');
    this.emit('initialized');
  }
  
  /**
   * Create a new kernel
   */
  async createKernel(language: CellLanguage): Promise<Kernel> {
    // Check kernel limit
    if (this.kernels.size >= this.config.maxKernels) {
      throw new Error(`Maximum number of kernels (${this.config.maxKernels}) reached`);
    }
    
    let kernel: Kernel;
    
    switch (language) {
      case CellLanguage.JavaScript:
        kernel = new JavaScriptKernel();
        break;
        
      case CellLanguage.TypeScript:
        kernel = new TypeScriptKernel();
        break;
        
      case CellLanguage.Python:
        kernel = new PythonKernel();
        break;
        
      default:
        throw new Error(`Unsupported language: ${language}`);
    }
    
    // Initialize kernel
    await kernel.initialize();
    
    // Store kernel
    this.kernels.set(kernel.id, kernel);
    
    // Set up event forwarding
    this.setupKernelEvents(kernel);
    
    this.logger.info(`Created ${language} kernel: ${kernel.id}`);
    this.emit('kernel-created', kernel.getInfo());
    
    return kernel;
  }
  
  /**
   * Get or create kernel for language
   */
  async getKernel(language: CellLanguage): Promise<Kernel> {
    // Find existing kernel
    for (const kernel of this.kernels.values()) {
      if (kernel.language === language && kernel.status !== KernelStatus.Dead) {
        return kernel;
      }
    }
    
    // Create new kernel
    return this.createKernel(language);
  }
  
  /**
   * Execute code in a kernel
   */
  async execute(request: ExecuteRequest): Promise<ExecuteResult> {
    const kernel = await this.getKernel(request.language);
    this.activeKernel = kernel;
    
    try {
      const result = await kernel.execute(request);
      this.emit('execution-complete', result);
      return result;
    } catch (error) {
      this.logger.error('Execution failed', error);
      throw error;
    }
  }
  
  /**
   * Interrupt the active kernel
   */
  async interruptKernel(kernelId?: string): Promise<void> {
    const kernel = kernelId ? this.kernels.get(kernelId) : this.activeKernel;
    
    if (!kernel) {
      throw new Error('No kernel to interrupt');
    }
    
    await kernel.interrupt();
    this.emit('kernel-interrupted', kernel.getInfo());
  }
  
  /**
   * Restart a kernel
   */
  async restartKernel(kernelId: string): Promise<void> {
    const kernel = this.kernels.get(kernelId);
    
    if (!kernel) {
      throw new Error(`Kernel not found: ${kernelId}`);
    }
    
    await kernel.restart();
    this.emit('kernel-restarted', kernel.getInfo());
  }
  
  /**
   * Shutdown a kernel
   */
  async shutdownKernel(kernelId: string): Promise<void> {
    const kernel = this.kernels.get(kernelId);
    
    if (!kernel) {
      throw new Error(`Kernel not found: ${kernelId}`);
    }
    
    await kernel.shutdown();
    this.kernels.delete(kernelId);
    
    if (this.activeKernel === kernel) {
      this.activeKernel = undefined;
    }
    
    this.emit('kernel-shutdown', kernelId);
  }
  
  /**
   * Shutdown all kernels
   */
  async shutdownAll(): Promise<void> {
    const shutdownPromises = Array.from(this.kernels.values()).map(kernel =>
      kernel.shutdown().catch(error => 
        this.logger.error(`Failed to shutdown kernel ${kernel.id}`, error)
      )
    );
    
    await Promise.all(shutdownPromises);
    this.kernels.clear();
    this.activeKernel = undefined;
    
    this.emit('all-kernels-shutdown');
  }
  
  /**
   * Get all kernels
   */
  getKernels(): KernelInfo[] {
    return Array.from(this.kernels.values()).map(kernel => kernel.getInfo());
  }
  
  /**
   * Get kernel by ID
   */
  getKernelById(kernelId: string): Kernel | undefined {
    return this.kernels.get(kernelId);
  }
  
  /**
   * Get active kernel
   */
  getActiveKernel(): Kernel | undefined {
    return this.activeKernel;
  }
  
  /**
   * Set active kernel
   */
  setActiveKernel(kernelId: string): void {
    const kernel = this.kernels.get(kernelId);
    
    if (!kernel) {
      throw new Error(`Kernel not found: ${kernelId}`);
    }
    
    this.activeKernel = kernel;
    this.emit('active-kernel-changed', kernel.getInfo());
  }
  
  /**
   * Get kernel specs
   */
  getKernelSpecs(): KernelSpec[] {
    return this.config.kernelSpecs;
  }
  
  /**
   * Register custom kernel spec
   */
  registerKernelSpec(spec: KernelSpec): void {
    this.config.kernelSpecs.push(spec);
    this.emit('kernel-spec-registered', spec);
  }
  
  // Private methods
  
  private async registerBuiltinKernels(): Promise<void> {
    // Built-in kernels are created on demand
    this.logger.info('Built-in kernels registered');
  }
  
  private setupKernelEvents(kernel: Kernel): void {
    // Forward kernel events
    kernel.on('status-changed', (status: KernelStatus) => {
      const info = kernel.getInfo();
      this.emit('kernel-status-changed', { kernelId: info.id, status });
      
      // Handle dead kernels
      if (status === KernelStatus.Dead) {
        this.handleDeadKernel(kernel);
      }
    });
    
    kernel.on('execution-state-changed', (state: any) => {
      this.emit('execution-state-changed', { kernelId: kernel.id, state });
    });
    
    kernel.on('output', (output: any) => {
      this.emit('kernel-output', { kernelId: kernel.id, output });
    });
  }
  
  private handleDeadKernel(kernel: Kernel): void {
    this.logger.warn(`Kernel died: ${kernel.id}`);
    
    // Remove from active
    if (this.activeKernel === kernel) {
      this.activeKernel = undefined;
    }
    
    // Attempt cleanup
    this.kernels.delete(kernel.id);
    
    this.emit('kernel-died', kernel.getInfo());
  }
  
  private getDefaultKernelSpecs(): KernelSpec[] {
    return [
      {
        name: 'javascript',
        displayName: 'JavaScript',
        language: CellLanguage.JavaScript,
        argv: ['node']
      },
      {
        name: 'typescript',
        displayName: 'TypeScript',
        language: CellLanguage.TypeScript,
        argv: ['ts-node']
      },
      {
        name: 'python',
        displayName: 'Python',
        language: CellLanguage.Python,
        argv: ['python', '-m', 'ipykernel_launcher']
      }
    ];
  }
  
  /**
   * Clean up resources
   */
  async dispose(): Promise<void> {
    await this.shutdownAll();
    this.removeAllListeners();
  }
}