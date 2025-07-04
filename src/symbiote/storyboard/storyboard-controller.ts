/**
 * Storyboard Controller
 * 
 * Main controller for storyboard functionality
 */

import * as vscode from 'vscode';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { StoryboardModel } from './storyboard-model';
import { MermaidRenderer } from './mermaid/mermaid-renderer';
import { ComponentAnalyzer } from './analysis/component-analyzer';
import {
  Storyboard,
  StoryboardType,
  Scene,
  SceneType,
  Flow,
  TransitionType,
  Position,
  ComponentAnalysis,
  ExportOptions,
  ExportResult
} from './types';

export interface StoryboardControllerConfig {
  extensionContext: vscode.ExtensionContext;
  orchestrationEngine?: any;
}

export class StoryboardController extends EventEmitter {
  private storyboards = new Map<string, StoryboardModel>();
  private activeStoryboard?: StoryboardModel;
  private mermaidRenderer: MermaidRenderer;
  private componentAnalyzer: ComponentAnalyzer;
  private logger = new Logger('StoryboardController');
  private disposables: vscode.Disposable[] = [];
  
  constructor(private config: StoryboardControllerConfig) {
    super();
    
    // Initialize components
    this.mermaidRenderer = new MermaidRenderer({
      enableInteractivity: true,
      enableZoom: true
    });
    
    this.componentAnalyzer = new ComponentAnalyzer({
      framework: 'auto',
      analyzePropTypes: true,
      analyzeState: true
    });
    
    // Set up event listeners
    this.setupEventListeners();
  }
  
  /**
   * Initialize the controller
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing storyboard controller...');
    
    // Initialize Mermaid renderer
    await this.mermaidRenderer.initialize();
    
    // Register commands
    this.registerCommands();
    
    this.logger.info('Storyboard controller initialized');
    this.emit('initialized');
  }
  
  /**
   * Create a new storyboard
   */
  async createStoryboard(
    name?: string,
    type?: StoryboardType
  ): Promise<StoryboardModel> {
    const storyboard = new StoryboardModel({
      name: name || 'Untitled Storyboard',
      type: type || StoryboardType.UserFlow
    });
    
    // Add to collection
    this.storyboards.set(storyboard.getId(), storyboard);
    this.activeStoryboard = storyboard;
    
    // Set up storyboard event listeners
    this.setupStoryboardListeners(storyboard);
    
    this.emit('storyboard-created', storyboard.getStoryboard());
    
    return storyboard;
  }
  
  /**
   * Open an existing storyboard
   */
  async openStoryboard(uri: vscode.Uri): Promise<StoryboardModel> {
    try {
      // Read storyboard file
      const content = await vscode.workspace.fs.readFile(uri);
      const json = new TextDecoder().decode(content);
      
      // Load storyboard model
      const storyboard = StoryboardModel.fromJSON(json);
      
      // Add to collection
      this.storyboards.set(storyboard.getId(), storyboard);
      this.activeStoryboard = storyboard;
      
      // Set up listeners
      this.setupStoryboardListeners(storyboard);
      
      this.emit('storyboard-opened', storyboard.getStoryboard());
      
      return storyboard;
      
    } catch (error) {
      this.logger.error('Failed to open storyboard', error);
      throw error;
    }
  }
  
  /**
   * Save storyboard
   */
  async saveStoryboard(storyboardId: string, uri?: vscode.Uri): Promise<void> {
    const storyboard = this.storyboards.get(storyboardId);
    if (!storyboard) {
      throw new Error(`Storyboard not found: ${storyboardId}`);
    }
    
    try {
      const json = storyboard.toJSON();
      const content = new TextEncoder().encode(json);
      
      if (uri) {
        await vscode.workspace.fs.writeFile(uri, content);
      } else {
        // Show save dialog
        const saveUri = await vscode.window.showSaveDialog({
          defaultUri: vscode.Uri.file(`${storyboard.getName()}.story`),
          filters: {
            'Storyboard': ['story'],
            'JSON': ['json']
          }
        });
        
        if (saveUri) {
          await vscode.workspace.fs.writeFile(saveUri, content);
        }
      }
      
      storyboard.markClean();
      this.emit('storyboard-saved', storyboardId);
      
    } catch (error) {
      this.logger.error('Failed to save storyboard', error);
      throw error;
    }
  }
  
  /**
   * Render storyboard as diagram
   */
  async renderStoryboard(storyboardId: string): Promise<string> {
    const storyboard = this.storyboards.get(storyboardId);
    if (!storyboard) {
      throw new Error(`Storyboard not found: ${storyboardId}`);
    }
    
    return this.mermaidRenderer.renderStoryboard(storyboard.getStoryboard());
  }
  
  /**
   * Analyze project components
   */
  async analyzeProject(rootPath?: string): Promise<ComponentAnalysis> {
    const workspaceRoot = rootPath || 
      vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    
    if (!workspaceRoot) {
      throw new Error('No workspace folder open');
    }
    
    const analysis = await this.componentAnalyzer.analyzeProject(workspaceRoot);
    
    this.emit('analysis-completed', analysis);
    
    return analysis;
  }
  
  /**
   * Generate storyboard from component analysis
   */
  async generateFromAnalysis(
    analysis: ComponentAnalysis,
    type: StoryboardType = StoryboardType.ComponentHierarchy
  ): Promise<StoryboardModel> {
    const storyboard = await this.createStoryboard(
      'Generated Component Storyboard',
      type
    );
    
    // Create scenes for components
    const sceneMap = new Map<string, Scene>();
    const positions = this.calculateComponentPositions(analysis.components);
    
    analysis.components.forEach((component, index) => {
      const position = positions[index] || { x: 100, y: 100 };
      const scene = storyboard.addScene(
        SceneType.Component,
        component.name,
        position,
        {
          componentPath: component.path,
          props: component.props,
          documentation: `${component.type} component`
        }
      );
      
      sceneMap.set(component.name, scene);
    });
    
    // Create flows for relationships
    analysis.relationships.forEach(rel => {
      const sourceScene = sceneMap.get(rel.parent);
      const targetScene = sceneMap.get(rel.child);
      
      if (sourceScene && targetScene) {
        storyboard.addFlow(
          TransitionType.Navigation,
          sourceScene.id,
          targetScene.id,
          rel.type,
          {
            trigger: 'render'
          }
        );
      }
    });
    
    // Auto-layout
    storyboard.autoLayout('dagre');
    
    return storyboard;
  }
  
  /**
   * Generate code from storyboard
   */
  async generateCode(
    storyboardId: string,
    options: {
      framework?: 'react' | 'vue' | 'angular';
      generateRoutes?: boolean;
      generateComponents?: boolean;
      generateState?: boolean;
    } = {}
  ): Promise<Map<string, string>> {
    const storyboard = this.storyboards.get(storyboardId);
    if (!storyboard) {
      throw new Error(`Storyboard not found: ${storyboardId}`);
    }
    
    const files = new Map<string, string>();
    const framework = options.framework || 'react';
    
    // Generate routes
    if (options.generateRoutes !== false) {
      const routesCode = this.generateRoutes(storyboard.getStoryboard(), framework);
      files.set('routes.js', routesCode);
    }
    
    // Generate component scaffolds
    if (options.generateComponents !== false) {
      const components = this.generateComponents(storyboard.getStoryboard(), framework);
      components.forEach((code, name) => {
        files.set(`components/${name}.js`, code);
      });
    }
    
    // Generate state management
    if (options.generateState !== false) {
      const stateCode = this.generateStateManagement(storyboard.getStoryboard(), framework);
      files.set('store/index.js', stateCode);
    }
    
    this.emit('code-generated', { storyboardId, files });
    
    return files;
  }
  
  /**
   * Export storyboard
   */
  async exportStoryboard(
    storyboardId: string,
    options: ExportOptions
  ): Promise<ExportResult> {
    const storyboard = this.storyboards.get(storyboardId);
    if (!storyboard) {
      throw new Error(`Storyboard not found: ${storyboardId}`);
    }
    
    const storyboardData = storyboard.getStoryboard();
    let content: string | Buffer;
    let mimeType: string;
    let filename: string;
    
    switch (options.format) {
      case 'markdown':
        content = await this.exportAsMarkdown(storyboardData);
        mimeType = 'text/markdown';
        filename = `${storyboardData.name}.md`;
        break;
        
      case 'html':
        content = await this.exportAsHTML(storyboardData);
        mimeType = 'text/html';
        filename = `${storyboardData.name}.html`;
        break;
        
      case 'png':
      case 'svg':
        const svg = await this.renderStoryboard(storyboardId);
        if (options.format === 'png') {
          content = await this.mermaidRenderer.exportAsImage(svg, 'png', options.scale);
          mimeType = 'image/png';
          filename = `${storyboardData.name}.png`;
        } else {
          content = svg;
          mimeType = 'image/svg+xml';
          filename = `${storyboardData.name}.svg`;
        }
        break;
        
      case 'json':
        content = storyboard.toJSON();
        mimeType = 'application/json';
        filename = `${storyboardData.name}.json`;
        break;
        
      default:
        throw new Error(`Unsupported export format: ${options.format}`);
    }
    
    const result: ExportResult = {
      format: options.format,
      content,
      filename,
      mimeType,
      size: Buffer.isBuffer(content) ? content.length : Buffer.byteLength(content)
    };
    
    this.emit('export-completed', result);
    
    return result;
  }
  
  /**
   * Get active storyboard
   */
  getActiveStoryboard(): StoryboardModel | undefined {
    return this.activeStoryboard;
  }
  
  /**
   * Set active storyboard
   */
  setActiveStoryboard(storyboardId: string): void {
    const storyboard = this.storyboards.get(storyboardId);
    if (!storyboard) {
      throw new Error(`Storyboard not found: ${storyboardId}`);
    }
    
    this.activeStoryboard = storyboard;
    this.emit('active-storyboard-changed', storyboardId);
  }
  
  /**
   * Get all storyboards
   */
  getAllStoryboards(): StoryboardModel[] {
    return Array.from(this.storyboards.values());
  }
  
  /**
   * Dispose controller
   */
  dispose(): void {
    // Dispose all storyboards
    this.storyboards.forEach(storyboard => {
      storyboard.removeAllListeners();
    });
    this.storyboards.clear();
    
    // Dispose VS Code resources
    this.disposables.forEach(d => d.dispose());
    this.disposables = [];
    
    // Remove listeners
    this.removeAllListeners();
  }
  
  // Private methods
  
  private registerCommands(): void {
    // Create new storyboard
    const createCommand = vscode.commands.registerCommand(
      'symbiote.storyboard.new',
      async () => {
        const name = await vscode.window.showInputBox({
          prompt: 'Enter storyboard name',
          value: 'User Flow'
        });
        
        if (name) {
          const type = await vscode.window.showQuickPick([
            { label: 'User Flow', value: StoryboardType.UserFlow },
            { label: 'Component Hierarchy', value: StoryboardType.ComponentHierarchy },
            { label: 'State Machine', value: StoryboardType.StateMachine },
            { label: 'API Sequence', value: StoryboardType.APISequence },
            { label: 'Data Flow', value: StoryboardType.DataFlow }
          ], {
            placeHolder: 'Select storyboard type'
          });
          
          if (type) {
            await this.createStoryboard(name, type.value);
            vscode.window.showInformationMessage(`Created storyboard: ${name}`);
          }
        }
      }
    );
    
    // Analyze project
    const analyzeCommand = vscode.commands.registerCommand(
      'symbiote.storyboard.analyze',
      async () => {
        await vscode.window.withProgress({
          location: vscode.ProgressLocation.Notification,
          title: 'Analyzing project components...',
          cancellable: false
        }, async () => {
          const analysis = await this.analyzeProject();
          
          // Show results
          const action = await vscode.window.showInformationMessage(
            `Found ${analysis.components.length} components`,
            'Generate Storyboard',
            'View Details'
          );
          
          if (action === 'Generate Storyboard') {
            await this.generateFromAnalysis(analysis);
          }
        });
      }
    );
    
    // Export storyboard
    const exportCommand = vscode.commands.registerCommand(
      'symbiote.storyboard.export',
      async () => {
        if (!this.activeStoryboard) {
          vscode.window.showErrorMessage('No active storyboard');
          return;
        }
        
        const format = await vscode.window.showQuickPick([
          { label: 'Markdown', value: 'markdown' },
          { label: 'HTML', value: 'html' },
          { label: 'PNG Image', value: 'png' },
          { label: 'SVG Image', value: 'svg' },
          { label: 'JSON', value: 'json' }
        ], {
          placeHolder: 'Select export format'
        });
        
        if (format) {
          const result = await this.exportStoryboard(
            this.activeStoryboard.getId(),
            { format: format.value as any }
          );
          
          // Save file
          const uri = await vscode.window.showSaveDialog({
            defaultUri: vscode.Uri.file(result.filename),
            filters: {
              'All Files': ['*']
            }
          });
          
          if (uri) {
            const content = Buffer.isBuffer(result.content) ?
              result.content :
              Buffer.from(result.content);
            
            await vscode.workspace.fs.writeFile(uri, content);
            vscode.window.showInformationMessage('Storyboard exported successfully');
          }
        }
      }
    );
    
    this.disposables.push(createCommand, analyzeCommand, exportCommand);
  }
  
  private setupEventListeners(): void {
    // Mermaid renderer events
    this.mermaidRenderer.on('diagram-rendered', (data) => {
      this.emit('diagram-rendered', data);
    });
  }
  
  private setupStoryboardListeners(storyboard: StoryboardModel): void {
    storyboard.on('scene-added', (scene) => {
      this.emit('scene-added', { storyboardId: storyboard.getId(), scene });
    });
    
    storyboard.on('scene-updated', (scene) => {
      this.emit('scene-updated', { storyboardId: storyboard.getId(), scene });
    });
    
    storyboard.on('flow-added', (flow) => {
      this.emit('flow-added', { storyboardId: storyboard.getId(), flow });
    });
    
    storyboard.on('dirty-state-changed', (isDirty) => {
      this.emit('storyboard-dirty-state-changed', {
        storyboardId: storyboard.getId(),
        isDirty
      });
    });
  }
  
  private calculateComponentPositions(components: any[]): Position[] {
    const positions: Position[] = [];
    const cols = Math.ceil(Math.sqrt(components.length));
    const spacing = 250;
    const startX = 100;
    const startY = 100;
    
    components.forEach((_, index) => {
      const row = Math.floor(index / cols);
      const col = index % cols;
      
      positions.push({
        x: startX + col * spacing,
        y: startY + row * spacing
      });
    });
    
    return positions;
  }
  
  private generateRoutes(storyboard: Storyboard, framework: string): string {
    const routes: string[] = [];
    
    // Extract screen scenes
    const screens = storyboard.scenes.filter(s => s.type === SceneType.Screen);
    
    if (framework === 'react') {
      routes.push('import { Routes, Route } from "react-router-dom";');
      routes.push('');
      
      // Import components
      screens.forEach(screen => {
        const componentName = screen.data.component || screen.name;
        routes.push(`import ${componentName} from './components/${componentName}';`);
      });
      
      routes.push('');
      routes.push('export default function AppRoutes() {');
      routes.push('  return (');
      routes.push('    <Routes>');
      
      screens.forEach(screen => {
        const path = screen.data.route || `/${screen.name.toLowerCase()}`;
        const component = screen.data.component || screen.name;
        routes.push(`      <Route path="${path}" element={<${component} />} />`);
      });
      
      routes.push('    </Routes>');
      routes.push('  );');
      routes.push('}');
    }
    
    return routes.join('\n');
  }
  
  private generateComponents(
    storyboard: Storyboard,
    framework: string
  ): Map<string, string> {
    const components = new Map<string, string>();
    
    const componentScenes = storyboard.scenes.filter(
      s => s.type === SceneType.Component || s.type === SceneType.Screen
    );
    
    componentScenes.forEach(scene => {
      const componentName = scene.data.component || scene.name;
      let code = '';
      
      if (framework === 'react') {
        code = `import React from 'react';

export default function ${componentName}() {
  return (
    <div>
      <h1>${scene.name}</h1>
      ${scene.description ? `<p>${scene.description}</p>` : ''}
    </div>
  );
}`;
      }
      
      components.set(componentName, code);
    });
    
    return components;
  }
  
  private generateStateManagement(
    storyboard: Storyboard,
    framework: string
  ): string {
    const stateScenes = storyboard.scenes.filter(s => s.type === SceneType.State);
    
    if (framework === 'react' && stateScenes.length > 0) {
      const lines = [
        'import { createContext, useContext, useReducer } from "react";',
        '',
        '// Initial state',
        'const initialState = {'
      ];
      
      stateScenes.forEach(scene => {
        const stateName = scene.data.stateName || scene.name;
        const initialValue = scene.data.initialValue || 'null';
        lines.push(`  ${stateName}: ${JSON.stringify(initialValue)},`);
      });
      
      lines.push('};');
      lines.push('');
      lines.push('// Actions');
      
      const actions: string[] = [];
      storyboard.flows.forEach(flow => {
        if (flow.type === TransitionType.StateChange && flow.data?.mutation) {
          actions.push(flow.data.mutation);
        }
      });
      
      actions.forEach(action => {
        lines.push(`const ${action.toUpperCase()} = '${action.toUpperCase()}';`);
      });
      
      lines.push('');
      lines.push('// Reducer');
      lines.push('function reducer(state, action) {');
      lines.push('  switch (action.type) {');
      
      actions.forEach(action => {
        lines.push(`    case ${action.toUpperCase()}:`);
        lines.push('      return { ...state, /* update state */ };');
      });
      
      lines.push('    default:');
      lines.push('      return state;');
      lines.push('  }');
      lines.push('}');
      
      return lines.join('\n');
    }
    
    return '// State management code';
  }
  
  private async exportAsMarkdown(storyboard: Storyboard): Promise<string> {
    const lines: string[] = [
      `# ${storyboard.name}`,
      '',
      storyboard.description || 'Application storyboard',
      '',
      '## Diagram',
      '',
      '```mermaid',
      await this.mermaidRenderer.renderStoryboard(storyboard),
      '```',
      '',
      '## Scenes',
      ''
    ];
    
    storyboard.scenes.forEach(scene => {
      lines.push(`### ${scene.name}`);
      lines.push('');
      if (scene.description) {
        lines.push(scene.description);
        lines.push('');
      }
      lines.push(`- **Type**: ${scene.type}`);
      if (scene.data.route) {
        lines.push(`- **Route**: ${scene.data.route}`);
      }
      if (scene.data.component) {
        lines.push(`- **Component**: ${scene.data.component}`);
      }
      lines.push('');
    });
    
    return lines.join('\n');
  }
  
  private async exportAsHTML(storyboard: Storyboard): Promise<string> {
    const markdown = await this.exportAsMarkdown(storyboard);
    
    return `<!DOCTYPE html>
<html>
<head>
  <title>${storyboard.name}</title>
  <style>
    body { 
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      max-width: 1200px;
      margin: 0 auto;
      padding: 20px;
    }
    h1, h2, h3 { color: #333; }
    code { background: #f4f4f4; padding: 2px 4px; }
    pre { background: #f4f4f4; padding: 16px; overflow-x: auto; }
    .mermaid { text-align: center; }
  </style>
  <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
  <script>mermaid.initialize({ startOnLoad: true });</script>
</head>
<body>
  ${this.markdownToHTML(markdown)}
</body>
</html>`;
  }
  
  private markdownToHTML(markdown: string): string {
    // Simple markdown to HTML conversion
    return markdown
      .replace(/^# (.+)$/gm, '<h1>$1</h1>')
      .replace(/^## (.+)$/gm, '<h2>$1</h2>')
      .replace(/^### (.+)$/gm, '<h3>$1</h3>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\n\n/g, '</p><p>')
      .replace(/^```mermaid\n([\s\S]+?)\n```$/gm, '<div class="mermaid">$1</div>')
      .replace(/^```(\w+)?\n([\s\S]+?)\n```$/gm, '<pre><code>$2</code></pre>')
      .replace(/^- (.+)$/gm, '<li>$1</li>')
      .replace(/(<li>[\s\S]+?<\/li>)/g, '<ul>$1</ul>')
      .replace(/<p>$/g, '<p>');
  }
}