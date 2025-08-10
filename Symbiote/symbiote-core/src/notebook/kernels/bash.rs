//! # Bash Kernel Implementation
//! 
//! Bash/Shell kernel for system commands.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Bash kernel implementation
#[derive(Debug)]
pub struct BashKernel {
    variables: Arc<RwLock<HashMap<String, Variable>>>,
    execution_count: Arc<RwLock<u32>>,
    status: Arc<RwLock<KernelStatus>>,
}

impl BashKernel {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            execution_count: Arc::new(RwLock::new(0)),
            status: Arc::new(RwLock::new(KernelStatus::Idle)),
        }
    }
}

#[async_trait]
impl NotebookKernel for BashKernel {
    fn language(&self) -> &str {
        "bash"
    }

    fn version(&self) -> &str {
        "5.0+"
    }

    fn display_name(&self) -> &str {
        "Bash"
    }

    async fn execute_code(&self, code: &str, context: &ExecutionContext) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut outputs = Vec::new();

        // Mock bash execution
        if code.contains("echo") {
            outputs.push(CellOutput::stream(StreamType::Stdout, "Hello from Bash!".to_string()));
        }

        if code.contains("ls") {
            outputs.push(CellOutput::stream(StreamType::Stdout, "file1.txt  file2.txt  directory/".to_string()));
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
            implementation: "symbiote-bash".to_string(),
            implementation_version: "1.0.0".to_string(),
            language_info: LanguageInfo {
                name: "bash".to_string(),
                version: "5.0+".to_string(),
                mimetype: "text/x-sh".to_string(),
                file_extension: ".sh".to_string(),
                pygments_lexer: Some("bash".to_string()),
                codemirror_mode: Some("shell".to_string()),
            },
            banner: "Bash kernel for Symbiote IDE".to_string(),
            help_links: vec![
                HelpLink {
                    text: "Bash Manual".to_string(),
                    url: "https://www.gnu.org/software/bash/manual/".to_string(),
                },
            ],
            status: KernelStatus::Idle,
        }
    }

    async fn complete_code(&self, _code: &str, cursor_pos: usize) -> Result<CompletionResult> {
        Ok(CompletionResult {
            matches: vec!["echo".to_string(), "ls".to_string(), "cd".to_string()],
            cursor_start: cursor_pos.saturating_sub(3),
            cursor_end: cursor_pos,
            metadata: HashMap::new(),
        })
    }

    async fn inspect_code(&self, _code: &str, _cursor_pos: usize) -> Result<InspectionResult> {
        let mut data = OutputData::new();
        data.add_text("Bash documentation".to_string());

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
