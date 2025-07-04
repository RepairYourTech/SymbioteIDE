/**
 * Neo4j Connection Manager
 * 
 * Manages multiple Neo4j connections for per-project and global knowledge graphs
 */

import neo4j, { Driver, Session, Result, Transaction } from 'neo4j-driver';
import { EventEmitter } from 'events';
import * as path from 'path';
import { 
  GraphConfig, 
  GraphContext, 
  GraphDatabase,
  GraphStatistics,
  GraphError,
  GraphErrorCode,
  NodeType,
  RelationType
} from '../types/graph-types';

export interface ConnectionManagerConfig {
  maxConnectionsPerProject: number;
  connectionTimeout: number;
  maxRetries: number;
  retryDelay: number;
  autoProvision: boolean;
  globalGraphConfig?: GraphConfig;
}

export interface ConnectionEventMap {
  'connected': (database: GraphDatabase) => void;
  'disconnected': (databaseId: string) => void;
  'error': (error: GraphError) => void;
  'statistics-updated': (databaseId: string, stats: GraphStatistics) => void;
}

export class Neo4jConnectionManager extends EventEmitter {
  private drivers: Map<string, Driver> = new Map();
  private databases: Map<string, GraphDatabase> = new Map();
  private config: ConnectionManagerConfig;
  private connectionRetries: Map<string, number> = new Map();
  
  constructor(config: Partial<ConnectionManagerConfig> = {}) {
    super();
    
    this.config = {
      maxConnectionsPerProject: config.maxConnectionsPerProject || 5,
      connectionTimeout: config.connectionTimeout || 30000,
      maxRetries: config.maxRetries || 3,
      retryDelay: config.retryDelay || 1000,
      autoProvision: config.autoProvision ?? true,
      globalGraphConfig: config.globalGraphConfig
    };
    
    // Initialize global connection if configured
    if (this.config.globalGraphConfig) {
      this.initializeGlobalConnection();
    }
  }
  
  /**
   * Initialize the global knowledge graph connection
   */
  private async initializeGlobalConnection(): Promise<void> {
    if (!this.config.globalGraphConfig) {
      return;
    }
    
    try {
      await this.connect('global', this.config.globalGraphConfig);
    } catch (error) {
      console.error('Failed to initialize global knowledge graph:', error);
      this.emit('error', new GraphError(
        'Failed to initialize global knowledge graph',
        GraphErrorCode.CONNECTION_FAILED,
        error
      ));
    }
  }
  
  /**
   * Connect to a Neo4j database
   */
  async connect(databaseId: string, config: GraphConfig): Promise<GraphDatabase> {
    try {
      // Check if already connected
      if (this.drivers.has(databaseId)) {
        const existingDb = this.databases.get(databaseId);
        if (existingDb && existingDb.status === 'connected') {
          return existingDb;
        }
      }
      
      // Create driver
      const driver = neo4j.driver(
        config.uri,
        neo4j.auth.basic(config.username, config.password),
        {
          maxConnectionPoolSize: config.options?.maxConnectionPoolSize || 10,
          connectionTimeout: config.options?.connectionTimeout || this.config.connectionTimeout,
          encrypted: config.options?.encrypted ?? 'ENCRYPTION_OFF'
        }
      );
      
      // Verify connectivity
      await driver.verifyConnectivity();
      
      // Store driver
      this.drivers.set(databaseId, driver);
      
      // Create database record
      const database: GraphDatabase = {
        id: databaseId,
        context: config.context,
        projectId: config.projectId,
        name: config.database || 'neo4j',
        status: 'connected',
        statistics: await this.gatherStatistics(driver, config.database),
        config
      };
      
      this.databases.set(databaseId, database);
      this.connectionRetries.delete(databaseId);
      
      // Set up auto-provisioning for project databases
      if (config.context === 'project' && this.config.autoProvision) {
        await this.provisionProjectDatabase(driver, database);
      }
      
      this.emit('connected', database);
      return database;
      
    } catch (error) {
      // Handle connection retry
      const retries = this.connectionRetries.get(databaseId) || 0;
      if (retries < this.config.maxRetries) {
        this.connectionRetries.set(databaseId, retries + 1);
        await new Promise(resolve => setTimeout(resolve, this.config.retryDelay));
        return this.connect(databaseId, config);
      }
      
      const graphError = new GraphError(
        `Failed to connect to Neo4j database: ${databaseId}`,
        GraphErrorCode.CONNECTION_FAILED,
        error
      );
      
      this.emit('error', graphError);
      throw graphError;
    }
  }
  
  /**
   * Disconnect from a database
   */
  async disconnect(databaseId: string): Promise<void> {
    const driver = this.drivers.get(databaseId);
    if (driver) {
      await driver.close();
      this.drivers.delete(databaseId);
      this.databases.delete(databaseId);
      this.emit('disconnected', databaseId);
    }
  }
  
  /**
   * Disconnect from all databases
   */
  async disconnectAll(): Promise<void> {
    const disconnectPromises = Array.from(this.drivers.keys()).map(id => 
      this.disconnect(id)
    );
    await Promise.all(disconnectPromises);
  }
  
  /**
   * Get a database connection
   */
  getDatabase(databaseId: string): GraphDatabase | undefined {
    return this.databases.get(databaseId);
  }
  
  /**
   * Get all connected databases
   */
  getAllDatabases(): GraphDatabase[] {
    return Array.from(this.databases.values());
  }
  
  /**
   * Get databases by context
   */
  getDatabasesByContext(context: GraphContext): GraphDatabase[] {
    return Array.from(this.databases.values()).filter(db => 
      db.context === context
    );
  }
  
  /**
   * Get project database
   */
  getProjectDatabase(projectId: string): GraphDatabase | undefined {
    return Array.from(this.databases.values()).find(db => 
      db.context === 'project' && db.projectId === projectId
    );
  }
  
  /**
   * Create a session for executing queries
   */
  createSession(databaseId: string, options?: { database?: string }): Session {
    const driver = this.drivers.get(databaseId);
    if (!driver) {
      throw new GraphError(
        `No connection found for database: ${databaseId}`,
        GraphErrorCode.CONNECTION_FAILED
      );
    }
    
    const database = this.databases.get(databaseId);
    const dbName = options?.database || database?.config.database || 'neo4j';
    
    return driver.session({ database: dbName });
  }
  
  /**
   * Get a session (alias for createSession for compatibility)
   */
  getSession(databaseId: string, options?: { database?: string }): Session {
    return this.createSession(databaseId, options);
  }
  
  /**
   * Execute a query
   */
  async executeQuery(
    databaseId: string,
    query: string,
    parameters?: Record<string, any>
  ): Promise<Result> {
    const session = this.createSession(databaseId);
    
    try {
      const result = await session.run(query, parameters);
      return result;
    } catch (error) {
      throw new GraphError(
        'Query execution failed',
        GraphErrorCode.QUERY_FAILED,
        { query, parameters, error }
      );
    } finally {
      await session.close();
    }
  }
  
  /**
   * Execute a transaction
   */
  async executeTransaction<T>(
    databaseId: string,
    work: (tx: Transaction) => Promise<T>
  ): Promise<T> {
    const session = this.createSession(databaseId);
    
    try {
      const result = await session.writeTransaction(work);
      return result;
    } catch (error) {
      throw new GraphError(
        'Transaction execution failed',
        GraphErrorCode.TRANSACTION_FAILED,
        error
      );
    } finally {
      await session.close();
    }
  }
  
  /**
   * Create a project database for a new project
   */
  async createProjectDatabase(projectId: string, workspacePath: string): Promise<GraphDatabase> {
    // Generate database name from project ID
    const dbName = `project_${projectId.replace(/[^a-zA-Z0-9]/g, '_')}`;
    
    // Default to local Neo4j instance for project databases
    const config: GraphConfig = {
      context: 'project',
      projectId,
      uri: process.env.NEO4J_URI || 'bolt://localhost:7687',
      username: process.env.NEO4J_USERNAME || 'neo4j',
      password: process.env.NEO4J_PASSWORD || 'password',
      database: dbName
    };
    
    // Connect and provision
    const database = await this.connect(projectId, config);
    
    // Initialize project metadata
    await this.executeQuery(projectId, `
      CREATE (p:Project {
        id: $projectId,
        workspacePath: $workspacePath,
        createdAt: datetime(),
        updatedAt: datetime()
      })
    `, { projectId, workspacePath });
    
    return database;
  }
  
  /**
   * Provision a project database with schema and indexes
   */
  private async provisionProjectDatabase(driver: Driver, database: GraphDatabase): Promise<void> {
    const session = driver.session({ database: database.name });
    
    try {
      // Create constraints for unique IDs
      const nodeTypes = Object.values(NodeType);
      for (const nodeType of nodeTypes) {
        try {
          await session.run(`
            CREATE CONSTRAINT ${nodeType.toLowerCase()}_id_unique IF NOT EXISTS
            FOR (n:${nodeType})
            REQUIRE n.id IS UNIQUE
          `);
        } catch (error) {
          // Constraint might already exist
          console.debug(`Constraint for ${nodeType} might already exist:`, error);
        }
      }
      
      // Create indexes for common queries
      const indexQueries = [
        // File path index
        'CREATE INDEX file_path_index IF NOT EXISTS FOR (f:File) ON (f.path)',
        
        // Function/Method name index
        'CREATE INDEX function_name_index IF NOT EXISTS FOR (f:Function) ON (f.name)',
        'CREATE INDEX method_name_index IF NOT EXISTS FOR (m:Method) ON (m.name)',
        
        // Class name index
        'CREATE INDEX class_name_index IF NOT EXISTS FOR (c:Class) ON (c.name)',
        
        // Package name and version index
        'CREATE INDEX package_name_version_index IF NOT EXISTS FOR (p:Package) ON (p.name, p.version)',
        
        // Timestamp indexes for change tracking
        'CREATE INDEX node_updated_index IF NOT EXISTS FOR (n) ON (n.updatedAt)',
        
        // Full-text search indexes
        'CREATE FULLTEXT INDEX code_search_index IF NOT EXISTS FOR (n:Function|Method|Class|Interface) ON EACH [n.name, n.description]'
      ];
      
      for (const indexQuery of indexQueries) {
        try {
          await session.run(indexQuery);
        } catch (error) {
          console.debug('Index creation error (might already exist):', error);
        }
      }
      
    } finally {
      await session.close();
    }
  }
  
  /**
   * Gather statistics about a database
   */
  private async gatherStatistics(driver: Driver, database?: string): Promise<GraphStatistics> {
    const session = driver.session({ database: database || 'neo4j' });
    
    try {
      // Get node count
      const nodeCountResult = await session.run('MATCH (n) RETURN count(n) as count');
      const nodeCount = nodeCountResult.records[0]?.get('count').toNumber() || 0;
      
      // Get relationship count
      const relCountResult = await session.run('MATCH ()-[r]->() RETURN count(r) as count');
      const relationshipCount = relCountResult.records[0]?.get('count').toNumber() || 0;
      
      // Get node type distribution
      const nodeTypesResult = await session.run(`
        MATCH (n)
        RETURN labels(n)[0] as label, count(n) as count
        ORDER BY count DESC
      `);
      
      const nodeTypes = new Map<string, number>();
      nodeTypesResult.records.forEach(record => {
        const label = record.get('label');
        const count = record.get('count').toNumber();
        if (label) {
          nodeTypes.set(label, count);
        }
      });
      
      // Get relationship type distribution
      const relTypesResult = await session.run(`
        MATCH ()-[r]->()
        RETURN type(r) as type, count(r) as count
        ORDER BY count DESC
      `);
      
      const relationshipTypes = new Map<string, number>();
      relTypesResult.records.forEach(record => {
        const type = record.get('type');
        const count = record.get('count').toNumber();
        relationshipTypes.set(type, count);
      });
      
      return {
        nodeCount,
        relationshipCount,
        nodeTypes,
        relationshipTypes,
        lastUpdated: new Date()
      };
      
    } catch (error) {
      console.error('Failed to gather statistics:', error);
      return {
        nodeCount: 0,
        relationshipCount: 0,
        nodeTypes: new Map(),
        relationshipTypes: new Map(),
        lastUpdated: new Date()
      };
    } finally {
      await session.close();
    }
  }
  
  /**
   * Update statistics for a database
   */
  async updateStatistics(databaseId: string): Promise<void> {
    const driver = this.drivers.get(databaseId);
    const database = this.databases.get(databaseId);
    
    if (!driver || !database) {
      return;
    }
    
    const statistics = await this.gatherStatistics(driver, database.config.database);
    database.statistics = statistics;
    
    this.emit('statistics-updated', databaseId, statistics);
  }
  
  /**
   * Check if a database exists
   */
  async databaseExists(databaseId: string): Promise<boolean> {
    return this.databases.has(databaseId);
  }
  
  /**
   * Health check for all connections
   */
  async healthCheck(): Promise<Map<string, boolean>> {
    const results = new Map<string, boolean>();
    
    for (const [id, driver] of this.drivers.entries()) {
      try {
        await driver.verifyConnectivity();
        results.set(id, true);
      } catch (error) {
        results.set(id, false);
        
        // Update database status
        const database = this.databases.get(id);
        if (database) {
          database.status = 'error';
        }
      }
    }
    
    return results;
  }
}