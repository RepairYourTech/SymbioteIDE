/**
 * Routing Engine - Intelligently routes tasks to optimal AI models
 */

import {
  AITask,
  TaskAnalysis,
  ModelProfile,
  ModelSelection,
  TaskType,
  ComplexityLevel,
  QualityLevel,
  CostEstimate,
  TaskConstraints,
  ProviderType
} from './interfaces';
import { ModelRegistry } from './model-registry';
import { TaskAnalyzer } from './task-analyzer';

export interface RoutingConfig {
  enableCostOptimization: boolean;
  enableLoadBalancing: boolean;
  preferLocalModels: boolean;
  maxFallbacks: number;
  scoringWeights: ScoringWeights;
}

export interface ScoringWeights {
  capability: number;
  cost: number;
  performance: number;
  quality: number;
  availability: number;
}

export class RoutingEngine {
  private config: RoutingConfig = {
    enableCostOptimization: true,
    enableLoadBalancing: true,
    preferLocalModels: false,
    maxFallbacks: 3,
    scoringWeights: {
      capability: 0.3,
      cost: 0.2,
      performance: 0.2,
      quality: 0.2,
      availability: 0.1
    }
  };

  constructor(
    private modelRegistry: ModelRegistry,
    private taskAnalyzer: TaskAnalyzer,
    config?: Partial<RoutingConfig>
  ) {
    if (config) {
      this.config = { ...this.config, ...config };
    }
  }

  /**
   * Route a task to the optimal model
   */
  async routeTask(task: AITask): Promise<ModelSelection> {
    // Analyze the task
    const analysis = await this.taskAnalyzer.analyzeTask(task);
    
    // Get candidate models
    const candidateModels = this.getCandidateModels(analysis, task.constraints);
    
    if (candidateModels.length === 0) {
      throw new Error('No suitable models available for this task');
    }
    
    // Score and rank models
    const scoredModels = this.scoreModels(candidateModels, analysis, task.constraints);
    
    // Select primary and fallback models
    const primary = scoredModels[0];
    const fallbacks = scoredModels
      .slice(1, this.config.maxFallbacks + 1)
      .map(sm => sm.model);
    
    // Calculate cost estimate
    const estimatedCost = this.estimateCost(primary.model, analysis);
    
    // Generate reasoning
    const reasoning = this.generateRoutingReason(primary, analysis, task);
    
    // Check for warnings
    const warnings = this.checkForWarnings(primary.model, analysis, task.constraints);
    
    return {
      primary: primary.model,
      fallbacks,
      reasoning,
      estimatedCost,
      estimatedLatency: primary.model.averageLatency,
      confidence: primary.score * analysis.confidence,
      warnings
    };
  }

  /**
   * Get candidate models based on requirements
   */
  private getCandidateModels(
    analysis: TaskAnalysis,
    constraints?: TaskConstraints
  ): ModelProfile[] {
    // Start with models that support the task type
    let candidates = this.modelRegistry.getModelsForTaskType(analysis.type);
    
    // Filter by required capabilities
    candidates = candidates.filter(model =>
      analysis.requiredCapabilities.every(cap => 
        model.capabilities.includes(cap)
      )
    );
    
    // Filter by context window
    candidates = candidates.filter(model =>
      model.contextWindow >= analysis.contextWindow
    );
    
    // Apply constraints
    if (constraints) {
      // Exclude specific providers
      if (constraints.excludeProviders) {
        candidates = candidates.filter(model =>
          !constraints.excludeProviders!.includes(model.provider)
        );
      }
      
      // Filter by max cost
      if (constraints.maxCost !== undefined) {
        candidates = candidates.filter(model => {
          const estimatedCost = this.estimateCost(model, analysis);
          return estimatedCost.expected <= constraints.maxCost!;
        });
      }
      
      // Filter by max latency
      if (constraints.maxLatency !== undefined) {
        candidates = candidates.filter(model =>
          model.averageLatency <= constraints.maxLatency!
        );
      }
    }
    
    // Prefer local models if configured
    if (this.config.preferLocalModels) {
      const localModels = candidates.filter(m => m.provider === ProviderType.Local);
      if (localModels.length > 0) {
        return localModels;
      }
    }
    
    return candidates;
  }

  /**
   * Score models based on multiple factors
   */
  private scoreModels(
    models: ModelProfile[],
    analysis: TaskAnalysis,
    constraints?: TaskConstraints
  ): Array<{ model: ModelProfile; score: number; breakdown: Record<string, number> }> {
    return models
      .map(model => {
        const breakdown: Record<string, number> = {};
        
        // Capability score (0-1)
        breakdown.capability = this.calculateCapabilityScore(model, analysis);
        
        // Cost score (0-1, lower cost = higher score)
        breakdown.cost = this.calculateCostScore(model, analysis, constraints);
        
        // Performance score (0-1, lower latency = higher score)
        breakdown.performance = this.calculatePerformanceScore(model, analysis, constraints);
        
        // Quality score (0-1)
        breakdown.quality = this.calculateQualityScore(model, analysis);
        
        // Availability score (0-1)
        breakdown.availability = this.calculateAvailabilityScore(model);
        
        // Calculate weighted total
        const weights = this.config.scoringWeights;
        const totalScore = Object.entries(breakdown).reduce((sum, [factor, score]) => {
          return sum + score * (weights[factor as keyof ScoringWeights] || 0);
        }, 0);
        
        return { model, score: totalScore, breakdown };
      })
      .sort((a, b) => b.score - a.score);
  }

  /**
   * Calculate capability score
   */
  private calculateCapabilityScore(model: ModelProfile, analysis: TaskAnalysis): number {
    let score = 0.5; // Base score
    
    // Bonus for having all required capabilities
    const hasAllCapabilities = analysis.requiredCapabilities.every(cap =>
      model.capabilities.includes(cap)
    );
    if (hasAllCapabilities) score += 0.2;
    
    // Bonus for specialization
    if (model.metadata?.preferredForTasks?.includes(analysis.type)) {
      score += 0.3;
    } else if (model.specializations.some(spec => 
      analysis.type.toLowerCase().includes(spec.toLowerCase())
    )) {
      score += 0.2;
    }
    
    return Math.min(1, score);
  }

  /**
   * Calculate cost score
   */
  private calculateCostScore(
    model: ModelProfile,
    analysis: TaskAnalysis,
    constraints?: TaskConstraints
  ): number {
    const estimatedCost = this.estimateCost(model, analysis);
    const costPerToken = (model.costPerToken.input + model.costPerToken.output) / 2;
    
    // Normalize cost (assuming $0.10 is very expensive)
    let score = 1 - (estimatedCost.expected / 0.10);
    score = Math.max(0, Math.min(1, score));
    
    // Apply budget constraint pressure
    if (constraints?.maxCost !== undefined) {
      const budgetUtilization = estimatedCost.expected / constraints.maxCost;
      if (budgetUtilization > 0.8) {
        score *= 0.5; // Penalize models close to budget limit
      }
    }
    
    // Bonus for free models
    if (costPerToken === 0) {
      score = 1;
    }
    
    return score;
  }

  /**
   * Calculate performance score
   */
  private calculatePerformanceScore(
    model: ModelProfile,
    analysis: TaskAnalysis,
    constraints?: TaskConstraints
  ): number {
    // Normalize latency (assuming 10s is very slow)
    let score = 1 - (model.averageLatency / 10000);
    score = Math.max(0, Math.min(1, score));
    
    // Consider current load if available
    if (model.currentLoad !== undefined) {
      const loadFactor = 1 - (model.currentLoad / 100);
      score *= loadFactor;
    }
    
    // Apply latency constraint pressure
    if (constraints?.maxLatency !== undefined) {
      const latencyUtilization = model.averageLatency / constraints.maxLatency;
      if (latencyUtilization > 0.8) {
        score *= 0.5; // Penalize models close to latency limit
      }
    }
    
    // Bonus for low-latency requirements
    if (analysis.urgency === 'critical' && model.averageLatency < 1000) {
      score *= 1.2;
    }
    
    return Math.min(1, score);
  }

  /**
   * Calculate quality score
   */
  private calculateQualityScore(model: ModelProfile, analysis: TaskAnalysis): number {
    let score = model.reliability; // Start with reliability
    
    // Adjust based on model tier and quality requirements
    const modelTier = this.getModelTier(model);
    const requiredQuality = analysis.qualityRequirement;
    
    if (modelTier === 'premium' && requiredQuality === QualityLevel.Premium) {
      score *= 1.2;
    } else if (modelTier === 'standard' && requiredQuality === QualityLevel.Premium) {
      score *= 0.8;
    } else if (modelTier === 'budget' && requiredQuality !== QualityLevel.Draft) {
      score *= 0.6;
    }
    
    // Consider success rate if available
    if (model.metadata?.successRate) {
      score = (score + model.metadata.successRate) / 2;
    }
    
    return Math.min(1, score);
  }

  /**
   * Calculate availability score
   */
  private calculateAvailabilityScore(model: ModelProfile): number {
    switch (model.availability.status) {
      case 'available':
        return 1.0;
      case 'degraded':
        return 0.5;
      case 'unavailable':
        return 0;
      default:
        return 0.8;
    }
  }

  /**
   * Get model tier based on cost and capabilities
   */
  private getModelTier(model: ModelProfile): 'premium' | 'standard' | 'budget' {
    const avgCost = (model.costPerToken.input + model.costPerToken.output) / 2;
    
    if (avgCost > 0.00001) return 'premium';
    if (avgCost > 0.000001) return 'standard';
    return 'budget';
  }

  /**
   * Estimate cost for a model and task
   */
  private estimateCost(model: ModelProfile, analysis: TaskAnalysis): CostEstimate {
    const inputTokens = analysis.estimatedTokens * 0.6; // Assume 60% input
    const outputTokens = analysis.estimatedTokens * 0.4; // Assume 40% output
    
    const inputCost = inputTokens * model.costPerToken.input;
    const outputCost = outputTokens * model.costPerToken.output;
    const totalCost = inputCost + outputCost;
    
    return {
      minimum: totalCost * 0.8, // 20% lower bound
      expected: totalCost,
      maximum: totalCost * 1.5, // 50% upper bound
      currency: model.costPerToken.currency,
      breakdown: {
        inputTokens,
        outputTokens,
        inputCost,
        outputCost
      }
    };
  }

  /**
   * Generate human-readable routing reasoning
   */
  private generateRoutingReason(
    selection: { model: ModelProfile; score: number; breakdown: Record<string, number> },
    analysis: TaskAnalysis,
    task: AITask
  ): string {
    const reasons: string[] = [];
    
    // Primary reason based on highest scoring factor
    const topFactor = Object.entries(selection.breakdown)
      .sort((a, b) => b[1] - a[1])[0][0];
    
    switch (topFactor) {
      case 'capability':
        reasons.push(`${selection.model.displayName} has excellent support for ${analysis.type} tasks`);
        break;
      case 'cost':
        reasons.push(`${selection.model.displayName} offers the best cost-efficiency for this task`);
        break;
      case 'performance':
        reasons.push(`${selection.model.displayName} provides optimal performance with ${selection.model.averageLatency}ms average latency`);
        break;
      case 'quality':
        reasons.push(`${selection.model.displayName} delivers the required ${analysis.qualityRequirement} quality level`);
        break;
      case 'availability':
        reasons.push(`${selection.model.displayName} is readily available with high reliability`);
        break;
    }
    
    // Add task-specific reasoning
    if (analysis.complexity === ComplexityLevel.Expert) {
      reasons.push('Selected a premium model due to high task complexity');
    }
    
    if (task.constraints?.maxCost && selection.breakdown.cost > 0.8) {
      reasons.push('Optimized for cost constraints while maintaining quality');
    }
    
    if (analysis.estimatedTokens > 50000) {
      reasons.push('Model supports the large context window required');
    }
    
    return reasons.join('. ');
  }

  /**
   * Check for potential warnings
   */
  private checkForWarnings(
    model: ModelProfile,
    analysis: TaskAnalysis,
    constraints?: TaskConstraints
  ): string[] {
    const warnings: string[] = [];
    
    // Cost warnings
    const estimatedCost = this.estimateCost(model, analysis);
    if (constraints?.maxCost && estimatedCost.maximum > constraints.maxCost) {
      warnings.push(`Cost may exceed budget limit in worst case (up to $${estimatedCost.maximum.toFixed(4)})`);
    }
    
    // Performance warnings
    if (constraints?.maxLatency && model.averageLatency > constraints.maxLatency * 0.8) {
      warnings.push(`Latency approaching constraint limit (${model.averageLatency}ms average)`);
    }
    
    // Availability warnings
    if (model.availability.status === 'degraded') {
      warnings.push('Model is currently experiencing degraded performance');
    }
    
    // Context warnings
    if (analysis.estimatedTokens > model.contextWindow * 0.9) {
      warnings.push('Task approaching model context limit');
    }
    
    // Quality mismatch warnings
    const modelTier = this.getModelTier(model);
    if (analysis.qualityRequirement === QualityLevel.Premium && modelTier === 'budget') {
      warnings.push('Selected model may not meet premium quality requirements');
    }
    
    return warnings;
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<RoutingConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get routing statistics
   */
  getRoutingStats(): {
    availableModels: number;
    providerDistribution: Record<string, number>;
    averageCostPerToken: number;
  } {
    const models = this.modelRegistry.getAvailableModels();
    const providerDist: Record<string, number> = {};
    let totalCost = 0;
    
    for (const model of models) {
      providerDist[model.provider] = (providerDist[model.provider] || 0) + 1;
      totalCost += (model.costPerToken.input + model.costPerToken.output) / 2;
    }
    
    return {
      availableModels: models.length,
      providerDistribution: providerDist,
      averageCostPerToken: totalCost / models.length
    };
  }
}