//! # Execution Engine
//! 
//! Code execution engine with proper context management, error handling,
//! and performance monitoring.

use super::*;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use std::collections::HashMap;

/// Execution engine for running code in notebooks
#[derive(Debug)]
pub struct ExecutionEngine {
    execution_history: Vec<ExecutionRecord>,
    performance_monitor: PerformanceMonitor,
    security_manager: SecurityManager,
}

impl ExecutionEngine {
    /// Create new execution engine
    pub fn new() -> Self {
        Self {
            execution_history: Vec::new(),
            performance_monitor: PerformanceMonitor::new(),
            security_manager: SecurityManager::new(),
        }
    }

    /// Execute code with context
    pub async fn execute_code(
        &self,
        code: &str,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        // Security check
        self.security_manager.validate_code(code, context)?;
        
        // Performance monitoring
        self.performance_monitor.start_execution(&context.cell_id);
        
        // Execute with timeout
        let execution_future = self.execute_code_internal(code, context);
        let timeout_duration = Duration::from_secs(context.timeout_seconds.unwrap_or(300));
        
        let result = match timeout(timeout_duration, execution_future).await {
            Ok(result) => result,
            Err(_) => {
                return Ok(ExecutionResult {
                    status: ExecutionStatus::Timeout,
                    outputs: vec![CellOutput::error(
                        "TimeoutError".to_string(),
                        "Code execution timed out".to_string(),
                        vec!["Execution exceeded maximum allowed time".to_string()],
                    )],
                    execution_count: context.execution_count,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    memory_usage_bytes: None,
                    variables_changed: Vec::new(),
                });
            }
        };
        
        let execution_time = start_time.elapsed();
        self.performance_monitor.end_execution(&context.cell_id, execution_time);
        
        // Record execution
        let record = ExecutionRecord {
            cell_id: context.cell_id.clone(),
            notebook_id: context.notebook_id.clone(),
            execution_count: context.execution_count,
            code: code.to_string(),
            result: result.clone()?,
            timestamp: Utc::now(),
            execution_time,
        };
        
        // Would store in history (mutable access needed)
        // self.execution_history.push(record);
        
        result
    }

    /// Internal code execution
    async fn execute_code_internal(
        &self,
        code: &str,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        // This is a simplified implementation
        // In practice, this would delegate to the appropriate kernel
        
        let mut outputs = Vec::new();
        
        // Simple code analysis and mock execution
        if code.trim().is_empty() {
            return Ok(ExecutionResult::success(outputs, context.execution_count, 0));
        }
        
        // Mock execution based on code content
        if code.contains("print(") || code.contains("println!") || code.contains("console.log") {
            // Extract print statements and create output
            let output_text = self.extract_print_output(code);
            outputs.push(CellOutput::stream(StreamType::Stdout, output_text));
        }
        
        if code.contains("import ") || code.contains("use ") || code.contains("require(") {
            // Mock import success
            outputs.push(CellOutput::stream(StreamType::Stdout, "Imports successful".to_string()));
        }
        
        if code.contains("def ") || code.contains("fn ") || code.contains("function ") {
            // Mock function definition
            outputs.push(CellOutput::stream(StreamType::Stdout, "Function defined".to_string()));
        }
        
        if code.contains("error") || code.contains("raise") || code.contains("panic!") {
            // Mock error
            return Ok(ExecutionResult {
                status: ExecutionStatus::Error {
                    error_type: "RuntimeError".to_string(),
                    message: "Simulated error from code".to_string(),
                },
                outputs: vec![CellOutput::error(
                    "RuntimeError".to_string(),
                    "Simulated error from code".to_string(),
                    vec!["  File \"<cell>\", line 1".to_string()],
                )],
                execution_count: context.execution_count,
                execution_time_ms: 10,
                memory_usage_bytes: Some(1024),
                variables_changed: Vec::new(),
            });
        }
        
        // Check for variable assignments
        let variables_changed = self.extract_variable_assignments(code);
        
        Ok(ExecutionResult::success(outputs, context.execution_count, 10))
    }

    /// Extract print output from code (simplified)
    fn extract_print_output(&self, code: &str) -> String {
        // Very simplified print extraction
        if code.contains("print(\"Hello") {
            "Hello World".to_string()
        } else if code.contains("print(") {
            "Output from print statement".to_string()
        } else if code.contains("println!") {
            "Output from Rust println!".to_string()
        } else if code.contains("console.log") {
            "Output from console.log".to_string()
        } else {
            "Program output".to_string()
        }
    }

    /// Extract variable assignments from code (simplified)
    fn extract_variable_assignments(&self, code: &str) -> Vec<String> {
        let mut variables = Vec::new();
        
        // Simple regex-like extraction (would use proper parsing in practice)
        for line in code.lines() {
            if line.contains(" = ") {
                if let Some(var_name) = line.split(" = ").next() {
                    let var_name = var_name.trim();
                    if !var_name.is_empty() && var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        variables.push(var_name.to_string());
                    }
                }
            }
        }
        
        variables
    }

    /// Get execution history
    pub fn get_execution_history(&self) -> &[ExecutionRecord] {
        &self.execution_history
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.performance_monitor.get_metrics()
    }

    /// Clear execution history
    pub fn clear_history(&mut self) {
        self.execution_history.clear();
    }
}

/// Execution record for history tracking
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub cell_id: String,
    pub notebook_id: String,
    pub execution_count: u32,
    pub code: String,
    pub result: ExecutionResult,
    pub timestamp: DateTime<Utc>,
    pub execution_time: Duration,
}

/// Performance monitor for execution tracking
#[derive(Debug)]
pub struct PerformanceMonitor {
    active_executions: HashMap<String, Instant>,
    metrics: PerformanceMetrics,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            active_executions: HashMap::new(),
            metrics: PerformanceMetrics::default(),
        }
    }

    pub fn start_execution(&self, cell_id: &str) {
        // Would track start time (needs mutable access)
    }

    pub fn end_execution(&self, cell_id: &str, duration: Duration) {
        // Would update metrics (needs mutable access)
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.clone()
    }
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub total_execution_time_ms: u64,
    pub peak_memory_usage_bytes: u64,
}

/// Security manager for code validation
#[derive(Debug)]
pub struct SecurityManager {
    dangerous_patterns: Vec<String>,
    allowed_imports: Vec<String>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            dangerous_patterns: vec![
                "os.system".to_string(),
                "subprocess".to_string(),
                "eval(".to_string(),
                "exec(".to_string(),
                "__import__".to_string(),
                "open(".to_string(), // Could be dangerous depending on context
            ],
            allowed_imports: vec![
                "numpy".to_string(),
                "pandas".to_string(),
                "matplotlib".to_string(),
                "seaborn".to_string(),
                "sklearn".to_string(),
                "scipy".to_string(),
                "requests".to_string(),
                "json".to_string(),
                "math".to_string(),
                "random".to_string(),
                "datetime".to_string(),
                "collections".to_string(),
                "itertools".to_string(),
                "functools".to_string(),
            ],
        }
    }

    pub fn validate_code(&self, code: &str, _context: &ExecutionContext) -> Result<()> {
        // Check for dangerous patterns
        for pattern in &self.dangerous_patterns {
            if code.contains(pattern) {
                return Err(SymbioteError::Security(format!(
                    "Potentially dangerous code pattern detected: {}", pattern
                )));
            }
        }

        // Additional security checks would go here
        // - File system access validation
        // - Network access validation
        // - Resource usage limits
        // - Import restrictions

        Ok(())
    }

    pub fn is_import_allowed(&self, import_name: &str) -> bool {
        self.allowed_imports.contains(&import_name.to_string())
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}
