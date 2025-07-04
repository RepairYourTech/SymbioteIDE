import React, { memo } from 'react';
import { Handle, Position } from 'reactflow';
import classNames from 'classnames';

interface AgentNodeProps {
  data: {
    label: string;
    config?: any;
    executionStatus?: string;
  };
  selected?: boolean;
}

const AgentNode: React.FC<AgentNodeProps> = ({ data, selected }) => {
  const nodeClasses = classNames('custom-node', 'agent-node', {
    'selected': selected,
    'executing': data.executionStatus === 'running',
    'completed': data.executionStatus === 'completed',
    'failed': data.executionStatus === 'failed'
  });
  
  return (
    <div className={nodeClasses}>
      <Handle
        type="target"
        position={Position.Top}
        className="handle handle-target"
      />
      
      <div className="node-header">
        <span className="node-icon">🤖</span>
        <span className="node-type">Agent</span>
      </div>
      
      <div className="node-content">
        <div className="node-label">{data.label}</div>
        {data.config?.agentType && (
          <div className="node-subtitle">{data.config.agentType}</div>
        )}
      </div>
      
      {data.executionStatus && (
        <div className="node-status">
          <span className={`status-indicator ${data.executionStatus}`} />
        </div>
      )}
      
      <Handle
        type="source"
        position={Position.Bottom}
        className="handle handle-source"
      />
    </div>
  );
};

export default memo(AgentNode);