/**
 * VS Code WebSocket Integration
 * 
 * Integrates WebSocket functionality with VS Code
 */

import * as vscode from 'vscode';
import { WebSocketClient } from './websocket-client';
import { WebSocketServer } from './websocket-server';
import { Logger } from '../utils/logger';
import { RedisManager } from '../redis';
import {
  SocketNamespace,
  CollabCursor,
  CollabSelection,
  CollabEdit,
  TaskProgress,
  StreamResponse
} from './types';

export class WebSocketIntegration {
  private server?: WebSocketServer;
  private clients = new Map<SocketNamespace, WebSocketClient>();
  private logger = new Logger('WebSocketIntegration');
  private statusBarItem: vscode.StatusBarItem;
  private decorationTypes = new Map<string, vscode.TextEditorDecorationType>();
  private outputChannel: vscode.OutputChannel;
  
  constructor(private context: vscode.ExtensionContext) {
    this.outputChannel = vscode.window.createOutputChannel('SymbioteIDE WebSocket');
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );
    this.statusBarItem.text = '$(cloud-offline) WebSocket';
    this.statusBarItem.show();
    
    context.subscriptions.push(this.statusBarItem);
    context.subscriptions.push(this.outputChannel);
  }
  
  /**
   * Initialize server and clients
   */
  async initialize(redis?: RedisManager): Promise<void> {
    try {
      // Start server if configured
      if (vscode.workspace.getConfiguration('symbiote.websocket').get('server.enabled')) {
        await this.startServer(redis);
      }
      
      // Connect clients
      await this.connectClients();
      
      // Set up VS Code integration
      this.setupVSCodeIntegration();
      
      this.logger.info('WebSocket integration initialized');
      
    } catch (error) {
      this.logger.error('Failed to initialize WebSocket integration', error);
      throw error;
    }
  }
  
  /**
   * Start WebSocket server
   */
  private async startServer(redis?: RedisManager): Promise<void> {
    const config = vscode.workspace.getConfiguration('symbiote.websocket.server');
    
    this.server = new WebSocketServer({
      port: config.get('port', 3001),
      cors: {
        origin: config.get('cors.origin', true),
        credentials: config.get('cors.credentials', true)
      }
    });
    
    if (redis) {
      await this.server.initializeRedis(redis);
    }
    
    await this.server.start();
    
    this.outputChannel.appendLine(`WebSocket server started on port ${config.get('port', 3001)}`);
    this.updateStatusBar('$(cloud) Server Running', 'Server is running');
  }
  
  /**
   * Connect WebSocket clients
   */
  private async connectClients(): Promise<void> {
    const config = vscode.workspace.getConfiguration('symbiote.websocket.client');
    const url = config.get('url', 'http://localhost:3001');
    
    // Connect to different namespaces based on features
    if (config.get('agents.enabled')) {
      await this.connectNamespace(SocketNamespace.Agents, url);
    }
    
    if (config.get('collaboration.enabled')) {
      await this.connectNamespace(SocketNamespace.Collaboration, url);
    }
    
    if (config.get('tasks.enabled')) {
      await this.connectNamespace(SocketNamespace.Tasks, url);
    }
    
    if (config.get('memory.enabled')) {
      await this.connectNamespace(SocketNamespace.Memory, url);
    }
    
    this.updateStatusBar('$(cloud) Connected', 'Connected to WebSocket server');
  }
  
  /**
   * Connect to a namespace
   */
  private async connectNamespace(
    namespace: SocketNamespace,
    url: string
  ): Promise<void> {
    const client = new WebSocketClient(namespace, {
      url,
      auth: {
        userId: vscode.env.machineId,
        projectId: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
      }
    });
    
    // Set up handlers
    this.setupClientHandlers(client, namespace);
    
    // Wait for connection
    await client.waitForConnection();
    
    this.clients.set(namespace, client);
    this.outputChannel.appendLine(`Connected to ${namespace} namespace`);
  }
  
  /**
   * Set up client handlers
   */
  private setupClientHandlers(
    client: WebSocketClient,
    namespace: SocketNamespace
  ): void {
    // Common handlers
    client.on('connected', () => {
      this.logger.info(`Connected to ${namespace}`);
    });
    
    client.on('disconnected', () => {
      this.logger.warn(`Disconnected from ${namespace}`);
      this.updateStatusBar('$(cloud-offline) Disconnected', 'Disconnected from server');
    });
    
    client.on('error', (error) => {
      this.logger.error(`Error in ${namespace}`, error);
    });
    
    // Namespace-specific handlers
    switch (namespace) {
      case SocketNamespace.Collaboration:
        this.setupCollaborationHandlers(client);
        break;
      case SocketNamespace.Tasks:
        this.setupTaskHandlers(client);
        break;
      case SocketNamespace.Memory:
        this.setupMemoryHandlers(client);
        break;
    }
  }
  
  /**
   * Set up collaboration handlers
   */
  private setupCollaborationHandlers(client: WebSocketClient): void {
    // Handle remote cursors
    client.on('collab-cursor', (cursor: CollabCursor) => {
      this.showRemoteCursor(cursor);
    });
    
    // Handle remote selections
    client.on('collab-selection', (selection: CollabSelection) => {
      this.showRemoteSelection(selection);
    });
    
    // Handle remote edits
    client.on('collab-edit', (edit: CollabEdit) => {
      this.applyRemoteEdit(edit);
    });
    
    // Handle join/leave
    client.on('collab-join', (data) => {
      vscode.window.showInformationMessage(
        `${data.userId} joined ${data.fileUri}`
      );
    });
  }
  
  /**
   * Set up task handlers
   */
  private setupTaskHandlers(client: WebSocketClient): void {
    // Handle task progress
    client.on('task-progress', (progress: TaskProgress) => {
      this.showTaskProgress(progress);
    });
    
    // Handle task completion
    client.on('task-complete', (result) => {
      vscode.window.showInformationMessage(
        `Task completed: ${result.taskId}`
      );
    });
    
    // Handle task errors
    client.on('task-error', (error) => {
      vscode.window.showErrorMessage(
        `Task failed: ${error.error}`
      );
    });
    
    // Handle streaming responses
    client.on('stream-data', (data: StreamResponse) => {
      this.handleStreamData(data);
    });
  }
  
  /**
   * Set up memory handlers
   */
  private setupMemoryHandlers(client: WebSocketClient): void {
    // Handle memory updates
    client.on('memory-update', (memory) => {
      this.outputChannel.appendLine(`Memory updated: ${memory.id}`);
    });
    
    // Handle memory sharing
    client.on('memory-share', (data) => {
      vscode.window.showInformationMessage(
        `Memory shared by ${data.sharedBy}: ${data.memory.content.substring(0, 50)}...`
      );
    });
  }
  
  /**
   * Set up VS Code integration
   */
  private setupVSCodeIntegration(): void {
    const disposables: vscode.Disposable[] = [];
    
    // Track cursor movements
    disposables.push(
      vscode.window.onDidChangeTextEditorSelection((event) => {
        this.broadcastCursorPosition(event.textEditor);
      })
    );
    
    // Track text changes
    disposables.push(
      vscode.workspace.onDidChangeTextDocument((event) => {
        this.broadcastTextChanges(event);
      })
    );
    
    // Track file opens
    disposables.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor) {
          this.joinFileCollaboration(editor.document.uri);
        }
      })
    );
    
    // Commands
    disposables.push(
      vscode.commands.registerCommand('symbiote.websocket.connect', () => {
        this.connectClients();
      })
    );
    
    disposables.push(
      vscode.commands.registerCommand('symbiote.websocket.disconnect', () => {
        this.disconnectAll();
      })
    );
    
    disposables.push(
      vscode.commands.registerCommand('symbiote.websocket.showOutput', () => {
        this.outputChannel.show();
      })
    );
    
    this.context.subscriptions.push(...disposables);
  }
  
  /**
   * Broadcast cursor position
   */
  private broadcastCursorPosition(editor: vscode.TextEditor): void {
    const client = this.clients.get(SocketNamespace.Collaboration);
    if (!client || !client.isConnectedToServer()) return;
    
    const position = editor.selection.active;
    
    client.updateCursor({
      userId: vscode.env.machineId,
      fileUri: editor.document.uri.toString(),
      position: {
        line: position.line,
        character: position.character
      },
      label: vscode.env.appName
    });
  }
  
  /**
   * Broadcast text changes
   */
  private broadcastTextChanges(event: vscode.TextDocumentChangeEvent): void {
    const client = this.clients.get(SocketNamespace.Collaboration);
    if (!client || !client.isConnectedToServer()) return;
    
    // Only broadcast content changes
    if (event.contentChanges.length === 0) return;
    
    const edits = event.contentChanges.map(change => ({
      range: {
        start: {
          line: change.range.start.line,
          character: change.range.start.character
        },
        end: {
          line: change.range.end.line,
          character: change.range.end.character
        }
      },
      text: change.text
    }));
    
    client.sendEdit({
      userId: vscode.env.machineId,
      fileUri: event.document.uri.toString(),
      edits,
      timestamp: new Date()
    });
  }
  
  /**
   * Join file collaboration
   */
  private joinFileCollaboration(uri: vscode.Uri): void {
    const client = this.clients.get(SocketNamespace.Collaboration);
    if (!client || !client.isConnectedToServer()) return;
    
    client.joinFileCollaboration(
      uri.toString(),
      vscode.env.machineId
    );
  }
  
  /**
   * Show remote cursor
   */
  private showRemoteCursor(cursor: CollabCursor): void {
    // Skip own cursor
    if (cursor.userId === vscode.env.machineId) return;
    
    const editor = vscode.window.visibleTextEditors.find(
      e => e.document.uri.toString() === cursor.fileUri
    );
    
    if (!editor) return;
    
    // Create or update decoration
    const decorationType = this.getOrCreateDecoration(cursor.userId, cursor.color);
    
    const position = new vscode.Position(cursor.position.line, cursor.position.character);
    const decoration = {
      range: new vscode.Range(position, position),
      renderOptions: {
        after: {
          contentText: cursor.label || cursor.userId,
          backgroundColor: cursor.color || '#007ACC',
          color: 'white',
          margin: '0 0 0 1em'
        }
      }
    };
    
    editor.setDecorations(decorationType, [decoration]);
  }
  
  /**
   * Show remote selection
   */
  private showRemoteSelection(selection: CollabSelection): void {
    // Skip own selection
    if (selection.userId === vscode.env.machineId) return;
    
    const editor = vscode.window.visibleTextEditors.find(
      e => e.document.uri.toString() === selection.fileUri
    );
    
    if (!editor) return;
    
    const decorationType = this.getOrCreateDecoration(
      `${selection.userId}-selection`,
      selection.color,
      true
    );
    
    const decorations = selection.selections.map(sel => ({
      range: new vscode.Range(
        sel.start.line,
        sel.start.character,
        sel.end.line,
        sel.end.character
      )
    }));
    
    editor.setDecorations(decorationType, decorations);
  }
  
  /**
   * Apply remote edit
   */
  private async applyRemoteEdit(edit: CollabEdit): Promise<void> {
    // Skip own edits
    if (edit.userId === vscode.env.machineId) return;
    
    const document = vscode.workspace.textDocuments.find(
      d => d.uri.toString() === edit.fileUri
    );
    
    if (!document) return;
    
    const workspaceEdit = new vscode.WorkspaceEdit();
    
    for (const change of edit.edits) {
      const range = new vscode.Range(
        change.range.start.line,
        change.range.start.character,
        change.range.end.line,
        change.range.end.character
      );
      
      workspaceEdit.replace(document.uri, range, change.text);
    }
    
    await vscode.workspace.applyEdit(workspaceEdit);
  }
  
  /**
   * Show task progress
   */
  private showTaskProgress(progress: TaskProgress): void {
    vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: progress.status,
      cancellable: false
    }, async (progressReporter) => {
      progressReporter.report({
        increment: progress.progress,
        message: progress.message
      });
      
      // Keep notification open
      await new Promise(resolve => setTimeout(resolve, 3000));
    });
  }
  
  /**
   * Handle stream data
   */
  private handleStreamData(data: StreamResponse): void {
    // Show in output channel
    if (data.type === 'text' || data.type === 'code') {
      this.outputChannel.append(data.data);
    }
  }
  
  /**
   * Get or create decoration type
   */
  private getOrCreateDecoration(
    key: string,
    color?: string,
    isSelection: boolean = false
  ): vscode.TextEditorDecorationType {
    const existing = this.decorationTypes.get(key);
    if (existing) {
      return existing;
    }
    
    const decoration = vscode.window.createTextEditorDecorationType({
      backgroundColor: isSelection ? `${color}33` : undefined,
      borderColor: !isSelection ? color : undefined,
      borderWidth: !isSelection ? '2px' : undefined,
      borderStyle: !isSelection ? 'solid' : undefined
    });
    
    this.decorationTypes.set(key, decoration);
    this.context.subscriptions.push(decoration);
    
    return decoration;
  }
  
  /**
   * Update status bar
   */
  private updateStatusBar(text: string, tooltip?: string): void {
    this.statusBarItem.text = text;
    this.statusBarItem.tooltip = tooltip;
  }
  
  /**
   * Disconnect all clients
   */
  private disconnectAll(): void {
    for (const client of this.clients.values()) {
      client.disconnect();
    }
    this.clients.clear();
    
    this.updateStatusBar('$(cloud-offline) Disconnected', 'Not connected');
  }
  
  /**
   * Dispose resources
   */
  async dispose(): Promise<void> {
    // Disconnect clients
    this.disconnectAll();
    
    // Stop server
    if (this.server) {
      await this.server.stop();
    }
    
    // Clear decorations
    for (const decoration of this.decorationTypes.values()) {
      decoration.dispose();
    }
    this.decorationTypes.clear();
  }
}