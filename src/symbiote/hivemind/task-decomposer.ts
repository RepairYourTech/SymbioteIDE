/**
 * Task Decomposer
 * 
 * Intelligently breaks down complex tasks into manageable subtasks
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import {
  HivemindTask,
  TaskDecomposition,
  TaskDependencyGraph,
  AgentSpecialty,
  TaskPriority
} from './types';

export interface DecomposerConfig {
  maxSubtaskDepth: number;
  maxSubtasksPerTask: number;
  enableParallelAnalysis: boolean;
  orchestrationEngine: OrchestrationEngine;
}

export class TaskDecomposer extends EventEmitter {
  private logger = new Logger('TaskDecomposer');
  private config: Required<DecomposerConfig>;
  private orchestrator: OrchestrationEngine;
  
  constructor(config: DecomposerConfig) {
    super();
    
    this.config = {
      maxSubtaskDepth: config.maxSubtaskDepth || 3,
      maxSubtasksPerTask: config.maxSubtasksPerTask || 10,
      enableParallelAnalysis: config.enableParallelAnalysis ?? true,
      orchestrationEngine: config.orchestrationEngine
    };
    
    this.orchestrator = config.orchestrationEngine;
  }
  
  /**
   * Decompose a complex task into subtasks
   */
  async decomposeTask(task: HivemindTask): Promise<TaskDecomposition> {
    this.logger.info(`Decomposing task: ${task.title}`);
    
    try {
      // Analyze task complexity
      const complexity = await this.analyzeTaskComplexity(task);
      
      // Determine decomposition strategy
      const strategy = this.determineStrategy(task, complexity);
      
      // Generate subtasks
      const subtasks = await this.generateSubtasks(task, strategy);
      
      // Build dependency graph
      const dependencies = this.buildDependencyGraph(subtasks);
      
      // Calculate metrics
      const metrics = this.calculateDecompositionMetrics(subtasks, dependencies);
      
      const decomposition: TaskDecomposition = {
        originalTask: task,
        subtasks,
        dependencies,
        estimatedTotalTime: metrics.totalTime,
        parallelizationFactor: metrics.parallelizationFactor
      };
      
      this.emit('task-decomposed', decomposition);
      
      return decomposition;
      
    } catch (error) {
      this.logger.error('Failed to decompose task', error);
      throw error;
    }
  }
  
  /**
   * Analyze task complexity
   */
  private async analyzeTaskComplexity(task: HivemindTask): Promise<any> {
    const prompt = `
Analyze the complexity of this software development task:

Title: ${task.title}
Description: ${task.description}
Required Specialties: ${task.requiredSpecialties.join(', ')}

Provide a complexity analysis with:
1. Overall complexity score (1-10)
2. Number of distinct components/modules involved
3. Estimated lines of code
4. Required expertise areas
5. Potential risks or challenges
6. Suggested decomposition approach

Return as JSON.`;
    
    const result = await this.orchestrator.executeTask({
      id: `analyze_${task.id}`,
      type: 'analysis',
      prompt,
      context: {},
      parameters: {
        temperature: 0.3,
        maxTokens: 1000
      }
    });
    
    try {
      return JSON.parse(result.response);
    } catch {
      // Fallback to default complexity
      return {
        complexityScore: task.estimatedComplexity || 5,
        components: 3,
        estimatedLOC: 500,
        expertiseAreas: task.requiredSpecialties,
        risks: [],
        approach: 'sequential'
      };
    }
  }
  
  /**
   * Determine decomposition strategy
   */
  private determineStrategy(task: HivemindTask, complexity: any): string {
    // High complexity tasks need more careful decomposition
    if (complexity.complexityScore >= 8) {
      return 'hierarchical';
    }
    
    // Tasks with many components can be parallelized
    if (complexity.components >= 5) {
      return 'parallel';
    }
    
    // Tasks requiring multiple specialties benefit from pipeline approach
    if (task.requiredSpecialties.length >= 3) {
      return 'pipeline';
    }
    
    // Default to sequential for simpler tasks
    return 'sequential';
  }
  
  /**
   * Generate subtasks based on strategy
   */
  private async generateSubtasks(
    task: HivemindTask,
    strategy: string
  ): Promise<HivemindTask[]> {
    const prompt = `
Decompose this software development task into subtasks using a ${strategy} approach:

Title: ${task.title}
Description: ${task.description}
Required Specialties: ${task.requiredSpecialties.join(', ')}
Context: ${JSON.stringify(task.context)}

Generate subtasks with:
1. Clear titles and descriptions
2. Required specialties for each subtask
3. Estimated complexity (1-10)
4. Dependencies between subtasks
5. Whether subtasks can run in parallel

Maximum ${this.config.maxSubtasksPerTask} subtasks.
Return as JSON array.`;
    
    const result = await this.orchestrator.executeTask({
      id: `decompose_${task.id}`,
      type: 'task_decomposition',
      prompt,
      context: {},
      parameters: {
        temperature: 0.5,
        maxTokens: 2000
      }
    });
    
    try {
      const subtaskData = JSON.parse(result.response);
      
      // Convert to HivemindTask objects
      const subtasks: HivemindTask[] = subtaskData.map((data: any, index: number) => ({
        id: `${task.id}_sub${index + 1}`,
        type: data.type || task.type,
        title: data.title,
        description: data.description,
        requiredSpecialties: this.parseSpecialties(data.requiredSpecialties),
        priority: task.priority,
        status: 'pending',
        dependencies: data.dependencies || [],
        assignedAgents: [],
        estimatedComplexity: data.complexity || 5,
        context: {
          ...task.context,
          parentTask: task.id,
          subtaskIndex: index
        },
        createdAt: new Date()
      }));
      
      return subtasks;
      
    } catch (error) {
      this.logger.error('Failed to parse subtasks', error);
      
      // Fallback to simple decomposition
      return this.createFallbackSubtasks(task);
    }
  }
  
  /**
   * Parse specialties from string array
   */
  private parseSpecialties(specialties: string[]): AgentSpecialty[] {
    const specialtyMap: { [key: string]: AgentSpecialty } = {
      'frontend': AgentSpecialty.Frontend,
      'backend': AgentSpecialty.Backend,
      'testing': AgentSpecialty.Testing,
      'security': AgentSpecialty.Security,
      'devops': AgentSpecialty.DevOps,
      'performance': AgentSpecialty.Performance,
      'database': AgentSpecialty.Database,
      'documentation': AgentSpecialty.Documentation,
      'architecture': AgentSpecialty.Architecture
    };
    
    return specialties
      .map(s => specialtyMap[s.toLowerCase()])
      .filter(s => s !== undefined);
  }
  
  /**
   * Create fallback subtasks
   */
  private createFallbackSubtasks(task: HivemindTask): HivemindTask[] {
    const subtasks: HivemindTask[] = [];
    
    // Create basic subtasks based on required specialties
    task.requiredSpecialties.forEach((specialty, index) => {
      subtasks.push({
        id: `${task.id}_sub${index + 1}`,
        type: task.type,
        title: `${specialty} implementation for ${task.title}`,
        description: `Handle ${specialty} aspects of: ${task.description}`,
        requiredSpecialties: [specialty],
        priority: task.priority,
        status: 'pending',
        dependencies: index > 0 ? [`${task.id}_sub${index}`] : [],
        assignedAgents: [],
        estimatedComplexity: Math.ceil(task.estimatedComplexity / task.requiredSpecialties.length),
        context: {
          ...task.context,
          parentTask: task.id,
          subtaskIndex: index
        },
        createdAt: new Date()
      });
    });
    
    return subtasks;
  }
  
  /**
   * Build dependency graph
   */
  private buildDependencyGraph(subtasks: HivemindTask[]): TaskDependencyGraph {
    const nodes = new Map<string, HivemindTask>();
    const edges = new Map<string, string[]>();
    
    // Add all nodes
    subtasks.forEach(task => {
      nodes.set(task.id, task);
      edges.set(task.id, []);
    });
    
    // Build edges based on dependencies
    subtasks.forEach(task => {
      task.dependencies.forEach(depId => {
        const dependentTasks = edges.get(depId);
        if (dependentTasks) {
          dependentTasks.push(task.id);
        }
      });
    });
    
    // Validate graph (check for cycles)
    if (this.hasCycles(edges)) {
      this.logger.warn('Dependency graph contains cycles, attempting to fix');
      this.removeCycles(edges);
    }
    
    return { nodes, edges };
  }
  
  /**
   * Check if dependency graph has cycles
   */
  private hasCycles(edges: Map<string, string[]>): boolean {
    const visited = new Set<string>();
    const recursionStack = new Set<string>();
    
    const hasCycleDFS = (node: string): boolean => {
      visited.add(node);
      recursionStack.add(node);
      
      const neighbors = edges.get(node) || [];
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          if (hasCycleDFS(neighbor)) {
            return true;
          }
        } else if (recursionStack.has(neighbor)) {
          return true;
        }
      }
      
      recursionStack.delete(node);
      return false;
    };
    
    for (const node of edges.keys()) {
      if (!visited.has(node)) {
        if (hasCycleDFS(node)) {
          return true;
        }
      }
    }
    
    return false;
  }
  
  /**
   * Remove cycles from dependency graph
   */
  private removeCycles(edges: Map<string, string[]>): void {
    // Simple approach: remove back edges
    const visited = new Set<string>();
    const recursionStack = new Set<string>();
    
    const removeCyclesDFS = (node: string): void => {
      visited.add(node);
      recursionStack.add(node);
      
      const neighbors = edges.get(node) || [];
      const validNeighbors: string[] = [];
      
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          removeCyclesDFS(neighbor);
          validNeighbors.push(neighbor);
        } else if (!recursionStack.has(neighbor)) {
          validNeighbors.push(neighbor);
        }
        // Skip if neighbor is in recursion stack (back edge)
      }
      
      edges.set(node, validNeighbors);
      recursionStack.delete(node);
    };
    
    for (const node of edges.keys()) {
      if (!visited.has(node)) {
        removeCyclesDFS(node);
      }
    }
  }
  
  /**
   * Calculate decomposition metrics
   */
  private calculateDecompositionMetrics(
    subtasks: HivemindTask[],
    dependencies: TaskDependencyGraph
  ): { totalTime: number; parallelizationFactor: number } {
    // Calculate critical path
    const criticalPathLength = this.calculateCriticalPath(subtasks, dependencies);
    
    // Calculate total work
    const totalWork = subtasks.reduce(
      (sum, task) => sum + task.estimatedComplexity,
      0
    );
    
    // Estimate time (complexity * 30 minutes per unit)
    const totalTime = criticalPathLength * 30 * 60 * 1000; // milliseconds
    
    // Calculate parallelization factor
    const parallelizationFactor = Math.min(
      1,
      criticalPathLength / totalWork
    );
    
    return { totalTime, parallelizationFactor };
  }
  
  /**
   * Calculate critical path length
   */
  private calculateCriticalPath(
    subtasks: HivemindTask[],
    dependencies: TaskDependencyGraph
  ): number {
    const taskWeights = new Map<string, number>();
    const maxPathLength = new Map<string, number>();
    
    // Initialize weights
    subtasks.forEach(task => {
      taskWeights.set(task.id, task.estimatedComplexity);
      maxPathLength.set(task.id, 0);
    });
    
    // Topological sort
    const sorted = this.topologicalSort(dependencies);
    
    // Calculate longest path
    sorted.forEach(taskId => {
      const task = dependencies.nodes.get(taskId);
      if (!task) return;
      
      const weight = taskWeights.get(taskId) || 0;
      let maxPredecessorLength = 0;
      
      // Find max length among predecessors
      task.dependencies.forEach(depId => {
        const predLength = maxPathLength.get(depId) || 0;
        maxPredecessorLength = Math.max(maxPredecessorLength, predLength);
      });
      
      maxPathLength.set(taskId, maxPredecessorLength + weight);
    });
    
    // Return maximum path length
    return Math.max(...Array.from(maxPathLength.values()));
  }
  
  /**
   * Topological sort of tasks
   */
  private topologicalSort(dependencies: TaskDependencyGraph): string[] {
    const sorted: string[] = [];
    const visited = new Set<string>();
    
    const visit = (node: string): void => {
      if (visited.has(node)) return;
      visited.add(node);
      
      // Visit dependencies first
      const task = dependencies.nodes.get(node);
      if (task) {
        task.dependencies.forEach(dep => visit(dep));
      }
      
      sorted.push(node);
    };
    
    // Visit all nodes
    dependencies.nodes.forEach((_, nodeId) => visit(nodeId));
    
    return sorted;
  }
  
  /**
   * Re-decompose a failed task
   */
  async redecomposeTask(
    task: HivemindTask,
    previousAttempt: TaskDecomposition,
    failureReason: string
  ): Promise<TaskDecomposition> {
    this.logger.info(`Re-decomposing failed task: ${task.title}`);
    
    // Analyze what went wrong
    const analysisPrompt = `
A task decomposition failed. Analyze and suggest improvements:

Original Task: ${task.title}
Failure Reason: ${failureReason}
Previous Subtasks: ${previousAttempt.subtasks.map(t => t.title).join(', ')}

Suggest:
1. What went wrong with the decomposition
2. Better decomposition strategy
3. Additional subtasks or different approach
4. Risk mitigation steps

Return as JSON.`;
    
    const analysis = await this.orchestrator.executeTask({
      id: `reanalyze_${task.id}`,
      type: 'analysis',
      prompt: analysisPrompt,
      context: {},
      parameters: {
        temperature: 0.7,
        maxTokens: 1000
      }
    });
    
    // Create new decomposition with improvements
    const improvedTask = {
      ...task,
      description: `${task.description}\n\nNote: Previous attempt failed due to: ${failureReason}`,
      estimatedComplexity: Math.min(10, task.estimatedComplexity + 2)
    };
    
    return this.decomposeTask(improvedTask);
  }
}
