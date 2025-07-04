import React from 'react';
import { AgentStatus } from '../../../types/dashboard-types';
import { formatDate, formatDuration, formatPercentage } from '../utils/formatters';
import classNames from 'classnames';

interface AgentsPanelProps {
  agents: AgentStatus[];
  detailed?: boolean;
  onToggleAgent: (agentId: string, enabled: boolean) => void;
}

export const AgentsPanel: React.FC<AgentsPanelProps> = ({ agents, detailed = false, onToggleAgent }) => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'var(--vscode-testing-iconPassed)';
      case 'idle': return 'var(--vscode-testing-iconQueued)';
      case 'error': return 'var(--vscode-testing-iconFailed)';
      case 'stopped': return 'var(--vscode-testing-iconSkipped)';
      default: return 'var(--vscode-foreground)';
    }
  };
  
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'active': return '🟢';
      case 'idle': return '🟡';
      case 'error': return '🔴';
      case 'stopped': return '⚫';
      default: return '⚪';
    }
  };
  
  return (
    <div className={classNames('panel agents-panel', { detailed })}>
      <div className="panel-header">
        <h2>Agents ({agents.length})</h2>
        <div className="agent-summary">
          <span className="active-count">
            {agents.filter(a => a.status === 'active').length} active
          </span>
          <span className="idle-count">
            {agents.filter(a => a.status === 'idle').length} idle
          </span>
        </div>
      </div>
      
      <div className="panel-content">
        <div className="agents-list">
          {agents.map(agent => (
            <div key={agent.id} className={classNames('agent-card', agent.status)}>
              <div className="agent-header">
                <div className="agent-info">
                  <span className="agent-status" title={agent.status}>
                    {getStatusIcon(agent.status)}
                  </span>
                  <div className="agent-name-container">
                    <h3 className="agent-name">{agent.name}</h3>
                    <span className="agent-type">{agent.type}</span>
                  </div>
                </div>
                
                <button
                  className={classNames('toggle-button', {
                    active: agent.status !== 'stopped'
                  })}
                  onClick={() => onToggleAgent(agent.id, agent.status === 'stopped')}
                >
                  {agent.status === 'stopped' ? 'Start' : 'Stop'}
                </button>
              </div>
              
              <div className="agent-details">
                <div className="agent-meta">
                  <span className="meta-item">
                    <span className="meta-label">Provider:</span>
                    <span className="meta-value">{agent.provider}</span>
                  </span>
                  {agent.model && (
                    <span className="meta-item">
                      <span className="meta-label">Model:</span>
                      <span className="meta-value">{agent.model}</span>
                    </span>
                  )}
                  {agent.lastActivity && (
                    <span className="meta-item">
                      <span className="meta-label">Last Active:</span>
                      <span className="meta-value">{formatDate(agent.lastActivity)}</span>
                    </span>
                  )}
                </div>
                
                <div className="agent-metrics">
                  <div className="metric">
                    <span className="metric-label">Executions</span>
                    <span className="metric-value">{agent.metrics.totalExecutions}</span>
                  </div>
                  <div className="metric">
                    <span className="metric-label">Success Rate</span>
                    <span className="metric-value">{formatPercentage(agent.metrics.successRate)}</span>
                  </div>
                  <div className="metric">
                    <span className="metric-label">Avg Response</span>
                    <span className="metric-value">{formatDuration(agent.metrics.averageResponseTime)}</span>
                  </div>
                </div>
                
                {agent.capabilities && agent.capabilities.length > 0 && (
                  <div className="agent-capabilities">
                    <span className="cap-label">Capabilities:</span>
                    <div className="cap-list">
                      {agent.capabilities.map(cap => (
                        <span key={cap} className="capability-tag">{cap}</span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
              
              {detailed && (
                <div className="agent-actions">
                  <button className="action-button">Configure</button>
                  <button className="action-button">View Logs</button>
                  <button className="action-button">Metrics</button>
                </div>
              )}
            </div>
          ))}
          
          {agents.length === 0 && (
            <div className="empty-state">
              <p>No agents configured</p>
              <button className="create-agent-button">Create Agent</button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};