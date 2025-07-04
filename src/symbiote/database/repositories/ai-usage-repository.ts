/**
 * AI Usage Repository
 */

import { BaseRepository } from './base-repository';

export interface AIUsageRecord {
  id: string;
  user_id: string;
  session_id?: string;
  model_provider: string;
  model_name: string;
  operation_type?: string;
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  cost_usd?: number;
  latency_ms?: number;
  success: boolean;
  error_message?: string;
  metadata: Record<string, any>;
  created_at: Date;
}

export class AIUsageRepository extends BaseRepository<AIUsageRecord> {
  constructor() {
    super('ai_usage');
  }
  
  /**
   * Get usage statistics for user
   */
  async getUsageStats(userId: string, days: number = 30): Promise<any> {
    const sql = `
      SELECT 
        model_provider,
        model_name,
        COUNT(*) as request_count,
        SUM(total_tokens) as total_tokens,
        SUM(cost_usd) as total_cost,
        AVG(latency_ms) as avg_latency,
        COUNT(CASE WHEN success = true THEN 1 END) as successful,
        COUNT(CASE WHEN success = false THEN 1 END) as failed
      FROM ${this.table}
      WHERE user_id = $1
        AND created_at >= CURRENT_DATE - INTERVAL '${days} days'
      GROUP BY model_provider, model_name
      ORDER BY total_cost DESC
    `;
    
    return this.query(sql, [userId]);
  }
  
  /**
   * Get daily usage
   */
  async getDailyUsage(userId: string, days: number = 7): Promise<any[]> {
    const sql = `
      SELECT 
        DATE(created_at) as date,
        COUNT(*) as requests,
        SUM(total_tokens) as tokens,
        SUM(cost_usd) as cost
      FROM ${this.table}
      WHERE user_id = $1
        AND created_at >= CURRENT_DATE - INTERVAL '${days} days'
      GROUP BY DATE(created_at)
      ORDER BY date DESC
    `;
    
    return this.query(sql, [userId]);
  }
}