/**
 * JavaScript Kernel
 * 
 * Executes JavaScript code with support for React, modules, and hot reloading
 */

import { EventEmitter } from 'events';
import * as vm from 'vm';
import * as path from 'path';
import { Logger } from '../../utils/logger';
import { Kernel } from '../kernel-manager';
import {
  KernelInfo,
  KernelStatus,
  CellLanguage,
  ExecuteRequest,
  ExecuteResult,
  CellOutput,
  ExecutionError
} from '../types';

interface VMContext {
  console: Console;
  require: NodeRequire;
  module: NodeModule;
  exports: any;
  __dirname: string;
  __filename: string;
  global: any;
  process: NodeJS.Process;
  setTimeout: typeof setTimeout;
  setInterval: typeof setInterval;
  clearTimeout: typeof clearTimeout;
  clearInterval: typeof clearInterval;
  React?: any;
  ReactDOM?: any;
  [key: string]: any;
}

export class JavaScriptKernel extends EventEmitter implements Kernel {
  public id: string;
  public name = 'JavaScript';
  public language = CellLanguage.JavaScript;
  public status = KernelStatus.Starting;
  
  private context?: vm.Context;
  private executionCount = 0;
  private logger = new Logger('JavaScriptKernel');
  private globals: Map<string, any> = new Map();
  private moduleCache: Map<string, any> = new Map();
  private timers: Set<NodeJS.Timeout> = new Set();
  
  constructor() {
    super();
    this.id = `js-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
  
  async initialize(): Promise<void> {
    try {
      this.logger.info('Initializing JavaScript kernel...');
      
      // Create VM context
      this.context = this.createContext();
      
      // Load React and other common libraries
      await this.loadCommonLibraries();
      
      this.status = KernelStatus.Idle;
      this.emit('status-changed', this.status);
      
      this.logger.info('JavaScript kernel initialized');
    } catch (error) {
      this.status = KernelStatus.Dead;
      this.emit('status-changed', this.status);
      throw error;
    }
  }
  
  async execute(request: ExecuteRequest): Promise<ExecuteResult> {
    const startTime = Date.now();
    this.status = KernelStatus.Busy;
    this.emit('status-changed', this.status);
    
    const outputs: CellOutput[] = [];
    let error: ExecutionError | undefined;
    
    try {
      // Set up execution environment
      this.setupExecutionEnvironment(request, outputs);
      
      // Handle React component detection
      const isReactComponent = this.isReactComponent(request.code);
      let code = request.code;
      
      if (isReactComponent) {
        code = this.wrapReactComponent(code, request.cellId);
      }
      
      // Execute code
      const script = new vm.Script(code, {
        filename: `cell-${request.cellId}.js`,
        lineOffset: 0,
        columnOffset: 0
      });
      
      const result = script.runInContext(this.context!, {
        timeout: 30000,
        breakOnSigint: true
      });
      
      // Handle execution result
      if (result !== undefined) {
        if (isReactComponent) {
          outputs.push({
            type: 'react',
            data: {
              component: result,
              cellId: request.cellId
            }
          });
        } else {
          outputs.push({
            type: 'data',
            data: this.serializeOutput(result)
          });
        }
      }
      
      this.executionCount++;
      
    } catch (err: any) {
      error = {
        name: err.name || 'Error',
        message: err.message || String(err),
        stack: err.stack,
        lineNumber: err.lineNumber,
        columnNumber: err.columnNumber
      };
      
      outputs.push({
        type: 'error',
        data: error
      });
    } finally {
      this.status = KernelStatus.Idle;
      this.emit('status-changed', this.status);
    }
    
    const endTime = Date.now();
    
    return {
      cellId: request.cellId,
      executionCount: this.executionCount,
      outputs,
      status: error ? 'error' : 'ok',
      error,
      timing: {
        startTime,
        endTime,
        duration: endTime - startTime
      }
    };
  }
  
  async interrupt(): Promise<void> {
    this.logger.info('Interrupting JavaScript kernel');
    
    // Clear all timers
    this.timers.forEach(timer => clearTimeout(timer));
    this.timers.clear();
    
    // Reset status
    this.status = KernelStatus.Idle;
    this.emit('status-changed', this.status);
  }
  
  async restart(): Promise<void> {
    this.logger.info('Restarting JavaScript kernel');
    
    this.status = KernelStatus.Restarting;
    this.emit('status-changed', this.status);
    
    // Clear state
    this.globals.clear();
    this.moduleCache.clear();
    this.timers.forEach(timer => clearTimeout(timer));
    this.timers.clear();
    this.executionCount = 0;
    
    // Reinitialize
    await this.initialize();
  }
  
  async shutdown(): Promise<void> {
    this.logger.info('Shutting down JavaScript kernel');
    
    this.status = KernelStatus.Terminating;
    this.emit('status-changed', this.status);
    
    // Clear all resources
    this.globals.clear();
    this.moduleCache.clear();
    this.timers.forEach(timer => clearTimeout(timer));
    this.timers.clear();
    this.context = undefined;
    
    this.status = KernelStatus.Dead;
    this.emit('status-changed', this.status);
  }
  
  getInfo(): KernelInfo {
    return {
      id: this.id,
      name: this.name,
      status: this.status,
      language: this.language,
      executionCount: this.executionCount,
      lastActivity: new Date()
    };
  }
  
  // Private methods
  
  private createContext(): vm.Context {
    const sandbox: VMContext = {
      console: this.createConsoleProxy(),
      require: this.createRequireProxy(),
      module: { exports: {} } as any,
      exports: {},
      __dirname: process.cwd(),
      __filename: 'notebook.js',
      global: {},
      process: {
        env: {},
        version: process.version,
        versions: process.versions,
        platform: process.platform,
        arch: process.arch
      } as any,
      setTimeout: (fn: Function, ms: number) => {
        const timer = setTimeout(fn, ms);
        this.timers.add(timer);
        return timer;
      },
      setInterval: (fn: Function, ms: number) => {
        const timer = setInterval(fn, ms);
        this.timers.add(timer);
        return timer;
      },
      clearTimeout: (timer: NodeJS.Timeout) => {
        clearTimeout(timer);
        this.timers.delete(timer);
      },
      clearInterval: (timer: NodeJS.Timeout) => {
        clearInterval(timer);
        this.timers.delete(timer);
      }
    };
    
    // Add globals
    this.globals.forEach((value, key) => {
      sandbox[key] = value;
    });
    
    sandbox.global = sandbox;
    
    return vm.createContext(sandbox);
  }
  
  private createConsoleProxy(): Console {
    const outputs: CellOutput[] = [];
    
    return {
      log: (...args: any[]) => {
        this.emit('output', {
          type: 'text',
          data: args.map(arg => this.stringify(arg)).join(' ')
        });
      },
      error: (...args: any[]) => {
        this.emit('output', {
          type: 'error',
          data: args.map(arg => this.stringify(arg)).join(' ')
        });
      },
      warn: (...args: any[]) => {
        this.emit('output', {
          type: 'text',
          data: `[WARN] ${args.map(arg => this.stringify(arg)).join(' ')}`
        });
      },
      info: (...args: any[]) => {
        this.emit('output', {
          type: 'text',
          data: `[INFO] ${args.map(arg => this.stringify(arg)).join(' ')}`
        });
      },
      debug: (...args: any[]) => {
        this.emit('output', {
          type: 'text',
          data: `[DEBUG] ${args.map(arg => this.stringify(arg)).join(' ')}`
        });
      },
      table: (data: any) => {
        this.emit('output', {
          type: 'data',
          data: { type: 'table', value: data }
        });
      },
      time: () => {},
      timeEnd: () => {},
      trace: () => {},
      assert: () => {},
      clear: () => {},
      count: () => {},
      countReset: () => {},
      group: () => {},
      groupEnd: () => {},
      groupCollapsed: () => {},
      dir: () => {},
      dirxml: () => {},
      profile: () => {},
      profileEnd: () => {},
      timeLog: () => {},
      timeStamp: () => {}
    } as Console;
  }
  
  private createRequireProxy(): NodeRequire {
    return ((moduleName: string) => {
      // Check module cache
      if (this.moduleCache.has(moduleName)) {
        return this.moduleCache.get(moduleName);
      }
      
      // Handle special modules
      switch (moduleName) {
        case 'react':
          return this.globals.get('React');
        case 'react-dom':
          return this.globals.get('ReactDOM');
        default:
          try {
            const module = require(moduleName);
            this.moduleCache.set(moduleName, module);
            return module;
          } catch (error) {
            throw new Error(`Cannot find module '${moduleName}'`);
          }
      }
    }) as NodeRequire;
  }
  
  private async loadCommonLibraries(): Promise<void> {
    try {
      // Load React (we'll implement dynamic loading later)
      // For now, we'll use a placeholder
      this.globals.set('React', {
        createElement: () => {},
        Component: class Component {},
        useState: () => [null, () => {}],
        useEffect: () => {},
        Fragment: Symbol('Fragment')
      });
      
      this.globals.set('ReactDOM', {
        render: () => {},
        createRoot: () => ({ render: () => {} })
      });
      
    } catch (error) {
      this.logger.warn('Failed to load some common libraries', error);
    }
  }
  
  private setupExecutionEnvironment(request: ExecuteRequest, outputs: CellOutput[]): void {
    const sandbox = (this.context as any) as VMContext;
    
    // Clear previous cell exports
    sandbox.module.exports = {};
    sandbox.exports = sandbox.module.exports;
    
    // Set up output collection
    sandbox.__notebookOutputs = outputs;
    sandbox.__cellId = request.cellId;
    
    // Add display function
    sandbox.display = (data: any, type: string = 'auto') => {
      if (type === 'auto') {
        type = this.detectOutputType(data);
      }
      
      outputs.push({
        type: type as any,
        data: this.serializeOutput(data)
      });
    };
  }
  
  private isReactComponent(code: string): boolean {
    // Simple heuristic to detect React components
    return (
      code.includes('React.') ||
      code.includes('export default function') ||
      code.includes('export default class') ||
      code.includes('return <') ||
      code.includes('return (')
    ) && (
      code.includes('</') ||
      code.includes('/>')
    );
  }
  
  private wrapReactComponent(code: string, cellId: string): string {
    // Wrap code to capture React component
    return `
(function() {
  ${code}
  
  // Try to find exported component
  if (typeof module.exports === 'function') {
    return module.exports;
  } else if (module.exports.default) {
    return module.exports.default;
  } else {
    // Look for last defined function
    const matches = ${JSON.stringify(code)}.match(/function\\s+(\\w+)|const\\s+(\\w+)\\s*=/g);
    if (matches && matches.length > 0) {
      const lastMatch = matches[matches.length - 1];
      const name = lastMatch.match(/\\w+$/)[0];
      if (typeof eval(name) === 'function') {
        return eval(name);
      }
    }
  }
})()
    `;
  }
  
  private serializeOutput(value: any): any {
    try {
      // Handle circular references
      const seen = new WeakSet();
      
      return JSON.parse(JSON.stringify(value, (key, val) => {
        if (typeof val === 'object' && val !== null) {
          if (seen.has(val)) {
            return '[Circular]';
          }
          seen.add(val);
        }
        
        // Handle special types
        if (typeof val === 'function') {
          return `[Function: ${val.name || 'anonymous'}]`;
        }
        if (typeof val === 'symbol') {
          return val.toString();
        }
        if (val instanceof Date) {
          return val.toISOString();
        }
        if (val instanceof RegExp) {
          return val.toString();
        }
        
        return val;
      }));
    } catch (error) {
      return String(value);
    }
  }
  
  private detectOutputType(data: any): string {
    if (typeof data === 'string' && data.startsWith('<')) {
      return 'html';
    }
    if (data && typeof data === 'object') {
      if (data.type === 'table' || Array.isArray(data)) {
        return 'data';
      }
    }
    return 'text';
  }
  
  private stringify(value: any): string {
    if (typeof value === 'string') return value;
    if (value === undefined) return 'undefined';
    if (value === null) return 'null';
    
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }
}