import React, { useState } from 'react';
import { WorkflowState } from '../VisualBuilderPanel';

interface ExecutionPanelProps {
  workflow: WorkflowState;
  onExecute: () => Promise<void>;
  onClear: () => void;
}

export const ExecutionPanel: React.FC<ExecutionPanelProps> = ({
  workflow,
  onExecute,
  onClear
}) => {
  const [executionLog, setExecutionLog] = useState<string[]>([
    '🟢 Workflow execution system ready',
    '📊 Waiting for workflow execution...'
  ]);

  const validateWorkflow = () => {
    const issues: string[] = [];
    
    if (workflow.nodes.length === 0) {
      issues.push('No nodes in workflow');
    }

    // Check for disconnected nodes
    const connectedNodes = new Set();
    workflow.connections.forEach(conn => {
      connectedNodes.add(conn.sourceNodeId);
      connectedNodes.add(conn.targetNodeId);
    });

    const disconnectedNodes = workflow.nodes.filter(node => 
      !connectedNodes.has(node.id) && workflow.nodes.length > 1
    );

    if (disconnectedNodes.length > 0) {
      issues.push(`${disconnectedNodes.length} disconnected nodes`);
    }

    // Check for input nodes
    const inputNodes = workflow.nodes.filter(node => node.type.startsWith('input-'));
    if (inputNodes.length === 0 && workflow.nodes.length > 0) {
      issues.push('No input nodes found');
    }

    // Check for output nodes
    const outputNodes = workflow.nodes.filter(node => node.type.startsWith('output-'));
    if (outputNodes.length === 0 && workflow.nodes.length > 0) {
      issues.push('No output nodes found');
    }

    return issues;
  };

  const getExecutionStats = () => {
    const nodesByType = workflow.nodes.reduce((acc, node) => {
      const category = node.type.split('-')[0];
      acc[category] = (acc[category] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    return {
      totalNodes: workflow.nodes.length,
      totalConnections: workflow.connections.length,
      nodesByType,
      estimatedDuration: workflow.nodes.length * 0.5 // Mock estimation
    };
  };

  const handleExecute = async () => {
    const issues = validateWorkflow();
    if (issues.length > 0) {
      setExecutionLog(prev => [
        ...prev,
        '❌ Workflow validation failed:',
        ...issues.map(issue => `  • ${issue}`)
      ]);
      return;
    }

    setExecutionLog(prev => [
      ...prev,
      '🚀 Starting workflow execution...',
      `📊 Executing ${workflow.nodes.length} nodes with ${workflow.connections.length} connections`
    ]);

    try {
      await onExecute();
      setExecutionLog(prev => [
        ...prev,
        '✅ Workflow executed successfully!',
        `⏱️ Completed in ${Math.random() * 5 + 1}s`
      ]);
    } catch (error) {
      setExecutionLog(prev => [
        ...prev,
        '❌ Workflow execution failed:',
        `  Error: ${error}`
      ]);
    }
  };

  const clearLog = () => {
    setExecutionLog([
      '🟢 Workflow execution system ready',
      '📊 Execution log cleared'
    ]);
  };

  const stats = getExecutionStats();
  const validationIssues = validateWorkflow();

  return (
    <div className="execution-panel">
      <div className="execution-header">
        <h4>▶️ Execution</h4>
        <p>Run and monitor workflows</p>
      </div>

      <div className="execution-content">
        {/* Workflow Statistics */}
        <div className="execution-section">
          <h5>📊 Workflow Stats</h5>
          <div className="stats-grid">
            <div className="stat-item">
              <div className="stat-label">Total Nodes</div>
              <div className="stat-value">{stats.totalNodes}</div>
            </div>
            <div className="stat-item">
              <div className="stat-label">Connections</div>
              <div className="stat-value">{stats.totalConnections}</div>
            </div>
            <div className="stat-item">
              <div className="stat-label">Est. Duration</div>
              <div className="stat-value">{stats.estimatedDuration.toFixed(1)}s</div>
            </div>
          </div>

          {Object.keys(stats.nodesByType).length > 0 && (
            <div className="node-breakdown">
              <h6>Node Breakdown</h6>
              {Object.entries(stats.nodesByType).map(([type, count]) => (
                <div key={type} className="breakdown-item">
                  <span className="breakdown-type">{type}</span>
                  <span className="breakdown-count">{count}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Validation Status */}
        <div className="execution-section">
          <h5>✅ Validation</h5>
          {validationIssues.length === 0 ? (
            <div className="validation-success">
              <div className="validation-icon">✅</div>
              <div className="validation-text">Workflow is valid and ready to execute</div>
            </div>
          ) : (
            <div className="validation-errors">
              <div className="validation-icon">❌</div>
              <div className="validation-text">Workflow has validation issues:</div>
              <ul className="validation-issues">
                {validationIssues.map((issue, index) => (
                  <li key={index}>{issue}</li>
                ))}
              </ul>
            </div>
          )}
        </div>

        {/* Execution Controls */}
        <div className="execution-section">
          <h5>🎮 Controls</h5>
          <div className="execution-controls">
            <button
              className="execute-button primary"
              onClick={handleExecute}
              disabled={workflow.isExecuting || validationIssues.length > 0}
            >
              {workflow.isExecuting ? '⏳ Executing...' : '▶️ Execute Workflow'}
            </button>
            <button
              className="control-button"
              onClick={onClear}
              disabled={workflow.isExecuting}
            >
              🗑️ Clear Workflow
            </button>
            <button
              className="control-button"
              onClick={clearLog}
            >
              📝 Clear Log
            </button>
          </div>
        </div>

        {/* Execution Log */}
        <div className="execution-section">
          <h5>📝 Execution Log</h5>
          <div className="execution-log">
            {executionLog.map((entry, index) => (
              <div key={index} className="log-entry">
                <span className="log-timestamp">
                  {new Date().toLocaleTimeString()}
                </span>
                <span className="log-message">{entry}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Execution Results */}
        {Object.keys(workflow.executionResults).length > 0 && (
          <div className="execution-section">
            <h5>📋 Results</h5>
            <div className="execution-results">
              {Object.entries(workflow.executionResults).map(([nodeId, result]) => {
                const node = workflow.nodes.find(n => n.id === nodeId);
                return (
                  <div key={nodeId} className="result-item">
                    <div className="result-node">{node?.data.label || nodeId}</div>
                    <div className="result-value">
                      {typeof result === 'object' 
                        ? JSON.stringify(result, null, 2)
                        : String(result)
                      }
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Performance Metrics */}
        <div className="execution-section">
          <h5>⚡ Performance</h5>
          <div className="performance-metrics">
            <div className="metric-item">
              <div className="metric-label">Memory Usage</div>
              <div className="metric-value">
                {(Math.random() * 50 + 10).toFixed(1)} MB
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">CPU Usage</div>
              <div className="metric-value">
                {(Math.random() * 30 + 5).toFixed(1)}%
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">Network I/O</div>
              <div className="metric-value">
                {(Math.random() * 100 + 10).toFixed(0)} KB/s
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
