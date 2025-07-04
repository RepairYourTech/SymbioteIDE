/**
 * Chat Module
 * 
 * Integrated AI chat interface for SymbioteIDE
 */

export * from './chat-provider';
export * from './conversation-manager';
export * from './context/context-enricher';
export * from './actions/code-action-executor';

import * as vscode from 'vscode';
import { ChatProvider } from './chat-provider';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { KnowledgeGraphSystem } from '../knowledge';

/**
 * Initialize and register the chat interface
 */
export function registerChatInterface(
  context: vscode.ExtensionContext,
  orchestrationEngine: OrchestrationEngine,
  knowledgeGraph: KnowledgeGraphSystem
): ChatProvider {
  // Create chat provider
  const chatProvider = new ChatProvider(
    context.extensionUri,
    orchestrationEngine,
    knowledgeGraph
  );
  
  // Register webview view provider
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      ChatProvider.viewType,
      chatProvider,
      {
        webviewOptions: {
          retainContextWhenHidden: true
        }
      }
    )
  );
  
  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.chat.newSession', () => {
      chatProvider.createSession();
    })
  );
  
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.chat.clearHistory', async () => {
      const sessions = await chatProvider.listSessions();
      for (const session of sessions) {
        await chatProvider.deleteSession(session.id);
      }
      vscode.window.showInformationMessage('Chat history cleared');
    })
  );
  
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.chat.exportSession', async () => {
      const sessions = await chatProvider.listSessions();
      if (sessions.length === 0) {
        vscode.window.showWarningMessage('No chat sessions to export');
        return;
      }
      
      const selected = await vscode.window.showQuickPick(
        sessions.map(s => ({
          label: s.title,
          description: `${s.messages.length} messages`,
          session: s
        })),
        { placeHolder: 'Select a session to export' }
      );
      
      if (selected) {
        const data = await chatProvider.exportSession(selected.session.id);
        const uri = await vscode.window.showSaveDialog({
          defaultUri: vscode.Uri.file(`chat-${selected.session.id}.json`),
          filters: { 'JSON': ['json'] }
        });
        
        if (uri) {
          await vscode.workspace.fs.writeFile(uri, Buffer.from(data));
          vscode.window.showInformationMessage('Session exported successfully');
        }
      }
    })
  );
  
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.chat.importSession', async () => {
      const uri = await vscode.window.showOpenDialog({
        canSelectFiles: true,
        canSelectFolders: false,
        canSelectMany: false,
        filters: { 'JSON': ['json'] }
      });
      
      if (uri && uri[0]) {
        const content = await vscode.workspace.fs.readFile(uri[0]);
        const data = content.toString();
        await chatProvider.importSession(data);
        vscode.window.showInformationMessage('Session imported successfully');
      }
    })
  );
  
  // Register status bar item
  const statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.text = '$(comment-discussion) AI Chat';
  statusBarItem.tooltip = 'Open SymbioteIDE Chat';
  statusBarItem.command = 'workbench.view.extension.symbiote-chat';
  statusBarItem.show();
  
  context.subscriptions.push(statusBarItem);
  
  return chatProvider;
}

/**
 * Create contribution points for package.json
 */
export function getChatContributions() {
  return {
    views: {
      'symbiote-chat': [
        {
          type: 'webview',
          id: 'symbiote.chatView',
          name: 'Chat',
          icon: '$(comment-discussion)',
          contextualTitle: 'SymbioteIDE Chat'
        }
      ]
    },
    viewsContainers: {
      activitybar: [
        {
          id: 'symbiote-chat',
          title: 'SymbioteIDE Chat',
          icon: 'media/chat-icon.svg'
        }
      ]
    },
    commands: [
      {
        command: 'symbiote.chat.newSession',
        title: 'New Chat Session',
        category: 'SymbioteIDE'
      },
      {
        command: 'symbiote.chat.clearHistory',
        title: 'Clear Chat History',
        category: 'SymbioteIDE'
      },
      {
        command: 'symbiote.chat.exportSession',
        title: 'Export Chat Session',
        category: 'SymbioteIDE'
      },
      {
        command: 'symbiote.chat.importSession',
        title: 'Import Chat Session',
        category: 'SymbioteIDE'
      }
    ],
    configuration: {
      title: 'SymbioteIDE Chat',
      properties: {
        'symbiote.chat.ui.fontSize': {
          type: 'number',
          default: 14,
          description: 'Font size for chat messages'
        },
        'symbiote.chat.ui.theme': {
          type: 'string',
          enum: ['auto', 'light', 'dark'],
          default: 'auto',
          description: 'Chat interface theme'
        },
        'symbiote.chat.ui.showTokenUsage': {
          type: 'boolean',
          default: true,
          description: 'Show token usage for each message'
        },
        'symbiote.chat.ai.defaultModel': {
          type: 'string',
          default: 'gpt-4',
          description: 'Default AI model for chat'
        },
        'symbiote.chat.ai.temperature': {
          type: 'number',
          default: 0.7,
          minimum: 0,
          maximum: 2,
          description: 'Temperature for AI responses'
        },
        'symbiote.chat.ai.maxTokens': {
          type: 'number',
          default: 2048,
          description: 'Maximum tokens per response'
        },
        'symbiote.chat.ai.includeWorkspaceContext': {
          type: 'boolean',
          default: true,
          description: 'Include workspace context in AI prompts'
        },
        'symbiote.chat.persistence.enabled': {
          type: 'boolean',
          default: true,
          description: 'Enable chat history persistence'
        },
        'symbiote.chat.persistence.location': {
          type: 'string',
          enum: ['workspace', 'global'],
          default: 'workspace',
          description: 'Where to store chat history'
        },
        'symbiote.chat.features.voiceInput': {
          type: 'boolean',
          default: true,
          description: 'Enable voice input'
        },
        'symbiote.chat.features.codeActions': {
          type: 'boolean',
          default: true,
          description: 'Enable code action suggestions'
        }
      }
    }
  };
}