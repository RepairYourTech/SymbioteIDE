/**
 * Session Model
 */

export interface Session {
  id: string;
  user_id: string;
  project_id?: string;
  started_at: Date;
  ended_at?: Date;
  duration_seconds?: number;
  activities: Activity[];
  metrics: SessionMetrics;
}

export interface Activity {
  type: 'file_open' | 'file_save' | 'ai_query' | 'code_review' | 'agent_execution' | 'search' | 'debug';
  timestamp: Date;
  details: Record<string, any>;
}

export interface SessionMetrics {
  filesOpened: number;
  filesSaved: number;
  aiQueries: number;
  tokensUsed: number;
  costUsd: number;
  errorsEncountered: number;
  agentsExecuted: number;
}