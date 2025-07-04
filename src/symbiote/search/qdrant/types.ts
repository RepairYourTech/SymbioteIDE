/**
 * Qdrant Semantic Search Types
 * 
 * Type definitions for the semantic search system using Qdrant vector database
 */

import { QdrantClient } from '@qdrant/js-client';

/**
 * Code entity types that can be embedded
 */
export enum CodeEntityType {
  Function = 'function',
  Class = 'class',
  Method = 'method',
  Module = 'module',
  Interface = 'interface',
  Type = 'type',
  Variable = 'variable',
  Comment = 'comment',
  Documentation = 'documentation',
  Error = 'error',
  Test = 'test'
}

/**
 * Embedding model providers
 */
export enum EmbeddingProvider {
  OpenAI = 'openai',
  Anthropic = 'anthropic',
  Google = 'google',
  Local = 'local',
  Custom = 'custom'
}

/**
 * Code embedding metadata
 */
export interface CodeEmbeddingMetadata {
  // Entity identification
  id: string;
  type: CodeEntityType;
  name: string;
  
  // Location information
  filePath: string;
  startLine: number;
  endLine: number;
  startColumn?: number;
  endColumn?: number;
  
  // Code context
  language: string;
  framework?: string;
  parentId?: string;  // Parent entity (e.g., class for a method)
  signature?: string;  // Function/method signature
  
  // Semantic information
  description?: string;
  docstring?: string;
  comments?: string[];
  tags?: string[];
  
  // Relationships
  imports?: string[];
  exports?: string[];
  calls?: string[];
  calledBy?: string[];
  
  // Quality metrics
  complexity?: number;
  testCoverage?: number;
  lastModified: Date;
  
  // Neo4j integration
  neo4jNodeId?: string;
  
  // Additional metadata
  metadata?: Record<string, any>;
}

/**
 * Embedding configuration
 */
export interface EmbeddingConfig {
  provider: EmbeddingProvider;
  model: string;
  dimensions: number;
  batchSize: number;
  maxTokens: number;
  
  // Provider-specific options
  options?: {
    temperature?: number;
    topP?: number;
    apiKey?: string;
    endpoint?: string;
  };
}

/**
 * Search request interface
 */
export interface SemanticSearchRequest {
  // Query
  query: string;
  queryType: 'natural_language' | 'code' | 'hybrid';
  
  // Filters
  filters?: {
    language?: string[];
    type?: CodeEntityType[];
    filePath?: string | RegExp;
    framework?: string[];
    tags?: string[];
    dateRange?: {
      from?: Date;
      to?: Date;
    };
  };
  
  // Search parameters
  limit?: number;
  offset?: number;
  scoreThreshold?: number;
  
  // Context enhancement
  includeContext?: boolean;
  contextLines?: number;
  includeRelated?: boolean;
  
  // Neo4j integration
  combineWithGraph?: boolean;
  graphDepth?: number;
}

/**
 * Search result interface
 */
export interface SemanticSearchResult {
  // Entity information
  entity: CodeEmbeddingMetadata;
  
  // Search relevance
  score: number;
  distance: number;
  
  // Code content
  code: string;
  highlightedCode?: string;
  
  // Context
  context?: {
    before: string;
    after: string;
  };
  
  // Related entities
  related?: {
    imports: CodeEmbeddingMetadata[];
    exports: CodeEmbeddingMetadata[];
    calls: CodeEmbeddingMetadata[];
    similar: CodeEmbeddingMetadata[];
  };
  
  // Explanation
  explanation?: string;
}

/**
 * Collection configuration
 */
export interface CollectionConfig {
  name: string;
  vectorSize: number;
  distance: 'Cosine' | 'Euclid' | 'Dot';
  
  // Optimization settings
  optimizers?: {
    memmap_threshold?: number;
    indexing_threshold?: number;
  };
  
  // Replication settings
  replication?: {
    factor: number;
    write_consistency_factor?: number;
  };
}

/**
 * Batch operation interfaces
 */
export interface BatchEmbeddingJob {
  id: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  totalEntities: number;
  processedEntities: number;
  failedEntities: number;
  startTime: Date;
  endTime?: Date;
  errors?: Array<{
    entityId: string;
    error: string;
  }>;
}

/**
 * Index statistics
 */
export interface IndexStatistics {
  totalVectors: number;
  totalCollections: number;
  collections: Array<{
    name: string;
    vectorCount: number;
    indexedSize: number;
    segmentsCount: number;
  }>;
  lastUpdated: Date;
  indexHealth: 'healthy' | 'degraded' | 'unhealthy';
}

/**
 * Embedding cache entry
 */
export interface EmbeddingCacheEntry {
  key: string;
  embedding: number[];
  metadata: CodeEmbeddingMetadata;
  timestamp: Date;
  ttl?: number;
}

/**
 * Search analytics
 */
export interface SearchAnalytics {
  query: string;
  timestamp: Date;
  resultCount: number;
  topScore: number;
  latency: number;
  filters: Record<string, any>;
  userId?: string;
  sessionId?: string;
}

/**
 * Qdrant manager interface
 */
export interface IQdrantManager {
  // Connection management
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  isConnected(): boolean;
  
  // Collection management
  createCollection(config: CollectionConfig): Promise<void>;
  deleteCollection(name: string): Promise<void>;
  listCollections(): Promise<string[]>;
  
  // Embedding operations
  embedCode(code: string, metadata: CodeEmbeddingMetadata): Promise<number[]>;
  upsertEmbedding(embedding: number[], metadata: CodeEmbeddingMetadata): Promise<void>;
  batchUpsert(embeddings: Array<{ vector: number[]; metadata: CodeEmbeddingMetadata }>): Promise<void>;
  
  // Search operations
  search(request: SemanticSearchRequest): Promise<SemanticSearchResult[]>;
  searchSimilar(entityId: string, limit?: number): Promise<SemanticSearchResult[]>;
  
  // Maintenance
  optimize(collectionName: string): Promise<void>;
  getStatistics(): Promise<IndexStatistics>;
  clearCollection(name: string): Promise<void>;
}

/**
 * Embedding service interface
 */
export interface IEmbeddingService {
  // Generate embeddings
  embed(text: string): Promise<number[]>;
  batchEmbed(texts: string[]): Promise<number[][]>;
  
  // Code-specific embeddings
  embedCode(code: string, language: string): Promise<number[]>;
  embedWithContext(code: string, context: CodeEmbeddingMetadata): Promise<number[]>;
  
  // Model management
  getModelInfo(): { provider: string; model: string; dimensions: number };
  switchModel(config: EmbeddingConfig): Promise<void>;
}

/**
 * Code chunking strategy
 */
export interface ChunkingStrategy {
  type: 'fixed' | 'semantic' | 'ast' | 'sliding';
  maxChunkSize: number;
  overlapSize: number;
  preserveBoundaries: boolean;
  
  // Custom chunking function
  customChunker?: (code: string, language: string) => string[];
}

/**
 * Integration with Neo4j
 */
export interface GraphIntegration {
  // Link embeddings to graph nodes
  linkToGraphNode(embeddingId: string, neo4jNodeId: string): Promise<void>;
  
  // Combined search
  searchWithGraph(
    semanticResults: SemanticSearchResult[],
    graphDepth: number
  ): Promise<SemanticSearchResult[]>;
  
  // Sync operations
  syncWithGraph(): Promise<void>;
  updateFromGraphChanges(changes: any[]): Promise<void>;
}

/**
 * AI agent context interface
 */
export interface AIAgentContext {
  // Current task context
  taskDescription: string;
  currentFile?: string;
  recentFiles: string[];
  
  // Search and retrieve
  findRelevantCode(query: string): Promise<CodeContext>;
  findSimilarPatterns(codeSnippet: string): Promise<CodePattern[]>;
  getContextForFile(filePath: string): Promise<FileContext>;
  
  // Learning and memory
  rememberPattern(pattern: CodePattern): Promise<void>;
  forgetOutdated(): Promise<void>;
}

/**
 * Code context for AI agents
 */
export interface CodeContext {
  primary: SemanticSearchResult[];
  related: SemanticSearchResult[];
  patterns: CodePattern[];
  summary: string;
}

/**
 * Code pattern interface
 */
export interface CodePattern {
  id: string;
  name: string;
  description: string;
  examples: SemanticSearchResult[];
  usage: number;
  confidence: number;
}

/**
 * File context interface
 */
export interface FileContext {
  file: CodeEmbeddingMetadata;
  imports: SemanticSearchResult[];
  exports: SemanticSearchResult[];
  dependencies: SemanticSearchResult[];
  similar: SemanticSearchResult[];
}