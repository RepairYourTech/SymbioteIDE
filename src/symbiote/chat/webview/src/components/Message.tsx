import React from 'react';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import remarkGfm from 'remark-gfm';
import clsx from 'clsx';
import { ChatMessage } from '../types';
import { MessageMetadata } from './MessageMetadata';
import { AttachmentPreview } from './AttachmentPreview';
import './Message.css';

interface MessageProps {
  message: ChatMessage;
}

export const Message: React.FC<MessageProps> = ({ message }) => {
  const formatTime = (date: Date): string => {
    return new Date(date).toLocaleTimeString([], { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
  };

  const messageClass = clsx('message', `message-${message.role}`);

  return (
    <div className={messageClass}>
      <div className="message-header">
        <span className="message-role">
          {message.role === 'user' ? 'You' : 
           message.role === 'assistant' ? 'Assistant' : 'System'}
        </span>
        <span className="message-time">{formatTime(message.timestamp)}</span>
        {message.metadata && <MessageMetadata metadata={message.metadata} />}
      </div>
      
      {message.attachments && message.attachments.length > 0 && (
        <div className="message-attachments">
          {message.attachments.map((attachment, index) => (
            <AttachmentPreview key={index} attachment={attachment} />
          ))}
        </div>
      )}
      
      <div className="message-content">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            code({ node, inline, className, children, ...props }) {
              const match = /language-(\w+)/.exec(className || '');
              const language = match ? match[1] : '';
              
              return !inline && language ? (
                <div className="code-block">
                  <div className="code-block-header">
                    <span className="code-language">{language}</span>
                    <button 
                      className="code-copy-button"
                      onClick={() => {
                        navigator.clipboard.writeText(String(children).replace(/\n$/, ''));
                      }}
                    >
                      <span className="codicon codicon-copy"></span>
                    </button>
                  </div>
                  <SyntaxHighlighter
                    style={vscDarkPlus}
                    language={language}
                    PreTag="div"
                    {...props}
                  >
                    {String(children).replace(/\n$/, '')}
                  </SyntaxHighlighter>
                </div>
              ) : (
                <code className={className} {...props}>
                  {children}
                </code>
              );
            }
          }}
        >
          {message.content}
        </ReactMarkdown>
      </div>
    </div>
  );
};