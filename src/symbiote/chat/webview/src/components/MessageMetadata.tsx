import React from 'react';
import { MessageMetadata as IMessageMetadata } from '../types';
import './MessageMetadata.css';

interface MessageMetadataProps {
  metadata: IMessageMetadata;
}

export const MessageMetadata: React.FC<MessageMetadataProps> = ({ metadata }) => {
  return (
    <div className="message-metadata">
      {metadata.model && (
        <span className="metadata-item">
          <span className="codicon codicon-chip"></span>
          {metadata.model}
        </span>
      )}
      
      {metadata.tokensUsed && (
        <span className="metadata-item">
          <span className="codicon codicon-symbol-numeric"></span>
          {metadata.tokensUsed.total} tokens
        </span>
      )}
      
      {metadata.processingTime && (
        <span className="metadata-item">
          <span className="codicon codicon-clock"></span>
          {(metadata.processingTime / 1000).toFixed(1)}s
        </span>
      )}
      
      {metadata.confidence !== undefined && (
        <span className="metadata-item">
          <span className="codicon codicon-dashboard"></span>
          {Math.round(metadata.confidence * 100)}%
        </span>
      )}
    </div>
  );
};