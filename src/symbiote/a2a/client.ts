/**
 * A2A Client
 * 
 * Client implementation for A2A protocol communication
 */

import { EventEmitter } from 'events';
import { 
  A2AClientConfig,
  AgentCard,
  Task,
  TaskStatus,
  Message,
  Artifact,
  JsonRpcRequest,
  JsonRpcResponse,
  A2AError,
  StatelessTaskRequest,
  StatelessTaskResponse,
  DiscoveryRequest,
  DiscoveryResponse
} from './types';
import { Logger } from '../utils/logger';

export class A2AClient extends EventEmitter {
  private logger: Logger;
  private requestId = 0;
  private pendingRequests = new Map<string | number, {
    resolve: (value: any) => void;
    reject: (error: any) => void;
    timeout: NodeJS.Timeout;
  }>();

  constructor(private config: A2AClientConfig) {
    super();
    this.logger = new Logger(`A2AClient:${config.agentId}`);
  }

  /**
   * Discover available agents
   */
  async discover(request?: DiscoveryRequest): Promise<DiscoveryResponse> {
    try {
      const agents: AgentCard[] = [];
      
      // Well-known discovery endpoint
      const discoveryUrl = `${this.config.baseUrl}/.well-known/agents`;
      
      const response = await this.fetch(discoveryUrl, {
        method: 'GET',
        headers: this.getHeaders()
      });

      if (response.ok) {
        const data = await response.json() as { agents?: AgentCard[] };
        agents.push(...(data.agents || []));
      }

      // Filter by capabilities if requested
      if (request?.capabilities) {
        return {
          agents: agents.filter(agent => 
            request.capabilities!.every(cap => 
              agent.capabilities.includes(cap)
            )
          )
        };
      }

      return { agents };
    } catch (error) {
      this.logger.error('Discovery failed', error);
      throw error;
    }
  }

  /**
   * Get agent card for specific agent
   */
  async getAgentCard(agentUrl: string): Promise<AgentCard> {
    try {
      const cardUrl = `${agentUrl}/.well-known/agent.json`;
      
      const response = await this.fetch(cardUrl, {
        method: 'GET',
        headers: this.getHeaders()
      });

      if (!response.ok) {
        throw new Error(`Failed to get agent card: ${response.statusText}`);
      }

      return response.json() as Promise<AgentCard>;
    } catch (error) {
      this.logger.error('Failed to get agent card', error);
      throw error;
    }
  }

  /**
   * Create a task
   */
  async createTask(agentUrl: string, task: Partial<Task>): Promise<Task> {
    return this.callMethod(agentUrl, 'task.create', { task });
  }

  /**
   * Get task status
   */
  async getTask(agentUrl: string, taskId: string): Promise<Task> {
    return this.callMethod(agentUrl, 'task.get', { taskId });
  }

  /**
   * Cancel a task
   */
  async cancelTask(agentUrl: string, taskId: string): Promise<void> {
    await this.callMethod(agentUrl, 'task.cancel', { taskId });
  }

  /**
   * Send message to agent
   */
  async sendMessage(agentUrl: string, message: Message): Promise<void> {
    await this.callMethod(agentUrl, 'message.send', { message });
  }

  /**
   * Get artifact
   */
  async getArtifact(agentUrl: string, taskId: string, artifactId: string): Promise<Artifact> {
    return this.callMethod(agentUrl, 'artifact.get', { taskId, artifactId });
  }

  /**
   * List artifacts for task
   */
  async listArtifacts(agentUrl: string, taskId: string): Promise<Artifact[]> {
    const result = await this.callMethod(agentUrl, 'artifact.list', { taskId });
    return result.artifacts || [];
  }

  /**
   * Execute stateless task (v0.2)
   */
  async executeStateless(
    agentUrl: string, 
    request: StatelessTaskRequest
  ): Promise<StatelessTaskResponse> {
    try {
      // Create task
      const task = await this.createTask(agentUrl, request.task);
      
      if (!request.waitForCompletion) {
        return {
          taskId: task.id,
          status: task.status
        };
      }

      // Wait for completion
      const timeout = request.timeout || 300000; // 5 minutes default
      const startTime = Date.now();
      
      while (Date.now() - startTime < timeout) {
        const currentTask = await this.getTask(agentUrl, task.id);
        
        if (currentTask.status === TaskStatus.Completed) {
          const artifacts = await this.listArtifacts(agentUrl, task.id);
          return {
            taskId: task.id,
            status: currentTask.status,
            output: currentTask.output,
            artifacts
          };
        }
        
        if (currentTask.status === TaskStatus.Failed) {
          return {
            taskId: task.id,
            status: currentTask.status,
            error: currentTask.error
          };
        }
        
        // Wait before polling again
        await new Promise(resolve => setTimeout(resolve, 1000));
      }
      
      // Timeout
      throw new A2AError('TASK_TIMEOUT', `Task ${task.id} timed out`);
    } catch (error) {
      this.logger.error('Stateless execution failed', error);
      throw error;
    }
  }

  /**
   * Stream task updates via Server-Sent Events
   */
  streamTask(agentUrl: string, taskId: string): EventSource {
    const streamUrl = `${agentUrl}/tasks/${taskId}/stream`;
    const eventSource = new EventSource(streamUrl, {
      headers: this.getHeaders()
    } as any);

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.emit('task:update', data);
      } catch (error) {
        this.logger.error('Failed to parse stream event', error);
      }
    };

    eventSource.onerror = (error) => {
      this.logger.error('Stream error', error);
      this.emit('stream:error', error);
    };

    return eventSource;
  }

  /**
   * Call JSON-RPC method
   */
  private async callMethod(agentUrl: string, method: string, params?: any): Promise<any> {
    const id = ++this.requestId;
    
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id
    };

    try {
      const response = await this.fetch(`${agentUrl}/rpc`, {
        method: 'POST',
        headers: {
          ...this.getHeaders(),
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(request)
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const jsonResponse = await response.json() as JsonRpcResponse;
      
      if (jsonResponse.error) {
        throw new A2AError(
          'RPC_ERROR',
          jsonResponse.error.message,
          jsonResponse.error.data
        );
      }

      return jsonResponse.result;
    } catch (error) {
      this.logger.error(`RPC call failed: ${method}`, error);
      throw error;
    }
  }

  /**
   * Get headers for requests
   */
  private getHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      'User-Agent': `A2AClient/${this.config.agentId}`
    };

    // Add authentication
    if (this.config.authentication) {
      switch (this.config.authentication.type) {
        case 'apiKey':
          headers['X-API-Key'] = this.config.authentication.credentials.apiKey;
          break;
        
        case 'bearer':
          headers['Authorization'] = `Bearer ${this.config.authentication.credentials.token}`;
          break;
        
        case 'oauth2':
          headers['Authorization'] = `Bearer ${this.config.authentication.credentials.accessToken}`;
          break;
      }
    }

    return headers;
  }

  /**
   * Fetch with timeout and retries
   */
  private async fetch(url: string, options: RequestInit): Promise<Response> {
    const timeout = this.config.timeout || 30000;
    const retries = this.config.retries || 3;
    
    for (let attempt = 0; attempt < retries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeout);
        
        const response = await fetch(url, {
          ...options,
          signal: controller.signal
        });
        
        clearTimeout(timeoutId);
        return response;
      } catch (error: any) {
        if (attempt === retries - 1) {
          throw error;
        }
        
        // Wait before retry with exponential backoff
        await new Promise(resolve => 
          setTimeout(resolve, Math.pow(2, attempt) * 1000)
        );
      }
    }
    
    throw new Error('All retries failed');
  }

  /**
   * Close client and cleanup
   */
  close(): void {
    // Clear any pending requests
    for (const [id, pending] of this.pendingRequests) {
      clearTimeout(pending.timeout);
      pending.reject(new Error('Client closed'));
    }
    this.pendingRequests.clear();
    
    this.removeAllListeners();
  }
}