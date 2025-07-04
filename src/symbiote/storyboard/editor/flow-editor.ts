/**
 * Flow Editor
 * 
 * Visual drag-and-drop flow editor for storyboards
 */

import * as vscode from 'vscode';
import { EventEmitter } from 'events';
import { StoryboardModel } from '../storyboard-model';
import {
  Scene,
  Flow,
  Position,
  SceneType,
  TransitionType,
  SceneStyle,
  FlowStyle
} from '../types';

export interface FlowEditorConfig {
  gridSize?: number;
  snapToGrid?: boolean;
  showGrid?: boolean;
  enableMultiSelect?: boolean;
  enableKeyboardShortcuts?: boolean;
}

export interface EditorState {
  selectedScenes: Set<string>;
  selectedFlows: Set<string>;
  isDragging: boolean;
  isConnecting: boolean;
  connectionStart?: string;
  connectionPreview?: Position;
  panOffset: Position;
  zoomLevel: number;
}

export interface DragEvent {
  sceneId: string;
  startPosition: Position;
  currentPosition: Position;
  delta: Position;
}

export interface ConnectEvent {
  sourceId: string;
  targetId: string;
  type: TransitionType;
}

export class FlowEditor extends EventEmitter {
  private config: Required<FlowEditorConfig>;
  private state: EditorState;
  private storyboard?: StoryboardModel;
  private webview?: vscode.WebviewPanel;
  private disposables: vscode.Disposable[] = [];
  
  constructor(config: FlowEditorConfig = {}) {
    super();
    
    this.config = {
      gridSize: config.gridSize || 20,
      snapToGrid: config.snapToGrid ?? true,
      showGrid: config.showGrid ?? true,
      enableMultiSelect: config.enableMultiSelect ?? true,
      enableKeyboardShortcuts: config.enableKeyboardShortcuts ?? true
    };
    
    this.state = {
      selectedScenes: new Set(),
      selectedFlows: new Set(),
      isDragging: false,
      isConnecting: false,
      panOffset: { x: 0, y: 0 },
      zoomLevel: 1
    };
  }
  
  /**
   * Initialize editor with a storyboard
   */
  initialize(storyboard: StoryboardModel): void {
    this.storyboard = storyboard;
    
    // Listen to storyboard events
    this.setupStoryboardListeners();
    
    this.emit('initialized');
  }
  
  /**
   * Create and show webview editor
   */
  async show(): Promise<void> {
    if (!this.storyboard) {
      throw new Error('No storyboard loaded');
    }
    
    // Create webview panel
    this.webview = vscode.window.createWebviewPanel(
      'storyboardFlowEditor',
      `Storyboard: ${this.storyboard.getName()}`,
      vscode.ViewColumn.One,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: []
      }
    );
    
    // Set webview content
    this.webview.webview.html = this.getWebviewContent();
    
    // Handle messages from webview
    this.webview.webview.onDidReceiveMessage(
      message => this.handleWebviewMessage(message),
      undefined,
      this.disposables
    );
    
    // Handle panel disposal
    this.webview.onDidDispose(() => {
      this.webview = undefined;
      this.emit('closed');
    });
    
    // Send initial state
    this.updateWebview();
  }
  
  /**
   * Select scene(s)
   */
  selectScene(sceneId: string, multiSelect: boolean = false): void {
    if (!multiSelect || !this.config.enableMultiSelect) {
      this.state.selectedScenes.clear();
      this.state.selectedFlows.clear();
    }
    
    this.state.selectedScenes.add(sceneId);
    
    this.emit('selection-changed', {
      scenes: Array.from(this.state.selectedScenes),
      flows: Array.from(this.state.selectedFlows)
    });
    
    this.updateWebview();
  }
  
  /**
   * Select flow(s)
   */
  selectFlow(flowId: string, multiSelect: boolean = false): void {
    if (!multiSelect || !this.config.enableMultiSelect) {
      this.state.selectedScenes.clear();
      this.state.selectedFlows.clear();
    }
    
    this.state.selectedFlows.add(flowId);
    
    this.emit('selection-changed', {
      scenes: Array.from(this.state.selectedScenes),
      flows: Array.from(this.state.selectedFlows)
    });
    
    this.updateWebview();
  }
  
  /**
   * Clear selection
   */
  clearSelection(): void {
    this.state.selectedScenes.clear();
    this.state.selectedFlows.clear();
    
    this.emit('selection-changed', {
      scenes: [],
      flows: []
    });
    
    this.updateWebview();
  }
  
  /**
   * Start dragging scene(s)
   */
  startDrag(sceneId: string, position: Position): void {
    if (!this.state.selectedScenes.has(sceneId)) {
      this.selectScene(sceneId);
    }
    
    this.state.isDragging = true;
    
    this.emit('drag-start', {
      scenes: Array.from(this.state.selectedScenes),
      position
    });
  }
  
  /**
   * Update drag position
   */
  updateDrag(position: Position): void {
    if (!this.state.isDragging || !this.storyboard) return;
    
    // Calculate delta
    const delta = position;
    
    // Update scene positions
    this.state.selectedScenes.forEach(sceneId => {
      const scene = this.storyboard!.getScene(sceneId);
      if (scene) {
        let newPosition = {
          x: scene.position.x + delta.x,
          y: scene.position.y + delta.y
        };
        
        // Snap to grid
        if (this.config.snapToGrid) {
          newPosition = this.snapToGrid(newPosition);
        }
        
        this.storyboard!.moveScene(sceneId, newPosition);
      }
    });
    
    this.emit('drag-update', { position });
    this.updateWebview();
  }
  
  /**
   * End drag operation
   */
  endDrag(): void {
    this.state.isDragging = false;
    
    this.emit('drag-end', {
      scenes: Array.from(this.state.selectedScenes)
    });
  }
  
  /**
   * Start connection
   */
  startConnection(sceneId: string): void {
    this.state.isConnecting = true;
    this.state.connectionStart = sceneId;
    
    this.emit('connection-start', { sceneId });
  }
  
  /**
   * Update connection preview
   */
  updateConnection(position: Position): void {
    if (!this.state.isConnecting) return;
    
    this.state.connectionPreview = position;
    this.updateWebview();
  }
  
  /**
   * Complete connection
   */
  completeConnection(targetId: string): void {
    if (!this.state.isConnecting || !this.state.connectionStart || !this.storyboard) {
      return;
    }
    
    if (targetId !== this.state.connectionStart) {
      // Show transition type picker
      this.showTransitionTypePicker().then(type => {
        if (type) {
          this.storyboard!.addFlow(
            type,
            this.state.connectionStart!,
            targetId
          );
          
          this.emit('connection-created', {
            sourceId: this.state.connectionStart!,
            targetId,
            type
          });
        }
      });
    }
    
    this.cancelConnection();
  }
  
  /**
   * Cancel connection
   */
  cancelConnection(): void {
    this.state.isConnecting = false;
    this.state.connectionStart = undefined;
    this.state.connectionPreview = undefined;
    
    this.emit('connection-cancelled');
    this.updateWebview();
  }
  
  /**
   * Delete selected items
   */
  deleteSelected(): void {
    if (!this.storyboard) return;
    
    // Delete selected flows first
    this.state.selectedFlows.forEach(flowId => {
      this.storyboard!.deleteFlow(flowId);
    });
    
    // Delete selected scenes
    this.state.selectedScenes.forEach(sceneId => {
      this.storyboard!.deleteScene(sceneId);
    });
    
    this.clearSelection();
    
    this.emit('items-deleted');
  }
  
  /**
   * Pan view
   */
  pan(delta: Position): void {
    this.state.panOffset = {
      x: this.state.panOffset.x + delta.x,
      y: this.state.panOffset.y + delta.y
    };
    
    this.updateWebview();
  }
  
  /**
   * Zoom view
   */
  zoom(level: number): void {
    this.state.zoomLevel = Math.max(0.1, Math.min(3, level));
    
    this.emit('zoom-changed', this.state.zoomLevel);
    this.updateWebview();
  }
  
  /**
   * Reset view
   */
  resetView(): void {
    this.state.panOffset = { x: 0, y: 0 };
    this.state.zoomLevel = 1;
    
    this.updateWebview();
  }
  
  /**
   * Fit to content
   */
  fitToContent(): void {
    if (!this.storyboard) return;
    
    const scenes = this.storyboard.getScenes();
    if (scenes.length === 0) return;
    
    // Calculate bounds
    let minX = Infinity, minY = Infinity;
    let maxX = -Infinity, maxY = -Infinity;
    
    scenes.forEach(scene => {
      minX = Math.min(minX, scene.position.x);
      minY = Math.min(minY, scene.position.y);
      maxX = Math.max(maxX, scene.position.x + 200); // Assume 200px width
      maxY = Math.max(maxY, scene.position.y + 100); // Assume 100px height
    });
    
    // Calculate zoom and pan to fit
    const padding = 50;
    const contentWidth = maxX - minX + 2 * padding;
    const contentHeight = maxY - minY + 2 * padding;
    
    // This would need viewport dimensions from webview
    // For now, just center
    this.state.panOffset = {
      x: -(minX + maxX) / 2,
      y: -(minY + maxY) / 2
    };
    
    this.updateWebview();
  }
  
  /**
   * Export as image
   */
  async exportAsImage(format: 'png' | 'svg' = 'png'): Promise<Buffer> {
    if (!this.webview) {
      throw new Error('Editor not open');
    }
    
    // Request image data from webview
    return new Promise((resolve, reject) => {
      const listener = this.webview!.webview.onDidReceiveMessage(message => {
        if (message.type === 'export-result') {
          listener.dispose();
          
          if (message.success) {
            resolve(Buffer.from(message.data, 'base64'));
          } else {
            reject(new Error(message.error));
          }
        }
      });
      
      this.webview!.webview.postMessage({
        type: 'export',
        format
      });
    });
  }
  
  /**
   * Dispose editor
   */
  dispose(): void {
    if (this.webview) {
      this.webview.dispose();
    }
    
    this.disposables.forEach(d => d.dispose());
    this.disposables = [];
    
    this.removeAllListeners();
  }
  
  // Private methods
  
  private setupStoryboardListeners(): void {
    if (!this.storyboard) return;
    
    this.storyboard.on('scene-added', () => this.updateWebview());
    this.storyboard.on('scene-updated', () => this.updateWebview());
    this.storyboard.on('scene-deleted', () => this.updateWebview());
    this.storyboard.on('flow-added', () => this.updateWebview());
    this.storyboard.on('flow-updated', () => this.updateWebview());
    this.storyboard.on('flow-deleted', () => this.updateWebview());
  }
  
  private handleWebviewMessage(message: any): void {
    switch (message.type) {
      case 'select-scene':
        this.selectScene(message.sceneId, message.multiSelect);
        break;
        
      case 'select-flow':
        this.selectFlow(message.flowId, message.multiSelect);
        break;
        
      case 'clear-selection':
        this.clearSelection();
        break;
        
      case 'start-drag':
        this.startDrag(message.sceneId, message.position);
        break;
        
      case 'update-drag':
        this.updateDrag(message.position);
        break;
        
      case 'end-drag':
        this.endDrag();
        break;
        
      case 'start-connection':
        this.startConnection(message.sceneId);
        break;
        
      case 'update-connection':
        this.updateConnection(message.position);
        break;
        
      case 'complete-connection':
        this.completeConnection(message.targetId);
        break;
        
      case 'cancel-connection':
        this.cancelConnection();
        break;
        
      case 'delete-selected':
        this.deleteSelected();
        break;
        
      case 'pan':
        this.pan(message.delta);
        break;
        
      case 'zoom':
        this.zoom(message.level);
        break;
        
      case 'reset-view':
        this.resetView();
        break;
        
      case 'fit-to-content':
        this.fitToContent();
        break;
        
      case 'add-scene':
        this.showAddSceneDialog(message.position);
        break;
        
      case 'edit-scene':
        this.showEditSceneDialog(message.sceneId);
        break;
        
      case 'edit-flow':
        this.showEditFlowDialog(message.flowId);
        break;
    }
  }
  
  private updateWebview(): void {
    if (!this.webview || !this.storyboard) return;
    
    this.webview.webview.postMessage({
      type: 'update',
      data: {
        storyboard: this.storyboard.getStoryboard(),
        state: {
          selectedScenes: Array.from(this.state.selectedScenes),
          selectedFlows: Array.from(this.state.selectedFlows),
          isDragging: this.state.isDragging,
          isConnecting: this.state.isConnecting,
          connectionStart: this.state.connectionStart,
          connectionPreview: this.state.connectionPreview,
          panOffset: this.state.panOffset,
          zoomLevel: this.state.zoomLevel
        },
        config: this.config
      }
    });
  }
  
  private snapToGrid(position: Position): Position {
    const { gridSize } = this.config;
    
    return {
      x: Math.round(position.x / gridSize) * gridSize,
      y: Math.round(position.y / gridSize) * gridSize
    };
  }
  
  private async showTransitionTypePicker(): Promise<TransitionType | undefined> {
    const items = [
      { label: 'Navigation', value: TransitionType.Navigation },
      { label: 'Action', value: TransitionType.Action },
      { label: 'State Change', value: TransitionType.StateChange },
      { label: 'API Call', value: TransitionType.APICall },
      { label: 'Data Flow', value: TransitionType.DataFlow },
      { label: 'Conditional', value: TransitionType.Conditional }
    ];
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: 'Select transition type'
    });
    
    return selected?.value;
  }
  
  private async showAddSceneDialog(position: Position): Promise<void> {
    if (!this.storyboard) return;
    
    const type = await this.showSceneTypePicker();
    if (!type) return;
    
    const name = await vscode.window.showInputBox({
      prompt: 'Enter scene name',
      value: `New ${type}`
    });
    
    if (name) {
      this.storyboard.addScene(type, name, position);
    }
  }
  
  private async showSceneTypePicker(): Promise<SceneType | undefined> {
    const items = [
      { label: 'Screen', value: SceneType.Screen },
      { label: 'Component', value: SceneType.Component },
      { label: 'State', value: SceneType.State },
      { label: 'API', value: SceneType.API },
      { label: 'Decision', value: SceneType.Decision },
      { label: 'Process', value: SceneType.Process }
    ];
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: 'Select scene type'
    });
    
    return selected?.value;
  }
  
  private async showEditSceneDialog(sceneId: string): Promise<void> {
    // Implementation would show a more complex form
    // For now, just allow name editing
    if (!this.storyboard) return;
    
    const scene = this.storyboard.getScene(sceneId);
    if (!scene) return;
    
    const name = await vscode.window.showInputBox({
      prompt: 'Edit scene name',
      value: scene.name
    });
    
    if (name && name !== scene.name) {
      this.storyboard.updateScene(sceneId, { name });
    }
  }
  
  private async showEditFlowDialog(flowId: string): Promise<void> {
    // Implementation would show a more complex form
    // For now, just allow label editing
    if (!this.storyboard) return;
    
    const flow = this.storyboard.getFlow(flowId);
    if (!flow) return;
    
    const label = await vscode.window.showInputBox({
      prompt: 'Edit flow label',
      value: flow.label || ''
    });
    
    if (label !== flow.label) {
      this.storyboard.updateFlow(flowId, { label });
    }
  }
  
  private getWebviewContent(): string {
    // This would be a full HTML/JS/CSS implementation
    // For now, return a placeholder
    return `
<!DOCTYPE html>
<html>
<head>
  <style>
    body { margin: 0; padding: 0; overflow: hidden; }
    #canvas { width: 100vw; height: 100vh; }
  </style>
</head>
<body>
  <div id="canvas"></div>
  <script>
    const vscode = acquireVsCodeApi();
    
    // Handle messages from extension
    window.addEventListener('message', event => {
      const message = event.data;
      if (message.type === 'update') {
        // Update canvas with storyboard data
        renderStoryboard(message.data);
      }
    });
    
    function renderStoryboard(data) {
      // Implementation would render the storyboard
      console.log('Rendering storyboard', data);
    }
  </script>
</body>
</html>
    `;
  }
}
