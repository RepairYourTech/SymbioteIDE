import React, { useState } from 'react';
import { DashboardSettings, RefreshInterval } from '../../../types/dashboard-types';

interface SettingsPanelProps {
  settings: DashboardSettings;
  onSettingsChange: (settings: DashboardSettings) => void;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ settings, onSettingsChange }) => {
  const [localSettings, setLocalSettings] = useState<DashboardSettings>(settings);
  const [hasChanges, setHasChanges] = useState(false);
  
  const handleChange = (key: keyof DashboardSettings, value: any) => {
    setLocalSettings(prev => ({
      ...prev,
      [key]: value
    }));
    setHasChanges(true);
  };
  
  const handleThemePreferenceChange = (key: keyof DashboardSettings['themePreferences'], value: any) => {
    setLocalSettings(prev => ({
      ...prev,
      themePreferences: {
        ...prev.themePreferences,
        [key]: value
      }
    }));
    setHasChanges(true);
  };
  
  const handleNotificationChange = (key: keyof DashboardSettings['notifications'], value: any) => {
    setLocalSettings(prev => ({
      ...prev,
      notifications: {
        ...prev.notifications,
        [key]: value
      }
    }));
    setHasChanges(true);
  };
  
  const handleLayoutItemToggle = (item: string) => {
    setLocalSettings(prev => {
      const items = new Set(prev.layout.visiblePanels);
      if (items.has(item)) {
        items.delete(item);
      } else {
        items.add(item);
      }
      return {
        ...prev,
        layout: {
          ...prev.layout,
          visiblePanels: Array.from(items)
        }
      };
    });
    setHasChanges(true);
  };
  
  const handleSave = () => {
    onSettingsChange(localSettings);
    setHasChanges(false);
  };
  
  const handleReset = () => {
    setLocalSettings(settings);
    setHasChanges(false);
  };
  
  const availablePanels = [
    { id: 'system-overview', name: 'System Overview' },
    { id: 'agents', name: 'Agents' },
    { id: 'orchestration-metrics', name: 'Orchestration Metrics' },
    { id: 'cost-tracking', name: 'Cost Tracking' },
    { id: 'activity-log', name: 'Activity Log' },
    { id: 'mcp-servers', name: 'MCP Servers' }
  ];
  
  return (
    <div className="panel settings-panel">
      <div className="panel-header">
        <h2>Dashboard Settings</h2>
        {hasChanges && (
          <div className="settings-actions">
            <button className="button secondary" onClick={handleReset}>Reset</button>
            <button className="button primary" onClick={handleSave}>Save Changes</button>
          </div>
        )}
      </div>
      
      <div className="panel-content">
        <div className="settings-section">
          <h3>General Settings</h3>
          
          <div className="setting-item">
            <label htmlFor="refresh-interval">Refresh Interval</label>
            <select
              id="refresh-interval"
              value={localSettings.refreshInterval}
              onChange={(e) => handleChange('refreshInterval', parseInt(e.target.value) as RefreshInterval)}
            >
              <option value={5000}>5 seconds</option>
              <option value={10000}>10 seconds</option>
              <option value={30000}>30 seconds</option>
              <option value={60000}>1 minute</option>
            </select>
          </div>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="auto-refresh"
              checked={localSettings.autoRefresh}
              onChange={(e) => handleChange('autoRefresh', e.target.checked)}
            />
            <label htmlFor="auto-refresh">Auto Refresh</label>
          </div>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="show-detailed-metrics"
              checked={localSettings.showDetailedMetrics}
              onChange={(e) => handleChange('showDetailedMetrics', e.target.checked)}
            />
            <label htmlFor="show-detailed-metrics">Show Detailed Metrics</label>
          </div>
        </div>
        
        <div className="settings-section">
          <h3>Layout Configuration</h3>
          
          <div className="setting-item">
            <label>Visible Panels</label>
            <div className="panel-checkboxes">
              {availablePanels.map(panel => (
                <div key={panel.id} className="checkbox-item">
                  <input
                    type="checkbox"
                    id={`panel-${panel.id}`}
                    checked={localSettings.layout.visiblePanels.includes(panel.id)}
                    onChange={() => handleLayoutItemToggle(panel.id)}
                  />
                  <label htmlFor={`panel-${panel.id}`}>{panel.name}</label>
                </div>
              ))}
            </div>
          </div>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="compact-mode"
              checked={localSettings.layout.compactMode}
              onChange={(e) => handleChange('layout', {
                ...localSettings.layout,
                compactMode: e.target.checked
              })}
            />
            <label htmlFor="compact-mode">Compact Mode</label>
          </div>
        </div>
        
        <div className="settings-section">
          <h3>Notifications</h3>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="enable-notifications"
              checked={localSettings.notifications.enabled}
              onChange={(e) => handleNotificationChange('enabled', e.target.checked)}
            />
            <label htmlFor="enable-notifications">Enable Notifications</label>
          </div>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="notify-errors"
              checked={localSettings.notifications.errorAlerts}
              onChange={(e) => handleNotificationChange('errorAlerts', e.target.checked)}
              disabled={!localSettings.notifications.enabled}
            />
            <label htmlFor="notify-errors">Error Alerts</label>
          </div>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="notify-cost-alerts"
              checked={localSettings.notifications.costAlerts}
              onChange={(e) => handleNotificationChange('costAlerts', e.target.checked)}
              disabled={!localSettings.notifications.enabled}
            />
            <label htmlFor="notify-cost-alerts">Cost Alerts</label>
          </div>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="notify-performance"
              checked={localSettings.notifications.performanceAlerts}
              onChange={(e) => handleNotificationChange('performanceAlerts', e.target.checked)}
              disabled={!localSettings.notifications.enabled}
            />
            <label htmlFor="notify-performance">Performance Alerts</label>
          </div>
        </div>
        
        <div className="settings-section">
          <h3>Theme Preferences</h3>
          
          <div className="setting-item checkbox">
            <input
              type="checkbox"
              id="use-vscode-theme"
              checked={localSettings.themePreferences.useVSCodeTheme}
              onChange={(e) => handleThemePreferenceChange('useVSCodeTheme', e.target.checked)}
            />
            <label htmlFor="use-vscode-theme">Use VS Code Theme</label>
          </div>
          
          {!localSettings.themePreferences.useVSCodeTheme && (
            <>
              <div className="setting-item">
                <label htmlFor="chart-colors">Chart Color Scheme</label>
                <select
                  id="chart-colors"
                  value={localSettings.themePreferences.chartColorScheme}
                  onChange={(e) => handleThemePreferenceChange('chartColorScheme', e.target.value)}
                >
                  <option value="default">Default</option>
                  <option value="colorblind">Colorblind Friendly</option>
                  <option value="vibrant">Vibrant</option>
                  <option value="pastel">Pastel</option>
                </select>
              </div>
              
              <div className="setting-item checkbox">
                <input
                  type="checkbox"
                  id="high-contrast"
                  checked={localSettings.themePreferences.highContrast}
                  onChange={(e) => handleThemePreferenceChange('highContrast', e.target.checked)}
                />
                <label htmlFor="high-contrast">High Contrast Mode</label>
              </div>
            </>
          )}
        </div>
        
        <div className="settings-section">
          <h3>Export & Import</h3>
          
          <div className="setting-actions">
            <button 
              className="button secondary"
              onClick={() => {
                window.vscode.postMessage({
                  type: 'exportSettings',
                  payload: localSettings
                });
              }}
            >
              Export Settings
            </button>
            
            <button 
              className="button secondary"
              onClick={() => {
                window.vscode.postMessage({
                  type: 'importSettings'
                });
              }}
            >
              Import Settings
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};