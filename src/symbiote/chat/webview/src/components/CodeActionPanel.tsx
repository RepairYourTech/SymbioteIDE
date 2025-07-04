import React from 'react';
import { VSCodeButton } from '@vscode/webview-ui-toolkit/react';
import { CodeAction } from '../types';
import { CodeActionCard } from './CodeActionCard';
import './CodeActionPanel.css';

interface CodeActionPanelProps {
  actions: CodeAction[];
  onAction: (action: CodeAction, command: 'preview' | 'apply' | 'reject') => void;
}

export const CodeActionPanel: React.FC<CodeActionPanelProps> = ({ actions, onAction }) => {
  if (actions.length === 0) return null;

  return (
    <div className="code-action-panel">
      <div className="code-action-panel-header">
        <span className="codicon codicon-lightbulb"></span>
        <span className="code-action-panel-title">
          Suggested Code Actions ({actions.length})
        </span>
      </div>
      
      <div className="code-action-list">
        {actions.map((action) => (
          <CodeActionCard
            key={action.id}
            action={action}
            onPreview={() => onAction(action, 'preview')}
            onApply={() => onAction(action, 'apply')}
            onReject={() => onAction(action, 'reject')}
          />
        ))}
      </div>
      
      <div className="code-action-panel-footer">
        <VSCodeButton 
          appearance="secondary"
          onClick={() => actions.forEach(a => onAction(a, 'reject'))}
        >
          Dismiss All
        </VSCodeButton>
      </div>
    </div>
  );
};