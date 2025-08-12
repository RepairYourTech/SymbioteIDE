# Technical Specifications
## AI Master Tool - Architecture and Implementation

**Version:** 1.0  
**Date:** January 2025  
**Status:** Draft

---

## System Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Desktop Application Shell                 │
│                         (Tauri 2.0)                         │
├─────────────────────────────────────────────────────────────┤
│                    Frontend (React + TypeScript)             │
├──────────────┬──────────────┬──────────────┬───────────────┤
│   IDE View   │ Visual Builder│  Agent Chat  │  Settings     │
│              │     View      │    View      │   View        │
├──────────────┴──────────────┴──────────────┴───────────────┤
│                  State Management (Zustand)                  │
├─────────────────────────────────────────────────────────────┤
│                  IPC Communication Layer                     │
├─────────────────────────────────────────────────────────────┤
│                    Backend (Rust)                           │
├──────────────┬──────────────┬──────────────┬───────────────┤
│ AI Provider  │   Browser    │   System     │   Plugin      │
│   Manager    │ Automation   │  Control     │   Engine      │
├──────────────┴──────────────┴──────────────┴───────────────┤
│                    Core Services                            │
├──────────────┬──────────────┬──────────────┬───────────────┤
│   Security   │  File System │   Database   │   Network     │
│   Manager    │   Handler    │   (SQLite)   │   Manager     │
└──────────────┴──────────────┴──────────────┴───────────────┘
```

## Technology Stack

### Frontend Technologies
- **Framework**: React 18+ with TypeScript 5+
- **Build Tool**: Vite 5+
- **State Management**: Zustand
- **UI Components**: Proprietary component system (no external UI libraries)
- **Code Editor**: CodeMirror 6
- **Visual Builder**: React DnD + Custom animation engine
- **Styling**: Custom CSS architecture with scoped styles

### Backend Technologies
- **Core Runtime**: Rust (latest stable)
- **Desktop Framework**: Tauri 2.0
- **Async Runtime**: Tokio
- **Database**: SQLite with SQLx
- **Serialization**: Serde
- **HTTP Client**: Reqwest
- **WebSocket**: Tungstenite

### AI Integration
- **LLM Providers**: 40+ providers including:
  - OpenAI, Anthropic, Google Gemini, Mistral
  - Chinese providers: DeepSeek, Qwen, Moonshot, GLM
  - Fast inference: Groq, SambaNova, Cerebras
  - Open source: HuggingFace, Replicate, Together
  - Local: Ollama, LM Studio, LocalAI, vLLM
  - Specialized: Perplexity (search), Claude Code
  - Cloud: Amazon Bedrock, GCP Vertex AI, Azure OpenAI
  - See `ai-provider-integration.md` for complete list
- **Streaming**: Server-sent events (SSE)
- **Embedding Storage**: Vector database (Qdrant embedded)
- **Memory System**: Custom intelligent memory (not Mem0)
- **Code Analysis**: Proprietary evaluation engine
- **Best Practices**: Built-in enforcement system

### Development Tools
- **Monorepo**: Turborepo
- **Testing**: Vitest (frontend), Rust built-in tests
- **Linting**: ESLint, Clippy
- **Formatting**: Prettier, Rustfmt
- **CI/CD**: GitHub Actions

## Core Modules

### 1. AI Provider Manager

```rust
pub struct AIProviderManager {
    providers: HashMap<String, Box<dyn AIProvider>>,
    active_provider: String,
    config: ProviderConfig,
}

pub trait AIProvider: Send + Sync {
    async fn complete(&self, prompt: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, prompt: CompletionRequest) -> Result<CompletionStream>;
    fn capabilities(&self) -> ProviderCapabilities;
    fn estimate_cost(&self, tokens: TokenCount) -> Cost;
}

pub struct ProviderConfig {
    api_keys: HashMap<String, EncryptedString>,
    endpoints: HashMap<String, Url>,
    retry_policy: RetryPolicy,
    timeout: Duration,
}
```

**Key Features**:
- Unified interface for all AI providers
- Automatic retry with exponential backoff
- Cost tracking and limits
- Request/response caching
- Streaming support for real-time responses

### 2. Code Intelligence Engine

```typescript
interface CodeIntelligence {
  // Multi-file context analysis
  analyzeContext(files: FileMap): Promise<ContextGraph>;
  
  // Smart completions with duplication prevention
  getCompletions(
    position: Position,
    context: ContextGraph
  ): Promise<Completion[]>;
  
  // Refactoring suggestions
  suggestRefactoring(
    selection: Selection,
    context: ContextGraph
  ): Promise<Refactoring[]>;
  
  // Code evaluation (CodeRabbit-style)
  evaluateCode(code: string): Promise<CodeEvaluation>;
  
  // Comprehensive codebase awareness
  getCodebaseContext(query: Query): Promise<EnrichedContext>;
}

class ContextGraph {
  nodes: Map<string, FileNode>;
  edges: Map<string, Dependency[]>;
  semanticIndex: SemanticIndex;
  vectorStore: VectorStore;
  
  // Build dependency graph with semantic understanding
  buildFromFiles(files: FileMap): void;
  
  // Get relevant context for position
  getRelevantContext(file: string, position: Position): Context;
  
  // Find similar code patterns
  findSimilarPatterns(pattern: CodePattern): SimilarPattern[];
}
```

**Note**: See `core-ai-features.md` for detailed implementation of code evaluation, codebase awareness, and anti-duplication systems.

**Implementation Details**:
- AST-based code analysis
- Language server protocol (LSP) integration
- Incremental parsing for performance
- Smart context window management

### 3. Visual Builder Engine

```typescript
interface VisualBuilder {
  // Component library
  components: ComponentRegistry;
  
  // Drag-drop handling
  dndManager: DragDropManager;
  
  // Code generation
  generateCode(tree: ComponentTree): GeneratedCode;
  
  // Two-way sync
  syncWithCode(code: string): ComponentTree;
}

class ComponentRegistry {
  // Built-in components
  private builtIn: Map<string, ComponentDefinition>;
  
  // User custom components  
  private custom: Map<string, ComponentDefinition>;
  
  // Component metadata
  getComponentMeta(type: string): ComponentMeta;
  
  // Validation
  validateProps(type: string, props: any): ValidationResult;
}
```

**Code Generation Pipeline**:
1. Visual tree → Abstract syntax tree (AST)
2. AST → Framework-specific code
3. Code formatting and optimization
4. Import management and deduplication

### 4. Agent Orchestration System

```rust
pub struct AgentSystem {
    agents: HashMap<String, Agent>,
    orchestrator: Orchestrator,
    memory: MemoryStore,
    tools: ToolRegistry,
    context_manager: ContextManager,  // Critical component
    task_manager: TaskManagementSystem,  // Deep integration
    integration_bus: SystemIntegrationBus,  // Central coordination
}

pub struct Agent {
    id: String,
    capabilities: Vec<Capability>,
    model: String,
    system_prompt: String,
    tools: Vec<Tool>,
    context_window: usize,  // Agent-specific context limits
}

pub struct Orchestrator {
    context_orchestrator: ContextOrchestrator,
    
    // Plan multi-agent tasks
    async fn plan_task(&self, objective: String) -> TaskPlan;
    
    // Execute with intelligent context management
    async fn execute_plan(&self, plan: TaskPlan) -> Result<TaskResult> {
        match plan.execution_mode {
            ExecutionMode::Sequential => {
                self.execute_sequential_with_context(plan).await
            },
            ExecutionMode::Parallel => {
                self.execute_parallel_with_context(plan).await
            },
            ExecutionMode::Hybrid => {
                self.execute_hybrid_with_context(plan).await
            }
        }
    }
    
    // Human-in-the-loop
    async fn request_approval(&self, action: Action) -> ApprovalResult;
}
```

**Context Management Features**:
- Hierarchical context model (global, task, local)
- Context compression and expansion
- Intelligent routing between agents
- Parallel execution context slicing
- Sequential context accumulation
- Conflict resolution for parallel results

**Note**: See `agent-context-management.md` for detailed implementation.

**Agent Communication Protocol**:
```json
{
  "type": "agent_message",
  "from": "planner_agent",
  "to": "executor_agent",
  "content": {
    "task": "create_react_component",
    "parameters": {
      "name": "UserProfile",
      "props": ["user", "onEdit"]
    }
  },
  "context": {
    "conversation_id": "123",
    "parent_task": "456"
  }
}
```

**Task Management Integration**:
- Visual task graph in IDE sidebar
- Real-time conflict detection
- Parallel execution lanes
- Smart lock management
- Progress visualization

**Note**: See `task-management-system.md` for complete task orchestration details.

### 5. Browser Automation Module

```rust
use playwright::Browser;

pub struct BrowserAutomation {
    browser: Browser,
    contexts: HashMap<String, BrowserContext>,
}

impl BrowserAutomation {
    pub async fn create_page(&self, url: &str) -> Result<Page> {
        // Sandboxed browser context
        let context = self.browser.new_context(
            ContextOptions::default()
                .permissions(vec![])  // No permissions by default
                .user_agent("AI-Master-Tool/1.0")
        ).await?;
        
        context.new_page().await
    }
    
    pub async fn execute_action(&self, action: WebAction) -> Result<ActionResult> {
        // Permission check
        self.check_permission(&action)?;
        
        // Execute with timeout
        timeout(Duration::from_secs(30), 
            self.perform_action(action)
        ).await?
    }
}
```

### 6. Security Layer

```rust
pub struct SecurityManager {
    key_store: KeyStore,
    permissions: PermissionManager,
    sandbox: SandboxManager,
}

pub struct KeyStore {
    // OS keychain integration
    backend: Box<dyn KeychainBackend>,
    
    // Encryption
    cipher: ChaCha20Poly1305,
}

impl KeyStore {
    pub async fn store_api_key(
        &self, 
        provider: &str, 
        key: &str
    ) -> Result<()> {
        let encrypted = self.cipher.encrypt(key.as_bytes())?;
        self.backend.store(&format!("ai_tool_{}", provider), encrypted).await
    }
}

pub struct PermissionManager {
    policies: HashMap<String, Policy>,
    
    pub fn check_permission(&self, action: &Action) -> Result<()> {
        let policy = self.get_policy(&action.resource)?;
        policy.evaluate(action)
    }
}
```

### 7. Plugin System (MCP Implementation)

```typescript
interface MCPServer {
  // Server metadata
  info(): ServerInfo;
  
  // Available tools
  tools(): Tool[];
  
  // Execute tool
  execute(tool: string, params: any): Promise<any>;
  
  // Integration with native systems
  enhanceContext(context: Context): Promise<EnhancedContext>;
  validateOutput(output: any): Promise<ValidationResult>;
}

class MCPManager {
  private servers: Map<string, MCPServer>;
  
  // Start MCP server
  async startServer(config: MCPConfig): Promise<void> {
    const process = spawn(config.command, config.args);
    const transport = new StdioTransport(process);
    const server = new MCPServer(transport);
    
    await server.initialize();
    this.servers.set(config.name, server);
  }
  
  // List all available tools
  getAllTools(): Tool[] {
    return Array.from(this.servers.values())
      .flatMap(server => server.tools());
  }
}
```

## Data Flow Architecture

### 1. AI Request Flow

```
User Input → Frontend → IPC → Provider Manager → AI Provider
                                    ↓
                              Rate Limiter
                                    ↓
                              Cache Check
                                    ↓
                            API Request/Response
                                    ↓
                              Cost Tracking
                                    ↓
                            Response Processing → IPC → Frontend
```

### 2. Code Intelligence Flow

```
Code Change → Parser → AST Generation → Context Analysis
                              ↓
                      Dependency Graph Update
                              ↓
                    Relevant Context Extraction
                              ↓
                      AI Provider Request
                              ↓
                    Suggestion Generation → Editor Update
```

### 3. Visual Builder Sync

```
Visual Change                    Code Change
      ↓                               ↓
Component Tree Update            Parse to AST
      ↓                               ↓
   Validate                    Extract Components
      ↓                               ↓
Generate Code ←─── Conflict Resolution ──→ Update Visual Tree
      ↓                               ↓
Update Editor                  Update Canvas
```

## Performance Optimization

### 1. Caching Strategy

```rust
pub struct CacheManager {
    memory_cache: LruCache<String, CacheEntry>,
    disk_cache: DiskCache,
    
    pub async fn get_or_compute<F, T>(
        &self,
        key: &str,
        compute: F
    ) -> Result<T> 
    where 
        F: Future<Output = Result<T>>,
        T: Serialize + DeserializeOwned
    {
        // L1: Memory cache
        if let Some(entry) = self.memory_cache.get(key) {
            return Ok(entry.value);
        }
        
        // L2: Disk cache
        if let Some(entry) = self.disk_cache.get(key).await? {
            self.memory_cache.put(key, entry.clone());
            return Ok(entry.value);
        }
        
        // Compute and cache
        let value = compute.await?;
        self.cache_value(key, &value).await?;
        Ok(value)
    }
}
```

### 2. Resource Management

```rust
pub struct ResourcePool {
    // GPU memory tracking
    gpu_memory: GpuMemoryTracker,
    
    // CPU thread pool
    cpu_pool: ThreadPool,
    
    // Model loading
    model_cache: ModelCache,
}

impl ResourcePool {
    pub async fn allocate_for_inference(
        &self,
        model_size: usize
    ) -> Result<InferenceResources> {
        if self.gpu_memory.available() >= model_size {
            Ok(InferenceResources::Gpu(self.gpu_memory.allocate(model_size)?))
        } else {
            Ok(InferenceResources::Cpu(self.cpu_pool.reserve(4)?))
        }
    }
}
```

### 3. Streaming Architecture

```typescript
class StreamingManager {
  // Handle AI streaming responses
  async *handleStream(
    response: ReadableStream
  ): AsyncGenerator<StreamChunk> {
    const reader = response.getReader();
    const decoder = new TextDecoder();
    
    let buffer = '';
    
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      
      buffer += decoder.decode(value, { stream: true });
      
      // Parse SSE chunks
      const chunks = this.parseSSEChunks(buffer);
      for (const chunk of chunks) {
        yield chunk;
      }
      
      // Keep incomplete chunk in buffer
      buffer = this.getIncompleteChunk(buffer);
    }
  }
}
```

## Database Schema

### Core Tables

```sql
-- User preferences and settings
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- API usage tracking
CREATE TABLE api_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    tokens_input INTEGER NOT NULL,
    tokens_output INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Project metadata
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    type TEXT NOT NULL,
    last_opened TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Agent conversation history
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id),
    messages TEXT NOT NULL, -- JSON array
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Plugin installations
CREATE TABLE plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    config TEXT, -- JSON configuration
    enabled BOOLEAN DEFAULT TRUE,
    installed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Security Architecture

### 1. API Key Encryption

```rust
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

pub struct KeyEncryption {
    cipher: ChaCha20Poly1305,
    
    pub fn encrypt_api_key(&self, key: &str) -> Result<EncryptedKey> {
        let nonce = generate_nonce();
        let ciphertext = self.cipher.encrypt(&nonce, key.as_bytes())?;
        
        Ok(EncryptedKey {
            nonce: nonce.to_vec(),
            ciphertext,
            version: 1,
        })
    }
    
    pub fn decrypt_api_key(&self, encrypted: &EncryptedKey) -> Result<String> {
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let plaintext = self.cipher.decrypt(nonce, &encrypted.ciphertext)?;
        
        Ok(String::from_utf8(plaintext)?)
    }
}
```

### 2. Sandboxed Execution

```rust
pub struct Sandbox {
    runtime: WasmRuntime,
    permissions: Permissions,
    
    pub async fn execute_untrusted_code(
        &self,
        code: &str,
        timeout: Duration
    ) -> Result<ExecutionResult> {
        let module = self.compile_to_wasm(code)?;
        
        let instance = self.runtime.instantiate(
            module,
            self.create_restricted_imports()
        )?;
        
        // Execute with timeout and memory limits
        let result = timeout(
            timeout,
            instance.run_with_limits(
                memory_limit: 50 * 1024 * 1024, // 50MB
                cpu_limit: 1_000_000_000, // 1B instructions
            )
        ).await??;
        
        Ok(result)
    }
}
```

### 3. Permission Model

```typescript
enum Permission {
  FileSystemRead = 'fs:read',
  FileSystemWrite = 'fs:write',
  NetworkAccess = 'network:access',
  SystemControl = 'system:control',
  BrowserAutomation = 'browser:automate',
}

class PermissionManager {
  private granted: Set<Permission> = new Set();
  
  async requestPermission(
    permission: Permission,
    context: string
  ): Promise<boolean> {
    // Show user prompt
    const dialog = await this.showPermissionDialog(permission, context);
    
    if (dialog.granted) {
      this.granted.add(permission);
      
      // Persist decision if "remember" checked
      if (dialog.remember) {
        await this.persistPermission(permission);
      }
      
      return true;
    }
    
    return false;
  }
}
```

## Integration Points

### 1. MCP Server Integration

```json
{
  "mcp_servers": {
    "filesystem": {
      "command": "node",
      "args": ["./mcp-servers/filesystem/index.js"],
      "env": {
        "ALLOWED_PATHS": "/home/user/projects"
      }
    },
    "git": {
      "command": "python",
      "args": ["-m", "mcp_server_git"],
      "env": {
        "REPO_PATH": "."
      }
    }
  }
}
```

### 2. Local Model Integration

```rust
pub struct OllamaClient {
    base_url: Url,
    client: HttpClient,
    
    pub async fn list_models(&self) -> Result<Vec<Model>> {
        let response = self.client
            .get(self.base_url.join("/api/tags")?)
            .send()
            .await?;
            
        Ok(response.json().await?)
    }
    
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str
    ) -> Result<GenerateResponse> {
        let response = self.client
            .post(self.base_url.join("/api/generate")?)
            .json(&json!({
                "model": model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await?;
            
        Ok(response.json().await?)
    }
}
```

## Testing Strategy

### 1. Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ai_provider_retry() {
        let provider = MockProvider::new()
            .with_failure_count(2)
            .with_response("Success");
            
        let manager = AIProviderManager::new(provider);
        
        let result = manager.complete("test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().text, "Success");
    }
}
```

### 2. Integration Testing

```typescript
describe('Visual Builder Integration', () => {
  it('should sync visual changes to code', async () => {
    const builder = new VisualBuilder();
    const editor = new CodeEditor();
    
    // Add component visually
    await builder.addComponent({
      type: 'Button',
      props: { label: 'Click me' }
    });
    
    // Check code update
    const code = editor.getContent();
    expect(code).toContain('<Button label="Click me" />');
  });
});
```

### 3. End-to-End Testing

```typescript
test('Complete AI-assisted development flow', async () => {
  const app = await launch();
  
  // Create new project
  await app.createProject('test-app');
  
  // Use AI to generate component
  await app.aiChat.send('Create a user profile component');
  
  // Verify component created
  await expect(app.fileExplorer).toHaveFile('UserProfile.tsx');
  
  // Test visual builder
  await app.switchToVisualBuilder();
  await expect(app.canvas).toHaveComponent('UserProfile');
});
```

## Deployment Architecture

### 1. Build Pipeline

```yaml
# .github/workflows/build.yml
name: Build and Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'
          
      - name: Install dependencies
        run: |
          npm ci
          cargo build --release
          
      - name: Build Tauri app
        run: npm run tauri build
        
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.os }}-build
          path: src-tauri/target/release/bundle/
```

### 2. Auto-Update System

```rust
pub struct UpdateManager {
    current_version: Version,
    update_url: Url,
    
    pub async fn check_for_updates(&self) -> Result<Option<Update>> {
        let manifest = self.fetch_update_manifest().await?;
        
        if manifest.version > self.current_version {
            Ok(Some(Update {
                version: manifest.version,
                download_url: manifest.download_url,
                signature: manifest.signature,
                release_notes: manifest.release_notes,
            }))
        } else {
            Ok(None)
        }
    }
    
    pub async fn download_and_verify(
        &self,
        update: &Update
    ) -> Result<PathBuf> {
        let bytes = self.download_update(&update.download_url).await?;
        
        // Verify signature
        self.verify_signature(&bytes, &update.signature)?;
        
        // Save to temp location
        let path = self.save_update(bytes).await?;
        
        Ok(path)
    }
}
```

## Monitoring and Analytics

### 1. Performance Metrics

```rust
pub struct MetricsCollector {
    pub fn record_ai_latency(&self, provider: &str, duration: Duration) {
        self.metrics.record_histogram(
            "ai.request.duration",
            duration.as_millis() as f64,
            &[("provider", provider)]
        );
    }
    
    pub fn record_memory_usage(&self) {
        let usage = self.get_memory_usage();
        self.metrics.record_gauge(
            "app.memory.usage",
            usage as f64,
            &[]
        );
    }
}
```

### 2. Error Tracking

```typescript
class ErrorReporter {
  async reportError(error: Error, context: ErrorContext) {
    // Sanitize sensitive data
    const sanitized = this.sanitizeError(error);
    
    // Local storage for offline
    await this.storeLocally({
      error: sanitized,
      context,
      timestamp: Date.now(),
    });
    
    // Send to telemetry if enabled
    if (this.telemetryEnabled) {
      await this.sendToTelemetry(sanitized, context);
    }
  }
}
```

## System Integration Notes

### Critical Integration Points

1. **Parser → Anti-Duplication Pipeline**: Real-time pattern detection feeds directly into AI constraints
2. **Context → Task Synchronization**: Bidirectional updates maintain consistency
3. **Memory System Federation**: All components share learning through unified memory
4. **DIFF → Visual Builder**: Specialized strategies for visual-code synchronization
5. **Native Integrations → Context**: Platform data enriches AI understanding

For detailed integration architecture, see `system-integration.md`.

## Appendices

### A. API Specifications
Detailed API documentation for all services

### B. Error Codes
Complete list of error codes and meanings

### C. Performance Benchmarks
Target performance metrics and testing methodology

### D. Integration Specifications
See `system-integration.md` for complete integration architecture

---

*This technical specification will be updated as implementation progresses.*