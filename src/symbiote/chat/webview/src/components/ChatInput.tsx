import React, { useState, useRef, KeyboardEvent, ChangeEvent } from 'react';
import { VSCodeButton } from '@vscode/webview-ui-toolkit/react';
import { VoiceInput } from './VoiceInput';
import './ChatInput.css';

interface ChatInputProps {
  onSendMessage: (message: string) => void;
  onAttachment: (file: File) => void;
  disabled?: boolean;
}

export const ChatInput: React.FC<ChatInputProps> = ({ 
  onSendMessage, 
  onAttachment,
  disabled = false 
}) => {
  const [message, setMessage] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleSend = () => {
    if (message.trim() && !disabled) {
      onSendMessage(message);
      setMessage('');
      
      // Reset textarea height
      if (textareaRef.current) {
        textareaRef.current.style.height = 'auto';
      }
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleInput = (e: ChangeEvent<HTMLTextAreaElement>) => {
    setMessage(e.target.value);
    
    // Auto-resize textarea
    const textarea = e.target;
    textarea.style.height = 'auto';
    textarea.style.height = `${textarea.scrollHeight}px`;
  };

  const handleFileSelect = (e: ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      Array.from(files).forEach(file => onAttachment(file));
    }
    // Reset input
    e.target.value = '';
  };

  const handleVoiceInput = (transcript: string, audioData: string) => {
    // Send voice message with transcript
    onSendMessage(transcript);
    // Note: In a real implementation, we'd also attach the audio data
  };

  // Handle paste for images
  const handlePaste = (e: React.ClipboardEvent) => {
    const items = e.clipboardData.items;
    
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      
      if (item.type.indexOf('image') !== -1) {
        const blob = item.getAsFile();
        if (blob) {
          onAttachment(blob);
          e.preventDefault();
        }
      }
    }
  };

  return (
    <div className="chat-input-container">
      <div className="chat-input-wrapper">
        <textarea
          ref={textareaRef}
          className="chat-input"
          value={message}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder="Type a message..."
          disabled={disabled}
          rows={1}
        />
        
        <div className="chat-input-actions">
          <input
            ref={fileInputRef}
            type="file"
            multiple
            style={{ display: 'none' }}
            onChange={handleFileSelect}
            accept="image/*,.txt,.js,.ts,.jsx,.tsx,.json,.md,.py,.java,.cpp,.c,.h,.cs,.php,.rb,.go,.rs,.swift"
          />
          
          <VSCodeButton
            appearance="icon"
            aria-label="Attach file"
            onClick={() => fileInputRef.current?.click()}
            disabled={disabled}
          >
            <span className="codicon codicon-attach"></span>
          </VSCodeButton>
          
          <VoiceInput 
            onVoiceInput={handleVoiceInput}
            disabled={disabled}
          />
          
          <VSCodeButton
            appearance="icon"
            aria-label="Send message"
            onClick={handleSend}
            disabled={disabled || !message.trim()}
          >
            <span className="codicon codicon-send"></span>
          </VSCodeButton>
        </div>
      </div>
      
      <div className="chat-input-hints">
        <span className="hint">
          <kbd>Enter</kbd> to send • <kbd>Shift + Enter</kbd> for new line
        </span>
      </div>
    </div>
  );
};