/**
 * Unified Context API
 * 
 * Combines Neo4j, Qdrant, and Mem0 into a unified context system for AI agents
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { MemoryManager } from './mem0/memory-manager';
import { QdrantManager } from '../search/qdrant/qdrant-manager';
import { Neo4jConnectionManager } from '../knowledge/neo4j/connection-manager';
import { AIContextBuilder } from '../search/qdrant/ai-context-builder';
import { RedisManager, PubSubChannel } from '../redis';
import {
  Memory,
  MemoryType,
  MemoryScope,
  MemorySearchRequest,
  MemorySearchResult,
  TaskContext,
  Learning,
  TeamKnowledge
} from './mem0/types';
import {
  CodeContext,
  CodeEntityType,
  SearchRequest
} from '../search/qdrant/types';

export interface UnifiedContextConfig {
  memoryManager: MemoryManager;
  qdrantManager: QdrantManager;
  neo4jManager: Neo4jConnectionManager;
  redisManager?: RedisManager;
  defaultProjectId?: string;
  enableRealtime?: boolean;
}

export interface ContextQuery {
  query: string;
  taskContext?: TaskContext;
  memoryTypes?: MemoryType[];
  includeCode?: boolean;
  includeMemories?: boolean;
  includeGraph?: boolean;
  limit?: number;
}

export interface UnifiedContext {
  // Code context from Qdrant
  codeContext?: CodeContext;
  
  // Memories from Mem0
  memories?: MemorySearchResult[];
  
  // Graph insights from Neo4j
  graphInsights?: GraphInsights;
  
  // Combined summary
  summary: string;
  
  // Relevance score
  relevanceScore: number;
  
  // Metadata
  metadata: {
    queryTime: number;
    sources: string[];
    projectId?: string;
  };
}

export interface GraphInsights {
  relatedEntities: Array<{
    type: string;
    name: string;
    relationships: string[];
  }>;
  patterns: string[];
  dependencies: string[];
  complexity: number;
}

export interface LearningContext {
  task: TaskContext;
  outcome: 'success' | 'failure';
  solution?: string;
  error?: string;
  duration: number;
  filesModified?: string[];
}

export class UnifiedContextAPI extends EventEmitter {
  private memoryManager: MemoryManager;
  private qdrantManager: QdrantManager;
  private neo4jManager: Neo4jConnectionManager;
  private contextBuilder: AIContextBuilder;
  private redisManager?: RedisManager;
  private config: UnifiedContextConfig;
  private logger = new Logger('UnifiedContextAPI');
  
  constructor(config: UnifiedContextConfig) {
    super();
    
    this.config = config;
    this.memoryManager = config.memoryManager;
    this.qdrantManager = config.qdrantManager;
    this.neo4jManager = config.neo4jManager;
    this.redisManager = config.redisManager;
    
    // Create AI context builder
    this.contextBuilder = new AIContextBuilder(
      this.qdrantManager,
      undefined // Neo4j service will be injected later
    );
    
    // Set up real-time updates if enabled
    if (config.enableRealtime && this.redisManager) {
      this.setupRealtimeUpdates();
    }
  }
  
  /**
   * Initialize the unified context system
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing unified context API...');
    
    // Initialize memory manager
    await this.memoryManager.initialize();
    
    // Connect to Qdrant
    await this.qdrantManager.connect();
    
    // Set up Redis subscriptions
    if (this.redisManager) {
      await this.setupRedisHandlers();
    }
    
    this.logger.info('Unified context API initialized');
    this.emit('initialized');
  }
  
  /**
   * Search for relevant context across all systems
   */
  async search(query: ContextQuery): Promise<UnifiedContext> {
    const startTime = Date.now();
    const sources: string[] = [];
    
    try {
      // Update context builder with current task
      if (query.taskContext) {
        this.contextBuilder.updateContext({
          taskDescription: query.taskContext.description,
          currentFile: query.taskContext.currentFile,
          recentFiles: query.taskContext.recentFiles
        });
      }
      
      // Parallel searches across all systems
      const [codeContext, memories, graphInsights] = await Promise.all([
        query.includeCode !== false ? this.searchCode(query) : undefined,
        query.includeMemories !== false ? this.searchMemories(query) : undefined,
        query.includeGraph !== false ? this.searchGraph(query) : undefined
      ]);
      
      // Track sources
      if (codeContext) sources.push('qdrant');
      if (memories?.length) sources.push('mem0');
      if (graphInsights) sources.push('neo4j');
      
      // Generate unified summary
      const summary = this.generateSummary({
        query: query.query,
        codeContext,
        memories,
        graphInsights
      });
      
      // Calculate relevance score
      const relevanceScore = this.calculateRelevance({
        codeContext,
        memories,
        graphInsights
      });
      
      const result: UnifiedContext = {
        codeContext,
        memories,
        graphInsights,
        summary,
        relevanceScore,
        metadata: {
          queryTime: Date.now() - startTime,
          sources,
          projectId: this.config.defaultProjectId
        }
      };
      
      // Cache result if Redis available
      if (this.redisManager) {
        await this.cacheResult(query, result);
      }
      
      this.emit('search-complete', { query, result });
      return result;
      
    } catch (error) {
      this.logger.error('Unified search failed', error);
      throw error;
    }
  }
  
  /**
   * Store a learning from task execution
   */
  async learn(context: LearningContext): Promise<Learning[]> {
    try {
      // Create task memory
      const taskMemory: Memory = {
        id: `task_${Date.now()}`,
        type: MemoryType.Episodic,
        scope: MemoryScope.Project,
        content: `Task: ${context.task.description}. Outcome: ${context.outcome}. ${
          context.outcome === 'success' ? 
          `Solution: ${context.solution}` : 
          `Error: ${context.error}`
        }`,
        metadata: {
          agentId: 'system',
          projectId: this.config.defaultProjectId,
          taskId: context.task.taskId,
          taskDescription: context.task.description,
          taskType: context.task.type,
          confidence: context.outcome === 'success' ? 0.9 : 0.3,
          importance: 0.7,
          shareable: true,
          success: context.outcome === 'success',
          errorMessage: context.error,
          solution: context.solution,
          files: context.filesModified,
          language: context.task.language,
          framework: context.task.framework
        },
        createdAt: new Date(),
        updatedAt: new Date(),
        accessedAt: new Date(),
        accessCount: 0
      };
      
      // Store memory
      await this.memoryManager.store(taskMemory);
      
      // Extract learnings
      const learnings = await this.memoryManager.extractLearnings([taskMemory.id]);
      
      // Update code embeddings if files were modified
      if (context.filesModified && context.filesModified.length > 0) {
        await this.updateCodeEmbeddings(context.filesModified);
      }
      
      // Broadcast learning event
      if (this.redisManager) {
        await this.redisManager.publish(
          PubSubChannel.MemoryUpdates,
          'learning',
          { context, learnings }
        );
      }
      
      this.emit('learning-stored', { context, learnings });
      return learnings;
      
    } catch (error) {
      this.logger.error('Failed to store learning', error);
      throw error;
    }
  }
  
  /**
   * Get team knowledge
   */
  async getTeamKnowledge(teamId?: string): Promise<TeamKnowledge> {
    const id = teamId || 'default';
    return this.memoryManager.getTeamKnowledge(id);
  }
  
  /**
   * Share memory with team
   */
  async shareMemory(memoryId: string, teamId?: string): Promise<void> {
    const id = teamId || 'default';
    await this.memoryManager.shareWithTeam(memoryId, id);
    
    // Broadcast to team members
    if (this.redisManager) {
      await this.redisManager.publish(
        PubSubChannel.MemorySync,
        'memory-shared',
        { memoryId, teamId: id }
      );
    }
  }
  
  /**
   * Get context for a specific file
   */
  async getFileContext(filePath: string): Promise<UnifiedContext> {
    // Search for all references to this file
    const query: ContextQuery = {
      query: filePath,
      includeCode: true,
      includeMemories: true,
      includeGraph: true,
      taskContext: {
        taskId: 'file-context',
        description: `Understanding ${filePath}`,
        type: 'analysis',
        currentFile: filePath
      }
    };
    
    return this.search(query);
  }
  
  /**
   * Get similar implementations
   */
  async findSimilarImplementations(
    code: string,
    language?: string
  ): Promise<CodeContext> {
    const patterns = await this.contextBuilder.findSimilarPatterns(code);
    
    // Store as a memory for future reference
    const memory: Memory = {
      id: `pattern_${Date.now()}`,
      type: MemoryType.Semantic,
      scope: MemoryScope.Project,
      content: `Code pattern: ${code.substring(0, 200)}...`,
      metadata: {
        agentId: 'system',
        projectId: this.config.defaultProjectId,
        confidence: 0.7,
        importance: 0.5,
        shareable: true,
        language,
        codeSnippets: [{
          code,
          language: language || 'unknown'
        }]
      },
      createdAt: new Date(),
      updatedAt: new Date(),
      accessedAt: new Date(),
      accessCount: 0
    };
    
    await this.memoryManager.store(memory);
    
    return patterns;
  }
  
  // Private helper methods
  
  private async searchCode(query: ContextQuery): Promise<CodeContext | undefined> {
    try {
      return await this.contextBuilder.findRelevantCode(query.query);
    } catch (error) {
      this.logger.error('Code search failed', error);
      return undefined;
    }
  }
  
  private async searchMemories(query: ContextQuery): Promise<MemorySearchResult[]> {
    try {
      const request: MemorySearchRequest = {
        query: query.query,
        type: query.memoryTypes,
        projectId: this.config.defaultProjectId,
        limit: query.limit || 10,
        minConfidence: 0.5
      };
      
      return await this.memoryManager.search(request);
    } catch (error) {
      this.logger.error('Memory search failed', error);
      return [];
    }
  }
  
  private async searchGraph(query: ContextQuery): Promise<GraphInsights | undefined> {
    try {
      // This would query Neo4j for graph insights
      // For now, return mock data
      return {
        relatedEntities: [],
        patterns: [],
        dependencies: [],
        complexity: 0
      };
    } catch (error) {
      this.logger.error('Graph search failed', error);
      return undefined;
    }
  }
  
  private generateSummary(data: {
    query: string;
    codeContext?: CodeContext;
    memories?: MemorySearchResult[];
    graphInsights?: GraphInsights;
  }): string {
    const parts: string[] = [];
    
    // Add code context summary
    if (data.codeContext?.summary) {
      parts.push(`Code Context: ${data.codeContext.summary}`);
    }
    
    // Add memory insights
    if (data.memories && data.memories.length > 0) {
      const topMemory = data.memories[0];
      parts.push(`Related Experience: ${topMemory.memory.content.substring(0, 200)}...`);
    }
    
    // Add graph insights
    if (data.graphInsights && data.graphInsights.patterns.length > 0) {
      parts.push(`Patterns: ${data.graphInsights.patterns.join(', ')}`);
    }
    
    return parts.join('\n\n') || 'No relevant context found.';
  }
  
  private calculateRelevance(data: {
    codeContext?: CodeContext;
    memories?: MemorySearchResult[];
    graphInsights?: GraphInsights;
  }): number {
    let score = 0;
    let sources = 0;
    
    if (data.codeContext && data.codeContext.primary.length > 0) {
      score += 0.4;
      sources++;
    }
    
    if (data.memories && data.memories.length > 0) {
      score += 0.3 * Math.min(data.memories[0].score, 1);
      sources++;
    }
    
    if (data.graphInsights && data.graphInsights.relatedEntities.length > 0) {
      score += 0.3;
      sources++;
    }
    
    // Bonus for multiple sources
    if (sources > 1) {
      score += 0.1 * (sources - 1);
    }
    
    return Math.min(score, 1);
  }
  
  private async updateCodeEmbeddings(files: string[]): Promise<void> {
    // This would trigger re-indexing of modified files
    this.emit('files-modified', { files });
  }
  
  private async cacheResult(
    query: ContextQuery,
    result: UnifiedContext
  ): Promise<void> {
    if (!this.redisManager) return;
    
    const key = `context:${this.hashQuery(query)}`;
    await this.redisManager.set(key, result, 300); // 5 minute TTL
  }
  
  private hashQuery(query: ContextQuery): string {
    const crypto = require('crypto');
    const data = JSON.stringify({
      query: query.query,
      memoryTypes: query.memoryTypes,
      includeCode: query.includeCode,
      includeMemories: query.includeMemories,
      includeGraph: query.includeGraph
    });
    return crypto.createHash('sha256').update(data).digest('hex').substring(0, 16);
  }
  
  private async setupRealtimeUpdates(): Promise<void> {
    // Subscribe to memory updates
    await this.redisManager!.subscribe(PubSubChannel.MemoryUpdates);
    await this.redisManager!.subscribe(PubSubChannel.IndexingComplete);
    
    this.logger.info('Real-time updates enabled');
  }
  
  private async setupRedisHandlers(): Promise<void> {
    if (!this.redisManager) return;
    
    this.redisManager.on('message', (message) => {
      switch (message.channel) {
        case PubSubChannel.MemoryUpdates:
          this.handleMemoryUpdate(message);
          break;
        case PubSubChannel.IndexingComplete:
          this.handleIndexingComplete(message);
          break;
      }
    });
  }
  
  private handleMemoryUpdate(message: any): void {
    this.emit('memory-update', message.data);
  }
  
  private handleIndexingComplete(message: any): void {
    this.emit('indexing-complete', message.data);
  }
  
  /**
   * Generate documentation for code
   */
  async generateDocumentation(
    scope: 'file' | 'folder' | 'workspace',
    target?: string
  ): Promise<{
    content: string;
    type: 'api' | 'usage' | 'architecture';
    metadata: any;
  }> {
    try {
      let codeContext: CodeContext | undefined;
      let contextQuery: string = '';
      
      // Build context based on scope
      switch (scope) {
        case 'file':
          if (!target) throw new Error('Target file required');
          contextQuery = `Generate documentation for ${target}`;
          codeContext = await this.searchCode({ 
            query: target, 
            includeCode: true 
          });
          break;
          
        case 'folder':
          if (!target) throw new Error('Target folder required');
          contextQuery = `Generate documentation for folder ${target}`;
          codeContext = await this.searchCode({ 
            query: `folder:${target}`,
            includeCode: true 
          });
          break;
          
        case 'workspace':
          contextQuery = 'Generate project documentation overview';
          codeContext = await this.searchCode({ 
            query: 'main index exports',
            includeCode: true,
            limit: 50
          });
          break;
      }
      
      if (!codeContext || codeContext.primary.length === 0) {
        throw new Error('No code found for documentation generation');
      }
      
      // Use orchestration engine to generate documentation
      const orchestrationEngine = (global as any).orchestrationEngine;
      if (!orchestrationEngine) {
        throw new Error('Orchestration engine not available');
      }
      
      const prompt = this.buildDocumentationPrompt(contextQuery, codeContext);
      const response = await orchestrationEngine.generateResponse({
        prompt,
        context: codeContext,
        maxTokens: 2000
      });
      
      // Store as memory for future reference
      const memory: Memory = {
        id: `doc_${Date.now()}`,
        type: MemoryType.Semantic,
        scope: MemoryScope.Project,
        content: response.content,
        metadata: {
          agentId: 'system',
          projectId: this.config.defaultProjectId,
          confidence: 0.9,
          importance: 0.8,
          shareable: true,
          documentationType: 'api',
          scope,
          target,
          generatedAt: new Date().toISOString()
        },
        createdAt: new Date(),
        updatedAt: new Date(),
        accessedAt: new Date(),
        accessCount: 0
      };
      
      await this.memoryManager.store(memory);
      
      return {
        content: response.content,
        type: 'api',
        metadata: {
          scope,
          target,
          tokensUsed: response.tokensUsed,
          model: response.model
        }
      };
      
    } catch (error) {
      this.logger.error('Failed to generate documentation', error);
      throw error;
    }
  }
  
  /**
   * Generate migration guide between versions
   */
  async generateMigrationGuide(
    fromVersion: string,
    toVersion: string
  ): Promise<{
    content: string;
    steps: string[];
    breakingChanges: string[];
    recommendations: string[];
  }> {
    try {
      // Search for past migration experiences
      const pastMigrations = await this.memoryManager.search({
        query: `migration ${fromVersion} ${toVersion}`,
        type: [MemoryType.Episodic, MemoryType.Semantic],
        projectId: this.config.defaultProjectId,
        limit: 10
      });
      
      // Find similar code patterns between versions
      const codePatterns = await this.contextBuilder.findSimilarPatterns(
        `${fromVersion} ${toVersion} migration patterns`
      );
      
      // Build migration context
      const migrationContext = {
        fromVersion,
        toVersion,
        pastExperiences: pastMigrations.map(m => m.memory.content),
        codePatterns: codePatterns.map(p => ({
          name: p.name,
          description: p.description,
          examples: p.examples.length
        }))
      };
      
      // Generate guide using orchestration
      const orchestrationEngine = (global as any).orchestrationEngine;
      const prompt = this.buildMigrationPrompt(migrationContext);
      
      const response = await orchestrationEngine.generateResponse({
        prompt,
        context: migrationContext,
        maxTokens: 3000
      });
      
      // Parse response into structured format
      const guide = this.parseMigrationGuide(response.content);
      
      // Store as team knowledge
      const memory: Memory = {
        id: `migration_${fromVersion}_${toVersion}_${Date.now()}`,
        type: MemoryType.Semantic,
        scope: MemoryScope.Team,
        content: response.content,
        metadata: {
          agentId: 'system',
          projectId: this.config.defaultProjectId,
          confidence: 0.85,
          importance: 0.9,
          shareable: true,
          fromVersion,
          toVersion,
          tags: ['migration', 'guide', fromVersion, toVersion]
        },
        createdAt: new Date(),
        updatedAt: new Date(),
        accessedAt: new Date(),
        accessCount: 0
      };
      
      await this.memoryManager.store(memory);
      await this.memoryManager.shareWithTeam(memory.id, 'default');
      
      return guide;
      
    } catch (error) {
      this.logger.error('Failed to generate migration guide', error);
      throw error;
    }
  }
  
  /**
   * Export knowledge to various formats
   */
  async exportKnowledge(
    format: 'markdown' | 'html' | 'json',
    options: {
      includeCode?: boolean;
      includeMemories?: boolean;
      includeGraph?: boolean;
      scope?: 'project' | 'team' | 'all';
    } = {}
  ): Promise<string> {
    try {
      const exportData: any = {
        metadata: {
          exportDate: new Date().toISOString(),
          project: this.config.defaultProjectId,
          format
        }
      };
      
      // Gather code context
      if (options.includeCode !== false) {
        const codeContext = await this.searchCode({
          query: 'main exports functions classes',
          includeCode: true,
          limit: 100
        });
        exportData.code = codeContext;
      }
      
      // Gather memories
      if (options.includeMemories !== false) {
        const memories = await this.memoryManager.search({
          scope: options.scope === 'team' ? [MemoryScope.Team] : 
                options.scope === 'all' ? undefined : [MemoryScope.Project],
          projectId: this.config.defaultProjectId,
          limit: 1000
        });
        exportData.memories = memories;
      }
      
      // Gather graph insights
      if (options.includeGraph !== false && this.neo4jManager) {
        // This would query Neo4j for project structure
        exportData.graphStructure = {
          nodes: 0, // Placeholder
          relationships: 0,
          patterns: []
        };
      }
      
      // Format based on requested type
      switch (format) {
        case 'markdown':
          return this.formatAsMarkdown(exportData);
          
        case 'html':
          return this.formatAsHTML(exportData);
          
        case 'json':
          return JSON.stringify(exportData, null, 2);
          
        default:
          throw new Error(`Unsupported format: ${format}`);
      }
      
    } catch (error) {
      this.logger.error('Failed to export knowledge', error);
      throw error;
    }
  }
  
  // Private helper methods for documentation
  
  private buildDocumentationPrompt(query: string, context: CodeContext): string {
    const parts = [
      `Task: ${query}`,
      '',
      'Based on the following code context, generate comprehensive documentation.',
      '',
      'Code Context Summary:',
      context.summary,
      '',
      'Primary code entities:',
      ...context.primary.slice(0, 10).map(r => 
        `- ${r.entity.type} ${r.entity.name} in ${r.entity.filePath}`
      ),
      '',
      'Generate documentation that includes:',
      '1. Overview and purpose',
      '2. API reference for public functions/classes',
      '3. Usage examples',
      '4. Important notes or warnings',
      '',
      'Format the documentation in Markdown.'
    ];
    
    return parts.join('\n');
  }
  
  private buildMigrationPrompt(context: any): string {
    const parts = [
      `Generate a migration guide from ${context.fromVersion} to ${context.toVersion}`,
      '',
      'Past migration experiences:',
      ...context.pastExperiences.slice(0, 5).map((exp: string) => `- ${exp}`),
      '',
      'Identified patterns:',
      ...context.codePatterns.map((p: any) => `- ${p.name}: ${p.description}`),
      '',
      'Generate a comprehensive migration guide that includes:',
      '1. Overview of major changes',
      '2. Step-by-step migration instructions',
      '3. Breaking changes and how to handle them',
      '4. Recommendations and best practices',
      '5. Common pitfalls to avoid',
      '',
      'Format the response as JSON with keys: content, steps, breakingChanges, recommendations'
    ];
    
    return parts.join('\n');
  }
  
  private parseMigrationGuide(content: string): any {
    try {
      // Try to parse as JSON first
      return JSON.parse(content);
    } catch {
      // Fallback to text parsing
      return {
        content,
        steps: this.extractListItems(content, 'steps'),
        breakingChanges: this.extractListItems(content, 'breaking'),
        recommendations: this.extractListItems(content, 'recommend')
      };
    }
  }
  
  private extractListItems(content: string, section: string): string[] {
    const lines = content.split('\n');
    const items: string[] = [];
    let inSection = false;
    
    for (const line of lines) {
      if (line.toLowerCase().includes(section)) {
        inSection = true;
        continue;
      }
      
      if (inSection && line.match(/^[-*]\s+/)) {
        items.push(line.replace(/^[-*]\s+/, '').trim());
      } else if (inSection && line.match(/^\d+\.\s+/)) {
        items.push(line.replace(/^\d+\.\s+/, '').trim());
      } else if (inSection && line.trim() === '') {
        inSection = false;
      }
    }
    
    return items;
  }
  
  private formatAsMarkdown(data: any): string {
    const sections: string[] = [
      `# Knowledge Export`,
      '',
      `**Project:** ${data.metadata.project}`,
      `**Export Date:** ${data.metadata.exportDate}`,
      ''
    ];
    
    if (data.code) {
      sections.push('## Code Structure');
      sections.push('');
      sections.push(data.code.summary);
      sections.push('');
      
      sections.push('### Primary Components');
      for (const item of data.code.primary.slice(0, 20)) {
        sections.push(`- **${item.entity.type}** \`${item.entity.name}\` - ${item.entity.filePath}`);
      }
      sections.push('');
    }
    
    if (data.memories) {
      sections.push('## Knowledge & Memories');
      sections.push('');
      
      const byType = new Map<string, any[]>();
      for (const result of data.memories) {
        const type = result.memory.type;
        if (!byType.has(type)) byType.set(type, []);
        byType.get(type)!.push(result);
      }
      
      for (const [type, memories] of byType) {
        sections.push(`### ${type} Memories (${memories.length})`);
        for (const mem of memories.slice(0, 10)) {
          sections.push(`- ${mem.memory.content.substring(0, 100)}...`);
        }
        sections.push('');
      }
    }
    
    return sections.join('\n');
  }
  
  private formatAsHTML(data: any): string {
    const markdown = this.formatAsMarkdown(data);
    // Simple markdown to HTML conversion
    return `
<!DOCTYPE html>
<html>
<head>
  <title>Knowledge Export</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 40px; }
    h1, h2, h3 { color: #333; }
    code { background: #f4f4f4; padding: 2px 4px; }
    pre { background: #f4f4f4; padding: 16px; overflow-x: auto; }
  </style>
</head>
<body>
  ${markdown
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^### (.+)$/gm, '<h3>$1</h3>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/`(.+?)`/g, '<code>$1</code>')
    .replace(/^- (.+)$/gm, '<li>$1</li>')
    .replace(/\n\n/g, '</p><p>')
    .replace(/^<li>/gm, '<ul><li>')
    .replace(/<\/li>\n(?!<li>)/g, '</li></ul>')
  }
</body>
</html>`;
  }
}