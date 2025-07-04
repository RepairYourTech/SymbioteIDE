/**
 * Workflow Manager
 * 
 * Manages terminal workflows and their execution
 */

import {
  TerminalWorkflow,
  WorkflowStep,
  WorkflowExecution,
  WorkflowStepExecution,
  WorkflowTrigger
} from '../../types/terminal-api';
import { TerminalManager } from './terminal-manager';
import { generateId } from '../utils/id-generator';

export class WorkflowManager {
  private workflows: Map<string, TerminalWorkflow> = new Map();
  private executions: Map<string, WorkflowExecution> = new Map();
  
  /**
   * Create a new workflow
   */
  async createWorkflow(
    workflow: Omit<TerminalWorkflow, 'id' | 'created_at'>
  ): Promise<TerminalWorkflow> {
    const id = generateId('workflow');
    const created = new Date().toISOString();
    
    const newWorkflow: TerminalWorkflow = {
      ...workflow,
      id,
      created_at: created
    };
    
    this.workflows.set(id, newWorkflow);
    return newWorkflow;
  }
  
  /**
   * Get a workflow by ID
   */
  async getWorkflow(id: string): Promise<TerminalWorkflow> {
    const workflow = this.workflows.get(id);
    if (!workflow) {
      throw new Error(`Workflow ${id} not found`);
    }
    return workflow;
  }
  
  /**
   * Update a workflow
   */
  async updateWorkflow(
    id: string,
    updates: Partial<TerminalWorkflow>
  ): Promise<TerminalWorkflow> {
    const workflow = await this.getWorkflow(id);
    const updated = { ...workflow, ...updates, id }; // Ensure ID can't be changed
    this.workflows.set(id, updated);
    return updated;
  }
  
  /**
   * Delete a workflow
   */
  async deleteWorkflow(id: string): Promise<void> {
    this.workflows.delete(id);
  }
  
  /**
   * Execute a workflow
   */
  async executeWorkflow(
    workflow: TerminalWorkflow,
    variables: Record<string, any> = {},
    terminalManager: TerminalManager
  ): Promise<WorkflowExecution> {
    const executionId = generateId('exec');
    const sessionId = generateId('workflow-session');
    
    // Create a dedicated session for this workflow
    const session = await terminalManager.createSession({
      workingDirectory: variables.workingDirectory
    });
    
    const execution: WorkflowExecution = {
      id: executionId,
      workflow_id: workflow.id,
      status: 'running',
      started_at: new Date().toISOString(),
      steps: workflow.steps.map(step => ({
        step_id: step.id,
        status: 'pending',
        command: this.interpolateVariables(step.command, { ...workflow.variables, ...variables })
      })),
      variables: { ...workflow.variables, ...variables },
      trigger: 'manual'
    };
    
    this.executions.set(executionId, execution);
    
    // Execute steps sequentially
    for (let i = 0; i < workflow.steps.length; i++) {
      const step = workflow.steps[i];
      const stepExecution = execution.steps[i];
      
      try {
        // Check condition
        if (step.condition && !this.evaluateCondition(step.condition, variables)) {
          stepExecution.status = 'skipped';
          continue;
        }
        
        stepExecution.status = 'running';
        stepExecution.started_at = new Date().toISOString();
        
        // Execute command
        const result = await terminalManager.executeCommand(session.id, stepExecution.command);
        
        stepExecution.status = 'completed';
        stepExecution.completed_at = new Date().toISOString();
        stepExecution.exit_code = result.exit_code;
        stepExecution.output = result.output;
        
        // Handle failures
        if (result.exit_code !== 0) {
          stepExecution.status = 'failed';
          
          if (step.retry && stepExecution.retries !== undefined && stepExecution.retries < step.retry.attempts) {
            // Retry logic
            stepExecution.retries = (stepExecution.retries || 0) + 1;
            await new Promise(resolve => setTimeout(resolve, step.retry!.delay * 1000));
            i--; // Retry this step
            continue;
          }
          
          // Execute on_failure if defined
          if (step.on_failure) {
            await terminalManager.executeCommand(session.id, step.on_failure);
          }
          
          // Stop execution unless we should continue
          if (!workflow.steps.some(s => s.dependencies?.includes(step.id))) {
            execution.status = 'failed';
            break;
          }
        } else {
          // Execute on_success if defined
          if (step.on_success) {
            await terminalManager.executeCommand(session.id, step.on_success);
          }
        }
        
      } catch (error) {
        stepExecution.status = 'failed';
        stepExecution.error = error instanceof Error ? error.message : 'Unknown error';
        execution.status = 'failed';
        break;
      }
    }
    
    // Set final status
    if (execution.status === 'running') {
      execution.status = 'completed';
    }
    execution.completed_at = new Date().toISOString();
    
    // Clean up session
    await terminalManager.terminateSession(session.id);
    
    return execution;
  }
  
  /**
   * Get workflow execution
   */
  async getExecution(executionId: string): Promise<WorkflowExecution> {
    const execution = this.executions.get(executionId);
    if (!execution) {
      throw new Error(`Execution ${executionId} not found`);
    }
    return execution;
  }
  
  /**
   * List all workflows
   */
  async listWorkflows(): Promise<TerminalWorkflow[]> {
    return Array.from(this.workflows.values());
  }
  
  /**
   * List executions for a workflow
   */
  async listExecutions(workflowId: string): Promise<WorkflowExecution[]> {
    return Array.from(this.executions.values())
      .filter(exec => exec.workflow_id === workflowId);
  }
  
  // Private methods
  
  private interpolateVariables(template: string, variables: Record<string, any>): string {
    let result = template;
    
    // Replace ${variable} patterns
    Object.entries(variables).forEach(([key, value]) => {
      result = result.replace(new RegExp(`\\$\\{${key}\\}`, 'g'), String(value));
    });
    
    // Replace $variable patterns
    Object.entries(variables).forEach(([key, value]) => {
      result = result.replace(new RegExp(`\\$${key}\\b`, 'g'), String(value));
    });
    
    return result;
  }
  
  private evaluateCondition(condition: string, variables: Record<string, any>): boolean {
    try {
      // Simple condition evaluation
      // In production, use a proper expression evaluator
      const interpolated = this.interpolateVariables(condition, variables);
      
      // Basic comparisons
      if (interpolated.includes('==')) {
        const [left, right] = interpolated.split('==').map(s => s.trim());
        return left === right;
      }
      if (interpolated.includes('!=')) {
        const [left, right] = interpolated.split('!=').map(s => s.trim());
        return left !== right;
      }
      
      // Default to true if we can't evaluate
      return true;
    } catch {
      return true;
    }
  }
}