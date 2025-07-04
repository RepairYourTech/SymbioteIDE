/**
 * Streaming Adapter
 * 
 * Enhanced provider adapter that gracefully handles both streaming and non-streaming models
 */

import { EventEmitter } from 'events';
import { AITask, TaskResult, ModelProfile } from '../interfaces';
import { ProviderAdapter, ProviderAdapterOptions } from './provider-adapter';
import { WebSocketServer } from '../../websocket/websocket-server';
import { StreamResponse, SocketEvent } from '../../websocket/types';
import { Logger } from '../../utils/logger';

export interface StreamingAdapterOptions extends ProviderAdapterOptions {
  wsServer?: WebSocketServer;
  clientId?: string;
  streamId?: string;
  simulateStreaming?: boolean; // For non-streaming models
  chunkSize?: number; // Size of chunks for simulation
  chunkDelay?: number; // Delay between chunks (ms)
}

export abstract class StreamingAdapter extends ProviderAdapter {
  protected logger: Logger;
  protected wsServer?: WebSocketServer;
  protected emitter = new EventEmitter();
  
  constructor(config?: any) {
    super(config);
    this.logger = new Logger(this.constructor.name);
    this.wsServer = config?.wsServer;
  }
  
  /**
   * Execute with streaming support
   */
  async execute(
    task: AITask,
    model: ModelProfile,
    options?: StreamingAdapterOptions
  ): Promise<TaskResult> {
    const startTime = Date.now();
    
    // Check if model supports streaming
    const supportsStreaming = this.doesModelSupportStreaming(model);
    const shouldStream = options?.stream && (supportsStreaming || options.simulateStreaming);
    
    try {
      let result: TaskResult;
      
      if (shouldStream) {
        // Create stream if WebSocket server available
        const streamId = options.streamId || this.createStreamId();
        if (this.wsServer && options.clientId) {
          this.wsServer.createStream(options.clientId, {
            model: model.name,
            task: task.type,
            streaming: supportsStreaming ? 'native' : 'simulated'
          });
        }
        
        if (supportsStreaming) {
          // Use native streaming
          result = await this.executeWithNativeStreaming(
            task, 
            model, 
            { ...options, streamId },
            startTime
          );
        } else {
          // Simulate streaming for non-streaming models
          result = await this.executeWithSimulatedStreaming(
            task,
            model,
            { ...options, streamId },
            startTime
          );
        }
        
        // End stream
        if (this.wsServer && streamId) {
          this.wsServer.endStream(streamId, {
            totalTokens: result.usage?.totalTokens,
            duration: result.metadata?.latency
          });
        }
      } else {
        // Regular non-streaming execution
        result = await this.executeNonStreaming(task, model, options, startTime);
      }
      
      return result;
      
    } catch (error: any) {
      // Handle streaming errors
      if (options?.streamId && this.wsServer) {
        this.wsServer.errorStream(options.streamId, error);
      }
      
      throw error;
    }
  }
  
  /**
   * Execute with native streaming
   */
  protected async executeWithNativeStreaming(
    task: AITask,
    model: ModelProfile,
    options: StreamingAdapterOptions,
    startTime: number
  ): Promise<TaskResult> {
    // Format request
    const requestBody = this.formatRequest(task, model, {
      ...options,
      stream: true
    });
    
    // Make streaming request
    const response = await this.makeStreamingRequest(
      `${this.baseURL}${this.getEndpoint(model)}`,
      requestBody,
      options
    );
    
    // Process stream
    let fullContent = '';
    let tokenCount = 0;
    
    for await (const chunk of this.readStream(response.body!, options)) {
      fullContent += chunk.content;
      tokenCount += chunk.tokens || 0;
      
      // Send to WebSocket
      if (this.wsServer && options.streamId) {
        this.sendStreamChunk(options.streamId, chunk);
      }
      
      // Emit progress
      if (options.callbacks?.onProgress) {
        options.callbacks.onProgress({
          stage: 'executing',
          message: fullContent,
          progress: undefined // Streaming doesn't have clear progress
        });
      }
    }
    
    // Create result
    return this.createStreamingResult(
      task,
      model,
      fullContent,
      tokenCount,
      startTime
    );
  }
  
  /**
   * Execute with simulated streaming
   */
  protected async executeWithSimulatedStreaming(
    task: AITask,
    model: ModelProfile,
    options: StreamingAdapterOptions,
    startTime: number
  ): Promise<TaskResult> {
    // Get non-streaming response
    const result = await this.executeNonStreaming(task, model, options, startTime);
    
    // Simulate streaming the response
    const content = result.response;
    const chunkSize = options.chunkSize || 50; // Characters per chunk
    const chunkDelay = options.chunkDelay || 50; // 50ms between chunks
    
    let sentContent = '';
    
    for (let i = 0; i < content.length; i += chunkSize) {
      const chunk = content.slice(i, Math.min(i + chunkSize, content.length));
      sentContent += chunk;
      
      // Send chunk
      if (this.wsServer && options.streamId) {
        this.sendStreamChunk(options.streamId, {
          content: chunk,
          tokens: Math.ceil(chunk.length / 4),
          isSimulated: true
        });
      }
      
      // Progress callback
      if (options.callbacks?.onProgress) {
        options.callbacks.onProgress({
          stage: 'executing',
          message: sentContent,
          progress: (i + chunk.length) / content.length
        });
      }
      
      // Delay between chunks
      if (i + chunkSize < content.length) {
        await new Promise(resolve => setTimeout(resolve, chunkDelay));
      }
    }
    
    return result;
  }
  
  /**
   * Execute without streaming
   */
  protected async executeNonStreaming(
    task: AITask,
    model: ModelProfile,
    options: StreamingAdapterOptions,
    startTime: number
  ): Promise<TaskResult> {
    // Format request
    const requestBody = this.formatRequest(task, model, {
      ...options,
      stream: false
    });
    
    // Make request
    const response = await this.makeRequest(
      `${this.baseURL}${this.getEndpoint(model)}`,
      requestBody,
      options
    );
    
    // Parse response
    return this.parseResponse(response, task, model, startTime);
  }
  
  /**
   * Check if model supports streaming
   */
  protected doesModelSupportStreaming(model: ModelProfile): boolean {
    // Override in specific adapters
    // Default to checking model capabilities
    return model.capabilities?.includes('streaming' as any) || false;
  }
  
  /**
   * Get API endpoint for model
   */
  protected abstract getEndpoint(model: ModelProfile): string;
  
  /**
   * Make streaming request
   */
  protected async makeStreamingRequest(
    url: string,
    body: any,
    options?: StreamingAdapterOptions
  ): Promise<Response> {
    const controller = options?.signal ? undefined : new AbortController();
    const signal = options?.signal || controller?.signal;
    
    const timeout = options?.timeout || 300000; // 5 minutes for streaming
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
      
      return response;
      
    } catch (error: any) {
      clearTimeout(timeoutId);
      
      if (error.name === 'AbortError') {
        throw {
          code: 'TIMEOUT',
          message: 'Streaming request timed out',
          type: 'timeout',
          retryable: true
        };
      }
      
      throw error;
    }
  }
  
  /**
   * Read stream chunks
   */
  protected async *readStream(
    stream: ReadableStream,
    options: StreamingAdapterOptions
  ): AsyncGenerator<any> {
    const reader = stream.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        
        buffer += decoder.decode(value, { stream: true });
        
        // Process complete chunks
        const chunks = this.extractChunks(buffer);
        buffer = chunks.remainder;
        
        for (const chunk of chunks.complete) {
          yield chunk;
        }
      }
      
      // Process any remaining buffer
      if (buffer) {
        const finalChunk = this.parseChunk(buffer);
        if (finalChunk) {
          yield finalChunk;
        }
      }
    } finally {
      reader.releaseLock();
    }
  }
  
  /**
   * Extract complete chunks from buffer
   */
  protected extractChunks(buffer: string): {
    complete: any[];
    remainder: string;
  } {
    // Override in specific adapters
    // Default implementation for newline-delimited JSON
    const lines = buffer.split('\n');
    const complete: any[] = [];
    
    // Keep last line as remainder if not empty
    const remainder = lines[lines.length - 1];
    const completeLines = lines.slice(0, -1);
    
    for (const line of completeLines) {
      if (line.trim()) {
        try {
          const chunk = this.parseChunk(line);
          if (chunk) {
            complete.push(chunk);
          }
        } catch (e) {
          this.logger.debug('Failed to parse chunk', { line, error: e });
        }
      }
    }
    
    return { complete, remainder };
  }
  
  /**
   * Parse a single chunk
   */
  protected parseChunk(line: string): any {
    // Override in specific adapters
    return null;
  }
  
  /**
   * Send stream chunk via WebSocket
   */
  protected sendStreamChunk(streamId: string, chunk: any): void {
    if (!this.wsServer) return;
    
    const streamResponse: StreamResponse = {
      streamId,
      type: chunk.type || 'text',
      data: chunk.content || chunk,
      metadata: {
        model: chunk.model,
        tokens: chunk.tokens,
        finished: chunk.finished || false,
        isSimulated: chunk.isSimulated
      }
    };
    
    this.wsServer.writeToStream(streamId, streamResponse);
  }
  
  /**
   * Create result from streaming response
   */
  protected createStreamingResult(
    task: AITask,
    model: ModelProfile,
    content: string,
    tokenCount: number,
    startTime: number
  ): TaskResult {
    const endTime = Date.now();
    
    return {
      taskId: task.id,
      status: 'success',
      response: content,
      model: model.name,
      usage: {
        promptTokens: Math.ceil(task.prompt.length / 4),
        completionTokens: tokenCount,
        totalTokens: Math.ceil(task.prompt.length / 4) + tokenCount
      },
      cost: this.calculateCost({
        promptTokens: Math.ceil(task.prompt.length / 4),
        completionTokens: tokenCount
      }, model),
      latency: endTime - startTime,
      timestamp: endTime,
      metadata: {
        streaming: true,
        provider: model.provider
      }
    };
  }
  
  /**
   * Create stream ID
   */
  protected createStreamId(): string {
    return `stream_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}