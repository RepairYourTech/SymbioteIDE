/**
 * Agent Creator Agent
 * 
 * Specialized agent that can create other agents dynamically
 */

// Import from our types since we're using the ADK client bridge
import { ADKAgent, ADKAgent as Agent } from './types';
import { Logger } from '../utils/logger';
import { AgentFactory } from './agent-factory';
import { 
  ADKAgentConfig, 
  ADKAgentType,
  AgentRole,
  ADKTool,
  ModelProfile,
  ProviderType,
  LlmAgent
} from './types';
import { Capability } from '../orchestration/interfaces';

export class AgentCreatorAgent {
  private logger = new Logger('AgentCreatorAgent');
  private agentFactory: AgentFactory;
  private creatorAgent!: LlmAgent;
  
  // Agent templates for common patterns
  private agentTemplates = new Map<string, Partial<ADKAgentConfig>>();
  
  constructor(agentFactory: AgentFactory) {
    this.agentFactory = agentFactory;
    
    // Initialize the creator agent
    this.initializeCreator();
    this.loadTemplates();
  }

  private createModelProfile(
    id: string,
    name: string,
    provider: ProviderType,
    capabilities: Capability[],
    maxOutputTokens: number,
    contextWindow?: number
  ): ModelProfile {
    return {
      id,
      name,
      provider,
      displayName: name,
      capabilities,
      contextWindow: contextWindow || maxOutputTokens || 128000,
      maxOutputTokens,
      costPerToken: {
        input: 0.00001,
        output: 0.00003,
        currency: 'USD'
      },
      averageLatency: 2000,
      reliability: 0.95,
      specializations: capabilities.map(c => c.toLowerCase()),
      rateLimit: {
        requestsPerMinute: 60,
        tokensPerMinute: 100000
      },
      availability: {
        status: 'available' as const
      }
    };
  }
  
  private async initializeCreator(): Promise<void> {
    const config: ADKAgentConfig = {
      id: 'agent-creator',
      name: 'Agent Creator',
      type: ADKAgentType.LLM,
      description: 'Creates and configures other agents based on requirements',
      systemPrompt: `You are an expert at designing and creating AI agents. Your task is to:

1. Analyze requirements and determine the best agent architecture
2. Select appropriate models based on capabilities needed
3. Design tool sets for agents
4. Create effective system prompts
5. Configure agent parameters for optimal performance

When creating agents, consider:
- Task complexity and required capabilities
- Model strengths (Gemini for long context, GPT-4 for reasoning, Claude for coding)
- Cost vs performance tradeoffs
- Tool requirements and integrations
- Memory and context needs

Always return agent configurations as valid JSON.`,
      model: {
        id: 'gemini-1.5-pro',
        name: 'Gemini 1.5 Pro',
        provider: ProviderType.Google,
        displayName: 'Gemini 1.5 Pro',
        capabilities: [Capability.LongContext, Capability.Vision, Capability.FunctionCalling],
        contextWindow: 1000000,
        maxOutputTokens: 1000000,
        costPerToken: {
          input: 0.00001,
          output: 0.00003,
          currency: 'USD'
        },
        averageLatency: 2000,
        reliability: 0.98,
        specializations: ['long-context', 'vision', 'function-calling'],
        rateLimit: {
          requestsPerMinute: 60,
          tokensPerMinute: 1000000
        },
        availability: {
          status: 'available' as const
        }
      },
      temperature: 0.7,
      capabilities: {
        streaming: true,
        multiModal: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true
      }
    };
    
    this.creatorAgent = await this.agentFactory.createAgent(config) as LlmAgent;
  }
  
  /**
   * Load common agent templates
   */
  private loadTemplates(): void {
    // Code Generation Agent Template
    this.agentTemplates.set('code_generator', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are an expert code generator. Your responsibilities:
- Write clean, efficient, and well-documented code
- Follow language best practices and conventions
- Consider performance and security implications
- Include appropriate error handling
- Write code that is testable and maintainable`,
      model: this.createModelProfile(
        'claude-3-opus',
        'Claude 3 Opus',
        ProviderType.Anthropic,
        [Capability.CodeGeneration, Capability.LongContext],
        200000
      ),
      temperature: 0.2
    });
    
    // Architecture Analyst Template
    this.agentTemplates.set('architecture_analyst', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are a system architecture expert. Analyze codebases and provide:
- Architecture patterns and design decisions
- Component relationships and dependencies
- Performance bottlenecks and optimization opportunities
- Scalability considerations
- Security architecture review`,
      model: this.createModelProfile(
        'gemini-1.5-pro',
        'Gemini 1.5 Pro',
        ProviderType.Google,
        [Capability.LongContext, Capability.Vision, Capability.CodeAnalysis],
        1000000
      ),
      temperature: 0.5
    });
    
    // Security Auditor Template
    this.agentTemplates.set('security_auditor', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are a security expert. Your focus:
- Identify security vulnerabilities and risks
- Check for OWASP Top 10 issues
- Review authentication and authorization
- Analyze data handling and encryption
- Suggest security improvements`,
      model: this.createModelProfile(
        'gpt-4-turbo',
        'GPT-4 Turbo',
        ProviderType.OpenAI,
        [Capability.Reasoning, Capability.CodeAnalysis],
        128000
      ),
      temperature: 0.1
    });
    
    // Test Engineer Template
    this.agentTemplates.set('test_engineer', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are a test automation expert. Create:
- Comprehensive test strategies
- Unit, integration, and e2e tests
- Test data and fixtures
- Performance and load tests
- Test coverage analysis`,
      model: this.createModelProfile(
        'claude-3-sonnet',
        'Claude 3 Sonnet',
        ProviderType.Anthropic,
        [Capability.CodeGeneration, Capability.CodeAnalysis],
        200000
      ),
      temperature: 0.3
    });
    
    // Performance Optimizer Template
    this.agentTemplates.set('performance_optimizer', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are a performance optimization specialist. Focus on:
- Identifying performance bottlenecks
- Optimizing algorithms and data structures
- Database query optimization
- Caching strategies
- Resource utilization improvements`,
      model: this.createModelProfile(
        'gemini-1.5-flash',
        'Gemini 1.5 Flash',
        ProviderType.Google,
        [Capability.LongContext, Capability.Vision, Capability.CodeAnalysis],
        1000000
      ),
      temperature: 0.4
    });
    
    // Documentation Writer Template
    this.agentTemplates.set('documentation_writer', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are a technical documentation expert. Create:
- Clear and comprehensive documentation
- API documentation with examples
- Architecture and design documents
- User guides and tutorials
- README files and getting started guides`,
      model: this.createModelProfile(
        'gpt-4',
        'GPT-4',
        ProviderType.OpenAI,
        [Capability.CodeGeneration, Capability.NaturalLanguage],
        32768
      ),
      temperature: 0.6
    });
    
    // Research Analyst Template
    this.agentTemplates.set('research_analyst', {
      type: ADKAgentType.LLM,
      systemPrompt: `You are a technology research specialist. Provide:
- Technology evaluations and comparisons
- Best practices and industry standards
- Emerging trends and innovations
- Tool and framework recommendations
- Technical feasibility analysis`,
      model: this.createModelProfile(
        'perplexity-llama-3.1-sonar-large-128k-online',
        'Perplexity Sonar',
        ProviderType.Custom,
        [Capability.Reasoning, Capability.CodeAnalysis],
        128000
      ),
      temperature: 0.7
    });
  }
  
  /**
   * Create a specialized agent based on role requirements
   */
  async createSpecializedAgent(role: AgentRole): Promise<Agent> {
    this.logger.info(`Creating specialized agent for role: ${role.name}`);
    
    try {
      // Check if we have a template
      const templateKey = role.id.replace('role_', '');
      const template = this.agentTemplates.get(templateKey);
      
      if (template) {
        // Use template as base
        const config: ADKAgentConfig = {
          id: `agent_${role.id}_${Date.now()}`,
          name: role.name,
          type: template.type || ADKAgentType.LLM,
          description: `Specialized agent for ${role.name}`,
          model: template.model as ModelProfile,
          systemPrompt: template.systemPrompt,
          temperature: template.temperature,
          capabilities: {
            streaming: true,
            multiModal: false,
            toolUse: true,
            memoryAccess: true,
            a2aProtocol: true,
            mcpTools: true
          }
        };
        
        // Add role-specific tools
        config.tools = await this.determineTools(role);
        
        return await this.agentFactory.createAgent(config);
      } else {
        // Create custom agent
        return await this.createCustomAgent({
          name: role.name,
          description: `Agent for ${role.capabilities.join(', ')}`,
          capabilities: role.capabilities
        });
      }
      
    } catch (error) {
      this.logger.error(`Failed to create agent for role ${role.name}`, error);
      throw error;
    }
  }
  
  /**
   * Create a completely custom agent based on specifications
   */
  async createCustomAgent(specification: {
    name: string;
    description: string;
    capabilities: string[];
    preferredModel?: string;
    tools?: string[];
  }): Promise<Agent> {
    this.logger.info(`Creating custom agent: ${specification.name}`);
    
    try {
      // Ask the creator agent to design the configuration
      const designPrompt = `Design an agent configuration for:
      
Name: ${specification.name}
Description: ${specification.description}
Required Capabilities: ${specification.capabilities.join(', ')}
Preferred Model: ${specification.preferredModel || 'auto-select'}
Required Tools: ${specification.tools?.join(', ') || 'auto-determine'}

Return a complete ADKAgentConfig as JSON including:
- Appropriate model selection
- Optimized system prompt
- Temperature and other parameters
- Required tools configuration`;

      const design = await this.creatorAgent.execute({ input: designPrompt });
      
      // Parse the design
      const config = this.parseAgentDesign(design, specification);
      
      // Create the agent
      return await this.agentFactory.createAgent(config);
      
    } catch (error) {
      this.logger.error(`Failed to create custom agent ${specification.name}`, error);
      throw error;
    }
  }
  
  /**
   * Create a workflow agent that coordinates multiple agents
   */
  async createWorkflowAgent(specification: {
    name: string;
    description: string;
    workflow: 'sequential' | 'parallel' | 'conditional' | 'loop';
    steps: Array<{
      agentType: string;
      input: any;
      condition?: string;
    }>;
  }): Promise<Agent> {
    this.logger.info(`Creating workflow agent: ${specification.name}`);
    
    const config: ADKAgentConfig = {
      id: `workflow_${specification.name.toLowerCase().replace(/\s+/g, '_')}_${Date.now()}`,
      name: specification.name,
      type: this.getWorkflowType(specification.workflow),
      description: specification.description,
      capabilities: {
        streaming: false,
        multiModal: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: false
      }
    };
    
    // Create sub-agents for workflow steps
    const subAgents: Agent[] = [];
    for (const step of specification.steps) {
      const subAgent = await this.createSpecializedAgent({
        id: `step_${step.agentType}`,
        name: step.agentType,
        capabilities: [step.agentType],
        priority: 'high'
      });
      subAgents.push(subAgent);
    }
    
    // Configure workflow with sub-agents
    config.workflow = {
      type: specification.workflow,
      agents: subAgents.map(a => a.id),
      steps: specification.steps.map((step, index) => ({
        name: step.agentType,
        agentId: subAgents[index].id,
        input: step.input,
        condition: step.condition
      }))
    };
    
    return await this.agentFactory.createAgent(config);
  }
  
  /**
   * Clone an existing agent with modifications
   */
  async cloneAgent(
    sourceAgentId: string, 
    modifications: Partial<ADKAgentConfig>
  ): Promise<Agent> {
    this.logger.info(`Cloning agent ${sourceAgentId}`);
    
    // Get source agent configuration
    const sourceAgent = this.agentFactory.getAgent(sourceAgentId);
    if (!sourceAgent) {
      throw new Error(`Source agent not found: ${sourceAgentId}`);
    }
    
    // Create new configuration
    const config: ADKAgentConfig = {
      id: `${sourceAgentId}_clone_${Date.now()}`,
      name: `${modifications.name || sourceAgent.name} (Clone)`,
      type: modifications.type || ADKAgentType.LLM,
      capabilities: modifications.capabilities || {
        streaming: true,
        multiModal: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true
      },
      model: modifications.model,
      systemPrompt: modifications.systemPrompt,
      temperature: modifications.temperature,
      maxTokens: modifications.maxTokens,
      tools: modifications.tools,
      memory: modifications.memory,
      a2aConfig: modifications.a2aConfig
    };
    
    return await this.agentFactory.createAgent(config);
  }
  
  /**
   * Create an agent optimized for specific constraints
   */
  async createConstrainedAgent(constraints: {
    maxCost: number;
    maxLatency: number;
    minAccuracy: number;
    requiredCapabilities: string[];
  }): Promise<Agent> {
    this.logger.info('Creating constrained agent', constraints);
    
    // Select optimal model based on constraints
    const model = this.selectOptimalModel(constraints);
    
    const config: ADKAgentConfig = {
      id: `constrained_agent_${Date.now()}`,
      name: 'Constrained Agent',
      type: ADKAgentType.LLM,
      description: 'Agent optimized for specific constraints',
      model,
      temperature: constraints.minAccuracy > 0.9 ? 0.1 : 0.5,
      maxTokens: this.calculateMaxTokens(model, constraints.maxCost),
      capabilities: {
        streaming: constraints.maxLatency > 5000,
        multiModal: false,
        toolUse: true,
        memoryAccess: constraints.maxCost > 0.1,
        a2aProtocol: true,
        mcpTools: true
      }
    };
    
    return await this.agentFactory.createAgent(config);
  }
  
  /**
   * Create a learning agent that improves over time
   */
  async createLearningAgent(specification: {
    name: string;
    domain: string;
    learningStrategy: 'reinforcement' | 'supervised' | 'few-shot';
  }): Promise<Agent> {
    const config: ADKAgentConfig = {
      id: `learning_agent_${specification.domain}_${Date.now()}`,
      name: specification.name,
      type: ADKAgentType.LLM,
      description: `Learning agent for ${specification.domain}`,
      systemPrompt: this.generateLearningPrompt(specification),
      model: this.createModelProfile(
        'gemini-1.5-pro',
        'Gemini 1.5 Pro',
        ProviderType.Google,
        [Capability.LongContext, Capability.Vision, Capability.CodeAnalysis],
        1000000
      ),
      memory: {
        enabled: true,
        type: 'long_term' as const,
        provider: 'mem0' as const,
        scope: 'agent' as const
      },
      capabilities: {
        streaming: true,
        multiModal: true,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true
      }
    };
    
    return await this.agentFactory.createAgent(config);
  }
  
  // Helper methods
  
  private async determineTools(role: AgentRole): Promise<ADKTool[]> {
    const tools: ADKTool[] = [];
    
    // Map capabilities to tools
    const toolMapping: Record<string, ADKTool> = {
      'code_generation': {
        name: 'code_writer',
        description: 'Write and modify code',
        type: 'custom',
        parameters: { schema: {} },
        implementation: { source: 'code', handler: async () => {} }
      },
      'code_review': {
        name: 'code_analyzer',
        description: 'Analyze code quality',
        type: 'custom',
        parameters: { schema: {} },
        implementation: { source: 'code', handler: async () => {} }
      },
      'testing': {
        name: 'test_runner',
        description: 'Run and create tests',
        type: 'custom',
        parameters: { schema: {} },
        implementation: { source: 'code', handler: async () => {} }
      },
      'security_audit': {
        name: 'security_scanner',
        description: 'Scan for vulnerabilities',
        type: 'custom',
        parameters: { schema: {} },
        implementation: { source: 'code', handler: async () => {} }
      },
      'documentation': {
        name: 'doc_generator',
        description: 'Generate documentation',
        type: 'custom',
        parameters: { schema: {} },
        implementation: { source: 'code', handler: async () => {} }
      }
    };
    
    for (const capability of role.capabilities) {
      const tool = toolMapping[capability];
      if (tool) {
        tools.push(tool);
      }
    }
    
    return tools;
  }
  
  private parseAgentDesign(design: any, specification: any): ADKAgentConfig {
    try {
      const parsed = JSON.parse(design.output);
      
      // Ensure required fields
      return {
        id: `custom_${specification.name.toLowerCase().replace(/\s+/g, '_')}_${Date.now()}`,
        name: specification.name,
        type: ADKAgentType.LLM,
        description: specification.description,
        ...parsed,
        capabilities: {
          streaming: true,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: true,
          ...parsed.capabilities
        }
      };
    } catch (error) {
      // Fallback configuration
      return {
        id: `custom_${specification.name.toLowerCase().replace(/\s+/g, '_')}_${Date.now()}`,
        name: specification.name,
        type: ADKAgentType.LLM,
        description: specification.description,
        systemPrompt: `You are ${specification.name}. ${specification.description}`,
        model: this.createModelProfile(
          'gemini-1.5-flash',
          'Gemini 1.5 Flash',
          ProviderType.Google,
          [Capability.LongContext, Capability.Vision, Capability.CodeAnalysis],
          1000000
        ),
        temperature: 0.5,
        capabilities: {
          streaming: true,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: true
        }
      };
    }
  }
  
  private getWorkflowType(workflow: string): ADKAgentType {
    switch (workflow) {
      case 'sequential': return ADKAgentType.Sequential;
      case 'parallel': return ADKAgentType.Parallel;
      case 'loop': return ADKAgentType.Loop;
      default: return ADKAgentType.Sequential;
    }
  }
  
  private selectOptimalModel(constraints: any): ModelProfile {
    // Model selection logic based on constraints
    // This is simplified - would be more sophisticated in practice
    
    if (constraints.maxCost < 0.01) {
      // Very low cost - use small model
      return this.createModelProfile(
        'gemini-1.5-flash',
        'Gemini 1.5 Flash',
        ProviderType.Google,
        [Capability.LongContext, Capability.Vision, Capability.CodeAnalysis],
        1000000
      );
    } else if (constraints.minAccuracy > 0.95) {
      // High accuracy required - use best model
      return this.createModelProfile(
        'claude-3-opus',
        'Claude 3 Opus',
        ProviderType.Anthropic,
        [Capability.CodeGeneration, Capability.CodeAnalysis],
        200000
      );
    } else {
      // Balanced choice
      return this.createModelProfile(
        'gpt-4-turbo',
        'GPT-4 Turbo',
        ProviderType.OpenAI,
        [Capability.Reasoning, Capability.CodeAnalysis],
        128000
      );
    }
  }
  
  private calculateMaxTokens(model: ModelProfile, maxCost: number): number {
    // Calculate max tokens based on model pricing
    // This is simplified - would use actual pricing data
    const costPerToken = 0.00001; // Example rate
    return Math.min(
      Math.floor(maxCost / costPerToken),
      model.maxOutputTokens || 100000
    );
  }
  
  private generateLearningPrompt(specification: any): string {
    const strategies = {
      reinforcement: 'Learn from feedback and improve responses based on rewards',
      supervised: 'Learn from examples and apply patterns to new situations',
      'few-shot': 'Adapt quickly from a small number of examples'
    };
    
    return `You are a learning agent specialized in ${specification.domain}.
Your learning strategy: ${strategies[specification.learningStrategy as keyof typeof strategies]}

Key behaviors:
1. Track patterns in successful interactions
2. Adapt responses based on feedback
3. Improve accuracy over time
4. Maintain memory of past experiences
5. Apply learned knowledge to new situations`;
  }
}