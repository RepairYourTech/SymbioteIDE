import React from 'react';
import { OrchestrationMetrics as Metrics, ModelUsageStats } from '../../../types/dashboard-types';
import { LineChart, Line, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { formatNumber, formatDuration, formatPercentage } from '../utils/formatters';

interface OrchestrationMetricsProps {
  metrics: Metrics;
  modelUsage?: ModelUsageStats[];
  detailed?: boolean;
}

export const OrchestrationMetrics: React.FC<OrchestrationMetricsProps> = ({ 
  metrics, 
  modelUsage = [], 
  detailed = false 
}) => {
  const chartTheme = {
    backgroundColor: 'var(--panel-background)',
    textColor: 'var(--foreground)',
    gridColor: 'var(--border)',
    tooltipBackground: 'var(--vscode-editorWidget-background)',
    tooltipBorder: 'var(--vscode-editorWidget-border)'
  };
  
  return (
    <div className="panel orchestration-metrics">
      <div className="panel-header">
        <h2>Orchestration Metrics</h2>
        <div className="metric-summary">
          <span className="metric-item">
            <span className="metric-label">Active:</span>
            <span className="metric-value">{metrics.activeRequests}</span>
          </span>
          <span className="metric-item">
            <span className="metric-label">Queue:</span>
            <span className="metric-value">{metrics.queueLength}</span>
          </span>
        </div>
      </div>
      
      <div className="panel-content">
        <div className="metrics-grid">
          <div className="metric-card">
            <h3>Throughput</h3>
            <div className="metric-stats">
              <div className="metric-stat">
                <span className="stat-label">Requests/min</span>
                <span className="stat-value">{formatNumber(metrics.throughput.requestsPerMinute)}</span>
              </div>
              <div className="metric-stat">
                <span className="stat-label">Tokens/min</span>
                <span className="stat-value">{formatNumber(metrics.throughput.tokensPerMinute)}</span>
              </div>
            </div>
          </div>
          
          <div className="metric-card">
            <h3>Performance</h3>
            <div className="metric-stats">
              <div className="metric-stat">
                <span className="stat-label">Avg Latency</span>
                <span className="stat-value">{formatDuration(metrics.averageLatency)}</span>
              </div>
              <div className="metric-stat">
                <span className="stat-label">Error Rate</span>
                <span className="stat-value error">{formatPercentage(metrics.errorRate)}</span>
              </div>
            </div>
          </div>
        </div>
        
        {metrics.rateLimits.length > 0 && (
          <div className="rate-limits">
            <h3>Rate Limits</h3>
            <div className="limits-list">
              {metrics.rateLimits.map(limit => (
                <div key={limit.provider} className="rate-limit-item">
                  <div className="limit-header">
                    <span className="limit-provider">{limit.provider}</span>
                    <span className="limit-usage">{limit.used} / {limit.limit}</span>
                  </div>
                  <div className="limit-bar">
                    <div 
                      className="limit-fill"
                      style={{ 
                        width: `${(limit.used / limit.limit) * 100}%`,
                        backgroundColor: limit.used / limit.limit > 0.8 ? 'var(--error-color)' : 'var(--success-color)'
                      }}
                    />
                  </div>
                  <span className="limit-reset">Resets: {new Date(limit.resetAt).toLocaleTimeString()}</span>
                </div>
              ))}
            </div>
          </div>
        )}
        
        {detailed && modelUsage.length > 0 && (
          <div className="model-usage-section">
            <h3>Model Usage</h3>
            <div className="usage-chart">
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={modelUsage}>
                  <CartesianGrid strokeDasharray="3 3" stroke={chartTheme.gridColor} />
                  <XAxis dataKey="model" stroke={chartTheme.textColor} />
                  <YAxis stroke={chartTheme.textColor} />
                  <Tooltip 
                    contentStyle={{ 
                      backgroundColor: chartTheme.tooltipBackground,
                      border: `1px solid ${chartTheme.tooltipBorder}`,
                      borderRadius: '4px'
                    }}
                  />
                  <Legend />
                  <Bar dataKey="requests" fill="var(--vscode-charts-blue)" name="Requests" />
                  <Bar dataKey="tokensUsed.total" fill="var(--vscode-charts-green)" name="Tokens" />
                </BarChart>
              </ResponsiveContainer>
            </div>
            
            <div className="usage-table">
              <table>
                <thead>
                  <tr>
                    <th>Model</th>
                    <th>Provider</th>
                    <th>Requests</th>
                    <th>Input Tokens</th>
                    <th>Output Tokens</th>
                    <th>Avg Latency</th>
                    <th>Error Rate</th>
                  </tr>
                </thead>
                <tbody>
                  {modelUsage.map(model => (
                    <tr key={`${model.provider}-${model.model}`}>
                      <td>{model.model}</td>
                      <td>{model.provider}</td>
                      <td>{formatNumber(model.requests)}</td>
                      <td>{formatNumber(model.tokensUsed.input)}</td>
                      <td>{formatNumber(model.tokensUsed.output)}</td>
                      <td>{formatDuration(model.averageLatency)}</td>
                      <td className={model.errorRate > 0.05 ? 'error' : ''}>
                        {formatPercentage(model.errorRate)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};