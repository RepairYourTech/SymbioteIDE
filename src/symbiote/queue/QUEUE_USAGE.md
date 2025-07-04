# SymbioteIDE Queue System Usage Guide

## Overview

The SymbioteIDE queue system is built on Bull (BullMQ) and provides robust background job processing for various async operations including code indexing, embedding generation, memory consolidation, graph synchronization, and agent task execution.

## Quick Start

```typescript
import { QueueManager } from './symbiote/queue';
import { RedisManager } from './symbiote/redis/redis-manager';
import { QueueName, JobType } from './symbiote/queue/types';

// Initialize Redis and Queue Manager
const redisManager = RedisManager.getInstance();
await redisManager.connect();

const queueManager = new QueueManager(redisManager);
await queueManager.initialize();

// Add a job
const job = await queueManager.addJob(
  QueueName.CodeIndexing,
  'index-file',
  {
    type: JobType.IndexFile,
    path: '/path/to/file.ts',
    language: 'typescript'
  }
);

// Monitor job progress
queueManager.on('job:progress', ({ jobId, progress }) => {
  console.log(`Job ${jobId}: ${progress.percentage}% - ${progress.message}`);
});

// Handle completion
queueManager.on('job:completed', ({ jobId, result }) => {
  console.log(`Job ${jobId} completed:`, result);
});
```

## Queue Types

### 1. Code Indexing Queue

Handles file and project indexing for code intelligence.

```typescript
// Index a single file
await queueManager.addJob(
  QueueName.CodeIndexing,
  'index-file',
  {
    type: JobType.IndexFile,
    path: '/src/components/Button.tsx',
    language: 'typescript'
  }
);

// Index entire project
await queueManager.addJob(
  QueueName.CodeIndexing,
  'index-project',
  {
    type: JobType.IndexProject,
    path: '/home/user/my-project',
    projectId: 'my-project-id',
    options: {
      includeTests: false,
      shallow: false
    }
  },
  {
    priority: 10 // Higher priority
  }
);

// Remove from index
await queueManager.addJob(
  QueueName.CodeIndexing,
  'remove-file',
  {
    type: JobType.RemoveFromIndex,
    path: '/src/deprecated.ts'
  }
);
```

### 2. Embedding Generation Queue

Generates vector embeddings for semantic search.

```typescript
// Single embedding
await queueManager.addJob(
  QueueName.EmbeddingGeneration,
  'generate-embedding',
  {
    type: JobType.GenerateEmbedding,
    content: 'function calculateTotal(items) { return items.reduce((sum, item) => sum + item.price, 0); }',
    metadata: {
      fileId: 'utils.js',
      chunkIndex: 0
    }
  }
);

// Batch embeddings
await queueManager.addJob(
  QueueName.EmbeddingGeneration,
  'batch-embeddings',
  {
    type: JobType.BatchEmbeddings,
    content: [
      'First code chunk...',
      'Second code chunk...',
      'Third code chunk...'
    ],
    metadata: {
      fileId: 'large-file.js'
    }
  }
);
```

### 3. Memory Consolidation Queue

Manages AI agent memory consolidation and pruning.

```typescript
// Consolidate memories
await queueManager.addJob(
  QueueName.MemoryConsolidation,
  'consolidate',
  {
    type: JobType.ConsolidateMemories,
    agentId: 'code-assistant',
    timeRange: {
      start: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // 7 days ago
      end: new Date()
    },
    options: {
      strategy: 'similarity',
      threshold: 0.8
    }
  }
);

// Prune old memories
await queueManager.addJob(
  QueueName.MemoryConsolidation,
  'prune',
  {
    type: JobType.PruneMemories,
    agentId: 'code-assistant',
    options: {
      threshold: 30 * 24 * 60 * 60 * 1000 // 30 days
    }
  }
);
```

### 4. Graph Sync Queue

Synchronizes Neo4j graph database operations.

```typescript
// Sync graph node
await queueManager.addJob(
  QueueName.GraphSync,
  'sync-node',
  {
    type: JobType.SyncGraphNode,
    nodeId: 'src/api/user-service.ts',
    nodeType: 'File'
  }
);

// Update relationships
await queueManager.addJob(
  QueueName.GraphSync,
  'update-relationships',
  {
    type: JobType.UpdateRelationships,
    nodeId: 'UserService',
    nodeType: 'Class',
    relationships: [
      { type: 'DEPENDS_ON', targetId: 'DatabaseService' },
      { type: 'IMPLEMENTS', targetId: 'IUserService' }
    ]
  }
);

// Analyze dependencies
await queueManager.addJob(
  QueueName.GraphSync,
  'analyze-deps',
  {
    type: JobType.AnalyzeDependencies,
    nodeId: 'src/core/app.ts',
    nodeType: 'File'
  }
);
```

### 5. Agent Tasks Queue

Executes AI agent tasks and workflows.

```typescript
// Execute agent task
await queueManager.addJob(
  QueueName.AgentTasks,
  'review-code',
  {
    type: JobType.ExecuteAgentTask,
    agentId: 'code-reviewer',
    taskId: 'review-pr-123',
    input: {
      prompt: 'Review this pull request for security issues',
      context: {
        files: ['src/auth.ts', 'src/api.ts'],
        prNumber: 123
      }
    },
    context: {
      userId: 'user-123',
      priority: 5
    }
  }
);

// Execute workflow
await queueManager.addJob(
  QueueName.AgentTasks,
  'complete-review',
  {
    type: JobType.AgentWorkflow,
    agentId: 'orchestrator',
    taskId: 'code-review-workflow',
    input: {
      prompt: 'Perform complete code review',
      files: ['src/**/*.ts']
    }
  }
);
```

## Bulk Operations

Add multiple jobs at once for better performance:

```typescript
const files = [
  '/src/file1.ts',
  '/src/file2.ts',
  '/src/file3.ts'
];

const bulkJobs = files.map(file => ({
  name: 'index-file',
  data: {
    type: JobType.IndexFile,
    path: file,
    language: 'typescript'
  },
  opts: {
    removeOnComplete: true,
    attempts: 3
  }
}));

await queueManager.addBulkJobs(QueueName.CodeIndexing, bulkJobs);
```

## Job Management

### Query Jobs

```typescript
// Get specific job
const job = await queueManager.getJob(QueueName.CodeIndexing, 'job-id-123');

// Get jobs by filter
const jobs = await queueManager.getJobs(QueueName.CodeIndexing, {
  status: ['completed', 'failed'],
  types: [JobType.IndexFile],
  dateRange: {
    start: new Date('2024-01-01'),
    end: new Date()
  },
  limit: 100
});
```

### Queue Control

```typescript
// Pause queue
await queueManager.pauseQueue(QueueName.EmbeddingGeneration);

// Resume queue
await queueManager.resumeQueue(QueueName.EmbeddingGeneration);

// Clean old jobs
await queueManager.cleanQueue(
  QueueName.CodeIndexing,
  5000,  // Grace period in ms
  100,   // Max jobs to remove
  'completed' // Status to clean
);
```

## Monitoring

### Get Queue Metrics

```typescript
// Single queue metrics
const metrics = await queueManager.getQueueMetrics(QueueName.CodeIndexing);
console.log({
  waiting: metrics.waiting,
  active: metrics.active,
  completed: metrics.completed,
  failed: metrics.failed,
  throughput: metrics.performance.throughput,
  successRate: metrics.performance.successRate
});

// All queues metrics
const allMetrics = await queueManager.getAllMetrics();
```

### Event Monitoring

```typescript
// Job events
queueManager.on('job:completed', ({ queue, jobId, result }) => {
  console.log(`Job ${jobId} in queue ${queue} completed`);
});

queueManager.on('job:failed', ({ queue, jobId, reason }) => {
  console.error(`Job ${jobId} in queue ${queue} failed: ${reason}`);
});

queueManager.on('job:progress', ({ queue, jobId, progress }) => {
  console.log(`Job ${jobId}: ${progress.percentage}%`);
});

// Worker events
queueManager.on('worker:error', ({ queue, error }) => {
  console.error(`Worker error in queue ${queue}:`, error);
});

// Metrics events
queueManager.on('metrics:updated', (metrics) => {
  // Send to monitoring system
});
```

## Advanced Features

### Custom Job Options

```typescript
await queueManager.addJob(
  QueueName.CodeIndexing,
  'complex-job',
  { /* job data */ },
  {
    priority: 10,              // Higher number = higher priority
    delay: 5000,              // Start after 5 seconds
    attempts: 5,              // Retry up to 5 times
    backoff: {
      type: 'exponential',
      delay: 2000             // Initial delay of 2 seconds
    },
    removeOnComplete: 100,    // Keep last 100 completed jobs
    removeOnFail: 1000,       // Keep last 1000 failed jobs
    timeout: 300000           // 5 minute timeout
  }
);
```

### Rate Limiting

Embedding generation queue has built-in rate limiting:

```typescript
// Configured at queue creation
{
  rateLimiter: {
    max: 100,        // 100 requests
    duration: 60000  // per minute
  }
}
```

### Progress Tracking

```typescript
// In your processor
await job.updateProgress({
  percentage: 50,
  message: 'Processing halfway complete',
  currentStep: 5,
  totalSteps: 10,
  details: {
    filesProcessed: 25,
    totalFiles: 50
  }
});
```

## Error Handling

### Retry Logic

Each processor implements custom retry logic:

```typescript
// In processor
protected shouldRetry(error: Error, job: Job): boolean {
  // Retry on transient errors
  if (error.message.includes('ECONNREFUSED')) return true;
  
  // Don't retry validation errors
  if (error.message.includes('INVALID')) return false;
  
  // Retry up to configured attempts
  return job.attemptsMade < (job.opts.attempts || 3);
}
```

### Final Failure Handling

```typescript
// Register handlers
queueManager.registerHandlers(QueueName.CodeIndexing, {
  onFailed: async (job, error) => {
    // Send notification
    // Log to error tracking service
    // Trigger cleanup
  }
});
```

## Best Practices

1. **Use Bulk Operations**: When adding many similar jobs, use `addBulkJobs`
2. **Set Priorities**: Use priority for time-sensitive operations
3. **Monitor Metrics**: Track queue performance and adjust concurrency
4. **Handle Failures**: Implement proper error handling and notifications
5. **Clean Up**: Regularly clean completed/failed jobs to save memory
6. **Use Appropriate Queues**: Each queue has specific concurrency and rate limits

## Environment Variables

```bash
# Redis Configuration
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=your_password
REDIS_QUEUE_DB=2  # Separate DB for queues

# Embedding Service
EMBEDDING_PROVIDER=openai
OPENAI_API_KEY=your_api_key
EMBEDDING_MODEL=text-embedding-3-small
EMBEDDING_DIMENSIONS=1536

# Neo4j
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=password

# Qdrant
QDRANT_URL=http://localhost:6333
QDRANT_API_KEY=optional_key
```

## Shutdown

Always gracefully shutdown the queue manager:

```typescript
// In your application shutdown handler
await queueManager.shutdown();
```

This ensures all workers complete their current jobs and connections are properly closed.