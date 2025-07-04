/**
 * Orchestration Engine Integration Tests
 */

import { TestRunner, expect, createMockFunction } from '../utils/test-framework';
import { OrchestrationEngine } from '../../orchestration-engine';
import { ModelProfile, AITask } from '../../types';
import { RequestPriority } from '../../queue';
import { createMockModel, createMockTask, delay } from '../utils/test-helpers';

const runner = new TestRunner();

runner.describe('OrchestrationEngine Integration', () => {
  let engine: OrchestrationEngine;
  
  runner.beforeAll(async () => {
    // Create engine with test configuration
    engine = new OrchestrationEngine({
      models: {
        providers: ['test'],
        fallbackModel: 'test-basic',
        retryAttempts: 2,
        retryDelay: 100
      },
      routing: {
        strategy: 'cost_optimized',
        scoringWeights: {
          cost: 0.4,
          performance: 0.3,
          quality: 0.3
        }
      },
      queue: {
        maxConcurrent: 5,
        maxQueueSize: 100,
        priorityBoost: {
          enabled: true,
          thresholdMs: 1000,
          boostAmount: 1
        }
      },
      monitoring: {
        enabled: true,
        metricsInterval: 1000
      }
    });
    
    // Register test models
    const models: ModelProfile[] = [
      createMockModel({
        id: 'test-basic',
        name: 'Test Basic',
        costPerToken: { input: 0.001, output: 0.002 },
        capabilities: ['chat']
      }),
      createMockModel({
        id: 'test-advanced',
        name: 'Test Advanced',
        costPerToken: { input: 0.01, output: 0.02 },
        capabilities: ['chat', 'code', 'function']
      }),
      createMockModel({
        id: 'test-pro',
        name: 'Test Pro',
        costPerToken: { input: 0.03, output: 0.06 },
        capabilities: ['chat', 'code', 'function', 'vision']
      })
    ];
    
    for (const model of models) {
      await engine.modelRegistry.registerModel(model);
    }
  });
  
  runner.afterAll(async () => {
    // Cleanup
    await engine.stop();
  });
  
  runner.it('should execute simple task successfully', async () => {
    const task = createMockTask({
      type: 'chat',
      prompt: 'Hello, world!'
    });
    
    const result = await engine.executeTask(task);
    
    expect(result).toBeTruthy();
    expect(result.status).toBe('completed');
    expect(result.model).toBeTruthy();
    expect(result.usage.totalTokens).toBeGreaterThan(0);
    expect(result.cost.amount).toBeGreaterThan(0);
  });
  
  runner.it('should route tasks based on capabilities', async () => {
    const codeTask = createMockTask({
      type: 'code_generation',
      prompt: 'Write a Python function'
    });
    
    const result = await engine.executeTask(codeTask);
    
    // Should use advanced or pro model for code
    expect(['test-advanced', 'test-pro']).toContain(result.model);
  });
  
  runner.it('should handle concurrent tasks', async () => {
    const tasks = Array(10).fill(null).map((_, i) => 
      createMockTask({
        id: `concurrent-${i}`,
        prompt: `Task ${i}`
      })
    );
    
    const results = await Promise.all(
      tasks.map(task => engine.executeTask(task))
    );
    
    expect(results.length).toBe(10);
    results.forEach(result => {
      expect(result.status).toBe('completed');
    });
  });
  
  runner.it('should respect task priorities in queue', async () => {
    const completionOrder: string[] = [];
    
    // Queue tasks with different priorities
    const tasks = [
      { task: createMockTask({ id: 'low-1' }), priority: RequestPriority.Low },
      { task: createMockTask({ id: 'critical-1' }), priority: RequestPriority.Critical },
      { task: createMockTask({ id: 'normal-1' }), priority: RequestPriority.Normal },
      { task: createMockTask({ id: 'high-1' }), priority: RequestPriority.High },
      { task: createMockTask({ id: 'background-1' }), priority: RequestPriority.Background }
    ];
    
    // Queue all tasks
    const promises = tasks.map(({ task, priority }) => 
      engine.queueTask(task, { 
        priority,
        metadata: {
          onComplete: () => completionOrder.push(task.id)
        }
      })
    );
    
    // Wait for all to complete
    await Promise.all(promises);
    await delay(100);
    
    // Check completion order (critical should be first)
    expect(completionOrder[0]).toBe('critical-1');
    expect(completionOrder[completionOrder.length - 1]).toBe('background-1');
  });
  
  runner.it('should handle task failures with retry', async () => {
    let attempts = 0;
    
    // Mock a failing task
    const task = createMockTask({
      id: 'retry-test',
      metadata: {
        failUntilAttempt: 2
      }
    });
    
    // Override execute to simulate failures
    const originalExecute = engine.executeTask.bind(engine);
    engine.executeTask = async (t: AITask, options?: any) => {
      attempts++;
      if (attempts < 2) {
        throw new Error('Simulated failure');
      }
      return originalExecute(t, options);
    };
    
    const result = await engine.executeTask(task);
    
    expect(attempts).toBe(2);
    expect(result.status).toBe('completed');
    
    // Restore original method
    engine.executeTask = originalExecute;
  });
  
  runner.it('should track metrics accurately', async () => {
    // Execute several tasks
    const tasks = Array(5).fill(null).map((_, i) => 
      createMockTask({ id: `metrics-${i}` })
    );
    
    for (const task of tasks) {
      await engine.executeTask(task);
    }
    
    const metrics = await engine.getMetrics();
    
    expect(metrics.usage.totalRequests).toBeGreaterThan(5);
    expect(metrics.performance.averageLatency).toBeGreaterThan(0);
    expect(metrics.quality.successRate).toBeGreaterThan(0.9);
    expect(metrics.cost.totalCost).toBeGreaterThan(0);
  });
  
  runner.it('should estimate costs accurately', async () => {
    const task = createMockTask({
      type: 'code_generation',
      prompt: 'Create a complex application with multiple features and comprehensive testing',
      constraints: {
        maxTokens: 2000
      }
    });
    
    const estimate = await engine.getCostEstimate(task);
    
    expect(estimate.minimum).toBeGreaterThan(0);
    expect(estimate.expected).toBeGreaterThan(estimate.minimum);
    expect(estimate.maximum).toBeGreaterThan(estimate.expected);
    expect(estimate.confidence).toBeGreaterThan(0.5);
  });
  
  runner.it('should handle configuration updates', async () => {
    const originalConfig = engine.getConfiguration();
    
    const newConfig = {
      ...originalConfig,
      routing: {
        ...originalConfig.routing,
        strategy: 'quality_focused' as const
      }
    };
    
    await engine.updateConfig(newConfig);
    
    const updatedConfig = engine.getConfiguration();
    expect(updatedConfig.routing.strategy).toBe('quality_focused');
  });
  
  runner.it('should emit events correctly', async () => {
    const events: string[] = [];
    
    engine.on('taskQueued', () => events.push('queued'));
    engine.on('taskStarted', () => events.push('started'));
    engine.on('taskCompleted', () => events.push('completed'));
    
    const task = createMockTask();
    await engine.executeTask(task);
    
    await delay(100);
    
    expect(events).toContain('queued');
    expect(events).toContain('started');
    expect(events).toContain('completed');
  });
  
  runner.it('should handle queue saturation gracefully', async () => {
    // Fill queue to capacity
    const tasks = Array(150).fill(null).map((_, i) => 
      createMockTask({ id: `saturation-${i}` })
    );
    
    let rejected = 0;
    const promises = tasks.map(task => 
      engine.queueTask(task).catch(() => {
        rejected++;
      })
    );
    
    await Promise.allSettled(promises);
    
    // Should reject tasks beyond queue capacity
    expect(rejected).toBeGreaterThan(0);
    
    const stats = await engine.getQueueStats();
    expect(stats?.queueLength).toBeLessThan(101); // Max queue size + 1
  });
});

export default runner;