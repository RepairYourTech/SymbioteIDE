/**
 * OpenAI Adapter - GPT API integration
 */

import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { ProviderAdapter, ProviderAdapterOptions } from './provider-adapter';

export class OpenAIAdapter extends ProviderAdapter {
  protected getApiKeyFromEnv(): string {
    return process.env.OPENAI_API_KEY || '';
  }

  protected getDefaultBaseURL(): string {
    return 'https://api.openai.com/v1';
  }

  async execute(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      const request = this.formatRequest(task, model, options);
      const response = await this.makeRequest(
        `${this.baseURL}/chat/completions`,
        request,
        options
      );
      return this.parseResponse(response, task, model, startTime);
    } catch (error) {
      this.handleError(error);
    }
  }

  protected formatRequest(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): any {
    const messages = this.buildMessages(task);
    
    let request: any = {
      model: model.name,
      messages,
      max_tokens: model.maxOutputTokens
    };
    
    request = this.applyConstraints(request, task);
    
    if (options?.stream) {
      request.stream = true;
    }
    
    return request;
  }

  protected parseResponse(
    response: any,
    task: AITask,
    model: ModelProfile,
    startTime: number
  ): TaskResult {
    const choice = response.choices?.[0];
    const content = choice?.message?.content || '';
    const usage = {
      promptTokens: response.usage?.prompt_tokens || 0,
      completionTokens: response.usage?.completion_tokens || 0,
      totalTokens: response.usage?.total_tokens || 0
    };
    
    const cost = this.calculateCost(usage, model);
    
    return {
      id: response.id || `result-${Date.now()}`,
      taskId: task.id,
      status: 'success',
      content,
      model: model.id,
      usage,
      cost,
      latency: Date.now() - startTime,
      timestamp: Date.now(),
      metadata: {
        finishReason: choice?.finish_reason
      }
    };
  }

  protected handleError(error: any): never {
    if (error.status === 429) {
      throw {
        code: 'RATE_LIMIT',
        message: 'OpenAI rate limit exceeded',
        type: 'rate_limit',
        retryable: true,
        details: error
      };
    }
    
    throw {
      code: 'OPENAI_ERROR',
      message: error.message || 'Unknown OpenAI error',
      type: 'unknown',
      retryable: true,
      details: error
    };
  }
}