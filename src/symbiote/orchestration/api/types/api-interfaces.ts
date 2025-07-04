/**
 * API Layer Interfaces
 */

import { 
  AITask, 
  TaskResult, 
  ModelProfile,
  OrchestrationMetrics,
  CostEstimate,
  TaskType
} from '../../interfaces';
import { QueueStats, RequestPriority } from '../../queue';
import { OrchestrationConfig } from '../../config';

// Request/Response types

export interface ExecuteTaskRequest {
  task: AITask;
  options?: {
    priority?: RequestPriority;
    timeout?: number;
    maxRetries?: number;
    preferredModels?: string[];
    excludeModels?: string[];
    metadata?: Record<string, any>;
  };
}

export interface ExecuteTaskResponse {
  requestId: string;
  status: 'accepted' | 'queued' | 'rejected';
  estimatedWaitTime?: number;
  queuePosition?: number;
  message?: string;
}

export interface GetTaskResultRequest {
  requestId: string;
  wait?: boolean;
  timeout?: number;
}

export interface GetTaskResultResponse {
  requestId: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  result?: TaskResult;
  error?: APIError;
  metadata?: {
    queuedAt?: string;
    startedAt?: string;
    completedAt?: string;
    retries?: number;
  };
}

export interface ExecuteBatchRequest {
  tasks: AITask[];
  options?: {
    maxConcurrency?: number;
    continueOnError?: boolean;
    priority?: RequestPriority;
  };
}

export interface ExecuteBatchResponse {
  batchId: string;
  taskCount: number;
  status: 'accepted' | 'rejected';
  estimatedCompletionTime?: number;
}

export interface GetBatchResultsRequest {
  batchId: string;
  includePartial?: boolean;
}

export interface GetBatchResultsResponse {
  batchId: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  totalTasks: number;
  completedTasks: number;
  failedTasks: number;
  results?: TaskResult[];
  errors?: APIError[];
}

export interface StreamTaskRequest {
  task: AITask;
  options?: {
    priority?: RequestPriority;
    includePartialResults?: boolean;
  };
}

export interface StreamTaskResponse {
  type: 'start' | 'progress' | 'result' | 'error' | 'complete';
  requestId: string;
  data?: {
    content?: string;
    tokens?: number;
    progress?: number;
    result?: TaskResult;
    error?: APIError;
  };
}

export interface GetModelsRequest {
  includeUnavailable?: boolean;
  taskType?: TaskType;
}

export interface GetModelsResponse {
  models: ModelProfile[];
  defaultModel?: string;
}

export interface GetMetricsRequest {
  startTime?: string;
  endTime?: string;
  aggregation?: 'minute' | 'hour' | 'day';
}

export interface GetMetricsResponse {
  metrics: OrchestrationMetrics;
  period: {
    start: string;
    end: string;
  };
}

export interface GetQueueStatusRequest {
  detailed?: boolean;
}

export interface GetQueueStatusResponse {
  stats: QueueStats;
  isAcceptingRequests: boolean;
  backpressureActive?: boolean;
  details?: {
    priorityBreakdown: Record<RequestPriority, number>;
    oldestRequestAge?: number;
    estimatedWaitTimes?: Record<RequestPriority, number>;
  };
}

export interface GetCostEstimateRequest {
  task: AITask;
  preferredModels?: string[];
}

export interface GetCostEstimateResponse {
  estimate: CostEstimate;
  selectedModel: string;
  alternatives?: Array<{
    model: string;
    estimate: CostEstimate;
  }>;
}

export interface UpdateConfigurationRequest {
  config: Partial<OrchestrationConfig>;
  validateOnly?: boolean;
}

export interface UpdateConfigurationResponse {
  success: boolean;
  validation?: {
    valid: boolean;
    errors?: string[];
    warnings?: string[];
  };
  appliedChanges?: string[];
}

export interface GetConfigurationRequest {
  path?: string;
  includeDefaults?: boolean;
}

export interface GetConfigurationResponse {
  config: OrchestrationConfig | any;
  version?: string;
  lastModified?: string;
}

export interface HealthCheckResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version: string;
  uptime: number;
  components: {
    orchestrationEngine: ComponentHealth;
    queueManager?: ComponentHealth;
    configManager?: ComponentHealth;
    providers: Record<string, ComponentHealth>;
  };
}

export interface ComponentHealth {
  status: 'healthy' | 'degraded' | 'unhealthy';
  message?: string;
  lastCheck: string;
  metadata?: Record<string, any>;
}

// Error types

export interface APIError {
  code: string;
  message: string;
  details?: any;
  timestamp: string;
  requestId?: string;
}

// Authentication types

export interface AuthToken {
  token: string;
  type: 'Bearer' | 'ApiKey';
  expiresAt?: string;
}

export interface AuthRequest {
  credentials: {
    apiKey?: string;
    username?: string;
    password?: string;
  };
  scope?: string[];
}

export interface AuthResponse {
  token: AuthToken;
  user?: {
    id: string;
    name: string;
    roles: string[];
  };
}

// WebSocket message types

export interface WebSocketMessage {
  id: string;
  type: 'request' | 'response' | 'event' | 'error';
  method?: string;
  data?: any;
  error?: APIError;
}

export interface WebSocketRequest extends WebSocketMessage {
  type: 'request';
  method: string;
  params?: any;
}

export interface WebSocketResponse extends WebSocketMessage {
  type: 'response';
  requestId: string;
  result?: any;
  error?: APIError;
}

export interface WebSocketEvent extends WebSocketMessage {
  type: 'event';
  event: string;
  data: any;
}

// API Options

export interface APIServerOptions {
  port?: number;
  host?: string;
  cors?: CORSOptions;
  auth?: AuthOptions;
  rateLimit?: RateLimitOptions;
  ssl?: SSLOptions;
  websocket?: WebSocketOptions;
  grpc?: GRPCOptions;
}

export interface CORSOptions {
  enabled: boolean;
  origins?: string[];
  methods?: string[];
  headers?: string[];
  credentials?: boolean;
}

export interface AuthOptions {
  enabled: boolean;
  type: 'apiKey' | 'jwt' | 'oauth' | 'custom';
  config?: any;
  publicPaths?: string[];
}

export interface RateLimitOptions {
  enabled: boolean;
  windowMs: number;
  maxRequests: number;
  keyGenerator?: (req: any) => string;
  skipPaths?: string[];
}

export interface SSLOptions {
  enabled: boolean;
  cert?: string;
  key?: string;
  ca?: string;
}

export interface WebSocketOptions {
  enabled: boolean;
  path?: string;
  pingInterval?: number;
  maxConnections?: number;
}

export interface GRPCOptions {
  enabled: boolean;
  port?: number;
  maxMessageSize?: number;
  keepAlive?: {
    time?: number;
    timeout?: number;
  };
}