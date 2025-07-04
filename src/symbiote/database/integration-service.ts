/**
 * Database Integration Service - Connects database with other services
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { getRepositories, Session, AgentExecution } from './index';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { CodeReview } from '../code-review/types';

export class DatabaseIntegrationService extends EventEmitter {
  private logger = new Logger('DatabaseIntegration');
  private currentSession: Session | null = null;
  private orchestrator?: OrchestrationEngine;
  
  constructor(orchestrator?: OrchestrationEngine) {
    super();
    this.orchestrator = orchestrator;
    this.setupEventListeners();
  }
  
  /**
   * Setup event listeners
   */
  private setupEventListeners(): void {
    // Listen to orchestrator events
    if (this.orchestrator) {
      this.orchestrator.on('query:start', this.handleQueryStart.bind(this));
      this.orchestrator.on('query:complete', this.handleQueryComplete.bind(this));
      this.orchestrator.on('query:error', this.handleQueryError.bind(this));
    }
  }
  
  /**
   * Start a new session
   */
  async startSession(userId: string, projectId?: string): Promise<Session> {
    const repos = getRepositories();
    
    // End any existing sessions
    if (this.currentSession) {
      await this.endSession();
    }
    
    // Create new session
    this.currentSession = await repos.sessions.createSession({
      userId,
      projectId
    });
    
    this.logger.info('Started new session', {
      sessionId: this.currentSession.id,
      userId,
      projectId
    });
    
    return this.currentSession;
  }
  
  /**
   * End current session
   */
  async endSession(): Promise<void> {
    if (!this.currentSession) return;
    
    const repos = getRepositories();
    await repos.sessions.endSession(this.currentSession.id);
    
    this.logger.info('Ended session', {
      sessionId: this.currentSession.id
    });
    
    this.currentSession = null;
  }
  
  /**
   * Track file activity
   */
  async trackFileActivity(type: 'open' | 'save', filePath: string): Promise<void> {
    if (!this.currentSession) return;
    
    const repos = getRepositories();
    await repos.sessions.addActivity(this.currentSession.id, {
      type: type === 'open' ? 'file_open' : 'file_save',
      details: { filePath }
    });
  }
  
  /**
   * Track agent execution
   */
  async trackAgentExecution(data: {
    agentType: string;
    agentId?: string;
    taskDescription?: string;
    inputData?: any;
  }): Promise<string> {
    const repos = getRepositories();
    
    // Create agent execution record
    const execution = await repos.agentExecutions.insert({
      session_id: this.currentSession?.id,
      agent_type: data.agentType,
      agent_id: data.agentId,
      task_description: data.taskDescription,
      input_data: data.inputData,
      status: 'pending',
      metadata: {}
    });
    
    // Track in session
    if (this.currentSession) {
      await repos.sessions.addActivity(this.currentSession.id, {
        type: 'agent_execution',
        details: {
          executionId: execution.id,
          agentType: data.agentType
        }
      });
    }
    
    return execution.id;
  }
  
  /**
   * Update agent execution
   */
  async updateAgentExecution(
    executionId: string,
    updates: Partial<AgentExecution>
  ): Promise<void> {
    const repos = getRepositories();
    await repos.agentExecutions.update(executionId, updates);
  }
  
  /**
   * Track code review
   */
  async trackCodeReview(review: CodeReview): Promise<void> {
    const repos = getRepositories();
    
    await repos.codeReviews.insert({
      project_id: this.currentSession?.project_id,
      session_id: this.currentSession?.id,
      review_type: 'ai',
      files: review.files.map(f => f.fsPath),
      issues_found: review.issues.length,
      suggestions_made: review.suggestions.length,
      suggestions_accepted: 0,
      review_data: review,
      started_at: review.startTime,
      completed_at: review.endTime,
      duration_ms: review.endTime ? 
        new Date(review.endTime).getTime() - new Date(review.startTime).getTime() : 
        undefined,
      metadata: {}
    });
    
    // Track in session
    if (this.currentSession) {
      await repos.sessions.addActivity(this.currentSession.id, {
        type: 'code_review',
        details: {
          reviewId: review.id,
          issuesFound: review.issues.length,
          suggestionsGenerated: review.suggestions.length
        }
      });
    }
  }
  
  /**
   * Track AI usage
   */
  async trackAIUsage(data: {
    provider: string;
    model: string;
    operation: string;
    promptTokens: number;
    completionTokens: number;
    cost: number;
    latency: number;
    success: boolean;
    error?: string;
  }): Promise<void> {
    if (!this.currentSession) return;
    
    const repos = getRepositories();
    
    await repos.aiUsage.insert({
      user_id: this.currentSession.user_id,
      session_id: this.currentSession.id,
      model_provider: data.provider,
      model_name: data.model,
      operation_type: data.operation,
      prompt_tokens: data.promptTokens,
      completion_tokens: data.completionTokens,
      total_tokens: data.promptTokens + data.completionTokens,
      cost_usd: data.cost,
      latency_ms: data.latency,
      success: data.success,
      error_message: data.error,
      metadata: {}
    });
    
    // Update session metrics
    await repos.sessions.updateMetrics(this.currentSession.id, {
      tokensUsed: this.currentSession.metrics.tokensUsed + data.promptTokens + data.completionTokens,
      costUsd: this.currentSession.metrics.costUsd + data.cost
    });
  }
  
  /**
   * Handle orchestrator query start
   */
  private async handleQueryStart(event: any): Promise<void> {
    // Track AI query in session
    if (this.currentSession) {
      const repos = getRepositories();
      await repos.sessions.addActivity(this.currentSession.id, {
        type: 'ai_query',
        details: {
          queryId: event.queryId,
          model: event.model,
          feature: event.feature
        }
      });
    }
  }
  
  /**
   * Handle orchestrator query complete
   */
  private async handleQueryComplete(event: any): Promise<void> {
    // Track AI usage
    await this.trackAIUsage({
      provider: event.provider,
      model: event.model,
      operation: event.feature || 'general',
      promptTokens: event.usage?.promptTokens || 0,
      completionTokens: event.usage?.completionTokens || 0,
      cost: event.cost || 0,
      latency: event.latency || 0,
      success: true
    });
  }
  
  /**
   * Handle orchestrator query error
   */
  private async handleQueryError(event: any): Promise<void> {
    // Track failed AI usage
    await this.trackAIUsage({
      provider: event.provider,
      model: event.model,
      operation: event.feature || 'general',
      promptTokens: 0,
      completionTokens: 0,
      cost: 0,
      latency: event.latency || 0,
      success: false,
      error: event.error?.message || 'Unknown error'
    });
    
    // Update session error count
    if (this.currentSession) {
      const repos = getRepositories();
      await repos.sessions.updateMetrics(this.currentSession.id, {
        errorsEncountered: this.currentSession.metrics.errorsEncountered + 1
      });
    }
  }
  
  /**
   * Get current session
   */
  getCurrentSession(): Session | null {
    return this.currentSession;
  }
  
  /**
   * Get user statistics
   */
  async getUserStats(userId: string): Promise<any> {
    const repos = getRepositories();
    
    // Get user
    const user = await repos.users.findById(userId);
    if (!user) return null;
    
    // Get project count
    const projectCount = await repos.projects.count({ user_id: userId });
    
    // Get session stats
    const recentActivity = await repos.sessions.getUserActivitySummary(userId, 30);
    
    // Get AI usage stats
    const aiUsageStats = await repos.aiUsage.getUsageStats(userId, 30);
    
    return {
      user,
      projects: projectCount,
      activity: recentActivity,
      aiUsage: aiUsageStats
    };
  }
}

// Export singleton instance
export const dbIntegration = new DatabaseIntegrationService();