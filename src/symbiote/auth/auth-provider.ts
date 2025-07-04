/**
 * VS Code Authentication Provider
 * 
 * Integrates Supabase auth with VS Code's authentication API
 */

import * as vscode from 'vscode';
import { AuthManager } from './auth-manager';
import { Session } from './types';

export class SymbioteAuthenticationProvider implements vscode.AuthenticationProvider, vscode.Disposable {
  private static readonly id = 'symbiote-supabase';
  private static readonly label = 'SymbioteIDE';
  
  private _onDidChangeSessions = new vscode.EventEmitter<vscode.AuthenticationProviderAuthenticationSessionsChangeEvent>();
  readonly onDidChangeSessions = this._onDidChangeSessions.event;
  
  private authManager: AuthManager;
  
  constructor(authManager: AuthManager) {
    this.authManager = authManager;
    
    // Listen for auth state changes
    this.authManager.on('auth:signed-in', () => {
      this._onDidChangeSessions.fire({ added: [this.createSession()], removed: [], changed: [] });
    });
    
    this.authManager.on('auth:signed-out', () => {
      this._onDidChangeSessions.fire({ added: [], removed: [this.createSession()], changed: [] });
    });
    
    this.authManager.on('auth:token-refreshed', () => {
      this._onDidChangeSessions.fire({ added: [], removed: [], changed: [this.createSession()] });
    });
  }

  /**
   * Get authentication sessions
   */
  async getSessions(scopes?: readonly string[]): Promise<readonly vscode.AuthenticationSession[]> {
    const session = this.authManager.getSession();
    if (!session) {
      return [];
    }
    
    return [this.createSession()];
  }

  /**
   * Create a new session (sign in)
   */
  async createSession(scopes?: readonly string[]): Promise<vscode.AuthenticationSession> {
    const user = this.authManager.getUser();
    const session = this.authManager.getSession();
    
    if (!user || !session) {
      // Show sign in dialog
      await this.authManager.showAuthMenu();
      
      // After sign in, get the new session
      const newUser = this.authManager.getUser();
      const newSession = this.authManager.getSession();
      
      if (!newUser || !newSession) {
        throw new Error('Authentication failed');
      }
    }
    
    return this.createSession();
  }

  /**
   * Remove session (sign out)
   */
  async removeSession(sessionId: string): Promise<void> {
    await this.authManager.signOut();
  }

  /**
   * Create VS Code session from Supabase session
   */
  private createSession(): vscode.AuthenticationSession {
    const user = this.authManager.getUser();
    const session = this.authManager.getSession();
    
    if (!user || !session) {
      throw new Error('No active session');
    }
    
    return {
      id: user.id,
      accessToken: session.access_token,
      account: {
        id: user.id,
        label: user.email
      },
      scopes: []
    };
  }

  /**
   * Register the authentication provider
   */
  static register(context: vscode.ExtensionContext, authManager: AuthManager): vscode.Disposable {
    const provider = new SymbioteAuthenticationProvider(authManager);
    
    const disposable = vscode.authentication.registerAuthenticationProvider(
      SymbioteAuthenticationProvider.id,
      SymbioteAuthenticationProvider.label,
      provider,
      { supportsMultipleAccounts: false }
    );
    
    context.subscriptions.push(disposable);
    context.subscriptions.push(provider);
    
    return disposable;
  }

  dispose(): void {
    this._onDidChangeSessions.dispose();
  }
}