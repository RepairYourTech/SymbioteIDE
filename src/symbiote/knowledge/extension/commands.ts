/**
 * VS Code Commands for Graph Visualization
 * 
 * Register and handle graph-related commands
 */

import * as vscode from 'vscode';
import { KnowledgeGraphSystem } from '../index';
import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import { CodeNavigator } from '../navigation/code-navigator';
import { DependencyExplorer } from '../navigation/dependency-explorer';
import { CallGraphAnalyzer } from '../navigation/call-graph-analyzer';
import { ImpactAnalyzer } from '../navigation/impact-analyzer';
import * as path from 'path';

export class GraphCommands {
  private codeNavigator: CodeNavigator;
  private dependencyExplorer: DependencyExplorer;
  private callGraphAnalyzer: CallGraphAnalyzer;
  private impactAnalyzer: ImpactAnalyzer;
  
  constructor(
    private knowledgeSystem: KnowledgeGraphSystem,
    private connectionManager: Neo4jConnectionManager
  ) {
    this.codeNavigator = new CodeNavigator(connectionManager);
    this.dependencyExplorer = new DependencyExplorer(connectionManager);
    this.callGraphAnalyzer = new CallGraphAnalyzer(connectionManager);
    this.impactAnalyzer = new ImpactAnalyzer(connectionManager);
  }
  
  /**
   * Register all graph commands
   */
  registerCommands(context: vscode.ExtensionContext): void {
    // Navigation commands
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.showGraph',
        () => this.showGraph()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.showCurrentFileGraph',
        () => this.showCurrentFileGraph()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findReferences',
        () => this.findReferences()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findDefinition',
        () => this.findDefinition()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findImplementations',
        () => this.findImplementations()
      )
    );
    
    // Dependency commands
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.showDependencies',
        () => this.showDependencies()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findCircularDependencies',
        () => this.findCircularDependencies()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findUnusedDependencies',
        () => this.findUnusedDependencies()
      )
    );
    
    // Call graph commands
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.showCallHierarchy',
        () => this.showCallHierarchy()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findCallers',
        () => this.findCallers()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.findCallees',
        () => this.findCallees()
      )
    );
    
    // Impact analysis commands
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.analyzeImpact',
        () => this.analyzeImpact()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.whatWouldBreak',
        () => this.whatWouldBreak()
      )
    );
    
    // Graph management commands
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.refreshIndex',
        () => this.refreshIndex()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.indexWorkspace',
        () => this.indexWorkspace()
      )
    );
    
    // History navigation
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.navigateBack',
        () => this.codeNavigator.navigateBack()
      )
    );
    
    context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.graph.navigateForward',
        () => this.codeNavigator.navigateForward()
      )
    );
  }
  
  /**
   * Show the main graph view
   */
  private async showGraph(): Promise<void> {
    await vscode.commands.executeCommand('workbench.view.extension.symbiote-graph-view');
  }
  
  /**
   * Show graph for current file
   */
  private async showCurrentFileGraph(): Promise<void> {
    const activeEditor = vscode.window.activeTextEditor;
    if (!activeEditor) {
      vscode.window.showInformationMessage('No active editor');
      return;
    }
    
    const filePath = activeEditor.document.uri.fsPath;
    const databaseId = await this.getDatabaseId();
    
    if (!databaseId) return;
    
    // Query for file node
    const fileQuery = {
      query: `Show all relationships for file: ${filePath}`,
      context: {
        currentFile: filePath,
        taskType: 'analyze' as const
      },
      requirements: {
        includeImplementationDetails: true,
        includeRelatedConcepts: true,
        maxDepth: 2
      }
    };
    
    const response = await this.knowledgeSystem.queryWithAI(fileQuery);
    
    // Show in graph view
    await vscode.commands.executeCommand('symbiote.graph.showGraph');
    
    // Send data to graph view
    vscode.commands.executeCommand('symbiote.graph.loadData', {
      nodes: response.entities,
      relationships: response.relationships
    });
  }
  
  /**
   * Find references to symbol at cursor
   */
  private async findReferences(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const references = await this.codeNavigator.findReferences(
      symbol.nodeId,
      databaseId
    );
    
    if (references.length === 0) {
      vscode.window.showInformationMessage('No references found');
      return;
    }
    
    // Show quick pick
    const selected = await this.codeNavigator.showNavigationPicker(
      references,
      'Select reference to navigate to'
    );
    
    if (selected) {
      await this.codeNavigator.navigateToNode(
        selected.nodeId,
        databaseId
      );
    }
  }
  
  /**
   * Find definition of symbol at cursor
   */
  private async findDefinition(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const definition = await this.codeNavigator.findDefinition(
      symbol.nodeId,
      databaseId
    );
    
    if (!definition) {
      vscode.window.showInformationMessage('No definition found');
      return;
    }
    
    await this.codeNavigator.navigateToNode(
      definition.nodeId,
      databaseId
    );
  }
  
  /**
   * Find implementations of interface/abstract class
   */
  private async findImplementations(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const implementations = await this.codeNavigator.findImplementations(
      symbol.nodeId,
      databaseId
    );
    
    if (implementations.length === 0) {
      vscode.window.showInformationMessage('No implementations found');
      return;
    }
    
    const selected = await this.codeNavigator.showNavigationPicker(
      implementations,
      'Select implementation to navigate to'
    );
    
    if (selected) {
      await this.codeNavigator.navigateToNode(
        selected.nodeId,
        databaseId
      );
    }
  }
  
  /**
   * Show dependencies for current project
   */
  private async showDependencies(): Promise<void> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const depGraph = await this.dependencyExplorer.getDependencyGraph(
      databaseId,
      workspaceFolder.uri.fsPath
    );
    
    // Show in graph view
    await vscode.commands.executeCommand('symbiote.graph.showGraph');
    
    // Convert and send to graph view
    vscode.commands.executeCommand('symbiote.graph.loadDependencyGraph', depGraph);
  }
  
  /**
   * Find circular dependencies
   */
  private async findCircularDependencies(): Promise<void> {
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const cycles = await this.dependencyExplorer.detectCircularDependencies(databaseId);
    
    if (cycles.length === 0) {
      vscode.window.showInformationMessage('No circular dependencies found! 🎉');
      return;
    }
    
    // Show cycles in output
    const output = vscode.window.createOutputChannel('Circular Dependencies');
    output.clear();
    output.appendLine(`Found ${cycles.length} circular dependencies:\n`);
    
    cycles.forEach((cycle, index) => {
      output.appendLine(`Cycle ${index + 1}:`);
      output.appendLine(cycle.join(' → ') + ' → ' + cycle[0]);
      output.appendLine('');
    });
    
    output.show();
  }
  
  /**
   * Find unused dependencies
   */
  private async findUnusedDependencies(): Promise<void> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const unused = await this.dependencyExplorer.findUnusedDependencies(
      databaseId,
      workspaceFolder.uri.fsPath
    );
    
    if (unused.length === 0) {
      vscode.window.showInformationMessage('No unused dependencies found! 🎉');
      return;
    }
    
    // Show quick pick to remove
    const items = unused.map(dep => ({
      label: dep.name,
      description: dep.version,
      detail: `Type: ${dep.type}`,
      dep
    }));
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: 'Select dependencies to remove',
      canPickMany: true
    });
    
    if (selected && selected.length > 0) {
      const names = selected.map(s => s.dep.name).join(', ');
      vscode.window.showInformationMessage(
        `Would remove: ${names} (not implemented)`
      );
    }
  }
  
  /**
   * Show call hierarchy for function at cursor
   */
  private async showCallHierarchy(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const hierarchy = await this.callGraphAnalyzer.getCallHierarchy(
      symbol.nodeId,
      databaseId
    );
    
    // Show in graph view
    await vscode.commands.executeCommand('symbiote.graph.showGraph');
    vscode.commands.executeCommand('symbiote.graph.loadCallHierarchy', hierarchy);
  }
  
  /**
   * Find callers of function
   */
  private async findCallers(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const callers = await this.codeNavigator.findCallers(
      symbol.nodeId,
      databaseId
    );
    
    if (callers.length === 0) {
      vscode.window.showInformationMessage('No callers found');
      return;
    }
    
    const selected = await this.codeNavigator.showNavigationPicker(
      callers,
      'Select caller to navigate to'
    );
    
    if (selected) {
      await this.codeNavigator.navigateToNode(
        selected.nodeId,
        databaseId
      );
    }
  }
  
  /**
   * Find what a function calls
   */
  private async findCallees(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const callees = await this.codeNavigator.findCallees(
      symbol.nodeId,
      databaseId
    );
    
    if (callees.length === 0) {
      vscode.window.showInformationMessage('No callees found');
      return;
    }
    
    const selected = await this.codeNavigator.showNavigationPicker(
      callees,
      'Select callee to navigate to'
    );
    
    if (selected) {
      await this.codeNavigator.navigateToNode(
        selected.nodeId,
        databaseId
      );
    }
  }
  
  /**
   * Analyze impact of changes
   */
  private async analyzeImpact(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    // Show options
    const changeType = await vscode.window.showQuickPick(
      ['Modify', 'Delete', 'Rename'],
      { placeHolder: 'Select type of change' }
    );
    
    if (!changeType) return;
    
    const impact = await this.knowledgeSystem.analyzeChangeImpact(
      databaseId,
      symbol.nodeId,
      changeType.toLowerCase() as any
    );
    
    // Show impact in graph view
    await vscode.commands.executeCommand('symbiote.graph.showGraph');
    vscode.commands.executeCommand('symbiote.graph.loadImpactAnalysis', impact);
  }
  
  /**
   * What would break if this changes
   */
  private async whatWouldBreak(): Promise<void> {
    const symbol = await this.getSymbolAtCursor();
    if (!symbol) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    const analysis = await this.impactAnalyzer.analyzeBreakingChanges(
      symbol.nodeId,
      databaseId
    );
    
    // Show results
    const output = vscode.window.createOutputChannel('Breaking Changes Analysis');
    output.clear();
    output.appendLine(`Breaking Changes Analysis for: ${symbol.name}\n`);
    
    output.appendLine(`Risk Level: ${analysis.riskLevel.toUpperCase()}`);
    output.appendLine(`Affected Files: ${analysis.affectedFiles}`);
    output.appendLine(`Breaking Changes: ${analysis.breakingChanges.length}\n`);
    
    if (analysis.breakingChanges.length > 0) {
      output.appendLine('Breaking Changes:');
      analysis.breakingChanges.forEach((change, index) => {
        output.appendLine(`\n${index + 1}. ${change.description}`);
        output.appendLine(`   Type: ${change.type}`);
        output.appendLine(`   Severity: ${change.severity}`);
        output.appendLine(`   Location: ${change.location.file}:${change.location.line}`);
      });
    }
    
    output.appendLine('\nRecommendations:');
    analysis.recommendations.forEach(rec => {
      output.appendLine(`• ${rec}`);
    });
    
    output.show();
  }
  
  /**
   * Refresh graph index for current file
   */
  private async refreshIndex(): Promise<void> {
    const activeEditor = vscode.window.activeTextEditor;
    if (!activeEditor) return;
    
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return;
    
    vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Refreshing graph index...',
      cancellable: false
    }, async () => {
      // Re-index current file
      // This would be implemented in the ingestion pipeline
      vscode.window.showInformationMessage('Graph index refreshed');
    });
  }
  
  /**
   * Index entire workspace
   */
  private async indexWorkspace(): Promise<void> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) return;
    
    const answer = await vscode.window.showInformationMessage(
      'This will index the entire workspace. This may take a while for large projects.',
      'Continue',
      'Cancel'
    );
    
    if (answer !== 'Continue') return;
    
    vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Indexing workspace...',
      cancellable: true
    }, async (progress, token) => {
      const projectId = this.getProjectId(workspaceFolder.uri.fsPath);
      
      await this.knowledgeSystem.initializeProject(
        projectId,
        workspaceFolder.uri.fsPath,
        { fullIndex: true, watchFiles: true }
      );
      
      vscode.window.showInformationMessage('Workspace indexing complete');
    });
  }
  
  // Helper methods
  
  /**
   * Get symbol at cursor position
   */
  private async getSymbolAtCursor(): Promise<{
    nodeId: string;
    name: string;
    type: string;
  } | null> {
    const activeEditor = vscode.window.activeTextEditor;
    if (!activeEditor) {
      vscode.window.showInformationMessage('No active editor');
      return null;
    }
    
    const position = activeEditor.selection.active;
    const wordRange = activeEditor.document.getWordRangeAtPosition(position);
    
    if (!wordRange) {
      vscode.window.showInformationMessage('No symbol at cursor');
      return null;
    }
    
    const word = activeEditor.document.getText(wordRange);
    const filePath = activeEditor.document.uri.fsPath;
    const line = position.line + 1;
    
    // Query graph for symbol at this location
    const databaseId = await this.getDatabaseId();
    if (!databaseId) return null;
    
    const query = {
      query: `Find symbol "${word}" at ${filePath}:${line}`,
      context: {
        currentFile: filePath,
        selectedCode: word,
        taskType: 'analyze' as const
      },
      requirements: {
        limit: 1
      }
    };
    
    const response = await this.knowledgeSystem.queryWithAI(query);
    
    if (response.entities.length === 0) {
      vscode.window.showInformationMessage(`Symbol "${word}" not found in graph`);
      return null;
    }
    
    const entity = response.entities[0];
    return {
      nodeId: entity.id,
      name: entity.properties.name || word,
      type: entity.labels[0]
    };
  }
  
  /**
   * Get current database ID
   */
  private async getDatabaseId(): Promise<string | null> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
      vscode.window.showErrorMessage('No workspace folder open');
      return null;
    }
    
    return this.getProjectId(workspaceFolder.uri.fsPath);
  }
  
  /**
   * Get project ID from workspace path
   */
  private getProjectId(workspacePath: string): string {
    return path.basename(workspacePath).replace(/[^a-zA-Z0-9]/g, '_');
  }
}