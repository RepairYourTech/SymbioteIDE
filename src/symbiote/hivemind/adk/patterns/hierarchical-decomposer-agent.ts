/**
 * Hierarchical Task Decomposer Agent Pattern
 * 
 * Breaks down complex tasks into hierarchical subtasks
 */

import { LLMAgent, Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty, TaskPriority, TaskStatus } from '../../types';
import { Logger } from '../../../utils/logger';

export interface TaskNode {
  task: HivemindTask;
  children: TaskNode[];
  parent?: TaskNode;
  level: number;
  executionOrder: number;
}

export interface DecompositionStrategy {
  maxDepth: number;
  minTaskComplexity: number;
  decompositionThreshold: number;
  parallelizationRules?: (task: HivemindTask) => boolean;
}

export interface HierarchicalConfig extends ADKHivemindConfig {
  decompositionStrategy: DecompositionStrategy;
  subAgents: Map<string, ADKHivemindAgent>;
  autoAssignAgents?: boolean;
}

export class HierarchicalDecomposerAgent extends ADKHivemindAgent {
  private decompositionStrategy: DecompositionStrategy;
  private subAgents: Map<string, ADKHivemindAgent>;
  private autoAssignAgents: boolean;
  private taskTree: TaskNode | null = null;
  private executionHistory: any[] = [];
  
  constructor(config: HierarchicalConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are a hierarchical task decomposer that breaks down complex tasks into manageable subtasks.
      
Your responsibilities:
1. Analyze task complexity and dependencies
2. Decompose tasks into logical subtasks
3. Create hierarchical task structures
4. Identify parallelization opportunities
5. Assign appropriate agents to subtasks
6. Coordinate execution across levels

Decomposition guidelines:
- Maximum depth: ${config.decompositionStrategy.maxDepth}
- Minimum task complexity: ${config.decompositionStrategy.minTaskComplexity}
- Decomposition threshold: ${config.decompositionStrategy.decompositionThreshold}

Available sub-agents:
${Array.from(config.subAgents.entries()).map(([id, agent]) => 
  `- ${id}: ${agent.name} (${agent.specialty})`
).join('\n')}`
    });
    
    this.decompositionStrategy = config.decompositionStrategy;
    this.subAgents = config.subAgents;
    this.autoAssignAgents = config.autoAssignAgents ?? true;
    
    // Add hierarchical tools
    this.addHierarchicalTools();
  }
  
  /**
   * Add hierarchical-specific tools
   */
  private addHierarchicalTools(): void {
    this.tools.push(new Tool({
      name: 'decompose_task',
      description: 'Decompose a task into subtasks',
      parameters: {
        task: { type: 'object', required: true },
        level: { type: 'number', required: false }
      },
      handler: async (params: any) => {
        return this.decomposeTask(params.task, params.level || 0);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'estimate_complexity',
      description: 'Estimate task complexity',
      parameters: {
        task: { type: 'object', required: true }
      },
      handler: async (params: any) => {
        return this.estimateComplexity(params.task);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'assign_agent',
      description: 'Assign an agent to a task',
      parameters: {
        taskId: { type: 'string', required: true },
        agentId: { type: 'string', required: true }
      },
      handler: async (params: any) => {
        return this.assignAgent(params.taskId, params.agentId);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'visualize_hierarchy',
      description: 'Generate visualization of task hierarchy',
      parameters: {},
      handler: async () => {
        return this.visualizeHierarchy();
      }
    }));
  }
  
  /**
   * Execute hierarchical decomposition
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    const hierarchyId = `hierarchy_${task.id}`;
    
    try {
      // Initialize hierarchy state
      const hierarchyState = {
        rootTaskId: task.id,
        startTime: new Date(),
        levels: 0,
        totalTasks: 0,
        completedTasks: 0,
        status: 'decomposing'
      };
      
      await this.writeState(hierarchyId, hierarchyState);
      
      // Decompose the root task
      this.logger.info(`Starting hierarchical decomposition of task ${task.id}`);
      this.taskTree = await this.buildTaskTree(task, 0);
      
      // Count total tasks and levels
      const stats = this.calculateTreeStats(this.taskTree);
      hierarchyState.levels = stats.maxLevel;
      hierarchyState.totalTasks = stats.totalTasks;
      
      // Assign agents if auto-assignment is enabled
      if (this.autoAssignAgents) {
        await this.autoAssignAgentsToTree(this.taskTree);
      }
      
      // Update state
      hierarchyState.status = 'executing';
      await this.writeState(hierarchyId, hierarchyState);
      
      // Execute the task tree
      const result = await this.executeTaskTree(this.taskTree, hierarchyState, hierarchyId);
      
      // Update final state
      hierarchyState.status = 'completed';
      await this.writeState(hierarchyId, hierarchyState);
      
      // Store execution history
      this.executionHistory.push({
        taskId: task.id,
        hierarchyId,
        levels: stats.maxLevel,
        totalTasks: stats.totalTasks,
        duration: Date.now() - startTime,
        success: result.success
      });
      
      return result;
      
    } catch (error) {
      this.logger.error(`Hierarchical execution failed`, error);
      
      return {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Hierarchical execution error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
    }
  }
  
  /**
   * Build task tree recursively
   */
  private async buildTaskTree(
    task: HivemindTask,
    level: number,
    parent?: TaskNode
  ): Promise<TaskNode> {
    const node: TaskNode = {
      task,
      children: [],
      parent,
      level,
      executionOrder: 0
    };
    
    // Check if decomposition is needed
    if (level < this.decompositionStrategy.maxDepth &&
        task.estimatedComplexity >= this.decompositionStrategy.decompositionThreshold) {
      
      // Decompose into subtasks
      const subtasks = await this.decomposeTask(task, level);
      
      // Build child nodes
      for (const subtask of subtasks) {
        const childNode = await this.buildTaskTree(subtask, level + 1, node);
        node.children.push(childNode);
      }
    }
    
    return node;
  }
  
  /**
   * Decompose task using LLM
   */
  private async decomposeTask(
    task: HivemindTask,
    level: number
  ): Promise<HivemindTask[]> {
    const prompt = `Decompose this task into subtasks:

Task: ${task.title}
Description: ${task.description}
Complexity: ${task.estimatedComplexity}/10
Current Level: ${level}

Consider:
1. Logical breakdown of work
2. Dependencies between subtasks
3. Potential for parallel execution
4. Appropriate granularity (not too fine, not too coarse)

Return a JSON array of subtasks with this structure:
[
  {
    "title": "Subtask title",
    "description": "Detailed description",
    "type": "task type",
    "estimatedComplexity": 1-10,
    "requiredSpecialties": ["specialty1", "specialty2"],
    "canRunInParallel": true/false,
    "dependencies": [] // indices of other subtasks in this array
  }
]`;

    const result = await this.run(prompt);
    
    try {
      const subtaskData = JSON.parse(result.output);
      const subtasks: HivemindTask[] = [];
      
      for (let i = 0; i < subtaskData.length; i++) {
        const data = subtaskData[i];
        const subtask: HivemindTask = {
          id: `${task.id}.${i + 1}`,
          type: data.type || task.type,
          title: data.title,
          description: data.description,
          requiredSpecialties: data.requiredSpecialties || [AgentSpecialty.General],
          priority: task.priority,
          status: TaskStatus.Pending,
          dependencies: data.dependencies?.map((idx: number) => `${task.id}.${idx + 1}`) || [],
          assignedAgents: [],
          estimatedComplexity: data.estimatedComplexity || 5,
          context: {
            ...task.context,
            parentTaskId: task.id,
            canRunInParallel: data.canRunInParallel || false,
            level: level + 1
          },
          createdAt: new Date()
        };
        
        subtasks.push(subtask);
      }
      
      return subtasks;
      
    } catch (error) {
      this.logger.error('Failed to parse decomposition', error);
      return [];
    }
  }
  
  /**
   * Calculate tree statistics
   */
  private calculateTreeStats(node: TaskNode): { maxLevel: number; totalTasks: number } {
    let maxLevel = node.level;
    let totalTasks = 1;
    
    for (const child of node.children) {
      const childStats = this.calculateTreeStats(child);
      maxLevel = Math.max(maxLevel, childStats.maxLevel);
      totalTasks += childStats.totalTasks;
    }
    
    return { maxLevel, totalTasks };
  }
  
  /**
   * Auto-assign agents to tree
   */
  private async autoAssignAgentsToTree(node: TaskNode): Promise<void> {
    // Find best agent for this task
    const bestAgent = this.findBestAgent(node.task);
    if (bestAgent) {
      node.task.assignedAgents = [bestAgent];
    }
    
    // Recursively assign to children
    for (const child of node.children) {
      await this.autoAssignAgentsToTree(child);
    }
  }
  
  /**
   * Find best agent for task
   */
  private findBestAgent(task: HivemindTask): string | null {
    let bestMatch = { agentId: '', score: 0 };
    
    for (const [agentId, agent] of this.subAgents) {
      let score = 0;
      
      // Check specialty match
      if (task.requiredSpecialties.includes(agent.specialty)) {
        score += 50;
      }
      
      // Check capability match
      const taskKeywords = `${task.type} ${task.title} ${task.description}`.toLowerCase();
      for (const capability of agent.capabilities) {
        if (taskKeywords.includes(capability.toLowerCase())) {
          score += 10;
        }
      }
      
      if (score > bestMatch.score) {
        bestMatch = { agentId, score };
      }
    }
    
    return bestMatch.score > 0 ? bestMatch.agentId : null;
  }
  
  /**
   * Execute task tree
   */
  private async executeTaskTree(
    node: TaskNode,
    hierarchyState: any,
    hierarchyId: string
  ): Promise<TaskResult> {
    // If leaf node, execute directly
    if (node.children.length === 0) {
      return this.executeLeafTask(node, hierarchyState, hierarchyId);
    }
    
    // Execute children first
    const childResults: TaskResult[] = [];
    
    // Group children by parallelization
    const parallelGroups = this.groupByParallelization(node.children);
    
    for (const group of parallelGroups) {
      if (group.length === 1) {
        // Execute single task
        const result = await this.executeTaskTree(group[0], hierarchyState, hierarchyId);
        childResults.push(result);
      } else {
        // Execute parallel tasks
        const promises = group.map(child => 
          this.executeTaskTree(child, hierarchyState, hierarchyId)
        );
        const results = await Promise.all(promises);
        childResults.push(...results);
      }
    }
    
    // Aggregate child results and execute parent
    return this.executeParentTask(node, childResults, hierarchyState, hierarchyId);
  }
  
  /**
   * Execute leaf task
   */
  private async executeLeafTask(
    node: TaskNode,
    hierarchyState: any,
    hierarchyId: string
  ): Promise<TaskResult> {
    const agentId = node.task.assignedAgents[0];
    const agent = this.subAgents.get(agentId);
    
    if (!agent) {
      return {
        taskId: node.task.id,
        success: false,
        output: { error: 'No agent assigned' },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: ['No agent available for task'],
        performanceMetrics: { duration: 0, tokensUsed: 0, cost: 0 }
      };
    }
    
    // Execute with assigned agent
    const result = await agent.executeTask(node.task);
    
    // Update hierarchy state
    hierarchyState.completedTasks++;
    await this.writeState(hierarchyId, hierarchyState);
    
    return result;
  }
  
  /**
   * Execute parent task after children
   */
  private async executeParentTask(
    node: TaskNode,
    childResults: TaskResult[],
    hierarchyState: any,
    hierarchyId: string
  ): Promise<TaskResult> {
    // Create aggregated task context
    const aggregatedTask: HivemindTask = {
      ...node.task,
      context: {
        ...node.task.context,
        childResults: childResults.map(r => ({
          taskId: r.taskId,
          success: r.success,
          output: r.output
        }))
      }
    };
    
    // Execute parent task with context from children
    const agentId = node.task.assignedAgents[0];
    const agent = this.subAgents.get(agentId);
    
    if (agent) {
      return agent.executeTask(aggregatedTask);
    }
    
    // Fallback: aggregate child results
    return this.aggregateResults(node.task, childResults);
  }
  
  /**
   * Group tasks by parallelization capability
   */
  private groupByParallelization(nodes: TaskNode[]): TaskNode[][] {
    const groups: TaskNode[][] = [];
    const processed = new Set<string>();
    
    for (const node of nodes) {
      if (processed.has(node.task.id)) continue;
      
      const group: TaskNode[] = [node];
      processed.add(node.task.id);
      
      // Find other tasks that can run in parallel
      if (node.task.context.canRunInParallel) {
        for (const other of nodes) {
          if (!processed.has(other.task.id) &&
              other.task.context.canRunInParallel &&
              !this.hasDependency(other.task, node.task)) {
            group.push(other);
            processed.add(other.task.id);
          }
        }
      }
      
      groups.push(group);
    }
    
    return groups;
  }
  
  /**
   * Check if task A depends on task B
   */
  private hasDependency(taskA: HivemindTask, taskB: HivemindTask): boolean {
    return taskA.dependencies.includes(taskB.id);
  }
  
  /**
   * Aggregate results from child tasks
   */
  private aggregateResults(
    task: HivemindTask,
    childResults: TaskResult[]
  ): TaskResult {
    const aggregated: TaskResult = {
      taskId: task.id,
      success: childResults.every(r => r.success),
      output: {
        summary: `Completed ${task.title} with ${childResults.length} subtasks`,
        childResults: childResults.map(r => ({
          taskId: r.taskId,
          success: r.success,
          summary: r.output
        }))
      },
      filesModified: [],
      filesCreated: [],
      testsAdded: 0,
      issuesFound: [],
      performanceMetrics: {
        duration: 0,
        tokensUsed: 0,
        cost: 0
      }
    };
    
    // Merge metrics from children
    for (const result of childResults) {
      aggregated.filesModified.push(...result.filesModified);
      aggregated.filesCreated.push(...result.filesCreated);
      aggregated.testsAdded += result.testsAdded;
      aggregated.issuesFound.push(...result.issuesFound);
      aggregated.performanceMetrics.duration += result.performanceMetrics.duration;
      aggregated.performanceMetrics.tokensUsed += result.performanceMetrics.tokensUsed;
      aggregated.performanceMetrics.cost += result.performanceMetrics.cost;
    }
    
    // Remove duplicates
    aggregated.filesModified = [...new Set(aggregated.filesModified)];
    aggregated.filesCreated = [...new Set(aggregated.filesCreated)];
    
    return aggregated;
  }
  
  /**
   * Estimate task complexity
   */
  private async estimateComplexity(task: HivemindTask): Promise<number> {
    // Simple heuristic-based estimation
    let complexity = 5; // Base complexity
    
    // Adjust based on task type
    const complexTypes = ['architecture', 'refactor', 'optimization', 'integration'];
    if (complexTypes.includes(task.type)) {
      complexity += 2;
    }
    
    // Adjust based on description length
    if (task.description.length > 500) {
      complexity += 1;
    }
    
    // Adjust based on file count
    if (task.context.files && task.context.files.length > 5) {
      complexity += 1;
    }
    
    // Adjust based on required specialties
    if (task.requiredSpecialties.length > 2) {
      complexity += 1;
    }
    
    return Math.min(10, Math.max(1, complexity));
  }
  
  /**
   * Assign agent to task
   */
  private async assignAgent(taskId: string, agentId: string): Promise<boolean> {
    // Find task in tree
    const taskNode = this.findTaskNode(this.taskTree, taskId);
    if (taskNode && this.subAgents.has(agentId)) {
      taskNode.task.assignedAgents = [agentId];
      return true;
    }
    return false;
  }
  
  /**
   * Find task node in tree
   */
  private findTaskNode(node: TaskNode | null, taskId: string): TaskNode | null {
    if (!node) return null;
    if (node.task.id === taskId) return node;
    
    for (const child of node.children) {
      const found = this.findTaskNode(child, taskId);
      if (found) return found;
    }
    
    return null;
  }
  
  /**
   * Visualize task hierarchy
   */
  private async visualizeHierarchy(): Promise<string> {
    if (!this.taskTree) return 'No task hierarchy available';
    
    const lines: string[] = [];
    this.buildVisualization(this.taskTree, lines, '', true);
    return lines.join('\n');
  }
  
  /**
   * Build visualization recursively
   */
  private buildVisualization(
    node: TaskNode,
    lines: string[],
    prefix: string,
    isLast: boolean
  ): void {
    const connector = isLast ? '└── ' : '├── ';
    const agentInfo = node.task.assignedAgents.length > 0 
      ? ` [${node.task.assignedAgents[0]}]` 
      : '';
    
    lines.push(`${prefix}${connector}${node.task.title}${agentInfo}`);
    
    const extension = isLast ? '    ' : '│   ';
    const newPrefix = prefix + extension;
    
    for (let i = 0; i < node.children.length; i++) {
      this.buildVisualization(
        node.children[i],
        lines,
        newPrefix,
        i === node.children.length - 1
      );
    }
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    return `Analyze and decompose this complex task hierarchically:

Task: ${task.title}
Description: ${task.description}
Estimated Complexity: ${task.estimatedComplexity}/10

Break this down into manageable subtasks that can be executed by specialized agents.
Consider dependencies and opportunities for parallel execution.`;
  }
}