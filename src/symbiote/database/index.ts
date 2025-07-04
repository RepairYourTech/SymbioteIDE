/**
 * Database Module - Main entry point
 */

import { db, DatabaseConnection } from './connection';
import { UserRepository } from './repositories/user-repository';
import { ProjectRepository } from './repositories/project-repository';
import { SessionRepository } from './repositories/session-repository';
import { AgentExecutionRepository } from './repositories/agent-execution-repository';
import { CodeReviewRepository } from './repositories/code-review-repository';
import { AIUsageRepository } from './repositories/ai-usage-repository';
import { Logger } from '../utils/logger';

// Export models
export * from './models/user';
export * from './models/project';
export * from './models/session';
export * from './models/agent-execution';

// Export connection
export { db, DatabaseConnection };

// Repository instances
let userRepository: UserRepository | null = null;
let projectRepository: ProjectRepository | null = null;
let sessionRepository: SessionRepository | null = null;
let agentExecutionRepository: AgentExecutionRepository | null = null;
let codeReviewRepository: CodeReviewRepository | null = null;
let aiUsageRepository: AIUsageRepository | null = null;

const logger = new Logger('Database');

/**
 * Initialize database and run migrations
 */
export async function initializeDatabase(): Promise<void> {
  try {
    logger.info('Initializing database...');
    
    // Initialize connection
    await db.initialize();
    
    // Run migrations
    await db.runMigrations();
    
    // Initialize repositories
    userRepository = new UserRepository();
    projectRepository = new ProjectRepository();
    sessionRepository = new SessionRepository();
    agentExecutionRepository = new AgentExecutionRepository();
    codeReviewRepository = new CodeReviewRepository();
    aiUsageRepository = new AIUsageRepository();
    
    logger.info('Database initialized successfully');
    
  } catch (error) {
    logger.error('Failed to initialize database', error);
    throw error;
  }
}

/**
 * Get repository instances
 */
export function getRepositories() {
  if (!userRepository || !projectRepository || !sessionRepository || 
      !agentExecutionRepository || !codeReviewRepository || !aiUsageRepository) {
    throw new Error('Database not initialized. Call initializeDatabase() first.');
  }
  
  return {
    users: userRepository,
    projects: projectRepository,
    sessions: sessionRepository,
    agentExecutions: agentExecutionRepository,
    codeReviews: codeReviewRepository,
    aiUsage: aiUsageRepository
  };
}

/**
 * Database health check
 */
export async function checkDatabaseHealth(): Promise<{
  healthy: boolean;
  details: any;
}> {
  try {
    const isHealthy = await db.isHealthy();
    const stats = db.getStats();
    
    return {
      healthy: isHealthy,
      details: {
        connected: isHealthy,
        poolStats: stats,
        timestamp: new Date()
      }
    };
  } catch (error) {
    logger.error('Database health check failed', error);
    return {
      healthy: false,
      details: {
        error: error instanceof Error ? error.message : 'Unknown error',
        timestamp: new Date()
      }
    };
  }
}

/**
 * Shutdown database connections
 */
export async function shutdownDatabase(): Promise<void> {
  try {
    logger.info('Shutting down database...');
    await db.close();
    
    // Clear repository instances
    userRepository = null;
    projectRepository = null;
    sessionRepository = null;
    agentExecutionRepository = null;
    codeReviewRepository = null;
    aiUsageRepository = null;
    
    logger.info('Database shutdown complete');
    
  } catch (error) {
    logger.error('Error during database shutdown', error);
  }
}