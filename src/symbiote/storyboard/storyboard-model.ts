/**
 * Storyboard Model
 * 
 * Manages storyboard document structure and operations
 */

import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import {
  Storyboard,
  StoryboardType,
  Scene,
  SceneType,
  Flow,
  TransitionType,
  Position,
  StoryboardMetadata,
  SceneData,
  FlowData
} from './types';

export interface StoryboardModelOptions {
  id?: string;
  name?: string;
  type?: StoryboardType;
  metadata?: StoryboardMetadata;
}

export class StoryboardModel extends EventEmitter {
  private storyboard: Storyboard;
  private sceneMap = new Map<string, Scene>();
  private flowMap = new Map<string, Flow>();
  private isDirty = false;
  
  constructor(options: StoryboardModelOptions = {}) {
    super();
    
    this.storyboard = {
      id: options.id || uuidv4(),
      name: options.name || 'Untitled Storyboard',
      type: options.type || StoryboardType.UserFlow,
      scenes: [],
      flows: [],
      metadata: options.metadata || {},
      createdAt: new Date(),
      updatedAt: new Date(),
      version: '1.0.0'
    };
  }
  
  /**
   * Get storyboard data
   */
  getStoryboard(): Storyboard {
    return {
      ...this.storyboard,
      scenes: [...this.storyboard.scenes],
      flows: [...this.storyboard.flows]
    };
  }
  
  /**
   * Get storyboard ID
   */
  getId(): string {
    return this.storyboard.id;
  }
  
  /**
   * Get storyboard name
   */
  getName(): string {
    return this.storyboard.name;
  }
  
  /**
   * Set storyboard name
   */
  setName(name: string): void {
    this.storyboard.name = name;
    this.markDirty();
    this.emit('name-changed', name);
  }
  
  /**
   * Get storyboard type
   */
  getType(): StoryboardType {
    return this.storyboard.type;
  }
  
  /**
   * Set storyboard type
   */
  setType(type: StoryboardType): void {
    this.storyboard.type = type;
    this.markDirty();
    this.emit('type-changed', type);
  }
  
  /**
   * Add a scene
   */
  addScene(
    type: SceneType,
    name: string,
    position: Position,
    data?: Partial<SceneData>
  ): Scene {
    const scene: Scene = {
      id: `scene_${uuidv4()}`,
      type,
      name,
      position,
      data: data || {},
      metadata: {}
    };
    
    this.storyboard.scenes.push(scene);
    this.sceneMap.set(scene.id, scene);
    
    this.markDirty();
    this.emit('scene-added', scene);
    
    return scene;
  }
  
  /**
   * Update scene
   */
  updateScene(sceneId: string, updates: Partial<Scene>): void {
    const scene = this.getScene(sceneId);
    if (!scene) {
      throw new Error(`Scene not found: ${sceneId}`);
    }
    
    // Update scene properties
    Object.assign(scene, updates);
    
    this.markDirty();
    this.emit('scene-updated', scene);
  }
  
  /**
   * Move scene
   */
  moveScene(sceneId: string, position: Position): void {
    const scene = this.getScene(sceneId);
    if (!scene) {
      throw new Error(`Scene not found: ${sceneId}`);
    }
    
    scene.position = position;
    
    this.markDirty();
    this.emit('scene-moved', { sceneId, position });
  }
  
  /**
   * Delete scene
   */
  deleteScene(sceneId: string): void {
    const index = this.storyboard.scenes.findIndex(s => s.id === sceneId);
    if (index === -1) {
      throw new Error(`Scene not found: ${sceneId}`);
    }
    
    // Remove scene
    this.storyboard.scenes.splice(index, 1);
    this.sceneMap.delete(sceneId);
    
    // Remove related flows
    this.storyboard.flows = this.storyboard.flows.filter(
      flow => flow.source !== sceneId && flow.target !== sceneId
    );
    
    this.markDirty();
    this.emit('scene-deleted', sceneId);
  }
  
  /**
   * Get scene by ID
   */
  getScene(sceneId: string): Scene | undefined {
    return this.sceneMap.get(sceneId);
  }
  
  /**
   * Get all scenes
   */
  getScenes(): Scene[] {
    return [...this.storyboard.scenes];
  }
  
  /**
   * Add a flow
   */
  addFlow(
    type: TransitionType,
    source: string,
    target: string,
    label?: string,
    data?: Partial<FlowData>
  ): Flow {
    // Validate source and target exist
    if (!this.sceneMap.has(source)) {
      throw new Error(`Source scene not found: ${source}`);
    }
    if (!this.sceneMap.has(target)) {
      throw new Error(`Target scene not found: ${target}`);
    }
    
    const flow: Flow = {
      id: `flow_${uuidv4()}`,
      type,
      source,
      target,
      label,
      data: data || {},
      metadata: {}
    };
    
    this.storyboard.flows.push(flow);
    this.flowMap.set(flow.id, flow);
    
    this.markDirty();
    this.emit('flow-added', flow);
    
    return flow;
  }
  
  /**
   * Update flow
   */
  updateFlow(flowId: string, updates: Partial<Flow>): void {
    const flow = this.getFlow(flowId);
    if (!flow) {
      throw new Error(`Flow not found: ${flowId}`);
    }
    
    // Update flow properties
    Object.assign(flow, updates);
    
    this.markDirty();
    this.emit('flow-updated', flow);
  }
  
  /**
   * Delete flow
   */
  deleteFlow(flowId: string): void {
    const index = this.storyboard.flows.findIndex(f => f.id === flowId);
    if (index === -1) {
      throw new Error(`Flow not found: ${flowId}`);
    }
    
    // Remove flow
    this.storyboard.flows.splice(index, 1);
    this.flowMap.delete(flowId);
    
    this.markDirty();
    this.emit('flow-deleted', flowId);
  }
  
  /**
   * Get flow by ID
   */
  getFlow(flowId: string): Flow | undefined {
    return this.flowMap.get(flowId);
  }
  
  /**
   * Get all flows
   */
  getFlows(): Flow[] {
    return [...this.storyboard.flows];
  }
  
  /**
   * Get flows connected to a scene
   */
  getSceneFlows(sceneId: string): {
    incoming: Flow[];
    outgoing: Flow[];
  } {
    const incoming = this.storyboard.flows.filter(f => f.target === sceneId);
    const outgoing = this.storyboard.flows.filter(f => f.source === sceneId);
    
    return { incoming, outgoing };
  }
  
  /**
   * Find path between two scenes
   */
  findPath(startId: string, endId: string): Scene[] | null {
    if (!this.sceneMap.has(startId) || !this.sceneMap.has(endId)) {
      return null;
    }
    
    const visited = new Set<string>();
    const queue: { sceneId: string; path: Scene[] }[] = [
      { sceneId: startId, path: [this.sceneMap.get(startId)!] }
    ];
    
    while (queue.length > 0) {
      const { sceneId, path } = queue.shift()!;
      
      if (sceneId === endId) {
        return path;
      }
      
      if (visited.has(sceneId)) {
        continue;
      }
      
      visited.add(sceneId);
      
      // Get outgoing flows
      const outgoing = this.storyboard.flows.filter(f => f.source === sceneId);
      
      for (const flow of outgoing) {
        const nextScene = this.sceneMap.get(flow.target);
        if (nextScene && !visited.has(flow.target)) {
          queue.push({
            sceneId: flow.target,
            path: [...path, nextScene]
          });
        }
      }
    }
    
    return null;
  }
  
  /**
   * Detect cycles in the storyboard
   */
  detectCycles(): Scene[][] {
    const cycles: Scene[][] = [];
    const visited = new Set<string>();
    const recursionStack = new Set<string>();
    
    const dfs = (sceneId: string, path: Scene[]): void => {
      visited.add(sceneId);
      recursionStack.add(sceneId);
      
      const outgoing = this.storyboard.flows.filter(f => f.source === sceneId);
      
      for (const flow of outgoing) {
        if (!visited.has(flow.target)) {
          const nextScene = this.sceneMap.get(flow.target)!;
          dfs(flow.target, [...path, nextScene]);
        } else if (recursionStack.has(flow.target)) {
          // Found a cycle
          const cycleStart = path.findIndex(s => s.id === flow.target);
          if (cycleStart !== -1) {
            cycles.push(path.slice(cycleStart));
          }
        }
      }
      
      recursionStack.delete(sceneId);
    };
    
    // Check all unvisited scenes
    for (const scene of this.storyboard.scenes) {
      if (!visited.has(scene.id)) {
        dfs(scene.id, [scene]);
      }
    }
    
    return cycles;
  }
  
  /**
   * Get orphaned scenes (no incoming or outgoing flows)
   */
  getOrphanedScenes(): Scene[] {
    return this.storyboard.scenes.filter(scene => {
      const flows = this.getSceneFlows(scene.id);
      return flows.incoming.length === 0 && flows.outgoing.length === 0;
    });
  }
  
  /**
   * Auto-layout scenes
   */
  autoLayout(algorithm: 'dagre' | 'force' | 'circular' = 'dagre'): void {
    // Simple grid layout for now
    const gridSize = Math.ceil(Math.sqrt(this.storyboard.scenes.length));
    const spacing = 200;
    const startX = 100;
    const startY = 100;
    
    this.storyboard.scenes.forEach((scene, index) => {
      const row = Math.floor(index / gridSize);
      const col = index % gridSize;
      
      scene.position = {
        x: startX + col * spacing,
        y: startY + row * spacing
      };
    });
    
    this.markDirty();
    this.emit('layout-applied', algorithm);
  }
  
  /**
   * Validate storyboard
   */
  validate(): {
    valid: boolean;
    errors: string[];
    warnings: string[];
  } {
    const errors: string[] = [];
    const warnings: string[] = [];
    
    // Check for orphaned scenes
    const orphans = this.getOrphanedScenes();
    if (orphans.length > 0) {
      warnings.push(`${orphans.length} orphaned scene(s) found`);
    }
    
    // Check for cycles
    const cycles = this.detectCycles();
    if (cycles.length > 0) {
      warnings.push(`${cycles.length} cycle(s) detected`);
    }
    
    // Check for missing required data
    this.storyboard.scenes.forEach(scene => {
      if (!scene.name) {
        errors.push(`Scene ${scene.id} has no name`);
      }
      
      // Type-specific validation
      switch (scene.type) {
        case SceneType.API:
          if (!scene.data.endpoint) {
            warnings.push(`API scene ${scene.name} has no endpoint`);
          }
          break;
          
        case SceneType.Component:
          if (!scene.data.componentPath) {
            warnings.push(`Component scene ${scene.name} has no component path`);
          }
          break;
      }
    });
    
    return {
      valid: errors.length === 0,
      errors,
      warnings
    };
  }
  
  /**
   * Clone the storyboard
   */
  clone(): StoryboardModel {
    const cloned = new StoryboardModel({
      name: `${this.storyboard.name} (Copy)`,
      type: this.storyboard.type,
      metadata: { ...this.storyboard.metadata }
    });
    
    // Clone scenes
    const sceneIdMap = new Map<string, string>();
    this.storyboard.scenes.forEach(scene => {
      const newScene = cloned.addScene(
        scene.type,
        scene.name,
        { ...scene.position },
        { ...scene.data }
      );
      sceneIdMap.set(scene.id, newScene.id);
    });
    
    // Clone flows with new IDs
    this.storyboard.flows.forEach(flow => {
      const newSource = sceneIdMap.get(flow.source);
      const newTarget = sceneIdMap.get(flow.target);
      
      if (newSource && newTarget) {
        cloned.addFlow(
          flow.type,
          newSource,
          newTarget,
          flow.label,
          { ...flow.data }
        );
      }
    });
    
    return cloned;
  }
  
  /**
   * Export to JSON
   */
  toJSON(): string {
    return JSON.stringify(this.storyboard, null, 2);
  }
  
  /**
   * Import from JSON
   */
  static fromJSON(json: string): StoryboardModel {
    const data = JSON.parse(json);
    const model = new StoryboardModel({
      id: data.id,
      name: data.name,
      type: data.type,
      metadata: data.metadata
    });
    
    // Restore scenes
    data.scenes.forEach((sceneData: Scene) => {
      const scene: Scene = {
        ...sceneData,
        metadata: sceneData.metadata || {}
      };
      model.storyboard.scenes.push(scene);
      model.sceneMap.set(scene.id, scene);
    });
    
    // Restore flows
    data.flows.forEach((flowData: Flow) => {
      const flow: Flow = {
        ...flowData,
        metadata: flowData.metadata || {}
      };
      model.storyboard.flows.push(flow);
      model.flowMap.set(flow.id, flow);
    });
    
    // Restore dates
    model.storyboard.createdAt = new Date(data.createdAt);
    model.storyboard.updatedAt = new Date(data.updatedAt);
    model.storyboard.version = data.version || '1.0.0';
    
    return model;
  }
  
  /**
   * Check if storyboard is dirty
   */
  isDirtyState(): boolean {
    return this.isDirty;
  }
  
  /**
   * Mark as clean
   */
  markClean(): void {
    this.isDirty = false;
    this.emit('dirty-state-changed', false);
  }
  
  // Private methods
  
  private markDirty(): void {
    this.isDirty = true;
    this.storyboard.updatedAt = new Date();
    this.emit('dirty-state-changed', true);
  }
}