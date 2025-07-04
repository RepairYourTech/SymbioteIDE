/**
 * MCP Types
 * 
 * Type definitions for Model Context Protocol integration
 */

export interface McpServerInfo {
  id: string;
  definition: McpServerDefinition;
  status: McpServerStatus;
  process?: McpServerProcess;
  connection?: McpConnection;
  capabilities?: McpCapabilities;
  lastError?: string;
  restartCount: number;
}

export interface McpServerDefinition {
  command: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  autoStart?: boolean;
  startupTimeout?: number;
  maxRestarts?: number;
}

export enum McpServerStatus {
  Stopped = 'stopped',
  Starting = 'starting',
  Running = 'running',
  Stopping = 'stopping',
  Error = 'error',
  Crashed = 'crashed'
}

export interface McpServerProcess {
  pid: number;
  startTime: Date;
  memoryUsage?: number;
  cpuUsage?: number;
}

export interface McpConnection {
  transport: 'stdio' | 'websocket' | 'http';
  connected: boolean;
  lastPing?: Date;
  latency?: number;
}

export interface McpCapabilities {
  tools?: McpTool[];
  resources?: McpResource[];
  prompts?: McpPrompt[];
  version: string;
}

export interface McpTool {
  name: string;
  description?: string;
  inputSchema?: any;
  outputSchema?: any;
}

export interface McpResource {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
}

export interface McpPrompt {
  name: string;
  description?: string;
  arguments?: McpPromptArgument[];
}

export interface McpPromptArgument {
  name: string;
  description?: string;
  type: string;
  required?: boolean;
  default?: any;
}

export interface McpRequest {
  jsonrpc: '2.0';
  id: string | number;
  method: string;
  params?: any;
}

export interface McpResponse {
  jsonrpc: '2.0';
  id: string | number;
  result?: any;
  error?: McpError;
}

export interface McpNotification {
  jsonrpc: '2.0';
  method: string;
  params?: any;
}

export interface McpError {
  code: number;
  message: string;
  data?: any;
}

export interface McpMessage {
  type: 'request' | 'response' | 'notification';
  data: McpRequest | McpResponse | McpNotification;
  timestamp: Date;
  direction: 'incoming' | 'outgoing';
}

export interface McpServerEvent {
  serverId: string;
  type: McpServerEventType;
  data?: any;
  timestamp: Date;
}

export enum McpServerEventType {
  Started = 'started',
  Stopped = 'stopped',
  Connected = 'connected',
  Disconnected = 'disconnected',
  Error = 'error',
  CapabilitiesUpdated = 'capabilities-updated',
  MessageReceived = 'message-received',
  MessageSent = 'message-sent'
}

export interface McpToolExecutionRequest {
  serverId: string;
  toolName: string;
  arguments: any;
  timeout?: number;
}

export interface McpToolExecutionResult {
  serverId: string;
  toolName: string;
  success: boolean;
  result?: any;
  error?: string;
  duration: number;
}

export interface McpResourceReadRequest {
  serverId: string;
  uri: string;
}

export interface McpResourceContent {
  uri: string;
  content: any;
  mimeType?: string;
  encoding?: string;
}

export interface McpPromptExecutionRequest {
  serverId: string;
  promptName: string;
  arguments: Record<string, any>;
}

export interface McpPromptResult {
  serverId: string;
  promptName: string;
  result: string;
}

// Configuration types
export interface McpConfiguration {
  mcpServers: Record<string, McpServerDefinition>;
}

// Manager options
export interface McpManagerOptions {
  autoStart?: boolean;
  maxConcurrentServers?: number;
  defaultTimeout?: number;
  healthCheckInterval?: number;
  messageQueueSize?: number;
}

// Integration types for orchestration
export interface McpToolMapping {
  [taskType: string]: string[];
}

export interface McpIntegrationConfig {
  serverId: string;
  toolMapping?: McpToolMapping;
  defaultTools?: string[];
  priority?: number;
}