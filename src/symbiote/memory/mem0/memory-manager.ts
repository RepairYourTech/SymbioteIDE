/**
 * Memory Manager
 * 
 * Manages persistent memories across Mem0, Qdrant, and Neo4j
 */

import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import { Logger } from '../../utils/logger';
import { Mem0Client } from './client';
import { QdrantManager } from '../../search/qdrant/qdrant-manager';
import { Neo4jService } from './neo4j-service';
import {
  IMemoryManager,
  Memory,
  MemoryType,
  MemoryScope,
  MemoryMetadata,
  MemorySearchRequest,
  MemorySearchResult,
  Learning,
  ConsolidationResult,
  TeamKnowledge,
  MemoryStats,
  MemoryDecayConfig,
  TaskMemory,
  CodePattern,
  CodingConvention,
  BestPractice,
  KnownIssue
} from './types';

export class MemoryManager extends EventEmitter implements IMemoryManager {
  private mem0Client: Mem0Client;
  private qdrantManager?: QdrantManager;
  private neo4jService?: Neo4jService;
  private logger = new Logger('MemoryManager');
  
  constructor(
    mem0Client: Mem0Client,
    qdrantManager?: QdrantManager,
    neo4jService?: Neo4jService
  ) {
    super();
    
    this.mem0Client = mem0Client;
    this.qdrantManager = qdrantManager;
    this.neo4jService = neo4jService;
  }
  
  /**
   * Initialize connections
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing memory manager...');
    
    // Connect to Mem0
    await this.mem0Client.connect();
    
    // Ensure Qdrant collection exists
    if (this.qdrantManager) {
      await this.qdrantManager.createCollection({
        name: 'agent_memories',
        vectorSize: 1536,
        distance: 'Cosine'
      });
    }
    
    // Create Neo4j schema if needed
    if (this.neo4jService) {
      await this.createNeo4jSchema();
    }
    
    this.logger.info('Memory manager initialized');
  }
  
  /**
   * Store a memory across all systems
   */
  async store(memory: Memory): Promise<string> {
    try {
      // Ensure ID
      if (!memory.id) {
        memory.id = `mem_${uuidv4()}`;
      }
      
      // Store in Mem0 (primary storage)
      const mem0Id = await this.mem0Client.add(memory.content, memory.metadata);
      memory.id = mem0Id;
      
      // Generate embedding and store in Qdrant
      if (this.qdrantManager) {
        const embedding = await this.generateEmbedding(memory);
        await this.qdrantManager.upsertEmbedding(embedding, {
          id: memory.id,
          type: 'memory' as any,
          name: memory.type,
          filePath: 'memory',
          startLine: 0,
          endLine: 0,
          language: 'memory',
          lastModified: memory.createdAt,
          metadata: memory.metadata as any
        }, 'agent_memories');
        
        memory.embedding = embedding;
      }
      
      // Create relationships in Neo4j
      if (this.neo4jService) {
        await this.createNeo4jRelationships(memory);
      }
      
      this.logger.debug('Stored memory', { id: memory.id, type: memory.type });
      this.emit('memory-stored', memory);
      
      return memory.id;
      
    } catch (error) {
      this.logger.error('Failed to store memory', error);
      throw error;
    }
  }
  
  /**
   * Store multiple memories
   */
  async storeMany(memories: Memory[]): Promise<string[]> {
    const ids: string[] = [];
    
    // Process in batches
    const batchSize = 10;
    for (let i = 0; i < memories.length; i += batchSize) {
      const batch = memories.slice(i, i + batchSize);
      
      const batchIds = await Promise.all(
        batch.map(memory => this.store(memory))
      );
      
      ids.push(...batchIds);
    }
    
    return ids;
  }
  
  /**
   * Get a memory by ID
   */
  async get(id: string): Promise<Memory | null> {
    try {
      const memory = await this.mem0Client.get(id);
      
      if (memory) {
        // Increment access count
        await this.incrementAccess(id);
      }
      
      return memory;
      
    } catch (error) {
      this.logger.error('Failed to get memory', error);
      throw error;
    }
  }
  
  /**
   * Search memories
   */
  async search(request: MemorySearchRequest): Promise<MemorySearchResult[]> {
    try {
      const results: MemorySearchResult[] = [];
      
      // If we have a query and Qdrant, use semantic search
      if (request.query && this.qdrantManager) {
        const qdrantResults = await this.qdrantManager.search({
          query: request.query,
          queryType: 'natural_language',
          filters: this.buildQdrantFilters(request),
          limit: request.limit || 20,
          includeContext: false
        });
        
        // Convert Qdrant results to memory results
        for (const result of qdrantResults) {
          const memory = await this.get(result.entity.id);
          if (memory) {
            results.push({
              memory,
              score: result.score,
              relevance: result.explanation || 'Semantic similarity'
            });
          }
        }
      } else {
        // Fallback to Mem0 search
        const mem0Results = await this.mem0Client.search(
          request.query || '',
          {
            limit: request.limit,
            filters: this.buildMem0Filters(request)
          }
        );
        
        for (const memory of mem0Results) {
          results.push({
            memory,
            score: 0.5,
            relevance: 'Text match'
          });
        }
      }
      
      // Apply additional filters
      return this.filterResults(results, request);
      
    } catch (error) {
      this.logger.error('Failed to search memories', error);
      throw error;
    }
  }
  
  /**
   * Get related memories
   */
  async getRelated(memoryId: string, limit: number = 10): Promise<Memory[]> {
    const memories: Memory[] = [];
    
    // Get the source memory
    const sourceMemory = await this.get(memoryId);
    if (!sourceMemory) return [];
    
    // Find semantically similar memories
    if (this.qdrantManager) {
      const similar = await this.qdrantManager.searchSimilar(memoryId, limit);
      
      for (const result of similar) {
        const memory = await this.get(result.entity.id);
        if (memory) {
          memories.push(memory);
        }
      }
    }
    
    // Find graph-related memories
    if (this.neo4jService && sourceMemory.metadata.graphNodeIds) {
      const relatedNodes = await this.neo4jService.getRelatedMemories(memoryId);
      
      for (const nodeId of relatedNodes) {
        const memory = await this.get(nodeId);
        if (memory && !memories.find(m => m.id === memory.id)) {
          memories.push(memory);
        }
      }
    }
    
    return memories.slice(0, limit);
  }
  
  /**
   * Update a memory
   */
  async update(id: string, updates: Partial<Memory>): Promise<void> {
    try {
      const existing = await this.get(id);
      if (!existing) {
        throw new Error(`Memory not found: ${id}`);
      }
      
      // Update in Mem0
      if (updates.content || updates.metadata) {
        await this.mem0Client.update(
          id,
          updates.content || existing.content,
          updates.metadata
        );
      }
      
      // Update embedding if content changed
      if (updates.content && this.qdrantManager) {
        const newMemory = { ...existing, ...updates };
        const embedding = await this.generateEmbedding(newMemory);
        
        await this.qdrantManager.upsertEmbedding(embedding, {
          id: newMemory.id,
          type: 'memory' as any,
          name: newMemory.type,
          filePath: 'memory',
          startLine: 0,
          endLine: 0,
          language: 'memory',
          lastModified: new Date(),
          metadata: newMemory.metadata as any
        }, 'agent_memories');
      }
      
      this.logger.debug('Updated memory', { id });
      this.emit('memory-updated', id);
      
    } catch (error) {
      this.logger.error('Failed to update memory', error);
      throw error;
    }
  }
  
  /**
   * Increment access count
   */
  async incrementAccess(id: string): Promise<void> {
    const memory = await this.mem0Client.get(id);
    if (!memory) return;
    
    await this.update(id, {
      accessedAt: new Date(),
      accessCount: (memory.accessCount || 0) + 1
    });
  }
  
  /**
   * Delete a memory
   */
  async delete(id: string): Promise<void> {
    try {
      // Delete from all systems
      await this.mem0Client.delete(id);
      
      if (this.qdrantManager) {
        await this.qdrantManager.deleteByFilter({
          key: 'id',
          match: { value: id }
        }, 'agent_memories');
      }
      
      if (this.neo4jService) {
        await this.neo4jService.deleteMemory(id);
      }
      
      this.logger.debug('Deleted memory', { id });
      this.emit('memory-deleted', id);
      
    } catch (error) {
      this.logger.error('Failed to delete memory', error);
      throw error;
    }
  }
  
  /**
   * Delete multiple memories
   */
  async deleteMany(ids: string[]): Promise<void> {
    await Promise.all(ids.map(id => this.delete(id)));
  }
  
  /**
   * Extract learnings from memories
   */
  async extractLearnings(memoryIds: string[]): Promise<Learning[]> {
    const learnings: Learning[] = [];
    const memories = await Promise.all(memoryIds.map(id => this.get(id)));
    
    for (const memory of memories) {
      if (!memory) continue;
      
      // Extract patterns from successful tasks
      if (memory.metadata.success) {
        learnings.push({
          insight: `Successful approach: ${memory.content}`,
          type: 'best_practice',
          confidence: memory.metadata.confidence,
          impact: 'medium',
          applicability: [memory.metadata.language || 'general']
        });
      }
      
      // Extract warnings from failures
      if (memory.metadata.errorMessage) {
        learnings.push({
          insight: `Avoid: ${memory.metadata.errorMessage}`,
          type: 'warning',
          confidence: memory.metadata.confidence,
          impact: 'high',
          applicability: [memory.metadata.language || 'general']
        });
      }
    }
    
    return learnings;
  }
  
  /**
   * Consolidate memories
   */
  async consolidateMemories(memoryIds: string[]): Promise<ConsolidationResult> {
    const memories = await Promise.all(memoryIds.map(id => this.get(id)));
    const validMemories = memories.filter(m => m !== null) as Memory[];
    
    if (validMemories.length === 0) {
      throw new Error('No valid memories to consolidate');
    }
    
    // Extract common patterns
    const patterns = await this.extractPatterns(validMemories);
    const learnings = await this.extractLearnings(memoryIds);
    
    // Create consolidated memory
    const consolidatedContent = this.createConsolidatedContent(validMemories, patterns);
    
    const consolidatedMemory: Memory = {
      id: `consolidated_${uuidv4()}`,
      type: MemoryType.Semantic,
      scope: MemoryScope.Team,
      content: consolidatedContent,
      metadata: {
        agentId: 'system',
        confidence: this.calculateAverageConfidence(validMemories),
        importance: 0.8,
        shareable: true,
        relatedMemories: memoryIds,
        tags: ['consolidated', 'pattern']
      },
      createdAt: new Date(),
      updatedAt: new Date(),
      accessedAt: new Date(),
      accessCount: 0
    };
    
    // Store consolidated memory
    await this.store(consolidatedMemory);
    
    // Delete original memories if configured
    // await this.deleteMany(memoryIds);
    
    return {
      originalMemories: memoryIds,
      consolidatedMemory,
      extractedPatterns: learnings,
      spaceSaved: validMemories.length - 1
    };
  }
  
  /**
   * Share memory with team
   */
  async shareWithTeam(memoryId: string, teamId: string): Promise<void> {
    const memory = await this.get(memoryId);
    if (!memory) {
      throw new Error(`Memory not found: ${memoryId}`);
    }
    
    if (!memory.metadata.shareable) {
      throw new Error('Memory is not shareable');
    }
    
    // Update memory scope and metadata
    await this.update(memoryId, {
      scope: MemoryScope.Team,
      metadata: {
        ...memory.metadata,
        teamId,
        sharedAt: new Date().toISOString()
      }
    });
    
    this.logger.info('Shared memory with team', { memoryId, teamId });
    this.emit('memory-shared', { memoryId, teamId });
  }
  
  /**
   * Get team knowledge
   */
  async getTeamKnowledge(teamId: string): Promise<TeamKnowledge> {
    // Search for team memories
    const memories = await this.search({
      scope: [MemoryScope.Team],
      teamId,
      limit: 1000
    });
    
    // Extract knowledge components
    const conventions = await this.extractConventions(memories);
    const bestPractices = await this.extractBestPractices(memories);
    const patterns = await this.extractPatterns(memories.map(r => r.memory));
    const issues = await this.extractKnownIssues(memories);
    const learnings = await this.extractLearnings(memories.map(r => r.memory.id));
    
    return {
      teamId,
      conventions,
      bestPractices,
      commonPatterns: patterns,
      knownIssues: issues,
      sharedLearnings: learnings
    };
  }
  
  /**
   * Apply memory decay
   */
  async applyDecay(config: MemoryDecayConfig): Promise<number> {
    if (!config.enabled) return 0;
    
    let decayedCount = 0;
    
    // Get old memories with low importance
    const oldMemories = await this.search({
      dateRange: {
        to: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000) // 30 days old
      },
      limit: 100
    });
    
    for (const result of oldMemories) {
      const memory = result.memory;
      
      // Skip if exempt
      if (config.exemptTags?.some(tag => memory.metadata.tags?.includes(tag))) {
        continue;
      }
      
      // Calculate new importance
      const ageInDays = (Date.now() - memory.updatedAt.getTime()) / (24 * 60 * 60 * 1000);
      const newImportance = memory.metadata.importance * Math.exp(-config.decayRate * ageInDays);
      
      if (newImportance < config.minImportance) {
        // Delete or consolidate
        if (newImportance < config.consolidationThreshold) {
          await this.delete(memory.id);
          decayedCount++;
        } else {
          // Mark for consolidation
          await this.update(memory.id, {
            metadata: {
              ...memory.metadata,
              importance: newImportance,
              tags: [...(memory.metadata.tags || []), 'consolidate']
            }
          });
        }
      }
    }
    
    return decayedCount;
  }
  
  /**
   * Get statistics
   */
  async getStatistics(): Promise<MemoryStats> {
    // This is a simplified implementation
    const allMemories = await this.search({ limit: 10000 });
    
    const byType: Record<MemoryType, number> = {
      [MemoryType.Episodic]: 0,
      [MemoryType.Semantic]: 0,
      [MemoryType.Working]: 0,
      [MemoryType.LongTerm]: 0
    };
    
    const byScope: Record<MemoryScope, number> = {
      [MemoryScope.Agent]: 0,
      [MemoryScope.Project]: 0,
      [MemoryScope.Team]: 0,
      [MemoryScope.Global]: 0
    };
    
    const agents = new Set<string>();
    const projects = new Set<string>();
    let totalConfidence = 0;
    
    for (const result of allMemories) {
      const memory = result.memory;
      byType[memory.type]++;
      byScope[memory.scope]++;
      agents.add(memory.metadata.agentId);
      if (memory.metadata.projectId) projects.add(memory.metadata.projectId);
      totalConfidence += memory.metadata.confidence;
    }
    
    // Sort by access count
    const sortedByAccess = [...allMemories].sort(
      (a, b) => (b.memory.accessCount || 0) - (a.memory.accessCount || 0)
    );
    
    // Sort by date
    const sortedByDate = [...allMemories].sort(
      (a, b) => b.memory.createdAt.getTime() - a.memory.createdAt.getTime()
    );
    
    return {
      totalMemories: allMemories.length,
      byType,
      byScope,
      totalAgents: agents.size,
      totalProjects: projects.size,
      averageConfidence: totalConfidence / allMemories.length || 0,
      mostAccessedMemories: sortedByAccess.slice(0, 10).map(r => r.memory),
      recentMemories: sortedByDate.slice(0, 10).map(r => r.memory),
      oldestMemories: sortedByDate.slice(-10).reverse().map(r => r.memory)
    };
  }
  
  /**
   * Optimize memory storage
   */
  async optimize(): Promise<void> {
    this.logger.info('Optimizing memory storage...');
    
    // Find memories to consolidate
    const toConsolidate = await this.search({
      tags: ['consolidate'],
      limit: 100
    });
    
    if (toConsolidate.length > 0) {
      // Group by similarity
      const groups = this.groupSimilarMemories(toConsolidate);
      
      // Consolidate each group
      for (const group of groups) {
        if (group.length > 1) {
          await this.consolidateMemories(group.map(r => r.memory.id));
        }
      }
    }
    
    // Optimize Qdrant collection
    if (this.qdrantManager) {
      await this.qdrantManager.optimize('agent_memories');
    }
    
    this.logger.info('Memory optimization complete');
  }
  
  // Private helper methods
  
  private async generateEmbedding(memory: Memory): Promise<number[]> {
    if (!this.qdrantManager) {
      // Return mock embedding
      return new Array(1536).fill(0).map(() => Math.random());
    }
    
    // Create rich text representation
    const text = `
      Type: ${memory.type}
      Scope: ${memory.scope}
      Content: ${memory.content}
      ${memory.metadata.taskDescription ? `Task: ${memory.metadata.taskDescription}` : ''}
      ${memory.metadata.language ? `Language: ${memory.metadata.language}` : ''}
      ${memory.metadata.tags ? `Tags: ${memory.metadata.tags.join(', ')}` : ''}
    `.trim();
    
    return await this.qdrantManager.embedCode(text, {
      id: memory.id,
      type: 'memory' as any,
      name: memory.type,
      filePath: 'memory',
      startLine: 0,
      endLine: 0,
      language: 'text',
      lastModified: memory.createdAt
    });
  }
  
  private async createNeo4jSchema(): Promise<void> {
    if (!this.neo4jService) return;
    
    await this.neo4jService.run(`
      CREATE CONSTRAINT memory_id IF NOT EXISTS
      FOR (m:Memory) REQUIRE m.id IS UNIQUE
    `);
    
    await this.neo4jService.run(`
      CREATE CONSTRAINT agent_id IF NOT EXISTS
      FOR (a:Agent) REQUIRE a.id IS UNIQUE
    `);
    
    await this.neo4jService.run(`
      CREATE CONSTRAINT task_id IF NOT EXISTS
      FOR (t:Task) REQUIRE t.id IS UNIQUE
    `);
  }
  
  private async createNeo4jRelationships(memory: Memory): Promise<void> {
    if (!this.neo4jService) return;
    
    // Create memory node
    await this.neo4jService.run(`
      MERGE (m:Memory {id: $memoryId})
      SET m += $properties
    `, {
      memoryId: memory.id,
      properties: {
        type: memory.type,
        scope: memory.scope,
        confidence: memory.metadata.confidence,
        importance: memory.metadata.importance
      }
    });
    
    // Link to agent
    if (memory.metadata.agentId) {
      await this.neo4jService.run(`
        MERGE (a:Agent {id: $agentId})
        MERGE (m:Memory {id: $memoryId})
        MERGE (a)-[:CREATED]->(m)
      `, {
        agentId: memory.metadata.agentId,
        memoryId: memory.id
      });
    }
    
    // Link to task
    if (memory.metadata.taskId) {
      await this.neo4jService.run(`
        MERGE (t:Task {id: $taskId})
        MERGE (m:Memory {id: $memoryId})
        MERGE (m)-[:RELATES_TO]->(t)
      `, {
        taskId: memory.metadata.taskId,
        memoryId: memory.id
      });
    }
    
    // Link to related memories
    if (memory.metadata.relatedMemories) {
      for (const relatedId of memory.metadata.relatedMemories) {
        await this.neo4jService.run(`
          MERGE (m1:Memory {id: $memoryId})
          MERGE (m2:Memory {id: $relatedId})
          MERGE (m1)-[:RELATED_TO]->(m2)
        `, {
          memoryId: memory.id,
          relatedId
        });
      }
    }
  }
  
  private buildQdrantFilters(request: MemorySearchRequest): any {
    const filters: any = {};
    
    if (request.type) {
      filters.type = request.type;
    }
    
    if (request.scope) {
      filters.scope = request.scope;
    }
    
    if (request.agentId) {
      filters['metadata.agentId'] = [request.agentId];
    }
    
    if (request.teamId) {
      filters['metadata.teamId'] = [request.teamId];
    }
    
    if (request.projectId) {
      filters['metadata.projectId'] = [request.projectId];
    }
    
    if (request.tags) {
      filters['metadata.tags'] = request.tags;
    }
    
    return filters;
  }
  
  private buildMem0Filters(request: MemorySearchRequest): any {
    const filters: any = {};
    
    if (request.agentId) filters.agent_id = request.agentId;
    if (request.teamId) filters.team_id = request.teamId;
    if (request.projectId) filters.project_id = request.projectId;
    
    return filters;
  }
  
  private filterResults(
    results: MemorySearchResult[],
    request: MemorySearchRequest
  ): MemorySearchResult[] {
    let filtered = results;
    
    // Filter by confidence
    if (request.minConfidence) {
      filtered = filtered.filter(r => r.memory.metadata.confidence >= request.minConfidence!);
    }
    
    // Filter by importance
    if (request.minImportance) {
      filtered = filtered.filter(r => r.memory.metadata.importance >= request.minImportance!);
    }
    
    // Filter by date range
    if (request.dateRange) {
      if (request.dateRange.from) {
        filtered = filtered.filter(r => r.memory.createdAt >= request.dateRange!.from!);
      }
      if (request.dateRange.to) {
        filtered = filtered.filter(r => r.memory.createdAt <= request.dateRange!.to!);
      }
    }
    
    // Apply limit
    if (request.limit) {
      filtered = filtered.slice(0, request.limit);
    }
    
    return filtered;
  }
  
  private async extractPatterns(memories: Memory[]): Promise<CodePattern[]> {
    // Simplified pattern extraction
    const patterns: CodePattern[] = [];
    
    // Group by language
    const byLanguage = new Map<string, Memory[]>();
    
    for (const memory of memories) {
      const lang = memory.metadata.language || 'general';
      if (!byLanguage.has(lang)) {
        byLanguage.set(lang, []);
      }
      byLanguage.get(lang)!.push(memory);
    }
    
    // Extract patterns per language
    for (const [language, langMemories] of byLanguage) {
      if (langMemories.length >= 3) {
        patterns.push({
          name: `${language} pattern`,
          description: `Common approach in ${language}`,
          useCase: 'General development',
          implementation: {
            code: langMemories[0].content,
            language
          },
          variations: langMemories.slice(1, 3).map(m => ({
            code: m.content,
            language
          })),
          frequency: langMemories.length
        });
      }
    }
    
    return patterns;
  }
  
  private createConsolidatedContent(memories: Memory[], patterns: CodePattern[]): string {
    const parts: string[] = [
      '# Consolidated Knowledge',
      '',
      `Based on ${memories.length} related memories:`,
      ''
    ];
    
    // Add patterns
    if (patterns.length > 0) {
      parts.push('## Identified Patterns');
      for (const pattern of patterns) {
        parts.push(`- ${pattern.name}: ${pattern.description}`);
      }
      parts.push('');
    }
    
    // Add key insights
    parts.push('## Key Insights');
    const insights = memories
      .filter(m => m.metadata.confidence > 0.7)
      .map(m => `- ${m.content}`)
      .slice(0, 5);
    parts.push(...insights);
    
    return parts.join('\n');
  }
  
  private calculateAverageConfidence(memories: Memory[]): number {
    const sum = memories.reduce((acc, m) => acc + m.metadata.confidence, 0);
    return sum / memories.length;
  }
  
  private groupSimilarMemories(results: MemorySearchResult[]): MemorySearchResult[][] {
    // Simple grouping by score similarity
    const groups: MemorySearchResult[][] = [];
    const used = new Set<string>();
    
    for (const result of results) {
      if (used.has(result.memory.id)) continue;
      
      const group = [result];
      used.add(result.memory.id);
      
      // Find similar scores
      for (const other of results) {
        if (!used.has(other.memory.id) && Math.abs(result.score - other.score) < 0.1) {
          group.push(other);
          used.add(other.memory.id);
        }
      }
      
      groups.push(group);
    }
    
    return groups;
  }
  
  private async extractConventions(results: MemorySearchResult[]): Promise<CodingConvention[]> {
    // Placeholder implementation
    return [];
  }
  
  private async extractBestPractices(results: MemorySearchResult[]): Promise<BestPractice[]> {
    // Placeholder implementation
    return [];
  }
  
  private async extractKnownIssues(results: MemorySearchResult[]): Promise<KnownIssue[]> {
    // Placeholder implementation
    return [];
  }
}