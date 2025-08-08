import React, { useState, useRef } from 'react';
import Editor, { OnMount } from '@monaco-editor/react';
import { Wand2, Play, Bug, Lightbulb } from 'lucide-react';

interface ParseResponse {
  ast: string;
  symbols: string[];
  errors: string[];
}

interface AIResponse {
  content: string;
  provider: string;
  model: string;
  tokens_used: number;
}

interface EditorProps {
  content: string;
  language: string;
  onChange: (content: string, language: string) => void;
  parseResult: ParseResponse | null;
  onAIRequest: (prompt: string) => Promise<AIResponse>;
}

const CodeEditor: React.FC<EditorProps> = ({
  content,
  language,
  onChange,
  parseResult,
  onAIRequest
}) => {
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiResponse, setAiResponse] = useState<AIResponse | null>(null);
  const [isAILoading, setIsAILoading] = useState(false);
  const editorRef = useRef<any>(null);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    
    // Configure Monaco editor
    monaco.editor.defineTheme('symbiote-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [],
      colors: {
        'editor.background': '#0d1117',
        'editor.foreground': '#c9d1d9',
        'editorLineNumber.foreground': '#484f58',
        'editor.selectionBackground': '#264f78',
        'editor.inactiveSelectionBackground': '#3a3d41',
      }
    });
    
    monaco.editor.setTheme('symbiote-dark');

    // Add AI-assisted features
    editor.addAction({
      id: 'ai-explain',
      label: 'AI: Explain Code',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyE],
      run: () => handleAIExplain()
    });

    editor.addAction({
      id: 'ai-refactor',
      label: 'AI: Refactor Code',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyR],
      run: () => handleAIRefactor()
    });

    editor.addAction({
      id: 'ai-debug',
      label: 'AI: Debug Code',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyD],
      run: () => handleAIDebug()
    });
  };

  const handleEditorChange = (value: string | undefined) => {
    if (value !== undefined) {
      onChange(value, language);
    }
  };

  const handleAIExplain = async () => {
    const selection = editorRef.current?.getSelection();
    const selectedText = selection ? editorRef.current?.getModel()?.getValueInRange(selection) : content;
    
    if (!selectedText) return;

    setIsAILoading(true);
    try {
      const response = await onAIRequest(`Explain this ${language} code:\n\n${selectedText}`);
      setAiResponse(response);
    } catch (error) {
      console.error('AI explain failed:', error);
    } finally {
      setIsAILoading(false);
    }
  };

  const handleAIRefactor = async () => {
    const selection = editorRef.current?.getSelection();
    const selectedText = selection ? editorRef.current?.getModel()?.getValueInRange(selection) : content;
    
    if (!selectedText) return;

    setIsAILoading(true);
    try {
      const response = await onAIRequest(`Refactor this ${language} code to improve readability, performance, and maintainability:\n\n${selectedText}`);
      setAiResponse(response);
    } catch (error) {
      console.error('AI refactor failed:', error);
    } finally {
      setIsAILoading(false);
    }
  };

  const handleAIDebug = async () => {
    if (!parseResult?.errors.length) {
      const response = await onAIRequest(`Analyze this ${language} code for potential bugs and issues:\n\n${content}`);
      setAiResponse(response);
      return;
    }

    setIsAILoading(true);
    try {
      const errorContext = `Errors found: ${parseResult.errors.join(', ')}`;
      const response = await onAIRequest(`Debug this ${language} code. ${errorContext}\n\nCode:\n${content}`);
      setAiResponse(response);
    } catch (error) {
      console.error('AI debug failed:', error);
    } finally {
      setIsAILoading(false);
    }
  };

  const handleCustomAIRequest = async () => {
    if (!aiPrompt.trim()) return;

    setIsAILoading(true);
    try {
      const response = await onAIRequest(aiPrompt);
      setAiResponse(response);
      setAiPrompt('');
    } catch (error) {
      console.error('Custom AI request failed:', error);
    } finally {
      setIsAILoading(false);
    }
  };

  const insertAIResponse = () => {
    if (!aiResponse || !editorRef.current) return;

    const position = editorRef.current.getPosition();
    const range = {
      startLineNumber: position.lineNumber,
      startColumn: position.column,
      endLineNumber: position.lineNumber,
      endColumn: position.column
    };

    editorRef.current.executeEdits('ai-insert', [{
      range,
      text: aiResponse.content
    }]);

    setAiResponse(null);
  };

  return (
    <div className="editor-container">
      <div className="editor-toolbar">
        <div className="editor-actions">
          <button 
            onClick={handleAIExplain}
            disabled={isAILoading}
            className="toolbar-btn"
            title="AI Explain (Ctrl+E)"
          >
            <Lightbulb size={16} />
            Explain
          </button>
          
          <button 
            onClick={handleAIRefactor}
            disabled={isAILoading}
            className="toolbar-btn"
            title="AI Refactor (Ctrl+R)"
          >
            <Wand2 size={16} />
            Refactor
          </button>
          
          <button 
            onClick={handleAIDebug}
            disabled={isAILoading}
            className="toolbar-btn"
            title="AI Debug (Ctrl+D)"
          >
            <Bug size={16} />
            Debug
          </button>
        </div>

        <div className="ai-prompt-section">
          <input
            type="text"
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            placeholder="Ask AI about your code..."
            className="ai-prompt-input"
            onKeyPress={(e) => e.key === 'Enter' && handleCustomAIRequest()}
          />
          <button 
            onClick={handleCustomAIRequest}
            disabled={isAILoading || !aiPrompt.trim()}
            className="toolbar-btn primary"
          >
            <Play size={16} />
          </button>
        </div>
      </div>

      <div className="editor-main">
        <Editor
          height="100%"
          language={language}
          value={content}
          onChange={handleEditorChange}
          onMount={handleEditorDidMount}
          options={{
            minimap: { enabled: true },
            fontSize: 14,
            lineNumbers: 'on',
            roundedSelection: false,
            scrollBeyondLastLine: false,
            automaticLayout: true,
            tabSize: 2,
            insertSpaces: true,
            wordWrap: 'on',
            suggestOnTriggerCharacters: true,
            quickSuggestions: true,
            folding: true,
            foldingStrategy: 'auto',
            showFoldingControls: 'always',
          }}
        />
      </div>

      {/* Parse Results Panel */}
      {parseResult && (
        <div className="parse-results">
          <div className="parse-section">
            <h4>Symbols ({parseResult.symbols.length})</h4>
            <div className="symbols-list">
              {parseResult.symbols.slice(0, 10).map((symbol, index) => (
                <span key={index} className="symbol-tag">{symbol}</span>
              ))}
              {parseResult.symbols.length > 10 && (
                <span className="symbol-tag more">+{parseResult.symbols.length - 10} more</span>
              )}
            </div>
          </div>

          {parseResult.errors.length > 0 && (
            <div className="parse-section errors">
              <h4>Errors ({parseResult.errors.length})</h4>
              <div className="errors-list">
                {parseResult.errors.map((error, index) => (
                  <div key={index} className="error-item">{error}</div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* AI Response Panel */}
      {aiResponse && (
        <div className="ai-response-panel">
          <div className="ai-response-header">
            <h4>AI Response ({aiResponse.provider} - {aiResponse.model})</h4>
            <div className="ai-response-actions">
              <button onClick={insertAIResponse} className="btn-insert">
                Insert Code
              </button>
              <button onClick={() => setAiResponse(null)} className="btn-close">
                ×
              </button>
            </div>
          </div>
          <div className="ai-response-content">
            <pre>{aiResponse.content}</pre>
          </div>
          <div className="ai-response-footer">
            Tokens used: {aiResponse.tokens_used}
          </div>
        </div>
      )}

      {isAILoading && (
        <div className="ai-loading-overlay">
          <div className="loading-spinner">AI Processing...</div>
        </div>
      )}
    </div>
  );
};

export default CodeEditor;
