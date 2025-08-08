import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface FileNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children?: FileNode[];
  expanded?: boolean;
}

interface FileExplorerProps {
  onFileSelect: (filePath: string) => void;
}

export const FileExplorer: React.FC<FileExplorerProps> = ({ onFileSelect }) => {
  const [fileTree, setFileTree] = useState<FileNode[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadFileTree();
  }, []);

  const loadFileTree = async () => {
    try {
      setLoading(true);
      // For now, create a mock file tree - will connect to Tauri later
      const mockTree: FileNode[] = [
        {
          name: 'src',
          path: '/src',
          isDirectory: true,
          expanded: true,
          children: [
            {
              name: 'components',
              path: '/src/components',
              isDirectory: true,
              children: [
                { name: 'App.tsx', path: '/src/components/App.tsx', isDirectory: false },
                { name: 'MainIDEInterface.tsx', path: '/src/components/MainIDEInterface.tsx', isDirectory: false },
              ]
            },
            { name: 'main.tsx', path: '/src/main.tsx', isDirectory: false },
          ]
        },
        { name: 'package.json', path: '/package.json', isDirectory: false },
        { name: 'README.md', path: '/README.md', isDirectory: false },
      ];
      setFileTree(mockTree);
    } catch (error) {
      console.error('Failed to load file tree:', error);
    } finally {
      setLoading(false);
    }
  };

  const toggleDirectory = (path: string) => {
    const updateNode = (nodes: FileNode[]): FileNode[] => {
      return nodes.map(node => {
        if (node.path === path && node.isDirectory) {
          return { ...node, expanded: !node.expanded };
        }
        if (node.children) {
          return { ...node, children: updateNode(node.children) };
        }
        return node;
      });
    };
    setFileTree(updateNode(fileTree));
  };

  const renderFileNode = (node: FileNode, level: number = 0) => {
    const indent = level * 16;
    
    return (
      <div key={node.path}>
        <div
          className="file-node"
          style={{ paddingLeft: `${indent}px` }}
          onClick={() => {
            if (node.isDirectory) {
              toggleDirectory(node.path);
            } else {
              onFileSelect(node.path);
            }
          }}
        >
          <span className="file-icon">
            {node.isDirectory ? (node.expanded ? '📂' : '📁') : '📄'}
          </span>
          <span className="file-name">{node.name}</span>
        </div>
        {node.isDirectory && node.expanded && node.children && (
          <div className="file-children">
            {node.children.map(child => renderFileNode(child, level + 1))}
          </div>
        )}
      </div>
    );
  };

  if (loading) {
    return (
      <div className="file-explorer loading">
        <div className="loading-spinner">Loading files...</div>
      </div>
    );
  }

  return (
    <div className="file-explorer">
      <div className="file-explorer-header">
        <h3>Explorer</h3>
        <div className="file-explorer-actions">
          <button onClick={loadFileTree} title="Refresh">
            🔄
          </button>
          <button title="New File">
            📄
          </button>
          <button title="New Folder">
            📁
          </button>
        </div>
      </div>
      <div className="file-tree">
        {fileTree.map(node => renderFileNode(node))}
      </div>
    </div>
  );
};

export default FileExplorer;
