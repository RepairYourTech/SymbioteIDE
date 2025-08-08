// PA System Manager - Main React component for PA system integration
import React, { useState, useEffect, useCallback } from 'react';
import { PASystemService, PASystemConfig as ServicePASystemConfig } from '../../services/PASystemService';
import { PAAgent, GeneratedPlan, Task, TaskStatus } from './types';
import PlanGenerator from './PlanGenerator';
import TaskExecutionMonitor from './TaskExecutionMonitor';
import './PASystemManager.css';

interface PASystemManagerProps {
  onSystemInitialized?: () => void;
  onError?: (error: string) => void;
}

export const PASystemManager: React.FC<PASystemManagerProps> = ({ 
  onSystemInitialized, 
  onError 
}) => {
  const [isInitialized, setIsInitialized] = useState<boolean>(false);
  const [agents, setAgents] = useState<any[]>([]);
  const [tasks, setTasks] = useState<any[]>([]);
  const [plans, setPlans] = useState<any[]>([]);
  const [systemStatus, setSystemStatus] = useState<{
    active_agents: number;
    pending_tasks: number;
    completed_tasks: number;
    system_health: 'healthy' | 'warning' | 'error';
  } | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const paService = PASystemService.getInstance();

  // Initialize PA System
  const initializeSystem = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const config: ServicePASystemConfig = {
        max_agents: 10,
        task_timeout_seconds: 300,
        enable_real_time_updates: true,
      };

      await paService.initializePASystem(config);
      setIsInitialized(true);
      onSystemInitialized?.();

      // Load initial data
      await loadSystemData();

      // Subscribe to real-time updates
      await paService.subscribeToUpdates(
        (agent: any) => {
          setAgents(prev => prev.map(a => a.agent_id === agent.agent_id ? agent : a));
        },
        (task: any) => {
          setTasks(prev => prev.map(t => t.id === task.id ? task : t));
        },
        (plan: any) => {
          setPlans(prev => prev.map(p => p.plan_id === plan.plan_id ? plan : p));
        }
      );

    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error occurred';
      setError(errorMessage);
      onError?.(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [onSystemInitialized, onError]);

  // Load system data
  const loadSystemData = useCallback(async () => {
    try {
      const [agentsData, tasksData, statusData] = await Promise.all([
        paService.getAgents(),
        paService.getTasks(),
        paService.getSystemStatus(),
      ]);

      // Mock plans data since getPlans doesn't exist
      const plansData: GeneratedPlan[] = [];

      setAgents(agentsData);
      setTasks(tasksData);
      setPlans(plansData);
      setSystemStatus(statusData);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load system data';
      setError(errorMessage);
    }
  }, []);

  // Initialize on mount
  useEffect(() => {
    initializeSystem();
  }, [initializeSystem]);

  if (loading) {
    return (
      <div className="pa-system-loading">
        <div className="loading-spinner"></div>
        <p>Initializing PA System...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="pa-system-error">
        <h3>PA System Error</h3>
        <p>{error}</p>
        <button onClick={initializeSystem}>Retry</button>
      </div>
    );
  }

  if (!isInitialized) {
    return (
      <div className="pa-system-not-initialized">
        <h3>PA System Not Initialized</h3>
        <button onClick={initializeSystem}>Initialize System</button>
      </div>
    );
  }

  return (
    <div className="pa-system-manager">
      <header className="pa-system-header">
        <h2>🤖 Planner Agent System</h2>
        {systemStatus && (
          <div className={`system-status ${systemStatus.system_health}`}>
            <span className="status-indicator"></span>
            <span>System {systemStatus.system_health}</span>
          </div>
        )}
      </header>

      <div className="pa-system-stats">
        <div className="stat-card">
          <h4>Active Agents</h4>
          <span className="stat-value">{systemStatus?.active_agents || 0}</span>
        </div>
        <div className="stat-card">
          <h4>Pending Tasks</h4>
          <span className="stat-value">{systemStatus?.pending_tasks || 0}</span>
        </div>
        <div className="stat-card">
          <h4>Completed Tasks</h4>
          <span className="stat-value">{systemStatus?.completed_tasks || 0}</span>
        </div>
      </div>

      <div className="pa-system-content">
        <div className="agents-section">
          <h3>Agents ({agents.length})</h3>
          <div className="agents-grid">
            {agents.map((agent, index) => (
              <div key={agent.agent_id || agent.id || index} className={`agent-card ${agent.status || 'active'}`}>
                <h4>{agent.name || agent.agent_name || 'Unknown Agent'}</h4>
                <p className="agent-role">{agent.role || agent.agent_type || 'General'}</p>
                <div className={`agent-status ${agent.status || 'active'}`}>
                  {agent.status || 'Active'}
                </div>
                {agent.current_task && (
                  <p className="current-task">Working on: {agent.current_task}</p>
                )}
              </div>
            ))}
          </div>
        </div>

        <div className="tasks-section">
          <h3>Tasks ({tasks.length})</h3>
          <div className="tasks-list">
            {tasks.map(task => (
              <div key={task.id} className={`task-item ${task.status}`}>
                <div className="task-header">
                  <h4>{task.title}</h4>
                  <span className={`task-priority ${task.priority}`}>
                    {task.priority}
                  </span>
                </div>
                <p className="task-description">{task.description}</p>
                <div className="task-meta">
                  <span className={`task-status ${task.status}`}>
                    {task.status}
                  </span>
                  {task.assigned_to && (
                    <span className="assigned-agent">
                      Assigned to: {task.assigned_to}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default PASystemManager;
