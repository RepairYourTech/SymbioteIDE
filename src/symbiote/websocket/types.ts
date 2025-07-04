/**
 * WebSocket Types and Interfaces
 * 
 * Real-time communication types for SymbioteIDE
 */

import { Socket } from 'socket.io';
import { 
  AITask, 
  TaskResult, 
  Capability
} from '../orchestration/interfaces';
import { 
  Memory, 
  MemorySearchResult, 
  Learning 
} from '../memory/mem0/types';
import { 
  CodeContext, 
  SemanticSearchResult
} from '../search/qdrant/types';

/**
 * Task Progress interface
 */
export interface TaskProgress {
  taskId: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  progress: number; // 0-100
  message?: string;
  details?: any;
}

/**
 * Model capability type (alias for Capability)
 */
export type ModelCapability = Capability;

/**
 * WebSocket namespaces
 */
export enum SocketNamespace {
  Default = '/',
  Agents = '/agents',
  Memory = '/memory',
  Tasks = '/tasks',
  Collaboration = '/collaboration',
  System = '/system'
}

/**
 * Socket event types
 */
export enum SocketEvent {
  // Connection events
  Connect = 'connect',
  Disconnect = 'disconnect',
  Error = 'error',
  
  // Authentication
  Authenticate = 'authenticate',
  Authorized = 'authorized',
  Unauthorized = 'unauthorized',
  
  // Agent events
  AgentJoin = 'agent:join',
  AgentLeave = 'agent:leave',
  AgentStatus = 'agent:status',
  AgentMessage = 'agent:message',
  
  // Task events
  TaskCreate = 'task:create',
  TaskUpdate = 'task:update',
  TaskComplete = 'task:complete',
  TaskError = 'task:error',
  TaskProgress = 'task:progress',
  
  // Memory events
  MemoryStore = 'memory:store',
  MemoryUpdate = 'memory:update',
  MemorySearch = 'memory:search',
  MemoryShare = 'memory:share',
  MemorySync = 'memory:sync',
  
  // Code events
  CodeIndex = 'code:index',
  CodeSearch = 'code:search',
  CodeUpdate = 'code:update',
  CodeAnalysis = 'code:analysis',
  
  // Collaboration events
  CollabJoin = 'collab:join',
  CollabLeave = 'collab:leave',
  CollabCursor = 'collab:cursor',
  CollabSelection = 'collab:selection',
  CollabEdit = 'collab:edit',
  
  // Stream events
  StreamStart = 'stream:start',
  StreamData = 'stream:data',
  StreamEnd = 'stream:end',
  StreamError = 'stream:error',
  
  // System events
  SystemStatus = 'system:status',
  SystemMetrics = 'system:metrics',
  SystemBroadcast = 'system:broadcast'
}

/**
 * Client authentication
 */
export interface SocketAuth {
  token?: string;
  apiKey?: string;
  userId?: string;
  agentId?: string;
  projectId?: string;
  permissions?: string[];
}

/**
 * Socket client info
 */
export interface SocketClient {
  id: string;
  auth?: SocketAuth;
  namespace: SocketNamespace;
  rooms: Set<string>;
  metadata?: Record<string, any>;
  connectedAt: Date;
  lastActivity: Date;
}

/**
 * Agent status
 */
export interface AgentStatus {
  agentId: string;
  status: 'idle' | 'working' | 'error' | 'offline';
  currentTask?: string;
  capabilities?: ModelCapability[];
  metadata?: {
    model?: string;
    temperature?: number;
    lastError?: string;
  };
}

// Note: TaskProgress is already defined above

/**
 * Streaming response
 */
export interface StreamResponse {
  streamId: string;
  type: 'text' | 'code' | 'data' | 'error';
  data: any;
  metadata?: {
    model?: string;
    tokens?: number;
    finished?: boolean;
  };
}

/**
 * Collaboration cursor
 */
export interface CollabCursor {
  userId: string;
  fileUri: string;
  position: {
    line: number;
    character: number;
  };
  color?: string;
  label?: string;
}

/**
 * Collaboration selection
 */
export interface CollabSelection {
  userId: string;
  fileUri: string;
  selections: Array<{
    start: { line: number; character: number };
    end: { line: number; character: number };
  }>;
  color?: string;
}

/**
 * Collaboration edit
 */
export interface CollabEdit {
  userId: string;
  fileUri: string;
  edits: Array<{
    range: {
      start: { line: number; character: number };
      end: { line: number; character: number };
    };
    text: string;
  }>;
  timestamp: Date;
}

/**
 * System metrics
 */
export interface SystemMetrics {
  timestamp: Date;
  memory: {
    used: number;
    total: number;
    percentage: number;
  };
  cpu: {
    usage: number;
    cores: number;
  };
  websocket: {
    connections: number;
    namespaces: Record<string, number>;
    messagesPerSecond: number;
  };
  cache: {
    hits: number;
    misses: number;
    size: number;
  };
  ai: {
    activeModels: number;
    requestsPerMinute: number;
    averageLatency: number;
    tokenUsage: {
      input: number;
      output: number;
      total: number;
    };
  };
}

/**
 * Room types
 */
export enum RoomType {
  Project = 'project',
  Team = 'team',
  Agent = 'agent',
  File = 'file',
  Task = 'task',
  Private = 'private'
}

/**
 * Room info
 */
export interface Room {
  id: string;
  type: RoomType;
  name: string;
  members: string[];
  created: Date;
  metadata?: Record<string, any>;
}

/**
 * Message types
 */
export interface SocketMessage<T = any> {
  id: string;
  event: SocketEvent | string;
  data: T;
  from?: string;
  to?: string | string[];
  room?: string;
  timestamp: Date;
  metadata?: Record<string, any>;
}

/**
 * Broadcast options
 */
export interface BroadcastOptions {
  namespace?: SocketNamespace;
  room?: string;
  exclude?: string[];
  volatile?: boolean;
  compress?: boolean;
}

/**
 * Socket server events
 */
export interface SocketServerEvents {
  // Client events
  [SocketEvent.Authenticate]: (auth: SocketAuth, callback: (response: any) => void) => void;
  [SocketEvent.AgentJoin]: (data: { agentId: string; capabilities?: ModelCapability[] }) => void;
  [SocketEvent.AgentStatus]: (status: AgentStatus) => void;
  [SocketEvent.TaskCreate]: (task: AITask) => void;
  [SocketEvent.TaskProgress]: (progress: TaskProgress) => void;
  [SocketEvent.MemorySearch]: (request: any, callback: (results: MemorySearchResult[]) => void) => void;
  [SocketEvent.CodeSearch]: (query: string, callback: (results: SemanticSearchResult[]) => void) => void;
  [SocketEvent.CollabJoin]: (data: { fileUri: string; userId: string }) => void;
  [SocketEvent.CollabCursor]: (cursor: CollabCursor) => void;
  [SocketEvent.CollabSelection]: (selection: CollabSelection) => void;
  [SocketEvent.CollabEdit]: (edit: CollabEdit) => void;
}

/**
 * Socket client events
 */
export interface SocketClientEvents {
  // Server events
  [SocketEvent.Authorized]: (auth: SocketAuth) => void;
  [SocketEvent.Unauthorized]: (reason: string) => void;
  [SocketEvent.AgentMessage]: (message: SocketMessage) => void;
  [SocketEvent.TaskUpdate]: (task: Partial<AITask>) => void;
  [SocketEvent.TaskComplete]: (result: TaskResult) => void;
  [SocketEvent.TaskError]: (error: { taskId: string; error: string }) => void;
  [SocketEvent.TaskProgress]: (progress: TaskProgress) => void;
  [SocketEvent.MemoryUpdate]: (memory: Memory) => void;
  [SocketEvent.MemoryShare]: (data: { memory: Memory; sharedBy: string }) => void;
  [SocketEvent.MemorySync]: (memories: Memory[]) => void;
  [SocketEvent.StreamStart]: (stream: { streamId: string; metadata?: any }) => void;
  [SocketEvent.StreamData]: (data: StreamResponse) => void;
  [SocketEvent.StreamEnd]: (stream: { streamId: string; summary?: any }) => void;
  [SocketEvent.SystemStatus]: (status: any) => void;
  [SocketEvent.SystemMetrics]: (metrics: SystemMetrics) => void;
  [SocketEvent.SystemBroadcast]: (message: SocketMessage) => void;
}

/**
 * WebSocket server configuration
 */
export interface WebSocketServerConfig {
  port?: number;
  path?: string;
  cors?: {
    origin?: string | string[] | boolean;
    credentials?: boolean;
  };
  pingTimeout?: number;
  pingInterval?: number;
  maxHttpBufferSize?: number;
  transports?: ('polling' | 'websocket')[];
  allowUpgrades?: boolean;
  perMessageDeflate?: boolean;
  httpCompression?: boolean;
  redis?: {
    host?: string;
    port?: number;
    password?: string;
  };
}

/**
 * WebSocket client configuration
 */
export interface WebSocketClientConfig {
  url?: string;
  path?: string;
  auth?: SocketAuth;
  reconnection?: boolean;
  reconnectionAttempts?: number;
  reconnectionDelay?: number;
  reconnectionDelayMax?: number;
  timeout?: number;
  autoConnect?: boolean;
  transports?: ('polling' | 'websocket')[];
}

/**
 * Middleware function type
 */
export type SocketMiddleware = (
  socket: Socket,
  next: (err?: Error) => void
) => void;

/**
 * Event handler function type
 */
export type SocketEventHandler<T = any> = (
  data: T,
  socket: Socket,
  callback?: (response: any) => void
) => void | Promise<void>;

/**
 * Typed socket interface
 */
export interface TypedSocket {
  id: string;
  connected: boolean;
  handshake: any;
  rooms: Set<string>;
  data: any;
  on<T extends keyof SocketServerEvents>(event: T, handler: SocketServerEvents[T]): this;
  emit<T extends keyof SocketClientEvents>(event: T, ...args: Parameters<SocketClientEvents[T]>): boolean;
  join(room: string | string[]): Promise<void>;
  leave(room: string): Promise<void>;
  disconnect(close?: boolean): this;
}

/**
 * Room manager interface
 */
export interface IRoomManager {
  createRoom(room: Room): Promise<void>;
  deleteRoom(roomId: string): Promise<void>;
  joinRoom(socketId: string, roomId: string): Promise<void>;
  leaveRoom(socketId: string, roomId: string): Promise<void>;
  getRoomMembers(roomId: string): Promise<string[]>;
  getUserRooms(socketId: string): Promise<Room[]>;
  broadcastToRoom(roomId: string, event: string, data: any, exclude?: string[]): Promise<void>;
}

/**
 * Stream manager interface
 */
export interface IStreamManager {
  createStream(streamId: string, metadata?: any): void;
  writeToStream(streamId: string, data: StreamResponse): void;
  endStream(streamId: string, summary?: any): void;
  errorStream(streamId: string, error: Error): void;
  getActiveStreams(): string[];
}