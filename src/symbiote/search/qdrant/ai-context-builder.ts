/**
 * AI Context Builder
 * 
 * Builds comprehensive context for AI agents by combining Qdrant semantic search
 * with Neo4j structural analysis and Mem0 persistent memory
 */

import { Logger } from '../../utils/logger';
import { QdrantManager } from './qdrant-manager';
import { Neo4jService } from '../../neo4j/neo4j-service';
import {
  AIAgentContext,
  CodeContext,
  FileContext,
  CodePattern,
  SemanticSearchResult,
  SemanticSearchRequest,
  CodeEntityType
} from './types';

interface ContextBuilderConfig {
  // Search parameters
  maxSemanticResults?: number;
  maxGraphDepth?: number;
  similarityThreshold?: number;
  
  // Context enhancement
  includeImports?: boolean;
  includeExports?: boolean;
  includeDependencies?: boolean;
  includeTests?: boolean;
  
  // Relevance scoring
  recentFilesWeight?: number;
  graphDistanceWeight?: number;
  semanticScoreWeight?: number;
}

interface RankedResult {
  result: SemanticSearchResult;
  relevanceScore: number;
  source: 'semantic' | 'graph' | 'combined';
}

export class AIContextBuilder implements AIAgentContext {
  private qdrantManager: QdrantManager;
  private neo4jService?: Neo4jService;
  private logger = new Logger('AIContextBuilder');
  private config: ContextBuilderConfig;
  
  // Context state
  taskDescription: string = '';
  currentFile?: string;
  recentFiles: string[] = [];
  private patterns = new Map<string, CodePattern>();
  
  constructor(
    qdrantManager: QdrantManager,
    neo4jService?: Neo4jService,
    config: ContextBuilderConfig = {}
  ) {
    this.qdrantManager = qdrantManager;
    this.neo4jService = neo4jService;
    
    this.config = {
      maxSemanticResults: 20,
      maxGraphDepth: 3,
      similarityThreshold: 0.7,
      includeImports: true,
      includeExports: true,
      includeDependencies: true,
      includeTests: false,
      recentFilesWeight: 0.2,
      graphDistanceWeight: 0.3,
      semanticScoreWeight: 0.5,
      ...config
    };
  }
  
  /**
   * Find relevant code for a query
   */
  async findRelevantCode(query: string): Promise<CodeContext> {
    this.logger.debug('Finding relevant code', { query });
    
    try {
      // Perform semantic search
      const semanticResults = await this.performSemanticSearch(query);
      
      // Enhance with graph data if available
      const enhancedResults = this.neo4jService
        ? await this.enhanceWithGraphData(semanticResults)
        : semanticResults;
      
      // Rank and filter results
      const rankedResults = await this.rankResults(enhancedResults, query);
      
      // Separate primary and related results
      const primary = rankedResults
        .slice(0, 10)
        .map(r => r.result);
      
      const related = rankedResults
        .slice(10, 20)
        .map(r => r.result);
      
      // Find patterns
      const patterns = await this.findPatterns(primary);
      
      // Generate summary
      const summary = this.generateContextSummary(primary, query);
      
      return {
        primary,
        related,
        patterns,
        summary
      };
      
    } catch (error) {
      this.logger.error('Failed to find relevant code', error);
      throw error;
    }
  }
  
  /**
   * Find similar code patterns
   */
  async findSimilarPatterns(codeSnippet: string): Promise<CodePattern[]> {
    this.logger.debug('Finding similar patterns');
    
    try {
      // Create temporary embedding for the snippet
      const embedding = await this.qdrantManager.embedCode(codeSnippet, {
        id: 'temp',
        type: CodeEntityType.Function,
        name: 'snippet',
        filePath: 'temp',
        startLine: 1,
        endLine: 1,
        language: 'javascript',
        lastModified: new Date()
      });
      
      // Search for similar code
      const results = await this.qdrantManager.search({
        query: codeSnippet,
        queryType: 'code',
        limit: 20,
        scoreThreshold: this.config.similarityThreshold
      });
      
      // Group by pattern
      const patterns = this.groupIntoPatterns(results);
      
      return Array.from(patterns.values());
      
    } catch (error) {
      this.logger.error('Failed to find similar patterns', error);
      throw error;
    }
  }
  
  /**
   * Get context for a specific file
   */
  async getContextForFile(filePath: string): Promise<FileContext> {
    this.logger.debug('Getting context for file', { filePath });
    
    try {
      // Get file metadata
      const fileResults = await this.qdrantManager.search({
        query: filePath,
        queryType: 'code',
        filters: {
          filePath: filePath
        },
        limit: 1
      });
      
      if (fileResults.length === 0) {
        throw new Error(`File not found in index: ${filePath}`);
      }
      
      const fileEntity = fileResults[0].entity;
      
      // Get imports
      const imports = await this.findImports(filePath);
      
      // Get exports
      const exports = await this.findExports(filePath);
      
      // Get dependencies from Neo4j
      const dependencies = this.neo4jService
        ? await this.findDependencies(filePath)
        : [];
      
      // Find similar files
      const similar = await this.findSimilarFiles(filePath);
      
      return {
        file: fileEntity,
        imports,
        exports,
        dependencies,
        similar
      };
      
    } catch (error) {
      this.logger.error('Failed to get file context', error);
      throw error;
    }
  }
  
  /**
   * Remember a code pattern
   */
  async rememberPattern(pattern: CodePattern): Promise<void> {
    this.patterns.set(pattern.id, pattern);
    
    // TODO: Persist to Mem0
    this.logger.debug('Remembered pattern', { id: pattern.id });
  }
  
  /**
   * Forget outdated patterns
   */
  async forgetOutdated(): Promise<void> {
    const threshold = Date.now() - 30 * 24 * 60 * 60 * 1000; // 30 days
    
    for (const [id, pattern] of this.patterns) {
      if (pattern.usage === 0 || pattern.confidence < 0.5) {
        this.patterns.delete(id);
      }
    }
    
    this.logger.debug('Forgot outdated patterns', {
      remaining: this.patterns.size
    });
  }
  
  /**
   * Update context state
   */
  updateContext(updates: Partial<AIAgentContext>): void {
    if (updates.taskDescription !== undefined) {
      this.taskDescription = updates.taskDescription;
    }
    
    if (updates.currentFile !== undefined) {
      this.currentFile = updates.currentFile;
      
      // Add to recent files
      if (updates.currentFile) {
        this.recentFiles = [
          updates.currentFile,
          ...this.recentFiles.filter(f => f !== updates.currentFile)
        ].slice(0, 10);
      }
    }
    
    if (updates.recentFiles !== undefined) {
      this.recentFiles = updates.recentFiles;
    }
  }
  
  /**
   * Perform semantic search
   */
  private async performSemanticSearch(query: string): Promise<SemanticSearchResult[]> {
    const request: SemanticSearchRequest = {
      query,
      queryType: 'hybrid',
      limit: this.config.maxSemanticResults,
      filters: {
        // Exclude test files if configured
        type: this.config.includeTests
          ? undefined
          : Object.values(CodeEntityType).filter(t => t !== CodeEntityType.Test)
      },
      includeContext: true,
      contextLines: 5
    };
    
    return await this.qdrantManager.search(request);
  }
  
  /**
   * Enhance results with graph data
   */
  private async enhanceWithGraphData(
    semanticResults: SemanticSearchResult[]
  ): Promise<SemanticSearchResult[]> {
    if (!this.neo4jService) return semanticResults;
    
    const enhanced = [];
    
    for (const result of semanticResults) {
      const entity = result.entity;
      
      if (entity.neo4jNodeId) {
        // Get graph relationships
        const graphData = await this.neo4jService.getNodeRelationships(
          entity.neo4jNodeId,
          this.config.maxGraphDepth!
        );
        
        // Add related entities
        if (graphData.imports) {
          result.related = result.related || {};
          result.related.imports = await this.convertGraphNodesToEntities(graphData.imports);
        }
        
        if (graphData.exports) {
          result.related = result.related || {};
          result.related.exports = await this.convertGraphNodesToEntities(graphData.exports);
        }
        
        if (graphData.calls) {
          result.related = result.related || {};
          result.related.calls = await this.convertGraphNodesToEntities(graphData.calls);
        }
      }
      
      enhanced.push(result);
    }
    
    return enhanced;
  }
  
  /**
   * Rank results by relevance
   */
  private async rankResults(
    results: SemanticSearchResult[],
    query: string
  ): Promise<RankedResult[]> {
    const ranked: RankedResult[] = [];
    
    for (const result of results) {
      let relevanceScore = 0;
      
      // Semantic score component
      relevanceScore += result.score * this.config.semanticScoreWeight!;
      
      // Recent files boost
      if (this.recentFiles.includes(result.entity.filePath)) {
        const recencyIndex = this.recentFiles.indexOf(result.entity.filePath);
        const recencyScore = 1 - (recencyIndex / this.recentFiles.length);
        relevanceScore += recencyScore * this.config.recentFilesWeight!;
      }
      
      // Current file boost
      if (result.entity.filePath === this.currentFile) {
        relevanceScore += 0.5;
      }
      
      // Graph distance penalty (if available)
      if (this.neo4jService && result.entity.neo4jNodeId && this.currentFile) {
        const distance = await this.calculateGraphDistance(
          result.entity.filePath,
          this.currentFile
        );
        
        if (distance !== -1) {
          const distanceScore = 1 / (1 + distance);
          relevanceScore += distanceScore * this.config.graphDistanceWeight!;
        }
      }
      
      // Task relevance
      if (this.taskDescription) {
        const taskRelevance = await this.calculateTaskRelevance(result, this.taskDescription);
        relevanceScore += taskRelevance * 0.2;
      }
      
      ranked.push({
        result,
        relevanceScore,
        source: 'combined'
      });
    }
    
    // Sort by relevance
    return ranked.sort((a, b) => b.relevanceScore - a.relevanceScore);
  }
  
  /**
   * Find patterns in results
   */
  private async findPatterns(results: SemanticSearchResult[]): Promise<CodePattern[]> {
    const patterns: CodePattern[] = [];
    
    // Group by entity type
    const typeGroups = new Map<CodeEntityType, SemanticSearchResult[]>();
    
    for (const result of results) {
      const type = result.entity.type;
      if (!typeGroups.has(type)) {
        typeGroups.set(type, []);
      }
      typeGroups.get(type)!.push(result);
    }
    
    // Create patterns for common types
    for (const [type, group] of typeGroups) {
      if (group.length >= 3) {
        patterns.push({
          id: `pattern_${type}_${Date.now()}`,
          name: `${type} pattern`,
          description: `Common ${type} implementations`,
          examples: group.slice(0, 5),
          usage: group.length,
          confidence: 0.8
        });
      }
    }
    
    // Look for naming patterns
    const namingPatterns = this.findNamingPatterns(results);
    patterns.push(...namingPatterns);
    
    return patterns;
  }
  
  /**
   * Generate context summary
   */
  private generateContextSummary(
    results: SemanticSearchResult[],
    query: string
  ): string {
    const summary: string[] = [];
    
    // Count by type
    const typeCounts = new Map<CodeEntityType, number>();
    for (const result of results) {
      const type = result.entity.type;
      typeCounts.set(type, (typeCounts.get(type) || 0) + 1);
    }
    
    summary.push(`Found ${results.length} relevant code entities for "${query}":`);
    
    for (const [type, count] of typeCounts) {
      summary.push(`- ${count} ${type}(s)`);
    }
    
    // Top files
    const fileCounts = new Map<string, number>();
    for (const result of results) {
      const file = result.entity.filePath;
      fileCounts.set(file, (fileCounts.get(file) || 0) + 1);
    }
    
    const topFiles = Array.from(fileCounts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3);
    
    if (topFiles.length > 0) {
      summary.push('\nMost relevant files:');
      for (const [file, count] of topFiles) {
        const fileName = file.split('/').pop();
        summary.push(`- ${fileName} (${count} matches)`);
      }
    }
    
    return summary.join('\n');
  }
  
  /**
   * Group results into patterns
   */
  private groupIntoPatterns(results: SemanticSearchResult[]): Map<string, CodePattern> {
    const patterns = new Map<string, CodePattern>();
    
    // Simple clustering by similarity
    for (const result of results) {
      let added = false;
      
      for (const [id, pattern] of patterns) {
        if (this.isSimilarToPattern(result, pattern)) {
          pattern.examples.push(result);
          pattern.usage++;
          added = true;
          break;
        }
      }
      
      if (!added) {
        const patternId = `pattern_${patterns.size}`;
        patterns.set(patternId, {
          id: patternId,
          name: `Pattern ${patterns.size + 1}`,
          description: `Similar to ${result.entity.name}`,
          examples: [result],
          usage: 1,
          confidence: result.score
        });
      }
    }
    
    return patterns;
  }
  
  /**
   * Check if result is similar to pattern
   */
  private isSimilarToPattern(result: SemanticSearchResult, pattern: CodePattern): boolean {
    // Check type similarity
    const sameType = pattern.examples.every(
      ex => ex.entity.type === result.entity.type
    );
    
    if (!sameType) return false;
    
    // Check score similarity
    const avgScore = pattern.examples.reduce((sum, ex) => sum + ex.score, 0) / pattern.examples.length;
    return Math.abs(result.score - avgScore) < 0.1;
  }
  
  /**
   * Find imports for a file
   */
  private async findImports(filePath: string): Promise<SemanticSearchResult[]> {
    // Query for import statements in the file
    const results = await this.qdrantManager.search({
      query: `imports in ${filePath}`,
      queryType: 'hybrid',
      filters: {
        filePath,
        type: [CodeEntityType.Module]
      },
      limit: 20
    });
    
    return results;
  }
  
  /**
   * Find exports for a file
   */
  private async findExports(filePath: string): Promise<SemanticSearchResult[]> {
    // Query for exported entities
    const results = await this.qdrantManager.search({
      query: `exports from ${filePath}`,
      queryType: 'hybrid',
      filters: {
        filePath
      },
      limit: 20
    });
    
    return results.filter(r => 
      r.entity.exports && r.entity.exports.length > 0
    );
  }
  
  /**
   * Find dependencies using Neo4j
   */
  private async findDependencies(filePath: string): Promise<SemanticSearchResult[]> {
    if (!this.neo4jService) return [];
    
    // Get file dependencies from graph
    const dependencies = await this.neo4jService.getFileDependencies(filePath);
    
    // Convert to semantic search results
    const results: SemanticSearchResult[] = [];
    
    for (const dep of dependencies) {
      // Find in Qdrant by Neo4j ID
      const qdrantResults = await this.qdrantManager.search({
        query: dep.path,
        queryType: 'code',
        filters: {
          neo4jNodeId: [dep.id]
        },
        limit: 1
      });
      
      if (qdrantResults.length > 0) {
        results.push(qdrantResults[0]);
      }
    }
    
    return results;
  }
  
  /**
   * Find similar files
   */
  private async findSimilarFiles(filePath: string): Promise<SemanticSearchResult[]> {
    // Get file embedding
    const fileResults = await this.qdrantManager.search({
      query: filePath,
      queryType: 'code',
      filters: {
        filePath
      },
      limit: 1
    });
    
    if (fileResults.length === 0) return [];
    
    // Find similar
    return await this.qdrantManager.searchSimilar(
      fileResults[0].entity.id,
      10
    );
  }
  
  /**
   * Convert graph nodes to embedding metadata
   */
  private async convertGraphNodesToEntities(nodes: any[]): Promise<any[]> {
    // This would convert Neo4j nodes to CodeEmbeddingMetadata
    // Placeholder implementation
    return nodes;
  }
  
  /**
   * Calculate graph distance between files
   */
  private async calculateGraphDistance(file1: string, file2: string): Promise<number> {
    if (!this.neo4jService) return -1;
    
    try {
      const distance = await this.neo4jService.calculateDistance(file1, file2);
      return distance;
    } catch {
      return -1;
    }
  }
  
  /**
   * Calculate task relevance
   */
  private async calculateTaskRelevance(
    result: SemanticSearchResult,
    task: string
  ): Promise<number> {
    // Simple keyword matching for now
    const taskKeywords = task.toLowerCase().split(/\s+/);
    const entityText = [
      result.entity.name,
      result.entity.description,
      result.entity.docstring
    ].filter(Boolean).join(' ').toLowerCase();
    
    let matches = 0;
    for (const keyword of taskKeywords) {
      if (entityText.includes(keyword)) {
        matches++;
      }
    }
    
    return matches / taskKeywords.length;
  }
  
  /**
   * Find naming patterns
   */
  private findNamingPatterns(results: SemanticSearchResult[]): CodePattern[] {
    const patterns: CodePattern[] = [];
    
    // Look for common prefixes/suffixes
    const names = results.map(r => r.entity.name);
    
    // Find common prefixes
    const prefixes = new Map<string, SemanticSearchResult[]>();
    
    for (const result of results) {
      const name = result.entity.name;
      const prefix = name.substring(0, 3);
      
      if (prefix.length === 3) {
        if (!prefixes.has(prefix)) {
          prefixes.set(prefix, []);
        }
        prefixes.get(prefix)!.push(result);
      }
    }
    
    // Create patterns for common prefixes
    for (const [prefix, examples] of prefixes) {
      if (examples.length >= 3) {
        patterns.push({
          id: `naming_${prefix}`,
          name: `${prefix}* naming pattern`,
          description: `Entities starting with "${prefix}"`,
          examples: examples.slice(0, 5),
          usage: examples.length,
          confidence: 0.7
        });
      }
    }
    
    return patterns;
  }
}