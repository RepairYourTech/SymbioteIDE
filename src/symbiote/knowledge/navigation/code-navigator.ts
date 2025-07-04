/**
 * Code Navigator
 * 
 * Navigate between code entities through the graph
 */

import * as vscode from 'vscode';
import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import {
  GraphNode,
  GraphRelationship,
  NodeType,
  RelationType,
  FileNodeProperties,
  ClassNodeProperties,
  FunctionNodeProperties
} from '../types/graph-types';

export interface NavigationTarget {
  nodeId: string;
  nodeType: NodeType;
  filePath: string;
  line?: number;
  column?: number;
  label: string;
  description?: string;
}

export interface NavigationHistory {
  past: NavigationTarget[];
  current?: NavigationTarget;
  future: NavigationTarget[];
}

export interface NavigationOptions {
  openInNewEditor?: boolean;
  preserveFocus?: boolean;
  preview?: boolean;
  showContext?: boolean;
}

export class CodeNavigator {
  private connectionManager: Neo4jConnectionManager;
  private history: NavigationHistory;
  private maxHistorySize: number = 50;
  
  constructor(connectionManager: Neo4jConnectionManager) {
    this.connectionManager = connectionManager;
    this.history = {
      past: [],
      future: []
    };
  }
  
  /**
   * Navigate to a graph node
   */
  async navigateToNode(
    nodeId: string,
    databaseId: string,
    options: NavigationOptions = {}
  ): Promise<void> {
    try {
      // Get node details
      const result = await this.connectionManager.executeQuery(
        databaseId,
        'MATCH (n {id: $id}) RETURN n',
        { id: nodeId }
      );
      
      if (result.records.length === 0) {
        throw new Error(`Node ${nodeId} not found`);
      }
      
      const node = result.records[0].get('n');
      const nodeType = node.labels[0] as NodeType;
      const properties = node.properties;
      
      // Create navigation target
      const target = this.createNavigationTarget(nodeType, properties);
      
      if (!target) {
        vscode.window.showWarningMessage(`Cannot navigate to ${nodeType} node`);
        return;
      }
      
      // Update history
      this.addToHistory(target);
      
      // Navigate
      await this.openLocation(target, options);
      
    } catch (error) {
      vscode.window.showErrorMessage(`Navigation failed: ${error}`);
    }
  }
  
  /**
   * Navigate to related entities
   */
  async navigateToRelated(
    nodeId: string,
    databaseId: string,
    relationType: RelationType,
    direction: 'incoming' | 'outgoing' | 'both' = 'both'
  ): Promise<NavigationTarget[]> {
    const directionClause = 
      direction === 'incoming' ? '<-' :
      direction === 'outgoing' ? '->' :
      '-';
    
    const query = `
      MATCH (n {id: $id})${directionClause}[r:${relationType}]${directionClause}(related)
      RETURN related
      ORDER BY related.name
      LIMIT 50
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const targets: NavigationTarget[] = [];
    
    result.records.forEach(record => {
      const node = record.get('related');
      const nodeType = node.labels[0] as NodeType;
      const target = this.createNavigationTarget(nodeType, node.properties);
      
      if (target) {
        targets.push(target);
      }
    });
    
    return targets;
  }
  
  /**
   * Find references to a symbol
   */
  async findReferences(
    nodeId: string,
    databaseId: string
  ): Promise<NavigationTarget[]> {
    const query = `
      MATCH (n {id: $id})<-[:REFERENCES|CALLS|USES|IMPORTS]-(ref)
      WHERE ref:Function OR ref:Method OR ref:Class OR ref:File
      RETURN ref, labels(ref) as labels
      ORDER BY ref.filePath, ref.startLine
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const targets: NavigationTarget[] = [];
    
    result.records.forEach(record => {
      const node = record.get('ref');
      const labels = record.get('labels');
      const nodeType = labels[0] as NodeType;
      const target = this.createNavigationTarget(nodeType, node.properties);
      
      if (target) {
        targets.push(target);
      }
    });
    
    return targets;
  }
  
  /**
   * Find definition of a symbol
   */
  async findDefinition(
    nodeId: string,
    databaseId: string
  ): Promise<NavigationTarget | null> {
    const query = `
      MATCH (n {id: $id})-[:REFERENCES|CALLS|USES|IMPORTS]->(def)
      WHERE def:Function OR def:Method OR def:Class OR def:Interface OR def:Type
      RETURN def, labels(def) as labels
      LIMIT 1
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    if (result.records.length === 0) {
      return null;
    }
    
    const node = result.records[0].get('def');
    const labels = result.records[0].get('labels');
    const nodeType = labels[0] as NodeType;
    
    return this.createNavigationTarget(nodeType, node.properties);
  }
  
  /**
   * Find implementations of an interface or abstract class
   */
  async findImplementations(
    nodeId: string,
    databaseId: string
  ): Promise<NavigationTarget[]> {
    const query = `
      MATCH (n {id: $id})<-[:IMPLEMENTS|EXTENDS]-(impl)
      WHERE impl:Class OR impl:Interface
      RETURN impl, labels(impl) as labels
      ORDER BY impl.name
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const targets: NavigationTarget[] = [];
    
    result.records.forEach(record => {
      const node = record.get('impl');
      const labels = record.get('labels');
      const nodeType = labels[0] as NodeType;
      const target = this.createNavigationTarget(nodeType, node.properties);
      
      if (target) {
        targets.push(target);
      }
    });
    
    return targets;
  }
  
  /**
   * Find call hierarchy (callers of a function/method)
   */
  async findCallers(
    nodeId: string,
    databaseId: string
  ): Promise<NavigationTarget[]> {
    const query = `
      MATCH (n {id: $id})<-[:CALLS]-(caller)
      WHERE caller:Function OR caller:Method
      RETURN caller, labels(caller) as labels
      ORDER BY caller.filePath, caller.startLine
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const targets: NavigationTarget[] = [];
    
    result.records.forEach(record => {
      const node = record.get('caller');
      const labels = record.get('labels');
      const nodeType = labels[0] as NodeType;
      const target = this.createNavigationTarget(nodeType, node.properties);
      
      if (target) {
        targets.push(target);
      }
    });
    
    return targets;
  }
  
  /**
   * Find what a function/method calls
   */
  async findCallees(
    nodeId: string,
    databaseId: string
  ): Promise<NavigationTarget[]> {
    const query = `
      MATCH (n {id: $id})-[:CALLS]->(callee)
      WHERE callee:Function OR callee:Method
      RETURN callee, labels(callee) as labels
      ORDER BY callee.name
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const targets: NavigationTarget[] = [];
    
    result.records.forEach(record => {
      const node = record.get('callee');
      const labels = record.get('labels');
      const nodeType = labels[0] as NodeType;
      const target = this.createNavigationTarget(nodeType, node.properties);
      
      if (target) {
        targets.push(target);
      }
    });
    
    return targets;
  }
  
  /**
   * Navigate through history
   */
  async navigateBack(): Promise<void> {
    if (this.history.past.length === 0) {
      return;
    }
    
    const previous = this.history.past.pop()!;
    
    if (this.history.current) {
      this.history.future.unshift(this.history.current);
    }
    
    this.history.current = previous;
    await this.openLocation(previous);
  }
  
  /**
   * Navigate forward in history
   */
  async navigateForward(): Promise<void> {
    if (this.history.future.length === 0) {
      return;
    }
    
    const next = this.history.future.shift()!;
    
    if (this.history.current) {
      this.history.past.push(this.history.current);
    }
    
    this.history.current = next;
    await this.openLocation(next);
  }
  
  /**
   * Show navigation quick pick
   */
  async showNavigationPicker(
    targets: NavigationTarget[],
    placeHolder: string = 'Select navigation target'
  ): Promise<NavigationTarget | undefined> {
    const items = targets.map(target => ({
      label: target.label,
      description: target.description,
      detail: `${target.filePath}${target.line ? ':' + target.line : ''}`,
      target
    }));
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder,
      matchOnDescription: true,
      matchOnDetail: true
    });
    
    return selected?.target;
  }
  
  /**
   * Create navigation target from node properties
   */
  private createNavigationTarget(
    nodeType: NodeType,
    properties: any
  ): NavigationTarget | null {
    switch (nodeType) {
      case NodeType.FILE:
        const fileProps = properties as FileNodeProperties;
        return {
          nodeId: properties.id,
          nodeType,
          filePath: fileProps.path,
          label: fileProps.name,
          description: `${fileProps.language} file`
        };
        
      case NodeType.CLASS:
      case NodeType.INTERFACE:
        const classProps = properties as ClassNodeProperties;
        return {
          nodeId: properties.id,
          nodeType,
          filePath: classProps.filePath,
          line: classProps.startLine,
          label: classProps.name,
          description: nodeType === NodeType.CLASS ? 'Class' : 'Interface'
        };
        
      case NodeType.FUNCTION:
      case NodeType.METHOD:
        const funcProps = properties as FunctionNodeProperties;
        return {
          nodeId: properties.id,
          nodeType,
          filePath: funcProps.filePath,
          line: funcProps.startLine,
          label: funcProps.name,
          description: funcProps.signature
        };
        
      case NodeType.VARIABLE:
      case NodeType.CONSTANT:
        return {
          nodeId: properties.id,
          nodeType,
          filePath: properties.filePath,
          line: properties.line,
          label: properties.name,
          description: properties.type || nodeType
        };
        
      default:
        // For other node types, check if they have file/line info
        if (properties.filePath) {
          return {
            nodeId: properties.id,
            nodeType,
            filePath: properties.filePath,
            line: properties.line || properties.startLine,
            label: properties.name,
            description: nodeType
          };
        }
        return null;
    }
  }
  
  /**
   * Open a file location
   */
  private async openLocation(
    target: NavigationTarget,
    options: NavigationOptions = {}
  ): Promise<void> {
    const uri = vscode.Uri.file(target.filePath);
    
    try {
      const document = await vscode.workspace.openTextDocument(uri);
      
      const showOptions: vscode.TextDocumentShowOptions = {
        preview: options.preview ?? false,
        preserveFocus: options.preserveFocus ?? false,
        viewColumn: options.openInNewEditor 
          ? vscode.ViewColumn.Beside 
          : vscode.ViewColumn.Active
      };
      
      if (target.line) {
        const position = new vscode.Position(target.line - 1, target.column || 0);
        showOptions.selection = new vscode.Range(position, position);
      }
      
      const editor = await vscode.window.showTextDocument(document, showOptions);
      
      // Highlight the line briefly
      if (target.line && !options.preserveFocus) {
        const line = target.line - 1;
        const range = new vscode.Range(line, 0, line, Number.MAX_VALUE);
        
        const decoration = vscode.window.createTextEditorDecorationType({
          backgroundColor: new vscode.ThemeColor('editor.findMatchHighlightBackground'),
          isWholeLine: true
        });
        
        editor.setDecorations(decoration, [range]);
        
        // Remove highlight after delay
        setTimeout(() => {
          decoration.dispose();
        }, 2000);
      }
      
      // Show context if requested
      if (options.showContext) {
        this.showNavigationContext(target);
      }
      
    } catch (error) {
      vscode.window.showErrorMessage(
        `Failed to open ${target.filePath}: ${error}`
      );
    }
  }
  
  /**
   * Add to navigation history
   */
  private addToHistory(target: NavigationTarget): void {
    if (this.history.current) {
      this.history.past.push(this.history.current);
      
      // Limit history size
      if (this.history.past.length > this.maxHistorySize) {
        this.history.past.shift();
      }
    }
    
    this.history.current = target;
    this.history.future = []; // Clear forward history on new navigation
  }
  
  /**
   * Show navigation context in output panel
   */
  private showNavigationContext(target: NavigationTarget): void {
    // Could show related entities, dependencies, etc.
    // For now, just log the navigation
    console.log(`Navigated to: ${target.label} at ${target.filePath}:${target.line}`);
  }
  
  /**
   * Get navigation history
   */
  getHistory(): NavigationHistory {
    return {
      past: [...this.history.past],
      current: this.history.current,
      future: [...this.history.future]
    };
  }
  
  /**
   * Clear navigation history
   */
  clearHistory(): void {
    this.history = {
      past: [],
      future: []
    };
  }
}