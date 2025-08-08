import React, { useState } from 'react';
import FileExplorer from './FileExplorer';
import CodeEditor from './CodeEditor';
import { APIBuilderPanel } from './APIBuilderPanel';
import SymbioteTerminal from './SymbioteTerminal';
import { CandlestickChart } from './Trading/CandlestickChart';
import { WalletManager } from './Trading/WalletManager';

interface IDELayoutProps {
  // Props for the main IDE layout
}

type PanelType = 'explorer' | 'search' | 'git' | 'debug' | 'extensions' | 'agents' | 'trading';
type BottomPanelType = 'terminal' | 'problems' | 'output' | 'debug-console';

export const IDELayout: React.FC<IDELayoutProps> = () => {
  // Layout state
  const [leftPanelVisible, setLeftPanelVisible] = useState(true);
  const [rightPanelVisible, setRightPanelVisible] = useState(true);
  const [bottomPanelVisible, setBottomPanelVisible] = useState(true);
  
  // Panel content state
  const [activeLeftPanel, setActiveLeftPanel] = useState<PanelType>('explorer');
  const [activeRightPanel, setActiveRightPanel] = useState<PanelType>('agents');
  const [activeBottomPanel, setActiveBottomPanel] = useState<BottomPanelType>('terminal');
  
  // Editor state
  const [activeFile, setActiveFile] = useState<string | null>(null);

  const handleFileSelect = (filePath: string) => {
    setActiveFile(filePath);
  };

  const renderLeftPanel = () => {
    switch (activeLeftPanel) {
      case 'explorer':
        return <FileExplorer onFileSelect={handleFileSelect} />;
      case 'search':
        return <div className="panel-content">Search functionality coming soon...</div>;
      case 'git':
        return <div className="panel-content">Git integration coming soon...</div>;
      case 'debug':
        return <div className="panel-content">Debug panel coming soon...</div>;
      case 'extensions':
        return <div className="panel-content">Extensions panel coming soon...</div>;
      default:
        return <FileExplorer onFileSelect={handleFileSelect} />;
    }
  };

  const renderRightPanel = () => {
    switch (activeRightPanel) {
      case 'agents':
        return <div className="panel-content">AI Agents panel coming soon...</div>;
      case 'trading':
        return (
          <div className="panel-content">
            <div className="trading-panels">
              <div className="trading-section">
                <h3>Portfolio</h3>
                <WalletManager />
              </div>
              <div className="trading-section">
                <h3>Market Data</h3>
                <CandlestickChart
                  symbol="BTC/USDT"
                  timeframe="1h"
                />
              </div>
            </div>
          </div>
        );
      default:
        return <div className="panel-content">AI Agents panel coming soon...</div>;
    }
  };

  const renderBottomPanel = () => {
    switch (activeBottomPanel) {
      case 'terminal':
        return <SymbioteTerminal />;
      case 'problems':
        return <div className="panel-content">Problems panel coming soon...</div>;
      case 'output':
        return <div className="panel-content">Output panel coming soon...</div>;
      case 'debug-console':
        return <div className="panel-content">Debug console coming soon...</div>;
      default:
        return <SymbioteTerminal />;
    }
  };

  return (
    <div className="ide-layout">
      {/* Menu Bar */}
      <div className="menu-bar">
        <div className="menu-items">
          <span className="menu-item">File</span>
          <span className="menu-item">Edit</span>
          <span className="menu-item">View</span>
          <span className="menu-item">Go</span>
          <span className="menu-item">Run</span>
          <span className="menu-item">Terminal</span>
          <span className="menu-item">Help</span>
        </div>
        <div className="window-controls">
          <span className="window-control minimize">−</span>
          <span className="window-control maximize">□</span>
          <span className="window-control close">×</span>
        </div>
      </div>

      {/* Main Content */}
      <div className="ide-body">
        {/* Activity Bar */}
        <div className="activity-bar">
          <div className="activity-items">
            <button
              className={`activity-item ${activeLeftPanel === 'explorer' ? 'active' : ''}`}
              onClick={() => setActiveLeftPanel('explorer')}
              title="Explorer"
            >
              📁
            </button>
            <button
              className={`activity-item ${activeLeftPanel === 'search' ? 'active' : ''}`}
              onClick={() => setActiveLeftPanel('search')}
              title="Search"
            >
              🔍
            </button>
            <button
              className={`activity-item ${activeLeftPanel === 'git' ? 'active' : ''}`}
              onClick={() => setActiveLeftPanel('git')}
              title="Source Control"
            >
              🌿
            </button>
            <button
              className={`activity-item ${activeLeftPanel === 'debug' ? 'active' : ''}`}
              onClick={() => setActiveLeftPanel('debug')}
              title="Run and Debug"
            >
              ▶️
            </button>
            <button
              className={`activity-item ${activeLeftPanel === 'extensions' ? 'active' : ''}`}
              onClick={() => setActiveLeftPanel('extensions')}
              title="Extensions"
            >
              🧩
            </button>
          </div>
          
          <div className="activity-items-bottom">
            <button
              className={`activity-item ${activeRightPanel === 'agents' ? 'active' : ''}`}
              onClick={() => setActiveRightPanel('agents')}
              title="AI Agents"
            >
              🤖
            </button>
            <button
              className={`activity-item ${activeRightPanel === 'trading' ? 'active' : ''}`}
              onClick={() => setActiveRightPanel('trading')}
              title="Trading"
            >
              📈
            </button>
          </div>
        </div>

        {/* Left Panel */}
        {leftPanelVisible && (
          <div className="left-panel">
            <div className="panel-header">
              <h3>{activeLeftPanel.toUpperCase()}</h3>
              <button
                className="panel-toggle"
                onClick={() => setLeftPanelVisible(false)}
              >
                ×
              </button>
            </div>
            <div className="panel-body">
              {renderLeftPanel()}
            </div>
          </div>
        )}

        {/* Editor Area */}
        <div className="editor-area">
          <CodeEditor 
            activeFile={activeFile || undefined}
            onFileChange={(path, content) => {
              console.log('File changed:', path, content);
            }}
          />
        </div>

        {/* Right Panel */}
        {rightPanelVisible && (
          <div className="right-panel">
            <div className="panel-header">
              <h3>{activeRightPanel.toUpperCase()}</h3>
              <button
                className="panel-toggle"
                onClick={() => setRightPanelVisible(false)}
              >
                ×
              </button>
            </div>
            <div className="panel-body">
              {renderRightPanel()}
            </div>
          </div>
        )}
      </div>

      {/* Bottom Panel */}
      {bottomPanelVisible && (
        <div className="bottom-panel">
          <div className="panel-tabs">
            <button
              className={`panel-tab ${activeBottomPanel === 'terminal' ? 'active' : ''}`}
              onClick={() => setActiveBottomPanel('terminal')}
            >
              Terminal
            </button>
            <button
              className={`panel-tab ${activeBottomPanel === 'problems' ? 'active' : ''}`}
              onClick={() => setActiveBottomPanel('problems')}
            >
              Problems
            </button>
            <button
              className={`panel-tab ${activeBottomPanel === 'output' ? 'active' : ''}`}
              onClick={() => setActiveBottomPanel('output')}
            >
              Output
            </button>
            <button
              className={`panel-tab ${activeBottomPanel === 'debug-console' ? 'active' : ''}`}
              onClick={() => setActiveBottomPanel('debug-console')}
            >
              Debug Console
            </button>
            <button
              className="panel-toggle"
              onClick={() => setBottomPanelVisible(false)}
            >
              ×
            </button>
          </div>
          <div className="panel-body">
            {renderBottomPanel()}
          </div>
        </div>
      )}

      {/* Status Bar */}
      <div className="status-bar">
        <div className="status-left">
          <span className="status-item">🤖 3 Agents Active</span>
          <span className="status-item">📈 Trading: $1,234.56</span>
        </div>
        <div className="status-right">
          <span className="status-item">TypeScript</span>
          <span className="status-item">UTF-8</span>
          <span className="status-item">Ln 1, Col 1</span>
        </div>
      </div>
    </div>
  );
};

export default IDELayout;
