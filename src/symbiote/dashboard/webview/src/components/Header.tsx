import React from 'react';
import classNames from 'classnames';

interface HeaderProps {
  activeView: string;
  onViewChange: (view: any) => void;
  onRefresh: () => void;
}

export const Header: React.FC<HeaderProps> = ({ activeView, onViewChange, onRefresh }) => {
  const views = [
    { id: 'overview', label: 'Overview', icon: '📊' },
    { id: 'agents', label: 'Agents', icon: '🤖' },
    { id: 'metrics', label: 'Metrics', icon: '📈' },
    { id: 'costs', label: 'Costs', icon: '💰' },
    { id: 'logs', label: 'Activity', icon: '📜' },
    { id: 'settings', label: 'Settings', icon: '⚙️' }
  ];
  
  return (
    <header className="dashboard-header">
      <div className="header-title">
        <h1>SymbioteIDE Dashboard</h1>
      </div>
      
      <nav className="header-nav">
        {views.map(view => (
          <button
            key={view.id}
            className={classNames('nav-button', {
              active: activeView === view.id
            })}
            onClick={() => onViewChange(view.id)}
          >
            <span className="nav-icon">{view.icon}</span>
            <span className="nav-label">{view.label}</span>
          </button>
        ))}
      </nav>
      
      <div className="header-actions">
        <button className="refresh-button" onClick={onRefresh} title="Refresh">
          🔄
        </button>
      </div>
    </header>
  );
};