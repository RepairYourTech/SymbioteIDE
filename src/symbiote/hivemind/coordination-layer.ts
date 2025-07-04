/**
 * Coordination Layer
 * 
 * Manages agent assignment, communication, and resource allocation
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import {
  HivemindTask,
  AgentSpecialty,
  BaseSpecialistAgent,
  AgentStatus,
  MessageType,
  CoordinationMessage
} from './types';

export interface CoordinationConfig {
  maxConcurrentAgents: number;
  messageTimeout?: number;
  loadBalancingStrategy?: 'round-robin' | 'least-loaded' | 'specialty-match';
}

export interface AgentWorkload {
  agentId: string;
  status: AgentStatus;
  currentTasks: string[];
  completedTasks: number;
  averageTaskDuration: number;
  specialtyMatchScore: number;
}

export class CoordinationLayer extends EventEmitter {
  private logger = new Logger('CoordinationLayer');
  private config: Required<CoordinationConfig>;
  
  private agentWorkloads = new Map<string, AgentWorkload>();
  private taskQueue: HivemindTask[] = [];
  private messageQueue = new Map<string, CoordinationMessage[]>();
  private agentAssignments = new Map<string, string>(); // taskId -> agentId
  
  constructor(config: CoordinationConfig) {
    super();
    
    this.config = {
      maxConcurrentAgents: config.maxConcurrentAgents,
      messageTimeout: config.messageTimeout || 30000,
      loadBalancingStrategy: config.loadBalancingStrategy || 'specialty-match'
    };
  }
  
  /**
   * Register an agent with the coordination layer
   */
  registerAgent(agent: BaseSpecialistAgent): void {
    const workload: AgentWorkload = {
      agentId: agent.id,
      status: AgentStatus.Idle,
      currentTasks: [],
      completedTasks: 0,
      averageTaskDuration: 0,
      specialtyMatchScore: 0
    };
    
    this.agentWorkloads.set(agent.id, workload);
    this.messageQueue.set(agent.id, []);
    
    this.logger.info(`Registered agent ${agent.id} (${agent.specialty})`);
    this.emit('agent-registered', { agentId: agent.id, specialty: agent.specialty });
  }
  
  /**
   * Unregister an agent
   */
  unregisterAgent(agentId: string): void {
    this.agentWorkloads.delete(agentId);
    this.messageQueue.delete(agentId);
    
    // Reassign any tasks assigned to this agent
    const tasksToReassign: string[] = [];
    this.agentAssignments.forEach((assignedAgentId, taskId) => {
      if (assignedAgentId === agentId) {
        tasksToReassign.push(taskId);
      }
    });
    
    tasksToReassign.forEach(taskId => {
      this.agentAssignments.delete(taskId);
      this.emit('task-unassigned', { taskId, agentId });
    });
    
    this.logger.info(`Unregistered agent ${agentId}`);
    this.emit('agent-unregistered', { agentId });
  }
  
  /**
   * Assign an agent to a task
   */
  async assignAgent(
    task: HivemindTask,
    availableAgents: BaseSpecialistAgent[]
  ): Promise<BaseSpecialistAgent | null> {
    // Filter agents by required specialties
    const eligibleAgents = availableAgents.filter(agent => 
      this.isAgentEligible(agent, task)
    );
    
    if (eligibleAgents.length === 0) {
      this.logger.warn(`No eligible agents found for task ${task.id}`);
      return null;
    }
    
    // Select best agent based on strategy
    const selectedAgent = await this.selectBestAgent(eligibleAgents, task);
    
    if (!selectedAgent) {
      this.logger.warn(`Could not select agent for task ${task.id}`);
      return null;
    }
    
    // Assign the agent
    this.assignAgentToTask(selectedAgent, task);
    
    return selectedAgent;
  }
  
  /**
   * Check if agent is eligible for task
   */
  private isAgentEligible(agent: BaseSpecialistAgent, task: HivemindTask): boolean {
    const workload = this.agentWorkloads.get(agent.id);
    if (!workload) return false;
    
    // Check if agent is available
    if (workload.status === AgentStatus.Busy && 
        workload.currentTasks.length >= this.config.maxConcurrentAgents) {
      return false;
    }
    
    // Check if agent has required specialty
    const hasRequiredSpecialty = task.requiredSpecialties.some(specialty =>
      agent.specialty === specialty
    );
    
    // Check if agent has required capabilities
    const hasRequiredCapabilities = this.checkCapabilities(agent, task);
    
    return hasRequiredSpecialty && hasRequiredCapabilities;
  }
  
  /**
   * Check if agent has required capabilities
   */
  private checkCapabilities(agent: BaseSpecialistAgent, task: HivemindTask): boolean {
    // Map task types to required capabilities
    const requiredCapabilities: { [key: string]: string[] } = {
      'create_component': ['component_creation', 'ui_design'],
      'create_api_endpoint': ['api_development', 'rest_api'],
      'security_audit': ['security_audit', 'vulnerability_assessment'],
      'optimize_performance': ['performance_profiling', 'optimization'],
      'create_unit_tests': ['unit_testing', 'test_automation'],
      'setup_ci_cd': ['ci_cd_pipeline', 'deployment_automation']
    };
    
    const required = requiredCapabilities[task.type] || [];
    
    return required.every(cap => agent.capabilities.includes(cap));
  }
  
  /**
   * Select best agent based on strategy
   */
  private async selectBestAgent(
    agents: BaseSpecialistAgent[],
    task: HivemindTask
  ): Promise<BaseSpecialistAgent | null> {
    switch (this.config.loadBalancingStrategy) {
      case 'round-robin':
        return this.selectRoundRobin(agents);
        
      case 'least-loaded':
        return this.selectLeastLoaded(agents);
        
      case 'specialty-match':
      default:
        return this.selectBySpecialtyMatch(agents, task);
    }
  }
  
  /**
   * Round-robin selection
   */
  private selectRoundRobin(agents: BaseSpecialistAgent[]): BaseSpecialistAgent {
    // Simple round-robin based on completed tasks
    let minCompleted = Infinity;
    let selectedAgent = agents[0];
    
    agents.forEach(agent => {
      const workload = this.agentWorkloads.get(agent.id);
      if (workload && workload.completedTasks < minCompleted) {
        minCompleted = workload.completedTasks;
        selectedAgent = agent;
      }
    });
    
    return selectedAgent;
  }
  
  /**
   * Select least loaded agent
   */
  private selectLeastLoaded(agents: BaseSpecialistAgent[]): BaseSpecialistAgent {
    let minLoad = Infinity;
    let selectedAgent = agents[0];
    
    agents.forEach(agent => {
      const workload = this.agentWorkloads.get(agent.id);
      if (workload) {
        const load = workload.currentTasks.length;
        if (load < minLoad) {
          minLoad = load;
          selectedAgent = agent;
        }
      }
    });
    
    return selectedAgent;
  }
  
  /**
   * Select by specialty match score
   */
  private async selectBySpecialtyMatch(
    agents: BaseSpecialistAgent[],
    task: HivemindTask
  ): Promise<BaseSpecialistAgent> {
    let bestScore = -1;
    let selectedAgent = agents[0];
    
    for (const agent of agents) {
      const score = await this.calculateSpecialtyMatchScore(agent, task);
      const workload = this.agentWorkloads.get(agent.id);
      
      if (workload) {
        workload.specialtyMatchScore = score;
        
        // Adjust score based on current load
        const loadPenalty = workload.currentTasks.length * 0.1;
        const adjustedScore = score - loadPenalty;
        
        if (adjustedScore > bestScore) {
          bestScore = adjustedScore;
          selectedAgent = agent;
        }
      }
    }
    
    return selectedAgent;
  }
  
  /**
   * Calculate specialty match score
   */
  private async calculateSpecialtyMatchScore(
    agent: BaseSpecialistAgent,
    task: HivemindTask
  ): Promise<number> {
    let score = 0;
    
    // Primary specialty match
    if (task.requiredSpecialties.includes(agent.specialty)) {
      score += 1.0;
    }
    
    // Capability matches
    const taskCapabilities = this.getTaskCapabilities(task);
    const matchingCapabilities = agent.capabilities.filter(cap =>
      taskCapabilities.includes(cap)
    );
    
    score += matchingCapabilities.length * 0.2;
    
    // Experience bonus (based on completed similar tasks)
    const workload = this.agentWorkloads.get(agent.id);
    if (workload && workload.completedTasks > 0) {
      score += Math.min(workload.completedTasks * 0.05, 0.5);
    }
    
    // Performance bonus (based on average task duration)
    if (workload && workload.averageTaskDuration > 0) {
      const performanceBonus = 1.0 / (workload.averageTaskDuration / 60000); // Convert to minutes
      score += Math.min(performanceBonus * 0.1, 0.3);
    }
    
    return Math.min(score, 2.0); // Cap at 2.0
  }
  
  /**
   * Get task capabilities
   */
  private getTaskCapabilities(task: HivemindTask): string[] {
    // Extract implied capabilities from task type and description
    const capabilities: string[] = [];
    
    const taskTypeMap: { [key: string]: string[] } = {
      'create_component': ['component_creation', 'ui_design', 'responsive_design'],
      'implement_auth': ['authentication', 'authorization', 'secure_coding'],
      'optimize_database': ['database_optimization', 'query_optimization', 'indexing'],
      'setup_monitoring': ['monitoring_setup', 'logging_configuration', 'alerting']
    };
    
    capabilities.push(...(taskTypeMap[task.type] || []));
    
    // Extract from description
    const keywords = task.description.toLowerCase();
    if (keywords.includes('api')) capabilities.push('api_development');
    if (keywords.includes('test')) capabilities.push('testing');
    if (keywords.includes('security')) capabilities.push('security');
    if (keywords.includes('performance')) capabilities.push('performance_optimization');
    
    return [...new Set(capabilities)];
  }
  
  /**
   * Assign agent to task
   */
  private assignAgentToTask(agent: BaseSpecialistAgent, task: HivemindTask): void {
    const workload = this.agentWorkloads.get(agent.id);
    if (!workload) return;
    
    // Update workload
    workload.status = AgentStatus.Busy;
    workload.currentTasks.push(task.id);
    
    // Record assignment
    this.agentAssignments.set(task.id, agent.id);
    task.assignedAgents.push(agent.id);
    
    this.logger.info(`Assigned agent ${agent.id} to task ${task.id}`);
    this.emit('agent-assigned', { agentId: agent.id, taskId: task.id });
  }
  
  /**
   * Release agent from task
   */
  releaseAgent(agentId: string, taskId?: string): void {
    const workload = this.agentWorkloads.get(agentId);
    if (!workload) return;
    
    if (taskId) {
      // Release from specific task
      const index = workload.currentTasks.indexOf(taskId);
      if (index > -1) {
        workload.currentTasks.splice(index, 1);
        this.agentAssignments.delete(taskId);
      }
    } else {
      // Release from all tasks
      workload.currentTasks.forEach(tid => {
        this.agentAssignments.delete(tid);
      });
      workload.currentTasks = [];
    }
    
    // Update status
    if (workload.currentTasks.length === 0) {
      workload.status = AgentStatus.Idle;
    }
    
    this.emit('agent-released', { agentId, taskId });
  }
  
  /**
   * Update agent performance metrics
   */
  updateAgentMetrics(
    agentId: string,
    taskId: string,
    duration: number,
    success: boolean
  ): void {
    const workload = this.agentWorkloads.get(agentId);
    if (!workload) return;
    
    if (success) {
      workload.completedTasks++;
      
      // Update average duration
      const totalDuration = workload.averageTaskDuration * (workload.completedTasks - 1);
      workload.averageTaskDuration = (totalDuration + duration) / workload.completedTasks;
    }
    
    this.emit('agent-metrics-updated', {
      agentId,
      taskId,
      duration,
      success,
      metrics: {
        completedTasks: workload.completedTasks,
        averageTaskDuration: workload.averageTaskDuration
      }
    });
  }
  
  /**
   * Send message between agents
   */
  async sendMessage(
    fromAgentId: string,
    toAgentId: string,
    message: CoordinationMessage
  ): Promise<void> {
    const queue = this.messageQueue.get(toAgentId);
    if (!queue) {
      throw new Error(`Agent ${toAgentId} not found`);
    }
    
    message.timestamp = new Date();
    queue.push(message);
    
    this.emit('message-sent', {
      from: fromAgentId,
      to: toAgentId,
      type: message.type,
      timestamp: message.timestamp
    });
    
    // Set timeout for message expiry
    setTimeout(() => {
      const idx = queue.indexOf(message);
      if (idx > -1) {
        queue.splice(idx, 1);
        this.emit('message-expired', {
          from: fromAgentId,
          to: toAgentId,
          type: message.type
        });
      }
    }, this.config.messageTimeout);
  }
  
  /**
   * Receive messages for agent
   */
  receiveMessages(agentId: string): CoordinationMessage[] {
    const queue = this.messageQueue.get(agentId);
    if (!queue) return [];
    
    const messages = [...queue];
    queue.length = 0; // Clear queue
    
    return messages;
  }
  
  /**
   * Broadcast message to all agents
   */
  async broadcastMessage(
    fromAgentId: string,
    message: CoordinationMessage
  ): Promise<void> {
    const promises: Promise<void>[] = [];
    
    this.agentWorkloads.forEach((_, agentId) => {
      if (agentId !== fromAgentId) {
        promises.push(this.sendMessage(fromAgentId, agentId, { ...message }));
      }
    });
    
    await Promise.all(promises);
    
    this.emit('message-broadcast', {
      from: fromAgentId,
      type: message.type,
      recipientCount: promises.length
    });
  }
  
  /**
   * Get agent workload information
   */
  getAgentWorkload(agentId: string): AgentWorkload | null {
    return this.agentWorkloads.get(agentId) || null;
  }
  
  /**
   * Get all agent workloads
   */
  getAllWorkloads(): Map<string, AgentWorkload> {
    return new Map(this.agentWorkloads);
  }
  
  /**
   * Get task assignment
   */
  getTaskAssignment(taskId: string): string | null {
    return this.agentAssignments.get(taskId) || null;
  }
  
  /**
   * Queue task for assignment
   */
  queueTask(task: HivemindTask): void {
    this.taskQueue.push(task);
    this.emit('task-queued', { taskId: task.id });
    
    // Try to assign immediately if agents available
    this.processTaskQueue();
  }
  
  /**
   * Process queued tasks
   */
  private async processTaskQueue(): Promise<void> {
    if (this.taskQueue.length === 0) return;
    
    // Find idle agents
    const idleAgents: string[] = [];
    this.agentWorkloads.forEach((workload, agentId) => {
      if (workload.status === AgentStatus.Idle) {
        idleAgents.push(agentId);
      }
    });
    
    if (idleAgents.length === 0) return;
    
    // Process tasks
    const tasksToProcess = Math.min(this.taskQueue.length, idleAgents.length);
    
    for (let i = 0; i < tasksToProcess; i++) {
      const task = this.taskQueue.shift();
      if (!task) break;
      
      this.emit('task-dequeued', { taskId: task.id });
      
      // Task assignment would be handled by the controller
      // This is just queue management
    }
  }
  
  /**
   * Get coordination statistics
   */
  getStatistics(): any {
    const stats = {
      totalAgents: this.agentWorkloads.size,
      activeAgents: 0,
      idleAgents: 0,
      totalTasks: 0,
      queuedTasks: this.taskQueue.length,
      averageTasksPerAgent: 0,
      agentUtilization: {} as { [key: string]: number }
    };
    
    this.agentWorkloads.forEach((workload, agentId) => {
      if (workload.status === AgentStatus.Busy) {
        stats.activeAgents++;
      } else {
        stats.idleAgents++;
      }
      
      stats.totalTasks += workload.currentTasks.length;
      
      // Calculate utilization (current tasks / max concurrent)
      const utilization = workload.currentTasks.length / this.config.maxConcurrentAgents;
      stats.agentUtilization[agentId] = Math.min(utilization * 100, 100);
    });
    
    if (stats.totalAgents > 0) {
      stats.averageTasksPerAgent = stats.totalTasks / stats.totalAgents;
    }
    
    return stats;
  }
  
  /**
   * Reset coordination layer
   */
  reset(): void {
    this.agentWorkloads.clear();
    this.taskQueue = [];
    this.messageQueue.clear();
    this.agentAssignments.clear();
    
    this.emit('coordination-reset');
  }
}