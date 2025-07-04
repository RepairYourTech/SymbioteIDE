/**
 * Queue Manager Tests
 */

import { 
  QueueManager, 
  QueueManagerOptions,
  RequestPriority,
  RequestStatus
} from '../index';
import { AITask, TaskType } from '../../interfaces';

describe('Queue Manager', () => {
  let queueManager: QueueManager;
  const defaultConfig: QueueManagerOptions = {
    maxConcurrency: 2,
    maxQueueSize: 10,
    defaultTimeout: 5000,
    maxRetries: 2,
    enablePersistence: false,
    enableDeduplication: true,
    enableDeadLetter: true
  };

  beforeEach(() => {
    queueManager = new QueueManager(defaultConfig);
  });

  afterEach(async () => {
    await queueManager.shutdown();
  });

  it('should enqueue and process tasks', async () => {
    await queueManager.initialize();

    const task: AITask = {
      id: 'test-task-1',
      type: TaskType.General,
      prompt: 'Test prompt'
    };

    const requestId = await queueManager.enqueue(task);
    expect(requestId).toBeDefined();

    const stats = queueManager.getStats();
    expect(stats.queueLength).toBeGreaterThan(0);
  });

  it('should respect priority ordering', async () => {
    await queueManager.initialize();

    const processedOrder: string[] = [];
    
    // Override process handler to track order
    queueManager.on('process', (request) => {
      processedOrder.push(request.task.id);
    });

    // Enqueue tasks with different priorities
    await queueManager.enqueue(
      { id: 'low-1', type: TaskType.General, prompt: 'Low priority' },
      { priority: RequestPriority.Low }
    );
    
    await queueManager.enqueue(
      { id: 'critical-1', type: TaskType.General, prompt: 'Critical' },
      { priority: RequestPriority.Critical }
    );
    
    await queueManager.enqueue(
      { id: 'normal-1', type: TaskType.General, prompt: 'Normal' },
      { priority: RequestPriority.Normal }
    );

    // Wait for processing
    await new Promise(resolve => setTimeout(resolve, 100));

    // Critical should be processed first
    expect(processedOrder[0]).toBe('critical-1');
  });

  it('should handle deduplication', async () => {
    await queueManager.initialize();

    const task: AITask = {
      id: 'dedup-task',
      type: TaskType.General,
      prompt: 'Duplicate task'
    };

    const dedupKey = 'unique-key-123';
    
    const id1 = await queueManager.enqueue(task, {
      metadata: { dedupKey }
    });
    
    const id2 = await queueManager.enqueue(task, {
      metadata: { dedupKey }
    });

    // Should return the same request ID
    expect(id1).toBe(id2);
  });

  it('should enforce queue size limits', async () => {
    await queueManager.initialize();

    // Fill the queue
    for (let i = 0; i < defaultConfig.maxQueueSize; i++) {
      await queueManager.enqueue({
        id: `task-${i}`,
        type: TaskType.General,
        prompt: `Task ${i}`
      });
    }

    // This should throw
    await expect(queueManager.enqueue({
      id: 'overflow-task',
      type: TaskType.General,
      prompt: 'This should fail'
    })).rejects.toThrow('Queue is full');
  });

  it('should handle task cancellation', async () => {
    await queueManager.initialize();

    const task: AITask = {
      id: 'cancel-task',
      type: TaskType.General,
      prompt: 'Task to cancel'
    };

    const requestId = await queueManager.enqueue(task);
    const cancelled = await queueManager.cancel(requestId);
    
    expect(cancelled).toBe(true);

    // Try to cancel again
    const cancelledAgain = await queueManager.cancel(requestId);
    expect(cancelledAgain).toBe(false);
  });

  it('should provide accurate statistics', async () => {
    await queueManager.initialize();

    // Enqueue tasks with different priorities
    await queueManager.enqueue(
      { id: 'task-1', type: TaskType.General, prompt: 'Task 1' },
      { priority: RequestPriority.High }
    );
    
    await queueManager.enqueue(
      { id: 'task-2', type: TaskType.General, prompt: 'Task 2' },
      { priority: RequestPriority.Normal }
    );

    const stats = queueManager.getStats();
    
    expect(stats.queueLength).toBe(2);
    expect(stats.priorityDistribution[RequestPriority.High]).toBe(1);
    expect(stats.priorityDistribution[RequestPriority.Normal]).toBe(1);
    expect(stats.errorRate).toBe(0);
  });
});

// Run tests if this file is executed directly
if (require.main === module) {
  console.log('Running queue manager tests...');
  // In a real environment, we would use Jest or another test runner
}