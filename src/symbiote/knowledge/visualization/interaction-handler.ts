/**
 * Graph Interaction Handler
 * 
 * Manages user interactions with the graph visualization
 */

import { EventEmitter } from 'events';
import {
  VisualizationNode,
  VisualizationEdge,
  ViewportState,
  SelectionState,
  InteractionConfig
} from './graph-visualization-engine';

export interface InteractionEvent {
  type: 'click' | 'dblclick' | 'hover' | 'drag' | 'wheel' | 'key' | 'contextmenu';
  x: number;
  y: number;
  worldX?: number;
  worldY?: number;
  button?: number;
  key?: string;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  delta?: number;
  target?: VisualizationNode | VisualizationEdge;
}

export interface DragState {
  isDragging: boolean;
  dragType: 'pan' | 'node' | 'selection' | null;
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  draggedNodes: Set<string>;
  originalPositions: Map<string, { x: number; y: number }>;
}

export interface BoxSelection {
  isActive: boolean;
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

export interface KeyBindings {
  [key: string]: () => void;
}

export class GraphInteractionHandler extends EventEmitter {
  private canvas: HTMLCanvasElement;
  private config: InteractionConfig;
  private viewport: ViewportState;
  private selection: SelectionState;
  private dragState: DragState;
  private boxSelection: BoxSelection;
  private nodes: Map<string, VisualizationNode>;
  private edges: Map<string, VisualizationEdge>;
  private keyBindings: KeyBindings;
  private lastClickTime: number = 0;
  private clickDelay: number = 250;
  
  constructor(
    canvas: HTMLCanvasElement,
    config: InteractionConfig,
    viewport: ViewportState,
    selection: SelectionState
  ) {
    super();
    
    this.canvas = canvas;
    this.config = config;
    this.viewport = viewport;
    this.selection = selection;
    
    this.dragState = {
      isDragging: false,
      dragType: null,
      startX: 0,
      startY: 0,
      currentX: 0,
      currentY: 0,
      draggedNodes: new Set(),
      originalPositions: new Map()
    };
    
    this.boxSelection = {
      isActive: false,
      startX: 0,
      startY: 0,
      endX: 0,
      endY: 0
    };
    
    this.nodes = new Map();
    this.edges = new Map();
    
    this.keyBindings = this.createDefaultKeyBindings();
    
    this.attachEventListeners();
  }
  
  /**
   * Update nodes and edges references
   */
  updateData(nodes: VisualizationNode[], edges: VisualizationEdge[]): void {
    this.nodes.clear();
    nodes.forEach(node => this.nodes.set(node.id, node));
    
    this.edges.clear();
    edges.forEach(edge => this.edges.set(edge.id, edge));
  }
  
  /**
   * Update viewport
   */
  updateViewport(viewport: ViewportState): void {
    this.viewport = viewport;
  }
  
  /**
   * Attach event listeners to canvas
   */
  private attachEventListeners(): void {
    // Mouse events
    this.canvas.addEventListener('mousedown', this.handleMouseDown.bind(this));
    this.canvas.addEventListener('mousemove', this.handleMouseMove.bind(this));
    this.canvas.addEventListener('mouseup', this.handleMouseUp.bind(this));
    this.canvas.addEventListener('dblclick', this.handleDoubleClick.bind(this));
    this.canvas.addEventListener('contextmenu', this.handleContextMenu.bind(this));
    
    // Wheel event
    this.canvas.addEventListener('wheel', this.handleWheel.bind(this));
    
    // Touch events for mobile support
    this.canvas.addEventListener('touchstart', this.handleTouchStart.bind(this));
    this.canvas.addEventListener('touchmove', this.handleTouchMove.bind(this));
    this.canvas.addEventListener('touchend', this.handleTouchEnd.bind(this));
    
    // Keyboard events
    if (this.config.enableKeyboardShortcuts) {
      document.addEventListener('keydown', this.handleKeyDown.bind(this));
      document.addEventListener('keyup', this.handleKeyUp.bind(this));
    }
    
    // Prevent default drag behavior
    this.canvas.addEventListener('dragstart', (e) => e.preventDefault());
  }
  
  /**
   * Detach event listeners
   */
  dispose(): void {
    // Remove all event listeners
    this.canvas.removeEventListener('mousedown', this.handleMouseDown.bind(this));
    this.canvas.removeEventListener('mousemove', this.handleMouseMove.bind(this));
    this.canvas.removeEventListener('mouseup', this.handleMouseUp.bind(this));
    this.canvas.removeEventListener('dblclick', this.handleDoubleClick.bind(this));
    this.canvas.removeEventListener('contextmenu', this.handleContextMenu.bind(this));
    this.canvas.removeEventListener('wheel', this.handleWheel.bind(this));
    this.canvas.removeEventListener('touchstart', this.handleTouchStart.bind(this));
    this.canvas.removeEventListener('touchmove', this.handleTouchMove.bind(this));
    this.canvas.removeEventListener('touchend', this.handleTouchEnd.bind(this));
    
    if (this.config.enableKeyboardShortcuts) {
      document.removeEventListener('keydown', this.handleKeyDown.bind(this));
      document.removeEventListener('keyup', this.handleKeyUp.bind(this));
    }
  }
  
  /**
   * Handle mouse down event
   */
  private handleMouseDown(event: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const worldCoords = this.screenToWorld(x, y);
    
    this.dragState.startX = x;
    this.dragState.startY = y;
    this.dragState.currentX = x;
    this.dragState.currentY = y;
    
    // Find target node or edge
    const target = this.findTarget(worldCoords.x, worldCoords.y);
    
    if (target) {
      if (target.type === 'node') {
        const node = target.element as VisualizationNode;
        
        // Handle selection
        if (event.ctrlKey || event.metaKey) {
          // Toggle selection
          if (this.selection.selectedNodes.has(node.id)) {
            this.selection.selectedNodes.delete(node.id);
          } else {
            this.selection.selectedNodes.add(node.id);
          }
        } else if (!this.selection.selectedNodes.has(node.id)) {
          // Clear selection and select only this node
          this.selection.selectedNodes.clear();
          this.selection.selectedEdges.clear();
          this.selection.selectedNodes.add(node.id);
        }
        
        // Start node drag
        if (this.config.enableNodeDrag) {
          this.startNodeDrag();
        }
        
        this.emit('nodeMouseDown', {
          node,
          event: this.createInteractionEvent('click', event, worldCoords, target.element)
        });
      } else {
        // Edge clicked
        const edge = target.element as VisualizationEdge;
        
        if (event.ctrlKey || event.metaKey) {
          if (this.selection.selectedEdges.has(edge.id)) {
            this.selection.selectedEdges.delete(edge.id);
          } else {
            this.selection.selectedEdges.add(edge.id);
          }
        } else {
          this.selection.selectedNodes.clear();
          this.selection.selectedEdges.clear();
          this.selection.selectedEdges.add(edge.id);
        }
        
        this.emit('edgeMouseDown', {
          edge,
          event: this.createInteractionEvent('click', event, worldCoords, target.element)
        });
      }
    } else {
      // Click on empty space
      if (!event.ctrlKey && !event.metaKey && !event.shiftKey) {
        // Clear selection
        this.selection.selectedNodes.clear();
        this.selection.selectedEdges.clear();
      }
      
      if (event.shiftKey && this.config.enableBoxSelection) {
        // Start box selection
        this.startBoxSelection(x, y);
      } else if (this.config.enablePan) {
        // Start pan
        this.dragState.isDragging = true;
        this.dragState.dragType = 'pan';
        this.canvas.style.cursor = 'grabbing';
      }
    }
    
    this.emit('selectionChanged', {
      nodes: Array.from(this.selection.selectedNodes),
      edges: Array.from(this.selection.selectedEdges)
    });
  }
  
  /**
   * Handle mouse move event
   */
  private handleMouseMove(event: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const worldCoords = this.screenToWorld(x, y);
    
    this.dragState.currentX = x;
    this.dragState.currentY = y;
    
    if (this.dragState.isDragging) {
      const dx = x - this.dragState.startX;
      const dy = y - this.dragState.startY;
      
      switch (this.dragState.dragType) {
        case 'pan':
          this.handlePan(dx, dy);
          break;
        case 'node':
          this.handleNodeDrag(dx, dy);
          break;
        case 'selection':
          this.updateBoxSelection(x, y);
          break;
      }
    } else {
      // Handle hover
      const target = this.findTarget(worldCoords.x, worldCoords.y);
      
      if (target) {
        if (target.type === 'node') {
          const node = target.element as VisualizationNode;
          if (this.selection.hoveredNode !== node.id) {
            this.selection.hoveredNode = node.id;
            this.selection.hoveredEdge = undefined;
            this.canvas.style.cursor = 'pointer';
            
            if (this.config.enableTooltips) {
              this.emit('nodeHover', {
                node,
                event: this.createInteractionEvent('hover', event, worldCoords, node)
              });
            }
          }
        } else {
          const edge = target.element as VisualizationEdge;
          if (this.selection.hoveredEdge !== edge.id) {
            this.selection.hoveredEdge = edge.id;
            this.selection.hoveredNode = undefined;
            this.canvas.style.cursor = 'pointer';
            
            if (this.config.enableTooltips) {
              this.emit('edgeHover', {
                edge,
                event: this.createInteractionEvent('hover', event, worldCoords, edge)
              });
            }
          }
        }
      } else {
        if (this.selection.hoveredNode || this.selection.hoveredEdge) {
          this.selection.hoveredNode = undefined;
          this.selection.hoveredEdge = undefined;
          this.canvas.style.cursor = this.config.enablePan ? 'grab' : 'default';
          this.emit('hoverEnd');
        }
      }
    }
  }
  
  /**
   * Handle mouse up event
   */
  private handleMouseUp(event: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const worldCoords = this.screenToWorld(x, y);
    
    if (this.dragState.isDragging) {
      switch (this.dragState.dragType) {
        case 'pan':
          this.canvas.style.cursor = this.config.enablePan ? 'grab' : 'default';
          break;
        case 'node':
          this.endNodeDrag();
          break;
        case 'selection':
          this.endBoxSelection();
          break;
      }
      
      this.dragState.isDragging = false;
      this.dragState.dragType = null;
    } else {
      // Handle click
      const now = Date.now();
      if (now - this.lastClickTime < this.clickDelay) {
        // Double click detected
        return;
      }
      this.lastClickTime = now;
      
      const target = this.findTarget(worldCoords.x, worldCoords.y);
      if (target) {
        this.emit('click', this.createInteractionEvent('click', event, worldCoords, target.element));
      } else {
        this.emit('backgroundClick', this.createInteractionEvent('click', event, worldCoords));
      }
    }
  }
  
  /**
   * Handle double click event
   */
  private handleDoubleClick(event: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const worldCoords = this.screenToWorld(x, y);
    
    const target = this.findTarget(worldCoords.x, worldCoords.y);
    
    if (target) {
      if (target.type === 'node') {
        const node = target.element as VisualizationNode;
        this.emit('nodeDoubleClick', {
          node,
          event: this.createInteractionEvent('dblclick', event, worldCoords, node)
        });
      } else {
        const edge = target.element as VisualizationEdge;
        this.emit('edgeDoubleClick', {
          edge,
          event: this.createInteractionEvent('dblclick', event, worldCoords, edge)
        });
      }
    } else {
      this.emit('backgroundDoubleClick', this.createInteractionEvent('dblclick', event, worldCoords));
    }
  }
  
  /**
   * Handle context menu event
   */
  private handleContextMenu(event: MouseEvent): void {
    event.preventDefault();
    
    if (!this.config.enableContextMenu) return;
    
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const worldCoords = this.screenToWorld(x, y);
    
    const target = this.findTarget(worldCoords.x, worldCoords.y);
    
    this.emit('contextMenu', {
      event: this.createInteractionEvent('contextmenu', event, worldCoords, target?.element),
      target: target?.element,
      selectedNodes: Array.from(this.selection.selectedNodes),
      selectedEdges: Array.from(this.selection.selectedEdges)
    });
  }
  
  /**
   * Handle wheel event for zooming
   */
  private handleWheel(event: WheelEvent): void {
    event.preventDefault();
    
    if (!this.config.enableZoom) return;
    
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    // Calculate zoom
    const delta = event.deltaY > 0 ? 0.9 : 1.1;
    const newZoom = Math.max(0.1, Math.min(5, this.viewport.zoom * delta));
    
    // Zoom towards mouse position
    const worldX = x / this.viewport.zoom - this.viewport.pan.x;
    const worldY = y / this.viewport.zoom - this.viewport.pan.y;
    
    this.viewport.zoom = newZoom;
    
    this.viewport.pan.x = x / newZoom - worldX;
    this.viewport.pan.y = y / newZoom - worldY;
    
    this.emit('viewportChanged', this.viewport);
  }
  
  /**
   * Handle key down event
   */
  private handleKeyDown(event: KeyboardEvent): void {
    // Check if focused on an input element
    if (event.target instanceof HTMLInputElement || 
        event.target instanceof HTMLTextAreaElement) {
      return;
    }
    
    const key = this.getKeyString(event);
    const handler = this.keyBindings[key];
    
    if (handler) {
      event.preventDefault();
      handler();
    }
  }
  
  /**
   * Handle key up event
   */
  private handleKeyUp(event: KeyboardEvent): void {
    // Handle key up events if needed
  }
  
  /**
   * Handle touch start (mobile support)
   */
  private handleTouchStart(event: TouchEvent): void {
    if (event.touches.length === 1) {
      const touch = event.touches[0];
      const mouseEvent = new MouseEvent('mousedown', {
        clientX: touch.clientX,
        clientY: touch.clientY,
        button: 0
      });
      this.handleMouseDown(mouseEvent);
    }
  }
  
  /**
   * Handle touch move
   */
  private handleTouchMove(event: TouchEvent): void {
    if (event.touches.length === 1) {
      const touch = event.touches[0];
      const mouseEvent = new MouseEvent('mousemove', {
        clientX: touch.clientX,
        clientY: touch.clientY
      });
      this.handleMouseMove(mouseEvent);
    }
  }
  
  /**
   * Handle touch end
   */
  private handleTouchEnd(event: TouchEvent): void {
    const mouseEvent = new MouseEvent('mouseup', {
      button: 0
    });
    this.handleMouseUp(mouseEvent);
  }
  
  /**
   * Start node drag
   */
  private startNodeDrag(): void {
    this.dragState.isDragging = true;
    this.dragState.dragType = 'node';
    this.dragState.draggedNodes = new Set(this.selection.selectedNodes);
    
    // Store original positions
    this.dragState.originalPositions.clear();
    this.dragState.draggedNodes.forEach(nodeId => {
      const node = this.nodes.get(nodeId);
      if (node && node.x !== undefined && node.y !== undefined) {
        this.dragState.originalPositions.set(nodeId, { x: node.x, y: node.y });
      }
    });
  }
  
  /**
   * Handle node drag
   */
  private handleNodeDrag(dx: number, dy: number): void {
    const worldDx = dx / this.viewport.zoom;
    const worldDy = dy / this.viewport.zoom;
    
    this.dragState.draggedNodes.forEach(nodeId => {
      const node = this.nodes.get(nodeId);
      const originalPos = this.dragState.originalPositions.get(nodeId);
      
      if (node && originalPos) {
        node.x = originalPos.x + worldDx;
        node.y = originalPos.y + worldDy;
      }
    });
    
    this.emit('nodesDragged', {
      nodes: Array.from(this.dragState.draggedNodes),
      dx: worldDx,
      dy: worldDy
    });
  }
  
  /**
   * End node drag
   */
  private endNodeDrag(): void {
    this.emit('nodesDragEnd', {
      nodes: Array.from(this.dragState.draggedNodes),
      positions: Array.from(this.dragState.draggedNodes).map(nodeId => {
        const node = this.nodes.get(nodeId);
        return {
          id: nodeId,
          x: node?.x || 0,
          y: node?.y || 0
        };
      })
    });
    
    this.dragState.draggedNodes.clear();
    this.dragState.originalPositions.clear();
  }
  
  /**
   * Handle pan
   */
  private handlePan(dx: number, dy: number): void {
    this.viewport.pan.x = this.viewport.pan.x + dx / this.viewport.zoom;
    this.viewport.pan.y = this.viewport.pan.y + dy / this.viewport.zoom;
    
    this.dragState.startX = this.dragState.currentX;
    this.dragState.startY = this.dragState.currentY;
    
    this.emit('viewportChanged', this.viewport);
  }
  
  /**
   * Start box selection
   */
  private startBoxSelection(x: number, y: number): void {
    this.boxSelection.isActive = true;
    this.boxSelection.startX = x;
    this.boxSelection.startY = y;
    this.boxSelection.endX = x;
    this.boxSelection.endY = y;
    
    this.dragState.isDragging = true;
    this.dragState.dragType = 'selection';
  }
  
  /**
   * Update box selection
   */
  private updateBoxSelection(x: number, y: number): void {
    this.boxSelection.endX = x;
    this.boxSelection.endY = y;
    
    // Find nodes in selection box
    const minX = Math.min(this.boxSelection.startX, this.boxSelection.endX);
    const maxX = Math.max(this.boxSelection.startX, this.boxSelection.endX);
    const minY = Math.min(this.boxSelection.startY, this.boxSelection.endY);
    const maxY = Math.max(this.boxSelection.startY, this.boxSelection.endY);
    
    const worldMin = this.screenToWorld(minX, minY);
    const worldMax = this.screenToWorld(maxX, maxY);
    
    const selectedNodes = new Set<string>();
    
    this.nodes.forEach(node => {
      if (node.visible && node.x !== undefined && node.y !== undefined) {
        if (node.x >= worldMin.x && node.x <= worldMax.x &&
            node.y >= worldMin.y && node.y <= worldMax.y) {
          selectedNodes.add(node.id);
        }
      }
    });
    
    this.emit('boxSelection', {
      box: { minX, minY, maxX, maxY },
      nodes: Array.from(selectedNodes)
    });
  }
  
  /**
   * End box selection
   */
  private endBoxSelection(): void {
    const minX = Math.min(this.boxSelection.startX, this.boxSelection.endX);
    const maxX = Math.max(this.boxSelection.startX, this.boxSelection.endX);
    const minY = Math.min(this.boxSelection.startY, this.boxSelection.endY);
    const maxY = Math.max(this.boxSelection.startY, this.boxSelection.endY);
    
    const worldMin = this.screenToWorld(minX, minY);
    const worldMax = this.screenToWorld(maxX, maxY);
    
    // Select nodes in box
    this.nodes.forEach(node => {
      if (node.visible && node.x !== undefined && node.y !== undefined) {
        if (node.x >= worldMin.x && node.x <= worldMax.x &&
            node.y >= worldMin.y && node.y <= worldMax.y) {
          this.selection.selectedNodes.add(node.id);
        }
      }
    });
    
    this.boxSelection.isActive = false;
    
    this.emit('selectionChanged', {
      nodes: Array.from(this.selection.selectedNodes),
      edges: Array.from(this.selection.selectedEdges)
    });
  }
  
  /**
   * Find target at coordinates
   */
  private findTarget(
    x: number, 
    y: number
  ): { type: 'node' | 'edge'; element: VisualizationNode | VisualizationEdge } | null {
    // Check nodes first (on top)
    for (const node of this.nodes.values()) {
      if (!node.visible || node.x === undefined || node.y === undefined) continue;
      
      const size = node.size || 20;
      const dx = x - node.x;
      const dy = y - node.y;
      const distance = Math.sqrt(dx * dx + dy * dy);
      
      if (distance <= size) {
        return { type: 'node', element: node };
      }
    }
    
    // Check edges
    // TODO: Implement edge hit testing
    
    return null;
  }
  
  /**
   * Convert screen coordinates to world coordinates
   */
  private screenToWorld(x: number, y: number): { x: number; y: number } {
    return {
      x: x / this.viewport.zoom - this.viewport.pan.x,
      y: y / this.viewport.zoom - this.viewport.pan.y
    };
  }
  
  /**
   * Create interaction event
   */
  private createInteractionEvent(
    type: InteractionEvent['type'],
    event: MouseEvent,
    worldCoords: { x: number; y: number },
    target?: VisualizationNode | VisualizationEdge
  ): InteractionEvent {
    return {
      type,
      x: event.clientX,
      y: event.clientY,
      worldX: worldCoords.x,
      worldY: worldCoords.y,
      button: event.button,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
      target
    };
  }
  
  /**
   * Get key string from keyboard event
   */
  private getKeyString(event: KeyboardEvent): string {
    const parts: string[] = [];
    
    if (event.ctrlKey) parts.push('Ctrl');
    if (event.shiftKey) parts.push('Shift');
    if (event.altKey) parts.push('Alt');
    if (event.metaKey) parts.push('Meta');
    
    const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
    parts.push(key);
    
    return parts.join('+');
  }
  
  /**
   * Create default key bindings
   */
  private createDefaultKeyBindings(): KeyBindings {
    return {
      // Selection
      'Ctrl+A': () => this.selectAll(),
      'Escape': () => this.clearSelection(),
      'Delete': () => this.deleteSelected(),
      
      // View
      'Ctrl+0': () => this.resetView(),
      'Ctrl+=': () => this.zoomIn(),
      'Ctrl+-': () => this.zoomOut(),
      
      // Layout
      'L': () => this.emit('layoutRequested'),
      
      // Navigation
      'F': () => this.fitToScreen(),
      'C': () => this.centerView(),
      
      // Search
      'Ctrl+F': () => this.emit('searchRequested'),
      
      // Undo/Redo
      'Ctrl+Z': () => this.emit('undoRequested'),
      'Ctrl+Y': () => this.emit('redoRequested'),
      'Ctrl+Shift+Z': () => this.emit('redoRequested')
    };
  }
  
  /**
   * Select all visible nodes
   */
  private selectAll(): void {
    this.selection.selectedNodes.clear();
    this.nodes.forEach(node => {
      if (node.visible) {
        this.selection.selectedNodes.add(node.id);
      }
    });
    
    this.emit('selectionChanged', {
      nodes: Array.from(this.selection.selectedNodes),
      edges: Array.from(this.selection.selectedEdges)
    });
  }
  
  /**
   * Clear selection
   */
  private clearSelection(): void {
    this.selection.selectedNodes.clear();
    this.selection.selectedEdges.clear();
    
    this.emit('selectionChanged', {
      nodes: [],
      edges: []
    });
  }
  
  /**
   * Delete selected nodes and edges
   */
  private deleteSelected(): void {
    this.emit('deleteRequested', {
      nodes: Array.from(this.selection.selectedNodes),
      edges: Array.from(this.selection.selectedEdges)
    });
  }
  
  /**
   * Reset view to default
   */
  private resetView(): void {
    this.viewport.zoom = 1;
    this.viewport.pan = { x: 0, y: 0 };
    this.emit('viewportChanged', this.viewport);
  }
  
  /**
   * Zoom in
   */
  private zoomIn(): void {
    this.viewport.zoom = Math.min(5, this.viewport.zoom * 1.2);
    this.emit('viewportChanged', this.viewport);
  }
  
  /**
   * Zoom out
   */
  private zoomOut(): void {
    this.viewport.zoom = Math.max(0.1, this.viewport.zoom / 1.2);
    this.emit('viewportChanged', this.viewport);
  }
  
  /**
   * Fit all visible nodes to screen
   */
  private fitToScreen(): void {
    const visibleNodes = Array.from(this.nodes.values()).filter(n => n.visible);
    if (visibleNodes.length === 0) return;
    
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;
    
    visibleNodes.forEach(node => {
      if (node.x !== undefined && node.y !== undefined) {
        minX = Math.min(minX, node.x);
        maxX = Math.max(maxX, node.x);
        minY = Math.min(minY, node.y);
        maxY = Math.max(maxY, node.y);
      }
    });
    
    const width = maxX - minX;
    const height = maxY - minY;
    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;
    
    const padding = 50;
    const canvasWidth = this.canvas.width - padding * 2;
    const canvasHeight = this.canvas.height - padding * 2;
    
    const scaleX = canvasWidth / width;
    const scaleY = canvasHeight / height;
    const scale = Math.min(scaleX, scaleY, 2);
    
    this.viewport.zoom = scale;
    this.viewport.pan.x = this.canvas.width / 2 / scale - centerX;
    this.viewport.pan.y = this.canvas.height / 2 / scale - centerY;
    
    this.emit('viewportChanged', this.viewport);
  }
  
  /**
   * Center view on visible nodes
   */
  private centerView(): void {
    const visibleNodes = Array.from(this.nodes.values()).filter(n => n.visible);
    if (visibleNodes.length === 0) return;
    
    let sumX = 0, sumY = 0;
    let count = 0;
    
    visibleNodes.forEach(node => {
      if (node.x !== undefined && node.y !== undefined) {
        sumX += node.x;
        sumY += node.y;
        count++;
      }
    });
    
    if (count > 0) {
      const centerX = sumX / count;
      const centerY = sumY / count;
      
      this.viewport.pan.x = this.canvas.width / 2 / this.viewport.zoom - centerX;
      this.viewport.pan.y = this.canvas.height / 2 / this.viewport.zoom - centerY;
      
      this.emit('viewportChanged', this.viewport);
    }
  }
}