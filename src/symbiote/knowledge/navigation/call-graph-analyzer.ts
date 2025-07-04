/**
 * Call Graph Analyzer
 * 
 * Analyze function call hierarchies and execution flows
 */

import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import {
  GraphNode,
  NodeType,
  RelationType,
  FunctionNodeProperties,
  CallRelationshipProperties
} from '../types/graph-types';

export interface CallNode {
  id: string;
  name: string;
  signature: string;
  filePath: string;
  line: number;
  complexity?: number;
  isAsync?: boolean;
  callCount: number;
  callerCount: number;
  depth: number;
}

export interface CallEdge {
  source: string;
  target: string;
  callCount: number;
  isAsync: boolean;
  parameters?: any[];
  location: {
    file: string;
    line: number;
  };
}

export interface CallHierarchy {
  root: CallNode;
  nodes: Map<string, CallNode>;
  edges: CallEdge[];
  maxDepth: number;
  totalCalls: number;
}

export interface ExecutionPath {
  nodes: CallNode[];
  edges: CallEdge[];
  isAsync: boolean;
  hasCycles: boolean;
  estimatedTime?: number;
}

export interface HotPath {
  path: ExecutionPath;
  frequency: number;
  avgExecutionTime?: number;
  bottlenecks: CallNode[];
}

export class CallGraphAnalyzer {
  constructor(private connectionManager: Neo4jConnectionManager) {}
  
  /**
   * Get complete call hierarchy for a function
   */
  async getCallHierarchy(
    functionId: string,
    databaseId: string,
    maxDepth: number = 5
  ): Promise<CallHierarchy> {
    const nodes = new Map<string, CallNode>();
    const edges: CallEdge[] = [];
    let totalCalls = 0;
    
    // Get root function
    const rootResult = await this.connectionManager.executeQuery(
      databaseId,
      'MATCH (f:Function {id: $id}) RETURN f',
      { id: functionId }
    );
    
    if (rootResult.records.length === 0) {
      throw new Error('Function not found');
    }
    
    const rootNode = this.createCallNode(
      rootResult.records[0].get('f'),
      0,
      0,
      0
    );
    nodes.set(rootNode.id, rootNode);
    
    // Recursively build call hierarchy
    await this.buildCallHierarchy(
      functionId,
      databaseId,
      nodes,
      edges,
      0,
      maxDepth,
      new Set([functionId])
    );
    
    // Calculate total calls
    edges.forEach(edge => {
      totalCalls += edge.callCount;
    });
    
    return {
      root: rootNode,
      nodes,
      edges,
      maxDepth: Math.max(...Array.from(nodes.values()).map(n => n.depth)),
      totalCalls
    };
  }
  
  /**
   * Find all execution paths between two functions
   */
  async findExecutionPaths(
    startId: string,
    endId: string,
    databaseId: string,
    maxPaths: number = 10
  ): Promise<ExecutionPath[]> {
    const query = `
      MATCH path = (start:Function {id: $startId})-[:CALLS*1..10]->(end:Function {id: $endId})
      WITH path, [r in relationships(path) | r] as rels
      RETURN path, rels
      LIMIT $maxPaths
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { startId, endId, maxPaths }
    );
    
    const paths: ExecutionPath[] = [];
    
    for (const record of result.records) {
      const pathNodes = record.get('path').segments.map((segment: any) => 
        this.createCallNode(segment.end, 0, 0, 0)
      );
      
      const pathEdges: CallEdge[] = [];
      const relationships = record.get('rels');
      
      for (let i = 0; i < relationships.length; i++) {
        const rel = relationships[i];
        pathEdges.push({
          source: pathNodes[i].id,
          target: pathNodes[i + 1].id,
          callCount: rel.properties.callCount || 1,
          isAsync: rel.properties.isAsync || false,
          parameters: rel.properties.parameters,
          location: {
            file: rel.properties.filePath,
            line: rel.properties.line
          }
        });
      }
      
      const isAsync = pathEdges.some(e => e.isAsync);
      const hasCycles = this.detectCyclesInPath(pathNodes);
      
      paths.push({
        nodes: pathNodes,
        edges: pathEdges,
        isAsync,
        hasCycles
      });
    }
    
    return paths;
  }
  
  /**
   * Find hot paths (frequently executed paths)
   */
  async findHotPaths(
    databaseId: string,
    threshold: number = 100
  ): Promise<HotPath[]> {
    const query = `
      MATCH (f1:Function)-[c:CALLS]->(f2:Function)
      WHERE c.callCount >= $threshold
      WITH f1, f2, c
      ORDER BY c.callCount DESC
      LIMIT 50
      MATCH path = (f1)-[:CALLS*1..3]->(fn:Function)
      WITH path, reduce(count = 0, r in relationships(path) | count + r.callCount) as totalCalls
      WHERE totalCalls >= $threshold
      RETURN path, totalCalls
      ORDER BY totalCalls DESC
      LIMIT 20
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { threshold }
    );
    
    const hotPaths: HotPath[] = [];
    
    for (const record of result.records) {
      const path = record.get('path');
      const frequency = record.get('totalCalls').toNumber();
      
      const nodes: CallNode[] = path.segments.map((segment: any, index: number) => 
        this.createCallNode(segment.end, 0, 0, index)
      );
      
      const edges: CallEdge[] = path.segments.map((segment: any, index: number) => ({
        source: index === 0 ? this.createCallNode(segment.start, 0, 0, 0).id : nodes[index - 1].id,
        target: nodes[index].id,
        callCount: segment.relationship.properties.callCount || 1,
        isAsync: segment.relationship.properties.isAsync || false,
        location: {
          file: segment.relationship.properties.filePath,
          line: segment.relationship.properties.line
        }
      }));
      
      // Identify bottlenecks (high complexity or many calls)
      const bottlenecks = nodes.filter(node => 
        (node.complexity && node.complexity > 10) || node.callCount > threshold
      );
      
      hotPaths.push({
        path: {
          nodes,
          edges,
          isAsync: edges.some(e => e.isAsync),
          hasCycles: false
        },
        frequency,
        bottlenecks
      });
    }
    
    return hotPaths;
  }
  
  /**
   * Analyze recursion patterns
   */
  async findRecursiveFunctions(
    databaseId: string
  ): Promise<{
    function: CallNode;
    recursionType: 'direct' | 'indirect';
    cycle: string[];
  }[]> {
    // Direct recursion
    const directQuery = `
      MATCH (f:Function)-[:CALLS]->(f)
      RETURN f
    `;
    
    // Indirect recursion (cycles)
    const indirectQuery = `
      MATCH path = (f:Function)-[:CALLS*2..]->(f)
      WITH f, path, [n in nodes(path) | n.name] as cycle
      RETURN DISTINCT f, cycle
      LIMIT 50
    `;
    
    const [directResult, indirectResult] = await Promise.all([
      this.connectionManager.executeQuery(databaseId, directQuery),
      this.connectionManager.executeQuery(databaseId, indirectQuery)
    ]);
    
    const recursiveFunctions: {
      function: CallNode;
      recursionType: 'direct' | 'indirect';
      cycle: string[];
    }[] = [];
    
    // Process direct recursion
    directResult.records.forEach(record => {
      const func = record.get('f');
      const node = this.createCallNode(func, 0, 0, 0);
      recursiveFunctions.push({
        function: node,
        recursionType: 'direct',
        cycle: [node.name]
      });
    });
    
    // Process indirect recursion
    indirectResult.records.forEach(record => {
      const func = record.get('f');
      const cycle = record.get('cycle');
      const node = this.createCallNode(func, 0, 0, 0);
      
      recursiveFunctions.push({
        function: node,
        recursionType: 'indirect',
        cycle
      });
    });
    
    return recursiveFunctions;
  }
  
  /**
   * Find dead code (uncalled functions)
   */
  async findDeadCode(
    databaseId: string,
    projectPath: string
  ): Promise<CallNode[]> {
    const query = `
      MATCH (f:Function)
      WHERE f.filePath STARTS WITH $projectPath
      AND NOT (f)<-[:CALLS]-()
      AND NOT f.name IN ['main', 'init', 'constructor', 'destructor']
      AND NOT f.name =~ '.*Test$'
      AND NOT exists(f.isExported)
      RETURN f
      ORDER BY f.filePath, f.startLine
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { projectPath }
    );
    
    return result.records.map(record => 
      this.createCallNode(record.get('f'), 0, 0, 0)
    );
  }
  
  /**
   * Analyze function complexity
   */
  async analyzeFunctionComplexity(
    functionId: string,
    databaseId: string
  ): Promise<{
    complexity: number;
    callsCount: number;
    calledByCount: number;
    cyclomaticComplexity: number;
    cognitiveComplexity: number;
    halsteadMetrics?: {
      vocabulary: number;
      volume: number;
      difficulty: number;
      effort: number;
    };
  }> {
    const query = `
      MATCH (f:Function {id: $id})
      OPTIONAL MATCH (f)-[:CALLS]->(called)
      WITH f, count(distinct called) as callsCount
      OPTIONAL MATCH (caller)-[:CALLS]->(f)
      WITH f, callsCount, count(distinct caller) as calledByCount
      RETURN f, callsCount, calledByCount
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: functionId }
    );
    
    if (result.records.length === 0) {
      throw new Error('Function not found');
    }
    
    const record = result.records[0];
    const func = record.get('f');
    const callsCount = record.get('callsCount').toNumber();
    const calledByCount = record.get('calledByCount').toNumber();
    
    // Calculate complexity metrics
    const complexity = func.properties.complexity || 1;
    const cyclomaticComplexity = func.properties.cyclomaticComplexity || complexity;
    const cognitiveComplexity = func.properties.cognitiveComplexity || complexity;
    
    return {
      complexity,
      callsCount,
      calledByCount,
      cyclomaticComplexity,
      cognitiveComplexity
    };
  }
  
  /**
   * Get call chain to a specific function
   */
  async getCallChain(
    targetId: string,
    databaseId: string,
    fromEntry: boolean = true
  ): Promise<ExecutionPath[]> {
    let query: string;
    
    if (fromEntry) {
      // Find paths from entry points to target
      query = `
        MATCH (entry:Function)
        WHERE entry.isEntry = true OR entry.name IN ['main', 'init', 'start']
        MATCH path = shortestPath((entry)-[:CALLS*]->(target:Function {id: $targetId}))
        RETURN path
        LIMIT 10
      `;
    } else {
      // Find paths from target to exit points
      query = `
        MATCH (target:Function {id: $targetId})
        MATCH (exit:Function)
        WHERE exit.isExit = true OR exit.name IN ['exit', 'return', 'throw']
        MATCH path = shortestPath((target)-[:CALLS*]->(exit))
        RETURN path
        LIMIT 10
      `;
    }
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { targetId }
    );
    
    const chains: ExecutionPath[] = [];
    
    for (const record of result.records) {
      const path = record.get('path');
      const nodes: CallNode[] = [];
      const edges: CallEdge[] = [];
      
      // Process path segments
      let previousNode: CallNode | null = null;
      
      path.segments.forEach((segment: any, index: number) => {
        if (index === 0) {
          previousNode = this.createCallNode(segment.start, 0, 0, 0);
          nodes.push(previousNode);
        }
        
        const currentNode = this.createCallNode(segment.end, 0, 0, index + 1);
        nodes.push(currentNode);
        
        edges.push({
          source: previousNode!.id,
          target: currentNode.id,
          callCount: segment.relationship.properties.callCount || 1,
          isAsync: segment.relationship.properties.isAsync || false,
          location: {
            file: segment.relationship.properties.filePath,
            line: segment.relationship.properties.line
          }
        });
        
        previousNode = currentNode;
      });
      
      chains.push({
        nodes,
        edges,
        isAsync: edges.some(e => e.isAsync),
        hasCycles: false
      });
    }
    
    return chains;
  }
  
  // Private helper methods
  
  private async buildCallHierarchy(
    functionId: string,
    databaseId: string,
    nodes: Map<string, CallNode>,
    edges: CallEdge[],
    currentDepth: number,
    maxDepth: number,
    visited: Set<string>
  ): Promise<void> {
    if (currentDepth >= maxDepth) return;
    
    // Get functions called by this function
    const query = `
      MATCH (f:Function {id: $id})-[c:CALLS]->(called:Function)
      OPTIONAL MATCH (called)<-[cc:CALLS]-(caller)
      WITH called, c, count(distinct caller) as callerCount
      RETURN called, c, callerCount
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query,
      { id: functionId }
    );
    
    for (const record of result.records) {
      const called = record.get('called');
      const callRel = record.get('c');
      const callerCount = record.get('callerCount').toNumber();
      
      const calledId = called.properties.id;
      
      // Create node if not exists
      if (!nodes.has(calledId)) {
        const callNode = this.createCallNode(
          called,
          0, // Will calculate call count later
          callerCount,
          currentDepth + 1
        );
        nodes.set(calledId, callNode);
      }
      
      // Create edge
      edges.push({
        source: functionId,
        target: calledId,
        callCount: callRel.properties.callCount || 1,
        isAsync: callRel.properties.isAsync || false,
        parameters: callRel.properties.parameters,
        location: {
          file: callRel.properties.filePath,
          line: callRel.properties.line
        }
      });
      
      // Recurse if not visited
      if (!visited.has(calledId)) {
        visited.add(calledId);
        await this.buildCallHierarchy(
          calledId,
          databaseId,
          nodes,
          edges,
          currentDepth + 1,
          maxDepth,
          visited
        );
      }
    }
  }
  
  private createCallNode(
    neo4jNode: any,
    callCount: number,
    callerCount: number,
    depth: number
  ): CallNode {
    const props = neo4jNode.properties as FunctionNodeProperties;
    
    return {
      id: props.id,
      name: props.name,
      signature: props.signature,
      filePath: props.filePath,
      line: props.startLine,
      complexity: props.complexity,
      isAsync: props.isAsync,
      callCount,
      callerCount,
      depth
    };
  }
  
  private detectCyclesInPath(nodes: CallNode[]): boolean {
    const seen = new Set<string>();
    
    for (const node of nodes) {
      if (seen.has(node.id)) {
        return true;
      }
      seen.add(node.id);
    }
    
    return false;
  }
}