/**
 * Hivemind Types
 * 
 * Type definitions for the Hivemind Parallel Agent System
 */

import { Agent } from '@google/adk';
import { AgentExecutionResult } from '../adk/types';

export enum AgentSpecialty {
  Frontend = 'frontend',
  Backend = 'backend',
  Testing = 'testing',
  Security = 'security',
  DevOps = 'devops',
  Performance = 'performance',
  Database = 'database',
  Documentation = 'documentation',
  Architecture = 'architecture'
}

export enum TaskPriority {
  Critical = 4,
  High = 3,
  Medium = 2,
  Low = 1
}

export enum TaskStatus {
  Pending = 'pending',
  Assigned = 'assigned',
  InProgress = 'in_progress',
  Review = 'review',
  Completed = 'completed',
  Failed = 'failed',
  Blocked = 'blocked'
}

export interface HivemindTask {
  id: string;
  type: string;
  title: string;
  description: string;
  requiredSpecialties: AgentSpecialty[];
  priority: TaskPriority;
  status: TaskStatus;
  dependencies: string[]; // Task IDs
  assignedAgents: string[]; // Agent IDs
  estimatedComplexity: number; // 1-10 scale
  context: {
    files?: string[];
    codeContext?: string;
    projectPath?: string;
    additionalInfo?: any;
  };
  result?: any;
  error?: string;
  createdAt: Date;
  startedAt?: Date;
  completedAt?: Date;
}

export interface SpecialistAgent {
  id: string;
  name: string;
  specialty: AgentSpecialty;
  capabilities: string[];
  status: AgentStatus;
  currentTask?: string;
  performance: AgentPerformance;
  instance: Agent;
}

export enum AgentStatus {
  Available = 'available',
  Busy = 'busy',
  Offline = 'offline',
  Error = 'error'
}

export interface AgentPerformance {
  tasksCompleted: number;
  successRate: number;
  averageExecutionTime: number;
  specialtyScore: number; // 0-100
  lastActive: Date;
}

export interface TaskDecomposition {
  originalTask: HivemindTask;
  subtasks: HivemindTask[];
  dependencies: TaskDependencyGraph;
  estimatedTotalTime: number;
  parallelizationFactor: number; // 0-1, how much can be done in parallel
}

export interface TaskDependencyGraph {
  nodes: Map<string, HivemindTask>;
  edges: Map<string, string[]>; // task ID -> dependent task IDs
}

export interface ConflictResolution {
  conflictId: string;
  type: ConflictType;
  affectedFiles: string[];
  affectedTasks: string[];
  resolution: ResolutionStrategy;
  resolvedBy?: string; // Agent ID
  resolvedAt?: Date;
}

export enum ConflictType {
  FileEdit = 'file_edit',
  ResourceLock = 'resource_lock',
  DependencyConflict = 'dependency_conflict',
  LogicalConflict = 'logical_conflict'
}

export enum ResolutionStrategy {
  Merge = 'merge',
  Sequential = 'sequential',
  Retry = 'retry',
  Manual = 'manual',
  Abort = 'abort'
}

export interface HivemindMetrics {
  activeAgents: number;
  totalAgents: number;
  tasksInProgress: number;
  tasksCompleted: number;
  tasksFailed: number;
  averageTaskTime: number;
  systemLoad: number; // 0-100
  conflictsResolved: number;
  parallelizationEfficiency: number; // 0-1
}

export interface AgentMessage {
  id: string;
  from: string; // Agent ID
  to: string | 'broadcast'; // Agent ID or broadcast
  type: MessageType;
  payload: any;
  timestamp: Date;
  requiresResponse?: boolean;
}

export enum MessageType {
  TaskAssignment = 'task_assignment',
  StatusUpdate = 'status_update',
  ResultShare = 'result_share',
  HelpRequest = 'help_request',
  ConflictNotification = 'conflict_notification',
  Heartbeat = 'heartbeat'
}

export interface HivemindConfig {
  maxConcurrentAgents: number;
  taskTimeoutMs: number;
  conflictResolutionStrategy: ResolutionStrategy;
  loadBalancingEnabled: boolean;
  autoScalingEnabled: boolean;
  minAgentsPerSpecialty: number;
  maxAgentsPerSpecialty: number;
  heartbeatIntervalMs: number;
  metricsCollectionIntervalMs: number;
}

export interface TaskExecutionPlan {
  taskId: string;
  phases: ExecutionPhase[];
  estimatedDuration: number;
  requiredResources: string[];
  parallelSteps: string[][];
}

export interface ExecutionPhase {
  id: string;
  name: string;
  tasks: string[];
  canRunInParallel: boolean;
  dependencies: string[];
  estimatedDuration: number;
}

export interface HivemindEvent {
  type: HivemindEventType;
  timestamp: Date;
  source: string;
  data: any;
}

export enum HivemindEventType {
  AgentJoined = 'agent_joined',
  AgentLeft = 'agent_left',
  TaskCreated = 'task_created',
  TaskAssigned = 'task_assigned',
  TaskCompleted = 'task_completed',
  TaskFailed = 'task_failed',
  ConflictDetected = 'conflict_detected',
  ConflictResolved = 'conflict_resolved',
  SystemOverload = 'system_overload',
  MetricsUpdate = 'metrics_update'
}

export interface SharedContext {
  projectInfo: {
    name: string;
    path: string;
    language: string;
    framework?: string;
    dependencies: string[];
  };
  codebaseKnowledge: Map<string, any>;
  completedTasks: Map<string, any>;
  learnedPatterns: Map<string, any>;
  globalConstraints: string[];
}

export interface TaskResult {
  taskId: string;
  success: boolean;
  output: any;
  filesModified: string[];
  filesCreated: string[];
  testsAdded: number;
  issuesFound: string[];
  performanceMetrics: {
    duration: number;
    tokensUsed: number;
    cost: number;
  };
}
