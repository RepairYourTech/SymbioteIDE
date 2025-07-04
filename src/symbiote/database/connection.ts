/**
 * PostgreSQL Database Connection Manager
 */

import { Pool, PoolConfig, Client } from 'pg';
import { Logger } from '../utils/logger';

export interface DatabaseConfig {
  connectionString?: string;
  host?: string;
  port?: number;
  database?: string;
  user?: string;
  password?: string;
  max?: number; // Max pool size
  idleTimeoutMillis?: number;
  connectionTimeoutMillis?: number;
}

export class DatabaseConnection {
  private static instance: DatabaseConnection;
  private pool: Pool | null = null;
  private logger = new Logger('DatabaseConnection');
  private config: PoolConfig;
  
  private constructor(config?: DatabaseConfig) {
    // Use connection string from environment or provided config
    const connectionString = config?.connectionString || process.env.POSTGRES_URL;
    
    if (connectionString) {
      this.config = {
        connectionString,
        max: config?.max || 20,
        idleTimeoutMillis: config?.idleTimeoutMillis || 30000,
        connectionTimeoutMillis: config?.connectionTimeoutMillis || 2000,
      };
    } else {
      // Fallback to individual parameters
      this.config = {
        host: config?.host || process.env.POSTGRES_HOST || 'localhost',
        port: config?.port || parseInt(process.env.POSTGRES_PORT || '5432'),
        database: config?.database || process.env.POSTGRES_DB || 'symbiote',
        user: config?.user || process.env.POSTGRES_USER || 'symbiote',
        password: config?.password || process.env.POSTGRES_PASSWORD || 'symbiote123',
        max: config?.max || 20,
        idleTimeoutMillis: config?.idleTimeoutMillis || 30000,
        connectionTimeoutMillis: config?.connectionTimeoutMillis || 2000,
      };
    }
  }
  
  /**
   * Get singleton instance
   */
  static getInstance(config?: DatabaseConfig): DatabaseConnection {
    if (!DatabaseConnection.instance) {
      DatabaseConnection.instance = new DatabaseConnection(config);
    }
    return DatabaseConnection.instance;
  }
  
  /**
   * Initialize database connection pool
   */
  async initialize(): Promise<void> {
    if (this.pool) {
      this.logger.warn('Database pool already initialized');
      return;
    }
    
    try {
      this.pool = new Pool(this.config);
      
      // Test connection
      const client = await this.pool.connect();
      await client.query('SELECT NOW()');
      client.release();
      
      this.logger.info('Database connection pool initialized');
      
      // Setup error handlers
      this.pool.on('error', (err) => {
        this.logger.error('Unexpected database error', err);
      });
      
      this.pool.on('connect', () => {
        this.logger.debug('New client connected to database');
      });
      
      this.pool.on('acquire', () => {
        this.logger.debug('Client acquired from pool');
      });
      
      this.pool.on('remove', () => {
        this.logger.debug('Client removed from pool');
      });
      
    } catch (error) {
      this.logger.error('Failed to initialize database connection', error);
      throw error;
    }
  }
  
  /**
   * Get connection pool
   */
  getPool(): Pool {
    if (!this.pool) {
      throw new Error('Database connection not initialized');
    }
    return this.pool;
  }
  
  /**
   * Execute a query
   */
  async query<T = any>(text: string, params?: any[]): Promise<T[]> {
    const pool = this.getPool();
    
    try {
      const start = Date.now();
      const result = await pool.query(text, params);
      const duration = Date.now() - start;
      
      this.logger.debug(`Query executed in ${duration}ms`, {
        text: text.substring(0, 100),
        rowCount: result.rowCount
      });
      
      return result.rows;
    } catch (error) {
      this.logger.error('Query failed', { text, params, error });
      throw error;
    }
  }
  
  /**
   * Execute a query and return single row
   */
  async queryOne<T = any>(text: string, params?: any[]): Promise<T | null> {
    const rows = await this.query<T>(text, params);
    return rows[0] || null;
  }
  
  /**
   * Execute query in transaction
   */
  async transaction<T>(
    callback: (client: Client) => Promise<T>
  ): Promise<T> {
    const pool = this.getPool();
    const client = await pool.connect();
    
    try {
      await client.query('BEGIN');
      const result = await callback(client);
      await client.query('COMMIT');
      return result;
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }
  
  /**
   * Check if database is healthy
   */
  async isHealthy(): Promise<boolean> {
    try {
      await this.query('SELECT 1');
      return true;
    } catch (error) {
      return false;
    }
  }
  
  /**
   * Get pool statistics
   */
  getStats() {
    if (!this.pool) {
      return null;
    }
    
    return {
      totalCount: this.pool.totalCount,
      idleCount: this.pool.idleCount,
      waitingCount: this.pool.waitingCount
    };
  }
  
  /**
   * Close database connection
   */
  async close(): Promise<void> {
    if (this.pool) {
      await this.pool.end();
      this.pool = null;
      this.logger.info('Database connection pool closed');
    }
  }
  
  /**
   * Run migrations
   */
  async runMigrations(): Promise<void> {
    this.logger.info('Running database migrations...');
    
    try {
      // Create migrations table if it doesn't exist
      await this.query(`
        CREATE TABLE IF NOT EXISTS symbiote.migrations (
          id SERIAL PRIMARY KEY,
          name VARCHAR(255) NOT NULL UNIQUE,
          executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
      `);
      
      // Import and run migrations
      const migrations = await this.loadMigrations();
      
      for (const migration of migrations) {
        const exists = await this.queryOne(
          'SELECT id FROM symbiote.migrations WHERE name = $1',
          [migration.name]
        );
        
        if (!exists) {
          this.logger.info(`Running migration: ${migration.name}`);
          
          await this.transaction(async (client) => {
            await migration.up(client);
            await client.query(
              'INSERT INTO symbiote.migrations (name) VALUES ($1)',
              [migration.name]
            );
          });
          
          this.logger.info(`Migration completed: ${migration.name}`);
        }
      }
      
      this.logger.info('All migrations completed');
      
    } catch (error) {
      this.logger.error('Migration failed', error);
      throw error;
    }
  }
  
  /**
   * Load migration files
   */
  private async loadMigrations(): Promise<any[]> {
    // This will be implemented to dynamically load migration files
    // For now, we'll import them directly
    const { migrations } = await import('./migrations');
    return migrations;
  }
}

// Export singleton instance
export const db = DatabaseConnection.getInstance();