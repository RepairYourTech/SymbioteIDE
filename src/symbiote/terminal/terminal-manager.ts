/**
 * Terminal Manager
 * 
 * Central management system for AI-powered terminal sessions
 */

import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import { 
  TerminalSession, 
  TerminalConfig, 
  TerminalCommand,
  TerminalEvent,
  TerminalEventType,
  AIAssistanceMode,
  TerminalDimensions
} from '../../types/terminal-api';
import { TerminalSessionHandler } from './terminal-session';
import { AICommandProcessor } from './ai-command-processor';
import { generateId } from '../utils/id-generator';

export interface TerminalManagerOptions {
  maxSessions?: number;
  defaultShell?: string;
  aiModeDefault?: AIAssistanceMode;
  workspaceRoot?: string;
}

export class TerminalManager extends EventEmitter {
  private sessions: Map<string, TerminalSessionHandler> = new Map();
  private vscodeTerminals: Map<string, vscode.Terminal> = new Map();
  private aiProcessor: AICommandProcessor;
  private options: Required<TerminalManagerOptions>;
  private commandHistory: Map<string, TerminalCommand[]> = new Map();
  
  constructor(
    aiProcessor: AICommandProcessor,
    options: TerminalManagerOptions = {}
  ) {
    super();
    
    this.aiProcessor = aiProcessor;
    this.options = {
      maxSessions: options.maxSessions || 10,
      defaultShell: options.defaultShell || process.env.SHELL || 'bash',
      aiModeDefault: options.aiModeDefault || {
        enabled: true,
        level: 'assist',
        features: {
          command_suggestions: true,
          error_explanation: true,
          command_completion: true,
          script_generation: true,
          output_analysis: true,
          workflow_automation: true,
          safety_checks: true
        }
      },
      workspaceRoot: options.workspaceRoot || vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd()
    };
    
    // Subscribe to VS Code terminal events
    this.setupVSCodeIntegration();
  }
  
  /**
   * Create a new terminal session
   */
  async createSession(config?: TerminalConfig): Promise<TerminalSession> {
    if (this.sessions.size >= this.options.maxSessions) {
      throw new Error(`Maximum number of sessions (${this.options.maxSessions}) reached`);
    }
    
    const sessionId = generateId('session');
    const now = new Date().toISOString();
    
    // Merge config with defaults
    const fullConfig: TerminalConfig = {
      shell: config?.shell || this.options.defaultShell,
      workingDirectory: config?.workingDirectory || this.options.workspaceRoot,
      env: {
        ...process.env,
        ...config?.env,
        SYMBIOTE_SESSION_ID: sessionId,
        SYMBIOTE_AI_ENABLED: 'true'
      },
      dimensions: config?.dimensions || { cols: 80, rows: 24 },
      options: {
        convertEol: true,
        fontFamily: 'monospace',
        fontSize: 14,
        scrollback: 10000,
        ...config?.options
      }
    };
    
    // Create VS Code terminal
    const terminal = vscode.window.createTerminal({
      name: `SymbioteIDE Terminal ${sessionId}`,
      shellPath: fullConfig.shell,
      cwd: fullConfig.workingDirectory,
      env: fullConfig.env as any,
      iconPath: new vscode.ThemeIcon('terminal')
    });
    
    this.vscodeTerminals.set(sessionId, terminal);
    
    // Create session handler
    const sessionHandler = new TerminalSessionHandler(
      sessionId,
      terminal,
      fullConfig,
      this.aiProcessor
    );
    
    // Set up event forwarding
    sessionHandler.on('command', (command: TerminalCommand) => {
      this.trackCommand(sessionId, command);
      this.emitEvent('command_completed', sessionId, command);
    });
    
    sessionHandler.on('output', (data: any) => {
      this.emitEvent('output_received', sessionId, data);
    });
    
    sessionHandler.on('error', (error: any) => {
      this.emitEvent('error_occurred', sessionId, error);
    });
    
    this.sessions.set(sessionId, sessionHandler);
    
    // Create session object
    const session: TerminalSession = {
      id: sessionId,
      name: `Session ${sessionId}`,
      shell: fullConfig.shell!,
      workingDirectory: fullConfig.workingDirectory!,
      status: 'active',
      created_at: now,
      last_activity: now,
      dimensions: fullConfig.dimensions!,
      buffer_size: 0,
      ai_mode: this.options.aiModeDefault
    };
    
    this.emitEvent('session_created', sessionId, session);
    
    return session;
  }
  
  /**
   * Get a session by ID
   */
  async getSession(sessionId: string): Promise<TerminalSession> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    return handler.getSessionInfo();
  }
  
  /**
   * List all sessions
   */
  async listSessions(): Promise<TerminalSession[]> {
    const sessions: TerminalSession[] = [];
    
    for (const handler of this.sessions.values()) {
      sessions.push(await handler.getSessionInfo());
    }
    
    return sessions;
  }
  
  /**
   * Terminate a session
   */
  async terminateSession(sessionId: string): Promise<void> {
    const handler = this.sessions.get(sessionId);
    const terminal = this.vscodeTerminals.get(sessionId);
    
    if (handler) {
      await handler.terminate();
      this.sessions.delete(sessionId);
    }
    
    if (terminal) {
      terminal.dispose();
      this.vscodeTerminals.delete(sessionId);
    }
    
    this.commandHistory.delete(sessionId);
    this.emitEvent('session_terminated', sessionId, {});
  }
  
  /**
   * Execute a command in a session
   */
  async executeCommand(sessionId: string, command: string): Promise<TerminalCommand> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    this.emitEvent('command_started', sessionId, { command });
    
    return await handler.executeCommand(command);
  }
  
  /**
   * Get command history for a session
   */
  async getCommandHistory(sessionId: string, limit?: number): Promise<TerminalCommand[]> {
    const history = this.commandHistory.get(sessionId) || [];
    
    if (limit) {
      return history.slice(-limit);
    }
    
    return history;
  }
  
  /**
   * Send input to a session
   */
  async sendInput(sessionId: string, data: string): Promise<void> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    await handler.sendInput(data);
  }
  
  /**
   * Resize a terminal
   */
  async resize(sessionId: string, dimensions: TerminalDimensions): Promise<void> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    await handler.resize(dimensions);
  }
  
  /**
   * Get terminal buffer
   */
  async getBuffer(sessionId: string, lines?: number): Promise<string[]> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    return await handler.getBuffer(lines);
  }
  
  /**
   * Clear terminal
   */
  async clear(sessionId: string): Promise<void> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    await handler.clear();
  }
  
  /**
   * Update AI assistance mode
   */
  async updateAIMode(sessionId: string, mode: AIAssistanceMode): Promise<void> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    await handler.updateAIMode(mode);
  }
  
  /**
   * Set environment variables
   */
  async setEnvironment(sessionId: string, env: Record<string, string>): Promise<void> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    await handler.setEnvironment(env);
  }
  
  /**
   * Change working directory
   */
  async changeDirectory(sessionId: string, path: string): Promise<void> {
    const handler = this.sessions.get(sessionId);
    if (!handler) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    await handler.changeDirectory(path);
  }
  
  /**
   * Get AI command processor
   */
  getAIProcessor(): AICommandProcessor {
    return this.aiProcessor;
  }
  
  /**
   * Shutdown the terminal manager
   */
  async shutdown(): Promise<void> {
    // Terminate all sessions
    const sessionIds = Array.from(this.sessions.keys());
    
    for (const sessionId of sessionIds) {
      await this.terminateSession(sessionId).catch(console.error);
    }
    
    this.removeAllListeners();
  }
  
  // Private methods
  
  private setupVSCodeIntegration(): void {
    // Watch for terminal close events
    vscode.window.onDidCloseTerminal((terminal) => {
      // Find session ID for this terminal
      for (const [sessionId, vscodeTerminal] of this.vscodeTerminals) {
        if (vscodeTerminal === terminal) {
          this.terminateSession(sessionId).catch(console.error);
          break;
        }
      }
    });
    
    // Watch for active terminal changes
    vscode.window.onDidChangeActiveTerminal((terminal) => {
      if (terminal) {
        // Could emit an event for UI updates
      }
    });
  }
  
  private trackCommand(sessionId: string, command: TerminalCommand): void {
    if (!this.commandHistory.has(sessionId)) {
      this.commandHistory.set(sessionId, []);
    }
    
    const history = this.commandHistory.get(sessionId)!;
    history.push(command);
    
    // Limit history size
    if (history.length > 1000) {
      history.shift();
    }
  }
  
  private emitEvent(type: TerminalEventType, sessionId: string, data: any): void {
    const event: TerminalEvent = {
      type,
      session_id: sessionId,
      timestamp: new Date().toISOString(),
      data
    };
    
    this.emit('terminal-event', event);
    this.emit(`terminal-event-${type}`, event);
  }
}

// Utility function - this would normally be in a separate utils file
function generateId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}