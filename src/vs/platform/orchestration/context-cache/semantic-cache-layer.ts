/**
 * Semantic Cache Layer
 * 
 * Embedding-based semantic caching for intelligent cache matching
 */

import { CacheEntry, SemanticCache, SemanticCacheConfig, CacheStorage } from './types';
import { OrchestrationEngine } from '../orchestration-engine';

interface EmbeddingIndex {
  key: string;
  embedding: number[];
  metadata: {
    provider: string;
    model: string;
    tokenCount: number;
  };
}

export class SemanticCacheLayer implements SemanticCache {
  private config: SemanticCacheConfig;
  private storage: CacheStorage;
  private embeddings: Map<string, EmbeddingIndex>;
  private embeddingModel: string;
  private dimensions: number;
  
  constructor(config: SemanticCacheConfig, storage: CacheStorage) {
    this.config = config;
    this.storage = storage;
    this.embeddings = new Map();
    this.embeddingModel = config.embeddingModel || 'text-embedding-ada-002';
    this.dimensions = config.maxDimensions || 1536;
    
    // Load existing embeddings from storage if available
    this.loadEmbeddings();
  }
  
  /**
   * Generate embedding for content
   */
  async generateEmbedding(content: string): Promise<number[]> {
    try {
      // Use orchestration engine to get embeddings
      const orchestration = new OrchestrationEngine({
        providers: {
          openai: {
            models: [{
              id: this.embeddingModel,
              name: 'OpenAI Embeddings',
              contextWindow: 8192,
              provider: 'openai',
              capabilities: ['embeddings'],
              costPer1kTokens: { input: 0.0001, output: 0 }
            }]
          }
        }
      });
      
      const response = await orchestration.executeTask({
        type: 'embeddings',
        prompt: content,
        model: this.embeddingModel,
        requirements: {
          provider: 'openai'
        }
      });
      
      if (response.embedding && Array.isArray(response.embedding)) {
        return response.embedding;
      }
      
      throw new Error('Invalid embedding response');
      
    } catch (error) {
      console.error('Failed to generate embedding:', error);
      // Return zero vector as fallback
      return new Array(this.dimensions).fill(0);
    }
  }
  
  /**
   * Find similar cached entries
   */
  async findSimilar(
    embedding: number[], 
    threshold: number,
    limit: number = 10
  ): Promise<Array<{ entry: CacheEntry; similarity: number }>> {
    const similarities: Array<{
      key: string;
      similarity: number;
      metadata: any;
    }> = [];
    
    // Calculate similarities with all embeddings
    for (const [key, index] of this.embeddings.entries()) {
      const similarity = this.cosineSimilarity(embedding, index.embedding);
      
      if (similarity >= threshold) {
        similarities.push({
          key,
          similarity,
          metadata: index.metadata
        });
      }
    }
    
    // Sort by similarity descending
    similarities.sort((a, b) => b.similarity - a.similarity);
    
    // Get top entries from storage
    const results: Array<{ entry: CacheEntry; similarity: number }> = [];
    
    for (const sim of similarities.slice(0, limit)) {
      const entry = await this.storage.get(sim.key);
      if (entry && !this.isExpired(entry)) {
        results.push({
          entry,
          similarity: sim.similarity
        });
      }
    }
    
    return results;
  }
  
  /**
   * Index new cache entry
   */
  async indexEntry(entry: CacheEntry, embedding: number[]): Promise<void> {
    const index: EmbeddingIndex = {
      key: entry.key,
      embedding,
      metadata: {
        provider: entry.provider,
        model: entry.model,
        tokenCount: entry.content.tokenCount || 0
      }
    };
    
    this.embeddings.set(entry.key, index);
    
    // Persist embeddings periodically
    if (this.embeddings.size % 10 === 0) {
      await this.saveEmbeddings();
    }
  }
  
  /**
   * Remove entry from index
   */
  async removeFromIndex(key: string): Promise<void> {
    this.embeddings.delete(key);
  }
  
  /**
   * Clear all embeddings
   */
  async clear(): Promise<void> {
    this.embeddings.clear();
    await this.saveEmbeddings();
  }
  
  /**
   * Dispose and save embeddings
   */
  async dispose(): Promise<void> {
    await this.saveEmbeddings();
  }
  
  // Private methods
  
  /**
   * Calculate cosine similarity between two vectors
   */
  private cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) {
      throw new Error('Vectors must have same dimensions');
    }
    
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;
    
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    
    normA = Math.sqrt(normA);
    normB = Math.sqrt(normB);
    
    if (normA === 0 || normB === 0) {
      return 0;
    }
    
    return dotProduct / (normA * normB);
  }
  
  /**
   * Check if entry has expired
   */
  private isExpired(entry: CacheEntry): boolean {
    if (!entry.ttl || entry.ttl <= 0) {
      return false;
    }
    
    const expiresAt = new Date(entry.createdAt.getTime() + entry.ttl * 1000);
    return new Date() > expiresAt;
  }
  
  /**
   * Load embeddings from storage
   */
  private async loadEmbeddings(): Promise<void> {
    try {
      // Check if embeddings are stored
      const embeddingsData = await this.storage.get('__embeddings__');
      if (embeddingsData && embeddingsData.content.data) {
        const indices = embeddingsData.content.data as EmbeddingIndex[];
        
        // Rebuild embeddings map
        this.embeddings.clear();
        for (const index of indices) {
          this.embeddings.set(index.key, index);
        }
      }
    } catch (error) {
      console.error('Failed to load embeddings:', error);
    }
  }
  
  /**
   * Save embeddings to storage
   */
  private async saveEmbeddings(): Promise<void> {
    try {
      const indices = Array.from(this.embeddings.values());
      
      const entry: CacheEntry = {
        key: '__embeddings__',
        provider: 'system',
        model: 'embeddings',
        content: {
          type: 'embedding',
          data: indices
        },
        metadata: {
          count: indices.length,
          model: this.embeddingModel
        },
        ttl: 0, // Never expire
        createdAt: new Date(),
        lastAccessedAt: new Date(),
        accessCount: 0
      };
      
      await this.storage.set('__embeddings__', entry);
      
    } catch (error) {
      console.error('Failed to save embeddings:', error);
    }
  }
  
  /**
   * Build more sophisticated index structures if needed
   */
  private buildIndex(): void {
    if (this.config.indexType === 'hnsw') {
      // TODO: Implement HNSW index for better performance
      // For now, we use brute force search
    }
  }
}