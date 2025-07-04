/**
 * Human-in-the-Loop Agent Pattern
 * 
 * Coordinates with human users for guidance, approval, and collaboration
 */

import { Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty } from '../../types';
import { Logger } from '../../../utils/logger';
import * as vscode from 'vscode';

export interface HumanInteractionPoint {
  id: string;
  type: 'approval' | 'input' | 'choice' | 'review' | 'guidance';
  title: string;
  description: string;
  options?: string[];
  defaultValue?: any;
  required: boolean;
  timeout?: number; // milliseconds
}

export interface HumanResponse {
  interactionId: string;
  response: any;
  timestamp: Date;
  responseTime: number; // milliseconds
}

export interface HumanInLoopConfig extends ADKHivemindConfig {
  interactionPoints: HumanInteractionPoint[];
  fallbackBehavior?: 'wait' | 'skip' | 'use-default' | 'abort';
  maxWaitTime?: number;
  showProgress?: boolean;
}

export class HumanInLoopAgent extends ADKHivemindAgent {
  private interactionPoints: Map<string, HumanInteractionPoint>;
  private fallbackBehavior: string;
  private maxWaitTime: number;
  private showProgress: boolean;
  private interactionHistory: HumanResponse[] = [];
  private pendingInteractions: Map<string, any> = new Map();
  
  constructor(config: HumanInLoopConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are a collaborative AI agent that works closely with human users.
      
Your responsibilities:
1. Identify when human input or approval is needed
2. Present clear, actionable requests to humans
3. Incorporate human feedback effectively
4. Provide progress updates and context
5. Handle human absence gracefully

Interaction points:
${config.interactionPoints.map(ip => 
  `- ${ip.id} (${ip.type}): ${ip.description}`
).join('\n')}

Always:
- Be clear and concise in your requests
- Provide sufficient context for decisions
- Respect human time and attention
- Document human decisions for audit trails
- Continue work where possible while waiting`
    });
    
    this.interactionPoints = new Map(
      config.interactionPoints.map(ip => [ip.id, ip])
    );
    this.fallbackBehavior = config.fallbackBehavior || 'wait';
    this.maxWaitTime = config.maxWaitTime || 300000; // 5 minutes default
    this.showProgress = config.showProgress ?? true;
    
    // Add human interaction tools
    this.addHumanInteractionTools();
  }
  
  /**
   * Add human interaction tools
   */
  private addHumanInteractionTools(): void {
    this.tools.push(new Tool({
      name: 'request_approval',
      description: 'Request human approval for an action',
      parameters: {
        action: { type: 'string', required: true },
        context: { type: 'object', required: true },
        options: { type: 'array', required: false }
      },
      handler: async (params: any) => {
        return this.requestApproval(params.action, params.context, params.options);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'request_input',
      description: 'Request input from human',
      parameters: {
        prompt: { type: 'string', required: true },
        inputType: { type: 'string', required: true },
        defaultValue: { type: 'any', required: false }
      },
      handler: async (params: any) => {
        return this.requestInput(params.prompt, params.inputType, params.defaultValue);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'request_choice',
      description: 'Request human to make a choice',
      parameters: {
        question: { type: 'string', required: true },
        options: { type: 'array', required: true },
        multiSelect: { type: 'boolean', required: false }
      },
      handler: async (params: any) => {
        return this.requestChoice(params.question, params.options, params.multiSelect);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'show_progress',
      description: 'Show progress update to human',
      parameters: {
        message: { type: 'string', required: true },
        percentage: { type: 'number', required: false }
      },
      handler: async (params: any) => {
        return this.showProgressUpdate(params.message, params.percentage);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'request_review',
      description: 'Request human review of output',
      parameters: {
        output: { type: 'any', required: true },
        reviewPrompt: { type: 'string', required: true }
      },
      handler: async (params: any) => {
        return this.requestReview(params.output, params.reviewPrompt);
      }
    }));
  }
  
  /**
   * Execute task with human interactions
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    const sessionId = `human_loop_${task.id}`;
    
    try {
      // Initialize session
      const sessionState = {
        taskId: task.id,
        interactions: [],
        startTime: new Date(),
        status: 'active'
      };
      
      await this.writeState(sessionId, sessionState);
      
      // Show initial progress
      if (this.showProgress) {
        await this.showProgressUpdate(`Starting task: ${task.title}`, 0);
      }
      
      // Check for required interaction points
      const requiredInteractions = await this.identifyRequiredInteractions(task);
      
      // Execute pre-interaction work
      const preWorkResult = await this.executePreInteractionWork(task);
      
      // Handle each interaction point
      const interactionResults = new Map<string, any>();
      
      for (const interactionId of requiredInteractions) {
        const interaction = this.interactionPoints.get(interactionId);
        if (!interaction) continue;
        
        const response = await this.handleInteraction(interaction, task, preWorkResult);
        interactionResults.set(interactionId, response);
        
        // Store interaction
        sessionState.interactions.push({
          interactionId,
          type: interaction.type,
          response,
          timestamp: new Date()
        });
        
        await this.writeState(sessionId, sessionState);
      }
      
      // Execute post-interaction work
      const finalResult = await this.executePostInteractionWork(
        task,
        preWorkResult,
        interactionResults
      );
      
      // Request final review if configured
      if (this.interactionPoints.has('final_review')) {
        const reviewResult = await this.requestReview(
          finalResult,
          'Please review the final output'
        );
        
        if (!reviewResult.approved) {
          finalResult.output.reviewFeedback = reviewResult.feedback;
          finalResult.success = false;
        }
      }
      
      // Complete session
      sessionState.status = 'completed';
      await this.writeState(sessionId, sessionState);
      
      if (this.showProgress) {
        await this.showProgressUpdate('Task completed', 100);
      }
      
      return finalResult;
      
    } catch (error) {
      this.logger.error(`Human-in-loop execution failed`, error);
      
      return {
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
    }
  }
  
  /**
   * Identify required interactions for task
   */
  private async identifyRequiredInteractions(task: HivemindTask): Promise<string[]> {
    const required: string[] = [];
    
    // Check task type
    if (task.type === 'deployment' || task.type === 'deletion') {
      required.push('approval');
    }
    
    // Check complexity
    if (task.estimatedComplexity >= 8) {
      required.push('guidance');
    }
    
    // Check for specific flags in context
    if (task.context.requiresApproval) {
      required.push('approval');
    }
    
    if (task.context.requiresInput) {
      required.push('input');
    }
    
    // Filter to only configured interaction points
    return required.filter(id => this.interactionPoints.has(id));
  }
  
  /**
   * Execute work before human interactions
   */
  private async executePreInteractionWork(task: HivemindTask): Promise<any> {
    const prompt = `Analyze this task and prepare for human interaction:

Task: ${task.title}
Description: ${task.description}
Type: ${task.type}
Complexity: ${task.estimatedComplexity}/10

Prepare:
1. Summary of what will be done
2. Key decisions that need human input
3. Potential risks or concerns
4. Recommended approach

Return a structured analysis.`;

    const result = await this.run(prompt);
    
    try {
      return JSON.parse(result.output);
    } catch {
      return { analysis: result.output };
    }
  }
  
  /**
   * Handle specific interaction
   */
  private async handleInteraction(
    interaction: HumanInteractionPoint,
    task: HivemindTask,
    context: any
  ): Promise<any> {
    const startTime = Date.now();
    
    try {
      let response: any;
      
      switch (interaction.type) {
        case 'approval':
          response = await this.requestApproval(
            interaction.title,
            { task, context },
            interaction.options
          );
          break;
          
        case 'input':
          response = await this.requestInput(
            interaction.description,
            'string',
            interaction.defaultValue
          );
          break;
          
        case 'choice':
          response = await this.requestChoice(
            interaction.title,
            interaction.options || [],
            false
          );
          break;
          
        case 'review':
          response = await this.requestReview(
            context,
            interaction.description
          );
          break;
          
        case 'guidance':
          response = await this.requestGuidance(
            interaction.title,
            context
          );
          break;
          
        default:
          response = interaction.defaultValue;
      }
      
      // Record interaction
      this.interactionHistory.push({
        interactionId: interaction.id,
        response,
        timestamp: new Date(),
        responseTime: Date.now() - startTime
      });
      
      return response;
      
    } catch (error) {
      // Handle timeout or error based on fallback behavior
      return this.handleInteractionFailure(interaction, error);
    }
  }
  
  /**
   * Execute work after human interactions
   */
  private async executePostInteractionWork(
    task: HivemindTask,
    preWorkResult: any,
    interactionResults: Map<string, any>
  ): Promise<TaskResult> {
    const prompt = `Complete the task based on human input:

Task: ${task.title}
Pre-work Analysis: ${JSON.stringify(preWorkResult)}

Human Inputs:
${Array.from(interactionResults.entries()).map(([id, result]) => 
  `- ${id}: ${JSON.stringify(result)}`
).join('\n')}

Execute the task according to the human's guidance and preferences.`;

    const result = await this.run(prompt);
    
    return {
      taskId: task.id,
      success: true,
      output: {
        result: result.output,
        humanInteractions: Object.fromEntries(interactionResults),
        preWorkAnalysis: preWorkResult
      },
      filesModified: [],
      filesCreated: [],
      testsAdded: 0,
      issuesFound: [],
      performanceMetrics: {
        duration: Date.now(),
        tokensUsed: 0,
        cost: 0
      }
    };
  }
  
  /**
   * Request approval from human
   */
  private async requestApproval(
    action: string,
    context: any,
    options?: string[]
  ): Promise<any> {
    const items = options || ['Approve', 'Reject', 'Modify'];
    
    const choice = await vscode.window.showQuickPick(items, {
      placeHolder: action,
      title: 'Approval Required',
      ignoreFocusOut: true
    });
    
    if (!choice) {
      throw new Error('Approval cancelled');
    }
    
    if (choice === 'Modify') {
      const modification = await vscode.window.showInputBox({
        prompt: 'Enter modifications',
        placeHolder: 'Describe changes needed'
      });
      
      return {
        approved: false,
        action: 'modify',
        modification
      };
    }
    
    return {
      approved: choice === 'Approve',
      action: choice.toLowerCase()
    };
  }
  
  /**
   * Request input from human
   */
  private async requestInput(
    prompt: string,
    inputType: string,
    defaultValue?: any
  ): Promise<any> {
    if (inputType === 'boolean') {
      const choice = await vscode.window.showQuickPick(['Yes', 'No'], {
        placeHolder: prompt
      });
      return choice === 'Yes';
    }
    
    const input = await vscode.window.showInputBox({
      prompt,
      value: defaultValue?.toString() || '',
      ignoreFocusOut: true
    });
    
    if (input === undefined) {
      throw new Error('Input cancelled');
    }
    
    // Parse based on type
    if (inputType === 'number') {
      return parseFloat(input);
    }
    
    return input;
  }
  
  /**
   * Request choice from human
   */
  private async requestChoice(
    question: string,
    options: string[],
    multiSelect: boolean = false
  ): Promise<any> {
    if (multiSelect) {
      const picks = await vscode.window.showQuickPick(
        options.map(opt => ({ label: opt, picked: false })),
        {
          placeHolder: question,
          canPickMany: true,
          ignoreFocusOut: true
        }
      );
      
      return picks?.map(p => p.label) || [];
    }
    
    const choice = await vscode.window.showQuickPick(options, {
      placeHolder: question,
      ignoreFocusOut: true
    });
    
    if (!choice) {
      throw new Error('Choice cancelled');
    }
    
    return choice;
  }
  
  /**
   * Show progress update
   */
  private async showProgressUpdate(message: string, percentage?: number): Promise<void> {
    const progressMessage = percentage !== undefined 
      ? `${message} (${percentage}%)`
      : message;
    
    vscode.window.setStatusBarMessage(progressMessage, 3000);
    
    // Also log for records
    this.logger.info(`Progress: ${progressMessage}`);
  }
  
  /**
   * Request review from human
   */
  private async requestReview(output: any, reviewPrompt: string): Promise<any> {
    // Create temporary document with output
    const doc = await vscode.workspace.openTextDocument({
      content: JSON.stringify(output, null, 2),
      language: 'json'
    });
    
    await vscode.window.showTextDocument(doc, { preview: true });
    
    const choice = await vscode.window.showQuickPick(
      ['Approve', 'Request Changes', 'Reject'],
      {
        placeHolder: reviewPrompt,
        ignoreFocusOut: true
      }
    );
    
    let feedback: string | undefined;
    if (choice !== 'Approve') {
      feedback = await vscode.window.showInputBox({
        prompt: 'Please provide feedback',
        placeHolder: 'What changes are needed?',
        ignoreFocusOut: true
      });
    }
    
    return {
      approved: choice === 'Approve',
      action: choice?.toLowerCase().replace(' ', '_'),
      feedback
    };
  }
  
  /**
   * Request guidance from human
   */
  private async requestGuidance(
    title: string,
    context: any
  ): Promise<any> {
    const guidance = await vscode.window.showInputBox({
      prompt: title,
      placeHolder: 'Provide guidance or preferences',
      ignoreFocusOut: true,
      validateInput: (value) => {
        return value.length < 10 ? 'Please provide more detailed guidance' : null;
      }
    });
    
    if (!guidance) {
      throw new Error('Guidance cancelled');
    }
    
    return {
      guidance,
      timestamp: new Date()
    };
  }
  
  /**
   * Handle interaction failure
   */
  private handleInteractionFailure(
    interaction: HumanInteractionPoint,
    error: any
  ): any {
    this.logger.warn(`Interaction ${interaction.id} failed: ${error.message}`);
    
    switch (this.fallbackBehavior) {
      case 'use-default':
        return interaction.defaultValue;
        
      case 'skip':
        return null;
        
      case 'abort':
        throw error;
        
      case 'wait':
      default:
        // In VS Code context, we've already waited
        return interaction.defaultValue || null;
    }
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    return `Execute this task with human collaboration:

Task: ${task.title}
Description: ${task.description}

This task requires human interaction at key decision points.
Prepare clear requests and incorporate human feedback effectively.`;
  }
  
  /**
   * Get interaction history
   */
  public getInteractionHistory(): HumanResponse[] {
    return [...this.interactionHistory];
  }
}