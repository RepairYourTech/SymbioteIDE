/**
 * Provider Adapter - Base interface for AI provider adapters
 */

import { AITask, TaskResult, ModelProfile, ExecutionOptions } from '../interfaces';

export interface ProviderAdapterOptions extends ExecutionOptions {
  signal?: AbortSignal;
  apiKey?: string;
  baseURL?: string;
  headers?: Record<string, string>;
  credentialId?: string; // Optional credential ID for BYOK system
}

export abstract class ProviderAdapter {
  protected apiKey: string = '';
  protected baseURL: string = '';
  protected headers: Record<string, string> = {};
  protected credentialManager?: any; // Will be injected by credential system

  constructor(config?: {
    apiKey?: string;
    baseURL?: string;
    headers?: Record<string, string>;
    credentialManager?: any;
  }) {
    if (config) {
      this.apiKey = config.apiKey || this.getApiKeyFromEnv();
      this.baseURL = config.baseURL || this.getDefaultBaseURL();
      this.headers = config.headers || {};
      this.credentialManager = config.credentialManager;
    } else {
      this.apiKey = this.getApiKeyFromEnv();
      this.baseURL = this.getDefaultBaseURL();
    }
  }

  /**
   * Execute a task using this provider
   */
  abstract execute(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): Promise<TaskResult>;

  /**
   * Get API key from environment
   */
  protected abstract getApiKeyFromEnv(): string;

  /**
   * Get default base URL for the provider
   */
  protected abstract getDefaultBaseURL(): string;

  /**
   * Format the task for the provider's API
   */
  protected abstract formatRequest(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): any;

  /**
   * Parse the provider's response into TaskResult
   */
  protected abstract parseResponse(
    response: any,
    task: AITask,
    model: ModelProfile,
    startTime: number
  ): TaskResult;

  /**
   * Handle provider-specific errors
   */
  protected abstract handleError(error: any): never;

  /**
   * Shutdown the adapter (optional)
   */
  async shutdown?(): Promise<void>;

  /**
   * Common request execution logic
   */
  protected async makeRequest(
    url: string,
    body: any,
    options?: ProviderAdapterOptions
  ): Promise<any> {
    const controller = options?.signal ? undefined : new AbortController();
    const signal = options?.signal || controller?.signal;

    const timeout = options?.timeout || 60000;
    const timeoutId = setTimeout(() => {
      controller?.abort();
    }, timeout);

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.apiKey}`,
          ...this.headers,
          ...(options?.headers || {})
        },
        body: JSON.stringify(body),
        signal
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const error = await response.text();
        throw new Error(`Provider returned ${response.status}: ${error}`);
      }

      // Handle streaming responses if needed
      if (options?.stream && response.body) {
        return this.handleStreamingResponse(response.body, options);
      }

      return await response.json();
    } catch (error: any) {
      clearTimeout(timeoutId);
      
      if (error.name === 'AbortError') {
        throw {
          code: 'TIMEOUT',
          message: 'Request timed out',
          type: 'timeout',
          retryable: true
        };
      }
      
      throw error;
    }
  }

  /**
   * Handle streaming responses
   */
  protected async handleStreamingResponse(
    stream: ReadableStream,
    options: ProviderAdapterOptions
  ): Promise<any> {
    // Default implementation for non-streaming
    // Providers that support streaming should override this
    const reader = stream.getReader();
    const decoder = new TextDecoder();
    let result = '';

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        result += decoder.decode(value, { stream: true });
      }
    } finally {
      reader.releaseLock();
    }

    return JSON.parse(result);
  }

  /**
   * Calculate token usage (provider-specific)
   */
  protected calculateTokenUsage(
    input: string,
    output: string,
    model: ModelProfile
  ): { promptTokens: number; completionTokens: number; totalTokens: number } {
    // Simple estimation - providers should override with accurate counting
    const promptTokens = Math.ceil(input.length / 4);
    const completionTokens = Math.ceil(output.length / 4);
    
    return {
      promptTokens,
      completionTokens,
      totalTokens: promptTokens + completionTokens
    };
  }

  /**
   * Calculate actual cost
   */
  protected calculateCost(
    usage: { promptTokens: number; completionTokens: number },
    model: ModelProfile
  ): {
    amount: number;
    currency: string;
    breakdown: any;
  } {
    const inputCost = usage.promptTokens * model.costPerToken.input;
    const outputCost = usage.completionTokens * model.costPerToken.output;
    
    return {
      amount: inputCost + outputCost,
      currency: model.costPerToken.currency,
      breakdown: {
        inputTokens: usage.promptTokens,
        outputTokens: usage.completionTokens,
        inputCost,
        outputCost
      }
    };
  }

  /**
   * Build context from task
   */
  protected buildContext(task: AITask): string {
    let context = '';

    if (task.context?.files) {
      for (const file of task.context.files) {
        context += `\n--- File: ${file.path} ---\n`;
        context += file.content;
        context += '\n--- End of file ---\n';
      }
    }

    if (task.context?.codebase) {
      context += `\nProject: ${task.context.codebase.rootPath}\n`;
      if (task.context.codebase.dependencies) {
        context += `Dependencies: ${task.context.codebase.dependencies.join(', ')}\n`;
      }
    }

    return context;
  }

  /**
   * Build messages array from task
   */
  protected buildMessages(task: AITask): Array<{
    role: 'system' | 'user' | 'assistant';
    content: string;
  }> {
    const messages: Array<{ role: 'system' | 'user' | 'assistant'; content: string }> = [];

    // Add system message if needed
    const systemPrompt = this.getSystemPrompt(task);
    if (systemPrompt) {
      messages.push({ role: 'system', content: systemPrompt });
    }

    // Add previous messages if any
    if (task.context?.previousMessages) {
      messages.push(...task.context.previousMessages);
    }

    // Add context
    const context = this.buildContext(task);
    if (context) {
      messages.push({ role: 'user', content: context });
    }

    // Add main prompt
    messages.push({ role: 'user', content: task.prompt });

    return messages;
  }

  /**
   * Get system prompt based on task type
   */
  protected getSystemPrompt(task: AITask): string {
    switch (task.type) {
      case 'code-generation':
        return 'You are an expert programmer. Generate clean, efficient, and well-documented code.';
      case 'code-review':
        return 'You are a senior code reviewer. Analyze code for bugs, security issues, and improvements.';
      case 'documentation':
        return 'You are a technical writer. Create clear and comprehensive documentation.';
      case 'testing':
        return 'You are a QA engineer. Generate thorough test cases and test code.';
      case 'refactoring':
        return 'You are a software architect. Suggest and implement code improvements.';
      default:
        return 'You are a helpful AI assistant.';
    }
  }

  /**
   * Apply task constraints to request
   */
  protected applyConstraints(request: any, task: AITask): any {
    if (task.constraints) {
      if (task.constraints.temperature !== undefined) {
        request.temperature = task.constraints.temperature;
      }
      if (task.constraints.maxTokens !== undefined) {
        request.max_tokens = task.constraints.maxTokens;
      }
      if (task.constraints.topP !== undefined) {
        request.top_p = task.constraints.topP;
      }
      if (task.constraints.stopSequences) {
        request.stop = task.constraints.stopSequences;
      }
    }
    return request;
  }
}