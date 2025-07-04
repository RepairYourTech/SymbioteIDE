/**
 * Dependency Explorer
 * 
 * Explore and analyze import/export dependencies in the codebase
 */

import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import {
  GraphNode,
  GraphRelationship,
  NodeType,
  RelationType,
  FileNodeProperties,
  PackageNodeProperties
} from '../types/graph-types';

export interface DependencyNode {
  id: string;
  name: string;
  type: 'file' | 'package' | 'module';
  path?: string;
  version?: string;
  size?: number;
  directDependencies: number;
  transitiveDependencies: number;
  dependents: number;
}

export interface DependencyEdge {
  source: string;
  target: string;
  type: 'imports' | 'exports' | 'depends_on' | 'requires';
  weight: number;
  details?: {
    importedNames?: string[];
    isDefault?: boolean;
    isDynamic?: boolean;
    isDevDependency?: boolean;
  };
}

export interface DependencyGraph {
  nodes: DependencyNode[];
  edges: DependencyEdge[];
  cycles: string[][];
  metrics: DependencyMetrics;
}

export interface DependencyMetrics {
  totalNodes: number;
  totalEdges: number;
  maxDepth: number;
  avgDependencies: number;
  circularDependencies: number;
  isolatedNodes: number;
  hubNodes: string[];  // Nodes with many dependents
  leafNodes: string[];  // Nodes with no dependencies
}

export interface DependencyAnalysisOptions {
  includeDevDependencies?: boolean;
  includeTransitive?: boolean;
  maxDepth?: number;
  filterPattern?: RegExp;
}

export class DependencyExplorer {
  private connectionManager: Neo4jConnectionManager;
  
  constructor(connectionManager: Neo4jConnectionManager) {
    this.connectionManager = connectionManager;
  }
  
  /**
   * Get dependency graph for a project or module
   */
  async getDependencyGraph(
    databaseId: string,
    rootPath?: string,
    options: DependencyAnalysisOptions = {}
  ): Promise<DependencyGraph> {
    const nodes = await this.getDependencyNodes(databaseId, rootPath, options);
    const edges = await this.getDependencyEdges(databaseId, nodes, options);
    const cycles = await this.detectCircularDependencies(databaseId, nodes);
    const metrics = this.calculateMetrics(nodes, edges, cycles);
    
    return {
      nodes,
      edges,
      cycles,
      metrics
    };
  }
  
  /**
   * Get dependencies for a specific file or module
   */
  async getDirectDependencies(
    nodeId: string,
    databaseId: string,
    direction: 'imports' | 'exports' | 'both' = 'imports'
  ): Promise<DependencyNode[]> {
    const directionClause = 
      direction === 'imports' ? '->' :
      direction === 'exports' ? '<-' :
      '-';
    
    const query = `
      MATCH (n {id: $id})${directionClause}[:IMPORTS|EXPORTS|DEPENDS_ON]${directionClause}(dep)
      WHERE dep:File OR dep:Module OR dep:Package
      RETURN dep, labels(dep) as labels
      ORDER BY dep.name
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const dependencies: DependencyNode[] = [];
    
    for (const record of result.records) {
      const node = record.get('dep');
      const labels = record.get('labels');
      const depNode = await this.createDependencyNode(node, labels[0], databaseId);
      dependencies.push(depNode);
    }
    
    return dependencies;
  }
  
  /**
   * Get transitive dependencies
   */
  async getTransitiveDependencies(
    nodeId: string,
    databaseId: string,
    maxDepth: number = 5
  ): Promise<{
    dependencies: DependencyNode[];
    paths: string[][];
  }> {
    const query = `
      MATCH path = (start {id: $id})-[:IMPORTS|DEPENDS_ON*1..${maxDepth}]->(dep)
      WHERE dep:File OR dep:Module OR dep:Package
      AND NOT dep.id = $id
      WITH dep, path
      RETURN DISTINCT dep, labels(dep) as labels, 
             collect(distinct [n in nodes(path) | n.name]) as paths
      ORDER BY length(paths[0])
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: nodeId }
    );
    
    const dependencies: DependencyNode[] = [];
    const paths: string[][] = [];
    
    for (const record of result.records) {
      const node = record.get('dep');
      const labels = record.get('labels');
      const nodePaths = record.get('paths');
      
      const depNode = await this.createDependencyNode(node, labels[0], databaseId);
      dependencies.push(depNode);
      paths.push(...nodePaths);
    }
    
    return { dependencies, paths };
  }
  
  /**
   * Find circular dependencies
   */
  async detectCircularDependencies(
    databaseId: string,
    nodes?: DependencyNode[]
  ): Promise<string[][]> {
    const query = `
      MATCH path = (n)-[:IMPORTS|DEPENDS_ON*]->(n)
      WHERE n:File OR n:Module OR n:Package
      ${nodes ? 'AND n.id IN $nodeIds' : ''}
      WITH path, [node in nodes(path) | node.name] as cycle
      RETURN DISTINCT cycle
      LIMIT 100
    `;
    
    const params = nodes ? { nodeIds: nodes.map(n => n.id) } : {};
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      params
    );
    
    return result.records.map(record => record.get('cycle'));
  }
  
  /**
   * Find unused dependencies
   */
  async findUnusedDependencies(
    databaseId: string,
    projectPath: string
  ): Promise<DependencyNode[]> {
    const query = `
      MATCH (pkg:Package)<-[:DEPENDS_ON]-(project:Project {path: $projectPath})
      WHERE NOT exists((project)-[:IMPORTS|USES]->(:File)-[:BELONGS_TO]->(pkg))
      RETURN pkg
      ORDER BY pkg.name
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { projectPath }
    );
    
    const unused: DependencyNode[] = [];
    
    for (const record of result.records) {
      const node = record.get('pkg');
      const depNode = await this.createDependencyNode(node, NodeType.PACKAGE, databaseId);
      unused.push(depNode);
    }
    
    return unused;
  }
  
  /**
   * Find missing dependencies
   */
  async findMissingDependencies(
    databaseId: string,
    projectPath: string
  ): Promise<string[]> {
    const query = `
      MATCH (file:File)-[:IMPORTS]->(target)
      WHERE file.path STARTS WITH $projectPath
      AND NOT exists((target:File))
      AND NOT exists((target:Package))
      RETURN DISTINCT target.path as missingPath
      ORDER BY missingPath
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { projectPath }
    );
    
    return result.records.map(record => record.get('missingPath'));
  }
  
  /**
   * Analyze dependency impact
   */
  async analyzeDependencyImpact(
    nodeId: string,
    databaseId: string
  ): Promise<{
    directImpact: DependencyNode[];
    transitiveImpact: DependencyNode[];
    affectedFiles: number;
    riskLevel: 'low' | 'medium' | 'high';
  }> {
    // Find all files that depend on this node
    const directQuery = `
      MATCH (n {id: $id})<-[:IMPORTS|DEPENDS_ON]-(dependent)
      WHERE dependent:File OR dependent:Module
      RETURN dependent, labels(dependent) as labels
    `;
    
    const transitiveQuery = `
      MATCH (n {id: $id})<-[:IMPORTS|DEPENDS_ON*1..3]-(dependent)
      WHERE dependent:File OR dependent:Module
      AND NOT dependent.id = $id
      RETURN DISTINCT dependent, labels(dependent) as labels
    `;
    
    const [directResult, transitiveResult] = await Promise.all([
      this.connectionManager.executeQuery(databaseId, directQuery, { id: nodeId }),
      this.connectionManager.executeQuery(databaseId, transitiveQuery, { id: nodeId })
    ]);
    
    const directImpact: DependencyNode[] = [];
    const transitiveImpact: DependencyNode[] = [];
    
    for (const record of directResult.records) {
      const node = record.get('dependent');
      const labels = record.get('labels');
      const depNode = await this.createDependencyNode(node, labels[0], databaseId);
      directImpact.push(depNode);
    }
    
    for (const record of transitiveResult.records) {
      const node = record.get('dependent');
      const labels = record.get('labels');
      const depNode = await this.createDependencyNode(node, labels[0], databaseId);
      transitiveImpact.push(depNode);
    }
    
    const affectedFiles = new Set([
      ...directImpact.filter(n => n.type === 'file').map(n => n.id),
      ...transitiveImpact.filter(n => n.type === 'file').map(n => n.id)
    ]).size;
    
    const riskLevel = 
      affectedFiles > 50 ? 'high' :
      affectedFiles > 10 ? 'medium' :
      'low';
    
    return {
      directImpact,
      transitiveImpact,
      affectedFiles,
      riskLevel
    };
  }
  
  /**
   * Get dependency bottlenecks (files with many dependents)
   */
  async findDependencyBottlenecks(
    databaseId: string,
    threshold: number = 10
  ): Promise<{
    node: DependencyNode;
    dependentCount: number;
    criticalityScore: number;
  }[]> {
    const query = `
      MATCH (n)<-[:IMPORTS|DEPENDS_ON]-(dependent)
      WHERE n:File OR n:Module OR n:Package
      WITH n, count(distinct dependent) as dependentCount
      WHERE dependentCount >= $threshold
      MATCH (n)-[:IMPORTS|DEPENDS_ON]->(dependency)
      WITH n, dependentCount, count(distinct dependency) as dependencyCount
      RETURN n, labels(n) as labels, dependentCount, dependencyCount
      ORDER BY dependentCount DESC
      LIMIT 50
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { threshold }
    );
    
    const bottlenecks: {
      node: DependencyNode;
      dependentCount: number;
      criticalityScore: number;
    }[] = [];
    
    for (const record of result.records) {
      const node = record.get('n');
      const labels = record.get('labels');
      const dependentCount = record.get('dependentCount').toNumber();
      const dependencyCount = record.get('dependencyCount').toNumber();
      
      const depNode = await this.createDependencyNode(node, labels[0], databaseId);
      
      // Calculate criticality score based on fan-in/fan-out
      const criticalityScore = dependentCount * (1 + 1 / (dependencyCount + 1));
      
      bottlenecks.push({
        node: depNode,
        dependentCount,
        criticalityScore
      });
    }
    
    return bottlenecks;
  }
  
  /**
   * Get dependency tree for visualization
   */
  async getDependencyTree(
    rootId: string,
    databaseId: string,
    maxDepth: number = 3
  ): Promise<{
    id: string;
    name: string;
    children: any[];
  }> {
    const buildTree = async (nodeId: string, depth: number, visited: Set<string>): Promise<any> => {
      if (depth >= maxDepth || visited.has(nodeId)) {
        return null;
      }
      
      visited.add(nodeId);
      
      const dependencies = await this.getDirectDependencies(nodeId, databaseId);
      
      const children = [];
      for (const dep of dependencies) {
        const child = await buildTree(dep.id, depth + 1, visited);
        if (child) {
          children.push(child);
        }
      }
      
      const nodeResult = await this.connectionManager.executeQuery(
        databaseId,
        'MATCH (n {id: $id}) RETURN n.name as name',
        { id: nodeId }
      );
      
      const name = nodeResult.records[0]?.get('name') || nodeId;
      
      return {
        id: nodeId,
        name,
        children
      };
    };
    
    return buildTree(rootId, 0, new Set());
  }
  
  // Private helper methods
  
  private async getDependencyNodes(
    databaseId: string,
    rootPath?: string,
    options: DependencyAnalysisOptions
  ): Promise<DependencyNode[]> {
    let query = `
      MATCH (n)
      WHERE (n:File OR n:Module OR n:Package)
    `;
    
    const params: any = {};
    
    if (rootPath) {
      query += ' AND n.path STARTS WITH $rootPath';
      params.rootPath = rootPath;
    }
    
    if (options.filterPattern) {
      query += ' AND n.path =~ $pattern';
      params.pattern = options.filterPattern.source;
    }
    
    query += ' RETURN n, labels(n) as labels';
    
    const result = await this.connectionManager.executeQuery(databaseId, query, params);
    
    const nodes: DependencyNode[] = [];
    
    for (const record of result.records) {
      const node = record.get('n');
      const labels = record.get('labels');
      const depNode = await this.createDependencyNode(node, labels[0], databaseId);
      nodes.push(depNode);
    }
    
    return nodes;
  }
  
  private async getDependencyEdges(
    databaseId: string,
    nodes: DependencyNode[],
    options: DependencyAnalysisOptions
  ): Promise<DependencyEdge[]> {
    const nodeIds = nodes.map(n => n.id);
    
    const query = `
      MATCH (source)-[r:IMPORTS|EXPORTS|DEPENDS_ON|REQUIRES]->(target)
      WHERE source.id IN $nodeIds AND target.id IN $nodeIds
      RETURN source.id as sourceId, target.id as targetId, 
             type(r) as relType, r as relationship
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { nodeIds }
    );
    
    const edges: DependencyEdge[] = [];
    
    result.records.forEach(record => {
      const edge: DependencyEdge = {
        source: record.get('sourceId'),
        target: record.get('targetId'),
        type: record.get('relType').toLowerCase() as any,
        weight: 1,
        details: record.get('relationship').properties
      };
      
      edges.push(edge);
    });
    
    return edges;
  }
  
  private async createDependencyNode(
    node: any,
    nodeType: string,
    databaseId: string
  ): Promise<DependencyNode> {
    const id = node.properties.id;
    
    // Count dependencies
    const depQuery = `
      MATCH (n {id: $id})
      OPTIONAL MATCH (n)-[:IMPORTS|DEPENDS_ON]->(dep)
      WITH n, count(distinct dep) as directDeps
      OPTIONAL MATCH (n)<-[:IMPORTS|DEPENDS_ON]-(dependent)
      RETURN directDeps, count(distinct dependent) as dependents
    `;
    
    const depResult = await this.connectionManager.executeQuery(
      databaseId,
      depQuery,
      { id }
    );
    
    const directDependencies = depResult.records[0]?.get('directDeps')?.toNumber() || 0;
    const dependents = depResult.records[0]?.get('dependents')?.toNumber() || 0;
    
    return {
      id,
      name: node.properties.name,
      type: nodeType === NodeType.FILE ? 'file' : 
            nodeType === NodeType.PACKAGE ? 'package' : 
            'module',
      path: node.properties.path,
      version: node.properties.version,
      size: node.properties.size,
      directDependencies,
      transitiveDependencies: 0, // Would need separate calculation
      dependents
    };
  }
  
  private calculateMetrics(
    nodes: DependencyNode[],
    edges: DependencyEdge[],
    cycles: string[][]
  ): DependencyMetrics {
    const inDegree = new Map<string, number>();
    const outDegree = new Map<string, number>();
    
    nodes.forEach(node => {
      inDegree.set(node.id, 0);
      outDegree.set(node.id, 0);
    });
    
    edges.forEach(edge => {
      inDegree.set(edge.target, (inDegree.get(edge.target) || 0) + 1);
      outDegree.set(edge.source, (outDegree.get(edge.source) || 0) + 1);
    });
    
    const hubNodes = nodes
      .filter(node => (inDegree.get(node.id) || 0) > 10)
      .map(node => node.id);
    
    const leafNodes = nodes
      .filter(node => (outDegree.get(node.id) || 0) === 0)
      .map(node => node.id);
    
    const isolatedNodes = nodes
      .filter(node => 
        (inDegree.get(node.id) || 0) === 0 && 
        (outDegree.get(node.id) || 0) === 0
      ).length;
    
    const totalDependencies = edges.length;
    const avgDependencies = nodes.length > 0 ? totalDependencies / nodes.length : 0;
    
    return {
      totalNodes: nodes.length,
      totalEdges: edges.length,
      maxDepth: this.calculateMaxDepth(nodes, edges),
      avgDependencies,
      circularDependencies: cycles.length,
      isolatedNodes,
      hubNodes,
      leafNodes
    };
  }
  
  private calculateMaxDepth(nodes: DependencyNode[], edges: DependencyEdge[]): number {
    // Build adjacency list
    const adjacency = new Map<string, string[]>();
    nodes.forEach(node => adjacency.set(node.id, []));
    
    edges.forEach(edge => {
      const list = adjacency.get(edge.source) || [];
      list.push(edge.target);
      adjacency.set(edge.source, list);
    });
    
    // Find roots (nodes with no incoming edges)
    const hasIncoming = new Set(edges.map(e => e.target));
    const roots = nodes.filter(n => !hasIncoming.has(n.id));
    
    // BFS to find max depth
    let maxDepth = 0;
    const visited = new Set<string>();
    
    const bfs = (startId: string) => {
      const queue: { id: string; depth: number }[] = [{ id: startId, depth: 0 }];
      
      while (queue.length > 0) {
        const { id, depth } = queue.shift()!;
        
        if (visited.has(id)) continue;
        visited.add(id);
        
        maxDepth = Math.max(maxDepth, depth);
        
        const neighbors = adjacency.get(id) || [];
        neighbors.forEach(neighborId => {
          if (!visited.has(neighborId)) {
            queue.push({ id: neighborId, depth: depth + 1 });
          }
        });
      }
    };
    
    roots.forEach(root => bfs(root.id));
    
    // Handle disconnected components
    nodes.forEach(node => {
      if (!visited.has(node.id)) {
        bfs(node.id);
      }
    });
    
    return maxDepth;
  }
}