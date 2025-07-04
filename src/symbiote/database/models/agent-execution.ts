/**
 * Agent Execution Model
 */

export interface AgentExecution {
  id: string;
  session_id?: string;
  agent_type: string;
  agent_id?: string;
  task_description?: string;
  input_data?: any;
  output_data?: any;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  started_at: Date;
  completed_at?: Date;
  duration_ms?: number;
  tokens_used?: number;
  cost_usd?: number;
  error?: string;
  metadata: Record<string, any>;
}

export interface AgentExecutionResult {
  success: boolean;
  output: any;
  logs: string[];
  artifacts?: Record<string, any>;
  metrics: {
    duration: number;
    tokensUsed: number;
    cost: number;
  };
}