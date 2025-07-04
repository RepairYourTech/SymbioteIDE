/**
 * Configuration Manager - Handles hot reloading and dynamic configuration
 */

import { EventEmitter } from 'events';
import * as fs from 'fs';
import * as path from 'path';
import {
  OrchestrationConfig,
  ConfigStore,
  ConfigProvider,
  ConfigChangeEvent,
  ConfigChangeType,
  ConfigChangeSource,
  ConfigValidationResult,
  ConfigMergeStrategy,
  ConfigDiff
} from './config-interfaces';
import { ConfigValidator } from './config-validator';
import { FileSystemConfigStore } from './config-store';

export class ConfigManager extends EventEmitter {
  private config: OrchestrationConfig = {};
  private store: ConfigStore;
  private validator: ConfigValidator;
  private providers: ConfigProvider[] = [];
  private watchers: Map<string, fs.FSWatcher> = new Map();
  private mergeStrategy: ConfigMergeStrategy;
  private configPath?: string;
  private isInitialized: boolean = false;
  private reloadTimeout?: NodeJS.Timeout;
  private readonly reloadDebounce = 1000; // 1 second

  constructor(options?: {
    store?: ConfigStore;
    configPath?: string;
    mergeStrategy?: ConfigMergeStrategy;
  }) {
    super();
    this.store = options?.store || new FileSystemConfigStore();
    this.configPath = options?.configPath;
    this.validator = new ConfigValidator();
    this.mergeStrategy = options?.mergeStrategy || {
      arrays: 'replace',
      objects: 'deep-merge',
      conflicts: 'prefer-source'
    };
  }

  /**
   * Initialize configuration manager
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) return;

    // Load initial configuration
    await this.loadConfiguration();

    // Start watching if path provided
    if (this.configPath) {
      this.watchConfigFile(this.configPath);
    }

    // Initialize providers
    for (const provider of this.providers) {
      if (provider.watch) {
        provider.watch((config) => {
          this.handleProviderUpdate(provider, config);
        });
      }
    }

    this.isInitialized = true;
    this.emit('initialized', this.config);
  }

  /**
   * Get current configuration
   */
  getConfig(): Readonly<OrchestrationConfig> {
    return Object.freeze(JSON.parse(JSON.stringify(this.config)));
  }

  /**
   * Get specific configuration value by path
   */
  get<T = any>(path: string, defaultValue?: T): T {
    const keys = path.split('.');
    let current: any = this.config;

    for (const key of keys) {
      if (current && typeof current === 'object' && key in current) {
        current = current[key];
      } else {
        return defaultValue as T;
      }
    }

    return current as T;
  }

  /**
   * Set configuration value
   */
  async set(path: string, value: any): Promise<void> {
    const keys = path.split('.');
    const newConfig = JSON.parse(JSON.stringify(this.config));
    let current: any = newConfig;

    // Navigate to parent
    for (let i = 0; i < keys.length - 1; i++) {
      const key = keys[i];
      if (!(key in current) || typeof current[key] !== 'object') {
        current[key] = {};
      }
      current = current[key];
    }

    // Set value
    const lastKey = keys[keys.length - 1];
    const oldValue = current[lastKey];
    current[lastKey] = value;

    // Validate new configuration
    const validation = this.validator.validate(newConfig);
    if (!validation.valid) {
      throw new Error(
        `Configuration validation failed: ${validation.errors.map(e => e.message).join(', ')}`
      );
    }

    // Apply change
    await this.updateConfiguration(newConfig, ConfigChangeSource.Runtime);

    // Emit specific change event
    this.emit('configChanged', {
      path,
      oldValue,
      newValue: value,
      timestamp: new Date()
    });
  }

  /**
   * Update entire configuration
   */
  async updateConfiguration(
    newConfig: OrchestrationConfig,
    source: ConfigChangeSource = ConfigChangeSource.API
  ): Promise<void> {
    // Validate configuration
    const validation = this.validator.validate(newConfig);
    if (!validation.valid) {
      throw new Error(
        `Configuration validation failed: ${validation.errors.map(e => e.message).join(', ')}`
      );
    }

    // Calculate diff
    const diff = this.calculateDiff(this.config, newConfig);
    const changedPaths = [
      ...Object.keys(diff.added),
      ...Object.keys(diff.changed),
      ...Object.keys(diff.removed)
    ];

    // Store previous config
    const previousConfig = this.config;
    this.config = newConfig;

    // Save to store
    await this.store.save(newConfig);

    // Emit change event
    const event: ConfigChangeEvent = {
      type: ConfigChangeType.Updated,
      timestamp: new Date(),
      previousConfig,
      newConfig,
      changedPaths,
      source
    };

    this.emit('change', event);

    // Emit warnings if any
    if (validation.warnings.length > 0) {
      this.emit('validationWarnings', validation.warnings);
    }
  }

  /**
   * Reload configuration from sources
   */
  async reload(): Promise<void> {
    await this.loadConfiguration();
    
    const event: ConfigChangeEvent = {
      type: ConfigChangeType.Reloaded,
      timestamp: new Date(),
      newConfig: this.config,
      changedPaths: [],
      source: ConfigChangeSource.File
    };

    this.emit('change', event);
  }

  /**
   * Add configuration provider
   */
  addProvider(provider: ConfigProvider): void {
    this.providers.push(provider);
    
    // Sort by priority
    this.providers.sort((a, b) => b.priority - a.priority);

    if (this.isInitialized && provider.watch) {
      provider.watch((config) => {
        this.handleProviderUpdate(provider, config);
      });
    }
  }

  /**
   * Remove configuration provider
   */
  removeProvider(name: string): void {
    const index = this.providers.findIndex(p => p.name === name);
    if (index !== -1) {
      const provider = this.providers[index];
      if (provider.unwatch) {
        provider.unwatch();
      }
      this.providers.splice(index, 1);
    }
  }

  /**
   * Validate configuration
   */
  validate(config?: OrchestrationConfig): ConfigValidationResult {
    return this.validator.validate(config || this.config);
  }

  /**
   * Get configuration diff
   */
  diff(config1: OrchestrationConfig, config2: OrchestrationConfig): ConfigDiff {
    return this.calculateDiff(config1, config2);
  }

  /**
   * Merge configurations
   */
  merge(
    source: OrchestrationConfig,
    target: OrchestrationConfig,
    strategy?: ConfigMergeStrategy
  ): OrchestrationConfig {
    const mergeStrategy = strategy || this.mergeStrategy;
    return this.mergeConfigs(source, target, mergeStrategy);
  }

  /**
   * Watch configuration file
   */
  watchConfigFile(filePath: string): void {
    // Stop existing watcher
    const existingWatcher = this.watchers.get(filePath);
    if (existingWatcher) {
      existingWatcher.close();
    }

    // Create new watcher
    const watcher = fs.watch(filePath, (eventType) => {
      if (eventType === 'change') {
        this.handleFileChange(filePath);
      }
    });

    watcher.on('error', (error) => {
      this.emit('error', new Error(`Config file watch error: ${error.message}`));
    });

    this.watchers.set(filePath, watcher);
  }

  /**
   * Stop watching configuration files
   */
  stopWatching(): void {
    for (const [path, watcher] of this.watchers) {
      watcher.close();
    }
    this.watchers.clear();
  }

  /**
   * Get configuration history
   */
  async getHistory(): Promise<any[]> {
    const versions = await this.store.listVersions();
    return versions;
  }

  /**
   * Rollback to previous version
   */
  async rollback(version: string): Promise<void> {
    const config = await this.store.rollback(version);
    
    const event: ConfigChangeEvent = {
      type: ConfigChangeType.RolledBack,
      timestamp: new Date(),
      previousConfig: this.config,
      newConfig: config,
      changedPaths: [],
      source: ConfigChangeSource.API
    };

    this.config = config;
    this.emit('change', event);
  }

  /**
   * Shutdown configuration manager
   */
  async shutdown(): Promise<void> {
    // Stop all watchers
    this.stopWatching();

    // Unwatch all providers
    for (const provider of this.providers) {
      if (provider.unwatch) {
        provider.unwatch();
      }
    }

    // Clear reload timeout
    if (this.reloadTimeout) {
      clearTimeout(this.reloadTimeout);
    }

    this.isInitialized = false;
    this.emit('shutdown');
  }

  // Private methods

  private async loadConfiguration(): Promise<void> {
    let config: OrchestrationConfig = {};

    // Load from store
    try {
      config = await this.store.load();
    } catch (error) {
      this.emit('error', new Error(`Failed to load config from store: ${error}`));
    }

    // Load from file if path provided
    if (this.configPath) {
      try {
        const fileContent = await fs.promises.readFile(this.configPath, 'utf-8');
        const fileConfig = JSON.parse(fileContent);
        config = this.mergeConfigs(fileConfig, config, this.mergeStrategy);
      } catch (error) {
        this.emit('error', new Error(`Failed to load config file: ${error}`));
      }
    }

    // Load from providers
    for (const provider of this.providers) {
      try {
        const providerConfig = await provider.load();
        config = this.mergeConfigs(providerConfig, config, this.mergeStrategy);
      } catch (error) {
        this.emit('error', new Error(`Failed to load config from provider ${provider.name}: ${error}`));
      }
    }

    // Validate final configuration
    const validation = this.validator.validate(config);
    if (!validation.valid) {
      this.emit('error', new Error(
        `Configuration validation failed: ${validation.errors.map(e => e.message).join(', ')}`
      ));
      // Use previous config if validation fails
      return;
    }

    this.config = config;
  }

  private handleFileChange(filePath: string): void {
    // Debounce reloads
    if (this.reloadTimeout) {
      clearTimeout(this.reloadTimeout);
    }

    this.reloadTimeout = setTimeout(async () => {
      try {
        await this.reload();
        this.emit('fileReloaded', filePath);
      } catch (error) {
        this.emit('error', new Error(`Failed to reload config: ${error}`));
      }
    }, this.reloadDebounce);
  }

  private async handleProviderUpdate(
    provider: ConfigProvider,
    config: Partial<OrchestrationConfig>
  ): Promise<void> {
    try {
      // Merge with current config
      const newConfig = this.mergeConfigs(config, this.config, this.mergeStrategy);
      
      // Update configuration
      await this.updateConfiguration(newConfig, ConfigChangeSource.Runtime);
      
      this.emit('providerUpdate', {
        provider: provider.name,
        config
      });
    } catch (error) {
      this.emit('error', new Error(
        `Failed to apply update from provider ${provider.name}: ${error}`
      ));
    }
  }

  private calculateDiff(
    oldConfig: any,
    newConfig: any,
    path: string = ''
  ): ConfigDiff {
    const diff: ConfigDiff = {
      added: {},
      removed: {},
      changed: {},
      unchanged: []
    };

    // Get all keys from both configs
    const allKeys = new Set([
      ...Object.keys(oldConfig || {}),
      ...Object.keys(newConfig || {})
    ]);

    for (const key of allKeys) {
      const currentPath = path ? `${path}.${key}` : key;
      const oldValue = oldConfig?.[key];
      const newValue = newConfig?.[key];

      if (oldValue === undefined && newValue !== undefined) {
        // Added
        diff.added[currentPath] = newValue;
      } else if (oldValue !== undefined && newValue === undefined) {
        // Removed
        diff.removed[currentPath] = oldValue;
      } else if (this.isObject(oldValue) && this.isObject(newValue)) {
        // Recurse into objects
        const subDiff = this.calculateDiff(oldValue, newValue, currentPath);
        Object.assign(diff.added, subDiff.added);
        Object.assign(diff.removed, subDiff.removed);
        Object.assign(diff.changed, subDiff.changed);
        diff.unchanged.push(...subDiff.unchanged);
      } else if (JSON.stringify(oldValue) !== JSON.stringify(newValue)) {
        // Changed
        diff.changed[currentPath] = { old: oldValue, new: newValue };
      } else {
        // Unchanged
        diff.unchanged.push(currentPath);
      }
    }

    return diff;
  }

  private mergeConfigs(
    source: any,
    target: any,
    strategy: ConfigMergeStrategy
  ): any {
    if (!this.isObject(source) || !this.isObject(target)) {
      return source;
    }

    const result: any = {};

    // Merge target first
    for (const key in target) {
      result[key] = target[key];
    }

    // Merge source
    for (const key in source) {
      const sourceValue = source[key];
      const targetValue = target[key];

      if (targetValue === undefined) {
        result[key] = sourceValue;
      } else if (Array.isArray(sourceValue) && Array.isArray(targetValue)) {
        // Handle arrays
        switch (strategy.arrays) {
          case 'replace':
            result[key] = sourceValue;
            break;
          case 'append':
            result[key] = [...targetValue, ...sourceValue];
            break;
          case 'merge':
            result[key] = [...targetValue];
            for (const item of sourceValue) {
              if (!result[key].includes(item)) {
                result[key].push(item);
              }
            }
            break;
          case 'unique':
            result[key] = Array.from(new Set([...targetValue, ...sourceValue]));
            break;
        }
      } else if (this.isObject(sourceValue) && this.isObject(targetValue)) {
        // Handle objects
        switch (strategy.objects) {
          case 'replace':
            result[key] = sourceValue;
            break;
          case 'merge':
            result[key] = { ...targetValue, ...sourceValue };
            break;
          case 'deep-merge':
            result[key] = this.mergeConfigs(sourceValue, targetValue, strategy);
            break;
        }
      } else {
        // Handle conflicts
        switch (strategy.conflicts) {
          case 'error':
            throw new Error(`Merge conflict at key: ${key}`);
          case 'prefer-source':
            result[key] = sourceValue;
            break;
          case 'prefer-target':
            result[key] = targetValue;
            break;
          case 'custom':
            if (strategy.customResolver) {
              result[key] = strategy.customResolver(key, sourceValue, targetValue);
            } else {
              result[key] = sourceValue;
            }
            break;
        }
      }
    }

    return result;
  }

  private isObject(value: any): boolean {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
  }
}