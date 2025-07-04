/**
 * Stream Manager
 * 
 * Manages streaming responses for real-time AI output
 */

import { Server as SocketServer } from 'socket.io';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { IStreamManager, StreamResponse, SocketEvent } from './types';

interface StreamInfo {
  id: string;
  clientId: string;
  startTime: Date;
  lastActivity: Date;
  metadata?: any;
  chunks: StreamResponse[];
  bytesSent: number;
  status: 'active' | 'completed' | 'error';
}

export class StreamManager extends EventEmitter implements IStreamManager {
  private io: SocketServer;
  private streams = new Map<string, StreamInfo>();
  private logger = new Logger('StreamManager');
  private cleanupInterval: NodeJS.Timeout;
  
  constructor(io: SocketServer) {
    super();
    this.io = io;
    
    // Clean up stale streams every minute
    this.cleanupInterval = setInterval(() => {
      this.cleanupStaleStreams();
    }, 60000);
  }
  
  /**
   * Create a new stream
   */
  createStream(streamId: string, metadata?: any): void {
    if (this.streams.has(streamId)) {
      throw new Error(`Stream ${streamId} already exists`);
    }
    
    const stream: StreamInfo = {
      id: streamId,
      clientId: '', // Will be set when first data is sent
      startTime: new Date(),
      lastActivity: new Date(),
      metadata,
      chunks: [],
      bytesSent: 0,
      status: 'active'
    };
    
    this.streams.set(streamId, stream);
    this.logger.debug('Stream created', { streamId, metadata });
    this.emit('stream-created', { streamId, metadata });
  }
  
  /**
   * Write data to stream
   */
  writeToStream(streamId: string, data: StreamResponse): void {
    const stream = this.streams.get(streamId);
    if (!stream) {
      throw new Error(`Stream ${streamId} not found`);
    }
    
    if (stream.status !== 'active') {
      throw new Error(`Stream ${streamId} is not active`);
    }
    
    // Update stream info
    stream.lastActivity = new Date();
    stream.chunks.push(data);
    
    // Calculate bytes (rough estimate)
    const bytes = JSON.stringify(data).length;
    stream.bytesSent += bytes;
    
    // Emit to all connected clients
    this.io.emit(SocketEvent.StreamData, data);
    
    this.logger.debug('Stream data sent', { 
      streamId, 
      type: data.type,
      bytes 
    });
    
    this.emit('stream-data', { streamId, data, bytes });
  }
  
  /**
   * End a stream
   */
  endStream(streamId: string, summary?: any): void {
    const stream = this.streams.get(streamId);
    if (!stream) {
      throw new Error(`Stream ${streamId} not found`);
    }
    
    stream.status = 'completed';
    stream.lastActivity = new Date();
    
    // Send end event
    this.io.emit(SocketEvent.StreamEnd, {
      streamId,
      summary: summary || this.generateSummary(stream)
    });
    
    this.logger.info('Stream ended', {
      streamId,
      duration: Date.now() - stream.startTime.getTime(),
      chunks: stream.chunks.length,
      bytesSent: stream.bytesSent
    });
    
    this.emit('stream-ended', { streamId, stream });
    
    // Clean up after a delay
    setTimeout(() => {
      this.streams.delete(streamId);
    }, 5000);
  }
  
  /**
   * Error a stream
   */
  errorStream(streamId: string, error: Error): void {
    const stream = this.streams.get(streamId);
    if (!stream) {
      throw new Error(`Stream ${streamId} not found`);
    }
    
    stream.status = 'error';
    stream.lastActivity = new Date();
    
    // Send error event
    this.io.emit(SocketEvent.StreamError, {
      streamId,
      error: {
        message: error.message,
        stack: error.stack
      }
    });
    
    this.logger.error('Stream error', {
      streamId,
      error: error.message
    });
    
    this.emit('stream-error', { streamId, error });
    
    // Clean up immediately
    this.streams.delete(streamId);
  }
  
  /**
   * Get active streams
   */
  getActiveStreams(): string[] {
    return Array.from(this.streams.entries())
      .filter(([_, stream]) => stream.status === 'active')
      .map(([id, _]) => id);
  }
  
  /**
   * Get stream info
   */
  getStreamInfo(streamId: string): StreamInfo | undefined {
    return this.streams.get(streamId);
  }
  
  /**
   * Get stream statistics
   */
  getStreamStats(): {
    active: number;
    completed: number;
    error: number;
    totalBytes: number;
  } {
    let active = 0;
    let completed = 0;
    let error = 0;
    let totalBytes = 0;
    
    for (const stream of this.streams.values()) {
      totalBytes += stream.bytesSent;
      
      switch (stream.status) {
        case 'active':
          active++;
          break;
        case 'completed':
          completed++;
          break;
        case 'error':
          error++;
          break;
      }
    }
    
    return { active, completed, error, totalBytes };
  }
  
  /**
   * Clean up stale streams
   */
  private cleanupStaleStreams(): void {
    const now = Date.now();
    const timeout = 5 * 60 * 1000; // 5 minutes
    let cleaned = 0;
    
    for (const [streamId, stream] of this.streams) {
      const age = now - stream.lastActivity.getTime();
      
      if (age > timeout && stream.status === 'active') {
        this.logger.warn('Cleaning up stale stream', { streamId, age });
        this.errorStream(streamId, new Error('Stream timeout'));
        cleaned++;
      }
    }
    
    if (cleaned > 0) {
      this.logger.info(`Cleaned up ${cleaned} stale streams`);
    }
  }
  
  /**
   * Generate stream summary
   */
  private generateSummary(stream: StreamInfo): any {
    const duration = Date.now() - stream.startTime.getTime();
    const textChunks = stream.chunks.filter(c => c.type === 'text');
    const codeChunks = stream.chunks.filter(c => c.type === 'code');
    const dataChunks = stream.chunks.filter(c => c.type === 'data');
    
    return {
      duration,
      chunks: {
        total: stream.chunks.length,
        text: textChunks.length,
        code: codeChunks.length,
        data: dataChunks.length
      },
      bytesSent: stream.bytesSent,
      metadata: stream.metadata
    };
  }
  
  /**
   * Destroy the stream manager
   */
  destroy(): void {
    clearInterval(this.cleanupInterval);
    
    // End all active streams
    for (const [streamId, stream] of this.streams) {
      if (stream.status === 'active') {
        this.endStream(streamId);
      }
    }
    
    this.streams.clear();
    this.removeAllListeners();
  }
}