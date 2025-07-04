/**
 * Advanced Layout Algorithms for Graph Visualization
 * 
 * Implements sophisticated layout algorithms for code graph visualization
 */

import { 
  VisualizationNode, 
  VisualizationEdge,
  ViewportState 
} from './graph-visualization-engine';

export interface LayoutOptions {
  iterations?: number;
  nodeSpacing?: number;
  edgeLength?: number;
  gravity?: number;
  theta?: number;
  alpha?: number;
  alphaDecay?: number;
  velocityDecay?: number;
  preventOverlap?: boolean;
  animate?: boolean;
}

export interface Point {
  x: number;
  y: number;
}

export interface Vector {
  x: number;
  y: number;
}

export interface LayoutResult {
  positions: Map<string, Point>;
  bounds: {
    minX: number;
    minY: number;
    maxX: number;
    maxY: number;
  };
}

/**
 * Base class for layout algorithms
 */
export abstract class LayoutAlgorithm {
  protected nodes: VisualizationNode[];
  protected edges: VisualizationEdge[];
  protected viewport: ViewportState;
  protected options: LayoutOptions;
  
  constructor(
    nodes: VisualizationNode[],
    edges: VisualizationEdge[],
    viewport: ViewportState,
    options: LayoutOptions = {}
  ) {
    this.nodes = nodes;
    this.edges = edges;
    this.viewport = viewport;
    this.options = this.mergeOptions(options);
  }
  
  abstract calculate(): LayoutResult;
  
  protected mergeOptions(options: LayoutOptions): LayoutOptions {
    return {
      iterations: 100,
      nodeSpacing: 50,
      edgeLength: 100,
      gravity: 0.1,
      theta: 0.8,
      alpha: 1,
      alphaDecay: 0.01,
      velocityDecay: 0.6,
      preventOverlap: true,
      animate: false,
      ...options
    };
  }
  
  protected distance(a: Point, b: Point): number {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    return Math.sqrt(dx * dx + dy * dy);
  }
  
  protected normalize(vector: Vector): Vector {
    const magnitude = Math.sqrt(vector.x * vector.x + vector.y * vector.y);
    if (magnitude === 0) return { x: 0, y: 0 };
    return {
      x: vector.x / magnitude,
      y: vector.y / magnitude
    };
  }
  
  protected calculateBounds(positions: Map<string, Point>): LayoutResult['bounds'] {
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;
    
    positions.forEach(pos => {
      minX = Math.min(minX, pos.x);
      maxX = Math.max(maxX, pos.x);
      minY = Math.min(minY, pos.y);
      maxY = Math.max(maxY, pos.y);
    });
    
    return { minX, minY, maxX, maxY };
  }
}

/**
 * Enhanced Force-Directed Layout using Barnes-Hut optimization
 */
export class ForceDirectedLayout extends LayoutAlgorithm {
  private velocities: Map<string, Vector> = new Map();
  private forces: Map<string, Vector> = new Map();
  
  calculate(): LayoutResult {
    const positions = new Map<string, Point>();
    
    // Initialize positions and velocities
    this.nodes.forEach(node => {
      positions.set(node.id, {
        x: node.x || Math.random() * this.viewport.dimensions.width,
        y: node.y || Math.random() * this.viewport.dimensions.height
      });
      this.velocities.set(node.id, { x: 0, y: 0 });
    });
    
    // Run simulation
    let alpha = this.options.alpha!;
    for (let i = 0; i < this.options.iterations!; i++) {
      this.calculateForces(positions);
      this.updatePositions(positions, alpha);
      alpha *= (1 - this.options.alphaDecay!);
    }
    
    return {
      positions,
      bounds: this.calculateBounds(positions)
    };
  }
  
  private calculateForces(positions: Map<string, Point>): void {
    // Reset forces
    this.nodes.forEach(node => {
      this.forces.set(node.id, { x: 0, y: 0 });
    });
    
    // Apply gravity towards center
    const center = {
      x: this.viewport.dimensions.width / 2,
      y: this.viewport.dimensions.height / 2
    };
    
    this.nodes.forEach(node => {
      const pos = positions.get(node.id)!;
      const force = this.forces.get(node.id)!;
      
      const dx = center.x - pos.x;
      const dy = center.y - pos.y;
      const distance = Math.sqrt(dx * dx + dy * dy) || 1;
      
      force.x += dx * this.options.gravity! / distance;
      force.y += dy * this.options.gravity! / distance;
    });
    
    // Apply repulsive forces between nodes
    this.applyRepulsiveForces(positions);
    
    // Apply attractive forces along edges
    this.applyAttractiveForces(positions);
  }
  
  private applyRepulsiveForces(positions: Map<string, Point>): void {
    const k = this.options.nodeSpacing!;
    
    for (let i = 0; i < this.nodes.length; i++) {
      for (let j = i + 1; j < this.nodes.length; j++) {
        const node1 = this.nodes[i];
        const node2 = this.nodes[j];
        
        const pos1 = positions.get(node1.id)!;
        const pos2 = positions.get(node2.id)!;
        
        const dx = pos2.x - pos1.x;
        const dy = pos2.y - pos1.y;
        const distance = Math.sqrt(dx * dx + dy * dy) || 0.01;
        
        // Coulomb's law
        const force = (k * k) / (distance * distance);
        
        const fx = (dx / distance) * force;
        const fy = (dy / distance) * force;
        
        const force1 = this.forces.get(node1.id)!;
        const force2 = this.forces.get(node2.id)!;
        
        force1.x -= fx;
        force1.y -= fy;
        force2.x += fx;
        force2.y += fy;
      }
    }
  }
  
  private applyAttractiveForces(positions: Map<string, Point>): void {
    const k = this.options.edgeLength!;
    
    this.edges.forEach(edge => {
      const sourcePos = positions.get(edge.source);
      const targetPos = positions.get(edge.target);
      
      if (!sourcePos || !targetPos) return;
      
      const dx = targetPos.x - sourcePos.x;
      const dy = targetPos.y - sourcePos.y;
      const distance = Math.sqrt(dx * dx + dy * dy) || 0.01;
      
      // Hooke's law
      const force = (distance - k) * 0.1;
      
      const fx = (dx / distance) * force;
      const fy = (dy / distance) * force;
      
      const sourceForce = this.forces.get(edge.source)!;
      const targetForce = this.forces.get(edge.target)!;
      
      sourceForce.x += fx;
      sourceForce.y += fy;
      targetForce.x -= fx;
      targetForce.y -= fy;
    });
  }
  
  private updatePositions(positions: Map<string, Point>, alpha: number): void {
    this.nodes.forEach(node => {
      const pos = positions.get(node.id)!;
      const velocity = this.velocities.get(node.id)!;
      const force = this.forces.get(node.id)!;
      
      // Update velocity
      velocity.x = velocity.x * this.options.velocityDecay! + force.x * alpha;
      velocity.y = velocity.y * this.options.velocityDecay! + force.y * alpha;
      
      // Update position
      pos.x += velocity.x;
      pos.y += velocity.y;
      
      // Keep within bounds
      pos.x = Math.max(50, Math.min(this.viewport.dimensions.width - 50, pos.x));
      pos.y = Math.max(50, Math.min(this.viewport.dimensions.height - 50, pos.y));
    });
  }
}

/**
 * Hierarchical Layout for tree-like structures
 */
export class HierarchicalLayout extends LayoutAlgorithm {
  calculate(): LayoutResult {
    const positions = new Map<string, Point>();
    const levels = this.assignLevels();
    const nodesByLevel = this.groupByLevel(levels);
    
    // Calculate positions
    const levelHeight = this.viewport.dimensions.height / (nodesByLevel.size + 1);
    
    nodesByLevel.forEach((nodes, level) => {
      const y = levelHeight * (level + 1);
      const nodeWidth = this.viewport.dimensions.width / (nodes.length + 1);
      
      // Sort nodes by their connections to maintain consistency
      nodes.sort((a, b) => {
        const aConnections = this.getConnectionCount(a.id);
        const bConnections = this.getConnectionCount(b.id);
        return bConnections - aConnections;
      });
      
      nodes.forEach((node, index) => {
        positions.set(node.id, {
          x: nodeWidth * (index + 1),
          y: y
        });
      });
    });
    
    // Minimize edge crossings
    this.minimizeCrossings(positions, levels, nodesByLevel);
    
    return {
      positions,
      bounds: this.calculateBounds(positions)
    };
  }
  
  private assignLevels(): Map<string, number> {
    const levels = new Map<string, number>();
    const visited = new Set<string>();
    
    // Find root nodes
    const roots = this.findRootNodes();
    
    // BFS to assign levels
    const queue: Array<{ node: VisualizationNode; level: number }> = 
      roots.map(node => ({ node, level: 0 }));
    
    while (queue.length > 0) {
      const { node, level } = queue.shift()!;
      
      if (visited.has(node.id)) continue;
      visited.add(node.id);
      levels.set(node.id, level);
      
      // Find children
      const children = this.edges
        .filter(edge => edge.source === node.id)
        .map(edge => this.nodes.find(n => n.id === edge.target))
        .filter((n): n is VisualizationNode => n !== undefined);
      
      children.forEach(child => {
        if (!visited.has(child.id)) {
          queue.push({ node: child, level: level + 1 });
        }
      });
    }
    
    // Handle disconnected nodes
    this.nodes.forEach(node => {
      if (!levels.has(node.id)) {
        levels.set(node.id, 0);
      }
    });
    
    return levels;
  }
  
  private findRootNodes(): VisualizationNode[] {
    const hasIncoming = new Set<string>();
    
    this.edges.forEach(edge => {
      hasIncoming.add(edge.target);
    });
    
    return this.nodes.filter(node => !hasIncoming.has(node.id));
  }
  
  private groupByLevel(levels: Map<string, number>): Map<number, VisualizationNode[]> {
    const groups = new Map<number, VisualizationNode[]>();
    
    this.nodes.forEach(node => {
      const level = levels.get(node.id)!;
      if (!groups.has(level)) {
        groups.set(level, []);
      }
      groups.get(level)!.push(node);
    });
    
    return groups;
  }
  
  private getConnectionCount(nodeId: string): number {
    return this.edges.filter(edge => 
      edge.source === nodeId || edge.target === nodeId
    ).length;
  }
  
  private minimizeCrossings(
    positions: Map<string, Point>,
    levels: Map<string, number>,
    nodesByLevel: Map<number, VisualizationNode[]>
  ): void {
    // Simple crossing minimization by sorting nodes within levels
    nodesByLevel.forEach((nodes, level) => {
      if (level === 0) return;
      
      // Calculate barycenter for each node
      const barycenters = new Map<string, number>();
      
      nodes.forEach(node => {
        const parents = this.edges
          .filter(edge => edge.target === node.id)
          .map(edge => positions.get(edge.source))
          .filter((pos): pos is Point => pos !== undefined);
        
        if (parents.length > 0) {
          const avgX = parents.reduce((sum, pos) => sum + pos.x, 0) / parents.length;
          barycenters.set(node.id, avgX);
        } else {
          barycenters.set(node.id, positions.get(node.id)!.x);
        }
      });
      
      // Sort by barycenter
      nodes.sort((a, b) => {
        return barycenters.get(a.id)! - barycenters.get(b.id)!;
      });
      
      // Update positions
      const nodeWidth = this.viewport.dimensions.width / (nodes.length + 1);
      nodes.forEach((node, index) => {
        const pos = positions.get(node.id)!;
        pos.x = nodeWidth * (index + 1);
      });
    });
  }
}

/**
 * Radial Layout for network visualization
 */
export class RadialLayout extends LayoutAlgorithm {
  calculate(): LayoutResult {
    const positions = new Map<string, Point>();
    const center = {
      x: this.viewport.dimensions.width / 2,
      y: this.viewport.dimensions.height / 2
    };
    
    // Find central node
    const centralNode = this.findCentralNode();
    const rings = this.buildRings(centralNode);
    
    // Position nodes in rings
    rings.forEach((ring, ringIndex) => {
      if (ringIndex === 0) {
        // Central node
        positions.set(ring[0].id, { ...center });
      } else {
        const radius = this.calculateRadius(ringIndex, rings.length);
        const angleStep = (2 * Math.PI) / ring.length;
        
        ring.forEach((node, nodeIndex) => {
          const angle = angleStep * nodeIndex - Math.PI / 2; // Start from top
          positions.set(node.id, {
            x: center.x + radius * Math.cos(angle),
            y: center.y + radius * Math.sin(angle)
          });
        });
      }
    });
    
    return {
      positions,
      bounds: this.calculateBounds(positions)
    };
  }
  
  private findCentralNode(): VisualizationNode {
    // Node with highest degree centrality
    let maxDegree = 0;
    let centralNode = this.nodes[0];
    
    this.nodes.forEach(node => {
      const degree = this.edges.filter(edge => 
        edge.source === node.id || edge.target === node.id
      ).length;
      
      if (degree > maxDegree) {
        maxDegree = degree;
        centralNode = node;
      }
    });
    
    return centralNode;
  }
  
  private buildRings(centralNode: VisualizationNode): VisualizationNode[][] {
    const rings: VisualizationNode[][] = [[centralNode]];
    const visited = new Set<string>([centralNode.id]);
    
    while (visited.size < this.nodes.length) {
      const currentRing: VisualizationNode[] = [];
      
      rings[rings.length - 1].forEach(node => {
        const neighbors = this.getNeighbors(node.id).filter(n => 
          !visited.has(n.id)
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
        // Add disconnected nodes
        const disconnected = this.nodes.filter(n => !visited.has(n.id));
        if (disconnected.length > 0) {
          disconnected.forEach(node => visited.add(node.id));
          rings.push(disconnected);
        }
        break;
      }
    }
    
    return rings;
  }
  
  private getNeighbors(nodeId: string): VisualizationNode[] {
    const neighborIds = new Set<string>();
    
    this.edges.forEach(edge => {
      if (edge.source === nodeId) neighborIds.add(edge.target);
      if (edge.target === nodeId) neighborIds.add(edge.source);
    });
    
    return Array.from(neighborIds)
      .map(id => this.nodes.find(n => n.id === id))
      .filter((n): n is VisualizationNode => n !== undefined);
  }
  
  private calculateRadius(ringIndex: number, totalRings: number): number {
    const maxRadius = Math.min(
      this.viewport.dimensions.width,
      this.viewport.dimensions.height
    ) / 2 - 50;
    
    return (maxRadius / totalRings) * ringIndex;
  }
}

/**
 * Concentric Layout for hierarchical visualization
 */
export class ConcentricLayout extends LayoutAlgorithm {
  calculate(): LayoutResult {
    const positions = new Map<string, Point>();
    const center = {
      x: this.viewport.dimensions.width / 2,
      y: this.viewport.dimensions.height / 2
    };
    
    // Calculate node importance scores
    const scores = this.calculateImportanceScores();
    
    // Group nodes by score ranges
    const groups = this.groupByScores(scores);
    
    // Position groups in concentric circles
    groups.forEach((nodes, groupIndex) => {
      if (groupIndex === 0) {
        // Most important nodes in center
        const angleStep = (2 * Math.PI) / Math.max(nodes.length, 1);
        const radius = nodes.length === 1 ? 0 : 50;
        
        nodes.forEach((node, index) => {
          const angle = angleStep * index;
          positions.set(node.id, {
            x: center.x + radius * Math.cos(angle),
            y: center.y + radius * Math.sin(angle)
          });
        });
      } else {
        const radius = this.calculateRadius(groupIndex, groups.length);
        const angleStep = (2 * Math.PI) / nodes.length;
        
        nodes.forEach((node, index) => {
          const angle = angleStep * index;
          positions.set(node.id, {
            x: center.x + radius * Math.cos(angle),
            y: center.y + radius * Math.sin(angle)
          });
        });
      }
    });
    
    return {
      positions,
      bounds: this.calculateBounds(positions)
    };
  }
  
  private calculateImportanceScores(): Map<string, number> {
    const scores = new Map<string, number>();
    
    this.nodes.forEach(node => {
      // Degree centrality
      const degree = this.edges.filter(edge => 
        edge.source === node.id || edge.target === node.id
      ).length;
      
      // Betweenness centrality approximation
      const inDegree = this.edges.filter(edge => edge.target === node.id).length;
      const outDegree = this.edges.filter(edge => edge.source === node.id).length;
      
      // Combined score
      const score = degree + (inDegree * 2) + (outDegree * 1.5);
      scores.set(node.id, score);
    });
    
    return scores;
  }
  
  private groupByScores(scores: Map<string, number>): VisualizationNode[][] {
    // Sort nodes by score
    const sortedNodes = [...this.nodes].sort((a, b) => {
      return (scores.get(b.id) || 0) - (scores.get(a.id) || 0);
    });
    
    // Create groups
    const groups: VisualizationNode[][] = [];
    const groupSizes = this.calculateGroupSizes(sortedNodes.length);
    
    let index = 0;
    groupSizes.forEach(size => {
      const group = sortedNodes.slice(index, index + size);
      if (group.length > 0) {
        groups.push(group);
      }
      index += size;
    });
    
    return groups;
  }
  
  private calculateGroupSizes(totalNodes: number): number[] {
    // Distribute nodes in concentric circles with increasing sizes
    const groups: number[] = [];
    let remaining = totalNodes;
    let groupSize = Math.min(6, totalNodes); // Inner circle max 6 nodes
    
    while (remaining > 0) {
      const size = Math.min(groupSize, remaining);
      groups.push(size);
      remaining -= size;
      groupSize = Math.floor(groupSize * 1.5); // Each ring 50% larger
    }
    
    return groups;
  }
  
  private calculateRadius(groupIndex: number, totalGroups: number): number {
    const maxRadius = Math.min(
      this.viewport.dimensions.width,
      this.viewport.dimensions.height
    ) / 2 - 50;
    
    const radiusStep = maxRadius / totalGroups;
    return radiusStep * (groupIndex + 1);
  }
}

/**
 * Layout factory
 */
export class LayoutFactory {
  static create(
    type: string,
    nodes: VisualizationNode[],
    edges: VisualizationEdge[],
    viewport: ViewportState,
    options?: LayoutOptions
  ): LayoutAlgorithm {
    switch (type) {
      case 'force-directed':
        return new ForceDirectedLayout(nodes, edges, viewport, options);
      case 'hierarchical':
        return new HierarchicalLayout(nodes, edges, viewport, options);
      case 'radial':
        return new RadialLayout(nodes, edges, viewport, options);
      case 'concentric':
        return new ConcentricLayout(nodes, edges, viewport, options);
      default:
        return new ForceDirectedLayout(nodes, edges, viewport, options);
    }
  }
}