/**
 * Cell Executor
 * 
 * Handles execution of notebook cells with dependency tracking and caching
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { KernelManager } from './kernel-manager';
import { NotebookModel } from './notebook-model';
import {
  NotebookCell,
  CellType,
  ExecutionState,
  ExecuteRequest,
  ExecuteResult,
  CellOutput,
  CellLanguage
} from './types';

export interface CellExecutorConfig {
  kernelManager: KernelManager;
  enableCaching?: boolean;
  maxConcurrentExecutions?: number;
  defaultTimeout?: number;
}

interface ExecutionContext {
  cellId: string;
  dependencies: string[];
  exports: any;
  isExecuting: boolean;
  lastResult?: ExecuteResult;
  lastExecutionTime?: number;
}

export class CellExecutor extends EventEmitter {
  private kernelManager: KernelManager;
  private config: Required<CellExecutorConfig>;
  private logger = new Logger('CellExecutor');
  private executionContexts = new Map<string, ExecutionContext>();
  private executionQueue: string[] = [];
  private currentExecutions = 0;
  
  constructor(config: CellExecutorConfig) {
    super();
    
    this.kernelManager = config.kernelManager;
    this.config = {
      kernelManager: config.kernelManager,
      enableCaching: config.enableCaching ?? true,
      maxConcurrentExecutions: config.maxConcurrentExecutions ?? 1,
      defaultTimeout: config.defaultTimeout ?? 30000
    };
  }
  
  /**
   * Execute a cell
   */
  async executeCell(
    notebookModel: NotebookModel,
    cellId: string,
    options: {
      skipDependencies?: boolean;
      forceRerun?: boolean;
    } = {}
  ): Promise<ExecuteResult> {
    const cell = notebookModel.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    // Skip non-code cells
    if (cell.type === CellType.Markdown) {
      return {
        cellId,
        executionCount: 0,
        outputs: [],
        status: 'ok'
      };
    }
    
    // Check cache
    if (this.config.enableCaching && !options.forceRerun) {
      const cached = this.getCachedResult(cellId, cell);
      if (cached) {
        this.logger.info(`Using cached result for cell ${cellId}`);
        return cached;
      }
    }
    
    // Execute dependencies first
    if (!options.skipDependencies) {
      await this.executeDependencies(notebookModel, cellId);
    }
    
    // Queue execution
    return this.queueExecution(notebookModel, cell);
  }
  
  /**
   * Execute all cells
   */
  async executeAll(
    notebookModel: NotebookModel,
    options: {
      stopOnError?: boolean;
      fromCell?: string;
    } = {}
  ): Promise<Map<string, ExecuteResult>> {
    const cells = notebookModel.getCells();
    const results = new Map<string, ExecuteResult>();
    
    let startIndex = 0;
    if (options.fromCell) {
      startIndex = cells.findIndex(c => c.id === options.fromCell);
      if (startIndex === -1) startIndex = 0;
    }
    
    for (let i = startIndex; i < cells.length; i++) {
      const cell = cells[i];
      
      try {
        const result = await this.executeCell(notebookModel, cell.id, {
          skipDependencies: false,
          forceRerun: true
        });
        
        results.set(cell.id, result);
        
        if (options.stopOnError && result.status === 'error') {
          break;
        }
      } catch (error) {
        this.logger.error(`Failed to execute cell ${cell.id}`, error);
        
        if (options.stopOnError) {
          break;
        }
      }
    }
    
    return results;
  }
  
  /**
   * Stop execution
   */
  async stopExecution(cellId?: string): Promise<void> {
    if (cellId) {
      // Stop specific cell
      const context = this.executionContexts.get(cellId);
      if (context && context.isExecuting) {
        await this.kernelManager.interruptKernel();
        context.isExecuting = false;
        this.emit('execution-stopped', { cellId });
      }
    } else {
      // Stop all executions
      await this.kernelManager.interruptKernel();
      this.executionQueue = [];
      this.executionContexts.forEach(context => {
        context.isExecuting = false;
      });
      this.emit('all-executions-stopped');
    }
  }
  
  /**
   * Clear execution cache
   */
  clearCache(cellId?: string): void {
    if (cellId) {
      this.executionContexts.delete(cellId);
    } else {
      this.executionContexts.clear();
    }
    this.emit('cache-cleared', { cellId });
  }
  
  /**
   * Get cell dependencies
   */
  getCellDependencies(
    notebookModel: NotebookModel,
    cellId: string
  ): string[] {
    const cell = notebookModel.getCell(cellId);
    if (!cell) return [];
    
    // Get explicit dependencies from metadata
    if (cell.metadata.dependencies) {
      return cell.metadata.dependencies;
    }
    
    // Auto-detect dependencies based on imports/requires
    const dependencies: string[] = [];
    const cells = notebookModel.getCells();
    const cellIndex = cells.findIndex(c => c.id === cellId);
    
    // Simple heuristic: previous code cells are dependencies
    for (let i = 0; i < cellIndex; i++) {
      const prevCell = cells[i];
      if (prevCell.type === CellType.Code || prevCell.type === CellType.React) {
        // Check if current cell references exports from previous cell
        if (this.detectDependency(prevCell.source, cell.source)) {
          dependencies.push(prevCell.id);
        }
      }
    }
    
    return dependencies;
  }
  
  // Private methods
  
  private async queueExecution(
    notebookModel: NotebookModel,
    cell: NotebookCell
  ): Promise<ExecuteResult> {
    return new Promise((resolve, reject) => {
      const execute = async () => {
        try {
          const result = await this.doExecuteCell(notebookModel, cell);
          resolve(result);
        } catch (error) {
          reject(error);
        }
      };
      
      if (this.currentExecutions < this.config.maxConcurrentExecutions) {
        this.currentExecutions++;
        execute();
      } else {
        this.executionQueue.push(cell.id);
        this.once(`execution-slot-available`, () => {
          if (this.executionQueue[0] === cell.id) {
            this.executionQueue.shift();
            this.currentExecutions++;
            execute();
          }
        });
      }
    });
  }
  
  private async doExecuteCell(
    notebookModel: NotebookModel,
    cell: NotebookCell
  ): Promise<ExecuteResult> {
    try {
      // Update execution state
      notebookModel.setCellExecutionState(cell.id, ExecutionState.Running);
      notebookModel.clearCellOutputs(cell.id);
      
      // Get or create execution context
      let context = this.executionContexts.get(cell.id);
      if (!context) {
        context = {
          cellId: cell.id,
          dependencies: this.getCellDependencies(notebookModel, cell.id),
          exports: {},
          isExecuting: false
        };
        this.executionContexts.set(cell.id, context);
      }
      
      context.isExecuting = true;
      
      // Prepare code with dependency injection
      const preparedCode = this.prepareCode(cell, context);
      
      // Create execute request
      const request: ExecuteRequest = {
        cellId: cell.id,
        code: preparedCode,
        language: cell.language || this.getLanguageForCellType(cell.type),
        metadata: cell.metadata
      };
      
      // Execute in kernel
      const result = await this.kernelManager.execute(request);
      
      // Update cell with results
      notebookModel.setCellOutputs(cell.id, result.outputs);
      notebookModel.setCellExecutionCount(cell.id, result.executionCount);
      notebookModel.setCellExecutionState(
        cell.id,
        result.status === 'ok' ? ExecutionState.Success : ExecutionState.Error
      );
      
      // Update context
      context.isExecuting = false;
      context.lastResult = result;
      context.lastExecutionTime = Date.now();
      
      // Extract exports if any
      if (result.status === 'ok') {
        this.extractExports(cell, result, context);
      }
      
      this.emit('cell-executed', { cellId: cell.id, result });
      
      return result;
      
    } catch (error) {
      // Update cell state on error
      notebookModel.setCellExecutionState(cell.id, ExecutionState.Error);
      
      const errorResult: ExecuteResult = {
        cellId: cell.id,
        executionCount: 0,
        outputs: [{
          type: 'error',
          data: {
            name: 'ExecutionError',
            message: error instanceof Error ? error.message : String(error)
          }
        }],
        status: 'error'
      };
      
      notebookModel.setCellOutputs(cell.id, errorResult.outputs);
      
      this.emit('cell-execution-error', { cellId: cell.id, error });
      
      throw error;
      
    } finally {
      this.currentExecutions--;
      if (this.executionQueue.length > 0) {
        this.emit('execution-slot-available');
      }
    }
  }
  
  private async executeDependencies(
    notebookModel: NotebookModel,
    cellId: string
  ): Promise<void> {
    const dependencies = this.getCellDependencies(notebookModel, cellId);
    
    for (const depId of dependencies) {
      const depContext = this.executionContexts.get(depId);
      
      // Skip if already executed recently
      if (depContext?.lastResult && depContext.lastResult.status === 'ok') {
        const age = Date.now() - (depContext.lastExecutionTime || 0);
        if (age < 60000) { // Less than 1 minute old
          continue;
        }
      }
      
      // Execute dependency
      await this.executeCell(notebookModel, depId, {
        skipDependencies: false,
        forceRerun: false
      });
    }
  }
  
  private prepareCode(cell: NotebookCell, context: ExecutionContext): string {
    let code = cell.source;
    
    // Inject dependencies
    if (context.dependencies.length > 0) {
      const imports: string[] = [];
      
      for (const depId of context.dependencies) {
        const depContext = this.executionContexts.get(depId);
        if (depContext && depContext.exports) {
          // Make dependency exports available
          imports.push(`// Imports from cell ${depId}`);
          for (const [key, value] of Object.entries(depContext.exports)) {
            imports.push(`const ${key} = __cellExports['${depId}']['${key}'];`);
          }
        }
      }
      
      if (imports.length > 0) {
        code = imports.join('\n') + '\n\n' + code;
      }
    }
    
    // Wrap code to capture exports
    return `
(function() {
  const __cellExports = arguments[0] || {};
  const __cellId = '${cell.id}';
  let module = { exports: {} };
  let exports = module.exports;
  
  ${code}
  
  // Capture exports
  __cellExports[__cellId] = module.exports;
  
  return module.exports;
})(globalThis.__notebookExports || (globalThis.__notebookExports = {}));
    `;
  }
  
  private extractExports(
    cell: NotebookCell,
    result: ExecuteResult,
    context: ExecutionContext
  ): void {
    // Try to extract exports from the execution result
    // This is a simplified version - real implementation would be more sophisticated
    if (result.outputs.length > 0) {
      const lastOutput = result.outputs[result.outputs.length - 1];
      if (lastOutput.type === 'data' && lastOutput.data) {
        context.exports = lastOutput.data;
      }
    }
  }
  
  private getCachedResult(cellId: string, cell: NotebookCell): ExecuteResult | null {
    const context = this.executionContexts.get(cellId);
    
    if (!context || !context.lastResult) {
      return null;
    }
    
    // Check if cell source has changed
    const currentHash = this.hashCode(cell.source);
    const cachedHash = this.hashCode(context.lastResult.cellId); // Simplified
    
    if (currentHash !== cachedHash) {
      return null;
    }
    
    // Check cache age
    const age = Date.now() - (context.lastExecutionTime || 0);
    if (age > 300000) { // 5 minutes
      return null;
    }
    
    return context.lastResult;
  }
  
  private detectDependency(prevCellSource: string, currentCellSource: string): boolean {
    // Simple heuristic: check if current cell references variables/functions from previous
    // This is a very basic implementation
    
    // Extract potential exports from previous cell
    const exportPattern = /export\s+(?:const|let|var|function|class)\s+(\w+)/g;
    const moduleExportPattern = /module\.exports\.(\w+)|exports\.(\w+)/g;
    
    const exports = new Set<string>();
    
    let match;
    while ((match = exportPattern.exec(prevCellSource)) !== null) {
      exports.add(match[1]);
    }
    while ((match = moduleExportPattern.exec(prevCellSource)) !== null) {
      exports.add(match[1] || match[2]);
    }
    
    // Check if current cell uses any of these exports
    for (const exportName of exports) {
      if (currentCellSource.includes(exportName)) {
        return true;
      }
    }
    
    return false;
  }
  
  private getLanguageForCellType(type: CellType): CellLanguage {
    switch (type) {
      case CellType.React:
        return CellLanguage.TypeScript;
      case CellType.Query:
        return CellLanguage.SQL;
      default:
        return CellLanguage.JavaScript;
    }
  }
  
  private hashCode(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return hash;
  }
}