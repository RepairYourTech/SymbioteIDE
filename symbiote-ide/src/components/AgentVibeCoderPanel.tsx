// Agent Vibe Coder Panel - Revolutionary Agents Building Agents Interface
// Phase 4 UI Component for Agent Creation and Management

import React, { useState, useEffect } from 'react';
// import { invoke } from '@tauri-apps/api/tauri';
import './AgentVibeCoderPanel.css';

interface AgentInfo {
  id: string;
  name: string;
  type: string;
  status: 'active' | 'idle' | 'busy' | 'error';
  currentTask?: string;
}

interface CreatedAgent {
  agent_id: string;
  name: string;
  creator_agent_id: string;
  specification: AgentSpecification;
  status: string;
  performance_metrics: PerformanceMetrics;
  created_at: string;
  last_updated: string;
}

// Note: Agent type alias removed as it's not needed

interface AgentSpecification {
  name: string;
  description: string;
  purpose: string;
  domain: string;
  capabilities: AgentCapability[];
  behavioral_traits: BehavioralTrait[];
  performance_requirements: PerformanceRequirements;
}

interface AgentCapability {
  capability_id: string;
  name: string;
  description: string;
  capability_type: string;
  proficiency_level: string;
}

interface BehavioralTrait {
  trait_id: string;
  name: string;
  description: string;
  trait_type: string;
  intensity: number;
}

interface PerformanceRequirements {
  response_time_ms: number;
  accuracy_threshold: number;
  throughput_requests_per_second: number;
  memory_limit_mb: number;
  cpu_limit_percentage: number;
  concurrent_tasks: number;
}

interface PerformanceMetrics {
  response_time_avg: number;
  accuracy_score: number;
  throughput: number;
  error_rate: number;
}

interface AgentVibeCoderPanelProps {
  agents: AgentInfo[];
  onAgentCreated: (agent: AgentInfo) => void;
}

export const AgentVibeCoderPanel: React.FC<AgentVibeCoderPanelProps> = ({
  agents: agentsProp,
  onAgentCreated,
}) => {
  const [activeTab, setActiveTab] = useState<'create' | 'manage' | 'templates'>('create');
  const [createdAgents, setCreatedAgents] = useState<CreatedAgent[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedAgent, setSelectedAgent] = useState<CreatedAgent | null>(null);

  // Agent creation form state
  const [agentForm, setAgentForm] = useState({
    name: '',
    description: '',
    purpose: '',
    domain: 'CodeGeneration',
    capabilities: [] as string[],
    behavioralTraits: [] as { type: string; intensity: number }[],
    performanceRequirements: {
      responseTime: 1000,
      accuracy: 0.95,
      throughput: 10,
      memoryLimit: 512,
      cpuLimit: 50,
      concurrentTasks: 5,
    },
  });

  const domains = [
    'CodeGeneration', 'Testing', 'Debugging', 'Refactoring', 'Documentation',
    'CodeAnalysis', 'PerformanceAnalysis', 'SecurityAnalysis', 'QualityAssurance',
    'Planning', 'Coordination', 'Monitoring', 'Reporting', 'MachineLearning',
    'DataScience', 'DevOps', 'UI_UX', 'Architecture',
  ];

  const availableCapabilities = [
    { id: 'reasoning', name: 'Advanced Reasoning', type: 'Reasoning' },
    { id: 'problem_solving', name: 'Problem Solving', type: 'ProblemSolving' },
    { id: 'pattern_recognition', name: 'Pattern Recognition', type: 'PatternRecognition' },
    { id: 'learning', name: 'Continuous Learning', type: 'Learning' },
    { id: 'code_generation', name: 'Code Generation', type: 'CodeGeneration' },
    { id: 'code_analysis', name: 'Code Analysis', type: 'CodeAnalysis' },
    { id: 'testing', name: 'Test Creation', type: 'Testing' },
    { id: 'debugging', name: 'Bug Detection', type: 'Debugging' },
    { id: 'natural_language', name: 'Natural Language', type: 'NaturalLanguage' },
    { id: 'technical_writing', name: 'Technical Writing', type: 'TechnicalWriting' },
    { id: 'collaboration', name: 'Team Collaboration', type: 'Collaboration' },
    { id: 'explanation', name: 'Code Explanation', type: 'Explanation' },
  ];

  const behavioralTraitTypes = [
    'Curiosity', 'Persistence', 'Creativity', 'Analytical', 'Methodical',
    'Collaborative', 'Independent', 'DetailOriented', 'BigPicture', 'RiskTaking',
    'Verbose', 'Concise', 'Formal', 'Casual', 'Empathetic',
  ];

  useEffect(() => {
    loadCreatedAgents();
  }, []);

  const loadCreatedAgents = async () => {
    try {
      // Mock implementation - replace with actual Tauri invoke when ready
      const agents: CreatedAgent[] = [];
      setCreatedAgents(agents || []);
      console.log('Agents loaded:', agents.length, 'Selected agent:', selectedAgent?.name || 'none');
      console.log('Props agents:', agentsProp.length);
    } catch (error) {
      console.error('Failed to load created agents:', error);
    }
  };

  const createAgent = async () => {
    if (!agentForm.name || !agentForm.description || !agentForm.purpose) {
      alert('Please fill in all required fields');
      return;
    }

    setLoading(true);
    try {
      // Generate unique agent ID
      const agentId = `agent_${Date.now()}`;
      console.log('Creating agent with specification:', {
        name: agentForm.name,
        domain: agentForm.domain,
        description: agentForm.description,
        capabilities: agentForm.capabilities,
        traits: agentForm.behavioralTraits,
        performance: agentForm.performanceRequirements,
      });

      // Mock agent creation success
      const newAgent: CreatedAgent = {
        agent_id: agentId,
        name: agentForm.name,
        creator_agent_id: 'system',
        specification: {
          name: agentForm.name,
          description: agentForm.description,
          purpose: agentForm.purpose,
          domain: agentForm.domain,
          capabilities: [],
          behavioral_traits: [],
          performance_requirements: {
            response_time_ms: agentForm.performanceRequirements.responseTime,
            accuracy_threshold: agentForm.performanceRequirements.accuracy,
            throughput_requests_per_second: agentForm.performanceRequirements.throughput,
            memory_limit_mb: agentForm.performanceRequirements.memoryLimit,
            cpu_limit_percentage: agentForm.performanceRequirements.cpuLimit,
            concurrent_tasks: agentForm.performanceRequirements.concurrentTasks,
          },
        },
        status: 'active',
        performance_metrics: {
          response_time_avg: 0,
          accuracy_score: 0,
          throughput: 0,
          error_rate: 0,
        },
        created_at: new Date().toISOString(),
        last_updated: new Date().toISOString(),
      };
      
      setCreatedAgents(prev => [...prev, newAgent]);

      // Reset form and reload agents
      setAgentForm({
        name: '', description: '', purpose: '', domain: 'CodeGeneration',
        capabilities: [], behavioralTraits: [],
        performanceRequirements: {
          responseTime: 1000, accuracy: 0.95, throughput: 10,
          memoryLimit: 512, cpuLimit: 50, concurrentTasks: 5,
        },
      });

      await loadCreatedAgents();
      onAgentCreated({
        id: agentId, name: agentForm.name, type: agentForm.domain, status: 'idle',
      });

      alert('Agent created successfully!');
    } catch (error) {
      console.error('Failed to create agent:', error);
      alert('Failed to create agent. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const toggleCapability = (capabilityId: string) => {
    setAgentForm(prev => ({
      ...prev,
      capabilities: prev.capabilities.includes(capabilityId)
        ? prev.capabilities.filter(id => id !== capabilityId)
        : [...prev.capabilities, capabilityId],
    }));
  };

  const addBehavioralTrait = (traitType: string) => {
    setAgentForm(prev => ({
      ...prev,
      behavioralTraits: [...prev.behavioralTraits, { type: traitType, intensity: 0.7 }],
    }));
  };

  const removeBehavioralTrait = (index: number) => {
    setAgentForm(prev => ({
      ...prev,
      behavioralTraits: prev.behavioralTraits.filter((_, i) => i !== index),
    }));
  };

  const updateTraitIntensity = (index: number, intensity: number) => {
    setAgentForm(prev => ({
      ...prev,
      behavioralTraits: prev.behavioralTraits.map((trait, i) =>
        i === index ? { ...trait, intensity } : trait
      ),
    }));
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Active': return '🟢';
      case 'Creating': return '🟡';
      case 'Testing': return '🔵';
      case 'Failed': return '🔴';
      default: return '⚪';
    }
  };

  const getDomainIcon = (domain: string) => {
    const icons: { [key: string]: string } = {
      'CodeGeneration': '⚡', 'Testing': '🧪', 'Debugging': '🐛',
      'Refactoring': '🔧', 'Documentation': '📝', 'CodeAnalysis': '🔍',
      'PerformanceAnalysis': '📊', 'SecurityAnalysis': '🛡️', 'QualityAssurance': '✅',
      'Planning': '📋', 'Coordination': '🎯', 'Monitoring': '👁️',
      'Reporting': '📈', 'MachineLearning': '🤖', 'DataScience': '📊',
      'DevOps': '🚀', 'UI_UX': '🎨', 'Architecture': '🏗️',
    };
    return icons[domain] || '🤖';
  };

  return (
    <div className="agent-vibe-coder-panel">
      {/* Tab Navigation */}
      <div className="tab-navigation">
        <button
          className={`tab ${activeTab === 'create' ? 'active' : ''}`}
          onClick={() => setActiveTab('create')}
        >
          🛠️ Create Agent
        </button>
        <button
          className={`tab ${activeTab === 'manage' ? 'active' : ''}`}
          onClick={() => setActiveTab('manage')}
        >
          👥 Manage Agents
        </button>
        <button
          className={`tab ${activeTab === 'templates' ? 'active' : ''}`}
          onClick={() => setActiveTab('templates')}
        >
          📋 Templates
        </button>
      </div>

      {/* Tab Content */}
      <div className="tab-content">
        {activeTab === 'create' && (
          <div className="create-agent-tab">
            <div className="form-section">
              <h3>Basic Information</h3>
              <div className="form-group">
                <label>Agent Name *</label>
                <input
                  type="text"
                  value={agentForm.name}
                  onChange={(e) => setAgentForm(prev => ({ ...prev, name: e.target.value }))}
                  placeholder="Enter agent name..."
                  className="form-input"
                />
              </div>
              
              <div className="form-group">
                <label>Description *</label>
                <textarea
                  value={agentForm.description}
                  onChange={(e) => setAgentForm(prev => ({ ...prev, description: e.target.value }))}
                  placeholder="Describe what this agent does..."
                  className="form-textarea"
                  rows={3}
                />
              </div>
              
              <div className="form-group">
                <label>Purpose *</label>
                <input
                  type="text"
                  value={agentForm.purpose}
                  onChange={(e) => setAgentForm(prev => ({ ...prev, purpose: e.target.value }))}
                  placeholder="Primary purpose of this agent..."
                  className="form-input"
                />
              </div>
              
              <div className="form-group">
                <label>Domain</label>
                <select
                  value={agentForm.domain}
                  onChange={(e) => setAgentForm(prev => ({ ...prev, domain: e.target.value }))}
                  className="form-select"
                >
                  {domains.map(domain => (
                    <option key={domain} value={domain}>
                      {getDomainIcon(domain)} {domain.replace(/([A-Z])/g, ' $1').trim()}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="form-section">
              <h3>Capabilities</h3>
              <div className="capabilities-grid">
                {availableCapabilities.map(capability => (
                  <div
                    key={capability.id}
                    className={`capability-card ${agentForm.capabilities.includes(capability.id) ? 'selected' : ''}`}
                    onClick={() => toggleCapability(capability.id)}
                  >
                    <div className="capability-name">{capability.name}</div>
                    <div className="capability-type">{capability.type}</div>
                  </div>
                ))}
              </div>
            </div>

            <div className="form-section">
              <h3>Behavioral Traits</h3>
              <div className="traits-section">
                <div className="trait-selector">
                  <select
                    onChange={(e) => {
                      if (e.target.value) {
                        addBehavioralTrait(e.target.value);
                        e.target.value = '';
                      }
                    }}
                    className="form-select"
                  >
                    <option value="">Add behavioral trait...</option>
                    {behavioralTraitTypes.map(trait => (
                      <option key={trait} value={trait}>{trait}</option>
                    ))}
                  </select>
                </div>
                
                <div className="traits-list">
                  {agentForm.behavioralTraits.map((trait, index) => (
                    <div key={index} className="trait-item">
                      <div className="trait-info">
                        <span className="trait-name">{trait.type}</span>
                        <span className="trait-intensity">
                          Intensity: {Math.round(trait.intensity * 100)}%
                        </span>
                      </div>
                      <div className="trait-controls">
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.1"
                          value={trait.intensity}
                          onChange={(e) => updateTraitIntensity(index, parseFloat(e.target.value))}
                          className="intensity-slider"
                        />
                        <button
                          onClick={() => removeBehavioralTrait(index)}
                          className="remove-trait-button"
                        >
                          ×
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="form-actions">
              <button
                onClick={createAgent}
                disabled={loading || !agentForm.name || !agentForm.description || !agentForm.purpose}
                className="create-button"
              >
                {loading ? '⏳ Creating Agent...' : '🤖 Create Agent'}
              </button>
            </div>
          </div>
        )}

        {activeTab === 'manage' && (
          <div className="manage-agents-tab">
            <div className="agents-header">
                  <h4>Your Created Agents ({createdAgents.length})</h4>
              <button onClick={loadCreatedAgents} className="refresh-button">
                🔄 Refresh
              </button>
            </div>
            
            <div className="agents-grid">
                {createdAgents.map((agent: CreatedAgent) => (
                <div
                  key={agent.agent_id}
                  className="agent-card"
                  onClick={() => setSelectedAgent(agent)}
                >
                  <div className="agent-header">
                    <div className="agent-info">
                      <div className="agent-name">{agent.name}</div>
                      <div className="agent-domain">
                        {getDomainIcon(agent.specification.domain)} {agent.specification.domain}
                      </div>
                    </div>
                    <div className="agent-status">
                      {getStatusIcon(agent.status)} {agent.status}
                    </div>
                  </div>
                  
                  <div className="agent-description">
                    {agent.specification.description}
                  </div>
                  
                  <div className="agent-metrics">
                    <div className="metric">
                      <span className="metric-label">Accuracy:</span>
                      <span className="metric-value">
                        {(agent.performance_metrics.accuracy_score * 100).toFixed(1)}%
                      </span>
                    </div>
                    <div className="metric">
                      <span className="metric-label">Response:</span>
                      <span className="metric-value">
                        {agent.performance_metrics.response_time_avg.toFixed(0)}ms
                      </span>
                    </div>
                  </div>
                  
                  <div className="agent-capabilities">
                        {agent.specification.capabilities.slice(0, 3).map((cap: AgentCapability) => (
                      <span key={cap.capability_id} className="capability-tag">
                        {cap.name}
                      </span>
                    ))}
                    {agent.specification.capabilities.length > 3 && (
                      <span className="capability-tag more">
                        +{agent.specification.capabilities.length - 3} more
                      </span>
                    )}
                  </div>
                  
                  <div className="agent-footer">
                    <span className="created-date">
                      Created: {new Date(agent.created_at).toLocaleDateString()}
                    </span>
                  </div>
                </div>
              ))}
            </div>
            
            {createdAgents.length === 0 && (
              <div className="empty-state">
                <h4>No agents created yet</h4>
                <p>Create your first AI agent using the Create Agent tab</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'templates' && (
          <div className="templates-tab">
            <div className="templates-header">
              <h3>Agent Templates</h3>
              <p>Pre-configured agent templates for common use cases</p>
            </div>
            
            <div className="templates-grid">
              {[
                { icon: '⚡', name: 'Code Generator', desc: 'Specialized in generating high-quality code across multiple languages' },
                { icon: '🧪', name: 'Test Engineer', desc: 'Creates comprehensive test suites and performs quality assurance' },
                { icon: '🐛', name: 'Debug Specialist', desc: 'Expert at identifying and fixing bugs in complex codebases' },
                { icon: '📝', name: 'Documentation Writer', desc: 'Generates clear, comprehensive documentation for code and APIs' },
                { icon: '🔍', name: 'Code Reviewer', desc: 'Performs thorough code reviews and suggests improvements' },
                { icon: '🏗️', name: 'System Architect', desc: 'Designs system architecture and makes technical decisions' },
              ].map((template, index) => (
                <div key={index} className="template-card">
                  <div className="template-icon">{template.icon}</div>
                  <div className="template-name">{template.name}</div>
                  <div className="template-description">{template.desc}</div>
                  <button className="use-template-button">Use Template</button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AgentVibeCoderPanel;
