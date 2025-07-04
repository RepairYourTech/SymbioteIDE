/**
 * Graph Types for Neo4j Code Intelligence
 * 
 * Defines the core types for the multi-graph knowledge system
 */

import { Node, Relationship, Integer } from 'neo4j-driver';

// Graph Database Types
export type GraphContext = 'project' | 'global';

export interface GraphConfig {
  context: GraphContext;
  projectId?: string; // Required for project context
  uri: string;
  username: string;
  password: string;
  database?: string;
  options?: {
    maxConnectionPoolSize?: number;
    connectionTimeout?: number;
    encrypted?: boolean;
  };
}

export interface GraphDatabase {
  id: string;
  context: GraphContext;
  projectId?: string;
  name: string;
  status: 'connected' | 'disconnected' | 'error';
  statistics: GraphStatistics;
  config: GraphConfig;
}

export interface GraphStatistics {
  nodeCount: number;
  relationshipCount: number;
  nodeTypes: Map<string, number>;
  relationshipTypes: Map<string, number>;
  lastUpdated: Date;
  sizeInBytes?: number;
}

// Code Entity Node Types
export enum NodeType {
  // File system entities
  PROJECT = 'Project',
  FILE = 'File',
  DIRECTORY = 'Directory',
  
  // Code entities
  MODULE = 'Module',
  CLASS = 'Class',
  INTERFACE = 'Interface',
  FUNCTION = 'Function',
  METHOD = 'Method',
  VARIABLE = 'Variable',
  CONSTANT = 'Constant',
  TYPE = 'Type',
  ENUM = 'Enum',
  
  // Package entities
  PACKAGE = 'Package',
  DEPENDENCY = 'Dependency',
  
  // Documentation entities
  COMMENT = 'Comment',
  DOCSTRING = 'DocString',
  
  // Version control entities
  COMMIT = 'Commit',
  BRANCH = 'Branch',
  TAG = 'Tag',
  
  // Developer entities
  DEVELOPER = 'Developer',
  TEAM = 'Team',
  
  // AI-specific entities
  CONCEPT = 'Concept',
  PATTERN = 'Pattern',
  ISSUE = 'Issue'
}

// Relationship Types
export enum RelationType {
  // Structural relationships
  CONTAINS = 'CONTAINS',
  BELONGS_TO = 'BELONGS_TO',
  
  // Code relationships
  IMPORTS = 'IMPORTS',
  EXPORTS = 'EXPORTS',
  CALLS = 'CALLS',
  IMPLEMENTS = 'IMPLEMENTS',
  EXTENDS = 'EXTENDS',
  INSTANTIATES = 'INSTANTIATES',
  REFERENCES = 'REFERENCES',
  USES = 'USES',
  DEFINES = 'DEFINES',
  DECLARES = 'DECLARES',
  
  // Type relationships
  HAS_TYPE = 'HAS_TYPE',
  RETURNS = 'RETURNS',
  ACCEPTS = 'ACCEPTS',
  
  // Dependency relationships
  DEPENDS_ON = 'DEPENDS_ON',
  REQUIRED_BY = 'REQUIRED_BY',
  
  // Version control relationships
  AUTHORED_BY = 'AUTHORED_BY',
  MODIFIED_BY = 'MODIFIED_BY',
  CREATED_IN = 'CREATED_IN',
  MODIFIED_IN = 'MODIFIED_IN',
  
  // Documentation relationships
  DOCUMENTS = 'DOCUMENTS',
  DESCRIBED_BY = 'DESCRIBED_BY',
  
  // AI relationships
  SIMILAR_TO = 'SIMILAR_TO',
  RELATED_TO = 'RELATED_TO',
  PATTERN_OF = 'PATTERN_OF'
}

// Base node properties
export interface BaseNodeProperties {
  id: string;
  name: string;
  createdAt: Date;
  updatedAt: Date;
  version: number;
  metadata?: Record<string, any>;
}

// Specific node properties
export interface FileNodeProperties extends BaseNodeProperties {
  path: string;
  extension: string;
  language: string;
  size: number;
  hash: string;
  encoding: string;
}

export interface ClassNodeProperties extends BaseNodeProperties {
  visibility: 'public' | 'private' | 'protected';
  isAbstract: boolean;
  isInterface: boolean;
  superClass?: string;
  interfaces: string[];
  filePath: string;
  startLine: number;
  endLine: number;
}

export interface FunctionNodeProperties extends BaseNodeProperties {
  signature: string;
  parameters: ParameterInfo[];
  returnType?: string;
  visibility: 'public' | 'private' | 'protected';
  isAsync: boolean;
  isGenerator: boolean;
  filePath: string;
  startLine: number;
  endLine: number;
  complexity?: number;
}

export interface VariableNodeProperties extends BaseNodeProperties {
  type?: string;
  value?: any;
  isConstant: boolean;
  scope: 'global' | 'module' | 'class' | 'function' | 'block';
  filePath: string;
  line: number;
}

export interface PackageNodeProperties extends BaseNodeProperties {
  version: string;
  registry?: string;
  description?: string;
  license?: string;
  author?: string;
  homepage?: string;
  dependencies?: Record<string, string>;
}

// Helper types
export interface ParameterInfo {
  name: string;
  type?: string;
  defaultValue?: any;
  isOptional: boolean;
  isRest: boolean;
}

// Relationship properties
export interface BaseRelationshipProperties {
  createdAt: Date;
  updatedAt: Date;
  confidence?: number; // For AI-inferred relationships
  metadata?: Record<string, any>;
}

export interface CallRelationshipProperties extends BaseRelationshipProperties {
  callCount?: number;
  isAsync?: boolean;
  parameters?: any[];
  filePath: string;
  line: number;
}

export interface ImportRelationshipProperties extends BaseRelationshipProperties {
  importType: 'default' | 'named' | 'namespace' | 'side-effect';
  importedNames?: string[];
  alias?: string;
  filePath: string;
  line: number;
}

// Graph operations
export interface GraphNode<T extends BaseNodeProperties = BaseNodeProperties> {
  id: string;
  labels: string[];
  properties: T;
}

export interface GraphRelationship<T extends BaseRelationshipProperties = BaseRelationshipProperties> {
  id: string;
  type: string;
  startNodeId: string;
  endNodeId: string;
  properties: T;
}

// Query types
export interface GraphQuery {
  cypher: string;
  parameters?: Record<string, any>;
  timeout?: number;
}

export interface GraphQueryResult<N = any, R = any> {
  nodes: GraphNode<N>[];
  relationships: GraphRelationship<R>[];
  executionTime: number;
  recordsAffected?: number;
}

// AI-specific query types
export interface AIGraphQuery {
  // Natural language query
  query: string;
  
  // Context about current task
  context: {
    currentFile?: string;
    selectedCode?: string;
    taskType?: 'refactor' | 'implement' | 'debug' | 'analyze' | 'explain';
    projectId?: string;
  };
  
  // What the AI needs
  requirements: {
    includeImplementationDetails?: boolean;
    includeUsageExamples?: boolean;
    includeRelatedConcepts?: boolean;
    includeDependencies?: boolean;
    includeTests?: boolean;
    maxDepth?: number;
    limit?: number;
  };
}

export interface AIGraphResponse {
  // Direct answer to the query
  answer: string;
  
  // Relevant nodes and relationships
  entities: GraphNode[];
  relationships: GraphRelationship[];
  
  // Structured data for AI consumption
  context: {
    mainEntity?: GraphNode;
    relatedEntities: GraphNode[];
    codeExamples: CodeExample[];
    dependencies: DependencyInfo[];
    impacts: ImpactInfo[];
  };
  
  // Confidence and metadata
  confidence: number;
  queryExecutionTime: number;
  suggestions?: string[];
}

export interface CodeExample {
  file: string;
  startLine: number;
  endLine: number;
  code: string;
  description: string;
}

export interface DependencyInfo {
  name: string;
  type: 'direct' | 'transitive';
  version?: string;
  usage: string[];
}

export interface ImpactInfo {
  entity: GraphNode;
  impactType: 'direct' | 'indirect';
  description: string;
  severity: 'low' | 'medium' | 'high';
}

// Project context types
export interface ProjectContext {
  projectId: string;
  workspacePath: string;
  graphDatabase: string;
  languages: string[];
  statistics: GraphStatistics;
  indexingStatus: IndexingStatus;
  aiAccessPatterns: AIAccessPatterns;
}

export interface IndexingStatus {
  status: 'idle' | 'indexing' | 'updating' | 'error';
  progress?: {
    total: number;
    processed: number;
    failed: number;
  };
  lastIndexed?: Date;
  nextScheduledIndex?: Date;
}

export interface AIAccessPatterns {
  frequentQueries: QueryPattern[];
  hotspots: HotspotInfo[];
  recentChanges: ChangeInfo[];
}

export interface QueryPattern {
  pattern: string;
  frequency: number;
  averageExecutionTime: number;
  lastUsed: Date;
}

export interface HotspotInfo {
  entityId: string;
  entityType: NodeType;
  accessCount: number;
  lastAccessed: Date;
  reason: string;
}

export interface ChangeInfo {
  entityId: string;
  changeType: 'created' | 'modified' | 'deleted';
  timestamp: Date;
  author?: string;
  description: string;
}

// Graph update types
export interface GraphUpdate {
  type: 'create' | 'update' | 'delete';
  entityType: 'node' | 'relationship';
  data: GraphNode | GraphRelationship;
  metadata?: {
    source: 'file-watcher' | 'ai-update' | 'manual' | 'git-sync';
    timestamp: Date;
    author?: string;
  };
}

export interface BatchGraphUpdate {
  updates: GraphUpdate[];
  transactional: boolean;
  validateBeforeApply: boolean;
}

// Error types
export class GraphError extends Error {
  constructor(
    message: string,
    public code: GraphErrorCode,
    public details?: any
  ) {
    super(message);
    this.name = 'GraphError';
  }
}

export enum GraphErrorCode {
  CONNECTION_FAILED = 'CONNECTION_FAILED',
  QUERY_FAILED = 'QUERY_FAILED',
  SCHEMA_VIOLATION = 'SCHEMA_VIOLATION',
  TRANSACTION_FAILED = 'TRANSACTION_FAILED',
  INDEX_FAILED = 'INDEX_FAILED',
  SYNC_FAILED = 'SYNC_FAILED',
  INVALID_OPERATION = 'INVALID_OPERATION'
}