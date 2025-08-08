// PA System Demo - Showcase the powerful PA system features
import React, { useState } from 'react';
import PlanGenerator from './PlanGenerator';
import TaskExecutionMonitor from './TaskExecutionMonitor';
import './PASystemDemo.css';

// Mock data for demonstration
const mockActivePlans = [
  {
    plan_id: 'plan-1',
    title: 'Build React Dashboard',
    description: 'Create a modern, responsive dashboard with real-time data visualization',
    tasks: [
      {
        id: 'task-1',
        title: 'Design UI Components',
        description: 'Create reusable React components for the dashboard',
        status: 'Completed' as any,
        priority: 'High' as any,
        estimated_duration: '4 hours'
      },
      {
        id: 'task-2', 
        title: 'Implement Data Integration',
        description: 'Connect dashboard to backend APIs',
        status: 'InProgress' as any,
        priority: 'High' as any,
        estimated_duration: '6 hours'
      },
      {
        id: 'task-3',
        title: 'Add Real-time Updates',
        description: 'Implement WebSocket connections for live data',
        status: 'Pending' as any,
        priority: 'Medium' as any,
        estimated_duration: '3 hours'
      }
    ],
    agent_assignments: [
      {
        task_id: 'task-1',
        agent_id: 'react-agent-001',
        assignment_reason: 'Specialized in React UI development',
        priority: 'High' as any
      },
      {
        task_id: 'task-2',
        agent_id: 'api-agent-002', 
        assignment_reason: 'Expert in API integration',
        priority: 'High' as any
      }
    ],
    created_at: new Date(),
    priority: 'High' as any,
    dependencies: []
  },
  {
    plan_id: 'plan-2',
    title: 'Optimize Performance',
    description: 'Improve application performance and reduce load times',
    tasks: [
      {
        id: 'task-4',
        title: 'Code Splitting',
        description: 'Implement lazy loading for components',
        status: 'InProgress' as any,
        priority: 'Medium' as any,
        estimated_duration: '2 hours'
      }
    ],
    agent_assignments: [
      {
        task_id: 'task-4',
        agent_id: 'performance-agent-003',
        assignment_reason: 'Performance optimization specialist',
        priority: 'Medium' as any
      }
    ],
    created_at: new Date(),
    priority: 'Medium' as any,
    dependencies: []
  }
];

export const PASystemDemo: React.FC = () => {
  const [showGenerator, setShowGenerator] = useState(false);
  const [demoPlans, setDemoPlans] = useState(mockActivePlans);
  const [notifications, setNotifications] = useState<string[]>([]);

  const handlePlanGenerated = (plan: any) => {
    console.log('New plan generated:', plan);
    setNotifications(prev => [...prev, `✨ Generated new plan: ${plan.title}`]);
    // In a real implementation, this would add to the actual plans
  };

  const handleError = (error: string) => {
    console.error('PA System error:', error);
    setNotifications(prev => [...prev, `❌ Error: ${error}`]);
  };

  const handleTaskUpdate = (taskId: string, status: any) => {
    console.log(`Task ${taskId} updated to ${status}`);
    setNotifications(prev => [...prev, `🔄 Task updated: ${taskId} → ${status}`]);
  };

  const handleAgentReassign = (taskId: string, agentId: string) => {
    console.log(`Task ${taskId} reassigned to ${agentId}`);
    setNotifications(prev => [...prev, `🤖 Agent reassigned: ${taskId} → ${agentId}`]);
  };

  return (
    <div className="pa-system-demo">
      <div className="demo-header">
        <h1>🧠 PA System - Intelligent Task Planning & Execution</h1>
        <p>Experience the power of AI-driven project management with real-time agent orchestration</p>
        
        <div className="demo-controls">
          <button 
            className={`demo-btn ${showGenerator ? 'active' : ''}`}
            onClick={() => setShowGenerator(!showGenerator)}
          >
            {showGenerator ? '📊 View Execution Monitor' : '✨ Create New Plan'}
          </button>
        </div>
      </div>

      {/* Notifications */}
      {notifications.length > 0 && (
        <div className="notifications">
          <h3>🔔 Live Notifications</h3>
          <div className="notification-list">
            {notifications.slice(-5).map((notification, index) => (
              <div key={index} className="notification-item">
                {notification}
              </div>
            ))}
          </div>
          <button 
            className="clear-btn"
            onClick={() => setNotifications([])}
          >
            Clear
          </button>
        </div>
      )}

      {/* Main Content */}
      <div className="demo-content">
        {showGenerator ? (
          <div className="generator-section">
            <PlanGenerator 
              onPlanGenerated={handlePlanGenerated}
              onError={handleError}
            />
            <div className="demo-info">
              <h3>🎯 AI Plan Generation Features</h3>
              <ul>
                <li>✨ Intelligent task breakdown from natural language descriptions</li>
                <li>🤖 Automatic agent assignment based on capabilities</li>
                <li>📊 Priority and timeline estimation</li>
                <li>🔗 Dependency detection and management</li>
                <li>⚡ Real-time plan optimization</li>
              </ul>
            </div>
          </div>
        ) : (
          <div className="monitor-section">
            <TaskExecutionMonitor
              activePlans={demoPlans as any}
              onTaskUpdate={handleTaskUpdate}
              onAgentReassign={handleAgentReassign}
            />
            <div className="demo-info">
              <h3>⚡ Live Execution Features</h3>
              <ul>
                <li>📊 Real-time progress tracking with visual indicators</li>
                <li>🤖 Agent status monitoring and reassignment</li>
                <li>🔄 Interactive task status management</li>
                <li>📈 Plan progress visualization</li>
                <li>🎯 Multi-plan coordination and oversight</li>
              </ul>
            </div>
          </div>
        )}
      </div>

      {/* Stats Dashboard */}
      <div className="stats-dashboard">
        <div className="stat-card">
          <div className="stat-number">{demoPlans.length}</div>
          <div className="stat-label">Active Plans</div>
        </div>
        <div className="stat-card">
          <div className="stat-number">
            {demoPlans.reduce((total, plan) => total + plan.tasks.length, 0)}
          </div>
          <div className="stat-label">Total Tasks</div>
        </div>
        <div className="stat-card">
          <div className="stat-number">
            {demoPlans.reduce((total, plan) => 
              total + plan.tasks.filter(task => task.status === 'Completed').length, 0
            )}
          </div>
          <div className="stat-label">Completed</div>
        </div>
        <div className="stat-card">
          <div className="stat-number">
            {new Set(demoPlans.flatMap(plan => 
              plan.agent_assignments.map(a => a.agent_id)
            )).size}
          </div>
          <div className="stat-label">Active Agents</div>
        </div>
      </div>
    </div>
  );
};

export default PASystemDemo;
