//! # JavaScript Kernel Implementation
//! 
//! JavaScript/Node.js kernel implementation.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// JavaScript kernel implementation
#[derive(Debug)]
pub struct JavaScriptKernel {
    variables: Arc<RwLock<HashMap<String, Variable>>>,
    execution_count: Arc<RwLock<u32>>,
    status: Arc<RwLock<KernelStatus>>,
}

impl JavaScriptKernel {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            execution_count: Arc::new(RwLock::new(0)),
            status: Arc::new(RwLock::new(KernelStatus::Idle)),
        }
    }
}

#[async_trait]
impl NotebookKernel for JavaScriptKernel {
    fn language(&self) -> &str {
        "javascript"
    }

    fn version(&self) -> &str {
        "Node.js 18+"
    }

    fn display_name(&self) -> &str {
        "JavaScript (Node.js)"
    }

    async fn execute_code(&self, code: &str, context: &ExecutionContext) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut outputs = Vec::new();

        // Mock JavaScript execution
        if code.contains("console.log") {
            outputs.push(CellOutput::stream(StreamType::Stdout, "Hello from JavaScript!".to_string()));
        }

        if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            outputs.push(CellOutput::stream(StreamType::Stdout, "JavaScript code executed".to_string()));
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
            implementation: "symbiote-javascript".to_string(),
            implementation_version: "1.0.0".to_string(),
            language_info: LanguageInfo {
                name: "javascript".to_string(),
                version: "ES2022".to_string(),
                mimetype: "application/javascript".to_string(),
                file_extension: ".js".to_string(),
                pygments_lexer: Some("javascript".to_string()),
                codemirror_mode: Some("javascript".to_string()),
            },
            banner: "JavaScript (Node.js) kernel for Symbiote IDE".to_string(),
            help_links: vec![
                HelpLink {
                    text: "MDN JavaScript".to_string(),
                    url: "https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string(),
                },
            ],
            status: KernelStatus::Idle,
        }
    }

    async fn complete_code(&self, _code: &str, cursor_pos: usize) -> Result<CompletionResult> {
        Ok(CompletionResult {
            matches: vec!["console.log".to_string(), "function".to_string(), "const".to_string()],
            cursor_start: cursor_pos.saturating_sub(3),
            cursor_end: cursor_pos,
            metadata: HashMap::new(),
        })
    }

    async fn inspect_code(&self, _code: &str, _cursor_pos: usize) -> Result<InspectionResult> {
        let mut data = OutputData::new();
        data.add_text("JavaScript documentation".to_string());

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
