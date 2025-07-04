/**
 * Storyboard API Type Definitions
 */

import * as vscode from 'vscode';
import {
  Storyboard,
  Scene,
  Flow,
  StoryboardType,
  SceneType,
  TransitionType,
  ComponentAnalysis,
  StateAnalysis,
  ExportOptions
} from '../symbiote/storyboard/types';

export interface StoryboardAPI {
  /**
   * Initialize the storyboard system
   */
  initialize(): Promise<void>;
  
  /**
   * Create a new storyboard
   */
  createStoryboard(
    name: string,
    type: StoryboardType
  ): Promise<Storyboard>;
  
  /**
   * Open an existing storyboard
   */
  openStoryboard(uri: vscode.Uri): Promise<Storyboard>;
  
  /**
   * Save storyboard to file
   */
  saveStoryboard(
    storyboardId: string,
    uri?: vscode.Uri
  ): Promise<void>;
  
  /**
   * Analyze project components
   */
  analyzeComponents(
    rootPath?: string
  ): Promise<ComponentAnalysis>;
  
  /**
   * Analyze state management
   */
  analyzeStateManagement(
    projectPath?: string
  ): Promise<StateAnalysis>;
  
  /**
   * Generate storyboard from analysis
   */
  generateFromAnalysis(
    analysis: ComponentAnalysis | StateAnalysis,
    type?: StoryboardType
  ): Promise<Storyboard>;
  
  /**
   * Add a scene to storyboard
   */
  addScene(
    storyboardId: string,
    type: SceneType,
    name: string,
    position: { x: number; y: number }
  ): Promise<Scene>;
  
  /**
   * Add a flow between scenes
   */
  addFlow(
    storyboardId: string,
    type: TransitionType,
    sourceId: string,
    targetId: string,
    label?: string
  ): Promise<Flow>;
  
  /**
   * Export storyboard in various formats
   */
  exportStoryboard(
    storyboardId: string,
    options: ExportOptions
  ): Promise<{ content: string | Buffer; filename: string }>;
  
  /**
   * Generate code from storyboard
   */
  generateCode(
    storyboardId: string,
    options: {
      framework?: 'react' | 'vue' | 'angular';
      outputPath?: string;
    }
  ): Promise<Map<string, string>>;
  
  /**
   * Show visual flow editor
   */
  showFlowEditor(storyboardId: string): Promise<void>;
  
  /**
   * Get active storyboard
   */
  getActiveStoryboard(): Storyboard | undefined;
  
  /**
   * Get all storyboards
   */
  getAllStoryboards(): Storyboard[];
  
  /**
   * Dispose the API
   */
  dispose(): void;
}

export interface StoryboardProvider {
  /**
   * Register custom editor provider
   */
  register(
    context: vscode.ExtensionContext
  ): vscode.Disposable;
}

export interface FlowEditorEvents {
  'initialized': () => void;
  'closed': () => void;
  'selection-changed': (selection: {
    scenes: string[];
    flows: string[];
  }) => void;
  'drag-start': (event: { scenes: string[]; position: any }) => void;
  'drag-update': (event: { position: any }) => void;
  'drag-end': (event: { scenes: string[] }) => void;
  'connection-start': (event: { sceneId: string }) => void;
  'connection-created': (event: {
    sourceId: string;
    targetId: string;
    type: TransitionType;
  }) => void;
  'connection-cancelled': () => void;
  'items-deleted': () => void;
  'zoom-changed': (level: number) => void;
}

export interface StateVisualizerEvents {
  'analysis-completed': (analysis: StateAnalysis) => void;
}
