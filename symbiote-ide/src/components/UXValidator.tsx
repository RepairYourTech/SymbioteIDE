/**
 * UX Validation Component for SymbioteIDE
 * Real-time UX monitoring and validation integrated into the main interface
 */

import React, { useState, useEffect } from 'react';
import { uxOptimizer, UXValidationResult } from '../utils/UXOptimizer';
import './UXValidator.css';

interface UXValidatorProps {
  isVisible: boolean;
  onClose: () => void;
}

export const UXValidator: React.FC<UXValidatorProps> = ({ isVisible, onClose }) => {
  const [validationResult, setValidationResult] = useState<UXValidationResult | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [autoValidation, setAutoValidation] = useState(true);

  useEffect(() => {
    if (autoValidation) {
      const interval = setInterval(() => {
        runValidation();
      }, 30000); // Run validation every 30 seconds

      return () => clearInterval(interval);
    }
  }, [autoValidation]);

  const runValidation = async () => {
    setIsRunning(true);
    try {
      // Simulate async validation
      await new Promise(resolve => setTimeout(resolve, 1000));
      const result = uxOptimizer.validateUX();
      setValidationResult(result);
    } catch (error) {
      console.error('UX validation failed:', error);
    } finally {
      setIsRunning(false);
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical': return '#ff4444';
      case 'high': return '#ff8800';
      case 'medium': return '#ffaa00';
      case 'low': return '#44aa44';
      default: return '#666666';
    }
  };

  const getPriorityColor = (priority: string) => {
    switch (priority) {
      case 'high': return '#ff4444';
      case 'medium': return '#ffaa00';
      case 'low': return '#44aa44';
      default: return '#666666';
    }
  };

  const getScoreColor = (score: number) => {
    if (score >= 90) return '#44aa44';
    if (score >= 80) return '#88aa44';
    if (score >= 70) return '#aaaa44';
    if (score >= 60) return '#aa8844';
    return '#aa4444';
  };

  if (!isVisible) return null;

  return (
    <div className="ux-validator-overlay">
      <div className="ux-validator-modal">
        <div className="ux-validator-header">
          <h2>🎯 UX Quality Validator</h2>
          <div className="ux-validator-controls">
            <label className="auto-validation-toggle">
              <input
                type="checkbox"
                checked={autoValidation}
                onChange={(e) => setAutoValidation(e.target.checked)}
              />
              Auto-validation
            </label>
            <button
              className="run-validation-btn"
              onClick={runValidation}
              disabled={isRunning}
            >
              {isRunning ? '🔄 Running...' : '▶️ Run Validation'}
            </button>
            <button className="close-btn" onClick={onClose}>×</button>
          </div>
        </div>

        <div className="ux-validator-content">
          {isRunning && (
            <div className="validation-loading">
              <div className="loading-spinner"></div>
              <p>Analyzing UX quality...</p>
            </div>
          )}

          {validationResult && !isRunning && (
            <>
              <div className="validation-summary">
                <div className="score-display">
                  <div 
                    className="score-circle"
                    style={{ borderColor: getScoreColor(validationResult.score) }}
                  >
                    <span 
                      className="score-number"
                      style={{ color: getScoreColor(validationResult.score) }}
                    >
                      {validationResult.score}
                    </span>
                    <span className="score-label">/100</span>
                  </div>
                  <div className="score-status">
                    {validationResult.isValid ? (
                      <span className="status-valid">✅ UX Quality: Excellent</span>
                    ) : (
                      <span className="status-invalid">❌ UX Quality: Needs Improvement</span>
                    )}
                  </div>
                </div>

                <div className="metrics-grid">
                  <div className="metric-card">
                    <h4>⚡ Performance</h4>
                    <div className="metric-value">
                      {uxOptimizer.getMetrics().loadTime.toFixed(0)}ms
                    </div>
                    <div className="metric-label">Load Time</div>
                  </div>
                  <div className="metric-card">
                    <h4>🖱️ Interaction</h4>
                    <div className="metric-value">
                      {uxOptimizer.getMetrics().interactionLatency.toFixed(0)}ms
                    </div>
                    <div className="metric-label">Latency</div>
                  </div>
                  <div className="metric-card">
                    <h4>🧠 Memory</h4>
                    <div className="metric-value">
                      {uxOptimizer.getMetrics().memoryUsage.toFixed(0)}MB
                    </div>
                    <div className="metric-label">Usage</div>
                  </div>
                  <div className="metric-card">
                    <h4>♿ Accessibility</h4>
                    <div className="metric-value">
                      {uxOptimizer.getMetrics().accessibilityScore}/100
                    </div>
                    <div className="metric-label">Score</div>
                  </div>
                </div>
              </div>

              {validationResult.issues.length > 0 && (
                <div className="issues-section">
                  <h3>🚨 Issues Found ({validationResult.issues.length})</h3>
                  <div className="issues-list">
                    {validationResult.issues.map((issue, index) => (
                      <div key={index} className="issue-item">
                        <div className="issue-header">
                          <span 
                            className="issue-severity"
                            style={{ backgroundColor: getSeverityColor(issue.severity) }}
                          >
                            {issue.severity.toUpperCase()}
                          </span>
                          <span className="issue-category">{issue.category}</span>
                          <span className="issue-component">{issue.component}</span>
                        </div>
                        <div className="issue-description">{issue.description}</div>
                        {issue.fix && (
                          <div className="issue-fix">
                            <strong>💡 Fix:</strong> {issue.fix}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {validationResult.recommendations.length > 0 && (
                <div className="recommendations-section">
                  <h3>💡 Recommendations ({validationResult.recommendations.length})</h3>
                  <div className="recommendations-list">
                    {validationResult.recommendations.map((rec, index) => (
                      <div key={index} className="recommendation-item">
                        <div className="recommendation-header">
                          <span 
                            className="recommendation-priority"
                            style={{ backgroundColor: getPriorityColor(rec.priority) }}
                          >
                            {rec.priority.toUpperCase()}
                          </span>
                          <span className="recommendation-category">{rec.category}</span>
                          <span className="recommendation-effort">Effort: {rec.effort}</span>
                        </div>
                        <div className="recommendation-description">{rec.description}</div>
                        <div className="recommendation-impact">
                          <strong>📈 Impact:</strong> {rec.impact}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <div className="validation-actions">
                <button 
                  className="export-report-btn"
                  onClick={() => {
                    const report = uxOptimizer.generateReport();
                    const blob = new Blob([report], { type: 'text/markdown' });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = 'ux-quality-report.md';
                    a.click();
                    URL.revokeObjectURL(url);
                  }}
                >
                  📄 Export Report
                </button>
                <button 
                  className="run-validation-btn"
                  onClick={runValidation}
                >
                  🔄 Re-run Validation
                </button>
              </div>
            </>
          )}

          {!validationResult && !isRunning && (
            <div className="no-validation">
              <p>Click "Run Validation" to analyze UX quality</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default UXValidator;
