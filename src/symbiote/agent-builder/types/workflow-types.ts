/**
 * Workflow Types
 * 
 * Type definitions for the visual agent workflow builder
 */

export interface AgentWorkflow {
  id: string;
  name: string;
  description?: string;
  version: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  variables: WorkflowVariable[];
  metadata: WorkflowMetadata;
}

export interface WorkflowNode {
  id: string;
  type: NodeType;
  position: { x: number; y: number };
  data: {
    label: string;
    config: any; // Node-specific configuration
    inputs?: NodePort[];
    outputs?: NodePort[];
  };
}

export type NodeType = 
  | 'agent'
  | 'tool'
  | 'condition'
  | 'loop'
  | 'parallel'
  | 'data'
  | 'input'
  | 'output'
  | 'transform'
  | 'merge'
  | 'split';

export interface NodePort {
  id: string;
  name: string;
  type: DataType;
  required?: boolean;
  multiple?: boolean;
}

export type DataType = 
  | 'any'
  | 'string'
  | 'number'
  | 'boolean'
  | 'object'
  | 'array'
  | 'file'
  | 'image'
  | 'code';

export interface WorkflowEdge {
  id: string;
  source: string;
  sourceHandle?: string;
  target: string;
  targetHandle?: string;
  type?: 'default' | 'conditional' | 'error';
  label?: string;
  data?: {
    condition?: string;
    transform?: string;
  };
}

export interface WorkflowVariable {
  id: string;
  name: string;
  type: DataType;
  value: any;
  scope: 'global' | 'local';
  description?: string;
}

export interface WorkflowMetadata {
  created: Date;
  modified: Date;
  author: string;
  tags: string[];
  category?: string;
  icon?: string;
  color?: string;
}

// Node-specific configurations

export interface AgentNodeConfig {
  agentType: string;
  agentId?: string;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  systemPrompt?: string;
  tools?: string[];
  memory?: boolean;
}

export interface ToolNodeConfig {
  toolType: 'mcp' | 'custom' | 'api' | 'builtin';
  toolId: string;
  mcpServer?: string;
  parameters?: Record<string, any>;
  timeout?: number;
  retries?: number;
}

export interface ConditionNodeConfig {
  expression: string;
  language: 'javascript' | 'python' | 'natural';
  trueBranch?: string;
  falseBranch?: string;
}

export interface LoopNodeConfig {
  loopType: 'for' | 'while' | 'forEach';
  condition?: string;
  items?: string;
  maxIterations?: number;
  parallel?: boolean;
}

export interface DataNodeConfig {
  dataType: 'input' | 'output' | 'constant' | 'variable';
  value?: any;
  schema?: any;
  validation?: string;
}

// Execution types

export interface WorkflowExecution {
  id: string;
  workflowId: string;
  status: ExecutionStatus;
  startTime: Date;
  endTime?: Date;
  input: any;
  output?: any;
  error?: WorkflowError;
  steps: ExecutionStep[];
  metrics: ExecutionMetrics;
}

export type ExecutionStatus = 
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'paused';

export interface ExecutionStep {
  nodeId: string;
  status: ExecutionStatus;
  startTime: Date;
  endTime?: Date;
  input: any;
  output?: any;
  error?: WorkflowError;
  metrics?: {
    duration: number;
    tokensUsed?: number;
    cost?: number;
  };
}

export interface WorkflowError {
  code: string;
  message: string;
  nodeId?: string;
  details?: any;
  stack?: string;
}

export interface ExecutionMetrics {
  totalDuration: number;
  nodeExecutions: number;
  tokensUsed: number;
  totalCost: number;
  memoryUsed: number;
}

// Export types

export interface WorkflowExport {
  format: ExportFormat;
  code: string;
  dependencies: string[];
  runtime: ExportRuntime;
  metadata: {
    generated: Date;
    version: string;
    checksum: string;
  };
}

export type ExportFormat = 
  | 'typescript'
  | 'javascript'
  | 'python'
  | 'docker'
  | 'serverless'
  | 'cli';

export interface ExportRuntime {
  node?: string;
  python?: string;
  docker?: string;
  requirements: string[];
}

// UI types

export interface NodePaletteItem {
  type: NodeType;
  label: string;
  icon: string;
  category: string;
  description: string;
  defaultConfig?: any;
}

export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  thumbnail?: string;
  workflow: Partial<AgentWorkflow>;
}

// Messages between extension and webview

export interface WebviewMessage {
  type: string;
  payload: any;
}

export interface SaveWorkflowMessage extends WebviewMessage {
  type: 'saveWorkflow';
  payload: AgentWorkflow;
}

export interface LoadWorkflowMessage extends WebviewMessage {
  type: 'loadWorkflow';
  payload: {
    path: string;
  };
}

export interface ExecuteWorkflowMessage extends WebviewMessage {
  type: 'executeWorkflow';
  payload: {
    workflow: AgentWorkflow;
    input?: any;
    debug?: boolean;
  };
}

export interface ExportWorkflowMessage extends WebviewMessage {
  type: 'exportWorkflow';
  payload: {
    workflow: AgentWorkflow;
    format: ExportFormat;
    options?: any;
  };
}

export interface NodeExecutionUpdate extends WebviewMessage {
  type: 'nodeExecutionUpdate';
  payload: {
    nodeId: string;
    status: ExecutionStatus;
    output?: any;
    error?: WorkflowError;
  };
}