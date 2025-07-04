/**
 * Graph Renderer with WebGL Acceleration
 * 
 * High-performance rendering for large code graphs
 */

import { EventEmitter } from 'events';
import {
  VisualizationNode,
  VisualizationEdge,
  ViewportState,
  VisualizationTheme,
  PerformanceConfig
} from './graph-visualization-engine';

export interface RenderContext {
  canvas: HTMLCanvasElement;
  gl?: WebGLRenderingContext;
  ctx?: CanvasRenderingContext2D;
  width: number;
  height: number;
  pixelRatio: number;
}

export interface RenderStats {
  fps: number;
  renderTime: number;
  nodeCount: number;
  edgeCount: number;
  drawCalls: number;
}

export interface LODLevel {
  minZoom: number;
  maxZoom: number;
  renderNodes: boolean;
  renderEdges: boolean;
  renderLabels: boolean;
  renderDetails: boolean;
  nodeSize: 'full' | 'reduced' | 'point';
  edgeStyle: 'full' | 'simple' | 'hidden';
}

/**
 * Abstract base renderer
 */
export abstract class GraphRenderer extends EventEmitter {
  protected context: RenderContext;
  protected theme: VisualizationTheme;
  protected performanceConfig: PerformanceConfig;
  protected viewport: ViewportState;
  protected stats: RenderStats;
  protected frameId?: number;
  protected lastFrameTime: number = 0;
  
  constructor(
    canvas: HTMLCanvasElement,
    theme: VisualizationTheme,
    performanceConfig: PerformanceConfig
  ) {
    super();
    
    this.theme = theme;
    this.performanceConfig = performanceConfig;
    this.viewport = {
      zoom: 1,
      pan: { x: 0, y: 0 },
      dimensions: { width: canvas.width, height: canvas.height }
    };
    
    this.context = this.createContext(canvas);
    this.stats = {
      fps: 0,
      renderTime: 0,
      nodeCount: 0,
      edgeCount: 0,
      drawCalls: 0
    };
  }
  
  abstract createContext(canvas: HTMLCanvasElement): RenderContext;
  abstract render(nodes: VisualizationNode[], edges: VisualizationEdge[]): void;
  abstract clear(): void;
  abstract dispose(): void;
  
  /**
   * Update viewport
   */
  updateViewport(viewport: ViewportState): void {
    this.viewport = viewport;
  }
  
  /**
   * Get current LOD level based on zoom
   */
  protected getLODLevel(): LODLevel {
    const zoom = this.viewport.zoom;
    
    if (zoom < 0.3) {
      return {
        minZoom: 0,
        maxZoom: 0.3,
        renderNodes: true,
        renderEdges: false,
        renderLabels: false,
        renderDetails: false,
        nodeSize: 'point',
        edgeStyle: 'hidden'
      };
    } else if (zoom < 0.6) {
      return {
        minZoom: 0.3,
        maxZoom: 0.6,
        renderNodes: true,
        renderEdges: true,
        renderLabels: false,
        renderDetails: false,
        nodeSize: 'reduced',
        edgeStyle: 'simple'
      };
    } else if (zoom < 1.5) {
      return {
        minZoom: 0.6,
        maxZoom: 1.5,
        renderNodes: true,
        renderEdges: true,
        renderLabels: true,
        renderDetails: false,
        nodeSize: 'full',
        edgeStyle: 'full'
      };
    } else {
      return {
        minZoom: 1.5,
        maxZoom: Infinity,
        renderNodes: true,
        renderEdges: true,
        renderLabels: true,
        renderDetails: true,
        nodeSize: 'full',
        edgeStyle: 'full'
      };
    }
  }
  
  /**
   * Transform world coordinates to screen coordinates
   */
  protected worldToScreen(x: number, y: number): { x: number; y: number } {
    return {
      x: (x + this.viewport.pan.x) * this.viewport.zoom,
      y: (y + this.viewport.pan.y) * this.viewport.zoom
    };
  }
  
  /**
   * Transform screen coordinates to world coordinates
   */
  protected screenToWorld(x: number, y: number): { x: number; y: number } {
    return {
      x: x / this.viewport.zoom - this.viewport.pan.x,
      y: y / this.viewport.zoom - this.viewport.pan.y
    };
  }
  
  /**
   * Check if point is in viewport
   */
  protected isInViewport(x: number, y: number, padding: number = 50): boolean {
    const screen = this.worldToScreen(x, y);
    return (
      screen.x >= -padding &&
      screen.x <= this.context.width + padding &&
      screen.y >= -padding &&
      screen.y <= this.context.height + padding
    );
  }
  
  /**
   * Update render statistics
   */
  protected updateStats(nodeCount: number, edgeCount: number, drawCalls: number): void {
    const now = performance.now();
    const delta = now - this.lastFrameTime;
    
    this.stats = {
      fps: 1000 / delta,
      renderTime: delta,
      nodeCount,
      edgeCount,
      drawCalls
    };
    
    this.lastFrameTime = now;
    this.emit('statsUpdate', this.stats);
  }
}

/**
 * Canvas 2D Renderer
 */
export class Canvas2DRenderer extends GraphRenderer {
  private ctx: CanvasRenderingContext2D;
  
  createContext(canvas: HTMLCanvasElement): RenderContext {
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to create 2D context');
    }
    
    this.ctx = ctx;
    
    const pixelRatio = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    canvas.width = width * pixelRatio;
    canvas.height = height * pixelRatio;
    
    ctx.scale(pixelRatio, pixelRatio);
    
    return {
      canvas,
      ctx,
      width,
      height,
      pixelRatio
    };
  }
  
  render(nodes: VisualizationNode[], edges: VisualizationEdge[]): void {
    const startTime = performance.now();
    let drawCalls = 0;
    
    this.clear();
    
    const lod = this.getLODLevel();
    
    // Set up canvas state
    this.ctx.save();
    this.ctx.translate(this.viewport.pan.x, this.viewport.pan.y);
    this.ctx.scale(this.viewport.zoom, this.viewport.zoom);
    
    // Render edges first (behind nodes)
    if (lod.renderEdges) {
      drawCalls += this.renderEdges(edges, lod);
    }
    
    // Render nodes
    if (lod.renderNodes) {
      drawCalls += this.renderNodes(nodes, lod);
    }
    
    this.ctx.restore();
    
    const visibleNodes = nodes.filter(n => n.visible).length;
    const visibleEdges = edges.filter(e => e.visible).length;
    this.updateStats(visibleNodes, visibleEdges, drawCalls);
  }
  
  private renderEdges(edges: VisualizationEdge[], lod: LODLevel): number {
    let drawCalls = 0;
    
    edges.forEach(edge => {
      if (!edge.visible) return;
      
      const source = edge.source;
      const target = edge.target;
      
      // TODO: Get actual positions from nodes
      // For now, we'll skip the actual rendering
      
      drawCalls++;
    });
    
    return drawCalls;
  }
  
  private renderNodes(nodes: VisualizationNode[], lod: LODLevel): number {
    let drawCalls = 0;
    
    nodes.forEach(node => {
      if (!node.visible || !node.x || !node.y) return;
      if (!this.isInViewport(node.x, node.y)) return;
      
      const screen = this.worldToScreen(node.x, node.y);
      
      // Draw node based on LOD
      switch (lod.nodeSize) {
        case 'point':
          this.renderNodePoint(node, screen);
          break;
        case 'reduced':
          this.renderNodeReduced(node, screen);
          break;
        case 'full':
          this.renderNodeFull(node, screen, lod);
          break;
      }
      
      drawCalls++;
    });
    
    return drawCalls;
  }
  
  private renderNodePoint(node: VisualizationNode, screen: { x: number; y: number }): void {
    this.ctx.fillStyle = node.color || this.theme.nodeColors[node.type];
    this.ctx.fillRect(screen.x - 2, screen.y - 2, 4, 4);
  }
  
  private renderNodeReduced(node: VisualizationNode, screen: { x: number; y: number }): void {
    const size = (node.size || 20) / 2;
    
    this.ctx.fillStyle = node.color || this.theme.nodeColors[node.type];
    this.ctx.beginPath();
    this.ctx.arc(screen.x, screen.y, size, 0, Math.PI * 2);
    this.ctx.fill();
  }
  
  private renderNodeFull(
    node: VisualizationNode, 
    screen: { x: number; y: number },
    lod: LODLevel
  ): void {
    const size = node.size || 20;
    
    // Draw node circle
    this.ctx.fillStyle = node.color || this.theme.nodeColors[node.type];
    this.ctx.strokeStyle = this.theme.font.color;
    this.ctx.lineWidth = 2;
    
    this.ctx.beginPath();
    this.ctx.arc(screen.x, screen.y, size, 0, Math.PI * 2);
    this.ctx.fill();
    this.ctx.stroke();
    
    // Draw icon
    if (node.icon && lod.renderDetails) {
      this.ctx.font = `${size}px Arial`;
      this.ctx.textAlign = 'center';
      this.ctx.textBaseline = 'middle';
      this.ctx.fillStyle = this.theme.font.color;
      this.ctx.fillText(node.icon, screen.x, screen.y);
    }
    
    // Draw label
    if (lod.renderLabels) {
      this.ctx.font = `${this.theme.font.size}px ${this.theme.font.family}`;
      this.ctx.textAlign = 'center';
      this.ctx.textBaseline = 'top';
      this.ctx.fillStyle = this.theme.font.color;
      
      // Background for better readability
      const metrics = this.ctx.measureText(node.label);
      const padding = 4;
      
      this.ctx.fillStyle = this.theme.backgroundColor + 'CC'; // Semi-transparent
      this.ctx.fillRect(
        screen.x - metrics.width / 2 - padding,
        screen.y + size + 5,
        metrics.width + padding * 2,
        this.theme.font.size + padding * 2
      );
      
      this.ctx.fillStyle = this.theme.font.color;
      this.ctx.fillText(node.label, screen.x, screen.y + size + 5 + padding);
    }
  }
  
  clear(): void {
    this.ctx.clearRect(0, 0, this.context.width, this.context.height);
    this.ctx.fillStyle = this.theme.backgroundColor;
    this.ctx.fillRect(0, 0, this.context.width, this.context.height);
  }
  
  dispose(): void {
    // Clean up any resources
  }
}

/**
 * WebGL Renderer for high-performance rendering
 */
export class WebGLRenderer extends GraphRenderer {
  private gl: WebGLRenderingContext;
  private shaderProgram?: WebGLProgram;
  private buffers: {
    nodePositions?: WebGLBuffer;
    nodeColors?: WebGLBuffer;
    nodeSizes?: WebGLBuffer;
    edgePositions?: WebGLBuffer;
    edgeColors?: WebGLBuffer;
  } = {};
  
  createContext(canvas: HTMLCanvasElement): RenderContext {
    const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (!gl) {
      throw new Error('WebGL not supported');
    }
    
    this.gl = gl as WebGLRenderingContext;
    this.initShaders();
    
    const pixelRatio = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    canvas.width = width * pixelRatio;
    canvas.height = height * pixelRatio;
    
    gl.viewport(0, 0, canvas.width, canvas.height);
    
    return {
      canvas,
      gl: this.gl,
      width,
      height,
      pixelRatio
    };
  }
  
  private initShaders(): void {
    // Vertex shader
    const vertexShaderSource = `
      attribute vec2 a_position;
      attribute vec3 a_color;
      attribute float a_size;
      
      uniform mat3 u_matrix;
      
      varying vec3 v_color;
      
      void main() {
        vec2 position = (u_matrix * vec3(a_position, 1)).xy;
        gl_Position = vec4(position, 0, 1);
        gl_PointSize = a_size;
        v_color = a_color;
      }
    `;
    
    // Fragment shader
    const fragmentShaderSource = `
      precision mediump float;
      
      varying vec3 v_color;
      
      void main() {
        vec2 coord = gl_PointCoord - vec2(0.5);
        if (length(coord) > 0.5) {
          discard;
        }
        gl_FragColor = vec4(v_color, 1.0);
      }
    `;
    
    const vertexShader = this.createShader(this.gl.VERTEX_SHADER, vertexShaderSource);
    const fragmentShader = this.createShader(this.gl.FRAGMENT_SHADER, fragmentShaderSource);
    
    if (!vertexShader || !fragmentShader) {
      throw new Error('Failed to create shaders');
    }
    
    this.shaderProgram = this.createProgram(vertexShader, fragmentShader);
  }
  
  private createShader(type: number, source: string): WebGLShader | null {
    const shader = this.gl.createShader(type);
    if (!shader) return null;
    
    this.gl.shaderSource(shader, source);
    this.gl.compileShader(shader);
    
    if (!this.gl.getShaderParameter(shader, this.gl.COMPILE_STATUS)) {
      console.error('Shader compilation error:', this.gl.getShaderInfoLog(shader));
      this.gl.deleteShader(shader);
      return null;
    }
    
    return shader;
  }
  
  private createProgram(vertexShader: WebGLShader, fragmentShader: WebGLShader): WebGLProgram {
    const program = this.gl.createProgram();
    if (!program) {
      throw new Error('Failed to create program');
    }
    
    this.gl.attachShader(program, vertexShader);
    this.gl.attachShader(program, fragmentShader);
    this.gl.linkProgram(program);
    
    if (!this.gl.getProgramParameter(program, this.gl.LINK_STATUS)) {
      throw new Error('Program linking error: ' + this.gl.getProgramInfoLog(program));
    }
    
    return program;
  }
  
  render(nodes: VisualizationNode[], edges: VisualizationEdge[]): void {
    const startTime = performance.now();
    let drawCalls = 0;
    
    this.clear();
    
    if (!this.shaderProgram) return;
    
    this.gl.useProgram(this.shaderProgram);
    
    // Set up transformation matrix
    const matrix = this.createTransformMatrix();
    const matrixLocation = this.gl.getUniformLocation(this.shaderProgram, 'u_matrix');
    this.gl.uniformMatrix3fv(matrixLocation, false, matrix);
    
    // Render edges
    drawCalls += this.renderEdges(edges);
    
    // Render nodes
    drawCalls += this.renderNodes(nodes);
    
    const visibleNodes = nodes.filter(n => n.visible).length;
    const visibleEdges = edges.filter(e => e.visible).length;
    this.updateStats(visibleNodes, visibleEdges, drawCalls);
  }
  
  private createTransformMatrix(): Float32Array {
    // Create transformation matrix for viewport
    const scale = this.viewport.zoom;
    const dx = this.viewport.pan.x;
    const dy = this.viewport.pan.y;
    
    // Convert to clip space (-1 to 1)
    const scaleX = scale * 2 / this.context.width;
    const scaleY = scale * 2 / this.context.height;
    
    return new Float32Array([
      scaleX, 0, 0,
      0, -scaleY, 0,
      dx * scaleX - 1, dy * scaleY + 1, 1
    ]);
  }
  
  private renderNodes(nodes: VisualizationNode[]): number {
    if (!this.shaderProgram) return 0;
    
    const visibleNodes = nodes.filter(n => n.visible && n.x !== undefined && n.y !== undefined);
    if (visibleNodes.length === 0) return 0;
    
    // Prepare data
    const positions = new Float32Array(visibleNodes.length * 2);
    const colors = new Float32Array(visibleNodes.length * 3);
    const sizes = new Float32Array(visibleNodes.length);
    
    visibleNodes.forEach((node, i) => {
      positions[i * 2] = node.x!;
      positions[i * 2 + 1] = node.y!;
      
      const color = this.hexToRgb(node.color || this.theme.nodeColors[node.type]);
      colors[i * 3] = color.r;
      colors[i * 3 + 1] = color.g;
      colors[i * 3 + 2] = color.b;
      
      sizes[i] = (node.size || 20) * this.viewport.zoom;
    });
    
    // Create/update buffers
    this.updateBuffer('nodePositions', positions);
    this.updateBuffer('nodeColors', colors);
    this.updateBuffer('nodeSizes', sizes);
    
    // Set up attributes
    this.setupAttribute('a_position', this.buffers.nodePositions!, 2);
    this.setupAttribute('a_color', this.buffers.nodeColors!, 3);
    this.setupAttribute('a_size', this.buffers.nodeSizes!, 1);
    
    // Draw
    this.gl.drawArrays(this.gl.POINTS, 0, visibleNodes.length);
    
    return 1;
  }
  
  private renderEdges(edges: VisualizationEdge[]): number {
    // TODO: Implement edge rendering
    return 0;
  }
  
  private updateBuffer(name: keyof typeof this.buffers, data: Float32Array): void {
    if (!this.buffers[name]) {
      this.buffers[name] = this.gl.createBuffer()!;
    }
    
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.buffers[name]!);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, data, this.gl.DYNAMIC_DRAW);
  }
  
  private setupAttribute(name: string, buffer: WebGLBuffer, size: number): void {
    if (!this.shaderProgram) return;
    
    const location = this.gl.getAttribLocation(this.shaderProgram, name);
    if (location === -1) return;
    
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, buffer);
    this.gl.enableVertexAttribArray(location);
    this.gl.vertexAttribPointer(location, size, this.gl.FLOAT, false, 0, 0);
  }
  
  private hexToRgb(hex: string): { r: number; g: number; b: number } {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
      r: parseInt(result[1], 16) / 255,
      g: parseInt(result[2], 16) / 255,
      b: parseInt(result[3], 16) / 255
    } : { r: 0, g: 0, b: 0 };
  }
  
  clear(): void {
    const color = this.hexToRgb(this.theme.backgroundColor);
    this.gl.clearColor(color.r, color.g, color.b, 1);
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);
  }
  
  dispose(): void {
    // Clean up WebGL resources
    Object.values(this.buffers).forEach(buffer => {
      if (buffer) this.gl.deleteBuffer(buffer);
    });
    
    if (this.shaderProgram) {
      this.gl.deleteProgram(this.shaderProgram);
    }
  }
}

/**
 * Renderer factory
 */
export class RendererFactory {
  static create(
    canvas: HTMLCanvasElement,
    theme: VisualizationTheme,
    performanceConfig: PerformanceConfig
  ): GraphRenderer {
    if (performanceConfig.enableWebGL) {
      try {
        return new WebGLRenderer(canvas, theme, performanceConfig);
      } catch (error) {
        console.warn('WebGL not available, falling back to Canvas2D', error);
      }
    }
    
    return new Canvas2DRenderer(canvas, theme, performanceConfig);
  }
}