/**
 * Orchestration Engine - Core orchestration functionality
 */

import { EventEmitter } from 'events';
import { 
  ModelConfig, 
  ModelCapability,
  OrchestrationRequest,
  OrchestrationResponse,
  ProviderType
} from './interfaces';

export interface OrchestrationEngineConfig {
  models: ModelConfig[];
  defaultModel?: string;
  maxRetries?: number;
  timeout?: number;
}

export interface ChatRequest {
  messages: Array<{
    role: 'system' | 'user' | 'assistant';
    content: string;
  }>;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  stream?: boolean;
}

export interface ChatResponse {
  content: string;
  model: string;
  usage?: {
    promptTokens: number;
    completionTokens: number;
    totalTokens: number;
  };
  metadata?: Record<string, any>;
}

export class OrchestrationEngine extends EventEmitter {
  private static instance: OrchestrationEngine;
  private config: OrchestrationEngineConfig;
  private modelRegistry: Map<string, ModelConfig> = new Map();
  private initialized: boolean = false;

  constructor(config: OrchestrationEngineConfig) {
    super();
    this.config = config;
    this.registerModels(config.models);
  }

  static getInstance(config?: OrchestrationEngineConfig): OrchestrationEngine {
    if (!OrchestrationEngine.instance && config) {
      OrchestrationEngine.instance = new OrchestrationEngine(config);
    }
    if (!OrchestrationEngine.instance) {
      throw new Error('OrchestrationEngine not initialized. Provide config on first call.');
    }
    return OrchestrationEngine.instance;
  }

  private registerModels(models: ModelConfig[]): void {
    for (const model of models) {
      this.modelRegistry.set(model.id, model);
    }
  }

  async initialize(): Promise<void> {
    // Initialize providers
    this.initialized = true;
    this.emit('initialized');
  }

  async processQuery(request: ChatRequest): Promise<ChatResponse> {
    if (!this.initialized) {
      throw new Error('Orchestration engine not initialized');
    }

    const modelId = request.model || this.config.defaultModel || this.getDefaultModel();
    const model = this.modelRegistry.get(modelId);

    if (!model) {
      throw new Error(`Model ${modelId} not found`);
    }

    // Simulate processing
    const response: ChatResponse = {
      content: 'Response from ' + modelId,
      model: modelId,
      usage: {
        promptTokens: 100,
        completionTokens: 50,
        totalTokens: 150
      }
    };

    this.emit('response', response);
    return response;
  }

  async process(request: OrchestrationRequest): Promise<OrchestrationResponse> {
    const chatRequest: ChatRequest = {
      messages: request.messages || [],
      model: request.modelId,
      temperature: request.temperature,
      maxTokens: request.maxTokens
    };

    const chatResponse = await this.processQuery(chatRequest);

    return {
      id: `resp_${Date.now()}`,
      modelId: chatResponse.model,
      content: chatResponse.content,
      usage: chatResponse.usage,
      metadata: chatResponse.metadata
    };
  }

  getAvailableModels(): ModelConfig[] {
    return Array.from(this.modelRegistry.values());
  }

  getModelCapabilities(modelId: string): ModelCapability[] {
    const model = this.modelRegistry.get(modelId);
    return model?.capabilities || [];
  }

  getModel(modelId: string): ModelConfig | undefined {
    return this.modelRegistry.get(modelId);
  }

  private getDefaultModel(): string {
    const models = Array.from(this.modelRegistry.keys());
    return models[0] || 'gpt-4';
  }

  async shutdown(): Promise<void> {
    this.initialized = false;
    this.emit('shutdown');
  }
}

export default OrchestrationEngine;