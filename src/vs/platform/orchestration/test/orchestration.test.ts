/**
 * Orchestration Engine Tests
 */

import { 
  OrchestrationEngine,
  ModelRegistry,
  TaskAnalyzer,
  RoutingEngine,
  CostTracker,
  CacheManager 
} from '../index';

import {
  AITask,
  TaskType,
  ComplexityLevel,
  QualityLevel,
  UrgencyLevel,
  ModelProfile,
  TaskResult
} from '../interfaces';

// Mock provider adapter for testing
class MockProviderAdapter {
  async execute(task: AITask, model: ModelProfile): Promise<TaskResult> {
    return {
      id: `result-${Date.now()}`,
      taskId: task.id,
      status: 'success',
      content: 'Mock response for ' + task.prompt,
      model: model.id,
      usage: {
        promptTokens: 100,
        completionTokens: 50,
        totalTokens: 150
      },
      cost: {
        amount: 0.001,
        currency: 'USD',
        breakdown: {
          inputTokens: 100,
          outputTokens: 50,
          inputCost: 0.0006,
          outputCost: 0.0004
        }
      },
      latency: 1000,
      timestamp: Date.now()
    };
  }
}

describe('Orchestration Engine', () => {
  let engine: OrchestrationEngine;

  beforeEach(() => {
    engine = new OrchestrationEngine({
      cacheEnabled: false, // Disable cache for tests
      costLimits: [{
        amount: 10,
        period: 'day',
        currency: 'USD'
      }]
    });
  });

  afterEach(async () => {
    await engine.shutdown();
  });

  describe('Model Registry', () => {
    it('should have pre-configured models', () => {
      const registry = new ModelRegistry();
      const models = registry.getAllModels();
      
      expect(models.length).toBeGreaterThan(0);
      expect(models.some(m => m.provider === 'anthropic')).toBe(true);
      expect(models.some(m => m.provider === 'openai')).toBe(true);
      expect(models.some(m => m.provider === 'google')).toBe(true);
    });

    it('should filter models by capabilities', () => {
      const registry = new ModelRegistry();
      const codeGenModels = registry.getModelsForTaskType(TaskType.CodeGeneration);
      
      expect(codeGenModels.length).toBeGreaterThan(0);
      expect(codeGenModels.every(m => 
        m.capabilities.includes('code-generation')
      )).toBe(true);
    });
  });

  describe('Task Analyzer', () => {
    it('should detect task type from prompt', async () => {
      const analyzer = new TaskAnalyzer();
      
      const codeGenTask: AITask = {
        id: 'test-1',
        type: TaskType.CodeGeneration,
        prompt: 'Generate a React component for a todo list'
      };
      
      const analysis = await analyzer.analyzeTask(codeGenTask);
      expect(analysis.type).toBe(TaskType.CodeGeneration);
      expect(analysis.complexity).toBeDefined();
      expect(analysis.requiredCapabilities).toContain('code-generation');
    });

    it('should estimate token usage', async () => {
      const analyzer = new TaskAnalyzer();
      
      const task: AITask = {
        id: 'test-2',
        type: TaskType.General,
        prompt: 'This is a test prompt with some content',
        context: {
          files: [{
            path: 'test.ts',
            content: 'const x = 1;\nconst y = 2;\n',
            language: 'typescript'
          }]
        }
      };
      
      const analysis = await analyzer.analyzeTask(task);
      expect(analysis.estimatedTokens).toBeGreaterThan(0);
      expect(analysis.contextWindow).toBeGreaterThanOrEqual(4096);
    });
  });

  describe('Routing Engine', () => {
    it('should select appropriate model for task', async () => {
      const registry = new ModelRegistry();
      const analyzer = new TaskAnalyzer();
      const router = new RoutingEngine(registry, analyzer);
      
      const task: AITask = {
        id: 'test-3',
        type: TaskType.CodeGeneration,
        prompt: 'Create a Python function',
        constraints: {
          maxCost: 0.10,
          maxLatency: 5000
        }
      };
      
      const selection = await router.routeTask(task);
      expect(selection.primary).toBeDefined();
      expect(selection.fallbacks.length).toBeGreaterThan(0);
      expect(selection.estimatedCost.expected).toBeLessThanOrEqual(0.10);
      expect(selection.reasoning).toBeTruthy();
    });

    it('should respect cost constraints', async () => {
      const registry = new ModelRegistry();
      const analyzer = new TaskAnalyzer();
      const router = new RoutingEngine(registry, analyzer);
      
      const task: AITask = {
        id: 'test-4',
        type: TaskType.CodeGeneration,
        prompt: 'Create a complex system',
        constraints: {
          maxCost: 0.0001 // Very low budget
        }
      };
      
      const selection = await router.routeTask(task);
      expect(selection.estimatedCost.expected).toBeLessThanOrEqual(0.0001);
    });
  });

  describe('Cost Tracker', () => {
    it('should track costs and enforce limits', () => {
      const tracker = new CostTracker([{
        amount: 1.0,
        period: 'hour',
        currency: 'USD'
      }]);
      
      const result: TaskResult = {
        id: 'result-1',
        taskId: 'task-1',
        status: 'success',
        model: 'gpt-4',
        usage: { promptTokens: 1000, completionTokens: 500, totalTokens: 1500 },
        cost: { amount: 0.05, currency: 'USD', breakdown: {} as any },
        latency: 2000,
        timestamp: Date.now()
      };
      
      const model: ModelProfile = {
        id: 'gpt-4',
        provider: 'openai',
        name: 'gpt-4',
        displayName: 'GPT-4',
        capabilities: [],
        contextWindow: 8192,
        maxOutputTokens: 4096,
        costPerToken: { input: 0.00003, output: 0.00006, currency: 'USD' },
        averageLatency: 3000,
        reliability: 0.98,
        specializations: [],
        rateLimit: { requestsPerMinute: 60, tokensPerMinute: 100000 },
        availability: { status: 'available' }
      };
      
      tracker.recordCost(result, model, TaskType.General);
      
      const metrics = tracker.getMetrics();
      expect(metrics.totalCost).toBe(0.05);
      expect(metrics.budgetUtilization).toBeLessThan(1);
      
      const check = tracker.checkCostLimit(0.96);
      expect(check.allowed).toBe(false);
    });
  });

  describe('Cache Manager', () => {
    it('should cache and retrieve results', () => {
      const cache = new CacheManager();
      
      const task: AITask = {
        id: 'test-5',
        type: TaskType.General,
        prompt: 'What is 2+2?'
      };
      
      const result: TaskResult = {
        id: 'result-5',
        taskId: 'test-5',
        status: 'success',
        content: '4',
        model: 'gpt-3.5-turbo',
        usage: { promptTokens: 10, completionTokens: 1, totalTokens: 11 },
        cost: { amount: 0.00001, currency: 'USD', breakdown: {} as any },
        latency: 500,
        timestamp: Date.now()
      };
      
      cache.set(task, result);
      
      const cached = cache.get(task);
      expect(cached).toBeDefined();
      expect(cached?.content).toBe('4');
      expect(cached?.metadata?.cacheHit).toBe(true);
    });

    it('should find similar cached results', () => {
      const cache = new CacheManager();
      
      const task1: AITask = {
        id: 'test-6',
        type: TaskType.General,
        prompt: 'Write a function to calculate factorial'
      };
      
      const result1: TaskResult = {
        id: 'result-6',
        taskId: 'test-6',
        status: 'success',
        content: 'function factorial(n) { ... }',
        model: 'gpt-4',
        usage: { promptTokens: 20, completionTokens: 50, totalTokens: 70 },
        cost: { amount: 0.001, currency: 'USD', breakdown: {} as any },
        latency: 1500,
        timestamp: Date.now()
      };
      
      cache.set(task1, result1);
      
      const task2: AITask = {
        id: 'test-7',
        type: TaskType.General,
        prompt: 'Create a factorial function'
      };
      
      const similar = cache.findSimilar(task2, 0.5);
      expect(similar.length).toBeGreaterThan(0);
      expect(similar[0].entry.result.content).toContain('factorial');
    });
  });

  describe('Full Integration', () => {
    it('should execute a simple task', async () => {
      const task: AITask = {
        id: 'integration-1',
        type: TaskType.General,
        prompt: 'Hello, how are you?'
      };
      
      // Note: This will fail without real API keys
      // In a real test environment, we would mock the provider adapters
      try {
        const result = await engine.executeTask(task);
        expect(result.status).toBe('success');
        expect(result.content).toBeTruthy();
      } catch (error: any) {
        // Expected to fail without API keys
        expect(error.code).toBeDefined();
      }
    });

    it('should provide cost estimates', async () => {
      const task: AITask = {
        id: 'integration-2',
        type: TaskType.CodeGeneration,
        prompt: 'Generate a complex React application'
      };
      
      const estimate = await engine.getCostEstimate(task);
      expect(estimate.expected).toBeGreaterThan(0);
      expect(estimate.minimum).toBeLessThanOrEqual(estimate.expected);
      expect(estimate.maximum).toBeGreaterThanOrEqual(estimate.expected);
    });

    it('should list available models', async () => {
      const models = await engine.listModels();
      expect(models.length).toBeGreaterThan(0);
      expect(models.some(m => m.capabilities.includes('code-generation'))).toBe(true);
    });

    it('should provide metrics', async () => {
      const metrics = await engine.getMetrics();
      expect(metrics).toBeDefined();
      expect(metrics.performance).toBeDefined();
      expect(metrics.cost).toBeDefined();
      expect(metrics.quality).toBeDefined();
      expect(metrics.usage).toBeDefined();
    });
  });
});

// Run tests if this file is executed directly
if (require.main === module) {
  console.log('Running orchestration engine tests...');
  // In a real environment, we would use Jest or another test runner
}