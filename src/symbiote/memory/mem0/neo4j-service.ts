/**
 * Neo4j Service Wrapper for Memory System
 * 
 * Provides Neo4j operations for the memory manager
 */

import { Driver, Session, Result } from 'neo4j-driver';
import { Neo4jConnectionManager } from '../../knowledge/neo4j/connection-manager';
import { GraphConfig } from '../../knowledge/types/graph-types';
import { Logger } from '../../utils/logger';

export class Neo4jService {
  private connectionManager: Neo4jConnectionManager;
  private databaseId: string;
  private logger = new Logger('Neo4jService');
  
  constructor(
    connectionManager: Neo4jConnectionManager,
    databaseId: string = 'memory'
  ) {
    this.connectionManager = connectionManager;
    this.databaseId = databaseId;
  }
  
  /**
   * Initialize connection
   */
  async connect(config: GraphConfig): Promise<void> {
    await this.connectionManager.connect(this.databaseId, config);
  }
  
  /**
   * Run a Cypher query
   */
  async run(
    query: string,
    params?: Record<string, any>
  ): Promise<Result> {
    const session = this.connectionManager.getSession(this.databaseId);
    
    try {
      const result = await session.run(query, params);
      return result;
    } finally {
      await session.close();
    }
  }
  
  /**
   * Get related memory nodes
   */
  async getRelatedMemories(memoryId: string): Promise<string[]> {
    const query = `
      MATCH (m:Memory {id: $memoryId})-[:RELATED_TO]-(related:Memory)
      RETURN DISTINCT related.id AS id
      LIMIT 20
    `;
    
    const result = await this.run(query, { memoryId });
    return result.records.map(record => record.get('id'));
  }
  
  /**
   * Delete memory and its relationships
   */
  async deleteMemory(memoryId: string): Promise<void> {
    const query = `
      MATCH (m:Memory {id: $memoryId})
      DETACH DELETE m
    `;
    
    await this.run(query, { memoryId });
  }
}