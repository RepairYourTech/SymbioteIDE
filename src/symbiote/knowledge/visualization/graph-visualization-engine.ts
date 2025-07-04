/**
 * Graph Visualization Engine
 * 
 * Core engine for rendering and managing code graph visualizations
 */

import { EventEmitter } from 'events';
import { 
  GraphNode, 
  GraphRelationship, 
  NodeType, 
  RelationType,
  GraphQueryResult,
  BaseNodeProperties,
  ClassNodeProperties,
  FunctionNodeProperties,
  FileNodeProperties
} from '../types/graph-types';

export interface VisualizationNode {
  id: string;
  label: string;
  type: NodeType;
  properties: BaseNodeProperties;
  x?: number;
  y?: number;
  size?: number;
  color?: string;
  icon?: string;
  expanded?: boolean;
  visible?: boolean;
  metadata?: any;
}

export interface VisualizationEdge {
  id: string;
  source: string;
  target: string;
  type: RelationType;
  label?: string;
  color?: string;
  width?: number;
  style?: 'solid' | 'dashed' | 'dotted';
  animated?: boolean;
  metadata?: any;
}

export interface VisualizationConfig {
  layout: LayoutType;
  theme: VisualizationTheme;
  performance: PerformanceConfig;
  interaction: InteractionConfig;
  filters: FilterConfig;
}

export enum LayoutType {
  FORCE_DIRECTED = 'force-directed',
  HIERARCHICAL = 'hierarchical',
  RADIAL = 'radial',
  CIRCULAR = 'circular',
  GRID = 'grid',
  CONCENTRIC = 'concentric',
  DAGRE = 'dagre',
  COLA = 'cola'
}

export interface VisualizationTheme {
  nodeColors: Record<NodeType, string>;
  edgeColors: Record<RelationType, string>;
  backgroundColor: string;
  highlightColor: string;
  selectionColor: string;
  font: {
    family: string;
    size: number;
    color: string;
  };
}

export interface PerformanceConfig {
  enableWebGL: boolean;
  maxNodes: number;
  maxEdges: number;
  enableClustering: boolean;
  clusterThreshold: number;
  enableLOD: boolean; // Level of Detail
  lodThresholds: {
    high: number;
    medium: number;
    low: number;
  };
}

export interface InteractionConfig {
  enableZoom: boolean;
  enablePan: boolean;
  enableNodeDrag: boolean;
  enableBoxSelection: boolean;
  enableContextMenu: boolean;
  enableTooltips: boolean;
  enableKeyboardShortcuts: boolean;
}

export interface FilterConfig {
  nodeTypes: NodeType[];
  relationTypes: RelationType[];
  minNodeSize?: number;
  maxNodeSize?: number;
  searchQuery?: string;
  customFilters?: Array<(node: VisualizationNode) => boolean>;
}

export interface GraphStatistics {
  nodeCount: number;
  edgeCount: number;
  density: number;
  avgDegree: number;
  clusters: number;
  maxDepth: number;
}

export interface ViewportState {
  zoom: number;
  pan: { x: number; y: number };
  dimensions: { width: number; height: number };
}

export interface SelectionState {
  selectedNodes: Set<string>;
  selectedEdges: Set<string>;
  hoveredNode?: string;
  hoveredEdge?: string;
}

export class GraphVisualizationEngine extends EventEmitter {
  private nodes: Map<string, VisualizationNode> = new Map();
  private edges: Map<string, VisualizationEdge> = new Map();
  private config: VisualizationConfig;
  private viewport: ViewportState;
  private selection: SelectionState;
  private clusters: Map<string, Set<string>> = new Map();
  private nodeIndex: Map<string, Set<string>> = new Map(); // For fast lookups
  
  constructor(config: Partial<VisualizationConfig> = {}) {
    super();
    
    this.config = this.mergeConfig(config);
    this.viewport = {
      zoom: 1,
      pan: { x: 0, y: 0 },
      dimensions: { width: 1200, height: 800 }
    };
    this.selection = {
      selectedNodes: new Set(),
      selectedEdges: new Set()
    };
  }
  
  /**
   * Load graph data from query result
   */
  loadGraphData(result: GraphQueryResult): void {
    this.clear();
    
    // Add nodes
    result.nodes.forEach(node => {
      this.addNode(this.convertToVisualizationNode(node));
    });
    
    // Add edges
    result.relationships.forEach(rel => {
      this.addEdge(this.convertToVisualizationEdge(rel));
    });
    
    // Build indexes
    this.buildIndexes();
    
    // Apply initial layout
    this.applyLayout();
    
    // Emit data loaded event
    this.emit('dataLoaded', {
      nodeCount: this.nodes.size,
      edgeCount: this.edges.size
    });
  }
  
  /**
   * Add a node to the visualization
   */
  addNode(node: VisualizationNode): void {
    this.nodes.set(node.id, node);
    
    // Update node index
    if (!this.nodeIndex.has(node.type)) {
      this.nodeIndex.set(node.type, new Set());
    }
    this.nodeIndex.get(node.type)!.add(node.id);
    
    this.emit('nodeAdded', node);
  }
  
  /**
   * Add an edge to the visualization
   */
  addEdge(edge: VisualizationEdge): void {
    this.edges.set(edge.id, edge);
    this.emit('edgeAdded', edge);
  }
  
  /**
   * Remove a node and its connected edges
   */
  removeNode(nodeId: string): void {
    const node = this.nodes.get(nodeId);
    if (!node) return;
    
    // Remove connected edges
    const connectedEdges = this.getConnectedEdges(nodeId);
    connectedEdges.forEach(edge => this.removeEdge(edge.id));
    
    // Remove from indexes
    this.nodeIndex.get(node.type)?.delete(nodeId);
    
    // Remove node
    this.nodes.delete(nodeId);
    this.selection.selectedNodes.delete(nodeId);
    
    this.emit('nodeRemoved', nodeId);
  }
  
  /**
   * Remove an edge
   */
  removeEdge(edgeId: string): void {
    this.edges.delete(edgeId);
    this.selection.selectedEdges.delete(edgeId);
    this.emit('edgeRemoved', edgeId);
  }
  
  /**
   * Get connected edges for a node
   */
  getConnectedEdges(nodeId: string): VisualizationEdge[] {
    return Array.from(this.edges.values()).filter(edge => 
      edge.source === nodeId || edge.target === nodeId
    );
  }
  
  /**
   * Get neighbor nodes
   */
  getNeighbors(nodeId: string): VisualizationNode[] {
    const edges = this.getConnectedEdges(nodeId);
    const neighborIds = new Set<string>();
    
    edges.forEach(edge => {
      if (edge.source === nodeId) neighborIds.add(edge.target);
      if (edge.target === nodeId) neighborIds.add(edge.source);
    });
    
    return Array.from(neighborIds)
      .map(id => this.nodes.get(id))
      .filter((node): node is VisualizationNode => node !== undefined);
  }
  
  /**
   * Apply layout algorithm
   */
  applyLayout(layoutType?: LayoutType): void {
    const layout = layoutType || this.config.layout;
    
    switch (layout) {
      case LayoutType.FORCE_DIRECTED:
        this.applyForceDirectedLayout();
        break;
      case LayoutType.HIERARCHICAL:
        this.applyHierarchicalLayout();
        break;
      case LayoutType.RADIAL:
        this.applyRadialLayout();
        break;
      case LayoutType.CIRCULAR:
        this.applyCircularLayout();
        break;
      case LayoutType.GRID:
        this.applyGridLayout();
        break;
      default:
        this.applyForceDirectedLayout();
    }
    
    this.emit('layoutApplied', layout);
  }
  
  /**
   * Filter nodes and edges
   */
  applyFilters(filters: Partial<FilterConfig>): void {
    this.config.filters = { ...this.config.filters, ...filters };
    
    // Apply node type filter
    this.nodes.forEach(node => {
      node.visible = this.config.filters.nodeTypes.includes(node.type);
      
      // Apply custom filters
      if (node.visible && this.config.filters.customFilters) {
        node.visible = this.config.filters.customFilters.every(filter => filter(node));
      }
      
      // Apply search filter
      if (node.visible && this.config.filters.searchQuery) {
        const query = this.config.filters.searchQuery.toLowerCase();
        node.visible = node.label.toLowerCase().includes(query) ||
                      JSON.stringify(node.properties).toLowerCase().includes(query);
      }
    });
    
    // Hide edges where nodes are hidden
    this.edges.forEach(edge => {
      const sourceNode = this.nodes.get(edge.source);
      const targetNode = this.nodes.get(edge.target);
      edge.visible = !!(sourceNode?.visible && targetNode?.visible);
      
      // Apply relation type filter
      if (edge.visible) {
        edge.visible = this.config.filters.relationTypes.includes(edge.type);
      }
    });
    
    this.emit('filtersApplied', this.config.filters);
  }
  
  /**
   * Cluster nodes based on criteria
   */
  clusterNodes(clusteringFn: (node: VisualizationNode) => string | null): void {
    this.clusters.clear();
    
    this.nodes.forEach(node => {
      const clusterId = clusteringFn(node);
      if (clusterId) {
        if (!this.clusters.has(clusterId)) {
          this.clusters.set(clusterId, new Set());
        }
        this.clusters.get(clusterId)!.add(node.id);
      }
    });
    
    this.emit('clusteringApplied', this.clusters);
  }
  
  /**
   * Expand/collapse node neighborhood
   */
  toggleNodeExpansion(nodeId: string): void {
    const node = this.nodes.get(nodeId);
    if (!node) return;
    
    node.expanded = !node.expanded;
    
    if (node.expanded) {
      // Show neighbors
      const neighbors = this.getNeighbors(nodeId);
      neighbors.forEach(neighbor => {
        neighbor.visible = true;
      });
    } else {
      // Hide neighbors (unless they have other visible connections)
      const neighbors = this.getNeighbors(nodeId);
      neighbors.forEach(neighbor => {
        const otherConnections = this.getConnectedEdges(neighbor.id)
          .filter(edge => edge.source !== nodeId && edge.target !== nodeId)
          .filter(edge => {
            const otherNode = edge.source === neighbor.id 
              ? this.nodes.get(edge.target)
              : this.nodes.get(edge.source);
            return otherNode?.visible && otherNode?.expanded;
          });
        
        if (otherConnections.length === 0) {
          neighbor.visible = false;
        }
      });
    }
    
    this.emit('nodeExpansionToggled', nodeId, node.expanded);
  }
  
  /**
   * Select nodes
   */
  selectNodes(nodeIds: string[], addToSelection = false): void {
    if (!addToSelection) {
      this.selection.selectedNodes.clear();
    }
    
    nodeIds.forEach(id => this.selection.selectedNodes.add(id));
    this.emit('nodesSelected', Array.from(this.selection.selectedNodes));
  }
  
  /**
   * Get graph statistics
   */
  getStatistics(): GraphStatistics {
    const visibleNodes = Array.from(this.nodes.values()).filter(n => n.visible);
    const visibleEdges = Array.from(this.edges.values()).filter(e => e.visible);
    
    const nodeCount = visibleNodes.length;
    const edgeCount = visibleEdges.length;
    const density = nodeCount > 1 ? (2 * edgeCount) / (nodeCount * (nodeCount - 1)) : 0;
    const avgDegree = nodeCount > 0 ? (2 * edgeCount) / nodeCount : 0;
    
    return {
      nodeCount,
      edgeCount,
      density,
      avgDegree,
      clusters: this.clusters.size,
      maxDepth: this.calculateMaxDepth()
    };
  }
  
  /**
   * Export visualization data
   */
  exportData(): {
    nodes: VisualizationNode[];
    edges: VisualizationEdge[];
    viewport: ViewportState;
    config: VisualizationConfig;
  } {
    return {
      nodes: Array.from(this.nodes.values()),
      edges: Array.from(this.edges.values()),
      viewport: this.viewport,
      config: this.config
    };
  }
  
  /**
   * Clear all data
   */
  clear(): void {
    this.nodes.clear();
    this.edges.clear();
    this.clusters.clear();
    this.nodeIndex.clear();
    this.selection.selectedNodes.clear();
    this.selection.selectedEdges.clear();
    this.emit('cleared');
  }
  
  // Layout algorithms
  
  private applyForceDirectedLayout(): void {
    // Simple force-directed layout implementation
    const nodes = Array.from(this.nodes.values()).filter(n => n.visible);
    const edges = Array.from(this.edges.values()).filter(e => e.visible);
    
    // Initialize random positions
    nodes.forEach(node => {
      node.x = Math.random() * this.viewport.dimensions.width;
      node.y = Math.random() * this.viewport.dimensions.height;
    });
    
    // Simulation parameters
    const iterations = 100;
    const k = Math.sqrt((this.viewport.dimensions.width * this.viewport.dimensions.height) / nodes.length);
    const c = 0.1;
    
    for (let i = 0; i < iterations; i++) {
      // Repulsive forces between all nodes
      nodes.forEach((node1, i) => {
        nodes.slice(i + 1).forEach(node2 => {
          const dx = node2.x! - node1.x!;
          const dy = node2.y! - node1.y!;
          const distance = Math.sqrt(dx * dx + dy * dy) || 0.01;
          const force = (k * k) / distance;
          
          const fx = (dx / distance) * force * c;
          const fy = (dy / distance) * force * c;
          
          node1.x! -= fx;
          node1.y! -= fy;
          node2.x! += fx;
          node2.y! += fy;
        });
      });
      
      // Attractive forces along edges
      edges.forEach(edge => {
        const source = this.nodes.get(edge.source);
        const target = this.nodes.get(edge.target);
        
        if (source?.visible && target?.visible) {
          const dx = target.x! - source.x!;
          const dy = target.y! - source.y!;
          const distance = Math.sqrt(dx * dx + dy * dy) || 0.01;
          const force = (distance * distance) / k;
          
          const fx = (dx / distance) * force * c;
          const fy = (dy / distance) * force * c;
          
          source.x! += fx;
          source.y! += fy;
          target.x! -= fx;
          target.y! -= fy;
        }
      });
    }
    
    // Center the graph
    this.centerGraph();
  }
  
  private applyHierarchicalLayout(): void {
    // Implement hierarchical layout
    const nodes = Array.from(this.nodes.values()).filter(n => n.visible);
    const levels = this.calculateNodeLevels();
    
    const levelHeight = this.viewport.dimensions.height / (levels.size + 1);
    const nodesByLevel = new Map<number, VisualizationNode[]>();
    
    // Group nodes by level
    nodes.forEach(node => {
      const level = levels.get(node.id) || 0;
      if (!nodesByLevel.has(level)) {
        nodesByLevel.set(level, []);
      }
      nodesByLevel.get(level)!.push(node);
    });
    
    // Position nodes
    nodesByLevel.forEach((levelNodes, level) => {
      const y = levelHeight * (level + 1);
      const nodeWidth = this.viewport.dimensions.width / (levelNodes.length + 1);
      
      levelNodes.forEach((node, index) => {
        node.x = nodeWidth * (index + 1);
        node.y = y;
      });
    });
  }
  
  private applyRadialLayout(): void {
    const nodes = Array.from(this.nodes.values()).filter(n => n.visible);
    const center = {
      x: this.viewport.dimensions.width / 2,
      y: this.viewport.dimensions.height / 2
    };
    
    // Find central node (highest degree)
    let centralNode = nodes[0];
    let maxDegree = 0;
    
    nodes.forEach(node => {
      const degree = this.getConnectedEdges(node.id).filter(e => e.visible).length;
      if (degree > maxDegree) {
        maxDegree = degree;
        centralNode = node;
      }
    });
    
    // Position central node
    centralNode.x = center.x;
    centralNode.y = center.y;
    
    // Position other nodes in concentric circles
    const visited = new Set<string>([centralNode.id]);
    const rings: VisualizationNode[][] = [[centralNode]];
    
    while (visited.size < nodes.length) {
      const currentRing: VisualizationNode[] = [];
      
      rings[rings.length - 1].forEach(node => {
        const neighbors = this.getNeighbors(node.id).filter(n => 
          n.visible && !visited.has(n.id)
        );
        neighbors.forEach(neighbor => {
          if (!visited.has(neighbor.id)) {
            visited.add(neighbor.id);
            currentRing.push(neighbor);
          }
        });
      });
      
      if (currentRing.length > 0) {
        rings.push(currentRing);
      } else {
        // Add any unconnected nodes
        nodes.forEach(node => {
          if (!visited.has(node.id)) {
            visited.add(node.id);
            currentRing.push(node);
          }
        });
        if (currentRing.length > 0) {
          rings.push(currentRing);
        }
      }
    }
    
    // Position nodes in rings
    const maxRadius = Math.min(
      this.viewport.dimensions.width,
      this.viewport.dimensions.height
    ) / 2 - 50;
    
    rings.slice(1).forEach((ring, ringIndex) => {
      const radius = (maxRadius / rings.length) * (ringIndex + 1);
      const angleStep = (2 * Math.PI) / ring.length;
      
      ring.forEach((node, nodeIndex) => {
        const angle = angleStep * nodeIndex;
        node.x = center.x + radius * Math.cos(angle);
        node.y = center.y + radius * Math.sin(angle);
      });
    });
  }
  
  private applyCircularLayout(): void {
    const nodes = Array.from(this.nodes.values()).filter(n => n.visible);
    const center = {
      x: this.viewport.dimensions.width / 2,
      y: this.viewport.dimensions.height / 2
    };
    const radius = Math.min(
      this.viewport.dimensions.width,
      this.viewport.dimensions.height
    ) / 2 - 50;
    
    const angleStep = (2 * Math.PI) / nodes.length;
    
    nodes.forEach((node, index) => {
      const angle = angleStep * index;
      node.x = center.x + radius * Math.cos(angle);
      node.y = center.y + radius * Math.sin(angle);
    });
  }
  
  private applyGridLayout(): void {
    const nodes = Array.from(this.nodes.values()).filter(n => n.visible);
    const cols = Math.ceil(Math.sqrt(nodes.length));
    const rows = Math.ceil(nodes.length / cols);
    
    const cellWidth = this.viewport.dimensions.width / (cols + 1);
    const cellHeight = this.viewport.dimensions.height / (rows + 1);
    
    nodes.forEach((node, index) => {
      const col = index % cols;
      const row = Math.floor(index / cols);
      
      node.x = cellWidth * (col + 1);
      node.y = cellHeight * (row + 1);
    });
  }
  
  // Helper methods
  
  private convertToVisualizationNode(graphNode: GraphNode): VisualizationNode {
    const nodeType = graphNode.labels[0] as NodeType;
    
    return {
      id: graphNode.id,
      label: graphNode.properties.name || graphNode.id,
      type: nodeType,
      properties: graphNode.properties,
      size: this.calculateNodeSize(graphNode),
      color: this.config.theme.nodeColors[nodeType] || '#999',
      icon: this.getNodeIcon(nodeType),
      expanded: false,
      visible: true,
      metadata: graphNode.properties
    };
  }
  
  private convertToVisualizationEdge(graphRel: GraphRelationship): VisualizationEdge {
    const relType = graphRel.type as RelationType;
    
    return {
      id: graphRel.id,
      source: graphRel.startNodeId,
      target: graphRel.endNodeId,
      type: relType,
      label: relType.replace(/_/g, ' ').toLowerCase(),
      color: this.config.theme.edgeColors[relType] || '#666',
      width: 1,
      style: 'solid',
      animated: false,
      visible: true,
      metadata: graphRel.properties
    };
  }
  
  private calculateNodeSize(node: GraphNode): number {
    // Size based on node type and properties
    const baseSize = 20;
    const nodeType = node.labels[0] as NodeType;
    
    switch (nodeType) {
      case NodeType.PROJECT:
        return baseSize * 3;
      case NodeType.FILE:
        return baseSize * 1.5;
      case NodeType.CLASS:
      case NodeType.INTERFACE:
        return baseSize * 2;
      case NodeType.FUNCTION:
      case NodeType.METHOD:
        return baseSize * 1.2;
      default:
        return baseSize;
    }
  }
  
  private getNodeIcon(nodeType: NodeType): string {
    const icons: Record<NodeType, string> = {
      [NodeType.PROJECT]: '📁',
      [NodeType.FILE]: '📄',
      [NodeType.DIRECTORY]: '📂',
      [NodeType.MODULE]: '📦',
      [NodeType.CLASS]: '🏛️',
      [NodeType.INTERFACE]: '📋',
      [NodeType.FUNCTION]: '⚡',
      [NodeType.METHOD]: '🔧',
      [NodeType.VARIABLE]: '📌',
      [NodeType.CONSTANT]: '🔒',
      [NodeType.TYPE]: '🏷️',
      [NodeType.ENUM]: '📊',
      [NodeType.PACKAGE]: '📦',
      [NodeType.DEPENDENCY]: '🔗',
      [NodeType.COMMENT]: '💬',
      [NodeType.DOCSTRING]: '📝',
      [NodeType.COMMIT]: '🔖',
      [NodeType.BRANCH]: '🌿',
      [NodeType.TAG]: '🏷️',
      [NodeType.DEVELOPER]: '👤',
      [NodeType.TEAM]: '👥',
      [NodeType.CONCEPT]: '💡',
      [NodeType.PATTERN]: '🎯',
      [NodeType.ISSUE]: '🐛'
    };
    
    return icons[nodeType] || '⚪';
  }
  
  private buildIndexes(): void {
    this.nodeIndex.clear();
    
    this.nodes.forEach(node => {
      if (!this.nodeIndex.has(node.type)) {
        this.nodeIndex.set(node.type, new Set());
      }
      this.nodeIndex.get(node.type)!.add(node.id);
    });
  }
  
  private calculateNodeLevels(): Map<string, number> {
    const levels = new Map<string, number>();
    const visited = new Set<string>();
    
    // Find root nodes (no incoming edges)
    const rootNodes = Array.from(this.nodes.values()).filter(node => {
      const incomingEdges = Array.from(this.edges.values()).filter(edge => 
        edge.target === node.id && edge.visible
      );
      return incomingEdges.length === 0 && node.visible;
    });
    
    // BFS to assign levels
    const queue: Array<{ node: VisualizationNode; level: number }> = 
      rootNodes.map(node => ({ node, level: 0 }));
    
    while (queue.length > 0) {
      const { node, level } = queue.shift()!;
      
      if (visited.has(node.id)) continue;
      visited.add(node.id);
      levels.set(node.id, level);
      
      const neighbors = this.getNeighbors(node.id).filter(n => n.visible);
      neighbors.forEach(neighbor => {
        if (!visited.has(neighbor.id)) {
          queue.push({ node: neighbor, level: level + 1 });
        }
      });
    }
    
    return levels;
  }
  
  private calculateMaxDepth(): number {
    const levels = this.calculateNodeLevels();
    return Math.max(...Array.from(levels.values()), 0);
  }
  
  private centerGraph(): void {
    const nodes = Array.from(this.nodes.values()).filter(n => n.visible);
    if (nodes.length === 0) return;
    
    // Calculate bounds
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;
    
    nodes.forEach(node => {
      if (node.x !== undefined && node.y !== undefined) {
        minX = Math.min(minX, node.x);
        maxX = Math.max(maxX, node.x);
        minY = Math.min(minY, node.y);
        maxY = Math.max(maxY, node.y);
      }
    });
    
    // Calculate center offset
    const graphWidth = maxX - minX;
    const graphHeight = maxY - minY;
    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;
    
    const viewportCenterX = this.viewport.dimensions.width / 2;
    const viewportCenterY = this.viewport.dimensions.height / 2;
    
    const offsetX = viewportCenterX - centerX;
    const offsetY = viewportCenterY - centerY;
    
    // Apply offset
    nodes.forEach(node => {
      if (node.x !== undefined && node.y !== undefined) {
        node.x += offsetX;
        node.y += offsetY;
      }
    });
  }
  
  private mergeConfig(partial: Partial<VisualizationConfig>): VisualizationConfig {
    return {
      layout: partial.layout || LayoutType.FORCE_DIRECTED,
      theme: {
        nodeColors: {
          [NodeType.PROJECT]: '#FF6B6B',
          [NodeType.FILE]: '#4ECDC4',
          [NodeType.DIRECTORY]: '#45B7D1',
          [NodeType.MODULE]: '#96CEB4',
          [NodeType.CLASS]: '#DDA0DD',
          [NodeType.INTERFACE]: '#98D8C8',
          [NodeType.FUNCTION]: '#F7DC6F',
          [NodeType.METHOD]: '#F8B500',
          [NodeType.VARIABLE]: '#AED6F1',
          [NodeType.CONSTANT]: '#85C1E2',
          [NodeType.TYPE]: '#BB8FCE',
          [NodeType.ENUM]: '#F0B27A',
          [NodeType.PACKAGE]: '#82E0AA',
          [NodeType.DEPENDENCY]: '#F1948A',
          [NodeType.COMMENT]: '#D5DBDB',
          [NodeType.DOCSTRING]: '#AEB6BF',
          [NodeType.COMMIT]: '#58D68D',
          [NodeType.BRANCH]: '#48C9B0',
          [NodeType.TAG]: '#5DADE2',
          [NodeType.DEVELOPER]: '#AF7AC5',
          [NodeType.TEAM]: '#EC7063',
          [NodeType.CONCEPT]: '#F4D03F',
          [NodeType.PATTERN]: '#EB984E',
          [NodeType.ISSUE]: '#CD6155',
          ...partial.theme?.nodeColors
        },
        edgeColors: {
          [RelationType.CONTAINS]: '#666',
          [RelationType.BELONGS_TO]: '#666',
          [RelationType.IMPORTS]: '#3498DB',
          [RelationType.EXPORTS]: '#2ECC71',
          [RelationType.CALLS]: '#E74C3C',
          [RelationType.IMPLEMENTS]: '#9B59B6',
          [RelationType.EXTENDS]: '#F39C12',
          [RelationType.INSTANTIATES]: '#1ABC9C',
          [RelationType.REFERENCES]: '#34495E',
          [RelationType.USES]: '#16A085',
          [RelationType.DEFINES]: '#27AE60',
          [RelationType.DECLARES]: '#2980B9',
          [RelationType.HAS_TYPE]: '#8E44AD',
          [RelationType.RETURNS]: '#C0392B',
          [RelationType.ACCEPTS]: '#D35400',
          [RelationType.DEPENDS_ON]: '#7F8C8D',
          [RelationType.REQUIRED_BY]: '#95A5A6',
          [RelationType.AUTHORED_BY]: '#2C3E50',
          [RelationType.MODIFIED_BY]: '#34495E',
          [RelationType.CREATED_IN]: '#16A085',
          [RelationType.MODIFIED_IN]: '#27AE60',
          [RelationType.DOCUMENTS]: '#3498DB',
          [RelationType.DESCRIBED_BY]: '#2980B9',
          [RelationType.SIMILAR_TO]: '#E67E22',
          [RelationType.RELATED_TO]: '#F39C12',
          [RelationType.PATTERN_OF]: '#D35400',
          ...partial.theme?.edgeColors
        },
        backgroundColor: partial.theme?.backgroundColor || '#1e1e1e',
        highlightColor: partial.theme?.highlightColor || '#FFD700',
        selectionColor: partial.theme?.selectionColor || '#00CED1',
        font: {
          family: partial.theme?.font?.family || 'Arial, sans-serif',
          size: partial.theme?.font?.size || 12,
          color: partial.theme?.font?.color || '#FFFFFF',
          ...partial.theme?.font
        }
      },
      performance: {
        enableWebGL: partial.performance?.enableWebGL ?? true,
        maxNodes: partial.performance?.maxNodes || 1000,
        maxEdges: partial.performance?.maxEdges || 5000,
        enableClustering: partial.performance?.enableClustering ?? true,
        clusterThreshold: partial.performance?.clusterThreshold || 50,
        enableLOD: partial.performance?.enableLOD ?? true,
        lodThresholds: {
          high: partial.performance?.lodThresholds?.high || 50,
          medium: partial.performance?.lodThresholds?.medium || 200,
          low: partial.performance?.lodThresholds?.low || 500,
          ...partial.performance?.lodThresholds
        }
      },
      interaction: {
        enableZoom: partial.interaction?.enableZoom ?? true,
        enablePan: partial.interaction?.enablePan ?? true,
        enableNodeDrag: partial.interaction?.enableNodeDrag ?? true,
        enableBoxSelection: partial.interaction?.enableBoxSelection ?? true,
        enableContextMenu: partial.interaction?.enableContextMenu ?? true,
        enableTooltips: partial.interaction?.enableTooltips ?? true,
        enableKeyboardShortcuts: partial.interaction?.enableKeyboardShortcuts ?? true,
        ...partial.interaction
      },
      filters: {
        nodeTypes: partial.filters?.nodeTypes || Object.values(NodeType),
        relationTypes: partial.filters?.relationTypes || Object.values(RelationType),
        ...partial.filters
      }
    };
  }
}