/**
 * Chat Provider
 * 
 * Main provider class that manages the chat webview panel lifecycle
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { 
  IChatAPI,
  ChatSession,
  ChatMessage,
  MessageAttachment,
  VoiceInput,
  CodeAction,
  DiffPreview,
  MessageContext,
  ChatConfiguration,
  ChatError,
  ChatErrorCode,
  WebviewMessage,
  ChatWebviewMessages
} from '../../types/chat-api';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { KnowledgeGraphSystem } from '../knowledge';
import { ConversationManager } from './conversation-manager';
import { ContextEnricher } from './context/context-enricher';
import { CodeActionExecutor } from './actions/code-action-executor';

export class ChatProvider implements vscode.WebviewViewProvider, IChatAPI {
  public static readonly viewType = 'symbiote.chatView';
  
  private _view?: vscode.WebviewView;
  private _extensionUri: vscode.Uri;
  private _orchestrationEngine: OrchestrationEngine;
  private _knowledgeGraph: KnowledgeGraphSystem;
  private _conversationManager: ConversationManager;
  private _contextEnricher: ContextEnricher;
  private _codeActionExecutor: CodeActionExecutor;
  
  // Event emitters
  private _onMessage = new vscode.EventEmitter<ChatMessage>();
  private _onSessionCreated = new vscode.EventEmitter<ChatSession>();
  private _onSessionUpdated = new vscode.EventEmitter<ChatSession>();
  private _onSessionDeleted = new vscode.EventEmitter<string>();
  private _onCodeActionPreview = new vscode.EventEmitter<CodeAction>();
  private _onCodeActionApplied = new vscode.EventEmitter<CodeAction>();
  private _onError = new vscode.EventEmitter<ChatError>();
  
  // Current state
  private _currentSession: ChatSession | null = null;
  private _pendingActions: Map<string, CodeAction> = new Map();
  private _configuration: ChatConfiguration;
  
  constructor(
    extensionUri: vscode.Uri,
    orchestrationEngine: OrchestrationEngine,
    knowledgeGraph: KnowledgeGraphSystem
  ) {
    this._extensionUri = extensionUri;
    this._orchestrationEngine = orchestrationEngine;
    this._knowledgeGraph = knowledgeGraph;
    
    // Initialize components
    this._conversationManager = new ConversationManager(extensionUri);
    this._contextEnricher = new ContextEnricher(knowledgeGraph);
    this._codeActionExecutor = new CodeActionExecutor();
    
    // Load configuration
    this._configuration = this.loadConfiguration();
  }
  
  // WebviewViewProvider implementation
  
  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;
    
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this._extensionUri, 'media'),
        vscode.Uri.joinPath(this._extensionUri, 'out')
      ]
    };
    
    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);
    
    // Set up message handling
    webviewView.webview.onDidReceiveMessage(
      this._handleWebviewMessage.bind(this)
    );
    
    // Initialize view with current session
    this._initializeView();
  }
  
  // IChatAPI implementation
  
  public get onMessage() { return this._onMessage.event; }
  public get onSessionCreated() { return this._onSessionCreated.event; }
  public get onSessionUpdated() { return this._onSessionUpdated.event; }
  public get onSessionDeleted() { return this._onSessionDeleted.event; }
  public get onCodeActionPreview() { return this._onCodeActionPreview.event; }
  public get onCodeActionApplied() { return this._onCodeActionApplied.event; }
  public get onError() { return this._onError.event; }
  
  async createSession(title?: string): Promise<ChatSession> {
    const session = await this._conversationManager.createSession(title);
    this._currentSession = session;
    this._onSessionCreated.fire(session);
    
    // Update webview
    this._postMessage('session-loaded', { session });
    
    return session;
  }
  
  async getSession(sessionId: string): Promise<ChatSession | null> {
    return this._conversationManager.getSession(sessionId);
  }
  
  async listSessions(): Promise<ChatSession[]> {
    return this._conversationManager.listSessions();
  }
  
  async deleteSession(sessionId: string): Promise<void> {
    await this._conversationManager.deleteSession(sessionId);
    this._onSessionDeleted.fire(sessionId);
    
    // Clear current session if it was deleted
    if (this._currentSession?.id === sessionId) {
      this._currentSession = null;
    }
    
    // Update webview
    const sessions = await this.listSessions();
    this._postMessage('sessions-updated', { sessions });
  }
  
  async sendMessage(
    sessionId: string,
    content: string,
    attachments?: MessageAttachment[]
  ): Promise<ChatMessage> {
    try {
      // Get current session
      const session = await this.getSession(sessionId);
      if (!session) {
        throw new Error('Session not found');
      }
      
      // Create user message
      const userMessage: ChatMessage = {
        id: this.generateId(),
        role: 'user',
        content,
        timestamp: new Date(),
        attachments
      };
      
      // Add to session
      session.messages.push(userMessage);
      await this._conversationManager.updateSession(session);
      
      // Emit user message
      this._onMessage.fire(userMessage);
      this._postMessage('message-received', { message: userMessage });
      
      // Enrich context
      const context = await this._contextEnricher.enrichContext(content, {
        sessionId,
        currentFile: vscode.window.activeTextEditor?.document.uri.fsPath,
        selectedCode: this.getSelectedCode(),
        attachments
      });
      
      // Process with AI
      const response = await this._processWithAI(content, context, attachments);
      
      // Parse code actions from response
      const codeActions = await this.parseCodeActions(response.content);
      
      // Create assistant message
      const assistantMessage: ChatMessage = {
        id: this.generateId(),
        role: 'assistant',
        content: response.content,
        timestamp: new Date(),
        metadata: {
          model: response.model,
          tokenUsage: response.tokenUsage,
          executionTime: response.executionTime,
          context
        },
        codeActions
      };
      
      // Add to session
      session.messages.push(assistantMessage);
      await this._conversationManager.updateSession(session);
      
      // Emit assistant message
      this._onMessage.fire(assistantMessage);
      this._postMessage('message-received', { message: assistantMessage });
      
      return assistantMessage;
      
    } catch (error) {
      const chatError: ChatError = {
        code: ChatErrorCode.MODEL_ERROR,
        message: error.message || 'Failed to process message',
        details: error
      };
      
      this._onError.fire(chatError);
      this._postMessage('error', { error: chatError });
      
      throw error;
    }
  }
  
  async sendVoiceMessage(
    sessionId: string,
    voice: VoiceInput
  ): Promise<ChatMessage> {
    // Transcribe voice to text
    const text = await this.transcribeVoice(voice);
    
    // Create voice attachment
    const attachment: MessageAttachment = {
      type: 'audio',
      data: voice.audio,
      mimeType: `audio/${voice.format}`,
      metadata: {
        duration: voice.duration,
        transcription: text
      }
    };
    
    // Send as regular message with attachment
    return this.sendMessage(sessionId, text, [attachment]);
  }
  
  async previewCodeAction(action: CodeAction): Promise<DiffPreview> {
    const preview = await this._codeActionExecutor.preview(action);
    
    // Store pending action
    this._pendingActions.set(action.id, action);
    
    // Emit preview event
    this._onCodeActionPreview.fire(action);
    this._postMessage('diff-preview', { preview });
    
    return preview;
  }
  
  async applyCodeAction(action: CodeAction): Promise<void> {
    try {
      await this._codeActionExecutor.apply(action);
      
      // Update knowledge graph
      if (this._configuration.ai.includeWorkspaceContext) {
        await this._knowledgeGraph.updateFromAI(
          this.getProjectId(),
          action.files.map(f => ({
            type: f.type === 'create' ? 'create' : 
                  f.type === 'delete' ? 'delete' : 'modify',
            filePath: f.path,
            code: f.content
          }))
        );
      }
      
      // Remove from pending
      this._pendingActions.delete(action.id);
      
      // Emit applied event
      this._onCodeActionApplied.fire(action);
      
    } catch (error) {
      const chatError: ChatError = {
        code: ChatErrorCode.INVALID_ACTION,
        message: 'Failed to apply code action',
        details: error
      };
      
      this._onError.fire(chatError);
      throw error;
    }
  }
  
  async rejectCodeAction(actionId: string): Promise<void> {
    this._pendingActions.delete(actionId);
  }
  
  async updateContext(
    sessionId: string,
    context: Partial<MessageContext>
  ): Promise<void> {
    const session = await this.getSession(sessionId);
    if (!session) {
      throw new Error('Session not found');
    }
    
    // Update session context
    session.context = {
      ...session.context,
      ...context
    };
    
    await this._conversationManager.updateSession(session);
  }
  
  async searchMessages(query: string): Promise<ChatMessage[]> {
    return this._conversationManager.searchMessages(query);
  }
  
  async exportSession(sessionId: string): Promise<string> {
    return this._conversationManager.exportSession(sessionId);
  }
  
  async importSession(data: string): Promise<ChatSession> {
    const session = await this._conversationManager.importSession(data);
    this._onSessionCreated.fire(session);
    return session;
  }
  
  getConfiguration(): ChatConfiguration {
    return this._configuration;
  }
  
  async updateConfiguration(config: Partial<ChatConfiguration>): Promise<void> {
    this._configuration = {
      ...this._configuration,
      ...config
    };
    
    // Save configuration
    await this.saveConfiguration();
    
    // Update webview
    this._postMessage('config-updated', { config: this._configuration });
  }
  
  // Private methods
  
  private async _handleWebviewMessage(message: WebviewMessage) {
    const handlers: Record<string, (payload: any) => Promise<void>> = {
      'send-message': async (payload) => {
        if (this._currentSession) {
          await this.sendMessage(
            this._currentSession.id,
            payload.content,
            payload.attachments
          );
        }
      },
      
      'preview-action': async (payload) => {
        const action = this._pendingActions.get(payload.actionId);
        if (action) {
          await this.previewCodeAction(action);
        }
      },
      
      'apply-action': async (payload) => {
        const action = this._pendingActions.get(payload.actionId);
        if (action) {
          await this.applyCodeAction(action);
        }
      },
      
      'reject-action': async (payload) => {
        await this.rejectCodeAction(payload.actionId);
      },
      
      'update-config': async (payload) => {
        await this.updateConfiguration(payload.config);
      },
      
      'load-session': async (payload) => {
        const session = await this.getSession(payload.sessionId);
        if (session) {
          this._currentSession = session;
          this._postMessage('session-loaded', { session });
        }
      },
      
      'create-session': async (payload) => {
        await this.createSession(payload.title);
      },
      
      'delete-session': async (payload) => {
        await this.deleteSession(payload.sessionId);
      },
      
      'search-messages': async (payload) => {
        const messages = await this.searchMessages(payload.query);
        this._postMessage('search-results', { messages });
      }
    };
    
    const handler = handlers[message.type];
    if (handler) {
      try {
        await handler(message.payload);
      } catch (error) {
        this._onError.fire({
          code: ChatErrorCode.INVALID_ACTION,
          message: error.message,
          details: error
        });
      }
    }
  }
  
  private _postMessage<K extends keyof ChatWebviewMessages>(
    type: K,
    payload: ChatWebviewMessages[K]
  ) {
    if (this._view) {
      this._view.webview.postMessage({ type, payload });
    }
  }
  
  private async _initializeView() {
    // Load sessions
    const sessions = await this.listSessions();
    this._postMessage('sessions-updated', { sessions });
    
    // Load current or create new session
    if (!this._currentSession && sessions.length > 0) {
      this._currentSession = sessions[0];
    } else if (!this._currentSession) {
      this._currentSession = await this.createSession('New Chat');
    }
    
    this._postMessage('session-loaded', { session: this._currentSession });
    this._postMessage('config-updated', { config: this._configuration });
  }
  
  private _getHtmlForWebview(webview: vscode.Webview): string {
    // Use the React build
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'out', 'chat-webview', 'bundle.js')
    );
    
    const codiconsUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'node_modules', '@vscode/codicons', 'dist', 'codicon.css')
    );
    
    const toolkitUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'node_modules', '@vscode/webview-ui-toolkit', 'dist', 'toolkit.min.js')
    );
    
    const nonce = this.getNonce();
    
    // Read the generated HTML template
    const htmlPath = vscode.Uri.joinPath(this._extensionUri, 'out', 'chat-webview', 'index.html');
    let html = '';
    
    try {
      const htmlContent = fs.readFileSync(htmlPath.fsPath, 'utf8');
      // Replace placeholders
      html = htmlContent
        .replace(/{{cspSource}}/g, webview.cspSource)
        .replace('</head>', `<link href="${codiconsUri}" rel="stylesheet" /></head>`)
        .replace('</body>', `<script type="module" src="${toolkitUri}"></script></body>`);
    } catch (error) {
      // Fallback to basic HTML if build not found
      html = `<!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src ${webview.cspSource}; font-src ${webview.cspSource}; img-src ${webview.cspSource} https: data:;">
        <link href="${codiconsUri}" rel="stylesheet" />
        <title>SymbioteIDE Chat</title>
      </head>
      <body>
        <div id="root"></div>
        <script type="module" src="${toolkitUri}"></script>
        <script src="${scriptUri}"></script>
      </body>
      </html>`;
    }
    
    return html;
  }
  
  private getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
      text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
  }
  
  private generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substring(2);
  }
  
  private loadConfiguration(): ChatConfiguration {
    const config = vscode.workspace.getConfiguration('symbiote.chat');
    
    return {
      ui: {
        fontSize: config.get('ui.fontSize', 14),
        theme: config.get('ui.theme', 'auto'),
        showTokenUsage: config.get('ui.showTokenUsage', true),
        showModelIndicator: config.get('ui.showModelIndicator', true),
        enableSyntaxHighlighting: config.get('ui.enableSyntaxHighlighting', true),
        autoScroll: config.get('ui.autoScroll', true),
        compactMode: config.get('ui.compactMode', false)
      },
      ai: {
        defaultModel: config.get('ai.defaultModel', 'gpt-4'),
        temperature: config.get('ai.temperature', 0.7),
        maxTokens: config.get('ai.maxTokens', 2048),
        contextWindow: config.get('ai.contextWindow', 8192),
        includeWorkspaceContext: config.get('ai.includeWorkspaceContext', true),
        includeTerminalOutput: config.get('ai.includeTerminalOutput', true),
        includeDebugContext: config.get('ai.includeDebugContext', false),
        autoSelectModel: config.get('ai.autoSelectModel', true)
      },
      persistence: {
        enabled: config.get('persistence.enabled', true),
        location: config.get('persistence.location', 'workspace'),
        maxHistorySize: config.get('persistence.maxHistorySize', 1000),
        autoSave: config.get('persistence.autoSave', true),
        encryptHistory: config.get('persistence.encryptHistory', false)
      },
      features: {
        voiceInput: config.get('features.voiceInput', true),
        imageInput: config.get('features.imageInput', true),
        diagramInput: config.get('features.diagramInput', true),
        codeActions: config.get('features.codeActions', true),
        diffPreview: config.get('features.diffPreview', true),
        multiFileOperations: config.get('features.multiFileOperations', true),
        teamSharing: config.get('features.teamSharing', false)
      }
    };
  }
  
  private async saveConfiguration(): Promise<void> {
    const config = vscode.workspace.getConfiguration('symbiote.chat');
    
    // Save UI config
    await config.update('ui.fontSize', this._configuration.ui.fontSize, true);
    await config.update('ui.theme', this._configuration.ui.theme, true);
    await config.update('ui.showTokenUsage', this._configuration.ui.showTokenUsage, true);
    
    // Save AI config
    await config.update('ai.defaultModel', this._configuration.ai.defaultModel, true);
    await config.update('ai.temperature', this._configuration.ai.temperature, true);
    await config.update('ai.maxTokens', this._configuration.ai.maxTokens, true);
    
    // Save other configs...
  }
  
  private async _processWithAI(
    content: string,
    context: MessageContext,
    attachments?: MessageAttachment[]
  ): Promise<{
    content: string;
    model: string;
    tokenUsage: any;
    executionTime: number;
  }> {
    const startTime = Date.now();
    
    // Build prompt with context
    const prompt = this._contextEnricher.buildPrompt(content, context, attachments);
    
    // Execute with orchestration engine
    const result = await this._orchestrationEngine.execute({
      type: 'chat',
      prompt,
      context: JSON.stringify(context),
      constraints: {
        maxTokens: this._configuration.ai.maxTokens,
        temperature: this._configuration.ai.temperature
      }
    });
    
    return {
      content: result.content,
      model: result.modelUsed,
      tokenUsage: result.usage,
      executionTime: Date.now() - startTime
    };
  }
  
  private async parseCodeActions(content: string): Promise<CodeAction[]> {
    // Parse code blocks and suggested actions from AI response
    // This is a simplified implementation
    const codeActions: CodeAction[] = [];
    
    // Look for code blocks with action indicators
    const codeBlockRegex = /```(\w+)?\s*\n([\s\S]*?)```/g;
    let match;
    
    while ((match = codeBlockRegex.exec(content)) !== null) {
      const language = match[1] || '';
      const code = match[2];
      
      // Check if this looks like a file creation/modification
      if (code.includes('// File:') || code.includes('# File:')) {
        // Parse file path and action
        // This would be more sophisticated in production
      }
    }
    
    return codeActions;
  }
  
  private getSelectedCode(): string | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.selection.isEmpty) {
      return undefined;
    }
    
    return editor.document.getText(editor.selection);
  }
  
  private getProjectId(): string {
    // Get project ID from workspace
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
      return workspaceFolder.uri.toString();
    }
    return 'default';
  }
  
  private async transcribeVoice(voice: VoiceInput): Promise<string> {
    // This would integrate with a speech-to-text service
    // For now, return placeholder
    return 'Transcribed voice message';
  }
}