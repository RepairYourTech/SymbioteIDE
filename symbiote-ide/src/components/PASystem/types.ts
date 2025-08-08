// PA System TypeScript types and interfaces
export interface PAAgent {
  agent_id: string;
  status: 'active' | 'inactive' | 'busy';
  model_provider: string;
  model_name: string;
  active_plans_count: number;
  available_agents_count: number;
  websocket_clients_count: number;
  last_activity: string;
}

export interface GeneratedPlan {
  plan_id: string;
  title: string;
  description: string;
  tasks: Task[];
  agent_assignments: AgentAssignment[];
  estimated_duration?: string;
  priority: TaskPriority;
  dependencies: TaskDependency[];
  created_at: string;
  status?: 'draft' | 'in_progress' | 'completed' | 'failed' | 'cancelled';
}

export interface Task {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  priority: TaskPriority;
  created_at: string;
  updated_at: string;
  due_date?: string;
  assigned_to?: string;
  tags: string[];
  estimated_duration?: string;
  actual_duration?: string;
  dependencies: string[];
  subtasks: string[];
  metadata: Record<string, any>;
  required_capabilities: string[];
}

export interface AgentAssignment {
  task_id: string;
  agent_id: string;
  assignment_reason: string;
  estimated_duration?: string;
  priority: TaskPriority;
  status?: 'assigned' | 'in_progress' | 'completed' | 'failed';
}

export interface TaskDependency {
  from_task: string;
  to_task: string;
  dependency_type: 'blocks' | 'enables' | 'requires';
}

export enum TaskStatus {
  Todo = 'todo',
  InProgress = 'in_progress',
  Completed = 'completed',
  Blocked = 'blocked',
  Cancelled = 'cancelled'
}

export enum TaskPriority {
  Low = 'low',
  Medium = 'medium',
  High = 'high',
  Critical = 'critical'
}

export interface PlanGenerationRequest {
  title: string;
  description: string;
  priority?: TaskPriority;
  deadline?: string;
  context?: string;
  preferred_strategy?: PlanningStrategy;
}

export enum PlanningStrategy {
  Sequential = 'sequential',
  Parallel = 'parallel',
  Hybrid = 'hybrid',
  Agile = 'agile'
}

export interface PASystemEvent {
  type: 'TaskCreated' | 'TaskUpdated' | 'TaskCompleted' | 'PlanGenerated' | 'AgentAssigned' | 'AgentStatusChanged';
  timestamp: string;
  data?: any;
  task_id?: string;
  agent_id?: string;
  plan_id?: string;
  status?: string;
}

export interface PAWebSocketMessage {
  type: string;
  data?: any;
  timestamp: string;
  task_id?: string;
  agent_id?: string;
  plan_id?: string;
}

export interface AgentStatus {
  agent_id: string;
  agent_type: string;
  status: 'available' | 'busy' | 'offline';
  current_task?: string;
  capabilities: string[];
  performance_metrics: {
    tasks_completed: number;
    success_rate: number;
    average_response_time: number;
  };
  last_activity: string;
}

export interface PlanProgress {
  plan_id: string;
  total_tasks: number;
  completed_tasks: number;
  in_progress_tasks: number;
  blocked_tasks: number;
  progress_percentage: number;
  estimated_completion: string;
}

export interface TaskUpdate {
  task_id: string;
  field: string;
  old_value: any;
  new_value: any;
  updated_by: string;
  timestamp: string;
}

export interface RealtimeMetrics {
  active_connections: number;
  events_per_second: number;
  average_response_time: number;
  error_rate: number;
  last_updated: string;
}

export interface PASystemConfig {
  model_provider: string;
  model_name: string;
  max_context_tokens: number;
  planning_temperature: number;
  enable_real_time_updates: boolean;
  websocket_port: number;
}

export interface TaskAssignment {
  task_id: string;
  agent_id: string;
  assignment_reason: string;
  estimated_duration?: string;
  priority: TaskPriority;
  status?: 'assigned' | 'in_progress' | 'completed' | 'failed';
}
