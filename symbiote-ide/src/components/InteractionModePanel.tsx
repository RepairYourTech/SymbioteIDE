import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './InteractionModePanel.css';

// Types for interaction modes
interface InteractionMode {
  id: 'easy' | 'interactive' | 'manual';
  name: string;
  description: string;
  icon: string;
  features: string[];
  aiAutomation: number; // 0-100%
  userControl: number; // 0-100%
  complexity: 'beginner' | 'intermediate' | 'advanced';
}

interface ModeSettings {
  autoApprove: boolean;
  confirmations: boolean;
  explanations: boolean;
  stepByStep: boolean;
  rollbackEnabled: boolean;
  qualityGates: boolean;
}

interface InteractionModePanelProps {
  currentMode: 'easy' | 'interactive' | 'manual';
  onModeChange: (mode: 'easy' | 'interactive' | 'manual') => void;
  onSettingsChange: (settings: ModeSettings) => void;
}

const InteractionModePanel: React.FC<InteractionModePanelProps> = ({
  currentMode,
  onModeChange,
  onSettingsChange
}) => {
  const [selectedMode, setSelectedMode] = useState<'easy' | 'interactive' | 'manual'>(currentMode);
  const [modeSettings, setModeSettings] = useState<ModeSettings>({
    autoApprove: true,
    confirmations: false,
    explanations: true,
    stepByStep: false,
    rollbackEnabled: true,
    qualityGates: true
  });
  const [isTransitioning, setIsTransitioning] = useState(false);
  const [showAdvancedSettings, setShowAdvancedSettings] = useState(false);

  // Define interaction modes
  const interactionModes: InteractionMode[] = [
    {
      id: 'easy',
      name: 'Easy Mode',
      description: 'AI handles everything automatically with minimal user input. Perfect for beginners and rapid prototyping.',
      icon: '🚀',
      features: [
        'Full AI automation',
        'Automatic code generation',
        'Smart error recovery',
        'Minimal confirmations',
        'Built-in best practices',
        'Auto-optimization'
      ],
      aiAutomation: 95,
      userControl: 20,
      complexity: 'beginner'
    },
    {
      id: 'interactive',
      name: 'Interactive Mode',
      description: 'Collaborative development with AI. Balanced control between user decisions and AI assistance.',
      icon: '🤝',
      features: [
        'Collaborative workflow',
        'Step-by-step guidance',
        'Smart suggestions',
        'User confirmations',
        'Explanation mode',
        'Flexible control'
      ],
      aiAutomation: 70,
      userControl: 60,
      complexity: 'intermediate'
    },
    {
      id: 'manual',
      name: 'Manual Mode',
      description: 'Full user control with AI as an assistant. Maximum flexibility and customization.',
      icon: '⚙️',
      features: [
        'Full user control',
        'AI as assistant only',
        'Manual approvals',
        'Custom workflows',
        'Advanced settings',
        'Expert features'
      ],
      aiAutomation: 30,
      userControl: 95,
      complexity: 'advanced'
    }
  ];

  // Update settings based on mode
  useEffect(() => {
    const mode = interactionModes.find(m => m.id === selectedMode);
    if (mode) {
      switch (mode.id) {
        case 'easy':
          setModeSettings({
            autoApprove: true,
            confirmations: false,
            explanations: true,
            stepByStep: false,
            rollbackEnabled: true,
            qualityGates: true
          });
          break;
        case 'interactive':
          setModeSettings({
            autoApprove: false,
            confirmations: true,
            explanations: true,
            stepByStep: true,
            rollbackEnabled: true,
            qualityGates: true
          });
          break;
        case 'manual':
          setModeSettings({
            autoApprove: false,
            confirmations: true,
            explanations: false,
            stepByStep: false,
            rollbackEnabled: true,
            qualityGates: false
          });
          break;
      }
    }
  }, [selectedMode]);

  const handleModeSwitch = async (modeId: 'easy' | 'interactive' | 'manual') => {
    if (modeId === currentMode) return;

    setIsTransitioning(true);
    try {
      // Call real Tauri backend to switch interaction mode
      const capitalizedMode = modeId.charAt(0).toUpperCase() + modeId.slice(1);
      const result = await invoke('switch_interaction_mode', { mode: capitalizedMode });
      
      setSelectedMode(modeId);
      onModeChange(modeId);
      
      console.log('Mode switch result:', result);
    } catch (error) {
      console.error('Failed to switch interaction mode:', error);
    } finally {
      setIsTransitioning(false);
    }
  };

  const handleSettingsUpdate = (newSettings: Partial<ModeSettings>) => {
    const updatedSettings = { ...modeSettings, ...newSettings };
    setModeSettings(updatedSettings);
    onSettingsChange(updatedSettings);
  };

  const currentModeData = interactionModes.find(m => m.id === selectedMode);

  return (
    <div className="interaction-mode-panel">
      <div className="panel-header">
        <h3>🎯 Interaction Mode</h3>
        <p>Choose how you want to work with AI</p>
      </div>

      {/* Mode Selection Cards */}
      <div className="mode-selection">
        {interactionModes.map((mode) => (
          <div
            key={mode.id}
            className={`mode-card ${selectedMode === mode.id ? 'active' : ''} ${mode.complexity}`}
            onClick={() => handleModeSwitch(mode.id)}
          >
            <div className="mode-header">
              <span className="mode-icon">{mode.icon}</span>
              <div className="mode-info">
                <h4>{mode.name}</h4>
                <span className="complexity-badge">{mode.complexity}</span>
              </div>
              {selectedMode === mode.id && (
                <span className="active-indicator">✓</span>
              )}
            </div>

            <p className="mode-description">{mode.description}</p>

            <div className="mode-metrics">
              <div className="metric">
                <label>AI Automation</label>
                <div className="metric-bar">
                  <div 
                    className="metric-fill ai-automation"
                    style={{ width: `${mode.aiAutomation}%` }}
                  />
                </div>
                <span className="metric-value">{mode.aiAutomation}%</span>
              </div>

              <div className="metric">
                <label>User Control</label>
                <div className="metric-bar">
                  <div 
                    className="metric-fill user-control"
                    style={{ width: `${mode.userControl}%` }}
                  />
                </div>
                <span className="metric-value">{mode.userControl}%</span>
              </div>
            </div>

            <div className="mode-features">
              <h5>Key Features</h5>
              <ul>
                {mode.features.slice(0, 3).map((feature, index) => (
                  <li key={index}>{feature}</li>
                ))}
                {mode.features.length > 3 && (
                  <li className="more-features">+{mode.features.length - 3} more</li>
                )}
              </ul>
            </div>
          </div>
        ))}
      </div>

      {/* Current Mode Status */}
      {currentModeData && (
        <div className="current-mode-status">
          <div className="status-header">
            <h4>
              {currentModeData.icon} Current Mode: {currentModeData.name}
            </h4>
            <button
              onClick={() => setShowAdvancedSettings(!showAdvancedSettings)}
              className="settings-toggle"
            >
              ⚙️ Settings
            </button>
          </div>

          {showAdvancedSettings && (
            <div className="advanced-settings">
              <h5>Mode Configuration</h5>
              
              <div className="settings-grid">
                <div className="setting-item">
                  <label>
                    <input
                      type="checkbox"
                      checked={modeSettings.autoApprove}
                      onChange={(e) => handleSettingsUpdate({ autoApprove: e.target.checked })}
                    />
                    Auto-approve AI actions
                  </label>
                  <span className="setting-description">
                    Automatically execute AI suggestions without confirmation
                  </span>
                </div>

                <div className="setting-item">
                  <label>
                    <input
                      type="checkbox"
                      checked={modeSettings.confirmations}
                      onChange={(e) => handleSettingsUpdate({ confirmations: e.target.checked })}
                    />
                    Show confirmations
                  </label>
                  <span className="setting-description">
                    Ask for confirmation before major actions
                  </span>
                </div>

                <div className="setting-item">
                  <label>
                    <input
                      type="checkbox"
                      checked={modeSettings.explanations}
                      onChange={(e) => handleSettingsUpdate({ explanations: e.target.checked })}
                    />
                    Show explanations
                  </label>
                  <span className="setting-description">
                    Provide detailed explanations for AI actions
                  </span>
                </div>

                <div className="setting-item">
                  <label>
                    <input
                      type="checkbox"
                      checked={modeSettings.stepByStep}
                      onChange={(e) => handleSettingsUpdate({ stepByStep: e.target.checked })}
                    />
                    Step-by-step mode
                  </label>
                  <span className="setting-description">
                    Break down complex tasks into individual steps
                  </span>
                </div>

                <div className="setting-item">
                  <label>
                    <input
                      type="checkbox"
                      checked={modeSettings.rollbackEnabled}
                      onChange={(e) => handleSettingsUpdate({ rollbackEnabled: e.target.checked })}
                    />
                    Enable rollback
                  </label>
                  <span className="setting-description">
                    Allow undoing AI actions with smart rollback
                  </span>
                </div>

                <div className="setting-item">
                  <label>
                    <input
                      type="checkbox"
                      checked={modeSettings.qualityGates}
                      onChange={(e) => handleSettingsUpdate({ qualityGates: e.target.checked })}
                    />
                    Quality gates
                  </label>
                  <span className="setting-description">
                    Enforce quality standards and best practices
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Mode Benefits */}
      <div className="mode-benefits">
        <h4>💡 Why Choose This Mode?</h4>
        <div className="benefits-grid">
          {selectedMode === 'easy' && (
            <>
              <div className="benefit-item">
                <span className="benefit-icon">⚡</span>
                <div>
                  <h5>Rapid Development</h5>
                  <p>Get from idea to working code in minutes</p>
                </div>
              </div>
              <div className="benefit-item">
                <span className="benefit-icon">🛡️</span>
                <div>
                  <h5>Error Prevention</h5>
                  <p>AI prevents common mistakes automatically</p>
                </div>
              </div>
              <div className="benefit-item">
                <span className="benefit-icon">📚</span>
                <div>
                  <h5>Learning Mode</h5>
                  <p>Perfect for beginners to learn best practices</p>
                </div>
              </div>
            </>
          )}

          {selectedMode === 'interactive' && (
            <>
              <div className="benefit-item">
                <span className="benefit-icon">🤝</span>
                <div>
                  <h5>Collaborative</h5>
                  <p>Work together with AI as your pair programmer</p>
                </div>
              </div>
              <div className="benefit-item">
                <span className="benefit-icon">🎯</span>
                <div>
                  <h5>Balanced Control</h5>
                  <p>Perfect mix of automation and user control</p>
                </div>
              </div>
              <div className="benefit-item">
                <span className="benefit-icon">💡</span>
                <div>
                  <h5>Educational</h5>
                  <p>Learn while building with detailed explanations</p>
                </div>
              </div>
            </>
          )}

          {selectedMode === 'manual' && (
            <>
              <div className="benefit-item">
                <span className="benefit-icon">⚙️</span>
                <div>
                  <h5>Full Control</h5>
                  <p>Complete control over every aspect of development</p>
                </div>
              </div>
              <div className="benefit-item">
                <span className="benefit-icon">🔧</span>
                <div>
                  <h5>Customizable</h5>
                  <p>Tailor the AI behavior to your exact needs</p>
                </div>
              </div>
              <div className="benefit-item">
                <span className="benefit-icon">🎓</span>
                <div>
                  <h5>Expert Mode</h5>
                  <p>For experienced developers who want precision</p>
                </div>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Transition Overlay */}
      {isTransitioning && (
        <div className="transition-overlay">
          <div className="transition-content">
            <div className="transition-spinner">⚙️</div>
            <h4>Switching to {selectedMode} mode...</h4>
            <p>Configuring AI behavior and interface</p>
          </div>
        </div>
      )}
    </div>
  );
};

export default InteractionModePanel;
