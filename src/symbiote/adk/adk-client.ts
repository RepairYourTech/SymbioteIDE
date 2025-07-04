/**
 * Google ADK TypeScript Client
 * 
 * This client communicates with the Python ADK bridge server
 * to access Google AI ADK functionality from TypeScript.
 */

import axios, { AxiosInstance } from 'axios';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { 
  ADKAgent,
  ADKAgentConfig,
  ADKTool,
  ADKAgentType,
  ADKModelInfo,
  AgentExecutionResult,
  ADKTask,
  ADKExecutionResult
} from './types';

export interface ADKClientConfig {
  serverUrl?: string;
  apiKey?: string;
  timeout?: number;
}

export interface CreateAgentRequest {
  id: string;
  name: string;
  type?: string;
  description?: string;
  systemPrompt?: string;
  model?: string;
  temperature?: number;
  tools?: string[];
  memoryEnabled?: boolean;
  a2aEnabled?: boolean;
}

export interface ExecuteTaskRequest {
  agentId: string;
  task: string;
  context?: Record<string, any>;
  stream?: boolean;
}

export interface RegisterToolRequest {
  name: string;
  description: string;
  parameters: Record<string, any>;
  functionName: string;
}

export class ADKClient extends EventEmitter {
  private client: AxiosInstance;
  private logger = new Logger('ADKClient');
  private serverUrl: string;
  
  constructor(config: ADKClientConfig = {}) {
    super();
    
    this.serverUrl = config.serverUrl || 'http://localhost:8888';
    
    this.client = axios.create({
      baseURL: this.serverUrl,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...(config.apiKey && { 'Authorization': `Bearer ${config.apiKey}` })
      }
    });
    
    this.setupInterceptors();
  }
  
  private setupInterceptors(): void {
    // Request interceptor
    this.client.interceptors.request.use(
      (config) => {
        this.logger.debug('ADK Request:', config.method, config.url);
        return config;
      },
      (error) => {
        this.logger.error('ADK Request Error:', error);
        return Promise.reject(error);
      }
    );
    
    // Response interceptor
    this.client.interceptors.response.use(
      (response) => {
        this.logger.debug('ADK Response:', response.status, response.data);
        return response;
      },
      (error) => {
        this.logger.error('ADK Response Error:', error.response?.data || error.message);
        return Promise.reject(error);
      }
    );
  }
  
  /**
   * Check server health
   */
  async healthCheck(): Promise<{ status: string; adkVersion: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }
  
  /**
   * Create a new agent
   */
  async createAgent(config: CreateAgentRequest): Promise<{ id: string; status: string; message: string }> {
    const response = await this.client.post('/agents/create', config);
    this.emit('agent-created', config.id);
    return response.data;
  }
  
  /**
   * Execute a task with an agent
   */
  async executeTask(request: ExecuteTaskRequest): Promise<AgentExecutionResult> {
    if (request.stream) {
      return this.executeStreamingTask(request);
    }
    
    const response = await this.client.post(`/agents/${request.agentId}/execute`, {
      agent_id: request.agentId,
      task: request.task,
      context: request.context,
      stream: false
    });
    
    return response.data;
  }
  
  /**
   * Execute a streaming task
   */
  private async executeStreamingTask(request: ExecuteTaskRequest): Promise<AgentExecutionResult> {
    const response = await this.client.post(
      `/agents/${request.agentId}/execute`,
      {
        agent_id: request.agentId,
        task: request.task,
        context: request.context,
        stream: true
      },
      {
        responseType: 'stream'
      }
    );
    
    // Handle streaming response
    const chunks: string[] = [];
    
    return new Promise((resolve, reject) => {
      response.data.on('data', (chunk: Buffer) => {
        const lines = chunk.toString().split('\n').filter(line => line.trim());
        
        for (const line of lines) {
          try {
            const data = JSON.parse(line);
            
            if (data.type === 'chunk') {
              chunks.push(data.data);
              this.emit('stream-chunk', {
                agentId: request.agentId,
                chunk: data.data
              });
            } else if (data.type === 'complete') {
              resolve({
                executionId: `exec_${Date.now()}`,
                agentId: request.agentId,
                status: 'completed' as const,
                result: chunks.join(''),
                metadata: {
                  startTime: new Date(),
                  endTime: new Date(),
                  duration: 0,
                  tokensUsed: 0
                }
              });
            }
          } catch (e) {
            this.logger.debug('Skipping non-JSON line:', line);
          }
        }
      });
      
      response.data.on('error', (error: Error) => {
        reject(error);
      });
    });
  }
  
  /**
   * List all agents
   */
  async listAgents(): Promise<{ agents: Array<{ id: string; name: string; type: string; status: string }> }> {
    const response = await this.client.get('/agents');
    return response.data;
  }
  
  /**
   * Delete an agent
   */
  async deleteAgent(agentId: string): Promise<{ message: string }> {
    const response = await this.client.delete(`/agents/${agentId}`);
    this.emit('agent-deleted', agentId);
    return response.data;
  }
  
  /**
   * Register a tool
   */
  async registerTool(config: RegisterToolRequest): Promise<{ name: string; status: string; message: string }> {
    const response = await this.client.post('/tools/register', config);
    this.emit('tool-registered', config.name);
    return response.data;
  }
  
  /**
   * List all tools
   */
  async listTools(): Promise<{ tools: ADKTool[] }> {
    const response = await this.client.get('/tools');
    return response.data;
  }
  
  /**
   * Orchestrate multiple agents
   */
  async orchestrate(task: string, agentIds: string[]): Promise<any> {
    const response = await this.client.post('/orchestrate', {
      task,
      agent_ids: agentIds
    });
    return response.data;
  }
  
  /**
   * List available models
   */
  async listModels(): Promise<{ models: ADKModelInfo[] }> {
    const response = await this.client.get('/models');
    return response.data;
  }
  
  /**
   * Start the ADK bridge server (if not already running)
   */
  async startServer(): Promise<void> {
    try {
      await this.healthCheck();
      this.logger.info('ADK server is already running');
    } catch (error) {
      this.logger.info('Starting ADK bridge server...');
      
      // Start the Python server
      const { spawn } = require('child_process');
      const pythonPath = process.env.PYTHON_PATH || 'python3';
      const serverPath = require('path').join(__dirname, 'python-bridge', 'adk_server.py');
      
      const serverProcess = spawn(pythonPath, [serverPath], {
        env: { ...process.env },
        detached: true
      });
      
      serverProcess.stdout.on('data', (data: Buffer) => {
        this.logger.debug('ADK Server:', data.toString());
      });
      
      serverProcess.stderr.on('data', (data: Buffer) => {
        this.logger.error('ADK Server Error:', data.toString());
      });
      
      // Wait for server to start
      await this.waitForServer();
    }
  }
  
  /**
   * Wait for server to be ready
   */
  private async waitForServer(maxAttempts = 30, delay = 1000): Promise<void> {
    for (let i = 0; i < maxAttempts; i++) {
      try {
        await this.healthCheck();
        this.logger.info('ADK server is ready');
        return;
      } catch (error) {
        if (i === maxAttempts - 1) {
          throw new Error('ADK server failed to start');
        }
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
  }
  
  /**
   * Create an ADK agent instance (convenience method)
   */
  async createADKAgent(config: ADKAgentConfig): Promise<ADKAgent> {
    // Create agent on server
    await this.createAgent({
      id: config.id,
      name: config.name,
      type: config.type || ADKAgentType.LLM,
      description: config.description,
      systemPrompt: config.systemPrompt,
      model: config.model?.id,
      temperature: config.temperature,
      tools: config.tools?.map(t => t.name),
      memoryEnabled: config.capabilities?.memoryAccess,
      a2aEnabled: config.capabilities?.a2aProtocol
    });
    
    // Return a proxy object that executes through the client
    const self = this;
    const agent: ADKAgent = {
      id: config.id,
      name: config.name,
      type: config.type,
      config,
      
      async execute(task: ADKTask | any): Promise<ADKExecutionResult> {
        const taskString = typeof task === 'string' ? task : JSON.stringify(task);
        const result = await self.executeTask({
          agentId: config.id,
          task: taskString,
          context: typeof task === 'object' ? task : undefined,
          stream: false
        });
        
        // Transform to ADKExecutionResult
        return {
          taskId: `task_${Date.now()}`,
          agentId: config.id,
          status: 'success',
          output: result.result || result,
          duration: result.metadata?.duration || 0,
          metadata: result.metadata
        };
      },
      
      executeTask: async (task: string, context?: any) => {
        const result = await self.executeTask({
          agentId: config.id,
          task: task,
          context: context,
          stream: false
        });
        return result.result;
      },
      
      stream: config.capabilities?.streaming || false,
      
      removeTool: async (toolName: string) => {
        // Tools are configured during agent creation
        self.logger.warn('Dynamic tool removal not yet implemented');
      },
      
      on: (event: string, handler: (data: any) => void) => {
        self.on(event, handler);
      },
      
      logger: self.logger,
      
      addTool: async (tool: any) => {
        // Tools are added during agent creation
        self.logger.warn('Dynamic tool addition not yet implemented');
      }
    };
    
    // No need to bind as we're using arrow functions
    
    return agent;
  }
}