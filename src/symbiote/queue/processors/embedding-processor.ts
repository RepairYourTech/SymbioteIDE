/**
 * Embedding Processor - Handles embedding generation jobs
 */

import { Job } from 'bullmq';
import { BaseProcessor } from './base-processor';
import { 
  EmbeddingJobData, 
  JobResult, 
  JobType,
  QueueName 
} from '../types';
import { QdrantManager } from '../../search/qdrant/qdrant-manager';
import { EmbeddingService } from '../../search/qdrant/embedding-service';
import { RedisManager } from '../../redis/redis-manager';

export class EmbeddingProcessor extends BaseProcessor<EmbeddingJobData> {
  private qdrantManager?: QdrantManager;
  private embeddingService?: EmbeddingService;
  private redisManager?: RedisManager;
  private rateLimiter?: any;

  constructor(queueManager: any) {
    super(queueManager, 'EmbeddingProcessor');
  }

  /**
   * Initialize services
   */
  private async initializeServices(): Promise<void> {
    if (!this.embeddingService) {
      this.embeddingService = new EmbeddingService({
        provider: process.env.EMBEDDING_PROVIDER || 'openai',
        apiKey: process.env.OPENAI_API_KEY || '',
        model: process.env.EMBEDDING_MODEL || 'text-embedding-3-small',
        dimensions: parseInt(process.env.EMBEDDING_DIMENSIONS || '1536')
      });
      
      this.qdrantManager = new QdrantManager({
        url: process.env.QDRANT_URL || 'http://localhost:6333',
        apiKey: process.env.QDRANT_API_KEY
      });
      await this.qdrantManager.initialize();
      
      this.redisManager = RedisManager.getInstance();
      
      // Initialize rate limiter for API calls
      this.rateLimiter = await this.redisManager.createRateLimiter(
        'embedding-api',
        {
          points: 100, // Number of requests
          duration: 60, // Per 60 seconds
          blockDuration: 10 // Block for 10 seconds if exceeded
        }
      );
    }
  }

  /**
   * Process embedding job
   */
  async process(job: Job<EmbeddingJobData>): Promise<JobResult> {
    try {
      this.validateJobData(job.data);
      await this.initializeServices();

      switch (job.data.type) {
        case JobType.GenerateEmbedding:
          return await this.generateEmbedding(job);
          
        case JobType.BatchEmbeddings:
          return await this.generateBatchEmbeddings(job);
          
        case JobType.UpdateEmbedding:
          return await this.updateEmbedding(job);
          
        default:
          throw new Error(`Unknown job type: ${job.data.type}`);
      }
    } catch (error: any) {
      return this.handleFailure(job, error);
    }
  }

  /**
   * Generate single embedding
   */
  private async generateEmbedding(job: Job<EmbeddingJobData>): Promise<JobResult> {
    const { content, metadata } = job.data;
    
    if (typeof content !== 'string') {
      return this.createErrorResult(
        'INVALID_CONTENT',
        'Content must be a string for single embedding generation'
      );
    }
    
    await this.updateProgress(job, 0, 'Checking rate limits');
    
    // Check rate limit
    try {
      await this.rateLimiter?.consume('global', 1);
    } catch (rateLimiterRes: any) {
      return this.createErrorResult(
        'RATE_LIMIT_EXCEEDED',
        `Rate limit exceeded. Retry after ${rateLimiterRes.msBeforeNext}ms`,
        { retryAfter: rateLimiterRes.msBeforeNext }
      );
    }
    
    await this.updateProgress(job, 20, 'Checking cache');
    
    // Check cache
    const cacheKey = `embedding:${this.hashContent(content)}:${metadata?.model || 'default'}`;
    const cached = await this.redisManager?.get(cacheKey);
    
    if (cached) {
      this.logger.debug(`Using cached embedding for job ${job.id}`);
      return this.createSuccessResult({
        embedding: cached,
        cached: true,
        metadata
      });
    }
    
    await this.updateProgress(job, 40, 'Generating embedding');
    
    // Generate embedding
    const startTime = Date.now();
    const embedding = await this.embeddingService!.generateEmbedding(content);
    const generationTime = Date.now() - startTime;
    
    await this.updateProgress(job, 70, 'Caching result');
    
    // Cache the result
    await this.redisManager?.set(
      cacheKey,
      embedding,
      3600 * 24 * 7 // 7 days TTL
    );
    
    await this.updateProgress(job, 90, 'Storing in vector DB');
    
    // Store in Qdrant if ID provided
    if (metadata?.fileId) {
      await this.qdrantManager?.upsertEmbedding(
        embedding,
        {
          id: `${metadata.fileId}_chunk_${metadata.chunkIndex || 0}`,
          metadata: {
            ...metadata,
            generatedAt: new Date().toISOString()
          }
        }
      );
    }
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      embedding,
      cached: false,
      metadata,
      metrics: {
        generationTime,
        dimensions: embedding.length
      }
    });
  }

  /**
   * Generate batch embeddings
   */
  private async generateBatchEmbeddings(job: Job<EmbeddingJobData>): Promise<JobResult> {
    const { content, metadata } = job.data;
    
    if (!Array.isArray(content)) {
      return this.createErrorResult(
        'INVALID_CONTENT',
        'Content must be an array for batch embedding generation'
      );
    }
    
    const totalItems = content.length;
    const batchSize = 10; // Process 10 at a time
    const embeddings: Array<{ content: string; embedding: number[] }> = [];
    let cachedCount = 0;
    let generatedCount = 0;
    let totalGenerationTime = 0;
    
    await this.updateProgress(job, 0, `Processing ${totalItems} items`);
    
    // Process in batches
    for (let i = 0; i < totalItems; i += batchSize) {
      const batch = content.slice(i, Math.min(i + batchSize, totalItems));
      const batchEmbeddings = [];
      
      // Check rate limit for batch
      try {
        await this.rateLimiter?.consume('global', batch.length);
      } catch (rateLimiterRes: any) {
        // Wait and retry
        const waitTime = rateLimiterRes.msBeforeNext || 1000;
        this.logger.warn(`Rate limit hit, waiting ${waitTime}ms`);
        await new Promise(resolve => setTimeout(resolve, waitTime));
        
        // Retry consuming
        await this.rateLimiter?.consume('global', batch.length);
      }
      
      for (const text of batch) {
        const cacheKey = `embedding:${this.hashContent(text)}:${metadata?.model || 'default'}`;
        const cached = await this.redisManager?.get(cacheKey);
        
        if (cached) {
          batchEmbeddings.push({
            content: text,
            embedding: cached
          });
          cachedCount++;
        } else {
          const startTime = Date.now();
          const embedding = await this.embeddingService!.generateEmbedding(text);
          totalGenerationTime += Date.now() - startTime;
          generatedCount++;
          
          // Cache the result
          await this.redisManager?.set(
            cacheKey,
            embedding,
            3600 * 24 * 7 // 7 days TTL
          );
          
          batchEmbeddings.push({
            content: text,
            embedding
          });
        }
      }
      
      embeddings.push(...batchEmbeddings);
      
      const progress = ((i + batch.length) / totalItems) * 100;
      await this.updateProgress(
        job,
        progress,
        `Processed ${i + batch.length}/${totalItems} items`
      );
    }
    
    // Store in Qdrant if requested
    if (metadata?.fileId && this.qdrantManager) {
      const qdrantPoints = embeddings.map((item, index) => ({
        id: `${metadata.fileId}_batch_${job.id}_${index}`,
        vector: item.embedding,
        metadata: {
          content: item.content,
          batchJobId: job.id,
          index,
          ...metadata,
          generatedAt: new Date().toISOString()
        }
      }));
      
      await this.qdrantManager.upsertBatch(qdrantPoints);
    }
    
    return this.createSuccessResult({
      embeddings: embeddings.map(e => e.embedding),
      totalItems,
      cachedCount,
      generatedCount,
      metrics: {
        averageGenerationTime: generatedCount > 0 ? totalGenerationTime / generatedCount : 0,
        totalGenerationTime,
        cacheHitRate: cachedCount / totalItems
      }
    });
  }

  /**
   * Update existing embedding
   */
  private async updateEmbedding(job: Job<EmbeddingJobData>): Promise<JobResult> {
    const { content, metadata } = job.data;
    
    if (!metadata?.fileId) {
      return this.createErrorResult(
        'MISSING_FILE_ID',
        'File ID is required for updating embeddings'
      );
    }
    
    // Generate new embedding
    const result = await this.generateEmbedding({
      ...job,
      data: {
        ...job.data,
        type: JobType.GenerateEmbedding
      }
    });
    
    if (!result.success) {
      return result;
    }
    
    // Delete old embedding from cache
    const oldCacheKey = `embedding:${metadata.fileId}:*`;
    const keys = await this.redisManager?.keys(oldCacheKey);
    if (keys && keys.length > 0) {
      await this.redisManager?.del(...keys);
    }
    
    return result;
  }

  /**
   * Hash content for cache key
   */
  private hashContent(content: string): string {
    const crypto = require('crypto');
    return crypto
      .createHash('sha256')
      .update(content)
      .digest('hex')
      .substring(0, 16);
  }

  /**
   * Validate job data
   */
  protected validateJobData(data: EmbeddingJobData): void {
    if (!data.content) {
      throw new Error('Content is required for embedding generation');
    }
    
    if (!data.type) {
      throw new Error('Job type is required');
    }
    
    if (data.type === JobType.BatchEmbeddings && !Array.isArray(data.content)) {
      throw new Error('Content must be an array for batch embeddings');
    }
    
    if (data.type === JobType.GenerateEmbedding && typeof data.content !== 'string') {
      throw new Error('Content must be a string for single embedding');
    }
  }

  /**
   * Custom retry logic for embedding jobs
   */
  protected shouldRetry(error: Error, job: Job<EmbeddingJobData>): boolean {
    // Always retry rate limit errors
    if (error.message.includes('RATE_LIMIT')) {
      return true;
    }
    
    // Retry API errors up to 3 times
    if (error.message.includes('API') && job.attemptsMade < 3) {
      return true;
    }
    
    // Don't retry validation errors
    if (error.message.includes('INVALID') || error.message.includes('MISSING')) {
      return false;
    }
    
    return super.shouldRetry(error, job);
  }
}