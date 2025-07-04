/**
 * Storyboard API Implementation
 * 
 * Implements the storyboard API for VS Code integration
 */

import * as vscode from 'vscode';
import { StoryboardAPI } from '../../types/storyboard-api';
import { StoryboardController } from './storyboard-controller';
import { FlowEditor } from './editor/flow-editor';
import { StateVisualizer } from './visualization/state-visualizer';
import { StoryboardProvider } from './providers/storyboard-provider';
import { Logger } from '../utils/logger';
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
} from './types';

export class StoryboardAPIImpl implements StoryboardAPI {
  private controller: StoryboardController;
  private stateVisualizer: StateVisualizer;
  private activeEditor?: FlowEditor;
  private logger = new Logger('StoryboardAPI');
  private disposables: vscode.Disposable[] = [];
  
  constructor(private context: vscode.ExtensionContext) {
    // Initialize controller
    this.controller = new StoryboardController({
      extensionContext: context
    });
    
    // Initialize state visualizer
    this.stateVisualizer = new StateVisualizer({
      framework: 'auto',
      showInitialState: true,
      showActions: true,
      showEffects: true,
      animateFlows: true
    });
  }
  
  /**
   * Initialize the storyboard system
   */
  async initialize(): Promise<void> {
    try {
      // Initialize controller
      await this.controller.initialize();
      
      // Register custom editor provider
      const providerDisposable = StoryboardProvider.register(
        this.context,
        this.controller
      );
      this.disposables.push(providerDisposable);
      
      // Register file association
      await this.registerFileAssociation();
      
      // Set up event listeners
      this.setupEventListeners();
      
      this.logger.info('Storyboard API initialized');
      
    } catch (error) {
      this.logger.error('Failed to initialize storyboard API', error);
      throw error;
    }
  }
  
  /**
   * Create a new storyboard
   */
  async createStoryboard(
    name: string,
    type: StoryboardType
  ): Promise<Storyboard> {
    try {
      const model = await this.controller.createStoryboard(name, type);
      const storyboard = model.getStoryboard();
      
      // Open in editor
      await this.openStoryboardEditor(storyboard);
      
      return storyboard;
      
    } catch (error) {
      this.logger.error('Failed to create storyboard', error);
      throw error;
    }
  }
  
  /**
   * Open existing storyboard
   */
  async openStoryboard(uri: vscode.Uri): Promise<Storyboard> {
    try {
      const model = await this.controller.openStoryboard(uri);
      return model.getStoryboard();
      
    } catch (error) {
      this.logger.error('Failed to open storyboard', error);
      throw error;
    }
  }
  
  /**
   * Save storyboard
   */
  async saveStoryboard(
    storyboardId: string,
    uri?: vscode.Uri
  ): Promise<void> {
    try {
      await this.controller.saveStoryboard(storyboardId, uri);
      
    } catch (error) {
      this.logger.error('Failed to save storyboard', error);
      throw error;
    }
  }
  
  /**
   * Analyze project components
   */
  async analyzeComponents(
    rootPath?: string
  ): Promise<ComponentAnalysis> {
    try {
      return await this.controller.analyzeProject(rootPath);
      
    } catch (error) {
      this.logger.error('Failed to analyze components', error);
      throw error;
    }
  }
  
  /**
   * Analyze state management
   */
  async analyzeStateManagement(
    projectPath?: string
  ): Promise<StateAnalysis> {
    try {
      const path = projectPath || 
        vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      
      if (!path) {
        throw new Error('No workspace folder open');
      }
      
      return await this.stateVisualizer.analyzeStateManagement(path);
      
    } catch (error) {
      this.logger.error('Failed to analyze state management', error);
      throw error;
    }
  }
  
  /**
   * Generate storyboard from analysis
   */
  async generateFromAnalysis(
    analysis: ComponentAnalysis | StateAnalysis,
    type?: StoryboardType
  ): Promise<Storyboard> {
    try {
      let model;
      
      if ('components' in analysis) {
        // Component analysis
        model = await this.controller.generateFromAnalysis(
          analysis as ComponentAnalysis,
          type || StoryboardType.ComponentHierarchy
        );
      } else {
        // State analysis
        const storyboard = this.stateVisualizer.generateStateStoryboard(
          analysis as StateAnalysis
        );
        
        // Create model from generated storyboard
        model = await this.controller.createStoryboard(
          storyboard.name,
          StoryboardType.StateMachine
        );
        
        // Add scenes and flows
        storyboard.scenes.forEach(scene => {
          model.addScene(scene.type, scene.name, scene.position, scene.data);
        });
        
        storyboard.flows.forEach(flow => {
          model.addFlow(flow.type, flow.source, flow.target, flow.label, flow.data);
        });
      }
      
      return model.getStoryboard();
      
    } catch (error) {
      this.logger.error('Failed to generate from analysis', error);
      throw error;
    }
  }
  
  /**
   * Add scene to storyboard
   */
  async addScene(
    storyboardId: string,
    type: SceneType,
    name: string,
    position: { x: number; y: number }
  ): Promise<Scene> {
    try {
      const model = this.controller.getAllStoryboards()
        .find(m => m.getId() === storyboardId);
      
      if (!model) {
        throw new Error(`Storyboard not found: ${storyboardId}`);
      }
      
      return model.addScene(type, name, position);
      
    } catch (error) {
      this.logger.error('Failed to add scene', error);
      throw error;
    }
  }
  
  /**
   * Add flow between scenes
   */
  async addFlow(
    storyboardId: string,
    type: TransitionType,
    sourceId: string,
    targetId: string,
    label?: string
  ): Promise<Flow> {
    try {
      const model = this.controller.getAllStoryboards()
        .find(m => m.getId() === storyboardId);
      
      if (!model) {
        throw new Error(`Storyboard not found: ${storyboardId}`);
      }
      
      return model.addFlow(type, sourceId, targetId, label);
      
    } catch (error) {
      this.logger.error('Failed to add flow', error);
      throw error;
    }
  }
  
  /**
   * Export storyboard
   */
  async exportStoryboard(
    storyboardId: string,
    options: ExportOptions
  ): Promise<{ content: string | Buffer; filename: string }> {
    try {
      const result = await this.controller.exportStoryboard(
        storyboardId,
        options
      );
      
      return {
        content: result.content,
        filename: result.filename
      };
      
    } catch (error) {
      this.logger.error('Failed to export storyboard', error);
      throw error;
    }
  }
  
  /**
   * Generate code from storyboard
   */
  async generateCode(
    storyboardId: string,
    options: {
      framework?: 'react' | 'vue' | 'angular';
      outputPath?: string;
    }
  ): Promise<Map<string, string>> {
    try {
      const files = await this.controller.generateCode(
        storyboardId,
        {
          framework: options.framework,
          generateRoutes: true,
          generateComponents: true,
          generateState: true
        }
      );
      
      // Write files if output path provided
      if (options.outputPath) {
        const outputUri = vscode.Uri.file(options.outputPath);
        
        for (const [filename, content] of files) {
          const fileUri = vscode.Uri.joinPath(outputUri, filename);
          const dirUri = vscode.Uri.joinPath(fileUri, '..');
          
          // Ensure directory exists
          try {
            await vscode.workspace.fs.createDirectory(dirUri);
          } catch {}
          
          // Write file
          await vscode.workspace.fs.writeFile(
            fileUri,
            Buffer.from(content, 'utf8')
          );
        }
        
        vscode.window.showInformationMessage(
          `Generated ${files.size} files in ${options.outputPath}`
        );
      }
      
      return files;
      
    } catch (error) {
      this.logger.error('Failed to generate code', error);
      throw error;
    }
  }
  
  /**
   * Show flow editor
   */
  async showFlowEditor(storyboardId: string): Promise<void> {
    try {
      const model = this.controller.getAllStoryboards()
        .find(m => m.getId() === storyboardId);
      
      if (!model) {
        throw new Error(`Storyboard not found: ${storyboardId}`);
      }
      
      // Create and show flow editor
      this.activeEditor = new FlowEditor({
        gridSize: 20,
        snapToGrid: true,
        showGrid: true,
        enableMultiSelect: true,
        enableKeyboardShortcuts: true
      });
      
      this.activeEditor.initialize(model);
      await this.activeEditor.show();
      
    } catch (error) {
      this.logger.error('Failed to show flow editor', error);
      throw error;
    }
  }
  
  /**
   * Get active storyboard
   */
  getActiveStoryboard(): Storyboard | undefined {
    const model = this.controller.getActiveStoryboard();
    return model?.getStoryboard();
  }
  
  /**
   * Get all storyboards
   */
  getAllStoryboards(): Storyboard[] {
    return this.controller.getAllStoryboards()
      .map(model => model.getStoryboard());
  }
  
  /**
   * Dispose the API
   */
  dispose(): void {
    this.controller.dispose();
    this.activeEditor?.dispose();
    
    this.disposables.forEach(d => d.dispose());
    this.disposables = [];
  }
  
  // Private methods
  
  private async registerFileAssociation(): Promise<void> {
    // Register .story file association
    const config = vscode.workspace.getConfiguration();
    const associations = config.get<any>('files.associations') || {};
    
    if (!associations['*.story']) {
      associations['*.story'] = 'json';
      await config.update(
        'files.associations',
        associations,
        vscode.ConfigurationTarget.Global
      );
    }
  }
  
  private setupEventListeners(): void {
    // Controller events
    this.controller.on('storyboard-created', (storyboard) => {
      vscode.window.showInformationMessage(
        `Created storyboard: ${storyboard.name}`
      );
    });
    
    this.controller.on('analysis-completed', (analysis) => {
      this.logger.info('Component analysis completed', {
        components: analysis.components.length,
        relationships: analysis.relationships.length
      });
    });
    
    this.controller.on('code-generated', ({ files }) => {
      vscode.window.showInformationMessage(
        `Generated ${files.size} code files`
      );
    });
    
    // State visualizer events
    this.stateVisualizer.on('analysis-completed', (analysis) => {
      this.logger.info('State analysis completed', {
        stores: analysis.stores.length,
        actions: analysis.actions.length,
        effects: analysis.effects.length
      });
    });
  }
  
  private async openStoryboardEditor(storyboard: Storyboard): Promise<void> {
    // Create temporary file with storyboard content
    const uri = vscode.Uri.parse(
      `untitled:${storyboard.name.replace(/\s+/g, '-')}-${Date.now()}.story`
    );
    
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc);
    
    // Insert storyboard JSON
    const model = this.controller.getAllStoryboards()
      .find(m => m.getId() === storyboard.id);
    
    if (model) {
      await editor.edit(editBuilder => {
        editBuilder.insert(new vscode.Position(0, 0), model.toJSON());
      });
    }
  }
}
