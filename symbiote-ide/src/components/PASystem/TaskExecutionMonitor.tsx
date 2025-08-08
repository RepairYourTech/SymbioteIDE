// Task Execution Monitor - Real-time task and agent progress tracking
import React, { useState, useEffect } from 'react';
import { PASystemService } from '../../services/PASystemService';
import { GeneratedPlan, Task, AgentAssignment, TaskStatus } from './types';
import './TaskExecutionMonitor.css';

interface TaskExecutionMonitorProps {
  activePlans: GeneratedPlan[];
  onTaskUpdate?: (taskId: string, status: TaskStatus) => void;
  onAgentReassign?: (taskId: string, agentId: string) => void;
}

export const TaskExecutionMonitor: React.FC<TaskExecutionMonitorProps> = ({
  activePlans,
  onTaskUpdate,
  onAgentReassign
}) => {
  const [selectedPlan, setSelectedPlan] = useState<string | null>(null);
  const [taskProgress, setTaskProgress] = useState<Record<string, number>>({});
  const [agentStatuses, setAgentStatuses] = useState<Record<string, string>>({});
  
  const paService = PASystemService.getInstance();

  useEffect(() => {
    // Subscribe to real-time updates
    const setupUpdates = async () => {
      try {
        const unsubscribe = await paService.subscribeToUpdates(
          (agent: any) => {
            setAgentStatuses(prev => ({
              ...prev,
              [agent.agent_id || agent.id]: agent.status
            }));
          },
          (task: any) => {
            setTaskProgress(prev => ({
              ...prev,
              [task.id]: task.progress || 0
            }));
          },
          (plan: any) => {
            console.log('Plan updated:', plan);
          }
        );
        return unsubscribe;
      } catch (error) {
        console.error('Failed to setup updates:', error);
        return () => {};
      }
    };

    let cleanup: (() => void) | undefined;
    setupUpdates().then(fn => cleanup = fn);

    return () => cleanup?.();
  }, []);

  const getTaskStatusIcon = (status: TaskStatus) => {
    switch (status) {
      case TaskStatus.Todo: return '⏳';
      case TaskStatus.InProgress: return '🔄';
      case TaskStatus.Completed: return '✅';
      case TaskStatus.Cancelled: return '❌';
      case TaskStatus.Blocked: return '🚫';
      default: return '📋';
    }
  };

  const getAgentStatusColor = (status: string) => {
    switch (status) {
      case 'active': return '#4ade80';
      case 'busy': return '#f59e0b';
      case 'inactive': return '#6b7280';
      default: return '#8b5cf6';
    }
  };

  const handleTaskStatusChange = async (taskId: string, newStatus: TaskStatus) => {
    try {
      await paService.updateTaskStatus(taskId, newStatus as any);
      onTaskUpdate?.(taskId, newStatus);
    } catch (error) {
      console.error('Failed to update task status:', error);
    }
  };

  const handleAgentReassignment = async (taskId: string, newAgentId: string) => {
    try {
      await paService.reassignTask(taskId, newAgentId);
      onAgentReassign?.(taskId, newAgentId);
    } catch (error) {
      console.error('Failed to reassign agent:', error);
    }
  };

  const calculatePlanProgress = (plan: GeneratedPlan) => {
    if (!plan.tasks.length) return 0;
    const completedTasks = plan.tasks.filter(task => task.status === TaskStatus.Completed).length;
    return Math.round((completedTasks / plan.tasks.length) * 100);
  };

  return (
    <div className="task-execution-monitor">
      <div className="monitor-header">
        <h2>⚡ Live Task Execution</h2>
        <div className="plan-selector">
          <select
            value={selectedPlan || ''}
            onChange={(e) => setSelectedPlan(e.target.value || null)}
          >
            <option value="">All Active Plans</option>
            {activePlans.map(plan => (
              <option key={plan.plan_id} value={plan.plan_id}>
                {plan.title} ({calculatePlanProgress(plan)}%)
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="execution-content">
        {activePlans
          .filter(plan => !selectedPlan || plan.plan_id === selectedPlan)
          .map(plan => (
            <div key={plan.plan_id} className="plan-execution-card">
              <div className="plan-header">
                <div className="plan-info">
                  <h3>{plan.title}</h3>
                  <p>{plan.description}</p>
                </div>
                <div className="plan-progress">
                  <div className="progress-circle">
                    <svg viewBox="0 0 36 36" className="circular-chart">
                      <path
                        className="circle-bg"
                        d="M18 2.0845
                          a 15.9155 15.9155 0 0 1 0 31.831
                          a 15.9155 15.9155 0 0 1 0 -31.831"
                      />
                      <path
                        className="circle"
                        strokeDasharray={`${calculatePlanProgress(plan)}, 100`}
                        d="M18 2.0845
                          a 15.9155 15.9155 0 0 1 0 31.831
                          a 15.9155 15.9155 0 0 1 0 -31.831"
                      />
                      <text x="18" y="20.35" className="percentage">
                        {calculatePlanProgress(plan)}%
                      </text>
                    </svg>
                  </div>
                </div>
              </div>

              <div className="tasks-grid">
                {plan.tasks.map(task => {
                  const assignment = plan.agent_assignments.find(a => a.task_id === task.id);
                  const progress = taskProgress[task.id] || 0;
                  const agentStatus = assignment ? agentStatuses[assignment.agent_id] || 'unknown' : 'unassigned';

                  return (
                    <div key={task.id} className="task-card">
                      <div className="task-header">
                        <span className="task-status-icon">
                          {getTaskStatusIcon(task.status)}
                        </span>
                        <h4>{task.title}</h4>
                        <div className="task-actions">
                          <select
                            value={task.status}
                            onChange={(e) => handleTaskStatusChange(task.id, e.target.value as TaskStatus)}
                            className="status-selector"
                          >
                            <option value={TaskStatus.Todo}>Todo</option>
                            <option value={TaskStatus.InProgress}>In Progress</option>
                            <option value={TaskStatus.Completed}>Completed</option>
                            <option value={TaskStatus.Cancelled}>Cancelled</option>
                            <option value={TaskStatus.Blocked}>Blocked</option>
                          </select>
                        </div>
                      </div>

                      <div className="task-progress">
                        <div className="progress-bar">
                          <div 
                            className="progress-fill" 
                            style={{ width: `${progress}%` }}
                          ></div>
                        </div>
                        <span className="progress-text">{progress}%</span>
                      </div>

                      {assignment && (
                        <div className="agent-assignment">
                          <div className="agent-info">
                            <div 
                              className="agent-status-dot"
                              style={{ backgroundColor: getAgentStatusColor(agentStatus) }}
                            ></div>
                            <span className="agent-id">🤖 {assignment.agent_id}</span>
                            <span className="agent-status">{agentStatus}</span>
                          </div>
                          <button
                            className="reassign-btn"
                            onClick={() => {
                              const newAgentId = prompt('Enter new agent ID:');
                              if (newAgentId) {
                                handleAgentReassignment(task.id, newAgentId);
                              }
                            }}
                          >
                            🔄
                          </button>
                        </div>
                      )}

                      <div className="task-details">
                        <p>{task.description}</p>
                        {task.estimated_duration && (
                          <span className="duration">⏱️ {task.estimated_duration}</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
      </div>

      {activePlans.length === 0 && (
        <div className="empty-state">
          <div className="empty-icon">📋</div>
          <h3>No Active Plans</h3>
          <p>Generate a plan to start tracking task execution</p>
        </div>
      )}
    </div>
  );
};

export default TaskExecutionMonitor;
