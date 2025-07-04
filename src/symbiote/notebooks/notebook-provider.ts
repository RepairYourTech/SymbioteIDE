/**
 * Notebook Provider
 * 
 * VS Code notebook integration for SymbioteIDE
 */

import * as vscode from 'vscode';
import { NotebookController } from './notebook-controller';
import { KernelManager } from './kernel-manager';
import { CellType } from './types';

export class NotebookProvider implements 
  vscode.NotebookCellStatusBarItemProvider,
  vscode.NotebookContentProvider {
  
  private notebookController: NotebookController;
  
  constructor(
    private context: vscode.ExtensionContext,
    private orchestrationEngine: any
  ) {
    // Initialize kernel manager
    const kernelManager = new KernelManager();
    
    // Initialize notebook controller
    this.notebookController = new NotebookController({
      kernelManager,
      extensionContext: context,
      orchestrationEngine
    });
  }
  
  /**
   * Initialize the provider
   */
  async initialize(): Promise<void> {
    await this.notebookController.initialize();
    
    // Register notebook type
    const notebookType = 'symbiote-notebook';
    
    // Register content provider
    vscode.workspace.registerNotebookContentProvider(notebookType, this);
    
    // Register commands
    this.registerCommands();
  }
  
  /**
   * Provide cell status bar items
   */
  provideCellStatusBarItems(
    cell: vscode.NotebookCell,
    token: vscode.CancellationToken
  ): vscode.NotebookCellStatusBarItem[] | undefined {
    const items: vscode.NotebookCellStatusBarItem[] = [];
    
    // Execute button
    items.push({
      text: '$(play) Run',
      tooltip: 'Execute this cell',
      command: 'symbiote.notebook.executeCell',
      alignment: vscode.NotebookCellStatusBarAlignment.Left,
      priority: 100
    });
    
    // AI assistance button
    if (cell.document.languageId === 'javascript' || 
        cell.document.languageId === 'typescript' ||
        cell.document.languageId === 'python') {
      items.push({
        text: '$(zap) AI Assist',
        tooltip: 'Get AI assistance for this cell',
        command: 'symbiote.notebook.aiAssist',
        alignment: vscode.NotebookCellStatusBarAlignment.Right,
        priority: 90
      });
    }
    
    // React preview button
    if (cell.metadata?.cellType === 'react') {
      items.push({
        text: '$(eye) Preview',
        tooltip: 'Preview React component',
        command: 'symbiote.notebook.previewReact',
        alignment: vscode.NotebookCellStatusBarAlignment.Right,
        priority: 80
      });
    }
    
    // Design tools button
    if (cell.metadata?.cellType === 'design') {
      items.push({
        text: '$(symbol-color) Design',
        tooltip: 'Open design tools',
        command: 'symbiote.notebook.openDesignTools',
        alignment: vscode.NotebookCellStatusBarAlignment.Right,
        priority: 80
      });
    }
    
    return items;
  }
  
  /**
   * Open notebook
   */
  async openNotebook(
    uri: vscode.Uri,
    openContext: vscode.NotebookDocumentOpenContext,
    token: vscode.CancellationToken
  ): Promise<vscode.NotebookData> {
    // Load notebook through controller
    const notebookModel = await this.notebookController.openNotebook(uri);
    const notebook = notebookModel.getNotebook();
    
    // Convert to VS Code notebook data
    const cells = notebook.cells.map(cell => {
      const cellData = new vscode.NotebookCellData(
        cell.type === CellType.Markdown ? 
          vscode.NotebookCellKind.Markup : 
          vscode.NotebookCellKind.Code,
        cell.source,
        cell.language || 'javascript'
      );
      
      // Add metadata
      cellData.metadata = {
        cellType: cell.type,
        ...cell.metadata
      };
      
      return cellData;
    });
    
    return new vscode.NotebookData(cells);
  }
  
  /**
   * Save notebook
   */
  async saveNotebook(
    document: vscode.NotebookDocument,
    token: vscode.CancellationToken
  ): Promise<void> {
    // Find corresponding notebook model
    const notebooks = this.notebookController.getAllNotebooks();
    const notebookModel = notebooks.find(n => 
      n.getName() === document.uri.path.split('/').pop()?.replace('.symbook', '')
    );
    
    if (notebookModel) {
      await this.notebookController.saveNotebook(
        notebookModel.getId(),
        document.uri
      );
    }
  }
  
  /**
   * Save notebook as
   */
  async saveNotebookAs(
    targetResource: vscode.Uri,
    document: vscode.NotebookDocument,
    token: vscode.CancellationToken
  ): Promise<void> {
    await this.saveNotebook(document, token);
  }
  
  /**
   * Backup notebook
   */
  async backupNotebook(
    document: vscode.NotebookDocument,
    context: vscode.NotebookDocumentBackupContext,
    token: vscode.CancellationToken
  ): Promise<vscode.NotebookDocumentBackup> {
    // Create backup
    const backup: vscode.NotebookDocumentBackup = {
      id: context.destination.toString(),
      delete: () => {
        // Cleanup backup
      }
    };
    
    return backup;
  }
  
  /**
   * Register commands
   */
  private registerCommands(): void {
    // Create new notebook
    vscode.commands.registerCommand('symbiote.notebook.new', async () => {
      const notebook = await this.notebookController.createNotebook();
      
      // Create VS Code document
      const doc = await vscode.workspace.openNotebookDocument(
        'symbiote-notebook',
        new vscode.NotebookData([
          new vscode.NotebookCellData(
            vscode.NotebookCellKind.Code,
            '// Welcome to SymbioteIDE Notebooks!',
            'javascript'
          )
        ])
      );
      
      await vscode.window.showNotebookDocument(doc);
    });
    
    // Add React cell
    vscode.commands.registerCommand('symbiote.notebook.addReactCell', () => {
      const editor = vscode.window.activeNotebookEditor;
      if (!editor) return;
      
      const notebook = this.notebookController.getActiveNotebook();
      if (!notebook) return;
      
      this.notebookController.addCell(
        notebook.getId(),
        CellType.React,
        editor.selection.end
      );
    });
    
    // Add design cell
    vscode.commands.registerCommand('symbiote.notebook.addDesignCell', () => {
      const editor = vscode.window.activeNotebookEditor;
      if (!editor) return;
      
      const notebook = this.notebookController.getActiveNotebook();
      if (!notebook) return;
      
      this.notebookController.addCell(
        notebook.getId(),
        CellType.Design,
        editor.selection.end
      );
    });
    
    // Create design system
    vscode.commands.registerCommand('symbiote.notebook.createDesignSystem', async () => {
      const notebook = this.notebookController.getActiveNotebook();
      if (!notebook) {
        vscode.window.showErrorMessage('No active notebook');
        return;
      }
      
      const baseColor = await vscode.window.showInputBox({
        prompt: 'Enter base color (hex)',
        value: '#3b82f6'
      });
      
      if (baseColor) {
        await this.notebookController.createDesignSystem(
          notebook.getId(),
          baseColor
        );
        
        vscode.window.showInformationMessage('Design system created!');
      }
    });
  }
  
  /**
   * Get notebook controller
   */
  getController(): NotebookController {
    return this.notebookController;
  }
  
  /**
   * Dispose provider
   */
  dispose(): void {
    this.notebookController.dispose();
  }
}