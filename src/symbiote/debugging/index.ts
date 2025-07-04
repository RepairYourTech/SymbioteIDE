/**
 * Advanced Debugging AI Module
 * 
 * AI-powered debugging system using Google ADK with multi-agent orchestration
 */

// Main service
export { DebugService, debugService, DebugServiceConfig } from './debug-service';

// Types
export * from './types';

// Agents
export { DebugControllerAgent } from './agents/debug-controller';
export { ErrorAnalyzerAgent } from './agents/error-analyzer';
export { StackNavigatorAgent } from './agents/stack-navigator';
export { VariableInspectorAgent } from './agents/variable-inspector';
export { FixGeneratorAgent } from './agents/fix-generator';

// Memory system
export { DebugMemorySystem, MemorySearchQuery, PatternLearningResult } from './memory/debug-memory-system';

// Integration helpers
export { BreakpointPredictor } from './predictive/breakpoint-predictor';
export { BrowserDebugIntegration } from './integration/browser-debug-integration';
export { DebugPanel } from './ui/debug-panel';

/**
 * Quick start example:
 * 
 * ```typescript
 * import { debugService } from '@symbiote/debugging';
 * 
 * // Initialize service
 * await debugService.initialize();
 * 
 * // Start debugging session
 * const session = await debugService.startSession({
 *   type: 'runtime',
 *   message: 'Cannot read property "name" of undefined',
 *   file: 'src/components/UserProfile.tsx',
 *   line: 42
 * }, {
 *   code: fileContent,
 *   preferences: {
 *     autoFix: true,
 *     explainInDetail: true
 *   }
 * });
 * 
 * // Apply suggested fix
 * if (session.fixes && session.fixes.length > 0) {
 *   await debugService.applyFix(session.id, session.fixes[0].id);
 * }
 * ```
 */

// Configuration types
export interface DebugConfig {
  /**
   * Enable memory system for learning from past debugging sessions
   */
  enableMemory?: boolean;
  
  /**
   * Enable predictive debugging features
   */
  enablePredictive?: boolean;
  
  /**
   * Automatically apply fixes with high confidence
   */
  autoFix?: boolean;
  
  /**
   * Model preferences for different agents
   */
  models?: {
    controller?: string;
    analyzer?: string;
    navigator?: string;
    inspector?: string;
    generator?: string;
  };
  
  /**
   * Integration settings
   */
  integrations?: {
    browser?: boolean;
    testing?: boolean;
    vscode?: boolean;
  };
}

// VS Code integration
export interface VSCodeDebugIntegration {
  /**
   * Register debug service with VS Code
   */
  register(context: any): void;
  
  /**
   * Handle error from VS Code diagnostics
   */
  handleDiagnostic(diagnostic: any): Promise<void>;
  
  /**
   * Create debug panel in VS Code
   */
  createPanel(): any;
}

// Event types
export interface DebugEvents {
  'initialized': () => void;
  'sessionStarted': (session: any) => void;
  'sessionCompleted': (session: any) => void;
  'sessionFailed': (session: any) => void;
  'debugStep': (step: any) => void;
  'analysisComplete': (analysis: any) => void;
  'autoFixAvailable': (fix: any) => void;
  'applyFix': (data: { sessionId: string; fix: any; code: string }) => void;
  'agentMessage': (message: any) => void;
  'error': (error: Error) => void;
  'shutdown': () => void;
}