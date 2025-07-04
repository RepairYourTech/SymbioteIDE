import React, { memo } from 'react';
import { Handle, Position } from 'reactflow';
import classNames from 'classnames';

interface ConditionalNodeProps {
  data: {
    label: string;
    config?: any;
    executionStatus?: string;
  };
  selected?: boolean;
}

const ConditionalNode: React.FC<ConditionalNodeProps> = ({ data, selected }) => {
  const nodeClasses = classNames('custom-node', 'conditional-node', {
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
        <span className="node-icon">❓</span>
        <span className="node-type">Condition</span>
      </div>
      
      <div className="node-content">
        <div className="node-label">{data.label}</div>
        {data.config?.expression && (
          <div className="node-expression">{data.config.expression}</div>
        )}
      </div>
      
      {data.executionStatus && (
        <div className="node-status">
          <span className={`status-indicator ${data.executionStatus}`} />
        </div>
      )}
      
      <div className="condition-handles">
        <Handle
          type="source"
          position={Position.Bottom}
          id="true"
          className="handle handle-source handle-true"
          style={{ left: '30%' }}
        />
        <span className="handle-label handle-label-true">True</span>
        
        <Handle
          type="source"
          position={Position.Bottom}
          id="false"
          className="handle handle-source handle-false"
          style={{ left: '70%' }}
        />
        <span className="handle-label handle-label-false">False</span>
      </div>
    </div>
  );
};

export default memo(ConditionalNode);