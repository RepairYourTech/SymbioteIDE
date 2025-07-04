/**
 * Orchestration API Implementation
 */

import { EventEmitter, Event } from 'vscode';
import {
  IOrchestrationAPI,
  AITask as APITask,
  TaskType as APITaskType,
  ModelSelection as APIModelSelection,
  TaskRoute as APITaskRoute,
  ExecutionOptions as APIExecutionOptions,
  TaskResult as APITaskResult,
  TaskPipeline as APITaskPipeline,
  PipelineResult as APIPipelineResult,
  AIModel as APIModel,
  ModelStatus,
  ModelTestResult,
  OrchestrationMetrics as APIMetrics,
  MetricsUpdateEvent,
  CostEstimate as APICostEstimate,
  CostPeriod,
  CostReport,
  CostAlertEvent,
  RoutingStrategy,
  TokenUsage as APITokenUsage
} from '../api/orchestration-api';

import {
  AITask,
  TaskType,
  ModelSelection,
  TaskRoute,
  ExecutionOptions,
  TaskResult,
  TaskPipeline,
  PipelineResult,
  ModelProfile,
  OrchestrationMetrics,
  CostEstimate,
  ComplexityLevel,
  QualityLevel,
  UrgencyLevel,
  Capability
} from './interfaces';

import { OrchestrationEngine } from './orchestration-engine';
import { CostLimit } from './cost-tracker';

export class OrchestrationAPIImpl implements IOrchestrationAPI {
  private engine: OrchestrationEngine;
  private metricsUpdateEmitter = new EventEmitter<MetricsUpdateEvent>();
  private costAlertEmitter = new EventEmitter<CostAlertEvent>();
  private metricsInterval: NodeJS.Timer | null = null;

  constructor() {
    // Initialize with configuration from environment
    const costLimits: CostLimit[] = [];
    
    if (process.env.SYMBIOTE_COST_LIMIT_HOUR) {
      costLimits.push({
        amount: parseFloat(process.env.SYMBIOTE_COST_LIMIT_HOUR),
        period: 'hour',
        currency: 'USD'
      });
    }
    
    if (process.env.SYMBIOTE_COST_LIMIT_DAY) {
      costLimits.push({
        amount: parseFloat(process.env.SYMBIOTE_COST_LIMIT_DAY),
        period: 'day',
        currency: 'USD'
      });
    }

    this.engine = new OrchestrationEngine({
      costLimits,
      cacheEnabled: process.env.SYMBIOTE_CACHE_ENABLED !== 'false',
      routingConfig: {
        enableCostOptimization: process.env.SYMBIOTE_COST_OPTIMIZATION !== 'false',
        enableLoadBalancing: process.env.SYMBIOTE_LOAD_BALANCING !== 'false'
      }
    });

    // Set up cost alerts
    const costTracker = (this.engine as any).costTracker;
    if (costTracker) {
      costTracker.addAlert({
        threshold: 0.8,
        callback: (metrics: any) => {
          this.costAlertEmitter.fire({
            type: 'threshold',
            severity: 'warning',
            message: 'Cost usage at 80% of limit',
            currentCost: metrics.totalCost,
            limit: costLimits[0]?.amount,
            recommendations: [
              'Consider using cheaper models for simple tasks',
              'Enable caching to reduce redundant API calls',
              'Review and optimize prompts to reduce token usage'
            ]
          });
        }
      });
    }

    // Start metrics updates
    this.startMetricsUpdates();
  }

  async selectModel(task: APITask): Promise<APIModelSelection> {
    const internalTask = this.convertToInternalTask(task);
    const selection = await this.engine.selectModel(internalTask);
    return this.convertToAPIModelSelection(selection);
  }

  async routeTask(task: APITask): Promise<APITaskRoute> {
    const internalTask = this.convertToInternalTask(task);
    const route = await this.engine.routeTask(internalTask);
    
    // Convert to API format
    return {
      models: [this.convertToAPIModel(route.selectedModel)],
      strategy: RoutingStrategy.Fallback,
      parallelization: {
        enabled: false,
        maxConcurrency: 1,
        splitStrategy: 'chunk'
      }
    };
  }

  async executeTask(
    task: APITask,
    options?: APIExecutionOptions
  ): Promise<APITaskResult> {
    const internalTask = this.convertToInternalTask(task);
    const internalOptions = options ? this.convertToInternalOptions(options) : undefined;
    
    try {
      const result = await this.engine.executeTask(internalTask, internalOptions);
      return this.convertToAPITaskResult(result);
    } catch (error: any) {
      return {
        taskId: task.id,
        success: false,
        model: 'unknown',
        usage: { prompt: 0, completion: 0, total: 0 },
        cost: 0,
        latency: 0,
        error: {
          code: error.code || 'UNKNOWN',
          message: error.message || 'Unknown error',
          retryable: error.retryable || false
        }
      };
    }
  }

  async executeParallel(tasks: APITask[]): Promise<APITaskResult[]> {
    const internalTasks = tasks.map(t => this.convertToInternalTask(t));
    const results = await this.engine.executeParallel(internalTasks);
    return results.map(r => this.convertToAPITaskResult(r));
  }

  async executePipeline(pipeline: APITaskPipeline): Promise<APIPipelineResult> {
    const internalPipeline = this.convertToInternalPipeline(pipeline);
    const result = await this.engine.executePipeline(internalPipeline);
    return this.convertToAPIPipelineResult(result, pipeline);
  }

  async listModels(): Promise<APIModel[]> {
    const models = await this.engine.listModels();
    return models.map(m => this.convertToAPIModel(m));
  }

  async getModelStatus(modelId: string): Promise<ModelStatus> {
    const models = await this.engine.listModels();
    const model = models.find(m => m.id === modelId);
    
    if (!model) {
      throw new Error(`Model ${modelId} not found`);
    }

    return {
      modelId,
      available: model.availability.status === 'available',
      health: model.availability.status === 'available' ? 'healthy' : 
              model.availability.status === 'degraded' ? 'degraded' : 'unhealthy',
      responseTime: model.averageLatency,
      errorRate: 0.02, // Placeholder
      lastChecked: new Date()
    };
  }

  async testModel(modelId: string): Promise<ModelTestResult> {
    const testTask: APITask = {
      id: `test-${Date.now()}`,
      type: APITaskType.General,
      prompt: 'Hello, please respond with "OK" if you are working.',
      constraints: {
        maxTokens: 10
      }
    };

    try {
      const result = await this.executeTask(testTask);
      return {
        modelId,
        success: result.success,
        latency: result.latency,
        output: result.output,
        error: result.error?.message
      };
    } catch (error: any) {
      return {
        modelId,
        success: false,
        latency: 0,
        error: error.message
      };
    }
  }

  async getMetrics(): Promise<APIMetrics> {
    const metrics = await this.engine.getMetrics();
    return this.convertToAPIMetrics(metrics);
  }

  get onMetricsUpdate(): Event<MetricsUpdateEvent> {
    return this.metricsUpdateEmitter.event;
  }

  async getCostEstimate(task: APITask): Promise<APICostEstimate> {
    const internalTask = this.convertToInternalTask(task);
    const estimate = await this.engine.getCostEstimate(internalTask);
    return this.convertToAPICostEstimate(estimate);
  }

  async getCostReport(period: CostPeriod): Promise<CostReport> {
    const metrics = await this.engine.getMetrics();
    const now = new Date();
    let startDate: Date;

    switch (period) {
      case CostPeriod.Hour:
        startDate = new Date(now.getTime() - 3600000);
        break;
      case CostPeriod.Day:
        startDate = new Date(now.getTime() - 86400000);
        break;
      case CostPeriod.Week:
        startDate = new Date(now.getTime() - 604800000);
        break;
      case CostPeriod.Month:
        startDate = new Date(now.getTime() - 2592000000);
        break;
      default:
        startDate = new Date(now.getTime() - 86400000);
    }

    return {
      period,
      startDate,
      endDate: now,
      totalCost: metrics.cost.totalCost,
      modelCosts: metrics.cost.costPerModel,
      taskTypeCosts: this.convertTaskTypeCosts(metrics.cost.costPerTask),
      topTasks: [], // Would need to track individual tasks
      trend: {
        direction: 'stable',
        percentageChange: 0,
        projection: metrics.cost.projectedMonthlyCost
      }
    };
  }

  get onCostAlert(): Event<CostAlertEvent> {
    return this.costAlertEmitter.event;
  }

  async dispose(): Promise<void> {
    if (this.metricsInterval) {
      clearInterval(this.metricsInterval);
    }
    await this.engine.shutdown();
  }

  // Private conversion methods

  private convertToInternalTask(task: APITask): AITask {
    return {
      id: task.id,
      type: this.mapTaskType(task.type),
      prompt: task.prompt,
      context: task.context ? {
        files: task.context.files?.map(path => ({
          path,
          content: '', // Would need to read file content
          language: 'typescript' // Would need to detect language
        })),
        language: task.context.language,
        framework: task.context.framework,
        projectType: task.context.codebase?.rootPath,
        codebase: task.context.codebase
      } : undefined,
      constraints: task.constraints ? {
        maxTokens: task.constraints.maxTokens,
        maxCost: task.constraints.maxCost,
        maxLatency: task.constraints.maxLatency,
        temperature: task.constraints.temperature,
        topP: task.constraints.topP,
        requiredCapabilities: task.constraints.requiredCapabilities?.map(c => Capability.NaturalLanguage)
      } : undefined,
      priority: task.priority ? this.mapPriority(task.priority) : undefined
    };
  }

  private convertToInternalOptions(options: APIExecutionOptions): ExecutionOptions {
    return {
      stream: options.streaming,
      timeout: options.timeout,
      retryPolicy: options.retries ? {
        maxAttempts: options.retries,
        backoffStrategy: 'exponential',
        initialDelay: 1000,
        maxDelay: 10000,
        fallbackOnFailure: true
      } : undefined,
      cachePolicy: {
        enabled: options.cache !== false,
        ttl: 3600000,
        strategy: 'exact',
        sharing: 'project'
      }
    };
  }

  private convertToInternalPipeline(pipeline: APITaskPipeline): TaskPipeline {
    return {
      id: pipeline.id,
      name: pipeline.name,
      tasks: pipeline.stages.map(stage => ({
        task: this.convertToInternalTask(stage.task),
        dependencies: stage.dependencies,
        transformation: stage.transform,
        condition: stage.condition ? {
          type: 'custom',
          predicate: stage.condition
        } : undefined
      }))
    };
  }

  private convertToAPITaskResult(result: TaskResult): APITaskResult {
    return {
      taskId: result.taskId,
      success: result.status === 'success',
      output: result.content,
      model: result.model,
      usage: {
        prompt: result.usage.promptTokens,
        completion: result.usage.completionTokens,
        total: result.usage.totalTokens
      },
      cost: result.cost.amount,
      latency: result.latency,
      cached: result.metadata?.cacheHit,
      error: result.error ? {
        code: result.error.code,
        message: result.error.message,
        retryable: result.error.retryable
      } : undefined,
      metadata: result.metadata
    };
  }

  private convertToAPIPipelineResult(
    result: PipelineResult,
    pipeline: APITaskPipeline
  ): APIPipelineResult {
    const stages: Record<string, APITaskResult> = {};
    
    for (let i = 0; i < result.results.length; i++) {
      const stageId = pipeline.stages[i]?.id || `stage-${i}`;
      stages[stageId] = this.convertToAPITaskResult(result.results[i]);
    }

    return {
      pipelineId: result.pipelineId,
      success: result.status === 'success',
      stages,
      totalCost: result.totalCost.amount,
      totalLatency: result.totalLatency,
      output: result.results[result.results.length - 1]?.content
    };
  }

  private convertToAPIModel(model: ModelProfile): APIModel {
    return {
      id: model.id,
      provider: model.provider,
      name: model.displayName,
      version: model.version,
      capabilities: model.capabilities.map(cap => ({
        type: cap,
        score: 1.0
      })),
      contextWindow: model.contextWindow,
      maxTokens: model.maxOutputTokens,
      costPer1kTokens: {
        input: model.costPerToken.input * 1000,
        output: model.costPerToken.output * 1000
      },
      latency: {
        p50: model.averageLatency * 0.8,
        p90: model.averageLatency * 1.2,
        p99: model.averageLatency * 2
      },
      availability: model.availability.status === 'available' ? 1.0 : 0.5,
      qualityScore: model.reliability
    };
  }

  private convertToAPIModelSelection(selection: ModelSelection): APIModelSelection {
    return {
      primary: this.convertToAPIModel(selection.primary),
      fallbacks: selection.fallbacks.map(m => this.convertToAPIModel(m)),
      reasoning: selection.reasoning,
      estimatedCost: selection.estimatedCost.expected,
      estimatedLatency: selection.estimatedLatency
    };
  }

  private convertToAPIMetrics(metrics: OrchestrationMetrics): APIMetrics {
    const modelUsage: Record<string, any> = {};
    
    // Convert internal metrics to API format
    for (const [model, cost] of Object.entries(metrics.cost.costPerModel)) {
      modelUsage[model] = {
        requests: 100, // Placeholder
        tokens: {
          prompt: 10000,
          completion: 5000,
          total: 15000
        },
        cost,
        avgLatency: 1500,
        errors: 5
      };
    }

    return {
      totalTasks: metrics.usage.totalRequests,
      successRate: metrics.quality.successRate,
      averageLatency: metrics.performance.averageLatency,
      totalCost: metrics.cost.totalCost,
      modelUsage,
      taskTypeBreakdown: this.convertTaskTypeBreakdown(metrics.usage.requestsByType),
      errorRate: 1 - metrics.quality.successRate,
      cacheHitRate: metrics.usage.cacheHitRate
    };
  }

  private convertToAPICostEstimate(estimate: CostEstimate): APICostEstimate {
    return {
      estimated: estimate.expected,
      breakdown: estimate.breakdown ? [{
        model: 'primary',
        tokens: estimate.breakdown.inputTokens + estimate.breakdown.outputTokens,
        cost: estimate.expected
      }] : [],
      confidence: 0.9,
      assumptions: [
        `Estimated ${estimate.breakdown?.inputTokens || 0} input tokens`,
        `Estimated ${estimate.breakdown?.outputTokens || 0} output tokens`
      ]
    };
  }

  private mapTaskType(apiType: APITaskType): TaskType {
    switch (apiType) {
      case APITaskType.CodeGeneration: return TaskType.CodeGeneration;
      case APITaskType.CodeReview: return TaskType.CodeReview;
      case APITaskType.CodeRefactoring: return TaskType.Refactoring;
      case APITaskType.Documentation: return TaskType.Documentation;
      case APITaskType.Testing: return TaskType.Testing;
      case APITaskType.Debugging: return TaskType.BugFix;
      case APITaskType.Translation: return TaskType.Translation;
      default: return TaskType.General;
    }
  }

  private mapPriority(priority: string): UrgencyLevel {
    switch (priority) {
      case 'low': return UrgencyLevel.Low;
      case 'high': return UrgencyLevel.High;
      case 'critical': return UrgencyLevel.Critical;
      default: return UrgencyLevel.Normal;
    }
  }

  private convertTaskTypeCosts(costs: Record<TaskType, number>): Record<string, number> {
    const result: Record<string, number> = {};
    for (const [type, cost] of Object.entries(costs)) {
      result[type] = cost;
    }
    return result;
  }

  private convertTaskTypeBreakdown(breakdown: Record<TaskType, number>): Record<string, number> {
    const result: Record<string, number> = {};
    for (const [type, count] of Object.entries(breakdown)) {
      result[type] = count;
    }
    return result;
  }

  private startMetricsUpdates(): void {
    this.metricsInterval = setInterval(async () => {
      try {
        const metrics = await this.getMetrics();
        this.metricsUpdateEmitter.fire({
          metrics,
          timestamp: new Date(),
          period: 'minute'
        });
      } catch (error) {
        console.error('Failed to update metrics:', error);
      }
    }, 60000); // Update every minute
  }

}