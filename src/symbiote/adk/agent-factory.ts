/**
 * Agent Factory
 * 
 * Factory for creating and managing Google AI ADK agents
 */

// Import from our ADK client which bridges to the real Python Google ADK
import { ADKClient } from './adk-client';
import { 
  ADKAgent,
  ADKAgentConfig, 
  ADKAgentType, 
  ADKTool,
  AgentBuilder,
  AgentRegistryEntry,
  MemoryConfig,
  A2AConfig,
  WorkflowDefinition
} from './types';
import { 
  Agent,
  LlmAgent,
  SequentialAgent,
  ParallelAgent,
  LoopAgent,
  Memory
} from './placeholder-agents';
import { ModelProfile } from '../orchestration/interfaces';
import { Logger } from '../utils/logger';
import { MCPToolAdapter } from './mcp-tool-adapter';
import { A2AAgentAdapter } from './a2a-agent-adapter';
import { MemoryAdapter } from './memory-adapter';
import * as SpecializedAgents from './specialized-agents';

export class AgentFactory {
  private logger = new Logger('AgentFactory');
  private registry = new Map<string, AgentRegistryEntry>();
  private mcpAdapter: MCPToolAdapter;
  private a2aAdapter: A2AAgentAdapter;
  private memoryAdapter: MemoryAdapter;
  private specializedConfigs = new Map<string, ADKAgentConfig>();

  constructor() {
    this.mcpAdapter = new MCPToolAdapter();
    this.a2aAdapter = new A2AAgentAdapter();
    this.memoryAdapter = new MemoryAdapter();
    this.loadSpecializedAgents();
  }

  /**
   * Load specialized agent configurations
   */
  private loadSpecializedAgents(): void {
    // Load all specialized agent configurations
    this.specializedConfigs.set('architecture-analyst', SpecializedAgents.ArchitectureAnalystAgent.getConfig());
    this.specializedConfigs.set('performance-optimizer', SpecializedAgents.PerformanceOptimizerAgent.getConfig());
    this.specializedConfigs.set('security-auditor', SpecializedAgents.SecurityAuditorAgent.getConfig());
    this.specializedConfigs.set('documentation-generator', SpecializedAgents.DocumentationGeneratorAgent.getConfig());
    this.specializedConfigs.set('database-architect', SpecializedAgents.DatabaseArchitectAgent.getConfig());
    this.specializedConfigs.set('test-strategy', SpecializedAgents.TestStrategyAgent.getConfig());
    this.specializedConfigs.set('code-refactoring', SpecializedAgents.CodeRefactoringAgent.getConfig());
    this.specializedConfigs.set('dependency-analyzer', SpecializedAgents.DependencyAnalyzerAgent.getConfig());
  }

  /**
   * Create an agent from configuration
   */
  async createAgent(config: ADKAgentConfig): Promise<Agent> {
    this.logger.info(`Creating agent: ${config.id} (${config.type})`);

    try {
      let agent: Agent;

      switch (config.type) {
        case ADKAgentType.LLM:
          agent = await this.createLLMAgent(config);
          break;
        case ADKAgentType.Sequential:
          agent = await this.createSequentialAgent(config);
          break;
        case ADKAgentType.Parallel:
          agent = await this.createParallelAgent(config);
          break;
        case ADKAgentType.Loop:
          agent = await this.createLoopAgent(config);
          break;
        case ADKAgentType.Custom:
          agent = await this.createCustomAgent(config);
          break;
        default:
          throw new Error(`Unknown agent type: ${config.type}`);
      }

      // Register agent
      this.registry.set(config.id, {
        agent: config,
        implementation: agent,
        status: 'active',
        lastUsed: new Date(),
        metrics: {
          totalExecutions: 0,
          successRate: 0,
          averageDuration: 0,
          totalTokensUsed: 0,
          totalCost: 0
        }
      });

      return agent;
    } catch (error) {
      this.logger.error(`Failed to create agent ${config.id}`, error);
      throw error;
    }
  }

  /**
   * Create LLM agent
   */
  private async createLLMAgent(config: ADKAgentConfig): Promise<LlmAgent> {
    const tools = await this.prepareTools(config.tools || []);
    const memory = await this.prepareMemory(config.memory);

    const agent = new LlmAgent({
      name: config.name,
      description: config.description,
      systemPrompt: config.systemPrompt,
      model: this.getModelName(config.model),
      temperature: config.temperature,
      maxTokens: config.maxTokens,
      tools,
      memory,
      // Enable streaming if supported
      streaming: config.capabilities.streaming,
      // Enable multi-modal if supported
      multiModal: config.capabilities.multiModal
    });

    // Set up A2A if enabled
    if (config.a2aConfig?.enabled) {
      await this.a2aAdapter.registerAgent(config.id, agent, config.a2aConfig);
    }

    return agent;
  }

  /**
   * Create Sequential agent for workflows
   */
  private async createSequentialAgent(config: ADKAgentConfig): Promise<SequentialAgent> {
    // Sequential agents execute tasks in order
    const agent = new SequentialAgent({
      name: config.name,
      description: config.description,
      agents: [], // Will be populated from workflow
      continueOnError: false
    });

    return agent;
  }

  /**
   * Create Parallel agent for concurrent execution
   */
  private async createParallelAgent(config: ADKAgentConfig): Promise<ParallelAgent> {
    // Parallel agents execute tasks concurrently
    const agent = new ParallelAgent({
      name: config.name,
      description: config.description,
      agents: [], // Will be populated from workflow
      maxConcurrency: 5,
      waitForAll: true
    });

    return agent;
  }

  /**
   * Create Loop agent for iterative tasks
   */
  private async createLoopAgent(config: ADKAgentConfig): Promise<LoopAgent> {
    // Loop agents repeat tasks until condition is met
    const agent = new LoopAgent({
      name: config.name,
      description: config.description,
      agent: null, // Will be set from workflow
      maxIterations: 10,
      condition: (result: any) => {
        // Default condition - can be overridden
        return result.shouldContinue === true;
      }
    });

    return agent;
  }

  /**
   * Create custom agent
   */
  private async createCustomAgent(config: ADKAgentConfig): Promise<Agent> {
    // Custom agents extend base Agent class
    class CustomAgent extends Agent {
      async execute(input: any): Promise<any> {
        // Custom implementation
        return {
          status: 'success',
          output: input
        };
      }
    }

    return new CustomAgent();
  }

  /**
   * Prepare tools for agent
   */
  private async prepareTools(toolConfigs: ADKTool[]): Promise<ADKTool[]> {
    const tools: ADKTool[] = [];

    for (const toolConfig of toolConfigs) {
      let tool: ADKTool;

      if (!toolConfig.implementation) {
        // If no implementation provided, create a simple tool
        tool = {
          name: toolConfig.name,
          description: toolConfig.description,
          parameters: toolConfig.parameters,
          execute: toolConfig.execute
        };
      } else {
        switch (toolConfig.implementation.source) {
          case 'mcp':
            // Adapt MCP tools
            tool = await this.mcpAdapter.adaptTool(
              toolConfig.implementation.mcpServer!,
              toolConfig.implementation.mcpTool!
            );
            break;
          
          case 'code':
            // Direct code implementation
            tool = {
              name: toolConfig.name,
              description: toolConfig.description,
              execute: toolConfig.implementation.handler as (params: any) => Promise<any>,
              parameters: toolConfig.parameters
            };
            break;
          
          case 'openapi':
            // OpenAPI spec tool
            tool = await this.createOpenAPITool(toolConfig);
            break;
          
          case 'agent':
            // Another agent as a tool
            tool = await this.createAgentTool(toolConfig);
            break;
          
          default:
            throw new Error(`Unknown tool source: ${toolConfig.implementation.source}`);
        }
      }

      tools.push(tool);
    }

    return tools;
  }

  /**
   * Prepare memory for agent
   */
  private async prepareMemory(config?: MemoryConfig): Promise<Memory | undefined> {
    if (!config?.enabled) {
      return undefined;
    }

    return this.memoryAdapter.createMemory(config);
  }

  /**
   * Create OpenAPI tool
   */
  private async createOpenAPITool(config: ADKTool): Promise<ADKTool> {
    // Implementation for OpenAPI tools
    return {
      name: config.name,
      description: config.description,
      execute: async (params: any) => {
        // Call OpenAPI endpoint
        const response = await fetch(config.implementation?.openApiSpec!, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(params)
        });
        return response.json();
      }
    };
  }

  /**
   * Create agent tool (agent as a tool)
   */
  private async createAgentTool(config: ADKTool): Promise<ADKTool> {
    const agentId = config.implementation?.agentId!;
    
    return {
      name: config.name,
      description: config.description,
      execute: async (params: any) => {
        // Execute another agent as a tool
        const agent = this.getAgent(agentId);
        if (!agent) {
          throw new Error(`Agent ${agentId} not found`);
        }
        return agent.execute(params);
      }
    };
  }

  /**
   * Get model name from profile
   */
  private getModelName(model?: ModelProfile): string {
    if (!model) {
      return 'gemini-1.5-flash'; // Default to Gemini
    }
    
    // Map model profiles to ADK model names
    if (model.provider === 'google') {
      return model.name;
    }
    
    // For other providers, use LiteLLM format
    return `litellm/${model.provider}/${model.name}`;
  }

  /**
   * Create workflow from definition
   */
  async createWorkflow(definition: WorkflowDefinition): Promise<Agent> {
    const agents: Agent[] = [];

    // Create all agents in workflow
    for (const workflowAgent of definition.agents) {
      const agentConfig = this.registry.get(workflowAgent.agentId)?.agent;
      if (!agentConfig) {
        throw new Error(`Agent ${workflowAgent.agentId} not found`);
      }

      // Merge workflow-specific config
      const mergedConfig = {
        ...agentConfig,
        ...workflowAgent.config
      };

      const agent = await this.createAgent(mergedConfig);
      agents.push(agent);
    }

    // Create workflow agent based on flow type
    switch (definition.flow.type) {
      case 'sequential':
        return new SequentialAgent({
          name: definition.name,
          description: definition.description,
          agents
        });
      
      case 'parallel':
        return new ParallelAgent({
          name: definition.name,
          description: definition.description,
          agents
        });
      
      case 'loop':
        return new LoopAgent({
          name: definition.name,
          description: definition.description,
          agent: agents[0], // Use first agent for loop
          maxIterations: 10
        });
      
      case 'conditional':
        // TODO: Implement conditional workflow
        throw new Error('Conditional workflows not yet implemented');
      
      default:
        throw new Error(`Unknown flow type: ${definition.flow.type}`);
    }
  }

  /**
   * Get agent by ID
   */
  getAgent(id: string): Agent | undefined {
    return this.registry.get(id)?.implementation;
  }

  /**
   * Get all agents
   */
  getAllAgents(): AgentRegistryEntry[] {
    return Array.from(this.registry.values());
  }

  /**
   * Update agent metrics
   */
  updateMetrics(
    agentId: string, 
    execution: {
      success: boolean;
      duration: number;
      tokensUsed: number;
      cost: number;
    }
  ): void {
    const entry = this.registry.get(agentId);
    if (!entry) return;

    const metrics = entry.metrics!;
    metrics.totalExecutions++;
    metrics.totalTokensUsed += execution.tokensUsed;
    metrics.totalCost += execution.cost;
    
    // Update success rate
    const successCount = metrics.successRate * (metrics.totalExecutions - 1);
    metrics.successRate = (successCount + (execution.success ? 1 : 0)) / metrics.totalExecutions;
    
    // Update average duration
    const totalDuration = metrics.averageDuration * (metrics.totalExecutions - 1);
    metrics.averageDuration = (totalDuration + execution.duration) / metrics.totalExecutions;
    
    entry.lastUsed = new Date();
  }

  /**
   * Create agent builder
   */
  createBuilder(): AgentBuilder {
    return new AgentBuilderImpl();
  }

  /**
   * Get specialized agent configuration
   */
  getSpecializedConfig(type: string): ADKAgentConfig | undefined {
    return this.specializedConfigs.get(type);
  }

  /**
   * Create specialized agent by type
   */
  async createSpecializedAgent(type: string): Promise<Agent> {
    const config = this.specializedConfigs.get(type);
    if (!config) {
      throw new Error(`Unknown specialized agent type: ${type}`);
    }
    
    return await this.createAgent(config);
  }

  /**
   * List available specialized agents
   */
  listSpecializedAgents(): string[] {
    return Array.from(this.specializedConfigs.keys());
  }
}

/**
 * Agent Builder Implementation
 */
class AgentBuilderImpl implements AgentBuilder {
  private config: Partial<ADKAgentConfig> = {
    capabilities: {
      streaming: true,
      multiModal: false,
      toolUse: true,
      memoryAccess: true,
      a2aProtocol: false,
      mcpTools: true
    }
  };

  withId(id: string): AgentBuilder {
    this.config.id = id;
    return this;
  }

  withName(name: string): AgentBuilder {
    this.config.name = name;
    return this;
  }

  withType(type: ADKAgentType): AgentBuilder {
    this.config.type = type;
    return this;
  }

  withModel(model: ModelProfile): AgentBuilder {
    this.config.model = model;
    return this;
  }

  withSystemPrompt(prompt: string): AgentBuilder {
    this.config.systemPrompt = prompt;
    return this;
  }

  withTools(tools: ADKTool[]): AgentBuilder {
    this.config.tools = tools;
    return this;
  }

  withMemory(config: MemoryConfig): AgentBuilder {
    this.config.memory = config;
    return this;
  }

  withA2A(config: A2AConfig): AgentBuilder {
    this.config.a2aConfig = config;
    this.config.capabilities!.a2aProtocol = true;
    return this;
  }

  build(): ADKAgentConfig {
    if (!this.config.id || !this.config.name || !this.config.type) {
      throw new Error('Agent requires id, name, and type');
    }

    return this.config as ADKAgentConfig;
  }
}