// Settings Panel - Comprehensive SymbioteIDE Configuration
// Exposes all backend settings, AI providers, agents, and advanced features

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SettingsPanel.css';

interface AIProviderConfig {
  name: string;
  enabled: boolean;
  apiKey?: string;
  baseUrl?: string;
  model?: string;
  maxTokens?: number;
  temperature?: number;
}

interface AgentConfig {
  type: string;
  enabled: boolean;
  maxConcurrent: number;
  timeout: number;
  capabilities: string[];
}

interface CompressionSettings {
  enabled: boolean;
  agentType: string;
  compressionRatio: number;
  maxContextSize: number;
}

interface PASystemConfig {
  maxAgents: number;
  taskTimeoutSeconds: number;
  enableRealTimeUpdates: boolean;
  autoAssignTasks: boolean;
  parallelExecution: boolean;
}

interface SettingsPanelProps {
  onClose: () => void;
}

const SettingsPanel: React.FC<SettingsPanelProps> = ({ onClose }) => {
  const [activeTab, setActiveTab] = useState<'general' | 'ai-providers' | 'agents' | 'pa-system' | 'compression' | 'advanced'>('general');
  const [loading, setLoading] = useState(false);
  
  // Settings state
  const [interactionMode, setInteractionMode] = useState<'Easy' | 'Interactive' | 'Manual'>('Interactive');
  const [aiProviders, setAiProviders] = useState<AIProviderConfig[]>([
    { name: 'OpenAI GPT-4', enabled: true, model: 'gpt-4', maxTokens: 8192, temperature: 0.7 },
    { name: 'Anthropic Claude', enabled: false, model: 'claude-3-sonnet', maxTokens: 4096, temperature: 0.5 },
    { name: 'Google Gemini', enabled: false, model: 'gemini-pro', maxTokens: 2048, temperature: 0.6 },
    { name: 'DeepSeek Coder', enabled: false, model: 'deepseek-coder', maxTokens: 4096, temperature: 0.3 },
  ]);
  
  const [agentConfigs, setAgentConfigs] = useState<AgentConfig[]>([
    { type: 'CodeGeneration', enabled: true, maxConcurrent: 3, timeout: 300, capabilities: ['typescript', 'react', 'rust'] },
    { type: 'Testing', enabled: true, maxConcurrent: 2, timeout: 180, capabilities: ['jest', 'playwright', 'unit-testing'] },
    { type: 'Documentation', enabled: false, maxConcurrent: 1, timeout: 120, capabilities: ['markdown', 'api-docs'] },
    { type: 'Security', enabled: false, maxConcurrent: 1, timeout: 240, capabilities: ['vulnerability-scan', 'code-analysis'] },
  ]);
  
  const [paSystemConfig, setPaSystemConfig] = useState<PASystemConfig>({
    maxAgents: 5,
    taskTimeoutSeconds: 600,
    enableRealTimeUpdates: true,
    autoAssignTasks: true,
    parallelExecution: true,
  });
  
  const [compressionSettings, setCompressionSettings] = useState<CompressionSettings>({
    enabled: true,
    agentType: 'CodeGeneration',
    compressionRatio: 0.7,
    maxContextSize: 8192,
  });

  useEffect(() => {
    loadCurrentSettings();
  }, []);

  const loadCurrentSettings = async () => {
    setLoading(true);
    try {
      // Load current settings from backend
      const compressionStats = await invoke<string>('get_compression_stats', { agentType: 'CodeGeneration' });
      console.log('Current compression stats:', compressionStats);
      
      // Load PA system status
      try {
        const paStatus = await invoke('get_pa_status');
        console.log('PA System status:', paStatus);
      } catch (error) {
        console.log('PA System not initialized yet');
      }
      
    } catch (error) {
      console.error('Failed to load settings:', error);
    } finally {
      setLoading(false);
    }
  };

  const saveSettings = async () => {
    setLoading(true);
    try {
      // Save interaction mode
      await invoke('switch_interaction_mode', { mode: interactionMode });
      
      // Save compression settings
      await invoke('update_compression_settings', { 
        settingsJson: JSON.stringify(compressionSettings) 
      });
      
      // Initialize PA system with new config
      await invoke('initialize_pa_system', { 
        request: { config: paSystemConfig } 
      });
      
      console.log('Settings saved successfully');
      alert('Settings saved successfully!');
    } catch (error) {
      console.error('Failed to save settings:', error);
      alert('Failed to save settings. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const testAIProvider = async (provider: AIProviderConfig) => {
    try {
      const response = await invoke('call_ai_provider', {
        request: {
          provider: provider.name,
          model: provider.model,
          prompt: 'Hello, this is a test message.',
          maxTokens: 100,
          temperature: provider.temperature,
        }
      });
      console.log('AI Provider test response:', response);
      alert(`${provider.name} test successful!`);
    } catch (error) {
      console.error('AI Provider test failed:', error);
      alert(`${provider.name} test failed: ${error}`);
    }
  };

  const activateAgent = async (agentType: string) => {
    try {
      const agentId = await invoke<string>('activate_agent', { agentType });
      console.log('Agent activated:', agentId);
      alert(`${agentType} agent activated with ID: ${agentId}`);
    } catch (error) {
      console.error('Failed to activate agent:', error);
      alert(`Failed to activate ${agentType} agent`);
    }
  };

  const runDuplicationAnalysis = async () => {
    try {
      const result = await invoke<string>('analyze_duplicates', {
        code: 'function test() { return "hello"; }\nfunction test2() { return "hello"; }',
        language: 'typescript'
      });
      console.log('Duplication analysis result:', result);
      alert(`Duplication analysis completed: ${result}`);
    } catch (error) {
      console.error('Duplication analysis failed:', error);
      alert('Duplication analysis failed');
    }
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>SymbioteIDE Settings</h2>
        <button className="close-button" onClick={onClose}>×</button>
      </div>

      <div className="settings-content">
        <div className="settings-tabs">
          <button 
            className={`tab ${activeTab === 'general' ? 'active' : ''}`}
            onClick={() => setActiveTab('general')}
          >
            General
          </button>
          <button 
            className={`tab ${activeTab === 'ai-providers' ? 'active' : ''}`}
            onClick={() => setActiveTab('ai-providers')}
          >
            AI Providers
          </button>
          <button 
            className={`tab ${activeTab === 'agents' ? 'active' : ''}`}
            onClick={() => setActiveTab('agents')}
          >
            Agents
          </button>
          <button 
            className={`tab ${activeTab === 'pa-system' ? 'active' : ''}`}
            onClick={() => setActiveTab('pa-system')}
          >
            PA System
          </button>
          <button 
            className={`tab ${activeTab === 'compression' ? 'active' : ''}`}
            onClick={() => setActiveTab('compression')}
          >
            Compression
          </button>
          <button 
            className={`tab ${activeTab === 'advanced' ? 'active' : ''}`}
            onClick={() => setActiveTab('advanced')}
          >
            Advanced
          </button>
        </div>

        <div className="settings-body">
          {activeTab === 'general' && (
            <div className="settings-section">
              <h3>General Settings</h3>
              
              <div className="setting-group">
                <label>Interaction Mode</label>
                <select 
                  value={interactionMode} 
                  onChange={(e) => setInteractionMode(e.target.value as any)}
                >
                  <option value="Easy">Easy - Automated decisions</option>
                  <option value="Interactive">Interactive - Ask for confirmation</option>
                  <option value="Manual">Manual - Full user control</option>
                </select>
                <p className="setting-description">
                  Controls how SymbioteIDE makes decisions and interacts with you.
                </p>
              </div>

              <div className="setting-group">
                <label>Theme</label>
                <select defaultValue="dark">
                  <option value="dark">Dark Theme</option>
                  <option value="light">Light Theme</option>
                  <option value="auto">Auto (System)</option>
                </select>
              </div>

              <div className="setting-group">
                <label>Auto-save</label>
                <input type="checkbox" defaultChecked />
                <span>Automatically save files when switching tabs</span>
              </div>
            </div>
          )}

          {activeTab === 'ai-providers' && (
            <div className="settings-section">
              <h3>AI Providers Configuration</h3>
              
              {aiProviders.map((provider, index) => (
                <div key={provider.name} className="provider-config">
                  <div className="provider-header">
                    <h4>{provider.name}</h4>
                    <div className="provider-controls">
                      <label className="switch">
                        <input 
                          type="checkbox" 
                          checked={provider.enabled}
                          onChange={(e) => {
                            const updated = [...aiProviders];
                            updated[index].enabled = e.target.checked;
                            setAiProviders(updated);
                          }}
                        />
                        <span className="slider"></span>
                      </label>
                      <button 
                        className="test-button"
                        onClick={() => testAIProvider(provider)}
                        disabled={!provider.enabled}
                      >
                        Test
                      </button>
                    </div>
                  </div>
                  
                  {provider.enabled && (
                    <div className="provider-details">
                      <div className="setting-row">
                        <label>Model</label>
                        <input 
                          type="text" 
                          value={provider.model || ''} 
                          onChange={(e) => {
                            const updated = [...aiProviders];
                            updated[index].model = e.target.value;
                            setAiProviders(updated);
                          }}
                        />
                      </div>
                      <div className="setting-row">
                        <label>Max Tokens</label>
                        <input 
                          type="number" 
                          value={provider.maxTokens || 4096} 
                          onChange={(e) => {
                            const updated = [...aiProviders];
                            updated[index].maxTokens = parseInt(e.target.value);
                            setAiProviders(updated);
                          }}
                        />
                      </div>
                      <div className="setting-row">
                        <label>Temperature</label>
                        <input 
                          type="range" 
                          min="0" 
                          max="1" 
                          step="0.1" 
                          value={provider.temperature || 0.7}
                          onChange={(e) => {
                            const updated = [...aiProviders];
                            updated[index].temperature = parseFloat(e.target.value);
                            setAiProviders(updated);
                          }}
                        />
                        <span>{provider.temperature}</span>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {activeTab === 'agents' && (
            <div className="settings-section">
              <h3>Agent System Configuration</h3>
              
              {agentConfigs.map((agent, index) => (
                <div key={agent.type} className="agent-config">
                  <div className="agent-header">
                    <h4>{agent.type} Agent</h4>
                    <div className="agent-controls">
                      <label className="switch">
                        <input 
                          type="checkbox" 
                          checked={agent.enabled}
                          onChange={(e) => {
                            const updated = [...agentConfigs];
                            updated[index].enabled = e.target.checked;
                            setAgentConfigs(updated);
                          }}
                        />
                        <span className="slider"></span>
                      </label>
                      <button 
                        className="activate-button"
                        onClick={() => activateAgent(agent.type)}
                        disabled={!agent.enabled}
                      >
                        Activate
                      </button>
                    </div>
                  </div>
                  
                  {agent.enabled && (
                    <div className="agent-details">
                      <div className="setting-row">
                        <label>Max Concurrent</label>
                        <input 
                          type="number" 
                          value={agent.maxConcurrent} 
                          onChange={(e) => {
                            const updated = [...agentConfigs];
                            updated[index].maxConcurrent = parseInt(e.target.value);
                            setAgentConfigs(updated);
                          }}
                        />
                      </div>
                      <div className="setting-row">
                        <label>Timeout (seconds)</label>
                        <input 
                          type="number" 
                          value={agent.timeout} 
                          onChange={(e) => {
                            const updated = [...agentConfigs];
                            updated[index].timeout = parseInt(e.target.value);
                            setAgentConfigs(updated);
                          }}
                        />
                      </div>
                      <div className="setting-row">
                        <label>Capabilities</label>
                        <div className="capabilities-list">
                          {agent.capabilities.map(cap => (
                            <span key={cap} className="capability-tag">{cap}</span>
                          ))}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {activeTab === 'pa-system' && (
            <div className="settings-section">
              <h3>PA (Personal Assistant) System</h3>
              
              <div className="setting-group">
                <label>Max Agents</label>
                <input 
                  type="number" 
                  value={paSystemConfig.maxAgents}
                  onChange={(e) => setPaSystemConfig(prev => ({
                    ...prev,
                    maxAgents: parseInt(e.target.value)
                  }))}
                />
                <p className="setting-description">
                  Maximum number of agents that can run simultaneously
                </p>
              </div>

              <div className="setting-group">
                <label>Task Timeout (seconds)</label>
                <input 
                  type="number" 
                  value={paSystemConfig.taskTimeoutSeconds}
                  onChange={(e) => setPaSystemConfig(prev => ({
                    ...prev,
                    taskTimeoutSeconds: parseInt(e.target.value)
                  }))}
                />
              </div>

              <div className="setting-group">
                <label>Real-time Updates</label>
                <input 
                  type="checkbox" 
                  checked={paSystemConfig.enableRealTimeUpdates}
                  onChange={(e) => setPaSystemConfig(prev => ({
                    ...prev,
                    enableRealTimeUpdates: e.target.checked
                  }))}
                />
                <span>Enable WebSocket real-time updates</span>
              </div>

              <div className="setting-group">
                <label>Auto-assign Tasks</label>
                <input 
                  type="checkbox" 
                  checked={paSystemConfig.autoAssignTasks}
                  onChange={(e) => setPaSystemConfig(prev => ({
                    ...prev,
                    autoAssignTasks: e.target.checked
                  }))}
                />
                <span>Automatically assign tasks to available agents</span>
              </div>

              <div className="setting-group">
                <label>Parallel Execution</label>
                <input 
                  type="checkbox" 
                  checked={paSystemConfig.parallelExecution}
                  onChange={(e) => setPaSystemConfig(prev => ({
                    ...prev,
                    parallelExecution: e.target.checked
                  }))}
                />
                <span>Allow multiple agents to work on tasks simultaneously</span>
              </div>
            </div>
          )}

          {activeTab === 'compression' && (
            <div className="settings-section">
              <h3>Context Compression Settings</h3>
              
              <div className="setting-group">
                <label>Enable Compression</label>
                <input 
                  type="checkbox" 
                  checked={compressionSettings.enabled}
                  onChange={(e) => setCompressionSettings(prev => ({
                    ...prev,
                    enabled: e.target.checked
                  }))}
                />
                <span>Enable intelligent context compression for agents</span>
              </div>

              <div className="setting-group">
                <label>Agent Type</label>
                <select 
                  value={compressionSettings.agentType}
                  onChange={(e) => setCompressionSettings(prev => ({
                    ...prev,
                    agentType: e.target.value
                  }))}
                >
                  <option value="CodeGeneration">Code Generation</option>
                  <option value="Testing">Testing</option>
                  <option value="Documentation">Documentation</option>
                  <option value="Security">Security</option>
                </select>
              </div>

              <div className="setting-group">
                <label>Compression Ratio</label>
                <input 
                  type="range" 
                  min="0.1" 
                  max="0.9" 
                  step="0.1" 
                  value={compressionSettings.compressionRatio}
                  onChange={(e) => setCompressionSettings(prev => ({
                    ...prev,
                    compressionRatio: parseFloat(e.target.value)
                  }))}
                />
                <span>{compressionSettings.compressionRatio}</span>
                <p className="setting-description">
                  Higher values = more compression (less context preserved)
                </p>
              </div>

              <div className="setting-group">
                <label>Max Context Size</label>
                <input 
                  type="number" 
                  value={compressionSettings.maxContextSize}
                  onChange={(e) => setCompressionSettings(prev => ({
                    ...prev,
                    maxContextSize: parseInt(e.target.value)
                  }))}
                />
                <span>tokens</span>
              </div>
            </div>
          )}

          {activeTab === 'advanced' && (
            <div className="settings-section">
              <h3>Advanced Features</h3>
              
              <div className="setting-group">
                <label>Anti-Duplication Engine</label>
                <button 
                  className="test-button"
                  onClick={runDuplicationAnalysis}
                >
                  Run Duplication Analysis
                </button>
                <p className="setting-description">
                  Analyze code for duplicate patterns and suggest refactoring opportunities
                </p>
              </div>

              <div className="setting-group">
                <label>Hybrid Codebase Intelligence</label>
                <button className="test-button">
                  Reindex Codebase
                </button>
                <p className="setting-description">
                  Rebuild vector store and knowledge graph for improved AI context
                </p>
              </div>

              <div className="setting-group">
                <label>Debug Mode</label>
                <input type="checkbox" />
                <span>Enable verbose logging and debug information</span>
              </div>

              <div className="setting-group">
                <label>Performance Monitoring</label>
                <input type="checkbox" defaultChecked />
                <span>Track agent performance and system metrics</span>
              </div>
            </div>
          )}
        </div>

        <div className="settings-footer">
          <button 
            className="save-button" 
            onClick={saveSettings}
            disabled={loading}
          >
            {loading ? 'Saving...' : 'Save Settings'}
          </button>
          <button className="cancel-button" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
};

export default SettingsPanel;
