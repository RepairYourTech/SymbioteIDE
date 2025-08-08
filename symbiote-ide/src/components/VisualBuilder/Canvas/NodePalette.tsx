import React, { useState } from 'react';
import { NodeType } from '../VisualBuilderPanel';

interface NodePaletteProps {
  onNodeDrop: (nodeType: NodeType, position: { x: number; y: number }) => void;
}

interface NodeCategory {
  id: string;
  name: string;
  icon: string;
  nodes: NodeDefinition[];
}

interface NodeDefinition {
  type: NodeType;
  name: string;
  icon: string;
  description: string;
  category: string;
}

const nodeDefinitions: NodeDefinition[] = [
  // Input Nodes
  { type: 'input-file', name: 'File Input', icon: '📁', description: 'Read files from disk', category: 'input' },
  { type: 'input-user', name: 'User Input', icon: '👤', description: 'Prompt user for input', category: 'input' },
  { type: 'input-api', name: 'API Input', icon: '🌐', description: 'Receive data from APIs', category: 'input' },
  { type: 'input-database', name: 'Database Input', icon: '🗄️', description: 'Query databases', category: 'input' },

  // Agent Nodes
  { type: 'agent-code', name: 'Code Agent', icon: '🤖', description: 'AI code generation and analysis', category: 'agent' },
  { type: 'agent-test', name: 'Test Agent', icon: '🧪', description: 'Automated testing', category: 'agent' },
  { type: 'agent-debug', name: 'Debug Agent', icon: '🐛', description: 'Code debugging and analysis', category: 'agent' },
  { type: 'agent-research', name: 'Research Agent', icon: '🔍', description: 'Information gathering', category: 'agent' },
  { type: 'agent-custom', name: 'Custom Agent', icon: '⚙️', description: 'User-defined agent', category: 'agent' },

  // Transform Nodes
  { type: 'transform-data', name: 'Data Transform', icon: '🔄', description: 'Transform data formats', category: 'transform' },
  { type: 'transform-text', name: 'Text Transform', icon: '📝', description: 'Text processing', category: 'transform' },
  { type: 'transform-json', name: 'JSON Transform', icon: '📋', description: 'JSON manipulation', category: 'transform' },
  { type: 'transform-code', name: 'Code Transform', icon: '💻', description: 'Code generation', category: 'transform' },

  // Control Flow Nodes
  { type: 'control-condition', name: 'Condition', icon: '❓', description: 'Conditional branching', category: 'control' },
  { type: 'control-loop', name: 'Loop', icon: '🔁', description: 'Iteration over data', category: 'control' },
  { type: 'control-parallel', name: 'Parallel', icon: '🔀', description: 'Parallel execution', category: 'control' },
  { type: 'control-merge', name: 'Merge', icon: '🔗', description: 'Merge data streams', category: 'control' },

  // Output Nodes
  { type: 'output-file', name: 'File Output', icon: '💾', description: 'Write files to disk', category: 'output' },
  { type: 'output-api', name: 'API Output', icon: '📤', description: 'Send data to APIs', category: 'output' },
  { type: 'output-database', name: 'Database Output', icon: '💽', description: 'Write to databases', category: 'output' },
  { type: 'output-notification', name: 'Notification', icon: '🔔', description: 'Send notifications', category: 'output' }
];

const categories: NodeCategory[] = [
  {
    id: 'input',
    name: 'Input',
    icon: '📥',
    nodes: nodeDefinitions.filter(n => n.category === 'input')
  },
  {
    id: 'agent',
    name: 'Agents',
    icon: '🤖',
    nodes: nodeDefinitions.filter(n => n.category === 'agent')
  },
  {
    id: 'transform',
    name: 'Transform',
    icon: '🔄',
    nodes: nodeDefinitions.filter(n => n.category === 'transform')
  },
  {
    id: 'control',
    name: 'Control',
    icon: '🎛️',
    nodes: nodeDefinitions.filter(n => n.category === 'control')
  },
  {
    id: 'output',
    name: 'Output',
    icon: '📤',
    nodes: nodeDefinitions.filter(n => n.category === 'output')
  }
];

export const NodePalette: React.FC<NodePaletteProps> = ({ onNodeDrop }) => {
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(
    new Set(['input', 'agent']) // Expand input and agent by default
  );
  const [searchTerm, setSearchTerm] = useState('');
  const [draggedNode, setDraggedNode] = useState<NodeDefinition | null>(null);

  const toggleCategory = (categoryId: string) => {
    setExpandedCategories(prev => {
      const newSet = new Set(prev);
      if (newSet.has(categoryId)) {
        newSet.delete(categoryId);
      } else {
        newSet.add(categoryId);
      }
      return newSet;
    });
  };

  const handleDragStart = (e: React.DragEvent, node: NodeDefinition) => {
    setDraggedNode(node);
    e.dataTransfer.setData('application/json', JSON.stringify(node));
    e.dataTransfer.effectAllowed = 'copy';
    
    // Create drag image
    const dragImage = e.currentTarget.cloneNode(true) as HTMLElement;
    dragImage.style.transform = 'rotate(5deg)';
    dragImage.style.opacity = '0.8';
    document.body.appendChild(dragImage);
    e.dataTransfer.setDragImage(dragImage, 0, 0);
    setTimeout(() => document.body.removeChild(dragImage), 0);
  };

  const handleDragEnd = () => {
    setDraggedNode(null);
  };

  const filteredCategories = categories.map(category => ({
    ...category,
    nodes: category.nodes.filter(node =>
      node.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      node.description.toLowerCase().includes(searchTerm.toLowerCase())
    )
  })).filter(category => category.nodes.length > 0);

  return (
    <div className="node-palette">
      {/* Header */}
      <div className="palette-header">
        <h4>🎨 Node Palette</h4>
        <p>Drag nodes to canvas</p>
      </div>

      {/* Search */}
      <div className="palette-search">
        <input
          type="text"
          placeholder="Search nodes..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="search-input"
        />
        <div className="search-icon">🔍</div>
      </div>

      {/* Categories */}
      <div className="palette-categories">
        {filteredCategories.map(category => (
          <div key={category.id} className="category">
            <div
              className={`category-header ${expandedCategories.has(category.id) ? 'expanded' : ''}`}
              onClick={() => toggleCategory(category.id)}
            >
              <div className="category-icon">{category.icon}</div>
              <div className="category-name">{category.name}</div>
              <div className="category-count">({category.nodes.length})</div>
              <div className="category-toggle">
                {expandedCategories.has(category.id) ? '▼' : '▶'}
              </div>
            </div>

            {expandedCategories.has(category.id) && (
              <div className="category-nodes">
                {category.nodes.map(node => (
                  <div
                    key={node.type}
                    className={`palette-node ${draggedNode?.type === node.type ? 'dragging' : ''}`}
                    draggable
                    onDragStart={(e) => handleDragStart(e, node)}
                    onDragEnd={handleDragEnd}
                    title={node.description}
                  >
                    <div className="node-icon">{node.icon}</div>
                    <div className="node-info">
                      <div className="node-name">{node.name}</div>
                      <div className="node-description">{node.description}</div>
                    </div>
                    <div className="drag-handle">⋮⋮</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Quick Actions */}
      <div className="palette-actions">
        <button
          className="action-button"
          onClick={() => setExpandedCategories(new Set(categories.map(c => c.id)))}
        >
          📖 Expand All
        </button>
        <button
          className="action-button"
          onClick={() => setExpandedCategories(new Set())}
        >
          📕 Collapse All
        </button>
      </div>

      {/* Usage Tips */}
      <div className="palette-tips">
        <h5>💡 Tips</h5>
        <ul>
          <li>Drag nodes to the canvas to add them</li>
          <li>Connect output ports to input ports</li>
          <li>Use agents for AI-powered operations</li>
          <li>Control flow nodes manage execution</li>
        </ul>
      </div>
    </div>
  );
};
