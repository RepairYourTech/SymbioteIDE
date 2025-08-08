import React, { useState, useEffect, useRef } from 'react';
import Editor from '@monaco-editor/react';
import { invoke } from '@tauri-apps/api/core';

interface EditorTab {
  id: string;
  name: string;
  path: string;
  content: string;
  modified: boolean;
  language: string;
}

interface CodeEditorProps {
  activeFile?: string;
  onFileChange?: (filePath: string, content: string) => void;
}

export const CodeEditor: React.FC<CodeEditorProps> = ({ 
  activeFile, 
  onFileChange 
}) => {
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [editorContent, setEditorContent] = useState('');
  const editorRef = useRef<any>(null);

  useEffect(() => {
    if (activeFile && !tabs.find(tab => tab.path === activeFile)) {
      openFile(activeFile);
    }
  }, [activeFile]);

  const openFile = async (filePath: string) => {
    try {
      // For now, create mock content - will connect to Tauri file system later
      const fileName = filePath.split('/').pop() || 'untitled';
      const language = getLanguageFromPath(filePath);
      
      let content = '';
      if (filePath.includes('App.tsx')) {
        content = `import React from 'react';
import './App.css';

function App() {
  return (
    <div className="App">
      <header className="App-header">
        <h1>SymbioteIDE</h1>
        <p>The most advanced AI-native development environment</p>
      </header>
    </div>
  );
}

export default App;`;
      } else if (filePath.includes('package.json')) {
        content = `{
  "name": "symbiote-ide",
  "version": "0.1.0",
  "private": true,
  "dependencies": {
    "react": "^18.2.0",
    "typescript": "^5.0.0"
  }
}`;
      } else {
        content = `// Welcome to SymbioteIDE
// This is a placeholder file
console.log('Hello from ${fileName}');`;
      }

      const newTab: EditorTab = {
        id: `tab-${Date.now()}`,
        name: fileName,
        path: filePath,
        content,
        modified: false,
        language,
      };

      setTabs(prev => [...prev, newTab]);
      setActiveTabId(newTab.id);
      setEditorContent(content);
    } catch (error) {
      console.error('Failed to open file:', error);
    }
  };

  const closeTab = (tabId: string) => {
    const tab = tabs.find(t => t.id === tabId);
    if (tab?.modified) {
      const confirmed = window.confirm(`Save changes to ${tab.name}?`);
      if (confirmed) {
        saveFile(tab);
      }
    }

    setTabs(prev => prev.filter(t => t.id !== tabId));
    
    if (activeTabId === tabId) {
      const remainingTabs = tabs.filter(t => t.id !== tabId);
      if (remainingTabs.length > 0) {
        const newActiveTab = remainingTabs[remainingTabs.length - 1];
        setActiveTabId(newActiveTab.id);
        setEditorContent(newActiveTab.content);
      } else {
        setActiveTabId(null);
        setEditorContent('');
      }
    }
  };

  const saveFile = async (tab: EditorTab) => {
    try {
      // Will connect to Tauri file system later
      console.log('Saving file:', tab.path, tab.content);
      
      // Update tab to mark as saved
      setTabs(prev => prev.map(t => 
        t.id === tab.id ? { ...t, modified: false } : t
      ));

      if (onFileChange) {
        onFileChange(tab.path, tab.content);
      }
    } catch (error) {
      console.error('Failed to save file:', error);
    }
  };

  const handleEditorChange = (value: string | undefined) => {
    if (value !== undefined && activeTabId) {
      setEditorContent(value);
      
      // Mark tab as modified
      setTabs(prev => prev.map(tab => 
        tab.id === activeTabId 
          ? { ...tab, content: value, modified: true }
          : tab
      ));
    }
  };

  const switchTab = (tabId: string) => {
    const tab = tabs.find(t => t.id === tabId);
    if (tab) {
      setActiveTabId(tabId);
      setEditorContent(tab.content);
    }
  };

  const getLanguageFromPath = (path: string): string => {
    const ext = path.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
      case 'tsx':
        return 'typescript';
      case 'js':
      case 'jsx':
        return 'javascript';
      case 'json':
        return 'json';
      case 'css':
        return 'css';
      case 'html':
        return 'html';
      case 'md':
        return 'markdown';
      case 'py':
        return 'python';
      case 'rs':
        return 'rust';
      default:
        return 'plaintext';
    }
  };

  const activeTab = tabs.find(t => t.id === activeTabId);

  return (
    <div className="code-editor">
      {/* Editor Tabs */}
      <div className="editor-tabs">
        {tabs.map(tab => (
          <div
            key={tab.id}
            className={`editor-tab ${activeTabId === tab.id ? 'active' : ''}`}
            onClick={() => switchTab(tab.id)}
          >
            <span className="tab-name">
              {tab.name}
              {tab.modified && <span className="modified-indicator">●</span>}
            </span>
            <button
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                closeTab(tab.id);
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Editor Content */}
      <div className="editor-content">
        {activeTab ? (
          <Editor
            height="100%"
            language={activeTab.language}
            value={editorContent}
            onChange={handleEditorChange}
            onMount={(editor) => {
              editorRef.current = editor;
            }}
            options={{
              minimap: { enabled: true },
              fontSize: 14,
              lineNumbers: 'on',
              roundedSelection: false,
              scrollBeyondLastLine: false,
              automaticLayout: true,
              theme: 'vs-dark',
            }}
          />
        ) : (
          <div className="editor-welcome">
            <h2>Welcome to SymbioteIDE</h2>
            <p>Open a file from the explorer to start editing</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default CodeEditor;
