/**
 * AI Graph Interface
 * 
 * Provides AI-friendly interface to query and update the knowledge graph
 */

import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import { OrchestrationEngine } from '../../orchestration/orchestration-engine';
import {
  AIGraphQuery,
  AIGraphResponse,
  GraphNode,
  GraphRelationship,
  CodeExample,
  DependencyInfo,
  ImpactInfo,
  NodeType,
  RelationType,
  GraphError,
  GraphErrorCode,
  GraphUpdate,
  ProjectContext
} from '../types/graph-types';

export interface AIGraphInterfaceConfig {
  maxQueryDepth: number;
  maxResultNodes: number;
  includeCodeByDefault: boolean;
  enableNaturalLanguageQueries: boolean;
  cacheQueryResults: boolean;
}

export class AIGraphInterface {
  private connectionManager: Neo4jConnectionManager;
  private orchestrationEngine: OrchestrationEngine;
  private config: AIGraphInterfaceConfig;
  private queryCache: Map<string, AIGraphResponse> = new Map();
  
  constructor(
    connectionManager: Neo4jConnectionManager,
    orchestrationEngine: OrchestrationEngine,
    config: Partial<AIGraphInterfaceConfig> = {}
  ) {
    this.connectionManager = connectionManager;
    this.orchestrationEngine = orchestrationEngine;
    
    this.config = {
      maxQueryDepth: config.maxQueryDepth || 3,
      maxResultNodes: config.maxResultNodes || 100,
      includeCodeByDefault: config.includeCodeByDefault ?? true,
      enableNaturalLanguageQueries: config.enableNaturalLanguageQueries ?? true,
      cacheQueryResults: config.cacheQueryResults ?? true
    };
  }
  
  /**
   * Query the knowledge graph using natural language
   */
  async query(query: AIGraphQuery): Promise<AIGraphResponse> {
    // Check cache
    const cacheKey = this.getCacheKey(query);
    if (this.config.cacheQueryResults && this.queryCache.has(cacheKey)) {
      return this.queryCache.get(cacheKey)!;
    }
    
    const startTime = Date.now();
    
    try {
      // Determine which database to query
      const databaseId = this.getDatabaseId(query.context.projectId);
      
      // Convert natural language to Cypher if needed
      let cypherQuery: string;
      let parameters: Record<string, any> = {};
      
      if (this.config.enableNaturalLanguageQueries) {
        const conversion = await this.convertToCypher(query);
        cypherQuery = conversion.cypher;
        parameters = conversion.parameters;
      } else {
        // Direct Cypher query
        cypherQuery = query.query;
      }
      
      // Execute the query
      const result = await this.connectionManager.executeQuery(
        databaseId,
        cypherQuery,
        parameters
      );
      
      // Process results
      const nodes: GraphNode[] = [];
      const relationships: GraphRelationship[] = [];
      const nodeMap = new Map<string, GraphNode>();
      
      result.records.forEach(record => {
        record.keys.forEach(key => {
          const value = record.get(key);
          
          if (this.isNode(value)) {
            const node = this.convertNode(value);
            nodes.push(node);
            nodeMap.set(node.id, node);
          } else if (this.isRelationship(value)) {
            relationships.push(this.convertRelationship(value));
          }
        });
      });
      
      // Build structured response
      const response = await this.buildAIResponse(
        query,
        nodes,
        relationships,
        nodeMap,
        databaseId
      );
      
      response.queryExecutionTime = Date.now() - startTime;
      
      // Cache the response
      if (this.config.cacheQueryResults) {
        this.queryCache.set(cacheKey, response);
      }
      
      return response;
      
    } catch (error) {
      throw new GraphError(
        'Failed to execute AI graph query',
        GraphErrorCode.QUERY_FAILED,
        { query, error }
      );
    }
  }
  
  /**
   * Update the graph based on AI-generated code changes
   */
  async updateFromAIChanges(
    projectId: string,
    changes: AICodeChange[]
  ): Promise<{ success: boolean; updatesApplied: number }> {
    const databaseId = this.getDatabaseId(projectId);
    const updates: GraphUpdate[] = [];
    
    for (const change of changes) {
      switch (change.type) {
        case 'create':
          updates.push(...await this.handleCreateChange(change, databaseId));
          break;
          
        case 'modify':
          updates.push(...await this.handleModifyChange(change, databaseId));
          break;
          
        case 'delete':
          updates.push(...await this.handleDeleteChange(change, databaseId));
          break;
          
        case 'refactor':
          updates.push(...await this.handleRefactorChange(change, databaseId));
          break;
      }
    }
    
    // Apply all updates in a transaction
    await this.applyUpdates(databaseId, updates);
    
    // Clear cache after updates
    this.queryCache.clear();
    
    return {
      success: true,
      updatesApplied: updates.length
    };
  }
  
  /**
   * Get context for the current code location
   */
  async getContextForLocation(
    projectId: string,
    filePath: string,
    line: number
  ): Promise<AICodeContext> {
    const databaseId = this.getDatabaseId(projectId);
    
    // Find the most specific entity at this location
    const result = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (f:File {path: $filePath})-[:CONTAINS*]->(n)
      WHERE n.startLine <= $line AND n.endLine >= $line
      RETURN n
      ORDER BY n.endLine - n.startLine ASC
      LIMIT 1
      `,
      { filePath, line }
    );
    
    if (result.records.length === 0) {
      throw new GraphError(
        'No code entity found at location',
        GraphErrorCode.INVALID_OPERATION
      );
    }
    
    const mainEntity = this.convertNode(result.records[0].get('n'));
    
    // Get surrounding context
    const context = await this.gatherContext(mainEntity, databaseId);
    
    return context;
  }
  
  /**
   * Analyze impact of proposed changes
   */
  async analyzeImpact(
    projectId: string,
    entityId: string,
    changeType: 'modify' | 'delete' | 'rename'
  ): Promise<ImpactAnalysis> {
    const databaseId = this.getDatabaseId(projectId);
    
    // Find all entities that depend on this one
    const result = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (target {id: $entityId})
      OPTIONAL MATCH (target)<-[:CALLS|REFERENCES|IMPORTS|EXTENDS|IMPLEMENTS]-(dependent)
      OPTIONAL MATCH (target)-[:CONTAINS]->(child)
      RETURN 
        collect(DISTINCT dependent) as dependents,
        collect(DISTINCT child) as children,
        target
      `,
      { entityId }
    );
    
    if (result.records.length === 0) {
      throw new GraphError(
        'Entity not found',
        GraphErrorCode.INVALID_OPERATION
      );
    }
    
    const record = result.records[0];
    const target = this.convertNode(record.get('target'));
    const dependents = record.get('dependents').map((n: any) => this.convertNode(n));
    const children = record.get('children').map((n: any) => this.convertNode(n));
    
    // Analyze impact
    const impacts: ImpactInfo[] = [];
    
    // Direct impacts on dependents
    for (const dependent of dependents) {
      impacts.push({
        entity: dependent,
        impactType: 'direct',
        description: this.getImpactDescription(changeType, target, dependent),
        severity: this.calculateImpactSeverity(changeType, target, dependent)
      });
    }
    
    // Impact on children (for deletions)
    if (changeType === 'delete') {
      for (const child of children) {
        impacts.push({
          entity: child,
          impactType: 'direct',
          description: `Will be deleted along with parent ${target.properties.name}`,
          severity: 'high'
        });
      }
    }
    
    // Find indirect impacts
    const indirectImpacts = await this.findIndirectImpacts(
      dependents.map(d => d.id),
      databaseId
    );
    
    impacts.push(...indirectImpacts);
    
    return {
      targetEntity: target,
      changeType,
      impacts,
      riskLevel: this.calculateOverallRisk(impacts),
      suggestions: this.generateMitigationSuggestions(impacts)
    };
  }
  
  /**
   * Get project context including statistics and patterns
   */
  async getProjectContext(projectId: string): Promise<ProjectContext> {
    const databaseId = this.getDatabaseId(projectId);
    const database = this.connectionManager.getDatabase(databaseId);
    
    if (!database) {
      throw new GraphError(
        'Database not found',
        GraphErrorCode.CONNECTION_FAILED
      );
    }
    
    // Get languages used
    const langResult = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (f:File)
      RETURN DISTINCT f.language as language, count(f) as count
      ORDER BY count DESC
      `
    );
    
    const languages = langResult.records.map(r => r.get('language') as string);
    
    // Get AI access patterns
    const patterns = await this.getAIAccessPatterns(databaseId);
    
    return {
      projectId,
      workspacePath: database.config.projectId || '',
      graphDatabase: databaseId,
      languages,
      statistics: database.statistics,
      indexingStatus: {
        status: 'idle',
        lastIndexed: new Date()
      },
      aiAccessPatterns: patterns
    };
  }
  
  // Private helper methods
  
  private async convertToCypher(query: AIGraphQuery): Promise<{
    cypher: string;
    parameters: Record<string, any>;
  }> {
    // Use the orchestration engine to convert natural language to Cypher
    const prompt = `
Convert this natural language query to a Neo4j Cypher query:

Query: ${query.query}
Context: ${JSON.stringify(query.context)}
Requirements: ${JSON.stringify(query.requirements)}

Available node types: File, Class, Function, Method, Variable, Interface, Type, Module, Package
Available relationships: CONTAINS, IMPORTS, EXPORTS, CALLS, EXTENDS, IMPLEMENTS, REFERENCES, DEPENDS_ON

Return only the Cypher query without explanation.
`;
    
    const result = await this.orchestrationEngine.execute({
      type: 'code_generation',
      prompt,
      context: '',
      constraints: {
        language: 'cypher',
        style: 'neo4j'
      }
    });
    
    // Parse the response to extract query and parameters
    const cypherQuery = result.content.trim();
    const parameters: Record<string, any> = {};
    
    // Extract parameters from context
    if (query.context.currentFile) {
      parameters.currentFile = query.context.currentFile;
    }
    if (query.context.selectedCode) {
      parameters.selectedCode = query.context.selectedCode;
    }
    
    return { cypher: cypherQuery, parameters };
  }
  
  private async buildAIResponse(
    query: AIGraphQuery,
    nodes: GraphNode[],
    relationships: GraphRelationship[],
    nodeMap: Map<string, GraphNode>,
    databaseId: string
  ): Promise<AIGraphResponse> {
    // Find the main entity if possible
    let mainEntity: GraphNode | undefined;
    if (query.context.currentFile) {
      mainEntity = nodes.find(n => 
        n.properties.filePath === query.context.currentFile ||
        n.properties.path === query.context.currentFile
      );
    }
    
    // Get code examples if requested
    const codeExamples: CodeExample[] = [];
    if (query.requirements.includeUsageExamples) {
      codeExamples.push(...await this.findCodeExamples(nodes, databaseId));
    }
    
    // Get dependencies if requested
    const dependencies: DependencyInfo[] = [];
    if (query.requirements.includeDependencies) {
      dependencies.push(...await this.findDependencies(nodes, databaseId));
    }
    
    // Generate answer using AI
    const answer = await this.generateAnswer(query, nodes, relationships);
    
    return {
      answer,
      entities: nodes.slice(0, this.config.maxResultNodes),
      relationships,
      context: {
        mainEntity,
        relatedEntities: nodes.filter(n => n !== mainEntity),
        codeExamples,
        dependencies,
        impacts: []
      },
      confidence: 0.9, // Would calculate based on query complexity
      queryExecutionTime: 0, // Set by caller
      suggestions: this.generateSuggestions(query, nodes)
    };
  }
  
  private async generateAnswer(
    query: AIGraphQuery,
    nodes: GraphNode[],
    relationships: GraphRelationship[]
  ): Promise<string> {
    // Use orchestration engine to generate natural language answer
    const prompt = `
Based on the following graph data, answer this question: ${query.query}

Nodes found: ${nodes.length}
${nodes.slice(0, 10).map(n => `- ${n.labels[0]}: ${n.properties.name}`).join('\n')}

Relationships found: ${relationships.length}
${relationships.slice(0, 10).map(r => `- ${r.type}`).join('\n')}

Provide a clear, concise answer focusing on what the user asked.
`;
    
    const result = await this.orchestrationEngine.execute({
      type: 'question_answering',
      prompt,
      context: JSON.stringify({ nodes: nodes.slice(0, 20), relationships: relationships.slice(0, 20) }),
      constraints: {
        maxTokens: 500
      }
    });
    
    return result.content;
  }
  
  private async findCodeExamples(
    nodes: GraphNode[],
    databaseId: string
  ): Promise<CodeExample[]> {
    const examples: CodeExample[] = [];
    
    // Get code snippets for functions and methods
    const codeEntities = nodes.filter(n => 
      n.labels.includes(NodeType.FUNCTION) || 
      n.labels.includes(NodeType.METHOD)
    ).slice(0, 5);
    
    for (const entity of codeEntities) {
      // In a real implementation, we'd fetch the actual code
      examples.push({
        file: entity.properties.filePath,
        startLine: entity.properties.startLine,
        endLine: entity.properties.endLine,
        code: `// Code for ${entity.properties.name}`,
        description: `Implementation of ${entity.properties.name}`
      });
    }
    
    return examples;
  }
  
  private async findDependencies(
    nodes: GraphNode[],
    databaseId: string
  ): Promise<DependencyInfo[]> {
    const dependencies: DependencyInfo[] = [];
    
    // Find package dependencies
    const packages = nodes.filter(n => n.labels.includes(NodeType.PACKAGE));
    
    for (const pkg of packages) {
      dependencies.push({
        name: pkg.properties.name,
        type: 'direct',
        version: pkg.properties.version,
        usage: [`Used by ${nodes.length} entities`]
      });
    }
    
    return dependencies;
  }
  
  private generateSuggestions(query: AIGraphQuery, nodes: GraphNode[]): string[] {
    const suggestions: string[] = [];
    
    // Suggest related queries based on found entities
    if (nodes.length > 0) {
      const entityTypes = new Set(nodes.flatMap(n => n.labels));
      
      if (entityTypes.has(NodeType.CLASS)) {
        suggestions.push('Show me all methods in this class');
        suggestions.push('What classes extend this class?');
      }
      
      if (entityTypes.has(NodeType.FUNCTION)) {
        suggestions.push('What functions call this function?');
        suggestions.push('Show me the complexity of this function');
      }
    }
    
    return suggestions;
  }
  
  private getDatabaseId(projectId?: string): string {
    if (projectId) {
      const projectDb = this.connectionManager.getProjectDatabase(projectId);
      if (projectDb) {
        return projectDb.id;
      }
    }
    
    // Fall back to global database
    const globalDb = this.connectionManager.getDatabasesByContext('global')[0];
    if (globalDb) {
      return globalDb.id;
    }
    
    throw new GraphError(
      'No database available',
      GraphErrorCode.CONNECTION_FAILED
    );
  }
  
  private getCacheKey(query: AIGraphQuery): string {
    return JSON.stringify({
      query: query.query,
      context: query.context,
      requirements: query.requirements
    });
  }
  
  private isNode(value: any): boolean {
    return value && typeof value === 'object' && 'labels' in value && 'properties' in value;
  }
  
  private isRelationship(value: any): boolean {
    return value && typeof value === 'object' && 'type' in value && 'start' in value && 'end' in value;
  }
  
  private convertNode(neo4jNode: any): GraphNode {
    return {
      id: neo4jNode.properties.id,
      labels: neo4jNode.labels,
      properties: neo4jNode.properties
    };
  }
  
  private convertRelationship(neo4jRel: any): GraphRelationship {
    return {
      id: neo4jRel.identity.toString(),
      type: neo4jRel.type,
      startNodeId: neo4jRel.start.toString(),
      endNodeId: neo4jRel.end.toString(),
      properties: neo4jRel.properties
    };
  }
  
  private async gatherContext(
    entity: GraphNode,
    databaseId: string
  ): Promise<AICodeContext> {
    // Get parent context
    const parentResult = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (n {id: $entityId})<-[:CONTAINS]-(parent)
      RETURN parent
      LIMIT 1
      `,
      { entityId: entity.id }
    );
    
    const parent = parentResult.records.length > 0
      ? this.convertNode(parentResult.records[0].get('parent'))
      : undefined;
    
    // Get siblings
    const siblingsResult = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (n {id: $entityId})<-[:CONTAINS]-(parent)-[:CONTAINS]->(sibling)
      WHERE sibling.id <> $entityId
      RETURN sibling
      LIMIT 10
      `,
      { entityId: entity.id }
    );
    
    const siblings = siblingsResult.records.map(r => this.convertNode(r.get('sibling')));
    
    // Get dependencies
    const depsResult = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (n {id: $entityId})-[:IMPORTS|CALLS|REFERENCES]->(dep)
      RETURN DISTINCT dep
      LIMIT 10
      `,
      { entityId: entity.id }
    );
    
    const dependencies = depsResult.records.map(r => this.convertNode(r.get('dep')));
    
    return {
      entity,
      parent,
      siblings,
      dependencies,
      file: entity.properties.filePath,
      language: this.getLanguageFromEntity(entity)
    };
  }
  
  private getLanguageFromEntity(entity: GraphNode): string {
    // Infer language from file extension or entity properties
    const filePath = entity.properties.filePath || entity.properties.path;
    if (filePath) {
      if (filePath.endsWith('.ts') || filePath.endsWith('.tsx')) return 'typescript';
      if (filePath.endsWith('.js') || filePath.endsWith('.jsx')) return 'javascript';
    }
    return 'unknown';
  }
  
  private async handleCreateChange(
    change: AICodeChange,
    databaseId: string
  ): Promise<GraphUpdate[]> {
    // Parse the new code and create graph updates
    // This would use the appropriate parser based on language
    return [];
  }
  
  private async handleModifyChange(
    change: AICodeChange,
    databaseId: string
  ): Promise<GraphUpdate[]> {
    // Update existing entities
    return [];
  }
  
  private async handleDeleteChange(
    change: AICodeChange,
    databaseId: string
  ): Promise<GraphUpdate[]> {
    // Create delete updates
    return [];
  }
  
  private async handleRefactorChange(
    change: AICodeChange,
    databaseId: string
  ): Promise<GraphUpdate[]> {
    // Handle complex refactoring like renames, moves
    return [];
  }
  
  private async applyUpdates(
    databaseId: string,
    updates: GraphUpdate[]
  ): Promise<void> {
    // Apply updates in a transaction
    await this.connectionManager.executeTransaction(databaseId, async (tx) => {
      for (const update of updates) {
        // Apply each update
      }
    });
  }
  
  private getImpactDescription(
    changeType: string,
    target: GraphNode,
    dependent: GraphNode
  ): string {
    const targetName = target.properties.name;
    const dependentName = dependent.properties.name;
    
    switch (changeType) {
      case 'modify':
        return `${dependentName} depends on ${targetName} and may need updates`;
      case 'delete':
        return `${dependentName} will lose its dependency on ${targetName}`;
      case 'rename':
        return `${dependentName} must update its reference to ${targetName}`;
      default:
        return `${dependentName} is affected by changes to ${targetName}`;
    }
  }
  
  private calculateImpactSeverity(
    changeType: string,
    target: GraphNode,
    dependent: GraphNode
  ): 'low' | 'medium' | 'high' {
    // Simplified severity calculation
    if (changeType === 'delete') return 'high';
    if (dependent.labels.includes(NodeType.TEST)) return 'low';
    return 'medium';
  }
  
  private async findIndirectImpacts(
    directImpactIds: string[],
    databaseId: string
  ): Promise<ImpactInfo[]> {
    // Find entities that depend on the directly impacted ones
    return [];
  }
  
  private calculateOverallRisk(impacts: ImpactInfo[]): 'low' | 'medium' | 'high' {
    const highCount = impacts.filter(i => i.severity === 'high').length;
    const mediumCount = impacts.filter(i => i.severity === 'medium').length;
    
    if (highCount > 0) return 'high';
    if (mediumCount > 2) return 'high';
    if (mediumCount > 0) return 'medium';
    return 'low';
  }
  
  private generateMitigationSuggestions(impacts: ImpactInfo[]): string[] {
    const suggestions: string[] = [];
    
    const highImpacts = impacts.filter(i => i.severity === 'high');
    if (highImpacts.length > 0) {
      suggestions.push('Consider breaking this change into smaller, incremental updates');
      suggestions.push('Add comprehensive tests before making this change');
    }
    
    return suggestions;
  }
  
  private async getAIAccessPatterns(databaseId: string): Promise<any> {
    // This would track actual AI access patterns
    return {
      frequentQueries: [],
      hotspots: [],
      recentChanges: []
    };
  }
}

// Supporting types
interface AICodeChange {
  type: 'create' | 'modify' | 'delete' | 'refactor';
  entityId?: string;
  filePath: string;
  code?: string;
  metadata?: Record<string, any>;
}

interface AICodeContext {
  entity: GraphNode;
  parent?: GraphNode;
  siblings: GraphNode[];
  dependencies: GraphNode[];
  file: string;
  language: string;
}

interface ImpactAnalysis {
  targetEntity: GraphNode;
  changeType: string;
  impacts: ImpactInfo[];
  riskLevel: 'low' | 'medium' | 'high';
  suggestions: string[];
}