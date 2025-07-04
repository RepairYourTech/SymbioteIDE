/**
 * Agent Orchestrator
 * 
 * Main orchestrator agent that manages teams of specialized agents dynamically
 */

// import { Agent, LlmAgent } from '@google/adk';
import { ADKAgent as Agent, LlmAgent } from './types';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { AgentFactory } from './agent-factory';
import { AgentCreatorAgent } from './agent-creator';
import { ToolBuilderAgent } from './tool-builder';
import { 
  ADKAgentConfig, 
  AgentTeam, 
  AgentRole,
  AgentCapability,
  AgentExecutionResult,
  AgentMetrics,
  ProviderType,
  ADKAgentType
} from './types';

interface TeamConfiguration {
  name: string;
  description: string;
  requiredCapabilities: string[];
  maxAgents?: number;
  autoScale?: boolean;
}

interface AgentRequest {
  taskType: string;
  requirements: string[];
  constraints?: {
    maxCost?: number;
    maxTime?: number;
    requiredModels?: string[];
  };
}

export class AgentOrchestrator extends EventEmitter {
  private logger = new Logger('AgentOrchestrator');
  private agentFactory: AgentFactory;
  private agentCreator: AgentCreatorAgent;
  private toolBuilder: ToolBuilderAgent;
  
  // Agent management
  private teams = new Map<string, AgentTeam>();
  private activeAgents = new Map<string, Agent>();
  private agentMetrics = new Map<string, AgentMetrics>();
  
  // Configuration
  private maxConcurrentAgents = 10;
  private autoCreateAgents = true;
  private learningEnabled = true;
  
  constructor(agentFactory: AgentFactory) {
    super();
    this.agentFactory = agentFactory;
    
    // Initialize core agents
    this.agentCreator = new AgentCreatorAgent(agentFactory);
    this.toolBuilder = new ToolBuilderAgent();
    
    this.initialize();
  }
  
  /**
   * Initialize the orchestrator with default teams
   */
  private async initialize(): Promise<void> {
    // Create default teams
    await this.createTeam({
      name: 'Core Development',
      description: 'Core coding and implementation team',
      requiredCapabilities: ['code_generation', 'code_review', 'testing'],
      maxAgents: 5,
      autoScale: true
    });
    
    await this.createTeam({
      name: 'Architecture & Design',
      description: 'System architecture and design team',
      requiredCapabilities: ['architecture_analysis', 'design_patterns', 'api_design'],
      maxAgents: 3
    });
    
    await this.createTeam({
      name: 'Quality & Security',
      description: 'Quality assurance and security team',
      requiredCapabilities: ['security_audit', 'performance_analysis', 'test_generation'],
      maxAgents: 4
    });
    
    await this.createTeam({
      name: 'Documentation & Support',
      description: 'Documentation and user support team',
      requiredCapabilities: ['documentation', 'api_docs', 'user_guides'],
      maxAgents: 3
    });
    
    await this.createTeam({
      name: 'Research & Innovation',
      description: 'Research and exploration team',
      requiredCapabilities: ['research', 'prototyping', 'technology_evaluation'],
      maxAgents: 2
    });
    
    this.logger.info('Agent Orchestrator initialized with default teams');
  }
  
  /**
   * Process a request by assembling the right team of agents
   */
  async processRequest(request: AgentRequest): Promise<AgentExecutionResult> {
    this.logger.info('Processing request', { taskType: request.taskType });
    
    try {
      // Analyze request to determine required agents
      const requiredAgents = await this.analyzeRequest(request);
      
      // Assemble team for the request
      const team = await this.assembleTeam(requiredAgents, request);
      
      // Execute with the assembled team
      const result = await this.executeWithTeam(team, request);
      
      // Learn from execution
      if (this.learningEnabled) {
        await this.learnFromExecution(team, request, result);
      }
      
      return result;
      
    } catch (error) {
      this.logger.error('Failed to process request', error);
      throw error;
    }
  }
  
  /**
   * Analyze request to determine required agents
   */
  private async analyzeRequest(request: AgentRequest): Promise<AgentRole[]> {
    const analysisAgent = await this.agentFactory.createAgent({
      id: 'request-analyzer',
      name: 'Request Analyzer',
      type: ADKAgentType.LLM,
      systemPrompt: `Analyze the following request and determine what types of specialized agents are needed.
      
Task Type: ${request.taskType}
Requirements: ${request.requirements.join(', ')}

Determine the optimal team composition from these available roles:
- Architecture Analyst: System design and architecture decisions
- Code Generator: Implementation and code writing
- Code Reviewer: Code quality and best practices
- Test Engineer: Test creation and validation
- Security Auditor: Security analysis and vulnerability detection
- Performance Optimizer: Performance analysis and optimization
- Documentation Writer: Technical documentation
- API Designer: API design and contracts
- Database Architect: Database design and optimization
- DevOps Engineer: CI/CD and deployment
- Research Analyst: Technology research and evaluation
- UI/UX Designer: User interface and experience design

Return a JSON array of required roles with priority (high/medium/low).`,
      model: {
        id: 'gemini-1.5-pro',
        name: 'Gemini 1.5 Pro',
        provider: ProviderType.Google,
        capabilities: ['long-context', 'analysis'],
        maxTokens: 1000000
      } as any,
      capabilities: {
        streaming: true,
        multiModal: false,
        toolUse: false,
        memoryAccess: false,
        a2aProtocol: false,
        mcpTools: false
      }
    });
    
    const analysis = await analysisAgent.execute({
      input: JSON.stringify(request)
    });
    
    // Parse the analysis result
    const roles = this.parseAnalysisResult(analysis);
    
    return roles;
  }
  
  /**
   * Assemble a team for the request
   */
  private async assembleTeam(
    requiredRoles: AgentRole[], 
    request: AgentRequest
  ): Promise<AgentTeam> {
    const team: AgentTeam = {
      id: `team_${Date.now()}`,
      name: `Team for ${request.taskType}`,
      agents: new Map(),
      capabilities: new Set(),
      metrics: {
        totalExecutions: 0,
        successRate: 0,
        averageDuration: 0,
        totalTokensUsed: 0,
        totalCost: 0
      }
    };
    
    // Try to reuse existing agents or create new ones
    for (const role of requiredRoles) {
      let agent = await this.findAvailableAgent(role);
      
      if (!agent && this.autoCreateAgents) {
        // Ask the Agent Creator to create a new agent
        agent = await this.agentCreator.createSpecializedAgent(role);
        
        if (agent) {
          this.activeAgents.set(agent.id, agent);
          this.logger.info(`Created new ${role.name} agent`);
        }
      }
      
      if (agent) {
        team.agents.set(role.id, agent);
        role.capabilities.forEach(cap => team.capabilities.add(cap));
      } else {
        this.logger.warn(`Could not find or create agent for role: ${role.name}`);
      }
    }
    
    // Store team
    this.teams.set(team.id, team);
    
    return team;
  }
  
  /**
   * Execute request with assembled team
   */
  private async executeWithTeam(
    team: AgentTeam, 
    request: AgentRequest
  ): Promise<AgentExecutionResult> {
    const startTime = Date.now();
    const results: any[] = [];
    let totalTokens = 0;
    let totalCost = 0;
    
    try {
      // Create execution plan
      const plan = await this.createExecutionPlan(team, request);
      
      // Execute plan steps
      for (const step of plan.steps) {
        const agent = team.agents.get(step.agentId);
        if (!agent) continue;
        
        this.logger.debug(`Executing step: ${step.name} with ${step.agentId}`);
        
        const stepResult = await agent.execute({
          ...step.input,
          previousResults: results
        });
        
        results.push({
          step: step.name,
          agentId: step.agentId,
          result: stepResult
        });
        
        // Track metrics
        if (stepResult.metrics) {
          totalTokens += stepResult.metrics.tokensUsed || 0;
          totalCost += stepResult.metrics.cost || 0;
        }
      }
      
      // Update team metrics
      const duration = Date.now() - startTime;
      this.updateTeamMetrics(team.id, {
        success: true,
        duration,
        tokensUsed: totalTokens,
        cost: totalCost
      });
      
      return {
        executionId: `exec_${Date.now()}`,
        agentId: team.id,
        status: 'completed' as const,
        result: results,
        metadata: {
          startTime: new Date(startTime),
          endTime: new Date(),
          duration,
          tokensUsed: totalTokens,
          cost: totalCost,
          agentsUsed: team.agents.size
        }
      };
      
    } catch (error) {
      this.logger.error('Team execution failed', error);
      
      // Update failure metrics
      this.updateTeamMetrics(team.id, {
        success: false,
        duration: Date.now() - startTime,
        tokensUsed: totalTokens,
        cost: totalCost
      });
      
      throw error;
    }
  }
  
  /**
   * Create execution plan for the team
   */
  private async createExecutionPlan(
    team: AgentTeam, 
    request: AgentRequest
  ): Promise<any> {
    // Use a planning agent to create the execution plan
    const plannerConfig: ADKAgentConfig = {
      id: 'execution-planner',
      name: 'Execution Planner',
      type: ADKAgentType.LLM,
      systemPrompt: `Create an execution plan for the team to complete the task.
      
Available agents: ${Array.from(team.agents.entries()).map(([id, agent]) => 
  `${id}: ${agent.name} - ${agent.description}`
).join('\n')}

Task: ${request.taskType}
Requirements: ${request.requirements.join(', ')}

Create a step-by-step plan with dependencies. Return as JSON.`,
      model: {
        id: 'gemini-1.5-flash',
        name: 'Gemini 1.5 Flash',
        provider: ProviderType.Google,
        capabilities: ['long-context', 'analysis'],
        maxTokens: 1000000
      } as any,
      capabilities: {
        streaming: true,
        multiModal: false,
        toolUse: true,
        memoryAccess: false,
        a2aProtocol: false,
        mcpTools: false
      }
    };
    
    const planner = await this.agentFactory.createAgent(plannerConfig);
    const planResult = await planner.execute({ request });
    
    return this.parsePlanResult(planResult);
  }
  
  /**
   * Learn from execution to improve future performance
   */
  private async learnFromExecution(
    team: AgentTeam,
    request: AgentRequest,
    result: AgentExecutionResult
  ): Promise<void> {
    // Store successful patterns
    if (result.status === 'completed') {
      const pattern = {
        taskType: request.taskType,
        teamComposition: Array.from(team.agents.keys()),
        metrics: result.metadata,
        timestamp: new Date()
      };
      
      // This would be stored in a learning system
      this.emit('pattern-learned', pattern);
    }
    
    // Analyze agent performance
    for (const [agentId, agent] of team.agents) {
      const agentMetrics = this.agentMetrics.get(agentId) || {
        totalExecutions: 0,
        successRate: 0,
        averageDuration: 0,
        totalTokensUsed: 0,
        totalCost: 0
      };
      
      // Update metrics
      agentMetrics.totalExecutions++;
      if (result.status === 'completed') {
        agentMetrics.successRate = 
          (agentMetrics.successRate * (agentMetrics.totalExecutions - 1) + 1) / 
          agentMetrics.totalExecutions;
      }
      
      this.agentMetrics.set(agentId, agentMetrics);
    }
  }
  
  /**
   * Create a new team configuration
   */
  async createTeam(config: TeamConfiguration): Promise<string> {
    const team: AgentTeam = {
      id: `team_${config.name.toLowerCase().replace(/\s+/g, '_')}`,
      name: config.name,
      agents: new Map(),
      capabilities: new Set(config.requiredCapabilities),
      metrics: {
        totalExecutions: 0,
        successRate: 0,
        averageDuration: 0,
        totalTokensUsed: 0,
        totalCost: 0
      }
    };
    
    this.teams.set(team.id, team);
    this.logger.info(`Created team: ${config.name}`);
    
    return team.id;
  }
  
  /**
   * Request creation of a new agent type
   */
  async requestNewAgent(specification: {
    name: string;
    description: string;
    capabilities: string[];
    preferredModel?: string;
    tools?: string[];
  }): Promise<Agent | null> {
    try {
      // Ask the Agent Creator to design and create the agent
      const agent = await this.agentCreator.createCustomAgent(specification);
      
      if (agent) {
        this.activeAgents.set(agent.id, agent);
        this.logger.info(`Successfully created new agent: ${specification.name}`);
      }
      
      return agent;
      
    } catch (error) {
      this.logger.error('Failed to create new agent', error);
      return null;
    }
  }
  
  /**
   * Request creation of a new tool
   */
  async requestNewTool(specification: {
    name: string;
    description: string;
    parameters: any;
    implementation: 'code' | 'api' | 'agent';
  }): Promise<any> {
    try {
      // Ask the Tool Builder to create the tool
      const tool = await this.toolBuilder.createTool(specification);
      
      if (tool) {
        this.logger.info(`Successfully created new tool: ${specification.name}`);
      }
      
      return tool;
      
    } catch (error) {
      this.logger.error('Failed to create new tool', error);
      return null;
    }
  }
  
  /**
   * Get team performance metrics
   */
  getTeamMetrics(teamId?: string): any {
    if (teamId) {
      const team = this.teams.get(teamId);
      return team ? team.metrics : null;
    }
    
    // Return all team metrics
    const allMetrics: any = {};
    for (const [id, team] of this.teams) {
      allMetrics[id] = team.metrics;
    }
    
    return allMetrics;
  }
  
  /**
   * Scale team by adding or removing agents
   */
  async scaleTeam(teamId: string, targetSize: number): Promise<void> {
    const team = this.teams.get(teamId);
    if (!team) {
      throw new Error(`Team not found: ${teamId}`);
    }
    
    const currentSize = team.agents.size;
    
    if (targetSize > currentSize) {
      // Add agents
      const agentsToAdd = targetSize - currentSize;
      this.logger.info(`Scaling up team ${teamId} by ${agentsToAdd} agents`);
      
      // Determine what types of agents to add based on team capabilities
      // This is simplified - in reality would be more sophisticated
      for (let i = 0; i < agentsToAdd; i++) {
        const role: AgentRole = {
          id: `${teamId}_agent_${Date.now()}_${i}`,
          name: `${team.name} Agent ${currentSize + i + 1}`,
          capabilities: Array.from(team.capabilities),
          priority: 'medium'
        };
        
        const agent = await this.agentCreator.createSpecializedAgent(role);
        if (agent) {
          team.agents.set(role.id, agent);
        }
      }
    } else if (targetSize < currentSize) {
      // Remove agents (least recently used)
      const agentsToRemove = currentSize - targetSize;
      this.logger.info(`Scaling down team ${teamId} by ${agentsToRemove} agents`);
      
      // In a real implementation, would use LRU or performance metrics
      const agentIds = Array.from(team.agents.keys());
      for (let i = 0; i < agentsToRemove; i++) {
        const agentId = agentIds[agentIds.length - 1 - i];
        team.agents.delete(agentId);
        this.activeAgents.delete(agentId);
      }
    }
  }
  
  // Helper methods
  
  private async findAvailableAgent(role: AgentRole): Promise<Agent | null> {
    // Look for an existing agent that matches the required capabilities
    for (const [id, agent] of this.activeAgents) {
      // Check if agent has required capabilities
      // This is simplified - would need proper capability matching
      if (this.agentMatchesRole(agent, role)) {
        return agent;
      }
    }
    
    return null;
  }
  
  private agentMatchesRole(agent: Agent, role: AgentRole): boolean {
    // Simplified matching - in reality would be more sophisticated
    return agent.name.toLowerCase().includes(role.name.toLowerCase());
  }
  
  private parseAnalysisResult(analysis: any): AgentRole[] {
    // Parse the LLM response to extract required roles
    // This is simplified - would need robust parsing
    try {
      const roles = JSON.parse(analysis.output);
      return roles.map((r: any) => ({
        id: `role_${r.name.toLowerCase().replace(/\s+/g, '_')}`,
        name: r.name,
        capabilities: r.capabilities || [],
        priority: r.priority || 'medium'
      }));
    } catch {
      // Fallback to default roles
      return [
        {
          id: 'role_code_generator',
          name: 'Code Generator',
          capabilities: ['code_generation'],
          priority: 'high'
        }
      ];
    }
  }
  
  private parsePlanResult(planResult: any): any {
    // Parse the execution plan
    try {
      return JSON.parse(planResult.output);
    } catch {
      // Fallback plan
      return {
        steps: [
          {
            name: 'Execute Task',
            agentId: Array.from(this.activeAgents.keys())[0],
            input: {}
          }
        ]
      };
    }
  }
  
  private updateTeamMetrics(
    teamId: string, 
    execution: {
      success: boolean;
      duration: number;
      tokensUsed: number;
      cost: number;
    }
  ): void {
    const team = this.teams.get(teamId);
    if (!team) return;
    
    const metrics = team.metrics;
    metrics.totalExecutions++;
    metrics.totalTokensUsed += execution.tokensUsed;
    metrics.totalCost += execution.cost;
    
    // Update success rate
    const successCount = metrics.successRate * (metrics.totalExecutions - 1);
    metrics.successRate = (successCount + (execution.success ? 1 : 0)) / metrics.totalExecutions;
    
    // Update average duration
    const totalDuration = metrics.averageDuration * (metrics.totalExecutions - 1);
    metrics.averageDuration = (totalDuration + execution.duration) / metrics.totalExecutions;
  }
}