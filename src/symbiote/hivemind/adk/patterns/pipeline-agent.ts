/**
 * Pipeline Agent Pattern
 * 
 * Sequential execution of agents with state passing between steps
 */

import { SequentialAgent, Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty } from '../../types';
import { Logger } from '../../../utils/logger';

export interface PipelineStep {
  agentId: string;
  agent: ADKHivemindAgent;
  name: string;
  description?: string;
  inputTransform?: (previousOutput: any, state: any) => any;
  outputTransform?: (output: any) => any;
  skipCondition?: (state: any) => boolean;
}

export interface PipelineConfig extends ADKHivemindConfig {
  steps: PipelineStep[];
  continueOnError?: boolean;
  stateKeys?: string[]; // Keys to track in shared state
}

export class PipelineAgent extends ADKHivemindAgent {
  private steps: PipelineStep[];
  private continueOnError: boolean;
  private stateKeys: string[];
  private executionHistory: any[] = [];
  
  constructor(config: PipelineConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are a pipeline coordinator that executes tasks through a series of specialized agents in sequence.
      
Pipeline steps:
${config.steps.map((step, i) => 
  `${i + 1}. ${step.name} (${step.agent.name}): ${step.description || step.agent.capabilities.join(', ')}`
).join('\n')}

Your role is to:
1. Execute each step in order
2. Pass results between steps
3. Handle errors appropriately
4. Track progress and state`
    });
    
    this.steps = config.steps;
    this.continueOnError = config.continueOnError ?? false;
    this.stateKeys = config.stateKeys || ['pipeline_state'];
    
    // Add pipeline control tools
    this.addPipelineTools();
  }
  
  /**
   * Add pipeline-specific tools
   */
  private addPipelineTools(): void {
    this.tools.push(new Tool({
      name: 'execute_step',
      description: 'Execute a specific pipeline step',
      parameters: {
        stepIndex: { type: 'number', required: true },
        input: { type: 'any', required: true }
      },
      handler: async (params: any) => {
        return this.executeStep(params.stepIndex, params.input);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'skip_step',
      description: 'Skip a pipeline step',
      parameters: {
        stepIndex: { type: 'number', required: true },
        reason: { type: 'string', required: true }
      },
      handler: async (params: any) => {
        return this.skipStep(params.stepIndex, params.reason);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'retry_step',
      description: 'Retry a failed pipeline step',
      parameters: {
        stepIndex: { type: 'number', required: true },
        modifications: { type: 'object', required: false }
      },
      handler: async (params: any) => {
        return this.retryStep(params.stepIndex, params.modifications);
      }
    }));
  }
  
  /**
   * Execute the pipeline
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    const pipelineId = `pipeline_${task.id}`;
    
    try {
      // Initialize pipeline state
      const pipelineState = {
        taskId: task.id,
        currentStep: 0,
        steps: this.steps.map(s => ({
          name: s.name,
          status: 'pending',
          startTime: null,
          endTime: null,
          output: null,
          error: null
        })),
        finalOutput: null
      };
      
      await this.writeState(pipelineId, pipelineState);
      
      let currentInput = task;
      let accumulatedOutput: any = {};
      
      // Execute each step
      for (let i = 0; i < this.steps.length; i++) {
        const step = this.steps[i];
        pipelineState.currentStep = i;
        
        // Check skip condition
        if (step.skipCondition) {
          const shouldSkip = await step.skipCondition(pipelineState);
          if (shouldSkip) {
            this.logger.info(`Skipping step ${i}: ${step.name}`);
            pipelineState.steps[i].status = 'skipped';
            await this.writeState(pipelineId, pipelineState);
            continue;
          }
        }
        
        // Update state
        pipelineState.steps[i].status = 'running';
        pipelineState.steps[i].startTime = new Date();
        await this.writeState(pipelineId, pipelineState);
        
        try {
          // Transform input if needed
          let stepInput = currentInput;
          if (step.inputTransform) {
            stepInput = await step.inputTransform(currentInput, pipelineState);
          }
          
          // Execute step
          this.logger.info(`Executing pipeline step ${i}: ${step.name}`);
          const stepResult = await step.agent.executeTask(stepInput as HivemindTask);
          
          // Transform output if needed
          let stepOutput = stepResult.output;
          if (step.outputTransform) {
            stepOutput = await step.outputTransform(stepOutput);
          }
          
          // Update state
          pipelineState.steps[i].status = 'completed';
          pipelineState.steps[i].endTime = new Date();
          pipelineState.steps[i].output = stepOutput;
          await this.writeState(pipelineId, pipelineState);
          
          // Store step result
          await this.writeState(`${pipelineId}_step_${i}`, stepResult);
          
          // Accumulate output
          accumulatedOutput[step.name] = stepOutput;
          
          // Prepare input for next step
          currentInput = {
            ...task,
            context: {
              ...task.context,
              previousStep: step.name,
              previousOutput: stepOutput,
              pipelineState: pipelineState
            }
          };
          
        } catch (error) {
          this.logger.error(`Pipeline step ${i} failed`, error);
          
          pipelineState.steps[i].status = 'failed';
          pipelineState.steps[i].endTime = new Date();
          pipelineState.steps[i].error = error.message;
          await this.writeState(pipelineId, pipelineState);
          
          if (!this.continueOnError) {
            throw new Error(`Pipeline failed at step ${i}: ${step.name} - ${error.message}`);
          }
        }
      }
      
      // Create final result
      const successCount = pipelineState.steps.filter(s => s.status === 'completed').length;
      const failureCount = pipelineState.steps.filter(s => s.status === 'failed').length;
      
      const result: TaskResult = {
        taskId: task.id,
        success: failureCount === 0,
        output: {
          pipeline: pipelineState,
          stepOutputs: accumulatedOutput,
          summary: `Pipeline completed: ${successCount}/${this.steps.length} steps successful`
        },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
      
      // Collect metrics from all steps
      for (let i = 0; i < this.steps.length; i++) {
        const stepResult = await this.readState(`${pipelineId}_step_${i}`);
        if (stepResult) {
          result.filesModified.push(...(stepResult.filesModified || []));
          result.filesCreated.push(...(stepResult.filesCreated || []));
          result.testsAdded += stepResult.testsAdded || 0;
          result.issuesFound.push(...(stepResult.issuesFound || []));
          result.performanceMetrics.tokensUsed += stepResult.performanceMetrics?.tokensUsed || 0;
          result.performanceMetrics.cost += stepResult.performanceMetrics?.cost || 0;
        }
      }
      
      // Remove duplicates
      result.filesModified = [...new Set(result.filesModified)];
      result.filesCreated = [...new Set(result.filesCreated)];
      
      // Store execution in history
      this.executionHistory.push({
        taskId: task.id,
        pipelineId,
        startTime: new Date(startTime),
        endTime: new Date(),
        success: result.success,
        stepsCompleted: successCount
      });
      
      return result;
      
    } catch (error) {
      this.logger.error(`Pipeline execution failed`, error);
      
      return {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Pipeline error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
    }
  }
  
  /**
   * Execute a specific step
   */
  private async executeStep(stepIndex: number, input: any): Promise<any> {
    if (stepIndex >= this.steps.length) {
      throw new Error(`Invalid step index: ${stepIndex}`);
    }
    
    const step = this.steps[stepIndex];
    return step.agent.executeTask(input);
  }
  
  /**
   * Skip a step
   */
  private async skipStep(stepIndex: number, reason: string): Promise<any> {
    this.logger.info(`Skipping step ${stepIndex}: ${reason}`);
    
    const pipelineId = await this.readState('current_pipeline_id');
    if (pipelineId) {
      const pipelineState = await this.readState(pipelineId);
      if (pipelineState && pipelineState.steps[stepIndex]) {
        pipelineState.steps[stepIndex].status = 'skipped';
        pipelineState.steps[stepIndex].skipReason = reason;
        await this.writeState(pipelineId, pipelineState);
      }
    }
    
    return { skipped: true, reason };
  }
  
  /**
   * Retry a failed step
   */
  private async retryStep(stepIndex: number, modifications?: any): Promise<any> {
    if (stepIndex >= this.steps.length) {
      throw new Error(`Invalid step index: ${stepIndex}`);
    }
    
    const step = this.steps[stepIndex];
    const pipelineId = await this.readState('current_pipeline_id');
    
    if (!pipelineId) {
      throw new Error('No active pipeline');
    }
    
    const pipelineState = await this.readState(pipelineId);
    const previousInput = await this.readState(`${pipelineId}_step_${stepIndex}_input`);
    
    // Apply modifications to input if provided
    const retryInput = modifications ? { ...previousInput, ...modifications } : previousInput;
    
    // Execute step again
    const result = await step.agent.executeTask(retryInput);
    
    // Update pipeline state
    if (pipelineState && pipelineState.steps[stepIndex]) {
      pipelineState.steps[stepIndex].status = result.success ? 'completed' : 'failed';
      pipelineState.steps[stepIndex].retryCount = (pipelineState.steps[stepIndex].retryCount || 0) + 1;
      await this.writeState(pipelineId, pipelineState);
    }
    
    return result;
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    return `Execute this task through the pipeline of specialized agents:

Task: ${task.title}
Description: ${task.description}

Pipeline steps:
${this.steps.map((step, i) => `${i + 1}. ${step.name}`).join('\n')}

Coordinate the execution and ensure results flow properly between steps.`;
  }
  
  /**
   * Get execution history
   */
  public getExecutionHistory(): any[] {
    return [...this.executionHistory];
  }
  
  /**
   * Add a step to the pipeline
   */
  public addStep(step: PipelineStep, index?: number): void {
    if (index !== undefined) {
      this.steps.splice(index, 0, step);
    } else {
      this.steps.push(step);
    }
  }
  
  /**
   * Remove a step from the pipeline
   */
  public removeStep(index: number): void {
    if (index >= 0 && index < this.steps.length) {
      this.steps.splice(index, 1);
    }
  }
}