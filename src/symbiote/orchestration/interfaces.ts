/**
 * Core interfaces for the Multi-Model Orchestration Engine
 */

// Task Types
export enum TaskType {
  CodeGeneration = 'code-generation',
  CodeReview = 'code-review',
  CodeCompletion = 'code-completion',
  Documentation = 'documentation',
  Testing = 'testing',
  Refactoring = 'refactoring',
  BugFix = 'bug-fix',
  Security = 'security',
  Performance = 'performance',
  General = 'general',
  Research = 'research',
  Translation = 'translation'
}

// Quality Levels
export enum QualityLevel {
  Draft = 'draft',
  Standard = 'standard',
  High = 'high',
  Premium = 'premium'
}

// Urgency Levels
export enum UrgencyLevel {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
  Critical = 'critical'
}

// Complexity Levels
export enum ComplexityLevel {
  Trivial = 'trivial',
  Simple = 'simple',
  Moderate = 'moderate',
  Complex = 'complex',
  Expert = 'expert'
}

// Model Capabilities
export enum Capability {
  CodeGeneration = 'code-generation',
  CodeAnalysis = 'code-analysis',
  NaturalLanguage = 'natural-language',
  Mathematics = 'mathematics',
  Reasoning = 'reasoning',
  CreativeWriting = 'creative-writing',
  Translation = 'translation',
  Summarization = 'summarization',
  QuestionAnswering = 'question-answering',
  FunctionCalling = 'function-calling',
  Vision = 'vision',
  LongContext = 'long-context'
}

// Provider Types
export enum ProviderType {
  Anthropic = 'anthropic',
  OpenAI = 'openai',
  Google = 'google',
  Microsoft = 'microsoft',
  Mistral = 'mistral',
  Cohere = 'cohere',
  Local = 'local',
  Custom = 'custom'
}

// Core Task Interface
export interface AITask {
  id: string;
  type: TaskType;
  prompt: string;
  context?: TaskContext;
  constraints?: TaskConstraints;
  metadata?: TaskMetadata;
  priority?: UrgencyLevel;
  sessionId?: string;
}

// Task Context
export interface TaskContext {
  files?: FileContext[];
  language?: string;
  framework?: string;
  projectType?: string;
  previousMessages?: Message[];
  codebase?: CodebaseContext;
  customContext?: Record<string, any>;
}

// File Context
export interface FileContext {
  path: string;
  content: string;
  language: string;
  startLine?: number;
  endLine?: number;
}

// Codebase Context
export interface CodebaseContext {
  rootPath: string;
  structure?: string;
  dependencies?: string[];
  configuration?: Record<string, any>;
}

// Task Constraints
export interface TaskConstraints {
  maxTokens?: number;
  maxCost?: number;
  maxLatency?: number;
  temperature?: number;
  topP?: number;
  stopSequences?: string[];
  requiredCapabilities?: Capability[];
  excludeProviders?: ProviderType[];
  preferredProviders?: ProviderType[];
}

// Task Metadata
export interface TaskMetadata {
  userId?: string;
  projectId?: string;
  timestamp?: number;
  tags?: string[];
  tracking?: Record<string, any>;
}

// Message Interface
export interface Message {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp?: number;
  model?: string;
}

// Task Analysis Result
export interface TaskAnalysis {
  type: TaskType;
  complexity: ComplexityLevel;
  estimatedTokens: number;
  requiredCapabilities: Capability[];
  contextWindow: number;
  qualityRequirement: QualityLevel;
  urgency: UrgencyLevel;
  confidence: number;
}

// Model Profile
export interface ModelProfile {
  id: string;
  provider: ProviderType;
  name: string;
  displayName: string;
  version?: string;
  capabilities: Capability[];
  contextWindow: number;
  maxOutputTokens: number;
  costPerToken: TokenCost;
  averageLatency: number;
  reliability: number;
  specializations: string[];
  rateLimit: RateLimit;
  currentLoad?: number;
  availability: ModelAvailability;
  metadata?: ModelMetadata;
}

// Token Cost
export interface TokenCost {
  input: number;
  output: number;
  currency: string;
}

// Rate Limit
export interface RateLimit {
  requestsPerMinute: number;
  tokensPerMinute: number;
  requestsPerDay?: number;
  tokensPerDay?: number;
}

// Model Availability
export interface ModelAvailability {
  status: 'available' | 'degraded' | 'unavailable';
  region?: string[];
  restrictions?: string[];
  maintenanceWindow?: MaintenanceWindow;
}

// Maintenance Window
export interface MaintenanceWindow {
  start: Date;
  end: Date;
  description?: string;
}

// Model Metadata
export interface ModelMetadata {
  releaseDate?: Date;
  deprecationDate?: Date;
  successRate?: number;
  averageQuality?: number;
  preferredForTasks?: TaskType[];
  notes?: string;
}

// Model Selection Result
export interface ModelSelection {
  primary: ModelProfile;
  fallbacks: ModelProfile[];
  reasoning: string;
  estimatedCost: CostEstimate;
  estimatedLatency: number;
  confidence: number;
  warnings?: string[];
}

// Cost Estimate
export interface CostEstimate {
  minimum: number;
  expected: number;
  maximum: number;
  currency: string;
  breakdown?: CostBreakdown;
}

// Cost Breakdown
export interface CostBreakdown {
  inputTokens: number;
  outputTokens: number;
  inputCost: number;
  outputCost: number;
  fixedCost?: number;
}

// Chat Interfaces
export interface ChatRequest {
  messages: Array<{
    role: 'system' | 'user' | 'assistant';
    content: string;
  }>;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  stream?: boolean;
  context?: Record<string, any>;
}

export interface ChatResponse {
  content: string;
  model: string;
  usage?: {
    inputTokens: number;
    outputTokens: number;
    totalTokens: number;
  };
  metadata?: Record<string, any>;
}

// Task Route
export interface TaskRoute {
  taskId: string;
  selectedModel: ModelProfile;
  routingReason: string;
  alternatives: ModelProfile[];
  timestamp: number;
}

// Task Result
export interface TaskResult {
  id: string;
  taskId: string;
  status: 'success' | 'partial' | 'failed';
  content?: string;
  error?: TaskError;
  model: string;
  usage: TokenUsage;
  cost: ActualCost;
  latency: number;
  timestamp: number;
  metadata?: ResultMetadata;
}

// Task Error
export interface TaskError {
  code: string;
  message: string;
  type: 'rate_limit' | 'timeout' | 'invalid_request' | 'server_error' | 'unknown';
  retryable: boolean;
  details?: any;
}

// Token Usage
export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

// Actual Cost
export interface ActualCost {
  amount: number;
  currency: string;
  breakdown: CostBreakdown;
}

// Result Metadata
export interface ResultMetadata {
  cacheHit?: boolean;
  retryCount?: number;
  fallbackUsed?: boolean;
  processingTime?: number;
  qualityScore?: number;
}

// Execution Options
export interface ExecutionOptions {
  stream?: boolean;
  timeout?: number;
  retryPolicy?: RetryPolicy;
  cachePolicy?: CachePolicy;
  validationLevel?: ValidationLevel;
  callbacks?: ExecutionCallbacks;
}

// Retry Policy
export interface RetryPolicy {
  maxAttempts: number;
  backoffStrategy: 'exponential' | 'linear' | 'fixed';
  initialDelay: number;
  maxDelay: number;
  fallbackOnFailure: boolean;
}

// Cache Policy
export interface CachePolicy {
  enabled: boolean;
  ttl: number;
  strategy: 'exact' | 'semantic' | 'hybrid';
  sharing: 'private' | 'project' | 'global';
}

// Validation Level
export enum ValidationLevel {
  None = 'none',
  Basic = 'basic',
  Strict = 'strict'
}

// Execution Callbacks
export interface ExecutionCallbacks {
  onStart?: (task: AITask) => void;
  onModelSelected?: (selection: ModelSelection) => void;
  onRetry?: (attempt: number, error: TaskError) => void;
  onProgress?: (progress: ExecutionProgress) => void;
  onComplete?: (result: TaskResult) => void;
  onError?: (error: TaskError) => void;
}

// Execution Progress
export interface ExecutionProgress {
  stage: 'routing' | 'executing' | 'processing' | 'validating';
  percentage?: number;
  message?: string;
}

// Pipeline Configuration
export interface TaskPipeline {
  id: string;
  name: string;
  tasks: PipelineTask[];
  options?: PipelineOptions;
}

// Pipeline Task
export interface PipelineTask {
  task: AITask;
  dependencies?: string[];
  condition?: PipelineCondition;
  transformation?: (input: any) => any;
}

// Pipeline Condition
export interface PipelineCondition {
  type: 'success' | 'contains' | 'custom';
  value?: any;
  predicate?: (result: TaskResult) => boolean;
}

// Pipeline Options
export interface PipelineOptions {
  parallel?: boolean;
  continueOnError?: boolean;
  timeout?: number;
  maxConcurrency?: number;
}

// Pipeline Result
export interface PipelineResult {
  id: string;
  pipelineId: string;
  status: 'success' | 'partial' | 'failed';
  results: TaskResult[];
  totalCost: ActualCost;
  totalLatency: number;
  timestamp: number;
}

// Orchestration Metrics
export interface OrchestrationMetrics {
  performance: PerformanceMetrics;
  cost: CostMetrics;
  quality: QualityMetrics;
  usage: UsageMetrics;
  timestamp: number;
}

// Performance Metrics
export interface PerformanceMetrics {
  averageLatency: number;
  p50Latency: number;
  p95Latency: number;
  p99Latency: number;
  throughput: number;
  activeRequests: number;
  queuedRequests: number;
}

// Cost Metrics
export interface CostMetrics {
  totalCost: number;
  costPerTask: Record<TaskType, number>;
  costPerModel: Record<string, number>;
  budgetUtilization: number;
  projectedMonthlyCost: number;
}

// Quality Metrics
export interface QualityMetrics {
  successRate: number;
  fallbackRate: number;
  retryRate: number;
  validationFailureRate: number;
  userSatisfaction?: number;
  averageConfidence: number;
}

// Usage Metrics
export interface UsageMetrics {
  totalRequests: number;
  requestsByType: Record<TaskType, number>;
  requestsByModel: Record<string, number>;
  cacheHitRate: number;
  tokenUsage: TokenUsage;
}