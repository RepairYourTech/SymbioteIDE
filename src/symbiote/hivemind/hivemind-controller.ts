/**
 * Hivemind Controller
 * 
 * Main orchestrator for the Hivemind Parallel Agent System
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { MemoryManager } from '../memory/mem0/memory-manager';
import { TaskDecomposer } from './task-decomposer';
import { ConflictResolver } from './conflict-resolver';
import { ResultAggregator } from './result-aggregator';
import { CoordinationLayer } from './coordination-layer';
import { BaseSpecialistAgent } from './agents/base-specialist';
import { FrontendSpecialistAgent } from './agents/frontend-specialist';
import { BackendSpecialistAgent } from './agents/backend-specialist';
import { TestingSpecialistAgent } from './agents/testing-specialist';
import { SecuritySpecialistAgent } from './agents/security-specialist';
import { DevOpsSpecialistAgent } from './agents/devops-specialist';
import { PerformanceSpecialistAgent } from './agents/performance-specialist';
import {
  HivemindTask,
  HivemindConfig,
  HivemindSession,
  SessionStatus,
  AgentSpecialty,
  TaskStatus,
  ExecutionPlan,
  HivemindResult,
  TaskResult,
  ConflictDetection
} from './types';

export interface HivemindControllerConfig {
  maxConcurrentAgents: number;
  enableMemory: boolean;
  enableAutoDecomposition: boolean;
  orchestrationEngine: OrchestrationEngine;
  memoryManager?: MemoryManager;
  logLevel?: 'debug' | 'info' | 'warn' | 'error';
}

export class HivemindController extends EventEmitter {
  private logger = new Logger('HivemindController');
  private config: Required<HivemindControllerConfig>;
  private orchestrator: OrchestrationEngine;
  private memoryManager?: MemoryManager;
  private taskDecomposer: TaskDecomposer;
  private conflictResolver: ConflictResolver;
  private resultAggregator: ResultAggregator;
  private coordinationLayer: CoordinationLayer;
  
  private agents = new Map<string, BaseSpecialistAgent>();
  private sessions = new Map<string, HivemindSession>();
  private activeTasks = new Map<string, HivemindTask>();
  
  constructor(config: HivemindControllerConfig) {
    super();
    
    this.config = {
      maxConcurrentAgents: config.maxConcurrentAgents || 5,
      enableMemory: config.enableMemory ?? true,
      enableAutoDecomposition: config.enableAutoDecomposition ?? true,
      orchestrationEngine: config.orchestrationEngine,
      memoryManager: config.memoryManager,
      logLevel: config.logLevel || 'info'
    };
    
    this.orchestrator = config.orchestrationEngine;
    this.memoryManager = config.memoryManager;
    
    // Initialize components
    this.taskDecomposer = new TaskDecomposer({
      orchestrationEngine: this.orchestrator,
      maxSubtaskDepth: 3,
      maxSubtasksPerTask: 10,
      enableParallelAnalysis: true
    });
    
    this.conflictResolver = new ConflictResolver();
    this.resultAggregator = new ResultAggregator();
    this.coordinationLayer = new CoordinationLayer({
      maxConcurrentAgents: this.config.maxConcurrentAgents
    });
    
    this.initializeAgents();
    this.setupEventHandlers();
  }
  
  /**
   * Initialize specialist agents
   */
  private initializeAgents(): void {
    const agentConfigs = [
      { id: 'frontend_1', class: FrontendSpecialistAgent },
      { id: 'backend_1', class: BackendSpecialistAgent },
      { id: 'testing_1', class: TestingSpecialistAgent },
      { id: 'security_1', class: SecuritySpecialistAgent },
      { id: 'devops_1', class: DevOpsSpecialistAgent },
      { id: 'performance_1', class: PerformanceSpecialistAgent }
    ];
    
    agentConfigs.forEach(({ id, class: AgentClass }) => {
      const agent = new AgentClass(id);
      this.agents.set(id, agent);
      
      // Connect to memory if enabled
      if (this.config.enableMemory && this.memoryManager) {
        agent.setMemoryManager(this.memoryManager);
      }
      
      // Set up agent event handlers
      agent.on('task-completed', (result) => this.handleTaskCompleted(result));
      agent.on('task-failed', (error) => this.handleTaskFailed(error));
      agent.on('progress', (progress) => this.handleAgentProgress(progress));
    });
  }
  
  /**
   * Set up event handlers
   */
  private setupEventHandlers(): void {
    // Task decomposer events
    this.taskDecomposer.on('task-decomposed', (decomposition) => {
      this.logger.info('Task decomposed', {
        taskId: decomposition.originalTask.id,
        subtasks: decomposition.subtasks.length
      });
    });
    
    // Conflict resolver events
    this.conflictResolver.on('conflict-detected', (conflict) => {
      this.logger.warn('Conflict detected', conflict);
    });
    
    this.conflictResolver.on('conflict-resolved', (resolution) => {
      this.logger.info('Conflict resolved', resolution);
    });
    
    // Coordination layer events
    this.coordinationLayer.on('agent-assigned', ({ agentId, taskId }) => {
      this.logger.info(`Agent ${agentId} assigned to task ${taskId}`);
    });
    
    this.coordinationLayer.on('agent-released', ({ agentId }) => {
      this.logger.info(`Agent ${agentId} released`);
    });
  }
  
  /**
   * Create a new Hivemind session
   */
  async createSession(config: HivemindConfig): Promise<HivemindSession> {
    const session: HivemindSession = {
      id: `session_${Date.now()}`,
      config,
      status: SessionStatus.Initializing,
      tasks: [],
      results: [],
      startTime: new Date(),
      metrics: {
        totalTasks: 0,
        completedTasks: 0,
        failedTasks: 0,
        totalDuration: 0,
        agentUtilization: {}
      }
    };
    
    this.sessions.set(session.id, session);
    this.emit('session-created', session);
    
    // Initialize agents
    await this.initializeSession(session);
    
    session.status = SessionStatus.Ready;
    return session;
  }
  
  /**
   * Initialize session
   */
  private async initializeSession(session: HivemindSession): Promise<void> {
    // Initialize all agents
    const initPromises = Array.from(this.agents.values()).map(agent => 
      agent.initialize(this.orchestrator, this.memoryManager)
    );
    
    await Promise.all(initPromises);
    
    this.logger.info(`Session ${session.id} initialized with ${this.agents.size} agents`);
  }
  
  /**
   * Execute a task with the Hivemind
   */
  async executeTask(task: HivemindTask, sessionId: string): Promise<HivemindResult> {
    const session = this.sessions.get(sessionId);
    if (!session) {
      throw new Error(`Session ${sessionId} not found`);
    }
    
    if (session.status !== SessionStatus.Ready && session.status !== SessionStatus.Running) {
      throw new Error(`Session ${sessionId} is not ready (status: ${session.status})`);
    }
    
    session.status = SessionStatus.Running;
    session.tasks.push(task);
    
    try {
      // Analyze and potentially decompose task
      let executionPlan: ExecutionPlan;
      
      if (this.config.enableAutoDecomposition && task.estimatedComplexity >= 7) {
        // Decompose complex tasks
        const decomposition = await this.taskDecomposer.decomposeTask(task);
        executionPlan = await this.createExecutionPlan(decomposition.subtasks, decomposition.dependencies);
      } else {
        // Simple task - direct execution
        executionPlan = await this.createSimpleExecutionPlan(task);
      }
      
      // Execute the plan
      const result = await this.executePlan(executionPlan, session);
      
      // Update session metrics
      this.updateSessionMetrics(session, result);
      
      return result;
      
    } catch (error) {
      this.logger.error('Task execution failed', error);
      session.status = SessionStatus.Failed;
      throw error;
    }
  }
  
  /**
   * Create execution plan for tasks
   */
  private async createExecutionPlan(
    tasks: HivemindTask[],
    dependencies?: any
  ): Promise<ExecutionPlan> {
    const plan: ExecutionPlan = {
      id: `plan_${Date.now()}`,
      tasks: [],
      executionOrder: [],
      parallelGroups: [],
      estimatedDuration: 0
    };
    
    // Analyze tasks and dependencies
    const taskMap = new Map<string, HivemindTask>();
    tasks.forEach(task => {
      taskMap.set(task.id, task);
      plan.tasks.push(task);
    });
    
    // Determine execution order based on dependencies
    if (dependencies) {
      const sorted = this.topologicalSort(tasks, dependencies);
      plan.executionOrder = sorted.map(t => t.id);
      
      // Group tasks that can run in parallel
      plan.parallelGroups = this.groupParallelTasks(sorted, dependencies);
    } else {
      // No dependencies - all can run in parallel
      plan.executionOrder = tasks.map(t => t.id);
      plan.parallelGroups = [tasks.map(t => t.id)];
    }
    
    // Estimate duration
    plan.estimatedDuration = this.estimatePlanDuration(plan, taskMap);
    
    return plan;
  }
  
  /**
   * Create simple execution plan for single task
   */
  private async createSimpleExecutionPlan(task: HivemindTask): Promise<ExecutionPlan> {
    return {
      id: `plan_${Date.now()}`,
      tasks: [task],
      executionOrder: [task.id],
      parallelGroups: [[task.id]],
      estimatedDuration: task.estimatedComplexity * 60 * 1000 // minutes to ms
    };
  }
  
  /**
   * Execute the plan
   */
  private async executePlan(
    plan: ExecutionPlan,
    session: HivemindSession
  ): Promise<HivemindResult> {
    const startTime = Date.now();
    const results: TaskResult[] = [];
    const issues: string[] = [];
    
    this.logger.info(`Executing plan ${plan.id} with ${plan.tasks.length} tasks`);
    
    // Execute each parallel group
    for (const group of plan.parallelGroups) {
      const groupTasks = group.map(taskId => 
        plan.tasks.find(t => t.id === taskId)!
      );
      
      // Lock files for all tasks in group
      const lockResults = await this.lockTaskFiles(groupTasks);
      
      if (!lockResults.success) {
        issues.push(`Failed to lock files: ${lockResults.conflicts.join(', ')}`);
        continue;
      }
      
      try {
        // Execute tasks in parallel
        const groupResults = await this.executeParallelTasks(groupTasks, session);
        results.push(...groupResults);
        
        // Check for conflicts
        const conflicts = await this.checkForConflicts(groupResults);
        if (conflicts.length > 0) {
          const resolved = await this.resolveConflicts(conflicts);
          issues.push(...resolved.issues);
        }
        
      } finally {
        // Always unlock files
        this.unlockTaskFiles(groupTasks);
      }
    }
    
    // Aggregate results
    const aggregatedResult = await this.resultAggregator.aggregateResults(
      plan.tasks[0], // Original task
      results
    );
    
    const hivemindResult: HivemindResult = {
      sessionId: session.id,
      taskId: plan.tasks[0].id,
      success: aggregatedResult.success,
      output: aggregatedResult.aggregatedOutput,
      individualResults: results,
      filesModified: aggregatedResult.filesModified,
      filesCreated: aggregatedResult.filesCreated,
      issues,
      duration: Date.now() - startTime,
      parallelizationEfficiency: this.calculateEfficiency(plan, results)
    };
    
    session.results.push(hivemindResult);
    
    return hivemindResult;
  }
  
  /**
   * Execute tasks in parallel
   */
  private async executeParallelTasks(
    tasks: HivemindTask[],
    session: HivemindSession
  ): Promise<TaskResult[]> {
    const executionPromises = tasks.map(async (task) => {
      // Find suitable agent
      const agent = await this.coordinationLayer.assignAgent(
        task,
        Array.from(this.agents.values())
      );
      
      if (!agent) {
        throw new Error(`No suitable agent found for task ${task.id}`);
      }
      
      try {
        // Track active task
        this.activeTasks.set(task.id, task);
        
        // Execute task
        const result = await agent.executeTask(task);
        
        // Track file versions for conflict detection
        for (const file of result.filesModified) {
          await this.conflictResolver.trackFileVersion(task.id, file, ''); // Content would be read
        }
        
        return result;
        
      } finally {
        // Release agent and clean up
        this.coordinationLayer.releaseAgent(agent.id);
        this.activeTasks.delete(task.id);
      }
    });
    
    return Promise.all(executionPromises);
  }
  
  /**
   * Lock files for tasks
   */
  private async lockTaskFiles(tasks: HivemindTask[]): Promise<{
    success: boolean;
    conflicts: string[];
  }> {
    const conflicts: string[] = [];
    
    for (const task of tasks) {
      const files = task.context.files || [];
      const locked = await this.conflictResolver.lockFiles(task.id, files);
      
      if (!locked) {
        conflicts.push(task.id);
      }
    }
    
    return {
      success: conflicts.length === 0,
      conflicts
    };
  }
  
  /**
   * Unlock files for tasks
   */
  private unlockTaskFiles(tasks: HivemindTask[]): void {
    tasks.forEach(task => {
      const files = task.context.files || [];
      this.conflictResolver.unlockFiles(task.id, files);
    });
  }
  
  /**
   * Check for conflicts in results
   */
  private async checkForConflicts(results: TaskResult[]): Promise<ConflictDetection[]> {
    const conflicts: ConflictDetection[] = [];
    
    // Check each result for conflicts
    for (const result of results) {
      const files = [...result.filesModified, ...result.filesCreated];
      const conflict = await this.conflictResolver.detectConflicts(result.taskId, files);
      
      if (conflict.hasConflict) {
        conflicts.push(conflict);
      }
    }
    
    return conflicts;
  }
  
  /**
   * Resolve conflicts
   */
  private async resolveConflicts(conflicts: ConflictDetection[]): Promise<{
    issues: string[];
  }> {
    const issues: string[] = [];
    
    for (const conflict of conflicts) {
      try {
        const resolution = await this.conflictResolver.resolveConflicts(conflict);
        
        if (resolution.resolvedBy?.includes('manual')) {
          issues.push(`Manual intervention required for ${conflict.affectedFiles.join(', ')}`);
        }
      } catch (error) {
        issues.push(`Failed to resolve conflict: ${error.message}`);
      }
    }
    
    return { issues };
  }
  
  /**
   * Topological sort for task dependencies
   */
  private topologicalSort(tasks: HivemindTask[], dependencies: any): HivemindTask[] {
    const sorted: HivemindTask[] = [];
    const visited = new Set<string>();
    const taskMap = new Map<string, HivemindTask>();
    
    tasks.forEach(task => taskMap.set(task.id, task));
    
    const visit = (taskId: string): void => {
      if (visited.has(taskId)) return;
      visited.add(taskId);
      
      const task = taskMap.get(taskId);
      if (!task) return;
      
      // Visit dependencies first
      task.dependencies.forEach(dep => visit(dep));
      
      sorted.push(task);
    };
    
    tasks.forEach(task => visit(task.id));
    
    return sorted;
  }
  
  /**
   * Group tasks that can run in parallel
   */
  private groupParallelTasks(
    tasks: HivemindTask[],
    dependencies: any
  ): string[][] {
    const groups: string[][] = [];
    const assigned = new Set<string>();
    
    tasks.forEach(task => {
      if (assigned.has(task.id)) return;
      
      const group: string[] = [task.id];
      assigned.add(task.id);
      
      // Find other tasks that can run in parallel
      tasks.forEach(otherTask => {
        if (assigned.has(otherTask.id)) return;
        if (this.canRunInParallel(task, otherTask, dependencies)) {
          group.push(otherTask.id);
          assigned.add(otherTask.id);
        }
      });
      
      groups.push(group);
    });
    
    return groups;
  }
  
  /**
   * Check if two tasks can run in parallel
   */
  private canRunInParallel(
    task1: HivemindTask,
    task2: HivemindTask,
    dependencies: any
  ): boolean {
    // Check if they have conflicting file access
    const files1 = new Set(task1.context.files || []);
    const files2 = new Set(task2.context.files || []);
    
    for (const file of files1) {
      if (files2.has(file)) return false;
    }
    
    // Check if one depends on the other
    if (task1.dependencies.includes(task2.id) || 
        task2.dependencies.includes(task1.id)) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Estimate plan duration
   */
  private estimatePlanDuration(
    plan: ExecutionPlan,
    taskMap: Map<string, HivemindTask>
  ): number {
    let maxDuration = 0;
    
    // Calculate critical path duration
    plan.parallelGroups.forEach(group => {
      const groupDuration = Math.max(...group.map(taskId => {
        const task = taskMap.get(taskId);
        return task ? task.estimatedComplexity * 30 * 60 * 1000 : 0;
      }));
      maxDuration += groupDuration;
    });
    
    return maxDuration;
  }
  
  /**
   * Calculate parallelization efficiency
   */
  private calculateEfficiency(plan: ExecutionPlan, results: TaskResult[]): number {
    const totalSequentialTime = results.reduce(
      (sum, r) => sum + r.performanceMetrics.duration,
      0
    );
    
    const actualTime = Math.max(...results.map(r => r.performanceMetrics.duration));
    
    return actualTime > 0 ? totalSequentialTime / actualTime : 1;
  }
  
  /**
   * Update session metrics
   */
  private updateSessionMetrics(session: HivemindSession, result: HivemindResult): void {
    session.metrics.totalTasks++;
    
    if (result.success) {
      session.metrics.completedTasks++;
    } else {
      session.metrics.failedTasks++;
    }
    
    session.metrics.totalDuration += result.duration;
    
    // Update agent utilization
    result.individualResults.forEach(taskResult => {
      const agentId = this.findAgentForTask(taskResult.taskId);
      if (agentId) {
        session.metrics.agentUtilization[agentId] = 
          (session.metrics.agentUtilization[agentId] || 0) + 1;
      }
    });
  }
  
  /**
   * Find which agent executed a task
   */
  private findAgentForTask(taskId: string): string | null {
    // This would be tracked during execution
    return null;
  }
  
  /**
   * Handle task completed
   */
  private handleTaskCompleted(result: TaskResult): void {
    this.logger.info(`Task ${result.taskId} completed successfully`);
    this.emit('task-completed', result);
    
    // Clear task versions from conflict resolver
    this.conflictResolver.clearTaskVersions(result.taskId);
  }
  
  /**
   * Handle task failed
   */
  private handleTaskFailed(error: any): void {
    this.logger.error('Task failed', error);
    this.emit('task-failed', error);
  }
  
  /**
   * Handle agent progress
   */
  private handleAgentProgress(progress: any): void {
    this.emit('agent-progress', progress);
  }
  
  /**
   * Get session status
   */
  getSession(sessionId: string): HivemindSession | undefined {
    return this.sessions.get(sessionId);
  }
  
  /**
   * Close session
   */
  async closeSession(sessionId: string): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (!session) return;
    
    session.status = SessionStatus.Completed;
    session.endTime = new Date();
    
    // Clean up resources
    await this.cleanupSession(session);
    
    this.emit('session-closed', session);
  }
  
  /**
   * Clean up session resources
   */
  private async cleanupSession(session: HivemindSession): Promise<void> {
    // Clean up any remaining locks
    this.activeTasks.forEach((task, taskId) => {
      const files = task.context.files || [];
      this.conflictResolver.unlockFiles(taskId, files);
    });
    
    this.activeTasks.clear();
    
    // Reset agents
    const resetPromises = Array.from(this.agents.values()).map(agent => 
      agent.reset()
    );
    
    await Promise.all(resetPromises);
  }
  
  /**
   * Get agent statistics
   */
  getAgentStats(): Map<string, any> {
    const stats = new Map<string, any>();
    
    this.agents.forEach((agent, id) => {
      stats.set(id, {
        name: agent.name,
        specialty: agent.specialty,
        capabilities: agent.capabilities,
        performanceHistory: agent.getPerformanceHistory()
      });
    });
    
    return stats;
  }
  
  /**
   * Shutdown the Hivemind
   */
  async shutdown(): Promise<void> {
    this.logger.info('Shutting down Hivemind Controller');
    
    // Close all sessions
    const closePromises = Array.from(this.sessions.keys()).map(sessionId =>
      this.closeSession(sessionId)
    );
    
    await Promise.all(closePromises);
    
    // Clean up agents
    this.agents.clear();
    
    // Remove all listeners
    this.removeAllListeners();
  }
}