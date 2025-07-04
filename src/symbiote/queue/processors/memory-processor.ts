/**
 * Memory Processor - Handles memory consolidation and management jobs
 */

import { Job } from 'bullmq';
import { BaseProcessor } from './base-processor';
import { 
  MemoryJobData, 
  JobResult, 
  JobType 
} from '../types';
import { MemoryManager } from '../../memory/mem0/memory-manager';
import { QdrantManager } from '../../search/qdrant/qdrant-manager';
import { Neo4jConnectionManager } from '../../neo4j/connection-manager';
import { RedisManager } from '../../redis/redis-manager';

interface ConsolidationStrategy {
  consolidate(memories: any[]): Promise<any[]>;
}

export class MemoryProcessor extends BaseProcessor<MemoryJobData> {
  private memoryManager?: MemoryManager;
  private qdrantManager?: QdrantManager;
  private neo4jManager?: Neo4jConnectionManager;
  private redisManager?: RedisManager;

  constructor(queueManager: any) {
    super(queueManager, 'MemoryProcessor');
  }

  /**
   * Initialize services
   */
  private async initializeServices(): Promise<void> {
    if (!this.memoryManager) {
      this.qdrantManager = new QdrantManager({
        url: process.env.QDRANT_URL || 'http://localhost:6333',
        apiKey: process.env.QDRANT_API_KEY
      });
      await this.qdrantManager.initialize();
      
      this.neo4jManager = Neo4jConnectionManager.getInstance();
      await this.neo4jManager.connect();
      
      this.redisManager = RedisManager.getInstance();
      
      this.memoryManager = new MemoryManager({
        qdrantManager: this.qdrantManager,
        neo4jService: {
          getSession: () => this.neo4jManager!.getSession(),
          close: async () => {}
        }
      });
      
      await this.memoryManager.initialize();
    }
  }

  /**
   * Process memory job
   */
  async process(job: Job<MemoryJobData>): Promise<JobResult> {
    try {
      this.validateJobData(job.data);
      await this.initializeServices();

      switch (job.data.type) {
        case JobType.ConsolidateMemories:
          return await this.consolidateMemories(job);
          
        case JobType.PruneMemories:
          return await this.pruneMemories(job);
          
        case JobType.ExportMemories:
          return await this.exportMemories(job);
          
        default:
          throw new Error(`Unknown job type: ${job.data.type}`);
      }
    } catch (error: any) {
      return this.handleFailure(job, error);
    }
  }

  /**
   * Consolidate memories
   */
  private async consolidateMemories(job: Job<MemoryJobData>): Promise<JobResult> {
    const { userId, agentId, timeRange, options } = job.data;
    
    await this.updateProgress(job, 0, 'Fetching memories');
    
    // Fetch memories to consolidate
    const filter: any = {};
    if (userId) filter.userId = userId;
    if (agentId) filter.agentId = agentId;
    if (timeRange) {
      filter.timestamp = {
        $gte: timeRange.start.toISOString(),
        $lte: timeRange.end.toISOString()
      };
    }
    
    const memories = await this.memoryManager!.search('', {
      filter,
      limit: 1000
    });
    
    if (memories.length === 0) {
      return this.createSuccessResult({
        consolidated: 0,
        message: 'No memories found to consolidate'
      });
    }
    
    await this.updateProgress(job, 20, `Found ${memories.length} memories to consolidate`);
    
    // Select consolidation strategy
    const strategy = this.getConsolidationStrategy(options?.strategy || 'similarity');
    
    await this.updateProgress(job, 30, 'Grouping memories');
    
    // Group memories for consolidation
    const groups = await this.groupMemories(memories, options);
    const totalGroups = groups.length;
    
    await this.updateProgress(job, 40, `Processing ${totalGroups} groups`);
    
    // Consolidate each group
    const consolidatedMemories = [];
    let processedGroups = 0;
    
    for (const group of groups) {
      const consolidated = await strategy.consolidate(group);
      consolidatedMemories.push(...consolidated);
      
      processedGroups++;
      const progress = 40 + (50 * processedGroups / totalGroups);
      await this.updateProgress(
        job,
        progress,
        `Consolidated ${processedGroups}/${totalGroups} groups`
      );
    }
    
    await this.updateProgress(job, 90, 'Storing consolidated memories');
    
    // Store consolidated memories
    for (const memory of consolidatedMemories) {
      await this.memoryManager!.store(memory);
    }
    
    // Mark original memories as consolidated
    await this.markAsConsolidated(memories.map(m => m.id));
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      originalCount: memories.length,
      consolidatedCount: consolidatedMemories.length,
      reductionRatio: 1 - (consolidatedMemories.length / memories.length),
      groups: totalGroups
    });
  }

  /**
   * Prune old or irrelevant memories
   */
  private async pruneMemories(job: Job<MemoryJobData>): Promise<JobResult> {
    const { userId, agentId, options } = job.data;
    
    await this.updateProgress(job, 0, 'Analyzing memories for pruning');
    
    // Define pruning criteria
    const pruneCriteria = {
      // Old memories (> 30 days by default)
      ageThreshold: options?.threshold || 30 * 24 * 60 * 60 * 1000,
      // Low importance score
      importanceThreshold: 0.3,
      // Memories marked as consolidated
      includeConsolidated: true
    };
    
    // Find memories to prune
    const filter: any = {};
    if (userId) filter.userId = userId;
    if (agentId) filter.agentId = agentId;
    
    const memories = await this.memoryManager!.search('', {
      filter,
      limit: 5000
    });
    
    await this.updateProgress(job, 20, `Evaluating ${memories.length} memories`);
    
    // Evaluate each memory
    const memoriesToPrune = [];
    const now = Date.now();
    
    for (const memory of memories) {
      let shouldPrune = false;
      
      // Check age
      const age = now - new Date(memory.timestamp).getTime();
      if (age > pruneCriteria.ageThreshold) {
        shouldPrune = true;
      }
      
      // Check importance
      if (memory.metadata?.importance < pruneCriteria.importanceThreshold) {
        shouldPrune = true;
      }
      
      // Check if consolidated
      if (memory.metadata?.consolidated && pruneCriteria.includeConsolidated) {
        shouldPrune = true;
      }
      
      // Skip if recently accessed
      if (memory.metadata?.lastAccessed) {
        const lastAccessAge = now - new Date(memory.metadata.lastAccessed).getTime();
        if (lastAccessAge < 7 * 24 * 60 * 60 * 1000) { // 7 days
          shouldPrune = false;
        }
      }
      
      if (shouldPrune) {
        memoriesToPrune.push(memory);
      }
    }
    
    await this.updateProgress(job, 50, `Pruning ${memoriesToPrune.length} memories`);
    
    // Delete memories
    let deleted = 0;
    for (const memory of memoriesToPrune) {
      try {
        await this.memoryManager!.delete(memory.id);
        deleted++;
        
        if (deleted % 100 === 0) {
          const progress = 50 + (40 * deleted / memoriesToPrune.length);
          await this.updateProgress(
            job,
            progress,
            `Deleted ${deleted}/${memoriesToPrune.length} memories`
          );
        }
      } catch (error) {
        this.logger.error(`Failed to delete memory ${memory.id}`, error);
      }
    }
    
    await this.updateProgress(job, 90, 'Updating statistics');
    
    // Update statistics in Redis
    await this.updateMemoryStats({
      totalMemories: memories.length - deleted,
      prunedMemories: deleted,
      lastPruned: new Date().toISOString()
    });
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      evaluated: memories.length,
      pruned: deleted,
      remaining: memories.length - deleted,
      criteria: pruneCriteria
    });
  }

  /**
   * Export memories
   */
  private async exportMemories(job: Job<MemoryJobData>): Promise<JobResult> {
    const { userId, agentId, timeRange } = job.data;
    
    await this.updateProgress(job, 0, 'Preparing export');
    
    // Build filter
    const filter: any = {};
    if (userId) filter.userId = userId;
    if (agentId) filter.agentId = agentId;
    if (timeRange) {
      filter.timestamp = {
        $gte: timeRange.start.toISOString(),
        $lte: timeRange.end.toISOString()
      };
    }
    
    await this.updateProgress(job, 20, 'Fetching memories');
    
    // Fetch all memories
    const memories = await this.memoryManager!.search('', {
      filter,
      limit: 10000
    });
    
    await this.updateProgress(job, 50, `Exporting ${memories.length} memories`);
    
    // Format export data
    const exportData = {
      exportDate: new Date().toISOString(),
      version: '1.0',
      filter,
      count: memories.length,
      memories: memories.map(m => ({
        id: m.id,
        content: m.content,
        timestamp: m.timestamp,
        metadata: m.metadata,
        embedding: m.embedding?.slice(0, 10), // Only first 10 dimensions for preview
        relationships: m.relationships
      }))
    };
    
    await this.updateProgress(job, 80, 'Storing export');
    
    // Store export in Redis with TTL
    const exportId = `memory_export_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    await this.redisManager!.set(
      `export:${exportId}`,
      exportData,
      3600 * 24 // 24 hours TTL
    );
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      exportId,
      count: memories.length,
      size: JSON.stringify(exportData).length,
      expiresIn: '24 hours'
    });
  }

  /**
   * Get consolidation strategy
   */
  private getConsolidationStrategy(strategyName: string): ConsolidationStrategy {
    switch (strategyName) {
      case 'similarity':
        return new SimilarityConsolidationStrategy(this);
        
      case 'temporal':
        return new TemporalConsolidationStrategy(this);
        
      case 'importance':
        return new ImportanceConsolidationStrategy(this);
        
      default:
        return new SimilarityConsolidationStrategy(this);
    }
  }

  /**
   * Group memories for consolidation
   */
  private async groupMemories(
    memories: any[],
    options?: any
  ): Promise<any[][]> {
    const groups: any[][] = [];
    const used = new Set<string>();
    const threshold = options?.threshold || 0.8;
    
    for (const memory of memories) {
      if (used.has(memory.id)) continue;
      
      const group = [memory];
      used.add(memory.id);
      
      // Find similar memories
      for (const other of memories) {
        if (used.has(other.id)) continue;
        
        const similarity = await this.calculateSimilarity(memory, other);
        if (similarity >= threshold) {
          group.push(other);
          used.add(other.id);
        }
      }
      
      if (group.length > 1) {
        groups.push(group);
      }
    }
    
    return groups;
  }

  /**
   * Calculate similarity between memories
   */
  private async calculateSimilarity(memory1: any, memory2: any): Promise<number> {
    // Use embeddings if available
    if (memory1.embedding && memory2.embedding) {
      return this.cosineSimilarity(memory1.embedding, memory2.embedding);
    }
    
    // Fallback to text similarity
    return this.textSimilarity(memory1.content, memory2.content);
  }

  /**
   * Calculate cosine similarity
   */
  private cosineSimilarity(vec1: number[], vec2: number[]): number {
    let dotProduct = 0;
    let norm1 = 0;
    let norm2 = 0;
    
    for (let i = 0; i < vec1.length; i++) {
      dotProduct += vec1[i] * vec2[i];
      norm1 += vec1[i] * vec1[i];
      norm2 += vec2[i] * vec2[i];
    }
    
    return dotProduct / (Math.sqrt(norm1) * Math.sqrt(norm2));
  }

  /**
   * Calculate text similarity
   */
  private textSimilarity(text1: string, text2: string): number {
    const words1 = new Set(text1.toLowerCase().split(/\s+/));
    const words2 = new Set(text2.toLowerCase().split(/\s+/));
    
    const intersection = new Set([...words1].filter(x => words2.has(x)));
    const union = new Set([...words1, ...words2]);
    
    return union.size > 0 ? intersection.size / union.size : 0;
  }

  /**
   * Mark memories as consolidated
   */
  private async markAsConsolidated(memoryIds: string[]): Promise<void> {
    // Update in batches
    const batchSize = 100;
    
    for (let i = 0; i < memoryIds.length; i += batchSize) {
      const batch = memoryIds.slice(i, i + batchSize);
      
      // Update in Neo4j
      if (this.neo4jManager) {
        const session = this.neo4jManager.getSession();
        try {
          await session.run(
            `
            MATCH (m:Memory)
            WHERE m.id IN $ids
            SET m.consolidated = true,
                m.consolidatedAt = datetime()
            `,
            { ids: batch }
          );
        } finally {
          await session.close();
        }
      }
    }
  }

  /**
   * Update memory statistics
   */
  private async updateMemoryStats(stats: any): Promise<void> {
    await this.redisManager!.hset('memory:stats', stats);
  }

  /**
   * Validate job data
   */
  protected validateJobData(data: MemoryJobData): void {
    if (!data.type) {
      throw new Error('Job type is required');
    }
    
    if (data.timeRange) {
      if (!data.timeRange.start || !data.timeRange.end) {
        throw new Error('Time range must have start and end dates');
      }
      
      if (data.timeRange.start > data.timeRange.end) {
        throw new Error('Start date must be before end date');
      }
    }
  }
}

/**
 * Similarity-based consolidation strategy
 */
class SimilarityConsolidationStrategy implements ConsolidationStrategy {
  constructor(private processor: MemoryProcessor) {}
  
  async consolidate(memories: any[]): Promise<any[]> {
    if (memories.length < 2) return memories;
    
    // Sort by timestamp
    memories.sort((a, b) => 
      new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
    
    // Create consolidated memory
    const consolidated = {
      content: this.summarizeContent(memories),
      timestamp: new Date().toISOString(),
      metadata: {
        type: 'consolidated',
        sourceCount: memories.length,
        sourceIds: memories.map(m => m.id),
        userId: memories[0].metadata?.userId,
        agentId: memories[0].metadata?.agentId,
        importance: Math.max(...memories.map(m => m.metadata?.importance || 0))
      }
    };
    
    return [consolidated];
  }
  
  private summarizeContent(memories: any[]): string {
    // Simple concatenation with deduplication
    const contents = new Set(memories.map(m => m.content));
    return `Consolidated memory from ${memories.length} similar memories:\n\n` +
           Array.from(contents).join('\n\n');
  }
}

/**
 * Temporal consolidation strategy
 */
class TemporalConsolidationStrategy implements ConsolidationStrategy {
  constructor(private processor: MemoryProcessor) {}
  
  async consolidate(memories: any[]): Promise<any[]> {
    // Group by time periods (e.g., daily summaries)
    const dailyGroups = new Map<string, any[]>();
    
    for (const memory of memories) {
      const date = new Date(memory.timestamp).toISOString().split('T')[0];
      if (!dailyGroups.has(date)) {
        dailyGroups.set(date, []);
      }
      dailyGroups.get(date)!.push(memory);
    }
    
    const consolidated = [];
    
    for (const [date, group] of dailyGroups) {
      consolidated.push({
        content: `Daily summary for ${date}:\n\n` + 
                group.map(m => `- ${m.content}`).join('\n'),
        timestamp: new Date(`${date}T23:59:59Z`).toISOString(),
        metadata: {
          type: 'daily_summary',
          date,
          sourceCount: group.length,
          sourceIds: group.map(m => m.id)
        }
      });
    }
    
    return consolidated;
  }
}

/**
 * Importance-based consolidation strategy
 */
class ImportanceConsolidationStrategy implements ConsolidationStrategy {
  constructor(private processor: MemoryProcessor) {}
  
  async consolidate(memories: any[]): Promise<any[]> {
    // Keep only high-importance memories
    const importantMemories = memories.filter(
      m => (m.metadata?.importance || 0) >= 0.7
    );
    
    // Consolidate low-importance memories
    const lowImportanceMemories = memories.filter(
      m => (m.metadata?.importance || 0) < 0.7
    );
    
    const consolidated = [...importantMemories];
    
    if (lowImportanceMemories.length > 0) {
      consolidated.push({
        content: `Summary of ${lowImportanceMemories.length} low-importance memories`,
        timestamp: new Date().toISOString(),
        metadata: {
          type: 'low_importance_summary',
          sourceCount: lowImportanceMemories.length,
          sourceIds: lowImportanceMemories.map(m => m.id)
        }
      });
    }
    
    return consolidated;
  }
}