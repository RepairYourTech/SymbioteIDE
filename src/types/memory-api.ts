/**
 * Memory API Types for Mem0 Integration
 * Provides persistent memory and context management for AI interactions
 */

export interface MemoryConfig {
  apiKey: string;
  baseUrl?: string;
  organizationId?: string;
  projectId?: string;
  options?: {
    maxMemories?: number;
    enableAutoSync?: boolean;
    syncInterval?: number;
    compressionEnabled?: boolean;
    encryptionEnabled?: boolean;
  };
}

export interface Memory {
  id: string;
  content: string;
  metadata: MemoryMetadata;
  embedding?: number[];
  created_at: string;
  updated_at: string;
  relevance_score?: number;
  usage_count: number;
  last_accessed?: string;
}

export interface MemoryMetadata {
  type: MemoryType;
  source: MemorySource;
  tags: string[];
  context?: {
    project?: string;
    file?: string;
    function?: string;
    conversation_id?: string;
    session_id?: string;
  };
  importance: 'low' | 'medium' | 'high' | 'critical';
  expiry?: string;
  permissions?: MemoryPermissions;
  relations?: MemoryRelation[];
}

export type MemoryType = 
  | 'code_snippet'
  | 'conversation'
  | 'decision'
  | 'learning'
  | 'preference'
  | 'pattern'
  | 'error_resolution'
  | 'optimization'
  | 'documentation'
  | 'workflow';

export interface MemorySource {
  type: 'user' | 'system' | 'ai' | 'external';
  identifier: string;
  timestamp: string;
}

export interface MemoryPermissions {
  read: string[];
  write: string[];
  delete: string[];
  share: string[];
}

export interface MemoryRelation {
  target_id: string;
  relation_type: 'references' | 'derives_from' | 'contradicts' | 'extends' | 'replaces';
  strength: number;
}

export interface MemoryQuery {
  query?: string;
  filters?: MemoryFilters;
  limit?: number;
  offset?: number;
  sort?: MemorySortOptions;
  include_embeddings?: boolean;
  similarity_threshold?: number;
}

export interface MemoryFilters {
  types?: MemoryType[];
  tags?: string[];
  date_range?: {
    start: string;
    end: string;
  };
  metadata?: Record<string, any>;
  importance?: Array<'low' | 'medium' | 'high' | 'critical'>;
  sources?: string[];
}

export interface MemorySortOptions {
  field: 'created_at' | 'updated_at' | 'relevance' | 'usage_count' | 'importance';
  order: 'asc' | 'desc';
}

export interface MemorySearchResult {
  memories: Memory[];
  total: number;
  has_more: boolean;
  next_offset?: number;
  execution_time: number;
}

export interface MemoryCreateRequest {
  content: string;
  metadata: Partial<MemoryMetadata>;
  embedding?: number[];
  auto_relate?: boolean;
}

export interface MemoryUpdateRequest {
  content?: string;
  metadata?: Partial<MemoryMetadata>;
  increment_usage?: boolean;
}

export interface MemoryBulkOperation {
  operation: 'create' | 'update' | 'delete';
  memories: Array<{
    id?: string;
    data?: MemoryCreateRequest | MemoryUpdateRequest;
  }>;
}

export interface MemoryAnalytics {
  total_memories: number;
  memory_by_type: Record<MemoryType, number>;
  memory_by_importance: Record<string, number>;
  average_usage_count: number;
  most_accessed: Memory[];
  least_accessed: Memory[];
  storage_used: number;
  last_sync: string;
}

export interface MemorySyncStatus {
  status: 'idle' | 'syncing' | 'error';
  last_sync: string;
  next_sync?: string;
  items_synced: number;
  items_pending: number;
  errors?: string[];
}

export interface MemoryExportOptions {
  format: 'json' | 'csv' | 'markdown';
  include_embeddings: boolean;
  filters?: MemoryFilters;
  compress?: boolean;
}

export interface MemoryImportOptions {
  format: 'json' | 'csv';
  merge_strategy: 'replace' | 'merge' | 'skip';
  validate?: boolean;
  batch_size?: number;
}

export interface MemoryContext {
  active_memories: Memory[];
  session_id: string;
  context_window: number;
  auto_prune: boolean;
  retention_policy: MemoryRetentionPolicy;
}

export interface MemoryRetentionPolicy {
  max_age_days?: number;
  max_count?: number;
  importance_threshold?: 'low' | 'medium' | 'high' | 'critical';
  usage_threshold?: number;
  auto_archive?: boolean;
}

export interface MemoryAPIClient {
  // Memory CRUD operations
  createMemory(request: MemoryCreateRequest): Promise<Memory>;
  getMemory(id: string): Promise<Memory>;
  updateMemory(id: string, request: MemoryUpdateRequest): Promise<Memory>;
  deleteMemory(id: string): Promise<void>;
  
  // Search and retrieval
  searchMemories(query: MemoryQuery): Promise<MemorySearchResult>;
  getSimilarMemories(id: string, limit?: number): Promise<Memory[]>;
  getRelatedMemories(id: string): Promise<Memory[]>;
  
  // Bulk operations
  bulkOperation(operation: MemoryBulkOperation): Promise<void>;
  
  // Context management
  getContext(session_id: string): Promise<MemoryContext>;
  updateContext(session_id: string, context: Partial<MemoryContext>): Promise<void>;
  clearContext(session_id: string): Promise<void>;
  
  // Analytics and monitoring
  getAnalytics(): Promise<MemoryAnalytics>;
  getSyncStatus(): Promise<MemorySyncStatus>;
  
  // Import/Export
  exportMemories(options: MemoryExportOptions): Promise<Blob>;
  importMemories(data: File | Blob, options: MemoryImportOptions): Promise<void>;
  
  // Maintenance
  pruneMemories(policy: MemoryRetentionPolicy): Promise<number>;
  optimizeStorage(): Promise<void>;
  rebuildIndex(): Promise<void>;
}

export interface MemoryEvent {
  type: 'created' | 'updated' | 'deleted' | 'accessed';
  memory_id: string;
  timestamp: string;
  metadata?: Record<string, any>;
}

export interface MemoryWebhook {
  id: string;
  url: string;
  events: MemoryEvent['type'][];
  active: boolean;
  secret?: string;
  created_at: string;
}

export interface MemoryIntegration {
  type: 'github' | 'gitlab' | 'jira' | 'slack' | 'discord';
  config: Record<string, any>;
  sync_enabled: boolean;
  last_sync?: string;
  mappings: MemoryMapping[];
}

export interface MemoryMapping {
  source_field: string;
  target_field: string;
  transform?: 'none' | 'lowercase' | 'uppercase' | 'hash' | 'custom';
  custom_transform?: string;
}

export interface MemoryTemplate {
  id: string;
  name: string;
  description: string;
  metadata_template: Partial<MemoryMetadata>;
  content_template?: string;
  validation_schema?: Record<string, any>;
}