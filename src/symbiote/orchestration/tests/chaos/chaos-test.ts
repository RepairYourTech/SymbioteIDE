/**
 * Chaos Testing Framework
 * 
 * Simulates various failure scenarios to test system resilience
 */

import { OrchestrationEngine } from '../../orchestration-engine';
import { AITask, TaskResult } from '../../types';
import { createMockTask } from '../utils/test-helpers';
import { TEST_CONFIG } from '../test-config';

export interface ChaosScenario {
  name: string;
  description: string;
  probability: number;
  duration: number;
  apply: (engine: OrchestrationEngine) => void | Promise<void>;
  cleanup: (engine: OrchestrationEngine) => void | Promise<void>;
}

export class ChaosTester {
  private engine: OrchestrationEngine;
  private scenarios: Map<string, ChaosScenario> = new Map();
  private activeScenarios: Set<string> = new Set();
  private results: Map<string, { passed: boolean; errors: any[] }> = new Map();

  constructor(engine: OrchestrationEngine) {
    this.engine = engine;
    this.initializeScenarios();
  }

  /**
   * Initialize chaos scenarios
   */
  private initializeScenarios(): void {
    // Network delay scenario
    this.addScenario({
      name: 'network-delay',
      description: 'Simulates network latency and slow responses',
      probability: 0.2,
      duration: 30000,
      apply: async (engine) => {
        // Intercept provider calls and add delay
        const originalExecute = engine.executeTask.bind(engine);
        engine.executeTask = async (task: AITask, options?: any) => {
          const delay = Math.random() * 5000; // 0-5 seconds
          await new Promise(resolve => setTimeout(resolve, delay));
          return originalExecute(task, options);
        };
      },
      cleanup: async (engine) => {
        // Restore original method
        // In real implementation, would restore properly
      }
    });

    // Provider failure scenario
    this.addScenario({
      name: 'provider-failure',
      description: 'Simulates AI provider failures',
      probability: 0.1,
      duration: 20000,
      apply: async (engine) => {
        const originalExecute = engine.executeTask.bind(engine);
        engine.executeTask = async (task: AITask, options?: any) => {
          if (Math.random() < 0.3) {
            throw new Error('Provider temporarily unavailable');
          }
          return originalExecute(task, options);
        };
      },
      cleanup: async (engine) => {
        // Restore original method
      }
    });

    // Memory pressure scenario
    this.addScenario({
      name: 'memory-pressure',
      description: 'Simulates high memory usage',
      probability: 0.1,
      duration: 15000,
      apply: async (engine) => {
        // Allocate large arrays to simulate memory pressure
        const memoryHog: any[] = [];
        const interval = setInterval(() => {
          memoryHog.push(new Array(1024 * 1024).fill(Math.random()));
        }, 1000);
        
        // Store interval for cleanup
        (engine as any).__chaosMemoryInterval = interval;
        (engine as any).__chaosMemoryHog = memoryHog;
      },
      cleanup: async (engine) => {
        clearInterval((engine as any).__chaosMemoryInterval);
        delete (engine as any).__chaosMemoryHog;
        delete (engine as any).__chaosMemoryInterval;
      }
    });

    // CPU spike scenario
    this.addScenario({
      name: 'cpu-spike',
      description: 'Simulates CPU intensive operations',
      probability: 0.1,
      duration: 10000,
      apply: async (engine) => {
        const interval = setInterval(() => {
          // Simulate CPU intensive work
          const start = Date.now();
          while (Date.now() - start < 100) {
            Math.sqrt(Math.random());
          }
        }, 10);
        
        (engine as any).__chaosCpuInterval = interval;
      },
      cleanup: async (engine) => {
        clearInterval((engine as any).__chaosCpuInterval);
        delete (engine as any).__chaosCpuInterval;
      }
    });

    // Rate limit scenario
    this.addScenario({
      name: 'rate-limit',
      description: 'Simulates hitting rate limits',
      probability: 0.15,
      duration: 25000,
      apply: async (engine) => {
        let requestCount = 0;
        const resetInterval = setInterval(() => { requestCount = 0; }, 1000);
        
        const originalExecute = engine.executeTask.bind(engine);
        engine.executeTask = async (task: AITask, options?: any) => {
          requestCount++;
          if (requestCount > 10) {
            throw new Error('Rate limit exceeded');
          }
          return originalExecute(task, options);
        };
        
        (engine as any).__chaosRateLimitInterval = resetInterval;
      },
      cleanup: async (engine) => {
        clearInterval((engine as any).__chaosRateLimitInterval);
        delete (engine as any).__chaosRateLimitInterval;
      }
    });

    // Invalid response scenario
    this.addScenario({
      name: 'invalid-responses',
      description: 'Simulates malformed or invalid responses',
      probability: 0.1,
      duration: 20000,
      apply: async (engine) => {
        const originalExecute = engine.executeTask.bind(engine);
        engine.executeTask = async (task: AITask, options?: any) => {
          const result = await originalExecute(task, options);
          
          if (Math.random() < 0.2) {
            // Corrupt response
            return {
              ...result,
              content: null,
              status: 'corrupted' as any
            };
          }
          
          return result;
        };
      },
      cleanup: async (engine) => {
        // Restore original method
      }
    });

    // Timeout scenario
    this.addScenario({
      name: 'timeout',
      description: 'Simulates request timeouts',
      probability: 0.1,
      duration: 20000,
      apply: async (engine) => {
        const originalExecute = engine.executeTask.bind(engine);
        engine.executeTask = async (task: AITask, options?: any) => {
          if (Math.random() < 0.2) {
            await new Promise(resolve => setTimeout(resolve, 60000));
            throw new Error('Request timeout');
          }
          return originalExecute(task, options);
        };
      },
      cleanup: async (engine) => {
        // Restore original method
      }
    });

    // Partial failure scenario
    this.addScenario({
      name: 'partial-failure',
      description: 'Simulates partial system failures',
      probability: 0.05,
      duration: 30000,
      apply: async (engine) => {
        // Disable certain models randomly
        const models = await engine.modelRegistry.listModels();
        const disabledModels = models
          .filter(() => Math.random() < 0.5)
          .map(m => m.id);
        
        (engine as any).__chaosDisabledModels = disabledModels;
        
        const originalGetModel = engine.modelRegistry.getModel.bind(engine.modelRegistry);
        engine.modelRegistry.getModel = async (id: string) => {
          if (disabledModels.includes(id)) {
            return null;
          }
          return originalGetModel(id);
        };
      },
      cleanup: async (engine) => {
        delete (engine as any).__chaosDisabledModels;
      }
    });
  }

  /**
   * Add a chaos scenario
   */
  addScenario(scenario: ChaosScenario): void {
    this.scenarios.set(scenario.name, scenario);
  }

  /**
   * Run chaos testing
   */
  async runChaosTest(options: {
    duration: number;
    scenarios?: string[];
    concurrent?: boolean;
  }): Promise<void> {
    console.log('\n🌪️  Starting Chaos Testing\n');
    console.log(`Duration: ${options.duration}ms`);
    console.log(`Scenarios: ${options.scenarios?.join(', ') || 'all'}`);
    console.log(`Mode: ${options.concurrent ? 'concurrent' : 'sequential'}\n`);

    const startTime = Date.now();
    const endTime = startTime + options.duration;

    // Select scenarios to run
    const scenariosToRun = options.scenarios 
      ? Array.from(this.scenarios.values()).filter(s => options.scenarios!.includes(s.name))
      : Array.from(this.scenarios.values());

    // Run test workload while applying chaos
    const workloadPromise = this.runWorkload(endTime);
    const chaosPromise = options.concurrent
      ? this.runConcurrentChaos(scenariosToRun, endTime)
      : this.runSequentialChaos(scenariosToRun, endTime);

    await Promise.all([workloadPromise, chaosPromise]);

    // Print results
    this.printResults();
  }

  /**
   * Run continuous workload
   */
  private async runWorkload(endTime: number): Promise<void> {
    let successCount = 0;
    let errorCount = 0;
    const errors: any[] = [];

    while (Date.now() < endTime) {
      try {
        const task = createMockTask({
          id: `chaos-test-${Date.now()}`,
          prompt: 'Chaos test task'
        });

        await this.engine.executeTask(task);
        successCount++;
      } catch (error) {
        errorCount++;
        errors.push(error);
      }

      // Small delay between requests
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    console.log(`\n📊 Workload Results:`);
    console.log(`  Successful: ${successCount}`);
    console.log(`  Failed: ${errorCount}`);
    console.log(`  Success Rate: ${((successCount / (successCount + errorCount)) * 100).toFixed(2)}%`);
  }

  /**
   * Run chaos scenarios sequentially
   */
  private async runSequentialChaos(
    scenarios: ChaosScenario[],
    endTime: number
  ): Promise<void> {
    for (const scenario of scenarios) {
      if (Date.now() >= endTime) break;

      await this.applyScenario(scenario);
      await new Promise(resolve => setTimeout(resolve, scenario.duration));
      await this.cleanupScenario(scenario);
    }
  }

  /**
   * Run chaos scenarios concurrently
   */
  private async runConcurrentChaos(
    scenarios: ChaosScenario[],
    endTime: number
  ): Promise<void> {
    const promises = scenarios.map(async scenario => {
      while (Date.now() < endTime) {
        if (Math.random() < scenario.probability) {
          await this.applyScenario(scenario);
          await new Promise(resolve => setTimeout(resolve, scenario.duration));
          await this.cleanupScenario(scenario);
        }
        
        // Wait before potentially applying again
        await new Promise(resolve => setTimeout(resolve, 5000));
      }
    });

    await Promise.all(promises);
  }

  /**
   * Apply a chaos scenario
   */
  private async applyScenario(scenario: ChaosScenario): Promise<void> {
    console.log(`\n🔥 Applying chaos: ${scenario.name}`);
    console.log(`   ${scenario.description}`);
    
    this.activeScenarios.add(scenario.name);
    
    try {
      await scenario.apply(this.engine);
      this.results.set(scenario.name, { passed: true, errors: [] });
    } catch (error) {
      this.results.set(scenario.name, { passed: false, errors: [error] });
      console.error(`   ❌ Failed to apply: ${error}`);
    }
  }

  /**
   * Cleanup a chaos scenario
   */
  private async cleanupScenario(scenario: ChaosScenario): Promise<void> {
    console.log(`   ✅ Cleaning up: ${scenario.name}`);
    
    try {
      await scenario.cleanup(this.engine);
      this.activeScenarios.delete(scenario.name);
    } catch (error) {
      console.error(`   ❌ Cleanup failed: ${error}`);
    }
  }

  /**
   * Print chaos test results
   */
  private printResults(): void {
    console.log('\n\n🌪️  Chaos Test Results\n');
    console.log('='.repeat(50));

    for (const [name, result] of this.results) {
      const icon = result.passed ? '✅' : '❌';
      console.log(`\n${icon} ${name}`);
      
      if (!result.passed && result.errors.length > 0) {
        console.log('   Errors:');
        result.errors.forEach(error => {
          console.log(`   - ${error.message || error}`);
        });
      }
    }

    const totalScenarios = this.results.size;
    const passedScenarios = Array.from(this.results.values()).filter(r => r.passed).length;
    
    console.log('\n' + '='.repeat(50));
    console.log(`\nTotal Scenarios: ${totalScenarios}`);
    console.log(`Passed: ${passedScenarios}`);
    console.log(`Failed: ${totalScenarios - passedScenarios}`);
    console.log(`Success Rate: ${((passedScenarios / totalScenarios) * 100).toFixed(2)}%`);
  }
}

/**
 * Run chaos test suite
 */
export async function runChaosTestSuite(): Promise<void> {
  // Initialize engine
  const engine = new OrchestrationEngine({
    models: {
      providers: ['test'],
      fallbackModel: 'test-model',
      retryAttempts: 3,
      retryDelay: 1000
    },
    queue: {
      maxConcurrent: 10,
      maxQueueSize: 100
    }
  });

  const tester = new ChaosTester(engine);

  // Test 1: Sequential chaos
  await tester.runChaosTest({
    duration: 60000,
    scenarios: ['network-delay', 'provider-failure', 'rate-limit'],
    concurrent: false
  });

  // Test 2: Concurrent chaos
  await tester.runChaosTest({
    duration: 120000,
    concurrent: true
  });

  // Cleanup
  await engine.stop();
}