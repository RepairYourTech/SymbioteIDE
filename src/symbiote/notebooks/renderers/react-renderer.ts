/**
 * React Renderer
 * 
 * Renders React components in notebook cells with hot reloading support
 */

import * as vscode from 'vscode';
import { Logger } from '../../utils/logger';
import { CellOutput, ComponentPlaygroundState } from '../types';

export interface ReactRendererConfig {
  enableHotReload?: boolean;
  enableDevTools?: boolean;
  defaultViewport?: {
    width: number;
    height: number;
  };
}

export class ReactRenderer {
  private logger = new Logger('ReactRenderer');
  private config: Required<ReactRendererConfig>;
  private componentCache = new Map<string, any>();
  private renderTargets = new Map<string, vscode.WebviewPanel>();
  
  constructor(config: ReactRendererConfig = {}) {
    this.config = {
      enableHotReload: config.enableHotReload ?? true,
      enableDevTools: config.enableDevTools ?? true,
      defaultViewport: config.defaultViewport ?? { width: 800, height: 600 }
    };
  }
  
  /**
   * Render a React component output
   */
  async renderComponent(
    cellId: string,
    output: CellOutput,
    context: vscode.ExtensionContext
  ): Promise<vscode.WebviewPanel> {
    try {
      // Get or create webview panel
      let panel = this.renderTargets.get(cellId);
      
      if (!panel) {
        panel = vscode.window.createWebviewPanel(
          'reactComponent',
          `React: Cell ${cellId}`,
          vscode.ViewColumn.Two,
          {
            enableScripts: true,
            retainContextWhenHidden: true,
            localResourceRoots: [
              vscode.Uri.joinPath(context.extensionUri, 'media'),
              vscode.Uri.joinPath(context.extensionUri, 'node_modules')
            ]
          }
        );
        
        this.renderTargets.set(cellId, panel);
        
        // Clean up on dispose
        panel.onDidDispose(() => {
          this.renderTargets.delete(cellId);
          this.componentCache.delete(cellId);
        });
      }
      
      // Update webview content
      panel.webview.html = this.getWebviewContent(cellId, output, context, panel.webview);
      
      // Set up message handling
      this.setupMessageHandling(panel, cellId);
      
      return panel;
      
    } catch (error) {
      this.logger.error('Failed to render React component', error);
      throw error;
    }
  }
  
  /**
   * Create a component playground
   */
  async createPlayground(
    cellId: string,
    component: any,
    context: vscode.ExtensionContext
  ): Promise<vscode.WebviewPanel> {
    const panel = vscode.window.createWebviewPanel(
      'reactPlayground',
      `Playground: ${component.name || 'Component'}`,
      vscode.ViewColumn.Two,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [
          vscode.Uri.joinPath(context.extensionUri, 'media'),
          vscode.Uri.joinPath(context.extensionUri, 'node_modules')
        ]
      }
    );
    
    panel.webview.html = this.getPlaygroundContent(cellId, component, context, panel.webview);
    
    this.setupPlaygroundHandling(panel, cellId, component);
    
    return panel;
  }
  
  /**
   * Update component props
   */
  updateComponentProps(cellId: string, props: any): void {
    const panel = this.renderTargets.get(cellId);
    
    if (panel) {
      panel.webview.postMessage({
        type: 'updateProps',
        props
      });
    }
  }
  
  /**
   * Update component state
   */
  updateComponentState(cellId: string, state: any): void {
    const panel = this.renderTargets.get(cellId);
    
    if (panel) {
      panel.webview.postMessage({
        type: 'updateState',
        state
      });
    }
  }
  
  /**
   * Dispose all render targets
   */
  dispose(): void {
    this.renderTargets.forEach(panel => panel.dispose());
    this.renderTargets.clear();
    this.componentCache.clear();
  }
  
  // Private methods
  
  private getWebviewContent(
    cellId: string,
    output: CellOutput,
    context: vscode.ExtensionContext,
    webview: vscode.Webview
  ): string {
    const reactUri = webview.asWebviewUri(
      vscode.Uri.joinPath(context.extensionUri, 'node_modules', 'react', 'umd', 'react.development.js')
    );
    const reactDomUri = webview.asWebviewUri(
      vscode.Uri.joinPath(context.extensionUri, 'node_modules', 'react-dom', 'umd', 'react-dom.development.js')
    );
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(context.extensionUri, 'media', 'notebook-react.css')
    );
    
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>React Component</title>
    <link href="${styleUri}" rel="stylesheet">
    <script crossorigin src="${reactUri}"></script>
    <script crossorigin src="${reactDomUri}"></script>
    <style>
        body {
            margin: 0;
            padding: 20px;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
        }
        #root {
            width: 100%;
            height: 100%;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .error {
            color: var(--vscode-errorForeground);
            background: var(--vscode-inputValidation-errorBackground);
            padding: 10px;
            border-radius: 4px;
            border: 1px solid var(--vscode-inputValidation-errorBorder);
        }
        .loading {
            color: var(--vscode-descriptionForeground);
        }
    </style>
</head>
<body>
    <div id="root">
        <div class="loading">Loading component...</div>
    </div>
    
    <script>
        const vscode = acquireVsCodeApi();
        let Component = null;
        let currentProps = {};
        let currentState = {};
        
        // Component definition will be injected here
        ${this.getComponentCode(output)}
        
        // Render function
        function renderComponent() {
            try {
                if (!Component) {
                    throw new Error('Component not defined');
                }
                
                const element = React.createElement(Component, currentProps);
                ReactDOM.render(element, document.getElementById('root'));
                
                vscode.postMessage({
                    type: 'rendered',
                    cellId: '${cellId}'
                });
            } catch (error) {
                document.getElementById('root').innerHTML = \`
                    <div class="error">
                        <strong>Error rendering component:</strong><br>
                        \${error.message}
                    </div>
                \`;
                
                vscode.postMessage({
                    type: 'error',
                    cellId: '${cellId}',
                    error: error.message
                });
            }
        }
        
        // Message handling
        window.addEventListener('message', event => {
            const message = event.data;
            
            switch (message.type) {
                case 'updateProps':
                    currentProps = message.props;
                    renderComponent();
                    break;
                    
                case 'updateState':
                    currentState = message.state;
                    renderComponent();
                    break;
                    
                case 'refresh':
                    renderComponent();
                    break;
            }
        });
        
        // Initial render
        renderComponent();
        
        // Enable hot reload
        if (${this.config.enableHotReload}) {
            setInterval(() => {
                vscode.postMessage({
                    type: 'checkForUpdates',
                    cellId: '${cellId}'
                });
            }, 1000);
        }
    </script>
</body>
</html>`;
  }
  
  private getPlaygroundContent(
    cellId: string,
    component: any,
    context: vscode.ExtensionContext,
    webview: vscode.Webview
  ): string {
    // Similar to getWebviewContent but with additional playground controls
    const baseContent = this.getWebviewContent(cellId, { type: 'react', data: component }, context, webview);
    
    // Add playground controls
    const playgroundControls = `
<div id="playground-controls">
    <div class="control-panel">
        <h3>Props Editor</h3>
        <div id="props-editor"></div>
        
        <h3>State Inspector</h3>
        <div id="state-inspector"></div>
        
        <h3>Viewport</h3>
        <div id="viewport-controls">
            <select id="viewport-preset">
                <option value="mobile">Mobile (375x667)</option>
                <option value="tablet">Tablet (768x1024)</option>
                <option value="desktop" selected>Desktop (1280x720)</option>
                <option value="custom">Custom</option>
            </select>
        </div>
    </div>
</div>

<style>
    #playground-controls {
        position: fixed;
        right: 0;
        top: 0;
        width: 300px;
        height: 100%;
        background: var(--vscode-sideBar-background);
        border-left: 1px solid var(--vscode-panel-border);
        padding: 20px;
        overflow-y: auto;
    }
    
    #root {
        margin-right: 320px;
    }
    
    .control-panel h3 {
        margin-top: 20px;
        margin-bottom: 10px;
        font-size: 14px;
        font-weight: 600;
    }
    
    #props-editor input,
    #props-editor select,
    #props-editor textarea {
        width: 100%;
        margin-bottom: 8px;
        padding: 4px 8px;
        background: var(--vscode-input-background);
        color: var(--vscode-input-foreground);
        border: 1px solid var(--vscode-input-border);
        border-radius: 2px;
    }
</style>

<script>
    // Additional playground functionality
    function initializePlayground() {
        // Prop editor setup
        const propsEditor = document.getElementById('props-editor');
        
        // Generate prop inputs based on component propTypes or TypeScript types
        // This would be populated dynamically based on component analysis
        
        // Viewport control
        document.getElementById('viewport-preset').addEventListener('change', (e) => {
            const preset = e.target.value;
            let width, height;
            
            switch (preset) {
                case 'mobile':
                    width = 375; height = 667;
                    break;
                case 'tablet':
                    width = 768; height = 1024;
                    break;
                case 'desktop':
                    width = 1280; height = 720;
                    break;
                default:
                    // Custom size
                    return;
            }
            
            vscode.postMessage({
                type: 'updateViewport',
                width,
                height
            });
        });
    }
    
    initializePlayground();
</script>
    `;
    
    return baseContent.replace('</body>', playgroundControls + '</body>');
  }
  
  private getComponentCode(output: CellOutput): string {
    if (output.type !== 'react' || !output.data) {
      return 'Component = null;';
    }
    
    // In a real implementation, we would serialize the component function
    // For now, we'll use a placeholder
    return `
// Component would be injected here
Component = function DemoComponent(props) {
    const [count, setCount] = React.useState(0);
    
    return React.createElement('div', {
        style: {
            padding: '20px',
            textAlign: 'center',
            background: '#f0f0f0',
            borderRadius: '8px'
        }
    }, [
        React.createElement('h2', null, props.title || 'Demo Component'),
        React.createElement('p', null, 'Count: ' + count),
        React.createElement('button', {
            onClick: () => setCount(count + 1),
            style: {
                padding: '8px 16px',
                fontSize: '16px',
                cursor: 'pointer'
            }
        }, 'Increment')
    ]);
};
    `;
  }
  
  private setupMessageHandling(panel: vscode.WebviewPanel, cellId: string): void {
    panel.webview.onDidReceiveMessage(
      message => {
        switch (message.type) {
          case 'rendered':
            this.logger.info(`Component rendered successfully: ${cellId}`);
            break;
            
          case 'error':
            this.logger.error(`Component render error: ${cellId}`, message.error);
            break;
            
          case 'checkForUpdates':
            // In a real implementation, check if component code has changed
            break;
        }
      },
      undefined
    );
  }
  
  private setupPlaygroundHandling(
    panel: vscode.WebviewPanel,
    cellId: string,
    component: any
  ): void {
    panel.webview.onDidReceiveMessage(
      message => {
        switch (message.type) {
          case 'updateViewport':
            // Update viewport size
            panel.webview.postMessage({
              type: 'setViewportSize',
              width: message.width,
              height: message.height
            });
            break;
            
          case 'propChanged':
            // Handle prop changes from playground
            this.updateComponentProps(cellId, message.props);
            break;
            
          case 'exportComponent':
            // Export component with current settings
            this.exportComponent(cellId, component, message.settings);
            break;
        }
      },
      undefined
    );
  }
  
  private exportComponent(cellId: string, component: any, settings: any): void {
    // Export component code with current props and styling
    this.logger.info(`Exporting component ${cellId} with settings`, settings);
  }
}