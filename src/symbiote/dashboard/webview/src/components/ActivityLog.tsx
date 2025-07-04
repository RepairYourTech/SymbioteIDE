import React, { useState, useEffect, useRef } from 'react';
import { DashboardLogEntry, LogLevel } from '../../../types/dashboard-types';
import { formatDate } from '../utils/formatters';

interface ActivityLogProps {
  logs: DashboardLogEntry[];
  maxEntries?: number;
  autoScroll?: boolean;
}

const LOG_LEVEL_COLORS: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: 'var(--vscode-editorInfo-foreground)',
  [LogLevel.INFO]: 'var(--vscode-foreground)',
  [LogLevel.WARN]: 'var(--vscode-editorWarning-foreground)',
  [LogLevel.ERROR]: 'var(--vscode-editorError-foreground)'
};

const LOG_LEVEL_ICONS: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: '🐛',
  [LogLevel.INFO]: 'ℹ️',
  [LogLevel.WARN]: '⚠️',
  [LogLevel.ERROR]: '❌'
};

export const ActivityLog: React.FC<ActivityLogProps> = ({ 
  logs, 
  maxEntries = 100, 
  autoScroll = true 
}) => {
  const [filter, setFilter] = useState<string>('');
  const [levelFilter, setLevelFilter] = useState<LogLevel | 'all'>('all');
  const [sourceFilter, setSourceFilter] = useState<string>('all');
  const logEndRef = useRef<HTMLDivElement>(null);
  
  const sources = React.useMemo(() => {
    const uniqueSources = new Set(logs.map(log => log.source));
    return Array.from(uniqueSources).sort();
  }, [logs]);
  
  const filteredLogs = React.useMemo(() => {
    let filtered = logs;
    
    if (levelFilter !== 'all') {
      filtered = filtered.filter(log => log.level === levelFilter);
    }
    
    if (sourceFilter !== 'all') {
      filtered = filtered.filter(log => log.source === sourceFilter);
    }
    
    if (filter) {
      const searchTerm = filter.toLowerCase();
      filtered = filtered.filter(log => 
        log.message.toLowerCase().includes(searchTerm) ||
        log.details?.toLowerCase().includes(searchTerm) ||
        log.source.toLowerCase().includes(searchTerm)
      );
    }
    
    return filtered.slice(-maxEntries);
  }, [logs, filter, levelFilter, sourceFilter, maxEntries]);
  
  useEffect(() => {
    if (autoScroll && logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [filteredLogs, autoScroll]);
  
  const clearLogs = () => {
    // Send message to clear logs
    window.vscode.postMessage({
      type: 'clearLogs'
    });
  };
  
  const exportLogs = () => {
    const logText = filteredLogs.map(log => 
      `[${formatDate(log.timestamp)}] [${log.level}] [${log.source}] ${log.message}${log.details ? '\n' + log.details : ''}`
    ).join('\n\n');
    
    window.vscode.postMessage({
      type: 'exportLogs',
      payload: logText
    });
  };
  
  return (
    <div className="panel activity-log">
      <div className="panel-header">
        <h2>Activity Log</h2>
        <div className="log-controls">
          <button className="icon-button" onClick={clearLogs} title="Clear logs">
            🗑️
          </button>
          <button className="icon-button" onClick={exportLogs} title="Export logs">
            📤
          </button>
        </div>
      </div>
      
      <div className="panel-content">
        <div className="log-filters">
          <input
            type="text"
            className="filter-input"
            placeholder="Filter logs..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          
          <select 
            className="filter-select"
            value={levelFilter}
            onChange={(e) => setLevelFilter(e.target.value as LogLevel | 'all')}
          >
            <option value="all">All Levels</option>
            <option value={LogLevel.DEBUG}>Debug</option>
            <option value={LogLevel.INFO}>Info</option>
            <option value={LogLevel.WARN}>Warning</option>
            <option value={LogLevel.ERROR}>Error</option>
          </select>
          
          <select
            className="filter-select"
            value={sourceFilter}
            onChange={(e) => setSourceFilter(e.target.value)}
          >
            <option value="all">All Sources</option>
            {sources.map(source => (
              <option key={source} value={source}>{source}</option>
            ))}
          </select>
        </div>
        
        <div className="log-entries">
          {filteredLogs.length === 0 ? (
            <div className="empty-state">
              <p>No log entries to display</p>
            </div>
          ) : (
            <>
              {filteredLogs.map((log, index) => (
                <div 
                  key={`${log.timestamp}-${index}`} 
                  className={`log-entry log-${log.level.toLowerCase()}`}
                >
                  <div className="log-header">
                    <span className="log-timestamp">{formatDate(log.timestamp)}</span>
                    <span 
                      className="log-level"
                      style={{ color: LOG_LEVEL_COLORS[log.level] }}
                    >
                      {LOG_LEVEL_ICONS[log.level]} {log.level}
                    </span>
                    <span className="log-source">{log.source}</span>
                  </div>
                  <div className="log-message">{log.message}</div>
                  {log.details && (
                    <div className="log-details">
                      <pre>{log.details}</pre>
                    </div>
                  )}
                  {log.metadata && (
                    <div className="log-metadata">
                      {Object.entries(log.metadata).map(([key, value]) => (
                        <span key={key} className="metadata-item">
                          <span className="metadata-key">{key}:</span>
                          <span className="metadata-value">{JSON.stringify(value)}</span>
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))}
              <div ref={logEndRef} />
            </>
          )}
        </div>
      </div>
    </div>
  );
};