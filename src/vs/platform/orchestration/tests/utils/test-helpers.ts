/**
 * Test Helper Utilities
 */

import { OrchestrationEngine } from '../../orchestration-engine';
import { ModelProfile, AITask, TaskResult } from '../../types';
import { TEST_CONFIG } from '../test-config';

/**
 * Create a mock orchestration engine for testing
 */
export function createMockEngine(overrides?: Partial<any>): OrchestrationEngine {
  const mockEngine = {
    executeTask: jest.fn(),
    queueTask: jest.fn(),
    listModels: jest.fn(),
    getMetrics: jest.fn(),
    getQueueStats: jest.fn(),
    getCostEstimate: jest.fn(),
    getConfiguration: jest.fn(),
    updateConfig: jest.fn(),
    on: jest.fn(),
    off: jest.fn(),
    emit: jest.fn(),
    ...overrides
  } as any;

  return mockEngine;
}

/**
 * Create a mock model profile
 */
export function createMockModel(overrides?: Partial<ModelProfile>): ModelProfile {
  return {
    id: 'test-model',
    provider: 'test',
    name: 'Test Model',
    capabilities: ['chat', 'code'],
    contextWindow: 8192,
    costPerToken: {
      input: 0.001,
      output: 0.002
    },
    supportsStreaming: true,
    supportsFunctions: true,
    metadata: {},
    ...overrides
  };
}

/**
 * Create a mock AI task
 */
export function createMockTask(overrides?: Partial<AITask>): AITask {
  return {
    id: `task-${Date.now()}`,
    type: 'code_generation',
    prompt: 'Test prompt',
    context: {
      files: [],
      variables: {},
      workspacePath: '/test'
    },
    constraints: {
      maxTokens: 1000,
      temperature: 0.7
    },
    metadata: {
      source: 'test',
      userId: 'test-user'
    },
    ...overrides
  };
}

/**
 * Create a mock task result
 */
export function createMockResult(overrides?: Partial<TaskResult>): TaskResult {
  return {
    id: `result-${Date.now()}`,
    taskId: 'test-task',
    status: 'completed',
    content: 'Test response',
    model: 'test-model',
    usage: {
      promptTokens: 100,
      completionTokens: 200,
      totalTokens: 300
    },
    cost: {
      amount: 0.5,
      currency: 'USD'
    },
    latency: 1000,
    timestamp: Date.now(),
    ...overrides
  };
}

/**
 * Wait for a condition to be true
 */
export async function waitFor(
  condition: () => boolean | Promise<boolean>,
  timeout: number = 5000,
  interval: number = 100
): Promise<void> {
  const startTime = Date.now();
  
  while (Date.now() - startTime < timeout) {
    if (await condition()) {
      return;
    }
    await new Promise(resolve => setTimeout(resolve, interval));
  }
  
  throw new Error('Timeout waiting for condition');
}

/**
 * Create a delay
 */
export function delay(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Measure execution time
 */
export async function measureTime<T>(
  fn: () => Promise<T>
): Promise<{ result: T; duration: number }> {
  const start = Date.now();
  const result = await fn();
  const duration = Date.now() - start;
  return { result, duration };
}

/**
 * Create a mock API request
 */
export function createMockRequest(overrides?: any): any {
  return {
    method: 'POST',
    path: '/api/v1/tasks/execute',
    headers: {
      'content-type': 'application/json',
      'x-api-key': 'test-key'
    },
    body: {},
    query: {},
    params: {},
    user: { id: 'test-user' },
    id: `req-${Date.now()}`,
    ...overrides
  };
}

/**
 * Create a mock API response
 */
export function createMockResponse(): any {
  const response = {
    status: jest.fn().mockReturnThis(),
    json: jest.fn().mockReturnThis(),
    send: jest.fn().mockReturnThis(),
    setHeader: jest.fn().mockReturnThis(),
    end: jest.fn().mockReturnThis()
  };
  
  return response;
}

/**
 * Assert error matches expected pattern
 */
export function assertError(error: any, expectedPattern: string | RegExp): void {
  expect(error).toBeDefined();
  expect(error.message).toMatch(expectedPattern);
}

/**
 * Create test fixtures from templates
 */
export function createFixtures(): {
  models: ModelProfile[];
  tasks: AITask[];
  results: TaskResult[];
} {
  return {
    models: [
      createMockModel({ id: 'gpt-4', provider: 'openai', name: 'GPT-4' }),
      createMockModel({ id: 'claude-3', provider: 'anthropic', name: 'Claude 3' }),
      createMockModel({ id: 'gemini-pro', provider: 'google', name: 'Gemini Pro' })
    ],
    tasks: [
      createMockTask({ type: 'code_generation' }),
      createMockTask({ type: 'chat' }),
      createMockTask({ type: 'analysis' })
    ],
    results: [
      createMockResult({ status: 'completed' }),
      createMockResult({ status: 'failed' }),
      createMockResult({ status: 'cancelled' })
    ]
  };
}