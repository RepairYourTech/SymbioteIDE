import React from 'react';
import { NodeConnection } from '../VisualBuilderPanel';

interface ConnectionProps {
  connection: NodeConnection;
  path: string;
  isSelected: boolean;
  onClick: () => void;
  onDelete: () => void;
}

export const Connection: React.FC<ConnectionProps> = ({
  connection,
  path,
  isSelected,
  onClick,
  onDelete
}) => {
  const getConnectionColor = (dataType: string): string => {
    const colors: Record<string, string> = {
      'string': '#4CAF50',
      'number': '#2196F3',
      'boolean': '#FF9800',
      'object': '#9C27B0',
      'array': '#F44336',
      'file': '#795548',
      'any': '#607D8B'
    };
    return colors[dataType] || '#607D8B';
  };

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClick();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.stopPropagation();
      onDelete();
    }
  };

  const connectionColor = getConnectionColor(connection.dataType);

  return (
    <g className="connection-group">
      {/* Connection Path */}
      <path
        d={path}
        className={`connection-path ${isSelected ? 'selected' : ''}`}
        style={{
          stroke: connectionColor,
          strokeWidth: isSelected ? 3 : 2
        }}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        tabIndex={0}
      />
      
      {/* Connection Hit Area (invisible, larger for easier clicking) */}
      <path
        d={path}
        className="connection-hit-area"
        style={{
          stroke: 'transparent',
          strokeWidth: 10,
          fill: 'none',
          cursor: 'pointer'
        }}
        onClick={handleClick}
      />

      {/* Connection Label */}
      {isSelected && (
        <g className="connection-label">
          <text
            x="50%"
            y="50%"
            textAnchor="middle"
            dominantBaseline="middle"
            className="connection-text"
            style={{
              fill: connectionColor,
              fontSize: '12px',
              fontWeight: 'bold'
            }}
          >
            {connection.dataType}
          </text>
          
          {/* Delete Button */}
          <circle
            cx="60%"
            cy="40%"
            r="8"
            className="connection-delete-button"
            style={{
              fill: '#ff4444',
              cursor: 'pointer'
            }}
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
          />
          <text
            x="60%"
            y="40%"
            textAnchor="middle"
            dominantBaseline="middle"
            className="connection-delete-text"
            style={{
              fill: 'white',
              fontSize: '10px',
              fontWeight: 'bold',
              pointerEvents: 'none'
            }}
          >
            ×
          </text>
        </g>
      )}

      {/* Connection Flow Animation */}
      <circle
        r="3"
        className="connection-flow"
        style={{
          fill: connectionColor,
          opacity: 0.8
        }}
      >
        <animateMotion
          dur="2s"
          repeatCount="indefinite"
          path={path}
        />
      </circle>
    </g>
  );
};
