/**
 * Test Configuration
 */

export const TEST_CONFIG = {
  // Test environment
  environment: process.env.NODE_ENV || 'test',
  
  // Timeouts
  timeouts: {
    unit: 5000,
    integration: 30000,
    e2e: 60000,
    chaos: 120000,
    load: 300000
  },

  // Mock data paths
  fixtures: {
    models: './fixtures/models.json',
    tasks: './fixtures/tasks.json',
    configs: './fixtures/configs.json',
    responses: './fixtures/responses.json'
  },

  // Test API server ports
  ports: {
    rest: 3001,
    grpc: 50052,
    mockProviders: {
      anthropic: 3010,
      openai: 3011,
      google: 3012,
      mistral: 3013
    }
  },

  // Load testing configuration
  load: {
    users: {
      min: 10,
      max: 1000,
      rampUp: 60 // seconds
    },
    duration: 300, // seconds
    thresholds: {
      p95ResponseTime: 1000, // ms
      p99ResponseTime: 2000, // ms
      errorRate: 0.01, // 1%
      throughput: 100 // requests per second
    }
  },

  // Chaos testing configuration
  chaos: {
    scenarios: [
      'network-delay',
      'provider-failure',
      'memory-pressure',
      'cpu-spike',
      'rate-limit',
      'invalid-responses',
      'timeout',
      'partial-failure'
    ],
    probability: 0.1, // 10% chance of chaos
    duration: 60 // seconds per scenario
  },

  // Coverage thresholds
  coverage: {
    statements: 90,
    branches: 85,
    functions: 90,
    lines: 90
  }
};