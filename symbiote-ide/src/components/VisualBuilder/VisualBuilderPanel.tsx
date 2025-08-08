import React, { useState, useCallback, useRef } from 'react';
import './VisualBuilderPanel.css';
import { WorkflowCanvas } from './Canvas/WorkflowCanvas';
import { NodePalette } from './Canvas/NodePalette';
import { PropertyPanel } from './Properties/PropertyPanel';
import { ExecutionPanel } from './Execution/ExecutionPanel';
import { TemplateLibrary } from './Templates/TemplateLibrary';

// Core types for Visual Builder
export interface WorkflowNode {
  id: string;
  type: NodeType;
  position: { x: number; y: number };
  data: NodeData;
  inputs: NodePort[];
  outputs: NodePort[];
}

export interface NodeConnection {
  id: string;
  sourceNodeId: string;
  sourcePortId: string;
  targetNodeId: string;
  targetPortId: string;
  dataType: DataType;
}

export interface NodePort {
  id: string;
  name: string;
  dataType: DataType;
  required: boolean;
  connected: boolean;
}

export interface NodeData {
  label: string;
  description?: string;
  properties: Record<string, any>;
  config: Record<string, any>;
}

export type NodeType = 
  | 'input-file' | 'input-user' | 'input-api' | 'input-database'
  | 'agent-code' | 'agent-test' | 'agent-debug' | 'agent-research' | 'agent-custom'
  | 'transform-data' | 'transform-text' | 'transform-json' | 'transform-code'
  | 'control-condition' | 'control-loop' | 'control-parallel' | 'control-merge'
  | 'output-file' | 'output-api' | 'output-database' | 'output-notification';

export type DataType = 'string' | 'number' | 'boolean' | 'object' | 'array' | 'file' | 'any';

export interface WorkflowState {
  nodes: WorkflowNode[];
  connections: NodeConnection[];
  selectedNodeId: string | null;
  selectedConnectionId: string | null;
  isExecuting: boolean;
  executionResults: Record<string, any>;
}

export interface VisualBuilderProps {
  onWorkflowSave?: (workflow: WorkflowState) => void;
  onWorkflowLoad?: () => WorkflowState | null;
  onWorkflowExecute?: (workflow: WorkflowState) => Promise<void>;
}

export const VisualBuilderPanel: React.FC<VisualBuilderProps> = ({
  onWorkflowSave,
  onWorkflowLoad,
  onWorkflowExecute
}) => {
  const [workflow, setWorkflow] = useState<WorkflowState>({
    nodes: [],
    connections: [],
    selectedNodeId: null,
    selectedConnectionId: null,
    isExecuting: false,
    executionResults: {}
  });

  const [activePanel, setActivePanel] = useState<'properties' | 'execution' | 'templates'>('properties');
  const [canvasMode, setCanvasMode] = useState<'design' | 'execute' | 'debug'>('design');
  const [showGrid, setShowGrid] = useState(true);
  const [snapToGrid, setSnapToGrid] = useState(true);

  const canvasRef = useRef<HTMLDivElement>(null);

  // Node management
  const addNode = useCallback((nodeType: NodeType, position: { x: number; y: number }) => {
    const newNode: WorkflowNode = {
      id: `node-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      type: nodeType,
      position,
      data: {
        label: getDefaultNodeLabel(nodeType),
        properties: {},
        config: {}
      },
      inputs: getDefaultInputs(nodeType),
      outputs: getDefaultOutputs(nodeType)
    };

    setWorkflow(prev => ({
      ...prev,
      nodes: [...prev.nodes, newNode],
      selectedNodeId: newNode.id
    }));
  }, []);

  const updateNode = useCallback((nodeId: string, updates: Partial<WorkflowNode>) => {
    setWorkflow(prev => ({
      ...prev,
      nodes: prev.nodes.map(node => 
        node.id === nodeId ? { ...node, ...updates } : node
      )
    }));
  }, []);

  const deleteNode = useCallback((nodeId: string) => {
    setWorkflow(prev => ({
      ...prev,
      nodes: prev.nodes.filter(node => node.id !== nodeId),
      connections: prev.connections.filter(conn => 
        conn.sourceNodeId !== nodeId && conn.targetNodeId !== nodeId
      ),
      selectedNodeId: prev.selectedNodeId === nodeId ? null : prev.selectedNodeId
    }));
  }, []);

  // Connection management
  const addConnection = useCallback((connection: Omit<NodeConnection, 'id'>) => {
    // Validate connection
    const sourceNode = workflow.nodes.find(n => n.id === connection.sourceNodeId);
    const targetNode = workflow.nodes.find(n => n.id === connection.targetNodeId);
    
    if (!sourceNode || !targetNode) return;

    const sourcePort = sourceNode.outputs.find(p => p.id === connection.sourcePortId);
    const targetPort = targetNode.inputs.find(p => p.id === connection.targetPortId);

    if (!sourcePort || !targetPort) return;

    // Check data type compatibility
    if (!isDataTypeCompatible(sourcePort.dataType, targetPort.dataType)) {
      console.warn('Incompatible data types:', sourcePort.dataType, targetPort.dataType);
      return;
    }

    const newConnection: NodeConnection = {
      ...connection,
      id: `conn-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      dataType: targetPort.dataType
    };

    setWorkflow(prev => ({
      ...prev,
      connections: [...prev.connections, newConnection]
    }));
  }, [workflow.nodes]);

  const deleteConnection = useCallback((connectionId: string) => {
    setWorkflow(prev => ({
      ...prev,
      connections: prev.connections.filter(conn => conn.id !== connectionId),
      selectedConnectionId: prev.selectedConnectionId === connectionId ? null : prev.selectedConnectionId
    }));
  }, []);

  // Workflow operations
  const saveWorkflow = useCallback(() => {
    if (onWorkflowSave) {
      onWorkflowSave(workflow);
    }
    // Also save to localStorage as backup
    localStorage.setItem('visualBuilder_workflow', JSON.stringify(workflow));
  }, [workflow, onWorkflowSave]);

  const loadWorkflow = useCallback(() => {
    let loadedWorkflow: WorkflowState | null = null;
    
    if (onWorkflowLoad) {
      loadedWorkflow = onWorkflowLoad();
    }
    
    if (!loadedWorkflow) {
      // Try loading from localStorage
      const saved = localStorage.getItem('visualBuilder_workflow');
      if (saved) {
        try {
          loadedWorkflow = JSON.parse(saved);
        } catch (error) {
          console.error('Failed to parse saved workflow:', error);
        }
      }
    }

    if (loadedWorkflow) {
      setWorkflow(loadedWorkflow);
    }
  }, [onWorkflowLoad]);

  const executeWorkflow = useCallback(async () => {
    if (workflow.isExecuting) return;

    setWorkflow(prev => ({ ...prev, isExecuting: true }));
    setCanvasMode('execute');

    try {
      if (onWorkflowExecute) {
        await onWorkflowExecute(workflow);
      } else {
        // Mock execution for development
        await new Promise(resolve => setTimeout(resolve, 2000));
        console.log('Workflow executed successfully');
      }
    } catch (error) {
      console.error('Workflow execution failed:', error);
    } finally {
      setWorkflow(prev => ({ ...prev, isExecuting: false }));
      setCanvasMode('design');
    }
  }, [workflow, onWorkflowExecute]);

  const clearWorkflow = useCallback(() => {
    setWorkflow({
      nodes: [],
      connections: [],
      selectedNodeId: null,
      selectedConnectionId: null,
      isExecuting: false,
      executionResults: {}
    });
  }, []);

  const selectedNode = workflow.selectedNodeId 
    ? workflow.nodes.find(n => n.id === workflow.selectedNodeId) 
    : null;

  return (
    <div className="visual-builder-panel">
      {/* Header */}
      <div className="visual-builder-header">
        <div className="header-left">
          <h3>🎨 Visual Builder</h3>
          <p>Create custom agents and workflows visually</p>
        </div>
        <div className="header-controls">
          <div className="mode-selector">
            <button 
              className={`mode-button ${canvasMode === 'design' ? 'active' : ''}`}
              onClick={() => setCanvasMode('design')}
            >
              🎨 Design
            </button>
            <button 
              className={`mode-button ${canvasMode === 'execute' ? 'active' : ''}`}
              onClick={() => setCanvasMode('execute')}
            >
              ▶️ Execute
            </button>
            <button 
              className={`mode-button ${canvasMode === 'debug' ? 'active' : ''}`}
              onClick={() => setCanvasMode('debug')}
            >
              🐛 Debug
            </button>
          </div>
          <div className="workflow-controls">
            <button onClick={saveWorkflow} className="control-button">
              💾 Save
            </button>
            <button onClick={loadWorkflow} className="control-button">
              📁 Load
            </button>
            <button onClick={clearWorkflow} className="control-button">
              🗑️ Clear
            </button>
            <button 
              onClick={executeWorkflow} 
              className="control-button execute-button"
              disabled={workflow.isExecuting || workflow.nodes.length === 0}
            >
              {workflow.isExecuting ? '⏳ Executing...' : '▶️ Execute'}
            </button>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="visual-builder-content">
        {/* Left Panel - Node Palette */}
        <div className="left-panel">
          <NodePalette onNodeDrop={addNode} />
        </div>

        {/* Center Panel - Canvas */}
        <div className="center-panel">
          <div className="canvas-header">
            <div className="canvas-controls">
              <button 
                className={`canvas-toggle ${showGrid ? 'active' : ''}`}
                onClick={() => setShowGrid(!showGrid)}
              >
                📊 Grid
              </button>
              <button 
                className={`canvas-toggle ${snapToGrid ? 'active' : ''}`}
                onClick={() => setSnapToGrid(!snapToGrid)}
              >
                🧲 Snap
              </button>
              <span className="node-count">
                {workflow.nodes.length} nodes, {workflow.connections.length} connections
              </span>
            </div>
          </div>
          <WorkflowCanvas
            ref={canvasRef}
            workflow={workflow}
            canvasMode={canvasMode}
            showGrid={showGrid}
            snapToGrid={snapToGrid}
            onNodeSelect={(nodeId) => setWorkflow(prev => ({ ...prev, selectedNodeId: nodeId }))}
            onNodeUpdate={updateNode}
            onNodeDelete={deleteNode}
            onConnectionAdd={addConnection}
            onConnectionDelete={deleteConnection}
            onConnectionSelect={(connectionId) => setWorkflow(prev => ({ ...prev, selectedConnectionId: connectionId }))}
          />
        </div>

        {/* Right Panel - Properties/Execution/Templates */}
        <div className="right-panel">
          <div className="panel-tabs">
            <button 
              className={`tab ${activePanel === 'properties' ? 'active' : ''}`}
              onClick={() => setActivePanel('properties')}
            >
              ⚙️ Properties
            </button>
            <button 
              className={`tab ${activePanel === 'execution' ? 'active' : ''}`}
              onClick={() => setActivePanel('execution')}
            >
              ▶️ Execution
            </button>
            <button 
              className={`tab ${activePanel === 'templates' ? 'active' : ''}`}
              onClick={() => setActivePanel('templates')}
            >
              📋 Templates
            </button>
          </div>
          <div className="panel-content">
            {activePanel === 'properties' && (
              <PropertyPanel 
                selectedNode={selectedNode || null}
                onNodeUpdate={updateNode}
              />
            )}
            {activePanel === 'execution' && (
              <ExecutionPanel 
                workflow={workflow}
                onExecute={executeWorkflow}
                onClear={clearWorkflow}
              />
            )}
            {activePanel === 'templates' && (
              <TemplateLibrary 
                onTemplateLoad={(template) => setWorkflow(template)}
              />
            )}
          </div>
        </div>
      </div>

      {/* Status Bar */}
      <div className="visual-builder-status">
        <div className="status-left">
          <span className="status-item">
            {workflow.isExecuting ? '🟡 Executing Workflow' : '🟢 Ready'}
          </span>
          {selectedNode && (
            <span className="status-item">
              Selected: {selectedNode.data.label} ({selectedNode.type})
            </span>
          )}
        </div>
        <div className="status-right">
          <span className="status-item">
            Mode: {canvasMode.charAt(0).toUpperCase() + canvasMode.slice(1)}
          </span>
        </div>
      </div>
    </div>
  );
};

// Helper functions
function getDefaultNodeLabel(nodeType: NodeType): string {
  const labels: Record<NodeType, string> = {
    'input-file': 'File Input',
    'input-user': 'User Input',
    'input-api': 'API Input',
    'input-database': 'Database Input',
    'agent-code': 'Code Agent',
    'agent-test': 'Test Agent',
    'agent-debug': 'Debug Agent',
    'agent-research': 'Research Agent',
    'agent-custom': 'Custom Agent',
    'transform-data': 'Data Transform',
    'transform-text': 'Text Transform',
    'transform-json': 'JSON Transform',
    'transform-code': 'Code Transform',
    'control-condition': 'Condition',
    'control-loop': 'Loop',
    'control-parallel': 'Parallel',
    'control-merge': 'Merge',
    'output-file': 'File Output',
    'output-api': 'API Output',
    'output-database': 'Database Output',
    'output-notification': 'Notification'
  };
  return labels[nodeType] || 'Unknown Node';
}

function getDefaultInputs(nodeType: NodeType): NodePort[] {
  // Define default inputs based on node type
  const commonInputs: NodePort[] = [];
  
  if (nodeType.startsWith('transform-') || nodeType.startsWith('output-') || nodeType.startsWith('agent-')) {
    commonInputs.push({
      id: 'input-data',
      name: 'Data',
      dataType: 'any',
      required: true,
      connected: false
    });
  }

  if (nodeType.startsWith('control-')) {
    commonInputs.push({
      id: 'input-condition',
      name: 'Condition',
      dataType: 'boolean',
      required: true,
      connected: false
    });
  }

  return commonInputs;
}

function getDefaultOutputs(nodeType: NodeType): NodePort[] {
  // Define default outputs based on node type
  const commonOutputs: NodePort[] = [];
  
  if (!nodeType.startsWith('output-')) {
    commonOutputs.push({
      id: 'output-result',
      name: 'Result',
      dataType: 'any',
      required: false,
      connected: false
    });
  }

  if (nodeType.startsWith('control-condition')) {
    commonOutputs.push(
      {
        id: 'output-true',
        name: 'True',
        dataType: 'any',
        required: false,
        connected: false
      },
      {
        id: 'output-false',
        name: 'False',
        dataType: 'any',
        required: false,
        connected: false
      }
    );
  }

  return commonOutputs;
}

function isDataTypeCompatible(sourceType: DataType, targetType: DataType): boolean {
  if (sourceType === 'any' || targetType === 'any') return true;
  if (sourceType === targetType) return true;
  
  // Define compatible type conversions
  const compatibleTypes: Record<DataType, DataType[]> = {
    'string': ['object'],
    'number': ['string', 'object'],
    'boolean': ['string', 'object'],
    'object': ['string'],
    'array': ['object'],
    'file': ['string', 'object'],
    'any': ['string', 'number', 'boolean', 'object', 'array', 'file']
  };

  return compatibleTypes[sourceType]?.includes(targetType) || false;
}
