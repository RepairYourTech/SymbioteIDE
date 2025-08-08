import React, { useState } from 'react';
import { Database, Eye, Code, FileText, Layers } from 'lucide-react';

interface ParseResponse {
  ast: string;
  symbols: string[];
  errors: string[];
}

interface ContextPanelProps {
  currentFile: string;
  parseResult: ParseResponse | null;
}

const ContextPanel: React.FC<ContextPanelProps> = ({ currentFile, parseResult }) => {
  const [activeTab, setActiveTab] = useState<'symbols' | 'context' | 'ast'>('symbols');

  const tabs = [
    { id: 'symbols' as const, name: 'Symbols', icon: Code },
    { id: 'context' as const, name: 'Context', icon: Database },
    { id: 'ast' as const, name: 'AST', icon: Layers },
  ];

  const renderSymbols = () => {
    if (!parseResult?.symbols.length) {
      return (
        <div className="empty-state">
          <Code size={24} />
          <p>No symbols found</p>
        </div>
      );
    }

    const groupedSymbols = parseResult.symbols.reduce((acc, symbol) => {
      const [type, name] = symbol.split(':');
      if (!acc[type]) acc[type] = [];
      acc[type].push(name);
      return acc;
    }, {} as Record<string, string[]>);

    return (
      <div className="symbols-content">
        {Object.entries(groupedSymbols).map(([type, names]) => (
          <div key={type} className="symbol-group">
            <h5 className="symbol-type">{type.replace('_', ' ')}</h5>
            <div className="symbol-list">
              {names.map((name, index) => (
                <div key={index} className="symbol-item">
                  <span className="symbol-name">{name}</span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  const renderContext = () => {
    return (
      <div className="context-content">
        <div className="context-section">
          <h5>Current File</h5>
          <div className="context-item">
            <FileText size={16} />
            <span>{currentFile || 'No file selected'}</span>
          </div>
        </div>

        <div className="context-section">
          <h5>Workspace Context</h5>
          <div className="context-item">
            <Database size={16} />
            <span>SymbioteIDE Project</span>
          </div>
          <div className="context-item">
            <Code size={16} />
            <span>Tauri + React + TypeScript</span>
          </div>
        </div>

        {parseResult && (
          <div className="context-section">
            <h5>Parse Context</h5>
            <div className="context-item">
              <span className="context-label">Symbols:</span>
              <span className="context-value">{parseResult.symbols.length}</span>
            </div>
            <div className="context-item">
              <span className="context-label">Errors:</span>
              <span className={`context-value ${parseResult.errors.length > 0 ? 'error' : 'success'}`}>
                {parseResult.errors.length}
              </span>
            </div>
          </div>
        )}

        <div className="context-section">
          <h5>AI Context</h5>
          <div className="context-item">
            <span className="context-label">Providers:</span>
            <span className="context-value">OpenAI, Anthropic, Google</span>
          </div>
          <div className="context-item">
            <span className="context-label">Active Model:</span>
            <span className="context-value">GPT-4</span>
          </div>
        </div>
      </div>
    );
  };

  const renderAST = () => {
    if (!parseResult?.ast) {
      return (
        <div className="empty-state">
          <Layers size={24} />
          <p>No AST available</p>
        </div>
      );
    }

    return (
      <div className="ast-content">
        <div className="ast-viewer">
          <pre className="ast-tree">{parseResult.ast}</pre>
        </div>
      </div>
    );
  };

  return (
    <div className="context-panel">
      <div className="panel-header">
        <h3>Context</h3>
        <Eye size={16} />
      </div>

      <div className="context-tabs">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              className={`context-tab ${activeTab === tab.id ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <Icon size={14} />
              <span>{tab.name}</span>
            </button>
          );
        })}
      </div>

      <div className="context-content">
        {activeTab === 'symbols' && renderSymbols()}
        {activeTab === 'context' && renderContext()}
        {activeTab === 'ast' && renderAST()}
      </div>
    </div>
  );
};

export default ContextPanel;
