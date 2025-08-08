import React, { useState } from 'react';
import { WorkflowState } from '../VisualBuilderPanel';

interface TemplateLibraryProps {
  onTemplateLoad: (template: WorkflowState) => void;
}

interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string;
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  nodeCount: number;
  tags: string[];
  workflow: WorkflowState;
}

const templates: WorkflowTemplate[] = [
  {
    id: 'hello-world',
    name: 'Hello World',
    description: 'Simple workflow that outputs "Hello, World!" message',
    category: 'Getting Started',
    icon: '👋',
    difficulty: 'beginner',
    nodeCount: 2,
    tags: ['basic', 'tutorial', 'output'],
    workflow: {
      nodes: [
        {
          id: 'input-1',
          type: 'input-user',
          position: { x: 100, y: 100 },
          data: {
            label: 'User Input',
            description: 'Get greeting message',
            properties: { prompt: 'Enter your name:' },
            config: {}
          },
          inputs: [],
          outputs: [{ id: 'out-1', name: 'Name', dataType: 'string', required: false, connected: true }]
        },
        {
          id: 'output-1',
          type: 'output-notification',
          position: { x: 400, y: 100 },
          data: {
            label: 'Greeting Output',
            description: 'Display greeting message',
            properties: { message: 'Hello, {{input}}!' },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Input', dataType: 'string', required: true, connected: true }],
          outputs: []
        }
      ],
      connections: [
        {
          id: 'conn-1',
          sourceNodeId: 'input-1',
          sourcePortId: 'out-1',
          targetNodeId: 'output-1',
          targetPortId: 'in-1',
          dataType: 'string'
        }
      ],
      selectedNodeId: null,
      selectedConnectionId: null,
      isExecuting: false,
      executionResults: {}
    }
  },
  {
    id: 'file-processor',
    name: 'File Processor',
    description: 'Read file, transform data, and save results',
    category: 'Data Processing',
    icon: '📁',
    difficulty: 'intermediate',
    nodeCount: 4,
    tags: ['file', 'transform', 'data'],
    workflow: {
      nodes: [
        {
          id: 'input-file-1',
          type: 'input-file',
          position: { x: 50, y: 100 },
          data: {
            label: 'Read File',
            description: 'Read input file',
            properties: { filePath: 'input.txt', encoding: 'utf-8' },
            config: {}
          },
          inputs: [],
          outputs: [{ id: 'out-1', name: 'Content', dataType: 'string', required: false, connected: true }]
        },
        {
          id: 'transform-1',
          type: 'transform-text',
          position: { x: 300, y: 100 },
          data: {
            label: 'Transform Text',
            description: 'Process text content',
            properties: { operation: 'uppercase' },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Text', dataType: 'string', required: true, connected: true }],
          outputs: [{ id: 'out-1', name: 'Result', dataType: 'string', required: false, connected: true }]
        },
        {
          id: 'output-file-1',
          type: 'output-file',
          position: { x: 550, y: 100 },
          data: {
            label: 'Save File',
            description: 'Save processed content',
            properties: { filePath: 'output.txt', format: 'text' },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Content', dataType: 'string', required: true, connected: true }],
          outputs: []
        }
      ],
      connections: [
        {
          id: 'conn-1',
          sourceNodeId: 'input-file-1',
          sourcePortId: 'out-1',
          targetNodeId: 'transform-1',
          targetPortId: 'in-1',
          dataType: 'string'
        },
        {
          id: 'conn-2',
          sourceNodeId: 'transform-1',
          sourcePortId: 'out-1',
          targetNodeId: 'output-file-1',
          targetPortId: 'in-1',
          dataType: 'string'
        }
      ],
      selectedNodeId: null,
      selectedConnectionId: null,
      isExecuting: false,
      executionResults: {}
    }
  },
  {
    id: 'ai-code-generator',
    name: 'AI Code Generator',
    description: 'Generate code using AI agents with testing and validation',
    category: 'AI Development',
    icon: '🤖',
    difficulty: 'advanced',
    nodeCount: 6,
    tags: ['ai', 'code', 'testing', 'agents'],
    workflow: {
      nodes: [
        {
          id: 'input-1',
          type: 'input-user',
          position: { x: 50, y: 100 },
          data: {
            label: 'Code Request',
            description: 'Get code requirements',
            properties: { prompt: 'Describe the code you want to generate:' },
            config: {}
          },
          inputs: [],
          outputs: [{ id: 'out-1', name: 'Requirements', dataType: 'string', required: false, connected: true }]
        },
        {
          id: 'agent-code-1',
          type: 'agent-code',
          position: { x: 300, y: 100 },
          data: {
            label: 'Code Agent',
            description: 'Generate code using AI',
            properties: { language: 'typescript', model: 'gpt-4', temperature: 0.7 },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Task', dataType: 'string', required: true, connected: true }],
          outputs: [{ id: 'out-1', name: 'Code', dataType: 'string', required: false, connected: true }]
        },
        {
          id: 'agent-test-1',
          type: 'agent-test',
          position: { x: 550, y: 100 },
          data: {
            label: 'Test Agent',
            description: 'Generate tests for code',
            properties: { testType: 'unit', framework: 'jest' },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Code', dataType: 'string', required: true, connected: true }],
          outputs: [{ id: 'out-1', name: 'Tests', dataType: 'string', required: false, connected: true }]
        },
        {
          id: 'output-file-1',
          type: 'output-file',
          position: { x: 800, y: 50 },
          data: {
            label: 'Save Code',
            description: 'Save generated code',
            properties: { filePath: 'generated.ts', format: 'text' },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Content', dataType: 'string', required: true, connected: true }],
          outputs: []
        },
        {
          id: 'output-file-2',
          type: 'output-file',
          position: { x: 800, y: 150 },
          data: {
            label: 'Save Tests',
            description: 'Save generated tests',
            properties: { filePath: 'generated.test.ts', format: 'text' },
            config: {}
          },
          inputs: [{ id: 'in-1', name: 'Content', dataType: 'string', required: true, connected: true }],
          outputs: []
        }
      ],
      connections: [
        {
          id: 'conn-1',
          sourceNodeId: 'input-1',
          sourcePortId: 'out-1',
          targetNodeId: 'agent-code-1',
          targetPortId: 'in-1',
          dataType: 'string'
        },
        {
          id: 'conn-2',
          sourceNodeId: 'agent-code-1',
          sourcePortId: 'out-1',
          targetNodeId: 'agent-test-1',
          targetPortId: 'in-1',
          dataType: 'string'
        },
        {
          id: 'conn-3',
          sourceNodeId: 'agent-code-1',
          sourcePortId: 'out-1',
          targetNodeId: 'output-file-1',
          targetPortId: 'in-1',
          dataType: 'string'
        },
        {
          id: 'conn-4',
          sourceNodeId: 'agent-test-1',
          sourcePortId: 'out-1',
          targetNodeId: 'output-file-2',
          targetPortId: 'in-1',
          dataType: 'string'
        }
      ],
      selectedNodeId: null,
      selectedConnectionId: null,
      isExecuting: false,
      executionResults: {}
    }
  }
];

export const TemplateLibrary: React.FC<TemplateLibraryProps> = ({ onTemplateLoad }) => {
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedDifficulty, setSelectedDifficulty] = useState<string>('all');

  const categories = ['all', ...new Set(templates.map(t => t.category))];
  const difficulties = ['all', 'beginner', 'intermediate', 'advanced'];

  const filteredTemplates = templates.filter(template => {
    const matchesCategory = selectedCategory === 'all' || template.category === selectedCategory;
    const matchesDifficulty = selectedDifficulty === 'all' || template.difficulty === selectedDifficulty;
    const matchesSearch = searchTerm === '' || 
      template.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      template.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
      template.tags.some(tag => tag.toLowerCase().includes(searchTerm.toLowerCase()));

    return matchesCategory && matchesDifficulty && matchesSearch;
  });

  const getDifficultyColor = (difficulty: string) => {
    switch (difficulty) {
      case 'beginner': return '#4CAF50';
      case 'intermediate': return '#FF9800';
      case 'advanced': return '#F44336';
      default: return '#888';
    }
  };

  const getDifficultyIcon = (difficulty: string) => {
    switch (difficulty) {
      case 'beginner': return '🟢';
      case 'intermediate': return '🟡';
      case 'advanced': return '🔴';
      default: return '⚪';
    }
  };

  return (
    <div className="template-library">
      <div className="template-header">
        <h4>📋 Templates</h4>
        <p>Pre-built workflow templates</p>
      </div>

      <div className="template-content">
        {/* Filters */}
        <div className="template-filters">
          <div className="filter-group">
            <label>Search</label>
            <input
              type="text"
              placeholder="Search templates..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="filter-input"
            />
          </div>

          <div className="filter-group">
            <label>Category</label>
            <select
              value={selectedCategory}
              onChange={(e) => setSelectedCategory(e.target.value)}
              className="filter-select"
            >
              {categories.map(category => (
                <option key={category} value={category}>
                  {category === 'all' ? 'All Categories' : category}
                </option>
              ))}
            </select>
          </div>

          <div className="filter-group">
            <label>Difficulty</label>
            <select
              value={selectedDifficulty}
              onChange={(e) => setSelectedDifficulty(e.target.value)}
              className="filter-select"
            >
              {difficulties.map(difficulty => (
                <option key={difficulty} value={difficulty}>
                  {difficulty === 'all' ? 'All Levels' : difficulty.charAt(0).toUpperCase() + difficulty.slice(1)}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Template Grid */}
        <div className="template-grid">
          {filteredTemplates.map(template => (
            <div key={template.id} className="template-card">
              <div className="template-card-header">
                <div className="template-icon">{template.icon}</div>
                <div className="template-info">
                  <div className="template-name">{template.name}</div>
                  <div className="template-category">{template.category}</div>
                </div>
                <div className="template-difficulty">
                  <span 
                    className="difficulty-badge"
                    style={{ color: getDifficultyColor(template.difficulty) }}
                  >
                    {getDifficultyIcon(template.difficulty)} {template.difficulty}
                  </span>
                </div>
              </div>

              <div className="template-card-body">
                <div className="template-description">
                  {template.description}
                </div>

                <div className="template-stats">
                  <div className="stat">
                    <span className="stat-icon">🔗</span>
                    <span className="stat-value">{template.nodeCount} nodes</span>
                  </div>
                  <div className="stat">
                    <span className="stat-icon">⚡</span>
                    <span className="stat-value">{template.workflow.connections.length} connections</span>
                  </div>
                </div>

                <div className="template-tags">
                  {template.tags.map(tag => (
                    <span key={tag} className="template-tag">
                      {tag}
                    </span>
                  ))}
                </div>
              </div>

              <div className="template-card-footer">
                <button
                  className="template-load-button"
                  onClick={() => onTemplateLoad(template.workflow)}
                >
                  📥 Load Template
                </button>
                <button className="template-preview-button">
                  👁️ Preview
                </button>
              </div>
            </div>
          ))}
        </div>

        {filteredTemplates.length === 0 && (
          <div className="no-templates">
            <div className="no-templates-icon">📭</div>
            <div className="no-templates-text">
              No templates found matching your criteria
            </div>
            <button
              className="clear-filters-button"
              onClick={() => {
                setSearchTerm('');
                setSelectedCategory('all');
                setSelectedDifficulty('all');
              }}
            >
              Clear Filters
            </button>
          </div>
        )}

        {/* Template Creation */}
        <div className="template-creation">
          <h5>💡 Create Your Own</h5>
          <p>Build a workflow and save it as a template for reuse</p>
          <div className="creation-actions">
            <button className="action-button">
              💾 Save Current as Template
            </button>
            <button className="action-button">
              📤 Export Template
            </button>
            <button className="action-button">
              📥 Import Template
            </button>
          </div>
        </div>

        {/* Quick Start Guide */}
        <div className="quick-start">
          <h5>🚀 Quick Start</h5>
          <ol>
            <li>Choose a template that matches your needs</li>
            <li>Click "Load Template" to add it to your canvas</li>
            <li>Customize the nodes and connections</li>
            <li>Execute your workflow</li>
          </ol>
        </div>
      </div>
    </div>
  );
};
