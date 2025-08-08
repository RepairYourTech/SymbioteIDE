// Unified Notebook System - AI-Powered Interactive Development Environment
// Phase 4 Feature: Revolutionary notebook system combining code, documentation, visualization, and AI collaboration

use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unified Notebook System
pub struct UnifiedNotebookSystem {
    // Core notebook management
    notebook_manager: NotebookManager,
    cell_engine: CellEngine,
    execution_engine: ExecutionEngine,
    
    // AI integration
    ai_assistant: NotebookAIAssistant,
    code_generator: NotebookCodeGenerator,
    documentation_generator: DocumentationGenerator,
    
    // Collaboration system
    collaboration_engine: CollaborationEngine,
    version_control: NotebookVersionControl,
    sharing_manager: SharingManager,
    
    // Visualization and rendering
    visualization_engine: VisualizationEngine,
    renderer: NotebookRenderer,
    export_engine: ExportEngine,
    
    // Data and state management
    notebooks: Arc<RwLock<HashMap<String, Notebook>>>,
    active_sessions: Arc<RwLock<HashMap<String, NotebookSession>>>,
    
    // Performance and analytics
    performance_monitor: PerformanceMonitor,
    analytics: Arc<RwLock<NotebookAnalytics>>,
}

/// Notebook structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub notebook_id: String,
    pub name: String,
    pub description: String,
    pub notebook_type: NotebookType,
    pub metadata: NotebookMetadata,
    pub cells: Vec<NotebookCell>,
    pub kernel_info: KernelInfo,
    pub environment: NotebookEnvironment,
    pub collaboration_info: CollaborationInfo,
    pub version_info: VersionInfo,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub last_executed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotebookType {
    // Standard notebook types
    Jupyter,
    Research,
    Tutorial,
    Documentation,
    
    // AI-enhanced types
    AIAssisted,
    CodeGeneration,
    DataAnalysis,
    MachineLearning,
    
    // Collaborative types
    Shared,
    Template,
    Workshop,
    Presentation,
    
    // Specialized types
    Experiment,
    Report,
    Dashboard,
    Interactive,
    
    // Custom type
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookMetadata {
    pub title: String,
    pub author: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub language: String,
    pub kernel_spec: KernelSpec,
    pub dependencies: Vec<String>,
    pub data_sources: Vec<DataSource>,
    pub output_formats: Vec<OutputFormat>,
    pub sharing_settings: SharingSettings,
    pub ai_settings: AISettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub display_name: String,
    pub language: String,
    pub version: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub environment_variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_id: String,
    pub name: String,
    pub source_type: DataSourceType,
    pub connection_info: ConnectionInfo,
    pub schema_info: Option<SchemaInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSourceType {
    Database,
    API,
    File,
    Stream,
    Cloud,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub connection_string: String,
    pub authentication: AuthenticationInfo,
    pub connection_options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationInfo {
    pub auth_type: AuthType,
    pub credentials: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
    OAuth,
    ApiKey,
    Certificate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub tables: Vec<TableInfo>,
    pub relationships: Vec<RelationshipInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInfo {
    pub from_table: String,
    pub to_table: String,
    pub relationship_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    HTML,
    PDF,
    Markdown,
    LaTeX,
    Slides,
    Dashboard,
    Interactive,
    JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingSettings {
    pub visibility: Visibility,
    pub permissions: Vec<Permission>,
    pub collaboration_mode: CollaborationMode,
    pub export_allowed: bool,
    pub comment_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Private,
    Team,
    Organization,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub user_id: String,
    pub permission_type: PermissionType,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionType {
    Read,
    Write,
    Execute,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationMode {
    ReadOnly,
    Collaborative,
    Synchronized,
    Branched,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISettings {
    pub ai_assistance_enabled: bool,
    pub auto_completion: bool,
    pub code_suggestions: bool,
    pub documentation_generation: bool,
    pub error_analysis: bool,
    pub performance_optimization: bool,
    pub preferred_models: Vec<String>,
    pub custom_prompts: HashMap<String, String>,
}

/// Notebook cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub cell_id: String,
    pub cell_type: CellType,
    pub content: CellContent,
    pub metadata: CellMetadata,
    pub execution_info: Option<ExecutionInfo>,
    pub outputs: Vec<CellOutput>,
    pub ai_annotations: Vec<AIAnnotation>,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellType {
    Code,
    Markdown,
    Raw,
    HTML,
    LaTeX,
    SQL,
    Visualization,
    Interactive,
    AI_Generated,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellContent {
    pub source: String,
    pub language: Option<String>,
    pub syntax_highlighting: bool,
    pub line_numbers: bool,
    pub word_wrap: bool,
    pub auto_indent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMetadata {
    pub tags: Vec<String>,
    pub collapsed: bool,
    pub scrolled: bool,
    pub editable: bool,
    pub deletable: bool,
    pub format: Option<String>,
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub execution_count: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time: Option<std::time::Duration>,
    pub status: ExecutionStatus,
    pub kernel_id: String,
    pub environment_info: EnvironmentInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Queued,
    Running,
    Completed,
    Error,
    Interrupted,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub python_version: Option<String>,
    pub installed_packages: Vec<PackageInfo>,
    pub environment_variables: HashMap<String, String>,
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellOutput {
    pub output_id: String,
    pub output_type: OutputType,
    pub data: OutputData,
    pub metadata: OutputMetadata,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    Stream,
    DisplayData,
    ExecuteResult,
    Error,
    Widget,
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData {
    pub mime_type: String,
    pub content: serde_json::Value,
    pub encoding: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub isolated: bool,
    pub scrolled: bool,
    pub collapsed: bool,
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAnnotation {
    pub annotation_id: String,
    pub annotation_type: AnnotationType,
    pub content: String,
    pub confidence: f64,
    pub source_model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationType {
    Suggestion,
    Explanation,
    Warning,
    Error,
    Optimization,
    Documentation,
    Alternative,
}

/// Notebook session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookSession {
    pub session_id: String,
    pub notebook_id: String,
    pub user_id: String,
    pub kernel_info: KernelInfo,
    pub session_state: SessionState,
    pub variables: HashMap<String, VariableInfo>,
    pub imports: Vec<ImportInfo>,
    pub execution_history: Vec<ExecutionRecord>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelInfo {
    pub kernel_id: String,
    pub kernel_name: String,
    pub language: String,
    pub status: KernelStatus,
    pub connections: u32,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelStatus {
    Starting,
    Idle,
    Busy,
    Terminating,
    Dead,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub current_cell: Option<String>,
    pub execution_count: u32,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub active_threads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub value_type: String,
    pub size: Option<u64>,
    pub shape: Option<Vec<u64>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module_name: String,
    pub alias: Option<String>,
    pub imported_names: Vec<String>,
    pub import_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub cell_id: String,
    pub code: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub output_summary: String,
}

/// Collaboration and version control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationInfo {
    pub collaborators: Vec<Collaborator>,
    pub active_sessions: Vec<String>,
    pub change_log: Vec<ChangeRecord>,
    pub comments: Vec<Comment>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub user_id: String,
    pub username: String,
    pub role: CollaboratorRole,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub cursor_position: Option<CursorPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaboratorRole {
    Owner,
    Editor,
    Reviewer,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub cell_id: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub change_id: String,
    pub user_id: String,
    pub change_type: ChangeType,
    pub target_cell: String,
    pub before_content: String,
    pub after_content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    CellAdded,
    CellDeleted,
    CellModified,
    CellMoved,
    MetadataChanged,
    OutputCleared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: String,
    pub user_id: String,
    pub target_cell: String,
    pub content: String,
    pub thread_id: Option<String>,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub suggestion_id: String,
    pub user_id: String,
    pub target_cell: String,
    pub suggested_content: String,
    pub rationale: String,
    pub status: SuggestionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version_id: String,
    pub version_number: String,
    pub branch_name: String,
    pub commit_message: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub parent_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookEnvironment {
    pub environment_id: String,
    pub name: String,
    pub base_image: String,
    pub packages: Vec<PackageInfo>,
    pub environment_variables: HashMap<String, String>,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory: u64,
    pub max_cpu: f64,
    pub max_execution_time: std::time::Duration,
    pub max_output_size: u64,
}

/// Supporting systems
pub struct NotebookManager;
pub struct CellEngine;
pub struct ExecutionEngine;
pub struct NotebookAIAssistant;
pub struct NotebookCodeGenerator;
pub struct DocumentationGenerator;
pub struct CollaborationEngine;
pub struct NotebookVersionControl;
pub struct SharingManager;
pub struct VisualizationEngine;
pub struct NotebookRenderer;
pub struct ExportEngine;
pub struct PerformanceMonitor;

/// Analytics
#[derive(Debug, Default)]
pub struct NotebookAnalytics {
    pub total_notebooks: u32,
    pub active_notebooks: u32,
    pub total_cells: u32,
    pub total_executions: u32,
    pub average_execution_time: f64,
    pub collaboration_events: u32,
    pub ai_interactions: u32,
    pub notebooks_by_type: HashMap<NotebookType, u32>,
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum NotebookError {
    #[error("Notebook not found: {0}")]
    NotebookNotFound(String),
    #[error("Cell not found: {0}")]
    CellNotFound(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Kernel error: {0}")]
    KernelError(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Collaboration error: {0}")]
    CollaborationError(String),
    #[error("Export failed: {0}")]
    ExportFailed(String),
}

impl UnifiedNotebookSystem {
    /// Create a new Unified Notebook System
    pub fn new() -> Self {
        Self {
            notebook_manager: NotebookManager,
            cell_engine: CellEngine,
            execution_engine: ExecutionEngine,
            ai_assistant: NotebookAIAssistant,
            code_generator: NotebookCodeGenerator,
            documentation_generator: DocumentationGenerator,
            collaboration_engine: CollaborationEngine,
            version_control: NotebookVersionControl,
            sharing_manager: SharingManager,
            visualization_engine: VisualizationEngine,
            renderer: NotebookRenderer,
            export_engine: ExportEngine,
            notebooks: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            performance_monitor: PerformanceMonitor,
            analytics: Arc::new(RwLock::new(NotebookAnalytics::default())),
        }
    }
    
    /// Create a new notebook
    pub async fn create_notebook(
        &self,
        name: String,
        notebook_type: NotebookType,
        author: String,
    ) -> Result<String, NotebookError> {
        let notebook_id = Uuid::new_v4().to_string();
        
        let notebook = Notebook {
            notebook_id: notebook_id.clone(),
            name,
            description: String::new(),
            notebook_type: notebook_type.clone(),
            metadata: NotebookMetadata {
                title: "New Notebook".to_string(),
                author,
                tags: Vec::new(),
                categories: Vec::new(),
                language: "python".to_string(),
                kernel_spec: KernelSpec {
                    name: "python3".to_string(),
                    display_name: "Python 3".to_string(),
                    language: "python".to_string(),
                    version: "3.9".to_string(),
                    executable: "python".to_string(),
                    arguments: Vec::new(),
                    environment_variables: HashMap::new(),
                },
                dependencies: Vec::new(),
                data_sources: Vec::new(),
                output_formats: vec![OutputFormat::HTML],
                sharing_settings: SharingSettings {
                    visibility: Visibility::Private,
                    permissions: Vec::new(),
                    collaboration_mode: CollaborationMode::ReadOnly,
                    export_allowed: true,
                    comment_allowed: true,
                },
                ai_settings: AISettings {
                    ai_assistance_enabled: true,
                    auto_completion: true,
                    code_suggestions: true,
                    documentation_generation: true,
                    error_analysis: true,
                    performance_optimization: false,
                    preferred_models: vec!["gpt-4".to_string()],
                    custom_prompts: HashMap::new(),
                },
            },
            cells: Vec::new(),
            kernel_info: KernelInfo {
                kernel_id: Uuid::new_v4().to_string(),
                kernel_name: "python3".to_string(),
                language: "python".to_string(),
                status: KernelStatus::Idle,
                connections: 0,
                last_activity: Utc::now(),
            },
            environment: NotebookEnvironment {
                environment_id: Uuid::new_v4().to_string(),
                name: "default".to_string(),
                base_image: "python:3.9".to_string(),
                packages: Vec::new(),
                environment_variables: HashMap::new(),
                resource_limits: ResourceLimits {
                    max_memory: 2_000_000_000, // 2GB
                    max_cpu: 2.0,
                    max_execution_time: std::time::Duration::from_secs(300),
                    max_output_size: 10_000_000, // 10MB
                },
            },
            collaboration_info: CollaborationInfo {
                collaborators: Vec::new(),
                active_sessions: Vec::new(),
                change_log: Vec::new(),
                comments: Vec::new(),
                suggestions: Vec::new(),
            },
            version_info: VersionInfo {
                version_id: Uuid::new_v4().to_string(),
                version_number: "1.0.0".to_string(),
                branch_name: "main".to_string(),
                commit_message: "Initial notebook creation".to_string(),
                author: "system".to_string(),
                created_at: Utc::now(),
                parent_versions: Vec::new(),
            },
            created_at: Utc::now(),
            last_modified: Utc::now(),
            last_executed: None,
        };
        
        // Store notebook
        {
            let mut notebooks = self.notebooks.write().await;
            notebooks.insert(notebook_id.clone(), notebook);
        }
        
        // Update analytics
        {
            let mut analytics = self.analytics.write().await;
            analytics.total_notebooks += 1;
            analytics.active_notebooks += 1;
            *analytics.notebooks_by_type.entry(notebook_type).or_insert(0) += 1;
        }
        
        Ok(notebook_id)
    }
    
    /// Add cell to notebook
    pub async fn add_cell(
        &self,
        notebook_id: String,
        cell_type: CellType,
        content: String,
    ) -> Result<String, NotebookError> {
        let cell_id = Uuid::new_v4().to_string();
        
        let cell = NotebookCell {
            cell_id: cell_id.clone(),
            cell_type,
            content: CellContent {
                source: content,
                language: Some("python".to_string()),
                syntax_highlighting: true,
                line_numbers: true,
                word_wrap: false,
                auto_indent: true,
            },
            metadata: CellMetadata {
                tags: Vec::new(),
                collapsed: false,
                scrolled: false,
                editable: true,
                deletable: true,
                format: None,
                custom_metadata: HashMap::new(),
            },
            execution_info: None,
            outputs: Vec::new(),
            ai_annotations: Vec::new(),
            created_at: Utc::now(),
            last_modified: Utc::now(),
        };
        
        // Add cell to notebook
        {
            let mut notebooks = self.notebooks.write().await;
            if let Some(notebook) = notebooks.get_mut(&notebook_id) {
                notebook.cells.push(cell);
                notebook.last_modified = Utc::now();
            } else {
                return Err(NotebookError::NotebookNotFound(notebook_id));
            }
        }
        
        // Update analytics
        {
            let mut analytics = self.analytics.write().await;
            analytics.total_cells += 1;
        }
        
        Ok(cell_id)
    }
    
    /// Execute cell
    pub async fn execute_cell(
        &self,
        notebook_id: String,
        cell_id: String,
    ) -> Result<Vec<CellOutput>, NotebookError> {
        // Mock execution - in real implementation, this would execute code in kernel
        let output = CellOutput {
            output_id: Uuid::new_v4().to_string(),
            output_type: OutputType::ExecuteResult,
            data: OutputData {
                mime_type: "text/plain".to_string(),
                content: serde_json::json!("Execution completed successfully"),
                encoding: None,
                size: 32,
            },
            metadata: OutputMetadata {
                isolated: false,
                scrolled: false,
                collapsed: false,
                custom_metadata: HashMap::new(),
            },
            created_at: Utc::now(),
        };
        
        // Update cell with execution info and outputs
        {
            let mut notebooks = self.notebooks.write().await;
            if let Some(notebook) = notebooks.get_mut(&notebook_id) {
                if let Some(cell) = notebook.cells.iter_mut().find(|c| c.cell_id == cell_id) {
                    cell.execution_info = Some(ExecutionInfo {
                        execution_count: 1,
                        started_at: Utc::now(),
                        completed_at: Some(Utc::now()),
                        execution_time: Some(std::time::Duration::from_millis(100)),
                        status: ExecutionStatus::Completed,
                        kernel_id: notebook.kernel_info.kernel_id.clone(),
                        environment_info: EnvironmentInfo {
                            python_version: Some("3.9.0".to_string()),
                            installed_packages: Vec::new(),
                            environment_variables: HashMap::new(),
                            working_directory: PathBuf::from("/workspace"),
                        },
                    });
                    cell.outputs = vec![output.clone()];
                    cell.last_modified = Utc::now();
                } else {
                    return Err(NotebookError::CellNotFound(cell_id));
                }
                notebook.last_executed = Some(Utc::now());
            } else {
                return Err(NotebookError::NotebookNotFound(notebook_id));
            }
        }
        
        // Update analytics
        {
            let mut analytics = self.analytics.write().await;
            analytics.total_executions += 1;
        }
        
        Ok(vec![output])
    }
    
    /// Get notebook
    pub async fn get_notebook(&self, notebook_id: String) -> Option<Notebook> {
        let notebooks = self.notebooks.read().await;
        notebooks.get(&notebook_id).cloned()
    }
    
    /// List notebooks
    pub async fn list_notebooks(&self) -> Vec<Notebook> {
        let notebooks = self.notebooks.read().await;
        notebooks.values().cloned().collect()
    }
    
    /// Get analytics
    pub async fn get_analytics(&self) -> NotebookAnalytics {
        let analytics = self.analytics.read().await;
        analytics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_notebook_creation() {
        let notebook_system = UnifiedNotebookSystem::new();
        
        let notebook_id = notebook_system
            .create_notebook(
                "Test Notebook".to_string(),
                NotebookType::Research,
                "test_user".to_string(),
            )
            .await
            .unwrap();
        
        assert!(!notebook_id.is_empty());
        
        let notebook = notebook_system.get_notebook(notebook_id).await.unwrap();
        assert_eq!(notebook.name, "Test Notebook");
        assert_eq!(notebook.metadata.author, "test_user");
    }
    
    #[tokio::test]
    async fn test_cell_operations() {
        let notebook_system = UnifiedNotebookSystem::new();
        
        let notebook_id = notebook_system
            .create_notebook(
                "Test Notebook".to_string(),
                NotebookType::Research,
                "test_user".to_string(),
            )
            .await
            .unwrap();
        
        let cell_id = notebook_system
            .add_cell(
                notebook_id.clone(),
                CellType::Code,
                "print('Hello, World!')".to_string(),
            )
            .await
            .unwrap();
        
        assert!(!cell_id.is_empty());
        
        let outputs = notebook_system
            .execute_cell(notebook_id, cell_id)
            .await
            .unwrap();
        
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].output_type, OutputType::ExecuteResult);
    }
}
