import React, { useState, useRef, useEffect } from 'react';
import { Terminal as TerminalIcon, Plus, X, Minimize2 } from 'lucide-react';

interface TerminalProps {}

const Terminal: React.FC<TerminalProps> = () => {
  const [isMinimized, setIsMinimized] = useState(false);
  const [activeTab, setActiveTab] = useState(0);
  const [terminals, setTerminals] = useState([
    { id: 1, name: 'Terminal 1', output: ['$ Welcome to SymbioteIDE Terminal'], input: '' }
  ]);
  const inputRef = useRef<HTMLInputElement>(null);

  const addTerminal = () => {
    const newId = terminals.length + 1;
    setTerminals([...terminals, {
      id: newId,
      name: `Terminal ${newId}`,
      output: [`$ Welcome to SymbioteIDE Terminal ${newId}`],
      input: ''
    }]);
    setActiveTab(terminals.length);
  };

  const closeTerminal = (index: number) => {
    if (terminals.length === 1) return; // Keep at least one terminal
    
    const newTerminals = terminals.filter((_, i) => i !== index);
    setTerminals(newTerminals);
    
    if (activeTab >= newTerminals.length) {
      setActiveTab(newTerminals.length - 1);
    }
  };

  const handleCommand = (command: string) => {
    if (!command.trim()) return;

    const currentTerminal = terminals[activeTab];
    const newOutput = [...currentTerminal.output, `$ ${command}`];

    // Mock command execution
    let response = '';
    switch (command.toLowerCase().trim()) {
      case 'help':
        response = 'Available commands: help, clear, ls, pwd, echo, npm, cargo, git';
        break;
      case 'clear':
        newOutput.length = 0;
        newOutput.push('$ Welcome to SymbioteIDE Terminal');
        break;
      case 'ls':
        response = 'src/  src-tauri/  package.json  README.md  node_modules/';
        break;
      case 'pwd':
        response = '/workspace/symbiote-ide';
        break;
      case 'npm install':
        response = 'Installing dependencies...\n✓ Dependencies installed successfully';
        break;
      case 'cargo build':
        response = 'Compiling symbiote-ide v0.1.0\n✓ Build completed successfully';
        break;
      case 'git status':
        response = 'On branch main\nYour branch is up to date with \'origin/main\'.\n\nnothing to commit, working tree clean';
        break;
      default:
        if (command.startsWith('echo ')) {
          response = command.substring(5);
        } else {
          response = `Command not found: ${command}`;
        }
    }

    if (response) {
      newOutput.push(response);
    }

    const updatedTerminals = terminals.map((term, index) => 
      index === activeTab 
        ? { ...term, output: newOutput, input: '' }
        : term
    );

    setTerminals(updatedTerminals);
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      const input = e.currentTarget as HTMLInputElement;
      handleCommand(input.value);
      input.value = '';
    }
  };

  const updateInput = (value: string) => {
    const updatedTerminals = terminals.map((term, index) => 
      index === activeTab 
        ? { ...term, input: value }
        : term
    );
    setTerminals(updatedTerminals);
  };

  useEffect(() => {
    if (inputRef.current && !isMinimized) {
      inputRef.current.focus();
    }
  }, [activeTab, isMinimized]);

  if (isMinimized) {
    return (
      <div className="terminal-minimized" onClick={() => setIsMinimized(false)}>
        <TerminalIcon size={16} />
        <span>Terminal ({terminals.length})</span>
      </div>
    );
  }

  return (
    <div className="terminal-container">
      <div className="terminal-header">
        <div className="terminal-tabs">
          {terminals.map((terminal, index) => (
            <div 
              key={terminal.id}
              className={`terminal-tab ${index === activeTab ? 'active' : ''}`}
              onClick={() => setActiveTab(index)}
            >
              <TerminalIcon size={14} />
              <span>{terminal.name}</span>
              {terminals.length > 1 && (
                <button 
                  className="tab-close"
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTerminal(index);
                  }}
                >
                  <X size={12} />
                </button>
              )}
            </div>
          ))}
          <button className="add-terminal" onClick={addTerminal} title="New Terminal">
            <Plus size={14} />
          </button>
        </div>
        
        <div className="terminal-controls">
          <button 
            className="control-btn"
            onClick={() => setIsMinimized(true)}
            title="Minimize"
          >
            <Minimize2 size={14} />
          </button>
        </div>
      </div>

      <div className="terminal-content">
        <div className="terminal-output">
          {terminals[activeTab]?.output.map((line, index) => (
            <div key={index} className="terminal-line">
              {line}
            </div>
          ))}
        </div>
        
        <div className="terminal-input-line">
          <span className="terminal-prompt">$ </span>
          <input
            ref={inputRef}
            type="text"
            className="terminal-input"
            value={terminals[activeTab]?.input || ''}
            onChange={(e) => updateInput(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="Type a command..."
          />
        </div>
      </div>
    </div>
  );
};

export default Terminal;
