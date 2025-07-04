import React from 'react';
import { MessageAttachment } from '../types';
import './AttachmentPreview.css';

interface AttachmentPreviewProps {
  attachment: MessageAttachment;
}

export const AttachmentPreview: React.FC<AttachmentPreviewProps> = ({ attachment }) => {
  if (attachment.type === 'image') {
    return (
      <div className="attachment-preview attachment-preview-image">
        <img src={attachment.content} alt={attachment.name} />
        <span className="attachment-preview-name">{attachment.name}</span>
      </div>
    );
  }

  return (
    <div className="attachment-preview attachment-preview-file">
      <span className={`codicon codicon-${attachment.type === 'code' ? 'file-code' : 'file'}`}></span>
      <span className="attachment-preview-name">{attachment.name}</span>
    </div>
  );
};