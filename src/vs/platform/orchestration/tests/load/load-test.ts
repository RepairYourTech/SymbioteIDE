/**
 * Load Testing Framework
 */

import { OrchestrationEngine } from '../../orchestration-engine';
import { AITask } from '../../types';
import { createMockTask, createMockModel } from '../utils/test-helpers';
import { TEST_CONFIG } from '../test-config';

interface LoadTestMetrics {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  averageLatency: number;
  p50Latency: number;
  p95Latency: number;
  p99Latency: number;
  throughput: number;
  errorRate: number;
  startTime: number;
  endTime: number;
  duration: number;
}

export class LoadTester {
  private engine: OrchestrationEngine;
  private metrics: LoadTestMetrics = {
    totalRequests: 0,
    successfulRequests: 0,
    failedRequests: 0,
    averageLatency: 0,
    p50Latency: 0,
    p95Latency: 0,
    p99Latency: 0,
    throughput: 0,
    errorRate: 0,
    startTime: 0,
    endTime: 0,
    duration: 0
  };
  private latencies: number[] = [];

  constructor(engine: OrchestrationEngine) {
    this.engine = engine;
  }

  /**
   * Run load test with specified configuration
   */
  async runLoadTest(config: {
    users: number;
    duration: number;
    rampUp: number;
    taskGenerator?: () => AITask;
  }): Promise<LoadTestMetrics> {
    console.log('\n🚀 Starting Load Test');
    console.log(`Users: ${config.users}`);
    console.log(`Duration: ${config.duration}s`);
    console.log(`Ramp-up: ${config.rampUp}s\n`);

    this.metrics.startTime = Date.now();
    
    // Create virtual users
    const users = await this.createVirtualUsers(config);
    
    // Run test
    await this.executeLoadTest(users, config);
    
    // Calculate final metrics
    this.calculateMetrics();
    
    // Print results
    this.printResults();
    
    return this.metrics;
  }

  /**
   * Create virtual users with ramp-up
   */
  private async createVirtualUsers(config: any): Promise<Array<() => Promise<void>>> {
    const users: Array<() => Promise<void>> = [];
    const usersPerSecond = config.users / config.rampUp;
    
    for (let i = 0; i < config.users; i++) {
      const delay = (i / usersPerSecond) * 1000;
      
      users.push(async () => {
        // Wait for ramp-up
        await new Promise(resolve => setTimeout(resolve, delay));
        
        // Execute tasks until duration expires
        const endTime = this.metrics.startTime + (config.duration * 1000);
        
        while (Date.now() < endTime) {
          await this.executeTask(config.taskGenerator);
        }
      });
    }
    
    return users;
  }

  /**
   * Execute load test with virtual users
   */
  private async executeLoadTest(
    users: Array<() => Promise<void>>, 
    config: any
  ): Promise<void> {
    // Start progress monitoring
    const progressInterval = setInterval(() => {
      this.printProgress();
    }, 1000);

    // Execute all users
    await Promise.all(users.map(user => user()));
    
    // Stop monitoring
    clearInterval(progressInterval);
    
    this.metrics.endTime = Date.now();
    this.metrics.duration = (this.metrics.endTime - this.metrics.startTime) / 1000;
  }

  /**
   * Execute a single task and track metrics
   */
  private async executeTask(taskGenerator?: () => AITask): Promise<void> {
    const task = taskGenerator ? taskGenerator() : createMockTask({
      id: `load-test-${Date.now()}-${Math.random()}`,
      prompt: `Load test task ${this.metrics.totalRequests}`
    });
    
    const startTime = Date.now();
    this.metrics.totalRequests++;
    
    try {
      await this.engine.executeTask(task);
      
      const latency = Date.now() - startTime;
      this.latencies.push(latency);
      this.metrics.successfulRequests++;
      
    } catch (error) {
      this.metrics.failedRequests++;
    }
  }

  /**
   * Calculate final metrics
   */
  private calculateMetrics(): void {
    if (this.latencies.length === 0) return;
    
    // Sort latencies for percentile calculation
    this.latencies.sort((a, b) => a - b);
    
    // Average latency
    this.metrics.averageLatency = 
      this.latencies.reduce((sum, l) => sum + l, 0) / this.latencies.length;
    
    // Percentiles
    this.metrics.p50Latency = this.getPercentile(50);
    this.metrics.p95Latency = this.getPercentile(95);
    this.metrics.p99Latency = this.getPercentile(99);
    
    // Throughput
    this.metrics.throughput = this.metrics.totalRequests / this.metrics.duration;
    
    // Error rate
    this.metrics.errorRate = this.metrics.failedRequests / this.metrics.totalRequests;
  }

  /**
   * Get percentile value from sorted latencies
   */
  private getPercentile(percentile: number): number {
    const index = Math.ceil((percentile / 100) * this.latencies.length) - 1;
    return this.latencies[Math.max(0, index)];
  }

  /**
   * Print progress during test
   */
  private printProgress(): void {
    const elapsed = (Date.now() - this.metrics.startTime) / 1000;
    const throughput = this.metrics.totalRequests / elapsed;
    
    process.stdout.write(
      `\r⏱️  Elapsed: ${elapsed.toFixed(0)}s | ` +
      `📊 Requests: ${this.metrics.totalRequests} | ` +
      `⚡ Throughput: ${throughput.toFixed(1)} req/s | ` +
      `❌ Errors: ${this.metrics.failedRequests}`
    );
  }

  /**
   * Print final results
   */
  private printResults(): void {
    console.log('\n\n📈 Load Test Results\n');
    console.log('='.repeat(50));
    
    console.log('\n📊 Summary:');
    console.log(`  Total Requests: ${this.metrics.totalRequests}`);
    console.log(`  Successful: ${this.metrics.successfulRequests}`);
    console.log(`  Failed: ${this.metrics.failedRequests}`);
    console.log(`  Error Rate: ${(this.metrics.errorRate * 100).toFixed(2)}%`);
    console.log(`  Duration: ${this.metrics.duration.toFixed(1)}s`);
    
    console.log('\n⚡ Performance:');
    console.log(`  Throughput: ${this.metrics.throughput.toFixed(2)} req/s`);
    console.log(`  Avg Latency: ${this.metrics.averageLatency.toFixed(0)}ms`);
    console.log(`  P50 Latency: ${this.metrics.p50Latency}ms`);
    console.log(`  P95 Latency: ${this.metrics.p95Latency}ms`);
    console.log(`  P99 Latency: ${this.metrics.p99Latency}ms`);
    
    console.log('\n✅ Threshold Validation:');
    const thresholds = TEST_CONFIG.load.thresholds;
    
    this.validateThreshold(
      'P95 Response Time',
      this.metrics.p95Latency,
      thresholds.p95ResponseTime,
      'ms'
    );
    
    this.validateThreshold(
      'P99 Response Time',
      this.metrics.p99Latency,
      thresholds.p99ResponseTime,
      'ms'
    );
    
    this.validateThreshold(
      'Error Rate',
      this.metrics.errorRate * 100,
      thresholds.errorRate * 100,
      '%'
    );
    
    this.validateThreshold(
      'Throughput',
      this.metrics.throughput,
      thresholds.throughput,
      'req/s',
      true
    );
    
    console.log('\n' + '='.repeat(50));
  }

  /**
   * Validate metric against threshold
   */
  private validateThreshold(
    name: string,
    actual: number,
    threshold: number,
    unit: string,
    higherIsBetter: boolean = false
  ): void {
    const passed = higherIsBetter ? actual >= threshold : actual <= threshold;
    const icon = passed ? '✅' : '❌';
    const comparison = higherIsBetter ? '>=' : '<=';
    
    console.log(
      `  ${icon} ${name}: ${actual.toFixed(2)}${unit} ` +
      `(threshold: ${comparison} ${threshold}${unit})`
    );
  }
}

/**
 * Run load test scenarios
 */
export async function runLoadTestScenarios(): Promise<void> {
  // Initialize engine
  const engine = new OrchestrationEngine({
    models: {
      providers: ['test'],
      fallbackModel: 'test-model'
    },
    queue: {
      maxConcurrent: 50,
      maxQueueSize: 1000
    }
  });

  // Register test model
  await engine.modelRegistry.registerModel(
    createMockModel({
      id: 'test-model',
      name: 'Load Test Model'
    })
  );

  const tester = new LoadTester(engine);

  // Scenario 1: Steady load
  console.log('\n🎯 Scenario 1: Steady Load\n');
  await tester.runLoadTest({
    users: 50,
    duration: 60,
    rampUp: 10
  });

  // Scenario 2: Spike test
  console.log('\n\n🎯 Scenario 2: Spike Test\n');
  await tester.runLoadTest({
    users: 200,
    duration: 30,
    rampUp: 5
  });

  // Scenario 3: Sustained load
  console.log('\n\n🎯 Scenario 3: Sustained Load\n');
  await tester.runLoadTest({
    users: 100,
    duration: 300,
    rampUp: 20,
    taskGenerator: () => createMockTask({
      type: Math.random() > 0.5 ? 'chat' : 'code_generation',
      prompt: `Complex task with ${Math.random() * 1000} tokens`,
      constraints: {
        maxTokens: Math.floor(Math.random() * 2000) + 500
      }
    })
  });

  // Cleanup
  await engine.stop();
}