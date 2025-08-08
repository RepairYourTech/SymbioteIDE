// MCP Server Panel - Revolutionary Native MCP Server & Enterprise Integrations
// Phase 4 UI Component for MCP Server & Enterprise Integrations system

import React, { useState, useEffect } from 'react';
// import { invoke } from '@tauri-apps/api/tauri';
import './MCPServerPanel.css';

interface MCPServer {
  server_id: string;
  name: string;
  description: string;
  version: string;
  status: string;
  port: number;
  protocol: string;
  capabilities: string[];
  resources: MCPResource[];
  tools: MCPTool[];
  created_at: string;
  last_activity: string;
}

interface MCPResource {
  resource_id: string;
  name: string;
  uri: string;
  mime_type: string;
  description: string;
  size: number;
  permissions: string[];
}

interface MCPTool {
  tool_id: string;
  name: string;
  description: string;
  input_schema: any;
  output_schema: any;
  category: string;
  permissions: string[];
}

interface EnterpriseConnector {
  connector_id: string;
  name: string;
  type: string;
  status: string;
  configuration: any;
  last_sync: string;
  sync_status: string;
  error_count: number;
}

interface MCPServerPanelProps {
  onServerStarted: (server: MCPServer) => void;
  onConnectorAdded: (connector: EnterpriseConnector) => void;
}

export const MCPServerPanel: React.FC<MCPServerPanelProps> = ({
  onServerStarted,
  onConnectorAdded,
}) => {
  const [activeTab, setActiveTab] = useState<'servers' | 'resources' | 'tools' | 'connectors'>('servers');
  const [servers, setServers] = useState<MCPServer[]>([]);
  const [connectors, setConnectors] = useState<EnterpriseConnector[]>([]);
  const [selectedServer, setSelectedServer] = useState<MCPServer | null>(null);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);

  // Server creation form
  const [serverForm, setServerForm] = useState({
    name: '',
    description: '',
    port: 8080,
    protocol: 'http',
    capabilities: [] as string[],
  });

  // Connector creation form
  const [connectorForm, setConnectorForm] = useState({
    name: '',
    type: 'github',
    configuration: {},
  });

  const serverCapabilities = [
    'resources', 'tools', 'prompts', 'sampling', 'logging',
    'completion', 'roots', 'initialize', 'ping', 'progress',
  ];

  const connectorTypes = [
    'github', 'gitlab', 'jira', 'confluence', 'slack', 'teams',
    'notion', 'airtable', 'salesforce', 'hubspot', 'zendesk',
    'aws', 'azure', 'gcp', 'docker', 'kubernetes', 'jenkins',
  ];

  useEffect(() => {
    loadServers();
    loadConnectors();
  }, []);

  const loadServers = async () => {
    try {
      // Mock get MCP servers - replace with actual Tauri invoke when ready
      const serverList: MCPServer[] = [];
      setServers(serverList);
    } catch (error) {
      console.error('Failed to load MCP servers:', error);
    }
  };

  const loadConnectors = async () => {
    try {
      // Mock get enterprise connectors - replace with actual Tauri invoke when ready
      const connectorList: EnterpriseConnector[] = [];
      setConnectors(connectorList);
    } catch (error) {
      console.error('Failed to load enterprise connectors:', error);
    }
  };

  const createServer = async () => {
    if (!serverForm.name) {
      alert('Please enter a server name');
      return;
    }

    setCreating(true);
    try {
      // Mock create MCP server - replace with actual Tauri invoke when ready
      const server: MCPServer = {
        server_id: `server_${Date.now()}`,
        name: serverForm.name,
        description: serverForm.description,
        version: '1.0.0',
        status: 'Running',
        port: serverForm.port,
        protocol: serverForm.protocol,
        capabilities: serverForm.capabilities,
        resources: [],
        tools: [],
        created_at: new Date().toISOString(),
        last_activity: new Date().toISOString(),
      };

      setServers(prev => [...prev, server]);
      onServerStarted(server);
      
      // Reset form
      setServerForm({
        name: '',
        description: '',
        port: 8080,
        protocol: 'http',
        capabilities: [],
      });

      alert('MCP Server created successfully!');
    } catch (error) {
      console.error('Failed to create MCP server:', error);
      alert('Failed to create MCP server. Please try again.');
    } finally {
      setCreating(false);
    }
  };

  const startServer = async (serverId: string) => {
    setLoading(true);
    try {
      // Mock start MCP server - replace with actual Tauri invoke when ready
      console.log('Starting MCP server:', serverId);
      await loadServers(); // Refresh server list
    } catch (error) {
      console.error('Failed to start server:', error);
      alert('Failed to start server.');
    } finally {
      setLoading(false);
    }
  };

  const stopServer = async (serverId: string) => {
    setLoading(true);
    try {
      // Mock stop MCP server - replace with actual Tauri invoke when ready
      console.log('Stopping MCP server:', serverId);
      await loadServers(); // Refresh server list
    } catch (error) {
      console.error('Failed to stop server:', error);
      alert('Failed to stop server.');
    } finally {
      setLoading(false);
    }
  };

  const createConnector = async () => {
    if (!connectorForm.name) {
      alert('Please enter a connector name');
      return;
    }

    setCreating(true);
    try {
      // Mock create enterprise connector - replace with actual Tauri invoke when ready
      const connector: EnterpriseConnector = {
        connector_id: `connector_${Date.now()}`,
        name: connectorForm.name,
        type: connectorForm.type,
        status: 'Connected',
        configuration: connectorForm.configuration,
        last_sync: new Date().toISOString(),
        sync_status: 'Success',
        error_count: 0,
      };

      setConnectors(prev => [...prev, connector]);
      onConnectorAdded(connector);
      
      // Reset form
      setConnectorForm({
        name: '',
        type: 'github',
        configuration: {},
      });

      alert('Enterprise connector created successfully!');
    } catch (error) {
      console.error('Failed to create enterprise connector:', error);
      alert('Failed to create enterprise connector. Please try again.');
    } finally {
      setCreating(false);
    }
  };

  const syncConnector = async (connectorId: string) => {
    setLoading(true);
    try {
      // Mock sync enterprise connector - replace with actual Tauri invoke when ready
      console.log('Syncing enterprise connector:', connectorId);
      await loadConnectors(); // Refresh connector list
    } catch (error) {
      console.error('Failed to sync connector:', error);
      alert('Failed to sync connector.');
    } finally {
      setLoading(false);
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Running': return '🟢';
      case 'Starting': return '🟡';
      case 'Stopped': return '🔴';
      case 'Error': return '❌';
      case 'Connected': return '🟢';
      case 'Disconnected': return '🔴';
      case 'Syncing': return '🔄';
      default: return '⚪';
    }
  };

  const getConnectorIcon = (type: string) => {
    const icons: { [key: string]: string } = {
      'github': '🐙',
      'gitlab': '🦊',
      'jira': '📋',
      'confluence': '📖',
      'slack': '💬',
      'teams': '👥',
      'notion': '📝',
      'airtable': '📊',
      'salesforce': '☁️',
      'hubspot': '🎯',
      'zendesk': '🎫',
      'aws': '☁️',
      'azure': '☁️',
      'gcp': '☁️',
      'docker': '🐳',
      'kubernetes': '⚙️',
      'jenkins': '🔧',
    };
    return icons[type] || '🔌';
  };

  const formatCapabilities = (capabilities: string[]) => {
    return capabilities.map(cap => cap.charAt(0).toUpperCase() + cap.slice(1)).join(', ');
  };

  const formatFileSize = (bytes: number) => {
    const sizes = ['B', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 B';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
  };

  return (
    <div className="mcp-server-panel">
      {/* Tab Navigation */}
      <div className="tab-navigation">
        <button
          className={`tab ${activeTab === 'servers' ? 'active' : ''}`}
          onClick={() => setActiveTab('servers')}
        >
          🖥️ MCP Servers ({servers.length})
        </button>
        <button
          className={`tab ${activeTab === 'resources' ? 'active' : ''}`}
          onClick={() => setActiveTab('resources')}
        >
          📁 Resources
        </button>
        <button
          className={`tab ${activeTab === 'tools' ? 'active' : ''}`}
          onClick={() => setActiveTab('tools')}
        >
          🔧 Tools
        </button>
        <button
          className={`tab ${activeTab === 'connectors' ? 'active' : ''}`}
          onClick={() => setActiveTab('connectors')}
        >
          🔌 Enterprise ({connectors.length})
        </button>
      </div>

      {/* Tab Content */}
      <div className="tab-content">
        {activeTab === 'servers' && (
          <div className="servers-tab">
            <div className="servers-header">
              <h3>MCP Servers</h3>
              <div className="header-actions">
                <button
                  onClick={loadServers}
                  className="refresh-button"
                >
                  🔄 Refresh
                </button>
              </div>
            </div>

            {/* Create Server Form */}
            <div className="create-server-section">
              <h4>Create New MCP Server</h4>
              <div className="server-form">
                <div className="form-row">
                  <div className="form-group">
                    <label>Server Name *</label>
                    <input
                      type="text"
                      value={serverForm.name}
                      onChange={(e) => setServerForm(prev => ({ ...prev, name: e.target.value }))}
                      placeholder="My MCP Server"
                      className="form-input"
                    />
                  </div>
                  
                  <div className="form-group">
                    <label>Port</label>
                    <input
                      type="number"
                      value={serverForm.port}
                      onChange={(e) => setServerForm(prev => ({ ...prev, port: parseInt(e.target.value) }))}
                      min="1024"
                      max="65535"
                      className="form-input"
                    />
                  </div>
                </div>

                <div className="form-group">
                  <label>Description</label>
                  <textarea
                    value={serverForm.description}
                    onChange={(e) => setServerForm(prev => ({ ...prev, description: e.target.value }))}
                    placeholder="Describe your MCP server..."
                    className="form-textarea"
                    rows={2}
                  />
                </div>

                <div className="form-group">
                  <label>Protocol</label>
                  <select
                    value={serverForm.protocol}
                    onChange={(e) => setServerForm(prev => ({ ...prev, protocol: e.target.value }))}
                    className="form-select"
                  >
                    <option value="http">HTTP</option>
                    <option value="https">HTTPS</option>
                    <option value="ws">WebSocket</option>
                    <option value="wss">WebSocket Secure</option>
                  </select>
                </div>

                <div className="form-group">
                  <label>Capabilities</label>
                  <div className="capabilities-grid">
                    {serverCapabilities.map(capability => (
                      <label key={capability} className="capability-checkbox">
                        <input
                          type="checkbox"
                          checked={serverForm.capabilities.includes(capability)}
                          onChange={(e) => {
                            if (e.target.checked) {
                              setServerForm(prev => ({
                                ...prev,
                                capabilities: [...prev.capabilities, capability],
                              }));
                            } else {
                              setServerForm(prev => ({
                                ...prev,
                                capabilities: prev.capabilities.filter(c => c !== capability),
                              }));
                            }
                          }}
                        />
                        {capability}
                      </label>
                    ))}
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    onClick={createServer}
                    disabled={creating || !serverForm.name}
                    className="create-button"
                  >
                    {creating ? '⏳ Creating...' : '🖥️ Create Server'}
                  </button>
                </div>
              </div>
            </div>

            {/* Servers List */}
            <div className="servers-list">
              {servers.map(server => (
                <div key={server.server_id} className="server-card">
                  <div className="server-header">
                    <div className="server-info">
                      <div className="server-name">{server.name}</div>
                      <div className="server-meta">
                        {server.protocol.toUpperCase()} • Port {server.port} • v{server.version}
                      </div>
                    </div>
                    
                    <div className="server-status">
                      {getStatusIcon(server.status)} {server.status}
                    </div>
                  </div>

                  <div className="server-description">
                    {server.description || 'No description provided'}
                  </div>

                  <div className="server-capabilities">
                    <strong>Capabilities:</strong> {formatCapabilities(server.capabilities)}
                  </div>

                  <div className="server-stats">
                    <div className="stat">
                      <span className="stat-label">Resources:</span>
                      <span className="stat-value">{server.resources.length}</span>
                    </div>
                    <div className="stat">
                      <span className="stat-label">Tools:</span>
                      <span className="stat-value">{server.tools.length}</span>
                    </div>
                    <div className="stat">
                      <span className="stat-label">Last Activity:</span>
                      <span className="stat-value">
                        {new Date(server.last_activity).toLocaleString()}
                      </span>
                    </div>
                  </div>

                  <div className="server-actions">
                    <button
                      onClick={() => setSelectedServer(server)}
                      className="view-button"
                    >
                      👁️ View Details
                    </button>
                    {server.status === 'Running' ? (
                      <button
                        onClick={() => stopServer(server.server_id)}
                        disabled={loading}
                        className="stop-button"
                      >
                        ⏹️ Stop
                      </button>
                    ) : (
                      <button
                        onClick={() => startServer(server.server_id)}
                        disabled={loading}
                        className="start-button"
                      >
                        ▶️ Start
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {servers.length === 0 && (
              <div className="empty-state">
                <h4>No MCP Servers</h4>
                <p>Create your first MCP server to get started</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'resources' && (
          <div className="resources-tab">
            <div className="resources-header">
              <h3>MCP Resources</h3>
              <div className="server-selector">
                <label>Server:</label>
                <select
                  value={selectedServer?.server_id || ''}
                  onChange={(e) => {
                    const server = servers.find(s => s.server_id === e.target.value);
                    setSelectedServer(server || null);
                  }}
                  className="form-select"
                >
                  <option value="">Select a server</option>
                  {servers.map(server => (
                    <option key={server.server_id} value={server.server_id}>
                      {server.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {selectedServer ? (
              <div className="resources-list">
                {selectedServer.resources.map(resource => (
                  <div key={resource.resource_id} className="resource-card">
                    <div className="resource-header">
                      <div className="resource-info">
                        <div className="resource-name">📁 {resource.name}</div>
                        <div className="resource-uri">{resource.uri}</div>
                      </div>
                      <div className="resource-size">
                        {formatFileSize(resource.size)}
                      </div>
                    </div>

                    <div className="resource-description">
                      {resource.description}
                    </div>

                    <div className="resource-meta">
                      <span className="resource-type">{resource.mime_type}</span>
                      <span className="resource-permissions">
                        Permissions: {resource.permissions.join(', ')}
                      </span>
                    </div>
                  </div>
                ))}

                {selectedServer.resources.length === 0 && (
                  <div className="empty-resources">
                    <p>No resources available on this server</p>
                  </div>
                )}
              </div>
            ) : (
              <div className="no-server-selected">
                <p>Select a server to view its resources</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'tools' && (
          <div className="tools-tab">
            <div className="tools-header">
              <h3>MCP Tools</h3>
              <div className="server-selector">
                <label>Server:</label>
                <select
                  value={selectedServer?.server_id || ''}
                  onChange={(e) => {
                    const server = servers.find(s => s.server_id === e.target.value);
                    setSelectedServer(server || null);
                  }}
                  className="form-select"
                >
                  <option value="">Select a server</option>
                  {servers.map(server => (
                    <option key={server.server_id} value={server.server_id}>
                      {server.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {selectedServer ? (
              <div className="tools-list">
                {selectedServer.tools.map(tool => (
                  <div key={tool.tool_id} className="tool-card">
                    <div className="tool-header">
                      <div className="tool-info">
                        <div className="tool-name">🔧 {tool.name}</div>
                        <div className="tool-category">{tool.category}</div>
                      </div>
                    </div>

                    <div className="tool-description">
                      {tool.description}
                    </div>

                    <div className="tool-schemas">
                      <div className="schema-section">
                        <h5>Input Schema</h5>
                        <pre className="schema-code">
                          {JSON.stringify(tool.input_schema, null, 2)}
                        </pre>
                      </div>
                      
                      <div className="schema-section">
                        <h5>Output Schema</h5>
                        <pre className="schema-code">
                          {JSON.stringify(tool.output_schema, null, 2)}
                        </pre>
                      </div>
                    </div>

                    <div className="tool-permissions">
                      <strong>Permissions:</strong> {tool.permissions.join(', ')}
                    </div>
                  </div>
                ))}

                {selectedServer.tools.length === 0 && (
                  <div className="empty-tools">
                    <p>No tools available on this server</p>
                  </div>
                )}
              </div>
            ) : (
              <div className="no-server-selected">
                <p>Select a server to view its tools</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'connectors' && (
          <div className="connectors-tab">
            <div className="connectors-header">
              <h3>Enterprise Connectors</h3>
              <div className="header-actions">
                <button
                  onClick={loadConnectors}
                  className="refresh-button"
                >
                  🔄 Refresh
                </button>
              </div>
            </div>

            {/* Create Connector Form */}
            <div className="create-connector-section">
              <h4>Add Enterprise Connector</h4>
              <div className="connector-form">
                <div className="form-row">
                  <div className="form-group">
                    <label>Connector Name *</label>
                    <input
                      type="text"
                      value={connectorForm.name}
                      onChange={(e) => setConnectorForm(prev => ({ ...prev, name: e.target.value }))}
                      placeholder="GitHub Integration"
                      className="form-input"
                    />
                  </div>
                  
                  <div className="form-group">
                    <label>Type</label>
                    <select
                      value={connectorForm.type}
                      onChange={(e) => setConnectorForm(prev => ({ ...prev, type: e.target.value }))}
                      className="form-select"
                    >
                      {connectorTypes.map(type => (
                        <option key={type} value={type}>
                          {getConnectorIcon(type)} {type.charAt(0).toUpperCase() + type.slice(1)}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    onClick={createConnector}
                    disabled={creating || !connectorForm.name}
                    className="create-button"
                  >
                    {creating ? '⏳ Creating...' : '🔌 Add Connector'}
                  </button>
                </div>
              </div>
            </div>

            {/* Connectors List */}
            <div className="connectors-list">
              {connectors.map(connector => (
                <div key={connector.connector_id} className="connector-card">
                  <div className="connector-header">
                    <div className="connector-info">
                      <div className="connector-name">
                        {getConnectorIcon(connector.type)} {connector.name}
                      </div>
                      <div className="connector-type">{connector.type}</div>
                    </div>
                    
                    <div className="connector-status">
                      {getStatusIcon(connector.status)} {connector.status}
                    </div>
                  </div>

                  <div className="connector-sync-info">
                    <div className="sync-status">
                      <span className="sync-label">Last Sync:</span>
                      <span className="sync-value">
                        {new Date(connector.last_sync).toLocaleString()}
                      </span>
                    </div>
                    <div className="sync-status">
                      <span className="sync-label">Status:</span>
                      <span className={`sync-value ${connector.sync_status.toLowerCase()}`}>
                        {connector.sync_status}
                      </span>
                    </div>
                    {connector.error_count > 0 && (
                      <div className="sync-errors">
                        ⚠️ {connector.error_count} error(s)
                      </div>
                    )}
                  </div>

                  <div className="connector-actions">
                    <button
                      onClick={() => syncConnector(connector.connector_id)}
                      disabled={loading}
                      className="sync-button"
                    >
                      🔄 Sync Now
                    </button>
                  </div>
                </div>
              ))}
            </div>

            {connectors.length === 0 && (
              <div className="empty-state">
                <h4>No Enterprise Connectors</h4>
                <p>Add your first enterprise connector to integrate with external services</p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default MCPServerPanel;
