/**
 * Conversation Manager
 * 
 * Manages chat persistence and history
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';
import { 
  ChatSession,
  ChatMessage,
  SessionMetadata
} from '../../types/chat-api';

export class ConversationManager {
  private extensionUri: vscode.Uri;
  private storageUri: vscode.Uri;
  private sessions: Map<string, ChatSession> = new Map();
  private initialized: boolean = false;
  
  constructor(extensionUri: vscode.Uri) {
    this.extensionUri = extensionUri;
    
    // Determine storage location
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
      this.storageUri = vscode.Uri.joinPath(workspaceFolder.uri, '.symbiote', 'chat');
    } else {
      this.storageUri = vscode.Uri.joinPath(extensionUri, 'chat-history');
    }
  }
  
  /**
   * Initialize the conversation manager
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }
    
    // Ensure storage directory exists
    await vscode.workspace.fs.createDirectory(this.storageUri);
    
    // Load existing sessions
    await this.loadSessions();
    
    this.initialized = true;
  }
  
  /**
   * Create a new chat session
   */
  async createSession(title?: string): Promise<ChatSession> {
    await this.initialize();
    
    const session: ChatSession = {
      id: this.generateId(),
      title: title || `Chat ${new Date().toLocaleString()}`,
      messages: [],
      createdAt: new Date(),
      updatedAt: new Date(),
      metadata: {
        projectId: this.getProjectId(),
        workspaceFolder: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
      }
    };
    
    this.sessions.set(session.id, session);
    await this.saveSession(session);
    
    return session;
  }
  
  /**
   * Get a session by ID
   */
  async getSession(sessionId: string): Promise<ChatSession | null> {
    await this.initialize();
    return this.sessions.get(sessionId) || null;
  }
  
  /**
   * List all sessions
   */
  async listSessions(): Promise<ChatSession[]> {
    await this.initialize();
    
    return Array.from(this.sessions.values())
      .sort((a, b) => b.updatedAt.getTime() - a.updatedAt.getTime());
  }
  
  /**
   * Update a session
   */
  async updateSession(session: ChatSession): Promise<void> {
    session.updatedAt = new Date();
    this.sessions.set(session.id, session);
    await this.saveSession(session);
  }
  
  /**
   * Delete a session
   */
  async deleteSession(sessionId: string): Promise<void> {
    this.sessions.delete(sessionId);
    
    // Delete from storage
    const sessionUri = vscode.Uri.joinPath(this.storageUri, `${sessionId}.json`);
    try {
      await vscode.workspace.fs.delete(sessionUri);
    } catch (error) {
      // Ignore if file doesn't exist
    }
  }
  
  /**
   * Search messages across all sessions
   */
  async searchMessages(query: string): Promise<ChatMessage[]> {
    const results: ChatMessage[] = [];
    const lowerQuery = query.toLowerCase();
    
    for (const session of this.sessions.values()) {
      for (const message of session.messages) {
        if (message.content.toLowerCase().includes(lowerQuery)) {
          results.push(message);
        }
      }
    }
    
    return results;
  }
  
  /**
   * Export a session to JSON
   */
  async exportSession(sessionId: string): Promise<string> {
    const session = await this.getSession(sessionId);
    if (!session) {
      throw new Error('Session not found');
    }
    
    return JSON.stringify(session, null, 2);
  }
  
  /**
   * Import a session from JSON
   */
  async importSession(data: string): Promise<ChatSession> {
    const sessionData = JSON.parse(data);
    
    // Generate new ID to avoid conflicts
    const session: ChatSession = {
      ...sessionData,
      id: this.generateId(),
      createdAt: new Date(sessionData.createdAt),
      updatedAt: new Date(sessionData.updatedAt),
      messages: sessionData.messages.map((msg: any) => ({
        ...msg,
        timestamp: new Date(msg.timestamp)
      }))
    };
    
    this.sessions.set(session.id, session);
    await this.saveSession(session);
    
    return session;
  }
  
  /**
   * Clean up old sessions based on retention policy
   */
  async cleanup(maxHistorySize: number): Promise<void> {
    const sessions = await this.listSessions();
    
    if (sessions.length > maxHistorySize) {
      // Delete oldest sessions
      const toDelete = sessions.slice(maxHistorySize);
      
      for (const session of toDelete) {
        await this.deleteSession(session.id);
      }
    }
  }
  
  // Private methods
  
  private async loadSessions(): Promise<void> {
    try {
      const files = await vscode.workspace.fs.readDirectory(this.storageUri);
      
      for (const [name, type] of files) {
        if (type === vscode.FileType.File && name.endsWith('.json')) {
          const uri = vscode.Uri.joinPath(this.storageUri, name);
          const content = await vscode.workspace.fs.readFile(uri);
          const sessionData = JSON.parse(content.toString());
          
          const session: ChatSession = {
            ...sessionData,
            createdAt: new Date(sessionData.createdAt),
            updatedAt: new Date(sessionData.updatedAt),
            messages: sessionData.messages.map((msg: any) => ({
              ...msg,
              timestamp: new Date(msg.timestamp)
            }))
          };
          
          this.sessions.set(session.id, session);
        }
      }
    } catch (error) {
      console.error('Failed to load chat sessions:', error);
    }
  }
  
  private async saveSession(session: ChatSession): Promise<void> {
    const uri = vscode.Uri.joinPath(this.storageUri, `${session.id}.json`);
    const content = Buffer.from(JSON.stringify(session, null, 2));
    
    await vscode.workspace.fs.writeFile(uri, content);
  }
  
  private generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substring(2);
  }
  
  private getProjectId(): string {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
      return workspaceFolder.uri.toString();
    }
    return 'default';
  }
}