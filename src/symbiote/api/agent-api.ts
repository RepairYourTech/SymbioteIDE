/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

import { Event } from 'vscode';

export interface IAgentAPI {
  // Agent lifecycle
  createAgent(config: AgentConfig): Promise<Agent>;
  startAgent(agentId: string): Promise<void>;
  stopAgent(agentId: string): Promise<void>;
  destroyAgent(agentId: string): Promise<void>;
  
  // Agent communication
  sendMessage(agentId: string, message: AgentMessage): Promise<void>;
  onMessage: Event<AgentMessageEvent>;
  
  // Agent management
  listAgents(): Promise<Agent[]>;
  getAgent(agentId: string): Promise<Agent | undefined>;
  getAgentStatus(agentId: string): Promise<AgentStatus>;
  
  // Workflow management
  createWorkflow(config: WorkflowConfig): Promise<Workflow>;
  executeWorkflow(workflowId: string, input: any): Promise<WorkflowResult>;
  
  // Template management
  loadTemplate(templateId: string): Promise<AgentTemplate>;
  saveTemplate(template: AgentTemplate): Promise<void>;
  listTemplates(): Promise<AgentTemplate[]>;
}

export interface Agent {
  id: string;
  name: string;
  type: AgentType;
  config: AgentConfig;
  status: AgentStatus;
  createdAt: Date;
  capabilities: string[];
  metadata: Record<string, any>;
}

export enum AgentType {
  Development = 'development',
  Testing = 'testing',
  Documentation = 'documentation',
  Review = 'review',
  Research = 'research',
  Custom = 'custom'
}

export interface AgentConfig {
  name: string;
  type: AgentType;
  model?: string;
  tools: string[];
  permissions: AgentPermissions;
  resources?: AgentResources;
  environment?: Record<string, string>;
  timeout?: number;
  retryPolicy?: RetryPolicy;
}

export interface AgentPermissions {
  fileSystem: FileSystemPermissions;
  network: NetworkPermissions;
  execution: ExecutionPermissions;
  memory: MemoryPermissions;
}

export interface FileSystemPermissions {
  read: string[];
  write: string[];
  delete: string[];
  execute: string[];
}

export interface NetworkPermissions {
  allowedHosts: string[];
  allowedPorts: number[];
  allowedProtocols: string[];
}

export interface ExecutionPermissions {
  allowSpawn: boolean;
  allowedCommands: string[];
  maxProcesses: number;
}

export interface MemoryPermissions {
  maxMemory: string;
  persistentStorage: boolean;
  sharedMemory: boolean;
}

export interface AgentResources {
  cpu: number;
  memory: string;
  disk: string;
  gpu?: boolean;
}

export interface RetryPolicy {
  maxRetries: number;
  backoffMultiplier: number;
  maxBackoff: number;
}

export enum AgentStatus {
  Created = 'created',
  Starting = 'starting',
  Running = 'running',
  Idle = 'idle',
  Busy = 'busy',
  Stopping = 'stopping',
  Stopped = 'stopped',
  Failed = 'failed',
  Crashed = 'crashed'
}

export interface AgentMessage {
  type: MessageType;
  content: any;
  context?: MessageContext;
  priority?: MessagePriority;
}

export enum MessageType {
  Command = 'command',
  Query = 'query',
  Response = 'response',
  Event = 'event',
  Error = 'error',
  Progress = 'progress',
  Result = 'result'
}

export interface MessageContext {
  workspaceFolder?: string;
  activeFile?: string;
  selection?: Range;
  variables?: Record<string, any>;
}

export interface Range {
  start: Position;
  end: Position;
}

export interface Position {
  line: number;
  character: number;
}

export enum MessagePriority {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
  Critical = 'critical'
}

export interface AgentMessageEvent {
  agentId: string;
  message: AgentMessage;
  timestamp: Date;
}

export interface Workflow {
  id: string;
  name: string;
  description?: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  variables: Record<string, any>;
  triggers?: WorkflowTrigger[];
}

export interface WorkflowNode {
  id: string;
  type: 'agent' | 'condition' | 'loop' | 'parallel' | 'sequence';
  agentId?: string;
  config: any;
  position: { x: number; y: number };
}

export interface WorkflowEdge {
  id: string;
  source: string;
  target: string;
  condition?: string;
  label?: string;
}

export interface WorkflowTrigger {
  type: 'manual' | 'event' | 'schedule' | 'webhook';
  config: any;
}

export interface WorkflowConfig {
  name: string;
  description?: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  variables?: Record<string, any>;
  triggers?: WorkflowTrigger[];
}

export interface WorkflowResult {
  success: boolean;
  outputs: Record<string, any>;
  errors?: Error[];
  duration: number;
  nodeResults: Record<string, NodeResult>;
}

export interface NodeResult {
  nodeId: string;
  status: 'success' | 'failure' | 'skipped';
  output?: any;
  error?: Error;
  duration: number;
}

export interface AgentTemplate {
  id: string;
  name: string;
  description: string;
  type: AgentType;
  config: Partial<AgentConfig>;
  author: string;
  version: string;
  tags: string[];
  icon?: string;
  examples?: TemplateExample[];
}

export interface TemplateExample {
  name: string;
  description: string;
  input: any;
  expectedOutput: any;
}