import React from 'react';
import { ChatMessage } from '../types';
import { Message } from './Message';
import { LoadingIndicator } from './LoadingIndicator';
import './MessageList.css';

interface MessageListProps {
  messages: ChatMessage[];
  isLoading: boolean;
}

export const MessageList: React.FC<MessageListProps> = ({ messages, isLoading }) => {
  if (messages.length === 0 && !isLoading) {
    return (
      <div className="message-list-empty">
        <div className="empty-state">
          <span className="empty-state-icon">💭</span>
          <h3>Start a conversation</h3>
          <p>Ask me anything about your code, or let me help you with development tasks.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="message-list">
      {messages.map((message) => (
        <Message key={message.id} message={message} />
      ))}
      
      {isLoading && <LoadingIndicator />}
    </div>
  );
};