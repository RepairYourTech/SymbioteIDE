/**
 * Chat API Types for Integrated AI Chat Interface
 * 
 * Provides native chat capabilities with deep IDE integration
 */

import { Event } from 'vscode';

// Message types
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
  tokenUsage?: TokenUsage;
  executionTime?: number;
  confidence?: number;
  context?: MessageContext;
}

export interface TokenUsage {
  prompt: number;
  completion: number;
  total: number;
  cost?: number;
}

export interface MessageContext {
  currentFile?: string;
  selectedCode?: string;
  cursorPosition?: Position;
  visibleRange?: Range;
  openFiles?: string[];
  terminalOutput?: string;
  debugContext?: any;
}

export interface MessageAttachment {
  type: 'image' | 'audio' | 'file' | 'diagram';
  data: string | ArrayBuffer;
  mimeType: string;
  name?: string;
  metadata?: Record<string, any>;
}

// Code actions
export interface CodeAction {
  id: string;
  type: 'create' | 'modify' | 'delete' | 'refactor' | 'move' | 'explain';
  files: FileChange[];
  description: string;
  preview?: DiffPreview;
  confidence: number;
  alternatives?: CodeAction[];
}

export interface FileChange {
  path: string;
  type: 'create' | 'modify' | 'delete' | 'rename';
  oldPath?: string; // For renames
  content?: string;
  diff?: TextDiff;
}

export interface TextDiff {
  original: string;
  modified: string;
  hunks: DiffHunk[];
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

export interface DiffPreview {
  summary: string;
  filesChanged: number;
  insertions: number;
  deletions: number;
  changes: FileChange[];
}

// Chat sessions
export interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: Date;
  updatedAt: Date;
  metadata?: SessionMetadata;
  context?: SessionContext;
}

export interface SessionMetadata {
  projectId?: string;
  workspaceFolder?: string;
  tags?: string[];
  shared?: boolean;
  teamId?: string;
}

export interface SessionContext {
  activeModel?: string;
  temperature?: number;
  maxTokens?: number;
  systemPrompt?: string;
  customInstructions?: string;
}

// Multi-modal inputs
export interface VoiceInput {
  audio: ArrayBuffer;
  format: 'webm' | 'mp3' | 'wav';
  duration: number;
  language?: string;
}

export interface ImageInput {
  data: string | ArrayBuffer;
  format: 'png' | 'jpeg' | 'webp' | 'svg';
  width: number;
  height: number;
  description?: string;
}

export interface DiagramInput {
  type: 'architecture' | 'flowchart' | 'sequence' | 'class' | 'er';
  data: string; // Could be SVG, Mermaid, PlantUML, etc.
  format: string;
}

// Chat configuration
export interface ChatConfiguration {
  ui: ChatUIConfig;
  ai: ChatAIConfig;
  persistence: ChatPersistenceConfig;
  features: ChatFeatureConfig;
}

export interface ChatUIConfig {
  fontSize: number;
  theme: 'auto' | 'light' | 'dark';
  showTokenUsage: boolean;
  showModelIndicator: boolean;
  enableSyntaxHighlighting: boolean;
  autoScroll: boolean;
  compactMode: boolean;
}

export interface ChatAIConfig {
  defaultModel: string;
  temperature: number;
  maxTokens: number;
  contextWindow: number;
  includeWorkspaceContext: boolean;
  includeTerminalOutput: boolean;
  includeDebugContext: boolean;
  autoSelectModel: boolean;
}

export interface ChatPersistenceConfig {
  enabled: boolean;
  location: 'workspace' | 'global';
  maxHistorySize: number;
  autoSave: boolean;
  encryptHistory: boolean;
}

export interface ChatFeatureConfig {
  voiceInput: boolean;
  imageInput: boolean;
  diagramInput: boolean;
  codeActions: boolean;
  diffPreview: boolean;
  multiFileOperations: boolean;
  teamSharing: boolean;
}

// Chat events
export interface ChatEventMap {
  'message': (message: ChatMessage) => void;
  'session-created': (session: ChatSession) => void;
  'session-updated': (session: ChatSession) => void;
  'session-deleted': (sessionId: string) => void;
  'code-action-preview': (action: CodeAction) => void;
  'code-action-applied': (action: CodeAction) => void;
  'error': (error: ChatError) => void;
}

export interface ChatError {
  code: ChatErrorCode;
  message: string;
  details?: any;
}

export enum ChatErrorCode {
  MODEL_ERROR = 'MODEL_ERROR',
  CONTEXT_TOO_LARGE = 'CONTEXT_TOO_LARGE',
  RATE_LIMIT = 'RATE_LIMIT',
  INVALID_ACTION = 'INVALID_ACTION',
  PERMISSION_DENIED = 'PERMISSION_DENIED',
  NETWORK_ERROR = 'NETWORK_ERROR'
}

// Position and range types (matching VS Code)
export interface Position {
  line: number;
  character: number;
}

export interface Range {
  start: Position;
  end: Position;
}

// Main chat API interface
export interface IChatAPI {
  // Session management
  createSession(title?: string): Promise<ChatSession>;
  getSession(sessionId: string): Promise<ChatSession | null>;
  listSessions(): Promise<ChatSession[]>;
  deleteSession(sessionId: string): Promise<void>;
  
  // Messaging
  sendMessage(
    sessionId: string,
    content: string,
    attachments?: MessageAttachment[]
  ): Promise<ChatMessage>;
  
  // Voice input
  sendVoiceMessage(
    sessionId: string,
    voice: VoiceInput
  ): Promise<ChatMessage>;
  
  // Code actions
  previewCodeAction(action: CodeAction): Promise<DiffPreview>;
  applyCodeAction(action: CodeAction): Promise<void>;
  rejectCodeAction(actionId: string): Promise<void>;
  
  // Context management
  updateContext(
    sessionId: string,
    context: Partial<MessageContext>
  ): Promise<void>;
  
  // Search and history
  searchMessages(query: string): Promise<ChatMessage[]>;
  exportSession(sessionId: string): Promise<string>;
  importSession(data: string): Promise<ChatSession>;
  
  // Configuration
  getConfiguration(): ChatConfiguration;
  updateConfiguration(config: Partial<ChatConfiguration>): Promise<void>;
  
  // Events
  onMessage: Event<ChatMessage>;
  onSessionCreated: Event<ChatSession>;
  onSessionUpdated: Event<ChatSession>;
  onSessionDeleted: Event<string>;
  onCodeActionPreview: Event<CodeAction>;
  onCodeActionApplied: Event<CodeAction>;
  onError: Event<ChatError>;
}

// Webview message protocol
export interface WebviewMessage {
  type: string;
  payload: any;
}

export interface ChatWebviewMessages {
  // From webview to extension
  'send-message': {
    content: string;
    attachments?: MessageAttachment[];
  };
  'preview-action': {
    actionId: string;
  };
  'apply-action': {
    actionId: string;
  };
  'reject-action': {
    actionId: string;
  };
  'update-config': {
    config: Partial<ChatConfiguration>;
  };
  'load-session': {
    sessionId: string;
  };
  'create-session': {
    title?: string;
  };
  'delete-session': {
    sessionId: string;
  };
  'search-messages': {
    query: string;
  };
  
  // From extension to webview
  'message-received': {
    message: ChatMessage;
  };
  'session-loaded': {
    session: ChatSession;
  };
  'sessions-updated': {
    sessions: ChatSession[];
  };
  'config-updated': {
    config: ChatConfiguration;
  };
  'diff-preview': {
    preview: DiffPreview;
  };
  'error': {
    error: ChatError;
  };
}

// Smart action suggestions
export interface SmartSuggestion {
  id: string;
  type: 'command' | 'snippet' | 'refactor' | 'fix';
  label: string;
  description?: string;
  icon?: string;
  action: () => Promise<void>;
}

// Team collaboration
export interface TeamMessage extends ChatMessage {
  userId: string;
  userName: string;
  avatar?: string;
}

export interface SharedSession extends ChatSession {
  participants: SessionParticipant[];
  permissions: SessionPermissions;
}

export interface SessionParticipant {
  userId: string;
  userName: string;
  role: 'owner' | 'editor' | 'viewer';
  joinedAt: Date;
  isActive: boolean;
}

export interface SessionPermissions {
  canEdit: boolean;
  canDelete: boolean;
  canShare: boolean;
  canApplyActions: boolean;
}