/**
 * Authentication Manager
 * 
 * Simple auth management with Supabase
 */

import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import { getSupabase } from './supabase-client';
import { User, Session, AuthState, AuthCredentials } from './types';

export class AuthManager extends EventEmitter {
  private state: AuthState = {
    user: null,
    session: null,
    loading: false,
    error: null
  };
  
  private statusBarItem: vscode.StatusBarItem;
  
  constructor() {
    super();
    
    // Create status bar item
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );
    this.statusBarItem.command = 'symbiote.showAuthMenu';
    this.updateStatusBar();
  }

  /**
   * Initialize auth manager and check for existing session
   */
  async initialize(): Promise<boolean> {
    this.state.loading = true;
    this.updateStatusBar();
    
    try {
      const supabase = getSupabase();
      
      // Check for existing session
      const { data: { session } } = await supabase.auth.getSession();
      
      if (session) {
        this.state.user = session.user as User;
        this.state.session = session as Session;
        this.emit('auth:signed-in', this.state.user);
        return true;
      }
      
      // Listen for auth state changes
      supabase.auth.onAuthStateChange((event, session) => {
        switch (event) {
          case 'SIGNED_IN':
            this.state.user = session?.user as User;
            this.state.session = session as Session;
            this.emit('auth:signed-in', this.state.user);
            break;
            
          case 'SIGNED_OUT':
            this.state.user = null;
            this.state.session = null;
            this.emit('auth:signed-out');
            vscode.window.showWarningMessage('You have been signed out. Please sign in to continue using SymbioteIDE.');
            this.showAuthMenu();
            break;
            
          case 'TOKEN_REFRESHED':
            this.state.session = session as Session;
            this.emit('auth:token-refreshed');
            break;
        }
        
        this.updateStatusBar();
      });
      
      // No existing session - require login
      return false;
      
    } catch (error) {
      this.state.error = error instanceof Error ? error.message : 'Authentication failed';
      console.error('Auth initialization error:', error);
      return false;
    } finally {
      this.state.loading = false;
      this.updateStatusBar();
    }
  }

  /**
   * Require authentication - shows login dialog if not authenticated
   */
  async requireAuth(): Promise<boolean> {
    if (this.isAuthenticated()) {
      return true;
    }
    
    const result = await vscode.window.showWarningMessage(
      'SymbioteIDE requires authentication to continue.',
      'Sign In',
      'Sign Up'
    );
    
    if (result === 'Sign In') {
      await this.showSignInDialog();
    } else if (result === 'Sign Up') {
      await this.showSignUpDialog();
    }
    
    return this.isAuthenticated();
  }

  /**
   * Sign in with email and password
   */
  async signIn(credentials: AuthCredentials): Promise<void> {
    this.state.loading = true;
    this.state.error = null;
    this.updateStatusBar();
    
    try {
      const supabase = getSupabase();
      const { data, error } = await supabase.auth.signInWithPassword({
        email: credentials.email,
        password: credentials.password
      });
      
      if (error) throw error;
      
      if (data.session) {
        this.state.user = data.user as User;
        this.state.session = data.session as Session;
        vscode.window.showInformationMessage(`Welcome back, ${data.user.email}!`);
      }
      
    } catch (error) {
      this.state.error = error instanceof Error ? error.message : 'Sign in failed';
      vscode.window.showErrorMessage(`Sign in failed: ${this.state.error}`);
      throw error;
    } finally {
      this.state.loading = false;
      this.updateStatusBar();
    }
  }

  /**
   * Sign up with email and password
   */
  async signUp(credentials: AuthCredentials): Promise<void> {
    this.state.loading = true;
    this.state.error = null;
    this.updateStatusBar();
    
    try {
      const supabase = getSupabase();
      const { data, error } = await supabase.auth.signUp({
        email: credentials.email,
        password: credentials.password
      });
      
      if (error) throw error;
      
      if (data.session) {
        this.state.user = data.user as User;
        this.state.session = data.session as Session;
        vscode.window.showInformationMessage(`Welcome to SymbioteIDE, ${data.user.email}!`);
      } else if (data.user) {
        vscode.window.showInformationMessage('Please check your email to confirm your account.');
      }
      
    } catch (error) {
      this.state.error = error instanceof Error ? error.message : 'Sign up failed';
      vscode.window.showErrorMessage(`Sign up failed: ${this.state.error}`);
      throw error;
    } finally {
      this.state.loading = false;
      this.updateStatusBar();
    }
  }

  /**
   * Sign out
   */
  async signOut(): Promise<void> {
    this.state.loading = true;
    this.updateStatusBar();
    
    try {
      const supabase = getSupabase();
      const { error } = await supabase.auth.signOut();
      
      if (error) throw error;
      
      this.state.user = null;
      this.state.session = null;
      vscode.window.showInformationMessage('Signed out successfully');
      
    } catch (error) {
      this.state.error = error instanceof Error ? error.message : 'Sign out failed';
      vscode.window.showErrorMessage(`Sign out failed: ${this.state.error}`);
      throw error;
    } finally {
      this.state.loading = false;
      this.updateStatusBar();
    }
  }

  /**
   * Get current user
   */
  getUser(): User | null {
    return this.state.user;
  }

  /**
   * Get current session
   */
  getSession(): Session | null {
    return this.state.session;
  }

  /**
   * Check if user is authenticated
   */
  isAuthenticated(): boolean {
    return !!this.state.user && !!this.state.session;
  }

  /**
   * Get auth headers for API requests
   */
  getAuthHeaders(): Record<string, string> {
    if (!this.state.session) {
      return {};
    }
    
    return {
      'Authorization': `Bearer ${this.state.session.access_token}`
    };
  }

  /**
   * Update status bar
   */
  private updateStatusBar(): void {
    if (this.state.loading) {
      this.statusBarItem.text = '$(sync~spin) Authenticating...';
    } else if (this.state.user) {
      this.statusBarItem.text = `$(account) ${this.state.user.email}`;
      this.statusBarItem.tooltip = 'Click to manage account';
    } else {
      this.statusBarItem.text = '$(account) Sign In';
      this.statusBarItem.tooltip = 'Click to sign in';
    }
    
    this.statusBarItem.show();
  }

  /**
   * Show auth menu
   */
  async showAuthMenu(): Promise<void> {
    if (this.isAuthenticated()) {
      const choice = await vscode.window.showQuickPick([
        { label: '$(account) Account', description: this.state.user?.email },
        { label: '$(sign-out) Sign Out' }
      ], {
        placeHolder: 'Account Options'
      });
      
      if (choice?.label.includes('Sign Out')) {
        await this.signOut();
      }
    } else {
      const choice = await vscode.window.showQuickPick([
        { label: '$(sign-in) Sign In' },
        { label: '$(add) Sign Up' }
      ], {
        placeHolder: 'Authentication Options'
      });
      
      if (choice?.label.includes('Sign In')) {
        await this.showSignInDialog();
      } else if (choice?.label.includes('Sign Up')) {
        await this.showSignUpDialog();
      }
    }
  }

  /**
   * Show sign in dialog
   */
  private async showSignInDialog(): Promise<void> {
    const email = await vscode.window.showInputBox({
      prompt: 'Email',
      placeHolder: 'user@example.com',
      validateInput: (value) => {
        if (!value || !value.includes('@')) {
          return 'Please enter a valid email';
        }
        return null;
      }
    });
    
    if (!email) return;
    
    const password = await vscode.window.showInputBox({
      prompt: 'Password',
      password: true,
      validateInput: (value) => {
        if (!value || value.length < 6) {
          return 'Password must be at least 6 characters';
        }
        return null;
      }
    });
    
    if (!password) return;
    
    await this.signIn({ email, password });
  }

  /**
   * Show sign up dialog
   */
  private async showSignUpDialog(): Promise<void> {
    const email = await vscode.window.showInputBox({
      prompt: 'Email',
      placeHolder: 'user@example.com',
      validateInput: (value) => {
        if (!value || !value.includes('@')) {
          return 'Please enter a valid email';
        }
        return null;
      }
    });
    
    if (!email) return;
    
    const password = await vscode.window.showInputBox({
      prompt: 'Password',
      password: true,
      validateInput: (value) => {
        if (!value || value.length < 6) {
          return 'Password must be at least 6 characters';
        }
        return null;
      }
    });
    
    if (!password) return;
    
    const confirmPassword = await vscode.window.showInputBox({
      prompt: 'Confirm Password',
      password: true,
      validateInput: (value) => {
        if (value !== password) {
          return 'Passwords do not match';
        }
        return null;
      }
    });
    
    if (!confirmPassword) return;
    
    await this.signUp({ email, password });
  }

  /**
   * Dispose
   */
  dispose(): void {
    this.statusBarItem.dispose();
    this.removeAllListeners();
  }
}