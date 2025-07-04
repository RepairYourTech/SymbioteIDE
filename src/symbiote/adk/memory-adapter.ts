/**
 * Memory Adapter
 * 
 * Adapts Mem0 and other memory systems for use with ADK agents
 */

import { Memory as ADKMemory } from './types';
import { MemoryConfig } from './types';
import { MemoryManager } from '../memory/mem0/memory-manager';
import { 
  Memory, 
  MemoryType, 
  MemoryScope,
  MemorySearchRequest 
} from '../memory/mem0/types';
import { QdrantManager } from '../memory/qdrant/qdrant-manager';
import { Neo4jConnectionManager } from '../knowledge/neo4j/connection-manager';
import { Neo4jService } from '../memory/mem0/neo4j-service';
import { Logger } from '../utils/logger';

export class MemoryAdapter {
  private logger = new Logger('MemoryAdapter');
  private memoryManager?: MemoryManager;

  constructor() {
    this.initializeMemoryManager();
  }

  /**
   * Initialize memory manager
   */
  private async initializeMemoryManager(): Promise<void> {
    try {
      // Initialize memory manager with existing systems
      const { QdrantManager } = await import('../search/qdrant/qdrant-manager');
      const { Neo4jService } = await import('../memory/mem0/neo4j-service');
      // Neo4jConnectionManager already imported at top
      
      const qdrantManager = new QdrantManager({
        url: process.env.QDRANT_URL || 'http://localhost:6333',
        apiKey: process.env.QDRANT_API_KEY
      });
      // QdrantManager initializes on construction
      
      const neo4jManager = Neo4jConnectionManager.getInstance();
      const neo4jService = new Neo4jService(neo4jManager);
      
      // Create Mem0Client
      const { Mem0Client } = await import('../memory/mem0/client');
      const mem0Client = new Mem0Client({
        apiKey: process.env.MEM0_API_KEY,
        baseUrl: process.env.MEM0_BASE_URL
      });
      
      this.memoryManager = new MemoryManager(mem0Client, qdrantManager, neo4jService);
      
      await this.memoryManager.initialize();
    } catch (error) {
      this.logger.error('Failed to initialize memory manager', error);
    }
  }

  /**
   * Create ADK memory instance
   */
  async createMemory(config: MemoryConfig): Promise<ADKMemory> {
    if (!this.memoryManager) {
      await this.initializeMemoryManager();
    }

    return new ADKMemoryImpl(this.memoryManager!, config);
  }
}

/**
 * ADK Memory Implementation
 */
class ADKMemoryImpl implements ADKMemory {
  type: string;
  private logger: Logger;
  private agentId: string = 'adk-agent';

  constructor(
    private memoryManager: MemoryManager,
    private config: MemoryConfig
  ) {
    this.logger = new Logger('ADKMemory');
    this.type = config.type;
  }

  /**
   * Store memory
   */
  async store(key: string, value: any): Promise<void> {
    try {
      const memory: Memory = {
        id: key,
        type: this.config.type as MemoryType,
        scope: this.config.scope as MemoryScope,
        content: JSON.stringify(value),
        metadata: {
          agentId: this.agentId,
          confidence: 0.8,
          importance: 0.5,
          shareable: this.config.scope !== 'agent'
        },
        createdAt: new Date(),
        updatedAt: new Date(),
        accessedAt: new Date(),
        accessCount: 0
      };

      await this.memoryManager.store(memory);
      this.logger.debug(`Stored memory: ${key}`);
    } catch (error) {
      this.logger.error('Failed to store memory', error);
      throw error;
    }
  }

  /**
   * Retrieve memory by ID
   */
  async retrieve(key: string): Promise<any> {
    try {
      const memory = await this.memoryManager.get(key);
      if (!memory) {
        return null;
      }

      try {
        return JSON.parse(memory.content);
      } catch {
        return memory.content;
      }
    } catch (error) {
      this.logger.error(`Failed to retrieve memory ${key}`, error);
      throw error;
    }
  }

  /**
   * Search memories
   */
  async search(query: string, options?: any): Promise<any[]> {
    try {
      const request: MemorySearchRequest = {
        query,
        type: options?.type || [this.config.type as MemoryType],
        scope: options?.scope || this.config.scope as MemoryScope,
        limit: options?.limit || 10,
        // filters: options?.filters  // TODO: Add filters to MemorySearchRequest type
      };

      const results = await this.memoryManager.search(request);
      
      return results.map(result => ({
        id: result.memory.id,
        content: result.memory.content,
        metadata: result.memory.metadata,
        score: result.score,
        createdAt: result.memory.createdAt
      }));
    } catch (error) {
      this.logger.error('Failed to search memories', error);
      throw error;
    }
  }

  /**
   * Update memory
   */
  async update(id: string, content: string, metadata?: any): Promise<void> {
    try {
      const existingMemory = await this.memoryManager.get(id);
      if (!existingMemory) {
        throw new Error(`Memory ${id} not found`);
      }

      const updatedMemory: Memory = {
        ...existingMemory,
        content,
        metadata: {
          ...existingMemory.metadata,
          ...metadata
        },
        updatedAt: new Date()
      };

      await this.memoryManager.update(id, updatedMemory);
      this.logger.debug(`Updated memory: ${id}`);
    } catch (error) {
      this.logger.error(`Failed to update memory ${id}`, error);
      throw error;
    }
  }

  /**
   * Delete memory
   */
  async delete(id: string): Promise<void> {
    try {
      await this.memoryManager.delete(id);
      this.logger.debug(`Deleted memory: ${id}`);
    } catch (error) {
      this.logger.error(`Failed to delete memory ${id}`, error);
      throw error;
    }
  }

  /**
   * Get context for agent
   */
  async getContext(limit: number = 10): Promise<string> {
    try {
      // Get recent memories for context
      const memories = await this.search('', { limit });
      
      if (memories.length === 0) {
        return '';
      }

      // Format memories as context
      const context = memories
        .map(m => `[${new Date(m.createdAt).toLocaleString()}] ${m.content}`)
        .join('\n\n');

      return `Previous context:\n${context}`;
    } catch (error) {
      this.logger.error('Failed to get context', error);
      return '';
    }
  }

  /**
   * Clear all memories (based on scope and retention)
   */
  async clear(): Promise<void> {
    try {
      // This would typically be handled by retention policies
      // For now, just log
      this.logger.info('Clear memories requested - retention policies apply');
    } catch (error) {
      this.logger.error('Failed to clear memories', error);
      throw error;
    }
  }

  /**
   * Set agent ID for memory attribution
   */
  setAgentId(agentId: string): void {
    this.agentId = agentId;
  }

  /**
   * Export memories
   */
  async export(format: 'json' | 'csv' = 'json'): Promise<string> {
    try {
      const memories = await this.search('', { limit: 1000 });
      
      if (format === 'json') {
        return JSON.stringify(memories, null, 2);
      }
      
      // CSV format
      const headers = ['ID', 'Content', 'Type', 'Created At'];
      const rows = memories.map(m => [
        m.id,
        m.content.replace(/"/g, '""'), // Escape quotes
        this.config.type,
        new Date(m.createdAt).toISOString()
      ]);
      
      const csv = [
        headers.join(','),
        ...rows.map(row => row.map(cell => `"${cell}"`).join(','))
      ].join('\n');
      
      return csv;
    } catch (error) {
      this.logger.error('Failed to export memories', error);
      throw error;
    }
  }

  /**
   * Import memories
   */
  async import(data: string, format: 'json' | 'csv' = 'json'): Promise<number> {
    try {
      let memories: any[] = [];
      
      if (format === 'json') {
        memories = JSON.parse(data);
      } else {
        // Parse CSV
        const lines = data.split('\n');
        const headers = lines[0].split(',').map(h => h.replace(/"/g, '').trim());
        
        for (let i = 1; i < lines.length; i++) {
          const values = lines[i].match(/(".*?"|[^,]+)/g) || [];
          const memory: any = {};
          
          headers.forEach((header, index) => {
            memory[header.toLowerCase().replace(' ', '_')] = 
              values[index]?.replace(/"/g, '').trim();
          });
          
          if (memory.content) {
            memories.push(memory);
          }
        }
      }
      
      // Import each memory
      let imported = 0;
      for (const memory of memories) {
        await this.store(memory.content, memory.metadata || {});
        imported++;
      }
      
      this.logger.info(`Imported ${imported} memories`);
      return imported;
    } catch (error) {
      this.logger.error('Failed to import memories', error);
      throw error;
    }
  }
}