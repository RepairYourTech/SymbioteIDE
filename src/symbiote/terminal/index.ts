/**
 * Terminal Module
 * 
 * AI-Powered Terminal Integration for SymbioteIDE
 */

export * from './terminal-manager';
export * from './terminal-session';
export * from './ai-command-processor';
export * from './command-translator';
export * from './error-analyzer';
export * from './workflow-manager';
export * from './learning-engine';
export * from './terminal-api-impl';

import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { TerminalAPIImplementation } from './terminal-api-impl';
import { TerminalAPIClient } from '../../types/terminal-api';

/**
 * Create and initialize the terminal API
 */
export function createTerminalAPI(orchestrationEngine: OrchestrationEngine): TerminalAPIClient {
  return new TerminalAPIImplementation(orchestrationEngine);
}

/**
 * Terminal integration configuration
 */
export interface TerminalIntegrationConfig {
  /**
   * Maximum number of concurrent sessions
   */
  maxSessions?: number;
  
  /**
   * Default shell to use
   */
  defaultShell?: string;
  
  /**
   * Enable AI assistance by default
   */
  aiEnabledByDefault?: boolean;
  
  /**
   * AI assistance level
   */
  defaultAILevel?: 'none' | 'suggest' | 'assist' | 'autopilot';
  
  /**
   * Enable learning from command history
   */
  enableLearning?: boolean;
  
  /**
   * Enable workflow automation
   */
  enableWorkflows?: boolean;
  
  /**
   * GPU acceleration for rendering
   */
  enableGPUAcceleration?: boolean;
}