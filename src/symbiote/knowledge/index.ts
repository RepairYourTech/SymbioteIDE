/**
 * Knowledge Graph Module
 * 
 * Provides Neo4j-based code intelligence for AI understanding
 */

export * from './types/graph-types';
export * from './neo4j/connection-manager';
export * from './neo4j/schema-manager';
export * from './parsers/typescript-parser';
export * from './ingestion/ingestion-pipeline';
export * from './sync/graph-sync-engine';
export * from './ai-integration/ai-graph-interface';

import { Neo4jConnectionManager, ConnectionManagerConfig } from './neo4j/connection-manager';
import { GraphSchemaManager } from './neo4j/schema-manager';
import { GraphIngestionPipeline } from './ingestion/ingestion-pipeline';
import { GraphSyncEngine } from './sync/graph-sync-engine';
import { AIGraphInterface } from './ai-integration/ai-graph-interface';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { KnowledgeAPIClient } from '../../types/knowledge-api';
import {
  KnowledgeEntity,
  KnowledgeRelation,
  KnowledgeQuery,
  KnowledgeSearchResult,
  KnowledgeIndex,
  GraphPath,
  GraphSchema,
  GraphVisualization,
  VectorSearchRequest,
  VectorSearchResult,
  VectorBatch,
  GraphQuery,
  GraphSearchResult,
  GraphMetrics
} from '../../types/knowledge-api';
import {
  GraphNode,
  GraphRelationship,
  NodeType,
  RelationType,
  ProjectContext,
  AIGraphQuery,
  AIGraphResponse
} from './types/graph-types';

/**
 * Main knowledge graph system configuration
 */
export interface KnowledgeSystemConfig {
  connectionConfig?: Partial<ConnectionManagerConfig>;
  ingestionConfig?: {
    batchSize?: number;
    parallelWorkers?: number;
    fileExtensions?: string[];
  };
  syncConfig?: {
    debounceDelay?: number;
    autoStart?: boolean;
  };
  aiConfig?: {
    enableNaturalLanguageQueries?: boolean;
    maxQueryDepth?: number;
  };
}

/**
 * Knowledge Graph System
 * 
 * Central system for managing code intelligence through Neo4j
 */
export class KnowledgeGraphSystem implements KnowledgeAPIClient {
  private connectionManager: Neo4jConnectionManager;
  private schemaManager: GraphSchemaManager;
  private ingestionPipeline: GraphIngestionPipeline;
  private syncEngine: GraphSyncEngine;
  private aiInterface: AIGraphInterface;
  private orchestrationEngine: OrchestrationEngine;
  
  constructor(
    orchestrationEngine: OrchestrationEngine,
    config: KnowledgeSystemConfig = {}
  ) {
    this.orchestrationEngine = orchestrationEngine;
    
    // Initialize components
    this.connectionManager = new Neo4jConnectionManager(config.connectionConfig);
    this.schemaManager = new GraphSchemaManager(this.connectionManager);
    this.ingestionPipeline = new GraphIngestionPipeline(
      this.connectionManager,
      config.ingestionConfig
    );
    this.syncEngine = new GraphSyncEngine(
      this.connectionManager,
      config.syncConfig
    );
    this.aiInterface = new AIGraphInterface(
      this.connectionManager,
      orchestrationEngine,
      config.aiConfig
    );
  }
  
  /**
   * Initialize the knowledge system for a project
   */
  async initializeProject(
    projectId: string,
    workspacePath: string,
    options?: {
      fullIndex?: boolean;
      watchFiles?: boolean;
    }
  ): Promise<ProjectContext> {
    // Create project database
    const database = await this.connectionManager.createProjectDatabase(
      projectId,
      workspacePath
    );
    
    // Initialize schema
    await this.schemaManager.initializeSchema(database.id);
    
    // Perform initial indexing if requested
    if (options?.fullIndex) {
      await this.ingestionPipeline.ingestProject(
        workspacePath,
        database.id,
        { fullReindex: true }
      );
    }
    
    // Start file watching if requested
    if (options?.watchFiles) {
      await this.syncEngine.startWatching(
        projectId,
        workspacePath,
        database.id
      );
    }
    
    // Return project context
    return this.aiInterface.getProjectContext(projectId);
  }
  
  /**
   * Query the knowledge graph using AI
   */
  async queryWithAI(query: AIGraphQuery): Promise<AIGraphResponse> {
    return this.aiInterface.query(query);
  }
  
  /**
   * Get AI context for a code location
   */
  async getAIContext(
    projectId: string,
    filePath: string,
    line: number
  ): Promise<any> {
    return this.aiInterface.getContextForLocation(projectId, filePath, line);
  }
  
  /**
   * Update graph from AI changes
   */
  async updateFromAI(
    projectId: string,
    changes: any[]
  ): Promise<{ success: boolean; updatesApplied: number }> {
    return this.aiInterface.updateFromAIChanges(projectId, changes);
  }
  
  /**
   * Analyze impact of changes
   */
  async analyzeChangeImpact(
    projectId: string,
    entityId: string,
    changeType: 'modify' | 'delete' | 'rename'
  ): Promise<any> {
    return this.aiInterface.analyzeImpact(projectId, entityId, changeType);
  }
  
  // Implement KnowledgeAPIClient interface
  
  async createEntity(entity: Omit<KnowledgeEntity, 'id'>): Promise<KnowledgeEntity> {
    const node: GraphNode = {
      id: this.generateId(),
      labels: [entity.type as unknown as NodeType],
      properties: {
        id: this.generateId(),
        name: entity.name,
        description: entity.description,
        ...entity.properties,
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    // Store in appropriate database
    const databaseId = entity.properties?.projectId 
      ? this.getDatabaseForProject(entity.properties.projectId)
      : 'global';
    
    await this.connectionManager.executeQuery(
      databaseId,
      `CREATE (n:${entity.type}) SET n = $properties RETURN n`,
      { properties: node.properties }
    );
    
    return {
      id: node.id,
      ...entity
    };
  }
  
  async getEntity(id: string): Promise<KnowledgeEntity> {
    // Search all databases for the entity
    const databases = this.connectionManager.getAllDatabases();
    
    for (const db of databases) {
      const result = await this.connectionManager.executeQuery(
        db.id,
        'MATCH (n {id: $id}) RETURN n',
        { id }
      );
      
      if (result.records.length > 0) {
        const node = result.records[0].get('n');
        return this.convertToKnowledgeEntity(node);
      }
    }
    
    throw new Error('Entity not found');
  }
  
  async updateEntity(
    id: string,
    updates: Partial<KnowledgeEntity>
  ): Promise<KnowledgeEntity> {
    // Find and update entity
    const entity = await this.getEntity(id);
    const databaseId = this.getDatabaseForEntity(entity);
    
    await this.connectionManager.executeQuery(
      databaseId,
      'MATCH (n {id: $id}) SET n += $updates SET n.updatedAt = datetime() RETURN n',
      { id, updates }
    );
    
    return this.getEntity(id);
  }
  
  async deleteEntity(id: string): Promise<void> {
    const entity = await this.getEntity(id);
    const databaseId = this.getDatabaseForEntity(entity);
    
    await this.connectionManager.executeQuery(
      databaseId,
      'MATCH (n {id: $id}) DETACH DELETE n',
      { id }
    );
  }
  
  async createRelation(
    relation: Omit<KnowledgeRelation, 'id'>
  ): Promise<KnowledgeRelation> {
    const relationId = this.generateId();
    
    // Determine database from source entity
    const sourceEntity = await this.getEntity(relation.source_id);
    const databaseId = this.getDatabaseForEntity(sourceEntity);
    
    await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (a {id: $sourceId})
      MATCH (b {id: $targetId})
      CREATE (a)-[r:${relation.type} {id: $id}]->(b)
      SET r += $properties
      SET r.createdAt = datetime()
      RETURN r
      `,
      {
        sourceId: relation.source_id,
        targetId: relation.target_id,
        id: relationId,
        properties: relation.properties || {}
      }
    );
    
    return {
      id: relationId,
      ...relation
    };
  }
  
  async getRelations(entity_id: string): Promise<KnowledgeRelation[]> {
    const entity = await this.getEntity(entity_id);
    const databaseId = this.getDatabaseForEntity(entity);
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (n {id: $id})-[r]-(m)
      RETURN r, n.id as source, m.id as target, type(r) as type
      `,
      { id: entity_id }
    );
    
    return result.records.map(record => ({
      id: record.get('r').properties.id || record.get('r').identity.toString(),
      type: record.get('type') as any,
      source_id: record.get('source'),
      target_id: record.get('target'),
      properties: record.get('r').properties,
      confidence: record.get('r').properties.confidence
    }));
  }
  
  async deleteRelation(id: string): Promise<void> {
    // Search all databases
    const databases = this.connectionManager.getAllDatabases();
    
    for (const db of databases) {
      await this.connectionManager.executeQuery(
        db.id,
        'MATCH ()-[r {id: $id}]-() DELETE r',
        { id }
      );
    }
  }
  
  async search(query: KnowledgeQuery): Promise<KnowledgeSearchResult> {
    // Convert to AI graph query
    const aiQuery: AIGraphQuery = {
      query: query.text || '',
      context: {
        currentFile: query.start_entities?.[0],
        taskType: 'analyze'
      },
      requirements: {
        includeImplementationDetails: true,
        maxDepth: query.traverse?.depth,
        limit: query.limit
      }
    };
    
    const response = await this.aiInterface.query(aiQuery);
    
    return {
      entities: response.entities.map(node => this.convertToKnowledgeEntity(node)),
      relationships: response.relationships.map(rel => ({
        id: rel.id,
        type: rel.type as any,
        source_id: rel.startNodeId,
        target_id: rel.endNodeId,
        properties: rel.properties
      })),
      total: response.entities.length,
      execution_time: response.queryExecutionTime
    };
  }
  
  async findSimilar(entity_id: string, limit?: number): Promise<KnowledgeEntity[]> {
    // This would use vector similarity search
    const entity = await this.getEntity(entity_id);
    const databaseId = this.getDatabaseForEntity(entity);
    
    // For now, find entities of the same type
    const result = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (n {id: $id})
      MATCH (similar)
      WHERE labels(n) = labels(similar) AND n <> similar
      RETURN similar
      LIMIT $limit
      `,
      { id: entity_id, limit: limit || 10 }
    );
    
    return result.records.map(r => this.convertToKnowledgeEntity(r.get('similar')));
  }
  
  async findPath(source_id: string, target_id: string): Promise<GraphPath | null> {
    // Find shortest path between two entities
    const sourceEntity = await this.getEntity(source_id);
    const databaseId = this.getDatabaseForEntity(sourceEntity);
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH path = shortestPath((a {id: $sourceId})-[*]-(b {id: $targetId}))
      RETURN path
      `,
      { sourceId: source_id, targetId: target_id }
    );
    
    if (result.records.length === 0) {
      return null;
    }
    
    // Convert path - implementation would extract nodes and relationships
    return {
      nodes: [],
      relationships: [],
      length: 0
    };
  }
  
  async executeGraphQuery(query: GraphQuery): Promise<GraphSearchResult> {
    // Execute raw Cypher query
    const databaseId = 'global'; // Or determine from query
    const result = await this.connectionManager.executeQuery(
      databaseId,
      query.cypher,
      query.parameters
    );
    
    const nodes: GraphNode[] = [];
    const relationships: GraphRelationship[] = [];
    
    result.records.forEach(record => {
      record.keys.forEach(key => {
        const value = record.get(key);
        if (this.isNode(value)) {
          nodes.push(this.convertNode(value));
        } else if (this.isRelationship(value)) {
          relationships.push(this.convertRelationship(value));
        }
      });
    });
    
    return {
      nodes,
      relationships,
      execution_time: 0
    };
  }
  
  async getSubgraph(entity_ids: string[], depth?: number): Promise<GraphVisualization> {
    // Get subgraph around entities
    const nodes: any[] = [];
    const edges: any[] = [];
    
    // Implementation would fetch subgraph
    
    return {
      nodes,
      edges,
      layout: 'force'
    };
  }
  
  async getGraphSchema(): Promise<GraphSchema> {
    const schema = this.schemaManager.getSchemaDefinition();
    
    return {
      node_labels: schema.nodeTypes.map(nt => nt.type),
      relationship_types: schema.relationshipTypes.map(rt => rt.type),
      property_keys: [], // Would extract from schema
      constraints: schema.constraints.map(c => ({
        name: c.name,
        type: c.type,
        label: c.nodeType,
        properties: c.properties
      })),
      indexes: schema.indexes.map(i => ({
        name: i.name,
        type: i.type,
        labels: i.nodeTypes,
        relationship_types: i.relationshipTypes,
        properties: i.properties
      }))
    };
  }
  
  async searchVectors(request: VectorSearchRequest): Promise<VectorSearchResult> {
    // Vector search would be implemented with Qdrant
    return {
      points: [],
      total: 0,
      execution_time: 0
    };
  }
  
  async upsertVectors(batch: VectorBatch): Promise<void> {
    // Vector operations would use Qdrant
  }
  
  async deleteVectors(ids: (string | number)[]): Promise<void> {
    // Vector operations would use Qdrant
  }
  
  async getIndexes(): Promise<KnowledgeIndex[]> {
    const databases = this.connectionManager.getAllDatabases();
    
    return databases.map(db => ({
      id: db.id,
      name: db.name,
      type: 'graph',
      status: db.status === 'connected' ? 'ready' : 'error',
      stats: {
        entities: db.statistics.nodeCount,
        relationships: db.statistics.relationshipCount,
        last_updated: db.statistics.lastUpdated.toISOString()
      }
    }));
  }
  
  async rebuildIndex(index_id: string): Promise<void> {
    const database = this.connectionManager.getDatabase(index_id);
    if (!database || !database.projectId) {
      throw new Error('Invalid index');
    }
    
    // Re-ingest the project
    const projectDb = this.connectionManager.getProjectDatabase(database.projectId);
    if (projectDb) {
      await this.ingestionPipeline.ingestProject(
        database.config.projectId!,
        database.id,
        { fullReindex: true }
      );
    }
  }
  
  async optimizeIndex(index_id: string): Promise<void> {
    // Run optimization queries
    await this.connectionManager.executeQuery(
      index_id,
      'CALL db.checkpoint()'
    );
  }
  
  async getEntityStats(): Promise<Record<string, number>> {
    const stats: Record<string, number> = {};
    const databases = this.connectionManager.getAllDatabases();
    
    for (const db of databases) {
      db.statistics.nodeTypes.forEach((count, type) => {
        stats[type] = (stats[type] || 0) + count;
      });
    }
    
    return stats;
  }
  
  async getRelationStats(): Promise<Record<string, number>> {
    const stats: Record<string, number> = {};
    const databases = this.connectionManager.getAllDatabases();
    
    for (const db of databases) {
      db.statistics.relationshipTypes.forEach((count, type) => {
        stats[type] = (stats[type] || 0) + count;
      });
    }
    
    return stats;
  }
  
  async getGraphMetrics(): Promise<GraphMetrics> {
    let totalNodes = 0;
    let totalEdges = 0;
    
    const databases = this.connectionManager.getAllDatabases();
    for (const db of databases) {
      totalNodes += db.statistics.nodeCount;
      totalEdges += db.statistics.relationshipCount;
    }
    
    return {
      nodes: totalNodes,
      edges: totalEdges,
      density: totalEdges / (totalNodes * (totalNodes - 1)),
      avg_degree: (2 * totalEdges) / totalNodes,
      clustering_coefficient: 0, // Would calculate
      connected_components: 1 // Would calculate
    };
  }
  
  // Helper methods
  
  private generateId(): string {
    return Math.random().toString(36).substring(2) + Date.now().toString(36);
  }
  
  private getDatabaseForProject(projectId: string): string {
    const db = this.connectionManager.getProjectDatabase(projectId);
    return db?.id || 'global';
  }
  
  private getDatabaseForEntity(entity: KnowledgeEntity): string {
    return entity.properties?.projectId 
      ? this.getDatabaseForProject(entity.properties.projectId)
      : 'global';
  }
  
  private convertToKnowledgeEntity(neo4jNode: any): KnowledgeEntity {
    return {
      id: neo4jNode.properties.id,
      type: neo4jNode.labels[0] as any,
      name: neo4jNode.properties.name,
      description: neo4jNode.properties.description,
      properties: neo4jNode.properties,
      embeddings: neo4jNode.properties.embeddings,
      graph_node_id: neo4jNode.identity?.toString()
    };
  }
  
  private isNode(value: any): boolean {
    return value && typeof value === 'object' && 'labels' in value;
  }
  
  private isRelationship(value: any): boolean {
    return value && typeof value === 'object' && 'type' in value && 'start' in value;
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
}

/**
 * Create and configure the knowledge graph system
 */
export function createKnowledgeGraphSystem(
  orchestrationEngine: OrchestrationEngine,
  config?: KnowledgeSystemConfig
): KnowledgeGraphSystem {
  return new KnowledgeGraphSystem(orchestrationEngine, config);
}