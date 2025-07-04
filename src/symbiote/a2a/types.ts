/**
 * A2A Protocol Types
 * 
 * Types for Google's Agent-to-Agent (A2A) protocol v0.2
 */

// Core A2A Types

export interface AgentCard {
  name: string;
  description: string;
  version: string;
  capabilities: string[];
  inputSchema?: any;
  outputSchema?: any;
  authentication?: AuthenticationSchema;
  endpoints: EndpointDefinition[];
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
  bearerFormat?: string;
  flows?: OAuthFlows;
}

export interface OAuthFlows {
  authorizationCode?: OAuthFlow;
  implicit?: OAuthFlow;
  password?: OAuthFlow;
  clientCredentials?: OAuthFlow;
}

export interface OAuthFlow {
  authorizationUrl?: string;
  tokenUrl?: string;
  refreshUrl?: string;
  scopes: Record<string, string>;
}

// Task Management

export interface Task {
  id: string;
  agentId: string;
  type: string;
  status: TaskStatus;
  input: any;
  output?: any;
  context?: TaskContext;
  artifacts?: string[]; // Artifact IDs
  error?: TaskError;
  metadata: TaskMetadata;
  createdAt: Date;
  updatedAt: Date;
  description?: string;
  name?: string;
}

export enum TaskStatus {
  Created = 'created',
  InProgress = 'in_progress',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled'
}

export interface TaskContext {
  previousTaskId?: string;
  parentTaskId?: string;
  sessionId?: string;
  userId?: string;
  additionalInstructions?: string;
  [key: string]: any;
}

export interface TaskError {
  code: string;
  message: string;
  details?: any;
}

export interface TaskMetadata {
  startTime?: Date;
  endTime?: Date;
  duration?: number;
  retryCount?: number;
  [key: string]: any;
}

// Messages

export interface Message {
  id: string;
  taskId: string;
  type: MessageType;
  sender: 'client' | 'agent';
  content: string;
  metadata?: Record<string, any>;
  timestamp: Date;
}

export enum MessageType {
  Instruction = 'instruction',
  Context = 'context',
  Status = 'status',
  Progress = 'progress',
  Thought = 'thought',
  Cancel = 'cancel',
  Custom = 'custom'
}

// Artifacts

export interface Artifact {
  id: string;
  taskId: string;
  type: string;
  content: any;
  metadata: ArtifactMetadata;
}

export interface ArtifactMetadata {
  mimeType: string;
  size?: number;
  encoding?: string;
  createdAt: Date;
  [key: string]: any;
}

// Parts (content units)

export interface Part {
  type: PartType;
  content: any;
  metadata?: Record<string, any>;
}

export enum PartType {
  Text = 'text',
  File = 'file',
  Form = 'form',
  Image = 'image',
  Audio = 'audio',
  Video = 'video',
  Custom = 'custom'
}

// JSON-RPC Types

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  method: string;
  params?: any;
  id: string | number;
}

export interface JsonRpcResponse {
  jsonrpc: '2.0';
  result?: any;
  error?: JsonRpcError;
  id: string | number;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: any;
}

// A2A Error Codes
export enum A2AErrorCode {
  ParseError = -32700,
  InvalidRequest = -32600,
  MethodNotFound = -32601,
  InvalidParams = -32602,
  InternalError = -32603,
  
  // A2A specific errors
  AgentNotFound = -32000,
  TaskNotFound = -32001,
  ArtifactNotFound = -32002,
  AuthenticationRequired = -32003,
  AuthorizationFailed = -32004,
  RateLimitExceeded = -32005,
  TaskTimeout = -32006,
  InvalidTaskState = -32007
}

// A2A Error
export class A2AError extends Error {
  constructor(
    public code: string,
    message: string,
    public data?: any
  ) {
    super(message);
    this.name = 'A2AError';
  }

  toJsonRpcError(): JsonRpcError {
    const errorCode = A2AErrorCode[this.code as keyof typeof A2AErrorCode] || A2AErrorCode.InternalError;
    return {
      code: errorCode,
      message: this.message,
      data: this.data
    };
  }
}

// Client Configuration

export interface A2AClientConfig {
  agentId: string;
  baseUrl?: string;
  authentication?: A2AAuthentication;
  timeout?: number;
  retries?: number;
}

export interface A2AAuthentication {
  type: 'apiKey' | 'bearer' | 'oauth2' | 'custom';
  credentials: any;
}

// Server Configuration

export interface A2AServerConfig {
  port?: number;
  host?: string;
  basePath?: string;
  authentication?: AuthenticationSchema;
  cors?: CorsConfig;
  rateLimit?: RateLimitConfig;
}

export interface CorsConfig {
  origin: string | string[] | boolean;
  methods?: string[];
  allowedHeaders?: string[];
  exposedHeaders?: string[];
  credentials?: boolean;
}

export interface RateLimitConfig {
  windowMs: number;
  max: number;
  message?: string;
}

// Event Types

export interface A2AEvent {
  type: A2AEventType;
  timestamp: Date;
  data: any;
}

export enum A2AEventType {
  TaskCreated = 'task.created',
  TaskUpdated = 'task.updated',
  TaskCompleted = 'task.completed',
  TaskFailed = 'task.failed',
  MessageReceived = 'message.received',
  ArtifactCreated = 'artifact.created',
  AgentConnected = 'agent.connected',
  AgentDisconnected = 'agent.disconnected'
}

// Discovery Types

export interface DiscoveryRequest {
  capabilities?: string[];
  version?: string;
}

export interface DiscoveryResponse {
  agents: AgentCard[];
}

// Stateless Interaction Support (v0.2)

export interface StatelessTaskRequest {
  task: Omit<Task, 'id' | 'status' | 'createdAt' | 'updatedAt'>;
  waitForCompletion?: boolean;
  timeout?: number;
}

export interface StatelessTaskResponse {
  taskId?: string;
  status: TaskStatus;
  output?: any;
  artifacts?: Artifact[];
  error?: TaskError;
}

// Additional types needed by ADK

export interface AgentRequest {
  taskId: string;
  input: any;
  context?: TaskContext;
  metadata?: Record<string, any>;
}

export interface AgentResponse {
  taskId: string;
  status: TaskStatus;
  output?: any;
  artifacts?: Artifact[];
  error?: TaskError;
  metadata?: Record<string, any>;
}

export interface TaskDefinition {
  id: string;
  name: string;
  description?: string;
  inputSchema?: any;
  outputSchema?: any;
  timeout?: number;
  retries?: number;
}

export interface TaskParameters {
  [key: string]: any;
}

export interface TaskResult {
  success: boolean;
  output?: any;
  error?: TaskError;
  artifacts?: Artifact[];
  metadata?: Record<string, any>;
}

export enum ArtifactType {
  Text = 'text',
  File = 'file', 
  Image = 'image',
  Audio = 'audio',
  Video = 'video',
  Code = 'code',
  Data = 'data',
  Model = 'model',
  Custom = 'custom'
}

export enum ChannelType {
  HTTP = 'http',
  WebSocket = 'websocket',
  RPC = 'rpc',
  Event = 'event',
  Stream = 'stream'
}

export interface A2AProtocolCapability {
  name: string;
  version: string;
  features: string[];
}

export const A2AProtocolVersion = '0.2';