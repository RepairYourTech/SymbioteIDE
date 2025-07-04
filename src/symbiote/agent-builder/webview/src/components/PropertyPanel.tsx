import React, { useState, useEffect } from 'react';
import { WorkflowNode, AgentNodeConfig, ToolNodeConfig, ConditionNodeConfig, LoopNodeConfig, DataNodeConfig } from '../../../types/workflow-types';

interface PropertyPanelProps {
  node: WorkflowNode | null;
  onNodeUpdate: (node: WorkflowNode) => void;
}

const PropertyPanel: React.FC<PropertyPanelProps> = ({ node, onNodeUpdate }) => {
  const [config, setConfig] = useState<any>({});
  
  useEffect(() => {
    if (node) {
      setConfig(node.data.config || {});
    }
  }, [node]);
  
  if (!node) {
    return (
      <div className="property-panel">
        <h3>Properties</h3>
        <p className="empty-state">Select a node to edit its properties</p>
      </div>
    );
  }
  
  const handleConfigChange = (key: string, value: any) => {
    const updatedConfig = { ...config, [key]: value };
    setConfig(updatedConfig);
    
    onNodeUpdate({
      ...node,
      data: {
        ...node.data,
        config: updatedConfig
      }
    });
  };
  
  const handleLabelChange = (label: string) => {
    onNodeUpdate({
      ...node,
      data: {
        ...node.data,
        label
      }
    });
  };
  
  const renderAgentProperties = () => {
    const agentConfig = config as AgentNodeConfig;
    const agentTypes = [
      'architecture-analyst',
      'performance-optimizer',
      'security-auditor',
      'documentation-generator',
      'database-architect',
      'test-strategy',
      'code-refactoring',
      'dependency-analyzer'
    ];
    
    return (
      <>
        <div className="property-group">
          <label>Agent Type</label>
          <select
            value={agentConfig.agentType || ''}
            onChange={(e) => handleConfigChange('agentType', e.target.value)}
          >
            <option value="">Select agent type...</option>
            {agentTypes.map(type => (
              <option key={type} value={type}>
                {type.replace(/-/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
              </option>
            ))}
          </select>
        </div>
        
        <div className="property-group">
          <label>Temperature</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.1"
            value={agentConfig.temperature || 0.5}
            onChange={(e) => handleConfigChange('temperature', parseFloat(e.target.value))}
          />
          <span>{agentConfig.temperature || 0.5}</span>
        </div>
        
        <div className="property-group">
          <label>Max Tokens</label>
          <input
            type="number"
            value={agentConfig.maxTokens || 2000}
            onChange={(e) => handleConfigChange('maxTokens', parseInt(e.target.value))}
          />
        </div>
        
        <div className="property-group">
          <label>Enable Memory</label>
          <input
            type="checkbox"
            checked={agentConfig.memory || false}
            onChange={(e) => handleConfigChange('memory', e.target.checked)}
          />
        </div>
      </>
    );
  };
  
  const renderToolProperties = () => {
    const toolConfig = config as ToolNodeConfig;
    
    return (
      <>
        <div className="property-group">
          <label>Tool Type</label>
          <select
            value={toolConfig.toolType || 'mcp'}
            onChange={(e) => handleConfigChange('toolType', e.target.value)}
          >
            <option value="mcp">MCP Tool</option>
            <option value="custom">Custom Tool</option>
            <option value="api">API Call</option>
            <option value="builtin">Built-in</option>
          </select>
        </div>
        
        <div className="property-group">
          <label>Tool ID</label>
          <input
            type="text"
            value={toolConfig.toolId || ''}
            onChange={(e) => handleConfigChange('toolId', e.target.value)}
            placeholder="Enter tool ID"
          />
        </div>
        
        {toolConfig.toolType === 'mcp' && (
          <div className="property-group">
            <label>MCP Server</label>
            <input
              type="text"
              value={toolConfig.mcpServer || ''}
              onChange={(e) => handleConfigChange('mcpServer', e.target.value)}
              placeholder="MCP server name"
            />
          </div>
        )}
        
        <div className="property-group">
          <label>Timeout (ms)</label>
          <input
            type="number"
            value={toolConfig.timeout || 30000}
            onChange={(e) => handleConfigChange('timeout', parseInt(e.target.value))}
          />
        </div>
        
        <div className="property-group">
          <label>Retries</label>
          <input
            type="number"
            value={toolConfig.retries || 3}
            onChange={(e) => handleConfigChange('retries', parseInt(e.target.value))}
          />
        </div>
      </>
    );
  };
  
  const renderConditionProperties = () => {
    const conditionConfig = config as ConditionNodeConfig;
    
    return (
      <>
        <div className="property-group">
          <label>Language</label>
          <select
            value={conditionConfig.language || 'javascript'}
            onChange={(e) => handleConfigChange('language', e.target.value)}
          >
            <option value="javascript">JavaScript</option>
            <option value="python">Python</option>
            <option value="natural">Natural Language</option>
          </select>
        </div>
        
        <div className="property-group">
          <label>Expression</label>
          <textarea
            value={conditionConfig.expression || ''}
            onChange={(e) => handleConfigChange('expression', e.target.value)}
            placeholder="Enter condition expression"
            rows={4}
          />
        </div>
      </>
    );
  };
  
  const renderLoopProperties = () => {
    const loopConfig = config as LoopNodeConfig;
    
    return (
      <>
        <div className="property-group">
          <label>Loop Type</label>
          <select
            value={loopConfig.loopType || 'forEach'}
            onChange={(e) => handleConfigChange('loopType', e.target.value)}
          >
            <option value="forEach">For Each</option>
            <option value="while">While</option>
            <option value="for">For</option>
          </select>
        </div>
        
        {loopConfig.loopType === 'while' && (
          <div className="property-group">
            <label>Condition</label>
            <input
              type="text"
              value={loopConfig.condition || ''}
              onChange={(e) => handleConfigChange('condition', e.target.value)}
              placeholder="Loop condition"
            />
          </div>
        )}
        
        {loopConfig.loopType === 'forEach' && (
          <div className="property-group">
            <label>Items Expression</label>
            <input
              type="text"
              value={loopConfig.items || ''}
              onChange={(e) => handleConfigChange('items', e.target.value)}
              placeholder="Array or expression"
            />
          </div>
        )}
        
        <div className="property-group">
          <label>Max Iterations</label>
          <input
            type="number"
            value={loopConfig.maxIterations || 100}
            onChange={(e) => handleConfigChange('maxIterations', parseInt(e.target.value))}
          />
        </div>
        
        <div className="property-group">
          <label>Parallel Execution</label>
          <input
            type="checkbox"
            checked={loopConfig.parallel || false}
            onChange={(e) => handleConfigChange('parallel', e.target.checked)}
          />
        </div>
      </>
    );
  };
  
  const renderDataProperties = () => {
    const dataConfig = config as DataNodeConfig;
    
    return (
      <>
        <div className="property-group">
          <label>Data Type</label>
          <select
            value={dataConfig.dataType || 'constant'}
            onChange={(e) => handleConfigChange('dataType', e.target.value)}
          >
            <option value="input">Input</option>
            <option value="output">Output</option>
            <option value="constant">Constant</option>
            <option value="variable">Variable</option>
          </select>
        </div>
        
        {(dataConfig.dataType === 'constant' || dataConfig.dataType === 'variable') && (
          <div className="property-group">
            <label>Value</label>
            <textarea
              value={dataConfig.value || ''}
              onChange={(e) => handleConfigChange('value', e.target.value)}
              placeholder="Enter value (JSON)"
              rows={4}
            />
          </div>
        )}
        
        <div className="property-group">
          <label>Validation</label>
          <textarea
            value={dataConfig.validation || ''}
            onChange={(e) => handleConfigChange('validation', e.target.value)}
            placeholder="Validation expression"
            rows={2}
          />
        </div>
      </>
    );
  };
  
  const renderProperties = () => {
    switch (node.type) {
      case 'agent':
        return renderAgentProperties();
      case 'tool':
        return renderToolProperties();
      case 'condition':
        return renderConditionProperties();
      case 'loop':
        return renderLoopProperties();
      case 'data':
      case 'input':
      case 'output':
      case 'transform':
      case 'merge':
      case 'split':
        return renderDataProperties();
      default:
        return <p>No properties available</p>;
    }
  };
  
  return (
    <div className="property-panel">
      <h3>Properties</h3>
      
      <div className="property-section">
        <h4>General</h4>
        
        <div className="property-group">
          <label>ID</label>
          <input type="text" value={node.id} disabled />
        </div>
        
        <div className="property-group">
          <label>Label</label>
          <input
            type="text"
            value={node.data.label}
            onChange={(e) => handleLabelChange(e.target.value)}
          />
        </div>
        
        <div className="property-group">
          <label>Type</label>
          <input type="text" value={node.type} disabled />
        </div>
      </div>
      
      <div className="property-section">
        <h4>Configuration</h4>
        {renderProperties()}
      </div>
    </div>
  );
};

export default PropertyPanel;