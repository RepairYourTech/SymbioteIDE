import React, { DragEvent } from 'react';
import { NodePaletteItem } from '../../../types/workflow-types';

const nodeCategories: { [key: string]: NodePaletteItem[] } = {
  'AI Agents': [
    {
      type: 'agent',
      label: 'AI Agent',
      icon: '🤖',
      category: 'AI Agents',
      description: 'Execute an AI agent',
      defaultConfig: {
        agentType: 'architecture-analyst'
      }
    }
  ],
  'Tools': [
    {
      type: 'tool',
      label: 'MCP Tool',
      icon: '🔧',
      category: 'Tools',
      description: 'Execute an MCP tool',
      defaultConfig: {
        toolType: 'mcp'
      }
    }
  ],
  'Control Flow': [
    {
      type: 'condition',
      label: 'Condition',
      icon: '❓',
      category: 'Control Flow',
      description: 'Conditional branching',
      defaultConfig: {
        expression: 'true',
        language: 'javascript'
      }
    },
    {
      type: 'loop',
      label: 'Loop',
      icon: '🔄',
      category: 'Control Flow',
      description: 'Loop over items',
      defaultConfig: {
        loopType: 'forEach'
      }
    },
    {
      type: 'parallel',
      label: 'Parallel',
      icon: '⚡',
      category: 'Control Flow',
      description: 'Execute in parallel',
      defaultConfig: {}
    }
  ],
  'Data': [
    {
      type: 'input',
      label: 'Input',
      icon: '📥',
      category: 'Data',
      description: 'Workflow input',
      defaultConfig: {
        dataType: 'input'
      }
    },
    {
      type: 'output',
      label: 'Output',
      icon: '📤',
      category: 'Data',
      description: 'Workflow output',
      defaultConfig: {
        dataType: 'output'
      }
    },
    {
      type: 'transform',
      label: 'Transform',
      icon: '🔄',
      category: 'Data',
      description: 'Transform data',
      defaultConfig: {
        dataType: 'transform'
      }
    },
    {
      type: 'merge',
      label: 'Merge',
      icon: '🔀',
      category: 'Data',
      description: 'Merge multiple inputs',
      defaultConfig: {
        dataType: 'merge'
      }
    },
    {
      type: 'split',
      label: 'Split',
      icon: '🔁',
      category: 'Data',
      description: 'Split data',
      defaultConfig: {
        dataType: 'split'
      }
    }
  ]
};

const NodePalette: React.FC = () => {
  const onDragStart = (event: DragEvent, item: NodePaletteItem) => {
    const nodeData = {
      nodeType: item.type,
      label: item.label,
      config: item.defaultConfig
    };
    
    event.dataTransfer.setData('application/reactflow', JSON.stringify(nodeData));
    event.dataTransfer.effectAllowed = 'move';
  };
  
  return (
    <div className="node-palette">
      <h3>Nodes</h3>
      
      {Object.entries(nodeCategories).map(([category, items]) => (
        <div key={category} className="node-category">
          <h4>{category}</h4>
          <div className="node-items">
            {items.map((item) => (
              <div
                key={`${category}-${item.type}`}
                className="node-item"
                draggable
                onDragStart={(e) => onDragStart(e, item)}
                title={item.description}
              >
                <span className="node-icon">{item.icon}</span>
                <span className="node-label">{item.label}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
      
      <div className="palette-footer">
        <p>Drag nodes to canvas</p>
      </div>
    </div>
  );
};

export default NodePalette;