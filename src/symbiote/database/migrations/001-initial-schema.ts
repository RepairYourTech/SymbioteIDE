/**
 * Initial database schema migration
 */

import { Client } from 'pg';

export const migration = {
  name: '001-initial-schema',
  
  async up(client: Client): Promise<void> {
    // Users table
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.users (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        email VARCHAR(255) UNIQUE NOT NULL,
        username VARCHAR(100) UNIQUE NOT NULL,
        full_name VARCHAR(255),
        avatar_url TEXT,
        preferences JSONB DEFAULT '{}',
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        last_login_at TIMESTAMP
      )
    `);
    
    // Projects table
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.projects (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        user_id UUID NOT NULL REFERENCES symbiote.users(id) ON DELETE CASCADE,
        name VARCHAR(255) NOT NULL,
        description TEXT,
        path TEXT NOT NULL,
        type VARCHAR(50) DEFAULT 'general',
        language VARCHAR(50),
        framework VARCHAR(50),
        metadata JSONB DEFAULT '{}',
        settings JSONB DEFAULT '{}',
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        last_opened_at TIMESTAMP,
        UNIQUE(user_id, path)
      )
    `);
    
    // Sessions table
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.sessions (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        user_id UUID NOT NULL REFERENCES symbiote.users(id) ON DELETE CASCADE,
        project_id UUID REFERENCES symbiote.projects(id) ON DELETE CASCADE,
        started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        ended_at TIMESTAMP,
        duration_seconds INTEGER,
        activities JSONB DEFAULT '[]',
        metrics JSONB DEFAULT '{}'
      )
    `);
    
    // Agent executions table
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.agent_executions (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        session_id UUID REFERENCES symbiote.sessions(id) ON DELETE CASCADE,
        agent_type VARCHAR(100) NOT NULL,
        agent_id VARCHAR(255),
        task_description TEXT,
        input_data JSONB,
        output_data JSONB,
        status VARCHAR(50) DEFAULT 'pending',
        started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        completed_at TIMESTAMP,
        duration_ms INTEGER,
        tokens_used INTEGER,
        cost_usd DECIMAL(10, 6),
        error TEXT,
        metadata JSONB DEFAULT '{}'
      )
    `);
    
    // Code reviews table
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.code_reviews (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        project_id UUID NOT NULL REFERENCES symbiote.projects(id) ON DELETE CASCADE,
        session_id UUID REFERENCES symbiote.sessions(id) ON DELETE SET NULL,
        review_type VARCHAR(50) DEFAULT 'manual',
        files TEXT[],
        issues_found INTEGER DEFAULT 0,
        suggestions_made INTEGER DEFAULT 0,
        suggestions_accepted INTEGER DEFAULT 0,
        review_data JSONB,
        started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        completed_at TIMESTAMP,
        duration_ms INTEGER,
        metadata JSONB DEFAULT '{}'
      )
    `);
    
    // Knowledge entries table
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.knowledge_entries (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        project_id UUID REFERENCES symbiote.projects(id) ON DELETE CASCADE,
        user_id UUID NOT NULL REFERENCES symbiote.users(id) ON DELETE CASCADE,
        type VARCHAR(50) NOT NULL,
        title VARCHAR(500) NOT NULL,
        content TEXT,
        embedding_id VARCHAR(255),
        tags TEXT[],
        metadata JSONB DEFAULT '{}',
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        accessed_count INTEGER DEFAULT 0,
        last_accessed_at TIMESTAMP
      )
    `);
    
    // User preferences history
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.user_preferences_history (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        user_id UUID NOT NULL REFERENCES symbiote.users(id) ON DELETE CASCADE,
        preference_key VARCHAR(255) NOT NULL,
        old_value JSONB,
        new_value JSONB,
        changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        changed_by VARCHAR(100)
      )
    `);
    
    // AI model usage tracking
    await client.query(`
      CREATE TABLE IF NOT EXISTS symbiote.ai_usage (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        user_id UUID NOT NULL REFERENCES symbiote.users(id) ON DELETE CASCADE,
        session_id UUID REFERENCES symbiote.sessions(id) ON DELETE CASCADE,
        model_provider VARCHAR(50) NOT NULL,
        model_name VARCHAR(100) NOT NULL,
        operation_type VARCHAR(100),
        prompt_tokens INTEGER,
        completion_tokens INTEGER,
        total_tokens INTEGER,
        cost_usd DECIMAL(10, 6),
        latency_ms INTEGER,
        success BOOLEAN DEFAULT true,
        error_message TEXT,
        metadata JSONB DEFAULT '{}',
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
      )
    `);
    
    // Create indexes
    await client.query(`
      CREATE INDEX idx_projects_user_id ON symbiote.projects(user_id);
      CREATE INDEX idx_sessions_user_id ON symbiote.sessions(user_id);
      CREATE INDEX idx_sessions_project_id ON symbiote.sessions(project_id);
      CREATE INDEX idx_agent_executions_session_id ON symbiote.agent_executions(session_id);
      CREATE INDEX idx_agent_executions_status ON symbiote.agent_executions(status);
      CREATE INDEX idx_code_reviews_project_id ON symbiote.code_reviews(project_id);
      CREATE INDEX idx_knowledge_entries_project_id ON symbiote.knowledge_entries(project_id);
      CREATE INDEX idx_knowledge_entries_user_id ON symbiote.knowledge_entries(user_id);
      CREATE INDEX idx_knowledge_entries_type ON symbiote.knowledge_entries(type);
      CREATE INDEX idx_ai_usage_user_id ON symbiote.ai_usage(user_id);
      CREATE INDEX idx_ai_usage_created_at ON symbiote.ai_usage(created_at);
    `);
    
    // Create update timestamp trigger function
    await client.query(`
      CREATE OR REPLACE FUNCTION symbiote.update_updated_at_column()
      RETURNS TRIGGER AS $$
      BEGIN
        NEW.updated_at = CURRENT_TIMESTAMP;
        RETURN NEW;
      END;
      $$ language 'plpgsql';
    `);
    
    // Apply update trigger to tables with updated_at
    const tablesWithUpdatedAt = ['users', 'projects', 'knowledge_entries'];
    for (const table of tablesWithUpdatedAt) {
      await client.query(`
        CREATE TRIGGER update_${table}_updated_at 
        BEFORE UPDATE ON symbiote.${table}
        FOR EACH ROW 
        EXECUTE FUNCTION symbiote.update_updated_at_column();
      `);
    }
  },
  
  async down(client: Client): Promise<void> {
    // Drop triggers
    await client.query(`
      DROP TRIGGER IF EXISTS update_users_updated_at ON symbiote.users;
      DROP TRIGGER IF EXISTS update_projects_updated_at ON symbiote.projects;
      DROP TRIGGER IF EXISTS update_knowledge_entries_updated_at ON symbiote.knowledge_entries;
    `);
    
    // Drop function
    await client.query('DROP FUNCTION IF EXISTS symbiote.update_updated_at_column()');
    
    // Drop tables in reverse order
    await client.query(`
      DROP TABLE IF EXISTS symbiote.ai_usage CASCADE;
      DROP TABLE IF EXISTS symbiote.user_preferences_history CASCADE;
      DROP TABLE IF EXISTS symbiote.knowledge_entries CASCADE;
      DROP TABLE IF EXISTS symbiote.code_reviews CASCADE;
      DROP TABLE IF EXISTS symbiote.agent_executions CASCADE;
      DROP TABLE IF EXISTS symbiote.sessions CASCADE;
      DROP TABLE IF EXISTS symbiote.projects CASCADE;
      DROP TABLE IF EXISTS symbiote.users CASCADE;
    `);
  }
};