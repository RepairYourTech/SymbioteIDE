/**
 * Agent Task Processor - Handles AI agent task execution jobs
 */

import { Job } from 'bullmq';
import { BaseProcessor } from './base-processor';
import { 
  AgentTaskJobData, 
  JobResult, 
  JobType,
  QueueName 
} from '../types';
import { OrchestrationEngine } from '../../orchestration/orchestration-engine';
import { WebSocketServer } from '../../websocket/websocket-server';
import { RedisManager } from '../../redis/redis-manager';
import { MemoryManager } from '../../memory/mem0/memory-manager';

export class AgentTaskProcessor extends BaseProcessor<AgentTaskJobData> {
  private orchestrator?: OrchestrationEngine;
  private wsServer?: WebSocketServer;
  private redisManager?: RedisManager;
  private memoryManager?: MemoryManager;

  constructor(queueManager: any) {
    super(queueManager, 'AgentTaskProcessor');
  }

  /**
   * Initialize services
   */
  private async initializeServices(): Promise<void> {
    if (!this.orchestrator) {
      this.orchestrator = new OrchestrationEngine();
      await this.orchestrator.initialize();
      
      this.wsServer = WebSocketServer.getInstance();
      this.redisManager = RedisManager.getInstance();
      
      // Initialize memory manager if needed
      const { QdrantManager } = await import('../../search/qdrant/qdrant-manager');
      const qdrantManager = new QdrantManager({
        url: process.env.QDRANT_URL || 'http://localhost:6333',
        apiKey: process.env.QDRANT_API_KEY
      });
      await qdrantManager.initialize();
      
      this.memoryManager = new MemoryManager({
        qdrantManager
      });
      await this.memoryManager.initialize();
    }
  }

  /**
   * Process agent task job
   */
  async process(job: Job<AgentTaskJobData>): Promise<JobResult> {
    try {
      this.validateJobData(job.data);
      await this.initializeServices();

      switch (job.data.type) {
        case JobType.ExecuteAgentTask:
          return await this.executeAgentTask(job);
          
        case JobType.AgentWorkflow:
          return await this.executeAgentWorkflow(job);
          
        default:
          throw new Error(`Unknown job type: ${job.data.type}`);
      }
    } catch (error: any) {
      return this.handleFailure(job, error);
    }
  }

  /**
   * Execute single agent task
   */
  private async executeAgentTask(job: Job<AgentTaskJobData>): Promise<JobResult> {
    const { agentId, taskId, input, context } = job.data;
    
    await this.updateProgress(job, 0, 'Initializing agent task');
    
    // Create execution context
    const executionContext = {
      jobId: job.id,
      agentId,
      taskId,
      userId: context?.userId,
      sessionId: context?.sessionId || `job_${job.id}`,
      priority: context?.priority || 1
    };
    
    // Notify via WebSocket
    if (this.wsServer && context?.userId) {
      this.wsServer.sendToNamespace('agents', 'task:started', {
        agentId,
        taskId,
        jobId: job.id,
        timestamp: Date.now()
      });
    }
    
    await this.updateProgress(job, 10, 'Loading agent configuration');
    
    // Load agent configuration
    const agentConfig = await this.loadAgentConfig(agentId);
    if (!agentConfig) {
      return this.createErrorResult(
        'AGENT_NOT_FOUND',
        `Agent ${agentId} not found`
      );
    }
    
    await this.updateProgress(job, 20, 'Loading agent memory');
    
    // Load agent memory context
    const memories = await this.memoryManager!.search('', {
      filter: { agentId },
      limit: 10
    });
    
    await this.updateProgress(job, 30, 'Preparing task');
    
    // Build AI task
    const aiTask = {
      id: taskId,
      type: agentConfig.capabilities[0] || 'general',
      prompt: this.buildPrompt(input, agentConfig, memories),
      context: {
        agentId,
        memories: memories.map(m => m.content),
        systemPrompt: agentConfig.systemPrompt,
        ...input.context
      },
      parameters: {
        temperature: agentConfig.temperature || 0.7,
        maxTokens: agentConfig.maxTokens || 2000,
        ...input.parameters
      },
      metadata: executionContext
    };
    
    await this.updateProgress(job, 40, 'Executing task');
    
    // Execute via orchestrator
    const result = await this.orchestrator!.executeTask(aiTask, {
      stream: false,
      callbacks: {
        onProgress: (progress) => {
          this.updateProgress(
            job,
            40 + (progress.progress || 0) * 40,
            progress.message
          );
        }
      }
    });
    
    await this.updateProgress(job, 80, 'Storing results');
    
    // Store execution result in memory
    await this.memoryManager!.store({
      content: `Task ${taskId}: ${input.prompt}\nResponse: ${result.response}`,
      metadata: {
        type: 'task_execution',
        agentId,
        taskId,
        jobId: job.id,
        userId: context?.userId,
        timestamp: new Date().toISOString(),
        success: result.status === 'success'
      }
    });
    
    await this.updateProgress(job, 90, 'Updating metrics');
    
    // Update agent metrics
    await this.updateAgentMetrics(agentId, {
      tasksExecuted: 1,
      lastExecution: new Date().toISOString(),
      successRate: result.status === 'success' ? 1 : 0
    });
    
    // Notify completion
    if (this.wsServer && context?.userId) {
      this.wsServer.sendToNamespace('agents', 'task:completed', {
        agentId,
        taskId,
        jobId: job.id,
        result: {
          status: result.status,
          response: result.response
        },
        timestamp: Date.now()
      });
    }
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      agentId,
      taskId,
      result: {
        response: result.response,
        model: result.model,
        usage: result.usage
      },
      executionTime: result.latency
    });
  }

  /**
   * Execute agent workflow
   */
  private async executeAgentWorkflow(job: Job<AgentTaskJobData>): Promise<JobResult> {
    const { agentId, taskId, input } = job.data;
    
    await this.updateProgress(job, 0, 'Loading workflow definition');
    
    // Load workflow
    const workflow = await this.loadWorkflow(taskId);
    if (!workflow) {
      return this.createErrorResult(
        'WORKFLOW_NOT_FOUND',
        `Workflow ${taskId} not found`
      );
    }
    
    const steps = workflow.steps;
    const totalSteps = steps.length;
    const results: any[] = [];
    
    await this.updateProgress(job, 10, `Executing ${totalSteps} workflow steps`);
    
    // Execute each step
    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      const stepProgress = 10 + (80 * i / totalSteps);
      
      await this.updateProgress(
        job,
        stepProgress,
        `Executing step ${i + 1}/${totalSteps}: ${step.name}`,
        { currentStep: i + 1, totalSteps }
      );
      
      try {
        // Prepare step input
        const stepInput = this.prepareStepInput(step, input, results);
        
        // Create sub-job for step
        const subJob = await this.queueManager.addJob(
          QueueName.AgentTasks,
          `workflow-step-${step.id}`,
          {
            type: JobType.ExecuteAgentTask,
            agentId: step.agentId || agentId,
            taskId: step.id,
            input: stepInput,
            context: {
              ...job.data.context,
              workflowId: taskId,
              stepIndex: i
            }
          },
          {
            priority: job.opts.priority
          }
        );
        
        // Wait for sub-job completion
        const stepResult = await this.waitForJob(subJob);
        results.push({
          stepId: step.id,
          stepName: step.name,
          result: stepResult
        });
        
        // Check if should continue
        if (step.onError === 'stop' && !stepResult.success) {
          this.logger.warn(`Workflow stopped at step ${i + 1} due to error`);
          break;
        }
        
      } catch (error: any) {
        this.logger.error(`Step ${step.id} failed`, error);
        
        if (step.onError === 'stop') {
          throw error;
        } else if (step.onError === 'skip') {
          results.push({
            stepId: step.id,
            stepName: step.name,
            error: error.message
          });
        }
      }
    }
    
    await this.updateProgress(job, 90, 'Aggregating results');
    
    // Aggregate workflow results
    const aggregatedResult = this.aggregateWorkflowResults(workflow, results);
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      workflowId: taskId,
      steps: results,
      aggregated: aggregatedResult,
      completedSteps: results.filter(r => r.result?.success).length,
      totalSteps
    });
  }

  /**
   * Load agent configuration
   */
  private async loadAgentConfig(agentId: string): Promise<any> {
    // Try cache first
    const cached = await this.redisManager!.get(`agent:config:${agentId}`);
    if (cached) return cached;
    
    // Mock agent configurations
    const agents: { [key: string]: any } = {
      'code-reviewer': {
        id: 'code-reviewer',
        name: 'Code Review Agent',
        systemPrompt: 'You are an expert code reviewer. Analyze code for bugs, security issues, and improvements.',
        capabilities: ['code-review', 'security-analysis'],
        temperature: 0.3,
        maxTokens: 3000
      },
      'test-generator': {
        id: 'test-generator',
        name: 'Test Generator Agent',
        systemPrompt: 'You are an expert at writing comprehensive test suites.',
        capabilities: ['testing', 'code-generation'],
        temperature: 0.5,
        maxTokens: 4000
      },
      'doc-writer': {
        id: 'doc-writer',
        name: 'Documentation Agent',
        systemPrompt: 'You are a technical writer. Create clear and comprehensive documentation.',
        capabilities: ['documentation'],
        temperature: 0.7,
        maxTokens: 2000
      }
    };
    
    const config = agents[agentId];
    
    if (config) {
      // Cache for future use
      await this.redisManager!.set(`agent:config:${agentId}`, config, 3600);
    }
    
    return config;
  }

  /**
   * Load workflow definition
   */
  private async loadWorkflow(workflowId: string): Promise<any> {
    // Mock workflow
    const workflows: { [key: string]: any } = {
      'code-review-workflow': {
        id: 'code-review-workflow',
        name: 'Complete Code Review',
        steps: [
          {
            id: 'analyze-code',
            name: 'Analyze Code Structure',
            agentId: 'code-reviewer',
            onError: 'stop'
          },
          {
            id: 'security-check',
            name: 'Security Analysis',
            agentId: 'code-reviewer',
            onError: 'continue'
          },
          {
            id: 'generate-tests',
            name: 'Generate Test Suggestions',
            agentId: 'test-generator',
            onError: 'skip'
          },
          {
            id: 'create-docs',
            name: 'Generate Documentation',
            agentId: 'doc-writer',
            onError: 'skip'
          }
        ]
      }
    };
    
    return workflows[workflowId];
  }

  /**
   * Build prompt for agent
   */
  private buildPrompt(
    input: any,
    agentConfig: any,
    memories: any[]
  ): string {
    let prompt = input.prompt || '';
    
    // Add memory context
    if (memories.length > 0) {
      prompt = `Previous context:\n${memories.map(m => m.content).join('\n')}\n\nCurrent task: ${prompt}`;
    }
    
    // Add agent-specific instructions
    if (agentConfig.instructions) {
      prompt = `${agentConfig.instructions}\n\n${prompt}`;
    }
    
    return prompt;
  }

  /**
   * Prepare step input
   */
  private prepareStepInput(
    step: any,
    originalInput: any,
    previousResults: any[]
  ): any {
    const input = { ...originalInput };
    
    // Add previous step results if needed
    if (step.usePreviousResults && previousResults.length > 0) {
      const lastResult = previousResults[previousResults.length - 1];
      input.previousResult = lastResult.result;
      input.context = {
        ...input.context,
        previousStepId: lastResult.stepId,
        previousStepOutput: lastResult.result?.response
      };
    }
    
    // Override with step-specific input
    if (step.input) {
      Object.assign(input, step.input);
    }
    
    return input;
  }

  /**
   * Wait for job completion
   */
  private async waitForJob(job: Job): Promise<any> {
    const maxWaitTime = 300000; // 5 minutes
    const checkInterval = 1000; // 1 second
    const startTime = Date.now();
    
    while (Date.now() - startTime < maxWaitTime) {
      const jobState = await job.getState();
      
      if (jobState === 'completed') {
        return job.returnvalue;
      } else if (jobState === 'failed') {
        throw new Error(`Sub-job ${job.id} failed: ${job.failedReason}`);
      }
      
      await new Promise(resolve => setTimeout(resolve, checkInterval));
    }
    
    throw new Error(`Sub-job ${job.id} timed out`);
  }

  /**
   * Aggregate workflow results
   */
  private aggregateWorkflowResults(workflow: any, results: any[]): any {
    const successful = results.filter(r => r.result?.success);
    const failed = results.filter(r => !r.result?.success);
    
    return {
      success: failed.length === 0,
      completedSteps: successful.length,
      failedSteps: failed.length,
      outputs: results.map(r => ({
        stepId: r.stepId,
        output: r.result?.response || r.error
      }))
    };
  }

  /**
   * Update agent metrics
   */
  private async updateAgentMetrics(agentId: string, metrics: any): Promise<void> {
    const key = `agent:metrics:${agentId}`;
    
    // Increment counters
    if (metrics.tasksExecuted) {
      await this.redisManager!.hincrby(key, 'tasksExecuted', metrics.tasksExecuted);
    }
    
    // Update fields
    const updates: { [key: string]: string } = {};
    if (metrics.lastExecution) updates.lastExecution = metrics.lastExecution;
    
    if (Object.keys(updates).length > 0) {
      await this.redisManager!.hset(key, updates);
    }
    
    // Update success rate (moving average)
    if (metrics.successRate !== undefined) {
      const currentRate = await this.redisManager!.hget(key, 'successRate');
      const currentCount = await this.redisManager!.hget(key, 'tasksExecuted');
      
      if (currentRate && currentCount) {
        const newRate = (parseFloat(currentRate) * (parseInt(currentCount) - 1) + metrics.successRate) / parseInt(currentCount);
        await this.redisManager!.hset(key, { successRate: newRate.toString() });
      } else {
        await this.redisManager!.hset(key, { successRate: metrics.successRate.toString() });
      }
    }
  }

  /**
   * Validate job data
   */
  protected validateJobData(data: AgentTaskJobData): void {
    if (!data.agentId) {
      throw new Error('Agent ID is required');
    }
    
    if (!data.taskId) {
      throw new Error('Task ID is required');
    }
    
    if (!data.input) {
      throw new Error('Input is required');
    }
    
    if (!data.type) {
      throw new Error('Job type is required');
    }
  }
}