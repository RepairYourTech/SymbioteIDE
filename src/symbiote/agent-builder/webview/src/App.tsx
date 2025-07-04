import React, { useState, useCallback, useRef, useEffect } from 'react';
import ReactFlow, {
  ReactFlowProvider,
  Controls,
  MiniMap,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  ReactFlowInstance,
  BackgroundVariant
} from 'reactflow';
import 'reactflow/dist/style.css';

import { AgentWorkflow, WorkflowNode, WorkflowEdge } from '../../types/workflow-types';
import { useVSCode } from './hooks/useVSCode';
import Toolbar from './components/Toolbar';
import NodePalette from './components/NodePalette';
import PropertyPanel from './components/PropertyPanel';
import Canvas from './components/Canvas';
import './styles/app.css';

const App: React.FC = () => {
  const vscode = useVSCode();
  const [workflow, setWorkflow] = useState<AgentWorkflow | null>(null);
  const [selectedNode, setSelectedNode] = useState<WorkflowNode | null>(null);
  const [executionStatus, setExecutionStatus] = useState<Map<string, string>>(new Map());
  const [isExecuting, setIsExecuting] = useState(false);

  // Handle messages from extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      
      switch (message.type) {
        case 'workflowLoaded':
          setWorkflow(message.payload);
          break;
          
        case 'nodeExecutionUpdate':
          setExecutionStatus(prev => {
            const newStatus = new Map(prev);
            newStatus.set(message.payload.nodeId, message.payload.status);
            return newStatus;
          });
          break;
          
        case 'executionComplete':
          setIsExecuting(false);
          vscode.postMessage({
            type: 'log',
            payload: 'Execution completed'
          });
          break;
          
        case 'executionError':
          setIsExecuting(false);
          console.error('Execution error:', message.payload);
          break;
          
        case 'agentTypesLoaded':
          // Handle agent types
          break;
          
        case 'toolTypesLoaded':
          // Handle tool types
          break;
      }
    };
    
    window.addEventListener('message', handleMessage);
    
    // Request initial data
    vscode.postMessage({ type: 'requestAgentTypes' });
    vscode.postMessage({ type: 'requestToolTypes' });
    
    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [vscode]);
  
  const handleWorkflowChange = useCallback((updatedWorkflow: AgentWorkflow) => {
    setWorkflow(updatedWorkflow);
    
    // Save to document
    vscode.postMessage({
      type: 'saveWorkflow',
      payload: updatedWorkflow
    });
  }, [vscode]);
  
  const handleNodeSelect = useCallback((node: WorkflowNode | null) => {
    setSelectedNode(node);
  }, []);
  
  const handleExecute = useCallback(() => {
    if (!workflow) return;
    
    setIsExecuting(true);
    setExecutionStatus(new Map());
    
    vscode.postMessage({
      type: 'executeWorkflow',
      payload: {
        workflow,
        input: {},
        debug: true
      }
    });
  }, [workflow, vscode]);
  
  const handleExport = useCallback((format: string) => {
    if (!workflow) return;
    
    vscode.postMessage({
      type: 'exportWorkflow',
      payload: {
        workflow,
        format,
        options: {}
      }
    });
  }, [workflow, vscode]);

  if (!workflow) {
    return (
      <div className="loading">
        <p>Loading workflow...</p>
      </div>
    );
  }

  return (
    <div className="app">
      <ReactFlowProvider>
        <div className="header">
          <Toolbar
            onExecute={handleExecute}
            onExport={handleExport}
            isExecuting={isExecuting}
          />
        </div>
        
        <div className="main">
          <div className="sidebar left">
            <NodePalette />
          </div>
          
          <div className="canvas-container">
            <Canvas
              workflow={workflow}
              onWorkflowChange={handleWorkflowChange}
              onNodeSelect={handleNodeSelect}
              executionStatus={executionStatus}
            />
          </div>
          
          <div className="sidebar right">
            <PropertyPanel
              node={selectedNode}
              onNodeUpdate={(updatedNode) => {
                if (!workflow) return;
                
                const updatedNodes = workflow.nodes.map(n =>
                  n.id === updatedNode.id ? updatedNode : n
                );
                
                handleWorkflowChange({
                  ...workflow,
                  nodes: updatedNodes
                });
              }}
            />
          </div>
        </div>
      </ReactFlowProvider>
    </div>
  );
};

export default App;