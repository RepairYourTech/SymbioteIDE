/**
 * A2A Server
 * 
 * Server implementation for A2A protocol
 */

import { EventEmitter } from 'events';
import express, { Application, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import rateLimit from 'express-rate-limit';
import { 
  A2AServerConfig,
  AgentCard,
  Task,
  Message,
  Artifact,
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcError,
  A2AError,
  A2AErrorCode,
  AuthenticationSchema
} from './types';
import { Logger } from '../utils/logger';

export class A2AServer extends EventEmitter {
  private logger = new Logger('A2AServer');
  private app: Application;
  private server?: any;
  private agents = new Map<string, AgentCard>();
  private endpoints = new Map<string, (req: any) => Promise<any>>();
  
  constructor(private config: A2AServerConfig = {}) {
    super();
    this.app = express();
    this.setupMiddleware();
    this.setupRoutes();
  }

  /**
   * Setup middleware
   */
  private setupMiddleware(): void {
    // JSON parsing
    this.app.use(express.json({ limit: '10mb' }));
    
    // CORS
    if (this.config.cors) {
      this.app.use(cors(this.config.cors));
    } else {
      this.app.use(cors());
    }
    
    // Rate limiting
    if (this.config.rateLimit) {
      const limiter = rateLimit(this.config.rateLimit);
      this.app.use(limiter);
    }
    
    // Authentication middleware
    if (this.config.authentication) {
      this.app.use(this.authenticationMiddleware.bind(this));
    }
    
    // Request logging
    this.app.use((req, res, next) => {
      this.logger.debug(`${req.method} ${req.path}`);
      next();
    });
  }

  /**
   * Setup routes
   */
  private setupRoutes(): void {
    const basePath = this.config.basePath || '';
    
    // Well-known discovery endpoints
    this.app.get('/.well-known/agents', this.handleDiscovery.bind(this));
    this.app.get('/.well-known/agent.json', this.handleAgentCard.bind(this));
    
    // JSON-RPC endpoint
    this.app.post(`${basePath}/rpc`, this.handleJsonRpc.bind(this));
    
    // Task streaming endpoint
    this.app.get(`${basePath}/tasks/:taskId/stream`, this.handleTaskStream.bind(this));
    
    // Error handling
    this.app.use(this.errorHandler.bind(this));
  }

  /**
   * Register an agent
   */
  async registerAgent(agentId: string, card: AgentCard): Promise<void> {
    this.agents.set(agentId, card);
    this.logger.info(`Registered agent: ${agentId}`);
    super.emit('agent:registered', { agentId, card });
  }

  /**
   * Add custom endpoint
   */
  addEndpoint(path: string, handler: (req: any) => Promise<any>): void {
    this.endpoints.set(path, handler);
    
    // Register with Express
    this.app.all(path, async (req, res, next) => {
      try {
        const result = await handler(req.body);
        res.json(result);
      } catch (error) {
        next(error);
      }
    });
  }

  /**
   * Start server
   */
  async start(): Promise<void> {
    const port = this.config.port || 3000;
    const host = this.config.host || '0.0.0.0';
    
    return new Promise((resolve) => {
      this.server = this.app.listen(port, host, () => {
        this.logger.info(`A2A server listening on ${host}:${port}`);
        resolve();
      });
    });
  }

  /**
   * Stop server
   */
  async stop(): Promise<void> {
    if (this.server) {
      return new Promise((resolve) => {
        this.server.close(() => {
          this.logger.info('A2A server stopped');
          resolve();
        });
      });
    }
  }

  /**
   * Handle discovery requests
   */
  private async handleDiscovery(req: Request, res: Response): Promise<void> {
    const agents = Array.from(this.agents.values());
    
    // Filter by capabilities if requested
    const capabilities = req.query.capabilities as string[];
    if (capabilities) {
      const filtered = agents.filter(agent =>
        capabilities.every(cap => agent.capabilities.includes(cap))
      );
      res.json({ agents: filtered });
    } else {
      res.json({ agents });
    }
  }

  /**
   * Handle agent card requests
   */
  private async handleAgentCard(req: Request, res: Response): Promise<void> {
    // Return first agent card or specific one if multiple
    const agentId = req.query.agentId as string;
    
    if (agentId) {
      const card = this.agents.get(agentId);
      if (card) {
        res.json(card);
      } else {
        res.status(404).json({ error: 'Agent not found' });
      }
    } else {
      // Return first agent
      const firstCard = this.agents.values().next().value;
      if (firstCard) {
        res.json(firstCard);
      } else {
        res.status(404).json({ error: 'No agents registered' });
      }
    }
  }

  /**
   * Handle JSON-RPC requests
   */
  private async handleJsonRpc(req: Request, res: Response): Promise<void> {
    const request = req.body as JsonRpcRequest;
    
    // Validate request
    if (request.jsonrpc !== '2.0') {
      res.json(this.createErrorResponse(
        request.id || null,
        A2AErrorCode.InvalidRequest,
        'Invalid JSON-RPC version'
      ));
      return;
    }

    try {
      // Route to appropriate handler
      const result = await this.routeJsonRpcMethod(request.method, request.params);
      
      const response: JsonRpcResponse = {
        jsonrpc: '2.0',
        result,
        id: request.id
      };
      
      res.json(response);
    } catch (error: any) {
      const errorResponse = this.createErrorResponse(
        request.id,
        error.code || A2AErrorCode.InternalError,
        error.message,
        error.data
      );
      res.json(errorResponse);
    }
  }

  /**
   * Route JSON-RPC methods
   */
  private async routeJsonRpcMethod(method: string, params: any): Promise<any> {
    switch (method) {
      case 'task.create':
        return this.handleTaskCreate(params);
      
      case 'task.get':
        return this.handleTaskGet(params);
      
      case 'task.cancel':
        return this.handleTaskCancel(params);
      
      case 'message.send':
        return this.handleMessageSend(params);
      
      case 'artifact.get':
        return this.handleArtifactGet(params);
      
      case 'artifact.list':
        return this.handleArtifactList(params);
      
      default:
        throw new A2AError('METHOD_NOT_FOUND', `Method ${method} not found`);
    }
  }

  /**
   * Handle task creation
   */
  private async handleTaskCreate(params: any): Promise<Task> {
    const task = await this.emitAndWait('task:create', params.task);
    return task;
  }

  /**
   * Handle task retrieval
   */
  private async handleTaskGet(params: any): Promise<Task> {
    const task = await this.emitAndWait('task:get', params.taskId);
    return task;
  }

  /**
   * Handle task cancellation
   */
  private async handleTaskCancel(params: any): Promise<void> {
    await this.emitAndWait('task:cancel', params.taskId);
  }

  /**
   * Handle message sending
   */
  private async handleMessageSend(params: any): Promise<void> {
    await this.emitAndWait('message:send', params.message.taskId, params.message);
  }

  /**
   * Handle artifact retrieval
   */
  private async handleArtifactGet(params: any): Promise<Artifact> {
    const artifact = await this.emitAndWait('artifact:get', params.taskId, params.artifactId);
    return artifact;
  }

  /**
   * Handle artifact listing
   */
  private async handleArtifactList(params: any): Promise<{ artifacts: Artifact[] }> {
    const artifacts = await this.emitAndWait('artifact:list', params.taskId);
    return { artifacts };
  }

  /**
   * Handle task streaming
   */
  private async handleTaskStream(req: Request, res: Response): Promise<void> {
    const { taskId } = req.params;
    
    // Set up SSE
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive'
    });
    
    // Send updates
    const sendUpdate = (data: any) => {
      res.write(`data: ${JSON.stringify(data)}\n\n`);
    };
    
    // Listen for task updates
    const updateHandler = (update: any) => {
      if (update.taskId === taskId) {
        sendUpdate(update);
      }
    };
    
    this.on('task:update', updateHandler);
    
    // Clean up on disconnect
    req.on('close', () => {
      this.off('task:update', updateHandler);
    });
    
    // Send initial ping
    sendUpdate({ type: 'connected', taskId });
  }

  /**
   * Authentication middleware
   */
  private async authenticationMiddleware(
    req: Request, 
    res: Response, 
    next: NextFunction
  ): Promise<void> {
    const auth = this.config.authentication!;
    
    try {
      switch (auth.type) {
        case 'none':
          next();
          break;
        
        case 'apiKey':
          const apiKey = req.headers['x-api-key'] as string;
          if (!apiKey) {
            throw new A2AError('AUTHENTICATION_REQUIRED', 'API key required');
          }
          // Validate API key (implementation specific)
          this.emitAndWait('auth:validate:apiKey', apiKey);
          next();
          break;
        
        case 'bearer':
          const authHeader = req.headers.authorization;
          if (!authHeader?.startsWith('Bearer ')) {
            throw new A2AError('AUTHENTICATION_REQUIRED', 'Bearer token required');
          }
          const token = authHeader.substring(7);
          // Validate token (implementation specific)
          this.emitAndWait('auth:validate:bearer', token);
          next();
          break;
        
        case 'oauth2':
          // OAuth2 validation (implementation specific)
          this.emitAndWait('auth:validate:oauth2', req);
          next();
          break;
        
        default:
          next();
      }
    } catch (error: any) {
      res.status(401).json({
        error: {
          code: 'AUTHENTICATION_FAILED',
          message: error.message
        }
      });
    }
  }

  /**
   * Error handler middleware
   */
  private errorHandler(
    err: any, 
    req: Request, 
    res: Response, 
    next: NextFunction
  ): void {
    this.logger.error('Request error', err);
    
    const status = err.status || 500;
    const message = err.message || 'Internal server error';
    
    res.status(status).json({
      error: {
        code: err.code || 'INTERNAL_ERROR',
        message,
        data: err.data
      }
    });
  }

  /**
   * Create error response
   */
  private createErrorResponse(
    id: string | number | null,
    code: number,
    message: string,
    data?: any
  ): JsonRpcResponse {
    return {
      jsonrpc: '2.0',
      error: {
        code,
        message,
        data
      },
      id: id!
    };
  }

  /**
   * Emit and wait for response
   */
  private async emitAndWait(event: string, ...args: any[]): Promise<any> {
    return new Promise((resolve, reject) => {
      const listeners = this.listeners(event);
      if (listeners.length === 0) {
        reject(new A2AError('NO_HANDLER', `No handler for event: ${event}`));
        return;
      }
      
      // Call first listener and wait for response
      const handler = listeners[0] as Function;
      Promise.resolve(handler(...args))
        .then(resolve)
        .catch(reject);
    });
  }
}