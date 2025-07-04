/**
 * Configuration Providers - Load config from various sources
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import {
  ConfigProvider,
  OrchestrationConfig
} from './config-interfaces';

/**
 * Environment Variable Configuration Provider
 */
export class EnvironmentConfigProvider implements ConfigProvider {
  name = 'environment';
  priority = 100; // High priority

  constructor(
    private prefix: string = 'SYMBIOTE_',
    private mapping?: Record<string, string>
  ) {}

  async load(): Promise<Partial<OrchestrationConfig>> {
    const config: any = {};

    // Process environment variables
    for (const [key, value] of Object.entries(process.env)) {
      if (key.startsWith(this.prefix)) {
        const configPath = this.envVarToPath(key);
        this.setNestedValue(config, configPath, this.parseValue(value));
      }
    }

    // Apply custom mappings
    if (this.mapping) {
      for (const [envVar, configPath] of Object.entries(this.mapping)) {
        const value = process.env[envVar];
        if (value !== undefined) {
          this.setNestedValue(config, configPath, this.parseValue(value));
        }
      }
    }

    return config;
  }

  private envVarToPath(envVar: string): string {
    // Remove prefix and convert to path
    // SYMBIOTE_ROUTING_DEFAULT_MODEL -> routing.defaultModel
    return envVar
      .slice(this.prefix.length)
      .toLowerCase()
      .replace(/_/g, '.')
      .replace(/([a-z])([A-Z])/g, '$1.$2')
      .toLowerCase();
  }

  private setNestedValue(obj: any, path: string, value: any): void {
    const keys = path.split('.');
    let current = obj;

    for (let i = 0; i < keys.length - 1; i++) {
      const key = keys[i];
      if (!(key in current)) {
        current[key] = {};
      }
      current = current[key];
    }

    current[keys[keys.length - 1]] = value;
  }

  private parseValue(value: string): any {
    // Try to parse as JSON
    try {
      return JSON.parse(value);
    } catch {
      // Try to parse as number
      const num = Number(value);
      if (!isNaN(num)) {
        return num;
      }

      // Try to parse as boolean
      if (value.toLowerCase() === 'true') return true;
      if (value.toLowerCase() === 'false') return false;

      // Return as string
      return value;
    }
  }
}

/**
 * JSON File Configuration Provider
 */
export class JsonFileConfigProvider implements ConfigProvider {
  name = 'json-file';
  priority = 50;

  constructor(
    private filePath: string,
    private optional: boolean = false
  ) {}

  async load(): Promise<Partial<OrchestrationConfig>> {
    try {
      const content = await fs.readFile(this.filePath, 'utf-8');
      return JSON.parse(content);
    } catch (error: any) {
      if (error.code === 'ENOENT' && this.optional) {
        return {};
      }
      throw error;
    }
  }

  watch(callback: (config: Partial<OrchestrationConfig>) => void): void {
    // Use fs.watch for simple file watching
    fs.watch(this.filePath).then(watcher => {
      watcher.on('change', async () => {
        try {
          const config = await this.load();
          callback(config);
        } catch (error) {
          console.error(`Failed to reload config from ${this.filePath}:`, error);
        }
      });
    });
  }
}

/**
 * Remote Configuration Provider (e.g., from API)
 */
export class RemoteConfigProvider implements ConfigProvider {
  name = 'remote';
  priority = 75;
  private pollInterval?: NodeJS.Timeout;

  constructor(
    private url: string,
    private options?: {
      headers?: Record<string, string>;
      pollInterval?: number;
      timeout?: number;
    }
  ) {}

  async load(): Promise<Partial<OrchestrationConfig>> {
    const controller = new AbortController();
    const timeout = this.options?.timeout || 30000;

    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
      const response = await fetch(this.url, {
        headers: this.options?.headers,
        signal: controller.signal
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const config = await response.json();
      return config;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  watch(callback: (config: Partial<OrchestrationConfig>) => void): void {
    if (!this.options?.pollInterval) return;

    this.pollInterval = setInterval(async () => {
      try {
        const config = await this.load();
        callback(config);
      } catch (error) {
        console.error('Failed to poll remote config:', error);
      }
    }, this.options.pollInterval);
  }

  unwatch(): void {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = undefined;
    }
  }
}

/**
 * Command Line Arguments Configuration Provider
 */
export class CliArgsConfigProvider implements ConfigProvider {
  name = 'cli-args';
  priority = 200; // Highest priority

  constructor(
    private args: string[] = process.argv.slice(2),
    private mapping?: Record<string, string>
  ) {}

  async load(): Promise<Partial<OrchestrationConfig>> {
    const config: any = {};

    for (let i = 0; i < this.args.length; i++) {
      const arg = this.args[i];

      if (arg.startsWith('--config.')) {
        // --config.routing.defaultModel=gpt-4
        const [path, value] = arg.slice(9).split('=');
        if (value) {
          this.setNestedValue(config, path, this.parseValue(value));
        }
      } else if (this.mapping && arg.startsWith('--')) {
        // Check custom mappings
        const flag = arg.slice(2);
        const configPath = this.mapping[flag];
        
        if (configPath) {
          const nextArg = this.args[i + 1];
          if (nextArg && !nextArg.startsWith('--')) {
            this.setNestedValue(config, configPath, this.parseValue(nextArg));
            i++; // Skip next arg
          }
        }
      }
    }

    return config;
  }

  private setNestedValue(obj: any, path: string, value: any): void {
    const keys = path.split('.');
    let current = obj;

    for (let i = 0; i < keys.length - 1; i++) {
      const key = keys[i];
      if (!(key in current)) {
        current[key] = {};
      }
      current = current[key];
    }

    current[keys[keys.length - 1]] = value;
  }

  private parseValue(value: string): any {
    // Try to parse as JSON
    try {
      return JSON.parse(value);
    } catch {
      // Try to parse as number
      const num = Number(value);
      if (!isNaN(num)) {
        return num;
      }

      // Try to parse as boolean
      if (value.toLowerCase() === 'true') return true;
      if (value.toLowerCase() === 'false') return false;

      // Return as string
      return value;
    }
  }
}

/**
 * Consul/etcd Configuration Provider
 */
export class KeyValueStoreConfigProvider implements ConfigProvider {
  name = 'kv-store';
  priority = 80;
  private watchHandle?: any;

  constructor(
    private client: any, // Consul or etcd client
    private keyPrefix: string,
    private type: 'consul' | 'etcd' = 'consul'
  ) {}

  async load(): Promise<Partial<OrchestrationConfig>> {
    if (this.type === 'consul') {
      return this.loadFromConsul();
    } else {
      return this.loadFromEtcd();
    }
  }

  watch(callback: (config: Partial<OrchestrationConfig>) => void): void {
    if (this.type === 'consul') {
      this.watchConsul(callback);
    } else {
      this.watchEtcd(callback);
    }
  }

  unwatch(): void {
    if (this.watchHandle) {
      if (this.type === 'consul') {
        // Cancel consul watch
      } else {
        // Cancel etcd watch
        this.watchHandle.cancel();
      }
      this.watchHandle = undefined;
    }
  }

  private async loadFromConsul(): Promise<Partial<OrchestrationConfig>> {
    const keys = await this.client.kv.keys(this.keyPrefix);
    const config: any = {};

    for (const key of keys) {
      const result = await this.client.kv.get(key);
      if (result) {
        const path = key.slice(this.keyPrefix.length).replace(/\//g, '.');
        const value = JSON.parse(result.Value);
        this.setNestedValue(config, path, value);
      }
    }

    return config;
  }

  private async loadFromEtcd(): Promise<Partial<OrchestrationConfig>> {
    const response = await this.client.getAll().prefix(this.keyPrefix);
    const config: any = {};

    for (const [key, value] of response.kvs) {
      const path = key.slice(this.keyPrefix.length).replace(/\//g, '.');
      const parsedValue = JSON.parse(value);
      this.setNestedValue(config, path, parsedValue);
    }

    return config;
  }

  private watchConsul(callback: (config: Partial<OrchestrationConfig>) => void): void {
    // Implement consul watch
    // This is a simplified version
    setInterval(async () => {
      try {
        const config = await this.load();
        callback(config);
      } catch (error) {
        console.error('Consul watch error:', error);
      }
    }, 5000);
  }

  private watchEtcd(callback: (config: Partial<OrchestrationConfig>) => void): void {
    this.watchHandle = this.client
      .watch()
      .prefix(this.keyPrefix)
      .create();

    this.watchHandle.on('put', async () => {
      const config = await this.load();
      callback(config);
    });

    this.watchHandle.on('delete', async () => {
      const config = await this.load();
      callback(config);
    });
  }

  private setNestedValue(obj: any, path: string, value: any): void {
    const keys = path.split('.');
    let current = obj;

    for (let i = 0; i < keys.length - 1; i++) {
      const key = keys[i];
      if (!(key in current)) {
        current[key] = {};
      }
      current = current[key];
    }

    current[keys[keys.length - 1]] = value;
  }
}

/**
 * Composite Configuration Provider
 * Combines multiple providers with priority ordering
 */
export class CompositeConfigProvider implements ConfigProvider {
  name = 'composite';
  priority = 0;

  constructor(private providers: ConfigProvider[]) {
    // Sort by priority
    this.providers.sort((a, b) => b.priority - a.priority);
  }

  async load(): Promise<Partial<OrchestrationConfig>> {
    let config: Partial<OrchestrationConfig> = {};

    // Load from providers in priority order
    for (const provider of this.providers) {
      try {
        const providerConfig = await provider.load();
        config = this.deepMerge(config, providerConfig);
      } catch (error) {
        console.error(`Failed to load from provider ${provider.name}:`, error);
      }
    }

    return config;
  }

  watch(callback: (config: Partial<OrchestrationConfig>) => void): void {
    for (const provider of this.providers) {
      if (provider.watch) {
        provider.watch(async () => {
          const config = await this.load();
          callback(config);
        });
      }
    }
  }

  unwatch(): void {
    for (const provider of this.providers) {
      if (provider.unwatch) {
        provider.unwatch();
      }
    }
  }

  private deepMerge(target: any, source: any): any {
    if (!source || typeof source !== 'object') {
      return source;
    }

    const result = { ...target };

    for (const key in source) {
      if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
        result[key] = this.deepMerge(result[key] || {}, source[key]);
      } else {
        result[key] = source[key];
      }
    }

    return result;
  }
}