/**
 * Orchestration Module - Multi-model AI orchestration engine
 */

// Main exports
export { OrchestrationEngine } from './orchestration-engine';
export { OrchestrationAPIImpl } from './orchestration-api-impl';
export { ModelRegistry } from './model-registry';
export { TaskAnalyzer } from './task-analyzer';
export { RoutingEngine } from './routing-engine';
export { CostTracker } from './cost-tracker';
export { CacheManager } from './cache-manager';

// Provider adapters
export { ProviderAdapter } from './providers/provider-adapter';
export { AnthropicAdapter } from './providers/anthropic-adapter';
export { OpenAIAdapter } from './providers/openai-adapter';
export { GoogleAdapter } from './providers/google-adapter';
export { MistralAdapter } from './providers/mistral-adapter';
export { LocalAdapter } from './providers/local-adapter';

// A/B Testing Framework
export * from './ab-testing';

// Queue Management
export * from './queue';

// Configuration Management
export * from './config';

// API Layer
export * from './api';

// Interfaces
export * from './interfaces';

// Types for external use
export type {
  RoutingConfig,
  ScoringWeights
} from './routing-engine';

export type {
  CostLimit,
  CostAlert,
  CostRecord
} from './cost-tracker';

export type {
  CacheEntry,
  CacheStats
} from './cache-manager';

export type {
  OrchestrationConfig
} from './orchestration-engine';