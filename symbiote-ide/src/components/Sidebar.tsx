import React, { useState } from 'react';
import { Folder, File, Plus, Search } from 'lucide-react';

interface SidebarProps {
  onFileSelect: (file: string) => void;
  currentFile: string;
}

const Sidebar: React.FC<SidebarProps> = ({ onFileSelect, currentFile }) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['src']));

  // Mock file structure - in real implementation this would come from the file system
  const fileStructure = {
    'src': {
      type: 'folder',
      children: {
        'components': {
          type: 'folder',
          children: {
            'Editor.tsx': { type: 'file', language: 'typescript' },
            'Sidebar.tsx': { type: 'file', language: 'typescript' },
            'Terminal.tsx': { type: 'file', language: 'typescript' },
          }
        },
        'App.tsx': { type: 'file', language: 'typescript' },
        'main.tsx': { type: 'file', language: 'typescript' },
        'App.css': { type: 'file', language: 'css' },
      }
    },
    'src-tauri': {
      type: 'folder',
      children: {
        'src': {
          type: 'folder',
          children: {
            'main.rs': { type: 'file', language: 'rust' },
            'parser.rs': { type: 'file', language: 'rust' },
            'ai_provider.rs': { type: 'file', language: 'rust' },
          }
        },
        'Cargo.toml': { type: 'file', language: 'toml' },
      }
    },
    'package.json': { type: 'file', language: 'json' },
    'README.md': { type: 'file', language: 'markdown' },
  };

  const toggleFolder = (path: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
    }
    setExpandedFolders(newExpanded);
  };

  const renderFileTree = (items: any, basePath = '') => {
    return Object.entries(items).map(([name, item]: [string, any]) => {
      const fullPath = basePath ? `${basePath}/${name}` : name;
      const isExpanded = expandedFolders.has(fullPath);
      const isSelected = currentFile === fullPath;

      if (item.type === 'folder') {
        return (
          <div key={fullPath} className="file-tree-item">
            <div 
              className={`file-tree-node folder ${isExpanded ? 'expanded' : ''}`}
              onClick={() => toggleFolder(fullPath)}
            >
              <Folder size={16} />
              <span>{name}</span>
            </div>
            {isExpanded && item.children && (
              <div className="file-tree-children">
                {renderFileTree(item.children, fullPath)}
              </div>
            )}
          </div>
        );
      } else {
        return (
          <div 
            key={fullPath} 
            className={`file-tree-item file ${isSelected ? 'selected' : ''}`}
            onClick={() => onFileSelect(fullPath)}
          >
            <div className="file-tree-node">
              <File size={16} />
              <span>{name}</span>
            </div>
          </div>
        );
      }
    });
  };

  const filteredFiles = searchTerm 
    ? Object.keys(fileStructure).filter(name => 
        name.toLowerCase().includes(searchTerm.toLowerCase())
      )
    : null;

  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <h3>Explorer</h3>
        <button className="icon-btn" title="New File">
          <Plus size={16} />
        </button>
      </div>

      <div className="search-section">
        <div className="search-input-container">
          <Search size={16} className="search-icon" />
          <input
            type="text"
            placeholder="Search files..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="search-input"
          />
        </div>
      </div>

      <div className="file-tree">
        {searchTerm ? (
          <div className="search-results">
            {filteredFiles?.map(file => (
              <div 
                key={file}
                className={`file-tree-item file ${currentFile === file ? 'selected' : ''}`}
                onClick={() => onFileSelect(file)}
              >
                <File size={16} />
                <span>{file}</span>
              </div>
            ))}
          </div>
        ) : (
          renderFileTree(fileStructure)
        )}
      </div>

      <div className="sidebar-footer">
        <div className="workspace-info">
          <small>SymbioteIDE Workspace</small>
        </div>
      </div>
    </div>
  );
};

export default Sidebar;
