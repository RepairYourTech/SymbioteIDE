/**
 * A/B Testing Experiment Manager
 */

import { EventEmitter } from 'events';
import * as crypto from 'crypto';
import {
  ExperimentConfig,
  ExperimentStatus,
  ExperimentVariant,
  ExperimentResult,
  ExperimentSample,
  ExperimentEvent,
  ExperimentEventType,
  AllocationMethod,
  VariantResult,
  MetricResult,
  StatisticalAnalysis
} from './experiment';
import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { ModelRegistry } from '../model-registry';
import { MetricsCollector } from './metrics-collector';
import { StatisticalAnalyzer } from './statistical-analyzer';
import { TrafficSplitter } from './traffic-splitter';

export class ExperimentManager extends EventEmitter {
  private experiments: Map<string, ExperimentConfig> = new Map();
  private samples: Map<string, ExperimentSample[]> = new Map();
  private metricsCollector: MetricsCollector;
  private statisticalAnalyzer: StatisticalAnalyzer;
  private trafficSplitter: TrafficSplitter;
  private modelRegistry: ModelRegistry;

  constructor(modelRegistry: ModelRegistry) {
    super();
    this.modelRegistry = modelRegistry;
    this.metricsCollector = new MetricsCollector();
    this.statisticalAnalyzer = new StatisticalAnalyzer();
    this.trafficSplitter = new TrafficSplitter();
  }

  /**
   * Create a new experiment
   */
  createExperiment(config: Omit<ExperimentConfig, 'id' | 'status'>): ExperimentConfig {
    const experiment: ExperimentConfig = {
      ...config,
      id: this.generateExperimentId(),
      status: ExperimentStatus.Draft,
      startTime: config.startTime || new Date()
    };

    // Validate experiment configuration
    this.validateExperiment(experiment);

    this.experiments.set(experiment.id, experiment);
    this.samples.set(experiment.id, []);

    this.emitEvent({
      type: ExperimentEventType.Started,
      experimentId: experiment.id,
      timestamp: new Date(),
      data: experiment
    });

    return experiment;
  }

  /**
   * Start an experiment
   */
  startExperiment(experimentId: string): void {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status !== ExperimentStatus.Draft) {
      throw new Error(`Experiment ${experimentId} is not in draft status`);
    }

    experiment.status = ExperimentStatus.Running;
    experiment.startTime = new Date();

    this.emitEvent({
      type: ExperimentEventType.Started,
      experimentId,
      timestamp: new Date(),
      data: { status: ExperimentStatus.Running }
    });
  }

  /**
   * Pause an experiment
   */
  pauseExperiment(experimentId: string): void {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status !== ExperimentStatus.Running) {
      throw new Error(`Experiment ${experimentId} is not running`);
    }

    experiment.status = ExperimentStatus.Paused;

    this.emitEvent({
      type: ExperimentEventType.Paused,
      experimentId,
      timestamp: new Date(),
      data: { status: ExperimentStatus.Paused }
    });
  }

  /**
   * Resume an experiment
   */
  resumeExperiment(experimentId: string): void {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status !== ExperimentStatus.Paused) {
      throw new Error(`Experiment ${experimentId} is not paused`);
    }

    experiment.status = ExperimentStatus.Running;

    this.emitEvent({
      type: ExperimentEventType.Resumed,
      experimentId,
      timestamp: new Date(),
      data: { status: ExperimentStatus.Running }
    });
  }

  /**
   * Complete an experiment
   */
  completeExperiment(experimentId: string): ExperimentResult {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status === ExperimentStatus.Completed) {
      throw new Error(`Experiment ${experimentId} is already completed`);
    }

    experiment.status = ExperimentStatus.Completed;
    experiment.endTime = new Date();

    const result = this.analyzeExperiment(experimentId);

    this.emitEvent({
      type: ExperimentEventType.Completed,
      experimentId,
      timestamp: new Date(),
      data: result
    });

    return result;
  }

  /**
   * Get variant assignment for a task
   */
  getVariantForTask(
    experimentId: string,
    task: AITask,
    context?: {
      userId?: string;
      sessionId?: string;
      metadata?: Record<string, any>;
    }
  ): ExperimentVariant | null {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status !== ExperimentStatus.Running) {
      return null;
    }

    // Check if experiment has reached constraints
    if (this.hasReachedConstraints(experiment)) {
      this.completeExperiment(experimentId);
      return null;
    }

    // Get variant assignment
    const variant = this.trafficSplitter.assignVariant(
      experiment,
      task,
      context
    );

    return variant;
  }

  /**
   * Record experiment sample
   */
  recordSample(
    experimentId: string,
    variantId: string,
    task: AITask,
    result: TaskResult,
    context?: Record<string, any>
  ): void {
    const experiment = this.getExperiment(experimentId);
    const samples = this.samples.get(experimentId) || [];

    // Collect metrics
    const metrics = this.metricsCollector.collectMetrics(
      task,
      result,
      experiment.metrics
    );

    const sample: ExperimentSample = {
      id: this.generateSampleId(),
      experimentId,
      variantId,
      task,
      result,
      metrics,
      timestamp: new Date(),
      userId: context?.userId,
      sessionId: context?.sessionId,
      metadata: context
    };

    samples.push(sample);
    this.samples.set(experimentId, samples);

    this.emitEvent({
      type: ExperimentEventType.SampleCollected,
      experimentId,
      timestamp: new Date(),
      data: sample
    });

    // Check early stopping rules
    if (experiment.constraints?.earlyStoppingRules) {
      this.checkEarlyStoppingRules(experiment, samples);
    }
  }

  /**
   * Analyze experiment results
   */
  analyzeExperiment(experimentId: string): ExperimentResult {
    const experiment = this.getExperiment(experimentId);
    const samples = this.samples.get(experimentId) || [];

    // Group samples by variant
    const variantSamples = new Map<string, ExperimentSample[]>();
    for (const sample of samples) {
      const existing = variantSamples.get(sample.variantId) || [];
      existing.push(sample);
      variantSamples.set(sample.variantId, existing);
    }

    // Calculate metrics for each variant
    const variantResults: VariantResult[] = [];
    for (const variant of experiment.variants) {
      const samples = variantSamples.get(variant.id) || [];
      const metricResults = this.metricsCollector.aggregateMetrics(
        samples,
        experiment.metrics
      );

      const errors = samples.filter(s => s.result.status === 'failed').length;

      variantResults.push({
        variantId: variant.id,
        variantName: variant.name,
        sampleSize: samples.length,
        metrics: metricResults,
        errors,
        errorRate: samples.length > 0 ? errors / samples.length : 0
      });
    }

    // Perform statistical analysis
    const statisticalAnalysis = this.statisticalAnalyzer.analyze(
      variantResults,
      experiment.metrics
    );

    // Calculate overall metrics
    const overallMetrics = this.metricsCollector.aggregateMetrics(
      samples,
      experiment.metrics
    );

    return {
      experimentId: experiment.id,
      status: experiment.status,
      startTime: experiment.startTime,
      endTime: experiment.endTime,
      variants: variantResults,
      overallMetrics,
      statisticalAnalysis,
      recommendation: this.generateRecommendation(
        variantResults,
        statisticalAnalysis
      )
    };
  }

  /**
   * Get running experiments for a task
   */
  getRunningExperiments(task: AITask): ExperimentConfig[] {
    const running: ExperimentConfig[] = [];
    
    for (const experiment of this.experiments.values()) {
      if (experiment.status !== ExperimentStatus.Running) {
        continue;
      }

      // Check task type filters
      if (experiment.trafficAllocation.taskTypeFilters) {
        if (!experiment.trafficAllocation.taskTypeFilters.includes(task.type)) {
          continue;
        }
      }

      running.push(experiment);
    }

    return running;
  }

  /**
   * Get experiment by ID
   */
  getExperiment(experimentId: string): ExperimentConfig {
    const experiment = this.experiments.get(experimentId);
    if (!experiment) {
      throw new Error(`Experiment ${experimentId} not found`);
    }
    return experiment;
  }

  /**
   * List all experiments
   */
  listExperiments(filter?: {
    status?: ExperimentStatus;
    startTime?: Date;
    endTime?: Date;
  }): ExperimentConfig[] {
    let experiments = Array.from(this.experiments.values());

    if (filter) {
      if (filter.status) {
        experiments = experiments.filter(e => e.status === filter.status);
      }
      if (filter.startTime) {
        experiments = experiments.filter(e => e.startTime >= filter.startTime!);
      }
      if (filter.endTime) {
        experiments = experiments.filter(e => 
          e.endTime && e.endTime <= filter.endTime!
        );
      }
    }

    return experiments;
  }

  /**
   * Update experiment configuration
   */
  updateExperiment(
    experimentId: string,
    updates: Partial<ExperimentConfig>
  ): ExperimentConfig {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status === ExperimentStatus.Completed) {
      throw new Error('Cannot update completed experiment');
    }

    // Apply updates
    Object.assign(experiment, updates);
    
    // Revalidate
    this.validateExperiment(experiment);

    this.emitEvent({
      type: ExperimentEventType.TrafficUpdated,
      experimentId,
      timestamp: new Date(),
      data: updates
    });

    return experiment;
  }

  /**
   * Delete an experiment
   */
  deleteExperiment(experimentId: string): void {
    const experiment = this.getExperiment(experimentId);
    
    if (experiment.status === ExperimentStatus.Running) {
      throw new Error('Cannot delete running experiment');
    }

    this.experiments.delete(experimentId);
    this.samples.delete(experimentId);
  }

  // Private methods

  private validateExperiment(experiment: ExperimentConfig): void {
    // Validate variants
    if (experiment.variants.length < 2) {
      throw new Error('Experiment must have at least 2 variants');
    }

    // Validate weights sum to 100
    const totalWeight = experiment.variants.reduce((sum, v) => sum + v.weight, 0);
    if (Math.abs(totalWeight - 100) > 0.01) {
      throw new Error('Variant weights must sum to 100');
    }

    // Validate model IDs
    for (const variant of experiment.variants) {
      const model = this.modelRegistry.getModel(variant.modelId);
      if (!model) {
        throw new Error(`Model ${variant.modelId} not found`);
      }
    }

    // Validate metrics
    if (experiment.metrics.length === 0) {
      throw new Error('Experiment must have at least one metric');
    }
  }

  private hasReachedConstraints(experiment: ExperimentConfig): boolean {
    if (!experiment.constraints) {
      return false;
    }

    const samples = this.samples.get(experiment.id) || [];

    // Check max samples
    if (experiment.constraints.maxSamples) {
      if (samples.length >= experiment.constraints.maxSamples) {
        return true;
      }
    }

    // Check max duration
    if (experiment.constraints.maxDuration) {
      const duration = Date.now() - experiment.startTime.getTime();
      if (duration >= experiment.constraints.maxDuration) {
        return true;
      }
    }

    // Check max cost
    if (experiment.constraints.maxCost) {
      const totalCost = samples.reduce((sum, s) => sum + s.result.cost.amount, 0);
      if (totalCost >= experiment.constraints.maxCost) {
        return true;
      }
    }

    return false;
  }

  private checkEarlyStoppingRules(
    experiment: ExperimentConfig,
    samples: ExperimentSample[]
  ): void {
    if (!experiment.constraints?.earlyStoppingRules) {
      return;
    }

    for (const rule of experiment.constraints.earlyStoppingRules) {
      // TODO: Implement early stopping logic
      // This would analyze current metrics and determine if the experiment
      // should be stopped early based on the rule conditions
    }
  }

  private generateRecommendation(
    variants: VariantResult[],
    analysis?: StatisticalAnalysis
  ): string {
    if (!analysis || !analysis.winner) {
      return 'Insufficient data to make a recommendation';
    }

    const winner = variants.find(v => v.variantId === analysis.winner);
    if (!winner) {
      return 'No clear winner identified';
    }

    const confidence = analysis.confidence || 0;
    const effectSize = analysis.effectSize || 0;

    let recommendation = `Variant "${winner.variantName}" is recommended`;

    if (confidence >= 0.95) {
      recommendation += ' with high confidence';
    } else if (confidence >= 0.90) {
      recommendation += ' with moderate confidence';
    } else {
      recommendation += ' but needs more data for higher confidence';
    }

    if (effectSize > 0.8) {
      recommendation += ' (large effect size)';
    } else if (effectSize > 0.5) {
      recommendation += ' (medium effect size)';
    } else if (effectSize > 0.2) {
      recommendation += ' (small effect size)';
    }

    return recommendation;
  }

  private generateExperimentId(): string {
    return `exp-${Date.now()}-${crypto.randomBytes(4).toString('hex')}`;
  }

  private generateSampleId(): string {
    return `sample-${Date.now()}-${crypto.randomBytes(4).toString('hex')}`;
  }

  private emitEvent(event: ExperimentEvent): void {
    this.emit('experiment-event', event);
  }
}