/**
 * Memory Module Exports
 * 
 * Unified context system combining Neo4j, Qdrant, and Mem0
 */

export * from './mem0/types';
export { MemoryManager } from './mem0/memory-manager';
export { Mem0Client } from './mem0/client';
export { UnifiedContextAPI } from './unified-context-api';

// Re-export commonly used types
export type {
  Memory,
  MemoryMetadata,
  MemorySearchRequest,
  MemorySearchResult,
  TaskContext,
  Learning,
  TeamKnowledge,
  UnifiedContext,
  ContextQuery,
  GraphInsights,
  LearningContext
} from './mem0/types';

export {
  MemoryType,
  MemoryScope
} from './mem0/types';