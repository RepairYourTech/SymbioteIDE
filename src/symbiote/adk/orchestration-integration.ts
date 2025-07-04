/**
 * Orchestration Integration
 * 
 * Integrates ADK agents with the existing SymbioteIDE orchestration engine
 */

import { EventEmitter } from 'events';
import { 
  ADKAgentConfig,
  AgentExecutionRequest,
  AgentExecutionResult,
  ADKAgentType
} from './types';
import { ADKOrchestrator, getADKOrchestrator } from './orchestrator';
import { OrchestrationEngine } from '../orchestration/engine';
import { ModelProfile, ChatRequest, ChatResponse } from '../orchestration/interfaces';
import { Logger } from '../utils/logger';
import { Stream, Readable } from 'stream';

export class OrchestrationIntegration extends EventEmitter {
  private logger = new Logger('OrchestrationIntegration');
  private adkOrchestrator: ADKOrchestrator;
  private orchestrationEngine: OrchestrationEngine;
  private agentModelMap = new Map<string, string>(); // agentId -> modelId

  constructor() {
    super();
    this.adkOrchestrator = getADKOrchestrator({
      mcpEnabled: true,
      cacheResults: true,
      redis: true,
      monitoring: true
    });
    this.orchestrationEngine = OrchestrationEngine.getInstance();
  }

  /**
   * Initialize integration
   */
  async initialize(): Promise<void> {
    try {
      await this.adkOrchestrator.initialize();
      await this.setupEventHandlers();
      
      this.logger.info('Orchestration integration initialized');
    } catch (error) {
      this.logger.error('Failed to initialize integration', error);
      throw error;
    }
  }

  /**
   * Create ADK agent from model profile
   */
  async createAgentFromModel(
    modelId: string,
    options: Partial<ADKAgentConfig> = {}
  ): Promise<string> {
    try {
      const model = await this.orchestrationEngine.getModel(modelId);
      if (!model) {
        throw new Error(`Model ${modelId} not found`);
      }

      const agentConfig: ADKAgentConfig = {
        id: options.id || `agent_${modelId}_${Date.now()}`,
        name: options.name || `${model.name} Agent`,
        type: options.type || ADKAgentType.LLM,
        description: options.description || `Agent powered by ${model.name}`,
        model,
        capabilities: {
          streaming: model.capabilities.streaming,
          multiModal: model.capabilities.multimodal || false,
          toolUse: model.capabilities.functionCalling || false,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: true
        },
        systemPrompt: options.systemPrompt,
        temperature: options.temperature || model.defaultParams?.temperature,
        maxTokens: options.maxTokens || model.defaultParams?.maxTokens,
        tools: options.tools,
        memory: options.memory,
        a2aConfig: options.a2aConfig
      };

      const agentId = await this.adkOrchestrator.createAgent(agentConfig);
      this.agentModelMap.set(agentId, modelId);
      
      return agentId;
    } catch (error) {
      this.logger.error('Failed to create agent from model', error);
      throw error;
    }
  }

  /**
   * Execute agent via orchestration engine
   */
  async executeViaOrchestration(
    agentId: string,
    request: ChatRequest
  ): Promise<ChatResponse> {
    try {
      const modelId = this.agentModelMap.get(agentId);
      if (!modelId) {
        throw new Error(`No model mapping found for agent ${agentId}`);
      }

      // Convert to ADK execution request
      const adkRequest: AgentExecutionRequest = {
        agentId,
        input: {
          messages: request.messages
        },
        context: request.context || {},
        streaming: request.stream
      };

      // Execute via ADK
      const result = await this.adkOrchestrator.executeAgent(adkRequest);
      
      // Convert result to orchestration response
      if (request.stream) {
        return this.createStreamingResponse(result);
      } else {
        return this.createNonStreamingResponse(result);
      }
    } catch (error) {
      this.logger.error('Failed to execute via orchestration', error);
      throw error;
    }
  }

  /**
   * Execute orchestration request via ADK agent
   */
  async executeOrchestrationRequest(
    request: ChatRequest
  ): Promise<ChatResponse> {
    try {
      // Check if we have an ADK agent for this model
      const modelId = request.model || 'default';
      let agentId = this.findAgentForModel(modelId);
      
      if (!agentId && modelId) {
        // Create agent on-demand
        agentId = await this.createAgentFromModel(modelId);
      }
      
      if (!agentId) {
        throw new Error('No model specified and no default agent available');
      }

      return this.executeViaOrchestration(agentId, request);
    } catch (error) {
      this.logger.error('Failed to execute orchestration request', error);
      throw error;
    }
  }

  /**
   * Create workflow from orchestration chain
   */
  async createWorkflowFromChain(
    chainId: string,
    steps: Array<{ modelId: string; prompt: string; tools?: any[] }>
  ): Promise<string> {
    try {
      const workflowSteps: ADKAgentConfig[] = [];
      
      for (const step of steps) {
        const agentConfig: Partial<ADKAgentConfig> = {
          id: `step_${chainId}_${workflowSteps.length}`,
          name: `Step ${workflowSteps.length + 1}`,
          type: ADKAgentType.LLM,
          systemPrompt: step.prompt,
          tools: step.tools
        };
        
        const fullConfig = await this.createAgentConfigFromModel(
          step.modelId,
          agentConfig
        );
        
        workflowSteps.push(fullConfig);
      }

      return this.adkOrchestrator.createWorkflow(chainId, workflowSteps);
    } catch (error) {
      this.logger.error('Failed to create workflow from chain', error);
      throw error;
    }
  }

  /**
   * Get agent metrics
   */
  async getAgentMetrics(agentId: string): Promise<any> {
    const agent = this.adkOrchestrator.getAgent(agentId);
    if (!agent) {
      throw new Error(`Agent ${agentId} not found`);
    }

    // TODO: Implement detailed metrics collection
    return {
      agentId,
      modelId: this.agentModelMap.get(agentId),
      status: 'active',
      metrics: this.adkOrchestrator.getMetrics()
    };
  }

  /**
   * Setup event handlers
   */
  private async setupEventHandlers(): Promise<void> {
    // Forward ADK events to orchestration engine
    this.adkOrchestrator.on('execution:started', (data) => {
      this.orchestrationEngine.emit('request:started', {
        requestId: data.executionId,
        modelId: this.agentModelMap.get(data.agentId),
        ...data
      });
    });

    this.adkOrchestrator.on('execution:completed', (data) => {
      this.orchestrationEngine.emit('request:completed', {
        requestId: data.executionId,
        modelId: this.agentModelMap.get(data.result.agentId),
        ...data
      });
    });

    this.adkOrchestrator.on('execution:failed', (data) => {
      this.orchestrationEngine.emit('request:failed', {
        requestId: data.executionId,
        error: data.error
      });
    });
  }

  /**
   * Find agent for model
   */
  private findAgentForModel(modelId: string): string | undefined {
    for (const [agentId, mappedModelId] of this.agentModelMap) {
      if (mappedModelId === modelId) {
        return agentId;
      }
    }
    return undefined;
  }

  /**
   * Create agent config from model
   */
  private async createAgentConfigFromModel(
    modelId: string,
    overrides: Partial<ADKAgentConfig> = {}
  ): Promise<ADKAgentConfig> {
    const model = await this.orchestrationEngine.getModel(modelId);
    if (!model) {
      throw new Error(`Model ${modelId} not found`);
    }

    return {
      id: overrides.id || `agent_${modelId}_${Date.now()}`,
      name: overrides.name || `${model.name} Agent`,
      type: overrides.type || ADKAgentType.LLM,
      description: overrides.description || `Agent powered by ${model.name}`,
      model,
      capabilities: {
        streaming: model.capabilities.streaming,
        multiModal: model.capabilities.multimodal || false,
        toolUse: model.capabilities.functionCalling || false,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true
      },
      systemPrompt: overrides.systemPrompt,
      temperature: overrides.temperature || model.defaultParams?.temperature,
      maxTokens: overrides.maxTokens || model.defaultParams?.maxTokens,
      tools: overrides.tools,
      memory: overrides.memory,
      a2aConfig: overrides.a2aConfig
    };
  }

  /**
   * Create streaming response
   */
  private createStreamingResponse(result: AgentExecutionResult): ChatResponse {
    // Create a readable stream
    const stream = new Readable({
      read() {}
    });

    // Push result data to stream
    setImmediate(() => {
      if (result.result) {
        stream.push(JSON.stringify({
          type: 'content',
          content: result.result
        }) + '\n');
      }

      if (result.artifacts) {
        stream.push(JSON.stringify({
          type: 'artifacts',
          artifacts: result.artifacts
        }) + '\n');
      }

      stream.push(null); // End stream
    });

    // For streaming, we'll need to handle this differently
    // For now, return the first chunk
    return new Promise<ChatResponse>((resolve) => {
      stream.once('data', (chunk: any) => {
        resolve({
          content: chunk,
          model: result.agentId,
          usage: result.metadata.tokensUsed ? {
            inputTokens: result.metadata.tokensUsed,
            outputTokens: 0,
            totalTokens: result.metadata.tokensUsed
          } : undefined
        });
      });
    });
  }

  /**
   * Create non-streaming response
   */
  private createNonStreamingResponse(result: AgentExecutionResult): ChatResponse {
    return {
      content: result.result || '',
      model: result.agentId,
      usage: result.metadata.tokensUsed ? {
        inputTokens: result.metadata.tokensUsed,
        outputTokens: 0,
        totalTokens: result.metadata.tokensUsed
      } : undefined
    };
  }

  /**
   * Shutdown integration
   */
  async shutdown(): Promise<void> {
    await this.adkOrchestrator.shutdown();
    this.removeAllListeners();
    this.logger.info('Orchestration integration shutdown');
  }
}

// Export singleton instance
let integrationInstance: OrchestrationIntegration | null = null;

export function getOrchestrationIntegration(): OrchestrationIntegration {
  if (!integrationInstance) {
    integrationInstance = new OrchestrationIntegration();
  }
  return integrationInstance;
}