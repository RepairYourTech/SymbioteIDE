/**
 * Context Manager
 * 
 * Handles intelligent context management for tasks including bubbles, handoffs, and inheritance
 */

import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import {
  TaskContext,
  ContextScope,
  ContextArtifact,
  ContextHandoff,
  HandoffType,
  HandoffStatus,
  ContextTransform,
  TransformType,
  ContextBubble,
  BubbleIsolation,
  ContextInheritance,
  InheritanceStrategy
} from './types';
import { Logger } from '../utils/logger';
import { Redis } from 'ioredis';
import { ModelProfile } from '../orchestration/interfaces';
import { OrchestrationEngine } from '../orchestration/engine';
import { EmbeddingService } from '../services/embedding-service';
import { ContextOptimizationAgent } from './context-optimization-agent';
import { HandoffCoordinationAgent } from './handoff-coordination-agent';

export class ContextManager extends EventEmitter {
  private logger = new Logger('ContextManager');
  private contexts = new Map<string, TaskContext>();
  private handoffs = new Map<string, ContextHandoff>();
  private bubbles = new Map<string, ContextBubble>();
  private transforms = new Map<string, ContextTransform>();
  private redis?: Redis;
  private orchestrationEngine?: OrchestrationEngine;
  private embeddingService?: EmbeddingService;
  private modelContextSizes = new Map<string, number>();
  private tokenCountCache = new Map<string, number>();
  private optimizationAgent?: ContextOptimizationAgent;
  private handoffCoordinator?: HandoffCoordinationAgent;

  constructor(private config: ContextManagerConfig = {}) {
    super();
    this.registerDefaultTransforms();
    this.initializeModelContextSizes();
  }

  /**
   * Initialize context manager
   */
  async initialize(): Promise<void> {
    if (this.config.redis) {
      const { createRedisClient } = await import('../infrastructure/redis/redis-client');
      this.redis = createRedisClient();
    }

    // Initialize orchestration engine for model info
    this.orchestrationEngine = OrchestrationEngine.getInstance();
    
    // Initialize embedding service for token counting
    this.embeddingService = new EmbeddingService({
      providers: ['openai', 'google', 'cohere']
    });
    await this.embeddingService.initialize();

    // Initialize optimization agent if configured
    if (this.config.useOptimizationAgent) {
      this.optimizationAgent = new ContextOptimizationAgent(this);
      await this.optimizationAgent.initialize();
    }

    this.logger.info('Context Manager initialized');
  }

  /**
   * Initialize known model context sizes
   */
  private initializeModelContextSizes(): void {
    // OpenAI models
    this.modelContextSizes.set('gpt-4-turbo', 128000);
    this.modelContextSizes.set('gpt-4-turbo-preview', 128000);
    this.modelContextSizes.set('gpt-4', 8192);
    this.modelContextSizes.set('gpt-4-32k', 32768);
    this.modelContextSizes.set('gpt-3.5-turbo', 16384);
    this.modelContextSizes.set('gpt-3.5-turbo-16k', 16384);
    
    // Anthropic models
    this.modelContextSizes.set('claude-3-opus', 200000);
    this.modelContextSizes.set('claude-3-sonnet', 200000);
    this.modelContextSizes.set('claude-3-haiku', 200000);
    this.modelContextSizes.set('claude-2.1', 200000);
    this.modelContextSizes.set('claude-2', 100000);
    this.modelContextSizes.set('claude-instant', 100000);
    
    // Google models
    this.modelContextSizes.set('gemini-1.5-pro', 1000000);
    this.modelContextSizes.set('gemini-1.5-flash', 1000000);
    this.modelContextSizes.set('gemini-pro', 32768);
    this.modelContextSizes.set('gemini-pro-vision', 32768);
    
    // Mistral models
    this.modelContextSizes.set('mistral-large', 32768);
    this.modelContextSizes.set('mistral-medium', 32768);
    this.modelContextSizes.set('mistral-small', 32768);
    this.modelContextSizes.set('mixtral-8x7b', 32768);
    
    // Cohere models
    this.modelContextSizes.set('command-r-plus', 128000);
    this.modelContextSizes.set('command-r', 128000);
    this.modelContextSizes.set('command', 4096);
    
    // Meta models
    this.modelContextSizes.set('llama-3.1-405b', 128000);
    this.modelContextSizes.set('llama-3.1-70b', 128000);
    this.modelContextSizes.set('llama-3.1-8b', 128000);
    this.modelContextSizes.set('llama-3-70b', 8192);
    this.modelContextSizes.set('llama-3-8b', 8192);
    
    // xAI models
    this.modelContextSizes.set('grok-1', 8192);
    this.modelContextSizes.set('grok-1.5', 128000);
  }

  /**
   * Create context for task
   */
  async createContext(taskId: string, params: CreateContextParams): Promise<TaskContext> {
    try {
      const contextId = `ctx_${uuidv4()}`;
      
      // Inherit from parent if specified
      let inheritedData = {};
      let inheritedVariables = {};
      
      if (params.parentContextId) {
        const parentContext = await this.getContext(params.parentContextId);
        if (parentContext) {
          const inherited = await this.applyInheritance(
            parentContext,
            params.inheritance
          );
          inheritedData = inherited.data;
          inheritedVariables = inherited.variables;
        }
      }

      // Create context
      const context: TaskContext = {
        id: contextId,
        taskId,
        parentContextId: params.parentContextId,
        scope: params.scope,
        data: { ...inheritedData, ...params.data },
        variables: { ...inheritedVariables, ...params.variables },
        artifacts: [],
        handoffs: [],
        bubbles: [],
        inheritance: params.inheritance,
        createdAt: new Date(),
        updatedAt: new Date()
      };

      // Store context
      await this.storeContext(context);
      
      this.logger.debug(`Created context ${contextId} for task ${taskId}`);
      return context;
    } catch (error) {
      this.logger.error('Failed to create context', error);
      throw error;
    }
  }

  /**
   * Update context
   */
  async updateContext(contextId: string, updates: UpdateContextParams): Promise<TaskContext> {
    try {
      const context = await this.getContext(contextId);
      if (!context) {
        throw new Error(`Context ${contextId} not found`);
      }

      // Merge updates
      if (updates.data) {
        context.data = { ...context.data, ...updates.data };
      }
      
      if (updates.variables) {
        context.variables = { ...context.variables, ...updates.variables };
      }
      
      if (updates.artifacts) {
        context.artifacts.push(...updates.artifacts);
      }

      context.updatedAt = new Date();

      // Store updated context
      await this.storeContext(context);
      
      // Propagate to children if cascade inheritance
      if (context.inheritance.strategy === InheritanceStrategy.Cascade) {
        await this.propagateToChildren(context);
      }

      return context;
    } catch (error) {
      this.logger.error(`Failed to update context ${contextId}`, error);
      throw error;
    }
  }

  /**
   * Get context
   */
  async getContext(contextId: string): Promise<TaskContext | null> {
    try {
      // Check memory cache
      if (this.contexts.has(contextId)) {
        return this.contexts.get(contextId)!;
      }

      // Check Redis if available
      if (this.redis) {
        const data = await this.redis.get(`context:${contextId}`);
        if (data) {
          const context = JSON.parse(data);
          this.contexts.set(contextId, context);
          return context;
        }
      }

      return null;
    } catch (error) {
      this.logger.error(`Failed to get context ${contextId}`, error);
      throw error;
    }
  }

  /**
   * Create handoff between tasks
   */
  async createHandoff(params: CreateHandoffParams): Promise<ContextHandoff> {
    try {
      const handoffId = `handoff_${uuidv4()}`;
      
      // Get source context
      const fromTask = await this.getContextByTaskId(params.fromTaskId);
      if (!fromTask) {
        throw new Error(`Source task context not found: ${params.fromTaskId}`);
      }

      // Get model information if agent IDs provided
      let sourceModelId: string | undefined;
      let targetModelId: string | undefined;
      
      if (params.fromAgentId && params.toAgentId) {
        sourceModelId = params.fromModelId || await this.getModelForAgent(params.fromAgentId);
        targetModelId = params.toModelId || await this.getModelForAgent(params.toAgentId);
      }

      // Prepare data for handoff
      let handoffData = params.data || fromTask.data;
      
      // Apply intelligent context sizing if models are known
      if (sourceModelId && targetModelId) {
        // Use optimization agent if available
        if (this.optimizationAgent && this.config.useOptimizationAgent) {
          const optimizationResult = await this.optimizationAgent.optimizeForHandoff({
            data: handoffData,
            sourceModelId,
            targetModelId,
            preservePriority: params.preservePriority,
            strategies: params.strategies
          });
          handoffData = optimizationResult.optimizedData;
        } else {
          // Fallback to built-in optimization
          handoffData = await this.intelligentContextResize(
            handoffData,
            sourceModelId,
            targetModelId,
            params.preservePriority
          );
        }
      }
      
      // Apply transforms
      if (params.transforms && params.transforms.length > 0) {
        handoffData = await this.applyTransforms(handoffData, params.transforms);
      }

      // Create handoff
      const handoff: ContextHandoff = {
        id: handoffId,
        type: params.type,
        fromTaskId: params.fromTaskId,
        toTaskId: params.toTaskId,
        fromAgentId: params.fromAgentId,
        toAgentId: params.toAgentId,
        data: handoffData,
        transforms: params.transforms || [],
        status: HandoffStatus.Pending,
        createdAt: new Date(),
        metadata: {
          sourceModelId,
          targetModelId,
          originalTokenCount: await this.estimateTokenCount(params.data || fromTask.data),
          optimizedTokenCount: await this.estimateTokenCount(handoffData)
        }
      };

      // Store handoff
      this.handoffs.set(handoffId, handoff);
      
      // Add to source context
      fromTask.handoffs.push(handoff);
      await this.storeContext(fromTask);

      this.logger.info(`Created handoff ${handoffId} from ${params.fromTaskId} to ${params.toTaskId}`);
      return handoff;
    } catch (error) {
      this.logger.error('Failed to create handoff', error);
      throw error;
    }
  }

  /**
   * Intelligently resize context for target model
   */
  private async intelligentContextResize(
    data: any,
    sourceModelId: string,
    targetModelId: string,
    preservePriority?: string[]
  ): Promise<any> {
    const sourceContextSize = this.getModelContextSize(sourceModelId);
    const targetContextSize = this.getModelContextSize(targetModelId);
    
    // If target has same or larger context, no need to resize
    if (targetContextSize >= sourceContextSize) {
      return data;
    }

    const currentTokens = await this.estimateTokenCount(data);
    
    // If already fits, no need to resize
    if (currentTokens <= targetContextSize) {
      return data;
    }

    this.logger.info(`Resizing context from ${sourceModelId} (${sourceContextSize} tokens) to ${targetModelId} (${targetContextSize} tokens)`);
    
    // Apply intelligent compression strategies
    let optimizedData = data;
    
    // Strategy 1: Preserve priority data
    if (preservePriority && preservePriority.length > 0) {
      optimizedData = await this.preservePriorityData(data, preservePriority, targetContextSize);
    }
    
    // Strategy 2: Summarization for text content
    if (typeof data === 'string' || (data.content && typeof data.content === 'string')) {
      optimizedData = await this.summarizeContent(data, targetContextSize);
    }
    
    // Strategy 3: Chunk and reference for structured data
    if (typeof data === 'object' && !Array.isArray(data)) {
      optimizedData = await this.chunkAndReference(data, targetContextSize);
    }
    
    // Strategy 4: Vector compression for embeddings
    if (data.embeddings || data.vectors) {
      optimizedData = await this.compressEmbeddings(data, targetContextSize);
    }
    
    // Strategy 5: Progressive detail reduction
    if (await this.estimateTokenCount(optimizedData) > targetContextSize) {
      optimizedData = await this.progressiveDetailReduction(optimizedData, targetContextSize);
    }
    
    return optimizedData;
  }

  /**
   * Get model context size
   */
  private getModelContextSize(modelId: string): number {
    // Check cache
    if (this.modelContextSizes.has(modelId)) {
      return this.modelContextSizes.get(modelId)!;
    }
    
    // Try to get from orchestration engine
    const model = this.orchestrationEngine?.getModel(modelId);
    if (model?.capabilities?.maxTokens) {
      this.modelContextSizes.set(modelId, model.capabilities.maxTokens);
      return model.capabilities.maxTokens;
    }
    
    // Default fallback
    return 8192;
  }

  /**
   * Estimate token count for data
   */
  private async estimateTokenCount(data: any): Promise<number> {
    const dataStr = JSON.stringify(data);
    
    // Check cache
    if (this.tokenCountCache.has(dataStr)) {
      return this.tokenCountCache.get(dataStr)!;
    }
    
    // Use embedding service for accurate count
    if (this.embeddingService) {
      try {
        const embedding = await this.embeddingService.createEmbedding(dataStr, {
          provider: 'openai'
        });
        const tokenCount = embedding.tokenCount || Math.ceil(dataStr.length / 4);
        this.tokenCountCache.set(dataStr, tokenCount);
        return tokenCount;
      } catch (error) {
        // Fallback to estimation
      }
    }
    
    // Simple estimation: ~4 characters per token
    const estimated = Math.ceil(dataStr.length / 4);
    this.tokenCountCache.set(dataStr, estimated);
    return estimated;
  }

  /**
   * Preserve priority data within token limit
   */
  private async preservePriorityData(
    data: any,
    priorityKeys: string[],
    tokenLimit: number
  ): Promise<any> {
    const result: any = {};
    let currentTokens = 0;
    
    // First, include all priority data
    for (const key of priorityKeys) {
      if (data[key] !== undefined) {
        result[key] = data[key];
        currentTokens = await this.estimateTokenCount(result);
        
        if (currentTokens > tokenLimit * 0.8) {
          // Priority data alone is too large, need to compress
          result[key] = await this.compressValue(data[key], tokenLimit / priorityKeys.length);
        }
      }
    }
    
    // Then, add non-priority data if space allows
    const remainingTokens = tokenLimit - currentTokens;
    for (const [key, value] of Object.entries(data)) {
      if (!priorityKeys.includes(key) && remainingTokens > 0) {
        const valueTokens = await this.estimateTokenCount(value);
        if (valueTokens < remainingTokens) {
          result[key] = value;
          currentTokens += valueTokens;
        }
      }
    }
    
    return result;
  }

  /**
   * Summarize content to fit token limit
   */
  private async summarizeContent(data: any, tokenLimit: number): Promise<any> {
    const content = typeof data === 'string' ? data : data.content;
    
    // Use AI to summarize if available
    if (this.orchestrationEngine) {
      try {
        const summary = await this.orchestrationEngine.chat({
          model: 'gpt-3.5-turbo',
          messages: [{
            role: 'system',
            content: `Summarize the following content to fit within ${tokenLimit} tokens while preserving key information:`
          }, {
            role: 'user',
            content: content
          }],
          maxTokens: Math.floor(tokenLimit * 0.8)
        });
        
        return typeof data === 'string' ? summary : { ...data, content: summary };
      } catch (error) {
        this.logger.warn('Failed to summarize content', error);
      }
    }
    
    // Fallback to truncation
    return this.truncateToTokenLimit(data, tokenLimit);
  }

  /**
   * Chunk and reference large data
   */
  private async chunkAndReference(data: any, tokenLimit: number): Promise<any> {
    const chunks: any[] = [];
    const references: any = {
      _type: 'chunked_context',
      _chunks: []
    };
    
    let currentChunk: any = {};
    let currentTokens = 0;
    
    for (const [key, value] of Object.entries(data)) {
      const valueTokens = await this.estimateTokenCount(value);
      
      if (currentTokens + valueTokens > tokenLimit * 0.7) {
        // Store current chunk
        const chunkId = `chunk_${chunks.length}`;
        chunks.push({ id: chunkId, data: currentChunk });
        references._chunks.push(chunkId);
        
        // Start new chunk
        currentChunk = {};
        currentTokens = 0;
      }
      
      currentChunk[key] = value;
      currentTokens += valueTokens;
    }
    
    // Store final chunk
    if (Object.keys(currentChunk).length > 0) {
      const chunkId = `chunk_${chunks.length}`;
      chunks.push({ id: chunkId, data: currentChunk });
      references._chunks.push(chunkId);
    }
    
    // Store chunks in context for later retrieval
    references._metadata = {
      originalSize: await this.estimateTokenCount(data),
      chunkCount: chunks.length,
      retrieval: 'Use context retrieval to access full data'
    };
    
    return references;
  }

  /**
   * Compress embeddings/vectors
   */
  private async compressEmbeddings(data: any, tokenLimit: number): Promise<any> {
    if (!data.embeddings && !data.vectors) {
      return data;
    }
    
    const vectors = data.embeddings || data.vectors;
    const metadata = {
      originalDimensions: vectors[0]?.length || 0,
      originalCount: vectors.length
    };
    
    // Dimensionality reduction
    const reducedVectors = vectors.map((vec: number[]) => {
      // Simple PCA-like reduction (in practice, use proper algorithms)
      const step = Math.ceil(vec.length / 128); // Reduce to 128 dimensions
      return vec.filter((_, i) => i % step === 0);
    });
    
    return {
      ...data,
      embeddings: reducedVectors,
      _compression: metadata
    };
  }

  /**
   * Progressive detail reduction
   */
  private async progressiveDetailReduction(data: any, tokenLimit: number): Promise<any> {
    let result = JSON.parse(JSON.stringify(data)); // Deep clone
    let currentTokens = await this.estimateTokenCount(result);
    
    // Reduction strategies in order of preference
    const reductionStrategies = [
      // Remove verbose descriptions
      () => this.removeKeys(result, ['description', 'details', 'notes']),
      // Truncate arrays
      () => this.truncateArrays(result, 10),
      // Remove metadata
      () => this.removeKeys(result, ['metadata', 'meta', '_meta']),
      // Truncate strings
      () => this.truncateStrings(result, 100),
      // Remove non-essential keys
      () => this.keepOnlyEssentialKeys(result)
    ];
    
    for (const strategy of reductionStrategies) {
      if (currentTokens <= tokenLimit) break;
      
      result = strategy();
      currentTokens = await this.estimateTokenCount(result);
    }
    
    return result;
  }

  /**
   * Helper: Remove keys from object recursively
   */
  private removeKeys(obj: any, keys: string[]): any {
    if (typeof obj !== 'object' || obj === null) return obj;
    
    const result = Array.isArray(obj) ? [...obj] : { ...obj };
    
    for (const key of keys) {
      delete result[key];
    }
    
    for (const [key, value] of Object.entries(result)) {
      result[key] = this.removeKeys(value, keys);
    }
    
    return result;
  }

  /**
   * Helper: Truncate arrays to max length
   */
  private truncateArrays(obj: any, maxLength: number): any {
    if (Array.isArray(obj)) {
      return obj.slice(0, maxLength);
    }
    
    if (typeof obj !== 'object' || obj === null) return obj;
    
    const result = { ...obj };
    for (const [key, value] of Object.entries(result)) {
      result[key] = this.truncateArrays(value, maxLength);
    }
    
    return result;
  }

  /**
   * Helper: Truncate strings to max length
   */
  private truncateStrings(obj: any, maxLength: number): any {
    if (typeof obj === 'string') {
      return obj.length > maxLength ? obj.substring(0, maxLength) + '...' : obj;
    }
    
    if (typeof obj !== 'object' || obj === null) return obj;
    
    const result = Array.isArray(obj) ? [...obj] : { ...obj };
    for (const [key, value] of Object.entries(result)) {
      result[key] = this.truncateStrings(value, maxLength);
    }
    
    return result;
  }

  /**
   * Helper: Keep only essential keys
   */
  private keepOnlyEssentialKeys(obj: any): any {
    const essentialKeys = ['id', 'type', 'name', 'status', 'result', 'error', 'data'];
    
    if (typeof obj !== 'object' || obj === null) return obj;
    
    const result: any = {};
    for (const key of essentialKeys) {
      if (obj[key] !== undefined) {
        result[key] = obj[key];
      }
    }
    
    return result;
  }

  /**
   * Helper: Compress a single value
   */
  private async compressValue(value: any, tokenLimit: number): Promise<any> {
    const currentTokens = await this.estimateTokenCount(value);
    if (currentTokens <= tokenLimit) return value;
    
    if (typeof value === 'string') {
      const ratio = tokenLimit / currentTokens;
      return value.substring(0, Math.floor(value.length * ratio)) + '...';
    }
    
    if (Array.isArray(value)) {
      const ratio = tokenLimit / currentTokens;
      return value.slice(0, Math.floor(value.length * ratio));
    }
    
    return value;
  }

  /**
   * Helper: Truncate to token limit
   */
  private truncateToTokenLimit(data: any, tokenLimit: number): any {
    if (typeof data === 'string') {
      // Rough estimate: 4 chars per token
      const maxChars = tokenLimit * 4;
      return data.length > maxChars ? data.substring(0, maxChars) + '...' : data;
    }
    
    return data;
  }

  /**
   * Get model for agent
   */
  private async getModelForAgent(agentId: string): Promise<string | undefined> {
    // This would be implemented to look up the model used by an agent
    // For now, return undefined
    return undefined;
  }

  /**
   * Get context window information for all models
   */
  getModelContextWindows(): Map<string, number> {
    return new Map(this.modelContextSizes);
  }

  /**
   * Update model context size
   */
  updateModelContextSize(modelId: string, contextSize: number): void {
    this.modelContextSizes.set(modelId, contextSize);
    this.logger.info(`Updated context size for ${modelId}: ${contextSize} tokens`);
  }

  /**
   * Get context optimization strategies
   */
  getOptimizationStrategies(): string[] {
    return [
      'preserve_priority',
      'summarization',
      'chunk_and_reference',
      'vector_compression',
      'progressive_reduction'
    ];
  }

  /**
   * Analyze context handoff feasibility
   */
  async analyzeHandoffFeasibility(
    sourceModelId: string,
    targetModelId: string,
    dataSize: number
  ): Promise<{
    feasible: boolean;
    sourceContext: number;
    targetContext: number;
    dataTokens: number;
    compressionNeeded: boolean;
    estimatedCompression?: number;
    strategies?: string[];
  }> {
    const sourceContext = this.getModelContextSize(sourceModelId);
    const targetContext = this.getModelContextSize(targetModelId);
    
    const feasible = dataSize <= targetContext;
    const compressionNeeded = dataSize > targetContext;
    
    let estimatedCompression: number | undefined;
    let strategies: string[] | undefined;
    
    if (compressionNeeded) {
      const compressionRatio = targetContext / dataSize;
      estimatedCompression = Math.floor(compressionRatio * 100);
      
      // Suggest strategies based on compression needed
      strategies = [];
      if (compressionRatio > 0.7) {
        strategies.push('progressive_reduction');
      } else if (compressionRatio > 0.5) {
        strategies.push('preserve_priority', 'summarization');
      } else if (compressionRatio > 0.3) {
        strategies.push('chunk_and_reference', 'vector_compression');
      } else {
        strategies.push('chunk_and_reference', 'external_storage');
      }
    }
    
    return {
      feasible,
      sourceContext,
      targetContext,
      dataTokens: dataSize,
      compressionNeeded,
      estimatedCompression,
      strategies
    };
  }

  /**
   * Get handoff by ID
   */
  async getHandoff(handoffId: string): Promise<ContextHandoff | null> {
    return this.handoffs.get(handoffId) || null;
  }

  /**
   * Apply transform to data
   */
  async applyTransform(data: any, transform: any): Promise<any> {
    if (typeof transform === 'string') {
      const transformObj = this.transforms.get(transform);
      if (transformObj) {
        return transformObj.apply(data);
      }
    }
    
    // Direct transform function
    if (typeof transform === 'function') {
      return transform(data);
    }
    
    return data;
  }

  /**
   * Set handoff coordinator
   */
  setHandoffCoordinator(coordinator: HandoffCoordinationAgent): void {
    this.handoffCoordinator = coordinator;
  }

  /**
   * Create intelligent handoff with coordination
   */
  async createIntelligentHandoff(params: CreateHandoffParams): Promise<ContextHandoff> {
    // Use coordinator if available
    if (this.handoffCoordinator) {
      return this.handoffCoordinator.coordinateHandoff({
        ...params,
        type: params.type
      });
    }
    
    // Fallback to regular handoff
    return this.createHandoff(params);
  }

  /**
   * Complete handoff
   */
  async completeHandoff(handoffId: string, result?: any): Promise<ContextHandoff> {
    try {
      const handoff = this.handoffs.get(handoffId);
      if (!handoff) {
        throw new Error(`Handoff ${handoffId} not found`);
      }

      // Update handoff status
      handoff.status = HandoffStatus.Completed;
      handoff.completedAt = new Date();

      // Transfer data to target context
      const targetContext = await this.getContextByTaskId(handoff.toTaskId);
      if (targetContext) {
        targetContext.data = {
          ...targetContext.data,
          handoff: {
            id: handoffId,
            data: result || handoff.data,
            fromTaskId: handoff.fromTaskId,
            timestamp: new Date()
          }
        };
        await this.storeContext(targetContext);
      }

      this.logger.info(`Completed handoff ${handoffId}`);
      return handoff;
    } catch (error) {
      this.logger.error(`Failed to complete handoff ${handoffId}`, error);
      throw error;
    }
  }

  /**
   * Create context bubble
   */
  async createBubble(params: CreateBubbleParams): Promise<ContextBubble> {
    try {
      const bubbleId = `bubble_${uuidv4()}`;
      
      const bubble: ContextBubble = {
        id: bubbleId,
        name: params.name,
        taskIds: params.taskIds,
        isolation: params.isolation,
        sharedData: params.sharedData || {},
        permissions: params.permissions || {
          read: params.taskIds,
          write: params.taskIds,
          execute: params.taskIds
        }
      };

      // Store bubble
      this.bubbles.set(bubbleId, bubble);
      
      // Update task contexts to include bubble
      for (const taskId of params.taskIds) {
        const context = await this.getContextByTaskId(taskId);
        if (context) {
          context.bubbles.push(bubble);
          await this.storeContext(context);
        }
      }

      this.logger.info(`Created context bubble ${bubbleId} for ${params.taskIds.length} tasks`);
      return bubble;
    } catch (error) {
      this.logger.error('Failed to create bubble', error);
      throw error;
    }
  }

  /**
   * Get bubble data for task
   */
  async getBubbleData(taskId: string, bubbleId: string): Promise<any> {
    try {
      const bubble = this.bubbles.get(bubbleId);
      if (!bubble) {
        throw new Error(`Bubble ${bubbleId} not found`);
      }

      // Check permissions
      if (!bubble.permissions.read.includes(taskId)) {
        throw new Error(`Task ${taskId} does not have read access to bubble ${bubbleId}`);
      }

      // Apply isolation rules
      if (bubble.isolation === BubbleIsolation.Full) {
        return { ...bubble.sharedData };
      } else if (bubble.isolation === BubbleIsolation.Partial) {
        // Filter based on task-specific rules
        const context = await this.getContextByTaskId(taskId);
        if (context) {
          return this.filterBubbleData(bubble.sharedData, context);
        }
      }

      return bubble.sharedData;
    } catch (error) {
      this.logger.error(`Failed to get bubble data for task ${taskId}`, error);
      throw error;
    }
  }

  /**
   * Add artifact to context
   */
  async addArtifact(contextId: string, artifact: Omit<ContextArtifact, 'id' | 'createdAt'>): Promise<ContextArtifact> {
    try {
      const context = await this.getContext(contextId);
      if (!context) {
        throw new Error(`Context ${contextId} not found`);
      }

      const fullArtifact: ContextArtifact = {
        ...artifact,
        id: `artifact_${uuidv4()}`,
        createdAt: new Date()
      };

      context.artifacts.push(fullArtifact);
      await this.storeContext(context);

      return fullArtifact;
    } catch (error) {
      this.logger.error(`Failed to add artifact to context ${contextId}`, error);
      throw error;
    }
  }

  /**
   * Register transform
   */
  registerTransform(transform: ContextTransform): void {
    this.transforms.set(transform.id, transform);
    this.logger.debug(`Registered transform: ${transform.id}`);
  }

  /**
   * Apply inheritance rules
   */
  private async applyInheritance(
    parentContext: TaskContext,
    inheritance: ContextInheritance
  ): Promise<{ data: any; variables: any }> {
    let data = {};
    let variables = {};

    switch (inheritance.strategy) {
      case InheritanceStrategy.All:
        data = { ...parentContext.data };
        variables = { ...parentContext.variables };
        break;
      
      case InheritanceStrategy.Selective:
        // Include only specified fields
        for (const field of inheritance.includes) {
          if (field in parentContext.data) {
            data[field] = parentContext.data[field];
          }
          if (field in parentContext.variables) {
            variables[field] = parentContext.variables[field];
          }
        }
        break;
      
      case InheritanceStrategy.Cascade:
        // Include all except excluded
        data = { ...parentContext.data };
        variables = { ...parentContext.variables };
        
        for (const field of inheritance.excludes) {
          delete data[field];
          delete variables[field];
        }
        break;
      
      case InheritanceStrategy.None:
        // No inheritance
        break;
    }

    // Apply overrides
    if (inheritance.overrides) {
      data = { ...data, ...inheritance.overrides };
    }

    return { data, variables };
  }

  /**
   * Apply transforms to data
   */
  private async applyTransforms(data: any, transformIds: string[]): Promise<any> {
    let result = data;
    
    for (const transformId of transformIds) {
      const transform = this.transforms.get(transformId);
      if (transform) {
        result = await transform.apply(result);
      }
    }
    
    return result;
  }

  /**
   * Propagate context updates to children
   */
  private async propagateToChildren(context: TaskContext): Promise<void> {
    // Find child contexts
    const childContexts = await this.getChildContexts(context.id);
    
    for (const childContext of childContexts) {
      if (childContext.inheritance.strategy === InheritanceStrategy.Cascade) {
        const inherited = await this.applyInheritance(context, childContext.inheritance);
        
        await this.updateContext(childContext.id, {
          data: inherited.data,
          variables: inherited.variables
        });
      }
    }
  }

  /**
   * Get child contexts
   */
  private async getChildContexts(parentContextId: string): Promise<TaskContext[]> {
    const children: TaskContext[] = [];
    
    for (const context of this.contexts.values()) {
      if (context.parentContextId === parentContextId) {
        children.push(context);
      }
    }
    
    return children;
  }

  /**
   * Get context by task ID
   */
  private async getContextByTaskId(taskId: string): Promise<TaskContext | null> {
    for (const context of this.contexts.values()) {
      if (context.taskId === taskId) {
        return context;
      }
    }
    
    // Check Redis if available
    if (this.redis) {
      const keys = await this.redis.keys('context:*');
      for (const key of keys) {
        const data = await this.redis.get(key);
        if (data) {
          const context = JSON.parse(data);
          if (context.taskId === taskId) {
            this.contexts.set(context.id, context);
            return context;
          }
        }
      }
    }
    
    return null;
  }

  /**
   * Store context
   */
  private async storeContext(context: TaskContext): Promise<void> {
    // Store in memory
    this.contexts.set(context.id, context);
    
    // Store in Redis if available
    if (this.redis) {
      await this.redis.set(
        `context:${context.id}`,
        JSON.stringify(context),
        'EX',
        86400 // 24 hours
      );
    }
  }

  /**
   * Filter bubble data based on context
   */
  private filterBubbleData(data: any, context: TaskContext): any {
    // Implement custom filtering logic based on context
    // This is a simplified version
    const filtered = {};
    
    for (const [key, value] of Object.entries(data)) {
      // Check if task has access to this data
      if (context.data.permissions?.[key] || context.scope === ContextScope.Global) {
        filtered[key] = value;
      }
    }
    
    return filtered;
  }

  /**
   * Register default transforms
   */
  private registerDefaultTransforms(): void {
    // Map transform
    this.registerTransform({
      id: 'map',
      name: 'Map Transform',
      type: TransformType.Map,
      config: {},
      apply: async (data: any) => {
        if (Array.isArray(data)) {
          return data.map(item => ({ ...item, transformed: true }));
        }
        return { ...data, transformed: true };
      }
    });

    // Filter transform
    this.registerTransform({
      id: 'filter',
      name: 'Filter Transform',
      type: TransformType.Filter,
      config: {},
      apply: async (data: any) => {
        if (Array.isArray(data)) {
          return data.filter(item => item.active !== false);
        }
        return data;
      }
    });

    // Format transform
    this.registerTransform({
      id: 'format',
      name: 'Format Transform',
      type: TransformType.Format,
      config: {},
      apply: async (data: any) => {
        return JSON.stringify(data, null, 2);
      }
    });
  }

  /**
   * Close context manager
   */
  async close(): Promise<void> {
    if (this.redis) {
      await this.redis.quit();
    }
    
    this.contexts.clear();
    this.handoffs.clear();
    this.bubbles.clear();
    this.transforms.clear();
    
    this.logger.info('Context Manager closed');
  }
}

// Configuration interface
export interface ContextManagerConfig {
  redis?: boolean;
  useOptimizationAgent?: boolean;
}

// Parameter interfaces
export interface CreateContextParams {
  parentContextId?: string;
  scope: ContextScope;
  data: Record<string, any>;
  variables: Record<string, any>;
  inheritance: ContextInheritance;
}

export interface UpdateContextParams {
  data?: Record<string, any>;
  variables?: Record<string, any>;
  artifacts?: ContextArtifact[];
}

export interface CreateHandoffParams {
  type: HandoffType;
  fromTaskId: string;
  toTaskId: string;
  fromAgentId?: string;
  toAgentId?: string;
  fromModelId?: string;
  toModelId?: string;
  data: any;
  transforms?: string[];
  preservePriority?: string[];
}

export interface CreateBubbleParams {
  name: string;
  taskIds: string[];
  isolation: BubbleIsolation;
  sharedData?: Record<string, any>;
  permissions?: {
    read: string[];
    write: string[];
    execute: string[];
  };
}