import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './APIBuilderPanel.css';

interface APIProject {
  id: string;
  name: string;
  description: string;
  version: string;
  base_url: string;
  endpoints: APIEndpoint[];
  schemas: DataSchema[];
  created_at: string;
  updated_at: string;
}

interface APIEndpoint {
  id: string;
  path: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  summary: string;
  description: string;
  parameters: Parameter[];
  request_body?: RequestBody;
  responses: Record<string, Response>;
  tags: string[];
}

interface Parameter {
  name: string;
  location: 'Query' | 'Path' | 'Header';
  required: boolean;
  param_type: string;
  description: string;
}

interface RequestBody {
  content_type: string;
  schema_name: string;
  required: boolean;
  description: string;
}

interface Response {
  status_code: number;
  description: string;
  content_type?: string;
  schema_name?: string;
}

interface DataSchema {
  id: string;
  name: string;
  description: string;
  properties: Record<string, SchemaProperty>;
  required: string[];
}

interface SchemaProperty {
  property_type: string;
  description: string;
  required: boolean;
  example?: string;
}

interface APITemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  endpoints: APIEndpoint[];
  schemas: DataSchema[];
}

interface GeneratedCode {
  files: Record<string, string>;
  dependencies: string[];
  instructions: string[];
}

export const APIBuilderPanel: React.FC = () => {
  const [currentProject, setCurrentProject] = useState<APIProject | null>(null);
  const [templates, setTemplates] = useState<APITemplate[]>([]);
  const [selectedEndpoint, setSelectedEndpoint] = useState<APIEndpoint | null>(null);
  const [selectedSchema, setSelectedSchema] = useState<DataSchema | null>(null);
  const [activeTab, setActiveTab] = useState<'endpoints' | 'schemas' | 'generate'>('endpoints');
  const [isGenerating, setIsGenerating] = useState(false);
  const [generatedCode, setGeneratedCode] = useState<GeneratedCode | null>(null);
  const [showNewProjectModal, setShowNewProjectModal] = useState(false);
  const [showNewEndpointModal, setShowNewEndpointModal] = useState(false);
  const [showNewSchemaModal, setShowNewSchemaModal] = useState(false);

  // New project form
  const [newProjectName, setNewProjectName] = useState('');
  const [newProjectDescription, setNewProjectDescription] = useState('');

  // New endpoint form
  const [newEndpoint, setNewEndpoint] = useState<Partial<APIEndpoint>>({
    path: '',
    method: 'GET',
    summary: '',
    description: '',
    parameters: [],
    responses: {},
    tags: [],
  });

  // New schema form
  const [newSchema, setNewSchema] = useState<Partial<DataSchema>>({
    name: '',
    description: '',
    properties: {},
    required: [],
  });

  useEffect(() => {
    loadTemplates();
  }, []);

  const loadTemplates = async () => {
    try {
      const templatesData = await invoke<Record<string, APITemplate>>('api_builder_get_templates');
      setTemplates(Object.values(templatesData));
    } catch (error) {
      console.error('Failed to load templates:', error);
    }
  };

  const createProject = async () => {
    if (!newProjectName.trim()) return;

    try {
      const projectId = await invoke<string>('api_builder_create_project', {
        name: newProjectName,
        description: newProjectDescription,
      });

      const project = await invoke<APIProject>('api_builder_get_project', { projectId });
      setCurrentProject(project);
      setShowNewProjectModal(false);
      setNewProjectName('');
      setNewProjectDescription('');
    } catch (error) {
      console.error('Failed to create project:', error);
    }
  };

  const addEndpoint = async () => {
    if (!currentProject || !newEndpoint.path || !newEndpoint.summary) return;

    try {
      const endpoint: APIEndpoint = {
        id: crypto.randomUUID(),
        path: newEndpoint.path!,
        method: newEndpoint.method!,
        summary: newEndpoint.summary!,
        description: newEndpoint.description || '',
        parameters: newEndpoint.parameters || [],
        request_body: newEndpoint.request_body,
        responses: newEndpoint.responses || {},
        tags: newEndpoint.tags || [],
      };

      await invoke('api_builder_add_endpoint', {
        projectId: currentProject.id,
        endpoint,
      });

      // Refresh project
      const updatedProject = await invoke<APIProject>('api_builder_get_project', {
        projectId: currentProject.id,
      });
      setCurrentProject(updatedProject);
      setShowNewEndpointModal(false);
      setNewEndpoint({
        path: '',
        method: 'GET',
        summary: '',
        description: '',
        parameters: [],
        responses: {},
        tags: [],
      });
    } catch (error) {
      console.error('Failed to add endpoint:', error);
    }
  };

  const generateCode = async (framework: string) => {
    if (!currentProject) return;

    setIsGenerating(true);
    try {
      const code = await invoke<GeneratedCode>('api_builder_generate_code', {
        projectId: currentProject.id,
        framework,
      });
      setGeneratedCode(code);
      setActiveTab('generate');
    } catch (error) {
      console.error('Failed to generate code:', error);
    } finally {
      setIsGenerating(false);
    }
  };

  const applyTemplate = async (templateId: string) => {
    if (!currentProject) return;

    try {
      await invoke('api_builder_apply_template', {
        projectId: currentProject.id,
        templateId,
      });

      // Refresh project
      const updatedProject = await invoke<APIProject>('api_builder_get_project', {
        projectId: currentProject.id,
      });
      setCurrentProject(updatedProject);
    } catch (error) {
      console.error('Failed to apply template:', error);
    }
  };

  const getMethodColor = (method: string) => {
    switch (method) {
      case 'GET': return '#00ff88';
      case 'POST': return '#ff9500';
      case 'PUT': return '#74c0fc';
      case 'DELETE': return '#ff4444';
      case 'PATCH': return '#ff6b6b';
      default: return '#888';
    }
  };

  if (!currentProject) {
    return (
      <div className="api-builder-panel">
        <div className="api-builder-header">
          <h3>🚀 API Builder</h3>
          <p>Visual API Designer & Code Generator</p>
        </div>

        <div className="no-project">
          <div className="no-project-content">
            <h4>🎯 Create Your First API</h4>
            <p>Design and generate production-ready APIs with visual tools</p>
            
            <button 
              className="create-project-button"
              onClick={() => setShowNewProjectModal(true)}
            >
              ➕ New API Project
            </button>

            <div className="templates-section">
              <h5>📋 Quick Start Templates</h5>
              <div className="templates-grid">
                {templates.map(template => (
                  <div key={template.id} className="template-card">
                    <h6>{template.name}</h6>
                    <p>{template.description}</p>
                    <span className="template-category">{template.category}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {showNewProjectModal && (
          <div className="modal-overlay">
            <div className="modal">
              <div className="modal-header">
                <h4>Create New API Project</h4>
                <button onClick={() => setShowNewProjectModal(false)}>✕</button>
              </div>
              <div className="modal-body">
                <div className="form-group">
                  <label>Project Name</label>
                  <input
                    type="text"
                    value={newProjectName}
                    onChange={(e) => setNewProjectName(e.target.value)}
                    placeholder="My Awesome API"
                  />
                </div>
                <div className="form-group">
                  <label>Description</label>
                  <textarea
                    value={newProjectDescription}
                    onChange={(e) => setNewProjectDescription(e.target.value)}
                    placeholder="API for managing..."
                    rows={3}
                  />
                </div>
              </div>
              <div className="modal-footer">
                <button onClick={() => setShowNewProjectModal(false)}>Cancel</button>
                <button onClick={createProject} className="primary">Create Project</button>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="api-builder-panel">
      <div className="api-builder-header">
        <div className="project-info">
          <h3>🚀 {currentProject.name}</h3>
          <p>{currentProject.description}</p>
          <span className="project-version">v{currentProject.version}</span>
        </div>
        <div className="header-actions">
          <button onClick={() => setShowNewProjectModal(true)}>📁 New Project</button>
          <button onClick={() => generateCode('express')} disabled={isGenerating}>
            {isGenerating ? '⏳' : '⚡'} Generate
          </button>
        </div>
      </div>

      <div className="api-builder-tabs">
        <button 
          className={activeTab === 'endpoints' ? 'active' : ''}
          onClick={() => setActiveTab('endpoints')}
        >
          🔗 Endpoints ({currentProject.endpoints.length})
        </button>
        <button 
          className={activeTab === 'schemas' ? 'active' : ''}
          onClick={() => setActiveTab('schemas')}
        >
          📋 Schemas ({currentProject.schemas.length})
        </button>
        <button 
          className={activeTab === 'generate' ? 'active' : ''}
          onClick={() => setActiveTab('generate')}
        >
          ⚡ Generate
        </button>
      </div>

      <div className="api-builder-content">
        {activeTab === 'endpoints' && (
          <div className="endpoints-tab">
            <div className="tab-header">
              <h4>API Endpoints</h4>
              <div className="tab-actions">
                <button onClick={() => setShowNewEndpointModal(true)}>➕ Add Endpoint</button>
                <select onChange={(e) => e.target.value && applyTemplate(e.target.value)}>
                  <option value="">📋 Apply Template</option>
                  {templates.map(template => (
                    <option key={template.id} value={template.id}>{template.name}</option>
                  ))}
                </select>
              </div>
            </div>

            <div className="endpoints-list">
              {currentProject.endpoints.map(endpoint => (
                <div 
                  key={endpoint.id} 
                  className={`endpoint-item ${selectedEndpoint?.id === endpoint.id ? 'selected' : ''}`}
                  onClick={() => setSelectedEndpoint(endpoint)}
                >
                  <div className="endpoint-method" style={{ backgroundColor: getMethodColor(endpoint.method) }}>
                    {endpoint.method}
                  </div>
                  <div className="endpoint-info">
                    <div className="endpoint-path">{endpoint.path}</div>
                    <div className="endpoint-summary">{endpoint.summary}</div>
                  </div>
                  <div className="endpoint-meta">
                    {endpoint.tags.map(tag => (
                      <span key={tag} className="endpoint-tag">{tag}</span>
                    ))}
                  </div>
                </div>
              ))}
            </div>

            {selectedEndpoint && (
              <div className="endpoint-details">
                <h5>Endpoint Details</h5>
                <div className="detail-grid">
                  <div className="detail-item">
                    <label>Path</label>
                    <span>{selectedEndpoint.path}</span>
                  </div>
                  <div className="detail-item">
                    <label>Method</label>
                    <span style={{ color: getMethodColor(selectedEndpoint.method) }}>
                      {selectedEndpoint.method}
                    </span>
                  </div>
                  <div className="detail-item">
                    <label>Summary</label>
                    <span>{selectedEndpoint.summary}</span>
                  </div>
                  <div className="detail-item">
                    <label>Description</label>
                    <span>{selectedEndpoint.description}</span>
                  </div>
                </div>

                {selectedEndpoint.parameters.length > 0 && (
                  <div className="parameters-section">
                    <h6>Parameters</h6>
                    <div className="parameters-list">
                      {selectedEndpoint.parameters.map((param, index) => (
                        <div key={index} className="parameter-item">
                          <span className="param-name">{param.name}</span>
                          <span className="param-location">{param.location}</span>
                          <span className="param-type">{param.param_type}</span>
                          {param.required && <span className="param-required">Required</span>}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {activeTab === 'schemas' && (
          <div className="schemas-tab">
            <div className="tab-header">
              <h4>Data Schemas</h4>
              <button onClick={() => setShowNewSchemaModal(true)}>➕ Add Schema</button>
            </div>

            <div className="schemas-list">
              {currentProject.schemas.map(schema => (
                <div 
                  key={schema.id} 
                  className={`schema-item ${selectedSchema?.id === schema.id ? 'selected' : ''}`}
                  onClick={() => setSelectedSchema(schema)}
                >
                  <div className="schema-info">
                    <div className="schema-name">{schema.name}</div>
                    <div className="schema-description">{schema.description}</div>
                  </div>
                  <div className="schema-meta">
                    <span className="property-count">{Object.keys(schema.properties).length} properties</span>
                  </div>
                </div>
              ))}
            </div>

            {selectedSchema && (
              <div className="schema-details">
                <h5>Schema: {selectedSchema.name}</h5>
                <p>{selectedSchema.description}</p>
                
                <div className="properties-section">
                  <h6>Properties</h6>
                  <div className="properties-list">
                    {Object.entries(selectedSchema.properties).map(([name, property]) => (
                      <div key={name} className="property-item">
                        <span className="property-name">{name}</span>
                        <span className="property-type">{property.property_type}</span>
                        <span className="property-description">{property.description}</span>
                        {property.required && <span className="property-required">Required</span>}
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'generate' && (
          <div className="generate-tab">
            <div className="tab-header">
              <h4>Code Generation</h4>
            </div>

            <div className="generation-options">
              <h5>Select Framework</h5>
              <div className="framework-grid">
                <button 
                  className="framework-button"
                  onClick={() => generateCode('express')}
                  disabled={isGenerating}
                >
                  <span className="framework-icon">🟢</span>
                  <span className="framework-name">Express.js</span>
                  <span className="framework-desc">Node.js REST API</span>
                </button>
                <button 
                  className="framework-button"
                  onClick={() => generateCode('fastapi')}
                  disabled={isGenerating}
                >
                  <span className="framework-icon">🐍</span>
                  <span className="framework-name">FastAPI</span>
                  <span className="framework-desc">Python REST API</span>
                </button>
                <button 
                  className="framework-button"
                  onClick={() => generateCode('axum')}
                  disabled={isGenerating}
                >
                  <span className="framework-icon">🦀</span>
                  <span className="framework-name">Axum</span>
                  <span className="framework-desc">Rust REST API</span>
                </button>
              </div>
            </div>

            {generatedCode && (
              <div className="generated-code">
                <h5>Generated Code</h5>
                
                <div className="code-files">
                  {Object.entries(generatedCode.files).map(([filename, content]) => (
                    <div key={filename} className="code-file">
                      <div className="file-header">
                        <span className="file-name">📄 {filename}</span>
                        <button onClick={() => navigator.clipboard.writeText(content)}>
                          📋 Copy
                        </button>
                      </div>
                      <pre className="file-content">{content}</pre>
                    </div>
                  ))}
                </div>

                <div className="generation-info">
                  <div className="dependencies">
                    <h6>Dependencies</h6>
                    <ul>
                      {generatedCode.dependencies.map(dep => (
                        <li key={dep}>{dep}</li>
                      ))}
                    </ul>
                  </div>
                  
                  <div className="instructions">
                    <h6>Setup Instructions</h6>
                    <ol>
                      {generatedCode.instructions.map((instruction, index) => (
                        <li key={index}>{instruction}</li>
                      ))}
                    </ol>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* New Endpoint Modal */}
      {showNewEndpointModal && (
        <div className="modal-overlay">
          <div className="modal">
            <div className="modal-header">
              <h4>Add New Endpoint</h4>
              <button onClick={() => setShowNewEndpointModal(false)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="form-row">
                <div className="form-group">
                  <label>Method</label>
                  <select 
                    value={newEndpoint.method} 
                    onChange={(e) => setNewEndpoint({...newEndpoint, method: e.target.value as any})}
                  >
                    <option value="GET">GET</option>
                    <option value="POST">POST</option>
                    <option value="PUT">PUT</option>
                    <option value="DELETE">DELETE</option>
                    <option value="PATCH">PATCH</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>Path</label>
                  <input
                    type="text"
                    value={newEndpoint.path}
                    onChange={(e) => setNewEndpoint({...newEndpoint, path: e.target.value})}
                    placeholder="/api/users"
                  />
                </div>
              </div>
              <div className="form-group">
                <label>Summary</label>
                <input
                  type="text"
                  value={newEndpoint.summary}
                  onChange={(e) => setNewEndpoint({...newEndpoint, summary: e.target.value})}
                  placeholder="Get all users"
                />
              </div>
              <div className="form-group">
                <label>Description</label>
                <textarea
                  value={newEndpoint.description}
                  onChange={(e) => setNewEndpoint({...newEndpoint, description: e.target.value})}
                  placeholder="Retrieves a list of all users..."
                  rows={3}
                />
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={() => setShowNewEndpointModal(false)}>Cancel</button>
              <button onClick={addEndpoint} className="primary">Add Endpoint</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
