import React from 'react';
import { MessageAttachment } from '../types';
import './AttachmentBar.css';

interface AttachmentBarProps {
  attachments: MessageAttachment[];
  onRemove: (index: number) => void;
}

export const AttachmentBar: React.FC<AttachmentBarProps> = ({ attachments, onRemove }) => {
  const getAttachmentIcon = (type: MessageAttachment['type']) => {
    switch (type) {
      case 'image': return 'file-media';
      case 'code': return 'file-code';
      default: return 'file';
    }
  };

  return (
    <div className="attachment-bar">
      <div className="attachment-list">
        {attachments.map((attachment, index) => (
          <div key={index} className="attachment-item">
            <span className={`codicon codicon-${getAttachmentIcon(attachment.type)}`}></span>
            <span className="attachment-name">{attachment.name}</span>
            <button
              className="attachment-remove"
              onClick={() => onRemove(index)}
              aria-label="Remove attachment"
            >
              <span className="codicon codicon-close"></span>
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};