/**
 * Project Repository
 */

import { BaseRepository } from './base-repository';
import { Project } from '../models/project';

export class ProjectRepository extends BaseRepository<Project> {
  constructor() {
    super('projects');
  }
  
  /**
   * Find projects by user
   */
  async findByUser(userId: string): Promise<Project[]> {
    return this.findWhere({ user_id: userId }, {
      orderBy: 'last_opened_at DESC NULLS LAST, created_at DESC'
    });
  }
  
  /**
   * Find project by path for user
   */
  async findByPath(userId: string, path: string): Promise<Project | null> {
    return this.findOneWhere({ user_id: userId, path });
  }
  
  /**
   * Create or update project
   */
  async upsertProject(data: {
    userId: string;
    path: string;
    name: string;
    description?: string;
    type?: string;
    language?: string;
    framework?: string;
    metadata?: any;
    settings?: any;
  }): Promise<Project> {
    // Check if project exists
    const existing = await this.findByPath(data.userId, data.path);
    
    if (existing) {
      // Update existing project
      const updated = await this.update(existing.id, {
        name: data.name,
        description: data.description,
        type: data.type || existing.type,
        language: data.language || existing.language,
        framework: data.framework || existing.framework,
        metadata: { ...existing.metadata, ...(data.metadata || {}) },
        settings: { ...existing.settings, ...(data.settings || {}) },
        last_opened_at: new Date()
      });
      
      return updated!;
    } else {
      // Create new project
      return this.insert({
        user_id: data.userId,
        path: data.path,
        name: data.name,
        description: data.description,
        type: data.type || 'general',
        language: data.language,
        framework: data.framework,
        metadata: data.metadata || {},
        settings: data.settings || {},
        last_opened_at: new Date()
      });
    }
  }
  
  /**
   * Update project settings
   */
  async updateSettings(
    projectId: string,
    settings: Record<string, any>
  ): Promise<Project | null> {
    const project = await this.findById(projectId);
    if (!project) return null;
    
    return this.update(projectId, {
      settings: { ...project.settings, ...settings }
    });
  }
  
  /**
   * Update last opened timestamp
   */
  async touchProject(projectId: string): Promise<void> {
    await this.update(projectId, {
      last_opened_at: new Date()
    });
  }
  
  /**
   * Get recent projects
   */
  async getRecentProjects(userId: string, limit: number = 10): Promise<Project[]> {
    return this.findWhere({ user_id: userId }, {
      orderBy: 'last_opened_at DESC NULLS LAST',
      limit
    });
  }
  
  /**
   * Search projects
   */
  async searchProjects(userId: string, query: string): Promise<Project[]> {
    const sql = `
      SELECT * FROM ${this.table}
      WHERE user_id = $1
        AND (
          name ILIKE $2
          OR description ILIKE $2
          OR path ILIKE $2
        )
      ORDER BY last_opened_at DESC NULLS LAST
      LIMIT 20
    `;
    
    return this.query(sql, [userId, `%${query}%`]);
  }
  
  /**
   * Get project statistics
   */
  async getProjectStats(projectId: string): Promise<any> {
    const sql = `
      SELECT 
        p.*,
        COUNT(DISTINCT s.id) as session_count,
        COUNT(DISTINCT ae.id) as agent_execution_count,
        COUNT(DISTINCT cr.id) as code_review_count,
        COALESCE(SUM(au.tokens_used), 0) as total_tokens_used,
        COALESCE(SUM(au.cost_usd), 0) as total_cost_usd
      FROM ${this.table} p
      LEFT JOIN symbiote.sessions s ON s.project_id = p.id
      LEFT JOIN symbiote.agent_executions ae ON ae.session_id = s.id
      LEFT JOIN symbiote.code_reviews cr ON cr.project_id = p.id
      LEFT JOIN symbiote.ai_usage au ON au.session_id = s.id
      WHERE p.id = $1
      GROUP BY p.id
    `;
    
    const result = await this.query<any>(sql, [projectId]);
    return result[0] || null;
  }
}