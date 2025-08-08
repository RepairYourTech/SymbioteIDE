import React, { useState, useEffect, useCallback } from 'react';
import './ProjectManagementPanel.css';

// Types for project management features
interface Project {
  id: string;
  name: string;
  description: string;
  type: 'web' | 'mobile' | 'desktop' | 'library' | 'api' | 'other';
  status: 'planning' | 'development' | 'testing' | 'deployment' | 'maintenance' | 'archived';
  priority: 'low' | 'medium' | 'high' | 'critical';
  created_at: string;
  updated_at: string;
  deadline?: string;
  progress: number;
  team_members: string[];
  technologies: string[];
  repository?: {
    url: string;
    branch: string;
    last_commit: string;
  };
  metrics: {
    lines_of_code: number;
    test_coverage: number;
    build_status: 'success' | 'failed' | 'pending';
    deployment_status: 'deployed' | 'staging' | 'local' | 'failed';
  };
}

interface ProjectTask {
  id: string;
  project_id: string;
  title: string;
  description: string;
  status: 'todo' | 'in_progress' | 'review' | 'done';
  priority: 'low' | 'medium' | 'high';
  assignee?: string;
  estimated_hours: number;
  actual_hours: number;
  created_at: string;
  due_date?: string;
  dependencies: string[];
  tags: string[];
}

interface ProjectInsight {
  type: 'performance' | 'quality' | 'timeline' | 'resource' | 'risk';
  title: string;
  description: string;
  severity: 'info' | 'warning' | 'error';
  recommendation: string;
  impact: 'low' | 'medium' | 'high';
  confidence: number;
}

interface ProjectManagementPanelProps {
  currentProject?: string;
  onProjectSelect: (projectId: string) => void;
  onTaskCreate: (task: Partial<ProjectTask>) => void;
  onProjectUpdate: (project: Partial<Project>) => void;
}

const ProjectManagementPanel: React.FC<ProjectManagementPanelProps> = ({
  currentProject,
  onProjectSelect,
  onTaskCreate,
  onProjectUpdate
}) => {
  const [activeTab, setActiveTab] = useState<'overview' | 'tasks' | 'analytics' | 'insights'>('overview');
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [projectTasks, setProjectTasks] = useState<ProjectTask[]>([]);
  const [projectInsights, setProjectInsights] = useState<ProjectInsight[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  // Mock data initialization
  useEffect(() => {
    loadProjects();
  }, []);

  useEffect(() => {
    if (selectedProject) {
      loadProjectTasks(selectedProject.id);
      loadProjectInsights(selectedProject.id);
    }
  }, [selectedProject]);

  const loadProjects = async () => {
    setIsLoading(true);
    try {
      // Mock project data - replace with actual Tauri invoke
      const mockProjects: Project[] = [
        {
          id: 'proj_1',
          name: 'SymbioteIDE',
          description: 'Revolutionary AI-powered IDE with advanced agent system',
          type: 'desktop',
          status: 'development',
          priority: 'critical',
          created_at: '2025-01-01T00:00:00Z',
          updated_at: '2025-01-06T22:00:00Z',
          deadline: '2025-03-01T00:00:00Z',
          progress: 75,
          team_members: ['AI Agent', 'Developer', 'Architect'],
          technologies: ['Rust', 'TypeScript', 'React', 'Tauri'],
          repository: {
            url: 'https://github.com/user/symbiote-ide',
            branch: 'main',
            last_commit: 'feat: Add project management panel'
          },
          metrics: {
            lines_of_code: 45000,
            test_coverage: 82,
            build_status: 'success',
            deployment_status: 'local'
          }
        }
      ];

      setProjects(mockProjects);
      if (currentProject) {
        const project = mockProjects.find(p => p.id === currentProject);
        if (project) setSelectedProject(project);
      } else if (mockProjects.length > 0) {
        setSelectedProject(mockProjects[0]);
      }
    } catch (error) {
      console.error('Failed to load projects:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const loadProjectTasks = async (projectId: string) => {
    try {
      // Mock task data - replace with actual Tauri invoke
      const mockTasks: ProjectTask[] = [
        {
          id: 'task_1',
          project_id: projectId,
          title: 'Implement Code Intelligence Panel',
          description: 'Create AI-powered code analysis and insights panel',
          status: 'done',
          priority: 'high',
          assignee: 'AI Agent',
          estimated_hours: 8,
          actual_hours: 6,
          created_at: '2025-01-06T10:00:00Z',
          due_date: '2025-01-06T18:00:00Z',
          dependencies: [],
          tags: ['ui', 'ai', 'analysis']
        },
        {
          id: 'task_2',
          project_id: projectId,
          title: 'Enhanced Terminal Integration',
          description: 'Build AI-powered terminal with natural language commands',
          status: 'done',
          priority: 'medium',
          assignee: 'Developer',
          estimated_hours: 6,
          actual_hours: 5,
          created_at: '2025-01-06T12:00:00Z',
          due_date: '2025-01-06T20:00:00Z',
          dependencies: ['task_1'],
          tags: ['terminal', 'ai', 'commands']
        },
        {
          id: 'task_3',
          project_id: projectId,
          title: 'Backend Service Integration',
          description: 'Connect frontend components to Rust/Tauri backend services',
          status: 'in_progress',
          priority: 'high',
          assignee: 'Architect',
          estimated_hours: 12,
          actual_hours: 8,
          created_at: '2025-01-06T14:00:00Z',
          due_date: '2025-01-07T18:00:00Z',
          dependencies: ['task_1', 'task_2'],
          tags: ['backend', 'integration', 'tauri']
        }
      ];

      setProjectTasks(mockTasks);
    } catch (error) {
      console.error('Failed to load project tasks:', error);
    }
  };

  const loadProjectInsights = async (projectId: string) => {
    try {
      // Mock insights data - replace with actual Tauri invoke
      const mockInsights: ProjectInsight[] = [
        {
          type: 'performance',
          title: 'Build Performance Optimization',
          description: 'Build times have increased by 15% over the last week',
          severity: 'warning',
          recommendation: 'Consider optimizing TypeScript compilation and reducing bundle size',
          impact: 'medium',
          confidence: 0.85
        },
        {
          type: 'quality',
          title: 'Code Quality Improvement',
          description: 'Test coverage has improved to 82%, exceeding the 80% target',
          severity: 'info',
          recommendation: 'Maintain current testing practices and focus on edge cases',
          impact: 'high',
          confidence: 0.95
        }
      ];

      setProjectInsights(mockInsights);
    } catch (error) {
      console.error('Failed to load project insights:', error);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'planning': return '#6c757d';
      case 'development': return '#007acc';
      case 'testing': return '#ffc107';
      case 'deployment': return '#28a745';
      case 'maintenance': return '#17a2b8';
      case 'archived': return '#6c757d';
      default: return '#6c757d';
    }
  };

  const getPriorityColor = (priority: string) => {
    switch (priority) {
      case 'low': return '#28a745';
      case 'medium': return '#ffc107';
      case 'high': return '#fd7e14';
      case 'critical': return '#dc3545';
      default: return '#6c757d';
    }
  };

  return (
    <div className="project-management-panel">
      <div className="panel-header">
        <div className="header-left">
          <h3>📋 Project Management</h3>
          <div className="project-selector">
            <select
              value={selectedProject?.id || ''}
              onChange={(e) => {
                const project = projects.find(p => p.id === e.target.value);
                if (project) {
                  setSelectedProject(project);
                  onProjectSelect(project.id);
                }
              }}
              className="project-select"
            >
              {projects.map(project => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="header-controls">
          <button className="create-project-btn">
            ➕ New Project
          </button>
          
          <button
            className="create-task-btn"
            disabled={!selectedProject}
          >
            📝 New Task
          </button>
        </div>
      </div>

      <div className="panel-tabs">
        <button
          className={`tab ${activeTab === 'overview' ? 'active' : ''}`}
          onClick={() => setActiveTab('overview')}
        >
          📊 Overview
        </button>
        <button
          className={`tab ${activeTab === 'tasks' ? 'active' : ''}`}
          onClick={() => setActiveTab('tasks')}
        >
          ✅ Tasks
        </button>
        <button
          className={`tab ${activeTab === 'analytics' ? 'active' : ''}`}
          onClick={() => setActiveTab('analytics')}
        >
          📈 Analytics
        </button>
        <button
          className={`tab ${activeTab === 'insights' ? 'active' : ''}`}
          onClick={() => setActiveTab('insights')}
        >
          💡 Insights
        </button>
      </div>

      <div className="panel-content">
        {activeTab === 'overview' && selectedProject && (
          <div className="overview-tab">
            <div className="project-summary">
              <div className="project-info">
                <div className="project-header">
                  <h4>{selectedProject.name}</h4>
                  <div className="project-badges">
                    <span 
                      className="status-badge"
                      style={{ backgroundColor: getStatusColor(selectedProject.status) }}
                    >
                      {selectedProject.status}
                    </span>
                    <span 
                      className="priority-badge"
                      style={{ backgroundColor: getPriorityColor(selectedProject.priority) }}
                    >
                      {selectedProject.priority}
                    </span>
                    <span className="type-badge">{selectedProject.type}</span>
                  </div>
                </div>
                
                <p className="project-description">{selectedProject.description}</p>
                
                <div className="project-metrics">
                  <div className="metric-card">
                    <div className="metric-value">{selectedProject.progress}%</div>
                    <div className="metric-label">Progress</div>
                    <div className="progress-bar">
                      <div 
                        className="progress-fill"
                        style={{ width: `${selectedProject.progress}%` }}
                      />
                    </div>
                  </div>
                  
                  <div className="metric-card">
                    <div className="metric-value">{selectedProject.metrics.lines_of_code.toLocaleString()}</div>
                    <div className="metric-label">Lines of Code</div>
                  </div>
                  
                  <div className="metric-card">
                    <div className="metric-value">{selectedProject.metrics.test_coverage}%</div>
                    <div className="metric-label">Test Coverage</div>
                  </div>
                  
                  <div className="metric-card">
                    <div className="metric-value">
                      <span className={`status-indicator ${selectedProject.metrics.build_status}`}>
                        {selectedProject.metrics.build_status === 'success' ? '✅' : 
                         selectedProject.metrics.build_status === 'failed' ? '❌' : '⏳'}
                      </span>
                    </div>
                    <div className="metric-label">Build Status</div>
                  </div>
                </div>
              </div>

              <div className="project-details">
                <div className="detail-section">
                  <h5>👥 Team Members</h5>
                  <div className="team-members">
                    {selectedProject.team_members.map((member, index) => (
                      <span key={index} className="team-member">{member}</span>
                    ))}
                  </div>
                </div>

                <div className="detail-section">
                  <h5>🛠️ Technologies</h5>
                  <div className="technologies">
                    {selectedProject.technologies.map((tech, index) => (
                      <span key={index} className="technology-tag">{tech}</span>
                    ))}
                  </div>
                </div>

                {selectedProject.repository && (
                  <div className="detail-section">
                    <h5>📂 Repository</h5>
                    <div className="repository-info">
                      <div className="repo-url">
                        <a href={selectedProject.repository.url} target="_blank" rel="noopener noreferrer">
                          {selectedProject.repository.url}
                        </a>
                      </div>
                      <div className="repo-details">
                        <span>Branch: {selectedProject.repository.branch}</span>
                        <span>Last Commit: {selectedProject.repository.last_commit}</span>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {activeTab === 'tasks' && (
          <div className="tasks-tab">
            <div className="tasks-header">
              <h4>Project Tasks</h4>
            </div>

            <div className="tasks-list">
              {projectTasks.map(task => (
                <div key={task.id} className={`task-card ${task.status}`}>
                  <div className="task-header">
                    <div className="task-info">
                      <h5 className="task-title">{task.title}</h5>
                      <p className="task-description">{task.description}</p>
                    </div>
                    
                    <div className="task-badges">
                      <span 
                        className="priority-badge"
                        style={{ backgroundColor: getPriorityColor(task.priority) }}
                      >
                        {task.priority}
                      </span>
                      <span className="status-badge">{task.status.replace('_', ' ')}</span>
                    </div>
                  </div>

                  <div className="task-details">
                    <div className="task-meta">
                      {task.assignee && (
                        <span className="assignee">👤 {task.assignee}</span>
                      )}
                      {task.due_date && (
                        <span className="due-date">
                          📅 {new Date(task.due_date).toLocaleDateString()}
                        </span>
                      )}
                      <span className="time-tracking">
                        ⏱️ {task.actual_hours}h / {task.estimated_hours}h
                      </span>
                    </div>

                    {task.tags.length > 0 && (
                      <div className="task-tags">
                        {task.tags.map((tag, index) => (
                          <span key={index} className="task-tag">{tag}</span>
                        ))}
                      </div>
                    )}
                  </div>

                  <div className="task-actions">
                    <button className="edit-task-btn">✏️ Edit</button>
                    <button className="view-task-btn">👁️ View</button>
                    {task.status !== 'done' && (
                      <button className="complete-task-btn">✅ Complete</button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'analytics' && selectedProject && (
          <div className="analytics-tab">
            <div className="analytics-overview">
              <h4>📊 Project Analytics</h4>
              
              <div className="analytics-cards">
                <div className="analytics-card">
                  <div className="card-header">
                    <h5>Task Completion Rate</h5>
                    <span className="card-icon">✅</span>
                  </div>
                  <div className="card-value">75%</div>
                  <div className="card-trend positive">↗️ +12% this week</div>
                </div>

                <div className="analytics-card">
                  <div className="card-header">
                    <h5>Code Quality Score</h5>
                    <span className="card-icon">🏆</span>
                  </div>
                  <div className="card-value">8.5/10</div>
                  <div className="card-trend positive">↗️ +0.3 this week</div>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'insights' && (
          <div className="insights-tab">
            <div className="insights-header">
              <h4>💡 AI-Powered Project Insights</h4>
              <p>Intelligent analysis and recommendations for your project</p>
            </div>

            <div className="insights-list">
              {projectInsights.map((insight, index) => (
                <div key={index} className={`insight-card ${insight.severity}`}>
                  <div className="insight-header">
                    <div className="insight-info">
                      <h5>{insight.title}</h5>
                      <span className="insight-type">{insight.type}</span>
                    </div>
                    
                    <div className="insight-meta">
                      <span className={`severity-badge ${insight.severity}`}>
                        {insight.severity}
                      </span>
                      <span className="confidence-score">
                        {(insight.confidence * 100).toFixed(0)}% confidence
                      </span>
                    </div>
                  </div>

                  <p className="insight-description">{insight.description}</p>

                  <div className="insight-recommendation">
                    <h6>💡 Recommendation</h6>
                    <p>{insight.recommendation}</p>
                  </div>

                  <div className="insight-impact">
                    <span className="impact-label">Impact:</span>
                    <span className={`impact-value ${insight.impact}`}>
                      {insight.impact}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {isLoading && (
          <div className="loading-overlay">
            <div className="loading-spinner">⏳</div>
            <p>Loading project data...</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default ProjectManagementPanel;
