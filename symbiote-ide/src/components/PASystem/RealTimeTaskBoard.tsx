// Real-Time Task Board - Live task visualization with agent status
import React, { useState, useEffect } from 'react';
import { GeneratedPlan, Task, AgentAssignment, TaskStatus } from './types';
import { PAWebSocketClient } from './PAWebSocketClient';
import './RealTimeTaskBoard.css';

interface RealTimeTaskBoardProps {
  plans: GeneratedPlan[];
  wsClient: PAWebSocketClient | null;
}

export const RealTimeTaskBoard: React.FC<RealTimeTaskBoardProps> = ({ 
  plans, 
  wsClient 
}) => {
  const [selectedPlan, setSelectedPlan] = useState<GeneratedPlan | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [agentAssignments, setAgentAssignments] = useState<AgentAssignment[]>([]);
  const [viewMode, setViewMode] = useState<'kanban' | 'timeline' | 'agents'>('kanban');
  const [filter, setFilter] = useState<'all' | TaskStatus>(TaskStatus.Todo);

  useEffect(() => {
    if (plans.length > 0 && !selectedPlan) {
      setSelectedPlan(plans[0]);
    }
  }, [plans]);

  useEffect(() => {
    if (selectedPlan) {
      setTasks(selectedPlan.tasks);
      setAgentAssignments(selectedPlan.agent_assignments);
    }
  }, [selectedPlan]);

  const filteredTasks = tasks.filter(task => 
    filter === 'all' || task.status === filter
  );

  const tasksByStatus = {
    [TaskStatus.Todo]: filteredTasks.filter(t => t.status === TaskStatus.Todo),
    [TaskStatus.InProgress]: filteredTasks.filter(t => t.status === TaskStatus.InProgress),
    [TaskStatus.Completed]: filteredTasks.filter(t => t.status === TaskStatus.Completed),
    [TaskStatus.Blocked]: filteredTasks.filter(t => t.status === TaskStatus.Blocked),
    [TaskStatus.Cancelled]: filteredTasks.filter(t => t.status === TaskStatus.Cancelled),
  };

  const handleTaskUpdate = (taskId: string, updates: Partial<Task>) => {
    setTasks(prev => prev.map(task => 
      task.id === taskId ? { ...task, ...updates } : task
    ));

    // Send update via WebSocket
    if (wsClient) {
      wsClient.updateTask(taskId, updates);
    }
  };

  return (
    <div className="real-time-task-board">
      {/* Header */}
      <div className="task-board-header">
        <div className="plan-selector">
          <h2>📋 Live Task Board</h2>
          <select 
            value={selectedPlan?.plan_id || ''}
            onChange={(e) => {
              const plan = plans.find(p => p.plan_id === e.target.value);
              setSelectedPlan(plan || null);
            }}
          >
            {plans.map(plan => (
              <option key={plan.plan_id} value={plan.plan_id}>
                {plan.title}
              </option>
            ))}
          </select>
        </div>

        <div className="board-controls">
          {/* View Mode Toggle */}
          <div className="view-mode-toggle">
            <button 
              className={viewMode === 'kanban' ? 'active' : ''}
              onClick={() => setViewMode('kanban')}
            >
              📋 Kanban
            </button>
            <button 
              className={viewMode === 'timeline' ? 'active' : ''}
              onClick={() => setViewMode('timeline')}
            >
              📅 Timeline
            </button>
            <button 
              className={viewMode === 'agents' ? 'active' : ''}
              onClick={() => setViewMode('agents')}
            >
              🤖 Agents
            </button>
          </div>

          {/* Filter */}
          <div className="task-filter">
            <select 
              value={filter}
              onChange={(e) => setFilter(e.target.value as any)}
            >
              <option value="all">All Tasks</option>
              <option value={TaskStatus.Todo}>To Do</option>
              <option value={TaskStatus.InProgress}>In Progress</option>
              <option value={TaskStatus.Completed}>Completed</option>
              <option value={TaskStatus.Blocked}>Blocked</option>
            </select>
          </div>
        </div>
      </div>

      {/* Task Board Content */}
      <div className="task-board-content">
        {viewMode === 'kanban' && (
          <KanbanView 
            tasksByStatus={tasksByStatus}
            agentAssignments={agentAssignments}
            onTaskUpdate={handleTaskUpdate}
          />
        )}

        {viewMode === 'timeline' && (
          <TimelineView 
            tasks={filteredTasks}
            agentAssignments={agentAssignments}
            onTaskUpdate={handleTaskUpdate}
          />
        )}

        {viewMode === 'agents' && (
          <AgentView 
            tasks={filteredTasks}
            agentAssignments={agentAssignments}
            onTaskUpdate={handleTaskUpdate}
          />
        )}
      </div>
    </div>
  );
};

// Kanban Board View
const KanbanView: React.FC<{
  tasksByStatus: Record<TaskStatus, Task[]>;
  agentAssignments: AgentAssignment[];
  onTaskUpdate: (taskId: string, updates: Partial<Task>) => void;
}> = ({ tasksByStatus, agentAssignments, onTaskUpdate }) => {
  const getAgentForTask = (taskId: string) => {
    return agentAssignments.find(a => a.task_id === taskId);
  };

  const handleDragStart = (e: React.DragEvent, task: Task) => {
    e.dataTransfer.setData('text/plain', task.id);
  };

  const handleDrop = (e: React.DragEvent, newStatus: TaskStatus) => {
    e.preventDefault();
    const taskId = e.dataTransfer.getData('text/plain');
    onTaskUpdate(taskId, { status: newStatus });
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  return (
    <div className="kanban-view">
      {Object.entries(tasksByStatus).map(([status, tasks]) => (
        <div 
          key={status}
          className="kanban-column"
          onDrop={(e) => handleDrop(e, status as TaskStatus)}
          onDragOver={handleDragOver}
        >
          <div className="column-header">
            <h3>{getStatusLabel(status as TaskStatus)}</h3>
            <span className="task-count">{tasks.length}</span>
          </div>

          <div className="task-list">
            {tasks.map(task => (
              <TaskCard
                key={task.id}
                task={task}
                agent={getAgentForTask(task.id)}
                onUpdate={onTaskUpdate}
                onDragStart={handleDragStart}
                draggable
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};

// Timeline View
const TimelineView: React.FC<{
  tasks: Task[];
  agentAssignments: AgentAssignment[];
  onTaskUpdate: (taskId: string, updates: Partial<Task>) => void;
}> = ({ tasks, agentAssignments, onTaskUpdate }) => {
  const sortedTasks = [...tasks].sort((a, b) => 
    new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
  );

  return (
    <div className="timeline-view">
      <div className="timeline-header">
        <h3>📅 Task Timeline</h3>
      </div>
      
      <div className="timeline-content">
        {sortedTasks.map((task, index) => (
          <div key={task.id} className="timeline-item">
            <div className="timeline-marker">
              <div className={`marker-dot ${task.status}`} />
              {index < sortedTasks.length - 1 && <div className="timeline-line" />}
            </div>
            
            <div className="timeline-task">
              <TaskCard
                task={task}
                agent={agentAssignments.find(a => a.task_id === task.id)}
                onUpdate={onTaskUpdate}
                compact
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

// Agent View
const AgentView: React.FC<{
  tasks: Task[];
  agentAssignments: AgentAssignment[];
  onTaskUpdate: (taskId: string, updates: Partial<Task>) => void;
}> = ({ tasks, agentAssignments, onTaskUpdate }) => {
  const tasksByAgent = agentAssignments.reduce((acc, assignment) => {
    if (!acc[assignment.agent_id]) {
      acc[assignment.agent_id] = [];
    }
    const task = tasks.find(t => t.id === assignment.task_id);
    if (task) {
      acc[assignment.agent_id].push({ task, assignment });
    }
    return acc;
  }, {} as Record<string, Array<{ task: Task; assignment: AgentAssignment }>>);

  return (
    <div className="agent-view">
      <div className="agent-columns">
        {Object.entries(tasksByAgent).map(([agentId, taskAssignments]) => (
          <div key={agentId} className="agent-column">
            <div className="agent-header">
              <div className="agent-info">
                <div className="agent-avatar">🤖</div>
                <div>
                  <h3>{agentId}</h3>
                  <span className="task-count">{taskAssignments.length} tasks</span>
                </div>
              </div>
              <div className="agent-status active">Active</div>
            </div>

            <div className="agent-tasks">
              {taskAssignments.map(({ task, assignment }) => (
                <TaskCard
                  key={task.id}
                  task={task}
                  agent={assignment}
                  onUpdate={onTaskUpdate}
                  showAgent={false}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

// Task Card Component
const TaskCard: React.FC<{
  task: Task;
  agent?: AgentAssignment;
  onUpdate: (taskId: string, updates: Partial<Task>) => void;
  onDragStart?: (e: React.DragEvent, task: Task) => void;
  draggable?: boolean;
  compact?: boolean;
  showAgent?: boolean;
}> = ({ 
  task, 
  agent, 
  onUpdate, 
  onDragStart, 
  draggable = false, 
  compact = false,
  showAgent = true 
}) => {
  const [isEditing, setIsEditing] = useState(false);

  return (
    <div 
      className={`task-card ${compact ? 'compact' : ''} ${task.status}`}
      draggable={draggable}
      onDragStart={onDragStart ? (e) => onDragStart(e, task) : undefined}
    >
      {/* Task Header */}
      <div className="task-header">
        <div className="task-priority">
          <div className={`priority-indicator ${task.priority}`} />
        </div>
        <div className="task-actions">
          <button 
            className="edit-btn"
            onClick={() => setIsEditing(!isEditing)}
          >
            ✏️
          </button>
        </div>
      </div>

      {/* Task Content */}
      <div className="task-content">
        <h4 className="task-title">{task.title}</h4>
        {!compact && (
          <p className="task-description">{task.description}</p>
        )}

        {/* Task Meta */}
        <div className="task-meta">
          <div className="task-tags">
            {task.tags.slice(0, 3).map(tag => (
              <span key={tag} className="task-tag">{tag}</span>
            ))}
          </div>

          {task.estimated_duration && (
            <div className="task-duration">
              ⏱️ {task.estimated_duration}
            </div>
          )}
        </div>

        {/* Agent Assignment */}
        {showAgent && agent && (
          <div className="task-agent">
            <div className="agent-avatar">🤖</div>
            <span className="agent-name">{agent.agent_id}</span>
            <div className={`agent-status ${agent.status || 'assigned'}`} />
          </div>
        )}

        {/* Dependencies */}
        {task.dependencies.length > 0 && (
          <div className="task-dependencies">
            <span className="dependencies-label">Depends on:</span>
            <div className="dependency-list">
              {task.dependencies.slice(0, 2).map(dep => (
                <span key={dep} className="dependency-item">{dep}</span>
              ))}
              {task.dependencies.length > 2 && (
                <span className="dependency-more">+{task.dependencies.length - 2}</span>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Quick Actions */}
      <div className="task-quick-actions">
        <button 
          className="status-btn"
          onClick={() => {
            const nextStatus = getNextStatus(task.status);
            onUpdate(task.id, { status: nextStatus });
          }}
        >
          {getStatusAction(task.status)}
        </button>
      </div>
    </div>
  );
};

// Helper functions
const getStatusLabel = (status: TaskStatus): string => {
  switch (status) {
    case TaskStatus.Todo: return '📝 To Do';
    case TaskStatus.InProgress: return '🔄 In Progress';
    case TaskStatus.Completed: return '✅ Completed';
    case TaskStatus.Blocked: return '🚫 Blocked';
    default: return status;
  }
};

const getNextStatus = (currentStatus: TaskStatus): TaskStatus => {
  switch (currentStatus) {
    case TaskStatus.Todo: return TaskStatus.InProgress;
    case TaskStatus.InProgress: return TaskStatus.Completed;
    case TaskStatus.Completed: return TaskStatus.Todo;
    case TaskStatus.Blocked: return TaskStatus.Todo;
    default: return TaskStatus.Todo;
  }
};

const getStatusAction = (status: TaskStatus): string => {
  switch (status) {
    case TaskStatus.Todo: return 'Start';
    case TaskStatus.InProgress: return 'Complete';
    case TaskStatus.Completed: return 'Reopen';
    case TaskStatus.Blocked: return 'Unblock';
    default: return 'Update';
  }
};
