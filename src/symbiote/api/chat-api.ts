/**
 * Chat API - Chat functionality API interface
 */

export interface ChatAPI {
  createSession(options?: ChatSessionOptions): Promise<string>;
  getSession(sessionId: string): Promise<ChatSession | null>;
  listSessions(): Promise<ChatSession[]>;
  deleteSession(sessionId: string): Promise<void>;
  
  sendMessage(sessionId: string, message: ChatMessage): Promise<ChatResponse>;
  getHistory(sessionId: string): Promise<ChatMessage[]>;
  clearHistory(sessionId: string): Promise<void>;
  
  streamMessage(sessionId: string, message: ChatMessage): AsyncIterable<ChatStreamChunk>;
}

export interface ChatSessionOptions {
  model?: string;
  systemPrompt?: string;
  temperature?: number;
  maxTokens?: number;
  metadata?: Record<string, any>;
}

export interface ChatSession {
  id: string;
  createdAt: Date;
  updatedAt: Date;
  model: string;
  systemPrompt?: string;
  temperature: number;
  maxTokens: number;
  messageCount: number;
  metadata?: Record<string, any>;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp?: Date;
  metadata?: Record<string, any>;
}

export interface ChatResponse {
  content: string;
  role: 'assistant';
  timestamp: Date;
  usage?: {
    promptTokens: number;
    completionTokens: number;
    totalTokens: number;
  };
  metadata?: Record<string, any>;
}

export interface ChatStreamChunk {
  content: string;
  done: boolean;
  metadata?: Record<string, any>;
}

export class ChatAPIImpl implements ChatAPI {
  private sessions: Map<string, ChatSession> = new Map();
  private histories: Map<string, ChatMessage[]> = new Map();

  async createSession(options?: ChatSessionOptions): Promise<string> {
    const id = `chat_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const now = new Date();
    
    const session: ChatSession = {
      id,
      createdAt: now,
      updatedAt: now,
      model: options?.model || 'gpt-4',
      systemPrompt: options?.systemPrompt,
      temperature: options?.temperature || 0.7,
      maxTokens: options?.maxTokens || 2048,
      messageCount: 0,
      metadata: options?.metadata
    };

    this.sessions.set(id, session);
    this.histories.set(id, []);
    
    // Add system prompt to history if provided
    if (options?.systemPrompt) {
      this.histories.get(id)!.push({
        role: 'system',
        content: options.systemPrompt,
        timestamp: now
      });
    }

    return id;
  }

  async getSession(sessionId: string): Promise<ChatSession | null> {
    return this.sessions.get(sessionId) || null;
  }

  async listSessions(): Promise<ChatSession[]> {
    return Array.from(this.sessions.values());
  }

  async deleteSession(sessionId: string): Promise<void> {
    this.sessions.delete(sessionId);
    this.histories.delete(sessionId);
  }

  async sendMessage(sessionId: string, message: ChatMessage): Promise<ChatResponse> {
    const session = this.sessions.get(sessionId);
    if (!session) {
      throw new Error(`Session ${sessionId} not found`);
    }

    const history = this.histories.get(sessionId)!;
    const timestamp = new Date();
    
    // Add user message to history
    history.push({
      ...message,
      timestamp: message.timestamp || timestamp
    });

    // Generate response (simulation)
    const response: ChatResponse = {
      content: `Response to: ${message.content}`,
      role: 'assistant',
      timestamp: new Date(),
      usage: {
        promptTokens: 100,
        completionTokens: 50,
        totalTokens: 150
      }
    };

    // Add assistant response to history
    history.push({
      role: response.role,
      content: response.content,
      timestamp: response.timestamp
    });

    // Update session
    session.messageCount = history.length;
    session.updatedAt = new Date();

    return response;
  }

  async getHistory(sessionId: string): Promise<ChatMessage[]> {
    const history = this.histories.get(sessionId);
    if (!history) {
      throw new Error(`Session ${sessionId} not found`);
    }
    return [...history];
  }

  async clearHistory(sessionId: string): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (!session) {
      throw new Error(`Session ${sessionId} not found`);
    }

    const history = this.histories.get(sessionId)!;
    
    // Keep system prompt if it exists
    const systemPrompt = history.find(msg => msg.role === 'system');
    this.histories.set(sessionId, systemPrompt ? [systemPrompt] : []);
    
    session.messageCount = systemPrompt ? 1 : 0;
    session.updatedAt = new Date();
  }

  async *streamMessage(sessionId: string, message: ChatMessage): AsyncIterable<ChatStreamChunk> {
    const session = this.sessions.get(sessionId);
    if (!session) {
      throw new Error(`Session ${sessionId} not found`);
    }

    const history = this.histories.get(sessionId)!;
    const timestamp = new Date();
    
    // Add user message to history
    history.push({
      ...message,
      timestamp: message.timestamp || timestamp
    });

    // Simulate streaming response
    const words = `This is a streaming response to: ${message.content}`.split(' ');
    let fullResponse = '';
    
    for (let i = 0; i < words.length; i++) {
      const chunk = words[i] + (i < words.length - 1 ? ' ' : '');
      fullResponse += chunk;
      
      yield {
        content: chunk,
        done: i === words.length - 1
      };
      
      // Simulate delay
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    // Add complete response to history
    history.push({
      role: 'assistant',
      content: fullResponse,
      timestamp: new Date()
    });

    // Update session
    session.messageCount = history.length;
    session.updatedAt = new Date();
  }
}

export default ChatAPIImpl;