/**
 * State Visualizer
 * 
 * Visualizes application state management and data flow
 */

import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import {
  StateFlow,
  StateStore,
  StateAction,
  ActionFlow,
  StateChange,
  SideEffect,
  Storyboard,
  Scene,
  SceneType,
  TransitionType
} from '../types';

export interface StateVisualizerConfig {
  framework?: 'redux' | 'mobx' | 'zustand' | 'context' | 'vuex' | 'pinia' | 'auto';
  showInitialState?: boolean;
  showActions?: boolean;
  showEffects?: boolean;
  animateFlows?: boolean;
}

export interface StateAnalysis {
  stores: StateStore[];
  actions: StateAction[];
  flows: ActionFlow[];
  effects: SideEffect[];
  complexity: StateComplexity;
}

export interface StateComplexity {
  storeCount: number;
  actionCount: number;
  effectCount: number;
  maxNesting: number;
  circularFlows: string[];
}

export class StateVisualizer extends EventEmitter {
  private logger = new Logger('StateVisualizer');
  private config: Required<StateVisualizerConfig>;
  private stateFlow?: StateFlow;
  
  constructor(config: StateVisualizerConfig = {}) {
    super();
    
    this.config = {
      framework: config.framework || 'auto',
      showInitialState: config.showInitialState ?? true,
      showActions: config.showActions ?? true,
      showEffects: config.showEffects ?? true,
      animateFlows: config.animateFlows ?? true
    };
  }
  
  /**
   * Analyze state management from code
   */
  async analyzeStateManagement(projectPath: string): Promise<StateAnalysis> {
    try {
      const framework = await this.detectFramework(projectPath);
      
      let analysis: StateAnalysis;
      
      switch (framework) {
        case 'redux':
          analysis = await this.analyzeRedux(projectPath);
          break;
          
        case 'mobx':
          analysis = await this.analyzeMobX(projectPath);
          break;
          
        case 'zustand':
          analysis = await this.analyzeZustand(projectPath);
          break;
          
        case 'context':
          analysis = await this.analyzeReactContext(projectPath);
          break;
          
        case 'vuex':
          analysis = await this.analyzeVuex(projectPath);
          break;
          
        case 'pinia':
          analysis = await this.analyzePinia(projectPath);
          break;
          
        default:
          throw new Error(`Unsupported framework: ${framework}`);
      }
      
      // Calculate complexity
      analysis.complexity = this.calculateComplexity(analysis);
      
      this.emit('analysis-completed', analysis);
      
      return analysis;
      
    } catch (error) {
      this.logger.error('Failed to analyze state management', error);
      throw error;
    }
  }
  
  /**
   * Generate storyboard from state analysis
   */
  generateStateStoryboard(analysis: StateAnalysis): Storyboard {
    const storyboard: Storyboard = {
      id: `state_${Date.now()}`,
      name: 'State Management Flow',
      type: 'state-machine',
      scenes: [],
      flows: [],
      metadata: {
        framework: this.config.framework
      },
      createdAt: new Date(),
      updatedAt: new Date(),
      version: '1.0.0'
    };
    
    // Create scenes for stores
    const storeScenes = new Map<string, Scene>();
    analysis.stores.forEach((store, index) => {
      const scene: Scene = {
        id: `store_${store.id}`,
        type: SceneType.State,
        name: store.name,
        position: {
          x: 100,
          y: 100 + index * 200
        },
        data: {
          stateName: store.name,
          initialValue: store.initialState,
          reducers: store.reducers,
          documentation: `${store.type} store`
        }
      };
      
      storyboard.scenes.push(scene);
      storeScenes.set(store.id, scene);
    });
    
    // Create scenes for actions
    const actionScenes = new Map<string, Scene>();
    analysis.actions.forEach((action, index) => {
      const scene: Scene = {
        id: `action_${action.id}`,
        type: SceneType.Process,
        name: action.name,
        position: {
          x: 400,
          y: 100 + index * 150
        },
        data: {
          steps: [{
            id: '1',
            action: action.type,
            description: `${action.async ? 'Async' : 'Sync'} action`
          }]
        }
      };
      
      storyboard.scenes.push(scene);
      actionScenes.set(action.id, scene);
    });
    
    // Create scenes for effects
    if (this.config.showEffects) {
      analysis.effects.forEach((effect, index) => {
        const scene: Scene = {
          id: `effect_${effect.id}`,
          type: SceneType.API,
          name: effect.type,
          position: {
            x: 700,
            y: 100 + index * 150
          },
          data: {
            endpoint: effect.description,
            method: 'POST'
          }
        };
        
        storyboard.scenes.push(scene);
      });
    }
    
    // Create flows for action effects
    analysis.flows.forEach(flow => {
      const actionScene = actionScenes.get(flow.actionId);
      if (!actionScene) return;
      
      // Action to state changes
      flow.stateChanges.forEach(change => {
        const storeScene = storeScenes.get(change.storeId);
        if (storeScene) {
          storyboard.flows.push({
            id: `flow_${Date.now()}_${Math.random()}`,
            type: TransitionType.StateChange,
            source: actionScene.id,
            target: storeScene.id,
            label: change.operation,
            data: {
              mutation: change.path,
              transform: JSON.stringify(change)
            }
          });
        }
      });
      
      // Action to effects
      flow.effects.forEach(effectId => {
        const effectScene = storyboard.scenes.find(s => s.id === `effect_${effectId}`);
        if (effectScene) {
          storyboard.flows.push({
            id: `flow_${Date.now()}_${Math.random()}`,
            type: TransitionType.Action,
            source: actionScene.id,
            target: effectScene.id,
            label: 'triggers'
          });
        }
      });
    });
    
    return storyboard;
  }
  
  /**
   * Visualize state changes over time
   */
  async visualizeStateChanges(
    actions: StateAction[],
    initialState: any
  ): Promise<StateChange[]> {
    const changes: StateChange[] = [];
    let currentState = { ...initialState };
    
    for (const action of actions) {
      const flow = await this.simulateAction(action, currentState);
      
      flow.stateChanges.forEach(change => {
        changes.push({
          ...change,
          oldValue: this.getValueAtPath(currentState, change.path),
          newValue: change.newValue
        });
        
        // Apply change to current state
        this.applyStateChange(currentState, change);
      });
    }
    
    return changes;
  }
  
  /**
   * Generate time-travel debugger data
   */
  generateTimeTravelData(
    actions: StateAction[],
    changes: StateChange[]
  ): any[] {
    const timeline: any[] = [];
    let changeIndex = 0;
    
    actions.forEach((action, actionIndex) => {
      const actionChanges: StateChange[] = [];
      
      // Collect changes for this action
      while (changeIndex < changes.length) {
        const change = changes[changeIndex];
        actionChanges.push(change);
        changeIndex++;
        
        // Assume changes are grouped by action
        if (changeIndex < changes.length &&
            changes[changeIndex].storeId !== change.storeId) {
          break;
        }
      }
      
      timeline.push({
        index: actionIndex,
        action: {
          type: action.type,
          payload: action.payload
        },
        changes: actionChanges,
        timestamp: Date.now() + actionIndex * 100
      });
    });
    
    return timeline;
  }
  
  /**
   * Detect circular dependencies in state flows
   */
  detectCircularDependencies(flows: ActionFlow[]): string[] {
    const circular: string[] = [];
    const graph = new Map<string, Set<string>>();
    
    // Build dependency graph
    flows.forEach(flow => {
      if (!graph.has(flow.actionId)) {
        graph.set(flow.actionId, new Set());
      }
      
      // Action depends on its triggers
      if (flow.triggers) {
        flow.triggers.forEach(trigger => {
          graph.get(flow.actionId)!.add(trigger);
        });
      }
    });
    
    // DFS to find cycles
    const visited = new Set<string>();
    const recursionStack = new Set<string>();
    
    const hasCycle = (node: string, path: string[] = []): boolean => {
      visited.add(node);
      recursionStack.add(node);
      path.push(node);
      
      const dependencies = graph.get(node);
      if (dependencies) {
        for (const dep of dependencies) {
          if (!visited.has(dep)) {
            if (hasCycle(dep, [...path])) {
              return true;
            }
          } else if (recursionStack.has(dep)) {
            // Found cycle
            const cycleStart = path.indexOf(dep);
            const cycle = path.slice(cycleStart).join(' -> ') + ' -> ' + dep;
            circular.push(cycle);
            return true;
          }
        }
      }
      
      recursionStack.delete(node);
      return false;
    };
    
    for (const node of graph.keys()) {
      if (!visited.has(node)) {
        hasCycle(node);
      }
    }
    
    return circular;
  }
  
  // Private analysis methods
  
  private async detectFramework(projectPath: string): Promise<string> {
    // Check package.json for state management libraries
    // This is a simplified implementation
    return this.config.framework === 'auto' ? 'redux' : this.config.framework;
  }
  
  private async analyzeRedux(projectPath: string): Promise<StateAnalysis> {
    // Analyze Redux store, actions, reducers, sagas/thunks
    const stores: StateStore[] = [{
      id: 'root',
      name: 'Root Store',
      type: 'redux',
      schema: {},
      initialState: {},
      reducers: ['rootReducer'],
      actions: ['increment', 'decrement', 'fetchData']
    }];
    
    const actions: StateAction[] = [
      {
        id: 'increment',
        name: 'INCREMENT',
        type: 'INCREMENT',
        storeId: 'root',
        async: false
      },
      {
        id: 'fetchData',
        name: 'FETCH_DATA',
        type: 'FETCH_DATA',
        storeId: 'root',
        async: true
      }
    ];
    
    const flows: ActionFlow[] = [
      {
        actionId: 'increment',
        effects: [],
        stateChanges: [{
          storeId: 'root',
          path: 'counter',
          operation: 'update',
          newValue: 'counter + 1'
        }]
      }
    ];
    
    const effects: SideEffect[] = [
      {
        id: 'api-call',
        type: 'api',
        trigger: 'fetchData',
        description: 'Fetch user data',
        async: true
      }
    ];
    
    return { stores, actions, flows, effects, complexity: {} as any };
  }
  
  private async analyzeMobX(projectPath: string): Promise<StateAnalysis> {
    // Analyze MobX stores, observables, actions, reactions
    return { stores: [], actions: [], flows: [], effects: [], complexity: {} as any };
  }
  
  private async analyzeZustand(projectPath: string): Promise<StateAnalysis> {
    // Analyze Zustand stores and actions
    return { stores: [], actions: [], flows: [], effects: [], complexity: {} as any };
  }
  
  private async analyzeReactContext(projectPath: string): Promise<StateAnalysis> {
    // Analyze React Context providers and consumers
    return { stores: [], actions: [], flows: [], effects: [], complexity: {} as any };
  }
  
  private async analyzeVuex(projectPath: string): Promise<StateAnalysis> {
    // Analyze Vuex stores, mutations, actions, getters
    return { stores: [], actions: [], flows: [], effects: [], complexity: {} as any };
  }
  
  private async analyzePinia(projectPath: string): Promise<StateAnalysis> {
    // Analyze Pinia stores
    return { stores: [], actions: [], flows: [], effects: [], complexity: {} as any };
  }
  
  private calculateComplexity(analysis: StateAnalysis): StateComplexity {
    const circularFlows = this.detectCircularDependencies(analysis.flows);
    
    return {
      storeCount: analysis.stores.length,
      actionCount: analysis.actions.length,
      effectCount: analysis.effects.length,
      maxNesting: this.calculateMaxNesting(analysis.stores),
      circularFlows
    };
  }
  
  private calculateMaxNesting(stores: StateStore[]): number {
    let maxDepth = 0;
    
    stores.forEach(store => {
      if (store.schema) {
        const depth = this.getObjectDepth(store.schema);
        maxDepth = Math.max(maxDepth, depth);
      }
    });
    
    return maxDepth;
  }
  
  private getObjectDepth(obj: any, currentDepth: number = 0): number {
    if (typeof obj !== 'object' || obj === null) {
      return currentDepth;
    }
    
    let maxDepth = currentDepth;
    
    for (const key in obj) {
      if (obj.hasOwnProperty(key)) {
        const depth = this.getObjectDepth(obj[key], currentDepth + 1);
        maxDepth = Math.max(maxDepth, depth);
      }
    }
    
    return maxDepth;
  }
  
  private async simulateAction(
    action: StateAction,
    currentState: any
  ): Promise<ActionFlow> {
    // Simulate action execution and return state changes
    // This is a simplified implementation
    return {
      actionId: action.id,
      effects: [],
      stateChanges: []
    };
  }
  
  private getValueAtPath(obj: any, path: string): any {
    const parts = path.split('.');
    let current = obj;
    
    for (const part of parts) {
      if (current && typeof current === 'object' && part in current) {
        current = current[part];
      } else {
        return undefined;
      }
    }
    
    return current;
  }
  
  private applyStateChange(state: any, change: StateChange): void {
    const parts = change.path.split('.');
    let current = state;
    
    for (let i = 0; i < parts.length - 1; i++) {
      const part = parts[i];
      if (!(part in current)) {
        current[part] = {};
      }
      current = current[part];
    }
    
    const lastPart = parts[parts.length - 1];
    
    switch (change.operation) {
      case 'set':
        current[lastPart] = change.newValue;
        break;
        
      case 'update':
        Object.assign(current[lastPart], change.newValue);
        break;
        
      case 'delete':
        delete current[lastPart];
        break;
        
      case 'append':
        if (Array.isArray(current[lastPart])) {
          current[lastPart].push(change.newValue);
        }
        break;
    }
  }
}
