# Integrated AI Chat Interface

A native AI chat interface for SymbioteIDE that provides seamless code manipulation, multi-modal input support, and deep integration with the IDE's features.

## Architecture

The chat interface is built as a VS Code webview panel that can be docked in the sidebar, similar to extensions like Cline. It leverages VS Code's extension API for deep IDE integration.

### Core Components

#### 1. Chat Provider (`chat-provider.ts`)
The main provider class that:
- Manages the webview panel lifecycle
- Handles message passing between webview and extension
- Integrates with the orchestration engine for AI routing
- Manages chat sessions and history
- Implements the IChatAPI interface

#### 2. Context Enricher (`context/context-enricher.ts`)
Automatically gathers relevant context:
- Current file and cursor position
- Selected code snippets
- Open files and recent changes
- Terminal output and debug state
- Knowledge graph queries
- Builds enriched prompts for AI

#### 3. Code Action Executor (`actions/code-action-executor.ts`)
Handles code manipulations:
- Parses AI responses for code changes
- Generates diff previews
- Applies changes with undo support
- Supports multi-file operations
- Validates changes before applying

#### 4. Conversation Manager (`conversation-manager.ts`)
Manages chat persistence:
- Stores conversations in workspace or globally
- Provides search through chat history
- Supports export/import of sessions
- Handles cleanup based on retention policies

## Features

### Natural Language Code Manipulation

```typescript
// User: "Create a user service with authentication"
// AI generates code and shows diff preview
// User can review and apply changes with one click
```

### Context-Aware Intelligence

The chat automatically includes:
- Current file context
- Selected code
- Cursor position
- Open files
- Terminal output (when relevant)
- Debug context (when active)
- Knowledge graph information

### Multi-Modal Input Support

- **Text**: Standard chat messages
- **Images**: Paste screenshots or mockups
- **Voice**: Speech-to-text input (future)
- **Files**: Attach files for context
- **Diagrams**: Architecture diagrams (future)

### Smart Code Actions

AI responses can include:
- File creation
- Code modifications
- Refactoring suggestions
- Multi-file changes
- All with diff preview before applying

### Conversation Management

- Persistent chat history
- Search through past conversations
- Export/import sessions
- Team sharing capabilities (future)

## Usage

### Opening the Chat

1. Click the chat icon in the activity bar
2. Use the command palette: `SymbioteIDE: Open Chat`
3. Click the status bar item

### Basic Commands

```typescript
// Create new session
vscode.commands.executeCommand('symbiote.chat.newSession');

// Clear history
vscode.commands.executeCommand('symbiote.chat.clearHistory');

// Export session
vscode.commands.executeCommand('symbiote.chat.exportSession');

// Import session
vscode.commands.executeCommand('symbiote.chat.importSession');
```

### Configuration

```json
{
  // UI Settings
  "symbiote.chat.ui.fontSize": 14,
  "symbiote.chat.ui.theme": "auto",
  "symbiote.chat.ui.showTokenUsage": true,
  
  // AI Settings
  "symbiote.chat.ai.defaultModel": "gpt-4",
  "symbiote.chat.ai.temperature": 0.7,
  "symbiote.chat.ai.maxTokens": 2048,
  "symbiote.chat.ai.includeWorkspaceContext": true,
  
  // Persistence Settings
  "symbiote.chat.persistence.enabled": true,
  "symbiote.chat.persistence.location": "workspace",
  
  // Feature Flags
  "symbiote.chat.features.voiceInput": true,
  "symbiote.chat.features.codeActions": true
}
```

## API Integration

The chat interface exposes the IChatAPI for programmatic access:

```typescript
interface IChatAPI {
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
  
  // Code actions
  previewCodeAction(action: CodeAction): Promise<DiffPreview>;
  applyCodeAction(action: CodeAction): Promise<void>;
  
  // Events
  onMessage: Event<ChatMessage>;
  onCodeActionPreview: Event<CodeAction>;
  onCodeActionApplied: Event<CodeAction>;
}
```

## Webview Communication

The webview communicates with the extension using VS Code's message passing:

```typescript
// From webview to extension
vscode.postMessage({
  type: 'send-message',
  payload: {
    content: 'Create a user service',
    attachments: []
  }
});

// From extension to webview
webview.postMessage({
  type: 'message-received',
  payload: {
    message: assistantMessage
  }
});
```

## Security

- All webview content is sanitized
- CSP headers prevent script injection
- Code actions are validated before execution
- Conversation history can be encrypted
- Rate limiting prevents abuse

## Future Enhancements

- [ ] Voice input with speech-to-text
- [ ] Real-time collaboration
- [ ] Custom AI agents
- [ ] Plugin system for chat extensions
- [ ] Advanced diff visualization
- [ ] Integrated debugging support
- [ ] Multi-language support