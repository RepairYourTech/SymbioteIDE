/**
 * Queue System Types and Interfaces
 */

import { Job, Queue, Worker, QueueEvents, JobsOptions, WorkerOptions } from 'bullmq';

// Queue Names
export enum QueueName {
  CodeIndexing = 'code-indexing',
  EmbeddingGeneration = 'embedding-generation',
  MemoryConsolidation = 'memory-consolidation',
  GraphSync = 'graph-sync',
  ContextCache = 'context-cache',
  AgentTasks = 'agent-tasks',
  Notifications = 'notifications',
  Analytics = 'analytics'
}

// Job Types
export enum JobType {
  // Code Indexing Jobs
  IndexFile = 'index-file',
  IndexDirectory = 'index-directory',
  IndexProject = 'index-project',
  UpdateIndex = 'update-index',
  RemoveFromIndex = 'remove-from-index',
  
  // Embedding Jobs
  GenerateEmbedding = 'generate-embedding',
  BatchEmbeddings = 'batch-embeddings',
  UpdateEmbedding = 'update-embedding',
  
  // Memory Jobs
  ConsolidateMemories = 'consolidate-memories',
  PruneMemories = 'prune-memories',
  ExportMemories = 'export-memories',
  
  // Graph Jobs
  SyncGraphNode = 'sync-graph-node',
  UpdateRelationships = 'update-relationships',
  AnalyzeDependencies = 'analyze-dependencies',
  
  // Context Jobs
  RefreshCache = 'refresh-cache',
  PrewarmContext = 'prewarm-context',
  
  // Agent Jobs
  ExecuteAgentTask = 'execute-agent-task',
  AgentWorkflow = 'agent-workflow',
  
  // System Jobs
  SendNotification = 'send-notification',
  TrackAnalytics = 'track-analytics',
  Cleanup = 'cleanup'
}

// Job Data Interfaces
export interface CodeIndexingJobData {
  type: JobType.IndexFile | JobType.IndexDirectory | JobType.IndexProject;
  path: string;
  projectId?: string;
  language?: string;
  options?: {
    force?: boolean;
    includeTests?: boolean;
    shallow?: boolean;
  };
}

export interface EmbeddingJobData {
  type: JobType.GenerateEmbedding | JobType.BatchEmbeddings;
  content: string | string[];
  metadata?: {
    fileId?: string;
    chunkIndex?: number;
    model?: string;
    dimensions?: number;
  };
}

export interface MemoryJobData {
  type: JobType.ConsolidateMemories | JobType.PruneMemories;
  userId?: string;
  agentId?: string;
  timeRange?: {
    start: Date;
    end: Date;
  };
  options?: {
    strategy?: 'similarity' | 'temporal' | 'importance';
    threshold?: number;
  };
}

export interface GraphSyncJobData {
  type: JobType.SyncGraphNode | JobType.UpdateRelationships;
  nodeId?: string;
  nodeType?: 'file' | 'class' | 'function' | 'variable';
  relationships?: Array<{
    type: string;
    targetId: string;
  }>;
}

export interface AgentTaskJobData {
  type: JobType.ExecuteAgentTask | JobType.AgentWorkflow;
  agentId: string;
  taskId: string;
  input: any;
  context?: {
    userId?: string;
    sessionId?: string;
    priority?: number;
  };
}

export type JobData = 
  | CodeIndexingJobData
  | EmbeddingJobData
  | MemoryJobData
  | GraphSyncJobData
  | AgentTaskJobData
  | { type: JobType; [key: string]: any };

// Job Options
export interface SymbioteJobOptions extends JobsOptions {
  priority?: number;
  removeOnComplete?: boolean | number;
  removeOnFail?: boolean | number;
  attempts?: number;
  backoff?: {
    type: 'fixed' | 'exponential';
    delay: number;
  };
}

// Job Result
export interface JobResult<T = any> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: any;
  };
  metrics?: {
    duration: number;
    itemsProcessed?: number;
    bytesProcessed?: number;
  };
}

// Worker Configuration
export interface WorkerConfig extends WorkerOptions {
  concurrency?: number;
  maxStalledCount?: number;
  stalledInterval?: number;
  lockDuration?: number;
  lockRenewTime?: number;
}

// Queue Configuration
export interface QueueConfig {
  name: QueueName;
  defaultJobOptions?: SymbioteJobOptions;
  workerConfig?: WorkerConfig;
  rateLimiter?: {
    max: number;
    duration: number;
  };
}

// Job Progress
export interface JobProgress {
  percentage: number;
  message?: string;
  currentStep?: number;
  totalSteps?: number;
  details?: any;
}

// Queue Metrics
export interface QueueMetrics {
  waiting: number;
  active: number;
  completed: number;
  failed: number;
  delayed: number;
  paused: boolean;
  jobCounts: {
    [JobType: string]: number;
  };
  performance: {
    averageProcessingTime: number;
    successRate: number;
    throughput: number;
  };
}

// Event Types
export interface QueueEventHandlers {
  onCompleted?: (job: Job, result: JobResult) => void | Promise<void>;
  onFailed?: (job: Job, error: Error) => void | Promise<void>;
  onProgress?: (job: Job, progress: JobProgress) => void | Promise<void>;
  onStalled?: (job: Job) => void | Promise<void>;
  onRemoved?: (job: Job) => void | Promise<void>;
}

// Bulk Operations
export interface BulkJobOperation {
  name: string;
  data: JobData;
  opts?: SymbioteJobOptions;
}

// Job Filter
export interface JobFilter {
  types?: JobType[];
  status?: ('waiting' | 'active' | 'completed' | 'failed' | 'delayed')[];
  dateRange?: {
    start: Date;
    end: Date;
  };
  limit?: number;
  offset?: number;
}