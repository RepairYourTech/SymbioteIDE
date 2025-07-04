/**
 * AI-Powered Code Review System
 * 
 * Provides intelligent code review capabilities using multi-model AI,
 * Neo4j knowledge graph, and Qdrant semantic search.
 */

export * from './types';
export * from './review-manager';
export * from './review-engine';

// Analyzers
export * from './analyzers/security-analyzer';
export * from './analyzers/performance-analyzer';
export * from './analyzers/best-practices-analyzer';
export * from './analyzers/dependency-analyzer';

// Semantic components
export * from './semantic/review-embedder';
export * from './semantic/similarity-finder';
export * from './semantic/pattern-matcher';

// Context system
export * from './context/review-context-builder';
export * from './context/impact-analyzer';
export * from './context/history-tracker';

// AI integration
export * from './ai/review-ai-interface';
export * from './ai/suggestion-generator';
export * from './ai/explanation-generator';

// UI components
export * from './ui/review-panel-provider';
export * from './ui/inline-decorations';
export * from './ui/review-commands';

// Workflow integration
export * from './workflow/pr-review-integration';
export * from './workflow/ci-integration';
export * from './workflow/team-collaboration';

// Configuration
export * from './config/review-config';
export * from './config/model-config';
export * from './config/team-rules';

// Main API
import { ReviewManager } from './review-manager';
import { ReviewEngine } from './review-engine';
import { ReviewConfig } from './types';

/**
 * Initialize the code review system
 */
export async function initializeCodeReview(config?: Partial<ReviewConfig>): Promise<ReviewManager> {
  const manager = new ReviewManager(config);
  await manager.initialize();
  return manager;
}

/**
 * Quick review function for immediate use
 */
export async function quickReview(
  files: string[],
  options?: {
    categories?: string[];
    depth?: 'quick' | 'standard' | 'deep';
  }
): Promise<any> {
  const manager = await initializeCodeReview();
  return manager.reviewFiles(files, options);
}