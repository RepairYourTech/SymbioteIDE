/**
 * Local Adapter - Local model integration (Ollama, llama.cpp, etc.)
 */

import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { ProviderAdapter, ProviderAdapterOptions } from './provider-adapter';

export class LocalAdapter extends ProviderAdapter {
  protected getApiKeyFromEnv(): string {
    // Local models typically don't need API keys
    return '';
  }

  protected getDefaultBaseURL(): string {
    // Default to Ollama's default port
    return process.env.LOCAL_MODEL_URL || 'http://localhost:11434';
  }

  async execute(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      // Check if using Ollama
      if (this.baseURL.includes(':11434')) {
        return this.executeOllama(task, model, options, startTime);
      }
      
      // Default to generic local model API
      const request = this.formatRequest(task, model, options);
      const response = await this.makeRequest(
        `${this.baseURL}/completions`,
        request,
        options
      );
      return this.parseResponse(response, task, model, startTime);
    } catch (error) {
      this.handleError(error);
    }
  }

  private async executeOllama(
    task: AITask,
    model: ModelProfile,
    options: ProviderAdapterOptions | undefined,
    startTime: number
  ): Promise<TaskResult> {
    const messages = this.buildMessages(task);
    const prompt = messages.map(m => `${m.role}: ${m.content}`).join('\n\n');
    
    const request = {
      model: model.name,
      prompt,
      stream: false,
      options: {
        temperature: task.constraints?.temperature,
        top_p: task.constraints?.topP,
        num_predict: task.constraints?.maxTokens || model.maxOutputTokens
      }
    };
    
    const response = await this.makeRequest(
      `${this.baseURL}/api/generate`,
      request,
      options
    );
    
    return this.parseOllamaResponse(response, task, model, startTime);
  }

  protected formatRequest(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): any {
    const messages = this.buildMessages(task);
    const prompt = messages.map(m => `${m.role}: ${m.content}`).join('\n\n');
    
    let request: any = {
      model: model.name,
      prompt,
      max_tokens: model.maxOutputTokens
    };
    
    request = this.applyConstraints(request, task);
    
    return request;
  }

  protected parseResponse(
    response: any,
    task: AITask,
    model: ModelProfile,
    startTime: number
  ): TaskResult {
    const content = response.text || response.response || '';
    
    // Estimate tokens for local models
    const usage = this.calculateTokenUsage(
      this.buildMessages(task).map(m => m.content).join('\n'),
      content,
      model
    );
    
    // Local models typically have no cost
    const cost = {
      amount: 0,
      currency: 'USD',
      breakdown: {
        inputTokens: usage.promptTokens,
        outputTokens: usage.completionTokens,
        inputCost: 0,
        outputCost: 0
      }
    };
    
    return {
      id: `result-${Date.now()}`,
      taskId: task.id,
      status: 'success',
      content,
      model: model.id,
      usage,
      cost,
      latency: Date.now() - startTime,
      timestamp: Date.now(),
      metadata: {
        localModel: true
      }
    };
  }

  private parseOllamaResponse(
    response: any,
    task: AITask,
    model: ModelProfile,
    startTime: number
  ): TaskResult {
    const content = response.response || '';
    
    // Ollama provides some token info
    const usage = {
      promptTokens: response.prompt_eval_count || 0,
      completionTokens: response.eval_count || 0,
      totalTokens: (response.prompt_eval_count || 0) + (response.eval_count || 0)
    };
    
    const cost = {
      amount: 0,
      currency: 'USD',
      breakdown: {
        inputTokens: usage.promptTokens,
        outputTokens: usage.completionTokens,
        inputCost: 0,
        outputCost: 0
      }
    };
    
    return {
      id: `result-${Date.now()}`,
      taskId: task.id,
      status: 'success',
      content,
      model: model.id,
      usage,
      cost,
      latency: Date.now() - startTime,
      timestamp: Date.now(),
      metadata: {
        localModel: true,
        totalDuration: response.total_duration,
        loadDuration: response.load_duration,
        evalDuration: response.eval_duration
      }
    };
  }

  protected handleError(error: any): never {
    if (error.code === 'ECONNREFUSED') {
      throw {
        code: 'LOCAL_MODEL_UNAVAILABLE',
        message: 'Local model server is not running',
        type: 'server_error',
        retryable: false,
        details: error
      };
    }
    
    throw {
      code: 'LOCAL_MODEL_ERROR',
      message: error.message || 'Unknown local model error',
      type: 'unknown',
      retryable: true,
      details: error
    };
  }
}