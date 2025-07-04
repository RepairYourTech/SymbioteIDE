/**
 * Workflow Executor
 * 
 * Executes visual workflows by interpreting nodes and edges
 */

import { 
  AgentWorkflow, 
  WorkflowNode, 
  WorkflowEdge,
  ExecutionStatus,
  WorkflowExecution,
  ExecutionStep,
  WorkflowError,
  AgentNodeConfig,
  ToolNodeConfig,
  ConditionNodeConfig,
  LoopNodeConfig,
  DataNodeConfig
} from '../types/workflow-types';
import { AgentFactory } from '../../adk/agent-factory';
import { AgentOrchestrator } from '../../adk/agent-orchestrator';
import { Logger } from '../../utils/logger';

export interface ExecutionHandler {
  onNodeStart?: (nodeId: string) => void;
  onNodeComplete?: (nodeId: string, output: any) => void;
  onNodeError?: (nodeId: string, error: WorkflowError) => void;
  onLog?: (message: string) => void;
}

export class WorkflowExecutor {
  private logger = new Logger('WorkflowExecutor');
  private agentFactory: AgentFactory;
  private orchestrator: AgentOrchestrator;
  private executionContext: Map<string, any> = new Map();
  
  constructor() {
    this.agentFactory = new AgentFactory();
    this.orchestrator = new AgentOrchestrator(this.agentFactory);
  }
  
  /**
   * Execute a workflow
   */
  async execute(
    workflow: AgentWorkflow,
    input: any,
    handler?: ExecutionHandler
  ): Promise<WorkflowExecution> {
    this.logger.info(`Starting workflow execution: ${workflow.name}`);
    
    const execution: WorkflowExecution = {
      id: this.generateExecutionId(),
      workflowId: workflow.id,
      status: 'running',
      startTime: new Date(),
      input,
      steps: [],
      metrics: {
        totalDuration: 0,
        nodeExecutions: 0,
        tokensUsed: 0,
        totalCost: 0,
        memoryUsed: 0
      }
    };
    
    try {
      // Initialize context with input and variables
      this.initializeContext(workflow, input);
      
      // Find start nodes (nodes with no incoming edges)
      const startNodes = this.findStartNodes(workflow);
      
      if (startNodes.length === 0) {
        throw new Error('No start nodes found in workflow');
      }
      
      // Execute workflow starting from start nodes
      const output = await this.executeNodes(startNodes, workflow, execution, handler);
      
      // Complete execution
      execution.status = 'completed';
      execution.endTime = new Date();
      execution.output = output;
      execution.metrics.totalDuration = execution.endTime.getTime() - execution.startTime.getTime();
      
      this.logger.info(`Workflow execution completed: ${workflow.name}`);
      
    } catch (error: any) {
      this.logger.error(`Workflow execution failed: ${error.message}`);
      
      execution.status = 'failed';
      execution.endTime = new Date();
      execution.error = {
        code: 'EXECUTION_ERROR',
        message: error.message,
        stack: error.stack
      };
      
      if (handler?.onLog) {
        handler.onLog(`Execution failed: ${error.message}`);
      }
    }
    
    return execution;
  }
  
  /**
   * Initialize execution context
   */
  private initializeContext(workflow: AgentWorkflow, input: any): void {
    // Clear previous context
    this.executionContext.clear();
    
    // Add input
    this.executionContext.set('$input', input);
    
    // Add global variables
    workflow.variables
      .filter(v => v.scope === 'global')
      .forEach(v => {
        this.executionContext.set(v.name, v.value);
      });
  }
  
  /**
   * Find nodes with no incoming edges
   */
  private findStartNodes(workflow: AgentWorkflow): WorkflowNode[] {
    const nodesWithIncoming = new Set(workflow.edges.map(e => e.target));
    return workflow.nodes.filter(n => !nodesWithIncoming.has(n.id));
  }
  
  /**
   * Execute a set of nodes
   */
  private async executeNodes(
    nodes: WorkflowNode[],
    workflow: AgentWorkflow,
    execution: WorkflowExecution,
    handler?: ExecutionHandler
  ): Promise<any> {
    const outputs = new Map<string, any>();
    
    // Execute nodes (in parallel if multiple)
    const promises = nodes.map(async node => {
      const output = await this.executeNode(node, workflow, execution, handler);
      outputs.set(node.id, output);
    });
    
    await Promise.all(promises);
    
    // Find next nodes to execute
    const nextNodes: WorkflowNode[] = [];
    
    for (const node of nodes) {
      const outgoingEdges = workflow.edges.filter(e => e.source === node.id);
      
      for (const edge of outgoingEdges) {
        const targetNode = workflow.nodes.find(n => n.id === edge.target);
        
        if (targetNode) {
          // Check if all dependencies are satisfied
          const incomingEdges = workflow.edges.filter(e => e.target === targetNode.id);
          const allDependenciesMet = incomingEdges.every(e => outputs.has(e.source));
          
          if (allDependenciesMet && !nextNodes.find(n => n.id === targetNode.id)) {
            nextNodes.push(targetNode);
          }
        }
      }
    }
    
    // Continue execution with next nodes
    if (nextNodes.length > 0) {
      return await this.executeNodes(nextNodes, workflow, execution, handler);
    }
    
    // Return the last output
    return Array.from(outputs.values()).pop();
  }
  
  /**
   * Execute a single node
   */
  private async executeNode(
    node: WorkflowNode,
    workflow: AgentWorkflow,
    execution: WorkflowExecution,
    handler?: ExecutionHandler
  ): Promise<any> {
    const step: ExecutionStep = {
      nodeId: node.id,
      status: 'running',
      startTime: new Date(),
      input: this.getNodeInput(node, workflow)
    };
    
    execution.steps.push(step);
    execution.metrics.nodeExecutions++;
    
    if (handler?.onNodeStart) {
      handler.onNodeStart(node.id);
    }
    
    if (handler?.onLog) {
      handler.onLog(`Executing node: ${node.data.label} (${node.type})`);
    }
    
    try {
      let output: any;
      
      switch (node.type) {
        case 'agent':
          output = await this.executeAgentNode(node, step.input);
          break;
          
        case 'tool':
          output = await this.executeToolNode(node, step.input);
          break;
          
        case 'condition':
          output = await this.executeConditionNode(node, step.input, workflow);
          break;
          
        case 'loop':
          output = await this.executeLoopNode(node, step.input, workflow, execution, handler);
          break;
          
        case 'data':
        case 'input':
        case 'output':
          output = await this.executeDataNode(node, step.input);
          break;
          
        case 'transform':
          output = await this.executeTransformNode(node, step.input);
          break;
          
        case 'merge':
          output = await this.executeMergeNode(node, step.input);
          break;
          
        case 'split':
          output = await this.executeSplitNode(node, step.input);
          break;
          
        default:
          throw new Error(`Unknown node type: ${node.type}`);
      }
      
      // Update step
      step.status = 'completed';
      step.endTime = new Date();
      step.output = output;
      step.metrics = {
        duration: step.endTime.getTime() - step.startTime.getTime()
      };
      
      // Store output in context
      this.executionContext.set(node.id, output);
      
      if (handler?.onNodeComplete) {
        handler.onNodeComplete(node.id, output);
      }
      
      if (handler?.onLog) {
        handler.onLog(`Node completed: ${node.data.label}`);
      }
      
      return output;
      
    } catch (error: any) {
      const workflowError: WorkflowError = {
        code: 'NODE_EXECUTION_ERROR',
        message: error.message,
        nodeId: node.id,
        stack: error.stack
      };
      
      step.status = 'failed';
      step.endTime = new Date();
      step.error = workflowError;
      
      if (handler?.onNodeError) {
        handler.onNodeError(node.id, workflowError);
      }
      
      throw error;
    }
  }
  
  /**
   * Get input for a node from incoming edges
   */
  private getNodeInput(node: WorkflowNode, workflow: AgentWorkflow): any {
    const incomingEdges = workflow.edges.filter(e => e.target === node.id);
    
    if (incomingEdges.length === 0) {
      // No incoming edges, use context input
      return this.executionContext.get('$input');
    }
    
    if (incomingEdges.length === 1) {
      // Single input
      const sourceOutput = this.executionContext.get(incomingEdges[0].source);
      return this.applyEdgeTransform(sourceOutput, incomingEdges[0]);
    }
    
    // Multiple inputs - create object with port names
    const inputs: any = {};
    
    for (const edge of incomingEdges) {
      const sourceOutput = this.executionContext.get(edge.source);
      const transformed = this.applyEdgeTransform(sourceOutput, edge);
      
      if (edge.targetHandle) {
        inputs[edge.targetHandle] = transformed;
      } else {
        inputs[edge.source] = transformed;
      }
    }
    
    return inputs;
  }
  
  /**
   * Apply edge transformation if specified
   */
  private applyEdgeTransform(data: any, edge: WorkflowEdge): any {
    if (!edge.data?.transform) {
      return data;
    }
    
    try {
      // Simple JavaScript expression evaluation
      // In production, use a safer evaluator
      const func = new Function('data', `return ${edge.data.transform}`);
      return func(data);
    } catch (error) {
      this.logger.warn(`Failed to apply edge transform: ${error}`);
      return data;
    }
  }
  
  /**
   * Execute agent node
   */
  private async executeAgentNode(node: WorkflowNode, input: any): Promise<any> {
    const config = node.data.config as AgentNodeConfig;
    
    if (!config.agentType) {
      throw new Error('Agent type not specified');
    }
    
    // Create or get agent
    const agent = await this.agentFactory.createSpecializedAgent(config.agentType);
    
    // Execute agent
    const result = await agent.execute({ input });
    
    // Update metrics
    if (result.metrics) {
      // Add to execution metrics
    }
    
    return result.output || result;
  }
  
  /**
   * Execute tool node
   */
  private async executeToolNode(node: WorkflowNode, input: any): Promise<any> {
    const config = node.data.config as ToolNodeConfig;
    
    // This would integrate with MCP tools
    // For now, return mock result
    this.logger.debug(`Executing tool: ${config.toolId}`);
    
    return {
      toolId: config.toolId,
      result: 'Tool execution result',
      input
    };
  }
  
  /**
   * Execute condition node
   */
  private async executeConditionNode(
    node: WorkflowNode, 
    input: any,
    workflow: AgentWorkflow
  ): Promise<any> {
    const config = node.data.config as ConditionNodeConfig;
    
    let result: boolean;
    
    if (config.language === 'javascript') {
      try {
        const func = new Function('input', 'context', `return ${config.expression}`);
        result = func(input, Object.fromEntries(this.executionContext));
      } catch (error) {
        throw new Error(`Invalid condition expression: ${error}`);
      }
    } else {
      // For natural language, use an agent to evaluate
      // For now, default to true
      result = true;
    }
    
    // Return the input to the appropriate branch
    return {
      result,
      output: input,
      branch: result ? config.trueBranch : config.falseBranch
    };
  }
  
  /**
   * Execute loop node
   */
  private async executeLoopNode(
    node: WorkflowNode,
    input: any,
    workflow: AgentWorkflow,
    execution: WorkflowExecution,
    handler?: ExecutionHandler
  ): Promise<any> {
    const config = node.data.config as LoopNodeConfig;
    const results: any[] = [];
    
    if (config.loopType === 'forEach') {
      const items = Array.isArray(input) ? input : [input];
      
      for (const item of items) {
        // Execute loop body
        // This would execute connected nodes
        results.push(item);
      }
    } else if (config.loopType === 'while') {
      let iteration = 0;
      
      while (iteration < (config.maxIterations || 100)) {
        // Evaluate condition
        const shouldContinue = true; // Would evaluate condition
        
        if (!shouldContinue) {
          break;
        }
        
        // Execute loop body
        results.push(input);
        iteration++;
      }
    }
    
    return results;
  }
  
  /**
   * Execute data node
   */
  private async executeDataNode(node: WorkflowNode, input: any): Promise<any> {
    const config = node.data.config as DataNodeConfig;
    
    switch (config.dataType) {
      case 'input':
        return input;
        
      case 'output':
        return input;
        
      case 'constant':
        return config.value;
        
      case 'variable':
        return this.executionContext.get(config.value) || null;
        
      default:
        return input;
    }
  }
  
  /**
   * Execute transform node
   */
  private async executeTransformNode(node: WorkflowNode, input: any): Promise<any> {
    // Transform logic would go here
    return input;
  }
  
  /**
   * Execute merge node
   */
  private async executeMergeNode(node: WorkflowNode, input: any): Promise<any> {
    if (Array.isArray(input)) {
      return input;
    }
    
    // Merge multiple inputs into array or object
    return Object.values(input);
  }
  
  /**
   * Execute split node
   */
  private async executeSplitNode(node: WorkflowNode, input: any): Promise<any> {
    if (Array.isArray(input)) {
      return input;
    }
    
    // Split input for parallel processing
    return [input];
  }
  
  private generateExecutionId(): string {
    return `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}