//! Symbiote IDE Main Application
//!
//! This is the main entry point for the Symbiote IDE application.
//! It initializes the IDE engine, starts the web server, and handles graceful shutdown.

use symbiote_ide::{IdeEngine, IdeServer, IdeConfig};
use symbiote_core::{initialize, shutdown, SymbioteResult};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Symbiote core
    initialize().await?;
    
    // Load configuration
    let config = load_config().await?;
    
    // Initialize IDE engine
    let ide_engine = Arc::new(IdeEngine::new(config.clone()).await?);
    
    // Start IDE engine
    ide_engine.start().await?;
    
    // Create and start web server
    let server = IdeServer::new(ide_engine.clone(), config.clone()).await?;
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            error!("Server error: {}", e);
        }
    });
    
    info!("Symbiote IDE started successfully");
    info!("Web interface available at: http://localhost:{}", config.server.port);
    
    // Wait for shutdown signal
    wait_for_shutdown().await;
    
    info!("Shutting down Symbiote IDE...");
    
    // Stop server
    server_handle.abort();
    
    // Stop IDE engine
    ide_engine.stop().await?;
    
    // Shutdown Symbiote core
    shutdown().await?;
    
    info!("Symbiote IDE shutdown complete");
    Ok(())
}

/// Load configuration from file or environment
async fn load_config() -> SymbioteResult<IdeConfig> {
    // Try to load from config file first
    if let Ok(config) = IdeConfig::from_file("config/ide.toml").await {
        return Ok(config);
    }
    
    // Fall back to environment variables
    if let Ok(config) = IdeConfig::from_env().await {
        return Ok(config);
    }
    
    // Use default configuration
    info!("Using default IDE configuration");
    Ok(IdeConfig::default())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_loading() {
        let config = load_config().await;
        assert!(config.is_ok());
    }
}
