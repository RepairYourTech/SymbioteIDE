import React, { memo } from 'react';
import { Handle, Position } from 'reactflow';
import classNames from 'classnames';

interface LoopNodeProps {
  data: {
    label: string;
    config?: any;
    executionStatus?: string;
  };
  selected?: boolean;
}

const LoopNode: React.FC<LoopNodeProps> = ({ data, selected }) => {
  const nodeClasses = classNames('custom-node', 'loop-node', {
    'selected': selected,
    'executing': data.executionStatus === 'running',
    'completed': data.executionStatus === 'completed',
    'failed': data.executionStatus === 'failed'
  });
  
  const getLoopIcon = () => {
    switch (data.config?.loopType) {
      case 'forEach': return '🔄';
      case 'while': return '♻️';
      case 'for': return '🔁';
      default: return '🔄';
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
        <span className="node-icon">{getLoopIcon()}</span>
        <span className="node-type">Loop</span>
      </div>
      
      <div className="node-content">
        <div className="node-label">{data.label}</div>
        {data.config?.loopType && (
          <div className="node-subtitle">{data.config.loopType}</div>
        )}
        {data.config?.maxIterations && (
          <div className="node-info">Max: {data.config.maxIterations}</div>
        )}
      </div>
      
      {data.executionStatus && (
        <div className="node-status">
          <span className={`status-indicator ${data.executionStatus}`} />
        </div>
      )}
      
      <div className="loop-handles">
        <Handle
          type="source"
          position={Position.Bottom}
          id="body"
          className="handle handle-source handle-body"
          style={{ left: '30%' }}
        />
        <span className="handle-label handle-label-body">Body</span>
        
        <Handle
          type="source"
          position={Position.Bottom}
          id="complete"
          className="handle handle-source handle-complete"
          style={{ left: '70%' }}
        />
        <span className="handle-label handle-label-complete">Done</span>
      </div>
    </div>
  );
};

export default memo(LoopNode);