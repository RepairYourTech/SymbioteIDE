import React, { memo } from 'react';
import { Handle, Position } from 'reactflow';
import classNames from 'classnames';

interface ToolNodeProps {
  data: {
    label: string;
    config?: any;
    executionStatus?: string;
  };
  selected?: boolean;
}

const ToolNode: React.FC<ToolNodeProps> = ({ data, selected }) => {
  const nodeClasses = classNames('custom-node', 'tool-node', {
    'selected': selected,
    'executing': data.executionStatus === 'running',
    'completed': data.executionStatus === 'completed',
    'failed': data.executionStatus === 'failed'
  });
  
  const getToolIcon = () => {
    switch (data.config?.toolType) {
      case 'mcp': return '🔧';
      case 'api': return '🌐';
      case 'custom': return '⚙️';
      default: return '🔨';
    }
  };
  
  return (
    <div className={nodeClasses}>
      <Handle
        type="target"
        position={Position.Top}
        className="handle handle-target"
      />
      
      <div className="node-header">
        <span className="node-icon">{getToolIcon()}</span>
        <span className="node-type">Tool</span>
      </div>
      
      <div className="node-content">
        <div className="node-label">{data.label}</div>
        {data.config?.toolId && (
          <div className="node-subtitle">{data.config.toolId}</div>
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

export default memo(ToolNode);