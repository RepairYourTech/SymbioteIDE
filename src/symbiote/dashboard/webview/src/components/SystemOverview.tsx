import React from 'react';
import { SystemStatus, MemoryMetrics } from '../../../types/dashboard-types';
import { formatBytes, formatUptime } from '../utils/formatters';

interface SystemOverviewProps {
  systemStatus: SystemStatus;
  memoryUsage: MemoryMetrics;
}

export const SystemOverview: React.FC<SystemOverviewProps> = ({ systemStatus, memoryUsage }) => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'healthy': return 'var(--vscode-testing-iconPassed)';
      case 'degraded': return 'var(--vscode-testing-iconQueued)';
      case 'error': return 'var(--vscode-testing-iconFailed)';
      default: return 'var(--vscode-foreground)';
    }
  };
  
  return (
    <div className="panel system-overview">
      <div className="panel-header">
        <h2>System Overview</h2>
        <span 
          className="status-indicator"
          style={{ color: getStatusColor(systemStatus.status) }}
        >
          ● {systemStatus.status}
        </span>
      </div>
      
      <div className="panel-content">
        <div className="stats-grid">
          <div className="stat-card">
            <div className="stat-label">Version</div>
            <div className="stat-value">{systemStatus.version}</div>
          </div>
          
          <div className="stat-card">
            <div className="stat-label">Uptime</div>
            <div className="stat-value">{formatUptime(systemStatus.uptime)}</div>
          </div>
          
          <div className="stat-card">
            <div className="stat-label">CPU Usage</div>
            <div className="stat-value">{systemStatus.resources.cpu}%</div>
            <div className="progress-bar">
              <div 
                className="progress-fill"
                style={{ 
                  width: `${systemStatus.resources.cpu}%`,
                  backgroundColor: systemStatus.resources.cpu > 80 ? 'var(--vscode-editorError-foreground)' : 'var(--vscode-progressBar-background)'
                }}
              />
            </div>
          </div>
          
          <div className="stat-card">
            <div className="stat-label">Memory Usage</div>
            <div className="stat-value">
              {formatBytes(systemStatus.resources.memory.used)} / {formatBytes(systemStatus.resources.memory.total)}
            </div>
            <div className="progress-bar">
              <div 
                className="progress-fill"
                style={{ 
                  width: `${systemStatus.resources.memory.percentage}%`,
                  backgroundColor: systemStatus.resources.memory.percentage > 80 ? 'var(--vscode-editorError-foreground)' : 'var(--vscode-progressBar-background)'
                }}
              />
            </div>
          </div>
        </div>
        
        <div className="memory-stats">
          <h3>Memory Breakdown</h3>
          <div className="memory-grid">
            <div className="memory-item">
              <span className="memory-label">Short-term Memory</span>
              <span className="memory-value">{memoryUsage.shortTerm.entries} entries</span>
              <span className="memory-size">{formatBytes(memoryUsage.shortTerm.sizeBytes)}</span>
            </div>
            
            <div className="memory-item">
              <span className="memory-label">Long-term Memory</span>
              <span className="memory-value">{memoryUsage.longTerm.entries} entries</span>
              <span className="memory-size">{formatBytes(memoryUsage.longTerm.sizeBytes)}</span>
            </div>
            
            <div className="memory-item">
              <span className="memory-label">Vector Store</span>
              <span className="memory-value">{memoryUsage.vectorStore.documents} docs / {memoryUsage.vectorStore.vectors} vectors</span>
              <span className="memory-size">{memoryUsage.vectorStore.dimensions}D</span>
            </div>
            
            <div className="memory-item">
              <span className="memory-label">Cache Hit Rate</span>
              <span className="memory-value">{(memoryUsage.cacheHitRate * 100).toFixed(1)}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};