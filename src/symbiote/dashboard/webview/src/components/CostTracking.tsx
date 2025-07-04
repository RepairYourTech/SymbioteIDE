import React from 'react';
import { CostMetrics, ModelUsageStats } from '../../../types/dashboard-types';
import { PieChart, Pie, Cell, LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { formatCurrency, formatDate } from '../utils/formatters';

interface CostTrackingProps {
  costs: CostMetrics;
  modelUsage?: ModelUsageStats[];
  detailed?: boolean;
}

const COLORS = [
  'var(--vscode-charts-blue)',
  'var(--vscode-charts-green)',
  'var(--vscode-charts-red)',
  'var(--vscode-charts-orange)',
  'var(--vscode-charts-purple)',
  'var(--vscode-charts-yellow)'
];

export const CostTracking: React.FC<CostTrackingProps> = ({ costs, modelUsage = [], detailed = false }) => {
  const providerData = Object.entries(costs.currentPeriod.costByProvider).map(([name, value]) => ({
    name,
    value
  }));
  
  const modelData = Object.entries(costs.currentPeriod.costByModel).map(([name, value]) => ({
    name,
    value
  }));
  
  const budgetPercentage = costs.budget ? (costs.budget.used / costs.budget.limit) * 100 : 0;
  const budgetStatus = budgetPercentage > costs.budget?.alertThreshold! ? 'warning' : 'normal';
  
  return (
    <div className="panel cost-tracking">
      <div className="panel-header">
        <h2>Cost Tracking</h2>
        <div className="cost-summary">
          <span className="current-cost">{formatCurrency(costs.currentPeriod.totalCost)}</span>
          <span className="cost-period">
            {formatDate(costs.currentPeriod.start)} - {formatDate(costs.currentPeriod.end)}
          </span>
        </div>
      </div>
      
      <div className="panel-content">
        {costs.budget && (
          <div className="budget-status">
            <div className="budget-header">
              <h3>Budget Status</h3>
              <span className={`budget-percentage ${budgetStatus}`}>
                {budgetPercentage.toFixed(1)}% used
              </span>
            </div>
            <div className="budget-bar">
              <div 
                className={`budget-fill ${budgetStatus}`}
                style={{ width: `${Math.min(budgetPercentage, 100)}%` }}
              />
              {costs.budget.alertThreshold && (
                <div 
                  className="budget-threshold"
                  style={{ left: `${costs.budget.alertThreshold}%` }}
                  title={`Alert threshold: ${costs.budget.alertThreshold}%`}
                />
              )}
            </div>
            <div className="budget-details">
              <span>Used: {formatCurrency(costs.budget.used)}</span>
              <span>Limit: {formatCurrency(costs.budget.limit)}</span>
              <span>Remaining: {formatCurrency(costs.budget.limit - costs.budget.used)}</span>
            </div>
          </div>
        )}
        
        <div className="cost-projections">
          <h3>Projections</h3>
          <div className="projection-grid">
            <div className="projection-item">
              <span className="projection-label">Daily</span>
              <span className="projection-value">{formatCurrency(costs.projection.daily)}</span>
            </div>
            <div className="projection-item">
              <span className="projection-label">Weekly</span>
              <span className="projection-value">{formatCurrency(costs.projection.weekly)}</span>
            </div>
            <div className="projection-item">
              <span className="projection-label">Monthly</span>
              <span className="projection-value">{formatCurrency(costs.projection.monthly)}</span>
            </div>
          </div>
        </div>
        
        <div className="cost-breakdown">
          <h3>Cost Breakdown</h3>
          <div className="breakdown-charts">
            <div className="chart-container">
              <h4>By Provider</h4>
              <ResponsiveContainer width="100%" height={200}>
                <PieChart>
                  <Pie
                    data={providerData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={entry => `${entry.name}: ${formatCurrency(entry.value)}`}
                    outerRadius={80}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {providerData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value: number) => formatCurrency(value)} />
                </PieChart>
              </ResponsiveContainer>
            </div>
            
            <div className="chart-container">
              <h4>By Model</h4>
              <ResponsiveContainer width="100%" height={200}>
                <PieChart>
                  <Pie
                    data={modelData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={entry => `${entry.name}: ${formatCurrency(entry.value)}`}
                    outerRadius={80}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {modelData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value: number) => formatCurrency(value)} />
                </PieChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>
        
        {detailed && modelUsage.length > 0 && (
          <div className="cost-details-table">
            <h3>Model Cost Details</h3>
            <table>
              <thead>
                <tr>
                  <th>Model</th>
                  <th>Provider</th>
                  <th>Requests</th>
                  <th>Total Tokens</th>
                  <th>Cost</th>
                  <th>Cost/Request</th>
                </tr>
              </thead>
              <tbody>
                {modelUsage.map(model => (
                  <tr key={`${model.provider}-${model.model}`}>
                    <td>{model.model}</td>
                    <td>{model.provider}</td>
                    <td>{model.requests.toLocaleString()}</td>
                    <td>{model.tokensUsed.total.toLocaleString()}</td>
                    <td>{formatCurrency(model.cost)}</td>
                    <td>{formatCurrency(model.cost / model.requests)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
};