/**
 * Task Manager Types
 * 
 * Comprehensive task management system with unlimited nesting and intelligent context handling
 */

import { ModelProfile } from '../orchestration/interfaces';

// Task Status
export enum TaskStatus {
  Pending = 'pending',
  InProgress = 'in_progress',
  Blocked = 'blocked',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled',
  Deferred = 'deferred'
}

// Task Priority
export enum TaskPriority {
  Critical = 'critical',
  High = 'high',
  Medium = 'medium',
  Low = 'low',
  None = 'none'
}

// Task Type
export enum TaskType {
  Epic = 'epic',
  Story = 'story',
  Task = 'task',
  SubTask = 'subtask',
  Bug = 'bug',
  Feature = 'feature',
  Research = 'research',
  Documentation = 'documentation'
}

// Context Types
export interface TaskContext {
  id: string;
  taskId: string;
  parentContextId?: string;
  scope: ContextScope;
  data: Record<string, any>;
  variables: Record<string, any>;
  artifacts: ContextArtifact[];
  handoffs: ContextHandoff[];
  bubbles: ContextBubble[];
  inheritance: ContextInheritance;
  createdAt: Date;
  updatedAt: Date;
}

export enum ContextScope {
  Local = 'local',
  Inherited = 'inherited',
  Shared = 'shared',
  Global = 'global'
}

export interface ContextArtifact {
  id: string;
  type: string;
  name: string;
  content: any;
  metadata: Record<string, any>;
  createdAt: Date;
}

export interface ContextHandoff {
  id: string;
  type: HandoffType;
  fromTaskId: string;
  toTaskId: string;
  fromAgentId?: string;
  toAgentId?: string;
  fromModelId?: string;
  toModelId?: string;
  data: any;
  transforms: ContextTransform[];
  status: HandoffStatus;
  createdAt: Date;
  completedAt?: Date;
  metadata?: {
    sourceModelId?: string;
    targetModelId?: string;
    originalTokenCount?: number;
    optimizedTokenCount?: number;
    compressionRatio?: number;
    strategies?: string[];
  };
}

export enum HandoffType {
  AgentToAgent = 'agent_to_agent',
  ToolToAgent = 'tool_to_agent',
  AgentToTool = 'agent_to_tool',
  TaskToTask = 'task_to_task',
  Workflow = 'workflow'
}

export enum HandoffStatus {
  Pending = 'pending',
  InProgress = 'in_progress',
  Completed = 'completed',
  Failed = 'failed'
}

export interface ContextTransform {
  id: string;
  name: string;
  type: TransformType;
  config: any;
  apply: (data: any) => any;
}

export enum TransformType {
  Map = 'map',
  Filter = 'filter',
  Reduce = 'reduce',
  Format = 'format',
  Enrich = 'enrich',
  Custom = 'custom'
}

export interface ContextBubble {
  id: string;
  name: string;
  taskIds: string[];
  isolation: BubbleIsolation;
  sharedData: Record<string, any>;
  permissions: BubblePermissions;
}

export enum BubbleIsolation {
  Full = 'full',
  Partial = 'partial',
  None = 'none'
}

export interface BubblePermissions {
  read: string[];
  write: string[];
  execute: string[];
}

export interface ContextInheritance {
  strategy: InheritanceStrategy;
  includes: string[];
  excludes: string[];
  overrides: Record<string, any>;
}

export enum InheritanceStrategy {
  All = 'all',
  Selective = 'selective',
  None = 'none',
  Cascade = 'cascade'
}

// Task Definition
export interface Task {
  id: string;
  title: string;
  description: string;
  type: TaskType;
  status: TaskStatus;
  priority: TaskPriority;
  parentId?: string;
  children: string[];
  dependencies: TaskDependency[];
  context: TaskContext;
  assignee?: TaskAssignee;
  labels: string[];
  tags: string[];
  metadata: TaskMetadata;
  execution?: TaskExecution;
  createdAt: Date;
  updatedAt: Date;
  completedAt?: Date;
}

export interface TaskDependency {
  taskId: string;
  type: DependencyType;
  required: boolean;
  description?: string;
}

export enum DependencyType {
  Blocking = 'blocking',
  NonBlocking = 'non_blocking',
  Soft = 'soft',
  Hard = 'hard'
}

export interface TaskAssignee {
  type: AssigneeType;
  id: string;
  name: string;
  capabilities?: string[];
}

export enum AssigneeType {
  Human = 'human',
  Agent = 'agent',
  Tool = 'tool',
  Workflow = 'workflow'
}

export interface TaskMetadata {
  createdBy: string;
  updatedBy: string;
  version: number;
  estimatedDuration?: number;
  actualDuration?: number;
  complexity?: TaskComplexity;
  resources?: TaskResource[];
  customFields?: Record<string, any>;
}

export interface TaskComplexity {
  score: number;
  factors: ComplexityFactor[];
  recommendation?: string;
}

export interface ComplexityFactor {
  name: string;
  weight: number;
  value: number;
  description?: string;
}

export interface TaskResource {
  type: ResourceType;
  id: string;
  name: string;
  required: boolean;
  quantity?: number;
}

export enum ResourceType {
  Agent = 'agent',
  Tool = 'tool',
  Model = 'model',
  Memory = 'memory',
  Compute = 'compute',
  Storage = 'storage'
}

// Task Execution
export interface TaskExecution {
  id: string;
  taskId: string;
  executorType: ExecutorType;
  executorId: string;
  status: ExecutionStatus;
  startedAt: Date;
  completedAt?: Date;
  result?: TaskResult;
  logs: ExecutionLog[];
  metrics: ExecutionMetrics;
}

export enum ExecutorType {
  Agent = 'agent',
  Tool = 'tool',
  Workflow = 'workflow',
  Human = 'human'
}

export enum ExecutionStatus {
  Queued = 'queued',
  Running = 'running',
  Completed = 'completed',
  Failed = 'failed',
  Retrying = 'retrying',
  Cancelled = 'cancelled'
}

export interface TaskResult {
  success: boolean;
  output: any;
  artifacts: string[];
  errors?: TaskError[];
  warnings?: string[];
}

export interface TaskError {
  code: string;
  message: string;
  details?: any;
  timestamp: Date;
}

export interface ExecutionLog {
  timestamp: Date;
  level: LogLevel;
  message: string;
  data?: any;
}

export enum LogLevel {
  Debug = 'debug',
  Info = 'info',
  Warning = 'warning',
  Error = 'error'
}

export interface ExecutionMetrics {
  duration: number;
  tokensUsed?: number;
  cost?: number;
  retries: number;
  resourceUsage?: Record<string, any>;
}

// Task Plans
export interface TaskPlan {
  id: string;
  name: string;
  description: string;
  rootTaskId: string;
  strategy: PlanStrategy;
  constraints: PlanConstraints;
  optimization: PlanOptimization;
  status: PlanStatus;
  createdAt: Date;
  updatedAt: Date;
}

export enum PlanStrategy {
  Sequential = 'sequential',
  Parallel = 'parallel',
  Adaptive = 'adaptive',
  Hybrid = 'hybrid'
}

export interface PlanConstraints {
  maxDuration?: number;
  maxCost?: number;
  requiredCapabilities?: string[];
  resourceLimits?: Record<string, number>;
}

export interface PlanOptimization {
  objective: OptimizationObjective;
  weights: Record<string, number>;
  algorithm?: string;
}

export enum OptimizationObjective {
  MinimizeTime = 'minimize_time',
  MinimizeCost = 'minimize_cost',
  MaximizeQuality = 'maximize_quality',
  Balanced = 'balanced'
}

export enum PlanStatus {
  Draft = 'draft',
  Active = 'active',
  Paused = 'paused',
  Completed = 'completed',
  Archived = 'archived'
}

// Task Templates
export interface TaskTemplate {
  id: string;
  name: string;
  description: string;
  type: TaskType;
  structure: TaskStructure;
  defaultContext: Partial<TaskContext>;
  variables: TemplateVariable[];
  tags: string[];
}

export interface TaskStructure {
  title: string;
  description: string;
  children?: TaskStructure[];
  dependencies?: string[];
  metadata?: Partial<TaskMetadata>;
}

export interface TemplateVariable {
  name: string;
  type: string;
  description?: string;
  required: boolean;
  default?: any;
}

// Events
export interface TaskEvent {
  id: string;
  type: TaskEventType;
  taskId: string;
  timestamp: Date;
  actor: {
    type: string;
    id: string;
  };
  data: any;
}

export enum TaskEventType {
  Created = 'task.created',
  Updated = 'task.updated',
  StatusChanged = 'task.status_changed',
  Assigned = 'task.assigned',
  Started = 'task.started',
  Completed = 'task.completed',
  Failed = 'task.failed',
  ContextUpdated = 'task.context_updated',
  HandoffInitiated = 'task.handoff_initiated',
  HandoffCompleted = 'task.handoff_completed',
  DependencyAdded = 'task.dependency_added',
  DependencyRemoved = 'task.dependency_removed',
  ChildAdded = 'task.child_added',
  ChildRemoved = 'task.child_removed'
}

// Search and Query
export interface TaskQuery {
  text?: string;
  status?: TaskStatus[];
  priority?: TaskPriority[];
  type?: TaskType[];
  assigneeId?: string[];
  labels?: string[];
  tags?: string[];
  parentId?: string;
  hasChildren?: boolean;
  dateRange?: {
    field: 'created' | 'updated' | 'completed';
    from?: Date;
    to?: Date;
  };
  contextQuery?: ContextQuery;
  limit?: number;
  offset?: number;
  sort?: TaskSort[];
}

export interface ContextQuery {
  hasArtifacts?: boolean;
  hasHandoffs?: boolean;
  bubbleId?: string;
  variables?: Record<string, any>;
}

export interface TaskSort {
  field: string;
  direction: 'asc' | 'desc';
}

// Analytics
export interface TaskAnalytics {
  totalTasks: number;
  byStatus: Record<TaskStatus, number>;
  byPriority: Record<TaskPriority, number>;
  byType: Record<TaskType, number>;
  completionRate: number;
  averageDuration: number;
  blockedTasks: number;
  criticalPath: string[];
  bottlenecks: TaskBottleneck[];
}

export interface TaskBottleneck {
  taskId: string;
  reason: string;
  impact: number;
  suggestions: string[];
}