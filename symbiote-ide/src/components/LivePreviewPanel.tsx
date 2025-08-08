// Live Preview Panel - Revolutionary Real-Time Multi-Framework Code Preview
// Phase 4 UI Component for Advanced Live Preview System integration

import React, { useState, useEffect, useRef } from 'react';
// import { invoke } from '@tauri-apps/api/tauri';
import './LivePreviewPanel.css';

interface PreviewSession {
  session_id: string;
  name: string;
  framework: string;
  status: string;
  port: number;
  url: string;
  created_at: string;
  last_updated: string;
}

interface PreviewConfig {
  framework: string;
  entry_point: string;
  build_command?: string;
  dev_command?: string;
  port: number;
  hot_reload: boolean;
  auto_refresh: boolean;
  element_inspection: boolean;
  live_editing: boolean;
  responsive_testing: boolean;
}

interface InspectedElement {
  tag_name: string;
  class_name?: string;
  id?: string;
  styles: { [key: string]: string };
  computed_styles: { [key: string]: string };
  position: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

interface LivePreviewPanelProps {
  currentFile?: string;
  onPreviewStarted: (session: PreviewSession) => void;
}

export const LivePreviewPanel: React.FC<LivePreviewPanelProps> = ({
  currentFile,
  onPreviewStarted,
}) => {
  const [activeTab, setActiveTab] = useState<'preview' | 'config' | 'sessions'>('preview');
  const [sessions, setSessions] = useState<PreviewSession[]>([]);
  const [currentSession, setCurrentSession] = useState<PreviewSession | null>(null);
  const [loading, setLoading] = useState(false);
  const [inspecting, setInspecting] = useState(false);
  const [inspectedElement, setInspectedElement] = useState<InspectedElement | null>(null);
  
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [iframeLoaded, setIframeLoaded] = useState(false);
  const [responsiveMode, setResponsiveMode] = useState<'desktop' | 'tablet' | 'mobile'>('desktop');
  const [zoom, setZoom] = useState(100);

  // Preview configuration
  const [previewConfig, setPreviewConfig] = useState<PreviewConfig>({
    framework: 'react',
    entry_point: 'src/App.tsx',
    port: 3000,
    hot_reload: true,
    auto_refresh: true,
    element_inspection: true,
    live_editing: true,
    responsive_testing: true,
  });

  const frameworks = [
    'react', 'vue', 'angular', 'svelte', 'nextjs', 'nuxtjs',
    'gatsby', 'astro', 'solid', 'qwik', 'vanilla', 'html',
  ];

  const responsiveSizes = {
    desktop: { width: '100%', height: '100%' },
    tablet: { width: '768px', height: '1024px' },
    mobile: { width: '375px', height: '667px' },
  };

  useEffect(() => {
    loadSessions();
  }, []);

  useEffect(() => {
    if (currentFile && previewConfig.auto_refresh && currentSession) {
      refreshPreview();
    }
  }, [currentFile]);

  const loadSessions = async () => {
    try {
      // Mock get preview sessions - replace with actual Tauri invoke when ready
      const sessionList: PreviewSession[] = [];
      setSessions(sessionList);
    } catch (error) {
      console.error('Failed to load preview sessions:', error);
    }
  };

  const startPreview = async () => {
    if (!previewConfig.entry_point) {
      alert('Please specify an entry point file');
      return;
    }

    setLoading(true);
    try {
      // Mock preview session creation - replace with actual Tauri invoke when ready
      const session: PreviewSession = {
        session_id: `session_${Date.now()}`,
        name: `${previewConfig.framework} Preview`,
        framework: previewConfig.framework,
        status: 'Running',
        port: previewConfig.port,
        url: `http://localhost:${previewConfig.port}`,
        created_at: new Date().toISOString(),
        last_updated: new Date().toISOString(),
      };

      setCurrentSession(session);
      setSessions(prev => [...prev, session]);
      onPreviewStarted(session);
      setActiveTab('preview');
      
      // Wait a moment for the server to start
      setTimeout(() => {
        setIframeLoaded(false);
        if (iframeRef.current) {
          iframeRef.current.src = session.url;
        }
      }, 2000);

    } catch (error) {
      console.error('Failed to start preview:', error);
      alert('Failed to start preview. Please check your configuration.');
    } finally {
      setLoading(false);
    }
  };

  const stopPreview = async (sessionId: string) => {
    try {
      // Mock stop preview session - replace with actual Tauri invoke when ready
      console.log('Stopping preview session:', sessionId);
      setSessions(prev => prev.filter(s => s.session_id !== sessionId));
      
      if (currentSession?.session_id === sessionId) {
        setCurrentSession(null);
        setIframeLoaded(false);
      }
    } catch (error) {
      console.error('Failed to stop preview:', error);
      alert('Failed to stop preview session.');
    }
  };

  const refreshPreview = async () => {
    if (!currentSession) return;

    try {
      // Mock refresh preview - replace with actual Tauri invoke when ready
      console.log('Refreshing preview session:', currentSession.session_id);
      
      if (iframeRef.current) {
        iframeRef.current.src = iframeRef.current.src; // Force reload
      }
    } catch (error) {
      console.error('Failed to refresh preview:', error);
    }
  };

  const toggleInspection = () => {
    setInspecting(!inspecting);
    setInspectedElement(null);
    
    if (iframeRef.current && iframeRef.current.contentWindow) {
      // Send message to iframe to toggle inspection mode
      iframeRef.current.contentWindow.postMessage({
        type: 'TOGGLE_INSPECTION',
        enabled: !inspecting,
      }, '*');
    }
  };

  const handleIframeLoad = () => {
    setIframeLoaded(true);
    
    if (iframeRef.current && iframeRef.current.contentWindow) {
      // Inject inspection script
      iframeRef.current.contentWindow.postMessage({
        type: 'INIT_INSPECTION',
        config: {
          enabled: previewConfig.element_inspection,
          liveEditing: previewConfig.live_editing,
        },
      }, '*');
    }
  };

  const handleElementInspect = (element: InspectedElement) => {
    setInspectedElement(element);
    console.log('Element inspected:', element);
  };

  const applyStyleChange = async (property: string, value: string) => {
    if (!currentSession || !inspectedElement) return;

    try {
      // Mock apply live style change - replace with actual Tauri invoke when ready
      console.log('Applying style change:', {
        sessionId: currentSession.session_id,
        selector: `${inspectedElement.tag_name}${inspectedElement.class_name ? '.' + inspectedElement.class_name : ''}${inspectedElement.id ? '#' + inspectedElement.id : ''}`,
        property,
        value,
      });

      // Update local element styles
      setInspectedElement(prev => prev ? {
        ...prev,
        styles: { ...prev.styles, [property]: value },
      } : null);

    } catch (error) {
      console.error('Failed to apply style change:', error);
    }
  };

  const getFrameworkIcon = (framework: string) => {
    const icons: { [key: string]: string } = {
      'react': '⚛️',
      'vue': '💚',
      'angular': '🅰️',
      'svelte': '🔥',
      'nextjs': '▲',
      'nuxtjs': '💚',
      'gatsby': '🚀',
      'astro': '🚀',
      'solid': '💎',
      'qwik': '⚡',
      'vanilla': '🍦',
      'html': '🌐',
    };
    return icons[framework] || '📦';
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Running': return '🟢';
      case 'Starting': return '🟡';
      case 'Stopped': return '🔴';
      case 'Error': return '❌';
      default: return '⚪';
    }
  };

  const getResponsiveIcon = (mode: string) => {
    switch (mode) {
      case 'desktop': return '🖥️';
      case 'tablet': return '📱';
      case 'mobile': return '📱';
      default: return '🖥️';
    }
  };

  return (
    <div className="live-preview-panel">
      {/* Tab Navigation */}
      <div className="tab-navigation">
        <button
          className={`tab ${activeTab === 'preview' ? 'active' : ''}`}
          onClick={() => setActiveTab('preview')}
        >
          👁️ Live Preview
        </button>
        <button
          className={`tab ${activeTab === 'config' ? 'active' : ''}`}
          onClick={() => setActiveTab('config')}
        >
          ⚙️ Configuration
        </button>
        <button
          className={`tab ${activeTab === 'sessions' ? 'active' : ''}`}
          onClick={() => setActiveTab('sessions')}
        >
          📋 Sessions ({sessions.length})
        </button>
      </div>

      {/* Tab Content */}
      <div className="tab-content">
        {activeTab === 'preview' && (
          <div className="preview-tab">
            {currentSession ? (
              <div className="preview-container">
                {/* Preview Controls */}
                <div className="preview-controls">
                  <div className="control-group">
                    <div className="session-info">
                      <span className="session-status">
                        {getStatusIcon(currentSession.status)} {currentSession.name}
                      </span>
                      <span className="session-url">{currentSession.url}</span>
                    </div>
                  </div>

                  <div className="control-group">
                    <button
                      onClick={refreshPreview}
                      className="control-button"
                      title="Refresh Preview"
                    >
                      🔄
                    </button>
                    <button
                      onClick={toggleInspection}
                      className={`control-button ${inspecting ? 'active' : ''}`}
                      title="Toggle Element Inspection"
                    >
                      🔍
                    </button>
                    <button
                      onClick={() => stopPreview(currentSession.session_id)}
                      className="control-button stop-button"
                      title="Stop Preview"
                    >
                      ⏹️
                    </button>
                  </div>
                </div>

                {/* Responsive Controls */}
                <div className="responsive-controls">
                  <div className="responsive-buttons">
                    {Object.keys(responsiveSizes).map(mode => (
                      <button
                        key={mode}
                        onClick={() => setResponsiveMode(mode as any)}
                        className={`responsive-button ${responsiveMode === mode ? 'active' : ''}`}
                      >
                        {getResponsiveIcon(mode)} {mode}
                      </button>
                    ))}
                  </div>

                  <div className="zoom-controls">
                    <button
                      onClick={() => setZoom(Math.max(25, zoom - 25))}
                      className="zoom-button"
                    >
                      -
                    </button>
                    <span className="zoom-display">{zoom}%</span>
                    <button
                      onClick={() => setZoom(Math.min(200, zoom + 25))}
                      className="zoom-button"
                    >
                      +
                    </button>
                  </div>
                </div>

                {/* Preview Frame */}
                <div className="preview-frame-container">
                  <div 
                    className="preview-frame-wrapper"
                    style={{
                      width: responsiveSizes[responsiveMode].width,
                      height: responsiveSizes[responsiveMode].height,
                      transform: `scale(${zoom / 100})`,
                      transformOrigin: 'top left',
                    }}
                  >
                    {!iframeLoaded && (
                      <div className="preview-loading">
                        <div className="loading-spinner"></div>
                        <p>Loading preview...</p>
                      </div>
                    )}
                    <iframe
                      ref={iframeRef}
                      className="preview-iframe"
                      onLoad={handleIframeLoad}
                      title="Live Preview"
                    />
                  </div>
                </div>

                {/* Element Inspector */}
                {inspectedElement && (
                  <div className="element-inspector">
                    <div className="inspector-header">
                      <h4>🔍 Element Inspector</h4>
                      <button
                        onClick={() => setInspectedElement(null)}
                        className="close-inspector"
                      >
                        ✕
                      </button>
                    </div>

                    <div className="inspector-content">
                      <div className="element-info">
                        <div className="element-tag">
                          &lt;{inspectedElement.tag_name}
                          {inspectedElement.class_name && ` class="${inspectedElement.class_name}"`}
                          {inspectedElement.id && ` id="${inspectedElement.id}"`}
                          &gt;
                        </div>
                      </div>

                      <div className="element-styles">
                        <h5>Styles</h5>
                        {Object.entries(inspectedElement.styles).map(([property, value]) => (
                          <div key={property} className="style-property">
                            <span className="property-name">{property}:</span>
                            <input
                              type="text"
                              value={value}
                              onChange={(e) => applyStyleChange(property, e.target.value)}
                              className="property-value"
                            />
                          </div>
                        ))}
                      </div>

                      <div className="element-position">
                        <h5>Position</h5>
                        <div className="position-info">
                          <span>X: {inspectedElement.position.x}px</span>
                          <span>Y: {inspectedElement.position.y}px</span>
                          <span>W: {inspectedElement.position.width}px</span>
                          <span>H: {inspectedElement.position.height}px</span>
                        </div>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="no-preview">
                <h3>No Active Preview</h3>
                <p>Configure and start a preview session to see your code live</p>
                <button
                  onClick={() => setActiveTab('config')}
                  className="start-preview-button"
                >
                  ⚙️ Configure Preview
                </button>
              </div>
            )}
          </div>
        )}

        {activeTab === 'config' && (
          <div className="config-tab">
            <div className="config-form">
              <h3>Preview Configuration</h3>

              <div className="form-section">
                <h4>Basic Settings</h4>
                
                <div className="form-group">
                  <label>Framework</label>
                  <select
                    value={previewConfig.framework}
                    onChange={(e) => setPreviewConfig(prev => ({ ...prev, framework: e.target.value }))}
                    className="form-select"
                  >
                    {frameworks.map(framework => (
                      <option key={framework} value={framework}>
                        {getFrameworkIcon(framework)} {framework.charAt(0).toUpperCase() + framework.slice(1)}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="form-group">
                  <label>Entry Point</label>
                  <input
                    type="text"
                    value={previewConfig.entry_point}
                    onChange={(e) => setPreviewConfig(prev => ({ ...prev, entry_point: e.target.value }))}
                    placeholder="src/App.tsx"
                    className="form-input"
                  />
                </div>

                <div className="form-group">
                  <label>Port</label>
                  <input
                    type="number"
                    value={previewConfig.port}
                    onChange={(e) => setPreviewConfig(prev => ({ ...prev, port: parseInt(e.target.value) }))}
                    min="3000"
                    max="9999"
                    className="form-input"
                  />
                </div>
              </div>

              <div className="form-section">
                <h4>Commands (Optional)</h4>
                
                <div className="form-group">
                  <label>Build Command</label>
                  <input
                    type="text"
                    value={previewConfig.build_command || ''}
                    onChange={(e) => setPreviewConfig(prev => ({ ...prev, build_command: e.target.value }))}
                    placeholder="npm run build"
                    className="form-input"
                  />
                </div>

                <div className="form-group">
                  <label>Dev Command</label>
                  <input
                    type="text"
                    value={previewConfig.dev_command || ''}
                    onChange={(e) => setPreviewConfig(prev => ({ ...prev, dev_command: e.target.value }))}
                    placeholder="npm run dev"
                    className="form-input"
                  />
                </div>
              </div>

              <div className="form-section">
                <h4>Features</h4>
                
                <div className="features-grid">
                  <label className="feature-checkbox">
                    <input
                      type="checkbox"
                      checked={previewConfig.hot_reload}
                      onChange={(e) => setPreviewConfig(prev => ({ ...prev, hot_reload: e.target.checked }))}
                    />
                    🔥 Hot Reload
                  </label>

                  <label className="feature-checkbox">
                    <input
                      type="checkbox"
                      checked={previewConfig.auto_refresh}
                      onChange={(e) => setPreviewConfig(prev => ({ ...prev, auto_refresh: e.target.checked }))}
                    />
                    🔄 Auto Refresh
                  </label>

                  <label className="feature-checkbox">
                    <input
                      type="checkbox"
                      checked={previewConfig.element_inspection}
                      onChange={(e) => setPreviewConfig(prev => ({ ...prev, element_inspection: e.target.checked }))}
                    />
                    🔍 Element Inspection
                  </label>

                  <label className="feature-checkbox">
                    <input
                      type="checkbox"
                      checked={previewConfig.live_editing}
                      onChange={(e) => setPreviewConfig(prev => ({ ...prev, live_editing: e.target.checked }))}
                    />
                    ✏️ Live Editing
                  </label>

                  <label className="feature-checkbox">
                    <input
                      type="checkbox"
                      checked={previewConfig.responsive_testing}
                      onChange={(e) => setPreviewConfig(prev => ({ ...prev, responsive_testing: e.target.checked }))}
                    />
                    📱 Responsive Testing
                  </label>
                </div>
              </div>

              <div className="form-actions">
                <button
                  onClick={startPreview}
                  disabled={loading}
                  className="start-button"
                >
                  {loading ? '⏳ Starting...' : '🚀 Start Preview'}
                </button>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'sessions' && (
          <div className="sessions-tab">
            <div className="sessions-header">
              <h3>Preview Sessions</h3>
              <button
                onClick={loadSessions}
                className="refresh-sessions-button"
              >
                🔄 Refresh
              </button>
            </div>

            <div className="sessions-list">
              {sessions.map(session => (
                <div key={session.session_id} className="session-card">
                  <div className="session-header">
                    <div className="session-info">
                      <div className="session-name">
                        {getFrameworkIcon(session.framework)} {session.name}
                      </div>
                      <div className="session-meta">
                        {session.framework} • Port {session.port}
                      </div>
                    </div>
                    
                    <div className="session-status">
                      {getStatusIcon(session.status)} {session.status}
                    </div>
                  </div>

                  <div className="session-url">
                    <a href={session.url} target="_blank" rel="noopener noreferrer">
                      {session.url}
                    </a>
                  </div>

                  <div className="session-actions">
                    <button
                      onClick={() => setCurrentSession(session)}
                      className="view-button"
                    >
                      👁️ View
                    </button>
                    <button
                      onClick={() => stopPreview(session.session_id)}
                      className="stop-button"
                    >
                      ⏹️ Stop
                    </button>
                  </div>

                  <div className="session-footer">
                    <span>Created: {new Date(session.created_at).toLocaleString()}</span>
                  </div>
                </div>
              ))}
            </div>

            {sessions.length === 0 && (
              <div className="empty-sessions">
                <h4>No Preview Sessions</h4>
                <p>Start your first preview session to see it here</p>
                <button
                  onClick={() => setActiveTab('config')}
                  className="create-session-button"
                >
                  ⚙️ Create Session
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default LivePreviewPanel;
