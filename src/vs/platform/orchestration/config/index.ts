/**
 * Configuration Management Module Exports
 */

// Main configuration manager
export { ConfigManager } from './config-manager';

// Interfaces
export * from './config-interfaces';

// Validator
export { ConfigValidator } from './config-validator';

// Store implementations
export { 
  FileSystemConfigStore,
  InMemoryConfigStore
} from './config-store';

// Configuration providers
export {
  EnvironmentConfigProvider,
  JsonFileConfigProvider,
  RemoteConfigProvider,
  CliArgsConfigProvider,
  KeyValueStoreConfigProvider,
  CompositeConfigProvider
} from './config-providers';

// Templates
export {
  configTemplates,
  getTemplate,
  getTemplatesByCategory,
  applyTemplate
} from './config-templates';

// Re-export key types for convenience
export type {
  OrchestrationConfig,
  ConfigVersion,
  ConfigValidationResult,
  ConfigTemplate,
  ConfigProvider,
  ConfigStore,
  ConfigChangeEvent,
  ConfigDiff
} from './config-interfaces';