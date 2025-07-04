/**
 * Neo4j Connection Manager
 */

import neo4j, { Driver, Session } from 'neo4j-driver';

export interface Neo4jConfig {
  uri: string;
  username: string;
  password: string;
  database?: string;
}

export class Neo4jConnectionManager {
  private static instance: Neo4jConnectionManager;
  private driver: Driver | null = null;
  private config: Neo4jConfig | null = null;

  private constructor() {}

  static getInstance(): Neo4jConnectionManager {
    if (!Neo4jConnectionManager.instance) {
      Neo4jConnectionManager.instance = new Neo4jConnectionManager();
    }
    return Neo4jConnectionManager.instance;
  }

  async connect(config: Neo4jConfig): Promise<void> {
    if (this.driver) {
      await this.disconnect();
    }

    this.config = config;
    this.driver = neo4j.driver(
      config.uri,
      neo4j.auth.basic(config.username, config.password)
    );

    // Verify connectivity
    const session = this.driver.session();
    try {
      await session.run('RETURN 1');
    } finally {
      await session.close();
    }
  }

  async disconnect(): Promise<void> {
    if (this.driver) {
      await this.driver.close();
      this.driver = null;
    }
  }

  getSession(database?: string): Session {
    if (!this.driver) {
      throw new Error('Neo4j driver not initialized');
    }

    return this.driver.session({
      database: database || this.config?.database || 'neo4j'
    });
  }

  async runQuery(query: string, params?: any, database?: string): Promise<any> {
    const session = this.getSession(database);
    try {
      const result = await session.run(query, params);
      return result.records.map(record => record.toObject());
    } finally {
      await session.close();
    }
  }

  isConnected(): boolean {
    return this.driver !== null;
  }
}

export default Neo4jConnectionManager;