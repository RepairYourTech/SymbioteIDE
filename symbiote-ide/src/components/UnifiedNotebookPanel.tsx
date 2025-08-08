// Unified Notebook Panel - Revolutionary AI-Powered Interactive Development Environment
// Phase 4 UI Component for Unified Notebook System integration

import React, { useState, useEffect } from 'react';
// import { invoke } from '@tauri-apps/api/tauri';
import './UnifiedNotebookPanel.css';

interface NotebookInfo {
  id: string;
  name: string;
  type: string;
  lastModified: Date;
  cellCount: number;
}

interface Notebook {
  notebook_id: string;
  name: string;
  description: string;
  notebook_type: string;
  metadata: NotebookMetadata;
  cells: NotebookCell[];
  created_at: string;
  last_modified: string;
  last_executed?: string;
}

interface NotebookMetadata {
  title: string;
  author: string;
  tags: string[];
  categories: string[];
  language: string;
  ai_settings: AISettings;
}

interface AISettings {
  ai_assistance_enabled: boolean;
  auto_completion: boolean;
  code_suggestions: boolean;
  documentation_generation: boolean;
  error_analysis: boolean;
  performance_optimization: boolean;
}

interface NotebookCell {
  cell_id: string;
  cell_type: string;
  content: CellContent;
  metadata: CellMetadata;
  execution_info?: ExecutionInfo;
  outputs: CellOutput[];
  ai_annotations: AIAnnotation[];
  created_at: string;
  last_modified: string;
}

interface CellContent {
  source: string;
  language?: string;
  syntax_highlighting: boolean;
  line_numbers: boolean;
}

interface CellMetadata {
  tags: string[];
  collapsed: boolean;
  editable: boolean;
  deletable: boolean;
}

interface ExecutionInfo {
  execution_count: number;
  started_at: string;
  completed_at?: string;
  execution_time?: number;
  status: string;
}

interface CellOutput {
  output_id: string;
  output_type: string;
  data: OutputData;
  created_at: string;
}

interface OutputData {
  mime_type: string;
  content: any;
  size: number;
}

interface AIAnnotation {
  annotation_id: string;
  annotation_type: string;
  content: string;
  confidence: number;
  source_model: string;
  created_at: string;
}

interface UnifiedNotebookPanelProps {
  notebooks: NotebookInfo[];
  onNotebookCreated: (notebook: NotebookInfo) => void;
}

export const UnifiedNotebookPanel: React.FC<UnifiedNotebookPanelProps> = ({
  notebooks,
  onNotebookCreated,
}) => {
  const [activeTab, setActiveTab] = useState<'browse' | 'create' | 'editor'>('browse');
  const [selectedNotebook, setSelectedNotebook] = useState<Notebook | null>(null);
  const [loading, setLoading] = useState(false);
  const [executing, setExecuting] = useState<Set<string>>(new Set());

  // Notebook creation form
  const [notebookForm, setNotebookForm] = useState({
    name: '',
    description: '',
    type: 'Research',
    language: 'python',
    aiAssistance: true,
  });

  // Cell editing
  const [editingCell, setEditingCell] = useState<string | null>(null);
  const [cellContent, setCellContent] = useState<string>('');

  const notebookTypes = [
    'Jupyter', 'Research', 'Tutorial', 'Documentation', 'AIAssisted',
    'CodeGeneration', 'DataAnalysis', 'MachineLearning', 'Shared',
    'Template', 'Workshop', 'Presentation', 'Experiment', 'Report',
    'Dashboard', 'Interactive',
  ];

  const languages = [
    'python', 'javascript', 'typescript', 'rust', 'go', 'java',
    'cpp', 'c', 'csharp', 'php', 'ruby', 'swift', 'kotlin',
    'scala', 'r', 'julia', 'sql', 'markdown',
  ];

  useEffect(() => {
    if (selectedNotebook) {
      setActiveTab('editor');
    }
  }, [selectedNotebook]);

  const createNotebook = async () => {
    if (!notebookForm.name) {
      alert('Please enter a notebook name');
      return;
    }

    setLoading(true);
    try {
      // Mock create notebook - replace with actual Tauri invoke when ready
      const notebookId = `notebook_${Date.now()}`;

      // Reset form
      setNotebookForm({
        name: '',
        description: '',
        type: 'Research',
        language: 'python',
        aiAssistance: true,
      });

      // Notify parent component
      onNotebookCreated({
        id: notebookId,
        name: notebookForm.name,
        type: notebookForm.type,
        lastModified: new Date(),
        cellCount: 0,
      });

      // Load the created notebook
      await loadNotebook(notebookId);
      
      alert('Notebook created successfully!');
    } catch (error) {
      console.error('Failed to create notebook:', error);
      alert('Failed to create notebook. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const loadNotebook = async (notebookId: string) => {
    setLoading(true);
    try {
      // Mock get notebook - replace with actual Tauri invoke when ready
      const notebook: Notebook = {
        notebook_id: notebookId,
        name: `Mock Notebook ${notebookId}`,
        description: 'Mock notebook for testing',
        notebook_type: 'Research',
        metadata: {
          title: `Mock Notebook ${notebookId}`,
          author: 'current_user',
          tags: [],
          categories: [],
          language: 'python',
          ai_settings: {
            ai_assistance_enabled: true,
            auto_completion: true,
            code_suggestions: true,
            documentation_generation: true,
            error_analysis: true,
            performance_optimization: true,
          },
        },
        cells: [],
        created_at: new Date().toISOString(),
        last_modified: new Date().toISOString(),
      };
      setSelectedNotebook(notebook);
    } catch (error) {
      console.error('Failed to load notebook:', error);
      alert('Failed to load notebook.');
    } finally {
      setLoading(false);
    }
  };

  const addCell = async (cellType: string) => {
    if (!selectedNotebook) return;

    try {
      // Mock add cell - replace with actual Tauri invoke when ready
      const cellId = `cell_${Date.now()}`;
      console.log('Adding cell:', { notebookId: selectedNotebook.notebook_id, cellType, cellId });

      // Reload notebook to get updated cells
      await loadNotebook(selectedNotebook.notebook_id);
    } catch (error) {
      console.error('Failed to add cell:', error);
      alert('Failed to add cell.');
    }
  };

  const executeCell = async (cellId: string) => {
    if (!selectedNotebook) return;

    setExecuting(prev => new Set(prev).add(cellId));
    try {
      // Mock execute cell - replace with actual Tauri invoke when ready
      console.log('Executing cell:', { notebookId: selectedNotebook.notebook_id, cellId });

      // Reload notebook to get updated execution results
      await loadNotebook(selectedNotebook.notebook_id);
    } catch (error) {
      console.error('Failed to execute cell:', error);
      alert('Failed to execute cell.');
    } finally {
      setExecuting(prev => {
        const newSet = new Set(prev);
        newSet.delete(cellId);
        return newSet;
      });
    }
  };

  const saveCell = async (cellId: string, content: string) => {
    if (!selectedNotebook) return;

    try {
      // Mock update cell content - replace with actual Tauri invoke when ready
      console.log('Updating cell content:', {
        notebookId: selectedNotebook.notebook_id,
        cellId,
        content,
      });

      // Update local state
      setSelectedNotebook(prev => {
        if (!prev) return prev;
        return {
          ...prev,
          cells: prev.cells.map(cell =>
            cell.cell_id === cellId
              ? { ...cell, content: { ...cell.content, source: content } }
              : cell
          ),
        };
      });

      setEditingCell(null);
      setCellContent('');
    } catch (error) {
      console.error('Failed to save cell:', error);
      alert('Failed to save cell.');
    }
  };

  const deleteCell = async (cellId: string) => {
    if (!selectedNotebook) return;
    if (!confirm('Are you sure you want to delete this cell?')) return;

    try {
      // Mock delete cell - replace with actual Tauri invoke when ready
      console.log('Deleting cell:', {
        notebookId: selectedNotebook.notebook_id,
        cellId,
      });

      // Reload notebook
      await loadNotebook(selectedNotebook.notebook_id);
    } catch (error) {
      console.error('Failed to delete cell:', error);
      alert('Failed to delete cell.');
    }
  };

  const getCellTypeIcon = (cellType: string) => {
    switch (cellType) {
      case 'Code': return '⚡';
      case 'Markdown': return '📝';
      case 'Raw': return '📄';
      case 'HTML': return '🌐';
      case 'SQL': return '🗃️';
      case 'Visualization': return '📊';
      default: return '📋';
    }
  };

  const getNotebookTypeIcon = (type: string) => {
    const icons: { [key: string]: string } = {
      'Jupyter': '📓',
      'Research': '🔬',
      'Tutorial': '📚',
      'Documentation': '📖',
      'AIAssisted': '🤖',
      'CodeGeneration': '⚡',
      'DataAnalysis': '📊',
      'MachineLearning': '🧠',
      'Shared': '👥',
      'Template': '📋',
      'Workshop': '🛠️',
      'Presentation': '📽️',
      'Experiment': '🧪',
      'Report': '📈',
      'Dashboard': '📊',
      'Interactive': '🎮',
    };
    return icons[type] || '📓';
  };

  const getExecutionStatusIcon = (status?: string) => {
    switch (status) {
      case 'Completed': return '✅';
      case 'Running': return '⏳';
      case 'Error': return '❌';
      case 'Interrupted': return '⏸️';
      default: return '⚪';
    }
  };

  const formatExecutionTime = (timeMs?: number) => {
    if (!timeMs) return '';
    if (timeMs < 1000) return `${timeMs}ms`;
    return `${(timeMs / 1000).toFixed(2)}s`;
  };

  const renderCellOutput = (output: CellOutput) => {
    const { data } = output;
    
    if (data.mime_type === 'text/plain') {
      return (
        <div className="cell-output text-output">
          <pre>{String(data.content)}</pre>
        </div>
      );
    }
    
    if (data.mime_type === 'text/html') {
      return (
        <div 
          className="cell-output html-output"
          dangerouslySetInnerHTML={{ __html: String(data.content) }}
        />
      );
    }
    
    if (data.mime_type.startsWith('image/')) {
      return (
        <div className="cell-output image-output">
          <img src={String(data.content)} alt="Output" />
        </div>
      );
    }
    
    return (
      <div className="cell-output json-output">
        <pre>{JSON.stringify(data.content, null, 2)}</pre>
      </div>
    );
  };

  return (
    <div className="unified-notebook-panel">
      {/* Tab Navigation */}
      <div className="tab-navigation">
        <button
          className={`tab ${activeTab === 'browse' ? 'active' : ''}`}
          onClick={() => setActiveTab('browse')}
        >
          📚 Browse Notebooks
        </button>
        <button
          className={`tab ${activeTab === 'create' ? 'active' : ''}`}
          onClick={() => setActiveTab('create')}
        >
          ➕ Create Notebook
        </button>
        {selectedNotebook && (
          <button
            className={`tab ${activeTab === 'editor' ? 'active' : ''}`}
            onClick={() => setActiveTab('editor')}
          >
            📝 {selectedNotebook.name}
          </button>
        )}
      </div>

      {/* Tab Content */}
      <div className="tab-content">
        {activeTab === 'browse' && (
          <div className="browse-notebooks-tab">
            <div className="notebooks-header">
              <h3>Your Notebooks ({notebooks.length})</h3>
              <div className="header-actions">
                <button
                  onClick={() => setActiveTab('create')}
                  className="create-notebook-button"
                >
                  ➕ New Notebook
                </button>
              </div>
            </div>

            <div className="notebooks-grid">
              {notebooks.map(notebook => (
                <div
                  key={notebook.id}
                  className="notebook-card"
                  onClick={() => loadNotebook(notebook.id)}
                >
                  <div className="notebook-header">
                    <div className="notebook-icon">
                      {getNotebookTypeIcon(notebook.type)}
                    </div>
                    <div className="notebook-info">
                      <div className="notebook-name">{notebook.name}</div>
                      <div className="notebook-type">{notebook.type}</div>
                    </div>
                  </div>
                  
                  <div className="notebook-stats">
                    <div className="stat">
                      <span className="stat-icon">📋</span>
                      <span>{notebook.cellCount} cells</span>
                    </div>
                    <div className="stat">
                      <span className="stat-icon">📅</span>
                      <span>{notebook.lastModified.toLocaleDateString()}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {notebooks.length === 0 && (
              <div className="empty-state">
                <h4>No notebooks yet</h4>
                <p>Create your first notebook to get started</p>
                <button
                  onClick={() => setActiveTab('create')}
                  className="create-first-notebook-button"
                >
                  ➕ Create First Notebook
                </button>
              </div>
            )}
          </div>
        )}

        {activeTab === 'create' && (
          <div className="create-notebook-tab">
            <div className="create-form">
              <h3>Create New Notebook</h3>
              
              <div className="form-group">
                <label>Notebook Name *</label>
                <input
                  type="text"
                  value={notebookForm.name}
                  onChange={(e) => setNotebookForm(prev => ({ ...prev, name: e.target.value }))}
                  placeholder="Enter notebook name..."
                  className="form-input"
                />
              </div>
              
              <div className="form-group">
                <label>Description</label>
                <textarea
                  value={notebookForm.description}
                  onChange={(e) => setNotebookForm(prev => ({ ...prev, description: e.target.value }))}
                  placeholder="Describe your notebook..."
                  className="form-textarea"
                  rows={3}
                />
              </div>
              
              <div className="form-row">
                <div className="form-group">
                  <label>Notebook Type</label>
                  <select
                    value={notebookForm.type}
                    onChange={(e) => setNotebookForm(prev => ({ ...prev, type: e.target.value }))}
                    className="form-select"
                  >
                    {notebookTypes.map(type => (
                      <option key={type} value={type}>
                        {getNotebookTypeIcon(type)} {type}
                      </option>
                    ))}
                  </select>
                </div>
                
                <div className="form-group">
                  <label>Primary Language</label>
                  <select
                    value={notebookForm.language}
                    onChange={(e) => setNotebookForm(prev => ({ ...prev, language: e.target.value }))}
                    className="form-select"
                  >
                    {languages.map(lang => (
                      <option key={lang} value={lang}>
                        {lang.charAt(0).toUpperCase() + lang.slice(1)}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              
              <div className="form-group">
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={notebookForm.aiAssistance}
                    onChange={(e) => setNotebookForm(prev => ({ ...prev, aiAssistance: e.target.checked }))}
                  />
                  Enable AI Assistance
                </label>
                <p className="help-text">
                  AI will provide code suggestions, documentation generation, and error analysis
                </p>
              </div>
              
              <div className="form-actions">
                <button
                  onClick={createNotebook}
                  disabled={loading || !notebookForm.name}
                  className="create-button"
                >
                  {loading ? '⏳ Creating...' : '📓 Create Notebook'}
                </button>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'editor' && selectedNotebook && (
          <div className="notebook-editor-tab">
            <div className="notebook-toolbar">
              <div className="notebook-title">
                <h3>{selectedNotebook.name}</h3>
                <span className="notebook-type-badge">
                  {getNotebookTypeIcon(selectedNotebook.notebook_type)} {selectedNotebook.notebook_type}
                </span>
              </div>
              
              <div className="toolbar-actions">
                <button
                  onClick={() => addCell('Code')}
                  className="add-cell-button"
                >
                  ⚡ Add Code Cell
                </button>
                <button
                  onClick={() => addCell('Markdown')}
                  className="add-cell-button"
                >
                  📝 Add Markdown Cell
                </button>
                <button
                  onClick={() => setActiveTab('browse')}
                  className="back-button"
                >
                  ← Back to Notebooks
                </button>
              </div>
            </div>

            <div className="cells-container">
                {selectedNotebook.cells.map((cell) => (
                <div key={cell.cell_id} className="notebook-cell">
                  <div className="cell-header">
                    <div className="cell-info">
                      <span className="cell-type">
                        {getCellTypeIcon(cell.cell_type)} {cell.cell_type}
                      </span>
                      {cell.execution_info && (
                        <span className="execution-info">
                          {getExecutionStatusIcon(cell.execution_info.status)}
                          [{cell.execution_info.execution_count}]
                          {cell.execution_info.execution_time && (
                            <span className="execution-time">
                              {formatExecutionTime(cell.execution_info.execution_time)}
                            </span>
                          )}
                        </span>
                      )}
                    </div>
                    
                    <div className="cell-actions">
                      {cell.cell_type === 'Code' && (
                        <button
                          onClick={() => executeCell(cell.cell_id)}
                          disabled={executing.has(cell.cell_id)}
                          className="execute-button"
                        >
                          {executing.has(cell.cell_id) ? '⏳' : '▶️'}
                        </button>
                      )}
                      <button
                        onClick={() => {
                          setEditingCell(cell.cell_id);
                          setCellContent(cell.content.source);
                        }}
                        className="edit-button"
                      >
                        ✏️
                      </button>
                      <button
                        onClick={() => deleteCell(cell.cell_id)}
                        className="delete-button"
                      >
                        🗑️
                      </button>
                    </div>
                  </div>

                  <div className="cell-content">
                    {editingCell === cell.cell_id ? (
                      <div className="cell-editor">
                        <textarea
                          value={cellContent}
                          onChange={(e) => setCellContent(e.target.value)}
                          className="cell-textarea"
                          rows={Math.max(3, cellContent.split('\n').length)}
                        />
                        <div className="editor-actions">
                          <button
                            onClick={() => saveCell(cell.cell_id, cellContent)}
                            className="save-button"
                          >
                            💾 Save
                          </button>
                          <button
                            onClick={() => {
                              setEditingCell(null);
                              setCellContent('');
                            }}
                            className="cancel-button"
                          >
                            ❌ Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div className="cell-display">
                        <pre className="cell-source">{cell.content.source}</pre>
                      </div>
                    )}
                  </div>

                  {cell.outputs.length > 0 && (
                    <div className="cell-outputs">
                      {cell.outputs.map(output => (
                        <div key={output.output_id}>
                          {renderCellOutput(output)}
                        </div>
                      ))}
                    </div>
                  )}

                  {cell.ai_annotations.length > 0 && (
                    <div className="ai-annotations">
                      <div className="annotations-header">🤖 AI Suggestions</div>
                      {cell.ai_annotations.map(annotation => (
                        <div key={annotation.annotation_id} className="ai-annotation">
                          <div className="annotation-type">{annotation.annotation_type}</div>
                          <div className="annotation-content">{annotation.content}</div>
                          <div className="annotation-meta">
                            Confidence: {Math.round(annotation.confidence * 100)}% | 
                            Model: {annotation.source_model}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ))}

              {selectedNotebook.cells.length === 0 && (
                <div className="empty-notebook">
                  <h4>Empty Notebook</h4>
                  <p>Add your first cell to get started</p>
                  <div className="quick-add-buttons">
                    <button
                      onClick={() => addCell('Code')}
                      className="quick-add-button"
                    >
                      ⚡ Add Code Cell
                    </button>
                    <button
                      onClick={() => addCell('Markdown')}
                      className="quick-add-button"
                    >
                      📝 Add Markdown Cell
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default UnifiedNotebookPanel;
