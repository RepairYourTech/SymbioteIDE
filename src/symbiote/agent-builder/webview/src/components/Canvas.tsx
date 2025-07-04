import React, { useCallback, useRef, useState, DragEvent } from 'react';
import ReactFlow, {
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
  BackgroundVariant,
  updateEdge,
  MarkerType
} from 'reactflow';
import { v4 as uuidv4 } from 'uuid';

import { AgentWorkflow, WorkflowNode, WorkflowEdge, NodeType } from '../../../types/workflow-types';
import AgentNode from '../nodes/AgentNode';
import ToolNode from '../nodes/ToolNode';
import ConditionalNode from '../nodes/ConditionalNode';
import LoopNode from '../nodes/LoopNode';
import DataNode from '../nodes/DataNode';

// Define custom node types
const nodeTypes = {
  agent: AgentNode,
  tool: ToolNode,
  condition: ConditionalNode,
  loop: LoopNode,
  data: DataNode,
  input: DataNode,
  output: DataNode,
  transform: DataNode,
  merge: DataNode,
  split: DataNode
};

interface CanvasProps {
  workflow: AgentWorkflow;
  onWorkflowChange: (workflow: AgentWorkflow) => void;
  onNodeSelect: (node: WorkflowNode | null) => void;
  executionStatus: Map<string, string>;
}

const Canvas: React.FC<CanvasProps> = ({
  workflow,
  onWorkflowChange,
  onNodeSelect,
  executionStatus
}) => {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const [reactFlowInstance, setReactFlowInstance] = useState<ReactFlowInstance | null>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState(
    workflow.nodes.map(node => ({
      ...node,
      data: {
        ...node.data,
        executionStatus: executionStatus.get(node.id)
      }
    }))
  );
  const [edges, setEdges, onEdgesChange] = useEdgesState(
    workflow.edges.map(edge => ({
      ...edge,
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 20,
        height: 20
      }
    }))
  );
  const edgeUpdateSuccessful = useRef(true);
  
  // Update nodes when execution status changes
  React.useEffect(() => {
    setNodes(nodes => 
      nodes.map(node => ({
        ...node,
        data: {
          ...node.data,
          executionStatus: executionStatus.get(node.id)
        }
      }))
    );
  }, [executionStatus, setNodes]);
  
  // Sync with workflow prop
  React.useEffect(() => {
    setNodes(workflow.nodes.map(node => ({
      ...node,
      data: {
        ...node.data,
        executionStatus: executionStatus.get(node.id)
      }
    })));
    setEdges(workflow.edges.map(edge => ({
      ...edge,
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 20,
        height: 20
      }
    })));
  }, [workflow, executionStatus, setNodes, setEdges]);
  
  const onConnect = useCallback(
    (params: Connection) => {
      const newEdge: Edge = {
        id: `edge_${Date.now()}`,
        ...params,
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,
          height: 20
        }
      } as Edge;
      
      setEdges((eds) => addEdge(newEdge, eds));
      
      // Update workflow
      const updatedWorkflow: AgentWorkflow = {
        ...workflow,
        edges: [...workflow.edges, newEdge as WorkflowEdge]
      };
      
      onWorkflowChange(updatedWorkflow);
    },
    [setEdges, workflow, onWorkflowChange]
  );
  
  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      const workflowNode = workflow.nodes.find(n => n.id === node.id);
      if (workflowNode) {
        onNodeSelect(workflowNode);
      }
    },
    [workflow, onNodeSelect]
  );
  
  const onPaneClick = useCallback(() => {
    onNodeSelect(null);
  }, [onNodeSelect]);
  
  const onDragOver = useCallback((event: DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);
  
  const onDrop = useCallback(
    (event: DragEvent) => {
      event.preventDefault();
      
      const nodeData = event.dataTransfer.getData('application/reactflow');
      if (!nodeData || !reactFlowInstance || !reactFlowWrapper.current) {
        return;
      }
      
      const { nodeType, label, config } = JSON.parse(nodeData);
      const reactFlowBounds = reactFlowWrapper.current.getBoundingClientRect();
      
      const position = reactFlowInstance.project({
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      });
      
      const newNode: WorkflowNode = {
        id: `${nodeType}_${uuidv4()}`,
        type: nodeType as NodeType,
        position,
        data: {
          label,
          config: config || {}
        }
      };
      
      setNodes((nds) => nds.concat(newNode as Node));
      
      // Update workflow
      const updatedWorkflow: AgentWorkflow = {
        ...workflow,
        nodes: [...workflow.nodes, newNode],
        metadata: {
          ...workflow.metadata,
          modified: new Date()
        }
      };
      
      onWorkflowChange(updatedWorkflow);
    },
    [reactFlowInstance, workflow, onWorkflowChange, setNodes]
  );
  
  const onEdgeUpdateStart = useCallback(() => {
    edgeUpdateSuccessful.current = false;
  }, []);
  
  const onEdgeUpdate = useCallback(
    (oldEdge: Edge, newConnection: Connection) => {
      edgeUpdateSuccessful.current = true;
      setEdges((els) => updateEdge(oldEdge, newConnection, els));
    },
    [setEdges]
  );
  
  const onEdgeUpdateEnd = useCallback(
    (_: any, edge: Edge) => {
      if (!edgeUpdateSuccessful.current) {
        setEdges((eds) => eds.filter((e) => e.id !== edge.id));
      }
      
      edgeUpdateSuccessful.current = true;
    },
    [setEdges]
  );
  
  // Handle node/edge changes
  const handleNodesChange = useCallback(
    (changes: any) => {
      onNodesChange(changes);
      
      // Update workflow with new node positions
      const updatedNodes = nodes.map(node => ({
        id: node.id,
        type: node.type as NodeType,
        position: node.position,
        data: node.data
      }));
      
      const updatedWorkflow: AgentWorkflow = {
        ...workflow,
        nodes: updatedNodes as WorkflowNode[],
        metadata: {
          ...workflow.metadata,
          modified: new Date()
        }
      };
      
      onWorkflowChange(updatedWorkflow);
    },
    [nodes, workflow, onWorkflowChange, onNodesChange]
  );
  
  const handleEdgesChange = useCallback(
    (changes: any) => {
      onEdgesChange(changes);
      
      // Update workflow with edge changes
      const updatedEdges = edges.map(edge => ({
        id: edge.id,
        source: edge.source,
        sourceHandle: edge.sourceHandle,
        target: edge.target,
        targetHandle: edge.targetHandle,
        type: edge.type,
        label: edge.label,
        data: edge.data
      }));
      
      const updatedWorkflow: AgentWorkflow = {
        ...workflow,
        edges: updatedEdges as WorkflowEdge[],
        metadata: {
          ...workflow.metadata,
          modified: new Date()
        }
      };
      
      onWorkflowChange(updatedWorkflow);
    },
    [edges, workflow, onWorkflowChange, onEdgesChange]
  );
  
  return (
    <div className="canvas" ref={reactFlowWrapper}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={onConnect}
        onInit={setReactFlowInstance}
        onDrop={onDrop}
        onDragOver={onDragOver}
        onNodeClick={onNodeClick}
        onPaneClick={onPaneClick}
        onEdgeUpdate={onEdgeUpdate}
        onEdgeUpdateStart={onEdgeUpdateStart}
        onEdgeUpdateEnd={onEdgeUpdateEnd}
        nodeTypes={nodeTypes}
        fitView
        attributionPosition="bottom-left"
      >
        <Background 
          variant={BackgroundVariant.Dots} 
          gap={12} 
          size={1}
          color="var(--vscode-editor-foreground)"
          style={{ opacity: 0.1 }}
        />
        <Controls />
        <MiniMap 
          style={{
            backgroundColor: 'var(--vscode-editor-background)',
            border: '1px solid var(--vscode-panel-border)'
          }}
          nodeColor={node => {
            const status = executionStatus.get(node.id);
            switch (status) {
              case 'running': return 'var(--vscode-debugIcon.startForeground)';
              case 'completed': return 'var(--vscode-debugIcon.continueForeground)';
              case 'failed': return 'var(--vscode-debugIcon.stopForeground)';
              default: return 'var(--vscode-editor-foreground)';
            }
          }}
        />
      </ReactFlow>
    </div>
  );
};

export default Canvas;