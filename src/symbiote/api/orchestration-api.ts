/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

import { Event } from 'vscode';

export interface IOrchestrationAPI {
  // Model selection and routing
  selectModel(task: AITask): Promise<ModelSelection>;
  routeTask(task: AITask): Promise<TaskRoute>;
  
  // Task execution
  executeTask(task: AITask, options?: ExecutionOptions): Promise<TaskResult>;
  executeParallel(tasks: AITask[]): Promise<TaskResult[]>;
  executePipeline(pipeline: TaskPipeline): Promise<PipelineResult>;
  
  // Model management
  listModels(): Promise<AIModel[]>;
  getModelStatus(modelId: string): Promise<ModelStatus>;
  testModel(modelId: string): Promise<ModelTestResult>;
  
  // Performance monitoring
  getMetrics(): Promise<OrchestrationMetrics>;
  onMetricsUpdate: Event<MetricsUpdateEvent>;
  
  // Cost tracking
  getCostEstimate(task: AITask): Promise<CostEstimate>;
  getCostReport(period: CostPeriod): Promise<CostReport>;
  onCostAlert: Event<CostAlertEvent>;
}

export interface AITask {
  id: string;
  type: TaskType;
  prompt: string;
  context?: TaskContext;
  constraints?: TaskConstraints;
  priority?: TaskPriority;
  metadata?: Record<string, any>;
}

export enum TaskType {
  CodeGeneration = 'code-generation',
  CodeReview = 'code-review',
  CodeRefactoring = 'code-refactoring',
  Documentation = 'documentation',
  Testing = 'testing',
  Debugging = 'debugging',
  Explanation = 'explanation',
  Translation = 'translation',
  General = 'general'
}

export interface TaskContext {
  language?: string;
  framework?: string;
  files?: string[];
  codebase?: CodebaseContext;
  previousTasks?: string[];
  userPreferences?: Record<string, any>;
}

export interface CodebaseContext {
  rootPath: string;
  relevantFiles: string[];
  dependencies: string[];
  patterns: string[];
}

export interface TaskConstraints {
  maxTokens?: number;
  maxCost?: number;
  maxLatency?: number;
  requiredCapabilities?: string[];
  excludeModels?: string[];
  temperature?: number;
  topP?: number;
}

export enum TaskPriority {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
  Critical = 'critical'
}

export interface ModelSelection {
  primary: AIModel;
  fallbacks: AIModel[];
  reasoning: string;
  estimatedCost: number;
  estimatedLatency: number;
}

export interface TaskRoute {
  models: AIModel[];
  strategy: RoutingStrategy;
  parallelization?: ParallelizationConfig;
  aggregation?: AggregationConfig;
}

export enum RoutingStrategy {
  Single = 'single',
  Fallback = 'fallback',
  Ensemble = 'ensemble',
  Specialized = 'specialized',
  CostOptimized = 'cost-optimized',
  QualityOptimized = 'quality-optimized'
}

export interface ParallelizationConfig {
  enabled: boolean;
  maxConcurrency: number;
  splitStrategy: 'chunk' | 'duplicate' | 'specialized';
}

export interface AggregationConfig {
  method: 'voting' | 'weighted' | 'best' | 'merge';
  weights?: Record<string, number>;
}

export interface ExecutionOptions {
  streaming?: boolean;
  cache?: boolean;
  timeout?: number;
  retries?: number;
  abortSignal?: AbortSignal;
}

export interface TaskResult {
  taskId: string;
  success: boolean;
  output?: string;
  model: string;
  usage: TokenUsage;
  cost: number;
  latency: number;
  cached?: boolean;
  error?: TaskError;
  metadata?: Record<string, any>;
}

export interface TokenUsage {
  prompt: number;
  completion: number;
  total: number;
}

export interface TaskError {
  code: string;
  message: string;
  modelErrors?: ModelError[];
  retryable: boolean;
}

export interface ModelError {
  model: string;
  error: string;
  timestamp: Date;
}

export interface TaskPipeline {
  id: string;
  name: string;
  stages: PipelineStage[];
  variables?: Record<string, any>;
  errorHandling?: ErrorHandlingStrategy;
}

export interface PipelineStage {
  id: string;
  task: AITask;
  dependencies?: string[];
  transform?: (input: any) => any;
  condition?: (context: any) => boolean;
}

export enum ErrorHandlingStrategy {
  FailFast = 'fail-fast',
  Continue = 'continue',
  Retry = 'retry',
  Fallback = 'fallback'
}

export interface PipelineResult {
  pipelineId: string;
  success: boolean;
  stages: Record<string, TaskResult>;
  totalCost: number;
  totalLatency: number;
  output?: any;
}

export interface AIModel {
  id: string;
  provider: string;
  name: string;
  version?: string;
  capabilities: ModelCapability[];
  contextWindow: number;
  maxTokens: number;
  costPer1kTokens: {
    input: number;
    output: number;
  };
  latency: ModelLatency;
  availability: number;
  qualityScore?: number;
}

export interface ModelCapability {
  type: string;
  score: number;
  metadata?: Record<string, any>;
}

export interface ModelLatency {
  p50: number;
  p90: number;
  p99: number;
}

export interface ModelStatus {
  modelId: string;
  available: boolean;
  health: 'healthy' | 'degraded' | 'unhealthy';
  responseTime: number;
  errorRate: number;
  lastChecked: Date;
}

export interface ModelTestResult {
  modelId: string;
  success: boolean;
  latency: number;
  output?: string;
  error?: string;
}

export interface OrchestrationMetrics {
  totalTasks: number;
  successRate: number;
  averageLatency: number;
  totalCost: number;
  modelUsage: Record<string, ModelUsageMetrics>;
  taskTypeBreakdown: Record<string, number>;
  errorRate: number;
  cacheHitRate: number;
}

export interface ModelUsageMetrics {
  requests: number;
  tokens: TokenUsage;
  cost: number;
  avgLatency: number;
  errors: number;
}

export interface MetricsUpdateEvent {
  metrics: OrchestrationMetrics;
  timestamp: Date;
  period: 'realtime' | 'minute' | 'hour' | 'day';
}

export interface CostEstimate {
  estimated: number;
  breakdown: CostBreakdown[];
  confidence: number;
  assumptions: string[];
}

export interface CostBreakdown {
  model: string;
  tokens: number;
  cost: number;
}

export enum CostPeriod {
  Hour = 'hour',
  Day = 'day',
  Week = 'week',
  Month = 'month',
  Custom = 'custom'
}

export interface CostReport {
  period: CostPeriod;
  startDate: Date;
  endDate: Date;
  totalCost: number;
  modelCosts: Record<string, number>;
  taskTypeCosts: Record<string, number>;
  topTasks: TaskCostInfo[];
  trend: CostTrend;
}

export interface TaskCostInfo {
  taskId: string;
  type: TaskType;
  cost: number;
  tokens: number;
  timestamp: Date;
}

export interface CostTrend {
  direction: 'increasing' | 'decreasing' | 'stable';
  percentageChange: number;
  projection: number;
}

export interface CostAlertEvent {
  type: 'threshold' | 'anomaly' | 'projection';
  severity: 'info' | 'warning' | 'critical';
  message: string;
  currentCost: number;
  limit?: number;
  recommendations?: string[];
}