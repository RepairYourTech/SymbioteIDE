import React from 'react';
import { WorkflowNode, NodePort } from '../VisualBuilderPanel';

interface NodeProps {
  node: WorkflowNode;
  isSelected: boolean;
  isExecuting: boolean;
  canvasMode: 'design' | 'execute' | 'debug';
  onMouseDown: (e: React.MouseEvent) => void;
  onDoubleClick: () => void;
  onDelete: () => void;
  onPortMouseDown: (e: React.MouseEvent, nodeId: string, portId: string, portType: 'input' | 'output') => void;
  onPortMouseUp: (e: React.MouseEvent, nodeId: string, portId: string, portType: 'input' | 'output') => void;
}

export const Node: React.FC<NodeProps> = ({
  node,
  isSelected,
  isExecuting,
  canvasMode,
  onMouseDown,
  onDoubleClick,
  onDelete,
  onPortMouseDown,
  onPortMouseUp
}) => {
  const getNodeIcon = (nodeType: string): string => {
    const icons: Record<string, string> = {
      'input-file': '📁',
      'input-user': '👤',
      'input-api': '🌐',
      'input-database': '🗄️',
      'agent-code': '🤖',
      'agent-test': '🧪',
      'agent-debug': '🐛',
      'agent-research': '🔍',
      'agent-custom': '⚙️',
      'transform-data': '🔄',
      'transform-text': '📝',
      'transform-json': '📋',
      'transform-code': '💻',
      'control-condition': '❓',
      'control-loop': '🔁',
      'control-parallel': '🔀',
      'control-merge': '🔗',
      'output-file': '💾',
      'output-api': '📤',
      'output-database': '💽',
      'output-notification': '🔔'
    };
    return icons[nodeType] || '⚪';
  };

  const getNodeCategory = (nodeType: string): string => {
    if (nodeType.startsWith('input-')) return 'input';
    if (nodeType.startsWith('agent-')) return 'agent';
    if (nodeType.startsWith('transform-')) return 'transform';
    if (nodeType.startsWith('control-')) return 'control';
    if (nodeType.startsWith('output-')) return 'output';
    return 'unknown';
  };

  const getPortPosition = (port: NodePort, index: number, total: number, isInput: boolean): { top: string } => {
    const spacing = total > 1 ? 100 / (total + 1) : 50;
    const position = spacing * (index + 1);
    return { top: `${position}%` };
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.stopPropagation();
      onDelete();
    }
  };

  const nodeCategory = getNodeCategory(node.type);
  const nodeIcon = getNodeIcon(node.type);

  return (
    <div
      className={`workflow-node ${nodeCategory} ${isSelected ? 'selected' : ''} ${isExecuting ? 'executing' : ''}`}
      style={{
        left: node.position.x,
        top: node.position.y,
        zIndex: isSelected ? 10 : 5
      }}
      onMouseDown={onMouseDown}
      onDoubleClick={onDoubleClick}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      {/* Node Header */}
      <div className="node-header">
        <div className="node-icon">{nodeIcon}</div>
        <div className="node-title">{node.data.label}</div>
        <div className="node-type">{node.type.split('-')[1]}</div>
        {canvasMode === 'design' && isSelected && (
          <button 
            className="node-delete-button"
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            title="Delete node"
          >
            ×
          </button>
        )}
      </div>

      {/* Node Body */}
      <div className="node-body">
        {node.data.description && (
          <div className="node-description">{node.data.description}</div>
        )}
        
        {/* Node Properties Display */}
        {Object.keys(node.data.properties).length > 0 && (
          <div className="node-properties">
            {Object.entries(node.data.properties).slice(0, 3).map(([key, value]) => (
              <div key={key} className="node-property">
                <span className="property-label">{key}:</span>
                <span className="property-value">
                  {typeof value === 'string' && value.length > 20 
                    ? `${value.substring(0, 20)}...` 
                    : String(value)
                  }
                </span>
              </div>
            ))}
            {Object.keys(node.data.properties).length > 3 && (
              <div className="node-property">
                <span className="property-label">...</span>
                <span className="property-value">
                  +{Object.keys(node.data.properties).length - 3} more
                </span>
              </div>
            )}
          </div>
        )}

        {/* Execution Status */}
        {canvasMode === 'execute' && (
          <div className="execution-status">
            {isExecuting ? (
              <div className="status-executing">
                <div className="spinner"></div>
                <span>Executing...</span>
              </div>
            ) : (
              <div className="status-ready">
                <span>✓ Ready</span>
              </div>
            )}
          </div>
        )}

        {/* Debug Information */}
        {canvasMode === 'debug' && (
          <div className="debug-info">
            <div className="debug-item">
              <span className="debug-label">ID:</span>
              <span className="debug-value">{node.id.split('-').pop()}</span>
            </div>
            <div className="debug-item">
              <span className="debug-label">Pos:</span>
              <span className="debug-value">
                {Math.round(node.position.x)}, {Math.round(node.position.y)}
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Input Ports */}
      {node.inputs.length > 0 && (
        <div className="node-ports inputs">
          {node.inputs.map((port, index) => (
            <div
              key={port.id}
              className={`node-port input-port ${port.connected ? 'connected' : ''}`}
              style={getPortPosition(port, index, node.inputs.length, true)}
              title={`${port.name} (${port.dataType})`}
              onMouseDown={(e) => onPortMouseDown(e, node.id, port.id, 'input')}
              onMouseUp={(e) => onPortMouseUp(e, node.id, port.id, 'input')}
            >
              <div className="port-indicator"></div>
              {canvasMode === 'debug' && (
                <div className="port-label">{port.name}</div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Output Ports */}
      {node.outputs.length > 0 && (
        <div className="node-ports outputs">
          {node.outputs.map((port, index) => (
            <div
              key={port.id}
              className={`node-port output-port ${port.connected ? 'connected' : ''}`}
              style={getPortPosition(port, index, node.outputs.length, false)}
              title={`${port.name} (${port.dataType})`}
              onMouseDown={(e) => onPortMouseDown(e, node.id, port.id, 'output')}
              onMouseUp={(e) => onPortMouseUp(e, node.id, port.id, 'output')}
            >
              <div className="port-indicator"></div>
              {canvasMode === 'debug' && (
                <div className="port-label">{port.name}</div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Resize Handle */}
      {canvasMode === 'design' && isSelected && (
        <div className="resize-handle" title="Resize node">
          ⋰
        </div>
      )}
    </div>
  );
};
