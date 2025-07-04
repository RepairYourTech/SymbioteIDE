/**
 * Knowledge API Types for Neo4j and Qdrant Integration
 * Provides knowledge graph and vector search capabilities
 */

// Neo4j Graph Database Types
export interface Neo4jConfig {
  uri: string;
  username: string;
  password: string;
  database?: string;
  options?: {
    maxConnectionPoolSize?: number;
    connectionTimeout?: number;
    maxTransactionRetryTime?: number;
    encrypted?: boolean;
  };
}

export interface GraphNode {
  id: string;
  labels: string[];
  properties: Record<string, any>;
  created_at: string;
  updated_at: string;
  version: number;
}

export interface GraphRelationship {
  id: string;
  type: string;
  start_node_id: string;
  end_node_id: string;
  properties: Record<string, any>;
  created_at: string;
}

export interface GraphQuery {
  cypher: string;
  parameters?: Record<string, any>;
  timeout?: number;
  explain?: boolean;
}

export interface GraphTraversal {
  start_node_id: string;
  direction: 'incoming' | 'outgoing' | 'both';
  relationship_types?: string[];
  max_depth?: number;
  filters?: GraphFilters;
}

export interface GraphFilters {
  node_labels?: string[];
  node_properties?: Record<string, any>;
  relationship_types?: string[];
  relationship_properties?: Record<string, any>;
}

export interface GraphSearchResult {
  nodes: GraphNode[];
  relationships: GraphRelationship[];
  paths?: GraphPath[];
  execution_time: number;
  query_plan?: string;
}

export interface GraphPath {
  nodes: GraphNode[];
  relationships: GraphRelationship[];
  length: number;
  cost?: number;
}

export interface GraphSchema {
  node_labels: string[];
  relationship_types: string[];
  property_keys: string[];
  constraints: GraphConstraint[];
  indexes: GraphIndex[];
}

export interface GraphConstraint {
  name: string;
  type: 'unique' | 'exists' | 'node_key';
  label: string;
  properties: string[];
}

export interface GraphIndex {
  name: string;
  type: 'btree' | 'fulltext' | 'vector';
  labels?: string[];
  relationship_types?: string[];
  properties: string[];
  config?: Record<string, any>;
}

// Qdrant Vector Database Types
export interface QdrantConfig {
  url: string;
  apiKey?: string;
  collection: string;
  options?: {
    timeout?: number;
    retries?: number;
    vector_size?: number;
    distance?: 'cosine' | 'euclidean' | 'dot_product';
  };
}

export interface VectorPoint {
  id: string | number;
  vector: number[];
  payload?: Record<string, any>;
  score?: number;
}

export interface VectorCollection {
  name: string;
  vector_size: number;
  distance: 'cosine' | 'euclidean' | 'dot_product';
  points_count: number;
  status: 'ready' | 'optimizing' | 'error';
  config: VectorCollectionConfig;
}

export interface VectorCollectionConfig {
  hnsw_config?: {
    m?: number;
    ef_construct?: number;
    max_elements?: number;
  };
  wal_config?: {
    wal_capacity_mb?: number;
    wal_segments_ahead?: number;
  };
  optimizers_config?: {
    deleted_threshold?: number;
    vacuum_min_vector_number?: number;
  };
}

export interface VectorSearchRequest {
  vector?: number[];
  text?: string;
  limit?: number;
  offset?: number;
  filter?: VectorFilter;
  with_payload?: boolean;
  with_vector?: boolean;
  score_threshold?: number;
}

export interface VectorFilter {
  must?: VectorCondition[];
  should?: VectorCondition[];
  must_not?: VectorCondition[];
}

export interface VectorCondition {
  field: string;
  match?: {
    value: any;
    text?: string;
  };
  range?: {
    gt?: number;
    gte?: number;
    lt?: number;
    lte?: number;
  };
  geo?: {
    radius: number;
    center: { lat: number; lon: number };
  };
}

export interface VectorSearchResult {
  points: VectorPoint[];
  total: number;
  execution_time: number;
  next_offset?: number;
}

export interface VectorBatch {
  points: VectorPoint[];
  wait?: boolean;
  ordering?: 'weak' | 'medium' | 'strong';
}

// Knowledge Graph Integration Types
export interface KnowledgeEntity {
  id: string;
  type: EntityType;
  name: string;
  description?: string;
  properties: Record<string, any>;
  embeddings?: Record<string, number[]>;
  graph_node_id?: string;
  vector_point_id?: string | number;
}

export type EntityType =
  | 'code_file'
  | 'function'
  | 'class'
  | 'module'
  | 'documentation'
  | 'issue'
  | 'commit'
  | 'developer'
  | 'concept'
  | 'pattern'
  | 'dependency';

export interface KnowledgeRelation {
  id: string;
  type: RelationType;
  source_id: string;
  target_id: string;
  properties?: Record<string, any>;
  confidence?: number;
}

export type RelationType =
  | 'imports'
  | 'exports'
  | 'calls'
  | 'implements'
  | 'extends'
  | 'references'
  | 'authored_by'
  | 'related_to'
  | 'depends_on'
  | 'similar_to';

export interface KnowledgeQuery {
  // Text search
  text?: string;
  
  // Graph traversal
  start_entities?: string[];
  traverse?: {
    depth?: number;
    relationship_types?: RelationType[];
    direction?: 'incoming' | 'outgoing' | 'both';
  };
  
  // Vector similarity
  similar_to?: string[];
  similarity_threshold?: number;
  
  // Filters
  entity_types?: EntityType[];
  properties?: Record<string, any>;
  
  // Results
  limit?: number;
  include_embeddings?: boolean;
  include_relationships?: boolean;
}

export interface KnowledgeSearchResult {
  entities: KnowledgeEntity[];
  relationships?: KnowledgeRelation[];
  graph_visualization?: GraphVisualization;
  facets?: Record<string, FacetResult>;
  total: number;
  execution_time: number;
}

export interface GraphVisualization {
  nodes: Array<{
    id: string;
    label: string;
    type: string;
    x?: number;
    y?: number;
    size?: number;
    color?: string;
  }>;
  edges: Array<{
    id: string;
    source: string;
    target: string;
    label: string;
    weight?: number;
    color?: string;
  }>;
  layout?: 'force' | 'hierarchical' | 'circular' | 'grid';
}

export interface FacetResult {
  values: Array<{
    value: any;
    count: number;
  }>;
  total: number;
}

export interface KnowledgeIndex {
  id: string;
  name: string;
  type: 'graph' | 'vector' | 'hybrid';
  status: 'building' | 'ready' | 'error';
  stats: {
    entities: number;
    relationships: number;
    vectors?: number;
    last_updated: string;
  };
}

export interface KnowledgeAPIClient {
  // Entity management
  createEntity(entity: Omit<KnowledgeEntity, 'id'>): Promise<KnowledgeEntity>;
  getEntity(id: string): Promise<KnowledgeEntity>;
  updateEntity(id: string, updates: Partial<KnowledgeEntity>): Promise<KnowledgeEntity>;
  deleteEntity(id: string): Promise<void>;
  
  // Relationship management
  createRelation(relation: Omit<KnowledgeRelation, 'id'>): Promise<KnowledgeRelation>;
  getRelations(entity_id: string): Promise<KnowledgeRelation[]>;
  deleteRelation(id: string): Promise<void>;
  
  // Search and query
  search(query: KnowledgeQuery): Promise<KnowledgeSearchResult>;
  findSimilar(entity_id: string, limit?: number): Promise<KnowledgeEntity[]>;
  findPath(source_id: string, target_id: string): Promise<GraphPath | null>;
  
  // Graph operations
  executeGraphQuery(query: GraphQuery): Promise<GraphSearchResult>;
  getSubgraph(entity_ids: string[], depth?: number): Promise<GraphVisualization>;
  getGraphSchema(): Promise<GraphSchema>;
  
  // Vector operations
  searchVectors(request: VectorSearchRequest): Promise<VectorSearchResult>;
  upsertVectors(batch: VectorBatch): Promise<void>;
  deleteVectors(ids: (string | number)[]): Promise<void>;
  
  // Index management
  getIndexes(): Promise<KnowledgeIndex[]>;
  rebuildIndex(index_id: string): Promise<void>;
  optimizeIndex(index_id: string): Promise<void>;
  
  // Analytics
  getEntityStats(): Promise<Record<EntityType, number>>;
  getRelationStats(): Promise<Record<RelationType, number>>;
  getGraphMetrics(): Promise<GraphMetrics>;
}

export interface GraphMetrics {
  nodes: number;
  edges: number;
  density: number;
  avg_degree: number;
  clustering_coefficient: number;
  connected_components: number;
  diameter?: number;
}

export interface KnowledgeExtractionConfig {
  source_type: 'code' | 'documentation' | 'comments' | 'commits';
  extractors: ExtractorConfig[];
  embedding_model?: string;
  batch_size?: number;
  parallel_jobs?: number;
}

export interface ExtractorConfig {
  type: 'ast' | 'nlp' | 'regex' | 'llm';
  config: Record<string, any>;
  entity_types: EntityType[];
  relation_types: RelationType[];
}

export interface KnowledgeIngestionJob {
  id: string;
  status: 'queued' | 'running' | 'completed' | 'failed';
  progress: {
    total: number;
    processed: number;
    failed: number;
  };
  started_at?: string;
  completed_at?: string;
  error?: string;
  results?: {
    entities_created: number;
    relations_created: number;
    vectors_indexed: number;
  };
}