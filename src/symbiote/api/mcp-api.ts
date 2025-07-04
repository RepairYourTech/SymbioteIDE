/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

import { Event } from 'vscode';

export interface IMcpAPI {
  // Server management
  connectServer(config: McpServerConfig): Promise<McpConnection>;
  disconnectServer(serverId: string): Promise<void>;
  listServers(): Promise<McpServer[]>;
  getServerStatus(serverId: string): Promise<McpServerStatus>;
  
  // Tool discovery and execution
  listTools(serverId?: string): Promise<McpTool[]>;
  executeTool(toolId: string, params: any): Promise<McpToolResult>;
  
  // Resource management
  listResources(serverId?: string): Promise<McpResource[]>;
  readResource(resourceUri: string): Promise<McpResourceContent>;
  subscribeResource(resourceUri: string, callback: (content: McpResourceContent) => void): Disposable;
  
  // Prompt management
  listPrompts(serverId?: string): Promise<McpPrompt[]>;
  getPrompt(promptId: string): Promise<McpPromptDetails>;
  
  // Communication
  sendMessage(serverId: string, message: McpMessage): Promise<McpResponse>;
  onMessage: Event<McpMessageEvent>;
  onError: Event<McpErrorEvent>;
  
  // Security
  validatePermissions(serverId: string, operation: McpOperation): Promise<boolean>;
  auditLog(serverId?: string): Promise<McpAuditEntry[]>;
}

export interface McpServerConfig {
  id: string;
  name: string;
  type: 'stdio' | 'websocket' | 'http';
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  workingDirectory?: string;
  permissions: McpPermissions;
  autoConnect?: boolean;
  timeout?: number;
}

export interface McpConnection {
  serverId: string;
  connected: boolean;
  protocol: string;
  capabilities: McpCapabilities;
  metadata?: Record<string, any>;
}

export interface McpServer {
  id: string;
  name: string;
  status: McpServerStatus;
  connection?: McpConnection;
  config: McpServerConfig;
  tools: number;
  resources: number;
  prompts: number;
}

export interface McpServerStatus {
  state: 'connected' | 'disconnected' | 'connecting' | 'error';
  lastSeen?: Date;
  error?: string;
  health?: ServerHealth;
}

export interface ServerHealth {
  responseTime: number;
  errorRate: number;
  uptime: number;
}

export interface McpCapabilities {
  tools?: boolean;
  resources?: boolean;
  prompts?: boolean;
  streaming?: boolean;
  batching?: boolean;
  versioning?: string;
  extensions?: string[];
}

export interface McpTool {
  id: string;
  serverId: string;
  name: string;
  description: string;
  inputSchema: JsonSchema;
  outputSchema?: JsonSchema;
  examples?: ToolExample[];
  permissions?: string[];
  cost?: number;
  experimental?: boolean;
}

export interface JsonSchema {
  type: string;
  properties?: Record<string, any>;
  required?: string[];
  additionalProperties?: boolean;
  [key: string]: any;
}

export interface ToolExample {
  input: any;
  output: any;
  description?: string;
}

export interface McpToolResult {
  success: boolean;
  output?: any;
  error?: string;
  duration: number;
  metadata?: Record<string, any>;
}

export interface McpResource {
  uri: string;
  serverId: string;
  name: string;
  description?: string;
  mimeType?: string;
  size?: number;
  lastModified?: Date;
  permissions?: string[];
  subscribable?: boolean;
}

export interface McpResourceContent {
  uri: string;
  content: any;
  mimeType: string;
  encoding?: string;
  metadata?: Record<string, any>;
}

export interface McpPrompt {
  id: string;
  serverId: string;
  name: string;
  description: string;
  arguments?: PromptArgument[];
  templates?: PromptTemplate[];
  examples?: PromptExample[];
}

export interface PromptArgument {
  name: string;
  description: string;
  type: string;
  required?: boolean;
  default?: any;
}

export interface PromptTemplate {
  id: string;
  content: string;
  variables: string[];
}

export interface PromptExample {
  arguments: Record<string, any>;
  result: string;
}

export interface McpPromptDetails extends McpPrompt {
  implementation: string;
  metadata?: Record<string, any>;
}

export interface McpMessage {
  type: 'request' | 'response' | 'notification' | 'error';
  method?: string;
  params?: any;
  id?: string | number;
}

export interface McpResponse {
  success: boolean;
  result?: any;
  error?: McpError;
  id?: string | number;
}

export interface McpError {
  code: number;
  message: string;
  data?: any;
}

export interface McpMessageEvent {
  serverId: string;
  message: McpMessage;
  timestamp: Date;
}

export interface McpErrorEvent {
  serverId: string;
  error: McpError;
  context?: string;
  timestamp: Date;
}

export interface McpOperation {
  type: 'tool' | 'resource' | 'prompt' | 'admin';
  action: string;
  target?: string;
  params?: any;
}

export interface McpPermissions {
  tools: ToolPermissions;
  resources: ResourcePermissions;
  system: SystemPermissions;
  custom?: Record<string, any>;
}

export interface ToolPermissions {
  allowed: string[] | '*';
  blocked?: string[];
  rateLimit?: RateLimit;
}

export interface ResourcePermissions {
  read: string[] | '*';
  write?: string[];
  subscribe?: string[];
  patterns?: PermissionPattern[];
}

export interface PermissionPattern {
  pattern: string;
  permissions: string[];
  deny?: boolean;
}

export interface SystemPermissions {
  maxMemory?: string;
  maxCpu?: number;
  networkAccess?: boolean;
  fileSystemAccess?: 'none' | 'read' | 'write';
  processSpawn?: boolean;
}

export interface RateLimit {
  requests: number;
  window: number; // seconds
  burst?: number;
}

export interface McpAuditEntry {
  id: string;
  serverId: string;
  timestamp: Date;
  operation: McpOperation;
  result: 'allowed' | 'denied' | 'error';
  user?: string;
  details?: Record<string, any>;
}

export interface Disposable {
  dispose(): void;
}

// MCP Protocol Constants
export const MCP_PROTOCOL_VERSION = '1.0';

export const MCP_ERROR_CODES = {
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,
  SERVER_ERROR: -32000,
  PERMISSION_DENIED: -32001,
  RESOURCE_NOT_FOUND: -32002,
  TOOL_EXECUTION_FAILED: -32003,
  TIMEOUT: -32004,
  RATE_LIMITED: -32005
} as const;