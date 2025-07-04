/**
 * Agent Execution Repository
 */

import { BaseRepository } from './base-repository';
import { AgentExecution } from '../models/agent-execution';

export class AgentExecutionRepository extends BaseRepository<AgentExecution> {
  constructor() {
    super('agent_executions');
  }
  
  /**
   * Find executions by session
   */
  async findBySession(sessionId: string): Promise<AgentExecution[]> {
    return this.findWhere({ session_id: sessionId }, {
      orderBy: 'started_at DESC'
    });
  }
  
  /**
   * Find executions by agent type
   */
  async findByAgentType(agentType: string): Promise<AgentExecution[]> {
    return this.findWhere({ agent_type: agentType }, {
      orderBy: 'started_at DESC'
    });
  }
  
  /**
   * Find executions by status
   */
  async findByStatus(status: AgentExecution['status']): Promise<AgentExecution[]> {
    return this.findWhere({ status }, {
      orderBy: 'started_at DESC'
    });
  }
  
  /**
   * Update execution status
   */
  async updateStatus(
    executionId: string,
    status: AgentExecution['status'],
    error?: string
  ): Promise<AgentExecution | null> {
    const updates: Partial<AgentExecution> = { status };
    
    if (status === 'completed' || status === 'failed') {
      updates.completed_at = new Date();
      
      // Calculate duration
      const execution = await this.findById(executionId);
      if (execution) {
        updates.duration_ms = new Date().getTime() - new Date(execution.started_at).getTime();
      }
    }
    
    if (error) {
      updates.error = error;
    }
    
    return this.update(executionId, updates);
  }
  
  /**
   * Complete execution
   */
  async completeExecution(
    executionId: string,
    result: {
      output: any;
      tokensUsed?: number;
      cost?: number;
      metadata?: any;
    }
  ): Promise<AgentExecution | null> {
    const execution = await this.findById(executionId);
    if (!execution) return null;
    
    const duration = new Date().getTime() - new Date(execution.started_at).getTime();
    
    return this.update(executionId, {
      status: 'completed',
      output_data: result.output,
      tokens_used: result.tokensUsed,
      cost_usd: result.cost,
      completed_at: new Date(),
      duration_ms: duration,
      metadata: { ...execution.metadata, ...(result.metadata || {}) }
    });
  }
  
  /**
   * Get execution statistics by agent type
   */
  async getStatsByAgentType(
    agentType: string,
    days: number = 30
  ): Promise<any> {
    const sql = `
      SELECT 
        COUNT(*) as total_executions,
        COUNT(CASE WHEN status = 'completed' THEN 1 END) as successful,
        COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed,
        AVG(duration_ms) as avg_duration_ms,
        SUM(tokens_used) as total_tokens,
        SUM(cost_usd) as total_cost,
        MIN(started_at) as first_execution,
        MAX(started_at) as last_execution
      FROM ${this.table}
      WHERE agent_type = $1
        AND started_at >= CURRENT_DATE - INTERVAL '${days} days'
    `;
    
    const result = await this.query<any>(sql, [agentType]);
    return result[0] || null;
  }
  
  /**
   * Get running executions
   */
  async getRunningExecutions(): Promise<AgentExecution[]> {
    return this.findWhere({ status: 'running' }, {
      orderBy: 'started_at ASC'
    });
  }
  
  /**
   * Cancel execution
   */
  async cancelExecution(executionId: string): Promise<AgentExecution | null> {
    return this.updateStatus(executionId, 'cancelled');
  }
}