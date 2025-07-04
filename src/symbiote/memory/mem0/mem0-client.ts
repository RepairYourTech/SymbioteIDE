/**
 * Mem0 Client - Memory management client
 */

export interface Mem0ClientConfig {
  apiKey?: string;
  baseUrl?: string;
}

export interface Mem0AddOptions {
  content: string;
  userId?: string;
  metadata?: Record<string, any>;
}

export interface Mem0SearchOptions {
  query: string;
  userId?: string;
  limit?: number;
  filters?: Record<string, any>;
}

export interface Mem0Memory {
  id: string;
  content: string;
  userId?: string;
  metadata?: Record<string, any>;
  createdAt?: string;
  score?: number;
}

export class Mem0Client {
  private config: Mem0ClientConfig;
  private memories: Map<string, Mem0Memory> = new Map();

  constructor(config: Mem0ClientConfig) {
    this.config = config;
  }

  async add(options: Mem0AddOptions): Promise<string> {
    const id = `mem_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const memory: Mem0Memory = {
      id,
      content: options.content,
      userId: options.userId,
      metadata: options.metadata,
      createdAt: new Date().toISOString()
    };
    
    this.memories.set(id, memory);
    return id;
  }

  async get(id: string): Promise<Mem0Memory | null> {
    return this.memories.get(id) || null;
  }

  async update(id: string, options: Partial<Mem0AddOptions>): Promise<void> {
    const memory = this.memories.get(id);
    if (!memory) {
      throw new Error(`Memory ${id} not found`);
    }
    
    if (options.content !== undefined) {
      memory.content = options.content;
    }
    if (options.metadata !== undefined) {
      memory.metadata = { ...memory.metadata, ...options.metadata };
    }
  }

  async delete(id: string): Promise<void> {
    this.memories.delete(id);
  }

  async search(options: Mem0SearchOptions): Promise<Mem0Memory[]> {
    const results: Mem0Memory[] = [];
    const query = options.query.toLowerCase();
    
    for (const memory of this.memories.values()) {
      // Filter by userId if provided
      if (options.userId && memory.userId !== options.userId) {
        continue;
      }
      
      // Simple text search
      if (memory.content.toLowerCase().includes(query)) {
        results.push({
          ...memory,
          score: 0.5 // Mock score
        });
      }
      
      if (results.length >= (options.limit || 10)) {
        break;
      }
    }
    
    return results;
  }
}

export default Mem0Client;