/**
 * Base Specialist Agent
 * 
 * Base class for all specialist agents in the Hivemind system
 */

import { Agent, LlmAgent } from '@google/adk';
import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import { MemoryManager } from '../../memory/mem0/memory-manager';
import {
  AgentSpecialty,
  AgentStatus,
  AgentPerformance,
  HivemindTask,
  TaskResult,
  AgentMessage,
  MessageType
} from '../types';

export abstract class BaseSpecialistAgent extends EventEmitter {
  protected logger: Logger;
  protected agent?: Agent;
  protected memoryManager?: MemoryManager;
  
  public readonly id: string;
  public readonly name: string;
  public readonly specialty: AgentSpecialty;
  public readonly capabilities: string[];
  
  protected _status: AgentStatus = AgentStatus.Offline;
  protected currentTask?: HivemindTask;
  protected performance: AgentPerformance;
  
  constructor(
    id: string,
    name: string,
    specialty: AgentSpecialty,
    capabilities: string[]
  ) {
    super();
    
    this.id = id;
    this.name = name;
    this.specialty = specialty;
    this.capabilities = capabilities;
    
    this.logger = new Logger(`SpecialistAgent:${name}`);
    
    this.performance = {
      tasksCompleted: 0,
      successRate: 1.0,
      averageExecutionTime: 0,
      specialtyScore: 80, // Start with good score
      lastActive: new Date()
    };
  }
  
  /**
   * Initialize the specialist agent
   */
  async initialize(
    agent: Agent,
    memoryManager?: MemoryManager
  ): Promise<void> {
    this.agent = agent;
    this.memoryManager = memoryManager;
    
    // Set up system prompt with specialty
    await this.configureAgent();
    
    this._status = AgentStatus.Available;
    this.emit('status-changed', this._status);
    
    this.logger.info(`${this.name} initialized and ready`);
  }
  
  /**
   * Get current status
   */
  get status(): AgentStatus {
    return this._status;
  }
  
  /**
   * Execute a task
   */
  async executeTask(task: HivemindTask): Promise<TaskResult> {
    if (this._status !== AgentStatus.Available) {
      throw new Error(`Agent ${this.name} is not available`);
    }
    
    this._status = AgentStatus.Busy;
    this.currentTask = task;
    this.emit('status-changed', this._status);
    
    const startTime = Date.now();
    
    try {
      this.logger.info(`Starting task: ${task.title}`);
      
      // Load relevant memories
      const memories = await this.loadRelevantMemories(task);
      
      // Execute task with specialty-specific logic
      const result = await this.performTask(task, memories);
      
      // Store execution in memory
      await this.storeTaskMemory(task, result);
      
      // Update performance metrics
      this.updatePerformance(true, Date.now() - startTime);
      
      this._status = AgentStatus.Available;
      this.currentTask = undefined;
      this.emit('status-changed', this._status);
      
      this.logger.info(`Completed task: ${task.title}`);
      
      return result;
      
    } catch (error) {
      this.logger.error(`Task failed: ${task.title}`, error);
      
      this.updatePerformance(false, Date.now() - startTime);
      
      this._status = AgentStatus.Available;
      this.currentTask = undefined;
      this.emit('status-changed', this._status);
      
      throw error;
    }
  }
  
  /**
   * Handle incoming message from other agents
   */
  async handleMessage(message: AgentMessage): Promise<any> {
    this.logger.debug(`Received message: ${message.type} from ${message.from}`);
    
    switch (message.type) {
      case MessageType.HelpRequest:
        return this.handleHelpRequest(message);
        
      case MessageType.ResultShare:
        return this.handleResultShare(message);
        
      case MessageType.StatusUpdate:
        this.emit('peer-status-update', message);
        break;
        
      case MessageType.Heartbeat:
        return { status: this._status, currentTask: this.currentTask?.id };
        
      default:
        this.logger.warn(`Unknown message type: ${message.type}`);
    }
  }
  
  /**
   * Get agent capabilities and current state
   */
  getCapabilities(): {
    specialty: AgentSpecialty;
    capabilities: string[];
    status: AgentStatus;
    performance: AgentPerformance;
  } {
    return {
      specialty: this.specialty,
      capabilities: this.capabilities,
      status: this._status,
      performance: this.performance
    };
  }
  
  /**
   * Abstract method - configure agent with specialty-specific prompts
   */
  protected abstract configureAgent(): Promise<void>;
  
  /**
   * Abstract method - perform the actual task execution
   */
  protected abstract performTask(
    task: HivemindTask,
    memories: any[]
  ): Promise<TaskResult>;
  
  /**
   * Load relevant memories for the task
   */
  protected async loadRelevantMemories(task: HivemindTask): Promise<any[]> {
    if (!this.memoryManager) return [];
    
    // Search for memories related to the task
    const query = `${task.type} ${task.title} ${this.specialty}`;
    const memories = await this.memoryManager.search(query, {
      filter: {
        agentId: this.id,
        type: 'task_execution'
      },
      limit: 5
    });
    
    return memories;
  }
  
  /**
   * Store task execution in memory
   */
  protected async storeTaskMemory(
    task: HivemindTask,
    result: TaskResult
  ): Promise<void> {
    if (!this.memoryManager) return;
    
    await this.memoryManager.store({
      content: JSON.stringify({
        task: {
          id: task.id,
          type: task.type,
          title: task.title
        },
        result: {
          success: result.success,
          summary: this.summarizeResult(result)
        },
        specialty: this.specialty
      }),
      metadata: {
        type: 'task_execution',
        agentId: this.id,
        taskId: task.id,
        timestamp: new Date().toISOString(),
        success: result.success
      }
    });
  }
  
  /**
   * Update performance metrics
   */
  protected updatePerformance(success: boolean, duration: number): void {
    const metrics = this.performance;
    
    // Update task count
    metrics.tasksCompleted++;
    
    // Update success rate (moving average)
    const totalTasks = metrics.tasksCompleted;
    const successCount = metrics.successRate * (totalTasks - 1) + (success ? 1 : 0);
    metrics.successRate = successCount / totalTasks;
    
    // Update average execution time
    const totalTime = metrics.averageExecutionTime * (totalTasks - 1) + duration;
    metrics.averageExecutionTime = totalTime / totalTasks;
    
    // Update specialty score based on success rate
    metrics.specialtyScore = Math.min(100, metrics.successRate * 100 + 10);
    
    // Update last active
    metrics.lastActive = new Date();
    
    this.emit('performance-updated', metrics);
  }
  
  /**
   * Handle help request from another agent
   */
  protected async handleHelpRequest(message: AgentMessage): Promise<any> {
    if (this._status !== AgentStatus.Available) {
      return { available: false, reason: 'busy' };
    }
    
    const { task, question } = message.payload;
    
    // Check if we can help with this task
    if (!this.canHelpWith(task)) {
      return { available: false, reason: 'not_qualified' };
    }
    
    try {
      // Provide assistance
      const assistance = await this.provideAssistance(task, question);
      return { available: true, assistance };
      
    } catch (error) {
      this.logger.error('Failed to provide assistance', error);
      return { available: false, reason: 'error', error: error.message };
    }
  }
  
  /**
   * Handle result sharing from another agent
   */
  protected async handleResultShare(message: AgentMessage): Promise<void> {
    const { task, result } = message.payload;
    
    // Store shared result in memory for future reference
    if (this.memoryManager) {
      await this.memoryManager.store({
        content: JSON.stringify({
          sharedBy: message.from,
          task,
          result
        }),
        metadata: {
          type: 'shared_result',
          fromAgent: message.from,
          taskId: task.id,
          timestamp: new Date().toISOString()
        }
      });
    }
    
    this.emit('result-shared', { from: message.from, task, result });
  }
  
  /**
   * Check if agent can help with a task
   */
  protected canHelpWith(task: HivemindTask): boolean {
    // Check if task requires our specialty
    if (task.requiredSpecialties.includes(this.specialty)) {
      return true;
    }
    
    // Check if we have any required capabilities
    return task.requiredSpecialties.some(specialty => 
      this.capabilities.includes(specialty)
    );
  }
  
  /**
   * Provide assistance for a task
   */
  protected async provideAssistance(
    task: HivemindTask,
    question: string
  ): Promise<any> {
    if (!this.agent) {
      throw new Error('Agent not initialized');
    }
    
    const prompt = `
As a ${this.specialty} specialist, provide assistance for the following:

Task: ${task.title}
Description: ${task.description}

Question: ${question}

Provide specific, actionable assistance based on your expertise.`;
    
    const response = await this.agent.execute({
      messages: [{ role: 'user', content: prompt }]
    });
    
    return response.output;
  }
  
  /**
   * Summarize task result
   */
  protected summarizeResult(result: TaskResult): string {
    const parts = [];
    
    if (result.filesModified.length > 0) {
      parts.push(`Modified ${result.filesModified.length} files`);
    }
    
    if (result.filesCreated.length > 0) {
      parts.push(`Created ${result.filesCreated.length} files`);
    }
    
    if (result.testsAdded > 0) {
      parts.push(`Added ${result.testsAdded} tests`);
    }
    
    if (result.issuesFound.length > 0) {
      parts.push(`Found ${result.issuesFound.length} issues`);
    }
    
    return parts.join(', ') || 'Task completed';
  }
  
  /**
   * Shutdown the agent
   */
  async shutdown(): Promise<void> {
    this._status = AgentStatus.Offline;
    this.currentTask = undefined;
    this.emit('status-changed', this._status);
    
    this.logger.info(`${this.name} shutting down`);
  }
}
