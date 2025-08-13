# Tools - Typed Tool SDK Plan

## Goals & Vision

The `tools` crate provides a comprehensive SDK for creating, managing, and executing tools within Symbiote. It offers:

- **Type-Safe Tool Definitions**: Compile-time validated tool interfaces
- **Automatic Schema Generation**: JSON Schema generation from Rust types
- **Sandboxed Execution**: Secure tool execution with resource limits
- **Tool Discovery**: Dynamic tool registration and discovery
- **Validation Framework**: Input/output validation and sanitization
- **Error Handling**: Robust error handling and recovery
- **Performance Monitoring**: Tool execution metrics and optimization

This SDK enables developers to create powerful, safe, and efficient tools that integrate seamlessly with Symbiote's AI agents.

## UI Design Specifications

### Tool Management Dashboard

#### Main Tool Registry
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔧 Tool Management Center                       [🔍] [➕] [📊] [⚙️] [🔄]    │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📊 Tool Overview                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Total Tools: 247         │ Active: 234        │ Disabled: 13           │ │
│ │ ┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐ │ │
│ │ │ 🤖 AI Tools │ 🌐 Network  │ 💾 Storage  │ 🔧 System   │ 🎨 Custom   │ │ │
│ │ │    89       │     67      │     45      │     32      │     14      │ │ │
│ │ │ (36%)       │ (27%)       │ (18%)       │ (13%)       │ (6%)        │ │ │
│ │ └─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘ │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔍 Tool Search & Filters                                                    │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Search: [Find tools for file processing...                        ] [🔍] │ │
│ │                                                                         │ │
│ │ Filters: [🏷️ All Categories ▼] [⭐ 4+ Rating ▼] [🔒 All Access ▼]      │ │
│ │ Status: [✅ Active] [⏸️ Disabled] [🔄 Updating] [❌ Error]              │ │
│ │ Source: [📦 Built-in] [🌐 Community] [🏢 Enterprise] [🎨 Custom]        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔧 Available Tools (247 found)                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 📁 File Manager                                    ⭐ 4.8 (1,247 uses) │ │
│ │ ├─ Category: Storage                │ Version: 2.1.0 │ Status: ✅ Active │ │
│ │ ├─ Description: Comprehensive file operations with AI assistance        │ │
│ │ ├─ Capabilities: Read, Write, Move, Copy, Search, Analyze              │ │
│ │ ├─ Security: Sandboxed, Rate limited, Audit logged                     │ │
│ │ └─ [⚙️ Configure] [📊 Usage Stats] [🧪 Test] [📋 Docs]                 │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 🌐 Web Scraper                                     ⭐ 4.6 (892 uses)   │ │
│ │ ├─ Category: Network               │ Version: 1.8.3 │ Status: ✅ Active │ │
│ │ ├─ Description: Intelligent web scraping with respect for robots.txt   │ │
│ │ ├─ Capabilities: HTML parsing, Data extraction, Rate limiting          │ │
│ │ ├─ Security: Proxy support, User-agent rotation, CAPTCHA handling      │ │
│ │ └─ [⚙️ Configure] [📊 Usage Stats] [🧪 Test] [📋 Docs]                 │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 🤖 Code Analyzer                                   ⭐ 4.9 (2,156 uses) │ │
│ │ ├─ Category: AI Tools              │ Version: 3.0.1 │ Status: ✅ Active │ │
│ │ ├─ Description: Advanced code analysis with security and quality checks │ │
│ │ ├─ Capabilities: Static analysis, Security scan, Performance review    │ │
│ │ ├─ Security: Isolated execution, Memory limits, Timeout protection     │ │
│ │ └─ [⚙️ Configure] [📊 Usage Stats] [🧪 Test] [📋 Docs]                 │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 📊 Data Transformer                                ⭐ 4.4 (567 uses)   │ │
│ │ ├─ Category: Storage               │ Version: 1.5.2 │ Status: ⚠️ Limited │ │
│ │ ├─ Description: Transform data between formats with validation         │ │
│ │ ├─ Capabilities: JSON, CSV, XML, YAML conversion and validation        │ │
│ │ ├─ Security: Schema validation, Size limits, Type checking             │ │
│ │ └─ [⚙️ Configure] [📊 Usage Stats] [🧪 Test] [📋 Docs]                 │ │
│ │                                                                         │ │
│ │ [Load More Tools...] [📦 Install New] [🔄 Update All] [📊 Analytics]   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Tool Configuration Interface
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⚙️ Configure Tool: File Manager                                      [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📋 Tool Information                                                         │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Name: File Manager                    │ Version: 2.1.0                   │ │
│ │ Author: Symbiote Team                 │ License: MIT                      │ │
│ │ Category: Storage                     │ Rating: ⭐ 4.8 (1,247 reviews)   │ │
│ │ Description: Comprehensive file operations with AI assistance            │ │
│ │                                                                         │ │
│ │ Capabilities:                                                           │ │
│ │ • 📖 Read files with encoding detection                                 │ │
│ │ • ✏️ Write files with backup creation                                   │ │
│ │ • 📁 Directory operations (create, list, delete)                       │ │
│ │ • 🔍 Search files with pattern matching                                │ │
│ │ • 📊 File analysis and metadata extraction                             │ │
│ │ • 🔒 Permission management and access control                          │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔒 Security Configuration                                                   │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Access Control:                                                         │ │
│ │ ├─ Allowed Paths: [/workspace, /tmp, /uploads] [+ Add Path]            │ │
│ │ ├─ Blocked Paths: [/etc, /sys, /proc] [+ Add Path]                     │ │
│ │ ├─ Max File Size: [100 MB ▼]                                          │ │
│ │ ├─ Max Files per Operation: [1000 ▼]                                   │ │
│ │ └─ Allowed Extensions: [.txt, .json, .csv, .md] [+ Add Extension]     │ │
│ │                                                                         │ │
│ │ Execution Limits:                                                       │ │
│ │ ├─ Memory Limit: [512 MB ▼]                                           │ │
│ │ ├─ CPU Time Limit: [30 seconds ▼]                                     │ │
│ │ ├─ Network Access: [❌ Disabled]                                       │ │
│ │ ├─ Subprocess Execution: [❌ Disabled]                                 │ │
│ │ └─ Temporary Files: [✅ Allowed in /tmp only]                         │ │
│ │                                                                         │ │
│ │ Audit & Logging:                                                        │ │
│ │ ├─ ☑️ Log all file operations                                          │ │
│ │ ├─ ☑️ Record access patterns                                           │ │
│ │ ├─ ☑️ Monitor for suspicious activity                                  │ │
│ │ ├─ ☑️ Generate security reports                                        │ │
│ │ └─ Log Level: [Detailed ▼]                                            │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ ⚡ Performance Settings                                                     │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Caching:                                                                │ │
│ │ ├─ File Content Cache: [256 MB ▼]                                     │ │
│ │ ├─ Metadata Cache: [64 MB ▼]                                          │ │
│ │ ├─ Cache TTL: [1 hour ▼]                                              │ │
│ │ └─ Cache Strategy: [LRU ▼]                                             │ │
│ │                                                                         │ │
│ │ Optimization:                                                           │ │
│ │ ├─ Parallel Operations: [4 threads ▼]                                 │ │
│ │ ├─ Batch Size: [100 files ▼]                                          │ │
│ │ ├─ Compression: [Auto ▼] for large files                              │ │
│ │ ├─ Streaming: [✅ Enabled] for large files                            │ │
│ │ └─ Prefetching: [✅ Enabled] for sequential access                    │ │
│ │                                                                         │ │
│ │ Rate Limiting:                                                          │ │
│ │ ├─ Operations per minute: [1000 ▼]                                     │ │
│ │ ├─ Bytes per minute: [100 MB ▼]                                       │ │
│ │ ├─ Burst allowance: [50 operations ▼]                                 │ │
│ │ └─ Cooldown period: [5 seconds ▼]                                     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🧪 Testing & Validation                                                     │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Test Configuration:                                                     │ │
│ │ ┌─────────────────────────────────────────────────────────────────────┐ │ │
│ │ │ Operation: [Read File ▼]                                           │ │ │
│ │ │ Test File: [/workspace/test.txt] [📁 Browse]                       │ │ │
│ │ │ Expected Result: [File content returned successfully]               │ │ │
│ │ │                                                                     │ │ │
│ │ │ [🧪 Run Test] [📋 View Results] [💾 Save Test Case]               │ │ │
│ │ └─────────────────────────────────────────────────────────────────────┘ │ │
│ │                                                                         │ │
│ │ Recent Test Results:                                                    │ │
│ │ • ✅ Read operation: 23ms (Expected: <50ms)                           │ │
│ │ • ✅ Write operation: 45ms (Expected: <100ms)                         │ │
│ │ • ✅ Directory listing: 12ms (Expected: <30ms)                        │ │
│ │ • ⚠️ Large file read: 2.3s (Expected: <2s, slightly slow)            │ │
│ │ • ✅ Permission check: 3ms (Expected: <10ms)                          │ │
│ │                                                                         │ │
│ │ [📊 Performance Benchmark] [🔍 Detailed Analysis] [📤 Export Report]  │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│                                    [💾 Save Configuration] [🔄 Reset]     │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Tool Development Studio
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🛠️ Tool Development Studio                                           [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🎯 Create New Tool                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Tool Metadata:                                                          │ │
│ │ ├─ Name: [Custom Data Processor                    ]                    │ │
│ │ ├─ Description: [Process and transform data files  ]                    │ │
│ │ ├─ Category: [Storage ▼]                                               │ │
│ │ ├─ Version: [1.0.0    ]                                                │ │
│ │ ├─ Author: [Your Name                              ]                    │ │
│ │ └─ License: [MIT ▼]                                                    │ │
│ │                                                                         │ │
│ │ Tool Template: [📊 Data Processing ▼]                                  │ │
│ │ • 🤖 AI-Powered Tool (with model integration)                          │ │
│ │ • 📊 Data Processing Tool (file transformation)                        │ │
│ │ • 🌐 Network Tool (API calls, web scraping)                           │ │
│ │ • 🔧 System Tool (OS operations, process management)                   │ │
│ │ • 🎨 Custom Tool (blank template)                                      │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📝 Tool Definition (Rust Code)                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ use symbiote_tools::prelude::*;                                        │ │
│ │                                                                         │ │
│ │ #[derive(Tool, Debug, Clone)]                                          │ │
│ │ #[tool(                                                                │ │
│ │     name = "custom_data_processor",                                    │ │
│ │     description = "Process and transform data files",                  │ │
│ │     category = "storage"                                               │ │
│ │ )]                                                                     │ │
│ │ pub struct CustomDataProcessor;                                        │ │
│ │                                                                         │ │
│ │ #[derive(Serialize, Deserialize, JsonSchema)]                         │ │
│ │ pub struct ProcessDataInput {                                          │ │
│ │     #[serde(description = "Path to input file")]                      │ │
│ │     input_path: String,                                                │ │
│ │     #[serde(description = "Output format")]                           │ │
│ │     format: DataFormat,                                                │ │
│ │     #[serde(description = "Processing options")]                       │ │
│ │     options: ProcessingOptions,                                        │ │
│ │ }                                                                      │ │
│ │                                                                         │ │
│ │ #[async_trait]                                                         │ │
│ │ impl ToolExecutor for CustomDataProcessor {                            │ │
│ │     type Input = ProcessDataInput;                                     │ │
│ │     type Output = ProcessDataOutput;                                   │ │
│ │                                                                         │ │
│ │     async fn execute(&self, input: Self::Input) -> ToolResult<Self::Output> {│ │
│ │         // Implementation here...                                      │ │
│ │         todo!("Implement data processing logic")                       │ │
│ │     }                                                                  │ │
│ │ }                                                                      │ │
│ │                                                                         │ │
│ │ [💾 Save] [🧪 Test] [📋 Generate Schema] [📚 View Docs] [🔄 Format]   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔧 Tool Configuration Schema                                               │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Generated JSON Schema:                                                  │ │
│ │ ```json                                                                │ │
│ │ {                                                                      │ │
│ │   "$schema": "http://json-schema.org/draft-07/schema#",               │ │
│ │   "title": "ProcessDataInput",                                        │ │
│ │   "type": "object",                                                   │ │
│ │   "properties": {                                                     │ │
│ │     "input_path": {                                                   │ │
│ │       "type": "string",                                               │ │
│ │       "description": "Path to input file"                            │ │
│ │     },                                                                │ │
│ │     "format": {                                                       │ │
│ │       "type": "string",                                               │ │
│ │       "enum": ["json", "csv", "xml", "yaml"],                        │ │
│ │       "description": "Output format"                                  │ │
│ │     }                                                                 │ │
│ │   },                                                                  │ │
│ │   "required": ["input_path", "format"]                               │ │
│ │ }                                                                     │ │
│ │ ```                                                                   │ │
│ │                                                                         │ │
│ │ [📋 Copy Schema] [✅ Validate] [📤 Export] [🔄 Regenerate]            │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🧪 Testing & Deployment                                                    │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Test Cases:                                                             │ │
│ │ ├─ ✅ Valid JSON input → CSV output                                    │ │
│ │ ├─ ✅ Large file processing (10MB)                                     │ │
│ │ ├─ ⚠️ Invalid file path handling                                       │ │
│ │ ├─ ❌ Unsupported format conversion                                     │ │
│ │ └─ [+ Add Test Case]                                                   │ │
│ │                                                                         │ │
│ │ Security Validation:                                                    │ │
│ │ ├─ ✅ Input sanitization                                               │ │
│ │ ├─ ✅ Path traversal protection                                        │ │
│ │ ├─ ✅ Resource limit compliance                                        │ │
│ │ ├─ ✅ Error handling coverage                                          │ │
│ │ └─ ✅ Audit logging implementation                                     │ │
│ │                                                                         │ │
│ │ [🚀 Deploy Tool] [📦 Package] [📋 Generate Docs] [🔍 Security Scan]   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Architecture & Design

### Core Modules

```
tools/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── definition/           # Tool definition framework
│   │   ├── mod.rs
│   │   ├── traits.rs         # Core tool traits
│   │   ├── macros.rs         # Tool definition macros
│   │   ├── schema.rs         # Schema generation
│   │   └── metadata.rs       # Tool metadata
│   ├── execution/            # Tool execution engine
│   │   ├── mod.rs
│   │   ├── executor.rs       # Tool executor
│   │   ├── sandbox.rs        # Sandboxed execution
│   │   ├── timeout.rs        # Timeout handling
│   │   └── resources.rs      # Resource management
│   ├── registry/             # Tool registry
│   │   ├── mod.rs
│   │   ├── registry.rs       # Tool registration
│   │   ├── discovery.rs      # Tool discovery
│   │   ├── versioning.rs     # Tool versioning
│   │   └── dependencies.rs   # Dependency management
│   ├── validation/           # Input/output validation
│   │   ├── mod.rs
│   │   ├── input.rs          # Input validation
│   │   ├── output.rs         # Output validation
│   │   ├── sanitization.rs   # Data sanitization
│   │   └── constraints.rs    # Validation constraints
│   ├── serialization/        # Data serialization
│   │   ├── mod.rs
│   │   ├── json.rs           # JSON serialization
│   │   ├── binary.rs         # Binary serialization
│   │   └── streaming.rs      # Streaming serialization
│   ├── monitoring/           # Tool monitoring
│   │   ├── mod.rs
│   │   ├── metrics.rs        # Execution metrics
│   │   ├── tracing.rs        # Execution tracing
│   │   ├── profiling.rs      # Performance profiling
│   │   └── logging.rs        # Tool logging
│   ├── security/             # Security framework
│   │   ├── mod.rs
│   │   ├── permissions.rs    # Permission checking
│   │   ├── isolation.rs      # Process isolation
│   │   ├── audit.rs          # Security auditing
│   │   └── policies.rs       # Security policies
│   └── types/                # Common types
│       ├── mod.rs
│       ├── tools.rs          # Tool type definitions
│       ├── execution.rs      # Execution types
│       └── errors.rs         # Error types
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── simple_tool.rs
    └── complex_tool.rs
```

### Key Design Principles

1. **Type Safety**: Compile-time validation of tool interfaces
2. **Security First**: Sandboxed execution with permission controls
3. **Performance**: Efficient execution with minimal overhead
4. **Composability**: Tools that can be combined and chained
5. **Observability**: Comprehensive monitoring and debugging

## APIs & Interfaces

### Core Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    type Input: ToolInput;
    type Output: ToolOutput;
    type Error: ToolError;
    
    /// Tool metadata
    fn metadata(&self) -> ToolMetadata;
    
    /// Execute the tool with given input
    async fn execute(&self, input: Self::Input, context: &ToolContext) -> Result<Self::Output, Self::Error>;
    
    /// Validate input before execution
    async fn validate_input(&self, input: &Self::Input) -> Result<(), ValidationError> {
        input.validate()
    }
    
    /// Validate output after execution
    async fn validate_output(&self, output: &Self::Output) -> Result<(), ValidationError> {
        output.validate()
    }
    
    /// Check if tool can execute with given permissions
    fn check_permissions(&self, permissions: &PermissionSet) -> Result<(), PermissionError> {
        let required = self.required_permissions();
        permissions.check_all(&required)
    }
    
    /// Required permissions for tool execution
    fn required_permissions(&self) -> Vec<Permission>;
    
    /// Resource requirements for execution
    fn resource_requirements(&self) -> ResourceRequirements;
    
    /// Maximum execution timeout
    fn timeout(&self) -> Option<Duration>;
    
    /// Whether tool is safe for concurrent execution
    fn is_thread_safe(&self) -> bool { true }
    
    /// Whether tool modifies external state
    fn is_pure(&self) -> bool { false }
}

pub trait ToolInput: Send + Sync + Clone + Serialize + DeserializeOwned + JsonSchema {
    fn validate(&self) -> Result<(), ValidationError>;
    fn sanitize(&mut self) -> Result<(), SanitizationError>;
}

pub trait ToolOutput: Send + Sync + Clone + Serialize + DeserializeOwned + JsonSchema {
    fn validate(&self) -> Result<(), ValidationError>;
}

pub trait ToolError: std::error::Error + Send + Sync + 'static {
    fn error_code(&self) -> &'static str;
    fn is_retryable(&self) -> bool;
    fn retry_delay(&self) -> Option<Duration>;
}
```

### Tool Definition Macros

```rust
/// Derive macro for automatic tool implementation
#[derive(Tool)]
#[tool(
    name = "file_reader",
    description = "Reads content from a file",
    version = "1.0.0",
    permissions = ["file:read"],
    timeout = "30s"
)]
pub struct FileReaderTool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToolInput)]
pub struct FileReaderInput {
    #[validate(path)]
    pub file_path: String,
    
    #[validate(range(min = 1, max = 1000000))]
    pub max_size: Option<usize>,
    
    #[validate(one_of = ["utf8", "binary"])]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToolOutput)]
pub struct FileReaderOutput {
    pub content: String,
    pub size: usize,
    pub encoding: String,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error, ToolError)]
pub enum FileReaderError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    #[error("File too large: {size} bytes")]
    FileTooLarge { size: usize },
    
    #[error("Encoding error: {details}")]
    EncodingError { details: String },
}

impl Tool for FileReaderTool {
    type Input = FileReaderInput;
    type Output = FileReaderOutput;
    type Error = FileReaderError;
    
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "file_reader".to_string(),
            description: "Reads content from a file".to_string(),
            version: "1.0.0".to_string(),
            author: "Symbiote".to_string(),
            tags: vec!["file", "io", "read"].into_iter().map(String::from).collect(),
            input_schema: Self::Input::json_schema(),
            output_schema: Self::Output::json_schema(),
        }
    }
    
    async fn execute(&self, input: Self::Input, context: &ToolContext) -> Result<Self::Output, Self::Error> {
        // Implementation
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead]
    }
    
    fn resource_requirements(&self) -> ResourceRequirements {
        ResourceRequirements {
            max_memory: Some(100 * 1024 * 1024), // 100MB
            max_cpu_time: Some(Duration::from_secs(30)),
            max_file_descriptors: Some(10),
            network_access: false,
        }
    }
    
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}
```

### Tool Registry

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn DynamicTool>>,
    schemas: HashMap<String, ToolSchema>,
    dependencies: DependencyGraph,
    security_policies: SecurityPolicyManager,
    metrics: RegistryMetrics,
}

impl ToolRegistry {
    pub fn new() -> Self;
    
    pub fn register<T: Tool + 'static>(&mut self, tool: T) -> Result<(), RegistrationError>;
    
    pub fn unregister(&mut self, name: &str) -> Result<(), RegistrationError>;
    
    pub async fn execute(&self, name: &str, input: serde_json::Value, context: &ToolContext) -> Result<serde_json::Value, ToolExecutionError>;
    
    pub fn get_tool_info(&self, name: &str) -> Option<&ToolMetadata>;
    
    pub fn list_tools(&self) -> Vec<&ToolMetadata>;
    
    pub fn search_tools(&self, query: &ToolQuery) -> Vec<&ToolMetadata>;
    
    pub fn get_schema(&self, name: &str) -> Option<&ToolSchema>;
    
    pub fn validate_dependencies(&self) -> Result<(), DependencyError>;
    
    pub fn get_metrics(&self) -> &RegistryMetrics;
}

pub trait DynamicTool: Send + Sync {
    fn metadata(&self) -> &ToolMetadata;
    
    async fn execute_dynamic(&self, input: serde_json::Value, context: &ToolContext) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
    
    fn input_schema(&self) -> &JsonSchema;
    
    fn output_schema(&self) -> &JsonSchema;
    
    fn required_permissions(&self) -> &[Permission];
    
    fn resource_requirements(&self) -> &ResourceRequirements;
}

impl<T: Tool> DynamicTool for T {
    fn metadata(&self) -> &ToolMetadata {
        // Implementation using Tool trait
    }
    
    async fn execute_dynamic(&self, input: serde_json::Value, context: &ToolContext) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let typed_input: T::Input = serde_json::from_value(input)?;
        let output = self.execute(typed_input, context).await?;
        Ok(serde_json::to_value(output)?)
    }
}
```

### Tool Execution Engine

```rust
pub struct ToolExecutor {
    sandbox: ToolSandbox,
    resource_manager: ResourceManager,
    timeout_manager: TimeoutManager,
    security_manager: SecurityManager,
    monitor: ExecutionMonitor,
}

impl ToolExecutor {
    pub fn new(config: ExecutorConfig) -> Self;
    
    pub async fn execute<T: Tool>(&self, tool: &T, input: T::Input, context: ToolContext) -> Result<T::Output, ExecutionError>;
    
    pub async fn execute_with_timeout<T: Tool>(&self, tool: &T, input: T::Input, context: ToolContext, timeout: Duration) -> Result<T::Output, ExecutionError>;
    
    pub async fn execute_batch<T: Tool>(&self, tool: &T, inputs: Vec<T::Input>, context: ToolContext) -> Result<Vec<Result<T::Output, T::Error>>, ExecutionError>;
    
    pub fn get_execution_metrics(&self, execution_id: ExecutionId) -> Option<ExecutionMetrics>;
    
    pub async fn cancel_execution(&self, execution_id: ExecutionId) -> Result<(), ExecutionError>;
}

pub struct ToolSandbox {
    isolation_level: IsolationLevel,
    resource_limits: ResourceLimits,
    network_policy: NetworkPolicy,
    filesystem_policy: FilesystemPolicy,
}

impl ToolSandbox {
    pub fn new(config: SandboxConfig) -> Self;
    
    pub async fn execute_isolated<F, R>(&self, f: F) -> Result<R, SandboxError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;
    
    pub fn check_resource_usage(&self) -> ResourceUsage;
    
    pub fn enforce_limits(&self, limits: &ResourceLimits) -> Result<(), SandboxError>;
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub execution_id: ExecutionId,
    pub agent_id: Option<AgentId>,
    pub user_id: Option<UserId>,
    pub permissions: PermissionSet,
    pub resource_limits: ResourceLimits,
    pub timeout: Option<Duration>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub trace_context: TraceContext,
}

#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub max_memory: Option<usize>,
    pub max_cpu_time: Option<Duration>,
    pub max_wall_time: Option<Duration>,
    pub max_file_descriptors: Option<u32>,
    pub network_access: bool,
    pub filesystem_access: FilesystemAccess,
    pub custom_requirements: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum FilesystemAccess {
    None,
    ReadOnly { paths: Vec<String> },
    ReadWrite { paths: Vec<String> },
    Full,
}
```

### Validation Framework

```rust
pub struct ValidationEngine {
    validators: HashMap<String, Box<dyn Validator>>,
    sanitizers: HashMap<String, Box<dyn Sanitizer>>,
    constraint_checker: ConstraintChecker,
}

impl ValidationEngine {
    pub fn new() -> Self;
    
    pub fn add_validator<V: Validator + 'static>(&mut self, name: String, validator: V);
    
    pub fn add_sanitizer<S: Sanitizer + 'static>(&mut self, name: String, sanitizer: S);
    
    pub fn validate_value(&self, value: &serde_json::Value, schema: &JsonSchema) -> Result<(), ValidationError>;
    
    pub fn sanitize_value(&self, value: &mut serde_json::Value, rules: &SanitizationRules) -> Result<(), SanitizationError>;
    
    pub fn check_constraints(&self, value: &serde_json::Value, constraints: &[Constraint]) -> Result<(), ConstraintError>;
}

pub trait Validator: Send + Sync {
    fn validate(&self, value: &serde_json::Value) -> Result<(), ValidationError>;
    
    fn name(&self) -> &str;
}

pub trait Sanitizer: Send + Sync {
    fn sanitize(&self, value: &mut serde_json::Value) -> Result<(), SanitizationError>;
    
    fn name(&self) -> &str;
}

pub struct PathValidator;

impl Validator for PathValidator {
    fn validate(&self, value: &serde_json::Value) -> Result<(), ValidationError> {
        if let Some(path_str) = value.as_str() {
            let path = Path::new(path_str);
            if path.is_absolute() && !path.to_string_lossy().contains("..") {
                Ok(())
            } else {
                Err(ValidationError::InvalidPath { path: path_str.to_string() })
            }
        } else {
            Err(ValidationError::InvalidType { expected: "string".to_string() })
        }
    }
    
    fn name(&self) -> &str { "path" }
}

pub struct HtmlSanitizer {
    allowed_tags: HashSet<String>,
    allowed_attributes: HashMap<String, HashSet<String>>,
}

impl Sanitizer for HtmlSanitizer {
    fn sanitize(&self, value: &mut serde_json::Value) -> Result<(), SanitizationError> {
        if let Some(html_str) = value.as_str() {
            let sanitized = self.sanitize_html(html_str)?;
            *value = serde_json::Value::String(sanitized);
            Ok(())
        } else {
            Err(SanitizationError::InvalidType { expected: "string".to_string() })
        }
    }
    
    fn name(&self) -> &str { "html" }
}
```

### Tool Composition

```rust
pub struct ToolChain {
    steps: Vec<ChainStep>,
    error_handling: ErrorHandlingStrategy,
    parallel_execution: bool,
}

impl ToolChain {
    pub fn new() -> Self;
    
    pub fn add_step<T: Tool + 'static>(mut self, tool: T, mapping: OutputMapping) -> Self;
    
    pub fn add_conditional_step<T: Tool + 'static>(mut self, condition: Condition, tool: T, mapping: OutputMapping) -> Self;
    
    pub fn add_parallel_steps(mut self, steps: Vec<ChainStep>) -> Self;
    
    pub async fn execute(&self, initial_input: serde_json::Value, context: &ToolContext) -> Result<serde_json::Value, ChainExecutionError>;
    
    pub fn validate_chain(&self) -> Result<(), ChainValidationError>;
}

#[derive(Debug, Clone)]
pub struct ChainStep {
    pub tool_name: String,
    pub input_mapping: InputMapping,
    pub output_mapping: OutputMapping,
    pub condition: Option<Condition>,
    pub error_handling: StepErrorHandling,
}

#[derive(Debug, Clone)]
pub enum OutputMapping {
    Direct,
    Transform { expression: String },
    Extract { fields: Vec<String> },
    Merge { with_previous: bool },
    Custom { mapper: String },
}

pub struct ToolComposer {
    registry: Arc<ToolRegistry>,
    chain_builder: ChainBuilder,
    optimizer: ChainOptimizer,
}

impl ToolComposer {
    pub fn new(registry: Arc<ToolRegistry>) -> Self;
    
    pub fn create_chain(&self) -> ChainBuilder;
    
    pub fn optimize_chain(&self, chain: ToolChain) -> Result<ToolChain, OptimizationError>;
    
    pub fn analyze_dependencies(&self, chain: &ToolChain) -> DependencyAnalysis;
    
    pub fn estimate_execution_time(&self, chain: &ToolChain) -> Duration;
    
    pub fn estimate_resource_usage(&self, chain: &ToolChain) -> ResourceEstimate;
}
```

## Implementation Details

### Technology Stack

- **Async Runtime**: Tokio for async tool execution
- **Serialization**: Serde with JSON Schema generation
- **Sandboxing**: Process isolation with resource limits
- **Validation**: jsonschema with custom validators
- **Monitoring**: OpenTelemetry for execution tracing
- **Security**: Permission-based access control
- **Performance**: Connection pooling and caching

### Key Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
jsonschema = "0.17"
schemars = "0.8"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
tracing = "0.1"
dashmap = "5.0"
parking_lot = "0.12"
opentelemetry = "0.20"
nix = "0.26"
libc = "0.2"
symbiote-core = { path = "../symbiote-core" }
symbiote-security = { path = "../security" }

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"
criterion = "0.5"
```

### Macro Implementation

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(Tool, attributes(tool))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    // Extract tool attributes
    let tool_attrs = extract_tool_attributes(&input.attrs);
    
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    let expanded = quote! {
        impl #impl_generics Tool for #name #ty_generics #where_clause {
            type Input = #tool_attrs.input_type;
            type Output = #tool_attrs.output_type;
            type Error = #tool_attrs.error_type;
            
            fn metadata(&self) -> ToolMetadata {
                ToolMetadata {
                    name: #tool_attrs.name.to_string(),
                    description: #tool_attrs.description.to_string(),
                    version: #tool_attrs.version.to_string(),
                    // ... other metadata fields
                }
            }
            
            fn required_permissions(&self) -> Vec<Permission> {
                vec![#(#tool_attrs.permissions),*]
            }
            
            fn timeout(&self) -> Option<Duration> {
                #tool_attrs.timeout
            }
        }
    };
    
    TokenStream::from(expanded)
}
```

## Testing Strategy

### Unit Tests

- **Tool Definition**: Test tool trait implementations
- **Validation**: Test input/output validation logic
- **Sandboxing**: Test resource isolation and limits
- **Registry**: Test tool registration and discovery
- **Execution**: Test tool execution engine

### Integration Tests

- **End-to-End**: Test complete tool execution pipeline
- **Security**: Test sandboxing and permission enforcement
- **Performance**: Test execution performance and resource usage
- **Composition**: Test tool chaining and composition
- **Error Handling**: Test error scenarios and recovery

### Property-Based Tests

- **Schema Generation**: Test that generated schemas are valid
- **Serialization**: Test that serialization round-trips correctly
- **Resource Limits**: Test that resource limits are enforced

## Integration Points

### Upstream Dependencies

- **symbiote-core**: Uses error types, configuration, and async utilities
- **security**: Uses permission management and sandboxing

### Downstream Consumers

- **Agent Framework**: Tools used by AI agents
- **Workflow Engine**: Tools used in workflow nodes
- **Assistant**: Tools available to personal AI assistant
- **Trading System**: Tools for trading operations
- **IDE Features**: Tools for development assistance

### External Integrations

- **System APIs**: Operating system and hardware interfaces
- **Web Services**: HTTP APIs and web service integrations
- **Databases**: Database query and manipulation tools
- **File Systems**: File and directory operations

## Acceptance Criteria

### Functional Requirements

- [ ] Type-safe tool definition with compile-time validation
- [ ] Automatic JSON Schema generation from Rust types
- [ ] Sandboxed tool execution with resource limits
- [ ] Comprehensive input/output validation
- [ ] Tool registry with discovery and versioning
- [ ] Tool composition and chaining capabilities
- [ ] Security and permission management

### Non-Functional Requirements

- [ ] Sub-millisecond tool execution overhead
- [ ] Support for 1000+ registered tools
- [ ] 99.9% execution success rate
- [ ] Memory usage under 50MB for tool registry
- [ ] Cross-platform sandboxing support
- [ ] Comprehensive error handling and recovery

### Quality Gates

- [ ] All tests pass with 95%+ coverage
- [ ] Performance benchmarks meet targets
- [ ] Security audit passes for sandboxing
- [ ] Documentation complete with examples
- [ ] Tool development tutorials available
- [ ] Schema validation accuracy verified

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with proc-macro support
- **Development Tools**: cargo-expand for macro debugging
- **Testing Tools**: criterion for performance testing

### Runtime Dependencies

- **Sandboxing**: OS support for process isolation
- **Resource Management**: System resource monitoring
- **Security**: Permission and access control systems
- **Monitoring**: Observability infrastructure

### Development Prerequisites

- **Tool Design Knowledge**: Understanding of tool architecture patterns
- **Security Knowledge**: Understanding of sandboxing and isolation
- **Performance Engineering**: Knowledge of efficient execution patterns
- **Documentation**: Comprehensive tool development guides

## Database Schema

### Tool Management Persistence

```sql
-- Tool registry and metadata
CREATE TABLE tools_registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) NOT NULL UNIQUE,
    tool_name VARCHAR(255) NOT NULL,
    tool_version VARCHAR(50) NOT NULL,
    tool_type VARCHAR(100), -- 'builtin', 'custom', 'external', 'ai_powered'
    description TEXT,
    author VARCHAR(255),
    license VARCHAR(100),
    category VARCHAR(100), -- 'ai', 'network', 'storage', 'system', 'custom'
    tags JSONB, -- Array of tags for categorization
    input_schema JSONB NOT NULL, -- JSON Schema for input validation
    output_schema JSONB NOT NULL, -- JSON Schema for output validation
    configuration JSONB, -- Tool-specific configuration
    resource_requirements JSONB, -- CPU, memory, disk, network requirements
    permissions JSONB, -- Required permissions array
    security_profile VARCHAR(100), -- 'minimal', 'standard', 'elevated', 'administrative'
    is_active BOOLEAN DEFAULT true,
    is_sandboxed BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_used TIMESTAMP,
    usage_count INTEGER DEFAULT 0,
    rating DECIMAL(3,2),
    metadata JSONB
);

-- Tool execution history and logs
CREATE TABLE tools_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id VARCHAR(255) NOT NULL UNIQUE,
    tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    user_id VARCHAR(255),
    session_id VARCHAR(255),
    agent_id VARCHAR(255),
    workflow_id VARCHAR(255),
    input_data JSONB NOT NULL,
    output_data JSONB,
    execution_status VARCHAR(50), -- 'running', 'completed', 'failed', 'timeout', 'cancelled'
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    duration_ms INTEGER,
    resource_usage JSONB, -- CPU, memory, disk, network usage
    error_message TEXT,
    error_code VARCHAR(100),
    stack_trace TEXT,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    context JSONB, -- Execution context and environment
    audit_trail JSONB, -- Security audit information
    created_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Tool dependencies and relationships
CREATE TABLE tools_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    dependency_tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    dependency_type VARCHAR(100), -- 'required', 'optional', 'suggested'
    version_constraint VARCHAR(100), -- Semantic version constraint
    created_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    UNIQUE(tool_id, dependency_tool_id)
);

-- Tool permissions and security policies
CREATE TABLE tools_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    permission_type VARCHAR(100) NOT NULL, -- 'file_read', 'file_write', 'network', 'system', etc.
    resource_pattern VARCHAR(500), -- Pattern for allowed resources
    access_level VARCHAR(50), -- 'read', 'write', 'execute', 'admin'
    conditions JSONB, -- Conditions for permission grant
    is_required BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Tool performance metrics
CREATE TABLE tools_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    metric_name VARCHAR(255) NOT NULL,
    metric_type VARCHAR(100), -- 'counter', 'gauge', 'histogram', 'timer'
    metric_value DECIMAL(20,6) NOT NULL,
    labels JSONB, -- Key-value pairs for metric labels
    recorded_at TIMESTAMP NOT NULL,
    execution_id VARCHAR(255),
    user_id VARCHAR(255),
    session_id VARCHAR(255),
    metadata JSONB
);

-- Tool validation rules and schemas
CREATE TABLE tools_validation_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    rule_name VARCHAR(255) NOT NULL,
    rule_type VARCHAR(100), -- 'input', 'output', 'security', 'performance'
    validation_schema JSONB NOT NULL, -- JSON Schema or custom validation rules
    error_message_template TEXT,
    is_active BOOLEAN DEFAULT true,
    severity VARCHAR(20), -- 'info', 'warning', 'error', 'critical'
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Tool chains and compositions
CREATE TABLE tools_chains (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain_id VARCHAR(255) NOT NULL UNIQUE,
    chain_name VARCHAR(255) NOT NULL,
    description TEXT,
    chain_definition JSONB NOT NULL, -- Array of tool steps with connections
    input_schema JSONB, -- Overall chain input schema
    output_schema JSONB, -- Overall chain output schema
    created_by VARCHAR(255),
    is_public BOOLEAN DEFAULT false,
    usage_count INTEGER DEFAULT 0,
    rating DECIMAL(3,2),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Tool sandbox configurations
CREATE TABLE tools_sandbox_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tool_id VARCHAR(255) REFERENCES tools_registry(tool_id),
    sandbox_type VARCHAR(100), -- 'process', 'container', 'vm', 'wasm'
    resource_limits JSONB NOT NULL, -- CPU, memory, disk, network limits
    allowed_syscalls JSONB, -- Array of allowed system calls
    blocked_syscalls JSONB, -- Array of blocked system calls
    environment_variables JSONB, -- Allowed environment variables
    network_policy JSONB, -- Network access policy
    filesystem_policy JSONB, -- Filesystem access policy
    timeout_seconds INTEGER DEFAULT 30,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Indexes for performance
CREATE INDEX idx_tools_registry_tool_id ON tools_registry(tool_id);
CREATE INDEX idx_tools_registry_category ON tools_registry(category);
CREATE INDEX idx_tools_registry_active ON tools_registry(is_active);
CREATE INDEX idx_tools_registry_tags ON tools_registry USING GIN(tags);
CREATE INDEX idx_tools_executions_tool_id ON tools_executions(tool_id);
CREATE INDEX idx_tools_executions_user ON tools_executions(user_id);
CREATE INDEX idx_tools_executions_status ON tools_executions(execution_status);
CREATE INDEX idx_tools_executions_started_at ON tools_executions(started_at);
CREATE INDEX idx_tools_dependencies_tool ON tools_dependencies(tool_id);
CREATE INDEX idx_tools_dependencies_dependency ON tools_dependencies(dependency_tool_id);
CREATE INDEX idx_tools_permissions_tool ON tools_permissions(tool_id);
CREATE INDEX idx_tools_permissions_type ON tools_permissions(permission_type);
CREATE INDEX idx_tools_metrics_tool ON tools_metrics(tool_id);
CREATE INDEX idx_tools_metrics_recorded_at ON tools_metrics(recorded_at);
CREATE INDEX idx_tools_validation_rules_tool ON tools_validation_rules(tool_id);
CREATE INDEX idx_tools_validation_rules_type ON tools_validation_rules(rule_type);
CREATE INDEX idx_tools_chains_chain_id ON tools_chains(chain_id);
CREATE INDEX idx_tools_chains_created_by ON tools_chains(created_by);
CREATE INDEX idx_tools_sandbox_configs_tool ON tools_sandbox_configs(tool_id);
```

## Error Handling

### Tool Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolsError {
    #[error("Tool registration failed: {tool_name} - {reason}")]
    ToolRegistrationFailed { tool_name: String, reason: String },

    #[error("Tool execution failed: {tool_id} - {error}")]
    ToolExecutionFailed { tool_id: String, error: String },

    #[error("Tool validation failed: {tool_id} - {validation_error}")]
    ToolValidationFailed { tool_id: String, validation_error: String },

    #[error("Tool discovery failed: {search_criteria} - {reason}")]
    ToolDiscoveryFailed { search_criteria: String, reason: String },

    #[error("Tool schema generation failed: {tool_id} - {error}")]
    ToolSchemaGenerationFailed { tool_id: String, error: String },

    #[error("Tool sandbox creation failed: {tool_id} - {reason}")]
    ToolSandboxCreationFailed { tool_id: String, reason: String },

    #[error("Tool permission denied: {tool_id} - {permission} - {reason}")]
    ToolPermissionDenied { tool_id: String, permission: String, reason: String },

    #[error("Tool dependency resolution failed: {tool_id} - {dependency} - {error}")]
    ToolDependencyResolutionFailed { tool_id: String, dependency: String, error: String },

    #[error("Tool chain execution failed: {chain_id} - {step} - {reason}")]
    ToolChainExecutionFailed { chain_id: String, step: String, reason: String },

    #[error("Tool resource limit exceeded: {tool_id} - {resource} - {limit}")]
    ToolResourceLimitExceeded { tool_id: String, resource: String, limit: String },

    #[error("Tool timeout: {tool_id} - {timeout_seconds}s")]
    ToolTimeout { tool_id: String, timeout_seconds: u64 },

    #[error("Tool not found: {tool_id}")]
    ToolNotFound { tool_id: String },

    #[error("Tool version conflict: {tool_id} - required {required}, found {found}")]
    ToolVersionConflict { tool_id: String, required: String, found: String },

    #[error("Tool configuration error: {tool_id} - {setting} - {issue}")]
    ToolConfigurationError { tool_id: String, setting: String, issue: String },

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

pub type ToolsResult<T> = Result<T, ToolsError>;

impl From<std::io::Error> for ToolsError {
    fn from(err: std::io::Error) -> Self {
        ToolsError::InternalError {
            details: format!("IO error: {}", err),
        }
    }
}

impl From<serde_json::Error> for ToolsError {
    fn from(err: serde_json::Error) -> Self {
        ToolsError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<jsonschema::ValidationError<'_>> for ToolsError {
    fn from(err: jsonschema::ValidationError) -> Self {
        ToolsError::ToolValidationFailed {
            tool_id: "unknown".to_string(),
            validation_error: err.to_string(),
        }
    }
}
```

This tools SDK provides the comprehensive, type-safe, and secure foundation for creating and managing tools within Symbiote, enabling developers to build powerful capabilities while maintaining safety and performance standards.
