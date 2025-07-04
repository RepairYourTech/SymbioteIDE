/**
 * Terminal Session Handler
 * 
 * Manages individual terminal sessions with AI integration
 */

import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import { 
  TerminalSession,
  TerminalConfig,
  TerminalCommand,
  TerminalOutput,
  AIAssistanceMode,
  TerminalDimensions,
  AICommandMetadata
} from '../../types/terminal-api';
import { AICommandProcessor } from './ai-command-processor';

export class TerminalSessionHandler extends EventEmitter {
  private sessionId: string;
  private terminal: vscode.Terminal;
  private config: TerminalConfig;
  private aiProcessor: AICommandProcessor;
  private aiMode: AIAssistanceMode;
  private status: 'active' | 'idle' | 'busy' | 'terminated' = 'idle';
  private commandBuffer: string = '';
  private outputBuffer: string[] = [];
  private currentCommand: TerminalCommand | null = null;
  private workingDirectory: string;
  private environment: Record<string, string>;
  private lastActivity: Date;
  private createdAt: Date;
  private dimensions: TerminalDimensions;
  
  constructor(
    sessionId: string,
    terminal: vscode.Terminal,
    config: TerminalConfig,
    aiProcessor: AICommandProcessor
  ) {
    super();
    
    this.sessionId = sessionId;
    this.terminal = terminal;
    this.config = config;
    this.aiProcessor = aiProcessor;
    this.workingDirectory = config.workingDirectory || process.cwd();
    this.environment = config.env || {};
    this.dimensions = config.dimensions || { cols: 80, rows: 24 };
    this.createdAt = new Date();
    this.lastActivity = new Date();
    
    // Initialize AI mode
    this.aiMode = {
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
    };
    
    // Set up terminal event handling
    this.setupTerminalHandlers();
  }
  
  /**
   * Get session information
   */
  async getSessionInfo(): Promise<TerminalSession> {
    return {
      id: this.sessionId,
      name: this.terminal.name,
      shell: this.config.shell || 'bash',
      workingDirectory: this.workingDirectory,
      status: this.status,
      created_at: this.createdAt.toISOString(),
      last_activity: this.lastActivity.toISOString(),
      dimensions: this.dimensions,
      buffer_size: this.outputBuffer.length,
      ai_mode: this.aiMode
    };
  }
  
  /**
   * Execute a command
   */
  async executeCommand(command: string): Promise<TerminalCommand> {
    this.status = 'busy';
    this.lastActivity = new Date();
    
    const commandId = this.generateCommandId();
    const startTime = Date.now();
    
    // Check if this is an AI command (starts with #)
    const isAICommand = command.trim().startsWith('#');
    let actualCommand = command;
    let aiMetadata: AICommandMetadata | undefined;
    
    if (isAICommand && this.aiMode.enabled) {
      // Process with AI
      const aiResult = await this.aiProcessor.processCommand(
        command.substring(1).trim(),
        {
          sessionId: this.sessionId,
          workingDirectory: this.workingDirectory,
          shell: this.config.shell || 'bash',
          history: this.getRecentHistory()
        }
      );
      
      if (aiResult.command) {
        actualCommand = aiResult.command;
        aiMetadata = {
          suggested: true,
          confidence: aiResult.confidence,
          explanation: aiResult.explanation,
          alternatives: aiResult.alternatives,
          warnings: aiResult.warnings
        };
        
        // Show AI interpretation to user
        this.terminal.sendText(`# AI: ${aiResult.explanation || 'Interpreting command...'}`);
        this.terminal.sendText(`# Command: ${actualCommand}`);
        
        // Safety check if enabled
        if (this.aiMode.features.safety_checks && aiResult.warnings?.length) {
          this.terminal.sendText(`# ⚠️ Warnings: ${aiResult.warnings.join(', ')}`);
          // Could implement confirmation prompt here
        }
      }
    }
    
    // Create command record
    this.currentCommand = {
      id: commandId,
      session_id: this.sessionId,
      command: actualCommand,
      timestamp: new Date().toISOString(),
      working_directory: this.workingDirectory,
      ai_metadata: aiMetadata
    };
    
    // Clear output buffer for new command
    this.commandBuffer = '';
    
    // Execute the command
    this.terminal.sendText(actualCommand);
    
    // Wait for command completion (simplified - in real implementation would use PTY)
    return new Promise((resolve) => {
      // Simulate command execution with timeout
      const timeout = setTimeout(() => {
        const endTime = Date.now();
        
        if (this.currentCommand) {
          this.currentCommand.duration = endTime - startTime;
          this.currentCommand.exit_code = 0; // Would get from PTY
          this.currentCommand.output = {
            stdout: this.commandBuffer,
            stderr: '',
            combined: this.commandBuffer,
            truncated: false,
            size: this.commandBuffer.length
          };
          
          this.status = 'idle';
          this.emit('command', this.currentCommand);
          resolve(this.currentCommand);
          this.currentCommand = null;
        }
      }, 100); // In real implementation, would wait for actual command completion
    });
  }
  
  /**
   * Send input to terminal
   */
  async sendInput(data: string): Promise<void> {
    this.terminal.sendText(data, false);
    this.lastActivity = new Date();
  }
  
  /**
   * Resize terminal
   */
  async resize(dimensions: TerminalDimensions): Promise<void> {
    this.dimensions = dimensions;
    // VS Code handles terminal resizing automatically
  }
  
  /**
   * Get terminal buffer
   */
  async getBuffer(lines?: number): Promise<string[]> {
    if (lines) {
      return this.outputBuffer.slice(-lines);
    }
    return [...this.outputBuffer];
  }
  
  /**
   * Clear terminal
   */
  async clear(): Promise<void> {
    this.terminal.sendText('clear');
    this.outputBuffer = [];
  }
  
  /**
   * Update AI mode
   */
  async updateAIMode(mode: AIAssistanceMode): Promise<void> {
    this.aiMode = mode;
  }
  
  /**
   * Set environment variables
   */
  async setEnvironment(env: Record<string, string>): Promise<void> {
    this.environment = { ...this.environment, ...env };
    
    // Export environment variables to the terminal
    for (const [key, value] of Object.entries(env)) {
      this.terminal.sendText(`export ${key}="${value}"`);
    }
  }
  
  /**
   * Change working directory
   */
  async changeDirectory(path: string): Promise<void> {
    this.terminal.sendText(`cd "${path}"`);
    this.workingDirectory = path;
  }
  
  /**
   * Terminate session
   */
  async terminate(): Promise<void> {
    this.status = 'terminated';
    this.terminal.dispose();
    this.removeAllListeners();
  }
  
  // Private methods
  
  private setupTerminalHandlers(): void {
    // In a real implementation, we would use node-pty or similar
    // to capture terminal output. For now, we'll simulate it.
    
    // VS Code doesn't provide direct access to terminal output
    // This is a limitation we need to work around
    
    // Set up periodic status check
    const statusInterval = setInterval(() => {
      if (this.status === 'terminated') {
        clearInterval(statusInterval);
        return;
      }
      
      // Update status based on activity
      const idleTime = Date.now() - this.lastActivity.getTime();
      if (idleTime > 60000 && this.status === 'active') {
        this.status = 'idle';
      }
    }, 5000);
  }
  
  private generateCommandId(): string {
    return `cmd-${this.sessionId}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
  
  private getRecentHistory(): string[] {
    // Return last 10 commands from output buffer
    // In real implementation, would track actual command history
    return this.outputBuffer.slice(-10);
  }
}