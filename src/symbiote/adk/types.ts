/**
 * Google AI ADK Types and Interfaces
 * 
 * Core types for integrating Google's Agent Development Kit (ADK)
 * as the primary agent framework for SymbioteIDE
 */

// Temporarily comment out @google/adk imports until we have the package
// import { LlmAgent, Agent, SequentialAgent, ParallelAgent, LoopAgent } from '@google/adk';

// Import orchestration types
import { 
  ModelProfile as OrchestrationModelProfile, 
  ProviderType,
  Capability as OrchestrationCapability
} from '../orchestration/interfaces';

// Re-export ModelProfile for local use
export type ModelProfile = OrchestrationModelProfile;

// Export the types that other files are expecting
export interface ADKAgent {
  id: string;
  name: string;
  type: ADKAgentType;
  config: ADKAgentConfig;
  execute: (task: ADKTask | any) => Promise<ADKExecutionResult>;
  executeTask?: (task: string, context?: any) => Promise<any>;
  on?: (event: string, handler: (data: any) => void) => void;
  logger?: any;
  stream?: boolean;
  addTool?: (tool: any) => void;
  removeTool?: (toolName: string) => void;
}

export interface ADKModelInfo {
  provider: string;
  model: string;
  maxTokens: number;
  temperature?: number;
  capabilities?: string[];
}

// Placeholder types for @google/adk until we have the actual package
export interface LlmAgent extends ADKAgent {
  model: ADKModelInfo;
}

export interface SequentialAgent extends ADKAgent {
  agents: ADKAgent[];
}

export interface ParallelAgent extends ADKAgent {
  agents: ADKAgent[];
}

export interface LoopAgent extends ADKAgent {
  agent: ADKAgent;
  condition: (result: any) => boolean;
}

// Use ADKTool interface instead of Tool

export interface Memory {
  type: string;
  store: (key: string, value: any) => Promise<void>;
  retrieve: (key: string) => Promise<any>;
  search: (query: string) => Promise<any[]>;
}

// ADK Agent Types
export enum ADKAgentType {
  LLM = 'llm',
  Workflow = 'workflow',
  Sequential = 'sequential',
  Parallel = 'parallel',
  Loop = 'loop',
  Custom = 'custom'
}

// Agent Capabilities
export interface AgentCapabilities {
  streaming: boolean;
  multiModal: boolean;
  toolUse: boolean;
  memoryAccess: boolean;
  a2aProtocol: boolean;
  mcpTools: boolean;
}

// Agent Configuration
export interface ADKAgentConfig {
  id: string;
  name: string;
  type: ADKAgentType;
  description?: string;
  model?: ModelProfile;
  capabilities: AgentCapabilities;
  systemPrompt?: string;
  temperature?: number;
  maxTokens?: number;
  tools?: ADKTool[];
  memory?: MemoryConfig;
  a2aConfig?: A2AConfig;
  workflow?: WorkflowConfig;
}

// Tool Definition
export interface ADKTool {
  name: string;
  description: string;
  type?: 'builtin' | 'mcp' | 'custom' | 'agent';
  implementation?: ToolImplementation;
  parameters?: ToolParameter[] | { schema?: any; [key: string]: any };
  execute?: (params: any) => Promise<any>;
}

export interface ToolImplementation {
  source: 'code' | 'mcp' | 'openapi' | 'agent';
  handler?: Function | ((params: any) => Promise<any>);
  mcpServer?: string;
  mcpTool?: string;
  openApiSpec?: string;
  agentId?: string;
}

export interface ToolParameter {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'object' | 'array';
  description?: string;
  required?: boolean;
  default?: any;
}

// Memory Configuration
export interface MemoryConfig {
  enabled: boolean;
  type: 'episodic' | 'semantic' | 'working' | 'long_term';
  provider: 'mem0' | 'custom';
  scope: 'agent' | 'project' | 'team' | 'global';
  retentionDays?: number;
}

// A2A Protocol Configuration
export interface A2AConfig {
  enabled: boolean;
  agentCard: AgentCard;
  endpoints: A2AEndpoints;
  authentication?: A2AAuth;
}

export interface AgentCard {
  name: string;
  description: string;
  version: string;
  capabilities: string[];
  inputSchema?: any;
  outputSchema?: any;
  authentication?: AuthenticationSchema;
  endpoints?: EndpointDefinition[];
  metadata?: Record<string, any>;
}

export interface EndpointDefinition {
  name: string;
  path: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  description?: string;
}

export interface AuthenticationSchema {
  type: 'none' | 'apiKey' | 'oauth2' | 'bearer' | 'custom';
  scheme?: string;
  description?: string;
}

export interface A2AEndpoints {
  discovery: string;
  task: string;
  message: string;
  artifact: string;
}

export interface A2AAuth {
  type: 'none' | 'apiKey' | 'oauth2' | 'custom' | 'bearer';
  config?: any;
  credentials?: any;
}

// Task Definition
export interface ADKTask {
  id: string;
  type: string;
  description: string;
  input: any;
  context?: TaskContext;
  constraints?: TaskConstraints;
  metadata?: Record<string, any>;
}

export interface TaskContext {
  previousTasks?: string[];
  memories?: string[];
  files?: string[];
  tools?: string[];
  agents?: string[];
}

export interface TaskConstraints {
  timeout?: number;
  maxRetries?: number;
  requiredCapabilities?: string[];
  preferredModels?: string[];
}

// Execution Result
export interface ADKExecutionResult {
  taskId: string;
  agentId: string;
  status: 'success' | 'failure' | 'partial';
  output: any;
  artifacts?: Artifact[];
  messages?: Message[];
  usage?: UsageMetrics;
  duration: number;
  metadata?: Record<string, any>;
}

export interface Artifact {
  id: string;
  type: string;
  content: any;
  mimeType?: string;
  metadata?: Record<string, any>;
}

export interface Message {
  role: 'agent' | 'user' | 'system';
  content: string;
  timestamp: Date;
  metadata?: Record<string, any>;
}

export interface UsageMetrics {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  cost?: number;
  model?: string;
}

// Workflow Types
export interface WorkflowDefinition {
  id: string;
  name: string;
  description?: string;
  agents: WorkflowAgent[];
  flow: WorkflowFlow;
  inputs?: WorkflowInput[];
  outputs?: WorkflowOutput[];
}

export interface WorkflowAgent {
  id: string;
  agentId: string;
  config?: Partial<ADKAgentConfig>;
  dependencies?: string[];
}

export interface WorkflowFlow {
  type: 'sequential' | 'parallel' | 'conditional' | 'loop';
  steps: WorkflowStep[];
  conditions?: WorkflowCondition[];
}

export interface WorkflowStep {
  id: string;
  agentId: string;
  task: Partial<ADKTask>;
  onSuccess?: string;
  onFailure?: string;
  retryPolicy?: RetryPolicy;
}

export interface WorkflowCondition {
  id: string;
  expression: string;
  trueBranch: string;
  falseBranch?: string;
}

export interface RetryPolicy {
  maxAttempts: number;
  backoff: 'fixed' | 'exponential';
  delay: number;
}

export interface WorkflowInput {
  name: string;
  type: string;
  description?: string;
  required?: boolean;
  default?: any;
}

export interface WorkflowOutput {
  name: string;
  type: string;
  source: string;
  transform?: string;
}

// Agent Registry
export interface AgentRegistryEntry {
  agent: ADKAgentConfig;
  implementation: any; // ADK Agent instance
  status: 'active' | 'inactive' | 'error';
  lastUsed?: Date;
  metrics?: AgentMetrics;
}

export interface AgentMetrics {
  totalExecutions: number;
  successRate: number;
  averageDuration: number;
  totalTokensUsed: number;
  totalCost: number;
}

// Execution Types
export interface ExecutionContext {
  taskId: string;
  agentId: string;
  parentContext?: ExecutionContext;
  variables: Record<string, any>;
  metadata?: Record<string, any>;
}

export interface ExecutionResult {
  success: boolean;
  output: any;
  errors?: string[];
  metadata?: Record<string, any>;
}

// Event Types
export interface ADKEvent {
  type: ADKEventType;
  agentId: string;
  taskId?: string;
  timestamp: Date;
  data: any;
}

export enum ADKEventType {
  AgentStarted = 'agent.started',
  AgentCompleted = 'agent.completed',
  AgentFailed = 'agent.failed',
  TaskStarted = 'task.started',
  TaskCompleted = 'task.completed',
  TaskFailed = 'task.failed',
  ToolCalled = 'tool.called',
  ToolCompleted = 'tool.completed',
  MemoryStored = 'memory.stored',
  MemoryRetrieved = 'memory.retrieved',
  A2AMessageSent = 'a2a.message.sent',
  A2AMessageReceived = 'a2a.message.received'
}

// Integration Points
export interface ADKIntegration {
  orchestrationEngine: boolean;
  mcpServers: string[];
  a2aAgents: string[];
  memorySystem: boolean;
  knowledgeBase: boolean;
  notebookSystem: boolean;
  taskManager: boolean;
}

// Builder Pattern for Agent Creation
export interface AgentBuilder {
  withId(id: string): AgentBuilder;
  withName(name: string): AgentBuilder;
  withType(type: ADKAgentType): AgentBuilder;
  withModel(model: ModelProfile): AgentBuilder;
  withSystemPrompt(prompt: string): AgentBuilder;
  withTools(tools: ADKTool[]): AgentBuilder;
  withMemory(config: MemoryConfig): AgentBuilder;
  withA2A(config: A2AConfig): AgentBuilder;
  build(): ADKAgentConfig;
}

// Orchestrator Types

export interface ADKOrchestratorConfig {
  agentManagerConfig?: any;
  redis?: boolean;
  mcpEnabled?: boolean;
  a2aServerConfig?: any;
  cacheResults?: boolean;
  cacheTTL?: number;
  environment?: string;
  monitoring?: boolean;
}

export interface AgentExecutionRequest {
  agentId: string;
  input: any;
  context?: Record<string, any>;
  userId?: string;
  sessionId?: string;
  parentExecutionId?: string;
  streaming?: boolean;
}

export interface AgentExecutionResult {
  executionId: string;
  agentId: string;
  status: 'completed' | 'failed' | 'cancelled';
  result?: any;
  artifacts?: any[];
  error?: {
    code: string;
    message: string;
    details?: any;
  };
  metadata: {
    startTime: Date;
    endTime: Date;
    duration: number;
    tokensUsed?: number;
    [key: string]: any;
  };
}

export enum OrchestratorEvent {
  Initialized = 'orchestrator:initialized',
  Shutdown = 'orchestrator:shutdown',
  AgentCreated = 'agent:created',
  AgentRemoved = 'agent:removed',
  ExecutionStarted = 'execution:started',
  ExecutionCompleted = 'execution:completed',
  ExecutionFailed = 'execution:failed',
  A2ATaskCreated = 'a2a:task:created'
}

// Additional types for dynamic agent management
export interface AgentTeam {
  id: string;
  name: string;
  agents: Map<string, any>; // agentId -> Agent instance
  capabilities: Set<string>;
  metrics: AgentMetrics;
}

export interface AgentRole {
  id: string;
  name: string;
  capabilities: string[];
  priority: 'low' | 'medium' | 'high';
}

export interface AgentCapability {
  name: string;
  description: string;
  requiredTools?: string[];
  requiredModels?: string[];
}

// Re-export ProviderType
export { ProviderType };

// Extended capability type that includes custom capabilities
export type Capability = OrchestrationCapability | 
  'long-context' | 
  'multimodal' | 
  'tool-use' | 
  'code-generation' | 
  'analysis' | 
  'reasoning' | 
  'security' | 
  'testing' |
  'speed' |
  'efficiency' |
  'web-search' |
  'research' |
  'memory' |
  'personalization' |
  'documentation';
  
export { OrchestrationCapability };

export interface WorkflowConfig {
  type: 'sequential' | 'parallel' | 'conditional' | 'loop';
  agents: string[]; // Agent IDs
  steps?: Array<{
    name: string;
    agentId: string;
    input: any;
    condition?: string;
  }>;
}