/**
 * WebSocket Client
 * 
 * Client for connecting to the SymbioteIDE WebSocket server
 */

import { io, Socket } from 'socket.io-client';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import {
  WebSocketClientConfig,
  SocketNamespace,
  SocketEvent,
  SocketAuth,
  SocketClientEvents,
  SocketServerEvents,
  StreamResponse,
  AgentStatus,
  TaskProgress,
  CollabCursor,
  CollabSelection,
  CollabEdit
} from './types';

export class WebSocketClient extends EventEmitter {
  private socket?: Socket<SocketClientEvents, SocketServerEvents>;
  private config: WebSocketClientConfig;
  private namespace: SocketNamespace;
  private logger: Logger;
  private reconnectAttempts = 0;
  private isConnected = false;
  private activeStreams = new Map<string, any[]>();
  
  constructor(
    namespace: SocketNamespace = SocketNamespace.Default,
    config: WebSocketClientConfig = {}
  ) {
    super();
    
    this.namespace = namespace;
    this.config = {
      url: config.url || 'http://localhost:3001',
      path: config.path || '/socket.io',
      auth: config.auth,
      reconnection: config.reconnection !== false,
      reconnectionAttempts: config.reconnectionAttempts || 5,
      reconnectionDelay: config.reconnectionDelay || 1000,
      reconnectionDelayMax: config.reconnectionDelayMax || 5000,
      timeout: config.timeout || 20000,
      autoConnect: config.autoConnect !== false,
      transports: config.transports || ['websocket', 'polling'],
      ...config
    };
    
    this.logger = new Logger(`WebSocketClient-${namespace}`);
    
    if (this.config.autoConnect) {
      this.connect();
    }
  }
  
  /**
   * Connect to server
   */
  connect(): void {
    if (this.socket && this.socket.connected) {
      this.logger.warn('Already connected');
      return;
    }
    
    const url = `${this.config.url}${this.namespace}`;
    
    this.socket = io(url, {
      path: this.config.path,
      auth: this.config.auth,
      reconnection: this.config.reconnection,
      reconnectionAttempts: this.config.reconnectionAttempts,
      reconnectionDelay: this.config.reconnectionDelay,
      reconnectionDelayMax: this.config.reconnectionDelayMax,
      timeout: this.config.timeout,
      transports: this.config.transports
    }) as Socket<SocketClientEvents, SocketServerEvents>;
    
    this.setupEventHandlers();
    
    this.logger.info('Connecting to server', { url, namespace: this.namespace });
  }
  
  /**
   * Disconnect from server
   */
  disconnect(): void {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = undefined;
      this.isConnected = false;
      this.logger.info('Disconnected from server');
    }
  }
  
  /**
   * Set up event handlers
   */
  private setupEventHandlers(): void {
    if (!this.socket) return;
    
    // Connection events
    this.socket.on('connect', () => {
      this.isConnected = true;
      this.reconnectAttempts = 0;
      this.logger.info('Connected to server', {
        id: this.socket!.id,
        namespace: this.namespace
      });
      this.emit('connected', { id: this.socket!.id });
    });
    
    this.socket.on('disconnect', (reason) => {
      this.isConnected = false;
      this.logger.info('Disconnected from server', { reason });
      this.emit('disconnected', { reason });
    });
    
    this.socket.on('connect_error', (error) => {
      this.reconnectAttempts++;
      this.logger.error('Connection error', error);
      this.emit('error', error);
    });
    
    // Authentication events
    this.socket.on(SocketEvent.Authorized as any, (auth: any) => {
      this.logger.info('Authorized', { auth });
      this.emit('authorized', auth);
    });
    
    this.socket.on(SocketEvent.Unauthorized as any, (reason: any) => {
      this.logger.error('Unauthorized', { reason });
      this.emit('unauthorized', reason);
    });
    
    // Stream events
    this.socket.on(SocketEvent.StreamStart as any, (stream: any) => {
      this.activeStreams.set(stream.streamId, []);
      this.emit('stream-start', stream);
    });
    
    this.socket.on(SocketEvent.StreamData as any, (data: StreamResponse) => {
      const chunks = this.activeStreams.get(data.streamId);
      if (chunks) {
        chunks.push(data);
      }
      this.emit('stream-data', data);
    });
    
    this.socket.on(SocketEvent.StreamEnd as any, (stream: any) => {
      const chunks = this.activeStreams.get(stream.streamId);
      this.activeStreams.delete(stream.streamId);
      this.emit('stream-end', { ...stream, chunks });
    });
    
    this.socket.on(SocketEvent.StreamError as any, (error: any) => {
      this.activeStreams.delete(error.streamId);
      this.emit('stream-error', error);
    });
    
    // Namespace-specific handlers
    this.setupNamespaceHandlers();
  }
  
  /**
   * Set up namespace-specific handlers
   */
  private setupNamespaceHandlers(): void {
    if (!this.socket) return;
    
    switch (this.namespace) {
      case SocketNamespace.Agents:
        this.setupAgentHandlers();
        break;
      case SocketNamespace.Memory:
        this.setupMemoryHandlers();
        break;
      case SocketNamespace.Tasks:
        this.setupTaskHandlers();
        break;
      case SocketNamespace.Collaboration:
        this.setupCollaborationHandlers();
        break;
      case SocketNamespace.System:
        this.setupSystemHandlers();
        break;
    }
  }
  
  /**
   * Set up agent handlers
   */
  private setupAgentHandlers(): void {
    if (!this.socket) return;
    
    this.socket.on(SocketEvent.AgentMessage, (message) => {
      this.emit('agent-message', message);
    });
    
    this.socket.on(SocketEvent.AgentStatus, (status: AgentStatus) => {
      this.emit('agent-status', status);
    });
  }
  
  /**
   * Set up memory handlers
   */
  private setupMemoryHandlers(): void {
    if (!this.socket) return;
    
    this.socket.on(SocketEvent.MemoryUpdate, (memory) => {
      this.emit('memory-update', memory);
    });
    
    this.socket.on(SocketEvent.MemoryShare, (data) => {
      this.emit('memory-share', data);
    });
    
    this.socket.on(SocketEvent.MemorySync, (memories) => {
      this.emit('memory-sync', memories);
    });
  }
  
  /**
   * Set up task handlers
   */
  private setupTaskHandlers(): void {
    if (!this.socket) return;
    
    this.socket.on(SocketEvent.TaskUpdate, (task) => {
      this.emit('task-update', task);
    });
    
    this.socket.on(SocketEvent.TaskComplete, (result) => {
      this.emit('task-complete', result);
    });
    
    this.socket.on(SocketEvent.TaskError, (error) => {
      this.emit('task-error', error);
    });
    
    this.socket.on(SocketEvent.TaskProgress, (progress: TaskProgress) => {
      this.emit('task-progress', progress);
    });
  }
  
  /**
   * Set up collaboration handlers
   */
  private setupCollaborationHandlers(): void {
    if (!this.socket) return;
    
    this.socket.on(SocketEvent.CollabJoin, (data) => {
      this.emit('collab-join', data);
    });
    
    this.socket.on(SocketEvent.CollabLeave, (data) => {
      this.emit('collab-leave', data);
    });
    
    this.socket.on(SocketEvent.CollabCursor, (cursor: CollabCursor) => {
      this.emit('collab-cursor', cursor);
    });
    
    this.socket.on(SocketEvent.CollabSelection, (selection: CollabSelection) => {
      this.emit('collab-selection', selection);
    });
    
    this.socket.on(SocketEvent.CollabEdit, (edit: CollabEdit) => {
      this.emit('collab-edit', edit);
    });
  }
  
  /**
   * Set up system handlers
   */
  private setupSystemHandlers(): void {
    if (!this.socket) return;
    
    this.socket.on(SocketEvent.SystemStatus, (status) => {
      this.emit('system-status', status);
    });
    
    this.socket.on(SocketEvent.SystemMetrics, (metrics) => {
      this.emit('system-metrics', metrics);
    });
    
    this.socket.on(SocketEvent.SystemBroadcast, (message) => {
      this.emit('system-broadcast', message);
    });
  }
  
  // Emit methods
  
  /**
   * Join as agent
   */
  joinAsAgent(agentId: string, capabilities?: any[]): void {
    this.emit(SocketEvent.AgentJoin, { agentId, capabilities });
  }
  
  /**
   * Update agent status
   */
  updateAgentStatus(status: AgentStatus): void {
    this.emit(SocketEvent.AgentStatus, status);
  }
  
  /**
   * Create task
   */
  createTask(task: any): void {
    this.emit(SocketEvent.TaskCreate, task);
  }
  
  /**
   * Update task progress
   */
  updateTaskProgress(progress: TaskProgress): void {
    this.emit(SocketEvent.TaskProgress, progress);
  }
  
  /**
   * Search memories
   */
  async searchMemories(request: any): Promise<any[]> {
    return new Promise((resolve, reject) => {
      this.socket!.emit(SocketEvent.MemorySearch, request, (response: any) => {
        if (response.error) {
          reject(new Error(response.error));
        } else {
          resolve(response);
        }
      });
    });
  }
  
  /**
   * Join file collaboration
   */
  joinFileCollaboration(fileUri: string, userId: string): void {
    this.emit(SocketEvent.CollabJoin, { fileUri, userId });
  }
  
  /**
   * Update cursor position
   */
  updateCursor(cursor: CollabCursor): void {
    this.emit(SocketEvent.CollabCursor, cursor);
  }
  
  /**
   * Update selection
   */
  updateSelection(selection: CollabSelection): void {
    this.emit(SocketEvent.CollabSelection, selection);
  }
  
  /**
   * Send edit
   */
  sendEdit(edit: CollabEdit): void {
    this.emit(SocketEvent.CollabEdit, edit);
  }
  
  /**
   * Subscribe to metrics
   */
  subscribeToMetrics(): void {
    this.emit('subscribe:metrics');
  }
  
  /**
   * Unsubscribe from metrics
   */
  unsubscribeFromMetrics(): void {
    this.emit('unsubscribe:metrics');
  }
  
  /**
   * Emit event to server
   */
  emit(event: string, ...args: any[]): boolean {
    if (!this.socket || !this.socket.connected) {
      this.logger.warn('Not connected, queuing event', { event });
      return false;
    }
    
    this.socket.emit(event as any, ...args);
    return true;
  }
  
  /**
   * Check if connected
   */
  isConnectedToServer(): boolean {
    return this.isConnected && this.socket?.connected || false;
  }
  
  /**
   * Get socket ID
   */
  getSocketId(): string | undefined {
    return this.socket?.id;
  }
  
  /**
   * Wait for connection
   */
  async waitForConnection(timeout: number = 5000): Promise<void> {
    if (this.isConnectedToServer()) {
      return;
    }
    
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error('Connection timeout'));
      }, timeout);
      
      this.once('connected', () => {
        clearTimeout(timer);
        resolve();
      });
    });
  }
}