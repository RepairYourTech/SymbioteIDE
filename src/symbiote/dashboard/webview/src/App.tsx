import React, { useState, useEffect, useCallback } from 'react';
import { DashboardState, DashboardMessage, DashboardMessageType } from '../../types/dashboard-types';
import { useVSCode } from './hooks/useVSCode';
import { Header } from './components/Header';
import { SystemOverview } from './components/SystemOverview';
import { AgentsPanel } from './components/AgentsPanel';
import { OrchestrationMetrics } from './components/OrchestrationMetrics';
import { CostTracking } from './components/CostTracking';
import { ActivityLog } from './components/ActivityLog';
import { SettingsPanel } from './components/SettingsPanel';
import GridLayout from 'react-grid-layout';
import 'react-grid-layout/css/styles.css';
import 'react-resizable/css/styles.css';
import './styles/app.css';

const App: React.FC = () => {
  const vscode = useVSCode();
  const [state, setState] = useState<DashboardState | null>(null);
  const [activeView, setActiveView] = useState<'overview' | 'agents' | 'metrics' | 'costs' | 'logs' | 'settings'>('overview');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  // Handle messages from extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent<DashboardMessage>) => {
      const message = event.data;
      
      switch (message.type) {
        case DashboardMessageType.STATE_UPDATE:
          setState(message.payload);
          setLoading(false);
          setError(null);
          break;
          
        case DashboardMessageType.PARTIAL_UPDATE:
          setState(prev => prev ? { ...prev, ...message.payload } : null);
          break;
          
        case DashboardMessageType.ERROR:
          setError(message.payload.message);
          setLoading(false);
          break;
          
        case DashboardMessageType.METRIC_UPDATE:
        case DashboardMessageType.AGENT_UPDATE:
        case DashboardMessageType.LOG_ENTRY:
        case DashboardMessageType.EXECUTION_UPDATE:
          // Handle real-time updates
          handleRealtimeUpdate(message);
          break;
      }
    };
    
    window.addEventListener('message', handleMessage);
    
    // Send ready message
    vscode.postMessage({
      type: DashboardMessageType.READY
    });
    
    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [vscode]);
  
  const handleRealtimeUpdate = useCallback((message: DashboardMessage) => {
    setState(prev => {
      if (!prev) return null;
      
      switch (message.type) {
        case DashboardMessageType.METRIC_UPDATE:
          return {
            ...prev,
            orchestrationMetrics: {
              ...prev.orchestrationMetrics,
              ...message.payload
            }
          };
          
        case DashboardMessageType.AGENT_UPDATE:
          return {
            ...prev,
            agents: prev.agents.map(agent => 
              agent.id === message.payload.id 
                ? { ...agent, ...message.payload }
                : agent
            )
          };
          
        case DashboardMessageType.LOG_ENTRY:
          return {
            ...prev,
            activityLog: [message.payload, ...prev.activityLog].slice(0, 100)
          };
          
        case DashboardMessageType.EXECUTION_UPDATE:
          return {
            ...prev,
            executionHistory: prev.executionHistory.map(exec =>
              exec.id === message.payload.id
                ? { ...exec, ...message.payload }
                : exec
            )
          };
          
        default:
          return prev;
      }
    });
  }, []);
  
  const refresh = useCallback((sections?: string[]) => {
    vscode.postMessage({
      type: DashboardMessageType.REFRESH,
      payload: { sections }
    });
  }, [vscode]);
  
  const executeAction = useCallback((action: any) => {
    vscode.postMessage({
      type: DashboardMessageType.EXECUTE_ACTION,
      payload: action
    });
  }, [vscode]);
  
  const updateSettings = useCallback((settings: any) => {
    vscode.postMessage({
      type: DashboardMessageType.UPDATE_SETTINGS,
      payload: settings
    });
  }, [vscode]);
  
  if (loading) {
    return (
      <div className="dashboard loading">
        <div className="loading-spinner">
          <div className="spinner"></div>
          <p>Loading dashboard...</p>
        </div>
      </div>
    );
  }
  
  if (error) {
    return (
      <div className="dashboard error">
        <div className="error-container">
          <h2>Error Loading Dashboard</h2>
          <p>{error}</p>
          <button onClick={() => refresh()}>Retry</button>
        </div>
      </div>
    );
  }
  
  if (!state) {
    return null;
  }
  
  return (
    <div className="dashboard">
      <Header 
        activeView={activeView}
        onViewChange={setActiveView}
        onRefresh={() => refresh()}
      />
      
      <div className="dashboard-content">
        {activeView === 'overview' && (
          <div className="overview-grid">
            <SystemOverview systemStatus={state.systemStatus} memoryUsage={state.memoryUsage} />
            <AgentsPanel 
              agents={state.agents} 
              onToggleAgent={(id, enabled) => executeAction({ type: 'toggleAgent', payload: { agentId: id, enabled }})}
            />
            <OrchestrationMetrics metrics={state.orchestrationMetrics} />
            <CostTracking costs={state.costTracking} />
          </div>
        )}
        
        {activeView === 'agents' && (
          <AgentsPanel 
            agents={state.agents}
            detailed={true}
            onToggleAgent={(id, enabled) => executeAction({ type: 'toggleAgent', payload: { agentId: id, enabled }})}
          />
        )}
        
        {activeView === 'metrics' && (
          <OrchestrationMetrics 
            metrics={state.orchestrationMetrics}
            modelUsage={state.modelUsage}
            detailed={true}
          />
        )}
        
        {activeView === 'costs' && (
          <CostTracking 
            costs={state.costTracking}
            modelUsage={state.modelUsage}
            detailed={true}
          />
        )}
        
        {activeView === 'logs' && (
          <ActivityLog 
            logs={state.activityLog}
          />
        )}
        
        {activeView === 'settings' && (
          <SettingsPanel
            settings={state.userSettings}
            onSettingsChange={updateSettings}
          />
        )}
      </div>
    </div>
  );
};

export default App;