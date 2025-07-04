/**
 * Anthropic Adapter - Claude API integration
 */

import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { ProviderAdapter, ProviderAdapterOptions } from './provider-adapter';

export class AnthropicAdapter extends ProviderAdapter {
  protected getApiKeyFromEnv(): string {
    return process.env.ANTHROPIC_API_KEY || '';
  }

  protected getDefaultBaseURL(): string {
    return 'https://api.anthropic.com/v1';
  }

  async execute(
    task: AITask,
    model: ModelProfile,
    options?: ProviderAdapterOptions
  ): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      // Format request for Anthropic API
      const request = this.formatRequest(task, model, options);
      
      // Make API call
      const response = await this.makeRequest(
        `${this.baseURL}/messages`,
        request,
        options
      );
      
      // Parse response
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
    
    // Anthropic expects system prompt separately
    const systemMessage = messages.find(m => m.role === 'system');
    const userMessages = messages.filter(m => m.role !== 'system');
    
    let request: any = {
      model: model.name,
      messages: userMessages.map(msg => ({
        role: msg.role,
        content: msg.content
      })),
      max_tokens: model.maxOutputTokens
    };
    
    if (systemMessage) {
      request.system = systemMessage.content;
    }
    
    // Apply constraints
    request = this.applyConstraints(request, task);
    
    // Add streaming if requested
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
    const content = response.content?.[0]?.text || '';
    const usage = {
      promptTokens: response.usage?.input_tokens || 0,
      completionTokens: response.usage?.output_tokens || 0,
      totalTokens: (response.usage?.input_tokens || 0) + (response.usage?.output_tokens || 0)
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
        stopReason: response.stop_reason,
        modelVersion: response.model
      }
    };
  }

  protected handleError(error: any): never {
    // Map Anthropic-specific errors
    if (error.status === 429) {
      throw {
        code: 'RATE_LIMIT',
        message: 'Rate limit exceeded',
        type: 'rate_limit',
        retryable: true,
        details: error
      };
    }
    
    if (error.status === 401) {
      throw {
        code: 'INVALID_API_KEY',
        message: 'Invalid API key',
        type: 'invalid_request',
        retryable: false,
        details: error
      };
    }
    
    if (error.status >= 500) {
      throw {
        code: 'SERVER_ERROR',
        message: 'Anthropic server error',
        type: 'server_error',
        retryable: true,
        details: error
      };
    }
    
    // Generic error
    throw {
      code: 'ANTHROPIC_ERROR',
      message: error.message || 'Unknown Anthropic error',
      type: 'unknown',
      retryable: true,
      details: error
    };
  }

  protected async handleStreamingResponse(
    stream: ReadableStream,
    options: ProviderAdapterOptions
  ): Promise<any> {
    const reader = stream.getReader();
    const decoder = new TextDecoder();
    let fullContent = '';
    let usage = { input_tokens: 0, output_tokens: 0 };
    let id = '';
    let model = '';
    let stopReason = '';

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split('\n');

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') continue;

            try {
              const parsed = JSON.parse(data);
              
              if (parsed.type === 'message_start') {
                id = parsed.message.id;
                model = parsed.message.model;
                usage = parsed.message.usage || usage;
              } else if (parsed.type === 'content_block_delta') {
                fullContent += parsed.delta.text || '';
              } else if (parsed.type === 'message_delta') {
                if (parsed.delta.stop_reason) {
                  stopReason = parsed.delta.stop_reason;
                }
                if (parsed.usage) {
                  usage = parsed.usage;
                }
              }

              // Call progress callback if provided
              if (options.callbacks?.onProgress) {
                options.callbacks.onProgress({
                  stage: 'executing',
                  message: fullContent
                });
              }
            } catch (e) {
              // Ignore parsing errors for incomplete chunks
            }
          }
        }
      }
    } finally {
      reader.releaseLock();
    }

    return {
      id,
      model,
      content: [{ type: 'text', text: fullContent }],
      stop_reason: stopReason,
      usage
    };
  }
}