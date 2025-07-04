/**
 * Browser Editor Provider - VS Code custom editor for browser tabs
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { Logger } from '../utils/logger';
import { browserService, BrowserTab } from './browser-service';
import { EventEmitter } from 'events';

export class BrowserDocument extends vscode.Disposable {
  private _uri: vscode.Uri;
  private _tab: BrowserTab | null = null;
  private _onDidChangeDocument = new vscode.EventEmitter<{
    readonly content?: string;
    readonly edits?: readonly any[];
  }>();
  
  public readonly onDidChangeContent = this._onDidChangeDocument.event;
  
  constructor(
    uri: vscode.Uri,
    initialContent: string
  ) {
    super(() => this.dispose());
    this._uri = uri;
  }
  
  get uri() { return this._uri; }
  get tab() { return this._tab; }
  
  setTab(tab: BrowserTab) {
    this._tab = tab;
  }
  
  dispose(): void {
    this._onDidChangeDocument.dispose();
    super.dispose();
  }
}

export class BrowserEditorProvider implements vscode.CustomEditorProvider<BrowserDocument> {
  public static readonly viewType = 'symbiote.browser';
  
  private readonly _onDidChangeCustomDocument = new vscode.EventEmitter<vscode.CustomDocumentEditEvent<BrowserDocument>>();
  public readonly onDidChangeCustomDocument = this._onDidChangeCustomDocument.event;
  
  private logger = new Logger('BrowserEditorProvider');
  private _context: vscode.ExtensionContext;
  
  constructor(private readonly context: vscode.ExtensionContext) {
    this._context = context;
  }
  
  saveCustomDocument(document: BrowserDocument, cancellation: vscode.CancellationToken): Thenable<void> {
    return this.saveCustomDocumentAs(document, document.uri, cancellation);
  }
  
  saveCustomDocumentAs(document: BrowserDocument, destination: vscode.Uri, cancellation: vscode.CancellationToken): Thenable<void> {
    // Browser tabs don't need saving in traditional sense
    return Promise.resolve();
  }
  
  revertCustomDocument(document: BrowserDocument, cancellation: vscode.CancellationToken): Thenable<void> {
    // Reload the page
    if (document.tab) {
      return browserService.reload(document.tab.id);
    }
    return Promise.resolve();
  }
  
  backupCustomDocument(document: BrowserDocument, context: vscode.CustomDocumentBackupContext, cancellation: vscode.CancellationToken): Thenable<vscode.CustomDocumentBackup> {
    // No backup needed for browser tabs
    return Promise.resolve({
      id: context.destination.toString(),
      delete: () => Promise.resolve()
    });
  }
  
  async openCustomDocument(
    uri: vscode.Uri,
    openContext: vscode.CustomDocumentOpenContext,
    token: vscode.CancellationToken
  ): Promise<BrowserDocument> {
    this.logger.info(`Opening browser document: ${uri.toString()}`);
    
    // Extract URL from URI
    const url = this.extractUrlFromUri(uri);
    
    // Create document
    const document = new BrowserDocument(uri, url);
    
    // Initialize browser if needed
    if (!browserService.isRunning()) {
      await browserService.initialize();
    }
    
    // Create browser tab
    const tab = await browserService.createTab(url);
    document.setTab(tab);
    
    return document;
  }
  
  async resolveCustomEditor(
    document: BrowserDocument,
    webviewPanel: vscode.WebviewPanel,
    token: vscode.CancellationToken
  ): Promise<void> {
    this.logger.info('Resolving browser editor');
    
    // Configure webview
    webviewPanel.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.file(path.join(this._context.extensionPath, 'media')),
        vscode.Uri.file(path.join(this._context.extensionPath, 'dist'))
      ]
    };
    
    // Set initial HTML
    webviewPanel.webview.html = this.getWebviewContent(webviewPanel.webview, document);
    
    // Handle messages from webview
    webviewPanel.webview.onDidReceiveMessage(
      message => this.handleWebviewMessage(message, document),
      undefined,
      this._context.subscriptions
    );
    
    // Setup browser event forwarding
    this.setupBrowserEventForwarding(document, webviewPanel);
    
    // Handle panel disposal
    webviewPanel.onDidDispose(() => {
      if (document.tab) {
        browserService.closeTab(document.tab.id).catch(err => {
          this.logger.error('Failed to close browser tab', err);
        });
      }
    });
  }
  
  private extractUrlFromUri(uri: vscode.Uri): string {
    // Format: browser:https://example.com
    const path = uri.path;
    if (path.startsWith('//')) {
      return 'https:' + path;
    } else if (path.startsWith('/')) {
      return 'https:/' + path;
    }
    return path;
  }
  
  private getWebviewContent(webview: vscode.Webview, document: BrowserDocument): string {
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.file(path.join(this._context.extensionPath, 'dist', 'browser-webview.js'))
    );
    
    const styleUri = webview.asWebviewUri(
      vscode.Uri.file(path.join(this._context.extensionPath, 'media', 'browser.css'))
    );
    
    const codiconsUri = webview.asWebviewUri(
      vscode.Uri.file(path.join(this._context.extensionPath, 'node_modules', '@vscode/codicons', 'dist', 'codicon.css'))
    );
    
    return `<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Browser</title>
      <link href="${styleUri}" rel="stylesheet">
      <link href="${codiconsUri}" rel="stylesheet">
    </head>
    <body>
      <div id="browser-container">
        <div id="browser-toolbar">
          <button id="back" class="toolbar-button" title="Back">
            <i class="codicon codicon-arrow-left"></i>
          </button>
          <button id="forward" class="toolbar-button" title="Forward">
            <i class="codicon codicon-arrow-right"></i>
          </button>
          <button id="reload" class="toolbar-button" title="Reload">
            <i class="codicon codicon-refresh"></i>
          </button>
          <input id="url-bar" type="text" value="${document.tab?.url || ''}" />
          <button id="popout" class="toolbar-button" title="Pop Out">
            <i class="codicon codicon-link-external"></i>
          </button>
          <button id="ai-chat" class="toolbar-button" title="AI Chat">
            <i class="codicon codicon-comment"></i>
          </button>
          <button id="element-selector" class="toolbar-button" title="Select Element">
            <i class="codicon codicon-inspect"></i>
          </button>
        </div>
        <div id="browser-viewport">
          <iframe id="browser-frame" src="about:blank" sandbox="allow-scripts allow-same-origin allow-forms allow-popups"></iframe>
        </div>
        <div id="element-overlay" style="display: none;">
          <div id="element-highlight"></div>
          <div id="element-info"></div>
        </div>
        <div id="ai-panel" style="display: none;">
          <div id="ai-header">
            <span>AI Assistant</span>
            <button id="close-ai" class="toolbar-button">
              <i class="codicon codicon-close"></i>
            </button>
          </div>
          <div id="ai-messages"></div>
          <div id="ai-input-container">
            <textarea id="ai-input" placeholder="Ask about this page or selected element..."></textarea>
            <button id="ai-send" class="toolbar-button">
              <i class="codicon codicon-send"></i>
            </button>
          </div>
        </div>
      </div>
      <script>
        const vscode = acquireVsCodeApi();
        const tabId = '${document.tab?.id || ''}';
      </script>
      <script src="${scriptUri}"></script>
    </body>
    </html>`;
  }
  
  private async handleWebviewMessage(message: any, document: BrowserDocument) {
    if (!document.tab) return;
    
    try {
      switch (message.command) {
        case 'navigate':
          await browserService.navigate(document.tab.id, message.url);
          break;
          
        case 'back':
          await browserService.goBack(document.tab.id);
          break;
          
        case 'forward':
          await browserService.goForward(document.tab.id);
          break;
          
        case 'reload':
          await browserService.reload(document.tab.id);
          break;
          
        case 'popout':
          // Open in external browser
          vscode.env.openExternal(vscode.Uri.parse(document.tab.url));
          break;
          
        case 'selectElement':
          // Enable element selection mode
          await this.enableElementSelection(document.tab.id);
          break;
          
        case 'elementSelected':
          // Handle element selection
          await this.handleElementSelection(message.selector, message.info, document);
          break;
          
        case 'aiChat':
          // Handle AI chat message
          await this.handleAIChat(message.text, message.context, document);
          break;
          
        case 'screenshot':
          // Take screenshot
          const screenshot = await browserService.screenshot(document.tab.id, {
            fullPage: message.fullPage
          });
          // Handle screenshot...
          break;
      }
    } catch (error) {
      this.logger.error('Failed to handle webview message', error);
      vscode.window.showErrorMessage(`Browser action failed: ${error}`);
    }
  }
  
  private setupBrowserEventForwarding(document: BrowserDocument, panel: vscode.WebviewPanel) {
    if (!document.tab) return;
    
    // Forward browser events to webview
    browserService.on('navigation', (data) => {
      if (data.tabId === document.tab?.id) {
        panel.webview.postMessage({
          type: 'navigation',
          url: data.url
        });
      }
    });
    
    browserService.on('pageLoad', (data) => {
      if (data.page === document.tab?.page) {
        panel.webview.postMessage({ type: 'pageLoad' });
      }
    });
    
    browserService.on('console', (data) => {
      panel.webview.postMessage({
        type: 'console',
        ...data
      });
    });
    
    browserService.on('pageError', (data) => {
      panel.webview.postMessage({
        type: 'error',
        error: data.error.message
      });
    });
  }
  
  private async enableElementSelection(tabId: string) {
    // Inject element selection script
    await browserService.evaluate(tabId, () => {
      let selectedElement: HTMLElement | null = null;
      let overlay: HTMLDivElement | null = null;
      
      const createOverlay = () => {
        overlay = document.createElement('div');
        overlay.style.position = 'absolute';
        overlay.style.border = '2px solid #007ACC';
        overlay.style.backgroundColor = 'rgba(0, 122, 204, 0.1)';
        overlay.style.pointerEvents = 'none';
        overlay.style.zIndex = '999999';
        document.body.appendChild(overlay);
      };
      
      const updateOverlay = (element: HTMLElement) => {
        if (!overlay) createOverlay();
        const rect = element.getBoundingClientRect();
        overlay!.style.left = `${rect.left + window.scrollX}px`;
        overlay!.style.top = `${rect.top + window.scrollY}px`;
        overlay!.style.width = `${rect.width}px`;
        overlay!.style.height = `${rect.height}px`;
      };
      
      const handleMouseMove = (e: MouseEvent) => {
        const element = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement;
        if (element && element !== selectedElement) {
          selectedElement = element;
          updateOverlay(element);
        }
      };
      
      const handleClick = (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        
        if (selectedElement) {
          // Generate selector
          let selector = '';
          if (selectedElement.id) {
            selector = `#${selectedElement.id}`;
          } else if (selectedElement.className) {
            selector = `.${selectedElement.className.split(' ').join('.')}`;
          } else {
            selector = selectedElement.tagName.toLowerCase();
          }
          
          // Get element info
          const info = {
            tagName: selectedElement.tagName,
            id: selectedElement.id,
            className: selectedElement.className,
            text: selectedElement.textContent?.substring(0, 100),
            attributes: Array.from(selectedElement.attributes).map((attr: Attr) => ({
              name: attr.name,
              value: attr.value
            }))
          };
          
          // Send selection event
          (window as any).vscode.postMessage({
            command: 'elementSelected',
            selector,
            info
          });
          
          // Cleanup
          document.removeEventListener('mousemove', handleMouseMove);
          document.removeEventListener('click', handleClick);
          if (overlay) {
            overlay.remove();
          }
        }
      };
      
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('click', handleClick);
    });
  }
  
  private async handleElementSelection(selector: string, info: any, document: BrowserDocument) {
    // Store selected element info for AI chat context
    (document as any).selectedElement = { selector, info };
    
    // Show element info in webview
    // This will be handled by the webview JavaScript
  }
  
  private async handleAIChat(text: string, context: any, document: BrowserDocument) {
    // This will integrate with the orchestration engine for AI responses
    // For now, just log the request
    this.logger.info('AI chat request', { text, context });
    
    // TODO: Integrate with orchestration engine
    // const response = await orchestrationEngine.processQuery(text, {
    //   type: 'browser',
    //   url: document.tab?.url,
    //   element: (document as any).selectedElement,
    //   ...context
    // });
  }
}

// Register the provider
export function registerBrowserEditor(context: vscode.ExtensionContext): vscode.Disposable {
  const provider = new BrowserEditorProvider(context);
  
  return vscode.window.registerCustomEditorProvider(
    BrowserEditorProvider.viewType,
    provider,
    {
      webviewOptions: {
        retainContextWhenHidden: true
      }
    }
  );
}