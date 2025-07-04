/**
 * Configuration Templates for common scenarios
 */

import {
  ConfigTemplate,
  ConfigTemplateCategory,
  OrchestrationConfig,
  RequestPriority
} from './config-interfaces';

export const configTemplates: ConfigTemplate[] = [
  {
    id: 'development',
    name: 'Development Configuration',
    description: 'Optimized for local development with debugging and fast iteration',
    category: ConfigTemplateCategory.Development,
    config: {
      environment: 'development',
      routing: {
        defaultModel: 'gpt-3.5-turbo',
        scoringWeights: {
          cost: 0.4,
          quality: 0.3,
          latency: 0.2,
          capabilities: 0.1
        }
      },
      cost: {
        limits: [
          {
            amount: 10,
            period: 'day',
            scope: 'total'
          }
        ],
        alerts: [
          {
            threshold: 8,
            type: 'total',
            action: 'log'
          }
        ]
      },
      cache: {
        enabled: true,
        ttl: 3600000, // 1 hour
        maxSize: 100 * 1024 * 1024, // 100MB
        strategy: {
          type: 'lru'
        }
      },
      queue: {
        maxConcurrency: 5,
        maxQueueSize: 100,
        defaultTimeout: 30000,
        maxRetries: 3,
        enablePersistence: false,
        enableDeduplication: true,
        enableDeadLetter: true
      },
      performance: {
        defaultTimeout: 30000,
        defaultRetries: 3,
        circuitBreaker: {
          enabled: false
        }
      },
      monitoring: {
        metricsInterval: 60000,
        logging: {
          level: 'debug',
          format: 'text',
          destinations: [
            { type: 'console' }
          ]
        }
      }
    },
    variables: [
      {
        name: 'apiKey',
        type: 'string',
        required: true,
        description: 'API key for the default model provider'
      }
    ]
  },
  {
    id: 'production',
    name: 'Production Configuration',
    description: 'Optimized for production with reliability, monitoring, and cost control',
    category: ConfigTemplateCategory.Production,
    config: {
      environment: 'production',
      routing: {
        defaultModel: 'gpt-3.5-turbo',
        fallbackChain: ['claude-3-haiku-20240307', 'gpt-4o-mini'],
        scoringWeights: {
          cost: 0.3,
          quality: 0.4,
          latency: 0.2,
          capabilities: 0.1
        }
      },
      cost: {
        limits: [
          {
            amount: 1000,
            period: 'day',
            scope: 'total'
          },
          {
            amount: 100,
            period: 'day',
            scope: 'per-user'
          }
        ],
        alerts: [
          {
            threshold: 800,
            type: 'total',
            action: 'email'
          },
          {
            threshold: 950,
            type: 'total',
            action: 'throttle'
          }
        ],
        tracking: {
          granularity: 'request',
          retention: 30
        }
      },
      cache: {
        enabled: true,
        ttl: 7200000, // 2 hours
        maxSize: 500 * 1024 * 1024, // 500MB
        strategy: {
          type: 'adaptive'
        },
        providers: [
          {
            type: 'redis',
            priority: 1,
            config: {
              host: '${REDIS_HOST}',
              port: 6379
            }
          },
          {
            type: 'memory',
            priority: 2
          }
        ]
      },
      queue: {
        maxConcurrency: 20,
        maxQueueSize: 1000,
        defaultTimeout: 60000,
        maxRetries: 3,
        enablePersistence: true,
        enableDeduplication: true,
        enableDeadLetter: true,
        rateLimits: [
          {
            name: 'global',
            limit: 1000,
            window: 60000,
            scope: 'global'
          },
          {
            name: 'per-user',
            limit: 100,
            window: 60000,
            scope: 'user'
          }
        ],
        backpressure: {
          highWaterMark: 80,
          lowWaterMark: 60,
          strategy: 'throttle' as any,
          rejectOnFull: true
        }
      },
      performance: {
        defaultTimeout: 60000,
        defaultRetries: 3,
        concurrency: {
          maxParallel: 50,
          maxPerModel: {
            'gpt-4': 10,
            'claude-3-opus-20240229': 5
          }
        },
        circuitBreaker: {
          enabled: true,
          failureThreshold: 5,
          resetTimeout: 60000,
          halfOpenRequests: 2
        }
      },
      monitoring: {
        metricsInterval: 30000,
        exporters: [
          {
            type: 'prometheus',
            endpoint: '/metrics',
            interval: 30000
          }
        ],
        logging: {
          level: 'info',
          format: 'json',
          destinations: [
            {
              type: 'console'
            },
            {
              type: 'file',
              config: {
                path: '/var/log/symbiote/orchestration.log',
                maxSize: '100m',
                maxFiles: 10
              }
            }
          ]
        },
        tracing: {
          enabled: true,
          sampler: {
            type: 'probability',
            config: {
              probability: 0.1
            }
          },
          exporter: {
            type: 'otlp',
            endpoint: '${OTLP_ENDPOINT}'
          }
        }
      },
      security: {
        apiKeys: [
          {
            provider: 'openai',
            keyRef: 'OPENAI_API_KEY',
            rotation: {
              enabled: true,
              interval: 90,
              overlap: 24,
              notifyBefore: 7
            }
          },
          {
            provider: 'anthropic',
            keyRef: 'ANTHROPIC_API_KEY',
            rotation: {
              enabled: true,
              interval: 90,
              overlap: 24,
              notifyBefore: 7
            }
          }
        ],
        rateLimit: {
          global: {
            requests: 10000,
            window: 60000
          },
          perUser: {
            requests: 100,
            window: 60000
          }
        }
      }
    },
    variables: [
      {
        name: 'REDIS_HOST',
        type: 'string',
        required: true,
        description: 'Redis server hostname'
      },
      {
        name: 'OTLP_ENDPOINT',
        type: 'string',
        required: true,
        description: 'OpenTelemetry collector endpoint'
      }
    ]
  },
  {
    id: 'high-performance',
    name: 'High Performance Configuration',
    description: 'Optimized for maximum throughput and low latency',
    category: ConfigTemplateCategory.HighPerformance,
    config: {
      environment: 'production',
      routing: {
        defaultModel: 'gpt-3.5-turbo',
        scoringWeights: {
          cost: 0.1,
          quality: 0.3,
          latency: 0.5,
          capabilities: 0.1
        },
        timeouts: {
          'gpt-3.5-turbo': 10000,
          'claude-3-haiku-20240307': 10000
        }
      },
      cache: {
        enabled: true,
        ttl: 3600000,
        maxSize: 1024 * 1024 * 1024, // 1GB
        strategy: {
          type: 'lfu'
        },
        providers: [
          {
            type: 'memcached',
            priority: 1,
            config: {
              servers: ['${MEMCACHED_SERVERS}']
            }
          }
        ]
      },
      queue: {
        maxConcurrency: 100,
        maxQueueSize: 10000,
        defaultTimeout: 30000,
        maxRetries: 1,
        enablePersistence: false,
        enableDeduplication: false,
        enableDeadLetter: false,
        priorityBoost: {
          waitTimeThreshold: 5000,
          boostAmount: 2,
          maxBoosts: 3
        }
      },
      performance: {
        defaultTimeout: 30000,
        defaultRetries: 1,
        concurrency: {
          maxParallel: 200,
          queueStrategy: 'priority'
        }
      }
    },
    variables: [
      {
        name: 'MEMCACHED_SERVERS',
        type: 'string',
        required: true,
        description: 'Comma-separated list of memcached servers'
      }
    ]
  },
  {
    id: 'cost-optimized',
    name: 'Cost Optimized Configuration',
    description: 'Optimized for minimum cost while maintaining quality',
    category: ConfigTemplateCategory.CostOptimized,
    config: {
      environment: 'production',
      routing: {
        defaultModel: 'gpt-3.5-turbo',
        preferredModels: ['gpt-3.5-turbo', 'claude-3-haiku-20240307'],
        scoringWeights: {
          cost: 0.7,
          quality: 0.2,
          latency: 0.05,
          capabilities: 0.05
        }
      },
      models: {
        defaults: {
          maxTokens: 1000,
          temperature: 0.7
        }
      },
      cost: {
        limits: [
          {
            amount: 100,
            period: 'day',
            scope: 'total'
          }
        ],
        alerts: [
          {
            threshold: 50,
            type: 'total',
            action: 'log'
          },
          {
            threshold: 80,
            type: 'total',
            action: 'throttle'
          },
          {
            threshold: 95,
            type: 'total',
            action: 'block'
          }
        ]
      },
      cache: {
        enabled: true,
        ttl: 86400000, // 24 hours
        maxSize: 200 * 1024 * 1024,
        strategy: {
          type: 'lru'
        }
      },
      queue: {
        maxConcurrency: 10,
        maxQueueSize: 500,
        defaultTimeout: 60000,
        maxRetries: 2,
        rateLimits: [
          {
            name: 'cost-control',
            limit: 100,
            window: 3600000, // 1 hour
            scope: 'global'
          }
        ]
      }
    }
  },
  {
    id: 'testing',
    name: 'Testing Configuration',
    description: 'Configuration for testing and CI/CD environments',
    category: ConfigTemplateCategory.Testing,
    config: {
      environment: 'development',
      routing: {
        defaultModel: 'gpt-3.5-turbo',
        mockMode: true
      },
      cache: {
        enabled: false
      },
      queue: {
        maxConcurrency: 2,
        maxQueueSize: 10,
        defaultTimeout: 5000,
        maxRetries: 0,
        enablePersistence: false
      },
      performance: {
        defaultTimeout: 5000,
        defaultRetries: 0
      },
      monitoring: {
        metricsInterval: 1000,
        logging: {
          level: 'debug',
          format: 'text',
          destinations: [
            { type: 'console' }
          ]
        }
      }
    }
  }
];

/**
 * Get template by ID
 */
export function getTemplate(templateId: string): ConfigTemplate | undefined {
  return configTemplates.find(t => t.id === templateId);
}

/**
 * Get templates by category
 */
export function getTemplatesByCategory(
  category: ConfigTemplateCategory
): ConfigTemplate[] {
  return configTemplates.filter(t => t.category === category);
}

/**
 * Apply template with variables
 */
export function applyTemplate(
  templateId: string,
  variables: Record<string, any>
): OrchestrationConfig {
  const template = getTemplate(templateId);
  if (!template) {
    throw new Error(`Template ${templateId} not found`);
  }

  // Deep clone config
  let config = JSON.parse(JSON.stringify(template.config));

  // Replace variables
  config = replaceVariables(config, variables);

  return config;
}

/**
 * Replace variables in configuration
 */
function replaceVariables(
  obj: any,
  variables: Record<string, any>
): any {
  if (typeof obj === 'string') {
    // Replace ${VAR_NAME} with variable value
    return obj.replace(/\$\{([^}]+)\}/g, (match, varName) => {
      return variables[varName] !== undefined ? variables[varName] : match;
    });
  }

  if (Array.isArray(obj)) {
    return obj.map(item => replaceVariables(item, variables));
  }

  if (obj && typeof obj === 'object') {
    const result: any = {};
    for (const key in obj) {
      result[key] = replaceVariables(obj[key], variables);
    }
    return result;
  }

  return obj;
}