/**
 * Mermaid Renderer
 * 
 * Renders Mermaid diagrams for storyboards with VS Code theme integration
 */

import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import {
  MermaidDiagram,
  MermaidDiagramType,
  MermaidTheme,
  MermaidConfig,
  Storyboard,
  Scene,
  Flow,
  SceneType,
  TransitionType
} from '../types';

export interface MermaidRendererConfig {
  defaultTheme?: 'default' | 'dark' | 'forest' | 'neutral';
  enableInteractivity?: boolean;
  enableZoom?: boolean;
  enablePan?: boolean;
  maxWidth?: number;
  maxHeight?: number;
}

export class MermaidRenderer extends EventEmitter {
  private logger = new Logger('MermaidRenderer');
  private config: Required<MermaidRendererConfig>;
  private mermaidAPI: any;
  private initialized = false;
  
  constructor(config: MermaidRendererConfig = {}) {
    super();
    
    this.config = {
      defaultTheme: config.defaultTheme || 'default',
      enableInteractivity: config.enableInteractivity ?? true,
      enableZoom: config.enableZoom ?? true,
      enablePan: config.enablePan ?? true,
      maxWidth: config.maxWidth || 2000,
      maxHeight: config.maxHeight || 2000
    };
  }
  
  /**
   * Initialize Mermaid
   */
  async initialize(): Promise<void> {
    try {
      // In a real implementation, we would load Mermaid.js
      // For now, we'll simulate it
      this.mermaidAPI = {
        initialize: (config: any) => {
          this.logger.info('Mermaid initialized with config', config);
        },
        render: async (id: string, code: string) => {
          // Simulate rendering
          return {
            svg: `<svg>${code}</svg>`,
            bindFunctions: (element: any) => {}
          };
        }
      };
      
      // Initialize with VS Code theme
      const theme = this.getVSCodeTheme();
      this.mermaidAPI.initialize({
        startOnLoad: false,
        theme: this.config.defaultTheme,
        themeVariables: theme,
        flowchart: {
          curve: 'basis',
          padding: 15,
          nodeSpacing: 50,
          rankSpacing: 50
        },
        sequence: {
          diagramMarginX: 50,
          diagramMarginY: 10,
          boxTextMargin: 5,
          noteMargin: 10,
          messageMargin: 35
        }
      });
      
      this.initialized = true;
      this.emit('initialized');
      
    } catch (error) {
      this.logger.error('Failed to initialize Mermaid', error);
      throw error;
    }
  }
  
  /**
   * Render a storyboard as a Mermaid diagram
   */
  async renderStoryboard(storyboard: Storyboard): Promise<string> {
    if (!this.initialized) {
      await this.initialize();
    }
    
    let diagramCode: string;
    
    switch (storyboard.type) {
      case 'user-flow':
        diagramCode = this.generateUserFlowDiagram(storyboard);
        break;
        
      case 'component-hierarchy':
        diagramCode = this.generateComponentHierarchyDiagram(storyboard);
        break;
        
      case 'state-machine':
        diagramCode = this.generateStateMachineDiagram(storyboard);
        break;
        
      case 'api-sequence':
        diagramCode = this.generateAPISequenceDiagram(storyboard);
        break;
        
      case 'data-flow':
        diagramCode = this.generateDataFlowDiagram(storyboard);
        break;
        
      default:
        diagramCode = this.generateFlowchartDiagram(storyboard);
    }
    
    return this.renderDiagram({
      type: this.getDiagramType(storyboard.type),
      code: diagramCode
    });
  }
  
  /**
   * Render a Mermaid diagram
   */
  async renderDiagram(diagram: MermaidDiagram): Promise<string> {
    if (!this.initialized) {
      await this.initialize();
    }
    
    try {
      const id = `mermaid-${Date.now()}`;
      const result = await this.mermaidAPI.render(id, diagram.code);
      
      this.emit('diagram-rendered', { diagram, svg: result.svg });
      
      return result.svg;
      
    } catch (error) {
      this.logger.error('Failed to render diagram', error);
      throw error;
    }
  }
  
  /**
   * Generate user flow diagram
   */
  private generateUserFlowDiagram(storyboard: Storyboard): string {
    const lines: string[] = ['flowchart TD'];
    
    // Add scenes
    storyboard.scenes.forEach(scene => {
      const shape = this.getNodeShape(scene.type);
      const label = this.escapeLabel(scene.name);
      lines.push(`    ${scene.id}${shape.start}"${label}"${shape.end}`);
      
      // Add styling
      if (scene.style) {
        const style = this.generateNodeStyle(scene.style);
        lines.push(`    style ${scene.id} ${style}`);
      }
    });
    
    // Add flows
    storyboard.flows.forEach(flow => {
      const arrow = this.getArrowType(flow.type);
      const label = flow.label ? `|${this.escapeLabel(flow.label)}|` : '';
      lines.push(`    ${flow.source} ${arrow}${label} ${flow.target}`);
    });
    
    return lines.join('\n');
  }
  
  /**
   * Generate component hierarchy diagram
   */
  private generateComponentHierarchyDiagram(storyboard: Storyboard): string {
    const lines: string[] = ['graph TD'];
    
    // Group components by parent
    const hierarchy = this.buildComponentHierarchy(storyboard);
    
    // Render hierarchy
    this.renderHierarchy(lines, hierarchy, '    ');
    
    return lines.join('\n');
  }
  
  /**
   * Generate state machine diagram
   */
  private generateStateMachineDiagram(storyboard: Storyboard): string {
    const lines: string[] = ['stateDiagram-v2'];
    
    // Add states
    storyboard.scenes.forEach(scene => {
      if (scene.type === SceneType.State) {
        lines.push(`    ${scene.id}: ${scene.name}`);
        
        // Add state details
        if (scene.data.stateName) {
          lines.push(`    ${scene.id}: ${scene.data.stateName}`);
        }
      }
    });
    
    // Add transitions
    storyboard.flows.forEach(flow => {
      if (flow.type === TransitionType.StateChange) {
        const label = flow.label || flow.data?.mutation || 'transition';
        lines.push(`    ${flow.source} --> ${flow.target}: ${label}`);
      }
    });
    
    return lines.join('\n');
  }
  
  /**
   * Generate API sequence diagram
   */
  private generateAPISequenceDiagram(storyboard: Storyboard): string {
    const lines: string[] = ['sequenceDiagram'];
    
    // Identify participants
    const participants = new Set<string>();
    storyboard.scenes.forEach(scene => {
      if (scene.type === SceneType.Screen || scene.type === SceneType.API) {
        participants.add(scene.id);
      }
    });
    
    // Add participants
    participants.forEach(p => {
      const scene = storyboard.scenes.find(s => s.id === p);
      if (scene) {
        lines.push(`    participant ${p} as ${scene.name}`);
      }
    });
    
    // Add interactions
    storyboard.flows.forEach(flow => {
      if (flow.type === TransitionType.APICall) {
        const method = flow.data?.request?.method || 'GET';
        const endpoint = flow.data?.request?.endpoint || '/api';
        lines.push(`    ${flow.source}->>+${flow.target}: ${method} ${endpoint}`);
        
        // Add response
        if (flow.data?.response) {
          lines.push(`    ${flow.target}-->>-${flow.source}: Response`);
        }
      }
    });
    
    return lines.join('\n');
  }
  
  /**
   * Generate data flow diagram
   */
  private generateDataFlowDiagram(storyboard: Storyboard): string {
    const lines: string[] = ['graph LR'];
    
    // Add data sources and sinks
    storyboard.scenes.forEach(scene => {
      const shape = scene.type === SceneType.Component ? '(' : '[';
      const endShape = scene.type === SceneType.Component ? ')' : ']';
      lines.push(`    ${scene.id}${shape}${scene.name}${endShape}`);
    });
    
    // Add data flows
    storyboard.flows.forEach(flow => {
      if (flow.type === TransitionType.DataFlow) {
        const arrow = '-->';
        const label = flow.label ? `|${flow.label}|` : '';
        lines.push(`    ${flow.source} ${arrow}${label} ${flow.target}`);
      }
    });
    
    return lines.join('\n');
  }
  
  /**
   * Generate generic flowchart diagram
   */
  private generateFlowchartDiagram(storyboard: Storyboard): string {
    return this.generateUserFlowDiagram(storyboard);
  }
  
  /**
   * Get node shape based on scene type
   */
  private getNodeShape(type: SceneType): { start: string; end: string } {
    switch (type) {
      case SceneType.Screen:
        return { start: '[', end: ']' };
      case SceneType.Component:
        return { start: '(', end: ')' };
      case SceneType.State:
        return { start: '[[', end: ']]' };
      case SceneType.API:
        return { start: '[/', end: '/]' };
      case SceneType.Decision:
        return { start: '{', end: '}' };
      case SceneType.Process:
        return { start: '[(', end: ')]' };
      default:
        return { start: '[', end: ']' };
    }
  }
  
  /**
   * Get arrow type based on flow type
   */
  private getArrowType(type: TransitionType): string {
    switch (type) {
      case TransitionType.Navigation:
        return '-->';
      case TransitionType.Action:
        return '==>';
      case TransitionType.StateChange:
        return '-.->';
      case TransitionType.APICall:
        return '-->>>';
      case TransitionType.DataFlow:
        return '--->';
      case TransitionType.Conditional:
        return '-..->';
      default:
        return '-->';
    }
  }
  
  /**
   * Generate node style
   */
  private generateNodeStyle(style: any): string {
    const styles: string[] = [];
    
    if (style.backgroundColor) {
      styles.push(`fill:${style.backgroundColor}`);
    }
    if (style.borderColor) {
      styles.push(`stroke:${style.borderColor}`);
    }
    if (style.borderWidth) {
      styles.push(`stroke-width:${style.borderWidth}`);
    }
    if (style.textColor) {
      styles.push(`color:${style.textColor}`);
    }
    
    return styles.join(',');
  }
  
  /**
   * Escape label for Mermaid
   */
  private escapeLabel(label: string): string {
    return label
      .replace(/"/g, '\\"')
      .replace(/\n/g, '<br/>')
      .replace(/[<>]/g, '');
  }
  
  /**
   * Get VS Code theme colors
   */
  private getVSCodeTheme(): MermaidTheme {
    // In a real implementation, we would get these from VS Code
    return {
      primaryColor: '#007ACC',
      primaryTextColor: '#FFFFFF',
      primaryBorderColor: '#005A9E',
      secondaryColor: '#F0F0F0',
      tertiaryColor: '#E0E0E0',
      background: '#FFFFFF',
      mainBkg: '#FFFFFF',
      secondBkg: '#F5F5F5',
      tertiaryBkg: '#EBEBEB',
      lineColor: '#333333',
      textColor: '#333333',
      fontSize: '14px',
      fontFamily: 'Consolas, Monaco, monospace'
    };
  }
  
  /**
   * Get diagram type from storyboard type
   */
  private getDiagramType(type: string): MermaidDiagramType {
    switch (type) {
      case 'state-machine':
        return MermaidDiagramType.State;
      case 'api-sequence':
        return MermaidDiagramType.Sequence;
      case 'component-hierarchy':
        return MermaidDiagramType.Class;
      default:
        return MermaidDiagramType.Flowchart;
    }
  }
  
  /**
   * Build component hierarchy
   */
  private buildComponentHierarchy(storyboard: Storyboard): any {
    const hierarchy: any = {};
    
    // Build parent-child relationships
    storyboard.flows.forEach(flow => {
      if (flow.type === TransitionType.Navigation) {
        const parent = storyboard.scenes.find(s => s.id === flow.source);
        const child = storyboard.scenes.find(s => s.id === flow.target);
        
        if (parent && child) {
          if (!hierarchy[parent.id]) {
            hierarchy[parent.id] = {
              scene: parent,
              children: []
            };
          }
          hierarchy[parent.id].children.push(child);
        }
      }
    });
    
    return hierarchy;
  }
  
  /**
   * Render hierarchy recursively
   */
  private renderHierarchy(lines: string[], hierarchy: any, indent: string): void {
    Object.entries(hierarchy).forEach(([id, node]: [string, any]) => {
      const shape = this.getNodeShape(node.scene.type);
      lines.push(`${indent}${id}${shape.start}${node.scene.name}${shape.end}`);
      
      if (node.children.length > 0) {
        node.children.forEach((child: any) => {
          lines.push(`${indent}${id} --> ${child.id}`);
        });
      }
    });
  }
  
  /**
   * Export diagram as image
   */
  async exportAsImage(
    svg: string,
    format: 'png' | 'svg',
    scale: number = 2
  ): Promise<Buffer> {
    // In a real implementation, we would convert SVG to PNG if needed
    // For now, return the SVG as a buffer
    return Buffer.from(svg);
  }
  
  /**
   * Add click handlers to diagram
   */
  addInteractivity(svg: string, handlers: Map<string, () => void>): string {
    // In a real implementation, we would add click handlers to SVG elements
    return svg;
  }
}