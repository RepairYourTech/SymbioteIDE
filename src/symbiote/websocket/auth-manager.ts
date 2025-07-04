/**
 * Auth Manager
 * 
 * Handles WebSocket authentication and authorization
 */

import { Logger } from '../utils/logger';
import { SocketAuth } from './types';

export class AuthManager {
  private logger = new Logger('AuthManager');
  private validTokens = new Set<string>();
  private validApiKeys = new Set<string>();
  
  constructor() {
    // Load valid tokens/keys from environment or config
    this.loadAuthConfig();
  }
  
  /**
   * Validate authentication
   */
  async validateAuth(auth: SocketAuth): Promise<boolean> {
    // Allow unauthenticated in development
    if (process.env.NODE_ENV === 'development' && !auth.token && !auth.apiKey) {
      this.logger.debug('Allowing unauthenticated connection in development');
      return true;
    }
    
    // Validate token
    if (auth.token) {
      return this.validateToken(auth.token);
    }
    
    // Validate API key
    if (auth.apiKey) {
      return this.validateApiKey(auth.apiKey);
    }
    
    // Check user/agent ID (basic validation)
    if (auth.userId || auth.agentId) {
      return true; // Would validate against database in production
    }
    
    this.logger.warn('Authentication failed', { auth });
    return false;
  }
  
  /**
   * Validate JWT token
   */
  private async validateToken(token: string): Promise<boolean> {
    // In production, this would verify JWT
    return this.validTokens.has(token);
  }
  
  /**
   * Validate API key
   */
  private async validateApiKey(apiKey: string): Promise<boolean> {
    // In production, this would check against database
    return this.validApiKeys.has(apiKey);
  }
  
  /**
   * Check permissions
   */
  hasPermission(auth: SocketAuth, permission: string): boolean {
    if (!auth.permissions) {
      return false;
    }
    
    return auth.permissions.includes(permission) || 
           auth.permissions.includes('*');
  }
  
  /**
   * Load auth configuration
   */
  private loadAuthConfig(): void {
    // Load from environment
    const validTokensEnv = process.env.WEBSOCKET_VALID_TOKENS;
    if (validTokensEnv) {
      validTokensEnv.split(',').forEach(token => {
        this.validTokens.add(token.trim());
      });
    }
    
    const validApiKeysEnv = process.env.WEBSOCKET_VALID_API_KEYS;
    if (validApiKeysEnv) {
      validApiKeysEnv.split(',').forEach(key => {
        this.validApiKeys.add(key.trim());
      });
    }
    
    this.logger.info('Auth configuration loaded', {
      tokens: this.validTokens.size,
      apiKeys: this.validApiKeys.size
    });
  }
  
  /**
   * Add valid token
   */
  addToken(token: string): void {
    this.validTokens.add(token);
  }
  
  /**
   * Remove token
   */
  removeToken(token: string): void {
    this.validTokens.delete(token);
  }
  
  /**
   * Add API key
   */
  addApiKey(apiKey: string): void {
    this.validApiKeys.add(apiKey);
  }
  
  /**
   * Remove API key
   */
  removeApiKey(apiKey: string): void {
    this.validApiKeys.delete(apiKey);
  }
}