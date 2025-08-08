import React, { useRef, useEffect, useState, useCallback, forwardRef } from 'react';
import { WorkflowState, WorkflowNode, NodeConnection } from '../VisualBuilderPanel';
import { Node } from './Node';
import { Connection } from './Connection';

interface WorkflowCanvasProps {
  workflow: WorkflowState;
  canvasMode: 'design' | 'execute' | 'debug';
  showGrid: boolean;
  snapToGrid: boolean;
  onNodeSelect: (nodeId: string | null) => void;
  onNodeUpdate: (nodeId: string, updates: Partial<WorkflowNode>) => void;
  onNodeDelete: (nodeId: string) => void;
  onConnectionAdd: (connection: Omit<NodeConnection, 'id'>) => void;
  onConnectionDelete: (connectionId: string) => void;
  onConnectionSelect: (connectionId: string | null) => void;
}

interface CanvasTransform {
  x: number;
  y: number;
  scale: number;
}

interface DragState {
  isDragging: boolean;
  dragType: 'canvas' | 'node' | 'connection';
  startPosition: { x: number; y: number };
  currentPosition: { x: number; y: number };
  draggedNodeId?: string;
  connectionStart?: { nodeId: string; portId: string; type: 'input' | 'output' };
}

export const WorkflowCanvas = forwardRef<HTMLDivElement, WorkflowCanvasProps>(({
  workflow,
  canvasMode,
  showGrid,
  snapToGrid,
  onNodeSelect,
  onNodeUpdate,
  onNodeDelete,
  onConnectionAdd,
  onConnectionDelete,
  onConnectionSelect
}, ref) => {
  const canvasRef = useRef<HTMLDivElement>(null);
  const [transform, setTransform] = useState<CanvasTransform>({ x: 0, y: 0, scale: 1 });
  const [dragState, setDragState] = useState<DragState>({
    isDragging: false,
    dragType: 'canvas',
    startPosition: { x: 0, y: 0 },
    currentPosition: { x: 0, y: 0 }
  });

  // Canvas interaction handlers
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return; // Only left mouse button

    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const startPosition = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    };

    setDragState({
      isDragging: true,
      dragType: 'canvas',
      startPosition,
      currentPosition: startPosition
    });

    // Clear selection when clicking on empty canvas
    onNodeSelect(null);
    onConnectionSelect(null);
  }, [onNodeSelect, onConnectionSelect]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragState.isDragging) return;

    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const currentPosition = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    };

    setDragState(prev => ({ ...prev, currentPosition }));

    if (dragState.dragType === 'canvas') {
      // Pan the canvas
      const deltaX = currentPosition.x - dragState.startPosition.x;
      const deltaY = currentPosition.y - dragState.startPosition.y;
      
      setTransform(prev => ({
        ...prev,
        x: prev.x + deltaX / prev.scale,
        y: prev.y + deltaY / prev.scale
      }));

      setDragState(prev => ({ ...prev, startPosition: currentPosition }));
    } else if (dragState.dragType === 'node' && dragState.draggedNodeId) {
      // Move the node
      const deltaX = (currentPosition.x - dragState.startPosition.x) / transform.scale;
      const deltaY = (currentPosition.y - dragState.startPosition.y) / transform.scale;
      
      const node = workflow.nodes.find(n => n.id === dragState.draggedNodeId);
      if (node) {
        let newX = node.position.x + deltaX;
        let newY = node.position.y + deltaY;

        // Snap to grid if enabled
        if (snapToGrid) {
          const gridSize = 20;
          newX = Math.round(newX / gridSize) * gridSize;
          newY = Math.round(newY / gridSize) * gridSize;
        }

        onNodeUpdate(dragState.draggedNodeId, {
          position: { x: newX, y: newY }
        });
      }

      setDragState(prev => ({ ...prev, startPosition: currentPosition }));
    }
  }, [dragState, transform.scale, workflow.nodes, snapToGrid, onNodeUpdate]);

  const handleMouseUp = useCallback(() => {
    if (dragState.dragType === 'connection' && dragState.connectionStart) {
      // Handle connection creation
      // This would be completed when the user drops on a valid port
    }

    setDragState({
      isDragging: false,
      dragType: 'canvas',
      startPosition: { x: 0, y: 0 },
      currentPosition: { x: 0, y: 0 }
    });
  }, [dragState]);

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    // Calculate zoom
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    const newScale = Math.max(0.1, Math.min(3, transform.scale * zoomFactor));

    // Calculate new position to zoom towards mouse
    const scaleRatio = newScale / transform.scale;
    const newX = mouseX - (mouseX - transform.x) * scaleRatio;
    const newY = mouseY - (mouseY - transform.y) * scaleRatio;

    setTransform({
      x: newX,
      y: newY,
      scale: newScale
    });
  }, [transform]);

  // Node interaction handlers
  const handleNodeMouseDown = useCallback((e: React.MouseEvent, nodeId: string) => {
    e.stopPropagation();
    
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const startPosition = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    };

    setDragState({
      isDragging: true,
      dragType: 'node',
      startPosition,
      currentPosition: startPosition,
      draggedNodeId: nodeId
    });

    onNodeSelect(nodeId);
  }, [onNodeSelect]);

  const handleNodeDoubleClick = useCallback((nodeId: string) => {
    // Center the node in view
    const node = workflow.nodes.find(n => n.id === nodeId);
    if (!node || !canvasRef.current) return;

    const rect = canvasRef.current.getBoundingClientRect();
    const centerX = rect.width / 2;
    const centerY = rect.height / 2;

    setTransform({
      x: centerX - node.position.x * transform.scale,
      y: centerY - node.position.y * transform.scale,
      scale: transform.scale
    });
  }, [workflow.nodes, transform.scale]);

  const handleNodeDelete = useCallback((nodeId: string) => {
    onNodeDelete(nodeId);
  }, [onNodeDelete]);

  // Port interaction handlers
  const handlePortMouseDown = useCallback((e: React.MouseEvent, nodeId: string, portId: string, portType: 'input' | 'output') => {
    e.stopPropagation();
    
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const startPosition = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    };

    setDragState({
      isDragging: true,
      dragType: 'connection',
      startPosition,
      currentPosition: startPosition,
      connectionStart: { nodeId, portId, type: portType }
    });
  }, []);

  const handlePortMouseUp = useCallback((e: React.MouseEvent, nodeId: string, portId: string, portType: 'input' | 'output') => {
    e.stopPropagation();
    
    if (dragState.dragType === 'connection' && dragState.connectionStart) {
      const { nodeId: sourceNodeId, portId: sourcePortId, type: sourceType } = dragState.connectionStart;
      
      // Validate connection
      if (sourceNodeId !== nodeId && sourceType !== portType) {
        if (sourceType === 'output' && portType === 'input') {
          onConnectionAdd({
            sourceNodeId,
            sourcePortId,
            targetNodeId: nodeId,
            targetPortId: portId,
            dataType: 'any' // This would be determined by port types
          });
        } else if (sourceType === 'input' && portType === 'output') {
          onConnectionAdd({
            sourceNodeId: nodeId,
            sourcePortId: portId,
            targetNodeId: sourceNodeId,
            targetPortId: sourcePortId,
            dataType: 'any'
          });
        }
      }
    }

    setDragState({
      isDragging: false,
      dragType: 'canvas',
      startPosition: { x: 0, y: 0 },
      currentPosition: { x: 0, y: 0 }
    });
  }, [dragState, onConnectionAdd]);

  // Connection interaction handlers
  const handleConnectionClick = useCallback((connectionId: string) => {
    onConnectionSelect(connectionId);
  }, [onConnectionSelect]);

  const handleConnectionDelete = useCallback((connectionId: string) => {
    onConnectionDelete(connectionId);
  }, [onConnectionDelete]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Delete' || e.key === 'Backspace') {
        if (workflow.selectedNodeId) {
          onNodeDelete(workflow.selectedNodeId);
        } else if (workflow.selectedConnectionId) {
          onConnectionDelete(workflow.selectedConnectionId);
        }
      } else if (e.key === 'Escape') {
        onNodeSelect(null);
        onConnectionSelect(null);
      } else if (e.ctrlKey || e.metaKey) {
        if (e.key === '0') {
          // Reset zoom
          e.preventDefault();
          setTransform({ x: 0, y: 0, scale: 1 });
        } else if (e.key === '=') {
          // Zoom in
          e.preventDefault();
          setTransform(prev => ({ ...prev, scale: Math.min(3, prev.scale * 1.2) }));
        } else if (e.key === '-') {
          // Zoom out
          e.preventDefault();
          setTransform(prev => ({ ...prev, scale: Math.max(0.1, prev.scale / 1.2) }));
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [workflow.selectedNodeId, workflow.selectedConnectionId, onNodeDelete, onConnectionDelete, onNodeSelect, onConnectionSelect]);

  // Calculate connection paths
  const getConnectionPath = useCallback((connection: NodeConnection) => {
    const sourceNode = workflow.nodes.find(n => n.id === connection.sourceNodeId);
    const targetNode = workflow.nodes.find(n => n.id === connection.targetNodeId);
    
    if (!sourceNode || !targetNode) return '';

    const sourceX = sourceNode.position.x + 160; // Node width
    const sourceY = sourceNode.position.y + 40; // Approximate port position
    const targetX = targetNode.position.x;
    const targetY = targetNode.position.y + 40;

    // Create curved path
    const controlPointOffset = Math.abs(targetX - sourceX) * 0.5;
    const controlX1 = sourceX + controlPointOffset;
    const controlX2 = targetX - controlPointOffset;

    return `M ${sourceX} ${sourceY} C ${controlX1} ${sourceY} ${controlX2} ${targetY} ${targetX} ${targetY}`;
  }, [workflow.nodes]);

  return (
    <div
      ref={canvasRef}
      className={`workflow-canvas ${dragState.isDragging ? 'grabbing' : ''}`}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onWheel={handleWheel}
    >
      {/* Grid */}
      {showGrid && (
        <div 
          className="canvas-grid"
          style={{
            transform: `translate(${transform.x % 20}px, ${transform.y % 20}px) scale(${transform.scale})`,
            backgroundSize: `${20 * transform.scale}px ${20 * transform.scale}px`
          }}
        />
      )}

      {/* Canvas Content */}
      <div
        className="canvas-content"
        style={{
          transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.scale})`
        }}
      >
        {/* Connections */}
        <svg
          className="connections-layer"
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: '100%',
            pointerEvents: 'none',
            zIndex: 1
          }}
        >
          {workflow.connections.map(connection => (
            <Connection
              key={connection.id}
              connection={connection}
              path={getConnectionPath(connection)}
              isSelected={workflow.selectedConnectionId === connection.id}
              onClick={() => handleConnectionClick(connection.id)}
              onDelete={() => handleConnectionDelete(connection.id)}
            />
          ))}
          
          {/* Temporary connection while dragging */}
          {dragState.dragType === 'connection' && dragState.connectionStart && (
            <path
              d={`M ${dragState.startPosition.x / transform.scale - transform.x / transform.scale} ${dragState.startPosition.y / transform.scale - transform.y / transform.scale} L ${dragState.currentPosition.x / transform.scale - transform.x / transform.scale} ${dragState.currentPosition.y / transform.scale - transform.y / transform.scale}`}
              className="connection-path temporary"
            />
          )}
        </svg>

        {/* Nodes */}
        {workflow.nodes.map(node => (
          <Node
            key={node.id}
            node={node}
            isSelected={workflow.selectedNodeId === node.id}
            isExecuting={workflow.isExecuting && canvasMode === 'execute'}
            canvasMode={canvasMode}
            onMouseDown={(e) => handleNodeMouseDown(e, node.id)}
            onDoubleClick={() => handleNodeDoubleClick(node.id)}
            onDelete={() => handleNodeDelete(node.id)}
            onPortMouseDown={handlePortMouseDown}
            onPortMouseUp={handlePortMouseUp}
          />
        ))}
      </div>

      {/* Canvas Info Overlay */}
      <div className="canvas-info">
        <div className="zoom-info">
          {Math.round(transform.scale * 100)}%
        </div>
        <div className="position-info">
          {Math.round(transform.x)}, {Math.round(transform.y)}
        </div>
      </div>
    </div>
  );
});

WorkflowCanvas.displayName = 'WorkflowCanvas';
