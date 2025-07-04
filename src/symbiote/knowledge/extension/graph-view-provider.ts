/**
 * Graph View Provider for VS Code
 * 
 * Provides a WebView panel for visualizing code graphs
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import { KnowledgeGraphSystem } from '../index';
import { CodeNavigator } from '../navigation/code-navigator';
import { DependencyExplorer } from '../navigation/dependency-explorer';
import { 
  GraphVisualizationEngine, 
  VisualizationNode, 
  VisualizationEdge,
  LayoutType 
} from '../visualization/graph-visualization-engine';
import {
  NodeType,
  RelationType,
  AIGraphQuery
} from '../types/graph-types';

export class GraphViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'symbiote.graphView';
  
  private _view?: vscode.WebviewView;
  private knowledgeSystem: KnowledgeGraphSystem;
  private codeNavigator: CodeNavigator;
  private dependencyExplorer: DependencyExplorer;
  private currentDatabaseId?: string;
  private disposables: vscode.Disposable[] = [];
  
  constructor(
    private readonly extensionUri: vscode.Uri,
    knowledgeSystem: KnowledgeGraphSystem,
    connectionManager: Neo4jConnectionManager
  ) {
    this.knowledgeSystem = knowledgeSystem;
    this.codeNavigator = new CodeNavigator(connectionManager);
    this.dependencyExplorer = new DependencyExplorer(connectionManager);
  }
  
  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;
    
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.extensionUri, 'media'),
        vscode.Uri.joinPath(this.extensionUri, 'out', 'symbiote', 'knowledge', 'webview')
      ]
    };
    
    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);
    
    // Handle messages from the webview
    webviewView.webview.onDidReceiveMessage(
      message => this.handleMessage(message),
      null,
      this.disposables
    );
    
    // Handle visibility changes
    webviewView.onDidChangeVisibility(() => {
      if (webviewView.visible) {
        this.refresh();
      }
    });
    
    // Initialize with current workspace
    this.initializeGraph();
  }
  
  /**
   * Initialize graph with current workspace
   */
  private async initializeGraph() {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) return;
    
    try {
      // Get or create project database
      const projectId = this.getProjectId(workspaceFolder.uri.fsPath);
      const projectContext = await this.knowledgeSystem.initializeProject(
        projectId,
        workspaceFolder.uri.fsPath,
        { fullIndex: false, watchFiles: true }
      );
      
      this.currentDatabaseId = projectContext.projectId;
      
      // Load initial graph data
      await this.loadGraphData('project-overview');
      
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to initialize graph: ${error}`);
    }
  }
  
  /**
   * Load graph data based on query type
   */
  private async loadGraphData(queryType: string, params?: any) {
    if (!this.currentDatabaseId) return;
    
    try {
      let query: AIGraphQuery;
      
      switch (queryType) {
        case 'project-overview':
          query = {
            query: 'Show project structure with main files and modules',
            context: { taskType: 'analyze' },
            requirements: {
              includeImplementationDetails: false,
              maxDepth: 2,
              limit: 100
            }
          };
          break;
          
        case 'dependencies':
          query = {
            query: 'Show dependency graph for the project',
            context: { taskType: 'analyze' },
            requirements: {
              includeDependencies: true,
              maxDepth: 3,
              limit: 200
            }
          };
          break;
          
        case 'file-context':
          query = {
            query: `Show relationships for file ${params.filePath}`,
            context: { 
              currentFile: params.filePath,
              taskType: 'analyze' 
            },
            requirements: {
              includeImplementationDetails: true,
              includeRelatedConcepts: true,
              maxDepth: 2
            }
          };
          break;
          
        case 'search':
          query = {
            query: params.searchQuery,
            context: { taskType: 'analyze' },
            requirements: {
              includeImplementationDetails: true,
              limit: 50
            }
          };
          break;
          
        default:
          return;
      }
      
      const response = await this.knowledgeSystem.queryWithAI(query);
      
      // Convert to visualization format
      const nodes = response.entities.map(entity => ({
        id: entity.id,
        label: entity.properties.name || entity.id,
        type: entity.labels[0] as NodeType,
        properties: entity.properties,
        x: undefined,
        y: undefined,
        size: 20,
        color: undefined,
        icon: undefined,
        expanded: false,
        visible: true,
        metadata: entity.properties
      }));
      
      const edges = response.relationships.map(rel => ({
        id: rel.id,
        source: rel.startNodeId,
        target: rel.endNodeId,
        type: rel.type as RelationType,
        label: rel.type.replace(/_/g, ' ').toLowerCase(),
        visible: true,
        metadata: rel.properties
      }));
      
      // Send to webview
      this.postMessage({
        type: 'loadGraph',
        payload: { nodes, edges }
      });
      
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to load graph: ${error}`);
    }
  }
  
  /**
   * Handle messages from webview
   */
  private async handleMessage(message: any) {
    switch (message.type) {
      case 'ready':
        // Webview is ready, initialize
        this.initializeGraph();
        break;
        
      case 'nodeClick':
        await this.handleNodeClick(message.payload.nodeId);
        break;
        
      case 'nodeDoubleClick':
        await this.handleNodeDoubleClick(message.payload.nodeId);
        break;
        
      case 'nodeContextMenu':
        await this.handleNodeContextMenu(message.payload);
        break;
        
      case 'search':
        await this.handleSearch(message.payload.query);
        break;
        
      case 'changeLayout':
        this.postMessage({
          type: 'changeLayout',
          payload: { layout: message.payload.layout }
        });
        break;
        
      case 'expandNode':
        await this.handleExpandNode(message.payload.nodeId);
        break;
        
      case 'filter':
        this.postMessage({
          type: 'applyFilter',
          payload: message.payload
        });
        break;
        
      case 'exportGraph':
        await this.handleExportGraph(message.payload.format);
        break;
        
      case 'showDependencies':
        await this.loadGraphData('dependencies');
        break;
        
      case 'showFileContext':
        const activeEditor = vscode.window.activeTextEditor;
        if (activeEditor) {
          await this.loadGraphData('file-context', {
            filePath: activeEditor.document.uri.fsPath
          });
        }
        break;
    }
  }
  
  /**
   * Handle node click - show info
   */
  private async handleNodeClick(nodeId: string) {
    if (!this.currentDatabaseId) return;
    
    try {
      // Get node details
      const entity = await this.knowledgeSystem.getEntity(nodeId);
      
      // Show in output or hover
      const output = vscode.window.createOutputChannel('Graph Node Info');
      output.clear();
      output.appendLine(`Node: ${entity.name}`);
      output.appendLine(`Type: ${entity.type}`);
      output.appendLine(`Description: ${entity.description || 'N/A'}`);
      output.appendLine('\nProperties:');
      
      Object.entries(entity.properties || {}).forEach(([key, value]) => {
        output.appendLine(`  ${key}: ${JSON.stringify(value)}`);
      });
      
      output.show(true);
      
    } catch (error) {
      console.error('Failed to get node info:', error);
    }
  }
  
  /**
   * Handle node double click - navigate to code
   */
  private async handleNodeDoubleClick(nodeId: string) {
    if (!this.currentDatabaseId) return;
    
    try {
      await this.codeNavigator.navigateToNode(
        nodeId,
        this.currentDatabaseId,
        { preview: false }
      );
    } catch (error) {
      vscode.window.showErrorMessage(`Navigation failed: ${error}`);
    }
  }
  
  /**
   * Handle node context menu
   */
  private async handleNodeContextMenu(params: {
    nodeId: string;
    nodeType: string;
    x: number;
    y: number;
  }) {
    const actions = [];
    
    // Add common actions
    actions.push({
      label: '$(search) Find References',
      action: 'findReferences'
    });
    
    actions.push({
      label: '$(symbol-method) Find Callers',
      action: 'findCallers'
    });
    
    actions.push({
      label: '$(extensions) Show Dependencies',
      action: 'showDependencies'
    });
    
    if (params.nodeType === NodeType.CLASS || params.nodeType === NodeType.INTERFACE) {
      actions.push({
        label: '$(symbol-class) Find Implementations',
        action: 'findImplementations'
      });
    }
    
    actions.push({
      label: '$(graph) Expand Node',
      action: 'expand'
    });
    
    actions.push({
      label: '$(eye-closed) Hide Node',
      action: 'hide'
    });
    
    // Send context menu to webview
    this.postMessage({
      type: 'showContextMenu',
      payload: {
        nodeId: params.nodeId,
        actions,
        x: params.x,
        y: params.y
      }
    });
  }
  
  /**
   * Handle search
   */
  private async handleSearch(query: string) {
    if (!query.trim()) return;
    
    await this.loadGraphData('search', { searchQuery: query });
  }
  
  /**
   * Handle expand node
   */
  private async handleExpandNode(nodeId: string) {
    if (!this.currentDatabaseId) return;
    
    try {
      // Get related nodes
      const references = await this.codeNavigator.findReferences(
        nodeId,
        this.currentDatabaseId
      );
      
      const dependencies = await this.dependencyExplorer.getDirectDependencies(
        nodeId,
        this.currentDatabaseId,
        'both'
      );
      
      // Convert and send to webview
      const newNodes: VisualizationNode[] = [];
      const newEdges: VisualizationEdge[] = [];
      
      // Add reference nodes
      references.forEach(ref => {
        newNodes.push({
          id: ref.nodeId,
          label: ref.label,
          type: ref.nodeType,
          properties: { filePath: ref.filePath, line: ref.line },
          visible: true,
          expanded: false
        } as VisualizationNode);
        
        newEdges.push({
          id: `${ref.nodeId}-${nodeId}`,
          source: ref.nodeId,
          target: nodeId,
          type: RelationType.REFERENCES,
          label: 'references',
          visible: true
        } as VisualizationEdge);
      });
      
      // Add dependency nodes
      dependencies.forEach(dep => {
        newNodes.push({
          id: dep.id,
          label: dep.name,
          type: dep.type === 'file' ? NodeType.FILE : NodeType.MODULE,
          properties: { path: dep.path },
          visible: true,
          expanded: false
        } as VisualizationNode);
        
        newEdges.push({
          id: `${nodeId}-${dep.id}`,
          source: nodeId,
          target: dep.id,
          type: RelationType.DEPENDS_ON,
          label: 'depends on',
          visible: true
        } as VisualizationEdge);
      });
      
      this.postMessage({
        type: 'addNodes',
        payload: { nodes: newNodes, edges: newEdges }
      });
      
    } catch (error) {
      console.error('Failed to expand node:', error);
    }
  }
  
  /**
   * Handle export graph
   */
  private async handleExportGraph(format: 'svg' | 'png' | 'json') {
    const defaultName = `graph-${new Date().toISOString().slice(0, 10)}`;
    
    const uri = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file(`${defaultName}.${format}`),
      filters: {
        'Graph Files': [format]
      }
    });
    
    if (uri) {
      this.postMessage({
        type: 'exportGraph',
        payload: {
          format,
          path: uri.fsPath
        }
      });
    }
  }
  
  /**
   * Refresh the graph view
   */
  public refresh() {
    if (this._view) {
      this.initializeGraph();
    }
  }
  
  /**
   * Post message to webview
   */
  private postMessage(message: any) {
    if (this._view) {
      this._view.webview.postMessage(message);
    }
  }
  
  /**
   * Get HTML for webview
   */
  private _getHtmlForWebview(webview: vscode.Webview) {
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.extensionUri, 'out', 'symbiote', 'knowledge', 'webview', 'dist', 'bundle.js')
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.extensionUri, 'out', 'symbiote', 'knowledge', 'webview', 'dist', 'bundle.css')
    );
    
    const cytoscapeUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.extensionUri, 'node_modules', 'cytoscape', 'dist', 'cytoscape.min.js')
    );
    
    const nonce = getNonce();
    
    return `<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <meta http-equiv="Content-Security-Policy" content="default-src 'none'; 
        style-src ${webview.cspSource} 'unsafe-inline'; 
        script-src 'nonce-${nonce}' ${webview.cspSource};
        img-src ${webview.cspSource} data:;
        font-src ${webview.cspSource};">
      <link href="${styleUri}" rel="stylesheet">
      <title>Code Graph</title>
    </head>
    <body>
      <div id="root"></div>
      <script nonce="${nonce}" src="${cytoscapeUri}"></script>
      <script nonce="${nonce}" src="${scriptUri}"></script>
    </body>
    </html>`;
  }
  
  /**
   * Get project ID from workspace path
   */
  private getProjectId(workspacePath: string): string {
    return path.basename(workspacePath).replace(/[^a-zA-Z0-9]/g, '_');
  }
  
  /**
   * Dispose resources
   */
  public dispose() {
    while (this.disposables.length) {
      const disposable = this.disposables.pop();
      if (disposable) {
        disposable.dispose();
      }
    }
  }
}

function getNonce() {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}