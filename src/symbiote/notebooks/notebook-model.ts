/**
 * Notebook Model
 * 
 * Manages notebook document structure and operations
 */

import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import {
  Notebook,
  NotebookCell,
  NotebookMetadata,
  CellType,
  CellLanguage,
  ExecutionState,
  CellOutput,
  CellMetadata,
  KernelSpec,
  DesignSystem
} from './types';

export interface NotebookModelOptions {
  id?: string;
  name?: string;
  metadata?: NotebookMetadata;
  kernelSpec?: KernelSpec;
}

export class NotebookModel extends EventEmitter {
  private notebook: Notebook;
  private cellOrder: string[] = [];
  private isDirty = false;
  
  constructor(options: NotebookModelOptions = {}) {
    super();
    
    this.notebook = {
      id: options.id || uuidv4(),
      name: options.name || 'Untitled Notebook',
      cells: [],
      metadata: options.metadata || {},
      kernelSpec: options.kernelSpec || this.getDefaultKernelSpec(),
      createdAt: new Date(),
      updatedAt: new Date(),
      version: '1.0.0'
    };
  }
  
  /**
   * Get notebook data
   */
  getNotebook(): Notebook {
    return {
      ...this.notebook,
      cells: [...this.notebook.cells]
    };
  }
  
  /**
   * Get notebook ID
   */
  getId(): string {
    return this.notebook.id;
  }
  
  /**
   * Get notebook name
   */
  getName(): string {
    return this.notebook.name;
  }
  
  /**
   * Set notebook name
   */
  setName(name: string): void {
    this.notebook.name = name;
    this.markDirty();
    this.emit('name-changed', name);
  }
  
  /**
   * Get metadata
   */
  getMetadata(): NotebookMetadata {
    return { ...this.notebook.metadata };
  }
  
  /**
   * Update metadata
   */
  updateMetadata(metadata: Partial<NotebookMetadata>): void {
    this.notebook.metadata = {
      ...this.notebook.metadata,
      ...metadata
    };
    this.markDirty();
    this.emit('metadata-updated', this.notebook.metadata);
  }
  
  /**
   * Get design system
   */
  getDesignSystem(): DesignSystem | undefined {
    return this.notebook.metadata.designSystem;
  }
  
  /**
   * Set design system
   */
  setDesignSystem(designSystem: DesignSystem): void {
    this.notebook.metadata.designSystem = designSystem;
    this.markDirty();
    this.emit('design-system-updated', designSystem);
  }
  
  /**
   * Get all cells
   */
  getCells(): NotebookCell[] {
    return [...this.notebook.cells];
  }
  
  /**
   * Get cell by ID
   */
  getCell(cellId: string): NotebookCell | undefined {
    return this.notebook.cells.find(cell => cell.id === cellId);
  }
  
  /**
   * Get cell index
   */
  getCellIndex(cellId: string): number {
    return this.notebook.cells.findIndex(cell => cell.id === cellId);
  }
  
  /**
   * Add a new cell
   */
  addCell(
    type: CellType,
    source: string = '',
    index?: number,
    metadata?: CellMetadata
  ): NotebookCell {
    const cell: NotebookCell = {
      id: uuidv4(),
      type,
      language: this.getDefaultLanguageForType(type),
      source,
      outputs: [],
      metadata: metadata || {},
      executionState: ExecutionState.Idle,
      createdAt: new Date(),
      updatedAt: new Date()
    };
    
    if (index !== undefined && index >= 0 && index <= this.notebook.cells.length) {
      this.notebook.cells.splice(index, 0, cell);
    } else {
      this.notebook.cells.push(cell);
    }
    
    this.markDirty();
    this.emit('cell-added', { cell, index: index ?? this.notebook.cells.length - 1 });
    
    return cell;
  }
  
  /**
   * Update cell source
   */
  updateCellSource(cellId: string, source: string): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.source = source;
    cell.updatedAt = new Date();
    
    this.markDirty();
    this.emit('cell-updated', { cellId, changes: { source } });
  }
  
  /**
   * Update cell type
   */
  updateCellType(cellId: string, type: CellType): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.type = type;
    cell.language = this.getDefaultLanguageForType(type);
    cell.updatedAt = new Date();
    
    // Clear outputs when changing type
    cell.outputs = [];
    cell.executionState = ExecutionState.Idle;
    
    this.markDirty();
    this.emit('cell-updated', { cellId, changes: { type } });
  }
  
  /**
   * Update cell metadata
   */
  updateCellMetadata(cellId: string, metadata: Partial<CellMetadata>): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.metadata = {
      ...cell.metadata,
      ...metadata
    };
    cell.updatedAt = new Date();
    
    this.markDirty();
    this.emit('cell-updated', { cellId, changes: { metadata } });
  }
  
  /**
   * Set cell outputs
   */
  setCellOutputs(cellId: string, outputs: CellOutput[]): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.outputs = outputs;
    cell.updatedAt = new Date();
    
    this.markDirty();
    this.emit('cell-outputs-updated', { cellId, outputs });
  }
  
  /**
   * Append cell output
   */
  appendCellOutput(cellId: string, output: CellOutput): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.outputs.push(output);
    cell.updatedAt = new Date();
    
    this.markDirty();
    this.emit('cell-output-appended', { cellId, output });
  }
  
  /**
   * Clear cell outputs
   */
  clearCellOutputs(cellId: string): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.outputs = [];
    cell.executionState = ExecutionState.Idle;
    cell.executionCount = undefined;
    cell.updatedAt = new Date();
    
    this.markDirty();
    this.emit('cell-outputs-cleared', { cellId });
  }
  
  /**
   * Set cell execution state
   */
  setCellExecutionState(cellId: string, state: ExecutionState): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.executionState = state;
    cell.updatedAt = new Date();
    
    this.emit('cell-execution-state-changed', { cellId, state });
  }
  
  /**
   * Set cell execution count
   */
  setCellExecutionCount(cellId: string, count: number): void {
    const cell = this.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    cell.executionCount = count;
    cell.updatedAt = new Date();
    
    this.emit('cell-execution-count-changed', { cellId, count });
  }
  
  /**
   * Delete a cell
   */
  deleteCell(cellId: string): void {
    const index = this.getCellIndex(cellId);
    if (index === -1) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    this.notebook.cells.splice(index, 1);
    
    this.markDirty();
    this.emit('cell-deleted', { cellId, index });
  }
  
  /**
   * Move a cell
   */
  moveCell(cellId: string, newIndex: number): void {
    const currentIndex = this.getCellIndex(cellId);
    if (currentIndex === -1) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    if (newIndex < 0 || newIndex >= this.notebook.cells.length) {
      throw new Error(`Invalid index: ${newIndex}`);
    }
    
    if (currentIndex === newIndex) {
      return;
    }
    
    const [cell] = this.notebook.cells.splice(currentIndex, 1);
    this.notebook.cells.splice(newIndex, 0, cell);
    
    this.markDirty();
    this.emit('cell-moved', { cellId, fromIndex: currentIndex, toIndex: newIndex });
  }
  
  /**
   * Clear all cells
   */
  clearAllCells(): void {
    this.notebook.cells = [];
    this.markDirty();
    this.emit('all-cells-cleared');
  }
  
  /**
   * Clear all outputs
   */
  clearAllOutputs(): void {
    this.notebook.cells.forEach(cell => {
      cell.outputs = [];
      cell.executionState = ExecutionState.Idle;
      cell.executionCount = undefined;
    });
    
    this.markDirty();
    this.emit('all-outputs-cleared');
  }
  
  /**
   * Check if notebook is dirty
   */
  isDirtyState(): boolean {
    return this.isDirty;
  }
  
  /**
   * Mark notebook as clean
   */
  markClean(): void {
    this.isDirty = false;
    this.emit('dirty-state-changed', false);
  }
  
  /**
   * Serialize notebook to JSON
   */
  toJSON(): string {
    return JSON.stringify(this.notebook, null, 2);
  }
  
  /**
   * Load notebook from JSON
   */
  static fromJSON(json: string): NotebookModel {
    const data = JSON.parse(json);
    const model = new NotebookModel({
      id: data.id,
      name: data.name,
      metadata: data.metadata,
      kernelSpec: data.kernelSpec
    });
    
    // Restore cells
    data.cells.forEach((cellData: NotebookCell) => {
      const cell = {
        ...cellData,
        createdAt: new Date(cellData.createdAt),
        updatedAt: new Date(cellData.updatedAt)
      };
      model.notebook.cells.push(cell);
    });
    
    // Restore dates
    model.notebook.createdAt = new Date(data.createdAt);
    model.notebook.updatedAt = new Date(data.updatedAt);
    model.notebook.version = data.version;
    
    return model;
  }
  
  /**
   * Clone the notebook
   */
  clone(): NotebookModel {
    return NotebookModel.fromJSON(this.toJSON());
  }
  
  // Private methods
  
  private markDirty(): void {
    this.isDirty = true;
    this.notebook.updatedAt = new Date();
    this.emit('dirty-state-changed', true);
  }
  
  private getDefaultLanguageForType(type: CellType): CellLanguage | undefined {
    switch (type) {
      case CellType.Code:
        return CellLanguage.JavaScript;
      case CellType.React:
        return CellLanguage.TypeScript;
      case CellType.Query:
        return CellLanguage.SQL;
      default:
        return undefined;
    }
  }
  
  private getDefaultKernelSpec(): KernelSpec {
    return {
      name: 'javascript',
      displayName: 'JavaScript',
      language: CellLanguage.JavaScript,
      argv: ['node']
    };
  }
}