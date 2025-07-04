/**
 * Qdrant Connection Manager
 * 
 * Manages connections to Qdrant vector database instances
 */

import { QdrantClient } from '@qdrant/js-client';
import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';

export interface QdrantConnectionConfig {
  url?: string;
  apiKey?: string;
  host?: string;
  port?: number;
  https?: boolean;
  prefix?: string;
  timeout?: number;
  retries?: number;
}

export interface ConnectionStatus {
  connected: boolean;
  lastConnected?: Date;
  lastError?: Error;
  retryCount: number;
  collections: string[];
}

export class QdrantConnectionManager extends EventEmitter {
  private client: QdrantClient | null = null;
  private config: QdrantConnectionConfig;
  private status: ConnectionStatus;
  private reconnectTimer?: NodeJS.Timeout;
  private healthCheckInterval?: NodeJS.Timeout;
  private logger = new Logger('QdrantConnectionManager');
  
  constructor(config: QdrantConnectionConfig = {}) {
    super();
    
    this.config = {
      host: config.host || process.env.QDRANT_HOST || 'localhost',
      port: config.port || parseInt(process.env.QDRANT_PORT || '6333'),
      https: config.https ?? process.env.QDRANT_HTTPS === 'true',
      apiKey: config.apiKey || process.env.QDRANT_API_KEY,
      timeout: config.timeout || 30000,
      retries: config.retries || 3,
      ...config
    };
    
    this.status = {
      connected: false,
      retryCount: 0,
      collections: []
    };
  }
  
  /**
   * Connect to Qdrant
   */
  async connect(): Promise<void> {
    try {
      this.logger.info('Connecting to Qdrant...', {
        host: this.config.host,
        port: this.config.port,
        https: this.config.https
      });
      
      // Create client
      this.client = new QdrantClient({
        url: this.config.url,
        apiKey: this.config.apiKey,
        host: this.config.host,
        port: this.config.port,
        https: this.config.https,
        prefix: this.config.prefix
      });
      
      // Test connection
      const info = await this.client.getCollections();
      
      this.status.connected = true;
      this.status.lastConnected = new Date();
      this.status.retryCount = 0;
      this.status.collections = info.collections.map(c => c.name);
      
      this.emit('connected', info);
      this.logger.info('Connected to Qdrant successfully', {
        collections: this.status.collections.length
      });
      
      // Start health check
      this.startHealthCheck();
      
    } catch (error) {
      this.status.connected = false;
      this.status.lastError = error as Error;
      
      this.logger.error('Failed to connect to Qdrant', error);
      this.emit('error', error);
      
      // Schedule reconnection
      if (this.status.retryCount < this.config.retries!) {
        this.scheduleReconnect();
      } else {
        throw new Error(`Failed to connect to Qdrant after ${this.config.retries} attempts: ${error.message}`);
      }
    }
  }
  
  /**
   * Disconnect from Qdrant
   */
  async disconnect(): Promise<void> {
    this.logger.info('Disconnecting from Qdrant...');
    
    // Clear timers
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
      this.healthCheckInterval = undefined;
    }
    
    // Clear client
    this.client = null;
    this.status.connected = false;
    
    this.emit('disconnected');
    this.logger.info('Disconnected from Qdrant');
  }
  
  /**
   * Get Qdrant client
   */
  getClient(): QdrantClient {
    if (!this.client || !this.status.connected) {
      throw new Error('Not connected to Qdrant');
    }
    return this.client;
  }
  
  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.status.connected;
  }
  
  /**
   * Get connection status
   */
  getStatus(): ConnectionStatus {
    return { ...this.status };
  }
  
  /**
   * Create collection if not exists
   */
  async ensureCollection(
    name: string,
    vectorSize: number,
    distance: 'Cosine' | 'Euclid' | 'Dot' = 'Cosine'
  ): Promise<void> {
    const client = this.getClient();
    
    try {
      // Check if collection exists
      const collections = await client.getCollections();
      const exists = collections.collections.some(c => c.name === name);
      
      if (!exists) {
        this.logger.info(`Creating collection: ${name}`);
        
        await client.createCollection(name, {
          vectors: {
            size: vectorSize,
            distance
          },
          optimizers_config: {
            default_segment_number: 2,
            memmap_threshold: 20000
          },
          replication_factor: 1
        });
        
        // Update status
        this.status.collections.push(name);
        this.emit('collection-created', name);
        
        this.logger.info(`Collection created: ${name}`);
      }
    } catch (error) {
      this.logger.error(`Failed to create collection ${name}`, error);
      throw error;
    }
  }
  
  /**
   * Delete collection
   */
  async deleteCollection(name: string): Promise<void> {
    const client = this.getClient();
    
    try {
      await client.deleteCollection(name);
      
      // Update status
      this.status.collections = this.status.collections.filter(c => c !== name);
      this.emit('collection-deleted', name);
      
      this.logger.info(`Collection deleted: ${name}`);
    } catch (error) {
      this.logger.error(`Failed to delete collection ${name}`, error);
      throw error;
    }
  }
  
  /**
   * List all collections
   */
  async listCollections(): Promise<string[]> {
    const client = this.getClient();
    
    try {
      const result = await client.getCollections();
      this.status.collections = result.collections.map(c => c.name);
      return this.status.collections;
    } catch (error) {
      this.logger.error('Failed to list collections', error);
      throw error;
    }
  }
  
  /**
   * Get collection info
   */
  async getCollectionInfo(name: string): Promise<any> {
    const client = this.getClient();
    
    try {
      return await client.getCollection(name);
    } catch (error) {
      this.logger.error(`Failed to get collection info for ${name}`, error);
      throw error;
    }
  }
  
  /**
   * Schedule reconnection
   */
  private scheduleReconnect(): void {
    const delay = Math.min(1000 * Math.pow(2, this.status.retryCount), 30000);
    
    this.logger.info(`Scheduling reconnection in ${delay}ms (attempt ${this.status.retryCount + 1})`);
    
    this.reconnectTimer = setTimeout(async () => {
      this.status.retryCount++;
      try {
        await this.connect();
      } catch (error) {
        this.logger.error('Reconnection failed', error);
      }
    }, delay);
  }
  
  /**
   * Start health check
   */
  private startHealthCheck(): void {
    this.healthCheckInterval = setInterval(async () => {
      try {
        const client = this.getClient();
        await client.getCollections();
        
        if (!this.status.connected) {
          this.status.connected = true;
          this.emit('reconnected');
        }
      } catch (error) {
        this.logger.warn('Health check failed', error);
        
        if (this.status.connected) {
          this.status.connected = false;
          this.status.lastError = error as Error;
          this.emit('connection-lost', error);
          
          // Try to reconnect
          this.scheduleReconnect();
        }
      }
    }, 30000); // Check every 30 seconds
  }
  
  /**
   * Execute with retry
   */
  async executeWithRetry<T>(
    operation: () => Promise<T>,
    retries: number = this.config.retries!
  ): Promise<T> {
    let lastError: Error | null = null;
    
    for (let i = 0; i <= retries; i++) {
      try {
        return await operation();
      } catch (error) {
        lastError = error as Error;
        
        if (i < retries) {
          const delay = Math.min(1000 * Math.pow(2, i), 10000);
          this.logger.warn(`Operation failed, retrying in ${delay}ms...`, error);
          await new Promise(resolve => setTimeout(resolve, delay));
          
          // Ensure we're connected
          if (!this.status.connected) {
            await this.connect();
          }
        }
      }
    }
    
    throw lastError;
  }
}