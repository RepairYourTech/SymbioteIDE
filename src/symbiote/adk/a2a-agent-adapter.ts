/**
 * A2A Agent Adapter
 * 
 * Adapts ADK agents to support A2A protocol for inter-agent communication
 */

// import { Agent } from '@google/adk';
import { ADKAgent as Agent, A2AConfig, AgentCard } from './types';
import { Logger } from '../utils/logger';
import { A2AClient } from '../a2a/client';
import { A2AServer } from '../a2a/server';
import { 
  Task, 
  TaskStatus, 
  Message, 
  Artifact,
  A2AError,
  A2AAuthentication
} from '../a2a/types';

export class A2AAgentAdapter {
  private logger = new Logger('A2AAgentAdapter');
  private agents = new Map<string, A2AAgentWrapper>();
  private a2aServer: A2AServer;
  private isServerStarted = false;

  constructor() {
    this.a2aServer = new A2AServer();
    this.setupServer();
  }

  /**
   * Register an ADK agent with A2A capabilities
   */
  async registerAgent(
    agentId: string, 
    agent: Agent, 
    config: A2AConfig
  ): Promise<void> {
    try {
      const wrapper = new A2AAgentWrapper(agentId, agent, config);
      this.agents.set(agentId, wrapper);

      // Register with A2A server
      await this.a2aServer.registerAgent(agentId, {
        ...config.agentCard,
        endpoints: config.agentCard.endpoints || []
      } as any);

      // Set up endpoints
      this.setupAgentEndpoints(agentId, wrapper);

      this.logger.info(`Registered agent ${agentId} with A2A protocol`);
    } catch (error) {
      this.logger.error(`Failed to register agent ${agentId}`, error);
      throw error;
    }
  }

  /**
   * Create A2A client for agent-to-agent communication
   */
  createA2AClient(agentId: string): A2AClient {
    const wrapper = this.agents.get(agentId);
    if (!wrapper) {
      throw new Error(`Agent ${agentId} not registered`);
    }

    return new A2AClient({
      agentId,
      authentication: wrapper.config.authentication ? {
        type: wrapper.config.authentication.type === 'none' ? 'custom' : wrapper.config.authentication.type,
        credentials: wrapper.config.authentication.credentials || wrapper.config.authentication.config
      } as A2AAuthentication : undefined
    });
  }

  /**
   * Setup A2A server
   */
  private setupServer(): void {
    // Handle incoming task requests
    this.a2aServer.on('task:create', async (task: Task) => {
      const wrapper = this.agents.get(task.agentId);
      if (!wrapper) {
        throw new A2AError('AGENT_NOT_FOUND', `Agent ${task.agentId} not found`);
      }

      return wrapper.handleTask(task);
    });

    // Handle incoming messages
    this.a2aServer.on('message:send', async (agentId: string, message: Message) => {
      const wrapper = this.agents.get(agentId);
      if (!wrapper) {
        throw new A2AError('AGENT_NOT_FOUND', `Agent ${agentId} not found`);
      }

      return wrapper.handleMessage(message);
    });
  }

  /**
   * Setup agent-specific endpoints
   */
  private setupAgentEndpoints(agentId: string, wrapper: A2AAgentWrapper): void {
    const endpoints = wrapper.config.endpoints;

    // Discovery endpoint
    this.a2aServer.addEndpoint(endpoints.discovery, async () => {
      return wrapper.config.agentCard;
    });

    // Task endpoint
    this.a2aServer.addEndpoint(endpoints.task, async (req: any) => {
      return wrapper.handleTaskRequest(req);
    });

    // Message endpoint
    this.a2aServer.addEndpoint(endpoints.message, async (req: any) => {
      return wrapper.handleMessageRequest(req);
    });

    // Artifact endpoint
    this.a2aServer.addEndpoint(endpoints.artifact, async (req: any) => {
      return wrapper.handleArtifactRequest(req);
    });
  }

  /**
   * Start A2A server
   */
  async start(config?: any): Promise<void> {
    if (this.isServerStarted) {
      return;
    }

    try {
      await this.a2aServer.start();
      this.isServerStarted = true;
      this.logger.info('A2A server started');
    } catch (error) {
      this.logger.error('Failed to start A2A server', error);
      throw error;
    }
  }

  /**
   * Stop A2A server
   */
  async stop(): Promise<void> {
    if (!this.isServerStarted) {
      return;
    }

    try {
      await this.a2aServer.stop();
      this.isServerStarted = false;
      this.logger.info('A2A server stopped');
    } catch (error) {
      this.logger.error('Failed to stop A2A server', error);
      throw error;
    }
  }

  /**
   * Unregister an agent
   */
  async unregisterAgent(agentId: string): Promise<void> {
    this.agents.delete(agentId);
    this.logger.info(`Unregistered agent ${agentId}`);
  }

  /**
   * Get all registered agents
   */
  getRegisteredAgents(): Array<{ id: string; card: AgentCard }> {
    return Array.from(this.agents.entries()).map(([id, wrapper]) => ({
      id,
      card: wrapper.config.agentCard
    }));
  }
}

/**
 * Wrapper for ADK agents with A2A capabilities
 */
class A2AAgentWrapper {
  private logger: Logger;
  private activeTasks = new Map<string, Task>();
  private artifacts = new Map<string, Artifact[]>();

  constructor(
    public id: string,
    public agent: Agent,
    public config: A2AConfig
  ) {
    this.logger = new Logger(`A2AAgent:${id}`);
  }

  /**
   * Handle incoming task
   */
  async handleTask(task: Task): Promise<Task> {
    this.logger.info(`Handling task ${task.id}`);
    
    // Store task
    this.activeTasks.set(task.id, task);
    
    // Update status
    task.status = TaskStatus.InProgress;
    task.metadata.startTime = new Date();

    try {
      // Execute with ADK agent
      const result = await this.agent.execute({
        id: task.id,
        type: 'a2a-task',
        description: (task as any).name || task.description || '',
        input: task.input,
        context: task.context ? {
          previousTasks: [task.context.previousTaskId].filter(Boolean),
          additionalInstructions: task.context.additionalInstructions
        } : undefined
      } as any);

      // Update task with result
      task.status = TaskStatus.Completed;
      task.output = result;
      task.metadata.endTime = new Date();
      task.metadata.duration = task.metadata.endTime.getTime() - task.metadata.startTime.getTime();

      // Create artifacts if any
      if (result.artifacts) {
        const artifacts = this.createArtifacts(task.id, result.artifacts);
        this.artifacts.set(task.id, artifacts);
        task.artifacts = artifacts.map(a => a.id);
      }

      return task;
    } catch (error: any) {
      task.status = TaskStatus.Failed;
      task.error = {
        code: error.code || 'EXECUTION_ERROR',
        message: error.message,
        details: error
      };
      task.metadata.endTime = new Date();
      
      throw error;
    }
  }

  /**
   * Handle incoming message
   */
  async handleMessage(message: Message): Promise<void> {
    this.logger.info(`Handling message for task ${message.taskId}`);
    
    const task = this.activeTasks.get(message.taskId);
    if (!task) {
      throw new A2AError('TASK_NOT_FOUND', `Task ${message.taskId} not found`);
    }

    // Process message based on type
    switch (message.type) {
      case 'instruction':
        // Additional instructions for the task
        if (task.context) {
          task.context.additionalInstructions = message.content;
        } else {
          task.context = { additionalInstructions: message.content };
        }
        break;
      
      case 'context':
        // Additional context
        task.context = { ...task.context, ...message.metadata };
        break;
      
      case 'cancel':
        // Cancel the task
        task.status = TaskStatus.Cancelled;
        break;
      
      default:
        this.logger.warn(`Unknown message type: ${message.type}`);
    }
  }

  /**
   * Handle task request (JSON-RPC)
   */
  async handleTaskRequest(req: any): Promise<any> {
    const { method, params, id } = req;

    switch (method) {
      case 'task.create':
        const task = await this.handleTask(params.task);
        return { id, result: task };
      
      case 'task.get':
        const existingTask = this.activeTasks.get(params.taskId);
        if (!existingTask) {
          throw new A2AError('TASK_NOT_FOUND', `Task ${params.taskId} not found`);
        }
        return { id, result: existingTask };
      
      case 'task.cancel':
        const taskToCancel = this.activeTasks.get(params.taskId);
        if (taskToCancel) {
          taskToCancel.status = TaskStatus.Cancelled;
        }
        return { id, result: { success: true } };
      
      default:
        throw new A2AError('METHOD_NOT_FOUND', `Method ${method} not found`);
    }
  }

  /**
   * Handle message request (JSON-RPC)
   */
  async handleMessageRequest(req: any): Promise<any> {
    const { method, params, id } = req;

    switch (method) {
      case 'message.send':
        await this.handleMessage(params.message);
        return { id, result: { success: true } };
      
      case 'message.list':
        // Return messages for a task
        return { id, result: { messages: [] } }; // TODO: Implement message storage
      
      default:
        throw new A2AError('METHOD_NOT_FOUND', `Method ${method} not found`);
    }
  }

  /**
   * Handle artifact request (JSON-RPC)
   */
  async handleArtifactRequest(req: any): Promise<any> {
    const { method, params, id } = req;

    switch (method) {
      case 'artifact.get':
        const taskArtifacts = this.artifacts.get(params.taskId) || [];
        const artifact = taskArtifacts.find(a => a.id === params.artifactId);
        
        if (!artifact) {
          throw new A2AError('ARTIFACT_NOT_FOUND', `Artifact ${params.artifactId} not found`);
        }
        
        return { id, result: artifact };
      
      case 'artifact.list':
        const artifacts = this.artifacts.get(params.taskId) || [];
        return { id, result: { artifacts } };
      
      default:
        throw new A2AError('METHOD_NOT_FOUND', `Method ${method} not found`);
    }
  }

  /**
   * Create artifacts from execution result
   */
  private createArtifacts(taskId: string, resultArtifacts: any[]): Artifact[] {
    return resultArtifacts.map((item, index) => ({
      id: `${taskId}_artifact_${index}`,
      taskId,
      type: item.type || 'text',
      content: item.content,
      metadata: {
        createdAt: new Date(),
        mimeType: item.mimeType || 'text/plain',
        ...item.metadata
      }
    }));
  }
}