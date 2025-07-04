/**
 * Google Adapter - Gemini API integration
 */

import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { ProviderAdapter, ProviderAdapterOptions } from './provider-adapter';

export class GoogleAdapter extends ProviderAdapter {
  protected getApiKeyFromEnv(): string {
    return process.env.GOOGLE_API_KEY || '';
  }

  protected getDefaultBaseURL(): string {
    return 'https://generativelanguage.googleapis.com/v1';
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
        `${this.baseURL}/models/${model.name}:generateContent?key=${this.apiKey}`,
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
    
    // Convert messages to Gemini format
    const contents = messages.map(msg => ({
      role: msg.role === 'assistant' ? 'model' : 'user',
      parts: [{ text: msg.content }]
    }));
    
    let request: any = {
      contents,
      generationConfig: {
        maxOutputTokens: model.maxOutputTokens
      }
    };
    
    // Apply constraints
    if (task.constraints?.temperature !== undefined) {
      request.generationConfig.temperature = task.constraints.temperature;
    }
    if (task.constraints?.topP !== undefined) {
      request.generationConfig.topP = task.constraints.topP;
    }
    
    return request;
  }

  protected parseResponse(
    response: any,
    task: AITask,
    model: ModelProfile,
    startTime: number
  ): TaskResult {
    const candidate = response.candidates?.[0];
    const content = candidate?.content?.parts?.[0]?.text || '';
    
    // Google doesn't provide token counts in response, estimate
    const usage = this.calculateTokenUsage(
      this.buildMessages(task).map(m => m.content).join('\n'),
      content,
      model
    );
    
    const cost = this.calculateCost(usage, model);
    
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
        finishReason: candidate?.finishReason
      }
    };
  }

  protected handleError(error: any): never {
    throw {
      code: 'GOOGLE_ERROR',
      message: error.message || 'Unknown Google AI error',
      type: 'unknown',
      retryable: true,
      details: error
    };
  }
}