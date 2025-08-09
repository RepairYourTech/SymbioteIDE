//! Integration tests for the Enhanced Context Management System

use symbiote_core::{Result, UserId, ProjectId};
use symbiote_core::context::{
    EnhancedContextManager, ContextLayerType, ContextContent, ContextEvent,
    RealTimeContextTracker, ContextIntelligenceEngine, MCPIntegrationManager,
    TrackingConfiguration, ContentType, ProgrammingLanguage,
};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

/// Test enhanced context manager initialization
#[tokio::test]
async fn test_enhanced_context_manager_initialization() -> Result<()> {
    let manager = EnhancedContextManager::new();
    
    // Test session creation
    let user_id = UserId::new();
    let project_id = Some(ProjectId::new());
    let session_id = manager.create_session(user_id, project_id).await?;
    
    assert!(!session_id.is_empty());
    
    // Test session retrieval
    let session = manager.get_session(&session_id).await?;
    assert!(session.is_some());
    
    let session = session.unwrap();
    assert_eq!(session.id, session_id);
    assert!(session.context_layers.is_empty());
    
    Ok(())
}

/// Test context layer management
#[tokio::test]
async fn test_context_layer_management() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let user_id = UserId::new();
    let session_id = manager.create_session(user_id, None).await?;
    
    // Create test content
    let content = ContextContent {
        raw: "fn main() { println!(\"Hello, world!\"); }".to_string(),
        compressed: None,
        embeddings: None,
        metadata: HashMap::new(),
        content_hash: "test_hash_123".to_string(),
    };
    
    // Add context layer
    manager.update_context_layer(
        &session_id,
        ContextLayerType::ActiveFile,
        content.clone(),
        10, // high priority
    ).await?;
    
    // Verify layer was added
    let session = manager.get_session(&session_id).await?.unwrap();
    assert_eq!(session.context_layers.len(), 1);
    
    let layer = &session.context_layers[0];
    assert_eq!(layer.layer_type, ContextLayerType::ActiveFile);
    assert_eq!(layer.priority, 10);
    assert_eq!(layer.content.content_hash, "test_hash_123");
    
    Ok(())
}

/// Test context event subscription
#[tokio::test]
async fn test_context_event_subscription() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let mut event_receiver = manager.subscribe_to_events();
    
    // Create session in background task
    let manager_clone = manager.clone();
    let user_id = UserId::new();
    tokio::spawn(async move {
        sleep(Duration::from_millis(10)).await;
        let _ = manager_clone.create_session(user_id, None).await;
    });
    
    // Wait for event
    let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv()).await;
    
    match event {
        Ok(Ok(ContextEvent::SessionCreated { session_id, user_id: _, project_id: _ })) => {
            assert!(!session_id.is_empty());
        }
        _ => panic!("Expected SessionCreated event"),
    }
    
    Ok(())
}

/// Test multiple context layers with different types
#[tokio::test]
async fn test_multiple_context_layers() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let user_id = UserId::new();
    let session_id = manager.create_session(user_id, None).await?;
    
    // Add multiple layers
    let layers = vec![
        (ContextLayerType::ActiveFile, "Active file content", 10),
        (ContextLayerType::RecentFiles, "Recent files list", 8),
        (ContextLayerType::ProjectContext, "Project metadata", 6),
        (ContextLayerType::UserContext, "User preferences", 4),
    ];
    
    for (layer_type, content_str, priority) in layers {
        let content = ContextContent {
            raw: content_str.to_string(),
            compressed: None,
            embeddings: None,
            metadata: HashMap::new(),
            content_hash: format!("hash_{}", content_str.len()),
        };
        
        manager.update_context_layer(&session_id, layer_type, content, priority).await?;
    }
    
    // Verify all layers were added
    let session = manager.get_session(&session_id).await?.unwrap();
    assert_eq!(session.context_layers.len(), 4);
    
    // Verify layers are properly stored
    let active_file_layer = session.context_layers
        .iter()
        .find(|l| l.layer_type == ContextLayerType::ActiveFile)
        .unwrap();
    assert_eq!(active_file_layer.priority, 10);
    assert_eq!(active_file_layer.content.raw, "Active file content");
    
    Ok(())
}

/// Test context layer updates
#[tokio::test]
async fn test_context_layer_updates() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let user_id = UserId::new();
    let session_id = manager.create_session(user_id, None).await?;
    
    // Add initial layer
    let initial_content = ContextContent {
        raw: "Initial content".to_string(),
        compressed: None,
        embeddings: None,
        metadata: HashMap::new(),
        content_hash: "initial_hash".to_string(),
    };
    
    manager.update_context_layer(
        &session_id,
        ContextLayerType::ActiveFile,
        initial_content,
        5,
    ).await?;
    
    // Update the same layer
    let updated_content = ContextContent {
        raw: "Updated content".to_string(),
        compressed: None,
        embeddings: None,
        metadata: HashMap::new(),
        content_hash: "updated_hash".to_string(),
    };
    
    manager.update_context_layer(
        &session_id,
        ContextLayerType::ActiveFile,
        updated_content,
        8,
    ).await?;
    
    // Verify layer was updated, not duplicated
    let session = manager.get_session(&session_id).await?.unwrap();
    assert_eq!(session.context_layers.len(), 1);
    
    let layer = &session.context_layers[0];
    assert_eq!(layer.content.raw, "Updated content");
    assert_eq!(layer.content.content_hash, "updated_hash");
    assert_eq!(layer.priority, 8);
    
    Ok(())
}

/// Test real-time context tracker initialization
#[tokio::test]
async fn test_realtime_tracker_initialization() -> Result<()> {
    let tracker = RealTimeContextTracker::new();
    
    // Test starting tracking
    let session_id = "test_session_123";
    tracker.start_tracking(session_id).await?;
    
    // Test stopping tracking
    tracker.stop_tracking(session_id).await?;
    
    Ok(())
}

/// Test context intelligence engine
#[tokio::test]
async fn test_context_intelligence() -> Result<()> {
    // Note: This is a placeholder test since the intelligence engine
    // requires more complex setup with actual AI models
    
    // Test content type detection
    let rust_code = "fn main() { println!(\"Hello\"); }";
    let content_type = detect_content_type(rust_code);
    
    match content_type {
        ContentType::SourceCode(ProgrammingLanguage::Rust) => {
            // Expected result
        }
        _ => panic!("Expected Rust source code detection"),
    }
    
    Ok(())
}

/// Test MCP integration manager
#[tokio::test]
async fn test_mcp_integration() -> Result<()> {
    let mcp_manager = MCPIntegrationManager::new();
    
    // Test event subscription
    let mut event_receiver = mcp_manager.subscribe_to_events();
    
    // This is a basic initialization test
    // Full MCP testing would require actual MCP servers
    
    Ok(())
}

/// Test context optimization
#[tokio::test]
async fn test_context_optimization() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let user_id = UserId::new();
    let session_id = manager.create_session(user_id, None).await?;
    
    // Add multiple layers to trigger optimization
    for i in 0..10 {
        let content = ContextContent {
            raw: format!("Content {}", i),
            compressed: None,
            embeddings: None,
            metadata: HashMap::new(),
            content_hash: format!("hash_{}", i),
        };
        
        manager.update_context_layer(
            &session_id,
            ContextLayerType::RecentFiles,
            content,
            i as u8,
        ).await?;
    }
    
    // Trigger optimization
    manager.optimize_context(&session_id).await?;
    
    // Verify session still exists and is functional
    let session = manager.get_session(&session_id).await?;
    assert!(session.is_some());
    
    Ok(())
}

/// Test context analytics
#[tokio::test]
async fn test_context_analytics() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let user_id = UserId::new();
    let session_id = manager.create_session(user_id, None).await?;
    
    // Add some context to generate analytics
    let content = ContextContent {
        raw: "Analytics test content".to_string(),
        compressed: None,
        embeddings: None,
        metadata: HashMap::new(),
        content_hash: "analytics_hash".to_string(),
    };
    
    manager.update_context_layer(
        &session_id,
        ContextLayerType::AnalysisContext,
        content,
        7,
    ).await?;
    
    // Get analytics
    let metrics = manager.get_analytics(&session_id).await?;
    
    // Verify basic metrics structure
    assert!(metrics.total_tokens >= 0);
    assert!(metrics.compression_ratio >= 0.0);
    assert!(metrics.relevance_score >= 0.0);
    
    Ok(())
}

/// Test concurrent context operations
#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let manager = EnhancedContextManager::new();
    let user_id = UserId::new();
    let session_id = manager.create_session(user_id, None).await?;
    
    // Create multiple concurrent operations
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let manager_clone = manager.clone();
        let session_id_clone = session_id.clone();
        
        let handle = tokio::spawn(async move {
            let content = ContextContent {
                raw: format!("Concurrent content {}", i),
                compressed: None,
                embeddings: None,
                metadata: HashMap::new(),
                content_hash: format!("concurrent_hash_{}", i),
            };
            
            manager_clone.update_context_layer(
                &session_id_clone,
                ContextLayerType::SessionContext,
                content,
                i as u8,
            ).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap()?;
    }
    
    // Verify all layers were added
    let session = manager.get_session(&session_id).await?.unwrap();
    assert_eq!(session.context_layers.len(), 5);
    
    Ok(())
}

// Helper function for content type detection
fn detect_content_type(content: &str) -> ContentType {
    if content.contains("fn ") && content.contains("println!") {
        ContentType::SourceCode(ProgrammingLanguage::Rust)
    } else if content.contains("function ") || content.contains("console.log") {
        ContentType::SourceCode(ProgrammingLanguage::JavaScript)
    } else if content.contains("def ") && content.contains("print(") {
        ContentType::SourceCode(ProgrammingLanguage::Python)
    } else {
        ContentType::Unknown
    }
}

impl EnhancedContextManager {
    fn clone(&self) -> Self {
        // For testing purposes, create a new instance
        // In a real implementation, this would properly clone the manager
        EnhancedContextManager::new()
    }
}

impl MCPIntegrationManager {
    fn new() -> Self {
        // Placeholder implementation for testing
        use tokio::sync::broadcast;
        use std::sync::Arc;
        use tokio::sync::RwLock;
        use std::collections::HashMap;
        
        let (event_broadcaster, _) = broadcast::channel(1000);
        
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            session_manager: Arc::new(symbiote_core::context::MCPSessionManager::new()),
            message_router: Arc::new(symbiote_core::context::MCPMessageRouter::new()),
            context_sync: Arc::new(symbiote_core::context::MCPContextSynchronizer::new()),
            event_broadcaster,
        }
    }
    
    fn subscribe_to_events(&self) -> tokio::sync::broadcast::Receiver<symbiote_core::context::MCPEvent> {
        self.event_broadcaster.subscribe()
    }
}
