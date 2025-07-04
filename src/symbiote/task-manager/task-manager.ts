/**
 * Task Manager
 * 
 * Core implementation of the comprehensive task management system
 */

import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import {
  Task,
  TaskStatus,
  TaskPriority,
  TaskType,
  TaskContext,
  TaskDependency,
  TaskEvent,
  TaskEventType,
  TaskQuery,
  TaskPlan,
  TaskTemplate,
  TaskAnalytics,
  ContextScope,
  ContextHandoff,
  HandoffStatus,
  ContextBubble,
  InheritanceStrategy,
  TaskExecution,
  ExecutionStatus,
  ExecutorType,
  DependencyType
} from './types';
import { ContextManager } from './context-manager';
import { TaskStore } from './task-store';
import { TaskExecutor } from './task-executor';
import { TaskAnalyzer } from './task-analyzer';
import { HandoffCoordinationAgent } from './handoff-coordination-agent';
import { Logger } from '../utils/logger';
import { Redis } from 'ioredis';
import { Neo4jConnectionManager } from '../neo4j/connection-manager';

export class TaskManager extends EventEmitter {
  private logger = new Logger('TaskManager');
  private contextManager: ContextManager;
  private taskStore: TaskStore;
  private taskExecutor: TaskExecutor;
  private taskAnalyzer: TaskAnalyzer;
  private redis?: Redis;
  private neo4j?: Neo4jConnectionManager;
  private isInitialized = false;

  constructor(private config: TaskManagerConfig = {}) {
    super();
    this.contextManager = new ContextManager();
    this.taskStore = new TaskStore(config.storage);
    this.taskExecutor = new TaskExecutor(this);
    this.taskAnalyzer = new TaskAnalyzer(this);
  }

  /**
   * Initialize task manager
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    try {
      // Initialize Redis if configured
      if (this.config.redis) {
        const { createRedisClient } = await import('../infrastructure/redis/redis-client');
        this.redis = createRedisClient();
      }

      // Initialize Neo4j if configured
      if (this.config.neo4j) {
        this.neo4j = Neo4jConnectionManager.getInstance();
      }

      // Initialize components
      await this.taskStore.initialize();
      await this.contextManager.initialize();
      await this.taskExecutor.initialize();
      await this.taskAnalyzer.initialize();

      // Initialize handoff coordinator if using optimization agents
      if (this.config.useIntelligentHandoffs) {
        const handoffCoordinator = new HandoffCoordinationAgent(
          this.contextManager,
          this
        );
        await handoffCoordinator.initialize();
        this.contextManager.setHandoffCoordinator(handoffCoordinator);
      }

      this.isInitialized = true;
      this.logger.info('Task Manager initialized');
      this.emit('initialized');
    } catch (error) {
      this.logger.error('Failed to initialize task manager', error);
      throw error;
    }
  }

  /**
   * Create a new task
   */
  async createTask(params: CreateTaskParams): Promise<Task> {
    try {
      // Generate task ID
      const taskId = params.id || `task_${uuidv4()}`;

      // Create context for the task
      const context = await this.contextManager.createContext(taskId, {
        parentContextId: params.parentId ? 
          await this.getTaskContext(params.parentId) : undefined,
        scope: params.contextScope || ContextScope.Local,
        data: params.contextData || {},
        variables: params.variables || {},
        inheritance: params.contextInheritance || {
          strategy: InheritanceStrategy.Cascade,
          includes: [],
          excludes: [],
          overrides: {}
        }
      });

      // Create task
      const task: Task = {
        id: taskId,
        title: params.title,
        description: params.description || '',
        type: params.type || TaskType.Task,
        status: params.status || TaskStatus.Pending,
        priority: params.priority || TaskPriority.Medium,
        parentId: params.parentId,
        children: [],
        dependencies: params.dependencies || [],
        context,
        assignee: params.assignee,
        labels: params.labels || [],
        tags: params.tags || [],
        metadata: {
          createdBy: params.createdBy || 'system',
          updatedBy: params.createdBy || 'system',
          version: 1,
          estimatedDuration: params.estimatedDuration,
          complexity: params.complexity,
          resources: params.resources,
          customFields: params.customFields
        },
        createdAt: new Date(),
        updatedAt: new Date()
      };

      // Validate dependencies
      if (task.dependencies.length > 0) {
        await this.validateDependencies(task.dependencies);
      }

      // Add to parent if specified
      if (params.parentId) {
        await this.addChildToParent(params.parentId, taskId);
      }

      // Store task
      await this.taskStore.save(task);

      // Store in graph if Neo4j is enabled
      if (this.neo4j) {
        await this.storeTaskInGraph(task);
      }

      // Emit event
      this.emitTaskEvent(TaskEventType.Created, task);

      this.logger.info(`Created task: ${taskId}`);
      return task;
    } catch (error) {
      this.logger.error('Failed to create task', error);
      throw error;
    }
  }

  /**
   * Update task
   */
  async updateTask(taskId: string, updates: UpdateTaskParams): Promise<Task> {
    try {
      const task = await this.getTask(taskId);
      if (!task) {
        throw new Error(`Task ${taskId} not found`);
      }

      // Update fields
      const updatedTask: Task = {
        ...task,
        ...updates,
        metadata: {
          ...task.metadata,
          ...updates.metadata,
          updatedBy: updates.updatedBy || task.metadata.updatedBy,
          version: task.metadata.version + 1
        },
        updatedAt: new Date()
      };

      // Handle status changes
      if (updates.status && updates.status !== task.status) {
        await this.handleStatusChange(updatedTask, task.status, updates.status);
      }

      // Update context if provided
      if (updates.contextData || updates.variables) {
        await this.contextManager.updateContext(task.context.id, {
          data: { ...task.context.data, ...updates.contextData },
          variables: { ...task.context.variables, ...updates.variables }
        });
      }

      // Validate dependencies if updated
      if (updates.dependencies) {
        await this.validateDependencies(updates.dependencies);
      }

      // Store updated task
      await this.taskStore.save(updatedTask);

      // Update graph if Neo4j is enabled
      if (this.neo4j) {
        await this.updateTaskInGraph(updatedTask);
      }

      // Emit event
      this.emitTaskEvent(TaskEventType.Updated, updatedTask);

      return updatedTask;
    } catch (error) {
      this.logger.error(`Failed to update task ${taskId}`, error);
      throw error;
    }
  }

  /**
   * Get task by ID
   */
  async getTask(taskId: string, includeChildren = false): Promise<Task | null> {
    try {
      const task = await this.taskStore.get(taskId);
      if (!task) {
        return null;
      }

      if (includeChildren && task.children.length > 0) {
        // Load children recursively
        const children = await this.getTaskChildren(taskId, true);
        return { ...task, childTasks: children };
      }

      return task;
    } catch (error) {
      this.logger.error(`Failed to get task ${taskId}`, error);
      throw error;
    }
  }

  /**
   * Get task children
   */
  async getTaskChildren(taskId: string, recursive = false): Promise<Task[]> {
    try {
      const task = await this.getTask(taskId);
      if (!task || task.children.length === 0) {
        return [];
      }

      const children: Task[] = [];
      for (const childId of task.children) {
        const child = await this.getTask(childId, recursive);
        if (child) {
          children.push(child);
        }
      }

      return children;
    } catch (error) {
      this.logger.error(`Failed to get children for task ${taskId}`, error);
      throw error;
    }
  }

  /**
   * Search tasks
   */
  async searchTasks(query: TaskQuery): Promise<{ tasks: Task[]; total: number }> {
    try {
      return await this.taskStore.search(query);
    } catch (error) {
      this.logger.error('Failed to search tasks', error);
      throw error;
    }
  }

  /**
   * Execute task
   */
  async executeTask(taskId: string, executorId: string, executorType: ExecutorType): Promise<TaskExecution> {
    try {
      const task = await this.getTask(taskId);
      if (!task) {
        throw new Error(`Task ${taskId} not found`);
      }

      // Check dependencies
      const readyToExecute = await this.checkDependencies(task);
      if (!readyToExecute) {
        throw new Error('Task dependencies not satisfied');
      }

      // Update task status
      await this.updateTask(taskId, { status: TaskStatus.InProgress });

      // Execute via executor
      const execution = await this.taskExecutor.execute(task, executorId, executorType);

      // Update task with execution info
      await this.updateTask(taskId, { execution });

      return execution;
    } catch (error) {
      this.logger.error(`Failed to execute task ${taskId}`, error);
      throw error;
    }
  }

  /**
   * Create task plan
   */
  async createPlan(rootTask: CreateTaskParams, strategy?: any): Promise<TaskPlan> {
    try {
      // Create root task
      const task = await this.createTask(rootTask);

      // Create plan
      const plan: TaskPlan = {
        id: `plan_${uuidv4()}`,
        name: rootTask.title,
        description: rootTask.description || '',
        rootTaskId: task.id,
        strategy: strategy?.type || 'sequential',
        constraints: strategy?.constraints || {},
        optimization: strategy?.optimization || {
          objective: 'balanced',
          weights: { time: 0.5, cost: 0.5 }
        },
        status: 'draft',
        createdAt: new Date(),
        updatedAt: new Date()
      };

      // Store plan
      await this.taskStore.savePlan(plan);

      return plan;
    } catch (error) {
      this.logger.error('Failed to create plan', error);
      throw error;
    }
  }

  /**
   * Create context handoff between tasks
   */
  async createHandoff(params: CreateHandoffParams): Promise<ContextHandoff> {
    try {
      // Use intelligent handoff if configured
      const handoff = this.config.useIntelligentHandoffs
        ? await this.contextManager.createIntelligentHandoff({
            type: params.type,
            fromTaskId: params.fromTaskId,
            toTaskId: params.toTaskId,
            fromAgentId: params.fromAgentId,
            toAgentId: params.toAgentId,
            fromModelId: params.fromModelId,
            toModelId: params.toModelId,
            data: params.data,
            transforms: params.transforms || [],
            preservePriority: params.preservePriority
          })
        : await this.contextManager.createHandoff({
            type: params.type,
            fromTaskId: params.fromTaskId,
            toTaskId: params.toTaskId,
            fromAgentId: params.fromAgentId,
            toAgentId: params.toAgentId,
            fromModelId: params.fromModelId,
            toModelId: params.toModelId,
            data: params.data,
            transforms: params.transforms || [],
            preservePriority: params.preservePriority
          });

      // Emit event
      this.emitTaskEvent(TaskEventType.HandoffInitiated, 
        await this.getTask(params.fromTaskId),
        { handoff }
      );

      return handoff;
    } catch (error) {
      this.logger.error('Failed to create handoff', error);
      throw error;
    }
  }

  /**
   * Complete handoff
   */
  async completeHandoff(handoffId: string, result?: any): Promise<void> {
    try {
      const handoff = await this.contextManager.completeHandoff(handoffId, result);

      // Emit event
      this.emitTaskEvent(TaskEventType.HandoffCompleted,
        await this.getTask(handoff.toTaskId),
        { handoff, result }
      );
    } catch (error) {
      this.logger.error(`Failed to complete handoff ${handoffId}`, error);
      throw error;
    }
  }

  /**
   * Create context bubble
   */
  async createContextBubble(params: CreateBubbleParams): Promise<ContextBubble> {
    try {
      return await this.contextManager.createBubble(params);
    } catch (error) {
      this.logger.error('Failed to create context bubble', error);
      throw error;
    }
  }

  /**
   * Get task analytics
   */
  async getAnalytics(options?: any): Promise<TaskAnalytics> {
    try {
      return await this.taskAnalyzer.analyze(options);
    } catch (error) {
      this.logger.error('Failed to get analytics', error);
      throw error;
    }
  }

  /**
   * Get task context
   */
  private async getTaskContext(taskId: string): Promise<string | undefined> {
    const task = await this.getTask(taskId);
    return task?.context.id;
  }

  /**
   * Validate dependencies
   */
  private async validateDependencies(dependencies: TaskDependency[]): Promise<void> {
    for (const dep of dependencies) {
      const task = await this.getTask(dep.taskId);
      if (!task) {
        throw new Error(`Dependency task ${dep.taskId} not found`);
      }
    }
  }

  /**
   * Check if dependencies are satisfied
   */
  private async checkDependencies(task: Task): Promise<boolean> {
    if (task.dependencies.length === 0) {
      return true;
    }

    for (const dep of task.dependencies) {
      const depTask = await this.getTask(dep.taskId);
      if (!depTask) {
        return false;
      }

      // Check based on dependency type
      if (dep.type === DependencyType.Blocking || dep.type === DependencyType.Hard) {
        if (depTask.status !== TaskStatus.Completed) {
          return false;
        }
      }
    }

    return true;
  }

  /**
   * Add child to parent
   */
  private async addChildToParent(parentId: string, childId: string): Promise<void> {
    const parent = await this.getTask(parentId);
    if (!parent) {
      throw new Error(`Parent task ${parentId} not found`);
    }

    if (!parent.children.includes(childId)) {
      parent.children.push(childId);
      await this.taskStore.save(parent);
    }
  }

  /**
   * Handle status change
   */
  private async handleStatusChange(
    task: Task,
    oldStatus: TaskStatus,
    newStatus: TaskStatus
  ): Promise<void> {
    // Mark completion time
    if (newStatus === TaskStatus.Completed && !task.completedAt) {
      task.completedAt = new Date();
    }

    // Update actual duration
    if (newStatus === TaskStatus.Completed && task.execution?.startedAt) {
      task.metadata.actualDuration = 
        task.completedAt!.getTime() - task.execution.startedAt.getTime();
    }

    // Emit status change event
    this.emitTaskEvent(TaskEventType.StatusChanged, task, {
      oldStatus,
      newStatus
    });
  }

  /**
   * Store task in graph database
   */
  private async storeTaskInGraph(task: Task): Promise<void> {
    if (!this.neo4j) return;

    try {
      const session = await this.neo4j.getSession();
      
      await session.run(
        `
        MERGE (t:Task {id: $id})
        SET t += $properties
        `,
        {
          id: task.id,
          properties: {
            title: task.title,
            type: task.type,
            status: task.status,
            priority: task.priority,
            createdAt: task.createdAt.toISOString(),
            updatedAt: task.updatedAt.toISOString()
          }
        }
      );

      // Create parent relationship
      if (task.parentId) {
        await session.run(
          `
          MATCH (p:Task {id: $parentId})
          MATCH (c:Task {id: $childId})
          MERGE (p)-[:HAS_CHILD]->(c)
          `,
          { parentId: task.parentId, childId: task.id }
        );
      }

      // Create dependency relationships
      for (const dep of task.dependencies) {
        await session.run(
          `
          MATCH (t:Task {id: $taskId})
          MATCH (d:Task {id: $depId})
          MERGE (t)-[:DEPENDS_ON {type: $type}]->(d)
          `,
          { taskId: task.id, depId: dep.taskId, type: dep.type }
        );
      }

      await session.close();
    } catch (error) {
      this.logger.error('Failed to store task in graph', error);
    }
  }

  /**
   * Update task in graph database
   */
  private async updateTaskInGraph(task: Task): Promise<void> {
    if (!this.neo4j) return;

    try {
      const session = await this.neo4j.getSession();
      
      await session.run(
        `
        MATCH (t:Task {id: $id})
        SET t += $properties
        `,
        {
          id: task.id,
          properties: {
            title: task.title,
            status: task.status,
            priority: task.priority,
            updatedAt: task.updatedAt.toISOString(),
            completedAt: task.completedAt?.toISOString()
          }
        }
      );

      await session.close();
    } catch (error) {
      this.logger.error('Failed to update task in graph', error);
    }
  }

  /**
   * Emit task event
   */
  private emitTaskEvent(type: TaskEventType, task: Task | null, data?: any): void {
    if (!task) return;

    const event: TaskEvent = {
      id: uuidv4(),
      type,
      taskId: task.id,
      timestamp: new Date(),
      actor: {
        type: 'system',
        id: 'task-manager'
      },
      data: data || {}
    };

    this.emit(type, event);
    this.emit('task:event', event);
  }

  /**
   * Shutdown task manager
   */
  async shutdown(): Promise<void> {
    try {
      await this.taskStore.close();
      await this.contextManager.close();
      
      if (this.redis) {
        await this.redis.quit();
      }

      this.isInitialized = false;
      this.logger.info('Task Manager shutdown');
    } catch (error) {
      this.logger.error('Error during shutdown', error);
      throw error;
    }
  }
}

// Configuration interface
export interface TaskManagerConfig {
  storage?: {
    type: 'memory' | 'redis' | 'postgres';
    config?: any;
  };
  redis?: boolean;
  neo4j?: boolean;
  useIntelligentHandoffs?: boolean;
}

// Parameter interfaces
export interface CreateTaskParams {
  id?: string;
  title: string;
  description?: string;
  type?: TaskType;
  status?: TaskStatus;
  priority?: TaskPriority;
  parentId?: string;
  dependencies?: TaskDependency[];
  assignee?: any;
  labels?: string[];
  tags?: string[];
  contextScope?: ContextScope;
  contextData?: Record<string, any>;
  variables?: Record<string, any>;
  contextInheritance?: any;
  estimatedDuration?: number;
  complexity?: any;
  resources?: any[];
  customFields?: Record<string, any>;
  createdBy?: string;
}

export interface UpdateTaskParams {
  title?: string;
  description?: string;
  status?: TaskStatus;
  priority?: TaskPriority;
  dependencies?: TaskDependency[];
  assignee?: any;
  labels?: string[];
  tags?: string[];
  contextData?: Record<string, any>;
  variables?: Record<string, any>;
  metadata?: Partial<any>;
  execution?: TaskExecution;
  updatedBy?: string;
}

export interface CreateHandoffParams {
  type: any;
  fromTaskId: string;
  toTaskId: string;
  fromAgentId?: string;
  toAgentId?: string;
  fromModelId?: string;
  toModelId?: string;
  data: any;
  transforms?: any[];
  preservePriority?: string[];
}

export interface CreateBubbleParams {
  name: string;
  taskIds: string[];
  isolation: any;
  sharedData?: Record<string, any>;
  permissions?: any;
}