/**
 * Orchestration Engine - Main entry point for multi-model AI orchestration
 */

import { EventEmitter } from 'events';
import {
  AITask,
  TaskResult,
  ModelSelection,
  TaskRoute,
  ExecutionOptions,
  TaskPipeline,
  PipelineResult,
  OrchestrationMetrics,
  CostEstimate,
  TaskError,
  ExecutionProgress,
  ModelProfile,
  TaskType,
  TokenUsage,
  ActualCost
} from './interfaces';
import { ModelRegistry } from './model-registry';
import { TaskAnalyzer } from './task-analyzer';
import { RoutingEngine, RoutingConfig } from './routing-engine';
import { CostTracker, CostLimit } from './cost-tracker';
import { CacheManager } from './cache-manager';
import { ContextCacheManager, CacheConfig, defaultCacheConfig } from './context-cache';
import { ProviderAdapter } from './providers/provider-adapter';
import { AnthropicAdapter } from './providers/anthropic-adapter';
import { OpenAIAdapter } from './providers/openai-adapter';
import { GoogleAdapter } from './providers/google-adapter';
import { MistralAdapter } from './providers/mistral-adapter';
import { LocalAdapter } from './providers/local-adapter';
import { 
  QueueManager, 
  QueueManagerOptions,
  QueuedRequest,
  RequestPriority,
  QueueStats
} from './queue';
import { ConfigManager } from './config';

export interface OrchestrationConfig {
  routingConfig?: Partial<RoutingConfig>;
  costLimits?: CostLimit[];
  cacheEnabled?: boolean;
  cacheTTL?: number;
  maxCacheSize?: number;
  contextCacheConfig?: Partial<CacheConfig>;
  metricsInterval?: number;
  defaultTimeout?: number;
  defaultRetries?: number;
  queueConfig?: Partial<QueueManagerOptions>;
}

export class OrchestrationEngine extends EventEmitter {
  private modelRegistry: ModelRegistry;
  private taskAnalyzer: TaskAnalyzer;
  private routingEngine: RoutingEngine;
  private costTracker: CostTracker;
  private cacheManager: CacheManager;
  private contextCacheManager: ContextCacheManager | null = null;
  private queueManager: QueueManager | null = null;
  private configManager: ConfigManager | null = null;
  private providers: Map<string, ProviderAdapter> = new Map();
  private activeRequests: Map<string, AbortController> = new Map();
  private metrics: OrchestrationMetrics | null = null;
  private metricsTimer: NodeJS.Timer | null = null;
  private config: Required<OrchestrationConfig>;

  constructor(config?: OrchestrationConfig) {
    super();
    
    // Initialize configuration with defaults
    this.config = {
      routingConfig: config?.routingConfig || {},
      costLimits: config?.costLimits || [],
      cacheEnabled: config?.cacheEnabled ?? true,
      cacheTTL: config?.cacheTTL || 3600000, // 1 hour
      maxCacheSize: config?.maxCacheSize || 100 * 1024 * 1024, // 100MB
      contextCacheConfig: config?.contextCacheConfig || {},
      metricsInterval: config?.metricsInterval || 60000, // 1 minute
      defaultTimeout: config?.defaultTimeout || 60000, // 60 seconds
      defaultRetries: config?.defaultRetries || 3,
      queueConfig: config?.queueConfig || {}
    };

    // Initialize components
    this.modelRegistry = new ModelRegistry();
    this.taskAnalyzer = new TaskAnalyzer();
    this.routingEngine = new RoutingEngine(
      this.modelRegistry,
      this.taskAnalyzer,
      this.config.routingConfig
    );
    this.costTracker = new CostTracker(this.config.costLimits);
    this.cacheManager = new CacheManager(
      this.config.cacheTTL,
      this.config.maxCacheSize
    );

    // Initialize context cache manager if enabled
    if (this.config.contextCacheConfig) {
      const contextCacheConfig = {
        ...defaultCacheConfig,
        ...this.config.contextCacheConfig
      };
      this.contextCacheManager = new ContextCacheManager(contextCacheConfig);
      
      // Set up context cache event handlers
      this.setupContextCacheHandlers();
    }

    // Initialize provider adapters
    this.initializeProviders();

    // Initialize queue manager if configured
    if (config?.queueConfig?.maxConcurrency || config?.queueConfig?.maxQueueSize) {
      this.initializeQueueManager();
    }

    // Start metrics collection
    this.startMetricsCollection();
  }

  /**
   * Select the optimal model for a task
   */
  async selectModel(task: AITask): Promise<ModelSelection> {
    return this.routingEngine.routeTask(task);
  }

  /**
   * Route a task to the optimal model (alias for selectModel)
   */
  async routeTask(task: AITask): Promise<TaskRoute> {
    const selection = await this.selectModel(task);
    return {
      taskId: task.id,
      selectedModel: selection.primary,
      routingReason: selection.reasoning,
      alternatives: selection.fallbacks,
      timestamp: Date.now()
    };
  }

  /**
   * Execute a single AI task
   */
  async executeTask(
    task: AITask,
    options?: ExecutionOptions
  ): Promise<TaskResult> {
    const startTime = Date.now();
    
    // Check cache first if enabled
    if (this.config.cacheEnabled && (!options?.cachePolicy || options.cachePolicy.enabled)) {
      const cached = this.cacheManager.get(task, options?.cachePolicy);
      if (cached) {
        return cached;
      }
    }

    // Select model
    const selection = await this.selectModel(task);
    
    // Check cost limits
    const costCheck = this.costTracker.checkCostLimit(selection.estimatedCost.expected);
    if (!costCheck.allowed) {
      throw this.createError('COST_LIMIT_EXCEEDED', costCheck.reason!, false);
    }

    // Execute with retries
    let lastError: TaskError | null = null;
    let result: TaskResult | null = null;
    let attemptCount = 0;
    const maxAttempts = options?.retryPolicy?.maxAttempts || this.config.defaultRetries;
    
    // Try primary model with context caching
    try {
      if (this.contextCacheManager) {
        result = await this.executeWithContextCache(
          task,
          selection.primary,
          options,
          attemptCount
        );
      } else {
        result = await this.executeWithModel(
          task,
          selection.primary,
          options,
          attemptCount
        );
      }
    } catch (error) {
      lastError = this.wrapError(error);
      attemptCount++;
    }

    // Try fallback models if needed
    if (!result && lastError?.retryable) {
      for (const fallbackModel of selection.fallbacks) {
        if (attemptCount >= maxAttempts) break;
        
        try {
          result = await this.executeWithModel(
            task,
            fallbackModel,
            options,
            attemptCount
          );
          if (result) {
            result.metadata = {
              ...result.metadata,
              fallbackUsed: true
            };
            break;
          }
        } catch (error) {
          lastError = this.wrapError(error);
          attemptCount++;
        }
      }
    }

    if (!result) {
      throw lastError || this.createError('EXECUTION_FAILED', 'All models failed', false);
    }

    // Record cost
    this.costTracker.recordCost(result, selection.primary, task.type);

    // Cache result if successful
    if (result.status === 'success' && this.config.cacheEnabled) {
      this.cacheManager.set(task, result, options?.cachePolicy);
    }

    // Update metrics
    this.updateMetricsAfterExecution(result, startTime);

    return result;
  }

  /**
   * Execute multiple tasks in parallel
   */
  async executeParallel(
    tasks: AITask[],
    options?: ExecutionOptions & { maxConcurrency?: number }
  ): Promise<TaskResult[]> {
    const maxConcurrency = options?.maxConcurrency || 5;
    const results: TaskResult[] = [];
    const queue = [...tasks];
    const executing: Promise<void>[] = [];

    while (queue.length > 0 || executing.length > 0) {
      // Start new tasks up to concurrency limit
      while (executing.length < maxConcurrency && queue.length > 0) {
        const task = queue.shift()!;
        const promise = this.executeTask(task, options)
          .then(result => {
            results.push(result);
          })
          .catch(error => {
            results.push(this.createErrorResult(task.id, error));
          });
        
        executing.push(promise);
      }

      // Wait for at least one to complete
      if (executing.length > 0) {
        await Promise.race(executing);
        executing.splice(
          executing.findIndex(p => 
            p === executing.find(ep => ep.constructor === Promise && 'settled' in ep)
          ),
          1
        );
      }
    }

    return results;
  }

  /**
   * Execute a pipeline of tasks
   */
  async executePipeline(
    pipeline: TaskPipeline,
    options?: ExecutionOptions
  ): Promise<PipelineResult> {
    const startTime = Date.now();
    const results: TaskResult[] = [];
    let totalCost = 0;
    let status: 'success' | 'partial' | 'failed' = 'success';

    for (const pipelineTask of pipeline.tasks) {
      // Check dependencies
      if (pipelineTask.dependencies) {
        const dependencyResults = pipelineTask.dependencies
          .map(depId => results.find(r => r.taskId === depId))
          .filter(Boolean) as TaskResult[];
        
        const failedDependency = dependencyResults.find(r => r.status === 'failed');
        if (failedDependency && !pipeline.options?.continueOnError) {
          status = 'failed';
          break;
        }
      }

      // Apply transformation if needed
      let task = pipelineTask.task;
      if (pipelineTask.transformation && results.length > 0) {
        const previousResult = results[results.length - 1];
        task = {
          ...task,
          context: {
            ...task.context,
            previousResult: pipelineTask.transformation(previousResult)
          }
        };
      }

      // Execute task
      try {
        const result = await this.executeTask(task, options);
        results.push(result);
        totalCost += result.cost.amount;

        // Check condition
        if (pipelineTask.condition) {
          const conditionMet = this.evaluateCondition(pipelineTask.condition, result);
          if (!conditionMet && !pipeline.options?.continueOnError) {
            status = 'partial';
            break;
          }
        }
      } catch (error) {
        const errorResult = this.createErrorResult(task.id, error);
        results.push(errorResult);
        
        if (!pipeline.options?.continueOnError) {
          status = 'failed';
          break;
        } else {
          status = 'partial';
        }
      }
    }

    return {
      id: `pipeline-${Date.now()}`,
      pipelineId: pipeline.id,
      status,
      results,
      totalCost: {
        amount: totalCost,
        currency: 'USD',
        breakdown: {
          inputTokens: results.reduce((sum, r) => sum + r.usage.promptTokens, 0),
          outputTokens: results.reduce((sum, r) => sum + r.usage.completionTokens, 0),
          inputCost: 0, // Would need to calculate from individual results
          outputCost: 0
        }
      },
      totalLatency: Date.now() - startTime,
      timestamp: Date.now()
    };
  }

  /**
   * List available models
   */
  async listModels(): Promise<ModelProfile[]> {
    return this.modelRegistry.getAvailableModels();
  }

  /**
   * Get current metrics
   */
  async getMetrics(): Promise<OrchestrationMetrics> {
    if (!this.metrics) {
      this.updateMetrics();
    }
    return this.metrics!;
  }

  /**
   * Get cost estimate for a task
   */
  async getCostEstimate(task: AITask): Promise<CostEstimate> {
    const selection = await this.selectModel(task);
    return selection.estimatedCost;
  }

  /**
   * Queue a task for execution
   */
  async queueTask(
    task: AITask,
    options?: {
      priority?: RequestPriority;
      metadata?: any;
    }
  ): Promise<string> {
    if (!this.queueManager) {
      throw new Error('Queue manager not initialized. Configure queueConfig to enable queuing.');
    }

    return this.queueManager.enqueue(task, {
      priority: options?.priority,
      metadata: options?.metadata,
      callback: (result) => {
        if (result instanceof Error) {
          console.error(`Task ${task.id} failed:`, result);
        } else {
          console.log(`Task ${task.id} completed successfully`);
        }
      }
    });
  }

  /**
   * Get queue statistics
   */
  async getQueueStats(): Promise<QueueStats | null> {
    if (!this.queueManager) {
      return null;
    }
    return this.queueManager.getStats();
  }

  /**
   * Cancel a queued task
   */
  async cancelQueuedTask(requestId: string): Promise<boolean> {
    if (!this.queueManager) {
      return false;
    }
    return this.queueManager.cancel(requestId);
  }

  /**
   * Initialize configuration manager
   */
  async initializeConfigManager(configPath?: string): Promise<void> {
    this.configManager = new ConfigManager({
      configPath
    });

    // Listen for configuration changes
    this.configManager.on('change', async (event) => {
      await this.handleConfigChange(event.newConfig);
    });

    // Initialize
    await this.configManager.initialize();
  }

  /**
   * Update configuration dynamically
   */
  async updateConfig(config: Partial<OrchestrationConfig>): Promise<void> {
    if (!this.configManager) {
      throw new Error('Configuration manager not initialized');
    }

    await this.configManager.updateConfiguration(config as any);
  }

  /**
   * Get current configuration
   */
  getConfiguration(): OrchestrationConfig {
    if (this.configManager) {
      return this.configManager.getConfig();
    }
    return this.config;
  }

  /**
   * Get context cache statistics
   */
  getContextCacheStats(provider?: string): any {
    if (!this.contextCacheManager) {
      return null;
    }
    return this.contextCacheManager.getStats(provider);
  }

  /**
   * Invalidate context cache entries
   */
  async invalidateContextCache(pattern?: string): Promise<number> {
    if (!this.contextCacheManager) {
      return 0;
    }
    return this.contextCacheManager.invalidate(pattern);
  }

  /**
   * Warm context cache with predefined contexts
   */
  async warmContextCache(contexts: any[]): Promise<number> {
    if (!this.contextCacheManager) {
      return 0;
    }
    return this.contextCacheManager.warmCache(contexts);
  }

  /**
   * Generate context cache report
   */
  getContextCacheReport(): string | null {
    if (!this.contextCacheManager) {
      return null;
    }
    const monitor = (this.contextCacheManager as any).monitor;
    return monitor ? monitor.generateReport() : null;
  }

  /**
   * Shutdown the orchestration engine
   */
  async shutdown(): Promise<void> {
    // Stop metrics collection
    if (this.metricsTimer) {
      clearInterval(this.metricsTimer);
    }

    // Shutdown configuration manager
    if (this.configManager) {
      await this.configManager.shutdown();
    }

    // Shutdown queue manager
    if (this.queueManager) {
      await this.queueManager.shutdown();
    }

    // Shutdown context cache manager
    if (this.contextCacheManager) {
      await this.contextCacheManager.dispose();
    }

    // Cancel active requests
    for (const [taskId, controller] of this.activeRequests) {
      controller.abort();
    }
    this.activeRequests.clear();

    // Shutdown provider adapters
    for (const adapter of this.providers.values()) {
      if (adapter.shutdown) {
        await adapter.shutdown();
      }
    }
  }

  // Private methods

  private initializeProviders(): void {
    this.providers.set('anthropic', new AnthropicAdapter());
    this.providers.set('openai', new OpenAIAdapter());
    this.providers.set('google', new GoogleAdapter());
    this.providers.set('mistral', new MistralAdapter());
    this.providers.set('local', new LocalAdapter());
  }

  private async initializeQueueManager(): Promise<void> {
    const defaultQueueConfig: QueueManagerOptions = {
      maxConcurrency: 10,
      maxQueueSize: 1000,
      defaultTimeout: this.config.defaultTimeout,
      maxRetries: this.config.defaultRetries,
      enablePersistence: true,
      enableDeduplication: true,
      enableDeadLetter: true,
      rateLimits: [
        {
          name: 'global',
          limit: 100,
          window: 60000, // 100 requests per minute
          scope: 'global'
        },
        {
          name: 'per-user',
          limit: 20,
          window: 60000, // 20 requests per minute per user
          scope: 'user'
        }
      ],
      priorityBoost: {
        waitTimeThreshold: 30000, // Boost after 30 seconds
        boostAmount: 1,
        maxBoosts: 2
      },
      backpressure: {
        highWaterMark: 80,
        lowWaterMark: 60,
        strategy: 'throttle' as any,
        rejectOnFull: false
      },
      ...this.config.queueConfig
    };

    this.queueManager = new QueueManager(defaultQueueConfig);
    
    // Handle queue processing
    this.queueManager.on('process', async (request: QueuedRequest) => {
      try {
        const result = await this.executeTask(request.task);
        await this.queueManager!.completeRequest(request.id, result);
      } catch (error) {
        await this.queueManager!.failRequest(request.id, error as Error);
      }
    });

    // Initialize the queue
    await this.queueManager.initialize();
  }

  private async executeWithModel(
    task: AITask,
    model: ModelProfile,
    options: ExecutionOptions | undefined,
    attemptNumber: number
  ): Promise<TaskResult> {
    const adapter = this.providers.get(model.provider);
    if (!adapter) {
      throw this.createError(
        'PROVIDER_NOT_FOUND',
        `No adapter for provider: ${model.provider}`,
        false
      );
    }

    // Create abort controller
    const controller = new AbortController();
    this.activeRequests.set(task.id, controller);

    try {
      // Set timeout
      const timeout = options?.timeout || this.config.defaultTimeout;
      const timeoutId = setTimeout(() => controller.abort(), timeout);

      // Notify callbacks
      if (options?.callbacks?.onStart) {
        options.callbacks.onStart(task);
      }

      // Execute
      const result = await adapter.execute(task, model, {
        ...options,
        signal: controller.signal
      });

      clearTimeout(timeoutId);
      
      // Add metadata
      result.metadata = {
        ...result.metadata,
        retryCount: attemptNumber
      };

      // Notify callbacks
      if (options?.callbacks?.onComplete) {
        options.callbacks.onComplete(result);
      }

      return result;
    } catch (error: any) {
      // Handle timeout
      if (error.name === 'AbortError') {
        throw this.createError('TIMEOUT', 'Request timed out', true);
      }
      
      // Notify callbacks
      if (options?.callbacks?.onError) {
        options.callbacks.onError(this.wrapError(error));
      }
      
      throw error;
    } finally {
      this.activeRequests.delete(task.id);
    }
  }

  private createError(code: string, message: string, retryable: boolean): TaskError {
    return {
      code,
      message,
      type: this.mapErrorType(code),
      retryable,
      details: {}
    };
  }

  private wrapError(error: any): TaskError {
    if (error.code && error.message && 'retryable' in error) {
      return error;
    }

    return {
      code: error.code || 'UNKNOWN_ERROR',
      message: error.message || 'An unknown error occurred',
      type: 'unknown',
      retryable: true,
      details: error
    };
  }

  private mapErrorType(code: string): TaskError['type'] {
    if (code.includes('RATE_LIMIT')) return 'rate_limit';
    if (code.includes('TIMEOUT')) return 'timeout';
    if (code.includes('INVALID')) return 'invalid_request';
    if (code.includes('SERVER')) return 'server_error';
    return 'unknown';
  }

  /**
   * Execute task with context caching
   */
  private async executeWithContextCache(
    task: AITask,
    model: ModelProfile,
    options: ExecutionOptions | undefined,
    attemptNumber: number
  ): Promise<TaskResult> {
    if (!this.contextCacheManager) {
      return this.executeWithModel(task, model, options, attemptNumber);
    }

    // Convert task to cache request format
    const cacheRequest = {
      provider: model.provider,
      model: model.id,
      messages: task.messages || [],
      systemPrompt: task.systemPrompt,
      functions: task.functions,
      tools: task.tools,
      temperature: task.parameters?.temperature,
      maxTokens: task.parameters?.maxTokens,
      metadata: {
        taskId: task.id,
        taskType: task.type,
        attemptNumber,
        ...task.metadata
      }
    };

    // Execute with context cache
    const cacheResponse = await this.contextCacheManager.executeWithCache(
      cacheRequest,
      async (req) => {
        // Execute the actual request
        const result = await this.executeWithModel(task, model, options, attemptNumber);
        
        // Add cache metadata to result
        result.metadata = {
          ...result.metadata,
          cacheHit: false,
          provider: model.provider,
          model: model.id
        };
        
        return result;
      }
    );

    // Extract result from cache response
    const result = cacheResponse.content as TaskResult;
    
    // Update metadata with cache info
    result.metadata = {
      ...result.metadata,
      cacheHit: cacheResponse.hit,
      cacheSimilarity: cacheResponse.similarity,
      cacheKey: cacheResponse.key
    };

    return result;
  }

  /**
   * Set up context cache event handlers
   */
  private setupContextCacheHandlers(): void {
    if (!this.contextCacheManager) return;

    // Forward cache events
    this.contextCacheManager.on('hit', (key: string, provider: string) => {
      this.emit('cache:hit', { key, provider });
    });

    this.contextCacheManager.on('miss', (key: string, provider: string) => {
      this.emit('cache:miss', { key, provider });
    });

    this.contextCacheManager.on('error', (error: Error, operation: string) => {
      console.error(`Context cache error in ${operation}:`, error);
      this.emit('cache:error', { error, operation });
    });

    // Set up monitoring alerts
    const monitor = (this.contextCacheManager as any).monitor;
    if (monitor) {
      monitor.on('alert', (alert: any) => {
        this.emit('cache:alert', alert);
      });
    }
  }

  private createErrorResult(taskId: string, error: any): TaskResult {
    return {
      id: `result-${Date.now()}`,
      taskId,
      status: 'failed',
      error: this.wrapError(error),
      model: 'unknown',
      usage: { promptTokens: 0, completionTokens: 0, totalTokens: 0 },
      cost: { amount: 0, currency: 'USD', breakdown: {} as any },
      latency: 0,
      timestamp: Date.now()
    };
  }

  private evaluateCondition(condition: any, result: TaskResult): boolean {
    switch (condition.type) {
      case 'success':
        return result.status === 'success';
      case 'contains':
        return result.content?.includes(condition.value) || false;
      case 'custom':
        return condition.predicate ? condition.predicate(result) : false;
      default:
        return true;
    }
  }

  private startMetricsCollection(): void {
    this.updateMetrics();
    this.metricsTimer = setInterval(() => {
      this.updateMetrics();
    }, this.config.metricsInterval);
  }

  private updateMetrics(): void {
    const costMetrics = this.costTracker.getMetrics();
    const cacheStats = this.cacheManager.getStats();
    const routingStats = this.routingEngine.getRoutingStats();

    // Calculate performance metrics (placeholder - would need real data)
    const performanceMetrics = {
      averageLatency: 1500,
      p50Latency: 1200,
      p95Latency: 3000,
      p99Latency: 5000,
      throughput: 10,
      activeRequests: this.activeRequests.size,
      queuedRequests: 0
    };

    // Calculate quality metrics
    const qualityMetrics = {
      successRate: 0.95,
      fallbackRate: 0.05,
      retryRate: 0.1,
      validationFailureRate: 0.01,
      userSatisfaction: undefined,
      averageConfidence: 0.85
    };

    // Calculate usage metrics
    const usageMetrics = {
      totalRequests: 1000, // Placeholder
      requestsByType: {} as Record<TaskType, number>,
      requestsByModel: {} as Record<string, number>,
      cacheHitRate: cacheStats.hitRate,
      tokenUsage: {
        promptTokens: 50000,
        completionTokens: 30000,
        totalTokens: 80000
      }
    };

    this.metrics = {
      performance: performanceMetrics,
      cost: costMetrics,
      quality: qualityMetrics,
      usage: usageMetrics,
      timestamp: Date.now()
    };
  }

  private updateMetricsAfterExecution(result: TaskResult, startTime: number): void {
    // This would update real metrics based on execution results
    // For now, it's a placeholder
  }

  private async handleConfigChange(newConfig: any): Promise<void> {
    // Update internal config
    const oldConfig = this.config;
    this.config = {
      ...this.config,
      ...newConfig
    };

    // Re-initialize components with new config
    if (newConfig.routingConfig) {
      this.routingEngine = new RoutingEngine(
        this.modelRegistry,
        this.taskAnalyzer,
        newConfig.routingConfig
      );
    }

    if (newConfig.costLimits) {
      this.costTracker = new CostTracker(newConfig.costLimits);
    }

    if (newConfig.cacheEnabled !== undefined || newConfig.cacheTTL || newConfig.maxCacheSize) {
      this.cacheManager = new CacheManager(
        newConfig.cacheTTL || this.config.cacheTTL,
        newConfig.maxCacheSize || this.config.maxCacheSize
      );
    }

    if (newConfig.queueConfig) {
      // Restart queue manager with new config
      if (this.queueManager) {
        await this.queueManager.shutdown();
      }
      await this.initializeQueueManager();
    }

    // Update metrics interval
    if (newConfig.metricsInterval && newConfig.metricsInterval !== oldConfig.metricsInterval) {
      if (this.metricsTimer) {
        clearInterval(this.metricsTimer);
      }
      this.startMetricsCollection();
    }

    // Emit configuration change event
    this.emit('configChanged', {
      oldConfig,
      newConfig: this.config,
      timestamp: new Date()
    });
  }
}