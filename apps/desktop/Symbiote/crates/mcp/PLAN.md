# MCP - Model Context Protocol Implementation Plan

## Goals & Vision

The `mcp` crate provides a comprehensive implementation of the Model Context Protocol (MCP) for Symbiote. It offers:

- **MCP Server/Client**: Full MCP protocol implementation for AI model communication
- **Tool Integration**: Seamless integration with Symbiote's tool ecosystem
- **Resource Management**: Efficient handling of MCP resources and contexts
- **Hot Reload**: Dynamic MCP configuration updates without restart
- **Global & Project Settings**: Flexible MCP configuration at multiple levels
- **JSON Import**: Convert any tool's MCP settings into Symbiote format
- **AI Agent Setup**: MCP agent that can configure MCPs for users via AI
- **Performance Optimization**: Efficient protocol handling and caching

This system enables Symbiote to communicate with AI models using the standardized MCP protocol while providing enhanced management and configuration capabilities.

## Database Schema

### MCP Configuration and State Persistence

```sql
-- MCP server configurations
CREATE TABLE mcp_servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id VARCHAR(255) NOT NULL UNIQUE,
    server_name VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    scope VARCHAR(50) NOT NULL, -- 'global', 'project'
    project_id VARCHAR(255), -- NULL for global scope
    server_type VARCHAR(100), -- 'stdio', 'websocket', 'http'
    command VARCHAR(1000), -- Command to start server (for stdio)
    args JSONB, -- Command arguments array
    env JSONB, -- Environment variables
    cwd VARCHAR(500), -- Working directory
    url VARCHAR(500), -- URL for websocket/http servers
    transport_config JSONB, -- Transport-specific configuration
    capabilities JSONB, -- Server capabilities (tools, resources, prompts)
    is_enabled BOOLEAN DEFAULT true,
    auto_start BOOLEAN DEFAULT true,
    restart_on_failure BOOLEAN DEFAULT true,
    max_restart_attempts INTEGER DEFAULT 3,
    health_check_interval_seconds INTEGER DEFAULT 30,
    timeout_seconds INTEGER DEFAULT 30,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_started TIMESTAMP,
    last_stopped TIMESTAMP,
    metadata JSONB
);

-- MCP server runtime state
CREATE TABLE mcp_server_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    status VARCHAR(50) NOT NULL, -- 'stopped', 'starting', 'running', 'stopping', 'error'
    process_id INTEGER, -- PID for stdio servers
    connection_id VARCHAR(255), -- Connection identifier
    last_heartbeat TIMESTAMP,
    error_message TEXT,
    error_count INTEGER DEFAULT 0,
    restart_count INTEGER DEFAULT 0,
    uptime_seconds INTEGER DEFAULT 0,
    memory_usage_mb INTEGER,
    cpu_usage_percent DECIMAL(5,2),
    message_count BIGINT DEFAULT 0,
    error_rate DECIMAL(5,2) DEFAULT 0,
    response_time_ms INTEGER,
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- MCP tools registry
CREATE TABLE mcp_tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) NOT NULL,
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    tool_name VARCHAR(255) NOT NULL,
    description TEXT,
    input_schema JSONB, -- JSON Schema for tool inputs
    output_schema JSONB, -- JSON Schema for tool outputs
    is_available BOOLEAN DEFAULT true,
    risk_level VARCHAR(50), -- 'low', 'medium', 'high', 'critical'
    requires_approval BOOLEAN DEFAULT false,
    execution_count BIGINT DEFAULT 0,
    success_count BIGINT DEFAULT 0,
    failure_count BIGINT DEFAULT 0,
    avg_execution_time_ms INTEGER,
    last_executed TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    UNIQUE(server_id, tool_name)
);

-- MCP resources registry
CREATE TABLE mcp_resources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id VARCHAR(255) NOT NULL,
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    uri VARCHAR(1000) NOT NULL,
    name VARCHAR(255),
    description TEXT,
    mime_type VARCHAR(100),
    supports_subscription BOOLEAN DEFAULT false,
    is_available BOOLEAN DEFAULT true,
    access_count BIGINT DEFAULT 0,
    last_accessed TIMESTAMP,
    size_bytes BIGINT,
    checksum VARCHAR(64),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    UNIQUE(server_id, uri)
);

-- MCP prompts registry
CREATE TABLE mcp_prompts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prompt_id VARCHAR(255) NOT NULL,
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    prompt_name VARCHAR(255) NOT NULL,
    description TEXT,
    arguments_schema JSONB, -- JSON Schema for prompt arguments
    is_available BOOLEAN DEFAULT true,
    usage_count BIGINT DEFAULT 0,
    last_used TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    UNIQUE(server_id, prompt_name)
);

-- MCP execution logs
CREATE TABLE mcp_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id VARCHAR(255) NOT NULL UNIQUE,
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    tool_id VARCHAR(255) REFERENCES mcp_tools(tool_id),
    user_id VARCHAR(255) NOT NULL,
    execution_type VARCHAR(50), -- 'tool_call', 'resource_read', 'prompt_get'
    request_data JSONB, -- Input parameters
    response_data JSONB, -- Output data
    status VARCHAR(50), -- 'pending', 'running', 'completed', 'failed', 'cancelled'
    started_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP,
    duration_ms INTEGER,
    error_message TEXT,
    approval_required BOOLEAN DEFAULT false,
    approval_status VARCHAR(50), -- 'pending', 'approved', 'denied'
    approved_by VARCHAR(255),
    approved_at TIMESTAMP,
    risk_assessment JSONB,
    security_scan_result JSONB,
    metadata JSONB
);

-- MCP configuration history
CREATE TABLE mcp_config_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    history_id VARCHAR(255) NOT NULL UNIQUE,
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    change_type VARCHAR(100), -- 'created', 'updated', 'deleted', 'enabled', 'disabled'
    old_config JSONB,
    new_config JSONB,
    changed_by VARCHAR(255),
    change_reason TEXT,
    change_timestamp TIMESTAMP DEFAULT NOW(),
    rollback_available BOOLEAN DEFAULT true,
    metadata JSONB
);

-- MCP security events
CREATE TABLE mcp_security_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id VARCHAR(255) NOT NULL UNIQUE,
    server_id VARCHAR(255) REFERENCES mcp_servers(server_id),
    event_type VARCHAR(100), -- 'unauthorized_access', 'suspicious_activity', 'policy_violation'
    severity VARCHAR(50), -- 'low', 'medium', 'high', 'critical'
    description TEXT,
    source_ip INET,
    user_id VARCHAR(255),
    tool_name VARCHAR(255),
    request_data JSONB,
    threat_indicators JSONB,
    action_taken VARCHAR(100), -- 'blocked', 'logged', 'quarantined', 'escalated'
    investigated BOOLEAN DEFAULT false,
    investigation_notes TEXT,
    resolved_at TIMESTAMP,
    event_timestamp TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Indexes for performance
CREATE INDEX idx_mcp_servers_user ON mcp_servers(user_id);
CREATE INDEX idx_mcp_servers_scope ON mcp_servers(scope);
CREATE INDEX idx_mcp_servers_project ON mcp_servers(project_id);
CREATE INDEX idx_mcp_servers_enabled ON mcp_servers(is_enabled);
CREATE INDEX idx_mcp_server_state_server ON mcp_server_state(server_id);
CREATE INDEX idx_mcp_server_state_status ON mcp_server_state(status);
CREATE INDEX idx_mcp_tools_server ON mcp_tools(server_id);
CREATE INDEX idx_mcp_tools_available ON mcp_tools(is_available);
CREATE INDEX idx_mcp_tools_risk_level ON mcp_tools(risk_level);
CREATE INDEX idx_mcp_resources_server ON mcp_resources(server_id);
CREATE INDEX idx_mcp_resources_uri ON mcp_resources(uri);
CREATE INDEX idx_mcp_prompts_server ON mcp_prompts(server_id);
CREATE INDEX idx_mcp_executions_server ON mcp_executions(server_id);
CREATE INDEX idx_mcp_executions_user ON mcp_executions(user_id);
CREATE INDEX idx_mcp_executions_status ON mcp_executions(status);
CREATE INDEX idx_mcp_executions_started_at ON mcp_executions(started_at);
CREATE INDEX idx_mcp_config_history_server ON mcp_config_history(server_id);
CREATE INDEX idx_mcp_config_history_timestamp ON mcp_config_history(change_timestamp);
CREATE INDEX idx_mcp_security_events_server ON mcp_security_events(server_id);
CREATE INDEX idx_mcp_security_events_severity ON mcp_security_events(severity);
CREATE INDEX idx_mcp_security_events_timestamp ON mcp_security_events(event_timestamp);
```

## Architecture & Design

### Core Modules

```
mcp/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── protocol/             # MCP protocol implementation
│   │   ├── mod.rs
│   │   ├── client.rs         # MCP client implementation
│   │   ├── server.rs         # MCP server implementation
│   │   ├── transport.rs      # Transport layer (stdio, websocket, etc.)
│   │   ├── messages.rs       # MCP message types
│   │   └── validation.rs     # Protocol validation
│   ├── tools/                # Tool integration
│   │   ├── mod.rs
│   │   ├── registry.rs       # Tool registry for MCP
│   │   ├── adapter.rs        # Symbiote tool to MCP adapter
│   │   ├── discovery.rs      # Tool discovery
│   │   └── execution.rs      # Tool execution via MCP
│   ├── resources/            # Resource management
│   │   ├── mod.rs
│   │   ├── manager.rs        # Resource manager
│   │   ├── providers.rs      # Resource providers
│   │   ├── cache.rs          # Resource caching
│   │   └── streaming.rs      # Streaming resources
│   ├── config/               # Configuration management
│   │   ├── mod.rs
│   │   ├── manager.rs        # Configuration manager
│   │   ├── global.rs         # Global MCP settings
│   │   ├── project.rs        # Project-specific settings
│   │   ├── hot_reload.rs     # Hot reload functionality
│   │   └── import.rs         # JSON import/export
│   ├── agents/               # MCP AI agent
│   │   ├── mod.rs
│   │   ├── setup_agent.rs    # MCP setup agent
│   │   ├── configuration.rs  # Configuration assistance
│   │   ├── troubleshooting.rs # Troubleshooting assistance
│   │   └── optimization.rs   # Performance optimization
│   ├── monitoring/           # Monitoring and observability
│   │   ├── mod.rs
│   │   ├── metrics.rs        # Performance metrics
│   │   ├── logging.rs        # Protocol logging
│   │   ├── health.rs         # Health monitoring
│   │   └── debugging.rs      # Debug utilities
│   └── types/                # MCP types
│       ├── mod.rs
│       ├── protocol.rs       # Protocol type definitions
│       ├── config.rs         # Configuration types
│       └── errors.rs         # Error types
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── basic_client.rs
    └── tool_server.rs
```

### Key Design Principles

1. **Protocol Compliance**: Full adherence to MCP specification
2. **Performance Optimized**: Efficient message handling and resource management
3. **Flexible Configuration**: Support for global and project-specific settings
4. **AI-Enhanced**: AI agent for automated MCP setup and configuration
5. **Developer Friendly**: Easy integration with existing tools and workflows

## APIs & Interfaces

### MCP Client

```rust
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    message_handler: MessageHandler,
    tool_registry: ToolRegistry,
    resource_manager: ResourceManager,
    config: McpClientConfig,
    metrics: ClientMetrics,
}

impl McpClient {
    pub async fn new(config: McpClientConfig) -> McpResult<Self>;
    
    pub async fn connect(&mut self) -> McpResult<()>;
    
    pub async fn disconnect(&mut self) -> McpResult<()>;
    
    pub async fn initialize(&mut self) -> McpResult<InitializeResult>;
    
    pub async fn list_tools(&self) -> McpResult<Vec<ToolInfo>>;
    
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> McpResult<ToolResult>;
    
    pub async fn list_resources(&self) -> McpResult<Vec<ResourceInfo>>;
    
    pub async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent>;
    
    pub async fn subscribe_to_resource(&self, uri: &str) -> McpResult<ResourceSubscription>;
    
    pub async fn list_prompts(&self) -> McpResult<Vec<PromptInfo>>;
    
    pub async fn get_prompt(&self, name: &str, arguments: Option<serde_json::Value>) -> McpResult<PromptResult>;
    
    pub async fn send_notification(&self, notification: Notification) -> McpResult<()>;
    
    pub fn get_metrics(&self) -> &ClientMetrics;
}

#[derive(Debug, Clone)]
pub struct McpClientConfig {
    pub server_config: ServerConfig,
    pub transport_config: TransportConfig,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub cache_config: CacheConfig,
    pub monitoring_config: MonitoringConfig,
}

#[derive(Debug, Clone)]
pub enum ServerConfig {
    Command {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    WebSocket {
        url: String,
        headers: HashMap<String, String>,
    },
    Stdio {
        executable: String,
        args: Vec<String>,
    },
}
```

### MCP Server

```rust
pub struct McpServer {
    transport: Box<dyn McpTransport>,
    message_handler: MessageHandler,
    tool_provider: ToolProvider,
    resource_provider: ResourceProvider,
    prompt_provider: PromptProvider,
    config: McpServerConfig,
    metrics: ServerMetrics,

    // Security & Trust Model (from master plan)
    security_manager: SecurityManager,
    sandbox_manager: SandboxManager,
    egress_proxy: EgressProxy,
    vault_integration: VaultIntegration,
    approval_system: ApprovalSystem,

    // Global Hooks Integration (connects to symbiote-core)
    hook_emitter: Box<dyn HookEmitter>,
}

impl McpServer {
    pub async fn new(config: McpServerConfig) -> McpResult<Self>;
    
    pub async fn start(&mut self) -> McpResult<()>;
    
    pub async fn stop(&mut self) -> McpResult<()>;
    
    pub fn register_tool_provider<P: ToolProvider + 'static>(&mut self, provider: P);
    
    pub fn register_resource_provider<P: ResourceProvider + 'static>(&mut self, provider: P);
    
    pub fn register_prompt_provider<P: PromptProvider + 'static>(&mut self, provider: P);
    
    pub async fn handle_client_connection(&self, transport: Box<dyn McpTransport>) -> McpResult<()>;
    
    pub fn get_metrics(&self) -> &ServerMetrics;
}

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn list_tools(&self) -> McpResult<Vec<ToolInfo>>;
    
    async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> McpResult<ToolResult>;
    
    fn get_tool_schema(&self, name: &str) -> Option<ToolSchema>;
}

#[async_trait]
pub trait ResourceProvider: Send + Sync {
    async fn list_resources(&self) -> McpResult<Vec<ResourceInfo>>;
    
    async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent>;
    
    async fn subscribe_to_resource(&self, uri: &str) -> McpResult<ResourceSubscription>;
    
    fn supports_subscription(&self, uri: &str) -> bool;
}

#[async_trait]
pub trait PromptProvider: Send + Sync {
    async fn list_prompts(&self) -> McpResult<Vec<PromptInfo>>;
    
    async fn get_prompt(&self, name: &str, arguments: Option<serde_json::Value>) -> McpResult<PromptResult>;
    
    fn get_prompt_schema(&self, name: &str) -> Option<PromptSchema>;
}
```

### Configuration Management

```rust
pub struct McpConfigManager {
    global_config: GlobalMcpConfig,
    project_configs: HashMap<ProjectId, ProjectMcpConfig>,
    hot_reload_manager: HotReloadManager,
    import_export: ImportExportManager,
    validation: ConfigValidation,
}

impl McpConfigManager {
    pub async fn new() -> McpResult<Self>;
    
    pub async fn load_global_config(&mut self) -> McpResult<()>;
    
    pub async fn save_global_config(&self) -> McpResult<()>;
    
    pub async fn load_project_config(&mut self, project_id: ProjectId) -> McpResult<()>;
    
    pub async fn save_project_config(&self, project_id: ProjectId) -> McpResult<()>;
    
    pub async fn add_mcp_server(&mut self, scope: ConfigScope, server_config: McpServerConfig) -> McpResult<()>;
    
    pub async fn remove_mcp_server(&mut self, scope: ConfigScope, server_id: &str) -> McpResult<()>;
    
    pub async fn enable_mcp_server(&mut self, scope: ConfigScope, server_id: &str) -> McpResult<()>;
    
    pub async fn disable_mcp_server(&mut self, scope: ConfigScope, server_id: &str) -> McpResult<()>;
    
    pub async fn import_config(&mut self, config_json: &str, scope: ConfigScope) -> McpResult<ImportResult>;
    
    pub async fn export_config(&self, scope: ConfigScope) -> McpResult<String>;
    
    pub async fn validate_config(&self, config: &McpConfig) -> McpResult<ValidationResult>;
    
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChange>;

    pub async fn test_server_connection(&self, server_id: &str) -> McpResult<ConnectionTestResult>;

    pub async fn debug_server(&self, server_id: &str) -> McpResult<DebugInfo>;

    pub async fn get_server_logs(&self, server_id: &str, lines: Option<usize>) -> McpResult<Vec<LogEntry>>;

    pub async fn restart_server(&self, server_id: &str) -> McpResult<()>;

    pub async fn get_server_metrics(&self, server_id: &str) -> McpResult<ServerMetrics>;

    pub async fn benchmark_server(&self, server_id: &str, test_cases: Vec<BenchmarkCase>) -> McpResult<BenchmarkResults>;

    pub async fn validate_server_compliance(&self, server_id: &str) -> McpResult<ComplianceReport>;

    pub async fn get_protocol_version_compatibility(&self, server_id: &str) -> McpResult<VersionCompatibility>;

    pub async fn export_server_config(&self, server_id: &str) -> McpResult<String>;

    pub async fn clone_server_config(&self, source_id: &str, target_id: &str) -> McpResult<()>;

    pub async fn backup_all_configs(&self) -> McpResult<BackupInfo>;

    pub async fn restore_configs(&self, backup_path: &str) -> McpResult<RestoreResult>;

    pub async fn migrate_config_version(&self, from_version: &str, to_version: &str) -> McpResult<MigrationResult>;

    pub async fn get_config_diff(&self, config1: &McpConfig, config2: &McpConfig) -> McpResult<ConfigDiff>;

    pub async fn merge_configs(&self, base: &McpConfig, changes: &McpConfig) -> McpResult<McpConfig>;
}

#[derive(Debug, Clone)]
pub struct GlobalMcpConfig {
    pub servers: HashMap<String, McpServerConfig>,
    pub default_settings: McpSettings,
    pub security_settings: SecuritySettings,
    pub monitoring_settings: MonitoringSettings,
}

#[derive(Debug, Clone)]
pub struct ProjectMcpConfig {
    pub project_id: ProjectId,
    pub servers: HashMap<String, McpServerConfig>,
    pub overrides: McpSettings,
    pub enabled_global_servers: Vec<String>,
    pub disabled_global_servers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigScope {
    Global,
    Project(ProjectId),
}

#[derive(Debug, Clone)]
pub struct McpSettings {
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub cache_enabled: bool,
    pub cache_ttl: Duration,
    pub max_concurrent_requests: usize,
    pub enable_metrics: bool,
    pub log_level: LogLevel,
}
```

### MCP Setup Agent

```rust
pub struct McpSetupAgent {
    ai_client: Arc<AiClient>,
    config_manager: Arc<McpConfigManager>,
    tool_analyzer: ToolAnalyzer,
    troubleshooter: McpTroubleshooter,
    optimizer: McpOptimizer,
}

impl McpSetupAgent {
    pub async fn new(config: McpAgentConfig) -> McpResult<Self>;
    
    pub async fn setup_mcp_from_description(&self, description: &str, scope: ConfigScope) -> McpResult<SetupResult>;
    
    pub async fn analyze_existing_tools(&self, project_path: &str) -> McpResult<ToolAnalysis>;
    
    pub async fn suggest_mcp_configuration(&self, requirements: McpRequirements) -> McpResult<ConfigSuggestion>;
    
    pub async fn troubleshoot_mcp_issue(&self, issue_description: &str, server_id: &str) -> McpResult<TroubleshootingResult>;
    
    pub async fn optimize_mcp_performance(&self, server_id: &str) -> McpResult<OptimizationResult>;
    
    pub async fn convert_tool_config(&self, tool_config: &str, tool_type: ToolType) -> McpResult<McpServerConfig>;
    
    pub async fn validate_mcp_setup(&self, server_config: &McpServerConfig) -> McpResult<ValidationResult>;
    
    pub async fn generate_mcp_documentation(&self, server_config: &McpServerConfig) -> McpResult<Documentation>;
}

#[derive(Debug, Clone)]
pub struct SetupResult {
    pub server_config: McpServerConfig,
    pub installation_steps: Vec<InstallationStep>,
    pub configuration_applied: bool,
    pub test_results: TestResults,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone)]
pub struct ToolAnalysis {
    pub discovered_tools: Vec<DiscoveredTool>,
    pub mcp_candidates: Vec<McpCandidate>,
    pub integration_suggestions: Vec<IntegrationSuggestion>,
    pub potential_conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone)]
pub struct McpCandidate {
    pub tool_name: String,
    pub tool_type: ToolType,
    pub mcp_compatibility: CompatibilityLevel,
    pub suggested_config: McpServerConfig,
    pub benefits: Vec<String>,
    pub requirements: Vec<Requirement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilityLevel {
    Native,      // Already supports MCP
    Adaptable,   // Can be adapted to MCP
    Wrapper,     // Needs wrapper implementation
    Incompatible, // Cannot be integrated
}
```

### Transport Layer

```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_message(&mut self, message: McpMessage) -> McpResult<()>;
    
    async fn receive_message(&mut self) -> McpResult<McpMessage>;
    
    async fn close(&mut self) -> McpResult<()>;
    
    fn is_connected(&self) -> bool;
    
    fn transport_type(&self) -> TransportType;
}

pub struct StdioTransport {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl McpTransport for StdioTransport {
    async fn send_message(&mut self, message: McpMessage) -> McpResult<()> {
        let json = serde_json::to_string(&message)?;
        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }
    
    async fn receive_message(&mut self) -> McpResult<McpMessage> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        let message: McpMessage = serde_json::from_str(&line)?;
        Ok(message)
    }
}

pub struct WebSocketTransport {
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    url: String,
}

impl McpTransport for WebSocketTransport {
    async fn send_message(&mut self, message: McpMessage) -> McpResult<()> {
        let json = serde_json::to_string(&message)?;
        self.websocket.send(Message::Text(json)).await?;
        Ok(())
    }
    
    async fn receive_message(&mut self) -> McpResult<McpMessage> {
        match self.websocket.next().await {
            Some(Ok(Message::Text(text))) => {
                let message: McpMessage = serde_json::from_str(&text)?;
                Ok(message)
            }
            Some(Ok(_)) => Err(McpError::InvalidMessage.into()),
            Some(Err(e)) => Err(e.into()),
            None => Err(McpError::ConnectionClosed.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    Stdio,
    WebSocket,
    Http,
    Custom(String),
}
```

### Protocol Messages

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum McpMessage {
    // Client to Server
    Initialize {
        id: MessageId,
        params: InitializeParams,
    },
    ListTools {
        id: MessageId,
    },
    CallTool {
        id: MessageId,
        params: CallToolParams,
    },
    ListResources {
        id: MessageId,
    },
    ReadResource {
        id: MessageId,
        params: ReadResourceParams,
    },
    SubscribeToResource {
        id: MessageId,
        params: SubscribeParams,
    },
    ListPrompts {
        id: MessageId,
    },
    GetPrompt {
        id: MessageId,
        params: GetPromptParams,
    },
    
    // Server to Client
    Notification {
        params: NotificationParams,
    },
    
    // Responses
    Response {
        id: MessageId,
        result: Option<serde_json::Value>,
        error: Option<McpError>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub tools: Option<ToolCapabilities>,
    pub resources: Option<ResourceCapabilities>,
    pub prompts: Option<PromptCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<Content>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    Text {
        text: String,
    },
    Image {
        data: String,
        mime_type: String,
    },
    Resource {
        resource: ResourceReference,
    },
}
```

## Implementation Details

### Technology Stack

- **Protocol**: JSON-RPC 2.0 over various transports
- **Transport**: stdio, WebSocket, HTTP support
- **Serialization**: serde for JSON handling
- **Process Management**: tokio::process for subprocess handling
- **WebSocket**: tokio-tungstenite for WebSocket transport
- **Configuration**: serde with hot reload support
- **AI Integration**: Integration with symbiote-ai for setup assistance

### Key Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
tracing = "0.1"
tokio-tungstenite = "0.20"
url = "2.0"
notify = "6.0"
dashmap = "5.0"
parking_lot = "0.12"
jsonschema = "0.17"
symbiote-core = { path = "../symbiote-core" }
symbiote-ai = { path = "../ai" }
symbiote-tools = { path = "../tools" }
symbiote-agents = { path = "../agents" }

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"
```

### Hot Reload Implementation

```rust
impl HotReloadManager {
    pub async fn watch_config_files(&mut self) -> McpResult<()> {
        let (tx, mut rx) = mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.try_send(event);
            }
        })?;
        
        watcher.watch(&self.config_path, RecursiveMode::Recursive)?;
        
        while let Some(event) = rx.recv().await {
            if let EventKind::Modify(_) = event.kind {
                for path in event.paths {
                    if path.extension() == Some(OsStr::new("json")) {
                        self.reload_config(&path).await?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn reload_config(&self, config_path: &Path) -> McpResult<()> {
        let config_content = tokio::fs::read_to_string(config_path).await?;
        let new_config: McpServerConfig = serde_json::from_str(&config_content)?;
        
        // Validate new configuration
        self.validate_config(&new_config).await?;
        
        // Apply configuration changes
        self.apply_config_changes(new_config).await?;
        
        // Notify subscribers
        self.notify_config_change(config_path).await?;
        
        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests

- **Protocol Implementation**: Test MCP message handling and validation
- **Transport Layer**: Test all transport implementations
- **Configuration**: Test config management and hot reload
- **Tool Integration**: Test tool provider implementations
- **AI Agent**: Test MCP setup and troubleshooting

### Integration Tests

- **End-to-End MCP**: Test complete MCP client-server communication
- **Real MCP Servers**: Test with actual MCP server implementations
- **Configuration Scenarios**: Test various configuration combinations
- **Hot Reload**: Test configuration changes without restart
- **Performance**: Test protocol performance and resource usage

### Compliance Tests

- **MCP Specification**: Test compliance with official MCP specification
- **Interoperability**: Test with other MCP implementations
- **Error Handling**: Test error scenarios and recovery
- **Security**: Test security aspects of MCP communication

## Integration Points

### Upstream Dependencies

- **symbiote-core**: Uses error types, configuration, and async utilities
- **ai**: Uses AI providers for setup assistance
- **tools**: Integrates with Symbiote tool system
- **agents**: Uses agent framework for MCP setup agent

### Downstream Consumers

- **Assistant**: Uses MCP for AI model communication
- **IDE**: Uses MCP for development tool integration
- **Workflow Engine**: Uses MCP for workflow tool integration
- **All AI Features**: Uses MCP for standardized AI communication

### External Integrations

- **MCP Servers**: Integration with external MCP server implementations
- **AI Models**: Communication with AI models via MCP
- **Development Tools**: Integration with MCP-compatible development tools
- **Third-Party Services**: Integration with services that support MCP

## Acceptance Criteria

### Functional Requirements

- [ ] Full MCP protocol implementation (client and server)
- [ ] Support for stdio, WebSocket, and HTTP transports
- [ ] Global and project-specific configuration management
- [ ] Hot reload of MCP configurations without restart
- [ ] JSON import/export for MCP configurations
- [ ] AI agent for automated MCP setup and troubleshooting
- [ ] Comprehensive tool and resource provider system

### Non-Functional Requirements

- [ ] Sub-10ms message processing latency
- [ ] Support for 100+ concurrent MCP connections
- [ ] 99.9% protocol compliance with MCP specification
- [ ] Memory usage under 100MB for typical workloads
- [ ] Hot reload within 1 second of configuration changes
- [ ] Cross-platform compatibility

### Quality Gates

- [ ] All tests pass with 95%+ coverage
- [ ] MCP specification compliance verified
- [ ] Performance benchmarks meet targets
- [ ] Security audit passes for protocol implementation
- [ ] Documentation complete with examples
- [ ] Interoperability tested with other MCP implementations

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with async support
- **Development Tools**: Standard Rust development environment

### Runtime Dependencies

- **MCP Servers**: External MCP server implementations for testing
- **AI Models**: Access to AI models for setup assistance
- **Network**: Connectivity for WebSocket and HTTP transports
- **File System**: Access for configuration file monitoring

### Development Prerequisites

- **MCP Knowledge**: Understanding of Model Context Protocol specification
- **Protocol Development**: Experience with protocol implementation
- **AI Integration**: Knowledge of AI model communication patterns
- **Configuration Management**: Understanding of dynamic configuration systems

This MCP implementation provides Symbiote with standardized, efficient communication with AI models and tools while offering advanced management capabilities that go beyond basic MCP compliance, including AI-powered setup assistance and dynamic configuration management.

## Enhanced Security & Trust Model (Master Plan Features)

### Comprehensive MCP Security

```rust
/// Enhanced MCP manager with comprehensive security
pub struct MCPManager {
    servers: HashMap<String, MCPServer>,
    clients: HashMap<String, MCPClient>,

    // Security & Trust Model
    security_manager: SecurityManager,
    sandbox_manager: SandboxManager,
    egress_proxy: EgressProxy,
    vault_integration: VaultIntegration,
    approval_system: ApprovalSystem,

    // Configuration and lifecycle
    config_manager: McpConfigManager,
    hot_reload_manager: HotReloadManager,

    // UI components
    tool_catalog: ToolCatalog,
    tool_runner: ToolRunner,
    health_dashboard: HealthDashboard,

    // Global Hooks Integration
    hook_emitter: Box<dyn HookEmitter>,
}

impl MCPManager {
    pub async fn new() -> McpResult<Self>;

    /// Execute tool with comprehensive security checks
    pub async fn execute_tool(&self, tool_name: &str, params: serde_json::Value) -> McpResult<ToolResult>;

    /// Check tool execution permission
    pub async fn check_tool_permission(&self, tool_name: &str, params: &serde_json::Value) -> McpResult<()>;

    /// Execute tool in sandbox with timeout
    pub async fn execute_tool_sandboxed(&self, tool_name: &str, params: serde_json::Value) -> McpResult<ToolResult>;
}

/// Security manager for MCP operations
pub struct SecurityManager {
    trust_policies: Vec<TrustPolicy>,
    auth_manager: AuthManager,
    audit_logger: AuditLogger,
}

impl SecurityManager {
    /// Trust boundaries: MCP servers run out-of-process, least-privilege, untrusted by default
    pub async fn enforce_trust_boundaries(&self, server_id: &str) -> McpResult<()>;

    /// AuthN/AuthZ: signed server manifests, token-scoped tool access, optional mTLS
    pub async fn authenticate_server(&self, server_manifest: &ServerManifest) -> McpResult<AuthResult>;

    pub async fn authorize_tool_access(&self, tool_name: &str, token: &str) -> McpResult<AuthzResult>;

    pub async fn setup_mtls(&self, server_id: &str) -> McpResult<TLSConfig>;

    /// Audit all tool executions
    pub async fn audit_tool_execution(&self, tool_name: &str, params: &serde_json::Value, result: &ToolResult) -> McpResult<()>;
}

/// Sandbox manager for process/container isolation
pub struct SandboxManager {
    process_isolator: ProcessIsolator,
    container_manager: ContainerManager,
    resource_limiter: ResourceLimiter,
}

impl SandboxManager {
    /// Process/container isolation with filesystem/network allowlists
    pub async fn create_sandbox(&self, server_id: &str) -> McpResult<Sandbox>;

    /// CPU/memory/time quotas enforcement
    pub async fn enforce_resource_limits(&self, sandbox: &Sandbox, limits: &ResourceLimits) -> McpResult<()>;

    /// Block and audit attempts to access disallowed resources
    pub async fn block_disallowed_access(&self, sandbox: &Sandbox, resource: &str) -> McpResult<()>;
}

/// Egress proxy for all MCP tool network access
pub struct EgressProxy {
    domain_allowlist: Vec<String>,
    tls_pinning: TLSPinning,
    request_interceptor: RequestInterceptor,
}

impl EgressProxy {
    /// All MCP tool egress via Egress Proxy with domain allowlist and TLS pinning
    pub async fn intercept_mcp_request(&self, request: &HttpRequest) -> McpResult<InterceptResult>;

    pub async fn validate_domain_allowlist(&self, domain: &str) -> McpResult<()>;

    pub async fn enforce_tls_pinning(&self, connection: &TLSConnection) -> McpResult<()>;
}

/// Vault integration for secrets handling
pub struct VaultIntegration {
    vault_client: VaultClient,
    secret_detector: SecretDetector,
    secret_handler: SecretHandler,
}

impl VaultIntegration {
    /// No raw secrets ever pass to MCP; SecretHandles only
    pub async fn convert_to_secret_handles(&self, params: &mut serde_json::Value) -> McpResult<()>;

    /// Signing/headers performed by Vault Daemon
    pub async fn sign_request(&self, request: &HttpRequest) -> McpResult<SignedRequest>;

    /// Detect and prevent secret leakage
    pub async fn scan_for_secrets(&self, data: &str) -> McpResult<Vec<SecretDetection>>;
}

/// Approval system for high-risk tools
pub struct ApprovalSystem {
    approval_manager: ApprovalManager,
    mfa_provider: MFAProvider,
    risk_assessor: RiskAssessor,
}

impl ApprovalSystem {
    /// High-risk tools require explicit user approval with MFA support
    pub async fn request_approval(&self, tool_name: &str, params: &serde_json::Value) -> McpResult<ApprovalResult>;

    pub async fn assess_tool_risk(&self, tool_name: &str, params: &serde_json::Value) -> McpResult<RiskLevel>;

    pub async fn require_mfa(&self, approval_request: &ApprovalRequest) -> McpResult<MFAResult>;
}

/// MCP hooks integration
impl HookEmitter for MCPManager {
    fn emit_event(&self, event: SystemEvent) -> McpResult<()> {
        self.hook_emitter.emit_event(event)
    }

    fn register_event_types(&self) -> Vec<SystemEventType> {
        vec![
            SystemEventType::Custom { event_type: "mcp.before_tool_call".to_string(), data: serde_json::Value::Null, metadata: EventMetadata::default() },
            SystemEventType::Custom { event_type: "mcp.after_tool_call".to_string(), data: serde_json::Value::Null, metadata: EventMetadata::default() },
            SystemEventType::Custom { event_type: "mcp.on_policy_violation".to_string(), data: serde_json::Value::Null, metadata: EventMetadata::default() },
            SystemEventType::Custom { event_type: "mcp.on_auth_required".to_string(), data: serde_json::Value::Null, metadata: EventMetadata::default() },
        ]
    }

    fn get_subsystem_id(&self) -> String {
        "mcp_manager".to_string()
    }
}

/// Risk levels for approval system
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
```

## Error Handling

### MCP Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("MCP protocol error: {operation} - {reason}")]
    ProtocolError { operation: String, reason: String },

    #[error("Server connection failed: {server_id} - {error}")]
    ServerConnectionFailed { server_id: String, error: String },

    #[error("Tool execution failed: {tool_name} - {reason}")]
    ToolExecutionFailed { tool_name: String, reason: String },

    #[error("Resource access failed: {uri} - {error}")]
    ResourceAccessFailed { uri: String, error: String },

    #[error("Prompt generation failed: {prompt_name} - {reason}")]
    PromptGenerationFailed { prompt_name: String, reason: String },

    #[error("Configuration error: {setting} - {issue}")]
    ConfigurationError { setting: String, issue: String },

    #[error("Transport error: {transport_type} - {error}")]
    TransportError { transport_type: String, error: String },

    #[error("Authentication failed: {server_id} - {reason}")]
    AuthenticationFailed { server_id: String, reason: String },

    #[error("Authorization denied: {operation} - {resource}")]
    AuthorizationDenied { operation: String, resource: String },

    #[error("Sandbox violation: {server_id} - {violation}")]
    SandboxViolation { server_id: String, violation: String },

    #[error("Security policy violation: {policy} - {violation}")]
    SecurityPolicyViolation { policy: String, violation: String },

    #[error("Rate limit exceeded: {server_id} - {limit}")]
    RateLimitExceeded { server_id: String, limit: String },

    #[error("Timeout error: {operation} timed out after {duration_ms}ms")]
    TimeoutError { operation: String, duration_ms: u64 },

    #[error("Invalid message format: {expected} expected, got {actual}")]
    InvalidMessageFormat { expected: String, actual: String },

    #[error("Server startup failed: {server_id} - {reason}")]
    ServerStartupFailed { server_id: String, reason: String },

    #[error("Hot reload failed: {config_path} - {error}")]
    HotReloadFailed { config_path: String, error: String },

    #[error("Import/export failed: {operation} - {reason}")]
    ImportExportFailed { operation: String, reason: String },

    #[error("Validation failed: {item} - {issue}")]
    ValidationFailed { item: String, issue: String },

    #[error("Permission denied: {operation} - {resource}")]
    PermissionDenied { operation: String, resource: String },

    #[error("Database error: {operation} - {error}")]
    DatabaseError { operation: String, error: String },

    #[error("Serialization error: {data_type} - {error}")]
    SerializationError { data_type: String, error: String },

    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },

    #[error("Internal error: {details}")]
    InternalError { details: String },
}

pub type McpResult<T> = Result<T, McpError>;

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        McpError::InternalError {
            details: format!("IO error: {}", err),
        }
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for McpError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        McpError::TransportError {
            transport_type: "websocket".to_string(),
            error: err.to_string(),
        }
    }
}
```

## UI Specifications

### MCP Management Dashboard

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔌 MCP Management Center                            [🔄] [⚙️] [📊] [🔒] [🤖] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📊 MCP Overview                                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Active Servers: 12        │ Available Tools: 47   │ Resources: 23       │ │
│ │ Running: 10 ✅           │ High Risk: 3 ⚠️       │ Subscriptions: 8    │ │
│ │ Stopped: 2 ⏸️            │ Executions: 156 today │ Prompts: 15         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚀 Quick Actions                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 🔧 Add Server         │ 📥 Import Config     │ 🤖 AI Setup          │ │
│ │ Configure new MCP     │ Import from JSON     │ AI-powered MCP        │ │
│ │ server connection     │ or existing tools    │ configuration         │ │
│ │ [🔧 Add Server]       │ [📥 Import]          │ [🤖 Setup AI]         │ │
│ │                                                                         │ │
│ │ 🛡️ Security Scan      │ 📈 Performance       │ 🔄 Hot Reload         │ │
│ │ Scan all servers      │ Monitor server       │ Reload configurations │ │
│ │ for vulnerabilities   │ performance metrics  │ without restart       │ │
│ │ [🛡️ Scan]             │ [📈 Monitor]         │ [🔄 Reload]           │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🖥️ MCP Servers                                                              │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Server               │ Status │ Tools │ Type     │ Risk  │ Actions       │ │
│ │ claude-tools         │ ✅ Run │ 12    │ stdio    │ 🟢 Low│ [⚙️][📊][⏸️] │ │
│ │ github-integration   │ ✅ Run │ 8     │ websock  │ 🟡 Med│ [⚙️][📊][⏸️] │ │
│ │ file-operations      │ ⚠️ Err │ 15    │ stdio    │ 🔴 Hi │ [🔧][📋][▶️] │ │
│ │ web-scraper          │ ⏸️ Stop│ 6     │ http     │ 🟡 Med│ [⚙️][📊][▶️] │ │
│ │ [➕ Add Server] [📥 Import] [🔄 Reload All] [⚙️ Global Settings]        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚨 Recent Activity                                                          │
│ │ • Tool "file_write" executed successfully - 2.3s                        │ │
│ │ • Server "claude-tools" restarted due to timeout                        │ │
│ │ • Security alert: High-risk tool "system_command" requires approval     │ │
│ │ • Configuration imported from "vscode-mcp-settings.json"                │ │
│ │ [📋 View All Activity] [🔔 Notification Settings] [📊 Activity Report]   │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### MCP Tool Execution Interface

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔧 Tool Execution - file_operations::write_file           [💾] [✅] [📤] [❌] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📊 Execution Details                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Tool: file_operations::write_file │ Risk Level: 🔴 High                 │ │
│ │ Server: file-operations           │ Approval: ✅ Required               │ │
│ │ Execution ID: exec_789abc123      │ Timeout: 30s                        │ │
│ │ Started: 2025-08-13 14:23:45      │ Status: ⏳ Pending Approval        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📝 Input Parameters                                                         │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ {                                                                       │ │
│ │   "file_path": "/home/user/important_config.json",                     │ │
│ │   "content": "{\n  \"api_key\": \"sk-...\",\n  \"endpoint\": \"...\"\n}",│ │
│ │   "create_backup": true,                                                │ │
│ │   "permissions": "0644"                                                 │ │
│ │ }                                                                       │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🛡️ Security Analysis                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ ⚠️ Security Concerns:                                                   │ │
│ │ • File path outside sandbox: /home/user/important_config.json          │ │
│ │ • Potential secret detected in content: "api_key"                      │ │
│ │ • High-privilege operation: file system write                          │ │
│ │                                                                         │ │
│ │ 🔒 Mitigations Applied:                                                 │ │
│ │ • Sandbox isolation enabled                                             │ │
│ │ • Secret redaction in logs                                              │ │
│ │ • Backup creation enforced                                              │ │
│ │ [🛡️ Security Details] [📋 Audit Log] [🔒 Vault Integration]            │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🎯 Approval Required                                                        │
│ │ This tool execution requires explicit approval due to high risk level.  │ │
│ │ [✅ Approve] [❌ Deny] [⏸️ Defer] [🔧 Modify Parameters]                 │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Security Model

### MCP Security Framework

```rust
pub struct McpSecurityManager {
    access_control: McpAccessControl,
    sandbox_manager: SandboxManager,
    audit_logger: McpAuditLogger,
    threat_detector: ThreatDetector,
}

impl McpSecurityManager {
    /// Validate MCP server access and permissions
    pub async fn validate_mcp_access(&self, user_id: &str, operation: McpOperation) -> McpResult<AccessDecision>;

    /// Secure MCP server execution in sandboxed environments
    pub async fn secure_server_execution(&self, server_id: &str, execution_context: &ExecutionContext) -> McpResult<SecureExecution>;

    /// Scan MCP tool parameters for security threats
    pub async fn scan_tool_parameters(&self, tool_name: &str, parameters: &serde_json::Value) -> McpResult<SecurityScanResult>;

    /// Enforce security policies on MCP operations
    pub async fn enforce_security_policies(&self, operation: &McpOperation, policies: &[SecurityPolicy]) -> McpResult<PolicyEnforcement>;

    /// Log MCP operations for audit and compliance
    pub async fn log_mcp_operation(&self, operation: &McpOperation, user_id: &str, result: &OperationResult) -> McpResult<()>;

    /// Handle sensitive data detection in MCP communications
    pub async fn detect_sensitive_data(&self, mcp_message: &McpMessage) -> McpResult<SensitiveDataResult>;

    /// Manage MCP server trust boundaries and isolation
    pub async fn enforce_trust_boundaries(&self, server_id: &str, trust_level: TrustLevel) -> McpResult<TrustBoundary>;
}

#[derive(Debug, Clone)]
pub enum McpOperation {
    StartServer { server_id: String, config: String },
    ExecuteTool { server_id: String, tool_name: String, parameters: String },
    AccessResource { server_id: String, uri: String, access_type: String },
    ModifyConfiguration { server_id: String, changes: String },
    ExportData { server_id: String, data_types: Vec<String> },
    ImportConfiguration { config_data: String, scope: String },
}

#[derive(Debug, Clone)]
pub enum TrustLevel {
    Untrusted,
    Limited,
    Trusted,
    FullyTrusted,
}
```

## Integration Points

### Upstream Dependencies

```rust
/// Integration with other Symbiote crates
pub struct SymbioteMcpIntegration {
    ai_client: AiClient,                    // For AI-powered MCP setup and configuration
    storage_manager: StorageManager,        // For MCP configuration and state persistence
    security_manager: SecurityManager,      // For MCP security and access control
    vault_client: VaultClient,             // For secure secret handling in MCP operations
    egress_proxy: EgressProxy,             // For secure network access from MCP servers
    assistant: PersonalAssistant,          // For natural language MCP management
}

impl SymbioteMcpIntegration {
    /// Initialize MCP with Symbiote ecosystem
    pub async fn initialize_mcp(&self, config: McpConfig) -> McpResult<McpManager>;

    /// Use AI for intelligent MCP setup and configuration
    pub async fn ai_setup_mcp(&self, description: &str, requirements: &McpRequirements) -> McpResult<McpSetupResult>;

    /// Store MCP data using storage crate
    pub async fn persist_mcp_data(&self, data: &McpData) -> McpResult<()>;

    /// Validate MCP permissions using security crate
    pub async fn validate_mcp_permissions(&self, user_id: &str, operation: &McpOperation) -> McpResult<bool>;

    /// Handle secrets in MCP operations using vault
    pub async fn secure_mcp_secrets(&self, parameters: &mut serde_json::Value) -> McpResult<()>;

    /// Route MCP network requests through egress proxy
    pub async fn proxy_mcp_request(&self, server_id: &str, request: &HttpRequest) -> McpResult<HttpResponse>;
}
```

### Downstream Consumers

```rust
/// Services that consume MCP capabilities
pub trait McpConsumer {
    /// Handle MCP events and notifications
    async fn on_mcp_event(&self, event: McpEvent) -> McpResult<()>;

    /// Process tool execution events
    async fn on_tool_executed(&self, execution_event: ToolExecutionEvent) -> McpResult<()>;

    /// Handle server state changes
    async fn on_server_state_changed(&self, state_event: ServerStateEvent) -> McpResult<()>;

    /// Process MCP errors and failures
    async fn on_mcp_error(&self, error: McpErrorEvent) -> McpResult<()>;
}

/// MCP event types
#[derive(Debug, Clone)]
pub enum McpEvent {
    ServerStarted { server_id: String, capabilities: Vec<String> },
    ServerStopped { server_id: String, reason: String },
    ToolExecuted { server_id: String, tool_name: String, duration_ms: u64, success: bool },
    ResourceAccessed { server_id: String, uri: String, access_type: String },
    ConfigurationChanged { server_id: String, change_type: String },
    SecurityViolation { server_id: String, violation_type: String, severity: String },
    ApprovalRequired { execution_id: String, tool_name: String, risk_level: String },
}
```

## Implementation Details

### Technology Stack

- **Protocol Implementation**: Full MCP specification compliance with JSON-RPC 2.0 transport
- **Transport Layer**: Support for stdio, WebSocket, and HTTP transports with connection pooling
- **Security Framework**: Comprehensive sandboxing, authentication, and authorization
- **Configuration Management**: Hot-reload capable configuration with validation and migration
- **AI Integration**: Uses ai crate for intelligent setup, troubleshooting, and optimization
- **Database**: PostgreSQL for configuration and state persistence with performance indexing
- **Monitoring**: Real-time metrics, health checks, and performance monitoring
- **Extensibility**: Plugin architecture for custom tool providers and transport types

### Key Features

1. **Full MCP Compliance**: Complete implementation of Model Context Protocol specification
2. **Multi-Transport Support**: stdio, WebSocket, and HTTP transport layers
3. **Advanced Security**: Sandboxing, trust boundaries, and comprehensive audit logging
4. **Hot Configuration Reload**: Dynamic configuration updates without service restart
5. **AI-Powered Setup**: Intelligent MCP server configuration and troubleshooting
6. **Global & Project Scopes**: Flexible configuration management at multiple levels
7. **Import/Export**: JSON-based configuration import from existing tools
8. **Performance Monitoring**: Real-time metrics and health monitoring

This comprehensive MCP implementation provides Symbiote with powerful Model Context Protocol capabilities, enabling seamless integration with AI models while maintaining security, performance, and ease of use.
