import React, { useState, useEffect } from 'react';
import { WorkflowNode } from '../VisualBuilderPanel';

interface PropertyPanelProps {
  selectedNode: WorkflowNode | null;
  onNodeUpdate: (nodeId: string, updates: Partial<WorkflowNode>) => void;
}

interface PropertyField {
  key: string;
  label: string;
  type: 'text' | 'number' | 'boolean' | 'select' | 'textarea' | 'file' | 'json';
  value: any;
  options?: string[];
  placeholder?: string;
  description?: string;
  required?: boolean;
}

export const PropertyPanel: React.FC<PropertyPanelProps> = ({
  selectedNode,
  onNodeUpdate
}) => {
  const [properties, setProperties] = useState<PropertyField[]>([]);
  const [nodeLabel, setNodeLabel] = useState('');
  const [nodeDescription, setNodeDescription] = useState('');

  useEffect(() => {
    if (selectedNode) {
      setNodeLabel(selectedNode.data.label);
      setNodeDescription(selectedNode.data.description || '');
      setProperties(getNodeProperties(selectedNode));
    } else {
      setNodeLabel('');
      setNodeDescription('');
      setProperties([]);
    }
  }, [selectedNode]);

  const getNodeProperties = (node: WorkflowNode): PropertyField[] => {
    const baseProperties: PropertyField[] = [];

    // Common properties for all nodes
    switch (node.type) {
      case 'input-file':
        return [
          { key: 'filePath', label: 'File Path', type: 'file', value: node.data.properties.filePath || '', placeholder: 'Select file...', required: true },
          { key: 'encoding', label: 'Encoding', type: 'select', value: node.data.properties.encoding || 'utf-8', options: ['utf-8', 'ascii', 'latin1', 'base64'] },
          { key: 'watchChanges', label: 'Watch Changes', type: 'boolean', value: node.data.properties.watchChanges || false, description: 'Monitor file for changes' }
        ];

      case 'input-user':
        return [
          { key: 'prompt', label: 'Prompt', type: 'textarea', value: node.data.properties.prompt || '', placeholder: 'Enter prompt for user...', required: true },
          { key: 'inputType', label: 'Input Type', type: 'select', value: node.data.properties.inputType || 'text', options: ['text', 'number', 'password', 'email'] },
          { key: 'required', label: 'Required', type: 'boolean', value: node.data.properties.required || true }
        ];

      case 'input-api':
        return [
          { key: 'url', label: 'API URL', type: 'text', value: node.data.properties.url || '', placeholder: 'https://api.example.com/data', required: true },
          { key: 'method', label: 'HTTP Method', type: 'select', value: node.data.properties.method || 'GET', options: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'] },
          { key: 'headers', label: 'Headers', type: 'json', value: node.data.properties.headers || '{}', placeholder: '{"Authorization": "Bearer token"}' },
          { key: 'timeout', label: 'Timeout (ms)', type: 'number', value: node.data.properties.timeout || 5000 }
        ];

      case 'agent-code':
        return [
          { key: 'language', label: 'Language', type: 'select', value: node.data.properties.language || 'typescript', options: ['typescript', 'javascript', 'python', 'rust', 'go', 'java'] },
          { key: 'task', label: 'Task', type: 'textarea', value: node.data.properties.task || '', placeholder: 'Describe the coding task...', required: true },
          { key: 'model', label: 'AI Model', type: 'select', value: node.data.properties.model || 'gpt-4', options: ['gpt-4', 'claude-3', 'gemini-pro', 'deepseek-coder'] },
          { key: 'temperature', label: 'Temperature', type: 'number', value: node.data.properties.temperature || 0.7 }
        ];

      case 'agent-test':
        return [
          { key: 'testType', label: 'Test Type', type: 'select', value: node.data.properties.testType || 'unit', options: ['unit', 'integration', 'e2e', 'performance'] },
          { key: 'framework', label: 'Framework', type: 'select', value: node.data.properties.framework || 'jest', options: ['jest', 'vitest', 'mocha', 'pytest', 'cargo-test'] },
          { key: 'coverage', label: 'Coverage Target', type: 'number', value: node.data.properties.coverage || 80 },
          { key: 'parallel', label: 'Parallel Execution', type: 'boolean', value: node.data.properties.parallel || true }
        ];

      case 'transform-data':
        return [
          { key: 'transformation', label: 'Transformation', type: 'textarea', value: node.data.properties.transformation || '', placeholder: 'Define data transformation...', required: true },
          { key: 'inputFormat', label: 'Input Format', type: 'select', value: node.data.properties.inputFormat || 'json', options: ['json', 'csv', 'xml', 'yaml', 'text'] },
          { key: 'outputFormat', label: 'Output Format', type: 'select', value: node.data.properties.outputFormat || 'json', options: ['json', 'csv', 'xml', 'yaml', 'text'] }
        ];

      case 'control-condition':
        return [
          { key: 'condition', label: 'Condition', type: 'textarea', value: node.data.properties.condition || '', placeholder: 'Enter condition expression...', required: true },
          { key: 'operator', label: 'Operator', type: 'select', value: node.data.properties.operator || 'equals', options: ['equals', 'not_equals', 'greater_than', 'less_than', 'contains', 'regex'] },
          { key: 'caseSensitive', label: 'Case Sensitive', type: 'boolean', value: node.data.properties.caseSensitive || false }
        ];

      case 'output-file':
        return [
          { key: 'filePath', label: 'Output Path', type: 'text', value: node.data.properties.filePath || '', placeholder: '/path/to/output.txt', required: true },
          { key: 'format', label: 'Format', type: 'select', value: node.data.properties.format || 'text', options: ['text', 'json', 'csv', 'xml', 'yaml'] },
          { key: 'append', label: 'Append Mode', type: 'boolean', value: node.data.properties.append || false },
          { key: 'createDirs', label: 'Create Directories', type: 'boolean', value: node.data.properties.createDirs || true }
        ];

      default:
        return [
          { key: 'customProperty', label: 'Custom Property', type: 'text', value: node.data.properties.customProperty || '', placeholder: 'Enter custom value...' }
        ];
    }
  };

  const updateProperty = (key: string, value: any) => {
    if (!selectedNode) return;

    const updatedProperties = { ...selectedNode.data.properties, [key]: value };
    onNodeUpdate(selectedNode.id, {
      data: {
        ...selectedNode.data,
        properties: updatedProperties
      }
    });
  };

  const updateNodeLabel = (label: string) => {
    if (!selectedNode) return;
    setNodeLabel(label);
    onNodeUpdate(selectedNode.id, {
      data: { ...selectedNode.data, label }
    });
  };

  const updateNodeDescription = (description: string) => {
    if (!selectedNode) return;
    setNodeDescription(description);
    onNodeUpdate(selectedNode.id, {
      data: { ...selectedNode.data, description }
    });
  };

  const renderPropertyField = (property: PropertyField) => {
    switch (property.type) {
      case 'text':
        return (
          <input
            type="text"
            value={property.value}
            onChange={(e) => updateProperty(property.key, e.target.value)}
            placeholder={property.placeholder}
            className="property-input"
            required={property.required}
          />
        );

      case 'number':
        return (
          <input
            type="number"
            value={property.value}
            onChange={(e) => updateProperty(property.key, parseFloat(e.target.value) || 0)}
            className="property-input"
            step="any"
          />
        );

      case 'boolean':
        return (
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={property.value}
              onChange={(e) => updateProperty(property.key, e.target.checked)}
              className="property-checkbox"
            />
            <span className="checkbox-text">{property.value ? 'Enabled' : 'Disabled'}</span>
          </label>
        );

      case 'select':
        return (
          <select
            value={property.value}
            onChange={(e) => updateProperty(property.key, e.target.value)}
            className="property-select"
          >
            {property.options?.map(option => (
              <option key={option} value={option}>{option}</option>
            ))}
          </select>
        );

      case 'textarea':
        return (
          <textarea
            value={property.value}
            onChange={(e) => updateProperty(property.key, e.target.value)}
            placeholder={property.placeholder}
            className="property-textarea"
            rows={3}
            required={property.required}
          />
        );

      case 'file':
        return (
          <div className="file-input-group">
            <input
              type="text"
              value={property.value}
              onChange={(e) => updateProperty(property.key, e.target.value)}
              placeholder={property.placeholder}
              className="property-input"
              required={property.required}
            />
            <button className="file-browse-button" onClick={() => {
              // In a real implementation, this would open a file dialog
              const filePath = prompt('Enter file path:');
              if (filePath) updateProperty(property.key, filePath);
            }}>
              📁
            </button>
          </div>
        );

      case 'json':
        return (
          <textarea
            value={property.value}
            onChange={(e) => updateProperty(property.key, e.target.value)}
            placeholder={property.placeholder}
            className="property-textarea json-editor"
            rows={4}
            onBlur={(e) => {
              try {
                JSON.parse(e.target.value);
                e.target.classList.remove('error');
              } catch {
                e.target.classList.add('error');
              }
            }}
          />
        );

      default:
        return null;
    }
  };

  if (!selectedNode) {
    return (
      <div className="property-panel">
        <div className="property-header">
          <h4>⚙️ Properties</h4>
        </div>
        <div className="no-selection">
          <div className="no-selection-icon">🎯</div>
          <p>Select a node to edit its properties</p>
          <div className="selection-tips">
            <h5>Tips:</h5>
            <ul>
              <li>Click on any node to select it</li>
              <li>Use the property panel to configure nodes</li>
              <li>Properties vary by node type</li>
            </ul>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="property-panel">
      <div className="property-header">
        <h4>⚙️ Properties</h4>
        <div className="selected-node-info">
          <span className="node-type-badge">{selectedNode.type}</span>
        </div>
      </div>

      <div className="property-content">
        {/* Basic Node Information */}
        <div className="property-section">
          <h5>📋 Basic Information</h5>
          
          <div className="property-field">
            <label className="property-label">Label</label>
            <input
              type="text"
              value={nodeLabel}
              onChange={(e) => updateNodeLabel(e.target.value)}
              className="property-input"
              placeholder="Node label..."
            />
          </div>

          <div className="property-field">
            <label className="property-label">Description</label>
            <textarea
              value={nodeDescription}
              onChange={(e) => updateNodeDescription(e.target.value)}
              className="property-textarea"
              placeholder="Node description..."
              rows={2}
            />
          </div>

          <div className="property-field">
            <label className="property-label">Node ID</label>
            <input
              type="text"
              value={selectedNode.id}
              className="property-input readonly"
              readOnly
            />
          </div>
        </div>

        {/* Node-Specific Properties */}
        {properties.length > 0 && (
          <div className="property-section">
            <h5>🔧 Configuration</h5>
            {properties.map(property => (
              <div key={property.key} className="property-field">
                <label className="property-label">
                  {property.label}
                  {property.required && <span className="required">*</span>}
                </label>
                {renderPropertyField(property)}
                {property.description && (
                  <div className="property-description">{property.description}</div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Port Information */}
        <div className="property-section">
          <h5>🔌 Ports</h5>
          
          {selectedNode.inputs.length > 0 && (
            <div className="port-group">
              <h6>Input Ports</h6>
              {selectedNode.inputs.map(port => (
                <div key={port.id} className="port-info">
                  <div className="port-name">{port.name}</div>
                  <div className="port-type">{port.dataType}</div>
                  <div className={`port-status ${port.connected ? 'connected' : 'disconnected'}`}>
                    {port.connected ? '🔗 Connected' : '⚪ Disconnected'}
                  </div>
                </div>
              ))}
            </div>
          )}

          {selectedNode.outputs.length > 0 && (
            <div className="port-group">
              <h6>Output Ports</h6>
              {selectedNode.outputs.map(port => (
                <div key={port.id} className="port-info">
                  <div className="port-name">{port.name}</div>
                  <div className="port-type">{port.dataType}</div>
                  <div className={`port-status ${port.connected ? 'connected' : 'disconnected'}`}>
                    {port.connected ? '🔗 Connected' : '⚪ Disconnected'}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Position Information */}
        <div className="property-section">
          <h5>📍 Position</h5>
          <div className="position-info">
            <div className="position-field">
              <label>X:</label>
              <span>{Math.round(selectedNode.position.x)}</span>
            </div>
            <div className="position-field">
              <label>Y:</label>
              <span>{Math.round(selectedNode.position.y)}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
