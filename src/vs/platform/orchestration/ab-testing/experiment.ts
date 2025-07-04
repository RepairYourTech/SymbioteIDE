/**
 * A/B Testing Experiment Definitions
 */

import { AITask, ModelProfile, TaskResult, TaskType } from '../interfaces';

export interface ExperimentConfig {
  id: string;
  name: string;
  description: string;
  startTime: Date;
  endTime?: Date;
  status: ExperimentStatus;
  variants: ExperimentVariant[];
  trafficAllocation: TrafficAllocation;
  metrics: MetricConfig[];
  constraints?: ExperimentConstraints;
  metadata?: Record<string, any>;
}

export enum ExperimentStatus {
  Draft = 'draft',
  Running = 'running',
  Paused = 'paused',
  Completed = 'completed',
  Cancelled = 'cancelled'
}

export interface ExperimentVariant {
  id: string;
  name: string;
  description?: string;
  modelId: string;
  modelConfig?: ModelConfig;
  weight: number; // 0-100 percentage
}

export interface ModelConfig {
  temperature?: number;
  topP?: number;
  maxTokens?: number;
  systemPrompt?: string;
  additionalParams?: Record<string, any>;
}

export interface TrafficAllocation {
  method: AllocationMethod;
  seed?: string; // For deterministic allocation
  userSegments?: UserSegment[];
  taskTypeFilters?: TaskType[];
}

export enum AllocationMethod {
  Random = 'random',
  Deterministic = 'deterministic',
  UserBased = 'user-based',
  SessionBased = 'session-based',
  TaskBased = 'task-based'
}

export interface UserSegment {
  id: string;
  name: string;
  criteria: SegmentCriteria;
  variantId: string;
}

export interface SegmentCriteria {
  userIds?: string[];
  projectIds?: string[];
  tags?: string[];
  customPredicate?: (context: any) => boolean;
}

export interface MetricConfig {
  id: string;
  name: string;
  type: MetricType;
  aggregation: AggregationType;
  unit?: string;
  higherIsBetter: boolean;
  minimumSampleSize?: number;
  confidenceLevel?: number; // 0.95 = 95%
}

export enum MetricType {
  Latency = 'latency',
  Cost = 'cost',
  TokenUsage = 'token-usage',
  SuccessRate = 'success-rate',
  UserSatisfaction = 'user-satisfaction',
  OutputQuality = 'output-quality',
  CustomNumeric = 'custom-numeric',
  CustomBoolean = 'custom-boolean'
}

export enum AggregationType {
  Average = 'average',
  Median = 'median',
  Percentile = 'percentile',
  Sum = 'sum',
  Count = 'count',
  Rate = 'rate'
}

export interface ExperimentConstraints {
  maxDuration?: number; // milliseconds
  maxSamples?: number;
  maxCost?: number;
  requiredSampleSize?: number;
  stopOnSignificance?: boolean;
  earlyStoppingRules?: EarlyStoppingRule[];
}

export interface EarlyStoppingRule {
  metric: string;
  condition: 'better_than' | 'worse_than' | 'equal_to';
  threshold: number;
  variantId?: string; // Compare against specific variant
  confidence?: number;
}

export interface ExperimentResult {
  experimentId: string;
  status: ExperimentStatus;
  startTime: Date;
  endTime?: Date;
  variants: VariantResult[];
  overallMetrics: MetricResult[];
  statisticalAnalysis?: StatisticalAnalysis;
  recommendation?: string;
  metadata?: Record<string, any>;
}

export interface VariantResult {
  variantId: string;
  variantName: string;
  sampleSize: number;
  metrics: MetricResult[];
  errors: number;
  errorRate: number;
}

export interface MetricResult {
  metricId: string;
  metricName: string;
  value: number;
  standardDeviation?: number;
  confidenceInterval?: ConfidenceInterval;
  samples: number;
  outliers?: number;
}

export interface ConfidenceInterval {
  lower: number;
  upper: number;
  confidence: number;
}

export interface StatisticalAnalysis {
  primaryMetric: string;
  winner?: string; // Variant ID
  confidence?: number;
  pValue?: number;
  effectSize?: number;
  powerAnalysis?: PowerAnalysis;
  tests: StatisticalTest[];
}

export interface PowerAnalysis {
  currentPower: number;
  requiredSampleSize: number;
  expectedDuration: number;
}

export interface StatisticalTest {
  name: string;
  metric: string;
  testType: 't-test' | 'chi-square' | 'mann-whitney' | 'anova';
  pValue: number;
  statistic: number;
  degreesOfFreedom?: number;
  significant: boolean;
}

export interface ExperimentSample {
  id: string;
  experimentId: string;
  variantId: string;
  task: AITask;
  result: TaskResult;
  metrics: Record<string, number>;
  timestamp: Date;
  userId?: string;
  sessionId?: string;
  metadata?: Record<string, any>;
}

export interface ExperimentEvent {
  type: ExperimentEventType;
  experimentId: string;
  timestamp: Date;
  data: any;
}

export enum ExperimentEventType {
  Started = 'started',
  Paused = 'paused',
  Resumed = 'resumed',
  Completed = 'completed',
  Cancelled = 'cancelled',
  VariantAdded = 'variant-added',
  VariantRemoved = 'variant-removed',
  TrafficUpdated = 'traffic-updated',
  MetricUpdated = 'metric-updated',
  SampleCollected = 'sample-collected',
  AnalysisCompleted = 'analysis-completed'
}