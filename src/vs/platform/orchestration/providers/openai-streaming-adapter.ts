/**
 * OpenAI Streaming Adapter
 * 
 * Example implementation showing how to handle both streaming and non-streaming OpenAI models
 */

import { ModelProfile, AITask } from '../interfaces';
import { StreamingAdapter } from './streaming-adapter';
import { StreamingAdapterOptions } from './streaming-adapter';

export class OpenAIStreamingAdapter extends StreamingAdapter {
  protected getApiKeyFromEnv(): string {
    return process.env.OPENAI_API_KEY || '';
  }
  
  protected getDefaultBaseURL(): string {
    return 'https://api.openai.com/v1';
  }
  
  protected getEndpoint(model: ModelProfile): string {
    return '/chat/completions';
  }
  
  /**
   * Check if model supports streaming
   */
  protected doesModelSupportStreaming(model: ModelProfile): boolean {
    // Most OpenAI models support streaming
    const streamingModels = [
      'gpt-4',
      'gpt-4-turbo',
      'gpt-4-turbo-preview',
      'gpt-3.5-turbo',
      'gpt-3.5-turbo-16k'
    ];
    
    return streamingModels.some(m => model.name.includes(m));
  }
  
  /**
   * Format request for OpenAI
   */
  protected formatRequest(
    task: AITask,
    model: ModelProfile,
    options?: StreamingAdapterOptions
  ): any {
    const messages = this.formatMessages(task);
    
    const request: any = {
      model: model.name,
      messages,
      temperature: task.parameters?.temperature ?? 0.7,
      max_tokens: task.parameters?.maxTokens ?? 2000,
      stream: options?.stream || false
    };
    
    if (task.parameters?.topP !== undefined) {
      request.top_p = task.parameters.topP;
    }
    
    if (task.parameters?.stopSequences) {
      request.stop = task.parameters.stopSequences;
    }
    
    if (task.parameters?.responseFormat) {
      request.response_format = { type: 'json_object' };
    }
    
    return request;
  }
  
  /**
   * Format messages
   */
  private formatMessages(task: AITask): any[] {
    const messages: any[] = [];
    
    // Add system message if context provided
    if (task.context?.systemPrompt) {
      messages.push({
        role: 'system',
        content: task.context.systemPrompt
      });
    }
    
    // Add conversation history
    if (task.context?.previousMessages) {
      messages.push(...task.context.previousMessages);
    }
    
    // Add main prompt
    messages.push({
      role: 'user',
      content: task.prompt
    });
    
    return messages;
  }
  
  /**
   * Parse non-streaming response
   */
  protected parseResponse(
    response: any,
    task: AITask,
    model: ModelProfile,
    startTime: number
  ): any {
    const content = response.choices[0]?.message?.content || '';
    
    return {
      taskId: task.id,
      status: 'success',
      response: content,
      model: response.model,
      usage: {
        promptTokens: response.usage?.prompt_tokens || 0,
        completionTokens: response.usage?.completion_tokens || 0,
        totalTokens: response.usage?.total_tokens || 0
      },
      cost: this.calculateCost({
        promptTokens: response.usage?.prompt_tokens || 0,
        completionTokens: response.usage?.completion_tokens || 0
      }, model),
      latency: Date.now() - startTime,
      timestamp: Date.now(),
      metadata: {
        finishReason: response.choices[0]?.finish_reason,
        modelVersion: response.model
      }
    };
  }
  
  /**
   * Parse streaming chunk
   */
  protected parseChunk(line: string): any {
    if (line.startsWith('data: ')) {
      const data = line.slice(6);
      
      if (data === '[DONE]') {
        return { finished: true };
      }
      
      try {
        const parsed = JSON.parse(data);
        const delta = parsed.choices[0]?.delta;
        
        if (delta?.content) {
          return {
            content: delta.content,
            role: delta.role,
            model: parsed.model
          };
        }
        
        if (parsed.choices[0]?.finish_reason) {
          return {
            finished: true,
            finishReason: parsed.choices[0].finish_reason
          };
        }
      } catch (e) {
        // Ignore parsing errors
      }
    }
    
    return null;
  }
  
  /**
   * Handle errors
   */
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
    
    if (error.status === 401) {
      throw {
        code: 'INVALID_API_KEY',
        message: 'Invalid OpenAI API key',
        type: 'invalid_request',
        retryable: false,
        details: error
      };
    }
    
    if (error.status >= 500) {
      throw {
        code: 'SERVER_ERROR',
        message: 'OpenAI server error',
        type: 'server_error',
        retryable: true,
        details: error
      };
    }
    
    throw {
      code: 'OPENAI_ERROR',
      message: error.message || 'Unknown OpenAI error',
      type: 'unknown',
      retryable: false,
      details: error
    };
  }
}

// Example usage showing flexibility:
/*
const adapter = new OpenAIStreamingAdapter({
  wsServer: websocketServer
});

// For a streaming model (e.g., GPT-4)
const streamingResult = await adapter.execute(task, gpt4Model, {
  stream: true,
  clientId: 'user123',
  callbacks: {
    onProgress: (progress) => {
      console.log('Streaming:', progress.message);
    }
  }
});

// For a non-streaming model or when streaming is disabled
const nonStreamingResult = await adapter.execute(task, gpt4Model, {
  stream: false
});

// For a model that doesn't support streaming but we want to simulate it
const simulatedResult = await adapter.execute(task, nonStreamingModel, {
  stream: true,
  simulateStreaming: true,
  chunkSize: 100, // 100 chars per chunk
  chunkDelay: 30, // 30ms between chunks
  clientId: 'user123'
});
*/