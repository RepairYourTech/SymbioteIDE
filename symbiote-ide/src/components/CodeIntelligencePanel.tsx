import React, { useState, useEffect, useCallback } from 'react';
import './CodeIntelligencePanel.css';

// Types for Code Intelligence features
interface CodeEntity {
  id: string;
  name: string;
  type: 'function' | 'class' | 'interface' | 'variable' | 'module';
  file_path: string;
  line_number: number;
  complexity_score: number;
  dependencies: string[];
  dependents: string[];
  last_modified: string;
  ai_insights: {
    quality_score: number;
    refactor_suggestions: string[];
    potential_bugs: string[];
    optimization_opportunities: string[];
  };
}

interface DuplicationResult {
  id: string;
  similarity_score: number;
  code_blocks: {
    file_path: string;
    start_line: number;
    end_line: number;
    code_snippet: string;
  }[];
  refactor_suggestion: string;
  estimated_savings: {
    lines_of_code: number;
    maintenance_effort: string;
  };
}

interface ContextInsight {
  type: 'dependency' | 'usage' | 'impact' | 'pattern';
  title: string;
  description: string;
  confidence: number;
  related_files: string[];
  action_items: string[];
}

interface CodeIntelligencePanelProps {
  currentFile?: string;
  selectedCode?: string;
  onNavigateToCode: (filePath: string, lineNumber: number) => void;
  onApplyRefactor: (suggestion: string) => void;
}

const CodeIntelligencePanel: React.FC<CodeIntelligencePanelProps> = ({
  currentFile,
  selectedCode,
  onNavigateToCode,
  onApplyRefactor
}) => {
  const [activeTab, setActiveTab] = useState<'analysis' | 'duplicates' | 'context' | 'insights'>('analysis');
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [codeEntities, setCodeEntities] = useState<CodeEntity[]>([]);
  const [duplications, setDuplications] = useState<DuplicationResult[]>([]);
  const [contextInsights, setContextInsights] = useState<ContextInsight[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<'all' | 'high-complexity' | 'needs-refactor' | 'bugs'>('all');
  const [analysisMode, setAnalysisMode] = useState<'file' | 'selection' | 'project'>('file');

  // Mock Tauri backend integration - replace with actual calls when ready
  const analyzeCode = useCallback(async () => {
    if (!currentFile && analysisMode === 'file') return;
    
    setIsAnalyzing(true);
    try {
      // Mock code analysis - replace with actual Tauri invoke
      const mockEntities: CodeEntity[] = [
        {
          id: 'entity_1',
          name: 'calculateComplexity',
          type: 'function',
          file_path: currentFile || 'src/utils/complexity.ts',
          line_number: 45,
          complexity_score: 8.5,
          dependencies: ['lodash', 'ast-parser'],
          dependents: ['CodeAnalyzer', 'QualityGate'],
          last_modified: '2025-01-06T10:30:00Z',
          ai_insights: {
            quality_score: 7.2,
            refactor_suggestions: [
              'Extract nested conditions into separate functions',
              'Consider using early returns to reduce nesting'
            ],
            potential_bugs: [
              'Possible null pointer exception on line 52'
            ],
            optimization_opportunities: [
              'Cache computation results for repeated calls',
              'Use memoization for expensive calculations'
            ]
          }
        },
        {
          id: 'entity_2',
          name: 'DataProcessor',
          type: 'class',
          file_path: currentFile || 'src/processors/DataProcessor.ts',
          line_number: 12,
          complexity_score: 12.3,
          dependencies: ['rxjs', 'validator'],
          dependents: ['MainService', 'BackgroundWorker'],
          last_modified: '2025-01-05T14:22:00Z',
          ai_insights: {
            quality_score: 6.1,
            refactor_suggestions: [
              'Split class into smaller, focused classes',
              'Extract interface for better testability'
            ],
            potential_bugs: [
              'Race condition in async processing',
              'Memory leak in event listeners'
            ],
            optimization_opportunities: [
              'Implement lazy loading for heavy operations',
              'Add connection pooling for database operations'
            ]
          }
        }
      ];

      setCodeEntities(mockEntities);
    } catch (error) {
      console.error('Code analysis failed:', error);
    } finally {
      setIsAnalyzing(false);
    }
  }, [currentFile, analysisMode]);

  const findDuplications = useCallback(async () => {
    setIsAnalyzing(true);
    try {
      // Mock duplication detection - replace with actual Tauri invoke
      const mockDuplications: DuplicationResult[] = [
        {
          id: 'dup_1',
          similarity_score: 0.92,
          code_blocks: [
            {
              file_path: 'src/utils/validation.ts',
              start_line: 15,
              end_line: 28,
              code_snippet: 'function validateEmail(email: string) {\n  const regex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/;\n  return regex.test(email);\n}'
            },
            {
              file_path: 'src/auth/helpers.ts',
              start_line: 42,
              end_line: 55,
              code_snippet: 'function isValidEmail(email: string) {\n  const emailRegex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/;\n  return emailRegex.test(email);\n}'
            }
          ],
          refactor_suggestion: 'Extract common email validation logic into shared utility function',
          estimated_savings: {
            lines_of_code: 14,
            maintenance_effort: 'Medium - Reduces code duplication and improves consistency'
          }
        }
      ];

      setDuplications(mockDuplications);
    } catch (error) {
      console.error('Duplication analysis failed:', error);
    } finally {
      setIsAnalyzing(false);
    }
  }, []);

  const analyzeContext = useCallback(async () => {
    if (!currentFile) return;
    
    setIsAnalyzing(true);
    try {
      // Mock context analysis - replace with actual Tauri invoke
      const mockInsights: ContextInsight[] = [
        {
          type: 'dependency',
          title: 'High Coupling Detected',
          description: 'This file has dependencies on 12 other modules, indicating high coupling',
          confidence: 0.85,
          related_files: ['src/core/engine.ts', 'src/utils/helpers.ts', 'src/types/interfaces.ts'],
          action_items: [
            'Consider dependency injection to reduce coupling',
            'Extract common interfaces to separate module'
          ]
        },
        {
          type: 'impact',
          title: 'Critical Path Component',
          description: 'Changes to this file affect 23 other components in the system',
          confidence: 0.91,
          related_files: ['src/components/**/*.tsx', 'src/services/**/*.ts'],
          action_items: [
            'Add comprehensive unit tests before making changes',
            'Consider backward compatibility for API changes'
          ]
        }
      ];

      setContextInsights(mockInsights);
    } catch (error) {
      console.error('Context analysis failed:', error);
    } finally {
      setIsAnalyzing(false);
    }
  }, [currentFile]);

  useEffect(() => {
    if (activeTab === 'analysis') {
      analyzeCode();
    } else if (activeTab === 'duplicates') {
      findDuplications();
    } else if (activeTab === 'context') {
      analyzeContext();
    }
  }, [activeTab, currentFile, analyzeCode, findDuplications, analyzeContext]);

  const getComplexityColor = (score: number) => {
    if (score < 5) return '#28a745'; // Green
    if (score < 10) return '#ffc107'; // Yellow
    return '#dc3545'; // Red
  };

  const getQualityColor = (score: number) => {
    if (score >= 8) return '#28a745'; // Green
    if (score >= 6) return '#ffc107'; // Yellow
    return '#dc3545'; // Red
  };

  const filteredEntities = codeEntities.filter(entity => {
    const matchesSearch = entity.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         entity.file_path.toLowerCase().includes(searchQuery.toLowerCase());
    
    switch (filterType) {
      case 'high-complexity':
        return matchesSearch && entity.complexity_score > 10;
      case 'needs-refactor':
        return matchesSearch && entity.ai_insights.refactor_suggestions.length > 0;
      case 'bugs':
        return matchesSearch && entity.ai_insights.potential_bugs.length > 0;
      default:
        return matchesSearch;
    }
  });

  return (
    <div className="code-intelligence-panel">
      <div className="panel-header">
        <h3>🧠 Code Intelligence</h3>
        <div className="header-controls">
          <select
            value={analysisMode}
            onChange={(e) => setAnalysisMode(e.target.value as any)}
            className="analysis-mode-select"
          >
            <option value="file">Current File</option>
            <option value="selection">Selection</option>
            <option value="project">Entire Project</option>
          </select>
          
          <button
            onClick={() => {
              if (activeTab === 'analysis') analyzeCode();
              else if (activeTab === 'duplicates') findDuplications();
              else if (activeTab === 'context') analyzeContext();
            }}
            disabled={isAnalyzing}
            className="refresh-button"
          >
            {isAnalyzing ? '⏳' : '🔄'} Analyze
          </button>
        </div>
      </div>

      <div className="panel-tabs">
        <button
          className={`tab ${activeTab === 'analysis' ? 'active' : ''}`}
          onClick={() => setActiveTab('analysis')}
        >
          📊 Analysis
        </button>
        <button
          className={`tab ${activeTab === 'duplicates' ? 'active' : ''}`}
          onClick={() => setActiveTab('duplicates')}
        >
          🔍 Duplicates
        </button>
        <button
          className={`tab ${activeTab === 'context' ? 'active' : ''}`}
          onClick={() => setActiveTab('context')}
        >
          🌐 Context
        </button>
        <button
          className={`tab ${activeTab === 'insights' ? 'active' : ''}`}
          onClick={() => setActiveTab('insights')}
        >
          💡 Insights
        </button>
      </div>

      <div className="panel-content">
        {activeTab === 'analysis' && (
          <div className="analysis-tab">
            <div className="analysis-controls">
              <input
                type="text"
                placeholder="Search entities..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="search-input"
              />
              
              <select
                value={filterType}
                onChange={(e) => setFilterType(e.target.value as any)}
                className="filter-select"
              >
                <option value="all">All Entities</option>
                <option value="high-complexity">High Complexity</option>
                <option value="needs-refactor">Needs Refactor</option>
                <option value="bugs">Potential Bugs</option>
              </select>
            </div>

            <div className="entities-list">
              {filteredEntities.map(entity => (
                <div key={entity.id} className="entity-card">
                  <div className="entity-header">
                    <div className="entity-info">
                      <span className="entity-type">{entity.type}</span>
                      <h4 className="entity-name">{entity.name}</h4>
                      <span className="entity-file">{entity.file_path}:{entity.line_number}</span>
                    </div>
                    
                    <div className="entity-metrics">
                      <div className="metric">
                        <span className="metric-label">Complexity</span>
                        <span 
                          className="metric-value"
                          style={{ color: getComplexityColor(entity.complexity_score) }}
                        >
                          {entity.complexity_score.toFixed(1)}
                        </span>
                      </div>
                      <div className="metric">
                        <span className="metric-label">Quality</span>
                        <span 
                          className="metric-value"
                          style={{ color: getQualityColor(entity.ai_insights.quality_score) }}
                        >
                          {entity.ai_insights.quality_score.toFixed(1)}
                        </span>
                      </div>
                    </div>
                  </div>

                  <div className="entity-insights">
                    {entity.ai_insights.potential_bugs.length > 0 && (
                      <div className="insight-section bugs">
                        <h5>🐛 Potential Bugs</h5>
                        <ul>
                          {entity.ai_insights.potential_bugs.map((bug, index) => (
                            <li key={index}>{bug}</li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {entity.ai_insights.refactor_suggestions.length > 0 && (
                      <div className="insight-section refactor">
                        <h5>🔧 Refactor Suggestions</h5>
                        <ul>
                          {entity.ai_insights.refactor_suggestions.map((suggestion, index) => (
                            <li key={index}>
                              {suggestion}
                              <button
                                onClick={() => onApplyRefactor(suggestion)}
                                className="apply-suggestion-btn"
                              >
                                Apply
                              </button>
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {entity.ai_insights.optimization_opportunities.length > 0 && (
                      <div className="insight-section optimization">
                        <h5>⚡ Optimizations</h5>
                        <ul>
                          {entity.ai_insights.optimization_opportunities.map((opt, index) => (
                            <li key={index}>{opt}</li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>

                  <div className="entity-actions">
                    <button
                      onClick={() => onNavigateToCode(entity.file_path, entity.line_number)}
                      className="navigate-btn"
                    >
                      📍 Go to Code
                    </button>
                    
                    <button className="analyze-dependencies-btn">
                      🔗 Dependencies ({entity.dependencies.length})
                    </button>
                    
                    <button className="analyze-impact-btn">
                      📈 Impact ({entity.dependents.length})
                    </button>
                  </div>
                </div>
              ))}
            </div>

            {filteredEntities.length === 0 && !isAnalyzing && (
              <div className="empty-state">
                <h4>No entities found</h4>
                <p>Try adjusting your search or filter criteria</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'duplicates' && (
          <div className="duplicates-tab">
            <div className="duplicates-header">
              <h4>Code Duplication Analysis</h4>
              <div className="duplication-stats">
                <span>Found {duplications.length} duplication groups</span>
              </div>
            </div>

            <div className="duplications-list">
              {duplications.map(dup => (
                <div key={dup.id} className="duplication-card">
                  <div className="duplication-header">
                    <div className="similarity-score">
                      <span className="score-label">Similarity</span>
                      <span className="score-value">
                        {(dup.similarity_score * 100).toFixed(1)}%
                      </span>
                    </div>
                    
                    <div className="savings-info">
                      <span>Potential savings: {dup.estimated_savings.lines_of_code} LOC</span>
                    </div>
                  </div>

                  <div className="code-blocks">
                    {dup.code_blocks.map((block, index) => (
                      <div key={index} className="code-block">
                        <div className="block-header">
                          <span className="file-path">{block.file_path}</span>
                          <span className="line-range">
                            Lines {block.start_line}-{block.end_line}
                          </span>
                          <button
                            onClick={() => onNavigateToCode(block.file_path, block.start_line)}
                            className="navigate-to-block-btn"
                          >
                            📍 Go
                          </button>
                        </div>
                        <pre className="code-snippet">{block.code_snippet}</pre>
                      </div>
                    ))}
                  </div>

                  <div className="refactor-suggestion">
                    <h5>💡 Refactor Suggestion</h5>
                    <p>{dup.refactor_suggestion}</p>
                    <div className="suggestion-actions">
                      <button
                        onClick={() => onApplyRefactor(dup.refactor_suggestion)}
                        className="apply-refactor-btn"
                      >
                        🔧 Apply Refactor
                      </button>
                      <button className="preview-refactor-btn">
                        👁️ Preview Changes
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {duplications.length === 0 && !isAnalyzing && (
              <div className="empty-state">
                <h4>🎉 No duplications found</h4>
                <p>Your code is well-structured with minimal duplication</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'context' && (
          <div className="context-tab">
            <div className="context-header">
              <h4>Contextual Code Insights</h4>
              <p>Understanding relationships and impact in your codebase</p>
            </div>

            <div className="insights-list">
              {contextInsights.map((insight, index) => (
                <div key={index} className={`insight-card ${insight.type}`}>
                  <div className="insight-header">
                    <div className="insight-info">
                      <h5>{insight.title}</h5>
                      <span className="insight-type">{insight.type}</span>
                    </div>
                    
                    <div className="confidence-score">
                      <span className="confidence-label">Confidence</span>
                      <span className="confidence-value">
                        {(insight.confidence * 100).toFixed(0)}%
                      </span>
                    </div>
                  </div>

                  <p className="insight-description">{insight.description}</p>

                  <div className="related-files">
                    <h6>Related Files</h6>
                    <div className="files-list">
                      {insight.related_files.map((file, fileIndex) => (
                        <button
                          key={fileIndex}
                          onClick={() => onNavigateToCode(file, 1)}
                          className="related-file-btn"
                        >
                          📄 {file}
                        </button>
                      ))}
                    </div>
                  </div>

                  <div className="action-items">
                    <h6>Recommended Actions</h6>
                    <ul>
                      {insight.action_items.map((action, actionIndex) => (
                        <li key={actionIndex}>{action}</li>
                      ))}
                    </ul>
                  </div>
                </div>
              ))}
            </div>

            {contextInsights.length === 0 && !isAnalyzing && (
              <div className="empty-state">
                <h4>No context insights available</h4>
                <p>Select a file to analyze its contextual relationships</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'insights' && (
          <div className="insights-tab">
            <div className="insights-overview">
              <h4>🎯 AI-Powered Code Insights</h4>
              <p>Advanced analysis and recommendations for your codebase</p>
            </div>

            <div className="insight-categories">
              <div className="insight-category">
                <h5>🏗️ Architecture Insights</h5>
                <div className="insight-items">
                  <div className="insight-item">
                    <span className="insight-icon">🔗</span>
                    <div className="insight-content">
                      <h6>Dependency Graph Analysis</h6>
                      <p>Your codebase has a healthy dependency structure with minimal circular dependencies</p>
                    </div>
                  </div>
                  
                  <div className="insight-item">
                    <span className="insight-icon">📊</span>
                    <div className="insight-content">
                      <h6>Module Cohesion</h6>
                      <p>Consider splitting large modules for better maintainability</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="insight-category">
                <h5>⚡ Performance Insights</h5>
                <div className="insight-items">
                  <div className="insight-item">
                    <span className="insight-icon">🚀</span>
                    <div className="insight-content">
                      <h6>Bundle Size Optimization</h6>
                      <p>Tree-shaking opportunities detected in utility modules</p>
                    </div>
                  </div>
                  
                  <div className="insight-item">
                    <span className="insight-icon">💾</span>
                    <div className="insight-content">
                      <h6>Memory Usage Patterns</h6>
                      <p>Potential memory leaks detected in event listeners</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="insight-category">
                <h5>🛡️ Security Insights</h5>
                <div className="insight-items">
                  <div className="insight-item">
                    <span className="insight-icon">🔒</span>
                    <div className="insight-content">
                      <h6>Input Validation</h6>
                      <p>All user inputs are properly validated and sanitized</p>
                    </div>
                  </div>
                  
                  <div className="insight-item">
                    <span className="insight-icon">🔑</span>
                    <div className="insight-content">
                      <h6>API Security</h6>
                      <p>Consider implementing rate limiting for API endpoints</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {isAnalyzing && (
          <div className="loading-overlay">
            <div className="loading-spinner">⏳</div>
            <p>Analyzing code with AI...</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default CodeIntelligencePanel;
