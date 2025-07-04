/**
 * Notebook Controller
 * 
 * Main controller for notebook functionality in VS Code
 */

import * as vscode from 'vscode';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { KernelManager } from './kernel-manager';
import { NotebookModel } from './notebook-model';
import { CellExecutor } from './cell-executor';
import { ReactRenderer } from './renderers/react-renderer';
import { TailwindProcessor } from './styling/tailwind-processor';
import { ColorSchemeBuilder } from './design/color-scheme-builder';
import {
  Notebook,
  NotebookCell,
  CellType,
  ExecutionState,
  CellOutput,
  DesignSystem
} from './types';

export interface NotebookControllerConfig {
  kernelManager: KernelManager;
  extensionContext: vscode.ExtensionContext;
  orchestrationEngine?: any;
}

export class NotebookController extends EventEmitter {
  private kernelManager: KernelManager;
  private cellExecutor: CellExecutor;
  private reactRenderer: ReactRenderer;
  private tailwindProcessor: TailwindProcessor;
  private colorSchemeBuilder: ColorSchemeBuilder;
  private notebooks = new Map<string, NotebookModel>();
  private activeNotebook?: NotebookModel;
  private logger = new Logger('NotebookController');
  private disposables: vscode.Disposable[] = [];
  
  constructor(private config: NotebookControllerConfig) {
    super();
    
    this.kernelManager = config.kernelManager;
    
    // Initialize components
    this.cellExecutor = new CellExecutor({
      kernelManager: this.kernelManager
    });
    
    this.reactRenderer = new ReactRenderer({
      enableHotReload: true,
      enableDevTools: true
    });
    
    this.tailwindProcessor = new TailwindProcessor();
    this.colorSchemeBuilder = new ColorSchemeBuilder();
    
    // Set up event listeners
    this.setupEventListeners();
  }
  
  /**
   * Initialize the notebook controller
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing notebook controller...');
    
    // Initialize kernel manager
    await this.kernelManager.initialize();
    
    // Register VS Code providers
    this.registerProviders();
    
    // Register commands
    this.registerCommands();
    
    this.logger.info('Notebook controller initialized');
    this.emit('initialized');
  }
  
  /**
   * Create a new notebook
   */
  async createNotebook(name?: string): Promise<NotebookModel> {
    const notebook = new NotebookModel({
      name: name || 'Untitled Notebook'
    });
    
    // Add to collection
    this.notebooks.set(notebook.getId(), notebook);
    this.activeNotebook = notebook;
    
    // Set up notebook event listeners
    this.setupNotebookListeners(notebook);
    
    // Add default cell
    notebook.addCell(CellType.Code, '// Welcome to SymbioteIDE Notebooks!\n// Start coding here...');
    
    this.emit('notebook-created', notebook.getNotebook());
    
    return notebook;
  }
  
  /**
   * Open an existing notebook
   */
  async openNotebook(uri: vscode.Uri): Promise<NotebookModel> {
    try {
      // Read notebook file
      const content = await vscode.workspace.fs.readFile(uri);
      const json = new TextDecoder().decode(content);
      
      // Load notebook model
      const notebook = NotebookModel.fromJSON(json);
      
      // Add to collection
      this.notebooks.set(notebook.getId(), notebook);
      this.activeNotebook = notebook;
      
      // Set up listeners
      this.setupNotebookListeners(notebook);
      
      this.emit('notebook-opened', notebook.getNotebook());
      
      return notebook;
      
    } catch (error) {
      this.logger.error('Failed to open notebook', error);
      throw error;
    }
  }
  
  /**
   * Save notebook
   */
  async saveNotebook(notebookId: string, uri?: vscode.Uri): Promise<void> {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    try {
      const json = notebook.toJSON();
      const content = new TextEncoder().encode(json);
      
      if (uri) {
        await vscode.workspace.fs.writeFile(uri, content);
      } else {
        // Show save dialog
        const saveUri = await vscode.window.showSaveDialog({
          defaultUri: vscode.Uri.file(`${notebook.getName()}.symbook`),
          filters: {
            'SymbioteIDE Notebook': ['symbook'],
            'Jupyter Notebook': ['ipynb']
          }
        });
        
        if (saveUri) {
          await vscode.workspace.fs.writeFile(saveUri, content);
        }
      }
      
      notebook.markClean();
      this.emit('notebook-saved', notebookId);
      
    } catch (error) {
      this.logger.error('Failed to save notebook', error);
      throw error;
    }
  }
  
  /**
   * Execute a cell
   */
  async executeCell(notebookId: string, cellId: string): Promise<void> {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    const cell = notebook.getCell(cellId);
    if (!cell) {
      throw new Error(`Cell not found: ${cellId}`);
    }
    
    try {
      // Handle different cell types
      switch (cell.type) {
        case CellType.Code:
        case CellType.React:
          await this.executorCodeCell(notebook, cell);
          break;
          
        case CellType.Design:
          await this.executeDesignCell(notebook, cell);
          break;
          
        case CellType.Query:
          await this.executeQueryCell(notebook, cell);
          break;
          
        case CellType.Markdown:
          // Markdown cells don't execute
          break;
      }
      
    } catch (error) {
      this.logger.error(`Failed to execute cell ${cellId}`, error);
      throw error;
    }
  }
  
  /**
   * Execute all cells
   */
  async executeAllCells(notebookId: string): Promise<void> {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    await this.cellExecutor.executeAll(notebook, {
      stopOnError: false
    });
  }
  
  /**
   * Stop execution
   */
  async stopExecution(notebookId: string, cellId?: string): Promise<void> {
    await this.cellExecutor.stopExecution(cellId);
    this.emit('execution-stopped', { notebookId, cellId });
  }
  
  /**
   * Add a new cell
   */
  addCell(
    notebookId: string,
    type: CellType,
    index?: number
  ): NotebookCell {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    const defaultSource = this.getDefaultSourceForType(type);
    return notebook.addCell(type, defaultSource, index);
  }
  
  /**
   * Delete a cell
   */
  deleteCell(notebookId: string, cellId: string): void {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    notebook.deleteCell(cellId);
  }
  
  /**
   * Update cell source
   */
  updateCellSource(notebookId: string, cellId: string, source: string): void {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    notebook.updateCellSource(cellId, source);
  }
  
  /**
   * Get active notebook
   */
  getActiveNotebook(): NotebookModel | undefined {
    return this.activeNotebook;
  }
  
  /**
   * Set active notebook
   */
  setActiveNotebook(notebookId: string): void {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    this.activeNotebook = notebook;
    this.emit('active-notebook-changed', notebookId);
  }
  
  /**
   * Get all notebooks
   */
  getAllNotebooks(): NotebookModel[] {
    return Array.from(this.notebooks.values());
  }
  
  /**
   * Create design system
   */
  async createDesignSystem(
    notebookId: string,
    baseColor?: string
  ): Promise<DesignSystem> {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    // Create color scheme
    const colorScheme = await this.colorSchemeBuilder.createFromBaseColor(
      baseColor || '#3b82f6'
    );
    
    // Create design system
    const designSystem: DesignSystem = {
      name: `${notebook.getName()} Design System`,
      version: '1.0.0',
      colors: colorScheme,
      typography: this.getDefaultTypography(),
      spacing: this.getDefaultSpacing(),
      shadows: this.getDefaultShadows(),
      tokens: {
        colors: {},
        typography: {},
        spacing: {},
        shadows: {},
        borders: {},
        radii: {},
        zIndices: {},
        transitions: {}
      }
    };
    
    // Update notebook
    notebook.setDesignSystem(designSystem);
    
    this.emit('design-system-created', { notebookId, designSystem });
    
    return designSystem;
  }
  
  /**
   * Export notebook
   */
  async exportNotebook(
    notebookId: string,
    format: 'html' | 'pdf' | 'markdown' | 'app'
  ): Promise<void> {
    const notebook = this.notebooks.get(notebookId);
    if (!notebook) {
      throw new Error(`Notebook not found: ${notebookId}`);
    }
    
    // TODO: Implement export functionality
    this.logger.info(`Exporting notebook ${notebookId} as ${format}`);
  }
  
  /**
   * Dispose controller
   */
  dispose(): void {
    // Dispose all notebooks
    this.notebooks.forEach(notebook => {
      notebook.removeAllListeners();
    });
    this.notebooks.clear();
    
    // Dispose components
    this.reactRenderer.dispose();
    this.tailwindProcessor.clearCache();
    
    // Dispose VS Code resources
    this.disposables.forEach(d => d.dispose());
    this.disposables = [];
    
    // Remove listeners
    this.removeAllListeners();
  }
  
  // Private methods
  
  private registerProviders(): void {
    // Register notebook serializer
    const serializer = vscode.workspace.registerNotebookSerializer(
      'symbiote-notebook',
      {
        deserializeNotebook: async (content: Uint8Array) => {
          const json = new TextDecoder().decode(content);
          const data = JSON.parse(json);
          
          const cells = data.cells.map((cell: any) => {
            return new vscode.NotebookCellData(
              cell.type === CellType.Markdown ? 
                vscode.NotebookCellKind.Markup : 
                vscode.NotebookCellKind.Code,
              cell.source,
              cell.language || 'javascript'
            );
          });
          
          return new vscode.NotebookData(cells);
        },
        
        serializeNotebook: async (data: vscode.NotebookData) => {
          const notebook = {
            cells: data.cells.map(cell => ({
              type: cell.kind === vscode.NotebookCellKind.Markup ? 
                CellType.Markdown : 
                CellType.Code,
              source: cell.value,
              language: cell.languageId
            }))
          };
          
          return new TextEncoder().encode(JSON.stringify(notebook, null, 2));
        }
      }
    );
    
    this.disposables.push(serializer);
  }
  
  private registerCommands(): void {
    // Execute cell
    const executeCell = vscode.commands.registerCommand(
      'symbiote.notebook.executeCell',
      async () => {
        const editor = vscode.window.activeNotebookEditor;
        if (!editor) return;
        
        const cell = editor.selection.start;
        // TODO: Map VS Code cell to our cell model
      }
    );
    
    // Execute all cells
    const executeAll = vscode.commands.registerCommand(
      'symbiote.notebook.executeAll',
      async () => {
        if (this.activeNotebook) {
          await this.executeAllCells(this.activeNotebook.getId());
        }
      }
    );
    
    // Add cell
    const addCell = vscode.commands.registerCommand(
      'symbiote.notebook.addCell',
      async (type?: CellType) => {
        if (this.activeNotebook) {
          this.addCell(
            this.activeNotebook.getId(),
            type || CellType.Code
          );
        }
      }
    );
    
    // AI assist
    const aiAssist = vscode.commands.registerCommand(
      'symbiote.notebook.aiAssist',
      async () => {
        // TODO: Implement AI assistance
        vscode.window.showInformationMessage('AI assistance coming soon!');
      }
    );
    
    this.disposables.push(executeCell, executeAll, addCell, aiAssist);
  }
  
  private setupEventListeners(): void {
    // Cell executor events
    this.cellExecutor.on('cell-executed', (data) => {
      this.emit('cell-executed', data);
    });
    
    this.cellExecutor.on('cell-execution-error', (data) => {
      this.emit('cell-execution-error', data);
    });
    
    // Kernel manager events
    this.kernelManager.on('kernel-status-changed', (data) => {
      this.emit('kernel-status-changed', data);
    });
  }
  
  private setupNotebookListeners(notebook: NotebookModel): void {
    notebook.on('cell-added', (data) => {
      this.emit('cell-added', { notebookId: notebook.getId(), ...data });
    });
    
    notebook.on('cell-deleted', (data) => {
      this.emit('cell-deleted', { notebookId: notebook.getId(), ...data });
    });
    
    notebook.on('cell-updated', (data) => {
      this.emit('cell-updated', { notebookId: notebook.getId(), ...data });
    });
    
    notebook.on('dirty-state-changed', (isDirty) => {
      this.emit('notebook-dirty-state-changed', {
        notebookId: notebook.getId(),
        isDirty
      });
    });
  }
  
  private async executorCodeCell(
    notebook: NotebookModel,
    cell: NotebookCell
  ): Promise<void> {
    // Execute with cell executor
    const result = await this.cellExecutor.executeCell(notebook, cell.id);
    
    // Handle React components
    if (cell.type === CellType.React && result.outputs.some(o => o.type === 'react')) {
      const reactOutput = result.outputs.find(o => o.type === 'react');
      if (reactOutput) {
        await this.reactRenderer.renderComponent(
          cell.id,
          reactOutput,
          this.config.extensionContext
        );
      }
    }
  }
  
  private async executeDesignCell(
    notebook: NotebookModel,
    cell: NotebookCell
  ): Promise<void> {
    // Parse design cell content
    try {
      const designData = JSON.parse(cell.source);
      
      if (designData.type === 'colorScheme') {
        const scheme = await this.colorSchemeBuilder.createFromBaseColor(
          designData.baseColor
        );
        
        const output: CellOutput = {
          type: 'data',
          data: scheme
        };
        
        notebook.setCellOutputs(cell.id, [output]);
        notebook.setCellExecutionState(cell.id, ExecutionState.Success);
      }
    } catch (error) {
      notebook.setCellOutputs(cell.id, [{
        type: 'error',
        data: { message: 'Invalid design cell content' }
      }]);
      notebook.setCellExecutionState(cell.id, ExecutionState.Error);
    }
  }
  
  private async executeQueryCell(
    notebook: NotebookModel,
    cell: NotebookCell
  ): Promise<void> {
    // TODO: Implement query execution
    notebook.setCellOutputs(cell.id, [{
      type: 'text',
      data: 'Query execution not yet implemented'
    }]);
    notebook.setCellExecutionState(cell.id, ExecutionState.Success);
  }
  
  private getDefaultSourceForType(type: CellType): string {
    switch (type) {
      case CellType.Code:
        return '// JavaScript code\n';
        
      case CellType.React:
        return `import React from 'react';\n\nexport default function Component() {\n  return (\n    <div className="p-4">\n      <h1 className="text-2xl font-bold">Hello, React!</h1>\n    </div>\n  );\n}`;
        
      case CellType.Design:
        return `{\n  "type": "colorScheme",\n  "baseColor": "#3b82f6"\n}`;
        
      case CellType.Query:
        return '-- SQL query\nSELECT * FROM table;';
        
      case CellType.Markdown:
        return '# Markdown Cell\n\nType your notes here...';
        
      default:
        return '';
    }
  }
  
  private getDefaultTypography(): any {
    return {
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        serif: ['Georgia', 'Cambria', 'serif'],
        mono: ['Fira Code', 'Consolas', 'monospace']
      },
      fontSize: {
        xs: '0.75rem',
        sm: '0.875rem',
        base: '1rem',
        lg: '1.125rem',
        xl: '1.25rem',
        '2xl': '1.5rem',
        '3xl': '1.875rem',
        '4xl': '2.25rem'
      },
      fontWeight: {
        thin: 100,
        light: 300,
        normal: 400,
        medium: 500,
        semibold: 600,
        bold: 700,
        extrabold: 800
      },
      lineHeight: {
        none: 1,
        tight: 1.25,
        snug: 1.375,
        normal: 1.5,
        relaxed: 1.625,
        loose: 2
      },
      letterSpacing: {
        tighter: '-0.05em',
        tight: '-0.025em',
        normal: '0em',
        wide: '0.025em',
        wider: '0.05em',
        widest: '0.1em'
      }
    };
  }
  
  private getDefaultSpacing(): any {
    return {
      '0': '0px',
      'px': '1px',
      '0.5': '0.125rem',
      '1': '0.25rem',
      '2': '0.5rem',
      '3': '0.75rem',
      '4': '1rem',
      '5': '1.25rem',
      '6': '1.5rem',
      '8': '2rem',
      '10': '2.5rem',
      '12': '3rem',
      '16': '4rem',
      '20': '5rem',
      '24': '6rem',
      '32': '8rem'
    };
  }
  
  private getDefaultShadows(): any {
    return {
      sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
      DEFAULT: '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
      md: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
      lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
      xl: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
      '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
      inner: 'inset 0 2px 4px 0 rgba(0, 0, 0, 0.06)',
      none: 'none'
    };
  }
}