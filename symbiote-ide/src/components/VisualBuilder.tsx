import React, { useState, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './VisualBuilder.css';

// Types for Visual Builder
interface Node {
  id: string;
  type: 'start' | 'agent' | 'condition' | 'action' | 'end' | 'code' | 'api';
  position: { x: number; y: number };
  data: {
    label: string;
    config: any;
    inputs?: string[];
    outputs?: string[];
  };
  selected: boolean;
}

interface Connection {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
}

interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  icon: string;
  nodes: Node[];
  connections: Connection[];
  category: 'agent' | 'automation' | 'api' | 'custom';
}

interface VisualBuilderProps {
  onWorkflowSave?: (workflow: any) => void;
  onWorkflowRun?: (workflow: any) => void;
}

const VisualBuilder: React.FC<VisualBuilderProps> = ({
  onWorkflowSave,
  onWorkflowRun
}) => {
  const [nodes, setNodes] = useState<Node[]>([]);
  const [connections, setConnections] = useState<Connection[]>([]);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [draggedNode, setDraggedNode] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionStart, setConnectionStart] = useState<string | null>(null);
  const [workflowName, setWorkflowName] = useState('New Workflow');
  const [showTemplates, setShowTemplates] = useState(false);
  const [zoom] = useState(1);
  const [pan] = useState({ x: 0, y: 0 });

  const canvasRef = useRef<HTMLDivElement>(null);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });

  // Node templates for the palette
  const nodeTemplates = [
    {
      type: 'start',
      label: 'Start',
      icon: '▶️',
      description: 'Workflow entry point',
      config: { trigger: 'manual' }
    },
    {
      type: 'agent',
      label: 'AI Agent',
      icon: '🤖',
      description: 'AI agent task execution',
      config: { agentType: 'general', model: 'gpt-4', instructions: '' }
    },
    {
      type: 'condition',
      label: 'Condition',
      icon: '❓',
      description: 'Conditional branching',
      config: { condition: '', trueAction: '', falseAction: '' }
    },
    {
      type: 'action',
      label: 'Action',
      icon: '⚡',
      description: 'Execute action or command',
      config: { actionType: 'command', command: '', parameters: {} }
    },
    {
      type: 'code',
      label: 'Code Block',
      icon: '💻',
      description: 'Execute custom code',
      config: { language: 'javascript', code: '', timeout: 30 }
    },
    {
      type: 'api',
      label: 'API Call',
      icon: '🌐',
      description: 'HTTP API request',
      config: { method: 'GET', url: '', headers: {}, body: '' }
    },
    {
      type: 'end',
      label: 'End',
      icon: '🏁',
      description: 'Workflow completion',
      config: { output: '', status: 'success' }
    }
  ];

  // Workflow templates
  const workflowTemplates: WorkflowTemplate[] = [
    {
      id: 'simple-agent',
      name: 'Simple AI Agent',
      description: 'Basic AI agent workflow',
      icon: '🤖',
      category: 'agent',
      nodes: [
        {
          id: 'start-1',
          type: 'start',
          position: { x: 100, y: 100 },
          data: { label: 'Start', config: { trigger: 'manual' } },
          selected: false
        },
        {
          id: 'agent-1',
          type: 'agent',
          position: { x: 300, y: 100 },
          data: { 
            label: 'AI Agent', 
            config: { 
              agentType: 'general', 
              model: 'gpt-4', 
              instructions: 'Process the input and provide a response' 
            } 
          },
          selected: false
        },
        {
          id: 'end-1',
          type: 'end',
          position: { x: 500, y: 100 },
          data: { label: 'End', config: { output: 'result', status: 'success' } },
          selected: false
        }
      ],
      connections: [
        { id: 'conn-1', source: 'start-1', target: 'agent-1' },
        { id: 'conn-2', source: 'agent-1', target: 'end-1' }
      ]
    },
    {
      id: 'code-review',
      name: 'Code Review Workflow',
      description: 'Automated code review with AI',
      icon: '🔍',
      category: 'automation',
      nodes: [
        {
          id: 'start-2',
          type: 'start',
          position: { x: 50, y: 150 },
          data: { label: 'Code Input', config: { trigger: 'file_change' } },
          selected: false
        },
        {
          id: 'agent-2',
          type: 'agent',
          position: { x: 250, y: 100 },
          data: { 
            label: 'Code Analyzer', 
            config: { 
              agentType: 'code_reviewer', 
              model: 'claude-3', 
              instructions: 'Analyze code for bugs, style, and improvements' 
            } 
          },
          selected: false
        },
        {
          id: 'condition-1',
          type: 'condition',
          position: { x: 450, y: 150 },
          data: { 
            label: 'Issues Found?', 
            config: { 
              condition: 'issues.length > 0', 
              trueAction: 'create_pr_comment', 
              falseAction: 'approve' 
            } 
          },
          selected: false
        },
        {
          id: 'action-1',
          type: 'action',
          position: { x: 350, y: 250 },
          data: { 
            label: 'Create Comment', 
            config: { 
              actionType: 'github_comment', 
              command: 'create_pr_comment', 
              parameters: { body: '{{issues}}' } 
            } 
          },
          selected: false
        },
        {
          id: 'end-2',
          type: 'end',
          position: { x: 550, y: 250 },
          data: { label: 'Complete', config: { output: 'review_complete', status: 'success' } },
          selected: false
        }
      ],
      connections: [
        { id: 'conn-3', source: 'start-2', target: 'agent-2' },
        { id: 'conn-4', source: 'agent-2', target: 'condition-1' },
        { id: 'conn-5', source: 'condition-1', target: 'action-1' },
        { id: 'conn-6', source: 'action-1', target: 'end-2' },
        { id: 'conn-7', source: 'condition-1', target: 'end-2' }
      ]
    }
  ];

  // Generate unique ID
  const generateId = () => `node-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

  // Add node to canvas
  const addNode = useCallback((template: any, position?: { x: number; y: number }) => {
    const newNode: Node = {
      id: generateId(),
      type: template.type,
      position: position || { x: 200, y: 200 },
      data: {
        label: template.label,
        config: { ...template.config }
      },
      selected: false
    };

    setNodes(prev => [...prev, newNode]);
  }, []);

  // Handle node drag
  const handleNodeMouseDown = useCallback((e: React.MouseEvent, nodeId: string) => {
    e.preventDefault();
    e.stopPropagation();
    
    const node = nodes.find(n => n.id === nodeId);
    if (!node) return;

    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const offsetX = e.clientX - rect.left - node.position.x * zoom - pan.x;
    const offsetY = e.clientY - rect.top - node.position.y * zoom - pan.y;

    setDragOffset({ x: offsetX, y: offsetY });
    setDraggedNode(nodeId);
    setSelectedNode(nodeId);

    // Clear other selections
    setNodes(prev => prev.map(n => ({ ...n, selected: n.id === nodeId })));
  }, [nodes, zoom, pan]);

  // Handle mouse move for dragging
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!draggedNode || !canvasRef.current) return;

    const rect = canvasRef.current.getBoundingClientRect();
    const x = (e.clientX - rect.left - pan.x - dragOffset.x) / zoom;
    const y = (e.clientY - rect.top - pan.y - dragOffset.y) / zoom;

    setNodes(prev => prev.map(node => 
      node.id === draggedNode 
        ? { ...node, position: { x: Math.max(0, x), y: Math.max(0, y) } }
        : node
    ));
  }, [draggedNode, zoom, pan, dragOffset]);

  // Handle mouse up
  const handleMouseUp = useCallback(() => {
    setDraggedNode(null);
    setDragOffset({ x: 0, y: 0 });
  }, []);

  // Handle connection creation
  const startConnection = useCallback((nodeId: string) => {
    setIsConnecting(true);
    setConnectionStart(nodeId);
  }, []);

  const completeConnection = useCallback((targetNodeId: string) => {
    if (!connectionStart || connectionStart === targetNodeId) {
      setIsConnecting(false);
      setConnectionStart(null);
      return;
    }

    const newConnection: Connection = {
      id: `conn-${Date.now()}`,
      source: connectionStart,
      target: targetNodeId
    };

    setConnections(prev => [...prev, newConnection]);
    setIsConnecting(false);
    setConnectionStart(null);
  }, [connectionStart]);

  // Delete selected node
  const deleteSelectedNode = useCallback(() => {
    if (!selectedNode) return;

    setNodes(prev => prev.filter(n => n.id !== selectedNode));
    setConnections(prev => prev.filter(c => 
      c.source !== selectedNode && c.target !== selectedNode
    ));
    setSelectedNode(null);
  }, [selectedNode]);

  // Load template
  const loadTemplate = useCallback((template: WorkflowTemplate) => {
    setNodes(template.nodes);
    setConnections(template.connections);
    setWorkflowName(template.name);
    setShowTemplates(false);
  }, []);

  // Save workflow
  const saveWorkflow = useCallback(() => {
    const workflow = {
      name: workflowName,
      nodes,
      connections,
      metadata: {
        created: new Date().toISOString(),
        version: '1.0'
      }
    };

    onWorkflowSave?.(workflow);
    console.log('Workflow saved:', workflow);
  }, [workflowName, nodes, connections, onWorkflowSave]);

  // Run workflow
  const runWorkflow = useCallback(async () => {
    const workflow = {
      name: workflowName,
      nodes,
      connections,
      metadata: {
        created: new Date().toISOString(),
        version: '1.0'
      }
    };

    try {
      // Convert workflow to agent execution plan
      const agentTasks = nodes
        .filter(node => node.type === 'agent')
        .map(node => ({
          agent_type: node.data.config.agentType || 'general',
          model: node.data.config.model || 'gpt-4',
          instructions: node.data.config.instructions || '',
          node_id: node.id,
          position: node.position
        }));

      // Execute workflow through PA system
      const planRequest = JSON.stringify({
        workflow_name: workflowName,
        agent_tasks: agentTasks,
        connections: connections,
        execution_mode: 'sequential' // or 'parallel' based on workflow structure
      });

      const planResult = await invoke('pa_generate_plan', { request: planRequest });
      console.log('Workflow plan generated:', planResult);

      // Execute the generated plan
      const planData = JSON.parse(planResult as string);
      const planId = planData.plan_id || 'workflow-' + Date.now();
      await invoke('pa_execute_plan', { plan_id: planId });
      
      console.log('Workflow execution started:', planId);
      onWorkflowRun?.(workflow);
    } catch (error) {
      console.error('Failed to execute workflow:', error);
    }
  }, [workflowName, nodes, connections, onWorkflowRun]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Delete' && selectedNode) {
        deleteSelectedNode();
      }
      if (e.key === 'Escape') {
        setSelectedNode(null);
        setIsConnecting(false);
        setConnectionStart(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedNode, deleteSelectedNode]);

  return (
    <div className="visual-builder">
      {/* Header */}
      <div className="builder-header">
        <div className="header-left">
          <input
            type="text"
            value={workflowName}
            onChange={(e) => setWorkflowName(e.target.value)}
            className="workflow-name-input"
            placeholder="Workflow Name"
          />
          <span className="node-count">{nodes.length} nodes</span>
        </div>

        <div className="header-controls">
          <button
            onClick={() => setShowTemplates(true)}
            className="control-button"
            title="Load Template"
          >
            📋 Templates
          </button>
          <button
            onClick={saveWorkflow}
            className="control-button save"
            title="Save Workflow"
          >
            💾 Save
          </button>
          <button
            onClick={runWorkflow}
            className="control-button run"
            title="Run Workflow"
            disabled={nodes.length === 0}
          >
            ▶️ Run
          </button>
        </div>
      </div>

      <div className="builder-content">
        {/* Node Palette */}
        <div className="node-palette">
          <h4>🧩 Components</h4>
          <div className="palette-nodes">
            {nodeTemplates.map((template) => (
              <div
                key={template.type}
                className="palette-node"
                draggable
                onDragEnd={(e) => {
                  const rect = canvasRef.current?.getBoundingClientRect();
                  if (rect) {
                    const x = (e.clientX - rect.left - pan.x) / zoom;
                    const y = (e.clientY - rect.top - pan.y) / zoom;
                    addNode(template, { x, y });
                  }
                }}
                onClick={() => addNode(template)}
                title={template.description}
              >
                <span className="palette-icon">{template.icon}</span>
                <span className="palette-label">{template.label}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Canvas */}
        <div
          ref={canvasRef}
          className="workflow-canvas"
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onClick={() => setSelectedNode(null)}
        >
          {/* Grid Background */}
          <div className="canvas-grid" />

          {/* Connections */}
          <svg className="connections-layer">
            {connections.map((connection) => {
              const sourceNode = nodes.find(n => n.id === connection.source);
              const targetNode = nodes.find(n => n.id === connection.target);
              
              if (!sourceNode || !targetNode) return null;

              const x1 = sourceNode.position.x + 100; // Node width/2
              const y1 = sourceNode.position.y + 25;  // Node height/2
              const x2 = targetNode.position.x;
              const y2 = targetNode.position.y + 25;

              return (
                <line
                  key={connection.id}
                  x1={x1}
                  y1={y1}
                  x2={x2}
                  y2={y2}
                  stroke="#007acc"
                  strokeWidth="2"
                  markerEnd="url(#arrowhead)"
                />
              );
            })}
            
            {/* Arrow marker definition */}
            <defs>
              <marker
                id="arrowhead"
                markerWidth="10"
                markerHeight="7"
                refX="9"
                refY="3.5"
                orient="auto"
              >
                <polygon
                  points="0 0, 10 3.5, 0 7"
                  fill="#007acc"
                />
              </marker>
            </defs>
          </svg>

          {/* Nodes */}
          {nodes.map((node) => (
            <div
              key={node.id}
              className={`workflow-node ${node.type} ${node.selected ? 'selected' : ''}`}
              style={{
                left: node.position.x,
                top: node.position.y,
                transform: `scale(${zoom})`
              }}
              onMouseDown={(e) => handleNodeMouseDown(e, node.id)}
              onClick={(e) => {
                e.stopPropagation();
                if (isConnecting) {
                  completeConnection(node.id);
                } else {
                  setSelectedNode(node.id);
                }
              }}
            >
              <div className="node-header">
                <span className="node-icon">
                  {nodeTemplates.find(t => t.type === node.type)?.icon || '⚪'}
                </span>
                <span className="node-label">{node.data.label}</span>
              </div>

              <div className="node-handles">
                <div 
                  className="node-handle input"
                  title="Input"
                />
                <div 
                  className="node-handle output"
                  onClick={(e) => {
                    e.stopPropagation();
                    startConnection(node.id);
                  }}
                  title="Output - Click to connect"
                />
              </div>

              {node.selected && (
                <div className="node-controls">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteSelectedNode();
                    }}
                    className="node-delete"
                    title="Delete Node"
                  >
                    🗑️
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Properties Panel */}
        {selectedNode && (
          <div className="properties-panel">
            <h4>⚙️ Properties</h4>
            {(() => {
              const node = nodes.find(n => n.id === selectedNode);
              if (!node) return null;

              return (
                <div className="node-properties">
                  <div className="property-group">
                    <label>Label</label>
                    <input
                      type="text"
                      value={node.data.label}
                      onChange={(e) => {
                        setNodes(prev => prev.map(n => 
                          n.id === selectedNode 
                            ? { ...n, data: { ...n.data, label: e.target.value } }
                            : n
                        ));
                      }}
                    />
                  </div>

                  <div className="property-group">
                    <label>Type</label>
                    <span className="property-value">{node.type}</span>
                  </div>

                  {node.type === 'agent' && (
                    <>
                      <div className="property-group">
                        <label>Agent Type</label>
                        <select
                          value={node.data.config.agentType || 'general'}
                          onChange={(e) => {
                            setNodes(prev => prev.map(n => 
                              n.id === selectedNode 
                                ? { 
                                    ...n, 
                                    data: { 
                                      ...n.data, 
                                      config: { ...n.data.config, agentType: e.target.value } 
                                    } 
                                  }
                                : n
                            ));
                          }}
                        >
                          <option value="general">General</option>
                          <option value="code_reviewer">Code Reviewer</option>
                          <option value="tester">Tester</option>
                          <option value="architect">Architect</option>
                          <option value="debugger">Debugger</option>
                        </select>
                      </div>

                      <div className="property-group">
                        <label>Model</label>
                        <select
                          value={node.data.config.model || 'gpt-4'}
                          onChange={(e) => {
                            setNodes(prev => prev.map(n => 
                              n.id === selectedNode 
                                ? { 
                                    ...n, 
                                    data: { 
                                      ...n.data, 
                                      config: { ...n.data.config, model: e.target.value } 
                                    } 
                                  }
                                : n
                            ));
                          }}
                        >
                          <option value="gpt-4">GPT-4</option>
                          <option value="claude-3">Claude 3</option>
                          <option value="gemini-pro">Gemini Pro</option>
                        </select>
                      </div>

                      <div className="property-group">
                        <label>Instructions</label>
                        <textarea
                          value={node.data.config.instructions || ''}
                          onChange={(e) => {
                            setNodes(prev => prev.map(n => 
                              n.id === selectedNode 
                                ? { 
                                    ...n, 
                                    data: { 
                                      ...n.data, 
                                      config: { ...n.data.config, instructions: e.target.value } 
                                    } 
                                  }
                                : n
                            ));
                          }}
                          placeholder="Enter agent instructions..."
                          rows={4}
                        />
                      </div>
                    </>
                  )}

                  {node.type === 'code' && (
                    <>
                      <div className="property-group">
                        <label>Language</label>
                        <select
                          value={node.data.config.language || 'javascript'}
                          onChange={(e) => {
                            setNodes(prev => prev.map(n => 
                              n.id === selectedNode 
                                ? { 
                                    ...n, 
                                    data: { 
                                      ...n.data, 
                                      config: { ...n.data.config, language: e.target.value } 
                                    } 
                                  }
                                : n
                            ));
                          }}
                        >
                          <option value="javascript">JavaScript</option>
                          <option value="python">Python</option>
                          <option value="rust">Rust</option>
                          <option value="typescript">TypeScript</option>
                        </select>
                      </div>

                      <div className="property-group">
                        <label>Code</label>
                        <textarea
                          value={node.data.config.code || ''}
                          onChange={(e) => {
                            setNodes(prev => prev.map(n => 
                              n.id === selectedNode 
                                ? { 
                                    ...n, 
                                    data: { 
                                      ...n.data, 
                                      config: { ...n.data.config, code: e.target.value } 
                                    } 
                                  }
                                : n
                            ));
                          }}
                          placeholder="Enter code..."
                          rows={6}
                          style={{ fontFamily: 'monospace' }}
                        />
                      </div>
                    </>
                  )}
                </div>
              );
            })()}
          </div>
        )}
      </div>

      {/* Templates Modal */}
      {showTemplates && (
        <div className="templates-modal">
          <div className="modal-content">
            <div className="modal-header">
              <h3>📋 Workflow Templates</h3>
              <button
                onClick={() => setShowTemplates(false)}
                className="modal-close"
              >
                ✕
              </button>
            </div>

            <div className="templates-grid">
              {workflowTemplates.map((template) => (
                <div
                  key={template.id}
                  className="template-card"
                  onClick={() => loadTemplate(template)}
                >
                  <div className="template-icon">{template.icon}</div>
                  <h4>{template.name}</h4>
                  <p>{template.description}</p>
                  <span className="template-category">{template.category}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Status Bar */}
      <div className="builder-status">
        <span>Zoom: {Math.round(zoom * 100)}%</span>
        <span>Nodes: {nodes.length}</span>
        <span>Connections: {connections.length}</span>
        {isConnecting && <span className="connecting-status">🔗 Connecting...</span>}
      </div>
    </div>
  );
};

export default VisualBuilder;
