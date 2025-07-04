/**
 * Session Repository
 */

import { BaseRepository } from './base-repository';
import { Session, Activity } from '../models/session';

export class SessionRepository extends BaseRepository<Session> {
  constructor() {
    super('sessions');
  }
  
  /**
   * Create new session
   */
  async createSession(data: {
    userId: string;
    projectId?: string;
  }): Promise<Session> {
    return this.insert({
      user_id: data.userId,
      project_id: data.projectId,
      activities: [],
      metrics: {
        filesOpened: 0,
        filesSaved: 0,
        aiQueries: 0,
        tokensUsed: 0,
        costUsd: 0,
        errorsEncountered: 0,
        agentsExecuted: 0
      }
    });
  }
  
  /**
   * End session
   */
  async endSession(sessionId: string): Promise<Session | null> {
    const session = await this.findById(sessionId);
    if (!session || session.ended_at) return null;
    
    const duration = Math.floor(
      (new Date().getTime() - new Date(session.started_at).getTime()) / 1000
    );
    
    return this.update(sessionId, {
      ended_at: new Date(),
      duration_seconds: duration
    });
  }
  
  /**
   * Add activity to session
   */
  async addActivity(
    sessionId: string,
    activity: Omit<Activity, 'timestamp'>
  ): Promise<void> {
    const session = await this.findById(sessionId);
    if (!session) return;
    
    const newActivity: Activity = {
      ...activity,
      timestamp: new Date()
    };
    
    const activities = [...session.activities, newActivity];
    
    // Update metrics based on activity type
    const metrics = { ...session.metrics };
    
    switch (activity.type) {
      case 'file_open':
        metrics.filesOpened++;
        break;
      case 'file_save':
        metrics.filesSaved++;
        break;
      case 'ai_query':
        metrics.aiQueries++;
        break;
      case 'agent_execution':
        metrics.agentsExecuted++;
        break;
    }
    
    await this.update(sessionId, { activities, metrics });
  }
  
  /**
   * Update session metrics
   */
  async updateMetrics(
    sessionId: string,
    updates: Partial<Session['metrics']>
  ): Promise<void> {
    const session = await this.findById(sessionId);
    if (!session) return;
    
    const metrics = { ...session.metrics, ...updates };
    await this.update(sessionId, { metrics });
  }
  
  /**
   * Get active sessions for user
   */
  async getActiveSessions(userId: string): Promise<Session[]> {
    return this.findWhere({
      user_id: userId,
      ended_at: null
    }, {
      orderBy: 'started_at DESC'
    });
  }
  
  /**
   * Get recent sessions
   */
  async getRecentSessions(
    userId: string,
    limit: number = 10
  ): Promise<Session[]> {
    return this.findWhere({ user_id: userId }, {
      orderBy: 'started_at DESC',
      limit
    });
  }
  
  /**
   * Get session statistics
   */
  async getSessionStats(sessionId: string): Promise<any> {
    const sql = `
      SELECT 
        s.*,
        COUNT(DISTINCT ae.id) as agent_executions,
        COALESCE(SUM(au.tokens_used), 0) as total_tokens,
        COALESCE(SUM(au.cost_usd), 0) as total_cost,
        COUNT(DISTINCT au.model_provider) as providers_used,
        COUNT(DISTINCT au.model_name) as models_used
      FROM ${this.table} s
      LEFT JOIN symbiote.agent_executions ae ON ae.session_id = s.id
      LEFT JOIN symbiote.ai_usage au ON au.session_id = s.id
      WHERE s.id = $1
      GROUP BY s.id
    `;
    
    const result = await this.query<any>(sql, [sessionId]);
    return result[0] || null;
  }
  
  /**
   * Get user activity summary
   */
  async getUserActivitySummary(
    userId: string,
    days: number = 30
  ): Promise<any> {
    const sql = `
      SELECT 
        DATE(started_at) as date,
        COUNT(*) as sessions,
        SUM(duration_seconds) as total_duration,
        SUM((metrics->>'filesOpened')::int) as files_opened,
        SUM((metrics->>'filesSaved')::int) as files_saved,
        SUM((metrics->>'aiQueries')::int) as ai_queries,
        SUM((metrics->>'agentsExecuted')::int) as agents_executed
      FROM ${this.table}
      WHERE user_id = $1
        AND started_at >= CURRENT_DATE - INTERVAL '${days} days'
      GROUP BY DATE(started_at)
      ORDER BY date DESC
    `;
    
    return this.query(sql, [userId]);
  }
}