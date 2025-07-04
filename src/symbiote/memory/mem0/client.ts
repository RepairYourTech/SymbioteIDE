/**
 * Mem0 Client Wrapper
 * 
 * Wraps the Mem0 SDK for persistent memory storage
 */

import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import { Memory, MemoryMetadata, MemorySearchRequest } from './types';

// Mem0 SDK types (these would come from @mem0/sdk when available)
interface Mem0Config {
  apiKey?: string;
  baseUrl?: string;
  userId?: string;
  agentId?: string;
  runId?: string;
}

interface Mem0Memory {
  id: string;
  memory: string;
  metadata?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

interface Mem0SearchResult {
  memory: Mem0Memory;
  score: number;
}

// Mock Mem0 client for now (replace with actual SDK when available)
class Mem0 {
  private config: Mem0Config;
  private memories: Map<string, Mem0Memory> = new Map();
  
  constructor(config: Mem0Config) {
    this.config = config;
  }
  
  async add(content: string, metadata?: Record<string, any>): Promise<{ id: string }> {
    const id = `mem_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const memory: Mem0Memory = {
      id,
      memory: content,
      metadata,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    
    this.memories.set(id, memory);
    return { id };
  }
  
  async get(id: string): Promise<Mem0Memory | null> {
    return this.memories.get(id) || null;
  }
  
  async search(query: string, options?: any): Promise<Mem0SearchResult[]> {
    // Simple mock search
    const results: Mem0SearchResult[] = [];
    
    for (const memory of this.memories.values()) {
      if (memory.memory.toLowerCase().includes(query.toLowerCase())) {
        results.push({
          memory,
          score: 0.8 // Mock score
        });
      }
    }
    
    return results;
  }
  
  async update(id: string, content: string, metadata?: Record<string, any>): Promise<void> {
    const memory = this.memories.get(id);
    if (memory) {
      memory.memory = content;
      memory.metadata = { ...memory.metadata, ...metadata };
      memory.updated_at = new Date().toISOString();
    }
  }
  
  async delete(id: string): Promise<void> {
    this.memories.delete(id);
  }
}

export class Mem0Client extends EventEmitter {
  private client: Mem0;
  private config: Mem0Config;
  private logger = new Logger('Mem0Client');
  private isConnected = false;
  
  constructor(config?: Mem0Config) {
    super();
    
    this.config = {
      apiKey: config?.apiKey || process.env.MEM0_API_KEY,
      baseUrl: config?.baseUrl || process.env.MEM0_BASE_URL || 'https://api.mem0.ai',
      userId: config?.userId,
      agentId: config?.agentId,
      runId: config?.runId
    };
    
    // Initialize client
    this.client = new Mem0(this.config);
  }
  
  /**
   * Connect to Mem0
   */
  async connect(): Promise<void> {
    try {
      this.logger.info('Connecting to Mem0...');
      
      // In real implementation, this would test the connection
      // For now, we just mark as connected
      this.isConnected = true;
      
      this.emit('connected');
      this.logger.info('Connected to Mem0');
      
    } catch (error) {
      this.logger.error('Failed to connect to Mem0', error);
      throw error;
    }
  }
  
  /**
   * Disconnect from Mem0
   */
  async disconnect(): Promise<void> {
    this.isConnected = false;
    this.emit('disconnected');
    this.logger.info('Disconnected from Mem0');
  }
  
  /**
   * Check connection status
   */
  isReady(): boolean {
    return this.isConnected;
  }
  
  /**
   * Add a memory
   */
  async add(content: string, metadata?: MemoryMetadata): Promise<string> {
    if (!this.isReady()) {
      throw new Error('Mem0 client not connected');
    }
    
    try {
      // Convert our metadata format to Mem0 format
      const mem0Metadata = this.convertMetadataToMem0(metadata);
      
      // Add to Mem0
      const result = await this.client.add(content, mem0Metadata);
      
      this.logger.debug('Added memory', { id: result.id });
      this.emit('memory-added', result.id);
      
      return result.id;
      
    } catch (error) {
      this.logger.error('Failed to add memory', error);
      throw error;
    }
  }
  
  /**
   * Get a memory by ID
   */
  async get(id: string): Promise<Memory | null> {
    if (!this.isReady()) {
      throw new Error('Mem0 client not connected');
    }
    
    try {
      const mem0Memory = await this.client.get(id);
      
      if (!mem0Memory) {
        return null;
      }
      
      return this.convertMem0ToMemory(mem0Memory);
      
    } catch (error) {
      this.logger.error('Failed to get memory', error);
      throw error;
    }
  }
  
  /**
   * Search memories
   */
  async search(query: string, options?: any): Promise<Memory[]> {
    if (!this.isReady()) {
      throw new Error('Mem0 client not connected');
    }
    
    try {
      const results = await this.client.search(query, options);
      
      return results.map(result => this.convertMem0ToMemory(result.memory));
      
    } catch (error) {
      this.logger.error('Failed to search memories', error);
      throw error;
    }
  }
  
  /**
   * Update a memory
   */
  async update(id: string, content: string, metadata?: Partial<MemoryMetadata>): Promise<void> {
    if (!this.isReady()) {
      throw new Error('Mem0 client not connected');
    }
    
    try {
      const mem0Metadata = metadata ? this.convertMetadataToMem0(metadata as MemoryMetadata) : undefined;
      
      await this.client.update(id, content, mem0Metadata);
      
      this.logger.debug('Updated memory', { id });
      this.emit('memory-updated', id);
      
    } catch (error) {
      this.logger.error('Failed to update memory', error);
      throw error;
    }
  }
  
  /**
   * Delete a memory
   */
  async delete(id: string): Promise<void> {
    if (!this.isReady()) {
      throw new Error('Mem0 client not connected');
    }
    
    try {
      await this.client.delete(id);
      
      this.logger.debug('Deleted memory', { id });
      this.emit('memory-deleted', id);
      
    } catch (error) {
      this.logger.error('Failed to delete memory', error);
      throw error;
    }
  }
  
  /**
   * Batch add memories
   */
  async batchAdd(memories: Array<{ content: string; metadata?: MemoryMetadata }>): Promise<string[]> {
    const ids: string[] = [];
    
    // Process in batches to avoid rate limits
    const batchSize = 10;
    for (let i = 0; i < memories.length; i += batchSize) {
      const batch = memories.slice(i, i + batchSize);
      
      const batchIds = await Promise.all(
        batch.map(memory => this.add(memory.content, memory.metadata))
      );
      
      ids.push(...batchIds);
    }
    
    return ids;
  }
  
  /**
   * Set context for all operations
   */
  setContext(context: { userId?: string; agentId?: string; runId?: string }): void {
    if (context.userId) this.config.userId = context.userId;
    if (context.agentId) this.config.agentId = context.agentId;
    if (context.runId) this.config.runId = context.runId;
    
    // Recreate client with new context
    this.client = new Mem0(this.config);
  }
  
  /**
   * Convert our metadata format to Mem0 format
   */
  private convertMetadataToMem0(metadata?: MemoryMetadata): Record<string, any> | undefined {
    if (!metadata) return undefined;
    
    return {
      // Core fields
      agent_id: metadata.agentId,
      user_id: metadata.userId,
      team_id: metadata.teamId,
      project_id: metadata.projectId,
      session_id: metadata.sessionId,
      
      // Task context
      task_id: metadata.taskId,
      task_description: metadata.taskDescription,
      task_type: metadata.taskType,
      
      // Memory properties
      confidence: metadata.confidence,
      importance: metadata.importance,
      shareable: metadata.shareable,
      
      // Code context
      language: metadata.language,
      framework: metadata.framework,
      files: metadata.files,
      
      // Learning context
      success: metadata.success,
      error_message: metadata.errorMessage,
      
      // Relationships
      related_memories: metadata.relatedMemories,
      parent_memory: metadata.parentMemory,
      
      // Integration IDs
      neo4j_node_ids: metadata.graphNodeIds,
      qdrant_vector_ids: metadata.vectorIds,
      
      // Custom fields
      tags: metadata.tags,
      labels: metadata.labels,
      ...metadata.extra
    };
  }
  
  /**
   * Convert Mem0 memory to our format
   */
  private convertMem0ToMemory(mem0Memory: Mem0Memory): Memory {
    const metadata = mem0Memory.metadata || {};
    
    return {
      id: mem0Memory.id,
      type: metadata.type || 'semantic',
      scope: metadata.scope || 'agent',
      content: mem0Memory.memory,
      metadata: {
        agentId: metadata.agent_id || '',
        userId: metadata.user_id,
        teamId: metadata.team_id,
        projectId: metadata.project_id,
        sessionId: metadata.session_id,
        taskId: metadata.task_id,
        taskDescription: metadata.task_description,
        taskType: metadata.task_type,
        confidence: metadata.confidence || 0.5,
        importance: metadata.importance || 0.5,
        shareable: metadata.shareable !== false,
        language: metadata.language,
        framework: metadata.framework,
        files: metadata.files,
        success: metadata.success,
        errorMessage: metadata.error_message,
        relatedMemories: metadata.related_memories,
        parentMemory: metadata.parent_memory,
        graphNodeIds: metadata.neo4j_node_ids,
        vectorIds: metadata.qdrant_vector_ids,
        tags: metadata.tags,
        labels: metadata.labels
      },
      createdAt: new Date(mem0Memory.created_at),
      updatedAt: new Date(mem0Memory.updated_at),
      accessedAt: new Date(mem0Memory.updated_at),
      accessCount: metadata.access_count || 0
    };
  }
}