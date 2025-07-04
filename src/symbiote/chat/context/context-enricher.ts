/**
 * Context Enricher
 * 
 * Automatically gathers and provides context for AI interactions
 */

import * as vscode from 'vscode';
import { 
  MessageContext,
  MessageAttachment
} from '../../../types/chat-api';
import { KnowledgeGraphSystem } from '../../knowledge';
import { AIGraphQuery } from '../../knowledge/types/graph-types';

export interface EnrichmentOptions {
  sessionId: string;
  currentFile?: string;
  selectedCode?: string;
  attachments?: MessageAttachment[];
}

export class ContextEnricher {
  private knowledgeGraph: KnowledgeGraphSystem;
  private contextCache: Map<string, MessageContext> = new Map();
  
  constructor(knowledgeGraph: KnowledgeGraphSystem) {
    this.knowledgeGraph = knowledgeGraph;
  }
  
  /**
   * Enrich context with relevant information
   */
  async enrichContext(
    message: string,
    options: EnrichmentOptions
  ): Promise<MessageContext> {
    const context: MessageContext = {
      currentFile: options.currentFile || this.getCurrentFile(),
      selectedCode: options.selectedCode || this.getSelectedCode(),
      cursorPosition: this.getCursorPosition(),
      visibleRange: this.getVisibleRange(),
      openFiles: this.getOpenFiles(),
      terminalOutput: await this.getRecentTerminalOutput(),
      debugContext: this.getDebugContext()
    };
    
    // Query knowledge graph for additional context
    if (context.currentFile && this.shouldQueryKnowledgeGraph(message)) {
      const graphContext = await this.queryKnowledgeGraph(
        message,
        context.currentFile,
        context.cursorPosition
      );
      
      // Merge graph context
      Object.assign(context, graphContext);
    }
    
    // Cache context for performance
    this.contextCache.set(options.sessionId, context);
    
    return context;
  }
  
  /**
   * Build enriched prompt for AI
   */
  buildPrompt(
    message: string,
    context: MessageContext,
    attachments?: MessageAttachment[]
  ): string {
    const parts: string[] = [];
    
    // Add system context
    parts.push('You are SymbioteIDE\'s AI assistant, helping with code development.');
    
    // Add file context
    if (context.currentFile) {
      parts.push(`\nCurrent file: ${context.currentFile}`);
      
      if (context.selectedCode) {
        parts.push(`\nSelected code:\n\`\`\`\n${context.selectedCode}\n\`\`\``);
      }
      
      if (context.cursorPosition) {
        parts.push(`Cursor at line ${context.cursorPosition.line + 1}, column ${context.cursorPosition.character + 1}`);
      }
    }
    
    // Add open files context
    if (context.openFiles && context.openFiles.length > 0) {
      parts.push(`\nOpen files: ${context.openFiles.join(', ')}`);
    }
    
    // Add terminal context if relevant
    if (context.terminalOutput && this.isTerminalRelevant(message)) {
      parts.push(`\nRecent terminal output:\n\`\`\`\n${context.terminalOutput}\n\`\`\``);
    }
    
    // Add debug context if active
    if (context.debugContext) {
      parts.push(`\nDebug context: ${JSON.stringify(context.debugContext)}`);
    }
    
    // Process attachments
    if (attachments && attachments.length > 0) {
      parts.push('\nAttachments:');
      attachments.forEach((attachment, index) => {
        if (attachment.type === 'image') {
          parts.push(`- Image ${index + 1}: ${attachment.name || 'Unnamed'} (${attachment.mimeType})`);
        } else if (attachment.type === 'file') {
          parts.push(`- File ${index + 1}: ${attachment.name || 'Unnamed'}`);
        }
      });
    }
    
    // Add user message
    parts.push(`\nUser request: ${message}`);
    
    return parts.join('\n');
  }
  
  /**
   * Get relevant context from a specific line
   */
  async getLineContext(
    filePath: string,
    line: number,
    contextLines: number = 10
  ): Promise<string> {
    try {
      const document = await vscode.workspace.openTextDocument(filePath);
      const startLine = Math.max(0, line - contextLines);
      const endLine = Math.min(document.lineCount - 1, line + contextLines);
      
      const lines: string[] = [];
      for (let i = startLine; i <= endLine; i++) {
        const lineText = document.lineAt(i).text;
        const prefix = i === line ? '> ' : '  ';
        lines.push(`${prefix}${i + 1}: ${lineText}`);
      }
      
      return lines.join('\n');
    } catch (error) {
      return '';
    }
  }
  
  /**
   * Clear context cache
   */
  clearCache(sessionId?: string): void {
    if (sessionId) {
      this.contextCache.delete(sessionId);
    } else {
      this.contextCache.clear();
    }
  }
  
  // Private methods
  
  private getCurrentFile(): string | undefined {
    return vscode.window.activeTextEditor?.document.uri.fsPath;
  }
  
  private getSelectedCode(): string | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.selection.isEmpty) {
      return undefined;
    }
    
    return editor.document.getText(editor.selection);
  }
  
  private getCursorPosition(): vscode.Position | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      return undefined;
    }
    
    return editor.selection.active;
  }
  
  private getVisibleRange(): vscode.Range | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      return undefined;
    }
    
    return editor.visibleRanges[0];
  }
  
  private getOpenFiles(): string[] {
    return vscode.window.tabGroups.all
      .flatMap(group => group.tabs)
      .map(tab => (tab.input as any)?.uri?.fsPath)
      .filter(Boolean);
  }
  
  private async getRecentTerminalOutput(): Promise<string | undefined> {
    // Get the active terminal
    const terminal = vscode.window.activeTerminal;
    if (!terminal) {
      return undefined;
    }
    
    // VS Code doesn't provide direct API to read terminal output
    // This would need to be implemented via terminal data write events
    // For now, return undefined
    return undefined;
  }
  
  private getDebugContext(): any {
    // Check if debugging is active
    if (vscode.debug.activeDebugSession) {
      return {
        sessionId: vscode.debug.activeDebugSession.id,
        name: vscode.debug.activeDebugSession.name,
        type: vscode.debug.activeDebugSession.type
      };
    }
    
    return undefined;
  }
  
  private shouldQueryKnowledgeGraph(message: string): boolean {
    // Determine if we should query the knowledge graph based on message content
    const codeRelatedKeywords = [
      'function', 'class', 'method', 'variable', 'import', 'export',
      'implement', 'extend', 'call', 'use', 'where', 'how', 'what',
      'explain', 'show', 'find', 'search', 'refactor', 'modify'
    ];
    
    const lowerMessage = message.toLowerCase();
    return codeRelatedKeywords.some(keyword => lowerMessage.includes(keyword));
  }
  
  private async queryKnowledgeGraph(
    message: string,
    currentFile: string,
    cursorPosition?: vscode.Position
  ): Promise<Partial<MessageContext>> {
    try {
      const query: AIGraphQuery = {
        query: message,
        context: {
          currentFile,
          taskType: this.inferTaskType(message)
        },
        requirements: {
          includeImplementationDetails: true,
          includeUsageExamples: true,
          maxDepth: 2,
          limit: 20
        }
      };
      
      const response = await this.knowledgeGraph.queryWithAI(query);
      
      // Extract relevant context from response
      const graphContext: any = {
        knowledgeGraphContext: {
          entities: response.entities.slice(0, 5).map(e => ({
            type: e.labels[0],
            name: e.properties.name,
            file: e.properties.filePath
          })),
          relationships: response.relationships.slice(0, 10).map(r => ({
            type: r.type,
            from: r.startNodeId,
            to: r.endNodeId
          })),
          suggestions: response.suggestions
        }
      };
      
      // If we have cursor position, get specific entity context
      if (cursorPosition) {
        const entityContext = await this.knowledgeGraph.getAIContext(
          this.getProjectId(),
          currentFile,
          cursorPosition.line
        );
        
        if (entityContext) {
          graphContext.currentEntity = {
            type: entityContext.entity.labels[0],
            name: entityContext.entity.properties.name,
            dependencies: entityContext.dependencies.map((d: any) => d.properties.name)
          };
        }
      }
      
      return graphContext;
      
    } catch (error) {
      console.error('Failed to query knowledge graph:', error);
      return {};
    }
  }
  
  private inferTaskType(message: string): 'refactor' | 'implement' | 'debug' | 'analyze' | 'explain' {
    const lowerMessage = message.toLowerCase();
    
    if (lowerMessage.includes('refactor') || lowerMessage.includes('rename') || lowerMessage.includes('move')) {
      return 'refactor';
    }
    if (lowerMessage.includes('create') || lowerMessage.includes('implement') || lowerMessage.includes('add')) {
      return 'implement';
    }
    if (lowerMessage.includes('fix') || lowerMessage.includes('debug') || lowerMessage.includes('error')) {
      return 'debug';
    }
    if (lowerMessage.includes('analyze') || lowerMessage.includes('find') || lowerMessage.includes('search')) {
      return 'analyze';
    }
    
    return 'explain';
  }
  
  private isTerminalRelevant(message: string): boolean {
    const terminalKeywords = [
      'error', 'command', 'terminal', 'console', 'output',
      'run', 'execute', 'build', 'test', 'deploy'
    ];
    
    const lowerMessage = message.toLowerCase();
    return terminalKeywords.some(keyword => lowerMessage.includes(keyword));
  }
  
  private getProjectId(): string {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
      return workspaceFolder.uri.toString();
    }
    return 'default';
  }
}