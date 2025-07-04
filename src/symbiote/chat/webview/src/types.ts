export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
  metadata?: MessageMetadata;
  codeActions?: CodeAction[];
  attachments?: MessageAttachment[];
}

export interface MessageMetadata {
  model?: string;
  confidence?: number;
  tokensUsed?: {
    prompt: number;
    completion: number;
    total: number;
  };
  processingTime?: number;
  context?: MessageContext;
}

export interface MessageContext {
  files?: FileContext[];
  symbols?: SymbolContext[];
  terminal?: TerminalContext;
  debug?: DebugContext;
  knowledgeGraph?: GraphContext;
}

export interface FileContext {
  path: string;
  language: string;
  range?: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  content?: string;
}

export interface SymbolContext {
  name: string;
  kind: string;
  filePath: string;
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
}

export interface TerminalContext {
  command: string;
  output: string;
  exitCode?: number;
}

export interface DebugContext {
  breakpoints: string[];
  callStack: string[];
  variables: Record<string, any>;
}

export interface GraphContext {
  nodes: string[];
  relationships: string[];
  query?: string;
}

export interface CodeAction {
  id: string;
  type: 'create' | 'modify' | 'delete' | 'refactor';
  title: string;
  description?: string;
  files: CodeActionFile[];
  confidence: number;
  impact: 'low' | 'medium' | 'high';
}

export interface CodeActionFile {
  path: string;
  action: 'create' | 'modify' | 'delete';
  content?: string;
  diff?: DiffHunk[];
}

export interface DiffHunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
}

export interface DiffLine {
  type: 'add' | 'delete' | 'context';
  content: string;
  lineNumber?: number;
}

export interface MessageAttachment {
  type: 'file' | 'image' | 'code';
  name: string;
  content: string;
  mimeType?: string;
  language?: string;
}

export interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: Date;
  updatedAt: Date;
  metadata?: {
    tags?: string[];
    model?: string;
    totalTokens?: number;
  };
}

export interface WebviewMessage {
  type: string;
  payload?: any;
}

export interface VSCodeAPI {
  postMessage(message: WebviewMessage): void;
  getState(): any;
  setState(state: any): void;
}