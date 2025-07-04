/**
 * A/B Testing Integration with Orchestration Engine
 */

import { EventEmitter } from 'events';
import {
  AITask,
  TaskResult,
  ModelSelection,
  ModelProfile,
  ExecutionOptions
} from '../interfaces';
import { OrchestrationEngine } from '../orchestration-engine';
import { ExperimentManager } from './experiment-manager';
import {
  ExperimentConfig,
  ExperimentVariant,
  ExperimentStatus,
  AllocationMethod,
  MetricType,
  AggregationType
} from './experiment';

export interface ABTestingOptions extends ExecutionOptions {
  experimentId?: string;
  skipExperiments?: boolean;
  userId?: string;
  sessionId?: string;
}

export class ABTestingIntegration extends EventEmitter {
  private experimentManager: ExperimentManager;
  private originalEngine: OrchestrationEngine;
  private experimentOverrides: Map<string, string> = new Map();

  constructor(
    orchestrationEngine: OrchestrationEngine,
    experimentManager: ExperimentManager
  ) {
    super();
    this.originalEngine = orchestrationEngine;
    this.experimentManager = experimentManager;
  }

  /**
   * Execute task with A/B testing
   */
  async executeTask(
    task: AITask,
    options?: ABTestingOptions
  ): Promise<TaskResult> {
    // Skip experiments if requested
    if (options?.skipExperiments) {
      return this.originalEngine.executeTask(task, options);
    }

    // Check for running experiments
    const experiments = this.experimentManager.getRunningExperiments(task);
    
    if (experiments.length === 0) {
      return this.originalEngine.executeTask(task, options);
    }

    // Get experiment assignment
    const experiment = options?.experimentId 
      ? experiments.find(e => e.id === options.experimentId)
      : experiments[0];

    if (!experiment) {
      return this.originalEngine.executeTask(task, options);
    }

    // Get variant assignment
    const variant = this.experimentManager.getVariantForTask(
      experiment.id,
      task,
      {
        userId: options?.userId,
        sessionId: options?.sessionId || task.sessionId,
        metadata: task.metadata
      }
    );

    if (!variant) {
      return this.originalEngine.executeTask(task, options);
    }

    // Execute with variant model
    const result = await this.executeWithVariant(task, variant, options);

    // Record sample
    this.experimentManager.recordSample(
      experiment.id,
      variant.id,
      task,
      result,
      {
        userId: options?.userId,
        sessionId: options?.sessionId,
        ...task.metadata
      }
    );

    return result;
  }

  /**
   * Execute task with specific variant
   */
  private async executeWithVariant(
    task: AITask,
    variant: ExperimentVariant,
    options?: ABTestingOptions
  ): Promise<TaskResult> {
    // Override model selection
    this.experimentOverrides.set(task.id, variant.modelId);

    try {
      // Apply variant configuration
      const modifiedOptions: ExecutionOptions = {
        ...options
      };

      if (variant.modelConfig) {
        task.constraints = {
          ...task.constraints,
          temperature: variant.modelConfig.temperature ?? task.constraints?.temperature,
          topP: variant.modelConfig.topP ?? task.constraints?.topP,
          maxTokens: variant.modelConfig.maxTokens ?? task.constraints?.maxTokens
        };

        if (variant.modelConfig.systemPrompt) {
          task.context = {
            ...task.context,
            customContext: {
              ...task.context?.customContext,
              systemPrompt: variant.modelConfig.systemPrompt
            }
          };
        }
      }

      const result = await this.originalEngine.executeTask(task, modifiedOptions);
      
      // Add variant information to metadata
      result.metadata = {
        ...result.metadata,
        experimentVariant: variant.id,
        experimentModel: variant.modelId
      };

      return result;
    } finally {
      this.experimentOverrides.delete(task.id);
    }
  }

  /**
   * Override model selection for experiments
   */
  async selectModel(task: AITask): Promise<ModelSelection> {
    const override = this.experimentOverrides.get(task.id);
    
    if (override) {
      const models = await this.originalEngine.listModels();
      const model = models.find(m => m.id === override);
      
      if (model) {
        return {
          primary: model,
          fallbacks: [],
          reasoning: `Overridden by A/B test experiment`,
          estimatedCost: {
            minimum: 0,
            expected: 0.001,
            maximum: 0.01,
            currency: 'USD'
          },
          estimatedLatency: model.averageLatency,
          confidence: 1.0
        };
      }
    }

    return this.originalEngine.selectModel(task);
  }

  /**
   * Create a model comparison experiment
   */
  async createModelComparison(config: {
    name: string;
    description: string;
    baselineModel: string;
    challengerModels: string[];
    trafficSplit?: number[]; // Weights for each model
    taskTypes?: string[];
    duration?: number;
    sampleSize?: number;
  }): Promise<ExperimentConfig> {
    const models = await this.originalEngine.listModels();
    const variants: ExperimentVariant[] = [];

    // Add baseline variant
    const baselineModel = models.find(m => m.id === config.baselineModel);
    if (!baselineModel) {
      throw new Error(`Baseline model ${config.baselineModel} not found`);
    }

    const weights = config.trafficSplit || 
      new Array(config.challengerModels.length + 1).fill(100 / (config.challengerModels.length + 1));

    variants.push({
      id: 'baseline',
      name: `Baseline (${baselineModel.displayName})`,
      modelId: config.baselineModel,
      weight: weights[0]
    });

    // Add challenger variants
    for (let i = 0; i < config.challengerModels.length; i++) {
      const modelId = config.challengerModels[i];
      const model = models.find(m => m.id === modelId);
      
      if (!model) {
        throw new Error(`Challenger model ${modelId} not found`);
      }

      variants.push({
        id: `challenger-${i + 1}`,
        name: `Challenger ${i + 1} (${model.displayName})`,
        modelId,
        weight: weights[i + 1]
      });
    }

    // Create experiment
    return this.experimentManager.createExperiment({
      name: config.name,
      description: config.description,
      startTime: new Date(),
      endTime: config.duration ? new Date(Date.now() + config.duration) : undefined,
      variants,
      trafficAllocation: {
        method: AllocationMethod.Random,
        taskTypeFilters: config.taskTypes as any
      },
      metrics: [
        {
          id: 'latency',
          name: 'Response Latency',
          type: MetricType.Latency,
          aggregation: AggregationType.Average,
          unit: 'ms',
          higherIsBetter: false,
          minimumSampleSize: 30,
          confidenceLevel: 0.95
        },
        {
          id: 'cost',
          name: 'Cost per Request',
          type: MetricType.Cost,
          aggregation: AggregationType.Average,
          unit: 'USD',
          higherIsBetter: false,
          minimumSampleSize: 30,
          confidenceLevel: 0.95
        },
        {
          id: 'success_rate',
          name: 'Success Rate',
          type: MetricType.SuccessRate,
          aggregation: AggregationType.Rate,
          unit: '%',
          higherIsBetter: true,
          minimumSampleSize: 30,
          confidenceLevel: 0.95
        },
        {
          id: 'tokens',
          name: 'Token Usage',
          type: MetricType.TokenUsage,
          aggregation: AggregationType.Average,
          unit: 'tokens',
          higherIsBetter: false,
          minimumSampleSize: 30,
          confidenceLevel: 0.95
        }
      ],
      constraints: config.sampleSize ? {
        requiredSampleSize: config.sampleSize,
        stopOnSignificance: true
      } : undefined,
      metadata: {
        isPrimary: 'latency' // Primary metric for winner determination
      }
    });
  }

  /**
   * Create a prompt variation experiment
   */
  async createPromptExperiment(config: {
    name: string;
    description: string;
    modelId: string;
    baselinePrompt: string;
    promptVariations: Array<{
      name: string;
      systemPrompt?: string;
      temperature?: number;
      topP?: number;
    }>;
    sampleSize?: number;
  }): Promise<ExperimentConfig> {
    const variants: ExperimentVariant[] = [];
    const weight = 100 / (config.promptVariations.length + 1);

    // Add baseline variant
    variants.push({
      id: 'baseline',
      name: 'Baseline Prompt',
      modelId: config.modelId,
      weight,
      modelConfig: {
        systemPrompt: config.baselinePrompt
      }
    });

    // Add variation variants
    for (let i = 0; i < config.promptVariations.length; i++) {
      const variation = config.promptVariations[i];
      variants.push({
        id: `variation-${i + 1}`,
        name: variation.name,
        modelId: config.modelId,
        weight,
        modelConfig: {
          systemPrompt: variation.systemPrompt,
          temperature: variation.temperature,
          topP: variation.topP
        }
      });
    }

    return this.experimentManager.createExperiment({
      name: config.name,
      description: config.description,
      startTime: new Date(),
      variants,
      trafficAllocation: {
        method: AllocationMethod.Random
      },
      metrics: [
        {
          id: 'quality',
          name: 'Output Quality',
          type: MetricType.OutputQuality,
          aggregation: AggregationType.Average,
          unit: 'score',
          higherIsBetter: true,
          minimumSampleSize: 30,
          confidenceLevel: 0.95,
          metadata: { isPrimary: true }
        },
        {
          id: 'latency',
          name: 'Response Latency',
          type: MetricType.Latency,
          aggregation: AggregationType.Average,
          unit: 'ms',
          higherIsBetter: false
        },
        {
          id: 'tokens',
          name: 'Token Usage',
          type: MetricType.TokenUsage,
          aggregation: AggregationType.Average,
          unit: 'tokens',
          higherIsBetter: false
        }
      ],
      constraints: config.sampleSize ? {
        requiredSampleSize: config.sampleSize
      } : undefined
    });
  }

  /**
   * Get experiment results
   */
  getExperimentResults(experimentId: string) {
    return this.experimentManager.analyzeExperiment(experimentId);
  }

  /**
   * List experiments
   */
  listExperiments(filter?: { status?: ExperimentStatus }) {
    return this.experimentManager.listExperiments(filter);
  }

  /**
   * Start experiment
   */
  startExperiment(experimentId: string) {
    return this.experimentManager.startExperiment(experimentId);
  }

  /**
   * Complete experiment
   */
  completeExperiment(experimentId: string) {
    return this.experimentManager.completeExperiment(experimentId);
  }
}