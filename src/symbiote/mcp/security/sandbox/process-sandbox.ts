/**
 * Process Sandbox
 * 
 * Creates isolated process environments with resource restrictions
 */

import { ChildProcess, spawn, SpawnOptions } from 'child_process';
import { Worker } from 'worker_threads';
import * as path from 'path';
import * as fs from 'fs';
import { SandboxConfig } from '../types';

export interface SandboxOptions extends SpawnOptions {
  timeout?: number;
  killSignal?: NodeJS.Signals;
}

export class ProcessSandbox {
  private process: ChildProcess | null = null;
  private worker: Worker | null = null;
  private killed: boolean = false;
  private timeout: NodeJS.Timeout | null = null;
  
  constructor(private config: SandboxConfig) {}
  
  /**
   * Spawn sandboxed process
   */
  async spawn(
    command: string,
    args: string[],
    options: SandboxOptions = {}
  ): Promise<ChildProcess> {
    // Validate command against restrictions
    await this.validateCommand(command, args);
    
    // Create sandbox wrapper script
    const wrapperPath = await this.createWrapper(command, args, options);
    
    // Spawn process with restrictions
    const spawnOptions: SpawnOptions = {
      ...options,
      // Detach from parent
      detached: false,
      // Use shell to apply ulimits
      shell: process.platform !== 'win32',
      // Inherit stdio for now (will be intercepted)
      stdio: ['pipe', 'pipe', 'pipe']
    };
    
    // Apply resource limits via ulimit (Unix only)
    if (process.platform !== 'win32') {
      const limits = this.buildResourceLimits();
      this.process = spawn('sh', ['-c', `${limits} && node "${wrapperPath}"`], spawnOptions);
    } else {
      // Windows: Use Job Objects via wrapper
      this.process = spawn('node', [wrapperPath], spawnOptions);
    }
    
    // Setup timeout if specified
    if (options.timeout || this.config.resourceLimits.maxExecutionTime) {
      const timeout = options.timeout || this.config.resourceLimits.maxExecutionTime * 1000;
      this.timeout = setTimeout(() => {
        this.terminate(true);
      }, timeout);
    }
    
    // Setup process monitoring
    this.setupProcessMonitoring();
    
    return this.process;
  }
  
  /**
   * Spawn sandboxed worker thread
   */
  async spawnWorker(
    scriptPath: string,
    options: any = {}
  ): Promise<Worker> {
    // Validate script
    await this.validateScript(scriptPath);
    
    // Create worker with restrictions
    this.worker = new Worker(path.join(__dirname, 'worker-wrapper.js'), {
      workerData: {
        scriptPath,
        config: this.config,
        originalOptions: options
      },
      // Resource limits for worker
      resourceLimits: {
        maxOldGenerationSizeMb: this.config.resourceLimits.maxMemoryMB,
        maxYoungGenerationSizeMb: Math.floor(this.config.resourceLimits.maxMemoryMB / 4),
        codeRangeSizeMb: 64
      }
    });
    
    return this.worker;
  }
  
  /**
   * Terminate sandbox
   */
  async terminate(force: boolean = false): Promise<void> {
    if (this.killed) return;
    this.killed = true;
    
    if (this.timeout) {
      clearTimeout(this.timeout);
    }
    
    if (this.process) {
      if (force) {
        this.process.kill('SIGKILL');
      } else {
        this.process.kill('SIGTERM');
        
        // Give it time to cleanup
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        // Force kill if still alive
        if (!this.process.killed) {
          this.process.kill('SIGKILL');
        }
      }
    }
    
    if (this.worker) {
      await this.worker.terminate();
    }
  }
  
  /**
   * Create wrapper script
   */
  private async createWrapper(
    command: string,
    args: string[],
    options: SandboxOptions
  ): Promise<string> {
    const wrapperId = Date.now().toString(36) + Math.random().toString(36).substr(2);
    const wrapperPath = path.join(
      process.cwd(),
      '.symbiote',
      'sandbox',
      `wrapper-${wrapperId}.js`
    );
    
    // Ensure directory exists
    await fs.promises.mkdir(path.dirname(wrapperPath), { recursive: true });
    
    // Create wrapper script
    const wrapperCode = `
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

// Sandbox configuration
const config = ${JSON.stringify(this.config)};

// Original command
const command = ${JSON.stringify(command)};
const args = ${JSON.stringify(args)};
const options = ${JSON.stringify(options)};

// Resource monitoring
let resourceInterval;
const startTime = Date.now();

// Override dangerous functions
const originalRequire = require;
require = new Proxy(originalRequire, {
  apply(target, thisArg, argumentsList) {
    const moduleName = argumentsList[0];
    
    // Block dangerous modules
    const blockedModules = ['child_process', 'cluster', 'dgram', 'net', 'tls', 'http2', 'repl'];
    if (config.process.allowSpawn === false && blockedModules.includes(moduleName)) {
      throw new Error(\`Module '\${moduleName}' is not allowed in sandbox\`);
    }
    
    return target.apply(thisArg, argumentsList);
  }
});

// Override process.env
if (config.process.envVarRestrictions.blockAll) {
  process.env = {};
}

// Apply filesystem restrictions
const originalFs = { ...fs };
const restrictedFs = new Proxy(fs, {
  get(target, prop) {
    if (typeof target[prop] === 'function') {
      return new Proxy(target[prop], {
        apply(fn, thisArg, args) {
          // Check filesystem access
          const pathArg = args[0];
          if (typeof pathArg === 'string') {
            if (!isPathAllowed(pathArg, prop)) {
              throw new Error(\`Access denied: \${pathArg}\`);
            }
          }
          return fn.apply(thisArg, args);
        }
      });
    }
    return target[prop];
  }
});

// Path checking function
function isPathAllowed(filePath, operation) {
  const normalized = path.normalize(filePath);
  
  // Check blocked paths
  for (const blocked of config.filesystem.blockedPaths) {
    if (minimatch(normalized, blocked)) {
      return false;
    }
  }
  
  // Check allowed paths based on operation
  const isWrite = ['write', 'append', 'mkdir', 'unlink', 'rmdir'].some(op => 
    operation.toLowerCase().includes(op)
  );
  
  const allowedPaths = isWrite ? 
    config.filesystem.allowedWritePaths : 
    config.filesystem.allowedReadPaths;
  
  for (const allowed of allowedPaths) {
    if (normalized.startsWith(path.normalize(allowed))) {
      return true;
    }
  }
  
  return false;
}

// Simple minimatch implementation
function minimatch(path, pattern) {
  const regex = pattern
    .replace(/\\*/g, '.*')
    .replace(/\\?/g, '.')
    .replace(/\\[([^\\]]+)\\]/g, '[$1]');
  return new RegExp('^' + regex + '$').test(path);
}

// Replace fs module
Object.keys(restrictedFs).forEach(key => {
  fs[key] = restrictedFs[key];
});

// Monitor resources
function checkResources() {
  const memUsage = process.memoryUsage();
  const elapsed = Date.now() - startTime;
  
  if (memUsage.heapTotal > config.resourceLimits.maxMemoryMB * 1024 * 1024) {
    console.error('Memory limit exceeded');
    process.exit(137);
  }
  
  if (elapsed > config.resourceLimits.maxExecutionTime * 1000) {
    console.error('Execution time limit exceeded');
    process.exit(124);
  }
}

if (config.monitoring.enabled) {
  resourceInterval = setInterval(checkResources, 100);
}

// Spawn actual process
const child = spawn(command, args, {
  ...options,
  stdio: 'inherit'
});

child.on('exit', (code, signal) => {
  if (resourceInterval) {
    clearInterval(resourceInterval);
  }
  process.exit(code || 0);
});

child.on('error', (err) => {
  console.error('Sandbox process error:', err);
  process.exit(1);
});

// Handle parent termination
process.on('SIGTERM', () => {
  child.kill('SIGTERM');
});

process.on('SIGINT', () => {
  child.kill('SIGINT');
});
`;
    
    await fs.promises.writeFile(wrapperPath, wrapperCode);
    
    // Cleanup after some time
    setTimeout(() => {
      fs.promises.unlink(wrapperPath).catch(() => {});
    }, 60000);
    
    return wrapperPath;
  }
  
  /**
   * Build resource limits command (Unix)
   */
  private buildResourceLimits(): string {
    const limits = this.config.resourceLimits;
    const commands: string[] = [];
    
    // CPU limit (via nice)
    commands.push(`nice -n 19`);
    
    // Memory limit
    const memoryKB = limits.maxMemoryMB * 1024;
    commands.push(`ulimit -v ${memoryKB}`);
    
    // File handles
    commands.push(`ulimit -n ${limits.maxFileHandles}`);
    
    // Process/thread limit
    commands.push(`ulimit -u ${limits.maxThreads}`);
    
    // Core dump disabled
    commands.push(`ulimit -c 0`);
    
    return commands.join(' && ');
  }
  
  /**
   * Validate command
   */
  private async validateCommand(command: string, args: string[]): Promise<void> {
    // Check if process spawning is allowed
    if (!this.config.process.allowSpawn) {
      throw new Error('Process spawning is not allowed');
    }
    
    // Check against allowed executables
    if (this.config.process.allowedExecutables) {
      const allowed = this.config.process.allowedExecutables;
      const cmdName = path.basename(command);
      
      if (!allowed.includes(command) && !allowed.includes(cmdName)) {
        throw new Error(`Executable '${command}' is not allowed`);
      }
    }
    
    // Check for shell injection attempts
    const dangerous = [';', '&&', '||', '|', '`', '$', '>', '<', '&'];
    const allArgs = [command, ...args].join(' ');
    
    for (const char of dangerous) {
      if (allArgs.includes(char)) {
        throw new Error(`Potentially dangerous character '${char}' in command`);
      }
    }
  }
  
  /**
   * Validate script path
   */
  private async validateScript(scriptPath: string): Promise<void> {
    // Normalize path
    const normalized = path.normalize(scriptPath);
    
    // Check if within allowed paths
    let allowed = false;
    for (const allowedPath of this.config.filesystem.allowedReadPaths) {
      if (normalized.startsWith(path.normalize(allowedPath))) {
        allowed = true;
        break;
      }
    }
    
    if (!allowed) {
      throw new Error(`Script path '${scriptPath}' is not in allowed directories`);
    }
    
    // Check file exists
    try {
      await fs.promises.access(scriptPath, fs.constants.R_OK);
    } catch {
      throw new Error(`Script '${scriptPath}' not found or not readable`);
    }
  }
  
  /**
   * Setup process monitoring
   */
  private setupProcessMonitoring(): void {
    if (!this.process) return;
    
    // Monitor stdout/stderr for suspicious patterns
    if (this.config.monitoring.behavioralAnalysis) {
      this.process.stdout?.on('data', (data) => {
        this.analyzeBehavior(data.toString());
      });
      
      this.process.stderr?.on('data', (data) => {
        this.analyzeBehavior(data.toString());
      });
    }
  }
  
  /**
   * Analyze process behavior
   */
  private analyzeBehavior(output: string): void {
    // Look for suspicious patterns
    const suspiciousPatterns = [
      /accessing.*\/etc\/passwd/i,
      /accessing.*\.ssh/i,
      /downloading.*malware/i,
      /establishing.*backdoor/i,
      /scanning.*ports/i,
      /cryptocurrency.*mining/i
    ];
    
    for (const pattern of suspiciousPatterns) {
      if (pattern.test(output)) {
        // Report suspicious behavior
        console.error('Suspicious behavior detected:', output);
        // Could terminate process here
      }
    }
  }
}