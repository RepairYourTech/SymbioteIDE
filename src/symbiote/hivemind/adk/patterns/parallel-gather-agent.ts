/**
 * Parallel Gather Agent Pattern
 * 
 * Executes multiple agents in parallel and gathers/aggregates results
 */

import { ParallelAgent, Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty } from '../../types';
import { Logger } from '../../../utils/logger';

export interface ParallelTask {
  agentId: string;
  agent: ADKHivemindAgent;
  task: Partial<HivemindTask>;
  weight?: number; // Importance weight for aggregation
}

export interface GatherStrategy {
  type: 'all' | 'first' | 'best' | 'weighted' | 'custom';
  evaluator?: (results: TaskResult[]) => TaskResult;
  successThreshold?: number; // Percentage of agents that must succeed
}

export interface ParallelGatherConfig extends ADKHivemindConfig {
  parallelTasks: ParallelTask[];
  gatherStrategy: GatherStrategy;
  maxConcurrency?: number;
  timeout?: number;
  isolateState?: boolean; // Use separate state keys per agent
}

export class ParallelGatherAgent extends ADKHivemindAgent {
  private parallelTasks: ParallelTask[];
  private gatherStrategy: GatherStrategy;
  private maxConcurrency: number;
  private timeout: number;
  private isolateState: boolean;
  private executionMetrics: any[] = [];
  
  constructor(config: ParallelGatherConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are a parallel execution coordinator that runs multiple agents concurrently and gathers their results.
      
Parallel agents:
${config.parallelTasks.map(task => 
  `- ${task.agentId} (${task.agent.name}): ${task.agent.capabilities.join(', ')}`
).join('\n')}

Your role is to:
1. Execute agents in parallel efficiently
2. Monitor progress and handle failures
3. Gather and aggregate results
4. Provide unified output`
    });
    
    this.parallelTasks = config.parallelTasks;
    this.gatherStrategy = config.gatherStrategy;
    this.maxConcurrency = config.maxConcurrency || 5;
    this.timeout = config.timeout || 300000; // 5 minutes default
    this.isolateState = config.isolateState ?? true;
    
    // Add parallel execution tools
    this.addParallelTools();
  }
  
  /**
   * Add parallel-specific tools
   */
  private addParallelTools(): void {
    this.tools.push(new Tool({
      name: 'execute_parallel_batch',
      description: 'Execute a batch of tasks in parallel',
      parameters: {
        taskIds: { type: 'array', required: true },
        maxConcurrency: { type: 'number', required: false }
      },
      handler: async (params: any) => {
        return this.executeBatch(params.taskIds, params.maxConcurrency);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'gather_results',
      description: 'Gather and aggregate parallel execution results',
      parameters: {
        results: { type: 'array', required: true },
        strategy: { type: 'string', required: false }
      },
      handler: async (params: any) => {
        return this.gatherResults(params.results, params.strategy);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'monitor_progress',
      description: 'Monitor parallel execution progress',
      parameters: {},
      handler: async () => {
        return this.getExecutionProgress();
      }
    }));
  }
  
  /**
   * Execute tasks in parallel
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    const executionId = `parallel_${task.id}`;
    
    try {
      // Initialize execution state
      const executionState = {
        taskId: task.id,
        startTime: new Date(),
        tasks: this.parallelTasks.map(pt => ({
          agentId: pt.agentId,
          status: 'pending',
          startTime: null,
          endTime: null,
          result: null
        }))
      };
      
      await this.writeState(executionId, executionState);
      
      // Prepare tasks for parallel execution
      const preparedTasks = this.parallelTasks.map(pt => ({
        ...pt,
        task: {
          ...task,
          ...pt.task,
          id: `${task.id}_${pt.agentId}`,
          context: {
            ...task.context,
            ...pt.task.context,
            parallelExecutionId: executionId,
            stateKeyPrefix: this.isolateState ? `${executionId}_${pt.agentId}` : undefined
          }
        }
      }));
      
      // Execute in batches based on max concurrency
      const results = await this.executeInBatches(preparedTasks, executionState, executionId);
      
      // Gather results according to strategy
      const gatheredResult = await this.applyGatherStrategy(results, task);
      
      // Store execution metrics
      this.executionMetrics.push({
        taskId: task.id,
        executionId,
        duration: Date.now() - startTime,
        parallelTasks: this.parallelTasks.length,
        successRate: results.filter(r => r.success).length / results.length,
        strategy: this.gatherStrategy.type
      });
      
      return gatheredResult;
      
    } catch (error) {
      this.logger.error(`Parallel execution failed`, error);
      
      return {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Parallel execution error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
    }
  }
  
  /**
   * Execute tasks in batches
   */
  private async executeInBatches(
    tasks: any[],
    executionState: any,
    executionId: string
  ): Promise<TaskResult[]> {
    const results: TaskResult[] = [];
    
    for (let i = 0; i < tasks.length; i += this.maxConcurrency) {
      const batch = tasks.slice(i, i + this.maxConcurrency);
      
      const batchPromises = batch.map(async (pt) => {
        const taskIndex = tasks.indexOf(pt);
        
        // Update state
        executionState.tasks[taskIndex].status = 'running';
        executionState.tasks[taskIndex].startTime = new Date();
        await this.writeState(executionId, executionState);
        
        try {
          // Add timeout wrapper
          const timeoutPromise = new Promise<TaskResult>((_, reject) => {
            setTimeout(() => reject(new Error('Task timeout')), this.timeout);
          });
          
          const executionPromise = pt.agent.executeTask(pt.task);
          const result = await Promise.race([executionPromise, timeoutPromise]);
          
          // Update state
          executionState.tasks[taskIndex].status = 'completed';
          executionState.tasks[taskIndex].endTime = new Date();
          executionState.tasks[taskIndex].result = result;
          await this.writeState(executionId, executionState);
          
          return result;
          
        } catch (error) {
          // Update state
          executionState.tasks[taskIndex].status = 'failed';
          executionState.tasks[taskIndex].endTime = new Date();
          executionState.tasks[taskIndex].error = error.message;
          await this.writeState(executionId, executionState);
          
          // Return failed result
          return {
            taskId: pt.task.id,
            success: false,
            output: { error: error.message },
            filesModified: [],
            filesCreated: [],
            testsAdded: 0,
            issuesFound: [`Agent ${pt.agentId} failed: ${error.message}`],
            performanceMetrics: {
              duration: Date.now() - executionState.tasks[taskIndex].startTime,
              tokensUsed: 0,
              cost: 0
            }
          };
        }
      });
      
      const batchResults = await Promise.all(batchPromises);
      results.push(...batchResults);
      
      this.logger.info(`Completed batch ${Math.floor(i / this.maxConcurrency) + 1}`);
    }
    
    return results;
  }
  
  /**
   * Apply gather strategy to results
   */
  private async applyGatherStrategy(
    results: TaskResult[],
    originalTask: HivemindTask
  ): Promise<TaskResult> {
    switch (this.gatherStrategy.type) {
      case 'all':
        return this.gatherAll(results, originalTask);
        
      case 'first':
        return this.gatherFirst(results, originalTask);
        
      case 'best':
        return this.gatherBest(results, originalTask);
        
      case 'weighted':
        return this.gatherWeighted(results, originalTask);
        
      case 'custom':
        if (this.gatherStrategy.evaluator) {
          return this.gatherStrategy.evaluator(results);
        }
        return this.gatherAll(results, originalTask);
        
      default:
        return this.gatherAll(results, originalTask);
    }
  }
  
  /**
   * Gather all results
   */
  private gatherAll(results: TaskResult[], originalTask: HivemindTask): TaskResult {
    const successCount = results.filter(r => r.success).length;
    const threshold = this.gatherStrategy.successThreshold || 0.8;
    const success = successCount / results.length >= threshold;
    
    const gathered: TaskResult = {
      taskId: originalTask.id,
      success,
      output: {
        strategy: 'all',
        successRate: successCount / results.length,
        agentResults: results.map((r, i) => ({
          agentId: this.parallelTasks[i].agentId,
          success: r.success,
          output: r.output
        }))
      },
      filesModified: [],
      filesCreated: [],
      testsAdded: 0,
      issuesFound: [],
      performanceMetrics: {
        duration: Math.max(...results.map(r => r.performanceMetrics.duration)),
        tokensUsed: results.reduce((sum, r) => sum + r.performanceMetrics.tokensUsed, 0),
        cost: results.reduce((sum, r) => sum + r.performanceMetrics.cost, 0)
      }
    };
    
    // Aggregate file changes and issues
    results.forEach(result => {
      gathered.filesModified.push(...result.filesModified);
      gathered.filesCreated.push(...result.filesCreated);
      gathered.testsAdded += result.testsAdded;
      gathered.issuesFound.push(...result.issuesFound);
    });
    
    // Remove duplicates
    gathered.filesModified = [...new Set(gathered.filesModified)];
    gathered.filesCreated = [...new Set(gathered.filesCreated)];
    
    return gathered;
  }
  
  /**
   * Return first successful result
   */
  private gatherFirst(results: TaskResult[], originalTask: HivemindTask): TaskResult {
    const firstSuccess = results.find(r => r.success);
    
    if (firstSuccess) {
      return {
        ...firstSuccess,
        taskId: originalTask.id,
        output: {
          strategy: 'first',
          selectedAgent: this.parallelTasks[results.indexOf(firstSuccess)].agentId,
          ...firstSuccess.output
        }
      };
    }
    
    // All failed - return aggregated failures
    return this.gatherAll(results, originalTask);
  }
  
  /**
   * Select best result based on metrics
   */
  private gatherBest(results: TaskResult[], originalTask: HivemindTask): TaskResult {
    // Simple scoring: success + fewer issues + more tests
    const scores = results.map((r, i) => ({
      index: i,
      result: r,
      score: (r.success ? 100 : 0) + 
             r.testsAdded - 
             r.issuesFound.length +
             (r.filesCreated.length + r.filesModified.length) * 0.1
    }));
    
    scores.sort((a, b) => b.score - a.score);
    const best = scores[0];
    
    return {
      ...best.result,
      taskId: originalTask.id,
      output: {
        strategy: 'best',
        selectedAgent: this.parallelTasks[best.index].agentId,
        selectionScore: best.score,
        ...best.result.output
      }
    };
  }
  
  /**
   * Weighted aggregation of results
   */
  private gatherWeighted(results: TaskResult[], originalTask: HivemindTask): TaskResult {
    const weights = this.parallelTasks.map(pt => pt.weight || 1);
    const totalWeight = weights.reduce((sum, w) => sum + w, 0);
    
    let weightedScore = 0;
    const weightedOutputs: any[] = [];
    
    results.forEach((result, i) => {
      const weight = weights[i] / totalWeight;
      weightedScore += (result.success ? 1 : 0) * weight;
      
      if (result.success) {
        weightedOutputs.push({
          agentId: this.parallelTasks[i].agentId,
          weight: weights[i],
          output: result.output
        });
      }
    });
    
    return {
      taskId: originalTask.id,
      success: weightedScore >= 0.5,
      output: {
        strategy: 'weighted',
        weightedScore,
        weightedOutputs
      },
      filesModified: [],
      filesCreated: [],
      testsAdded: results.reduce((sum, r) => sum + r.testsAdded, 0),
      issuesFound: [...new Set(results.flatMap(r => r.issuesFound))],
      performanceMetrics: {
        duration: Math.max(...results.map(r => r.performanceMetrics.duration)),
        tokensUsed: results.reduce((sum, r) => sum + r.performanceMetrics.tokensUsed, 0),
        cost: results.reduce((sum, r) => sum + r.performanceMetrics.cost, 0)
      }
    };
  }
  
  /**
   * Execute specific batch (tool function)
   */
  private async executeBatch(taskIds: string[], maxConcurrency?: number): Promise<any> {
    const tasks = taskIds.map(id => 
      this.parallelTasks.find(pt => pt.agentId === id)
    ).filter(Boolean);
    
    if (tasks.length === 0) {
      throw new Error('No valid tasks found');
    }
    
    const oldMaxConcurrency = this.maxConcurrency;
    if (maxConcurrency) {
      this.maxConcurrency = maxConcurrency;
    }
    
    const results = await this.executeInBatches(tasks as any[], {}, `batch_${Date.now()}`);
    
    this.maxConcurrency = oldMaxConcurrency;
    
    return results;
  }
  
  /**
   * Gather results (tool function)
   */
  private async gatherResults(results: TaskResult[], strategy?: string): Promise<any> {
    const originalStrategy = this.gatherStrategy.type;
    if (strategy) {
      this.gatherStrategy.type = strategy as any;
    }
    
    const gathered = await this.applyGatherStrategy(results, {
      id: 'gather_task',
      type: 'gather',
      title: 'Gather Results',
      description: 'Gathering parallel execution results',
      requiredSpecialties: [],
      priority: 'medium',
      status: 'pending',
      dependencies: [],
      assignedAgents: [],
      estimatedComplexity: 1,
      context: {},
      createdAt: new Date()
    });
    
    this.gatherStrategy.type = originalStrategy;
    
    return gathered;
  }
  
  /**
   * Get execution progress
   */
  private async getExecutionProgress(): Promise<any> {
    const states = await this.sessionState?.getKeys(/^parallel_/);
    const progress: any[] = [];
    
    if (states) {
      for (const key of states) {
        const state = await this.sessionState?.get(key);
        if (state && state.tasks) {
          const completed = state.tasks.filter((t: any) => 
            t.status === 'completed' || t.status === 'failed'
          ).length;
          
          progress.push({
            executionId: key,
            totalTasks: state.tasks.length,
            completed,
            percentage: (completed / state.tasks.length) * 100
          });
        }
      }
    }
    
    return progress;
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    return `Execute this task across multiple specialized agents in parallel:

Task: ${task.title}
Description: ${task.description}

Coordinate parallel execution and gather results according to the ${this.gatherStrategy.type} strategy.`;
  }
  
  /**
   * Get execution metrics
   */
  public getExecutionMetrics(): any[] {
    return [...this.executionMetrics];
  }
}