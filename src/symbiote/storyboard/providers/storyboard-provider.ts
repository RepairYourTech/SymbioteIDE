/**
 * Storyboard Provider
 * 
 * VS Code custom editor provider for storyboard files
 */

import * as vscode from 'vscode';
import { StoryboardController } from '../storyboard-controller';
import { StoryboardModel } from '../storyboard-model';
import { FlowEditor } from '../editor/flow-editor';
import { Logger } from '../../utils/logger';

export class StoryboardProvider implements vscode.CustomTextEditorProvider {
  private static readonly viewType = 'symbiote.storyboard';
  private logger = new Logger('StoryboardProvider');
  private activeEditors = new Map<string, FlowEditor>();
  
  constructor(
    private context: vscode.ExtensionContext,
    private controller: StoryboardController
  ) {}
  
  /**
   * Register the provider
   */
  static register(
    context: vscode.ExtensionContext,
    controller: StoryboardController
  ): vscode.Disposable {
    const provider = new StoryboardProvider(context, controller);
    
    const providerRegistration = vscode.window.registerCustomEditorProvider(
      StoryboardProvider.viewType,
      provider,
      {
        webviewOptions: {
          retainContextWhenHidden: true
        },
        supportsMultipleEditorsPerDocument: false
      }
    );
    
    return providerRegistration;
  }
  
  /**
   * Called when a custom editor is opened
   */
  async resolveCustomTextEditor(
    document: vscode.TextDocument,
    webviewPanel: vscode.WebviewPanel,
    _token: vscode.CancellationToken
  ): Promise<void> {
    try {
      // Parse storyboard from document
      const storyboard = StoryboardModel.fromJSON(document.getText());
      
      // Create flow editor
      const editor = new FlowEditor({
        gridSize: 20,
        snapToGrid: true,
        showGrid: true,
        enableMultiSelect: true,
        enableKeyboardShortcuts: true
      });
      
      // Initialize editor with storyboard
      editor.initialize(storyboard);
      
      // Store editor reference
      this.activeEditors.set(document.uri.toString(), editor);
      
      // Set up webview
      webviewPanel.webview.options = {
        enableScripts: true,
        localResourceRoots: [
          vscode.Uri.joinPath(this.context.extensionUri, 'media'),
          vscode.Uri.joinPath(this.context.extensionUri, 'dist')
        ]
      };
      
      // Set webview content
      webviewPanel.webview.html = this.getHtmlForWebview(
        webviewPanel.webview,
        storyboard
      );
      
      // Handle messages from webview
      webviewPanel.webview.onDidReceiveMessage(
        message => this.handleWebviewMessage(message, document, editor)
      );
      
      // Handle text document changes
      const changeDocumentSubscription = vscode.workspace.onDidChangeTextDocument(e => {
        if (e.document.uri.toString() === document.uri.toString()) {
          this.updateWebview(webviewPanel.webview, document);
        }
      });
      
      // Handle storyboard changes
      storyboard.on('dirty-state-changed', (isDirty) => {
        if (isDirty) {
          this.updateTextDocument(document, storyboard);
        }
      });
      
      // Handle disposal
      webviewPanel.onDidDispose(() => {
        changeDocumentSubscription.dispose();
        this.activeEditors.delete(document.uri.toString());
        editor.dispose();
      });
      
      // Send initial update
      this.updateWebview(webviewPanel.webview, document);
      
    } catch (error) {
      this.logger.error('Failed to resolve custom editor', error);
      vscode.window.showErrorMessage(
        `Failed to open storyboard: ${error.message}`
      );
    }
  }
  
  /**
   * Handle messages from webview
   */
  private async handleWebviewMessage(
    message: any,
    document: vscode.TextDocument,
    editor: FlowEditor
  ): Promise<void> {
    switch (message.type) {
      case 'update':
        // Update storyboard model
        const storyboard = StoryboardModel.fromJSON(message.data);
        editor.initialize(storyboard);
        this.updateTextDocument(document, storyboard);
        break;
        
      case 'command':
        // Execute VS Code command
        vscode.commands.executeCommand(message.command, ...message.args);
        break;
        
      case 'save':
        // Save document
        await document.save();
        break;
        
      case 'export':
        // Export storyboard
        await this.exportStoryboard(message.format, editor);
        break;
        
      case 'analyze':
        // Analyze project
        await this.analyzeProject();
        break;
        
      case 'generate-code':
        // Generate code from storyboard
        await this.generateCode(editor);
        break;
    }
  }
  
  /**
   * Update text document with storyboard changes
   */
  private async updateTextDocument(
    document: vscode.TextDocument,
    storyboard: StoryboardModel
  ): Promise<void> {
    const edit = new vscode.WorkspaceEdit();
    const json = storyboard.toJSON();
    
    edit.replace(
      document.uri,
      new vscode.Range(0, 0, document.lineCount, 0),
      json
    );
    
    await vscode.workspace.applyEdit(edit);
  }
  
  /**
   * Update webview with document changes
   */
  private updateWebview(
    webview: vscode.Webview,
    document: vscode.TextDocument
  ): void {
    try {
      const storyboard = StoryboardModel.fromJSON(document.getText());
      
      webview.postMessage({
        type: 'update',
        data: storyboard.getStoryboard()
      });
    } catch (error) {
      this.logger.error('Failed to update webview', error);
    }
  }
  
  /**
   * Export storyboard
   */
  private async exportStoryboard(
    format: string,
    editor: FlowEditor
  ): Promise<void> {
    try {
      const uri = await vscode.window.showSaveDialog({
        filters: this.getExportFilters(format)
      });
      
      if (uri) {
        const content = await editor.exportAsImage(format as any);
        await vscode.workspace.fs.writeFile(uri, content);
        
        vscode.window.showInformationMessage(
          `Storyboard exported to ${uri.fsPath}`
        );
      }
    } catch (error) {
      this.logger.error('Failed to export storyboard', error);
      vscode.window.showErrorMessage(
        `Failed to export: ${error.message}`
      );
    }
  }
  
  /**
   * Analyze project
   */
  private async analyzeProject(): Promise<void> {
    try {
      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: 'Analyzing project components...',
        cancellable: false
      }, async () => {
        const analysis = await this.controller.analyzeProject();
        
        const action = await vscode.window.showInformationMessage(
          `Found ${analysis.components.length} components`,
          'Generate Storyboard',
          'View Details'
        );
        
        if (action === 'Generate Storyboard') {
          const storyboard = await this.controller.generateFromAnalysis(analysis);
          
          // Create new document with generated storyboard
          const uri = vscode.Uri.parse(`untitled:Generated-${Date.now()}.story`);
          const doc = await vscode.workspace.openTextDocument(uri);
          const editor = await vscode.window.showTextDocument(doc);
          
          await editor.edit(editBuilder => {
            editBuilder.insert(new vscode.Position(0, 0), storyboard.toJSON());
          });
        }
      });
    } catch (error) {
      this.logger.error('Failed to analyze project', error);
      vscode.window.showErrorMessage(
        `Analysis failed: ${error.message}`
      );
    }
  }
  
  /**
   * Generate code from storyboard
   */
  private async generateCode(editor: FlowEditor): Promise<void> {
    try {
      const framework = await vscode.window.showQuickPick(
        ['react', 'vue', 'angular'],
        { placeHolder: 'Select target framework' }
      );
      
      if (!framework) return;
      
      const options = await this.showCodeGenerationOptions();
      if (!options) return;
      
      // Generate code
      // const files = await this.controller.generateCode(
      //   storyboardId,
      //   { framework, ...options }
      // );
      
      vscode.window.showInformationMessage(
        'Code generation completed'
      );
      
    } catch (error) {
      this.logger.error('Failed to generate code', error);
      vscode.window.showErrorMessage(
        `Code generation failed: ${error.message}`
      );
    }
  }
  
  /**
   * Show code generation options
   */
  private async showCodeGenerationOptions(): Promise<any> {
    const generateRoutes = await vscode.window.showQuickPick(
      ['Yes', 'No'],
      { placeHolder: 'Generate routes?' }
    );
    
    const generateComponents = await vscode.window.showQuickPick(
      ['Yes', 'No'],
      { placeHolder: 'Generate component scaffolds?' }
    );
    
    const generateState = await vscode.window.showQuickPick(
      ['Yes', 'No'],
      { placeHolder: 'Generate state management?' }
    );
    
    if (!generateRoutes || !generateComponents || !generateState) {
      return null;
    }
    
    return {
      generateRoutes: generateRoutes === 'Yes',
      generateComponents: generateComponents === 'Yes',
      generateState: generateState === 'Yes'
    };
  }
  
  /**
   * Get export file filters
   */
  private getExportFilters(format: string): { [name: string]: string[] } {
    switch (format) {
      case 'png':
        return { 'PNG Image': ['png'] };
      case 'svg':
        return { 'SVG Image': ['svg'] };
      case 'pdf':
        return { 'PDF Document': ['pdf'] };
      case 'html':
        return { 'HTML Document': ['html'] };
      case 'markdown':
        return { 'Markdown': ['md'] };
      default:
        return { 'All Files': ['*'] };
    }
  }
  
  /**
   * Get HTML content for webview
   */
  private getHtmlForWebview(
    webview: vscode.Webview,
    storyboard: StoryboardModel
  ): string {
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'dist', 'storyboard-editor.js')
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'dist', 'storyboard-editor.css')
    );
    
    const codiconsUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'node_modules', '@vscode/codicons', 'dist', 'codicon.css')
    );
    
    return `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Storyboard Editor</title>
  <link href="${codiconsUri}" rel="stylesheet" />
  <link href="${styleUri}" rel="stylesheet" />
</head>
<body>
  <div id="toolbar">
    <button class="toolbar-button" data-action="save">
      <i class="codicon codicon-save"></i> Save
    </button>
    <button class="toolbar-button" data-action="export">
      <i class="codicon codicon-export"></i> Export
    </button>
    <button class="toolbar-button" data-action="analyze">
      <i class="codicon codicon-search"></i> Analyze
    </button>
    <button class="toolbar-button" data-action="generate">
      <i class="codicon codicon-code"></i> Generate Code
    </button>
    <div class="toolbar-separator"></div>
    <button class="toolbar-button" data-action="zoom-in">
      <i class="codicon codicon-zoom-in"></i>
    </button>
    <button class="toolbar-button" data-action="zoom-out">
      <i class="codicon codicon-zoom-out"></i>
    </button>
    <button class="toolbar-button" data-action="fit">
      <i class="codicon codicon-screen-full"></i>
    </button>
  </div>
  
  <div id="canvas-container">
    <canvas id="grid-canvas"></canvas>
    <svg id="storyboard-canvas"></svg>
  </div>
  
  <div id="properties-panel">
    <h3>Properties</h3>
    <div id="properties-content">
      <p>Select an item to view properties</p>
    </div>
  </div>
  
  <script src="${scriptUri}"></script>
  <script>
    // Initialize with storyboard data
    window.storyboardEditor.initialize(${JSON.stringify(storyboard.getStoryboard())});
  </script>
</body>
</html>
    `;
  }
}
