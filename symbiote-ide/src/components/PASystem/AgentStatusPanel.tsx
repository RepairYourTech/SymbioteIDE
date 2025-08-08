// Agent Status Panel - Real-time agent monitoring and management
import React, { useState, useEffect } from 'react';
import { PAAgent, PASystemEvent, AgentStatus } from './types';
import './AgentStatusPanel.css';

interface AgentStatusPanelProps {
  paAgent: PAAgent | null;
  realtimeEvents: PASystemEvent[];
}

export const AgentStatusPanel: React.FC<AgentStatusPanelProps> = ({ 
  paAgent, 
  realtimeEvents 
}) => {
  const [agents, setAgents] = useState<AgentStatus[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<AgentStatus | null>(null);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

  useEffect(() => {
    // Mock agent data - in real implementation, this would come from the backend
    const mockAgents: AgentStatus[] = [
      {
        agent_id: 'pa-agent-001',
        agent_type: 'Planner',
        status: 'available',
        capabilities: ['task_planning', 'agent_coordination', 'project_management'],
        performance_metrics: {
          tasks_completed: 15,
          success_rate: 0.93,
          average_response_time: 1200,
        },
        last_activity: new Date().toISOString(),
      },
      {
        agent_id: 'code-agent-001',
        agent_type: 'Developer',
        status: 'busy',
        current_task: 'Implementing authentication system',
        capabilities: ['coding', 'debugging', 'testing', 'code_review'],
        performance_metrics: {
          tasks_completed: 28,
          success_rate: 0.89,
          average_response_time: 3400,
        },
        last_activity: new Date(Date.now() - 300000).toISOString(),
      },
      {
        agent_id: 'test-agent-001',
        agent_type: 'Tester',
        status: 'available',
        capabilities: ['testing', 'quality_assurance', 'automation'],
        performance_metrics: {
          tasks_completed: 12,
          success_rate: 0.95,
          average_response_time: 2100,
        },
        last_activity: new Date(Date.now() - 600000).toISOString(),
      },
      {
        agent_id: 'deploy-agent-001',
        agent_type: 'Deployment',
        status: 'offline',
        capabilities: ['deployment', 'ci_cd', 'infrastructure'],
        performance_metrics: {
          tasks_completed: 8,
          success_rate: 0.87,
          average_response_time: 5600,
        },
        last_activity: new Date(Date.now() - 3600000).toISOString(),
      },
    ];

    setAgents(mockAgents);
  }, []);

  const getAgentEvents = (agentId: string) => {
    return realtimeEvents.filter(event => event.agent_id === agentId).slice(0, 5);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'available': return '#22c55e';
      case 'busy': return '#f59e0b';
      case 'offline': return '#ef4444';
      default: return '#6b7280';
    }
  };

  return (
    <div className="agent-status-panel">
      {/* Header */}
      <div className="panel-header">
        <div className="header-title">
          <h2>🤖 Agent Status</h2>
          <span className="agent-count">{agents.length} agents</span>
        </div>

        <div className="panel-controls">
          <div className="view-toggle">
            <button 
              className={viewMode === 'grid' ? 'active' : ''}
              onClick={() => setViewMode('grid')}
            >
              ⊞ Grid
            </button>
            <button 
              className={viewMode === 'list' ? 'active' : ''}
              onClick={() => setViewMode('list')}
            >
              ☰ List
            </button>
          </div>
        </div>
      </div>

      {/* Agent Overview */}
      <div className="agent-overview">
        <div className="overview-stats">
          <div className="stat-card">
            <div className="stat-value">{agents.filter(a => a.status === 'available').length}</div>
            <div className="stat-label">Available</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{agents.filter(a => a.status === 'busy').length}</div>
            <div className="stat-label">Busy</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{agents.filter(a => a.status === 'offline').length}</div>
            <div className="stat-label">Offline</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">
              {Math.round(agents.reduce((sum, a) => sum + a.performance_metrics.success_rate, 0) / agents.length * 100)}%
            </div>
            <div className="stat-label">Avg Success</div>
          </div>
        </div>
      </div>

      {/* Agent List/Grid */}
      <div className={`agent-container ${viewMode}`}>
        {viewMode === 'grid' ? (
          <AgentGrid 
            agents={agents}
            selectedAgent={selectedAgent}
            onSelectAgent={setSelectedAgent}
            getStatusColor={getStatusColor}
          />
        ) : (
          <AgentList 
            agents={agents}
            selectedAgent={selectedAgent}
            onSelectAgent={setSelectedAgent}
            getStatusColor={getStatusColor}
            getAgentEvents={getAgentEvents}
          />
        )}
      </div>

      {/* Agent Details Modal */}
      {selectedAgent && (
        <AgentDetailsModal 
          agent={selectedAgent}
          events={getAgentEvents(selectedAgent.agent_id)}
          onClose={() => setSelectedAgent(null)}
        />
      )}
    </div>
  );
};

// Agent Grid View
const AgentGrid: React.FC<{
  agents: AgentStatus[];
  selectedAgent: AgentStatus | null;
  onSelectAgent: (agent: AgentStatus) => void;
  getStatusColor: (status: string) => string;
}> = ({ agents, selectedAgent, onSelectAgent, getStatusColor }) => {
  return (
    <div className="agent-grid">
      {agents.map(agent => (
        <div 
          key={agent.agent_id}
          className={`agent-card ${selectedAgent?.agent_id === agent.agent_id ? 'selected' : ''}`}
          onClick={() => onSelectAgent(agent)}
        >
          <div className="agent-header">
            <div className="agent-avatar">
              <div className="avatar-icon">🤖</div>
              <div 
                className="status-dot"
                style={{ backgroundColor: getStatusColor(agent.status) }}
              />
            </div>
            <div className="agent-info">
              <h3 className="agent-name">{agent.agent_id}</h3>
              <span className="agent-type">{agent.agent_type}</span>
            </div>
          </div>

          <div className="agent-status">
            <div className={`status-badge ${agent.status}`}>
              {agent.status}
            </div>
            {agent.current_task && (
              <div className="current-task">
                📋 {agent.current_task}
              </div>
            )}
          </div>

          <div className="agent-metrics">
            <div className="metric">
              <span className="metric-value">{agent.performance_metrics.tasks_completed}</span>
              <span className="metric-label">Tasks</span>
            </div>
            <div className="metric">
              <span className="metric-value">{Math.round(agent.performance_metrics.success_rate * 100)}%</span>
              <span className="metric-label">Success</span>
            </div>
            <div className="metric">
              <span className="metric-value">{agent.performance_metrics.average_response_time}ms</span>
              <span className="metric-label">Response</span>
            </div>
          </div>

          <div className="agent-capabilities">
            {agent.capabilities.slice(0, 3).map(cap => (
              <span key={cap} className="capability-tag">
                {cap.replace('_', ' ')}
              </span>
            ))}
            {agent.capabilities.length > 3 && (
              <span className="capability-more">+{agent.capabilities.length - 3}</span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
};

// Agent List View
const AgentList: React.FC<{
  agents: AgentStatus[];
  selectedAgent: AgentStatus | null;
  onSelectAgent: (agent: AgentStatus) => void;
  getStatusColor: (status: string) => string;
  getAgentEvents: (agentId: string) => PASystemEvent[];
}> = ({ agents, selectedAgent, onSelectAgent, getStatusColor, getAgentEvents }) => {
  return (
    <div className="agent-list">
      {agents.map(agent => (
        <div 
          key={agent.agent_id}
          className={`agent-row ${selectedAgent?.agent_id === agent.agent_id ? 'selected' : ''}`}
          onClick={() => onSelectAgent(agent)}
        >
          <div className="agent-basic-info">
            <div className="agent-avatar">
              <div className="avatar-icon">🤖</div>
              <div 
                className="status-dot"
                style={{ backgroundColor: getStatusColor(agent.status) }}
              />
            </div>
            <div className="agent-details">
              <h3 className="agent-name">{agent.agent_id}</h3>
              <span className="agent-type">{agent.agent_type}</span>
              {agent.current_task && (
                <div className="current-task">📋 {agent.current_task}</div>
              )}
            </div>
          </div>

          <div className="agent-performance">
            <div className="performance-metric">
              <span className="metric-label">Tasks:</span>
              <span className="metric-value">{agent.performance_metrics.tasks_completed}</span>
            </div>
            <div className="performance-metric">
              <span className="metric-label">Success:</span>
              <span className="metric-value">{Math.round(agent.performance_metrics.success_rate * 100)}%</span>
            </div>
            <div className="performance-metric">
              <span className="metric-label">Response:</span>
              <span className="metric-value">{agent.performance_metrics.average_response_time}ms</span>
            </div>
          </div>

          <div className="agent-activity">
            <div className="last-activity">
              Last active: {new Date(agent.last_activity).toLocaleTimeString()}
            </div>
            <div className="recent-events">
              {getAgentEvents(agent.agent_id).slice(0, 2).map((event, index) => (
                <div key={index} className="event-indicator">
                  {event.type === 'AgentStatusChanged' ? '🔄' : '📋'}
                </div>
              ))}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
};

// Agent Details Modal
const AgentDetailsModal: React.FC<{
  agent: AgentStatus;
  events: PASystemEvent[];
  onClose: () => void;
}> = ({ agent, events, onClose }) => {
  return (
    <div className="agent-details-modal">
      <div className="modal-backdrop" onClick={onClose} />
      <div className="modal-content">
        <div className="modal-header">
          <h2>🤖 {agent.agent_id}</h2>
          <button className="close-btn" onClick={onClose}>✕</button>
        </div>

        <div className="modal-body">
          {/* Agent Overview */}
          <div className="details-section">
            <h3>Agent Overview</h3>
            <div className="agent-overview-details">
              <div className="detail-item">
                <span className="label">Type:</span>
                <span className="value">{agent.agent_type}</span>
              </div>
              <div className="detail-item">
                <span className="label">Status:</span>
                <span className={`value status-${agent.status}`}>{agent.status}</span>
              </div>
              {agent.current_task && (
                <div className="detail-item">
                  <span className="label">Current Task:</span>
                  <span className="value">{agent.current_task}</span>
                </div>
              )}
              <div className="detail-item">
                <span className="label">Last Activity:</span>
                <span className="value">{new Date(agent.last_activity).toLocaleString()}</span>
              </div>
            </div>
          </div>

          {/* Performance Metrics */}
          <div className="details-section">
            <h3>Performance Metrics</h3>
            <div className="metrics-grid">
              <div className="metric-card">
                <div className="metric-icon">📋</div>
                <div className="metric-info">
                  <div className="metric-value">{agent.performance_metrics.tasks_completed}</div>
                  <div className="metric-label">Tasks Completed</div>
                </div>
              </div>
              <div className="metric-card">
                <div className="metric-icon">🎯</div>
                <div className="metric-info">
                  <div className="metric-value">{Math.round(agent.performance_metrics.success_rate * 100)}%</div>
                  <div className="metric-label">Success Rate</div>
                </div>
              </div>
              <div className="metric-card">
                <div className="metric-icon">⚡</div>
                <div className="metric-info">
                  <div className="metric-value">{agent.performance_metrics.average_response_time}ms</div>
                  <div className="metric-label">Avg Response Time</div>
                </div>
              </div>
            </div>
          </div>

          {/* Capabilities */}
          <div className="details-section">
            <h3>Capabilities</h3>
            <div className="capabilities-list">
              {agent.capabilities.map(capability => (
                <span key={capability} className="capability-badge">
                  {capability.replace('_', ' ')}
                </span>
              ))}
            </div>
          </div>

          {/* Recent Events */}
          <div className="details-section">
            <h3>Recent Activity</h3>
            <div className="events-list">
              {events.length > 0 ? (
                events.map((event, index) => (
                  <div key={index} className="event-item">
                    <div className="event-icon">
                      {event.type === 'AgentStatusChanged' ? '🔄' : '📋'}
                    </div>
                    <div className="event-details">
                      <div className="event-type">{event.type}</div>
                      <div className="event-time">
                        {new Date(event.timestamp).toLocaleString()}
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                <div className="no-events">No recent activity</div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
