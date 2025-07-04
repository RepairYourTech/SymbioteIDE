/**
 * Orchestration API Module Exports
 */

// Main API server
export { OrchestrationAPIServer, createOrchestrationAPI } from './api-server';

// REST API components
export { ExpressAPIServer } from './rest/express-server';

// gRPC API components  
export { GRPCAPIServer } from './grpc/grpc-server';

// Middleware exports
export { authMiddleware } from './middleware/auth';
export { rateLimitMiddleware } from './middleware/rate-limit';
export { validationMiddleware } from './middleware/validation';
export { errorHandler } from './middleware/error-handler';

// Types
export * from './types/api-interfaces';

// Re-export types from parent module for convenience
export type {
  AITask,
  TaskResult,
  ModelProfile,
  OrchestrationMetrics,
  CostEstimate
} from '../types';