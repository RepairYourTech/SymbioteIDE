import React, { useState, useEffect, useRef, useCallback } from 'react';
import './EnhancedTerminal.css';

// Types for enhanced terminal features
interface TerminalSession {
  id: string;
  name: string;
  cwd: string;
  shell: string;
  created_at: string;
  last_activity: string;
  is_active: boolean;
}

interface CommandHistory {
  id: string;
  command: string;
  natural_language?: string;
  output: string;
  exit_code: number;
  execution_time: number;
  timestamp: string;
  session_id: string;
}

interface AICommandSuggestion {
  command: string;
  description: string;
  confidence: number;
  category: 'file' | 'git' | 'npm' | 'system' | 'custom';
  parameters: {
    name: string;
    value: string;
    required: boolean;
  }[];
}

interface EnhancedTerminalProps {
  onCommandExecute?: (command: string, sessionId: string) => void;
  onSessionChange?: (sessionId: string) => void;
  workspaceRoot: string;
}

const EnhancedTerminal: React.FC<EnhancedTerminalProps> = ({
  onCommandExecute,
  onSessionChange,
  workspaceRoot
}) => {
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string>('');
  const [commandHistory, setCommandHistory] = useState<CommandHistory[]>([]);
  const [currentCommand, setCurrentCommand] = useState('');
  const [naturalLanguageInput, setNaturalLanguageInput] = useState('');
  const [isAIMode, setIsAIMode] = useState(false);
  const [aiSuggestions, setAiSuggestions] = useState<AICommandSuggestion[]>([]);
  const [isExecuting, setIsExecuting] = useState(false);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [terminalOutput, setTerminalOutput] = useState<string[]>([]);
  const [isMultiplexMode, setIsMultiplexMode] = useState(false);

  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const outputRef = useRef<HTMLDivElement>(null);

  // Initialize terminal sessions
  useEffect(() => {
    initializeTerminal();
  }, []);

  // Auto-scroll to bottom when new output is added
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [terminalOutput]);

  const initializeTerminal = async () => {
    try {
      // Mock terminal initialization - replace with actual Tauri invoke
      const defaultSession: TerminalSession = {
        id: 'session_1',
        name: 'Main Terminal',
        cwd: workspaceRoot,
        shell: 'pwsh',
        created_at: new Date().toISOString(),
        last_activity: new Date().toISOString(),
        is_active: true
      };

      setSessions([defaultSession]);
      setActiveSessionId(defaultSession.id);
      setTerminalOutput([
        `SymbioteIDE Enhanced Terminal v1.0`,
        `Working directory: ${workspaceRoot}`,
        `Shell: ${defaultSession.shell}`,
        `Type 'help' for available commands or toggle AI mode for natural language commands.`,
        ''
      ]);
    } catch (error) {
      console.error('Failed to initialize terminal:', error);
    }
  };

  const translateNaturalLanguage = useCallback(async (input: string): Promise<AICommandSuggestion[]> => {
    try {
      // Mock AI command translation - replace with actual Tauri invoke
      const mockSuggestions: AICommandSuggestion[] = [];

      // Simple pattern matching for demo
      if (input.toLowerCase().includes('list files')) {
        mockSuggestions.push({
          command: 'ls -la',
          description: 'List all files and directories with detailed information',
          confidence: 0.95,
          category: 'file',
          parameters: []
        });
      }

      if (input.toLowerCase().includes('git status')) {
        mockSuggestions.push({
          command: 'git status',
          description: 'Show the working tree status',
          confidence: 0.98,
          category: 'git',
          parameters: []
        });
      }

      if (input.toLowerCase().includes('install') && input.toLowerCase().includes('package')) {
        mockSuggestions.push({
          command: 'npm install',
          description: 'Install npm packages',
          confidence: 0.87,
          category: 'npm',
          parameters: [
            { name: 'package', value: '', required: true }
          ]
        });
      }

      if (input.toLowerCase().includes('create') && input.toLowerCase().includes('file')) {
        mockSuggestions.push({
          command: 'touch',
          description: 'Create a new file',
          confidence: 0.92,
          category: 'file',
          parameters: [
            { name: 'filename', value: '', required: true }
          ]
        });
      }

      if (input.toLowerCase().includes('find') || input.toLowerCase().includes('search')) {
        mockSuggestions.push({
          command: 'grep -r',
          description: 'Search for text in files recursively',
          confidence: 0.89,
          category: 'file',
          parameters: [
            { name: 'pattern', value: '', required: true },
            { name: 'path', value: '.', required: false }
          ]
        });
      }

      return mockSuggestions;
    } catch (error) {
      console.error('Failed to translate natural language:', error);
      return [];
    }
  }, []);

  const executeCommand = async (command: string) => {
    if (!command.trim()) return;

    setIsExecuting(true);
    const startTime = Date.now();

    try {
      // Add command to output
      setTerminalOutput(prev => [...prev, `$ ${command}`]);

      // Mock command execution - replace with actual Tauri invoke
      let output = '';
      let exitCode = 0;

      // Simple command simulation
      switch (command.toLowerCase().trim()) {
        case 'help':
          output = `SymbioteIDE Enhanced Terminal Commands:
  help                 - Show this help message
  clear               - Clear terminal output
  pwd                 - Print working directory
  ls, dir             - List directory contents
  cd <path>           - Change directory
  git status          - Show git status
  npm install         - Install npm packages
  ai on/off           - Toggle AI natural language mode
  multiplex on/off    - Toggle multiplex mode
  history             - Show command history
  sessions            - List terminal sessions`;
          break;

        case 'clear':
          setTerminalOutput([]);
          setIsExecuting(false);
          return;

        case 'pwd':
          output = workspaceRoot;
          break;

        case 'ls':
        case 'dir':
          output = `src/
dist/
node_modules/
package.json
README.md
tsconfig.json`;
          break;

        case 'git status':
          output = `On branch main
Your branch is up to date with 'origin/main'.

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
        modified:   src/components/EnhancedTerminal.tsx

no changes added to commit (use "git add ." or "git commit -a")`;
          break;

        case 'ai on':
          setIsAIMode(true);
          output = 'AI natural language mode enabled. You can now type commands in plain English.';
          break;

        case 'ai off':
          setIsAIMode(false);
          output = 'AI natural language mode disabled. Back to traditional command mode.';
          break;

        case 'multiplex on':
          setIsMultiplexMode(true);
          output = 'Multiplex mode enabled. You can now manage multiple terminal sessions.';
          break;

        case 'multiplex off':
          setIsMultiplexMode(false);
          output = 'Multiplex mode disabled.';
          break;

        case 'history':
          output = commandHistory.slice(-10).map((cmd, index) => 
            `${index + 1}  ${cmd.command}`
          ).join('\n');
          break;

        case 'sessions':
          output = sessions.map(session => 
            `${session.id} - ${session.name} (${session.shell}) ${session.is_active ? '[ACTIVE]' : ''}`
          ).join('\n');
          break;

        default:
          if (command.startsWith('cd ')) {
            const path = command.substring(3).trim();
            output = `Changed directory to: ${path}`;
          } else {
            output = `Command not found: ${command}`;
            exitCode = 1;
          }
      }

      // Add output to terminal
      if (output) {
        setTerminalOutput(prev => [...prev, output, '']);
      }

      // Add to command history
      const executionTime = Date.now() - startTime;
      const historyEntry: CommandHistory = {
        id: `cmd_${Date.now()}`,
        command,
        natural_language: isAIMode ? naturalLanguageInput : undefined,
        output,
        exit_code: exitCode,
        execution_time: executionTime,
        timestamp: new Date().toISOString(),
        session_id: activeSessionId
      };

      setCommandHistory(prev => [...prev, historyEntry]);

      // Notify parent component
      if (onCommandExecute) {
        onCommandExecute(command, activeSessionId);
      }

    } catch (error) {
      console.error('Command execution failed:', error);
      setTerminalOutput(prev => [...prev, `Error: ${error}`, '']);
    } finally {
      setIsExecuting(false);
      setCurrentCommand('');
      setNaturalLanguageInput('');
      setShowSuggestions(false);
    }
  };

  const handleNaturalLanguageInput = async (input: string) => {
    if (!input.trim()) {
      setAiSuggestions([]);
      setShowSuggestions(false);
      return;
    }

    const suggestions = await translateNaturalLanguage(input);
    setAiSuggestions(suggestions);
    setShowSuggestions(suggestions.length > 0);
  };

  const applySuggestion = (suggestion: AICommandSuggestion) => {
    setCurrentCommand(suggestion.command);
    setShowSuggestions(false);
    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  const createNewSession = async () => {
    const newSession: TerminalSession = {
      id: `session_${Date.now()}`,
      name: `Terminal ${sessions.length + 1}`,
      cwd: workspaceRoot,
      shell: 'pwsh',
      created_at: new Date().toISOString(),
      last_activity: new Date().toISOString(),
      is_active: false
    };

    setSessions(prev => [...prev, newSession]);
    switchToSession(newSession.id);
  };

  const switchToSession = (sessionId: string) => {
    setSessions(prev => prev.map(session => ({
      ...session,
      is_active: session.id === sessionId
    })));
    setActiveSessionId(sessionId);
    
    if (onSessionChange) {
      onSessionChange(sessionId);
    }
  };

  const closeSession = (sessionId: string) => {
    if (sessions.length <= 1) return; // Keep at least one session

    setSessions(prev => {
      const filtered = prev.filter(s => s.id !== sessionId);
      if (sessionId === activeSessionId && filtered.length > 0) {
        const newActive = filtered[0];
        newActive.is_active = true;
        setActiveSessionId(newActive.id);
      }
      return filtered;
    });
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      if (isAIMode && naturalLanguageInput.trim()) {
        if (aiSuggestions.length > 0) {
          executeCommand(aiSuggestions[0].command);
        }
      } else if (currentCommand.trim()) {
        executeCommand(currentCommand);
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      if (showSuggestions && aiSuggestions.length > 0) {
        applySuggestion(aiSuggestions[0]);
      }
    } else if (e.key === 'Escape') {
      setShowSuggestions(false);
    }
  };

  const activeSession = sessions.find(s => s.id === activeSessionId);

  return (
    <div className="enhanced-terminal" ref={terminalRef}>
      <div className="terminal-header">
        <div className="terminal-tabs">
          {sessions.map(session => (
            <div
              key={session.id}
              className={`terminal-tab ${session.is_active ? 'active' : ''}`}
              onClick={() => switchToSession(session.id)}
            >
              <span className="tab-name">{session.name}</span>
              <span className="tab-shell">({session.shell})</span>
              {sessions.length > 1 && (
                <button
                  className="close-tab"
                  onClick={(e) => {
                    e.stopPropagation();
                    closeSession(session.id);
                  }}
                >
                  ×
                </button>
              )}
            </div>
          ))}
          
          {isMultiplexMode && (
            <button className="new-tab" onClick={createNewSession}>
              + New Terminal
            </button>
          )}
        </div>

        <div className="terminal-controls">
          <button
            className={`ai-toggle ${isAIMode ? 'active' : ''}`}
            onClick={() => setIsAIMode(!isAIMode)}
            title="Toggle AI Natural Language Mode"
          >
            🤖 AI
          </button>
          
          <button
            className={`multiplex-toggle ${isMultiplexMode ? 'active' : ''}`}
            onClick={() => setIsMultiplexMode(!isMultiplexMode)}
            title="Toggle Multiplex Mode"
          >
            📋 Multiplex
          </button>
          
          <button
            className="clear-terminal"
            onClick={() => setTerminalOutput([])}
            title="Clear Terminal"
          >
            🗑️ Clear
          </button>
        </div>
      </div>

      <div className="terminal-body">
        <div className="terminal-output" ref={outputRef}>
          {terminalOutput.map((line, index) => (
            <div key={index} className="output-line">
              {line}
            </div>
          ))}
          
          {isExecuting && (
            <div className="output-line executing">
              <span className="spinner">⏳</span> Executing...
            </div>
          )}
        </div>

        <div className="terminal-input-area">
          {isAIMode ? (
            <div className="ai-input-section">
              <div className="input-line">
                <span className="prompt">🤖 AI:</span>
                <input
                  type="text"
                  value={naturalLanguageInput}
                  onChange={(e) => {
                    setNaturalLanguageInput(e.target.value);
                    handleNaturalLanguageInput(e.target.value);
                  }}
                  onKeyPress={handleKeyPress}
                  placeholder="Describe what you want to do in plain English..."
                  className="natural-language-input"
                  disabled={isExecuting}
                />
              </div>

              {showSuggestions && aiSuggestions.length > 0 && (
                <div className="ai-suggestions">
                  <div className="suggestions-header">💡 AI Suggestions:</div>
                  {aiSuggestions.map((suggestion, index) => (
                    <div
                      key={index}
                      className="suggestion-item"
                      onClick={() => applySuggestion(suggestion)}
                    >
                      <div className="suggestion-command">
                        <code>{suggestion.command}</code>
                        <span className="confidence">
                          {(suggestion.confidence * 100).toFixed(0)}%
                        </span>
                      </div>
                      <div className="suggestion-description">
                        {suggestion.description}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className="command-input-section">
              <div className="input-line">
                <span className="prompt">
                  {activeSession?.cwd.split('/').pop() || activeSession?.cwd.split('\\').pop() || '$'}:
                </span>
                <input
                  ref={inputRef}
                  type="text"
                  value={currentCommand}
                  onChange={(e) => setCurrentCommand(e.target.value)}
                  onKeyPress={handleKeyPress}
                  placeholder="Enter command..."
                  className="command-input"
                  disabled={isExecuting}
                />
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="terminal-status">
        <div className="status-left">
          <span className="session-info">
            Session: {activeSession?.name} | Shell: {activeSession?.shell}
          </span>
        </div>
        
        <div className="status-right">
          <span className="mode-indicator">
            {isAIMode ? '🤖 AI Mode' : '💻 Command Mode'}
          </span>
          {isMultiplexMode && (
            <span className="multiplex-indicator">
              📋 Multiplex ({sessions.length} sessions)
            </span>
          )}
        </div>
      </div>
    </div>
  );
};

export default EnhancedTerminal;
