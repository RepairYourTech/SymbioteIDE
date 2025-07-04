/**
 * ADK Orchestrator
 * 
 * Main orchestrator for Google AI ADK agents with A2A and MCP integration
 */

import { EventEmitter } from 'events';
// These are type definitions only - actual agents run in Python via ADKClient
import {
  ADKAgent as Agent,
  SequentialAgent,
  ParallelAgent,
  LoopAgent,
  LlmAgent as LLMAgent,
  ExecutionContext,
  ExecutionResult
} from './types';
import { 
  ADKAgentConfig,
  ADKAgentType,
  ADKOrchestratorConfig,
  AgentExecutionRequest,
  AgentExecutionResult,
  OrchestratorEvent
} from './types';
import { AgentFactory } from './agent-factory';
import { MCPToolAdapter } from './mcp-tool-adapter';
import { A2AAgentAdapter } from './a2a-agent-adapter';
import { MemoryAdapter } from './memory-adapter';
import { Logger } from '../utils/logger';
import { A2AClient } from '../a2a/client';
import { Task, TaskStatus, Message } from '../a2a/types';
import { RedisClient } from '../infrastructure/redis/redis-client';
import { MCPManager } from '../mcp/client/mcp-manager';

export class ADKOrchestrator extends EventEmitter {
  private logger = new Logger('ADKOrchestrator');
  // private agentManager: AgentManager;  // TODO: Implement AgentManager
  private agentFactory: AgentFactory;
  private mcpToolAdapter: MCPToolAdapter;
  private a2aAdapter: A2AAgentAdapter;
  private memoryAdapter: MemoryAdapter;
  private agents = new Map<string, Agent>();
  private executionContexts = new Map<string, ExecutionContext>();
  private redis?: RedisClient;
  private mcpManager?: MCPManager;
  private isInitialized = false;

  constructor(private config: ADKOrchestratorConfig = {}) {
    super();
    // this.agentManager = new AgentManager(config.agentManagerConfig);  // TODO: Implement AgentManager
    this.agentFactory = new AgentFactory();
    this.mcpToolAdapter = new MCPToolAdapter();
    this.a2aAdapter = new A2AAgentAdapter();
    this.memoryAdapter = new MemoryAdapter();
  }

  /**
   * Initialize orchestrator
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    try {
      // Initialize Redis if configured
      if (this.config.redis) {
        const { createRedisClient } = await import('../infrastructure/redis/redis-client');
        this.redis = createRedisClient({
          host: process.env.REDIS_HOST || 'localhost',
          port: parseInt(process.env.REDIS_PORT || '6379')
        });
      }

      // Initialize MCP Manager if configured
      if (this.config.mcpEnabled) {
        this.mcpManager = MCPManager.getInstance();
        await this.mcpManager.initialize([]);  // Initialize with empty server list
      }

      // Initialize adapters
      await this.mcpToolAdapter.initialize();
      
      // Start A2A server if configured
      if (this.config.a2aServerConfig) {
        await this.a2aAdapter.start(this.config.a2aServerConfig);
      }

      this.isInitialized = true;
      this.logger.info('ADK Orchestrator initialized');
      this.emit(OrchestratorEvent.Initialized);
    } catch (error) {
      this.logger.error('Failed to initialize orchestrator', error);
      throw error;
    }
  }

  /**
   * Create and register an agent
   */
  async createAgent(config: ADKAgentConfig): Promise<string> {
    try {
      // Create agent using factory
      const agent = await this.agentFactory.createAgent(config);
      
      // Register with agent manager
      await this.agentManager.registerAgent(agent);
      
      // Store in local registry
      this.agents.set(config.id, agent);
      
      // Register with A2A if configured
      if (config.a2aConfig) {
        await this.a2aAdapter.registerAgent(config.id, agent, config.a2aConfig);
      }
      
      // Set up memory if configured
      if (config.memory) {
        const memory = await this.memoryAdapter.createMemory(config.memory);
        // agent.setMemory(memory);  // TODO: Add memory support to agents
      }
      
      this.logger.info(`Created agent: ${config.id}`);
      this.emit(OrchestratorEvent.AgentCreated, { agentId: config.id, config });
      
      return config.id;
    } catch (error) {
      this.logger.error(`Failed to create agent ${config.id}`, error);
      throw error;
    }
  }

  /**
   * Execute agent task
   */
  async executeAgent(request: AgentExecutionRequest): Promise<AgentExecutionResult> {
    const startTime = Date.now();
    const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    try {
      const agent = this.agents.get(request.agentId);
      if (!agent) {
        throw new Error(`Agent ${request.agentId} not found`);
      }

      // Create execution context
      const context = this.createExecutionContext(executionId, request);
      this.executionContexts.set(executionId, context);
      
      // Emit execution started event
      this.emit(OrchestratorEvent.ExecutionStarted, {
        executionId,
        agentId: request.agentId,
        request
      });
      
      // Execute agent
      const result = await agent.execute({
        input: request.input,
        context: {
          ...context,
          ...request.context
        }
      });
      
      // Process result
      const executionResult: AgentExecutionResult = {
        executionId,
        agentId: request.agentId,
        status: 'completed',
        result: result.output,
        artifacts: result.artifacts,
        metadata: {
          startTime: new Date(startTime),
          endTime: new Date(),
          duration: Date.now() - startTime,
          tokensUsed: result.tokensUsed,
          ...result.metadata
        }
      };
      
      // Store result if caching enabled
      if (this.config.cacheResults && this.redis) {
        await this.cacheResult(executionId, executionResult);
      }
      
      // Emit execution completed event
      this.emit(OrchestratorEvent.ExecutionCompleted, {
        executionId,
        result: executionResult
      });
      
      return executionResult;
    } catch (error: any) {
      const executionResult: AgentExecutionResult = {
        executionId,
        agentId: request.agentId,
        status: 'failed',
        error: {
          code: error.code || 'EXECUTION_ERROR',
          message: error.message,
          details: error
        },
        metadata: {
          startTime: new Date(startTime),
          endTime: new Date(),
          duration: Date.now() - startTime
        }
      };
      
      // Emit execution failed event
      this.emit(OrchestratorEvent.ExecutionFailed, {
        executionId,
        error: executionResult.error
      });
      
      throw error;
    } finally {
      // Clean up execution context
      this.executionContexts.delete(executionId);
    }
  }

  /**
   * Execute agent-to-agent task
   */
  async executeA2ATask(
    sourceAgentId: string,
    targetAgentUrl: string,
    task: Partial<Task>
  ): Promise<Task> {
    try {
      const sourceAgent = this.agents.get(sourceAgentId);
      if (!sourceAgent) {
        throw new Error(`Source agent ${sourceAgentId} not found`);
      }

      // Create A2A client
      const a2aClient = this.a2aAdapter.createA2AClient(sourceAgentId);
      
      // Send task to target agent
      const result = await a2aClient.createTask(targetAgentUrl, task);
      
      // Emit A2A task event
      this.emit(OrchestratorEvent.A2ATaskCreated, {
        sourceAgentId,
        targetAgentUrl,
        taskId: result.id
      });
      
      return result;
    } catch (error) {
      this.logger.error('Failed to execute A2A task', error);
      throw error;
    }
  }

  /**
   * Create workflow agent
   */
  async createWorkflow(
    id: string,
    steps: ADKAgentConfig[],
    options?: any
  ): Promise<string> {
    try {
      // Create sub-agents
      const subAgents: Agent[] = [];
      for (const stepConfig of steps) {
        const agent = await this.agentFactory.createAgent(stepConfig);
        subAgents.push(agent);
      }
      
      // Create workflow agent
      const workflowConfig: ADKAgentConfig = {
        id,
        name: `Workflow: ${id}`,
        type: ADKAgentType.Workflow,
        description: options?.description || 'Workflow agent',
        capabilities: {
          streaming: true,
          parallel: true,
          contextAware: true
        }
      };
      
      const workflowAgent = new WorkflowAgent({
        ...workflowConfig,
        agents: subAgents,
        ...options
      });
      
      // Register workflow
      await this.agentManager.registerAgent(workflowAgent);
      this.agents.set(id, workflowAgent);
      
      this.logger.info(`Created workflow: ${id} with ${steps.length} steps`);
      return id;
    } catch (error) {
      this.logger.error(`Failed to create workflow ${id}`, error);
      throw error;
    }
  }

  /**
   * Get agent by ID
   */
  getAgent(agentId: string): Agent | undefined {
    return this.agents.get(agentId);
  }

  /**
   * List all agents
   */
  listAgents(): Array<{ id: string; type: string; status: string }> {
    return Array.from(this.agents.entries()).map(([id, agent]) => ({
      id,
      type: agent.type || 'unknown',
      status: agent.status || 'ready'
    }));
  }

  /**
   * Remove agent
   */
  async removeAgent(agentId: string): Promise<void> {
    try {
      const agent = this.agents.get(agentId);
      if (!agent) {
        throw new Error(`Agent ${agentId} not found`);
      }
      
      // Unregister from agent manager
      await this.agentManager.unregisterAgent(agentId);
      
      // Remove from local registry
      this.agents.delete(agentId);
      
      // Clean up A2A registration
      await this.a2aAdapter.unregisterAgent(agentId);
      
      this.logger.info(`Removed agent: ${agentId}`);
      this.emit(OrchestratorEvent.AgentRemoved, { agentId });
    } catch (error) {
      this.logger.error(`Failed to remove agent ${agentId}`, error);
      throw error;
    }
  }

  /**
   * Create execution context
   */
  private createExecutionContext(
    executionId: string,
    request: AgentExecutionRequest
  ): ExecutionContext {
    return {
      executionId,
      agentId: request.agentId,
      userId: request.userId,
      sessionId: request.sessionId,
      parentExecutionId: request.parentExecutionId,
      timestamp: new Date(),
      environment: this.config.environment || 'production',
      features: {
        streaming: request.streaming,
        caching: this.config.cacheResults,
        monitoring: this.config.monitoring
      }
    };
  }

  /**
   * Cache execution result
   */
  private async cacheResult(
    executionId: string,
    result: AgentExecutionResult
  ): Promise<void> {
    if (!this.redis) return;
    
    try {
      const key = `adk:execution:${executionId}`;
      const ttl = this.config.cacheTTL || 3600; // 1 hour default
      
      await this.redis.setex(
        key,
        ttl,
        JSON.stringify(result)
      );
      
      this.logger.debug(`Cached execution result: ${executionId}`);
    } catch (error) {
      this.logger.error('Failed to cache result', error);
    }
  }

  /**
   * Get cached result
   */
  async getCachedResult(executionId: string): Promise<AgentExecutionResult | null> {
    if (!this.redis) return null;
    
    try {
      const key = `adk:execution:${executionId}`;
      const cached = await this.redis.get(key);
      
      if (cached) {
        return JSON.parse(cached);
      }
      
      return null;
    } catch (error) {
      this.logger.error('Failed to get cached result', error);
      return null;
    }
  }

  /**
   * Subscribe to orchestrator events
   */
  subscribe(event: OrchestratorEvent, handler: (data: any) => void): void {
    this.on(event, handler);
  }

  /**
   * Unsubscribe from orchestrator events
   */
  unsubscribe(event: OrchestratorEvent, handler: (data: any) => void): void {
    this.off(event, handler);
  }

  /**
   * Get orchestrator metrics
   */
  getMetrics(): any {
    return {
      agentCount: this.agents.size,
      activeExecutions: this.executionContexts.size,
      a2aEnabled: !!this.config.a2aServerConfig,
      mcpEnabled: !!this.config.mcpEnabled,
      cacheEnabled: !!this.config.cacheResults
    };
  }

  /**
   * Shutdown orchestrator
   */
  async shutdown(): Promise<void> {
    try {
      // Stop all agents
      for (const [agentId, agent] of this.agents) {
        await this.removeAgent(agentId);
      }
      
      // Stop A2A server
      await this.a2aAdapter.stop();
      
      // Close Redis connection
      if (this.redis) {
        await this.redis.quit();
      }
      
      this.isInitialized = false;
      this.logger.info('ADK Orchestrator shutdown complete');
      this.emit(OrchestratorEvent.Shutdown);
    } catch (error) {
      this.logger.error('Error during shutdown', error);
      throw error;
    }
  }
}

// Export singleton instance
let orchestratorInstance: ADKOrchestrator | null = null;

export function getADKOrchestrator(config?: ADKOrchestratorConfig): ADKOrchestrator {
  if (!orchestratorInstance) {
    orchestratorInstance = new ADKOrchestrator(config);
  }
  return orchestratorInstance;
}