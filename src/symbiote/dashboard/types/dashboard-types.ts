/**
 * Dashboard Types
 * 
 * Type definitions for the SymbioteIDE User Dashboard
 */

export interface DashboardState {
  // System Overview
  systemStatus: SystemStatus;
  agents: AgentStatus[];
  mcpServers: MCPServerStatus[];
  memoryUsage: MemoryMetrics;
  
  // Orchestration Metrics
  orchestrationMetrics: OrchestrationMetrics;
  modelUsage: ModelUsageStats[];
  costTracking: CostMetrics;
  
  // Activity
  activityLog: ActivityEntry[];
  executionHistory: ExecutionRecord[];
  
  // Settings
  userSettings: UserSettings;
  apiKeys: APIKeyStatus[];
  featureFlags: FeatureFlag[];
}

export interface SystemStatus {
  version: string;
  uptime: number;
  status: 'healthy' | 'degraded' | 'error';
  lastUpdated: Date;
  resources: {
    cpu: number;
    memory: {
      used: number;
      total: number;
      percentage: number;
    };
    disk: {
      used: number;
      total: number;
      percentage: number;
    };
  };
}

export interface AgentStatus {
  id: string;
  name: string;
  type: string;
  status: 'active' | 'idle' | 'error' | 'stopped';
  provider: string;
  model?: string;
  lastActivity?: Date;
  metrics: {
    totalExecutions: number;
    successRate: number;
    averageResponseTime: number;
  };
  capabilities?: string[];
}

export interface MCPServerStatus {
  id: string;
  name: string;
  url: string;
  status: 'connected' | 'disconnected' | 'error';
  transport: 'stdio' | 'http' | 'websocket';
  lastPing?: Date;
  tools: number;
  resources: number;
  prompts: number;
}

export interface MemoryMetrics {
  shortTerm: {
    entries: number;
    sizeBytes: number;
  };
  longTerm: {
    entries: number;
    sizeBytes: number;
  };
  vectorStore: {
    documents: number;
    vectors: number;
    dimensions: number;
  };
  cacheHitRate: number;
}

export interface OrchestrationMetrics {
  activeRequests: number;
  queueLength: number;
  averageLatency: number;
  throughput: {
    requestsPerMinute: number;
    tokensPerMinute: number;
  };
  errorRate: number;
  rateLimits: {
    provider: string;
    used: number;
    limit: number;
    resetAt: Date;
  }[];
}

export interface ModelUsageStats {
  model: string;
  provider: string;
  requests: number;
  tokensUsed: {
    input: number;
    output: number;
    total: number;
  };
  cost: number;
  averageLatency: number;
  errorRate: number;
}

export interface CostMetrics {
  currentPeriod: {
    start: Date;
    end: Date;
    totalCost: number;
    costByProvider: Record<string, number>;
    costByModel: Record<string, number>;
  };
  projection: {
    daily: number;
    weekly: number;
    monthly: number;
  };
  budget?: {
    limit: number;
    used: number;
    percentage: number;
    alertThreshold: number;
  };
}

export interface ActivityEntry {
  id: string;
  timestamp: Date;
  type: 'agent' | 'system' | 'user' | 'error';
  severity: 'info' | 'warning' | 'error';
  source: string;
  message: string;
  metadata?: Record<string, any>;
}

export interface ExecutionRecord {
  id: string;
  agentId: string;
  agentName: string;
  task: string;
  startTime: Date;
  endTime?: Date;
  status: 'running' | 'completed' | 'failed';
  duration?: number;
  input?: any;
  output?: any;
  error?: string;
  tokens?: {
    input: number;
    output: number;
  };
  cost?: number;
}

export interface UserSettings {
  theme: 'light' | 'dark' | 'auto';
  dashboard: {
    refreshInterval: number;
    defaultView: string;
    widgets: WidgetConfig[];
  };
  notifications: {
    enabled: boolean;
    types: string[];
    channels: string[];
  };
  telemetry: {
    enabled: boolean;
    level: 'minimal' | 'standard' | 'detailed';
  };
}

export interface APIKeyStatus {
  provider: string;
  configured: boolean;
  valid?: boolean;
  lastValidated?: Date;
  expiresAt?: Date;
  usage?: {
    used: number;
    limit: number;
    percentage: number;
  };
}

export interface FeatureFlag {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  category: string;
  experimental?: boolean;
  requiresRestart?: boolean;
}

export interface WidgetConfig {
  id: string;
  type: string;
  title: string;
  position: {
    x: number;
    y: number;
    w: number;
    h: number;
  };
  settings?: Record<string, any>;
  visible: boolean;
}

// WebView Messages
export interface DashboardMessage {
  type: DashboardMessageType;
  payload?: any;
}

export enum DashboardMessageType {
  // From WebView to Extension
  READY = 'ready',
  REFRESH = 'refresh',
  UPDATE_SETTINGS = 'updateSettings',
  EXECUTE_ACTION = 'executeAction',
  REQUEST_DATA = 'requestData',
  
  // From Extension to WebView
  STATE_UPDATE = 'stateUpdate',
  PARTIAL_UPDATE = 'partialUpdate',
  ACTION_RESULT = 'actionResult',
  ERROR = 'error',
  
  // Real-time updates
  METRIC_UPDATE = 'metricUpdate',
  AGENT_UPDATE = 'agentUpdate',
  LOG_ENTRY = 'logEntry',
  EXECUTION_UPDATE = 'executionUpdate'
}

// Dashboard Actions
export interface DashboardAction {
  type: string;
  payload?: any;
}

export interface RefreshAction extends DashboardAction {
  type: 'refresh';
  payload: {
    sections?: string[];
    force?: boolean;
  };
}

export interface ToggleAgentAction extends DashboardAction {
  type: 'toggleAgent';
  payload: {
    agentId: string;
    enabled: boolean;
  };
}

export interface UpdateSettingAction extends DashboardAction {
  type: 'updateSetting';
  payload: {
    path: string;
    value: any;
  };
}

export interface ClearCacheAction extends DashboardAction {
  type: 'clearCache';
  payload: {
    cacheType: 'all' | 'memory' | 'context' | 'results';
  };
}

// Chart Data Types
export interface TimeSeriesData {
  timestamp: Date;
  value: number;
  label?: string;
}

export interface ChartConfig {
  type: 'line' | 'bar' | 'pie' | 'donut' | 'area' | 'scatter';
  title: string;
  data: any[];
  options?: any;
}