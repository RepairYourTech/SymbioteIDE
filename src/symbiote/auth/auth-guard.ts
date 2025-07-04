/**
 * Authentication Guard
 * 
 * Middleware for protecting features
 */

import * as vscode from 'vscode';
import { AuthManager } from './auth-manager';

export class AuthGuard {
  private static authManager: AuthManager | null = null;

  /**
   * Set the auth manager instance
   */
  static setAuthManager(authManager: AuthManager): void {
    this.authManager = authManager;
  }

  /**
   * Check if user is authenticated
   */
  static isAuthenticated(): boolean {
    if (!this.authManager) {
      console.error('AuthGuard: AuthManager not set');
      return false;
    }
    return this.authManager.isAuthenticated();
  }

  /**
   * Require authentication for a function
   */
  static async requireAuth<T>(
    action: () => T | Promise<T>,
    showPrompt: boolean = true
  ): Promise<T | undefined> {
    if (!this.authManager) {
      vscode.window.showErrorMessage('Authentication system not initialized');
      return undefined;
    }

    if (!this.authManager.isAuthenticated()) {
      if (showPrompt) {
        const authenticated = await this.authManager.requireAuth();
        if (!authenticated) {
          return undefined;
        }
      } else {
        return undefined;
      }
    }

    return await action();
  }

  /**
   * Create an authenticated wrapper for a function
   */
  static wrap<T extends (...args: any[]) => any>(
    fn: T,
    showPrompt: boolean = true
  ): T {
    return (async (...args: Parameters<T>) => {
      return await this.requireAuth(() => fn(...args), showPrompt);
    }) as T;
  }

  /**
   * Check auth for webview HTML
   */
  static getAuthenticatedHTML(html: string): string {
    if (!this.isAuthenticated()) {
      return `
        <!DOCTYPE html>
        <html>
          <head>
            <style>
              body {
                display: flex;
                align-items: center;
                justify-content: center;
                height: 100vh;
                margin: 0;
                font-family: var(--vscode-font-family);
                background: var(--vscode-editor-background);
                color: var(--vscode-editor-foreground);
              }
              .auth-required {
                text-align: center;
                padding: 2rem;
              }
              h2 {
                color: var(--vscode-errorForeground);
                margin-bottom: 1rem;
              }
              p {
                margin-bottom: 1.5rem;
                opacity: 0.8;
              }
              button {
                background: var(--vscode-button-background);
                color: var(--vscode-button-foreground);
                border: none;
                padding: 0.5rem 1rem;
                cursor: pointer;
                font-size: 14px;
              }
              button:hover {
                background: var(--vscode-button-hoverBackground);
              }
            </style>
          </head>
          <body>
            <div class="auth-required">
              <h2>Authentication Required</h2>
              <p>Please sign in to use SymbioteIDE features</p>
              <button onclick="acquireVsCodeApi().postMessage({command: 'requireAuth'})">
                Sign In
              </button>
            </div>
          </body>
        </html>
      `;
    }
    return html;
  }
}