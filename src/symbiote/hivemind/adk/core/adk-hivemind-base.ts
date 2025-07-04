/**
 * ADK Hivemind Base Agent
 * 
 * Base class for all ADK-powered Hivemind agents with shared functionality
 */

import { Agent, Tool, AgentTool } from '@google/adk';
import { Logger } from '../../../utils/logger';
import { MemoryManager } from '../../../memory/mem0/memory-manager';
import { EventEmitter } from 'events';
import { 
  AgentCapabilities,
  HivemindTask,
  TaskResult,
  AgentSpecialty
} from '../../types';
import { HivemindSessionState } from '../state/hivemind-session-state';

export interface ADKHivemindConfig {
  id: string;
  name: string;
  description?: string;
  specialty: AgentSpecialty;
  capabilities: string[];
  systemPrompt?: string;
  temperature?: number;
  maxTokens?: number;
  tools?: Tool[];
  memoryManager?: MemoryManager;
  sessionState?: HivemindSessionState;
}

export abstract class ADKHivemindAgent extends Agent {
  protected logger: Logger;
  protected eventEmitter = new EventEmitter();
  public readonly id: string;
  public readonly name: string;
  public readonly specialty: AgentSpecialty;
  public readonly capabilities: string[];
  protected memoryManager?: MemoryManager;
  protected sessionState?: HivemindSessionState;
  protected performanceHistory: any[] = [];
  
  constructor(config: ADKHivemindConfig) {
    super({
      name: config.name,
      description: config.description,
      instructions: config.systemPrompt,
      temperature: config.temperature,
      max_tokens: config.maxTokens,
      tools: config.tools || []
    });
    
    this.id = config.id;
    this.name = config.name;
    this.specialty = config.specialty;
    this.capabilities = config.capabilities;
    this.memoryManager = config.memoryManager;
    this.sessionState = config.sessionState;
    this.logger = new Logger(`ADKHivemind:${config.name}`);
    
    // Add common tools
    this.addCommonTools();
  }
  
  /**
   * Add common tools available to all Hivemind agents
   */
  protected addCommonTools(): void {
    // State management tool
    this.tools.push(new Tool({
      name: 'read_state',
      description: 'Read from shared session state',
      parameters: {
        key: { type: 'string', required: true }
      },
      handler: async ({ key }: { key: string }) => {
        return this.readState(key);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'write_state',
      description: 'Write to shared session state',
      parameters: {
        key: { type: 'string', required: true },
        value: { type: 'any', required: true }
      },
      handler: async ({ key, value }: { key: string; value: any }) => {
        return this.writeState(key, value);
      }
    }));
    
    // Memory tools
    if (this.memoryManager) {
      this.tools.push(new Tool({
        name: 'store_memory',
        description: 'Store information in long-term memory',
        parameters: {
          content: { type: 'string', required: true },
          metadata: { type: 'object', required: false }
        },
        handler: async ({ content, metadata }: { content: string; metadata?: any }) => {
          return this.storeMemory(content, metadata);
        }
      }));
      
      this.tools.push(new Tool({
        name: 'search_memory',
        description: 'Search for relevant memories',
        parameters: {
          query: { type: 'string', required: true },
          limit: { type: 'number', required: false }
        },
        handler: async ({ query, limit }: { query: string; limit?: number }) => {
          return this.searchMemory(query, limit);
        }
      }));
    }
    
    // Performance tracking
    this.tools.push(new Tool({
      name: 'log_performance',
      description: 'Log performance metrics',
      parameters: {
        metric: { type: 'string', required: true },
        value: { type: 'number', required: true },
        metadata: { type: 'object', required: false }
      },
      handler: async (params: any) => {
        return this.logPerformance(params);
      }
    }));
  }
  
  /**
   * Read from shared session state
   */
  protected async readState(key: string): Promise<any> {
    if (!this.sessionState) {
      throw new Error('Session state not initialized');
    }
    
    const value = await this.sessionState.get(key);
    this.logger.debug(`Read state: ${key} = ${JSON.stringify(value)}`);
    return value;
  }
  
  /**
   * Write to shared session state
   */
  protected async writeState(key: string, value: any): Promise<void> {
    if (!this.sessionState) {
      throw new Error('Session state not initialized');
    }
    
    await this.sessionState.set(key, value);
    this.logger.debug(`Wrote state: ${key} = ${JSON.stringify(value)}`);
    
    // Emit state change event
    this.eventEmitter.emit('state-changed', { key, value, agentId: this.id });
  }
  
  /**
   * Store in memory
   */
  protected async storeMemory(content: string, metadata?: any): Promise<string> {
    if (!this.memoryManager) {
      throw new Error('Memory manager not initialized');
    }
    
    const memoryId = await this.memoryManager.store({
      content,
      metadata: {
        ...metadata,
        agentId: this.id,
        specialty: this.specialty,
        timestamp: new Date()
      }
    });
    
    this.logger.debug(`Stored memory: ${memoryId}`);
    return memoryId;
  }
  
  /**
   * Search memories
   */
  protected async searchMemory(query: string, limit: number = 5): Promise<any[]> {
    if (!this.memoryManager) {
      return [];
    }
    
    const results = await this.memoryManager.search({
      query,
      limit,
      filters: {
        agentId: this.id
      }
    });
    
    this.logger.debug(`Found ${results.length} memories for query: ${query}`);
    return results;
  }
  
  /**
   * Log performance metrics
   */
  protected logPerformance(params: any): void {
    const metric = {
      ...params,
      agentId: this.id,
      timestamp: new Date()
    };
    
    this.performanceHistory.push(metric);
    this.eventEmitter.emit('performance-logged', metric);
  }
  
  /**
   * Convert this agent to an AgentTool for use by other agents
   */
  public asAgentTool(): AgentTool {
    return new AgentTool({
      agent: this,
      name: `${this.id}_tool`,
      description: `Use ${this.name} agent for ${this.specialty} tasks`
    });
  }
  
  /**
   * Execute a Hivemind task
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      // Store task context in session state
      await this.writeState(`task_${task.id}_context`, task.context);
      
      // Get relevant memories
      const memories = await this.searchMemory(task.description, 5);
      
      // Build execution prompt
      const prompt = this.buildTaskPrompt(task, memories);
      
      // Execute with ADK
      const result = await this.run(prompt, {
        task_id: task.id,
        task_type: task.type,
        files: task.context.files
      });
      
      // Process result
      const taskResult: TaskResult = {
        taskId: task.id,
        success: true,
        output: result,
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: result.usage?.total_tokens || 0,
          cost: result.usage?.total_cost || 0
        }
      };
      
      // Store result in memory
      await this.storeMemory(
        JSON.stringify({ task, result: taskResult }),
        { type: 'task_result', success: true }
      );
      
      // Store result in session state
      await this.writeState(`task_${task.id}_result`, taskResult);
      
      // Log performance
      this.logPerformance({
        metric: 'task_duration',
        value: taskResult.performanceMetrics.duration,
        metadata: { taskId: task.id, taskType: task.type }
      });
      
      return taskResult;
      
    } catch (error) {
      this.logger.error(`Task execution failed: ${error}`);
      
      const taskResult: TaskResult = {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Execution error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
      
      // Store failure in memory
      await this.storeMemory(
        JSON.stringify({ task, result: taskResult, error: error.message }),
        { type: 'task_result', success: false }
      );
      
      return taskResult;
    }
  }
  
  /**
   * Build task prompt with context
   */
  protected abstract buildTaskPrompt(task: HivemindTask, memories: any[]): string;
  
  /**
   * Get performance history
   */
  public getPerformanceHistory(): any[] {
    return [...this.performanceHistory];
  }
  
  /**
   * Reset agent state
   */
  public async reset(): Promise<void> {
    this.performanceHistory = [];
    this.logger.info('Agent reset');
  }
  
  /**
   * Subscribe to agent events
   */
  public on(event: string, handler: Function): void {
    this.eventEmitter.on(event, handler as any);
  }
  
  /**
   * Unsubscribe from agent events
   */
  public off(event: string, handler: Function): void {
    this.eventEmitter.off(event, handler as any);
  }
}