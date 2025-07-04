/**
 * Google AI ADK Integration
 * 
 * Export all ADK components and utilities
 */

// Core types
export * from './types';

// Main components
export { AgentFactory } from './agent-factory';
export { ADKOrchestrator, getADKOrchestrator } from './orchestrator';
export { OrchestrationIntegration, getOrchestrationIntegration } from './orchestration-integration';

// Adapters
export { MCPToolAdapter } from './mcp-tool-adapter';
export { A2AAgentAdapter } from './a2a-agent-adapter';
export { MemoryAdapter } from './memory-adapter';

// A2A Protocol
export { 
  // Only export types that don't conflict
  AgentRequest,
  AgentResponse,
  MessageType,
  TaskDefinition,
  TaskParameters,
  TaskResult as A2ATaskResult,
  TaskStatus,
  ArtifactType,
  ChannelType,
  A2AProtocolCapability,
  A2AProtocolVersion,
  AgentCard as A2AAgentCard,
  Artifact as A2AArtifact,
  EndpointDefinition as A2AEndpointDefinition,
  AuthenticationSchema as A2AAuthenticationSchema,
  Message as A2AMessage,
  TaskContext as A2ATaskContext
} from '../a2a/types';
export { A2AClient } from '../a2a/client';
export { A2AServer } from '../a2a/server';

// AgentBuilder interface is exported from types

// Re-export commonly used types with ADK prefix
export type {
  ADKAgentConfig as AgentConfig,
  ADKAgentType as AgentType,
  ADKTool as Tool,
  ADKTask as Task,
  ADKExecutionResult as ExecutionResult,
  AgentCapabilities,
  MemoryConfig,
  A2AConfig,
  WorkflowDefinition,
  AgentExecutionRequest,
  AgentExecutionResult
} from './types';