# Integrated Development Server & Visual Debugging
## AI Master Tool - Local Hosting, Live Preview, and Element Inspection

**Version:** 1.0  
**Date:** January 2025  
**Status:** Draft

---

## Overview

AI Master Tool includes a comprehensive local development server with an integrated browser panel that enables real-time preview, hot reloading, and most importantly, AI-assisted visual debugging through direct element interaction.

## Core Architecture

### 1. Development Server System

```rust
pub struct DevelopmentServer {
    servers: HashMap<ProjectId, LocalServer>,
    proxy_manager: ProxyManager,
    ssl_manager: SSLManager,
    port_manager: PortManager,
    
    pub async fn start_server(&mut self, project: &Project) -> Result<ServerInfo> {
        // Detect project type and requirements
        let config = self.analyze_project(project).await?;
        
        // Allocate port
        let port = self.port_manager.allocate_port(&config).await?;
        
        // Start appropriate server
        let server = match config.server_type {
            ServerType::Static => self.start_static_server(project, port).await?,
            ServerType::Node => self.start_node_server(project, port).await?,
            ServerType::Next => self.start_next_server(project, port).await?,
            ServerType::Vite => self.start_vite_server(project, port).await?,
            ServerType::Custom => self.start_custom_server(project, port, &config).await?,
        };
        
        // Set up hot reload
        self.setup_hot_reload(&server).await?;
        
        // Configure proxy if needed
        if config.needs_proxy {
            self.proxy_manager.setup_proxy(&server, &config.proxy_rules).await?;
        }
        
        Ok(ServerInfo {
            url: format!("http://localhost:{}", port),
            server_id: server.id,
            features: server.capabilities,
        })
    }
}
```

### 2. Integrated Browser Panel

```typescript
class IntegratedBrowser {
  private webview: WebviewPanel;
  private devtools: DevToolsIntegration;
  private elementInspector: ElementInspector;
  private aiAssistant: VisualAIAssistant;
  
  async initialize(serverUrl: string): Promise<void> {
    // Create webview with enhanced capabilities
    this.webview = await this.createEnhancedWebview({
      url: serverUrl,
      enableDevTools: true,
      enableElementSelection: true,
      enableNetworkInterception: true,
    });
    
    // Inject element inspection system
    await this.injectInspectionSystem();
    
    // Set up AI communication bridge
    await this.setupAIBridge();
    
    // Enable hot reload
    await this.enableHotReload();
  }
  
  private async injectInspectionSystem(): Promise<void> {
    // Inject JavaScript for element inspection
    await this.webview.executeScript(`
      window.__AI_MASTER_TOOL__ = {
        inspectionMode: false,
        selectedElement: null,
        
        enableInspection: function() {
          this.inspectionMode = true;
          document.body.style.cursor = 'crosshair';
          this.attachInspectionListeners();
        },
        
        attachInspectionListeners: function() {
          document.addEventListener('mouseover', this.handleHover);
          document.addEventListener('click', this.handleClick);
          document.addEventListener('contextmenu', this.handleRightClick);
        },
        
        handleHover: function(e) {
          if (!this.inspectionMode) return;
          
          // Highlight element
          const element = e.target;
          this.highlightElement(element);
          
          // Show element info tooltip
          this.showElementInfo(element);
        },
        
        handleClick: function(e) {
          if (!this.inspectionMode) return;
          e.preventDefault();
          e.stopPropagation();
          
          const element = e.target;
          this.selectedElement = element;
          
          // Send element data to AI
          window.postMessage({
            type: 'element-selected',
            data: this.extractElementData(element)
          }, '*');
        },
        
        extractElementData: function(element) {
          return {
            tagName: element.tagName,
            id: element.id,
            classes: Array.from(element.classList),
            attributes: this.getAttributes(element),
            computedStyles: window.getComputedStyle(element),
            boundingBox: element.getBoundingClientRect(),
            innerHTML: element.innerHTML.substring(0, 200),
            path: this.getElementPath(element),
            componentInfo: this.detectComponent(element),
          };
        },
        
        detectComponent: function(element) {
          // Detect React/Vue/Angular components
          const react = element._reactInternalFiber || element.__reactInternalInstance;
          const vue = element.__vue__;
          const angular = element.__ngContext__;
          
          return {
            framework: react ? 'React' : vue ? 'Vue' : angular ? 'Angular' : 'vanilla',
            componentName: this.getComponentName(element),
            props: this.getComponentProps(element),
            state: this.getComponentState(element),
          };
        }
      };
    `);
  }
}
```

### 3. Visual AI Assistant

```typescript
class VisualAIAssistant {
  private selectedElement: ElementData | null = null;
  private contextualChat: ContextualChat;
  
  async handleElementSelection(elementData: ElementData): Promise<void> {
    this.selectedElement = elementData;
    
    // CRITICAL: Deep codebase integration
    const enrichedElement = await this.enrichWithCodebaseKnowledge(elementData);
    
    // Update AI context with FULL codebase awareness
    await this.updateAIContext({
      type: 'element-focus',
      element: enrichedElement,
      screenshot: await this.captureElementScreenshot(elementData),
      parentContext: await this.getParentContext(elementData),
      // Deep codebase integration
      componentSource: enrichedElement.source,
      dataFlow: enrichedElement.dataFlow,
      relatedCode: enrichedElement.relatedCode,
      gitHistory: enrichedElement.history,
      patterns: enrichedElement.patterns,
      testCoverage: enrichedElement.tests,
      performance: enrichedElement.performance,
    });
    
    // Show contextual chat with full context
    await this.showContextualChat(enrichedElement);
  }
  
  private async enrichWithCodebaseKnowledge(
    element: ElementData
  ): Promise<EnrichedElementData> {
    // This is where the magic happens - deep integration with codebase awareness
    const codebaseIntegration = new ElementCodebaseIntegration(
      this.codebaseIndex,
      this.parser,
      this.vectorStore
    );
    
    return await codebaseIntegration.enrich(element);
  }
  
  async showContextualChat(element: ElementData): Promise<void> {
    const chat = new ContextualChat({
      title: `${element.tagName}${element.id ? '#' + element.id : ''}`,
      context: element,
      suggestions: this.generateSuggestions(element),
      position: this.calculateOptimalPosition(element.boundingBox),
    });
    
    chat.on('message', async (message) => {
      const response = await this.processElementQuery(message, element);
      chat.addResponse(response);
    });
    
    this.contextualChat = chat;
    chat.show();
  }
  
  private generateSuggestions(element: ElementData): Suggestion[] {
    const suggestions: Suggestion[] = [];
    
    // Style-related suggestions
    suggestions.push({
      text: "Change styling",
      icon: "palette",
      action: () => this.showStyleEditor(element),
    });
    
    // Content suggestions
    if (element.tagName === 'BUTTON' || element.tagName === 'A') {
      suggestions.push({
        text: "Update click handler",
        icon: "click",
        action: () => this.updateClickHandler(element),
      });
    }
    
    // Layout suggestions
    suggestions.push({
      text: "Adjust spacing",
      icon: "spacing",
      action: () => this.adjustSpacing(element),
    });
    
    // Component suggestions
    if (element.componentInfo.framework !== 'vanilla') {
      suggestions.push({
        text: "Refactor component",
        icon: "component",
        action: () => this.refactorComponent(element),
      });
    }
    
    return suggestions;
  }
  
  async processElementQuery(query: string, element: EnrichedElementData): Promise<AIResponse> {
    // Build comprehensive context using FULL codebase awareness
    const context = await this.codebaseContextBuilder.buildForElement(
      element,
      query
    );
    
    // Enhance query with deep element context
    const enhancedPrompt = `
User is asking about a specific UI element with COMPLETE codebase context:

--- ELEMENT ---
- Element: ${element.tagName}${element.id ? '#' + element.id : ''}
- Classes: ${element.classes.join(' ')}
- Component: ${element.componentInfo.componentName || 'N/A'}

--- SOURCE CODE ---
File: ${element.source.file_path} (lines ${element.source.line_start}-${element.source.line_end})
\`\`\`${element.source.language}
${element.source.implementation}
\`\`\`

--- DATA FLOW ---
Props origins: ${JSON.stringify(element.propsWithOrigins, null, 2)}
State mutations: ${JSON.stringify(element.stateMutations, null, 2)}

--- RELATED CODE ---
${element.relatedCode.map(c => `- ${c.type}: ${c.name} in ${c.file}`).join('\n')}

--- PATTERNS ---
${element.patterns.map(p => `- ${p.name}: ${p.description}`).join('\n')}

--- TESTS ---
Coverage: Unit ${element.tests.coverage.unit}%, Integration ${element.tests.coverage.integration}%
Missing: ${element.tests.suggestions.join(', ')}

--- PERFORMANCE ---
Render: ${element.performance.renderTime}ms, Issues: ${element.performance.issues.join(', ')}

--- USER QUERY ---
${query}

Provide specific, actionable advice using ALL the codebase context above.
`;
    
    const response = await this.ai.complete(enhancedPrompt);
    
    // Generate code changes if applicable
    if (this.requiresCodeChange(query)) {
      const codeChanges = await this.generateCodeChanges(element, query, response);
      return {
        text: response.text,
        codeChanges: codeChanges,
        preview: await this.generatePreview(codeChanges),
      };
    }
    
    return response;
  }
}
```

### 4. Hot Reload & Live Updates

```rust
pub struct HotReloadSystem {
    file_watcher: FileWatcher,
    websocket_server: WebSocketServer,
    change_detector: ChangeDetector,
    
    pub async fn setup(&mut self, project: &Project) -> Result<()> {
        // Watch project files
        self.file_watcher.watch(&project.root, WatchOptions {
            recursive: true,
            ignore: vec![
                "node_modules/**",
                ".git/**",
                "dist/**",
                "build/**",
            ],
        }).await?;
        
        // Set up WebSocket for browser communication
        self.websocket_server.start(WebSocketConfig {
            port: project.ws_port,
            heartbeat: Duration::from_secs(30),
        }).await?;
        
        // Handle file changes
        self.file_watcher.on_change(move |change| {
            self.handle_file_change(change).await
        });
        
        Ok(())
    }
    
    async fn handle_file_change(&self, change: FileChange) -> Result<()> {
        match change.file_type {
            FileType::JavaScript | FileType::TypeScript => {
                // Hot module replacement
                self.websocket_server.broadcast(json!({
                    "type": "hmr",
                    "file": change.path,
                    "content": change.content,
                })).await?;
            },
            FileType::CSS | FileType::SCSS => {
                // CSS hot reload
                self.websocket_server.broadcast(json!({
                    "type": "css-update",
                    "file": change.path,
                    "content": self.compile_css(&change.content)?,
                })).await?;
            },
            FileType::HTML => {
                // Full reload for HTML changes
                self.websocket_server.broadcast(json!({
                    "type": "reload",
                    "reason": "html-change",
                })).await?;
            },
            _ => {},
        }
        
        Ok(())
    }
}
```

### 5. Multi-Device Preview

```typescript
class MultiDevicePreview {
  private devices: Device[] = [
    { name: "iPhone 14", width: 390, height: 844 },
    { name: "iPad", width: 810, height: 1080 },
    { name: "Desktop", width: 1920, height: 1080 },
    { name: "Galaxy S21", width: 360, height: 800 },
  ];
  
  async showMultiDevice(serverUrl: string): Promise<void> {
    const panel = new MultiDevicePanel();
    
    for (const device of this.devices) {
      const frame = await panel.addDeviceFrame(device);
      
      // Create iframe with device dimensions
      const iframe = document.createElement('iframe');
      iframe.src = serverUrl;
      iframe.style.width = `${device.width}px`;
      iframe.style.height = `${device.height}px`;
      iframe.style.transform = `scale(${frame.scale})`;
      
      // Enable element inspection in each frame
      iframe.onload = () => {
        this.injectInspectionSystem(iframe.contentWindow);
      };
      
      frame.appendChild(iframe);
    }
    
    panel.show();
  }
}
```

### 6. Network & Performance Monitoring

```typescript
class DevServerMonitor {
  private metrics: PerformanceMetrics;
  private networkInterceptor: NetworkInterceptor;
  
  async startMonitoring(server: LocalServer): Promise<void> {
    // Network monitoring
    this.networkInterceptor.intercept(server, (request, response) => {
      this.metrics.record({
        url: request.url,
        method: request.method,
        status: response.status,
        duration: response.duration,
        size: response.size,
      });
      
      // Show in UI
      this.updateNetworkPanel();
    });
    
    // Performance monitoring
    await this.injectPerformanceMonitor();
    
    // Error tracking
    this.setupErrorTracking();
  }
  
  private async injectPerformanceMonitor(): Promise<void> {
    await this.webview.executeScript(`
      // Web Vitals monitoring
      import {onCLS, onFID, onLCP, onFCP, onTTFB} from 'web-vitals';
      
      const sendMetric = (metric) => {
        window.postMessage({
          type: 'performance-metric',
          data: {
            name: metric.name,
            value: metric.value,
            rating: metric.rating,
          }
        }, '*');
      };
      
      onCLS(sendMetric);
      onFID(sendMetric);
      onLCP(sendMetric);
      onFCP(sendMetric);
      onTTFB(sendMetric);
    `);
  }
}
```

### 7. Proxy & API Mocking

```rust
pub struct ProxyManager {
    rules: Vec<ProxyRule>,
    mock_server: MockServer,
    
    pub async fn setup_proxy(&mut self, config: ProxyConfig) -> Result<()> {
        // API proxying
        self.add_rule(ProxyRule {
            match_pattern: "/api/*",
            target: config.api_server,
            rewrite: Some(|path| path.strip_prefix("/api").unwrap_or(path)),
        });
        
        // Mock endpoints
        if config.enable_mocking {
            self.mock_server.add_endpoints(vec![
                MockEndpoint {
                    path: "/api/users",
                    method: Method::GET,
                    response: json!([
                        {"id": 1, "name": "Test User"},
                        {"id": 2, "name": "Another User"},
                    ]),
                    delay: Some(Duration::from_millis(300)),
                },
            ]);
        }
        
        Ok(())
    }
}
```

### 8. Visual Debugging Tools

```typescript
class VisualDebuggingTools {
  // Grid overlay for alignment
  async showGridOverlay(): Promise<void> {
    await this.webview.executeScript(`
      const grid = document.createElement('div');
      grid.className = 'ai-master-grid-overlay';
      grid.style.cssText = \`
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        pointer-events: none;
        background-image: 
          linear-gradient(rgba(255,0,0,0.1) 1px, transparent 1px),
          linear-gradient(90deg, rgba(255,0,0,0.1) 1px, transparent 1px);
        background-size: 8px 8px;
        z-index: 999999;
      \`;
      document.body.appendChild(grid);
    `);
  }
  
  // Responsive breakpoint indicators
  async showBreakpoints(): Promise<void> {
    const breakpoints = [
      { name: 'Mobile', width: 640, color: '#f59e0b' },
      { name: 'Tablet', width: 768, color: '#3b82f6' },
      { name: 'Desktop', width: 1024, color: '#10b981' },
      { name: 'Wide', width: 1280, color: '#8b5cf6' },
    ];
    
    const indicator = new BreakpointIndicator(breakpoints);
    indicator.attach(this.webview);
  }
  
  // Accessibility overlay
  async showAccessibilityOverlay(): Promise<void> {
    await this.webview.executeScript(`
      // Highlight elements with accessibility issues
      document.querySelectorAll('img:not([alt])').forEach(img => {
        img.style.outline = '3px solid red';
      });
      
      document.querySelectorAll('button:not([aria-label]):empty').forEach(btn => {
        btn.style.outline = '3px solid orange';
      });
      
      // Show contrast issues
      this.checkContrastIssues();
    `);
  }
}
```

### 9. Integration with AI Features

```typescript
class DevServerAIIntegration {
  async setupAIFeatures(): Promise<void> {
    // Auto-fix on save
    this.fileWatcher.on('save', async (file) => {
      if (this.settings.autoFixOnSave) {
        const issues = await this.detectIssues(file);
        if (issues.length > 0) {
          const fixes = await this.ai.generateFixes(issues);
          await this.applyFixes(fixes);
        }
      }
    });
    
    // Real-time suggestions
    this.elementInspector.on('hover', async (element) => {
      const suggestions = await this.ai.getElementSuggestions(element);
      this.showInlineHints(suggestions);
    });
    
    // Performance optimization
    this.performanceMonitor.on('issue', async (issue) => {
      const optimization = await this.ai.suggestOptimization(issue);
      this.notifyUser(optimization);
    });
  }
}
```

### 10. Server Configuration UI

```typescript
interface ServerConfigUI {
  render(): JSX.Element {
    return (
      <ServerPanel>
        <ServerStatus>
          <StatusIndicator active={this.server.isRunning} />
          <ServerUrl>{this.server.url}</ServerUrl>
          <CopyButton onClick={() => this.copyUrl()} />
        </ServerStatus>
        
        <ServerControls>
          <Button onClick={() => this.restart()}>
            <Icon name="refresh" /> Restart
          </Button>
          <Button onClick={() => this.changePort()}>
            <Icon name="port" /> Change Port
          </Button>
          <Button onClick={() => this.openExternal()}>
            <Icon name="external" /> Open External
          </Button>
        </ServerControls>
        
        <ProxySettings>
          <ProxyRule>
            <Input 
              placeholder="/api/*" 
              value={this.proxy.pattern} 
            />
            <Arrow>→</Arrow>
            <Input 
              placeholder="http://localhost:3001" 
              value={this.proxy.target} 
            />
          </ProxyRule>
          <AddProxyButton onClick={() => this.addProxy()}>
            Add Proxy Rule
          </AddProxyButton>
        </ProxySettings>
        
        <EnvironmentVariables>
          {this.env.map(([key, value]) => (
            <EnvVar key={key}>
              <EnvKey>{key}</EnvKey>
              <EnvValue 
                type={this.isSecret(key) ? "password" : "text"}
                value={value}
                onChange={(e) => this.updateEnv(key, e.target.value)}
              />
            </EnvVar>
          ))}
        </EnvironmentVariables>
      </ServerPanel>
    );
  }
}
```

## Integration with Existing Systems

### 1. Task Management Integration

```typescript
class DevServerTaskIntegration {
  async createServerTask(project: Project): Promise<Task> {
    return this.taskManager.createTask({
      type: TaskType.DevServer,
      name: `Start dev server for ${project.name}`,
      actions: [
        { type: 'install-dependencies', agent: 'DevOps' },
        { type: 'start-server', agent: 'Developer' },
        { type: 'validate-endpoints', agent: 'Tester' },
      ],
      autoStart: true,
    });
  }
}
```

### 2. Agent Assistance

```typescript
class DevServerAgentAssist {
  async handleServerIssue(issue: ServerIssue): Promise<void> {
    const specialist = await this.agentManager.getSpecialist('DevOps');
    
    const solution = await specialist.diagnose(issue);
    
    if (solution.autoFixable) {
      await specialist.applyFix(solution);
    } else {
      await this.showManualFixGuide(solution);
    }
  }
}
```

## Benefits

1. **Instant Feedback**: See changes immediately without manual refresh
2. **Surgical Precision**: Click on any element to modify it with AI assistance
3. **Framework Agnostic**: Works with any web framework
4. **Performance Insights**: Real-time metrics and optimization suggestions
5. **Mobile Testing**: Test responsive design without external tools
6. **API Mocking**: Develop frontend without backend dependencies
7. **Visual Debugging**: Grid overlays, breakpoint indicators, accessibility checks
8. **Integrated Experience**: No need to switch between browser and IDE

This integrated development server makes AI Master Tool a complete development environment where users can build, test, and refine their applications with unprecedented ease and precision.