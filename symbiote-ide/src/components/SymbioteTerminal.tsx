import React, { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SymbioteTerminal.css';

// Types for Symbiote Terminal
interface TerminalSession {
  id: string;
  name: string;
  working_directory: string;
  shell_type: 'PowerShell' | 'Cmd' | 'Bash' | 'Zsh' | 'Fish' | 'Nushell';
  status: 'Active' | 'Idle' | 'Busy' | 'Error' | 'Terminated';
  created_at: string;
  last_activity: string;
}

interface CommandHistoryEntry {
  id: string;
  session_id: string;
  original_input: string;
  interpreted_command?: string;
  natural_language: boolean;
  execution_time_ms: number;
  exit_code?: number;
  output: string;
  error_output: string;
  timestamp: string;
  ai_suggestions: string[];
}

interface TerminalWorkflow {
  id: string;
  name: string;
  description: string;
  commands: WorkflowCommand[];
  created_at: string;
}

interface WorkflowCommand {
  command: string;
  natural_language_description: string;
  timeout_seconds: number;
}

interface SymbioteTerminalProps {
  onCommandExecute?: (command: string, session_id: string) => void;
  onWorkflowCreate?: (workflow: TerminalWorkflow) => void;
}

const SymbioteTerminal: React.FC<SymbioteTerminalProps> = ({
  onCommandExecute,
  onWorkflowCreate
}) => {
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [commandHistory, setCommandHistory] = useState<CommandHistoryEntry[]>([]);
  const [workflows, setWorkflows] = useState<TerminalWorkflow[]>([]);
  const [currentInput, setCurrentInput] = useState('');
  const [isNaturalLanguage, setIsNaturalLanguage] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [showWorkflows, setShowWorkflows] = useState(false);
  const [showAIAssistant, setShowAIAssistant] = useState(true);
  const [aiSuggestions, setAiSuggestions] = useState<string[]>([]);

  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Initialize terminal
  useEffect(() => {
    initializeTerminal();
  }, []);

  // Auto-scroll to bottom when new output appears
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commandHistory]);

  // Focus input when component mounts
  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, [activeSessionId]);

  const initializeTerminal = async () => {
    try {
      // Create default session
      const sessionId = await invoke('create_terminal_session', {
        name: 'Main Terminal',
        shell_type: getDefaultShellType()
      });

      const session: TerminalSession = {
        id: sessionId as string,
        name: 'Main Terminal',
        working_directory: await getCurrentDirectory(),
        shell_type: getDefaultShellType(),
        status: 'Active',
        created_at: new Date().toISOString(),
        last_activity: new Date().toISOString()
      };

      setSessions([session]);
      setActiveSessionId(session.id);
      
      // Load existing workflows
      loadWorkflows();
    } catch (error) {
      console.error('Failed to initialize terminal:', error);
    }
  };

  const getDefaultShellType = (): 'PowerShell' | 'Cmd' | 'Bash' | 'Zsh' | 'Fish' | 'Nushell' => {
    // Detect platform and return appropriate shell
    if (navigator.platform.includes('Win')) {
      return 'PowerShell';
    } else if (navigator.platform.includes('Mac')) {
      return 'Zsh';
    } else {
      return 'Bash';
    }
  };

  const getCurrentDirectory = async (): Promise<string> => {
    try {
      // This would be implemented as a Tauri command
      return 'C:\\Users\\User\\Projects'; // Mock for now
    } catch {
      return '/home/user/projects';
    }
  };

  const loadWorkflows = async () => {
    try {
      const workflowsData = await invoke('get_terminal_workflows');
      setWorkflows(workflowsData as TerminalWorkflow[]);
    } catch (error) {
      console.error('Failed to load workflows:', error);
    }
  };

  const executeCommand = async () => {
    if (!currentInput.trim() || !activeSessionId || isExecuting) return;

    setIsExecuting(true);
    const command = currentInput.trim();
    setCurrentInput('');

    try {
      // Add command to display immediately
      const tempEntry: CommandHistoryEntry = {
        id: 'temp-' + Date.now(),
        session_id: activeSessionId,
        original_input: command,
        natural_language: isNaturalLanguage,
        execution_time_ms: 0,
        output: '',
        error_output: '',
        timestamp: new Date().toISOString(),
        ai_suggestions: []
      };

      setCommandHistory(prev => [...prev, tempEntry]);

      // Execute command through backend
      const result = await invoke('execute_terminal_command', {
        session_id: activeSessionId,
        input: command,
        natural_language: isNaturalLanguage
      }) as CommandHistoryEntry;

      // Update command history with real result
      setCommandHistory(prev => 
        prev.map(entry => 
          entry.id === tempEntry.id ? result : entry
        )
      );

      // Update AI suggestions
      setAiSuggestions(result.ai_suggestions);

      // Notify parent component
      onCommandExecute?.(command, activeSessionId);

    } catch (error) {
      console.error('Failed to execute command:', error);
      
      // Update with error
      setCommandHistory(prev => 
        prev.map(entry => 
          entry.id.startsWith('temp-') 
            ? { ...entry, error_output: `Error: ${error}`, exit_code: 1 }
            : entry
        )
      );
    } finally {
      setIsExecuting(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      executeCommand();
    } else if (e.key === 'ArrowUp') {
      // Navigate command history
      e.preventDefault();
      const lastCommand = commandHistory
        .filter(entry => entry.session_id === activeSessionId)
        .slice(-1)[0];
      if (lastCommand) {
        setCurrentInput(lastCommand.original_input);
      }
    } else if (e.key === 'Tab') {
      // Auto-complete (future enhancement)
      e.preventDefault();
    }
  };

  const createNewSession = async () => {
    try {
      const sessionName = `Terminal ${sessions.length + 1}`;
      const sessionId = await invoke('create_terminal_session', {
        name: sessionName,
        shell_type: getDefaultShellType()
      });

      const newSession: TerminalSession = {
        id: sessionId as string,
        name: sessionName,
        working_directory: await getCurrentDirectory(),
        shell_type: getDefaultShellType(),
        status: 'Active',
        created_at: new Date().toISOString(),
        last_activity: new Date().toISOString()
      };

      setSessions(prev => [...prev, newSession]);
      setActiveSessionId(newSession.id);
    } catch (error) {
      console.error('Failed to create session:', error);
    }
  };

  const createWorkflowFromHistory = async () => {
    if (!activeSessionId) return;

    try {
      const sessionHistory = commandHistory.filter(entry => 
        entry.session_id === activeSessionId && entry.exit_code === 0
      );

      if (sessionHistory.length === 0) {
        alert('No successful commands to create workflow from');
        return;
      }

      const workflowName = prompt('Enter workflow name:');
      if (!workflowName) return;

      const workflowDescription = prompt('Enter workflow description:') || '';

      const workflowId = await invoke('create_terminal_workflow', {
        name: workflowName,
        description: workflowDescription,
        command_ids: sessionHistory.map(entry => entry.id)
      });

      // Reload workflows
      loadWorkflows();
      
      console.log('Created workflow:', workflowId);
    } catch (error) {
      console.error('Failed to create workflow:', error);
    }
  };

  const executeWorkflow = async (workflowId: string) => {
    if (!activeSessionId) return;

    try {
      setIsExecuting(true);
      
      const results = await invoke('execute_terminal_workflow', {
        workflow_id: workflowId,
        session_id: activeSessionId
      }) as CommandHistoryEntry[];

      // Add workflow results to history
      setCommandHistory(prev => [...prev, ...results]);
      
    } catch (error) {
      console.error('Failed to execute workflow:', error);
    } finally {
      setIsExecuting(false);
    }
  };

  const getSessionHistory = () => {
    return commandHistory.filter(entry => entry.session_id === activeSessionId);
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  const activeSession = sessions.find(s => s.id === activeSessionId);

  return (
    <div className="symbiote-terminal">
      {/* Header */}
      <div className="terminal-header">
        <div className="header-left">
          <h3>🚀 Symbiote Terminal</h3>
          <span className="terminal-subtitle">AI-Enhanced Terminal</span>
        </div>
        
        <div className="header-controls">
          <button
            onClick={() => setShowAIAssistant(!showAIAssistant)}
            className={`control-button ${showAIAssistant ? 'active' : ''}`}
            title="Toggle AI Assistant"
          >
            🤖 AI
          </button>
          <button
            onClick={() => setShowWorkflows(!showWorkflows)}
            className={`control-button ${showWorkflows ? 'active' : ''}`}
            title="Show Workflows"
          >
            📋 Workflows
          </button>
          <button
            onClick={createNewSession}
            className="control-button"
            title="New Session"
          >
            ➕ New
          </button>
        </div>
      </div>

      <div className="terminal-content">
        {/* Session Tabs */}
        <div className="session-tabs">
          {sessions.map((session) => (
            <div
              key={session.id}
              className={`session-tab ${activeSessionId === session.id ? 'active' : ''}`}
              onClick={() => setActiveSessionId(session.id)}
            >
              <span className="session-name">{session.name}</span>
              <span className={`session-status ${session.status.toLowerCase()}`}>
                {session.status}
              </span>
            </div>
          ))}
        </div>

        <div className="terminal-main">
          {/* Terminal Output */}
          <div className="terminal-output" ref={terminalRef}>
            {activeSession && (
              <div className="session-info">
                <div className="session-header">
                  <span className="session-path">{activeSession.working_directory}</span>
                  <span className="session-shell">{activeSession.shell_type}</span>
                </div>
              </div>
            )}

            {getSessionHistory().map((entry) => (
              <div key={entry.id} className="command-entry">
                <div className="command-input">
                  <span className="prompt">$</span>
                  <span className="command-text">{entry.original_input}</span>
                  {entry.natural_language && (
                    <span className="nl-badge">🗣️ NL</span>
                  )}
                  {entry.interpreted_command && (
                    <div className="interpreted-command">
                      → {entry.interpreted_command}
                    </div>
                  )}
                  <span className="timestamp">{formatTimestamp(entry.timestamp)}</span>
                </div>

                {entry.output && (
                  <div className="command-output">
                    <pre>{entry.output}</pre>
                  </div>
                )}

                {entry.error_output && (
                  <div className="command-error">
                    <pre>{entry.error_output}</pre>
                  </div>
                )}

                {entry.ai_suggestions.length > 0 && showAIAssistant && (
                  <div className="ai-suggestions">
                    {entry.ai_suggestions.map((suggestion, index) => (
                      <div key={index} className="ai-suggestion">
                        {suggestion}
                      </div>
                    ))}
                  </div>
                )}

                <div className="command-meta">
                  <span className="execution-time">{entry.execution_time_ms}ms</span>
                  {entry.exit_code !== undefined && (
                    <span className={`exit-code ${entry.exit_code === 0 ? 'success' : 'error'}`}>
                      Exit: {entry.exit_code}
                    </span>
                  )}
                </div>
              </div>
            ))}

            {isExecuting && (
              <div className="executing-indicator">
                <span className="spinner">⚙️</span>
                <span>Executing command...</span>
              </div>
            )}
          </div>

          {/* Command Input */}
          <div className="terminal-input">
            <div className="input-controls">
              <button
                onClick={() => setIsNaturalLanguage(!isNaturalLanguage)}
                className={`nl-toggle ${isNaturalLanguage ? 'active' : ''}`}
                title="Toggle Natural Language Mode"
              >
                {isNaturalLanguage ? '🗣️' : '💻'}
              </button>
            </div>

            <div className="input-field">
              <span className="prompt">$</span>
              <input
                ref={inputRef}
                type="text"
                value={currentInput}
                onChange={(e) => setCurrentInput(e.target.value)}
                onKeyDown={handleKeyPress}
                placeholder={isNaturalLanguage ? "Type in natural language..." : "Enter command..."}
                disabled={isExecuting}
                className="command-input-field"
              />
            </div>

            <button
              onClick={executeCommand}
              disabled={!currentInput.trim() || isExecuting}
              className="execute-button"
            >
              ▶️
            </button>
          </div>

          {/* AI Suggestions Panel */}
          {showAIAssistant && aiSuggestions.length > 0 && (
            <div className="ai-panel">
              <h4>🤖 AI Suggestions</h4>
              <div className="suggestions-list">
                {aiSuggestions.map((suggestion, index) => (
                  <div key={index} className="suggestion-item">
                    {suggestion}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Workflows Sidebar */}
        {showWorkflows && (
          <div className="workflows-sidebar">
            <div className="workflows-header">
              <h4>📋 Workflows</h4>
              <button
                onClick={createWorkflowFromHistory}
                className="create-workflow-button"
                disabled={!activeSessionId}
              >
                ➕ Create
              </button>
            </div>

            <div className="workflows-list">
              {workflows.map((workflow) => (
                <div key={workflow.id} className="workflow-item">
                  <div className="workflow-info">
                    <h5>{workflow.name}</h5>
                    <p>{workflow.description}</p>
                    <span className="workflow-commands">
                      {workflow.commands.length} commands
                    </span>
                  </div>
                  <button
                    onClick={() => executeWorkflow(workflow.id)}
                    className="execute-workflow-button"
                    disabled={isExecuting}
                  >
                    ▶️
                  </button>
                </div>
              ))}

              {workflows.length === 0 && (
                <div className="no-workflows">
                  <p>No workflows created yet.</p>
                  <p>Execute some commands and create a workflow from your history!</p>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Status Bar */}
      <div className="terminal-status">
        <span>Session: {activeSession?.name || 'None'}</span>
        <span>Shell: {activeSession?.shell_type || 'None'}</span>
        <span>Commands: {getSessionHistory().length}</span>
        <span>Mode: {isNaturalLanguage ? 'Natural Language' : 'Command Line'}</span>
      </div>
    </div>
  );
};

export default SymbioteTerminal;
