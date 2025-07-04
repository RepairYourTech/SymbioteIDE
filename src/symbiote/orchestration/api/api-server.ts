/**
 * Unified API Server
 * 
 * Main entry point for the orchestration API layer that manages
 * both REST and gRPC servers
 */

import { OrchestrationEngine } from '../orchestration-engine';
import { ExpressAPIServer } from './rest/express-server';
import { GRPCAPIServer } from './grpc/grpc-server';
import { 
  APIServerOptions, 
  APIServerStatus,
  RESTOptions,
  GRPCOptions 
} from './types/api-interfaces';

export class OrchestrationAPIServer {
  private engine: OrchestrationEngine;
  private restServer?: ExpressAPIServer;
  private grpcServer?: GRPCAPIServer;
  private options: APIServerOptions;
  private status: APIServerStatus = 'stopped';

  constructor(engine: OrchestrationEngine, options: APIServerOptions = {}) {
    this.engine = engine;
    this.options = {
      rest: {
        enabled: true,
        port: 3000,
        ...options.rest
      },
      grpc: {
        enabled: true,
        port: 50051,
        ...options.grpc
      },
      ...options
    };
  }

  /**
   * Start the API servers
   */
  async start(): Promise<void> {
    try {
      this.status = 'starting';
      
      const startPromises: Promise<void>[] = [];

      // Start REST server if enabled
      if (this.options.rest?.enabled) {
        this.restServer = new ExpressAPIServer(
          this.engine,
          this.options.rest as RESTOptions
        );
        startPromises.push(this.restServer.start());
      }

      // Start gRPC server if enabled
      if (this.options.grpc?.enabled) {
        this.grpcServer = new GRPCAPIServer(
          this.engine,
          this.options.grpc as GRPCOptions
        );
        startPromises.push(this.grpcServer.start());
      }

      // Wait for all servers to start
      await Promise.all(startPromises);
      
      this.status = 'running';
      console.log('Orchestration API server started successfully');
    } catch (error) {
      this.status = 'error';
      console.error('Failed to start API server:', error);
      throw error;
    }
  }

  /**
   * Stop the API servers
   */
  async stop(): Promise<void> {
    try {
      this.status = 'stopping';
      
      const stopPromises: Promise<void>[] = [];

      // Stop REST server
      if (this.restServer) {
        stopPromises.push(this.restServer.stop());
      }

      // Stop gRPC server
      if (this.grpcServer) {
        stopPromises.push(this.grpcServer.stop());
      }

      // Wait for all servers to stop
      await Promise.all(stopPromises);
      
      this.status = 'stopped';
      console.log('Orchestration API server stopped');
    } catch (error) {
      console.error('Error stopping API server:', error);
      throw error;
    }
  }

  /**
   * Get server status
   */
  getStatus(): APIServerStatus {
    return this.status;
  }

  /**
   * Get server information
   */
  getInfo(): {
    status: APIServerStatus;
    servers: {
      rest?: {
        enabled: boolean;
        port?: number;
        url?: string;
      };
      grpc?: {
        enabled: boolean;
        port?: number;
        url?: string;
      };
    };
    uptime: number;
  } {
    return {
      status: this.status,
      servers: {
        rest: this.options.rest?.enabled ? {
          enabled: true,
          port: this.options.rest.port,
          url: `http://localhost:${this.options.rest.port}`
        } : { enabled: false },
        grpc: this.options.grpc?.enabled ? {
          enabled: true,
          port: this.options.grpc.port,
          url: `localhost:${this.options.grpc.port}`
        } : { enabled: false }
      },
      uptime: process.uptime()
    };
  }

  /**
   * Update server configuration
   */
  async updateConfig(config: Partial<APIServerOptions>): Promise<void> {
    // Merge new config
    this.options = { ...this.options, ...config };

    // If servers are running, restart with new config
    if (this.status === 'running') {
      await this.stop();
      await this.start();
    }
  }

  /**
   * Health check
   */
  async healthCheck(): Promise<{
    healthy: boolean;
    services: {
      orchestrationEngine: boolean;
      restServer?: boolean;
      grpcServer?: boolean;
    };
    details: any;
  }> {
    const health = {
      healthy: true,
      services: {
        orchestrationEngine: true,
        restServer: undefined as boolean | undefined,
        grpcServer: undefined as boolean | undefined
      },
      details: {}
    };

    try {
      // Check orchestration engine
      const engineStats = await this.engine.getQueueStats();
      health.services.orchestrationEngine = !!engineStats;
    } catch (error) {
      health.services.orchestrationEngine = false;
      health.healthy = false;
    }

    // Check REST server
    if (this.restServer && this.options.rest?.enabled) {
      health.services.restServer = this.status === 'running';
      if (!health.services.restServer) health.healthy = false;
    }

    // Check gRPC server
    if (this.grpcServer && this.options.grpc?.enabled) {
      health.services.grpcServer = this.status === 'running';
      if (!health.services.grpcServer) health.healthy = false;
    }

    return health;
  }
}

// Export convenient factory function
export function createOrchestrationAPI(
  engine: OrchestrationEngine,
  options?: APIServerOptions
): OrchestrationAPIServer {
  return new OrchestrationAPIServer(engine, options);
}