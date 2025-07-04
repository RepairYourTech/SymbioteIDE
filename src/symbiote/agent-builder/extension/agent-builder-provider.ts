/**
 * Agent Builder Provider
 * 
 * VS Code custom editor provider for visual agent workflow builder
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { AgentWorkflow, WebviewMessage } from '../types/workflow-types';
import { WorkflowExecutor } from './workflow-executor';
import { WorkflowExporter } from './workflow-exporter';

export class AgentBuilderProvider implements vscode.CustomTextEditorProvider {
  public static readonly viewType = 'symbiote.agentBuilder';
  
  private static readonly WORKFLOW_EXTENSION = '.aflow';
  
  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly executor: WorkflowExecutor = new WorkflowExecutor(),
    private readonly exporter: WorkflowExporter = new WorkflowExporter()
  ) {}
  
  public static register(context: vscode.ExtensionContext): vscode.Disposable {
    const executor = new WorkflowExecutor();
    const exporter = new WorkflowExporter();
    const provider = new AgentBuilderProvider(context, executor, exporter);
    
    const providerRegistration = vscode.window.registerCustomEditorProvider(
      AgentBuilderProvider.viewType,
      provider,
      {
        webviewOptions: {
          retainContextWhenHidden: true
        },
        supportsMultipleEditorsPerDocument: false
      }
    );
    
    // Register commands
    const commands = [
      vscode.commands.registerCommand('symbiote.agentBuilder.new', () => {
        provider.createNewWorkflow();
      }),
      vscode.commands.registerCommand('symbiote.agentBuilder.execute', () => {
        provider.executeCurrentWorkflow();
      }),
      vscode.commands.registerCommand('symbiote.agentBuilder.export', () => {
        provider.exportCurrentWorkflow();
      })
    ];
    
    return vscode.Disposable.from(providerRegistration, ...commands);
  }
  
  async resolveCustomTextEditor(
    document: vscode.TextDocument,
    webviewPanel: vscode.WebviewPanel,
    _token: vscode.CancellationToken
  ): Promise<void> {
    // Setup webview
    webviewPanel.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.context.extensionUri, 'src', 'symbiote', 'agent-builder', 'webview', 'dist')
      ]
    };
    
    webviewPanel.webview.html = this.getHtmlContent(webviewPanel.webview);
    
    // Setup message handling
    webviewPanel.webview.onDidReceiveMessage(
      message => this.handleMessage(message, document, webviewPanel),
      undefined,
      this.context.subscriptions
    );
    
    // Load workflow from document
    this.loadWorkflow(document, webviewPanel);
    
    // Handle document changes
    const changeDocumentSubscription = vscode.workspace.onDidChangeTextDocument(e => {
      if (e.document.uri.toString() === document.uri.toString()) {
        this.loadWorkflow(document, webviewPanel);
      }
    });
    
    webviewPanel.onDidDispose(() => {
      changeDocumentSubscription.dispose();
    });
  }
  
  private getHtmlContent(webview: vscode.Webview): string {
    const webviewPath = vscode.Uri.joinPath(
      this.context.extensionUri,
      'src',
      'symbiote',
      'agent-builder',
      'webview',
      'dist'
    );
    
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(webviewPath, 'main.js')
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(webviewPath, 'main.css')
    );
    
    const nonce = this.getNonce();
    
    return `<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'; font-src ${webview.cspSource}; img-src ${webview.cspSource} data:;">
      <link href="${styleUri}" rel="stylesheet">
      <title>Agent Workflow Builder</title>
    </head>
    <body>
      <div id="root"></div>
      <script nonce="${nonce}" src="${scriptUri}"></script>
    </body>
    </html>`;
  }
  
  private getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
      text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
  }
  
  private async handleMessage(
    message: WebviewMessage,
    document: vscode.TextDocument,
    panel: vscode.WebviewPanel
  ): Promise<void> {
    switch (message.type) {
      case 'saveWorkflow':
        await this.saveWorkflow(message.payload, document);
        break;
        
      case 'executeWorkflow':
        await this.executeWorkflow(message.payload.workflow, message.payload.input, panel);
        break;
        
      case 'exportWorkflow':
        await this.exportWorkflow(
          message.payload.workflow,
          message.payload.format,
          message.payload.options
        );
        break;
        
      case 'requestAgentTypes':
        await this.sendAgentTypes(panel);
        break;
        
      case 'requestToolTypes':
        await this.sendToolTypes(panel);
        break;
        
      case 'log':
        console.log('[Agent Builder]', message.payload);
        break;
    }
  }
  
  private async loadWorkflow(document: vscode.TextDocument, panel: vscode.WebviewPanel): Promise<void> {
    try {
      const text = document.getText();
      if (text.trim()) {
        const workflow = JSON.parse(text) as AgentWorkflow;
        
        panel.webview.postMessage({
          type: 'workflowLoaded',
          payload: workflow
        });
      }
    } catch (error) {
      console.error('Failed to load workflow:', error);
      vscode.window.showErrorMessage('Failed to load workflow: Invalid JSON');
    }
  }
  
  private async saveWorkflow(workflow: AgentWorkflow, document: vscode.TextDocument): Promise<void> {
    const edit = new vscode.WorkspaceEdit();
    const fullRange = new vscode.Range(
      document.positionAt(0),
      document.positionAt(document.getText().length)
    );
    
    edit.replace(
      document.uri,
      fullRange,
      JSON.stringify(workflow, null, 2)
    );
    
    await vscode.workspace.applyEdit(edit);
  }
  
  private async executeWorkflow(
    workflow: AgentWorkflow,
    input: any,
    panel: vscode.WebviewPanel
  ): Promise<void> {
    try {
      // Show output channel
      const outputChannel = vscode.window.createOutputChannel('Agent Workflow Execution');
      outputChannel.show(true);
      
      // Create execution handler
      const executionHandler = {
        onNodeStart: (nodeId: string) => {
          panel.webview.postMessage({
            type: 'nodeExecutionUpdate',
            payload: {
              nodeId,
              status: 'running'
            }
          });
        },
        onNodeComplete: (nodeId: string, output: any) => {
          panel.webview.postMessage({
            type: 'nodeExecutionUpdate',
            payload: {
              nodeId,
              status: 'completed',
              output
            }
          });
        },
        onNodeError: (nodeId: string, error: any) => {
          panel.webview.postMessage({
            type: 'nodeExecutionUpdate',
            payload: {
              nodeId,
              status: 'failed',
              error
            }
          });
        },
        onLog: (message: string) => {
          outputChannel.appendLine(message);
        }
      };
      
      // Execute workflow
      const result = await this.executor.execute(workflow, input, executionHandler);
      
      // Send completion
      panel.webview.postMessage({
        type: 'executionComplete',
        payload: result
      });
      
      vscode.window.showInformationMessage('Workflow execution completed');
      
    } catch (error: any) {
      console.error('Workflow execution failed:', error);
      vscode.window.showErrorMessage(`Workflow execution failed: ${error.message}`);
      
      panel.webview.postMessage({
        type: 'executionError',
        payload: {
          message: error.message,
          stack: error.stack
        }
      });
    }
  }
  
  private async exportWorkflow(
    workflow: AgentWorkflow,
    format: string,
    options: any
  ): Promise<void> {
    try {
      const exported = await this.exporter.export(workflow, format as any, options);
      
      // Ask user where to save
      const defaultUri = vscode.Uri.file(
        path.join(vscode.workspace.rootPath || '', `${workflow.name}.${format}`)
      );
      
      const uri = await vscode.window.showSaveDialog({
        defaultUri,
        filters: this.getExportFilters(format)
      });
      
      if (uri) {
        await vscode.workspace.fs.writeFile(
          uri,
          Buffer.from(exported.code, 'utf-8')
        );
        
        vscode.window.showInformationMessage(`Workflow exported to ${uri.fsPath}`);
      }
      
    } catch (error: any) {
      console.error('Export failed:', error);
      vscode.window.showErrorMessage(`Export failed: ${error.message}`);
    }
  }
  
  private getExportFilters(format: string): { [name: string]: string[] } {
    switch (format) {
      case 'typescript':
        return { 'TypeScript files': ['ts'] };
      case 'javascript':
        return { 'JavaScript files': ['js'] };
      case 'python':
        return { 'Python files': ['py'] };
      case 'docker':
        return { 'Dockerfile': ['Dockerfile'] };
      default:
        return { 'All files': ['*'] };
    }
  }
  
  private async sendAgentTypes(panel: vscode.WebviewPanel): Promise<void> {
    // Get available agent types from agent factory
    const agentTypes = [
      'architecture-analyst',
      'performance-optimizer',
      'security-auditor',
      'documentation-generator',
      'database-architect',
      'test-strategy',
      'code-refactoring',
      'dependency-analyzer'
    ];
    
    panel.webview.postMessage({
      type: 'agentTypesLoaded',
      payload: agentTypes
    });
  }
  
  private async sendToolTypes(panel: vscode.WebviewPanel): Promise<void> {
    // Get available tools
    // This would integrate with MCP servers
    const toolTypes = [
      { id: 'file-read', name: 'Read File', category: 'file' },
      { id: 'file-write', name: 'Write File', category: 'file' },
      { id: 'http-request', name: 'HTTP Request', category: 'network' },
      { id: 'database-query', name: 'Database Query', category: 'database' }
    ];
    
    panel.webview.postMessage({
      type: 'toolTypesLoaded',
      payload: toolTypes
    });
  }
  
  private async createNewWorkflow(): Promise<void> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
      vscode.window.showErrorMessage('Please open a workspace first');
      return;
    }
    
    const fileName = await vscode.window.showInputBox({
      prompt: 'Enter workflow name',
      value: 'new-workflow'
    });
    
    if (!fileName) {
      return;
    }
    
    const uri = vscode.Uri.file(
      path.join(workspaceFolders[0].uri.fsPath, `${fileName}${AgentBuilderProvider.WORKFLOW_EXTENSION}`)
    );
    
    const workflow: AgentWorkflow = {
      id: this.generateId(),
      name: fileName,
      version: '1.0.0',
      nodes: [],
      edges: [],
      variables: [],
      metadata: {
        created: new Date(),
        modified: new Date(),
        author: vscode.env.machineId,
        tags: []
      }
    };
    
    await vscode.workspace.fs.writeFile(
      uri,
      Buffer.from(JSON.stringify(workflow, null, 2), 'utf-8')
    );
    
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.Active });
  }
  
  private async executeCurrentWorkflow(): Promise<void> {
    const activeEditor = vscode.window.activeTextEditor;
    if (!activeEditor || !activeEditor.document.fileName.endsWith(AgentBuilderProvider.WORKFLOW_EXTENSION)) {
      vscode.window.showErrorMessage('Please open a workflow file first');
      return;
    }
    
    // The webview will handle execution
    vscode.commands.executeCommand('workbench.action.webview.postMessage', {
      type: 'executeFromCommand'
    });
  }
  
  private async exportCurrentWorkflow(): Promise<void> {
    const activeEditor = vscode.window.activeTextEditor;
    if (!activeEditor || !activeEditor.document.fileName.endsWith(AgentBuilderProvider.WORKFLOW_EXTENSION)) {
      vscode.window.showErrorMessage('Please open a workflow file first');
      return;
    }
    
    // The webview will handle export
    vscode.commands.executeCommand('workbench.action.webview.postMessage', {
      type: 'exportFromCommand'
    });
  }
  
  private generateId(): string {
    return `workflow_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}