/**
 * Coordinator Agent Pattern
 * 
 * Central routing agent that dispatches tasks to specialized sub-agents
 */

import { LLMAgent, Tool, AgentTool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty } from '../../types';
import { Logger } from '../../../utils/logger';

export interface CoordinatorConfig extends ADKHivemindConfig {
  subAgents: Map<string, ADKHivemindAgent>;
  routingStrategy?: 'llm' | 'rule-based' | 'hybrid';
  routingPrompt?: string;
}

export class CoordinatorAgent extends ADKHivemindAgent {
  private subAgents: Map<string, ADKHivemindAgent>;
  private routingStrategy: string;
  private routingPrompt: string;
  private routingHistory: any[] = [];
  
  constructor(config: CoordinatorConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are a coordinator agent that routes tasks to specialized sub-agents.
      
Available agents:
${Array.from(config.subAgents.entries()).map(([id, agent]) => 
  `- ${id}: ${agent.name} (${agent.specialty}) - Capabilities: ${agent.capabilities.join(', ')}`
).join('\n')}

Your job is to:
1. Analyze incoming tasks
2. Determine which agent(s) are best suited
3. Route tasks appropriately
4. Coordinate results from multiple agents if needed`
    });
    
    this.subAgents = config.subAgents;
    this.routingStrategy = config.routingStrategy || 'llm';
    this.routingPrompt = config.routingPrompt || this.getDefaultRoutingPrompt();
    
    // Add sub-agents as tools
    this.addSubAgentTools();
  }
  
  /**
   * Add sub-agents as callable tools
   */
  private addSubAgentTools(): void {
    for (const [agentId, agent] of this.subAgents) {
      this.tools.push(new Tool({
        name: `use_${agentId}`,
        description: `Use ${agent.name} for ${agent.specialty} tasks. Capabilities: ${agent.capabilities.join(', ')}`,
        parameters: {
          task_description: { type: 'string', required: true },
          context: { type: 'object', required: false }
        },
        handler: async (params: any) => {
          return this.delegateToAgent(agentId, params);
        }
      }));
    }
    
    // Add coordination tools
    this.tools.push(new Tool({
      name: 'coordinate_multiple',
      description: 'Coordinate tasks across multiple agents',
      parameters: {
        agents: { type: 'array', required: true },
        tasks: { type: 'array', required: true },
        strategy: { type: 'string', required: false }
      },
      handler: async (params: any) => {
        return this.coordinateMultiple(params);
      }
    }));
  }
  
  /**
   * Route task to appropriate agent(s)
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      // Determine routing
      const routing = await this.determineRouting(task);
      
      this.logger.info(`Routing task ${task.id} to agents: ${routing.agents.join(', ')}`);
      
      // Store routing decision
      await this.writeState(`routing_${task.id}`, routing);
      this.routingHistory.push({ task: task.id, routing, timestamp: new Date() });
      
      let result: TaskResult;
      
      if (routing.agents.length === 1) {
        // Single agent execution
        result = await this.delegateSingle(routing.agents[0], task);
      } else {
        // Multi-agent coordination
        result = await this.coordinateExecution(routing, task);
      }
      
      // Update metrics
      this.logPerformance({
        metric: 'routing_success',
        value: result.success ? 1 : 0,
        metadata: { taskId: task.id, agents: routing.agents }
      });
      
      return result;
      
    } catch (error) {
      this.logger.error(`Routing failed for task ${task.id}`, error);
      
      return {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Routing error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
    }
  }
  
  /**
   * Determine routing for task
   */
  private async determineRouting(task: HivemindTask): Promise<any> {
    if (this.routingStrategy === 'rule-based') {
      return this.ruleBasedRouting(task);
    } else if (this.routingStrategy === 'llm') {
      return this.llmRouting(task);
    } else {
      // Hybrid: try rules first, fall back to LLM
      const ruleResult = this.ruleBasedRouting(task);
      if (ruleResult.confidence > 0.8) {
        return ruleResult;
      }
      return this.llmRouting(task);
    }
  }
  
  /**
   * Rule-based routing
   */
  private ruleBasedRouting(task: HivemindTask): any {
    const agents: string[] = [];
    let confidence = 0;
    
    // Match based on required specialties
    for (const [agentId, agent] of this.subAgents) {
      if (task.requiredSpecialties.includes(agent.specialty)) {
        agents.push(agentId);
        confidence += 0.3;
      }
      
      // Check capability match
      const taskKeywords = `${task.type} ${task.title} ${task.description}`.toLowerCase();
      const capabilityMatch = agent.capabilities.some(cap => 
        taskKeywords.includes(cap.toLowerCase())
      );
      
      if (capabilityMatch) {
        if (!agents.includes(agentId)) {
          agents.push(agentId);
        }
        confidence += 0.2;
      }
    }
    
    // Default to general agent if no matches
    if (agents.length === 0) {
      const generalAgent = Array.from(this.subAgents.entries())
        .find(([_, agent]) => agent.specialty === AgentSpecialty.General);
      if (generalAgent) {
        agents.push(generalAgent[0]);
        confidence = 0.5;
      }
    }
    
    return {
      agents,
      strategy: agents.length > 1 ? 'parallel' : 'single',
      confidence: Math.min(confidence, 1.0),
      reasoning: 'Rule-based matching'
    };
  }
  
  /**
   * LLM-based routing
   */
  private async llmRouting(task: HivemindTask): Promise<any> {
    const prompt = `${this.routingPrompt}

Task Details:
- ID: ${task.id}
- Type: ${task.type}
- Title: ${task.title}
- Description: ${task.description}
- Required Specialties: ${task.requiredSpecialties.join(', ')}
- Complexity: ${task.estimatedComplexity}/10
- Context Files: ${task.context.files?.join(', ') || 'None'}

Determine the best agent(s) to handle this task. Consider:
1. Agent specialties and capabilities
2. Task complexity and requirements
3. Whether multiple agents should collaborate
4. Execution strategy (sequential, parallel)

Return a JSON object with:
{
  "agents": ["agent_id1", "agent_id2"],
  "strategy": "parallel" | "sequential" | "single",
  "reasoning": "explanation of routing decision"
}`;

    const result = await this.run(prompt);
    
    try {
      const routing = JSON.parse(result.output);
      return {
        ...routing,
        confidence: 0.9
      };
    } catch {
      // Fallback to first available agent
      const firstAgent = Array.from(this.subAgents.keys())[0];
      return {
        agents: [firstAgent],
        strategy: 'single',
        confidence: 0.3,
        reasoning: 'Failed to parse LLM routing, using fallback'
      };
    }
  }
  
  /**
   * Delegate to single agent
   */
  private async delegateSingle(agentId: string, task: HivemindTask): Promise<TaskResult> {
    const agent = this.subAgents.get(agentId);
    if (!agent) {
      throw new Error(`Agent ${agentId} not found`);
    }
    
    this.logger.info(`Delegating task ${task.id} to ${agent.name}`);
    
    // Execute with sub-agent
    const result = await agent.executeTask(task);
    
    // Store delegation result
    await this.writeState(`delegation_${task.id}_${agentId}`, result);
    
    return result;
  }
  
  /**
   * Coordinate multiple agents
   */
  private async coordinateExecution(routing: any, task: HivemindTask): Promise<TaskResult> {
    const results: TaskResult[] = [];
    
    if (routing.strategy === 'parallel') {
      // Execute in parallel
      const promises = routing.agents.map((agentId: string) => 
        this.delegateSingle(agentId, task)
      );
      
      const parallelResults = await Promise.allSettled(promises);
      
      for (const result of parallelResults) {
        if (result.status === 'fulfilled') {
          results.push(result.value);
        } else {
          this.logger.error('Agent execution failed', result.reason);
        }
      }
    } else {
      // Execute sequentially
      for (const agentId of routing.agents) {
        try {
          const result = await this.delegateSingle(agentId, task);
          results.push(result);
          
          // Pass result to next agent via state
          await this.writeState(`sequential_result_${task.id}`, result);
        } catch (error) {
          this.logger.error(`Sequential execution failed at ${agentId}`, error);
          break;
        }
      }
    }
    
    // Aggregate results
    return this.aggregateResults(task, results);
  }
  
  /**
   * Aggregate results from multiple agents
   */
  private aggregateResults(task: HivemindTask, results: TaskResult[]): TaskResult {
    const aggregated: TaskResult = {
      taskId: task.id,
      success: results.some(r => r.success),
      output: {},
      filesModified: [],
      filesCreated: [],
      testsAdded: 0,
      issuesFound: [],
      performanceMetrics: {
        duration: 0,
        tokensUsed: 0,
        cost: 0
      }
    };
    
    // Merge results
    const outputs: any[] = [];
    for (const result of results) {
      if (result.success) {
        outputs.push(result.output);
      }
      
      aggregated.filesModified.push(...result.filesModified);
      aggregated.filesCreated.push(...result.filesCreated);
      aggregated.testsAdded += result.testsAdded;
      aggregated.issuesFound.push(...result.issuesFound);
      
      aggregated.performanceMetrics.duration += result.performanceMetrics.duration;
      aggregated.performanceMetrics.tokensUsed += result.performanceMetrics.tokensUsed;
      aggregated.performanceMetrics.cost += result.performanceMetrics.cost;
    }
    
    // Remove duplicates
    aggregated.filesModified = [...new Set(aggregated.filesModified)];
    aggregated.filesCreated = [...new Set(aggregated.filesCreated)];
    aggregated.issuesFound = [...new Set(aggregated.issuesFound)];
    
    // Combine outputs
    aggregated.output = {
      agentResults: results.map(r => ({
        agentId: r.taskId,
        success: r.success,
        output: r.output
      })),
      combinedOutput: outputs
    };
    
    return aggregated;
  }
  
  /**
   * Delegate to specific agent (for tool use)
   */
  private async delegateToAgent(agentId: string, params: any): Promise<any> {
    const agent = this.subAgents.get(agentId);
    if (!agent) {
      throw new Error(`Agent ${agentId} not found`);
    }
    
    // Create a task for the agent
    const delegatedTask: HivemindTask = {
      id: `delegated_${Date.now()}`,
      type: 'delegated',
      title: params.task_description,
      description: params.task_description,
      requiredSpecialties: [agent.specialty],
      priority: 'medium',
      status: 'pending',
      dependencies: [],
      assignedAgents: [agentId],
      estimatedComplexity: 5,
      context: params.context || {},
      createdAt: new Date()
    };
    
    return agent.executeTask(delegatedTask);
  }
  
  /**
   * Coordinate multiple agents (for tool use)
   */
  private async coordinateMultiple(params: any): Promise<any> {
    const { agents, tasks, strategy = 'parallel' } = params;
    const results: any[] = [];
    
    if (strategy === 'parallel') {
      const promises = agents.map((agentId: string, index: number) => 
        this.delegateToAgent(agentId, { task_description: tasks[index] })
      );
      
      const parallelResults = await Promise.allSettled(promises);
      
      for (const result of parallelResults) {
        if (result.status === 'fulfilled') {
          results.push(result.value);
        }
      }
    } else {
      for (let i = 0; i < agents.length; i++) {
        const result = await this.delegateToAgent(agents[i], { 
          task_description: tasks[i] 
        });
        results.push(result);
      }
    }
    
    return results;
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    return `Coordinate the execution of this task across specialized agents:

Task: ${task.title}
Description: ${task.description}

Use the available agent tools to delegate work appropriately.
Coordinate results to provide a comprehensive solution.`;
  }
  
  /**
   * Get default routing prompt
   */
  private getDefaultRoutingPrompt(): string {
    return `You are a task routing coordinator. Analyze tasks and determine which specialized agents should handle them.
Consider agent capabilities, task requirements, and whether multiple agents should collaborate.`;
  }
  
  /**
   * Get routing history
   */
  public getRoutingHistory(): any[] {
    return [...this.routingHistory];
  }
}