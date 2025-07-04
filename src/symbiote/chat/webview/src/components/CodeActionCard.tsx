import React from 'react';
import { VSCodeButton, VSCodeBadge } from '@vscode/webview-ui-toolkit/react';
import { CodeAction } from '../types';
import './CodeActionCard.css';

interface CodeActionCardProps {
  action: CodeAction;
  onPreview: () => void;
  onApply: () => void;
  onReject: () => void;
}

export const CodeActionCard: React.FC<CodeActionCardProps> = ({ 
  action, 
  onPreview, 
  onApply, 
  onReject 
}) => {
  const getActionIcon = (type: CodeAction['type']) => {
    switch (type) {
      case 'create': return 'new-file';
      case 'modify': return 'edit';
      case 'delete': return 'trash';
      case 'refactor': return 'symbol-method';
      default: return 'lightbulb';
    }
  };

  const getImpactColor = (impact: CodeAction['impact']) => {
    switch (impact) {
      case 'low': return 'var(--vscode-charts-green)';
      case 'medium': return 'var(--vscode-charts-yellow)';
      case 'high': return 'var(--vscode-charts-red)';
    }
  };

  return (
    <div className="code-action-card">
      <div className="code-action-header">
        <span className={`codicon codicon-${getActionIcon(action.type)}`}></span>
        <span className="code-action-title">{action.title}</span>
        <VSCodeBadge style={{ backgroundColor: getImpactColor(action.impact) }}>
          {action.impact} impact
        </VSCodeBadge>
      </div>
      
      {action.description && (
        <div className="code-action-description">{action.description}</div>
      )}
      
      <div className="code-action-files">
        {action.files.map((file, index) => (
          <div key={index} className="code-action-file">
            <span className={`codicon codicon-${file.action === 'create' ? 'new-file' : 
                                            file.action === 'delete' ? 'trash' : 'edit'}`}></span>
            <span className="code-action-file-path">{file.path}</span>
          </div>
        ))}
      </div>
      
      <div className="code-action-confidence">
        Confidence: {Math.round(action.confidence * 100)}%
      </div>
      
      <div className="code-action-buttons">
        <VSCodeButton onClick={onPreview}>
          <span className="codicon codicon-eye"></span>
          Preview
        </VSCodeButton>
        <VSCodeButton appearance="primary" onClick={onApply}>
          <span className="codicon codicon-check"></span>
          Apply
        </VSCodeButton>
        <VSCodeButton appearance="secondary" onClick={onReject}>
          <span className="codicon codicon-close"></span>
          Reject
        </VSCodeButton>
      </div>
    </div>
  );
};