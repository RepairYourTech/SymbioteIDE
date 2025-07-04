/**
 * LLM-Driven Router
 * 
 * Uses language models to make intelligent routing decisions
 */

import { LLMAgent, Tool } from '@google/adk';
import { Logger } from '../../../utils/logger';
import { HivemindTask, AgentSpecialty } from '../../types';
import { ADKHivemindAgent } from '../core/adk-hivemind-base';

export interface RoutingDecision {
  targetAgents: string[];
  executionPattern: 'sequential' | 'parallel' | 'conditional' | 'loop' | 'hierarchical';
  reasoning: string;
  confidence: number;
  conditions?: RoutingCondition[];
  fallbackAgents?: string[];
}

export interface RoutingCondition {
  expression: string;
  trueBranch: string | string[];
  falseBranch?: string | string[];
}

export interface RouterContext {
  availableAgents: Map<string, AgentProfile>;
  taskHistory?: RoutingHistory[];
  systemConstraints?: SystemConstraints;
}

export interface AgentProfile {
  id: string;
  name: string;
  specialty: AgentSpecialty;
  capabilities: string[];
  performance: {
    successRate: number;
    averageDuration: number;
    specialtyMatch: Map<string, number>;
  };
  currentLoad: number;
  availability: boolean;
}

export interface RoutingHistory {
  taskType: string;
  selectedAgents: string[];
  pattern: string;
  success: boolean;
  duration: number;
}

export interface SystemConstraints {
  maxConcurrentAgents?: number;
  preferredPatterns?: string[];
  avoidPatterns?: string[];
  costOptimization?: boolean;
  latencyTarget?: number;
}

export class LLMDrivenRouter {
  private logger = new Logger('LLMDrivenRouter');
  private routingAgent: LLMAgent;
  private routingHistory: RoutingHistory[] = [];
  private learningEnabled: boolean;
  
  constructor(
    private context: RouterContext,
    private learningEnabled: boolean = true
  ) {
    // Create specialized routing agent
    this.routingAgent = new LLMAgent({
      name: 'Intelligent Task Router',
      description: 'Makes intelligent routing decisions for task execution',
      instructions: this.buildRoutingInstructions(),
      model: {
        name: 'gemini-1.5-pro',
        parameters: {
          temperature: 0.3,
          max_tokens: 1000
        }
      },
      tools: this.createRoutingTools()
    });
  }
  
  /**
   * Make routing decision for a task
   */
  async route(task: HivemindTask): Promise<RoutingDecision> {
    try {
      // Prepare routing context
      const routingPrompt = this.buildRoutingPrompt(task);
      
      // Get routing decision from LLM
      const result = await this.routingAgent.run(routingPrompt, {
        task,
        availableAgents: this.serializeAgentProfiles(),
        history: this.getRelevantHistory(task),
        constraints: this.context.systemConstraints
      });
      
      // Parse and validate decision
      const decision = this.parseRoutingDecision(result.output);
      
      // Learn from decision if enabled
      if (this.learningEnabled) {
        this.recordDecision(task, decision);
      }
      
      this.logger.info(`Routed task ${task.id} to ${decision.targetAgents.join(', ')} using ${decision.executionPattern} pattern`);
      
      return decision;
      
    } catch (error) {
      this.logger.error('Routing failed', error);
      
      // Fallback to simple routing
      return this.fallbackRouting(task);
    }
  }
  
  /**
   * Make conditional routing decision
   */
  async routeConditional(
    task: HivemindTask,
    currentState: any
  ): Promise<RoutingDecision> {
    const prompt = `Based on the current state, determine the next routing path:

Task: ${task.title}
Current State: ${JSON.stringify(currentState)}

Available branches:
${this.buildBranchOptions()}

Evaluate conditions and determine which path to take.`;

    const result = await this.routingAgent.run(prompt, { task, currentState });
    
    return this.parseRoutingDecision(result.output);
  }
  
  /**
   * Build routing instructions
   */
  private buildRoutingInstructions(): string {
    return `You are an intelligent task router that analyzes tasks and determines the best execution strategy.

Your responsibilities:
1. Analyze task requirements and complexity
2. Match tasks with appropriate specialist agents
3. Determine optimal execution patterns (sequential, parallel, loop, etc.)
4. Consider agent availability and performance history
5. Optimize for success rate and efficiency
6. Provide clear reasoning for routing decisions

Available execution patterns:
- Sequential: Tasks executed in order, output passed between steps
- Parallel: Multiple agents work simultaneously
- Conditional: Branch based on conditions
- Loop: Iterative refinement until criteria met
- Hierarchical: Parent agents delegate to child agents

Consider these factors:
- Agent specialties and capabilities
- Task dependencies and requirements
- System constraints (concurrency, cost, latency)
- Historical performance data
- Current agent workload

Always return routing decisions as structured JSON.`;
  }
  
  /**
   * Create routing tools
   */
  private createRoutingTools(): Tool[] {
    return [
      new Tool({
        name: 'analyze_task_complexity',
        description: 'Analyze task complexity and requirements',
        parameters: {
          task: { type: 'object', required: true }
        },
        handler: async ({ task }) => this.analyzeTaskComplexity(task)
      }),
      
      new Tool({
        name: 'match_agents',
        description: 'Find agents matching task requirements',
        parameters: {
          requirements: { type: 'array', required: true },
          minMatch: { type: 'number', required: false }
        },
        handler: async ({ requirements, minMatch }) => 
          this.matchAgents(requirements, minMatch)
      }),
      
      new Tool({
        name: 'check_agent_availability',
        description: 'Check if agents are available',
        parameters: {
          agentIds: { type: 'array', required: true }
        },
        handler: async ({ agentIds }) => this.checkAvailability(agentIds)
      }),
      
      new Tool({
        name: 'get_performance_stats',
        description: 'Get agent performance statistics',
        parameters: {
          agentId: { type: 'string', required: true },
          taskType: { type: 'string', required: false }
        },
        handler: async ({ agentId, taskType }) => 
          this.getPerformanceStats(agentId, taskType)
      }),
      
      new Tool({
        name: 'simulate_execution_pattern',
        description: 'Simulate execution pattern performance',
        parameters: {
          pattern: { type: 'string', required: true },
          agents: { type: 'array', required: true },
          taskComplexity: { type: 'number', required: true }
        },
        handler: async ({ pattern, agents, taskComplexity }) =>
          this.simulatePattern(pattern, agents, taskComplexity)
      })
    ];
  }
  
  /**
   * Build routing prompt
   */
  private buildRoutingPrompt(task: HivemindTask): string {
    return `Route the following task to appropriate agents:

Task Details:
- ID: ${task.id}
- Type: ${task.type}
- Title: ${task.title}
- Description: ${task.description}
- Required Specialties: ${task.requiredSpecialties.join(', ')}
- Estimated Complexity: ${task.estimatedComplexity}/10
- Dependencies: ${task.dependencies.length > 0 ? task.dependencies.join(', ') : 'None'}
- Context Files: ${task.context.files?.length || 0} files

Determine:
1. Which agents should handle this task
2. The best execution pattern
3. Any conditions or branches needed
4. Fallback options if primary agents fail

Return a JSON object with:
{
  "targetAgents": ["agent_id1", "agent_id2"],
  "executionPattern": "sequential|parallel|conditional|loop|hierarchical",
  "reasoning": "explanation of routing decision",
  "confidence": 0.0-1.0,
  "conditions": [...] // if conditional pattern
  "fallbackAgents": [...] // optional
}`;
  }
  
  /**
   * Serialize agent profiles for LLM
   */
  private serializeAgentProfiles(): any {
    const profiles: any = {};
    
    this.context.availableAgents.forEach((profile, id) => {
      profiles[id] = {
        name: profile.name,
        specialty: profile.specialty,
        capabilities: profile.capabilities,
        successRate: profile.performance.successRate,
        currentLoad: profile.currentLoad,
        available: profile.availability
      };
    });
    
    return profiles;
  }
  
  /**
   * Get relevant routing history
   */
  private getRelevantHistory(task: HivemindTask): RoutingHistory[] {
    // Get similar task types from history
    return this.routingHistory
      .filter(h => h.taskType === task.type)
      .slice(-10); // Last 10 similar tasks
  }
  
  /**
   * Parse routing decision from LLM output
   */
  private parseRoutingDecision(output: string): RoutingDecision {
    try {
      // Extract JSON from output
      const jsonMatch = output.match(/\{[\s\S]*\}/);
      if (!jsonMatch) {
        throw new Error('No JSON found in output');
      }
      
      const parsed = JSON.parse(jsonMatch[0]);
      
      // Validate and normalize
      return {
        targetAgents: parsed.targetAgents || [],
        executionPattern: parsed.executionPattern || 'sequential',
        reasoning: parsed.reasoning || 'No reasoning provided',
        confidence: parsed.confidence || 0.5,
        conditions: parsed.conditions,
        fallbackAgents: parsed.fallbackAgents
      };
      
    } catch (error) {
      this.logger.error('Failed to parse routing decision', error);
      throw error;
    }
  }
  
  /**
   * Fallback routing logic
   */
  private fallbackRouting(task: HivemindTask): RoutingDecision {
    // Simple specialty-based routing
    const matchingAgents: string[] = [];
    
    this.context.availableAgents.forEach((profile, id) => {
      if (task.requiredSpecialties.includes(profile.specialty) && profile.availability) {
        matchingAgents.push(id);
      }
    });
    
    // Default to first available agent if no matches
    if (matchingAgents.length === 0) {
      const firstAvailable = Array.from(this.context.availableAgents.entries())
        .find(([_, profile]) => profile.availability);
      if (firstAvailable) {
        matchingAgents.push(firstAvailable[0]);
      }
    }
    
    return {
      targetAgents: matchingAgents,
      executionPattern: matchingAgents.length > 1 ? 'parallel' : 'sequential',
      reasoning: 'Fallback routing based on specialty matching',
      confidence: 0.3
    };
  }
  
  /**
   * Record routing decision for learning
   */
  private recordDecision(task: HivemindTask, decision: RoutingDecision): void {
    const record: RoutingHistory = {
      taskType: task.type,
      selectedAgents: decision.targetAgents,
      pattern: decision.executionPattern,
      success: false, // Will be updated later
      duration: 0 // Will be updated later
    };
    
    this.routingHistory.push(record);
    
    // Keep history size manageable
    if (this.routingHistory.length > 1000) {
      this.routingHistory = this.routingHistory.slice(-500);
    }
  }
  
  /**
   * Update routing outcome
   */
  updateOutcome(taskId: string, success: boolean, duration: number): void {
    // Find and update the routing record
    const record = this.routingHistory.find(h => 
      h.taskType === taskId // This would need better tracking
    );
    
    if (record) {
      record.success = success;
      record.duration = duration;
      
      // Update agent performance stats
      record.selectedAgents.forEach(agentId => {
        const profile = this.context.availableAgents.get(agentId);
        if (profile) {
          // Update success rate (simple moving average)
          const oldRate = profile.performance.successRate;
          profile.performance.successRate = oldRate * 0.9 + (success ? 1 : 0) * 0.1;
          
          // Update average duration
          const oldDuration = profile.performance.averageDuration;
          profile.performance.averageDuration = oldDuration * 0.9 + duration * 0.1;
        }
      });
    }
  }
  
  // Tool implementations
  
  private async analyzeTaskComplexity(task: HivemindTask): Promise<any> {
    return {
      estimatedComplexity: task.estimatedComplexity,
      requiredSpecialties: task.requiredSpecialties,
      hasFiles: (task.context.files?.length || 0) > 0,
      hasDependencies: task.dependencies.length > 0,
      suggestedPattern: task.estimatedComplexity > 7 ? 'hierarchical' : 'sequential'
    };
  }
  
  private async matchAgents(requirements: string[], minMatch: number = 0.5): Promise<any> {
    const matches: any[] = [];
    
    this.context.availableAgents.forEach((profile, id) => {
      const matchScore = requirements.filter(req => 
        profile.capabilities.includes(req)
      ).length / requirements.length;
      
      if (matchScore >= minMatch) {
        matches.push({
          agentId: id,
          matchScore,
          capabilities: profile.capabilities
        });
      }
    });
    
    return matches.sort((a, b) => b.matchScore - a.matchScore);
  }
  
  private async checkAvailability(agentIds: string[]): Promise<any> {
    const availability: any = {};
    
    agentIds.forEach(id => {
      const profile = this.context.availableAgents.get(id);
      availability[id] = {
        available: profile?.availability || false,
        currentLoad: profile?.currentLoad || 0
      };
    });
    
    return availability;
  }
  
  private async getPerformanceStats(agentId: string, taskType?: string): Promise<any> {
    const profile = this.context.availableAgents.get(agentId);
    if (!profile) return null;
    
    const stats = {
      overall: {
        successRate: profile.performance.successRate,
        averageDuration: profile.performance.averageDuration
      }
    };
    
    if (taskType) {
      const typeHistory = this.routingHistory.filter(h => 
        h.taskType === taskType && h.selectedAgents.includes(agentId)
      );
      
      if (typeHistory.length > 0) {
        const typeSuccessRate = typeHistory.filter(h => h.success).length / typeHistory.length;
        const typeDuration = typeHistory.reduce((sum, h) => sum + h.duration, 0) / typeHistory.length;
        
        stats[taskType] = {
          successRate: typeSuccessRate,
          averageDuration: typeDuration,
          sampleSize: typeHistory.length
        };
      }
    }
    
    return stats;
  }
  
  private async simulatePattern(
    pattern: string,
    agents: string[],
    taskComplexity: number
  ): Promise<any> {
    // Simple simulation based on pattern characteristics
    const simulations = {
      sequential: {
        estimatedDuration: agents.length * taskComplexity * 60,
        parallelizationFactor: 0,
        riskLevel: 'low'
      },
      parallel: {
        estimatedDuration: taskComplexity * 60,
        parallelizationFactor: agents.length,
        riskLevel: 'medium'
      },
      loop: {
        estimatedDuration: taskComplexity * 60 * 3, // Assume 3 iterations
        parallelizationFactor: 0,
        riskLevel: 'medium'
      },
      hierarchical: {
        estimatedDuration: taskComplexity * 45, // Slightly optimized
        parallelizationFactor: agents.length / 2,
        riskLevel: 'high'
      }
    };
    
    return simulations[pattern] || simulations.sequential;
  }
  
  /**
   * Build branch options for conditional routing
   */
  private buildBranchOptions(): string {
    const agents = Array.from(this.context.availableAgents.entries());
    return agents.map(([id, profile]) => 
      `- ${id}: ${profile.name} (${profile.specialty})`
    ).join('\n');
  }
}