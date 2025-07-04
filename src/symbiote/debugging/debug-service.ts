/**
 * Debug Service - Main entry point for Advanced Debugging AI
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { ADKClient } from '../adk/adk-client';
import { DebugControllerAgent } from './agents/debug-controller';
import { DebugMemorySystem } from './memory/debug-memory-system';
import { 
  DebugSession,
  ErrorContext,
  DebugAgentRequest,
  DebugPreferences,
  DebugAgentConfigs,
  ADKAgentConfig,
  ADKAgentType
} from './types';
import { v4 as uuidv4 } from 'uuid';

export interface DebugServiceConfig {
  enableMemory: boolean;
  enablePredictive: boolean;
  autoFix: boolean;
  models?: {
    controller?: string;
    analyzer?: string;
    navigator?: string;
    inspector?: string;
    generator?: string;
  };
}

export class DebugService extends EventEmitter {
  private logger = new Logger('DebugService');
  private adkClient: ADKClient;
  private controller?: DebugControllerAgent;
  private memorySystem: DebugMemorySystem;
  private config: DebugServiceConfig;
  private initialized = false;
  private activeSessions: Map<string, DebugSession> = new Map();
  
  constructor(config?: Partial<DebugServiceConfig>) {
    super();
    
    this.config = {
      enableMemory: true,
      enablePredictive: true,
      autoFix: false,
      ...config
    };
    
    this.adkClient = new ADKClient();
    this.memorySystem = new DebugMemorySystem();
  }
  
  /**
   * Initialize the debug service
   */
  async initialize(): Promise<void> {
    if (this.initialized) return;
    
    try {
      this.logger.info('Initializing Debug Service...');
      
      // Connect to ADK
      await this.adkClient.connect();
      
      // Create agent configurations
      const agentConfigs = this.createAgentConfigs();
      
      // Initialize controller agent
      this.controller = new DebugControllerAgent(agentConfigs.controller);
      await this.controller.initialize();
      
      // Setup event handlers
      this.setupEventHandlers();
      
      this.initialized = true;
      this.logger.info('Debug Service initialized successfully');
      
      this.emit('initialized');
      
    } catch (error) {
      this.logger.error('Failed to initialize Debug Service', error);
      throw error;
    }
  }
  
  /**
   * Start a debugging session
   */
  async startSession(
    error: ErrorContext,
    options?: {
      code?: string;
      preferences?: DebugPreferences;
      sessionId?: string;
    }
  ): Promise<DebugSession> {
    if (!this.initialized) {
      await this.initialize();
    }
    
    const sessionId = options?.sessionId || uuidv4();
    
    try {
      this.logger.info(`Starting debug session ${sessionId}`);
      
      // Get historical context from memory
      let history;
      if (this.config.enableMemory) {
        history = await this.memorySystem.searchEpisodicMemory({
          errorType: error.type,
          errorMessage: error.message,
          file: error.file,
          limit: 5
        });
      }
      
      // Get pattern recommendations
      const patterns = this.memorySystem.getPatternRecommendations(error);
      
      // Create debug request
      const request: DebugAgentRequest = {
        sessionId,
        error,
        code: options?.code,
        history: history?.map(h => ({
          error: h.error,
          solution: h.solution,
          success: h.success
        })),
        preferences: options?.preferences
      };
      
      // Add pattern insights to working memory
      if (patterns.length > 0) {
        this.memorySystem.updateWorkingMemory(sessionId, {
          testedHypotheses: patterns.map(p => p.name)
        });
      }
      
      // Execute debugging workflow
      const session = await this.controller!.debug(request);
      
      // Store active session
      this.activeSessions.set(sessionId, session);
      
      // Emit progress events
      this.emit('sessionStarted', session);
      
      // Store in memory if enabled
      if (this.config.enableMemory && session.status === 'completed') {
        await this.memorySystem.storeSession(session);
      }
      
      // Auto-apply fix if enabled
      if (this.config.autoFix && session.fixes && session.fixes.length > 0) {
        const primaryFix = session.fixes[0];
        this.emit('autoFixAvailable', {
          sessionId,
          fix: primaryFix
        });
      }
      
      return session;
      
    } catch (error) {
      this.logger.error(`Debug session ${sessionId} failed`, error);
      
      const failedSession: DebugSession = {
        id: sessionId,
        projectId: 'current',
        userId: 'current',
        startTime: new Date(),
        endTime: new Date(),
        status: 'failed',
        error: error as ErrorContext
      };
      
      this.activeSessions.set(sessionId, failedSession);
      this.emit('sessionFailed', failedSession);
      
      throw error;
    }
  }
  
  /**
   * Get active debugging session
   */
  getSession(sessionId: string): DebugSession | undefined {
    return this.activeSessions.get(sessionId);
  }
  
  /**
   * Apply a fix from a session
   */
  async applyFix(
    sessionId: string,
    fixId: string
  ): Promise<boolean> {
    const session = this.activeSessions.get(sessionId);
    if (!session || !session.fixes) {
      throw new Error('Session or fixes not found');
    }
    
    const fix = session.fixes.find(f => f.id === fixId);
    if (!fix) {
      throw new Error('Fix not found');
    }
    
    try {
      // Emit fix application event (to be handled by VS Code extension)
      this.emit('applyFix', {
        sessionId,
        fix,
        code: fix.code
      });
      
      // Update fix status
      fix.appliedAt = new Date();
      fix.success = true;
      
      // Update session in memory
      if (this.config.enableMemory) {
        await this.memorySystem.storeSession(session);
      }
      
      return true;
      
    } catch (error) {
      this.logger.error('Failed to apply fix', error);
      fix.success = false;
      return false;
    }
  }
  
  /**
   * Get debugging history
   */
  async getHistory(
    query?: {
      errorType?: string;
      file?: string;
      limit?: number;
    }
  ): Promise<any[]> {
    if (!this.config.enableMemory) {
      return [];
    }
    
    const episodes = await this.memorySystem.searchEpisodicMemory({
      errorType: query?.errorType,
      file: query?.file,
      limit: query?.limit || 10
    });
    
    return episodes.map(e => ({
      sessionId: e.sessionId,
      timestamp: e.timestamp,
      error: e.error,
      solution: e.solution,
      success: e.success,
      duration: e.duration
    }));
  }
  
  /**
   * Get debugging patterns
   */
  getPatterns(errorType?: string): any[] {
    // Get all patterns or filter by error type
    const patterns = this.memorySystem.getPatternRecommendations({
      type: errorType || 'any',
      message: ''
    } as ErrorContext);
    
    return patterns.map(p => ({
      id: p.id,
      name: p.name,
      description: p.description,
      frequency: p.frequency,
      successRate: p.successRate,
      solutions: p.solutions
    }));
  }
  
  /**
   * Learn from debugging sessions
   */
  async learn(sessionIds?: string[]): Promise<any> {
    if (!this.config.enableMemory) {
      return { learned: false, reason: 'Memory disabled' };
    }
    
    const sessions = sessionIds 
      ? sessionIds.map(id => this.activeSessions.get(id)).filter(Boolean) as DebugSession[]
      : Array.from(this.activeSessions.values());
    
    const result = await this.memorySystem.learnPatterns(sessions);
    
    // Learn at agent level too
    for (const session of sessions) {
      if (session.status === 'completed') {
        await this.controller?.learn(session);
      }
    }
    
    return {
      learned: true,
      newPatterns: result.newPatterns,
      updatedPatterns: result.updatedPatterns,
      confidence: result.confidence
    };
  }
  
  /**
   * Get memory statistics
   */
  async getMemoryStats(): Promise<any> {
    return this.memorySystem.getMemoryStats();
  }
  
  /**
   * Create agent configurations
   */
  private createAgentConfigs(): DebugAgentConfigs {
    const models = this.config.models || {};
    
    return {
      controller: {
        id: 'debug-controller',
        name: 'Debug Controller',
        type: ADKAgentType.Sequential,
        description: 'Orchestrates debugging workflow',
        model: { 
          id: models.controller || 'claude-3-opus-20240229', 
          provider: 'anthropic' 
        },
        temperature: 0.3,
        capabilities: {
          streaming: true,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: true
        }
      } as ADKAgentConfig,
      
      errorAnalyzer: {
        id: 'error-analyzer',
        name: 'Error Analyzer',
        type: ADKAgentType.LLM,
        model: { 
          id: models.analyzer || 'gpt-4', 
          provider: 'openai' 
        },
        temperature: 0.3,
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: false
        }
      } as ADKAgentConfig,
      
      stackNavigator: {
        id: 'stack-navigator',
        name: 'Stack Navigator',
        type: ADKAgentType.LLM,
        model: { 
          id: models.navigator || 'gemini-1.5-flash', 
          provider: 'google' 
        },
        temperature: 0.1,
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: false,
          a2aProtocol: true,
          mcpTools: false
        }
      } as ADKAgentConfig,
      
      variableInspector: {
        id: 'variable-inspector',
        name: 'Variable Inspector',
        type: ADKAgentType.LLM,
        model: { 
          id: models.inspector || 'claude-3-sonnet-20240229', 
          provider: 'anthropic' 
        },
        temperature: 0.2,
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: false
        }
      } as ADKAgentConfig,
      
      fixGenerator: {
        id: 'fix-generator',
        name: 'Fix Generator',
        type: ADKAgentType.LLM,
        model: { 
          id: models.generator || 'gpt-4', 
          provider: 'openai' 
        },
        temperature: 0.4,
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: true
        }
      } as ADKAgentConfig
    };
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Forward controller events
    if (this.controller) {
      this.controller.on('stepCompleted', (data) => {
        this.emit('debugStep', data);
      });
      
      this.controller.on('analysisComplete', (data) => {
        this.emit('analysisComplete', data);
      });
    }
    
    // Handle ADK events
    this.adkClient.on('agentMessage', (data) => {
      this.emit('agentMessage', data);
    });
    
    this.adkClient.on('error', (error) => {
      this.logger.error('ADK error', error);
      this.emit('error', error);
    });
  }
  
  /**
   * Cleanup and shutdown
   */
  async shutdown(): Promise<void> {
    this.logger.info('Shutting down Debug Service');
    
    // Clear active sessions
    this.activeSessions.clear();
    
    // Disconnect from ADK
    await this.adkClient.disconnect();
    
    this.initialized = false;
    this.emit('shutdown');
  }
}

// Export singleton instance
export const debugService = new DebugService();