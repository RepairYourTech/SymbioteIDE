# Diff - Code Diff Analysis & Visualization Plan

## Goals & Vision

The `diff` crate provides comprehensive code diff analysis and visualization capabilities for Symbiote. It offers:

- **Intelligent Diff Analysis**: AI-powered diff analysis with semantic understanding
- **Multi-Format Support**: Git diffs, unified diffs, side-by-side comparisons
- **Visual Diff Viewer**: Rich visual diff interface with syntax highlighting
- **Change Impact Analysis**: Analyze the impact of code changes across the codebase
- **Conflict Resolution**: AI-assisted merge conflict resolution
- **Code Review Integration**: Seamless integration with code review workflows
- **Performance Analysis**: Analyze performance implications of changes
- **Security Analysis**: Detect security implications in code changes

This crate provides the foundation for intelligent code change analysis and review.

## UI Design Specifications

### Diff Viewer Interface

#### Main Diff Viewer
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 📊 Code Diff Analyzer                           [🔍] [⚙️] [📤] [🤖]        │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📁 Files Changed (12)                                                       │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ ✅ src/main.rs                    │ +47 -12 │ 🟢 Safe    │ [View]      │ │
│ │ ⚠️  src/auth.rs                   │ +23 -8  │ 🟡 Review  │ [View]      │ │
│ │ 🔴 src/database.rs                │ +15 -45 │ 🔴 Risk    │ [View]      │ │
│ │ ✅ tests/integration.rs           │ +89 -0  │ 🟢 Safe    │ [View]      │ │
│ │ ✅ Cargo.toml                     │ +2 -1   │ 🟢 Safe    │ [View]      │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔍 Side-by-Side Diff: src/auth.rs                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Before (v1.2.3)              │ After (working)                          │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 1  use std::collections::     │ 1  use std::collections::                │ │
│ │ 2  HashMap;                   │ 2  HashMap;                              │ │
│ │ 3                             │ 3  use bcrypt::{hash, verify};           │ │
│ │ 4  pub fn authenticate(       │ 4                                        │ │
│ │ 5      username: &str,        │ 5  pub fn authenticate(                  │ │
│ │ 6      password: &str         │ 6      username: &str,                   │ │
│ │ 7  ) -> bool {                │ 7      password: &str                    │ │
│ │ 8      // TODO: Add proper    │ 8  ) -> Result<bool, AuthError> {       │ │
│ │ 9      // password hashing    │ 9      let stored_hash = get_user_hash(  │ │
│ │10      username == "admin" && │10          username                      │ │
│ │11      password == "password" │11      )?;                               │ │
│ │12  }                          │12                                        │ │
│ │                               │13      Ok(verify(password, &stored_hash) │ │
│ │                               │14          .map_err(AuthError::Bcrypt)?) │ │
│ │                               │15  }                                     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🤖 AI Analysis                                                              │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ ✅ **Security Improvement Detected**                                    │ │
│ │                                                                         │ │
│ │ **Changes:**                                                            │ │
│ │ • Replaced hardcoded credentials with proper password hashing          │ │
│ │ • Added bcrypt dependency for secure password verification             │ │
│ │ • Improved error handling with Result type                             │ │
│ │ • Added proper database lookup for user credentials                    │ │
│ │                                                                         │ │
│ │ **Impact Assessment:**                                                  │ │
│ │ • 🔒 Security: Significantly improved (Critical vulnerability fixed)   │ │
│ │ • 🚀 Performance: Minimal impact (bcrypt is optimized)                │ │
│ │ • 🧪 Testing: Requires new test cases for error scenarios             │ │
│ │ • 📚 Documentation: Update API docs for new return type               │ │
│ │                                                                         │ │
│ │ **Recommendations:**                                                    │ │
│ │ • Add rate limiting to prevent brute force attacks                     │ │
│ │ • Consider adding 2FA support in future iterations                     │ │
│ │ • Update all callers to handle the new Result type                     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Database Schema

### Diff Analysis Persistence

```sql
-- Diff analysis sessions
CREATE TABLE diff_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id VARCHAR(255) NOT NULL UNIQUE,
    user_id VARCHAR(255) NOT NULL,
    session_name VARCHAR(255),
    diff_source VARCHAR(100), -- 'git', 'file_comparison', 'branch_comparison', 'commit_range'
    base_reference VARCHAR(255), -- Base commit, branch, or file
    target_reference VARCHAR(255), -- Target commit, branch, or file
    repository_url VARCHAR(500),
    repository_path VARCHAR(500),
    diff_format VARCHAR(50), -- 'unified', 'side_by_side', 'inline', 'context'
    analysis_status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'analyzing', 'completed', 'failed'
    total_files_changed INTEGER,
    total_lines_added INTEGER,
    total_lines_removed INTEGER,
    security_risk_level VARCHAR(50), -- 'low', 'medium', 'high', 'critical'
    performance_impact VARCHAR(50), -- 'none', 'minimal', 'moderate', 'significant'
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Individual file diffs within sessions
CREATE TABLE file_diffs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    diff_id VARCHAR(255) NOT NULL UNIQUE,
    session_id VARCHAR(255) REFERENCES diff_sessions(session_id),
    file_path VARCHAR(1000) NOT NULL,
    file_type VARCHAR(100), -- 'source_code', 'config', 'documentation', 'binary'
    language VARCHAR(100), -- Programming language detected
    change_type VARCHAR(50), -- 'added', 'modified', 'deleted', 'renamed', 'copied'
    lines_added INTEGER DEFAULT 0,
    lines_removed INTEGER DEFAULT 0,
    lines_context INTEGER DEFAULT 0,
    diff_content TEXT, -- The actual diff content
    parsed_diff JSONB, -- Structured diff data
    syntax_highlighted_diff TEXT, -- HTML with syntax highlighting
    security_issues JSONB, -- Array of security concerns
    performance_issues JSONB, -- Array of performance concerns
    code_smells JSONB, -- Array of code quality issues
    complexity_change INTEGER, -- Change in cyclomatic complexity
    test_coverage_impact DECIMAL(5,2), -- Impact on test coverage percentage
    created_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Merge conflicts and resolutions
CREATE TABLE merge_conflicts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conflict_id VARCHAR(255) NOT NULL UNIQUE,
    session_id VARCHAR(255) REFERENCES diff_sessions(session_id),
    file_path VARCHAR(1000) NOT NULL,
    conflict_type VARCHAR(100), -- 'content', 'rename', 'delete_modify', 'add_add'
    base_content TEXT,
    theirs_content TEXT,
    ours_content TEXT,
    conflict_markers TEXT, -- Content with conflict markers
    resolution_status VARCHAR(50) DEFAULT 'unresolved', -- 'unresolved', 'auto_resolved', 'manually_resolved'
    resolution_method VARCHAR(100), -- 'ai_assisted', 'manual', 'auto_merge', 'take_theirs', 'take_ours'
    resolved_content TEXT,
    resolution_confidence DECIMAL(3,2), -- AI confidence in resolution (0.00-1.00)
    resolved_by VARCHAR(255), -- User ID who resolved the conflict
    resolved_at TIMESTAMP,
    ai_suggestions JSONB, -- Array of AI-suggested resolutions
    created_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);
```

## Architecture & Design

### Core Modules

```
diff/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── parser/               # Diff parsing
│   │   ├── mod.rs
│   │   ├── git_diff.rs       # Git diff parsing
│   │   ├── unified_diff.rs   # Unified diff parsing
│   │   ├── patch_parser.rs   # Patch file parsing
│   │   └── binary_diff.rs    # Binary file diff handling
│   ├── analyzer/             # Diff analysis
│   │   ├── mod.rs
│   │   ├── semantic.rs       # Semantic analysis
│   │   ├── impact.rs         # Change impact analysis
│   │   ├── security.rs       # Security impact analysis
│   │   ├── performance.rs    # Performance impact analysis
│   │   └── complexity.rs     # Code complexity analysis
│   ├── viewer/               # Diff visualization
│   │   ├── mod.rs
│   │   ├── side_by_side.rs   # Side-by-side view
│   │   ├── unified.rs        # Unified view
│   │   ├── inline.rs         # Inline view
│   │   └── syntax_highlight.rs # Syntax highlighting
│   ├── merger/               # Merge operations
│   │   ├── mod.rs
│   │   ├── three_way.rs      # Three-way merge
│   │   ├── conflict_resolution.rs # Conflict resolution
│   │   ├── auto_merge.rs     # Automatic merging
│   │   └── ai_merge.rs       # AI-assisted merging
│   ├── statistics/           # Diff statistics
│   │   ├── mod.rs
│   │   ├── metrics.rs        # Change metrics
│   │   ├── churn.rs          # Code churn analysis
│   │   ├── hotspots.rs       # Change hotspot detection
│   │   └── trends.rs         # Change trend analysis
│   └── types/                # Diff types
│       ├── mod.rs
│       ├── diff_types.rs     # Core diff types
│       ├── change_types.rs   # Change classification
│       └── metadata.rs       # Diff metadata
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── basic_diff.rs
    └── ai_analysis.rs
```

## APIs & Interfaces

### Core Diff Engine

```rust
pub struct DiffEngine {
    parser: DiffParser,
    analyzer: DiffAnalyzer,
    viewer: DiffViewer,
    merger: DiffMerger,
    config: DiffConfig,
}

impl DiffEngine {
    pub async fn new(config: DiffConfig) -> DiffResult<Self>;
    
    pub async fn parse_diff(&self, diff_content: &str, format: DiffFormat) -> DiffResult<ParsedDiff>;
    
    pub async fn analyze_changes(&self, diff: &ParsedDiff) -> DiffResult<ChangeAnalysis>;
    
    pub async fn generate_visual_diff(&self, diff: &ParsedDiff, style: ViewStyle) -> DiffResult<VisualDiff>;
    
    pub async fn detect_conflicts(&self, diffs: &[ParsedDiff]) -> DiffResult<Vec<MergeConflict>>;
    
    pub async fn auto_merge(&self, base: &str, theirs: &str, ours: &str) -> DiffResult<MergeResult>;
    
    pub async fn ai_assisted_merge(&self, conflict: &MergeConflict) -> DiffResult<MergeResolution>;
    
    pub async fn calculate_diff_stats(&self, diff: &ParsedDiff) -> DiffResult<DiffStatistics>;
    
    pub async fn analyze_security_impact(&self, diff: &ParsedDiff) -> DiffResult<SecurityAnalysis>;
    
    pub async fn analyze_performance_impact(&self, diff: &ParsedDiff) -> DiffResult<PerformanceAnalysis>;
    
    pub async fn detect_code_smells(&self, diff: &ParsedDiff) -> DiffResult<Vec<CodeSmell>>;
    
    pub async fn suggest_improvements(&self, diff: &ParsedDiff) -> DiffResult<Vec<Improvement>>;
    
    pub async fn generate_change_summary(&self, diff: &ParsedDiff) -> DiffResult<ChangeSummary>;
    
    pub async fn export_diff(&self, diff: &ParsedDiff, format: ExportFormat) -> DiffResult<String>;
    
    pub async fn compare_branches(&self, base_branch: &str, target_branch: &str) -> DiffResult<BranchComparison>;
    
    pub async fn track_file_history(&self, file_path: &str, commits: &[String]) -> DiffResult<FileHistory>;
    
    pub async fn analyze_code_churn(&self, timeframe: TimeRange) -> DiffResult<ChurnAnalysis>;
    
    pub async fn detect_change_patterns(&self, diffs: &[ParsedDiff]) -> DiffResult<ChangePatterns>;
    
    pub async fn generate_review_checklist(&self, diff: &ParsedDiff) -> DiffResult<ReviewChecklist>;
    
    pub async fn estimate_review_time(&self, diff: &ParsedDiff) -> DiffResult<Duration>;
}
```

### Diff Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDiff {
    pub files: Vec<FileDiff>,
    pub metadata: DiffMetadata,
    pub statistics: DiffStatistics,
    pub format: DiffFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: ChangeType,
    pub hunks: Vec<DiffHunk>,
    pub binary: bool,
    pub mode_change: Option<ModeChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: LineType,
    pub content: String,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
    pub syntax_highlighting: Option<SyntaxHighlight>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineType {
    Context,
    Added,
    Removed,
    NoNewlineAtEof,
}

#[derive(Debug, Clone)]
pub struct ChangeAnalysis {
    pub impact_score: f32,
    pub complexity_change: i32,
    pub security_implications: Vec<SecurityIssue>,
    pub performance_implications: Vec<PerformanceIssue>,
    pub breaking_changes: Vec<BreakingChange>,
    pub test_coverage_impact: TestCoverageImpact,
    pub documentation_updates_needed: Vec<DocumentationUpdate>,
}
```

## Error Handling

### Diff Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiffError {
    #[error("Diff parsing failed: {format} - {reason}")]
    DiffParsingFailed { format: String, reason: String },

    #[error("File comparison failed: {file1} vs {file2} - {error}")]
    FileComparisonFailed { file1: String, file2: String, error: String },

    #[error("Merge conflict resolution failed: {file_path} - {reason}")]
    MergeResolutionFailed { file_path: String, reason: String },

    #[error("Git operation failed: {operation} - {error}")]
    GitOperationFailed { operation: String, error: String },

    #[error("Syntax highlighting failed: {language} - {reason}")]
    SyntaxHighlightingFailed { language: String, reason: String },

    #[error("Change analysis failed: {analysis_type} - {error}")]
    ChangeAnalysisFailed { analysis_type: String, error: String },

    #[error("Visualization generation failed: {view_type} - {reason}")]
    VisualizationFailed { view_type: String, reason: String },

    #[error("Export failed: {format} - {error}")]
    ExportFailed { format: String, error: String },

    #[error("Repository access failed: {repo_path} - {reason}")]
    RepositoryAccessFailed { repo_path: String, reason: String },

    #[error("Branch comparison failed: {base} vs {target} - {error}")]
    BranchComparisonFailed { base: String, target: String, error: String },

    #[error("File not found: {file_path}")]
    FileNotFound { file_path: String },

    #[error("Invalid diff format: {format} - expected {expected}")]
    InvalidDiffFormat { format: String, expected: String },

    #[error("Encoding error: {file_path} - {encoding_issue}")]
    EncodingError { file_path: String, encoding_issue: String },

    #[error("Binary file comparison not supported: {file_path}")]
    BinaryFileNotSupported { file_path: String },

    #[error("Permission denied: {operation} - {resource}")]
    PermissionDenied { operation: String, resource: String },

    #[error("Configuration error: {setting} - {issue}")]
    ConfigurationError { setting: String, issue: String },

    #[error("Memory limit exceeded: {operation} requires {required}MB")]
    MemoryLimitExceeded { operation: String, required: u64 },

    #[error("Timeout error: {operation} timed out after {duration_ms}ms")]
    TimeoutError { operation: String, duration_ms: u64 },

    #[error("Database error: {operation} - {error}")]
    DatabaseError { operation: String, error: String },

    #[error("Serialization error: {data_type} - {error}")]
    SerializationError { data_type: String, error: String },

    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },

    #[error("Internal error: {details}")]
    InternalError { details: String },
}

pub type DiffResult<T> = Result<T, DiffError>;

impl From<std::io::Error> for DiffError {
    fn from(err: std::io::Error) -> Self {
        DiffError::InternalError {
            details: format!("IO error: {}", err),
        }
    }
}

impl From<git2::Error> for DiffError {
    fn from(err: git2::Error) -> Self {
        DiffError::GitOperationFailed {
            operation: "git_operation".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for DiffError {
    fn from(err: serde_json::Error) -> Self {
        DiffError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}
```

## Security Model

### Diff Security Framework

```rust
pub struct DiffSecurityManager {
    access_control: DiffAccessControl,
    content_scanner: ContentSecurityScanner,
    audit_logger: DiffAuditLogger,
    privacy_manager: DiffPrivacyManager,
}

impl DiffSecurityManager {
    /// Validate user access to diff operations
    pub async fn validate_diff_access(&self, user_id: &str, operation: DiffOperation) -> DiffResult<AccessDecision>;

    /// Scan diff content for sensitive information
    pub async fn scan_sensitive_content(&self, diff: &ParsedDiff) -> DiffResult<SensitivityScanResult>;

    /// Redact sensitive information from diffs
    pub async fn redact_sensitive_data(&self, diff: &mut ParsedDiff, redaction_rules: &[RedactionRule]) -> DiffResult<RedactionResult>;

    /// Log diff operations for audit
    pub async fn log_diff_operation(&self, operation: &DiffOperation, user_id: &str, result: &OperationResult) -> DiffResult<()>;

    /// Validate repository access permissions
    pub async fn validate_repository_access(&self, user_id: &str, repo_path: &str, operation: RepositoryOperation) -> DiffResult<AccessDecision>;

    /// Secure diff sharing and collaboration
    pub async fn secure_diff_sharing(&self, diff_id: &str, sharing_config: SharingConfig) -> DiffResult<SecureShare>;

    /// Handle security violations in diffs
    pub async fn handle_security_violation(&self, violation: &SecurityViolation) -> DiffResult<ViolationResponse>;
}

#[derive(Debug, Clone)]
pub enum DiffOperation {
    ParseDiff { source: String, format: String },
    CompareBranches { base: String, target: String, repo_path: String },
    AnalyzeChanges { diff_id: String, analysis_types: Vec<String> },
    ResolveMergeConflict { conflict_id: String, resolution_method: String },
    ExportDiff { diff_id: String, format: String },
    ShareDiff { diff_id: String, recipients: Vec<String> },
    AccessRepository { repo_path: String, operation_type: String },
    ViewSensitiveChanges { diff_id: String, sensitivity_level: String },
}

#[derive(Debug, Clone)]
pub enum SensitivityLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
}
```

## Implementation Details

### Technology Stack

- **Diff Parsing**: Custom diff parsers for Git, unified, and context diff formats
- **Git Integration**: libgit2 for Git repository operations and history analysis
- **Syntax Highlighting**: Tree-sitter for accurate syntax highlighting across languages
- **AI Analysis**: Uses ai crate for intelligent change analysis and merge conflict resolution
- **Visualization**: Rich HTML/CSS diff viewers with interactive features
- **Performance**: Streaming diff processing for large files and repositories
- **Security**: Content scanning for sensitive data and security vulnerabilities
- **Export**: Multiple export formats including HTML, PDF, and structured data

### Key Features

1. **Intelligent Diff Analysis**: AI-powered diff analysis with semantic understanding
2. **Multi-Format Support**: Git diffs, unified diffs, side-by-side comparisons
3. **Visual Diff Viewer**: Rich visual diff interface with syntax highlighting
4. **Change Impact Analysis**: Analyze the impact of code changes across the codebase
5. **AI-Assisted Conflict Resolution**: Smart merge conflict resolution with AI suggestions
6. **Code Review Integration**: Seamless integration with code review workflows
7. **Performance Analysis**: Analyze performance implications of changes
8. **Security Analysis**: Detect security implications in code changes

## Integration Points

### Context Engine Integration
- Index diff content for semantic search
- Build relationships between related changes
- Track change patterns and evolution

### AI Integration
- Semantic diff analysis and explanation
- Automated merge conflict resolution
- Code review assistance and suggestions
- Impact prediction and risk assessment

### VCS Integration
- Git repository integration
- Branch comparison and analysis
- Commit history tracking
- Pull request integration

### IDE Integration
- Real-time diff visualization
- Inline change annotations
- Code review workflows
- Merge conflict resolution UI

## Performance Considerations

- **Streaming Processing**: Handle large diffs efficiently
- **Incremental Analysis**: Update analysis as changes are made
- **Caching**: Cache parsed diffs and analysis results
- **Parallel Processing**: Analyze multiple files concurrently
- **Memory Management**: Efficient memory usage for large diffs

## Security Considerations

- **Input Validation**: Validate all diff input for security
- **Sanitization**: Sanitize diff content for display
- **Access Control**: Ensure proper access to sensitive diffs
- **Audit Logging**: Log all diff operations for security
- **Secure Storage**: Encrypt sensitive diff data

## Testing Strategy

- **Unit Tests**: Test individual diff parsing and analysis functions
- **Integration Tests**: Test end-to-end diff workflows
- **Performance Tests**: Benchmark diff processing performance
- **Security Tests**: Test security vulnerability detection
- **UI Tests**: Test diff visualization components
- **Compatibility Tests**: Test with various diff formats and tools
