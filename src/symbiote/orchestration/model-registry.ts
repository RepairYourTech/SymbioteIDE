/**
 * Model Registry - Manages available AI models and their profiles
 */

import {
  ModelProfile,
  ProviderType,
  Capability,
  TaskType,
  ModelAvailability,
  RateLimit
} from './interfaces';

export class ModelRegistry {
  private models: Map<string, ModelProfile> = new Map();
  private providers: Map<ProviderType, Set<string>> = new Map();
  
  constructor() {
    this.initializeDefaultModels();
  }

  /**
   * Initialize with default model profiles
   */
  private initializeDefaultModels(): void {
    // Anthropic Models
    this.registerModel({
      id: 'claude-3-opus-20240229',
      provider: ProviderType.Anthropic,
      name: 'claude-3-opus',
      displayName: 'Claude 3 Opus',
      version: '20240229',
      capabilities: [
        Capability.CodeGeneration,
        Capability.CodeAnalysis,
        Capability.NaturalLanguage,
        Capability.Mathematics,
        Capability.Reasoning,
        Capability.CreativeWriting,
        Capability.LongContext
      ],
      contextWindow: 200000,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0.000015,
        output: 0.000075,
        currency: 'USD'
      },
      averageLatency: 2000,
      reliability: 0.98,
      specializations: ['code-generation', 'complex-reasoning', 'analysis'],
      rateLimit: {
        requestsPerMinute: 50,
        tokensPerMinute: 100000,
        requestsPerDay: 10000,
        tokensPerDay: 10000000
      },
      availability: {
        status: 'available',
        region: ['us', 'eu']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeGeneration, TaskType.CodeReview]
      }
    });

    this.registerModel({
      id: 'claude-3-sonnet-20240229',
      provider: ProviderType.Anthropic,
      name: 'claude-3-sonnet',
      displayName: 'Claude 3 Sonnet',
      version: '20240229',
      capabilities: [
        Capability.CodeGeneration,
        Capability.CodeAnalysis,
        Capability.NaturalLanguage,
        Capability.Mathematics,
        Capability.Reasoning,
        Capability.LongContext
      ],
      contextWindow: 200000,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0.000003,
        output: 0.000015,
        currency: 'USD'
      },
      averageLatency: 1500,
      reliability: 0.99,
      specializations: ['balanced-performance', 'code-review'],
      rateLimit: {
        requestsPerMinute: 100,
        tokensPerMinute: 200000,
        requestsPerDay: 20000,
        tokensPerDay: 20000000
      },
      availability: {
        status: 'available',
        region: ['us', 'eu', 'asia']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeReview, TaskType.Documentation]
      }
    });

    this.registerModel({
      id: 'claude-3-haiku-20240307',
      provider: ProviderType.Anthropic,
      name: 'claude-3-haiku',
      displayName: 'Claude 3 Haiku',
      version: '20240307',
      capabilities: [
        Capability.CodeGeneration,
        Capability.NaturalLanguage,
        Capability.Summarization,
        Capability.QuestionAnswering
      ],
      contextWindow: 200000,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0.00000025,
        output: 0.00000125,
        currency: 'USD'
      },
      averageLatency: 500,
      reliability: 0.99,
      specializations: ['fast-responses', 'simple-tasks'],
      rateLimit: {
        requestsPerMinute: 200,
        tokensPerMinute: 400000,
        requestsPerDay: 50000,
        tokensPerDay: 50000000
      },
      availability: {
        status: 'available',
        region: ['global']
      },
      metadata: {
        preferredForTasks: [TaskType.Documentation, TaskType.General]
      }
    });

    // OpenAI Models
    this.registerModel({
      id: 'gpt-4-turbo-preview',
      provider: ProviderType.OpenAI,
      name: 'gpt-4-turbo',
      displayName: 'GPT-4 Turbo',
      version: 'preview',
      capabilities: [
        Capability.CodeGeneration,
        Capability.CodeAnalysis,
        Capability.NaturalLanguage,
        Capability.Mathematics,
        Capability.Reasoning,
        Capability.FunctionCalling,
        Capability.Vision,
        Capability.LongContext
      ],
      contextWindow: 128000,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0.00001,
        output: 0.00003,
        currency: 'USD'
      },
      averageLatency: 2500,
      reliability: 0.97,
      specializations: ['code-generation', 'function-calling', 'vision'],
      rateLimit: {
        requestsPerMinute: 60,
        tokensPerMinute: 150000,
        requestsPerDay: 10000,
        tokensPerDay: 15000000
      },
      availability: {
        status: 'available',
        region: ['global']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeGeneration, TaskType.Testing]
      }
    });

    this.registerModel({
      id: 'gpt-4',
      provider: ProviderType.OpenAI,
      name: 'gpt-4',
      displayName: 'GPT-4',
      capabilities: [
        Capability.CodeGeneration,
        Capability.CodeAnalysis,
        Capability.NaturalLanguage,
        Capability.Mathematics,
        Capability.Reasoning,
        Capability.FunctionCalling
      ],
      contextWindow: 8192,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0.00003,
        output: 0.00006,
        currency: 'USD'
      },
      averageLatency: 3000,
      reliability: 0.98,
      specializations: ['high-quality-reasoning', 'complex-tasks'],
      rateLimit: {
        requestsPerMinute: 40,
        tokensPerMinute: 100000,
        requestsPerDay: 10000,
        tokensPerDay: 10000000
      },
      availability: {
        status: 'available',
        region: ['global']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeReview, TaskType.Refactoring]
      }
    });

    this.registerModel({
      id: 'gpt-3.5-turbo',
      provider: ProviderType.OpenAI,
      name: 'gpt-3.5-turbo',
      displayName: 'GPT-3.5 Turbo',
      capabilities: [
        Capability.CodeGeneration,
        Capability.NaturalLanguage,
        Capability.Summarization,
        Capability.QuestionAnswering,
        Capability.FunctionCalling
      ],
      contextWindow: 16384,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0.0000005,
        output: 0.0000015,
        currency: 'USD'
      },
      averageLatency: 800,
      reliability: 0.99,
      specializations: ['fast-responses', 'general-tasks'],
      rateLimit: {
        requestsPerMinute: 200,
        tokensPerMinute: 400000,
        requestsPerDay: 100000,
        tokensPerDay: 100000000
      },
      availability: {
        status: 'available',
        region: ['global']
      },
      metadata: {
        preferredForTasks: [TaskType.General, TaskType.Documentation]
      }
    });

    // Google Models
    this.registerModel({
      id: 'gemini-pro',
      provider: ProviderType.Google,
      name: 'gemini-pro',
      displayName: 'Gemini Pro',
      capabilities: [
        Capability.CodeGeneration,
        Capability.CodeAnalysis,
        Capability.NaturalLanguage,
        Capability.Mathematics,
        Capability.Reasoning,
        Capability.Vision
      ],
      contextWindow: 32768,
      maxOutputTokens: 8192,
      costPerToken: {
        input: 0.0000005,
        output: 0.0000015,
        currency: 'USD'
      },
      averageLatency: 1200,
      reliability: 0.97,
      specializations: ['multimodal', 'reasoning'],
      rateLimit: {
        requestsPerMinute: 60,
        tokensPerMinute: 200000,
        requestsPerDay: 20000,
        tokensPerDay: 20000000
      },
      availability: {
        status: 'available',
        region: ['global'],
        restrictions: ['some-regions-restricted']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeGeneration, TaskType.Research]
      }
    });

    // Mistral Models
    this.registerModel({
      id: 'mistral-large',
      provider: ProviderType.Mistral,
      name: 'mistral-large',
      displayName: 'Mistral Large',
      capabilities: [
        Capability.CodeGeneration,
        Capability.NaturalLanguage,
        Capability.Mathematics,
        Capability.Reasoning
      ],
      contextWindow: 32768,
      maxOutputTokens: 8192,
      costPerToken: {
        input: 0.000004,
        output: 0.000012,
        currency: 'USD'
      },
      averageLatency: 1500,
      reliability: 0.96,
      specializations: ['european-languages', 'code-generation'],
      rateLimit: {
        requestsPerMinute: 100,
        tokensPerMinute: 200000,
        requestsPerDay: 20000,
        tokensPerDay: 20000000
      },
      availability: {
        status: 'available',
        region: ['eu', 'us']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeGeneration, TaskType.Translation]
      }
    });

    // Local Models (placeholder)
    this.registerModel({
      id: 'codellama-34b',
      provider: ProviderType.Local,
      name: 'codellama-34b',
      displayName: 'Code Llama 34B',
      capabilities: [
        Capability.CodeGeneration,
        Capability.CodeAnalysis
      ],
      contextWindow: 16384,
      maxOutputTokens: 4096,
      costPerToken: {
        input: 0,
        output: 0,
        currency: 'USD'
      },
      averageLatency: 2000,
      reliability: 0.95,
      specializations: ['code-generation', 'offline-capable'],
      rateLimit: {
        requestsPerMinute: 30,
        tokensPerMinute: 50000
      },
      availability: {
        status: 'available',
        region: ['local']
      },
      metadata: {
        preferredForTasks: [TaskType.CodeCompletion],
        notes: 'Requires local GPU with 24GB+ VRAM'
      }
    });
  }

  /**
   * Register a new model
   */
  registerModel(profile: ModelProfile): void {
    this.models.set(profile.id, profile);
    
    if (!this.providers.has(profile.provider)) {
      this.providers.set(profile.provider, new Set());
    }
    this.providers.get(profile.provider)!.add(profile.id);
  }

  /**
   * Get a model by ID
   */
  getModel(id: string): ModelProfile | undefined {
    return this.models.get(id);
  }

  /**
   * Get all models
   */
  getAllModels(): ModelProfile[] {
    return Array.from(this.models.values());
  }

  /**
   * Get models by provider
   */
  getModelsByProvider(provider: ProviderType): ModelProfile[] {
    const modelIds = this.providers.get(provider) || new Set();
    return Array.from(modelIds)
      .map(id => this.models.get(id))
      .filter((model): model is ModelProfile => model !== undefined);
  }

  /**
   * Get available models (not unavailable)
   */
  getAvailableModels(): ModelProfile[] {
    return this.getAllModels().filter(
      model => model.availability.status !== 'unavailable'
    );
  }

  /**
   * Get models that support specific capabilities
   */
  getModelsWithCapabilities(capabilities: Capability[]): ModelProfile[] {
    return this.getAvailableModels().filter(model =>
      capabilities.every(cap => model.capabilities.includes(cap))
    );
  }

  /**
   * Get models suitable for a task type
   */
  getModelsForTaskType(taskType: TaskType): ModelProfile[] {
    return this.getAvailableModels().filter(model => {
      // Check if model has explicit preference for this task type
      if (model.metadata?.preferredForTasks?.includes(taskType)) {
        return true;
      }
      
      // Otherwise, check capabilities
      switch (taskType) {
        case TaskType.CodeGeneration:
        case TaskType.CodeCompletion:
          return model.capabilities.includes(Capability.CodeGeneration);
        case TaskType.CodeReview:
        case TaskType.BugFix:
        case TaskType.Security:
          return model.capabilities.includes(Capability.CodeAnalysis);
        case TaskType.Documentation:
          return model.capabilities.includes(Capability.NaturalLanguage);
        case TaskType.Testing:
          return model.capabilities.includes(Capability.CodeGeneration) &&
                 model.capabilities.includes(Capability.Reasoning);
        case TaskType.Refactoring:
          return model.capabilities.includes(Capability.CodeAnalysis) &&
                 model.capabilities.includes(Capability.CodeGeneration);
        case TaskType.Translation:
          return model.capabilities.includes(Capability.Translation);
        case TaskType.Research:
          return model.capabilities.includes(Capability.Reasoning) &&
                 model.capabilities.includes(Capability.NaturalLanguage);
        default:
          return model.capabilities.includes(Capability.NaturalLanguage);
      }
    });
  }

  /**
   * Update model availability
   */
  updateModelAvailability(
    modelId: string,
    availability: Partial<ModelAvailability>
  ): void {
    const model = this.models.get(modelId);
    if (model) {
      model.availability = { ...model.availability, ...availability };
    }
  }

  /**
   * Update model load
   */
  updateModelLoad(modelId: string, load: number): void {
    const model = this.models.get(modelId);
    if (model) {
      model.currentLoad = load;
    }
  }

  /**
   * Get model statistics
   */
  getModelStats(): {
    totalModels: number;
    byProvider: Record<string, number>;
    available: number;
    degraded: number;
    unavailable: number;
  } {
    const models = this.getAllModels();
    const byProvider: Record<string, number> = {};
    
    for (const [provider, modelIds] of this.providers) {
      byProvider[provider] = modelIds.size;
    }
    
    return {
      totalModels: models.length,
      byProvider,
      available: models.filter(m => m.availability.status === 'available').length,
      degraded: models.filter(m => m.availability.status === 'degraded').length,
      unavailable: models.filter(m => m.availability.status === 'unavailable').length
    };
  }

  /**
   * Export model registry as JSON
   */
  exportRegistry(): string {
    const data = {
      models: Array.from(this.models.entries()),
      providers: Array.from(this.providers.entries()).map(([provider, ids]) => ({
        provider,
        modelIds: Array.from(ids)
      }))
    };
    return JSON.stringify(data, null, 2);
  }

  /**
   * Import model registry from JSON
   */
  importRegistry(json: string): void {
    const data = JSON.parse(json);
    
    this.models.clear();
    this.providers.clear();
    
    for (const [id, profile] of data.models) {
      this.registerModel(profile);
    }
  }
}