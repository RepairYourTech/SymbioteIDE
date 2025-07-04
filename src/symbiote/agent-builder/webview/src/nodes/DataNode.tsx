import React, { memo } from 'react';
import { Handle, Position } from 'reactflow';
import classNames from 'classnames';

interface DataNodeProps {
  data: {
    label: string;
    config?: any;
    executionStatus?: string;
  };
  selected?: boolean;
  type?: string;
}

const DataNode: React.FC<DataNodeProps> = ({ data, selected, type }) => {
  const nodeClasses = classNames('custom-node', 'data-node', `data-node-${type}`, {
    'selected': selected,
    'executing': data.executionStatus === 'running',
    'completed': data.executionStatus === 'completed',
    'failed': data.executionStatus === 'failed'
  });
  
  const getNodeIcon = () => {
    switch (type) {
      case 'input': return '📥';
      case 'output': return '📤';
      case 'transform': return '🔄';
      case 'merge': return '🔀';
      case 'split': return '🔁';
      default: return '📊';
    }
  };
  
  const getNodeType = () => {
    switch (type) {
      case 'input': return 'Input';
      case 'output': return 'Output';
      case 'transform': return 'Transform';
      case 'merge': return 'Merge';
      case 'split': return 'Split';
      default: return 'Data';
    }
  };
  
  const showTopHandle = type !== 'input';
  const showBottomHandle = type !== 'output';
  
  return (
    <div className={nodeClasses}>
      {showTopHandle && (
        <Handle
          type="target"
          position={Position.Top}
          className="handle handle-target"
        />
      )}
      
      <div className="node-header">
        <span className="node-icon">{getNodeIcon()}</span>
        <span className="node-type">{getNodeType()}</span>
      </div>
      
      <div className="node-content">
        <div className="node-label">{data.label}</div>
        {data.config?.dataType && (
          <div className="node-subtitle">{data.config.dataType}</div>
        )}
        {data.config?.value && (
          <div className="node-value">
            {typeof data.config.value === 'object' 
              ? JSON.stringify(data.config.value, null, 2).substring(0, 50) + '...'
              : String(data.config.value).substring(0, 50)
            }
          </div>
        )}
      </div>
      
      {data.executionStatus && (
        <div className="node-status">
          <span className={`status-indicator ${data.executionStatus}`} />
        </div>
      )}
      
      {showBottomHandle && (
        <Handle
          type="source"
          position={Position.Bottom}
          className="handle handle-source"
        />
      )}
    </div>
  );
};

export default memo(DataNode);