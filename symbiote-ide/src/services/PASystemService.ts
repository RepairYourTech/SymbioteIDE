// src/services/PASystemService.ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';

export interface PASystemConfig {
  max_agents: number;
  task_timeout_seconds: number;
  enable_real_time_updates: boolean;
}

export interface Agent {
  id: string;
  name: string;
  role: string;
  status: 'idle' | 'working' | 'error' | 'completed';
  current_task?: string;
  capabilities: string[];
}

export interface Task {
  id: string;
  title: string;
  description: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  assigned_agent?: string;
  created_at: string;
  updated_at: string;
  priority: 'low' | 'medium' | 'high' | 'critical';
}

export interface PlanGenerationRequest {
  project_description: string;
  requirements: string[];
  constraints: string[];
  preferred_technologies: string[];
  timeline_days: number;
}

export interface GeneratedPlan {
  id: string;
  title: string;
  description: string;
  phases: PlanPhase[];
  estimated_duration_days: number;
  required_agents: string[];
  success_criteria: string[];
}

export interface PlanPhase {
  id: string;
  name: string;
  description: string;
  tasks: Task[];
  dependencies: string[];
  estimated_duration_days: number;
}

export class PASystemService {
  private static instance: PASystemService;

  public static getInstance(): PASystemService {
    if (!PASystemService.instance) {
      PASystemService.instance = new PASystemService();
    }
    return PASystemService.instance;
  }

  // Initialize PA System
  async initializePASystem(config: PASystemConfig): Promise<void> {
    try {
      await invoke('initialize_pa_system', { config });
    } catch (error) {
      console.error('Failed to initialize PA system:', error);
      throw error;
    }
  }

  // Agent Management
  async getAgents(): Promise<Agent[]> {
    try {
      return await invoke<Agent[]>('get_agents');
    } catch (error) {
      console.error('Failed to get agents:', error);
      throw error;
    }
  }

  async createAgent(name: string, role: string, capabilities: string[]): Promise<Agent> {
    try {
      return await invoke<Agent>('create_agent', { name, role, capabilities });
    } catch (error) {
      console.error('Failed to create agent:', error);
      throw error;
    }
  }

  async updateAgentStatus(agentId: string, status: Agent['status']): Promise<void> {
    try {
      await invoke('update_agent_status', { agentId, status });
    } catch (error) {
      console.error('Failed to update agent status:', error);
      throw error;
    }
  }

  // Task Management
  async getTasks(): Promise<Task[]> {
    try {
      return await invoke<Task[]>('get_tasks');
    } catch (error) {
      console.error('Failed to get tasks:', error);
      throw error;
    }
  }

  async createTask(task: Omit<Task, 'id' | 'created_at' | 'updated_at'>): Promise<Task> {
    try {
      return await invoke<Task>('create_task', { task });
    } catch (error) {
      console.error('Failed to create task:', error);
      throw error;
    }
  }

  async assignTask(taskId: string, agentId: string): Promise<void> {
    try {
      await invoke('assign_task', { taskId, agentId });
    } catch (error) {
      console.error('Failed to assign task:', error);
      throw error;
    }
  }

  async updateTaskStatus(taskId: string, status: Task['status']): Promise<void> {
    try {
      await invoke('update_task_status', { taskId, status });
    } catch (error) {
      console.error('Failed to update task status:', error);
      throw error;
    }
  }

  async reassignTask(taskId: string, agentId: string): Promise<void> {
    try {
      await invoke('pa_reassign_task', { taskId, agentId });
    } catch (error) {
      console.error('Failed to reassign task:', error);
      throw error;
    }
  }

  // Plan Generation
  async generatePlan(request: PlanGenerationRequest): Promise<GeneratedPlan> {
    try {
      return await invoke<GeneratedPlan>('pa_generate_plan', { request });
    } catch (error) {
      console.error('Failed to generate plan:', error);
      throw error;
    }
  }

  async getActivePlans(): Promise<GeneratedPlan[]> {
    try {
      return await invoke<GeneratedPlan[]>('pa_get_active_plans');
    } catch (error) {
      console.error('Failed to get active plans:', error);
      throw error;
    }
  }

  async executePlan(planId: string): Promise<void> {
    try {
      await invoke('execute_plan', { planId });
    } catch (error) {
      console.error('Failed to execute plan:', error);
      throw error;
    }
  }

  // Real-time Updates
  async subscribeToUpdates(
    onAgentUpdate: (agent: Agent) => void,
    onTaskUpdate: (task: Task) => void,
    onPlanUpdate: (plan: GeneratedPlan) => void
  ): Promise<() => void> {
    const unsubscribeFunctions: (() => void)[] = [];

    try {
      // Subscribe to agent updates
      const unsubscribeAgent = await listen<Agent>('agent-updated', (event: Event<Agent>) => {
        onAgentUpdate(event.payload);
      });
      unsubscribeFunctions.push(unsubscribeAgent);

      // Subscribe to task updates
      const unsubscribeTask = await listen<Task>('task-updated', (event: Event<Task>) => {
        onTaskUpdate(event.payload);
      });
      unsubscribeFunctions.push(unsubscribeTask);

      // Subscribe to plan updates
      const unsubscribePlan = await listen<GeneratedPlan>('plan-updated', (event: Event<GeneratedPlan>) => {
        onPlanUpdate(event.payload);
      });
      unsubscribeFunctions.push(unsubscribePlan);

      // Return cleanup function
      return () => {
        unsubscribeFunctions.forEach(unsubscribe => unsubscribe());
      };
    } catch (error) {
      console.error('Failed to subscribe to updates:', error);
      throw error;
    }
  }

  // System Status
  async getSystemStatus(): Promise<{
    active_agents: number;
    pending_tasks: number;
    completed_tasks: number;
    system_health: 'healthy' | 'warning' | 'error';
  }> {
    try {
      return await invoke('get_system_status');
    } catch (error) {
      console.error('Failed to get system status:', error);
      throw error;
    }
  }
}

export default PASystemService;
