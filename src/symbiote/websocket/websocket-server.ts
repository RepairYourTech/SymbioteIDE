/**
 * WebSocket Server
 * 
 * Real-time communication server for SymbioteIDE
 */

import { Server as SocketServer, Socket } from 'socket.io';
import { createServer, Server as HttpServer } from 'http';
import { createAdapter } from '@socket.io/redis-adapter';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { RedisManager } from '../redis';
import {
  WebSocketServerConfig,
  SocketNamespace,
  SocketEvent,
  SocketAuth,
  SocketClient,
  SocketServerEvents,
  SocketClientEvents,
  SocketMiddleware,
  Room,
  RoomType,
  BroadcastOptions,
  SystemMetrics,
  AgentStatus,
  StreamResponse,
  SocketMessage,
  TypedSocket
} from './types';
import { RoomManager } from './room-manager';
import { StreamManager } from './stream-manager';
import { AuthManager } from './auth-manager';

export class WebSocketServer extends EventEmitter {
  private io: SocketServer<SocketClientEvents, SocketServerEvents>;
  private httpServer: HttpServer;
  private config: WebSocketServerConfig;
  private logger = new Logger('WebSocketServer');
  private clients = new Map<string, SocketClient>();
  private roomManager: RoomManager;
  private streamManager: StreamManager;
  private authManager: AuthManager;
  private redis?: RedisManager;
  private metricsInterval?: NodeJS.Timeout;
  private namespaces: Record<string, Namespace<SocketClientEvents, SocketServerEvents>> = {};
  
  constructor(config: WebSocketServerConfig = {}) {
    super();
    
    this.config = {
      port: config.port || 3001,
      path: config.path || '/socket.io',
      cors: config.cors || { origin: true, credentials: true },
      pingTimeout: config.pingTimeout || 20000,
      pingInterval: config.pingInterval || 10000,
      maxHttpBufferSize: config.maxHttpBufferSize || 1e6,
      transports: config.transports || ['websocket', 'polling'],
      allowUpgrades: config.allowUpgrades !== false,
      perMessageDeflate: config.perMessageDeflate !== false,
      httpCompression: config.httpCompression !== false,
      ...config
    };
    
    // Create HTTP server
    this.httpServer = createServer();
    
    // Create Socket.IO server
    this.io = new SocketServer(this.httpServer, {
      path: this.config.path,
      cors: this.config.cors,
      pingTimeout: this.config.pingTimeout,
      pingInterval: this.config.pingInterval,
      maxHttpBufferSize: this.config.maxHttpBufferSize,
      transports: this.config.transports,
      allowUpgrades: this.config.allowUpgrades,
      perMessageDeflate: this.config.perMessageDeflate,
      httpCompression: this.config.httpCompression
    });
    
    // Initialize managers
    this.roomManager = new RoomManager(this.io);
    this.streamManager = new StreamManager(this.io);
    this.authManager = new AuthManager();
    
    // Set up namespaces
    this.setupNamespaces();
    
    // Set up middleware
    this.setupMiddleware();
    
    // Set up event handlers
    this.setupConnectionHandlers();
  }
  
  /**
   * Initialize with Redis for clustering
   */
  async initializeRedis(redis: RedisManager): Promise<void> {
    this.redis = redis;
    
    // Create Redis adapter for Socket.IO
    const pubClient = redis['pubClient'];
    const subClient = redis['subClient'];
    
    if (pubClient && subClient) {
      this.io.adapter(createAdapter(pubClient, subClient));
      this.logger.info('Redis adapter initialized for Socket.IO clustering');
    }
  }
  
  /**
   * Start the server
   */
  async start(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.httpServer.listen(this.config.port, () => {
        this.logger.info(`WebSocket server listening on port ${this.config.port}`);
        
        // Start metrics collection
        this.startMetricsCollection();
        
        this.emit('started', { port: this.config.port });
        resolve();
      });
      
      this.httpServer.on('error', (error) => {
        this.logger.error('Server error', error);
        reject(error);
      });
    });
  }
  
  /**
   * Stop the server
   */
  async stop(): Promise<void> {
    // Stop metrics collection
    if (this.metricsInterval) {
      clearInterval(this.metricsInterval);
    }
    
    // Disconnect all clients
    await this.io.disconnectSockets();
    
    // Close server
    return new Promise((resolve) => {
      this.httpServer.close(() => {
        this.logger.info('WebSocket server stopped');
        this.emit('stopped');
        resolve();
      });
    });
  }
  
  /**
   * Set up namespaces
   */
  private setupNamespaces(): void {
    // Default namespace
    this.setupNamespace(SocketNamespace.Default);
    
    // Agent namespace
    this.setupNamespace(SocketNamespace.Agents);
    
    // Memory namespace
    this.setupNamespace(SocketNamespace.Memory);
    
    // Tasks namespace
    this.setupNamespace(SocketNamespace.Tasks);
    
    // Collaboration namespace
    this.setupNamespace(SocketNamespace.Collaboration);
    
    // System namespace
    this.setupNamespace(SocketNamespace.System);
  }
  
  /**
   * Set up a namespace
   */
  private setupNamespace(namespace: SocketNamespace): void {
    const nsp = this.io.of(namespace);
    
    // Namespace-specific middleware
    nsp.use(this.createAuthMiddleware());
    
    // Connection handler
    nsp.on('connection', (socket) => {
      this.handleConnection(socket, namespace);
    });
    
    this.logger.info(`Namespace initialized: ${namespace}`);
  }
  
  /**
   * Set up connection handlers
   */
  private setupConnectionHandlers(): void {
    // Setup namespace connection handlers
    for (const [name, namespace] of Object.entries(this.namespaces)) {
      namespace.on('connection', (socket) => {
        this.handleConnection(socket, name as SocketNamespace);
      });
    }
  }

  /**
   * Set up global middleware
   */
  private setupMiddleware(): void {
    // Authentication middleware
    this.io.use(this.createAuthMiddleware());
    
    // Rate limiting middleware
    this.io.use(this.createRateLimitMiddleware());
    
    // Logging middleware
    this.io.use(this.createLoggingMiddleware());
  }
  
  /**
   * Create authentication middleware
   */
  private createAuthMiddleware(): SocketMiddleware {
    return async (socket: Socket, next: (err?: Error) => void) => {
      try {
        const auth = socket.handshake.auth as SocketAuth;
        
        // Validate authentication
        const isValid = await this.authManager.validateAuth(auth);
        
        if (!isValid) {
          return next(new Error('Authentication failed'));
        }
        
        // Store auth in socket data
        socket.data.auth = auth;
        next();
        
      } catch (error: any) {
        next(error);
      }
    };
  }
  
  /**
   * Create rate limiting middleware
   */
  private createRateLimitMiddleware(): SocketMiddleware {
    const requestCounts = new Map<string, number>();
    
    return (socket: Socket, next: (err?: Error) => void) => {
      const clientId = socket.handshake.address;
      const count = requestCounts.get(clientId) || 0;
      
      if (count > 100) { // 100 connections per minute
        return next(new Error('Rate limit exceeded'));
      }
      
      requestCounts.set(clientId, count + 1);
      
      // Reset count after 1 minute
      setTimeout(() => {
        requestCounts.delete(clientId);
      }, 60000);
      
      next();
    };
  }
  
  /**
   * Create logging middleware
   */
  private createLoggingMiddleware(): SocketMiddleware {
    return (socket: Socket, next: (err?: Error) => void) => {
      this.logger.debug('New connection attempt', {
        id: socket.id,
        address: socket.handshake.address,
        auth: socket.handshake.auth
      });
      next();
    };
  }
  
  /**
   * Handle new connection
   */
  private handleConnection(socket: Socket, namespace: SocketNamespace): void {
    const client: SocketClient = {
      id: socket.id,
      auth: socket.data.auth,
      namespace,
      rooms: new Set([socket.id]), // Default room
      connectedAt: new Date(),
      lastActivity: new Date()
    };
    
    this.clients.set(socket.id, client);
    
    this.logger.info('Client connected', {
      id: socket.id,
      namespace,
      auth: client.auth
    });
    
    // Send authorization confirmation
    socket.emit(SocketEvent.Authorized, client.auth);
    
    // Set up event handlers based on namespace
    this.setupSocketHandlers(socket, namespace);
    
    // Handle disconnection
    socket.on('disconnect', () => {
      this.handleDisconnection(socket);
    });
    
    // Emit connection event
    this.emit('client-connected', { client, namespace });
  }
  
  /**
   * Set up socket event handlers
   */
  private setupSocketHandlers(socket: Socket, namespace: SocketNamespace): void {
    switch (namespace) {
      case SocketNamespace.Agents:
        this.setupAgentHandlers(socket);
        break;
      case SocketNamespace.Memory:
        this.setupMemoryHandlers(socket);
        break;
      case SocketNamespace.Tasks:
        this.setupTaskHandlers(socket);
        break;
      case SocketNamespace.Collaboration:
        this.setupCollaborationHandlers(socket);
        break;
      case SocketNamespace.System:
        this.setupSystemHandlers(socket);
        break;
      default:
        this.setupDefaultHandlers(socket);
    }
  }
  
  /**
   * Set up agent namespace handlers
   */
  private setupAgentHandlers(socket: Socket): void {
    const typedSocket = socket as TypedSocket;
    // Agent join
    typedSocket.on(SocketEvent.AgentJoin, async (data) => {
      const client = this.clients.get(socket.id);
      if (!client) return;
      
      client.metadata = {
        ...client.metadata,
        agentId: data.agentId,
        capabilities: data.capabilities
      };
      
      // Join agent room
      const roomId = `agent:${data.agentId}`;
      await this.roomManager.joinRoom(socket.id, roomId);
      
      // Broadcast agent status
      this.broadcastToNamespace(SocketNamespace.Agents, SocketEvent.AgentStatus, {
        agentId: data.agentId,
        status: 'idle'
      });
      
      this.emit('agent-joined', { socketId: socket.id, agentId: data.agentId });
    });
    
    // Agent status update
    typedSocket.on(SocketEvent.AgentStatus, (status) => {
      this.broadcastToNamespace(
        SocketNamespace.Agents,
        SocketEvent.AgentStatus,
        status,
        { exclude: [socket.id] }
      );
    });
  }
  
  /**
   * Set up memory namespace handlers
   */
  private setupMemoryHandlers(socket: Socket): void {
    const typedSocket = socket as TypedSocket;
    // Memory search
    typedSocket.on(SocketEvent.MemorySearch, async (request, callback) => {
      try {
        // This would call the memory manager
        const results = await this.searchMemories(request);
        callback(results);
      } catch (error: any) {
        callback({ error: error.message });
      }
    });
    
    // Memory share
    typedSocket.on(SocketEvent.MemoryShare, async (data) => {
      const client = this.clients.get(socket.id);
      if (!client) return;
      
      // Broadcast to team
      this.broadcastToRoom(
        `team:${client.auth?.teamId || 'default'}`,
        SocketEvent.MemoryShare,
        {
          memory: data.memory,
          sharedBy: client.auth?.userId || 'unknown'
        }
      );
    });
  }
  
  /**
   * Set up task namespace handlers
   */
  private setupTaskHandlers(socket: Socket): void {
    const typedSocket = socket as TypedSocket;
    // Task progress updates
    typedSocket.on(SocketEvent.TaskProgress, (progress) => {
      // Broadcast to task room
      this.broadcastToRoom(
        `task:${progress.taskId}`,
        SocketEvent.TaskProgress,
        progress
      );
      
      // Store progress
      this.emit('task-progress', progress);
    });
    
    // Task creation
    typedSocket.on(SocketEvent.TaskCreate, async (task) => {
      // Create task room
      await this.roomManager.createRoom({
        id: `task:${task.id}`,
        type: RoomType.Task,
        name: task.prompt.substring(0, 50),
        members: [socket.id],
        created: new Date()
      });
      
      // Join creator to room
      await this.roomManager.joinRoom(socket.id, `task:${task.id}`);
      
      this.emit('task-created', task);
    });
  }
  
  /**
   * Set up collaboration namespace handlers
   */
  private setupCollaborationHandlers(socket: Socket): void {
    const typedSocket = socket as TypedSocket;
    // Join file collaboration
    typedSocket.on(SocketEvent.CollabJoin, async (data) => {
      const roomId = `file:${data.fileUri}`;
      await this.roomManager.joinRoom(socket.id, roomId);
      
      // Notify others
      socket.to(roomId).emit(SocketEvent.CollabJoin, {
        userId: data.userId,
        fileUri: data.fileUri
      });
    });
    
    // Cursor updates
    socket.on(SocketEvent.CollabCursor, (cursor) => {
      const roomId = `file:${cursor.fileUri}`;
      socket.to(roomId).emit(SocketEvent.CollabCursor, cursor);
    });
    
    // Selection updates
    socket.on(SocketEvent.CollabSelection, (selection) => {
      const roomId = `file:${selection.fileUri}`;
      socket.to(roomId).emit(SocketEvent.CollabSelection, selection);
    });
    
    // Edit events
    socket.on(SocketEvent.CollabEdit, (edit) => {
      const roomId = `file:${edit.fileUri}`;
      socket.to(roomId).emit(SocketEvent.CollabEdit, edit);
      
      // Store edit history
      this.emit('collab-edit', edit);
    });
  }
  
  /**
   * Set up system namespace handlers
   */
  private setupSystemHandlers(socket: Socket): void {
    // System status request
    socket.on(SocketEvent.SystemStatus, async () => {
      const status = await this.getSystemStatus();
      socket.emit(SocketEvent.SystemStatus, status);
    });
    
    // Subscribe to metrics
    socket.on('subscribe:metrics', () => {
      socket.join('metrics');
    });
    
    // Unsubscribe from metrics
    socket.on('unsubscribe:metrics', () => {
      socket.leave('metrics');
    });
  }
  
  /**
   * Set up default namespace handlers
   */
  private setupDefaultHandlers(socket: Socket): void {
    // Echo test
    socket.on('echo', (data, callback) => {
      callback(data);
    });
    
    // Ping test
    socket.on('ping', (callback) => {
      callback({ timestamp: Date.now() });
    });
  }
  
  /**
   * Handle client disconnection
   */
  private handleDisconnection(socket: Socket): void {
    const client = this.clients.get(socket.id);
    if (!client) return;
    
    this.logger.info('Client disconnected', {
      id: socket.id,
      namespace: client.namespace,
      duration: Date.now() - client.connectedAt.getTime()
    });
    
    // Clean up
    this.clients.delete(socket.id);
    
    // Leave all rooms
    socket.rooms.forEach(room => {
      if (room !== socket.id) {
        socket.leave(room);
      }
    });
    
    // Emit disconnection event
    this.emit('client-disconnected', { client });
  }
  
  /**
   * Create a stream
   */
  createStream(clientId: string, metadata?: any): string {
    const streamId = this.streamManager.createStream(metadata);
    
    // Send stream start event
    this.io.to(clientId).emit(SocketEvent.StreamStart, {
      streamId,
      metadata
    });
    
    return streamId;
  }
  
  /**
   * Write to stream
   */
  writeToStream(streamId: string, data: StreamResponse): void {
    this.streamManager.writeToStream(streamId, data);
  }
  
  /**
   * End stream
   */
  endStream(streamId: string, summary?: any): void {
    this.streamManager.endStream(streamId, summary);
  }
  
  /**
   * Broadcast to namespace
   */
  broadcastToNamespace(
    namespace: SocketNamespace,
    event: string,
    data: any,
    options?: BroadcastOptions
  ): void {
    let emitter = this.io.of(namespace);
    
    if (options?.room) {
      emitter = emitter.to(options.room);
    }
    
    if (options?.exclude) {
      options.exclude.forEach(id => {
        emitter = emitter.except(id);
      });
    }
    
    if (options?.volatile) {
      emitter = emitter.volatile;
    }
    
    if (options?.compress) {
      emitter = emitter.compress(true);
    }
    
    emitter.emit(event, data);
  }
  
  /**
   * Broadcast to room
   */
  broadcastToRoom(room: string, event: string, data: any): void {
    this.io.to(room).emit(event, data);
  }
  
  /**
   * Send to specific client
   */
  sendToClient(clientId: string, event: string, data: any): void {
    this.io.to(clientId).emit(event, data);
  }
  
  /**
   * Get connected clients
   */
  getClients(namespace?: SocketNamespace): SocketClient[] {
    const clients = Array.from(this.clients.values());
    
    if (namespace) {
      return clients.filter(c => c.namespace === namespace);
    }
    
    return clients;
  }
  
  /**
   * Get system status
   */
  private async getSystemStatus(): Promise<any> {
    return {
      server: {
        uptime: process.uptime(),
        memory: process.memoryUsage(),
        cpu: process.cpuUsage()
      },
      websocket: {
        connections: this.clients.size,
        namespaces: this.getNamespaceStats()
      },
      timestamp: new Date()
    };
  }
  
  /**
   * Get namespace statistics
   */
  private getNamespaceStats(): Record<string, number> {
    const stats: Record<string, number> = {};
    
    Object.values(SocketNamespace).forEach(namespace => {
      stats[namespace] = this.getClients(namespace).length;
    });
    
    return stats;
  }
  
  /**
   * Start metrics collection
   */
  private startMetricsCollection(): void {
    this.metricsInterval = setInterval(async () => {
      const metrics = await this.collectMetrics();
      
      // Broadcast to metrics subscribers
      this.io.of(SocketNamespace.System)
        .to('metrics')
        .emit(SocketEvent.SystemMetrics, metrics);
        
      // Emit for internal use
      this.emit('metrics', metrics);
    }, 5000); // Every 5 seconds
  }
  
  /**
   * Collect system metrics
   */
  private async collectMetrics(): Promise<SystemMetrics> {
    const memUsage = process.memoryUsage();
    const cpuUsage = process.cpuUsage();
    
    return {
      timestamp: new Date(),
      memory: {
        used: memUsage.heapUsed,
        total: memUsage.heapTotal,
        percentage: (memUsage.heapUsed / memUsage.heapTotal) * 100
      },
      cpu: {
        usage: cpuUsage.user + cpuUsage.system,
        cores: require('os').cpus().length
      },
      websocket: {
        connections: this.clients.size,
        namespaces: this.getNamespaceStats(),
        messagesPerSecond: 0 // Would need to track this
      },
      cache: {
        hits: 0, // Would get from cache manager
        misses: 0,
        size: 0
      },
      ai: {
        activeModels: 0, // Would get from orchestration
        requestsPerMinute: 0,
        averageLatency: 0,
        tokenUsage: {
          input: 0,
          output: 0,
          total: 0
        }
      }
    };
  }
  
  /**
   * Mock memory search (would connect to real memory manager)
   */
  private async searchMemories(request: any): Promise<any[]> {
    // This would call the actual memory manager
    return [];
  }
}