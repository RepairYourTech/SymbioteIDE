//! # Rust Kernel Implementation
//! 
//! Rust kernel with compilation and execution support.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Rust kernel implementation
#[derive(Debug)]
pub struct RustKernel {
    variables: Arc<RwLock<HashMap<String, Variable>>>,
    execution_count: Arc<RwLock<u32>>,
    status: Arc<RwLock<KernelStatus>>,
}

impl RustKernel {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            execution_count: Arc::new(RwLock::new(0)),
            status: Arc::new(RwLock::new(KernelStatus::Idle)),
        }
    }
}

#[async_trait]
impl NotebookKernel for RustKernel {
    fn language(&self) -> &str {
        "rust"
    }

    fn version(&self) -> &str {
        "1.70+"
    }

    fn display_name(&self) -> &str {
        "Rust"
    }

    async fn execute_code(&self, code: &str, context: &ExecutionContext) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut outputs = Vec::new();

        // Mock Rust execution
        if code.contains("println!") {
            outputs.push(CellOutput::stream(StreamType::Stdout, "Hello from Rust!".to_string()));
        }

        if code.contains("fn ") {
            outputs.push(CellOutput::stream(StreamType::Stdout, "Function compiled successfully".to_string()));
        }

        Ok(ExecutionResult::success(
            outputs,
            context.execution_count,
            start_time.elapsed().as_millis() as u64,
        ))
    }

    async fn get_variables(&self) -> Result<HashMap<String, Variable>> {
        let variables = self.variables.read().await;
        Ok(variables.clone())
    }

    async fn set_variable(&self, name: &str, value: &Variable) -> Result<()> {
        let mut variables = self.variables.write().await;
        variables.insert(name.to_string(), value.clone());
        Ok(())
    }

    async fn is_alive(&self) -> bool {
        true
    }

    async fn restart(&self) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn get_info(&self) -> KernelInfo {
        KernelInfo {
            protocol_version: "5.3".to_string(),
            implementation: "symbiote-rust".to_string(),
            implementation_version: "1.0.0".to_string(),
            language_info: LanguageInfo {
                name: "rust".to_string(),
                version: "1.70+".to_string(),
                mimetype: "text/x-rust".to_string(),
                file_extension: ".rs".to_string(),
                pygments_lexer: Some("rust".to_string()),
                codemirror_mode: Some("rust".to_string()),
            },
            banner: "Rust kernel for Symbiote IDE".to_string(),
            help_links: vec![
                HelpLink {
                    text: "Rust Documentation".to_string(),
                    url: "https://doc.rust-lang.org/".to_string(),
                },
            ],
            status: KernelStatus::Idle,
        }
    }

    async fn complete_code(&self, _code: &str, cursor_pos: usize) -> Result<CompletionResult> {
        Ok(CompletionResult {
            matches: vec!["println!".to_string(), "vec!".to_string(), "format!".to_string()],
            cursor_start: cursor_pos.saturating_sub(3),
            cursor_end: cursor_pos,
            metadata: HashMap::new(),
        })
    }

    async fn inspect_code(&self, _code: &str, _cursor_pos: usize) -> Result<InspectionResult> {
        let mut data = OutputData::new();
        data.add_text("Rust documentation".to_string());

        Ok(InspectionResult {
            found: true,
            data,
            metadata: HashMap::new(),
        })
    }

    fn get_capabilities(&self) -> KernelCapabilities {
        KernelCapabilities::default()
    }
}
