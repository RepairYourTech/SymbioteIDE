/**
 * Configuration Validator
 */

import {
  OrchestrationConfig,
  ConfigValidationResult,
  ConfigValidationError,
  ConfigValidationWarning,
  ConfigVariable,
  ConfigVariableValidation
} from './config-interfaces';

export class ConfigValidator {
  private rules: ValidationRule[] = [];

  constructor() {
    this.initializeDefaultRules();
  }

  /**
   * Validate a configuration object
   */
  validate(config: any): ConfigValidationResult {
    const errors: ConfigValidationError[] = [];
    const warnings: ConfigValidationWarning[] = [];

    // Check if config is an object
    if (!config || typeof config !== 'object') {
      errors.push({
        path: '',
        message: 'Configuration must be an object',
        value: config
      });
      return { valid: false, errors, warnings };
    }

    // Apply all validation rules
    for (const rule of this.rules) {
      const result = rule.validate(config);
      errors.push(...result.errors);
      warnings.push(...result.warnings);
    }

    // Validate specific sections
    this.validateRouting(config.routing, errors, warnings);
    this.validateModels(config.models, errors, warnings);
    this.validateCost(config.cost, errors, warnings);
    this.validateCache(config.cache, errors, warnings);
    this.validateQueue(config.queue, errors, warnings);
    this.validatePerformance(config.performance, errors, warnings);
    this.validateMonitoring(config.monitoring, errors, warnings);
    this.validateSecurity(config.security, errors, warnings);
    this.validateFeatures(config.features, errors, warnings);

    return {
      valid: errors.length === 0,
      errors,
      warnings
    };
  }

  /**
   * Validate configuration variables
   */
  validateVariables(
    values: Record<string, any>,
    variables: ConfigVariable[]
  ): ConfigValidationResult {
    const errors: ConfigValidationError[] = [];
    const warnings: ConfigValidationWarning[] = [];

    for (const variable of variables) {
      const value = values[variable.name];

      // Check required
      if (variable.required && value === undefined) {
        errors.push({
          path: variable.name,
          message: `Required variable '${variable.name}' is missing`,
          rule: 'required'
        });
        continue;
      }

      // Skip validation if not provided and not required
      if (value === undefined) {
        continue;
      }

      // Check type
      if (!this.validateType(value, variable.type)) {
        errors.push({
          path: variable.name,
          message: `Variable '${variable.name}' must be of type ${variable.type}`,
          value,
          rule: 'type'
        });
        continue;
      }

      // Apply custom validation
      if (variable.validation) {
        this.validateVariableValue(
          variable.name,
          value,
          variable.validation,
          errors,
          warnings
        );
      }
    }

    return { valid: errors.length === 0, errors, warnings };
  }

  /**
   * Add custom validation rule
   */
  addRule(rule: ValidationRule): void {
    this.rules.push(rule);
  }

  /**
   * Remove validation rule
   */
  removeRule(ruleName: string): void {
    this.rules = this.rules.filter(r => r.name !== ruleName);
  }

  // Private validation methods

  private initializeDefaultRules(): void {
    // Version format rule
    this.rules.push({
      name: 'version-format',
      validate: (config) => {
        const errors: ConfigValidationError[] = [];
        const warnings: ConfigValidationWarning[] = [];

        if (config.version && !this.isValidVersion(config.version)) {
          errors.push({
            path: 'version',
            message: 'Version must follow semantic versioning (e.g., 1.0.0)',
            value: config.version,
            rule: 'version-format'
          });
        }

        return { errors, warnings };
      }
    });

    // Environment rule
    this.rules.push({
      name: 'environment',
      validate: (config) => {
        const errors: ConfigValidationError[] = [];
        const warnings: ConfigValidationWarning[] = [];

        const validEnvironments = ['development', 'staging', 'production'];
        if (config.environment && !validEnvironments.includes(config.environment)) {
          errors.push({
            path: 'environment',
            message: `Environment must be one of: ${validEnvironments.join(', ')}`,
            value: config.environment,
            rule: 'environment'
          });
        }

        return { errors, warnings };
      }
    });
  }

  private validateRouting(
    routing: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!routing) return;

    // Validate scoring weights
    if (routing.scoringWeights) {
      const weights = routing.scoringWeights;
      const totalWeight = Object.values(weights)
        .filter((w): w is number => typeof w === 'number')
        .reduce((sum, w) => sum + w, 0);

      if (Math.abs(totalWeight - 1.0) > 0.001) {
        warnings.push({
          path: 'routing.scoringWeights',
          message: `Scoring weights should sum to 1.0 (current: ${totalWeight})`,
          suggestion: 'Adjust weights to sum to 1.0 for normalized scoring'
        });
      }
    }

    // Validate timeout ranges
    if (routing.timeouts) {
      for (const [model, timeout] of Object.entries(routing.timeouts)) {
        if (typeof timeout === 'number' && timeout < 1000) {
          warnings.push({
            path: `routing.timeouts.${model}`,
            message: `Timeout for ${model} is very low (${timeout}ms)`,
            suggestion: 'Consider increasing timeout to at least 1000ms'
          });
        }
      }
    }
  }

  private validateModels(
    models: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!models) return;

    // Validate model overrides
    if (models.overrides && Array.isArray(models.overrides)) {
      models.overrides.forEach((override: any, index: number) => {
        if (!override.modelId) {
          errors.push({
            path: `models.overrides[${index}].modelId`,
            message: 'Model override must specify modelId',
            value: override,
            rule: 'required'
          });
        }
      });
    }

    // Validate model defaults
    if (models.defaults) {
      const defaults = models.defaults;
      
      if (defaults.temperature !== undefined) {
        if (defaults.temperature < 0 || defaults.temperature > 2) {
          errors.push({
            path: 'models.defaults.temperature',
            message: 'Temperature must be between 0 and 2',
            value: defaults.temperature,
            rule: 'range'
          });
        }
      }

      if (defaults.topP !== undefined) {
        if (defaults.topP < 0 || defaults.topP > 1) {
          errors.push({
            path: 'models.defaults.topP',
            message: 'TopP must be between 0 and 1',
            value: defaults.topP,
            rule: 'range'
          });
        }
      }
    }
  }

  private validateCost(
    cost: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!cost) return;

    // Validate cost limits
    if (cost.limits && Array.isArray(cost.limits)) {
      cost.limits.forEach((limit: any, index: number) => {
        if (!limit.amount || limit.amount <= 0) {
          errors.push({
            path: `cost.limits[${index}].amount`,
            message: 'Cost limit amount must be positive',
            value: limit.amount,
            rule: 'positive'
          });
        }

        if (!limit.period) {
          errors.push({
            path: `cost.limits[${index}].period`,
            message: 'Cost limit must specify period',
            value: limit,
            rule: 'required'
          });
        }
      });
    }

    // Validate cost alerts
    if (cost.alerts && Array.isArray(cost.alerts)) {
      cost.alerts.forEach((alert: any, index: number) => {
        if (!alert.threshold || alert.threshold <= 0) {
          errors.push({
            path: `cost.alerts[${index}].threshold`,
            message: 'Alert threshold must be positive',
            value: alert.threshold,
            rule: 'positive'
          });
        }

        const validActions = ['log', 'email', 'webhook', 'throttle', 'block'];
        if (!validActions.includes(alert.action)) {
          errors.push({
            path: `cost.alerts[${index}].action`,
            message: `Alert action must be one of: ${validActions.join(', ')}`,
            value: alert.action,
            rule: 'enum'
          });
        }
      });
    }
  }

  private validateCache(
    cache: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!cache) return;

    if (cache.ttl !== undefined && cache.ttl < 0) {
      errors.push({
        path: 'cache.ttl',
        message: 'Cache TTL must be non-negative',
        value: cache.ttl,
        rule: 'non-negative'
      });
    }

    if (cache.maxSize !== undefined && cache.maxSize <= 0) {
      errors.push({
        path: 'cache.maxSize',
        message: 'Cache max size must be positive',
        value: cache.maxSize,
        rule: 'positive'
      });
    }

    if (cache.strategy) {
      const validStrategies = ['lru', 'lfu', 'ttl', 'adaptive'];
      if (!validStrategies.includes(cache.strategy.type)) {
        errors.push({
          path: 'cache.strategy.type',
          message: `Cache strategy must be one of: ${validStrategies.join(', ')}`,
          value: cache.strategy.type,
          rule: 'enum'
        });
      }
    }
  }

  private validateQueue(
    queue: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!queue) return;

    if (queue.maxConcurrency !== undefined) {
      if (queue.maxConcurrency <= 0) {
        errors.push({
          path: 'queue.maxConcurrency',
          message: 'Max concurrency must be positive',
          value: queue.maxConcurrency,
          rule: 'positive'
        });
      } else if (queue.maxConcurrency > 100) {
        warnings.push({
          path: 'queue.maxConcurrency',
          message: `Very high concurrency (${queue.maxConcurrency}) may cause issues`,
          suggestion: 'Consider using a value between 1 and 100'
        });
      }
    }

    if (queue.maxQueueSize !== undefined && queue.maxQueueSize <= 0) {
      errors.push({
        path: 'queue.maxQueueSize',
        message: 'Max queue size must be positive',
        value: queue.maxQueueSize,
        rule: 'positive'
      });
    }

    // Validate rate limits
    if (queue.rateLimits && Array.isArray(queue.rateLimits)) {
      queue.rateLimits.forEach((limit: any, index: number) => {
        if (!limit.name) {
          errors.push({
            path: `queue.rateLimits[${index}].name`,
            message: 'Rate limit must have a name',
            value: limit,
            rule: 'required'
          });
        }

        if (!limit.limit || limit.limit <= 0) {
          errors.push({
            path: `queue.rateLimits[${index}].limit`,
            message: 'Rate limit must be positive',
            value: limit.limit,
            rule: 'positive'
          });
        }

        if (!limit.window || limit.window <= 0) {
          errors.push({
            path: `queue.rateLimits[${index}].window`,
            message: 'Rate limit window must be positive',
            value: limit.window,
            rule: 'positive'
          });
        }
      });
    }
  }

  private validatePerformance(
    performance: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!performance) return;

    if (performance.defaultTimeout !== undefined) {
      if (performance.defaultTimeout <= 0) {
        errors.push({
          path: 'performance.defaultTimeout',
          message: 'Default timeout must be positive',
          value: performance.defaultTimeout,
          rule: 'positive'
        });
      } else if (performance.defaultTimeout < 1000) {
        warnings.push({
          path: 'performance.defaultTimeout',
          message: `Very low timeout (${performance.defaultTimeout}ms) may cause failures`,
          suggestion: 'Consider using at least 1000ms'
        });
      }
    }

    if (performance.defaultRetries !== undefined) {
      if (performance.defaultRetries < 0) {
        errors.push({
          path: 'performance.defaultRetries',
          message: 'Default retries must be non-negative',
          value: performance.defaultRetries,
          rule: 'non-negative'
        });
      } else if (performance.defaultRetries > 10) {
        warnings.push({
          path: 'performance.defaultRetries',
          message: `High retry count (${performance.defaultRetries}) may cause delays`,
          suggestion: 'Consider using 3-5 retries'
        });
      }
    }

    // Validate circuit breaker
    if (performance.circuitBreaker) {
      const cb = performance.circuitBreaker;
      
      if (cb.failureThreshold !== undefined && cb.failureThreshold <= 0) {
        errors.push({
          path: 'performance.circuitBreaker.failureThreshold',
          message: 'Failure threshold must be positive',
          value: cb.failureThreshold,
          rule: 'positive'
        });
      }

      if (cb.resetTimeout !== undefined && cb.resetTimeout <= 0) {
        errors.push({
          path: 'performance.circuitBreaker.resetTimeout',
          message: 'Reset timeout must be positive',
          value: cb.resetTimeout,
          rule: 'positive'
        });
      }
    }
  }

  private validateMonitoring(
    monitoring: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!monitoring) return;

    if (monitoring.metricsInterval !== undefined && monitoring.metricsInterval <= 0) {
      errors.push({
        path: 'monitoring.metricsInterval',
        message: 'Metrics interval must be positive',
        value: monitoring.metricsInterval,
        rule: 'positive'
      });
    }

    // Validate logging
    if (monitoring.logging) {
      const validLevels = ['debug', 'info', 'warn', 'error'];
      if (!validLevels.includes(monitoring.logging.level)) {
        errors.push({
          path: 'monitoring.logging.level',
          message: `Log level must be one of: ${validLevels.join(', ')}`,
          value: monitoring.logging.level,
          rule: 'enum'
        });
      }

      const validFormats = ['json', 'text', 'structured'];
      if (!validFormats.includes(monitoring.logging.format)) {
        errors.push({
          path: 'monitoring.logging.format',
          message: `Log format must be one of: ${validFormats.join(', ')}`,
          value: monitoring.logging.format,
          rule: 'enum'
        });
      }
    }
  }

  private validateSecurity(
    security: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!security) return;

    // Validate API keys
    if (security.apiKeys && Array.isArray(security.apiKeys)) {
      security.apiKeys.forEach((apiKey: any, index: number) => {
        if (!apiKey.provider) {
          errors.push({
            path: `security.apiKeys[${index}].provider`,
            message: 'API key must specify provider',
            value: apiKey,
            rule: 'required'
          });
        }

        if (!apiKey.key && !apiKey.keyRef) {
          errors.push({
            path: `security.apiKeys[${index}]`,
            message: 'API key must specify either key or keyRef',
            value: apiKey,
            rule: 'required'
          });
        }

        if (apiKey.key && apiKey.keyRef) {
          warnings.push({
            path: `security.apiKeys[${index}]`,
            message: 'Both key and keyRef specified, keyRef will be preferred',
            suggestion: 'Use only keyRef for better security'
          });
        }

        if (apiKey.key) {
          warnings.push({
            path: `security.apiKeys[${index}].key`,
            message: 'Storing API keys in config is not recommended',
            suggestion: 'Use keyRef to reference environment variables instead'
          });
        }
      });
    }

    // Validate encryption
    if (security.encryption) {
      const validKeyManagement = ['local', 'kms', 'vault'];
      if (!validKeyManagement.includes(security.encryption.keyManagement)) {
        errors.push({
          path: 'security.encryption.keyManagement',
          message: `Key management must be one of: ${validKeyManagement.join(', ')}`,
          value: security.encryption.keyManagement,
          rule: 'enum'
        });
      }
    }
  }

  private validateFeatures(
    features: any,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    if (!features) return;

    // Features can be arbitrary key-value pairs
    // Just validate the structure
    for (const [key, value] of Object.entries(features)) {
      if (typeof value === 'object' && value !== null) {
        const feature = value as any;
        if ('enabled' in feature && typeof feature.enabled !== 'boolean') {
          errors.push({
            path: `features.${key}.enabled`,
            message: 'Feature enabled flag must be boolean',
            value: feature.enabled,
            rule: 'type'
          });
        }

        if (feature.rollout) {
          if (feature.rollout.percentage !== undefined) {
            if (feature.rollout.percentage < 0 || feature.rollout.percentage > 100) {
              errors.push({
                path: `features.${key}.rollout.percentage`,
                message: 'Rollout percentage must be between 0 and 100',
                value: feature.rollout.percentage,
                rule: 'range'
              });
            }
          }
        }
      }
    }
  }

  private validateType(value: any, type: string): boolean {
    switch (type) {
      case 'string':
        return typeof value === 'string';
      case 'number':
        return typeof value === 'number' && !isNaN(value);
      case 'boolean':
        return typeof value === 'boolean';
      case 'object':
        return typeof value === 'object' && value !== null && !Array.isArray(value);
      case 'array':
        return Array.isArray(value);
      default:
        return true;
    }
  }

  private validateVariableValue(
    name: string,
    value: any,
    validation: ConfigVariableValidation,
    errors: ConfigValidationError[],
    warnings: ConfigValidationWarning[]
  ): void {
    // Min/max validation for numbers
    if (typeof value === 'number') {
      if (validation.min !== undefined && value < validation.min) {
        errors.push({
          path: name,
          message: `Value must be at least ${validation.min}`,
          value,
          rule: 'min'
        });
      }
      if (validation.max !== undefined && value > validation.max) {
        errors.push({
          path: name,
          message: `Value must be at most ${validation.max}`,
          value,
          rule: 'max'
        });
      }
    }

    // Pattern validation for strings
    if (typeof value === 'string' && validation.pattern) {
      const regex = new RegExp(validation.pattern);
      if (!regex.test(value)) {
        errors.push({
          path: name,
          message: `Value must match pattern: ${validation.pattern}`,
          value,
          rule: 'pattern'
        });
      }
    }

    // Enum validation
    if (validation.enum && !validation.enum.includes(value)) {
      errors.push({
        path: name,
        message: `Value must be one of: ${validation.enum.join(', ')}`,
        value,
        rule: 'enum'
      });
    }

    // Custom validation
    if (validation.custom && !validation.custom(value)) {
      errors.push({
        path: name,
        message: `Value failed custom validation`,
        value,
        rule: 'custom'
      });
    }
  }

  private isValidVersion(version: string): boolean {
    // Simple semantic versioning check
    const semverRegex = /^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$/;
    return semverRegex.test(version);
  }
}

interface ValidationRule {
  name: string;
  validate(config: any): {
    errors: ConfigValidationError[];
    warnings: ConfigValidationWarning[];
  };
}