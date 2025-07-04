/**
 * Configuration Manager Tests
 */

import { 
  ConfigManager,
  ConfigValidator,
  InMemoryConfigStore,
  EnvironmentConfigProvider,
  JsonFileConfigProvider,
  applyTemplate
} from '../index';
import { OrchestrationConfig } from '../config-interfaces';

describe('Configuration Manager', () => {
  let configManager: ConfigManager;

  beforeEach(() => {
    configManager = new ConfigManager({
      store: new InMemoryConfigStore()
    });
  });

  afterEach(async () => {
    await configManager.shutdown();
  });

  it('should initialize with default configuration', async () => {
    await configManager.initialize();
    const config = configManager.getConfig();
    expect(config).toBeDefined();
  });

  it('should get and set configuration values', async () => {
    await configManager.initialize();
    
    // Set a value
    await configManager.set('routing.defaultModel', 'gpt-4');
    
    // Get the value
    const model = configManager.get('routing.defaultModel');
    expect(model).toBe('gpt-4');
    
    // Get with default
    const missing = configManager.get('routing.missing', 'default');
    expect(missing).toBe('default');
  });

  it('should validate configuration', async () => {
    await configManager.initialize();
    
    // Valid config
    const validConfig: OrchestrationConfig = {
      environment: 'development',
      routing: {
        defaultModel: 'gpt-3.5-turbo'
      }
    };
    
    const validation = configManager.validate(validConfig);
    expect(validation.valid).toBe(true);
    
    // Invalid config
    const invalidConfig: any = {
      environment: 'invalid-env',
      cache: {
        ttl: -1000
      }
    };
    
    const invalidValidation = configManager.validate(invalidConfig);
    expect(invalidValidation.valid).toBe(false);
    expect(invalidValidation.errors.length).toBeGreaterThan(0);
  });

  it('should merge configurations', () => {
    const source = {
      routing: {
        defaultModel: 'gpt-4',
        scoringWeights: {
          cost: 0.5
        }
      }
    };
    
    const target = {
      routing: {
        defaultModel: 'gpt-3.5-turbo',
        scoringWeights: {
          quality: 0.5
        }
      },
      cache: {
        enabled: true
      }
    };
    
    const merged = configManager.merge(source, target);
    
    expect(merged.routing.defaultModel).toBe('gpt-4');
    expect(merged.routing.scoringWeights.cost).toBe(0.5);
    expect(merged.routing.scoringWeights.quality).toBe(0.5);
    expect(merged.cache.enabled).toBe(true);
  });

  it('should calculate configuration diff', () => {
    const config1 = {
      routing: {
        defaultModel: 'gpt-3.5-turbo'
      },
      cache: {
        enabled: true
      }
    };
    
    const config2 = {
      routing: {
        defaultModel: 'gpt-4'
      },
      cache: {
        enabled: true,
        ttl: 3600
      }
    };
    
    const diff = configManager.diff(config1, config2);
    
    expect(diff.changed['routing.defaultModel']).toEqual({
      old: 'gpt-3.5-turbo',
      new: 'gpt-4'
    });
    expect(diff.added['cache.ttl']).toBe(3600);
    expect(diff.unchanged).toContain('cache.enabled');
  });

  it('should handle configuration providers', async () => {
    // Add environment provider
    const envProvider = new EnvironmentConfigProvider('TEST_');
    process.env.TEST_ROUTING_DEFAULT_MODEL = 'claude-3-opus';
    
    configManager.addProvider(envProvider);
    await configManager.initialize();
    
    const config = configManager.getConfig();
    expect(config.routing?.defaultModel).toBe('claude-3-opus');
    
    // Clean up
    delete process.env.TEST_ROUTING_DEFAULT_MODEL;
  });

  it('should emit change events', async () => {
    await configManager.initialize();
    
    let changeEvent: any = null;
    configManager.on('change', (event) => {
      changeEvent = event;
    });
    
    await configManager.set('routing.defaultModel', 'gpt-4');
    
    expect(changeEvent).not.toBeNull();
    expect(changeEvent.newConfig.routing.defaultModel).toBe('gpt-4');
  });
});

describe('Configuration Validator', () => {
  let validator: ConfigValidator;

  beforeEach(() => {
    validator = new ConfigValidator();
  });

  it('should validate version format', () => {
    const validConfig = { version: '1.0.0' };
    const invalidConfig = { version: 'invalid' };
    
    const validResult = validator.validate(validConfig);
    expect(validResult.valid).toBe(true);
    
    const invalidResult = validator.validate(invalidConfig);
    expect(invalidResult.valid).toBe(false);
    expect(invalidResult.errors[0].rule).toBe('version-format');
  });

  it('should validate numeric ranges', () => {
    const config = {
      cache: {
        ttl: -1000,
        maxSize: 0
      },
      performance: {
        defaultTimeout: 0,
        defaultRetries: -5
      }
    };
    
    const result = validator.validate(config);
    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.path === 'cache.ttl')).toBe(true);
    expect(result.errors.some(e => e.path === 'cache.maxSize')).toBe(true);
  });

  it('should provide warnings', () => {
    const config = {
      routing: {
        scoringWeights: {
          cost: 0.4,
          quality: 0.3,
          latency: 0.1
          // Sum is 0.8, not 1.0
        }
      },
      performance: {
        defaultTimeout: 500 // Very low
      }
    };
    
    const result = validator.validate(config);
    expect(result.valid).toBe(true);
    expect(result.warnings.length).toBeGreaterThan(0);
    expect(result.warnings.some(w => w.path.includes('scoringWeights'))).toBe(true);
  });
});

describe('Configuration Templates', () => {
  it('should apply development template', () => {
    const config = applyTemplate('development', {
      apiKey: 'test-key'
    });
    
    expect(config.environment).toBe('development');
    expect(config.cache?.enabled).toBe(true);
    expect(config.monitoring?.logging?.level).toBe('debug');
  });

  it('should apply production template with variables', () => {
    const config = applyTemplate('production', {
      REDIS_HOST: 'redis.example.com',
      OTLP_ENDPOINT: 'http://collector:4317'
    });
    
    expect(config.environment).toBe('production');
    expect(config.cache?.providers?.[0].config?.host).toBe('redis.example.com');
    expect(config.monitoring?.tracing?.exporter?.endpoint).toBe('http://collector:4317');
  });

  it('should handle variable replacement', () => {
    const config = applyTemplate('cost-optimized', {});
    
    expect(config.routing?.scoringWeights?.cost).toBe(0.7);
    expect(config.cost?.limits?.[0].amount).toBe(100);
  });
});

// Run tests if this file is executed directly
if (require.main === module) {
  console.log('Running configuration manager tests...');
  // In a real environment, we would use Jest or another test runner
}