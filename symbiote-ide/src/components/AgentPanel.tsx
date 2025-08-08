import React, { useState } from 'react';

interface Agent {
  id: string;
  type: string;
  status: string;
}

interface AgentPanelProps {
  agents: Agent[];
  onCreateAgent: (agentType: string) => Promise<string>;
  onExecuteTask: (agentId: string, task: string) => Promise<string>;
}

const AgentPanel: React.FC<AgentPanelProps> = ({ agents, onCreateAgent, onExecuteTask }) => {
  const [selectedAgent, setSelectedAgent] = useState<string>('');
  const [taskInput, setTaskInput] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [taskResult, setTaskResult] = useState<string>('');

  const agentTypes = [
    { type: 'architect', name: 'Architect', icon: '⚙️', description: 'System design and architecture' },
    { type: 'developer', name: 'Developer', icon: '👨‍💻', description: 'Code implementation' },
    { type: 'tester', name: 'Tester', icon: '🧪', description: 'Testing and QA' },
    { type: 'reviewer', name: 'Reviewer', icon: '🤖', description: 'Code review and analysis' },
    { type: 'orchestrator', name: 'Orchestrator', icon: '🎯', description: 'Multi-agent coordination' },
  ];

  const handleCreateAgent = async (agentType: string) => {
    setIsCreating(true);
    try {
      await onCreateAgent(agentType);
    } catch (error) {
      console.error('Failed to create agent:', error);
    } finally {
      setIsCreating(false);
    }
  };

  const handleExecuteTask = async () => {
    if (!selectedAgent || !taskInput.trim()) return;

    setIsExecuting(true);
    try {
      const result = await onExecuteTask(selectedAgent, taskInput);
      setTaskResult(result);
      setTaskInput('');
    } catch (error) {
      console.error('Failed to execute task:', error);
      setTaskResult(`Error: ${error}`);
    } finally {
      setIsExecuting(false);
    }
  };

  const getAgentTypeInfo = (type: string) => {
    return agentTypes.find(at => at.type === type) || agentTypes[0];
  };

  return (
    <div className="agent-panel">
      <div className="panel-header">
        <h3>AI Agents</h3>
        <span className="agent-count">{agents.length}</span>
      </div>

      {/* Create Agent Section */}
      <div className="create-agent-section">
        <h4>Create Agent</h4>
        <div className="agent-types">
          {agentTypes.map((agentType) => {
            return (
              <button
                key={agentType.type}
                className="agent-type-btn"
                onClick={() => handleCreateAgent(agentType.type)}
                disabled={isCreating}
                title={agentType.description}
              >
                <span className="agent-icon">{agentType.icon}</span>
                <span>{agentType.name}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Active Agents */}
      <div className="active-agents-section">
        <h4>Active Agents</h4>
        <div className="agents-list">
          {agents.length === 0 ? (
            <div className="no-agents">
              <span className="no-agents-icon">🤖</span>
              <p>No agents created yet</p>
            </div>
          ) : (
            agents.map((agent) => {
              const typeInfo = getAgentTypeInfo(agent.type);
              return (
                <div 
                  key={agent.id}
                  className={`agent-item ${selectedAgent === agent.id ? 'selected' : ''}`}
                  onClick={() => setSelectedAgent(agent.id)}
                >
                  <div className="agent-info">
                    <span className="agent-icon">{typeInfo.icon}</span>
                    <div className="agent-details">
                      <span className="agent-name">{typeInfo.name}</span>
                      <span className="agent-id">{agent.id.substring(0, 8)}...</span>
                    </div>
                  </div>
                  <div className={`agent-status ${agent.status}`}>
                    {agent.status}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>

      {/* Task Execution */}
      {selectedAgent && (
        <div className="task-execution-section">
          <h4>Execute Task</h4>
          <div className="task-input-container">
            <textarea
              value={taskInput}
              onChange={(e) => setTaskInput(e.target.value)}
              placeholder="Describe the task for the agent..."
              className="task-input"
              rows={3}
            />
            <button
              onClick={handleExecuteTask}
              disabled={!selectedAgent || isExecuting}
              className="execute-btn"
            >
              <span className="execute-icon">▶️</span>
              {isExecuting ? 'Executing...' : 'Execute Task'}
            </button>
          </div>
        </div>
      )}

      {/* Task Result */}
      {taskResult && (
        <div className="task-result-section">
          <h4>Task Result</h4>
          <div className="task-result">
            <pre>{taskResult}</pre>
          </div>
          <button 
            className="clear-result-btn"
            onClick={() => setTaskResult('')}
          >
            Clear
          </button>
        </div>
      )}
    </div>
  );
};

export default AgentPanel;
