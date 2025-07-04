/**
 * Security Panel UI
 * 
 * React component for MCP security management interface
 */

import React, { useState, useEffect, useCallback } from 'react';
import { 
  SecurityPolicy, 
  SecurityRule, 
  TrustLevel,
  PermissionRequest,
  SecurityIncident,
  SecurityScanResult
} from '../types';

interface SecurityPanelProps {
  vscode: any;
}

interface SecurityState {
  activeTab: 'overview' | 'permissions' | 'trust' | 'policies' | 'incidents' | 'audit';
  servers: ServerInfo[];
  pendingRequests: PermissionRequest[];
  incidents: SecurityIncident[];
  policies: SecurityPolicy[];
  selectedServer: string | null;
}

interface ServerInfo {
  id: string;
  name: string;
  trustLevel: TrustLevel;
  trustScore: number;
  status: 'active' | 'inactive' | 'blocked';
  lastScan?: Date;
  scanScore?: number;
}

export const SecurityPanel: React.FC<SecurityPanelProps> = ({ vscode }) => {
  const [state, setState] = useState<SecurityState>({
    activeTab: 'overview',
    servers: [],
    pendingRequests: [],
    incidents: [],
    policies: [],
    selectedServer: null
  });

  // Handle messages from extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      
      switch (message.type) {
        case 'update-servers':
          setState(prev => ({ ...prev, servers: message.servers }));
          break;
          
        case 'new-permission-request':
          setState(prev => ({
            ...prev,
            pendingRequests: [...prev.pendingRequests, message.request],
            activeTab: 'permissions'
          }));
          break;
          
        case 'update-incidents':
          setState(prev => ({ ...prev, incidents: message.incidents }));
          break;
          
        case 'update-policies':
          setState(prev => ({ ...prev, policies: message.policies }));
          break;
      }
    };

    window.addEventListener('message', handleMessage);
    
    // Request initial data
    vscode.postMessage({ type: 'ready' });
    
    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [vscode]);

  const handlePermissionDecision = useCallback((
    requestId: string,
    allowed: boolean,
    remember: boolean,
    scope: 'once' | 'session' | 'permanent'
  ) => {
    vscode.postMessage({
      type: 'permission-decision',
      requestId,
      decision: {
        allowed,
        remember,
        scope,
        timestamp: new Date()
      }
    });
    
    // Remove from pending
    setState(prev => ({
      ...prev,
      pendingRequests: prev.pendingRequests.filter(r => r.id !== requestId)
    }));
  }, [vscode]);

  const renderOverview = () => {
    const trustedCount = state.servers.filter(s => s.trustLevel === TrustLevel.Trusted).length;
    const activeCount = state.servers.filter(s => s.status === 'active').length;
    const blockedCount = state.servers.filter(s => s.status === 'blocked').length;
    const unresolvedIncidents = state.incidents.filter(i => !i.resolved).length;

    return (
      <div className="security-overview">
        <h2>Security Overview</h2>
        
        <div className="metrics-grid">
          <div className="metric-card">
            <div className="metric-value">{state.servers.length}</div>
            <div className="metric-label">Total Servers</div>
          </div>
          
          <div className="metric-card">
            <div className="metric-value">{activeCount}</div>
            <div className="metric-label">Active</div>
          </div>
          
          <div className="metric-card trusted">
            <div className="metric-value">{trustedCount}</div>
            <div className="metric-label">Trusted</div>
          </div>
          
          <div className="metric-card warning">
            <div className="metric-value">{state.pendingRequests.length}</div>
            <div className="metric-label">Pending Requests</div>
          </div>
          
          <div className="metric-card danger">
            <div className="metric-value">{blockedCount}</div>
            <div className="metric-label">Blocked</div>
          </div>
          
          <div className="metric-card danger">
            <div className="metric-value">{unresolvedIncidents}</div>
            <div className="metric-label">Active Incidents</div>
          </div>
        </div>
        
        <div className="server-list">
          <h3>MCP Servers</h3>
          {state.servers.map(server => (
            <div 
              key={server.id} 
              className={`server-item ${server.status}`}
              onClick={() => setState(prev => ({ ...prev, selectedServer: server.id }))}
            >
              <div className="server-info">
                <div className="server-name">{server.name}</div>
                <div className="server-id">{server.id}</div>
              </div>
              
              <div className="server-trust">
                <TrustBadge level={server.trustLevel} />
                <div className="trust-score">{server.trustScore}</div>
              </div>
              
              <div className="server-status">
                <StatusIndicator status={server.status} />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  };

  const renderPermissions = () => {
    return (
      <div className="permissions-view">
        <h2>Permission Requests</h2>
        
        {state.pendingRequests.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon">🛡️</div>
            <div className="empty-message">No pending permission requests</div>
          </div>
        ) : (
          <div className="request-list">
            {state.pendingRequests.map(request => (
              <PermissionRequestCard
                key={request.id}
                request={request}
                onDecision={handlePermissionDecision}
              />
            ))}
          </div>
        )}
      </div>
    );
  };

  const renderTrust = () => {
    const selectedServer = state.servers.find(s => s.id === state.selectedServer);
    
    return (
      <div className="trust-view">
        <h2>Trust Management</h2>
        
        <div className="trust-content">
          <div className="server-selector">
            <select 
              value={state.selectedServer || ''}
              onChange={(e) => setState(prev => ({ ...prev, selectedServer: e.target.value }))}
            >
              <option value="">Select a server...</option>
              {state.servers.map(server => (
                <option key={server.id} value={server.id}>
                  {server.name} ({server.trustLevel})
                </option>
              ))}
            </select>
          </div>
          
          {selectedServer && (
            <ServerTrustDetails
              server={selectedServer}
              vscode={vscode}
            />
          )}
        </div>
      </div>
    );
  };

  const renderPolicies = () => {
    return (
      <div className="policies-view">
        <h2>Security Policies</h2>
        
        <div className="policy-actions">
          <button 
            className="button primary"
            onClick={() => vscode.postMessage({ type: 'create-policy' })}
          >
            Create Policy
          </button>
        </div>
        
        <div className="policy-list">
          {state.policies.map(policy => (
            <PolicyCard
              key={policy.id}
              policy={policy}
              vscode={vscode}
            />
          ))}
        </div>
      </div>
    );
  };

  const renderIncidents = () => {
    const unresolvedIncidents = state.incidents.filter(i => !i.resolved);
    const resolvedIncidents = state.incidents.filter(i => i.resolved);
    
    return (
      <div className="incidents-view">
        <h2>Security Incidents</h2>
        
        {unresolvedIncidents.length > 0 && (
          <div className="incident-section">
            <h3>Active Incidents</h3>
            {unresolvedIncidents.map(incident => (
              <IncidentCard
                key={incident.id}
                incident={incident}
                vscode={vscode}
              />
            ))}
          </div>
        )}
        
        {resolvedIncidents.length > 0 && (
          <div className="incident-section">
            <h3>Resolved Incidents</h3>
            {resolvedIncidents.slice(0, 10).map(incident => (
              <IncidentCard
                key={incident.id}
                incident={incident}
                vscode={vscode}
                compact
              />
            ))}
          </div>
        )}
      </div>
    );
  };

  const renderAudit = () => {
    return (
      <div className="audit-view">
        <h2>Audit Logs</h2>
        
        <div className="audit-actions">
          <button 
            className="button"
            onClick={() => vscode.postMessage({ type: 'export-audit-logs' })}
          >
            Export Logs
          </button>
          
          <button 
            className="button"
            onClick={() => vscode.postMessage({ type: 'generate-audit-report' })}
          >
            Generate Report
          </button>
        </div>
        
        <AuditLogViewer vscode={vscode} />
      </div>
    );
  };

  return (
    <div className="security-panel">
      <div className="panel-header">
        <h1>MCP Security</h1>
        
        <div className="tab-bar">
          <button 
            className={`tab ${state.activeTab === 'overview' ? 'active' : ''}`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'overview' }))}
          >
            Overview
          </button>
          
          <button 
            className={`tab ${state.activeTab === 'permissions' ? 'active' : ''}`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'permissions' }))}
          >
            Permissions
            {state.pendingRequests.length > 0 && (
              <span className="badge">{state.pendingRequests.length}</span>
            )}
          </button>
          
          <button 
            className={`tab ${state.activeTab === 'trust' ? 'active' : ''}`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'trust' }))}
          >
            Trust
          </button>
          
          <button 
            className={`tab ${state.activeTab === 'policies' ? 'active' : ''}`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'policies' }))}
          >
            Policies
          </button>
          
          <button 
            className={`tab ${state.activeTab === 'incidents' ? 'active' : ''}`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'incidents' }))}
          >
            Incidents
            {state.incidents.filter(i => !i.resolved).length > 0 && (
              <span className="badge danger">
                {state.incidents.filter(i => !i.resolved).length}
              </span>
            )}
          </button>
          
          <button 
            className={`tab ${state.activeTab === 'audit' ? 'active' : ''}`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'audit' }))}
          >
            Audit
          </button>
        </div>
      </div>
      
      <div className="panel-content">
        {state.activeTab === 'overview' && renderOverview()}
        {state.activeTab === 'permissions' && renderPermissions()}
        {state.activeTab === 'trust' && renderTrust()}
        {state.activeTab === 'policies' && renderPolicies()}
        {state.activeTab === 'incidents' && renderIncidents()}
        {state.activeTab === 'audit' && renderAudit()}
      </div>
    </div>
  );
};

// Sub-components
const TrustBadge: React.FC<{ level: TrustLevel }> = ({ level }) => {
  const getColor = () => {
    switch (level) {
      case TrustLevel.Trusted: return 'green';
      case TrustLevel.Verified: return 'blue';
      case TrustLevel.Unknown: return 'gray';
      case TrustLevel.Restricted: return 'orange';
      case TrustLevel.Untrusted: return 'red';
    }
  };
  
  return (
    <span className={`trust-badge ${getColor()}`}>
      {level}
    </span>
  );
};

const StatusIndicator: React.FC<{ status: string }> = ({ status }) => {
  return (
    <span className={`status-indicator ${status}`}>
      {status === 'active' && '●'}
      {status === 'inactive' && '○'}
      {status === 'blocked' && '⊗'}
    </span>
  );
};

const PermissionRequestCard: React.FC<{
  request: PermissionRequest;
  onDecision: (id: string, allowed: boolean, remember: boolean, scope: 'once' | 'session' | 'permanent') => void;
}> = ({ request, onDecision }) => {
  const [remember, setRemember] = useState(false);
  const [scope, setScope] = useState<'once' | 'session' | 'permanent'>('session');
  
  return (
    <div className="permission-request-card">
      <div className="request-header">
        <div className="request-server">{request.serverName}</div>
        <div className="request-type">{request.type}</div>
      </div>
      
      <div className="request-details">
        <div className="request-description">{request.description}</div>
        <div className="request-resource">{request.resource}</div>
        
        {request.metadata && (
          <pre className="request-metadata">
            {JSON.stringify(request.metadata, null, 2)}
          </pre>
        )}
      </div>
      
      <div className="request-options">
        <label>
          <input 
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
          />
          Remember this decision
        </label>
        
        {remember && (
          <select 
            value={scope}
            onChange={(e) => setScope(e.target.value as any)}
          >
            <option value="session">For this session</option>
            <option value="permanent">Always</option>
          </select>
        )}
      </div>
      
      <div className="request-actions">
        <button 
          className="button danger"
          onClick={() => onDecision(request.id, false, remember, scope)}
        >
          Deny
        </button>
        
        <button 
          className="button primary"
          onClick={() => onDecision(request.id, true, remember, scope)}
        >
          Allow
        </button>
      </div>
    </div>
  );
};

// Additional sub-components would follow similar patterns...
// ServerTrustDetails, PolicyCard, IncidentCard, AuditLogViewer, etc.