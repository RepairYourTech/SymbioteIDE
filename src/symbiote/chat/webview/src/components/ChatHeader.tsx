import React from 'react';
import { VSCodeButton } from '@vscode/webview-ui-toolkit/react';
import './ChatHeader.css';

interface ChatHeaderProps {
  sessionId: string | null;
  onNewSession: () => void;
  onSettings: () => void;
}

export const ChatHeader: React.FC<ChatHeaderProps> = ({ 
  sessionId, 
  onNewSession, 
  onSettings 
}) => {
  return (
    <div className="chat-header">
      <div className="chat-header-title">
        <span className="chat-header-icon">💬</span>
        <span className="chat-header-text">
          SymbioteIDE Chat
          {sessionId && <span className="chat-header-session"> • Session {sessionId.slice(0, 8)}</span>}
        </span>
      </div>
      
      <div className="chat-header-actions">
        <VSCodeButton 
          appearance="icon" 
          aria-label="New Session"
          onClick={onNewSession}
        >
          <span className="codicon codicon-add"></span>
        </VSCodeButton>
        
        <VSCodeButton 
          appearance="icon" 
          aria-label="Settings"
          onClick={onSettings}
        >
          <span className="codicon codicon-settings-gear"></span>
        </VSCodeButton>
      </div>
    </div>
  );
};