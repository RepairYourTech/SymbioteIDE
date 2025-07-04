import React, { useState, useEffect, useRef } from 'react';
import { ChatHeader } from './components/ChatHeader';
import { MessageList } from './components/MessageList';
import { ChatInput } from './components/ChatInput';
import { CodeActionPanel } from './components/CodeActionPanel';
import { AttachmentBar } from './components/AttachmentBar';
import { ChatMessage, CodeAction, MessageAttachment } from './types';
import { useVSCode } from './hooks/useVSCode';
import './styles/app.css';

export const App: React.FC = () => {
  const vscode = useVSCode();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [attachments, setAttachments] = useState<MessageAttachment[]>([]);
  const [pendingCodeActions, setPendingCodeActions] = useState<CodeAction[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Scroll to bottom when new messages arrive
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // Handle messages from extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      
      switch (message.type) {
        case 'message-received':
          const chatMessage = message.payload.message;
          setMessages(prev => [...prev, chatMessage]);
          setIsLoading(false);
          
          // Extract code actions if present
          if (chatMessage.codeActions && chatMessage.codeActions.length > 0) {
            setPendingCodeActions(chatMessage.codeActions);
          }
          break;
          
        case 'session-loaded':
          const session = message.payload.session;
          setActiveSession(session.id);
          setMessages(session.messages);
          break;
          
        case 'error':
          console.error('Chat error:', message.payload.error);
          setIsLoading(false);
          // Show error message
          const errorMessage: ChatMessage = {
            id: Date.now().toString(),
            role: 'system',
            content: `Error: ${message.payload.error.message}`,
            timestamp: new Date()
          };
          setMessages(prev => [...prev, errorMessage]);
          break;
          
        case 'code-action-preview':
          // Handle code action preview
          // This could open a diff view or show inline preview
          break;
          
        case 'code-action-applied':
          // Remove from pending actions
          const appliedActionId = message.payload.actionId;
          setPendingCodeActions(prev => 
            prev.filter(action => action.id !== appliedActionId)
          );
          break;
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  // Send message
  const sendMessage = (content: string) => {
    if (!content.trim()) return;
    
    // Add user message to UI
    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content,
      timestamp: new Date(),
      attachments: attachments.length > 0 ? attachments : undefined
    };
    
    setMessages(prev => [...prev, userMessage]);
    setIsLoading(true);
    setAttachments([]);
    
    // Send to extension
    vscode.postMessage({
      type: 'send-message',
      payload: {
        content,
        attachments
      }
    });
  };

  // Handle file attachments
  const handleAttachment = (file: File) => {
    const reader = new FileReader();
    
    reader.onload = (e) => {
      const attachment: MessageAttachment = {
        type: file.type.startsWith('image/') ? 'image' : 'file',
        name: file.name,
        content: e.target?.result as string,
        mimeType: file.type
      };
      
      setAttachments(prev => [...prev, attachment]);
    };
    
    if (file.type.startsWith('image/')) {
      reader.readAsDataURL(file);
    } else {
      reader.readAsText(file);
    }
  };

  // Remove attachment
  const removeAttachment = (index: number) => {
    setAttachments(prev => prev.filter((_, i) => i !== index));
  };

  // Handle code action
  const handleCodeAction = (action: CodeAction, command: 'preview' | 'apply' | 'reject') => {
    vscode.postMessage({
      type: `code-action-${command}`,
      payload: { action }
    });
    
    if (command === 'reject') {
      setPendingCodeActions(prev => prev.filter(a => a.id !== action.id));
    }
  };

  // Create new session
  const createNewSession = () => {
    vscode.postMessage({ type: 'new-session' });
    setMessages([]);
    setActiveSession(null);
    setPendingCodeActions([]);
  };

  return (
    <div className="chat-app">
      <ChatHeader 
        sessionId={activeSession}
        onNewSession={createNewSession}
        onSettings={() => vscode.postMessage({ type: 'open-settings' })}
      />
      
      <div className="chat-content">
        <MessageList 
          messages={messages}
          isLoading={isLoading}
        />
        <div ref={messagesEndRef} />
      </div>
      
      {pendingCodeActions.length > 0 && (
        <CodeActionPanel
          actions={pendingCodeActions}
          onAction={handleCodeAction}
        />
      )}
      
      {attachments.length > 0 && (
        <AttachmentBar
          attachments={attachments}
          onRemove={removeAttachment}
        />
      )}
      
      <ChatInput
        onSendMessage={sendMessage}
        onAttachment={handleAttachment}
        disabled={isLoading}
      />
    </div>
  );
};