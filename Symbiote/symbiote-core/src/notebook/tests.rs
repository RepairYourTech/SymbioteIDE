//! # Notebook System Tests
//! 
//! Comprehensive tests for the notebook system implementation.

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_notebook_creation() {
        let kernel_spec = KernelSpec {
            name: "python".to_string(),
            display_name: "Python 3".to_string(),
            language: "python".to_string(),
            version: "3.9".to_string(),
            executable: "python".to_string(),
            args: vec!["-u".to_string()],
            env: HashMap::new(),
        };

        let notebook = Notebook::new("Test Notebook".to_string(), kernel_spec);
        
        assert_eq!(notebook.name, "Test Notebook");
        assert_eq!(notebook.kernel_spec.language, "python");
        assert!(notebook.cells.is_empty());
        assert_eq!(notebook.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_cell_operations() {
        let mut notebook = create_test_notebook();
        
        // Add code cell
        let code_cell = NotebookCell::new_code("python".to_string(), "print('Hello World')".to_string());
        let cell_id = code_cell.id.clone();
        notebook.add_cell(code_cell);
        
        assert_eq!(notebook.cells.len(), 1);
        assert!(notebook.get_cell(&cell_id).is_some());
        
        // Add markdown cell
        let markdown_cell = NotebookCell::new_markdown("# Test Notebook".to_string());
        notebook.add_cell(markdown_cell);
        
        assert_eq!(notebook.cells.len(), 2);
        
        // Remove cell
        let removed = notebook.remove_cell(&cell_id);
        assert!(removed.is_some());
        assert_eq!(notebook.cells.len(), 1);
    }

    #[tokio::test]
    async fn test_notebook_system() {
        let mut system = NotebookSystem::new();
        
        // Register Python kernel
        let python_kernel = Box::new(PythonKernel::new());
        system.register_kernel("python".to_string(), python_kernel).await.unwrap();
        
        // Check available kernels
        let kernels = system.get_available_kernels().await;
        assert!(kernels.contains(&"python".to_string()));
        
        // Create notebook
        let notebook_id = system.create_notebook("Test".to_string(), "python".to_string()).await.unwrap();
        assert!(!notebook_id.is_empty());
        
        // Open notebook
        let notebook = system.open_notebook(&notebook_id).await.unwrap();
        assert_eq!(notebook.name, "Test");
    }

    #[tokio::test]
    async fn test_python_kernel() {
        let kernel = PythonKernel::new();
        
        assert_eq!(kernel.language(), "python");
        assert_eq!(kernel.display_name(), "Python 3");
        
        let context = ExecutionContext::new(1, "cell1".to_string(), "notebook1".to_string());
        
        // Test simple print execution
        let result = kernel.execute_code("print('Hello')", &context).await.unwrap();
        assert!(result.is_success());
        assert!(!result.outputs.is_empty());
        
        // Test error handling
        let error_result = kernel.execute_code("raise Exception('test')", &context).await.unwrap();
        assert!(!error_result.is_success());
    }

    #[tokio::test]
    async fn test_execution_engine() {
        let engine = ExecutionEngine::new();
        let context = ExecutionContext::new(1, "cell1".to_string(), "notebook1".to_string());
        
        // Test successful execution
        let result = engine.execute_code("print('test')", &context).await.unwrap();
        assert!(result.is_success());
        
        // Test error execution
        let error_result = engine.execute_code("error", &context).await.unwrap();
        assert!(!error_result.is_success());
    }

    #[tokio::test]
    async fn test_variable_bridge() {
        let bridge = VariableBridge::new();
        
        // Test variable conversion
        let python_var = Variable::string("test_var".to_string(), "hello".to_string());
        let rust_var = bridge.convert_variable(&python_var, "python", "rust").unwrap();
        
        assert_eq!(rust_var.var_type, "String");
        
        // Test serialization
        let serialized = bridge.serialize_variable(&python_var, "json").unwrap();
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized = bridge.deserialize_variable(&serialized, "json").unwrap();
        assert_eq!(deserialized.name, python_var.name);
    }

    #[tokio::test]
    async fn test_output_renderer() {
        let renderer = OutputRenderer::new();
        
        // Create test output
        let mut output_data = OutputData::new();
        output_data.add_text("Hello World".to_string());
        output_data.add_html("<b>Hello World</b>".to_string());
        
        let output = CellOutput::display_data(output_data);
        
        // Render output
        let rendered = renderer.render(&output).await.unwrap();
        assert!(rendered.has_mime_type("text/plain"));
        assert!(rendered.has_mime_type("text/html"));
        
        // Test preferred MIME type
        let mime_types = vec!["text/plain".to_string(), "text/html".to_string()];
        let preferred = renderer.get_preferred_mime_type(&mime_types);
        assert_eq!(preferred, Some("text/html".to_string()));
    }

    #[tokio::test]
    async fn test_collaboration() {
        let mut collab = NotebookCollaboration::new();
        
        // Start session
        let session_id = collab.start_session("notebook1", "user1").await.unwrap();
        assert!(!session_id.is_empty());
        
        // Stop session
        collab.stop_session(&session_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_cell_outputs() {
        // Test stream output
        let stream_output = CellOutput::stream(StreamType::Stdout, "Hello".to_string());
        assert!(matches!(stream_output.output_type, OutputType::Stream { .. }));
        
        // Test error output
        let error_output = CellOutput::error(
            "ValueError".to_string(),
            "Invalid value".to_string(),
            vec!["Traceback line 1".to_string()],
        );
        assert!(matches!(error_output.output_type, OutputType::Error));
        
        // Test execute result
        let mut data = OutputData::new();
        data.add_text("42".to_string());
        let result_output = CellOutput::execute_result(data, 1);
        assert!(matches!(result_output.output_type, OutputType::ExecuteResult));
    }

    #[tokio::test]
    async fn test_kernel_capabilities() {
        let python_kernel = PythonKernel::new();
        let capabilities = python_kernel.get_capabilities();
        
        assert!(capabilities.supports_completion);
        assert!(capabilities.supports_inspection);
        assert!(capabilities.supports_rich_output);
        assert!(capabilities.supports_variable_inspection);
    }

    #[tokio::test]
    async fn test_notebook_statistics() {
        let mut notebook = create_test_notebook();
        
        // Add cells
        let code_cell1 = NotebookCell::new_code("python".to_string(), "x = 1".to_string());
        let code_cell2 = NotebookCell::new_code("python".to_string(), "y = 2".to_string());
        let markdown_cell = NotebookCell::new_markdown("# Header".to_string());
        
        notebook.add_cell(code_cell1);
        notebook.add_cell(code_cell2);
        notebook.add_cell(markdown_cell);
        
        let stats = notebook.get_execution_stats();
        assert_eq!(stats.total_cells, 2); // Only code cells
        assert_eq!(stats.executed_cells, 0); // None executed yet
    }

    #[tokio::test]
    async fn test_multiple_kernels() {
        let kernels = create_default_kernels();
        assert_eq!(kernels.len(), 5);
        
        let languages = get_supported_languages();
        assert!(languages.contains(&"python".to_string()));
        assert!(languages.contains(&"rust".to_string()));
        assert!(languages.contains(&"javascript".to_string()));
        
        assert!(is_language_supported("python"));
        assert!(is_language_supported("rust"));
        assert!(!is_language_supported("unknown"));
    }

    // Helper functions
    fn create_test_notebook() -> Notebook {
        let kernel_spec = KernelSpec {
            name: "python".to_string(),
            display_name: "Python 3".to_string(),
            language: "python".to_string(),
            version: "3.9".to_string(),
            executable: "python".to_string(),
            args: vec![],
            env: HashMap::new(),
        };
        
        Notebook::new("Test Notebook".to_string(), kernel_spec)
    }
}
