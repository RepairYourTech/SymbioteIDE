/**
 * Qdrant Manager
 * 
 * Main interface for managing semantic search with Qdrant
 */

import { QdrantClient } from '@qdrant/js-client';
import { v4 as uuidv4 } from 'uuid';
import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import { QdrantConnectionManager } from './connection-manager';
import { EmbeddingService } from './embedding-service';
import {
  IQdrantManager,
  CodeEmbeddingMetadata,
  SemanticSearchRequest,
  SemanticSearchResult,
  CollectionConfig,
  IndexStatistics,
  CodeEntityType,
  EmbeddingConfig,
  EmbeddingProvider
} from './types';

const DEFAULT_COLLECTION = 'code_embeddings';
const DEFAULT_EMBEDDING_CONFIG: EmbeddingConfig = {
  provider: EmbeddingProvider.OpenAI,
  model: 'text-embedding-3-large',
  dimensions: 1536,
  batchSize: 100,
  maxTokens: 8192
};

// Alternative Google configuration
const GOOGLE_EMBEDDING_CONFIG: EmbeddingConfig = {
  provider: EmbeddingProvider.Google,
  model: 'text-embedding-004',
  dimensions: 768,  // Google's model supports 256, 512, or 768
  batchSize: 100,
  maxTokens: 8192
};

export class QdrantManager extends EventEmitter implements IQdrantManager {
  private connectionManager: QdrantConnectionManager;
  private embeddingService: EmbeddingService;
  private logger = new Logger('QdrantManager');
  private collections = new Map<string, CollectionConfig>();
  
  constructor(
    connectionConfig?: any,
    embeddingConfig: EmbeddingConfig = DEFAULT_EMBEDDING_CONFIG
  ) {
    super();
    
    this.connectionManager = new QdrantConnectionManager(connectionConfig);
    this.embeddingService = new EmbeddingService(embeddingConfig);
    
    // Forward connection events
    this.connectionManager.on('connected', () => this.emit('connected'));
    this.connectionManager.on('disconnected', () => this.emit('disconnected'));
    this.connectionManager.on('error', (error) => this.emit('error', error));
  }
  
  /**
   * Connect to Qdrant
   */
  async connect(): Promise<void> {
    await this.connectionManager.connect();
    
    // Ensure default collection exists
    await this.ensureDefaultCollection();
  }
  
  /**
   * Disconnect from Qdrant
   */
  async disconnect(): Promise<void> {
    await this.connectionManager.disconnect();
  }
  
  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.connectionManager.isConnected();
  }
  
  /**
   * Create a collection
   */
  async createCollection(config: CollectionConfig): Promise<void> {
    await this.connectionManager.ensureCollection(
      config.name,
      config.vectorSize,
      config.distance
    );
    
    this.collections.set(config.name, config);
  }
  
  /**
   * Delete a collection
   */
  async deleteCollection(name: string): Promise<void> {
    await this.connectionManager.deleteCollection(name);
    this.collections.delete(name);
  }
  
  /**
   * List all collections
   */
  async listCollections(): Promise<string[]> {
    return await this.connectionManager.listCollections();
  }
  
  /**
   * Embed code and generate metadata
   */
  async embedCode(code: string, metadata: CodeEmbeddingMetadata): Promise<number[]> {
    try {
      // Generate embedding with context
      const embedding = await this.embeddingService.embedWithContext(code, metadata);
      
      this.logger.debug('Generated embedding for code entity', {
        id: metadata.id,
        type: metadata.type,
        dimensions: embedding.length
      });
      
      return embedding;
    } catch (error) {
      this.logger.error('Failed to embed code', error);
      throw error;
    }
  }
  
  /**
   * Upsert an embedding
   */
  async upsertEmbedding(
    embedding: number[],
    metadata: CodeEmbeddingMetadata,
    collection: string = DEFAULT_COLLECTION
  ): Promise<void> {
    const client = this.connectionManager.getClient();
    
    try {
      // Ensure ID exists
      if (!metadata.id) {
        metadata.id = uuidv4();
      }
      
      // Prepare payload
      const payload = {
        ...metadata,
        lastModified: metadata.lastModified?.toISOString() || new Date().toISOString()
      };
      
      // Upsert to Qdrant
      await client.upsert(collection, {
        points: [{
          id: metadata.id,
          vector: embedding,
          payload
        }]
      });
      
      this.logger.debug('Upserted embedding', {
        id: metadata.id,
        collection
      });
      
    } catch (error) {
      this.logger.error('Failed to upsert embedding', error);
      throw error;
    }
  }
  
  /**
   * Batch upsert embeddings
   */
  async batchUpsert(
    embeddings: Array<{ vector: number[]; metadata: CodeEmbeddingMetadata }>,
    collection: string = DEFAULT_COLLECTION
  ): Promise<void> {
    const client = this.connectionManager.getClient();
    
    try {
      // Prepare points
      const points = embeddings.map(({ vector, metadata }) => ({
        id: metadata.id || uuidv4(),
        vector,
        payload: {
          ...metadata,
          lastModified: metadata.lastModified?.toISOString() || new Date().toISOString()
        }
      }));
      
      // Batch upsert
      const batchSize = 100;
      for (let i = 0; i < points.length; i += batchSize) {
        const batch = points.slice(i, i + batchSize);
        await client.upsert(collection, { points: batch });
        
        this.logger.debug(`Upserted batch ${i / batchSize + 1}/${Math.ceil(points.length / batchSize)}`);
      }
      
      this.logger.info(`Batch upserted ${points.length} embeddings`);
      
    } catch (error) {
      this.logger.error('Failed to batch upsert embeddings', error);
      throw error;
    }
  }
  
  /**
   * Search for similar code
   */
  async search(request: SemanticSearchRequest): Promise<SemanticSearchResult[]> {
    const client = this.connectionManager.getClient();
    
    try {
      // Generate query embedding
      const queryEmbedding = await this.embeddingService.embed(request.query);
      
      // Build filter conditions
      const filter = this.buildFilter(request.filters);
      
      // Perform search
      const searchResult = await client.search(DEFAULT_COLLECTION, {
        vector: queryEmbedding,
        limit: request.limit || 10,
        offset: request.offset || 0,
        filter,
        with_payload: true,
        with_vector: false
      });
      
      // Convert to semantic search results
      const results = await this.convertSearchResults(searchResult.points, request);
      
      this.logger.debug('Search completed', {
        query: request.query.substring(0, 50),
        resultCount: results.length
      });
      
      // Track search analytics
      this.trackSearch(request, results);
      
      return results;
      
    } catch (error) {
      this.logger.error('Search failed', error);
      throw error;
    }
  }
  
  /**
   * Search for similar entities
   */
  async searchSimilar(entityId: string, limit: number = 10): Promise<SemanticSearchResult[]> {
    const client = this.connectionManager.getClient();
    
    try {
      // Get the entity's vector
      const entity = await client.retrieve(DEFAULT_COLLECTION, {
        ids: [entityId],
        with_vector: true,
        with_payload: true
      });
      
      if (entity.points.length === 0) {
        throw new Error(`Entity not found: ${entityId}`);
      }
      
      const vector = entity.points[0].vector as number[];
      const metadata = entity.points[0].payload as CodeEmbeddingMetadata;
      
      // Search for similar
      const searchResult = await client.search(DEFAULT_COLLECTION, {
        vector,
        limit: limit + 1, // +1 to exclude self
        filter: {
          must_not: [{
            key: 'id',
            match: { value: entityId }
          }]
        },
        with_payload: true
      });
      
      // Convert results
      const request: SemanticSearchRequest = {
        query: `Similar to ${metadata.name || entityId}`,
        queryType: 'code',
        limit
      };
      
      return await this.convertSearchResults(searchResult.points, request);
      
    } catch (error) {
      this.logger.error('Similar search failed', error);
      throw error;
    }
  }
  
  /**
   * Optimize collection
   */
  async optimize(collectionName: string = DEFAULT_COLLECTION): Promise<void> {
    const client = this.connectionManager.getClient();
    
    try {
      // Trigger optimization
      await client.updateCollection(collectionName, {
        optimizer_config: {
          deleted_threshold: 0.2,
          vacuum_min_vector_number: 1000,
          default_segment_number: 2
        }
      });
      
      this.logger.info(`Optimized collection: ${collectionName}`);
      
    } catch (error) {
      this.logger.error('Optimization failed', error);
      throw error;
    }
  }
  
  /**
   * Get index statistics
   */
  async getStatistics(): Promise<IndexStatistics> {
    const client = this.connectionManager.getClient();
    
    try {
      const collections = await this.listCollections();
      const collectionStats = [];
      let totalVectors = 0;
      
      for (const name of collections) {
        const info = await client.getCollection(name);
        const stats = {
          name,
          vectorCount: info.points_count || 0,
          indexedSize: info.indexed_vectors_count || 0,
          segmentsCount: info.segments_count || 0
        };
        
        collectionStats.push(stats);
        totalVectors += stats.vectorCount;
      }
      
      return {
        totalVectors,
        totalCollections: collections.length,
        collections: collectionStats,
        lastUpdated: new Date(),
        indexHealth: totalVectors > 0 ? 'healthy' : 'degraded'
      };
      
    } catch (error) {
      this.logger.error('Failed to get statistics', error);
      throw error;
    }
  }
  
  /**
   * Clear a collection
   */
  async clearCollection(name: string = DEFAULT_COLLECTION): Promise<void> {
    try {
      // Delete and recreate
      await this.deleteCollection(name);
      await this.ensureDefaultCollection();
      
      this.logger.info(`Cleared collection: ${name}`);
      
    } catch (error) {
      this.logger.error('Failed to clear collection', error);
      throw error;
    }
  }
  
  /**
   * Delete by filter
   */
  async deleteByFilter(filter: any, collection: string = DEFAULT_COLLECTION): Promise<void> {
    const client = this.connectionManager.getClient();
    
    try {
      await client.delete(collection, {
        filter
      });
      
      this.logger.info('Deleted points by filter', { filter });
      
    } catch (error) {
      this.logger.error('Failed to delete by filter', error);
      throw error;
    }
  }
  
  /**
   * Update metadata
   */
  async updateMetadata(
    id: string,
    metadata: Partial<CodeEmbeddingMetadata>,
    collection: string = DEFAULT_COLLECTION
  ): Promise<void> {
    const client = this.connectionManager.getClient();
    
    try {
      await client.setPayload(collection, {
        points: [id],
        payload: {
          ...metadata,
          lastModified: new Date().toISOString()
        }
      });
      
      this.logger.debug('Updated metadata', { id });
      
    } catch (error) {
      this.logger.error('Failed to update metadata', error);
      throw error;
    }
  }
  
  /**
   * Ensure default collection exists
   */
  private async ensureDefaultCollection(): Promise<void> {
    const modelInfo = this.embeddingService.getModelInfo();
    
    await this.createCollection({
      name: DEFAULT_COLLECTION,
      vectorSize: modelInfo.dimensions,
      distance: 'Cosine'
    });
  }
  
  /**
   * Build Qdrant filter from request filters
   */
  private buildFilter(filters?: SemanticSearchRequest['filters']): any {
    if (!filters) return undefined;
    
    const conditions: any[] = [];
    
    // Language filter
    if (filters.language?.length) {
      conditions.push({
        key: 'language',
        match: {
          any: filters.language
        }
      });
    }
    
    // Type filter
    if (filters.type?.length) {
      conditions.push({
        key: 'type',
        match: {
          any: filters.type
        }
      });
    }
    
    // File path filter
    if (filters.filePath) {
      if (typeof filters.filePath === 'string') {
        conditions.push({
          key: 'filePath',
          match: {
            value: filters.filePath
          }
        });
      } else if (filters.filePath instanceof RegExp) {
        // Qdrant doesn't support regex directly, use prefix match
        conditions.push({
          key: 'filePath',
          match: {
            text: filters.filePath.source
          }
        });
      }
    }
    
    // Framework filter
    if (filters.framework?.length) {
      conditions.push({
        key: 'framework',
        match: {
          any: filters.framework
        }
      });
    }
    
    // Tags filter
    if (filters.tags?.length) {
      conditions.push({
        key: 'tags',
        match: {
          any: filters.tags
        }
      });
    }
    
    // Date range filter
    if (filters.dateRange) {
      const dateConditions: any[] = [];
      
      if (filters.dateRange.from) {
        dateConditions.push({
          key: 'lastModified',
          range: {
            gte: filters.dateRange.from.toISOString()
          }
        });
      }
      
      if (filters.dateRange.to) {
        dateConditions.push({
          key: 'lastModified',
          range: {
            lte: filters.dateRange.to.toISOString()
          }
        });
      }
      
      if (dateConditions.length > 0) {
        conditions.push(...dateConditions);
      }
    }
    
    return conditions.length > 0 ? { must: conditions } : undefined;
  }
  
  /**
   * Convert Qdrant results to semantic search results
   */
  private async convertSearchResults(
    points: any[],
    request: SemanticSearchRequest
  ): Promise<SemanticSearchResult[]> {
    const results: SemanticSearchResult[] = [];
    
    for (const point of points) {
      const metadata = point.payload as CodeEmbeddingMetadata;
      
      // Read code content (this would come from file system)
      const code = await this.readCodeContent(metadata);
      
      const result: SemanticSearchResult = {
        entity: metadata,
        score: point.score || 0,
        distance: 1 - (point.score || 0), // Convert similarity to distance
        code,
        explanation: this.generateExplanation(metadata, request)
      };
      
      // Add context if requested
      if (request.includeContext) {
        result.context = await this.getCodeContext(metadata, request.contextLines || 5);
      }
      
      // Add highlighted code
      if (request.queryType === 'natural_language') {
        result.highlightedCode = this.highlightCode(code, request.query);
      }
      
      results.push(result);
    }
    
    return results;
  }
  
  /**
   * Read code content from file
   */
  private async readCodeContent(metadata: CodeEmbeddingMetadata): Promise<string> {
    // This is a placeholder - in real implementation, this would read from file system
    // For now, return a mock based on metadata
    return `// ${metadata.type} ${metadata.name} in ${metadata.filePath}
// Lines ${metadata.startLine}-${metadata.endLine}
${metadata.signature || ''}
// ... code content ...`;
  }
  
  /**
   * Get code context
   */
  private async getCodeContext(
    metadata: CodeEmbeddingMetadata,
    contextLines: number
  ): Promise<{ before: string; after: string }> {
    // Placeholder - would read surrounding lines from file
    return {
      before: '// ... previous code ...',
      after: '// ... following code ...'
    };
  }
  
  /**
   * Highlight code based on query
   */
  private highlightCode(code: string, query: string): string {
    // Simple keyword highlighting
    const keywords = query.toLowerCase().split(/\s+/);
    let highlighted = code;
    
    for (const keyword of keywords) {
      if (keyword.length > 2) {
        const regex = new RegExp(`\\b${keyword}\\b`, 'gi');
        highlighted = highlighted.replace(regex, `**$&**`);
      }
    }
    
    return highlighted;
  }
  
  /**
   * Generate explanation for why result matches
   */
  private generateExplanation(
    metadata: CodeEmbeddingMetadata,
    request: SemanticSearchRequest
  ): string {
    const parts: string[] = [];
    
    // Type match
    if (request.filters?.type?.includes(metadata.type)) {
      parts.push(`Matches requested type: ${metadata.type}`);
    }
    
    // Language match
    if (request.filters?.language?.includes(metadata.language)) {
      parts.push(`Written in ${metadata.language}`);
    }
    
    // Description/docstring relevance
    if (metadata.description || metadata.docstring) {
      parts.push('Contains relevant documentation');
    }
    
    return parts.join('. ') || 'Semantic similarity match';
  }
  
  /**
   * Track search analytics
   */
  private trackSearch(request: SemanticSearchRequest, results: SemanticSearchResult[]): void {
    // Emit analytics event
    this.emit('search', {
      query: request.query,
      timestamp: new Date(),
      resultCount: results.length,
      topScore: results[0]?.score || 0,
      filters: request.filters
    });
  }
}