/**
 * Configuration Management Interfaces
 */

import { ModelProfile, TaskType } from '../interfaces';
import { RoutingConfig } from '../routing-engine';
import { CostLimit } from '../cost-tracker';
import { QueueManagerOptions } from '../queue';

export interface ConfigVersion {
  version: string;
  timestamp: Date;
  author?: string;
  description?: string;
  checksum: string;
}

export interface ConfigValidationResult {
  valid: boolean;
  errors: ConfigValidationError[];
  warnings: ConfigValidationWarning[];
}

export interface ConfigValidationError {
  path: string;
  message: string;
  value?: any;
  rule?: string;
}

export interface ConfigValidationWarning {
  path: string;
  message: string;
  suggestion?: string;
}

export interface ConfigTemplate {
  id: string;
  name: string;
  description: string;
  category: ConfigTemplateCategory;
  config: Partial<OrchestrationConfig>;
  variables?: ConfigVariable[];
  examples?: ConfigExample[];
}

export enum ConfigTemplateCategory {
  Development = 'development',
  Production = 'production',
  Testing = 'testing',
  HighPerformance = 'high-performance',
  CostOptimized = 'cost-optimized',
  Custom = 'custom'
}

export interface ConfigVariable {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'object' | 'array';
  required: boolean;
  default?: any;
  description?: string;
  validation?: ConfigVariableValidation;
}

export interface ConfigVariableValidation {
  min?: number;
  max?: number;
  pattern?: string;
  enum?: any[];
  custom?: (value: any) => boolean;
}

export interface ConfigExample {
  name: string;
  description: string;
  values: Record<string, any>;
  expectedBehavior?: string;
}

export interface ConfigChangeEvent {
  type: ConfigChangeType;
  timestamp: Date;
  previousConfig?: OrchestrationConfig;
  newConfig: OrchestrationConfig;
  changedPaths: string[];
  source: ConfigChangeSource;
}

export enum ConfigChangeType {
  Created = 'created',
  Updated = 'updated',
  Deleted = 'deleted',
  Reloaded = 'reloaded',
  RolledBack = 'rolled-back'
}

export enum ConfigChangeSource {
  File = 'file',
  API = 'api',
  Environment = 'environment',
  Runtime = 'runtime',
  Default = 'default'
}

export interface ConfigDiff {
  added: Record<string, any>;
  removed: Record<string, any>;
  changed: Record<string, { old: any; new: any }>;
  unchanged: string[];
}

export interface ConfigMergeStrategy {
  arrays: 'replace' | 'append' | 'merge' | 'unique';
  objects: 'replace' | 'merge' | 'deep-merge';
  conflicts: 'error' | 'prefer-source' | 'prefer-target' | 'custom';
  customResolver?: (path: string, source: any, target: any) => any;
}

export interface ConfigProvider {
  name: string;
  priority: number;
  load(): Promise<Partial<OrchestrationConfig>>;
  watch?(callback: (config: Partial<OrchestrationConfig>) => void): void;
  unwatch?(): void;
}

export interface ConfigStore {
  save(config: OrchestrationConfig, version?: ConfigVersion): Promise<void>;
  load(version?: string): Promise<OrchestrationConfig>;
  listVersions(): Promise<ConfigVersion[]>;
  rollback(version: string): Promise<OrchestrationConfig>;
  delete(version: string): Promise<void>;
}

// Extended orchestration config with all options
export interface OrchestrationConfig {
  // Core settings
  version?: string;
  name?: string;
  description?: string;
  environment?: 'development' | 'staging' | 'production';
  
  // Routing configuration
  routing?: RoutingConfig;
  
  // Model configurations
  models?: {
    registry?: ModelRegistryConfig;
    overrides?: ModelOverride[];
    defaults?: ModelDefaults;
  };
  
  // Cost management
  cost?: {
    limits?: CostLimit[];
    alerts?: CostAlert[];
    tracking?: CostTrackingConfig;
  };
  
  // Caching
  cache?: {
    enabled?: boolean;
    ttl?: number;
    maxSize?: number;
    strategy?: CacheStrategy;
    providers?: CacheProviderConfig[];
  };
  
  // Queue management
  queue?: QueueManagerOptions;
  
  // Performance
  performance?: {
    defaultTimeout?: number;
    defaultRetries?: number;
    concurrency?: ConcurrencyConfig;
    circuitBreaker?: CircuitBreakerConfig;
  };
  
  // Monitoring
  monitoring?: {
    metricsInterval?: number;
    exporters?: MetricsExporter[];
    logging?: LoggingConfig;
    tracing?: TracingConfig;
  };
  
  // Security
  security?: {
    apiKeys?: ApiKeyConfig[];
    encryption?: EncryptionConfig;
    rateLimit?: RateLimitConfig;
  };
  
  // Features
  features?: {
    [key: string]: boolean | FeatureConfig;
  };
}

export interface ModelRegistryConfig {
  refreshInterval?: number;
  sources?: ModelSource[];
  filters?: ModelFilter[];
}

export interface ModelSource {
  type: 'static' | 'dynamic' | 'api';
  url?: string;
  models?: ModelProfile[];
  authentication?: AuthConfig;
}

export interface ModelFilter {
  include?: Partial<ModelProfile>;
  exclude?: Partial<ModelProfile>;
}

export interface ModelOverride {
  modelId: string;
  override: Partial<ModelProfile>;
  conditions?: ModelOverrideCondition[];
}

export interface ModelOverrideCondition {
  type: 'time' | 'load' | 'error-rate' | 'custom';
  value: any;
  operator: 'eq' | 'gt' | 'lt' | 'gte' | 'lte' | 'between';
}

export interface ModelDefaults {
  temperature?: number;
  maxTokens?: number;
  topP?: number;
  topK?: number;
  frequencyPenalty?: number;
  presencePenalty?: number;
  stopSequences?: string[];
}

export interface CostAlert {
  threshold: number;
  type: 'total' | 'per-model' | 'per-user' | 'per-task-type';
  action: 'log' | 'email' | 'webhook' | 'throttle' | 'block';
  config?: any;
}

export interface CostTrackingConfig {
  granularity: 'request' | 'minute' | 'hour' | 'day';
  retention: number; // days
  aggregations?: CostAggregation[];
}

export interface CostAggregation {
  name: string;
  groupBy: string[];
  metrics: string[];
  interval: string;
}

export interface CacheStrategy {
  type: 'lru' | 'lfu' | 'ttl' | 'adaptive';
  config?: any;
}

export interface CacheProviderConfig {
  type: 'memory' | 'redis' | 'memcached' | 'custom';
  priority: number;
  config?: any;
}

export interface ConcurrencyConfig {
  maxParallel?: number;
  maxPerModel?: Record<string, number>;
  maxPerUser?: number;
  queueStrategy?: 'fifo' | 'lifo' | 'priority' | 'fair';
}

export interface CircuitBreakerConfig {
  enabled?: boolean;
  failureThreshold?: number;
  resetTimeout?: number;
  halfOpenRequests?: number;
}

export interface MetricsExporter {
  type: 'prometheus' | 'datadog' | 'cloudwatch' | 'custom';
  endpoint?: string;
  interval?: number;
  config?: any;
}

export interface LoggingConfig {
  level: 'debug' | 'info' | 'warn' | 'error';
  format: 'json' | 'text' | 'structured';
  destinations: LogDestination[];
}

export interface LogDestination {
  type: 'console' | 'file' | 'syslog' | 'http';
  config?: any;
}

export interface TracingConfig {
  enabled?: boolean;
  sampler?: TracingSampler;
  exporter?: TracingExporter;
}

export interface TracingSampler {
  type: 'always' | 'never' | 'probability' | 'adaptive';
  config?: any;
}

export interface TracingExporter {
  type: 'jaeger' | 'zipkin' | 'otlp' | 'custom';
  endpoint?: string;
  config?: any;
}

export interface ApiKeyConfig {
  provider: string;
  key?: string;
  keyRef?: string; // Reference to environment variable or secret
  rotation?: KeyRotationConfig;
}

export interface KeyRotationConfig {
  enabled?: boolean;
  interval?: number; // days
  overlap?: number; // hours
  notifyBefore?: number; // days
}

export interface EncryptionConfig {
  algorithm: string;
  keyManagement: 'local' | 'kms' | 'vault';
  config?: any;
}

export interface RateLimitConfig {
  global?: RateLimitRule;
  perUser?: RateLimitRule;
  perModel?: Record<string, RateLimitRule>;
  perEndpoint?: Record<string, RateLimitRule>;
}

export interface RateLimitRule {
  requests: number;
  window: number; // milliseconds
  burst?: number;
}

export interface FeatureConfig {
  enabled: boolean;
  rollout?: RolloutConfig;
  config?: any;
}

export interface RolloutConfig {
  percentage?: number;
  users?: string[];
  groups?: string[];
  conditions?: RolloutCondition[];
}

export interface RolloutCondition {
  type: 'user' | 'group' | 'time' | 'random' | 'custom';
  value: any;
}

export interface AuthConfig {
  type: 'none' | 'api-key' | 'oauth' | 'custom';
  config?: any;
}