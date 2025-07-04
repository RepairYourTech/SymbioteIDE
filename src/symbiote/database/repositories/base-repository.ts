/**
 * Base Repository - Abstract base class for all repositories
 */

import { Pool, Client } from 'pg';
import { db } from '../connection';
import { Logger } from '../../utils/logger';

export interface QueryOptions {
  orderBy?: string;
  limit?: number;
  offset?: number;
}

export abstract class BaseRepository<T> {
  protected pool: Pool;
  protected logger: Logger;
  protected tableName: string;
  protected schema: string = 'symbiote';
  
  constructor(tableName: string) {
    this.pool = db.getPool();
    this.tableName = tableName;
    this.logger = new Logger(`${tableName}Repository`);
  }
  
  /**
   * Get full table name with schema
   */
  protected get table(): string {
    return `${this.schema}.${this.tableName}`;
  }
  
  /**
   * Find by ID
   */
  async findById(id: string): Promise<T | null> {
    const query = `SELECT * FROM ${this.table} WHERE id = $1`;
    const result = await db.queryOne<T>(query, [id]);
    return result;
  }
  
  /**
   * Find all with options
   */
  async findAll(options?: QueryOptions): Promise<T[]> {
    let query = `SELECT * FROM ${this.table}`;
    const params: any[] = [];
    
    // Add ORDER BY
    if (options?.orderBy) {
      query += ` ORDER BY ${options.orderBy}`;
    }
    
    // Add LIMIT
    if (options?.limit) {
      params.push(options.limit);
      query += ` LIMIT $${params.length}`;
    }
    
    // Add OFFSET
    if (options?.offset) {
      params.push(options.offset);
      query += ` OFFSET $${params.length}`;
    }
    
    return db.query<T>(query, params);
  }
  
  /**
   * Find by condition
   */
  async findWhere(
    conditions: Record<string, any>,
    options?: QueryOptions
  ): Promise<T[]> {
    const keys = Object.keys(conditions);
    const values = Object.values(conditions);
    
    let query = `SELECT * FROM ${this.table} WHERE `;
    query += keys.map((key, i) => `${key} = $${i + 1}`).join(' AND ');
    
    // Add ORDER BY
    if (options?.orderBy) {
      query += ` ORDER BY ${options.orderBy}`;
    }
    
    // Add LIMIT
    if (options?.limit) {
      values.push(options.limit);
      query += ` LIMIT $${values.length}`;
    }
    
    // Add OFFSET
    if (options?.offset) {
      values.push(options.offset);
      query += ` OFFSET $${values.length}`;
    }
    
    return db.query<T>(query, values);
  }
  
  /**
   * Find one by condition
   */
  async findOneWhere(conditions: Record<string, any>): Promise<T | null> {
    const results = await this.findWhere(conditions, { limit: 1 });
    return results[0] || null;
  }
  
  /**
   * Insert new record
   */
  async insert(data: Partial<T>): Promise<T> {
    const keys = Object.keys(data);
    const values = Object.values(data);
    
    const columns = keys.join(', ');
    const placeholders = keys.map((_, i) => `$${i + 1}`).join(', ');
    
    const query = `
      INSERT INTO ${this.table} (${columns})
      VALUES (${placeholders})
      RETURNING *
    `;
    
    const result = await db.queryOne<T>(query, values);
    if (!result) {
      throw new Error('Insert failed');
    }
    
    this.logger.debug(`Inserted record into ${this.tableName}`, { id: (result as any).id });
    return result;
  }
  
  /**
   * Update record
   */
  async update(id: string, data: Partial<T>): Promise<T | null> {
    const keys = Object.keys(data);
    const values = Object.values(data);
    values.push(id); // Add ID as last parameter
    
    const setClause = keys.map((key, i) => `${key} = $${i + 1}`).join(', ');
    
    const query = `
      UPDATE ${this.table}
      SET ${setClause}
      WHERE id = $${values.length}
      RETURNING *
    `;
    
    const result = await db.queryOne<T>(query, values);
    
    if (result) {
      this.logger.debug(`Updated record in ${this.tableName}`, { id });
    }
    
    return result;
  }
  
  /**
   * Delete record
   */
  async delete(id: string): Promise<boolean> {
    const query = `DELETE FROM ${this.table} WHERE id = $1`;
    const result = await this.pool.query(query, [id]);
    
    const deleted = result.rowCount > 0;
    if (deleted) {
      this.logger.debug(`Deleted record from ${this.tableName}`, { id });
    }
    
    return deleted;
  }
  
  /**
   * Delete by condition
   */
  async deleteWhere(conditions: Record<string, any>): Promise<number> {
    const keys = Object.keys(conditions);
    const values = Object.values(conditions);
    
    let query = `DELETE FROM ${this.table} WHERE `;
    query += keys.map((key, i) => `${key} = $${i + 1}`).join(' AND ');
    
    const result = await this.pool.query(query, values);
    
    this.logger.debug(`Deleted ${result.rowCount} records from ${this.tableName}`);
    return result.rowCount;
  }
  
  /**
   * Count records
   */
  async count(conditions?: Record<string, any>): Promise<number> {
    let query = `SELECT COUNT(*) as count FROM ${this.table}`;
    const values: any[] = [];
    
    if (conditions) {
      const keys = Object.keys(conditions);
      query += ' WHERE ';
      query += keys.map((key, i) => {
        values.push(conditions[key]);
        return `${key} = $${i + 1}`;
      }).join(' AND ');
    }
    
    const result = await db.queryOne<{ count: string }>(query, values);
    return parseInt(result?.count || '0', 10);
  }
  
  /**
   * Check if exists
   */
  async exists(conditions: Record<string, any>): Promise<boolean> {
    const count = await this.count(conditions);
    return count > 0;
  }
  
  /**
   * Execute raw query
   */
  async query<R = T>(sql: string, params?: any[]): Promise<R[]> {
    return db.query<R>(sql, params);
  }
  
  /**
   * Execute in transaction
   */
  async transaction<R>(
    callback: (client: Client, repo: this) => Promise<R>
  ): Promise<R> {
    return db.transaction(async (client) => {
      // Create a temporary repository instance that uses the transaction client
      const transactionRepo = Object.create(this);
      transactionRepo.pool = client; // Override pool with client
      return callback(client, transactionRepo);
    });
  }
}