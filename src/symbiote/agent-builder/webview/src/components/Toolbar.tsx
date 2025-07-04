import React, { useState } from 'react';
import { ExportFormat } from '../../../types/workflow-types';

interface ToolbarProps {
  onExecute: () => void;
  onExport: (format: ExportFormat) => void;
  isExecuting: boolean;
}

const Toolbar: React.FC<ToolbarProps> = ({ onExecute, onExport, isExecuting }) => {
  const [showExportMenu, setShowExportMenu] = useState(false);
  
  const exportFormats: { format: ExportFormat; label: string; icon: string }[] = [
    { format: 'typescript', label: 'TypeScript', icon: '📘' },
    { format: 'javascript', label: 'JavaScript', icon: '📙' },
    { format: 'python', label: 'Python', icon: '🐍' },
    { format: 'docker', label: 'Docker', icon: '🐳' },
    { format: 'serverless', label: 'Serverless', icon: '☁️' },
    { format: 'cli', label: 'CLI Tool', icon: '💻' }
  ];
  
  return (
    <div className="toolbar">
      <div className="toolbar-section">
        <h2>Agent Workflow Builder</h2>
      </div>
      
      <div className="toolbar-section toolbar-actions">
        <button
          className={`toolbar-button execute ${isExecuting ? 'executing' : ''}`}
          onClick={onExecute}
          disabled={isExecuting}
          title="Execute workflow"
        >
          {isExecuting ? (
            <>
              <span className="icon">⏳</span>
              <span>Executing...</span>
            </>
          ) : (
            <>
              <span className="icon">▶️</span>
              <span>Execute</span>
            </>
          )}
        </button>
        
        <div className="export-dropdown">
          <button
            className="toolbar-button"
            onClick={() => setShowExportMenu(!showExportMenu)}
            title="Export workflow"
          >
            <span className="icon">📦</span>
            <span>Export</span>
            <span className="dropdown-arrow">▼</span>
          </button>
          
          {showExportMenu && (
            <div className="export-menu">
              {exportFormats.map(({ format, label, icon }) => (
                <button
                  key={format}
                  className="export-option"
                  onClick={() => {
                    onExport(format);
                    setShowExportMenu(false);
                  }}
                >
                  <span className="icon">{icon}</span>
                  <span>{label}</span>
                </button>
              ))}
            </div>
          )}
        </div>
        
        <button
          className="toolbar-button"
          onClick={() => {
            // Save workflow
          }}
          title="Save workflow"
        >
          <span className="icon">💾</span>
          <span>Save</span>
        </button>
      </div>
    </div>
  );
};

export default Toolbar;