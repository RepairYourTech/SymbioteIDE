/**
 * Express REST API Server for Orchestration Engine
 */

import express, { Express, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { createServer, Server as HttpServer } from 'http';
import { Server as SocketIOServer } from 'socket.io';
import {
  APIServerOptions,
  ExecuteTaskRequest,
  ExecuteTaskResponse,
  GetTaskResultRequest,
  GetTaskResultResponse,
  ExecuteBatchRequest,
  ExecuteBatchResponse,
  GetBatchResultsRequest,
  GetBatchResultsResponse,
  GetModelsRequest,
  GetModelsResponse,
  GetMetricsRequest,
  GetMetricsResponse,
  GetQueueStatusRequest,
  GetQueueStatusResponse,
  GetCostEstimateRequest,
  GetCostEstimateResponse,
  UpdateConfigurationRequest,
  UpdateConfigurationResponse,
  GetConfigurationRequest,
  GetConfigurationResponse,
  HealthCheckResponse,
  APIError,
  StreamTaskRequest,
  StreamTaskResponse
} from '../types/api-interfaces';
import { OrchestrationEngine } from '../../orchestration-engine';
import { authMiddleware } from '../middleware/auth';
import { rateLimitMiddleware } from '../middleware/rate-limit';
import { validationMiddleware } from '../middleware/validation';
import { errorHandler } from '../middleware/error-handler';
import { RequestPriority } from '../../queue';

export class ExpressAPIServer {
  private app: Express;
  private server: HttpServer;
  private io?: SocketIOServer;
  private engine: OrchestrationEngine;
  private options: APIServerOptions;
  private activeStreams: Map<string, any> = new Map();

  constructor(engine: OrchestrationEngine, options: APIServerOptions = {}) {
    this.engine = engine;
    this.options = {
      port: options.port || 3000,
      host: options.host || '0.0.0.0',
      cors: {
        enabled: true,
        origins: ['*'],
        ...options.cors
      },
      auth: {
        enabled: false,
        type: 'apiKey',
        ...options.auth
      },
      rateLimit: {
        enabled: true,
        windowMs: 60000,
        maxRequests: 100,
        ...options.rateLimit
      },
      websocket: {
        enabled: true,
        path: '/socket.io',
        ...options.websocket
      },
      ...options
    };

    this.app = express();
    this.server = createServer(this.app);
    this.setupMiddleware();
    this.setupRoutes();
    this.setupWebSocket();
  }

  /**
   * Start the API server
   */
  async start(): Promise<void> {
    return new Promise((resolve) => {
      this.server.listen(this.options.port, this.options.host!, () => {
        console.log(`REST API server listening on ${this.options.host}:${this.options.port}`);
        resolve();
      });
    });
  }

  /**
   * Stop the API server
   */
  async stop(): Promise<void> {
    // Close all active streams
    for (const [streamId, stream] of this.activeStreams) {
      if (stream.destroy) {
        stream.destroy();
      }
    }
    this.activeStreams.clear();

    // Close WebSocket connections
    if (this.io) {
      await new Promise<void>((resolve) => {
        this.io!.close(() => resolve());
      });
    }

    // Close HTTP server
    return new Promise((resolve) => {
      this.server.close(() => {
        console.log('REST API server stopped');
        resolve();
      });
    });
  }

  /**
   * Setup middleware
   */
  private setupMiddleware(): void {
    // Security
    this.app.use(helmet());

    // CORS
    if (this.options.cors?.enabled) {
      this.app.use(cors({
        origin: this.options.cors.origins,
        methods: this.options.cors.methods,
        allowedHeaders: this.options.cors.headers,
        credentials: this.options.cors.credentials
      }));
    }

    // Compression
    this.app.use(compression());

    // Body parsing
    this.app.use(express.json({ limit: '10mb' }));
    this.app.use(express.urlencoded({ extended: true }));

    // Request ID
    this.app.use((req, res, next) => {
      req.id = `req-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      res.setHeader('X-Request-ID', req.id);
      next();
    });

    // Logging
    this.app.use((req, res, next) => {
      console.log(`${new Date().toISOString()} ${req.method} ${req.path} [${req.id}]`);
      next();
    });

    // Authentication
    if (this.options.auth?.enabled) {
      this.app.use(authMiddleware(this.options.auth));
    }

    // Rate limiting
    if (this.options.rateLimit?.enabled) {
      this.app.use(rateLimitMiddleware(this.options.rateLimit));
    }
  }

  /**
   * Setup API routes
   */
  private setupRoutes(): void {
    const router = express.Router();

    // Health check
    router.get('/health', this.handleHealthCheck.bind(this));

    // Task execution
    router.post('/tasks/execute', 
      validationMiddleware('executeTask'),
      this.handleExecuteTask.bind(this)
    );

    router.post('/tasks/execute-batch',
      validationMiddleware('executeBatch'),
      this.handleExecuteBatch.bind(this)
    );

    router.get('/tasks/:requestId/result',
      this.handleGetTaskResult.bind(this)
    );

    router.get('/batches/:batchId/results',
      this.handleGetBatchResults.bind(this)
    );

    // Streaming
    router.post('/tasks/stream',
      validationMiddleware('streamTask'),
      this.handleStreamTask.bind(this)
    );

    // Models
    router.get('/models',
      this.handleGetModels.bind(this)
    );

    // Metrics
    router.get('/metrics',
      this.handleGetMetrics.bind(this)
    );

    // Queue
    router.get('/queue/status',
      this.handleGetQueueStatus.bind(this)
    );

    // Cost estimation
    router.post('/cost/estimate',
      validationMiddleware('costEstimate'),
      this.handleGetCostEstimate.bind(this)
    );

    // Configuration
    router.get('/config',
      this.handleGetConfiguration.bind(this)
    );

    router.put('/config',
      validationMiddleware('updateConfig'),
      this.handleUpdateConfiguration.bind(this)
    );

    // Apply routes
    this.app.use('/api/v1', router);

    // Error handling
    this.app.use(errorHandler);

    // 404 handler
    this.app.use((req, res) => {
      res.status(404).json({
        error: {
          code: 'NOT_FOUND',
          message: 'Endpoint not found',
          timestamp: new Date().toISOString()
        }
      });
    });
  }

  /**
   * Setup WebSocket server
   */
  private setupWebSocket(): void {
    if (!this.options.websocket?.enabled) return;

    this.io = new SocketIOServer(this.server, {
      path: this.options.websocket.path,
      cors: this.options.cors
    });

    this.io.on('connection', (socket) => {
      console.log(`WebSocket client connected: ${socket.id}`);

      // Handle task execution
      socket.on('execute', async (data: ExecuteTaskRequest, callback) => {
        try {
          const response = await this.executeTask(data);
          callback({ success: true, data: response });
        } catch (error: any) {
          callback({ success: false, error: this.createError(error) });
        }
      });

      // Handle streaming
      socket.on('stream', async (data: StreamTaskRequest) => {
        const streamId = `stream-${Date.now()}`;
        
        try {
          // Start streaming
          socket.emit('stream:start', { streamId });

          // Execute task and stream results
          const task = data.task;
          const result = await this.engine.executeTask(task, {
            callbacks: {
              onProgress: (progress) => {
                socket.emit('stream:progress', {
                  streamId,
                  progress: progress.percentage,
                  tokens: progress.tokens
                });
              }
            }
          });

          socket.emit('stream:complete', {
            streamId,
            result
          });
        } catch (error: any) {
          socket.emit('stream:error', {
            streamId,
            error: this.createError(error)
          });
        }
      });

      // Handle disconnection
      socket.on('disconnect', () => {
        console.log(`WebSocket client disconnected: ${socket.id}`);
      });
    });
  }

  // Route handlers

  private async handleHealthCheck(req: Request, res: Response): Promise<void> {
    const health: HealthCheckResponse = {
      status: 'healthy',
      version: '1.0.0',
      uptime: process.uptime(),
      components: {
        orchestrationEngine: {
          status: 'healthy',
          lastCheck: new Date().toISOString()
        },
        providers: {}
      }
    };

    // Check queue health
    const queueStats = await this.engine.getQueueStats();
    if (queueStats) {
      health.components.queueManager = {
        status: queueStats.queueLength > 1000 ? 'degraded' : 'healthy',
        lastCheck: new Date().toISOString(),
        metadata: {
          queueLength: queueStats.queueLength,
          activeRequests: queueStats.activeRequests
        }
      };
    }

    res.json(health);
  }

  private async handleExecuteTask(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const request: ExecuteTaskRequest = req.body;
      const response = await this.executeTask(request);
      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleExecuteBatch(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const request: ExecuteBatchRequest = req.body;
      const batchId = `batch-${Date.now()}`;
      
      // Queue batch for processing
      const promises = request.tasks.map(task => 
        this.engine.queueTask(task, {
          priority: request.options?.priority as any || RequestPriority.Normal,
          metadata: { batchId }
        })
      );

      await Promise.all(promises);

      const response: ExecuteBatchResponse = {
        batchId,
        taskCount: request.tasks.length,
        status: 'accepted',
        estimatedCompletionTime: request.tasks.length * 5000 // Rough estimate
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleGetTaskResult(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { requestId } = req.params;
      const { wait, timeout } = req.query;

      // For now, return a mock response
      // In a real implementation, this would check the queue status
      const response: GetTaskResultResponse = {
        requestId,
        status: 'completed',
        metadata: {
          queuedAt: new Date().toISOString(),
          completedAt: new Date().toISOString()
        }
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleGetBatchResults(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { batchId } = req.params;
      const { includePartial } = req.query;

      // Mock response
      const response: GetBatchResultsResponse = {
        batchId,
        status: 'completed',
        totalTasks: 10,
        completedTasks: 10,
        failedTasks: 0,
        results: []
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleStreamTask(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const request: StreamTaskRequest = req.body;
      const streamId = `stream-${Date.now()}`;

      // Set up SSE
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        'Connection': 'keep-alive'
      });

      // Send initial response
      res.write(`data: ${JSON.stringify({
        type: 'start',
        requestId: streamId
      })}\n\n`);

      // Execute task with streaming
      const result = await this.engine.executeTask(request.task, {
        callbacks: {
          onProgress: (progress) => {
            res.write(`data: ${JSON.stringify({
              type: 'progress',
              requestId: streamId,
              data: {
                progress: progress.percentage,
                tokens: progress.tokens
              }
            })}\n\n`);
          }
        }
      });

      // Send final result
      res.write(`data: ${JSON.stringify({
        type: 'complete',
        requestId: streamId,
        data: { result }
      })}\n\n`);

      res.end();
    } catch (error) {
      next(error);
    }
  }

  private async handleGetModels(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { includeUnavailable, taskType } = req.query;
      const models = await this.engine.listModels();

      const response: GetModelsResponse = {
        models,
        defaultModel: 'gpt-3.5-turbo'
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleGetMetrics(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const metrics = await this.engine.getMetrics();

      const response: GetMetricsResponse = {
        metrics,
        period: {
          start: new Date(Date.now() - 3600000).toISOString(),
          end: new Date().toISOString()
        }
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleGetQueueStatus(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const stats = await this.engine.getQueueStats();

      if (!stats) {
        res.json({
          stats: {
            queueLength: 0,
            activeRequests: 0,
            completedRequests: 0,
            failedRequests: 0,
            averageWaitTime: 0,
            averageProcessingTime: 0,
            throughput: 0,
            errorRate: 0,
            utilizationRate: 0,
            priorityDistribution: {}
          },
          isAcceptingRequests: true,
          backpressureActive: false
        });
        return;
      }

      const response: GetQueueStatusResponse = {
        stats,
        isAcceptingRequests: true,
        backpressureActive: false,
        details: {
          priorityBreakdown: stats.priorityDistribution,
          oldestRequestAge: stats.oldestRequestAge
        }
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleGetCostEstimate(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const request: GetCostEstimateRequest = req.body;
      const estimate = await this.engine.getCostEstimate(request.task);

      const response: GetCostEstimateResponse = {
        estimate,
        selectedModel: 'gpt-3.5-turbo',
        alternatives: []
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleGetConfiguration(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const config = this.engine.getConfiguration();

      const response: GetConfigurationResponse = {
        config,
        version: '1.0.0',
        lastModified: new Date().toISOString()
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  private async handleUpdateConfiguration(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const request: UpdateConfigurationRequest = req.body;

      if (request.validateOnly) {
        // Just validate
        const response: UpdateConfigurationResponse = {
          success: true,
          validation: {
            valid: true,
            errors: [],
            warnings: []
          }
        };
        res.json(response);
        return;
      }

      // Update configuration
      await this.engine.updateConfig(request.config);

      const response: UpdateConfigurationResponse = {
        success: true,
        appliedChanges: Object.keys(request.config)
      };

      res.json(response);
    } catch (error) {
      next(error);
    }
  }

  // Helper methods

  private async executeTask(request: ExecuteTaskRequest): Promise<ExecuteTaskResponse> {
    const requestId = await this.engine.queueTask(request.task, {
      priority: request.options?.priority as any,
      metadata: request.options?.metadata
    });

    const queueStats = await this.engine.getQueueStats();

    return {
      requestId,
      status: 'queued',
      estimatedWaitTime: queueStats?.averageWaitTime || 5000,
      queuePosition: queueStats?.queueLength || 1
    };
  }

  private createError(error: any): APIError {
    return {
      code: error.code || 'INTERNAL_ERROR',
      message: error.message || 'An internal error occurred',
      details: error.details,
      timestamp: new Date().toISOString()
    };
  }
}

// Extend Express Request type
declare global {
  namespace Express {
    interface Request {
      id?: string;
      user?: any;
    }
  }
}