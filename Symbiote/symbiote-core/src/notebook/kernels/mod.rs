//! # Notebook Kernels
//! 
//! Language-specific kernel implementations for multi-language notebook support.

pub mod python;
pub mod rust;
pub mod javascript;
pub mod sql;
pub mod bash;

// Re-export kernel implementations
pub use python::*;
pub use rust::*;
pub use javascript::*;
pub use sql::*;
pub use bash::*;

use super::*;

/// Create all available kernels
pub fn create_default_kernels() -> Vec<(String, Box<dyn NotebookKernel>)> {
    vec![
        ("python".to_string(), Box::new(PythonKernel::new())),
        ("rust".to_string(), Box::new(RustKernel::new())),
        ("javascript".to_string(), Box::new(JavaScriptKernel::new())),
        ("sql".to_string(), Box::new(SqlKernel::new())),
        ("bash".to_string(), Box::new(BashKernel::new())),
    ]
}

/// Get kernel by language name
pub fn get_kernel_for_language(language: &str) -> Option<Box<dyn NotebookKernel>> {
    match language.to_lowercase().as_str() {
        "python" | "py" => Some(Box::new(PythonKernel::new())),
        "rust" | "rs" => Some(Box::new(RustKernel::new())),
        "javascript" | "js" | "node" => Some(Box::new(JavaScriptKernel::new())),
        "sql" | "sqlite" | "postgres" | "mysql" => Some(Box::new(SqlKernel::new())),
        "bash" | "sh" | "shell" => Some(Box::new(BashKernel::new())),
        _ => None,
    }
}

/// Check if language is supported
pub fn is_language_supported(language: &str) -> bool {
    get_kernel_for_language(language).is_some()
}

/// Get all supported languages
pub fn get_supported_languages() -> Vec<String> {
    vec![
        "python".to_string(),
        "rust".to_string(),
        "javascript".to_string(),
        "sql".to_string(),
        "bash".to_string(),
    ]
}
