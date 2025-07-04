/**
 * Hivemind Parallel Agent System
 * 
 * Main entry point for the Hivemind system
 */

export * from './types';
export * from './hivemind-controller';
export * from './task-decomposer';
export * from './conflict-resolver';
export * from './result-aggregator';
export * from './coordination-layer';

// Export agents
export * from './agents/base-specialist';
export * from './agents/frontend-specialist';
export * from './agents/backend-specialist';
export * from './agents/testing-specialist';
export * from './agents/security-specialist';
export * from './agents/devops-specialist';
export * from './agents/performance-specialist';

// Re-export main controller for convenience
export { HivemindController as Hivemind } from './hivemind-controller';