//! # Multi-Database Connection Manager for Symbiote IDE
//! 
//! Comprehensive database integration supporting SQLite, Neo4j, and Qdrant
//! with connection pooling, health monitoring, and automatic failover.
//! 
//! Following Week 5-6 Database & Storage Systems implementation plan.

use crate::{Result, SymbioteError, config::DatabaseConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use sqlx::{SqlitePool, Pool, Sqlite};
use neo4rs::{Graph, ConfigBuilder};
use qdrant_client::{QdrantClient, client::QdrantClientConfig};

/// Database connection status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Error(String),
    Connecting,
}

/// Database health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    /// Connection status
    pub status: ConnectionStatus,
    
    /// Last successful connection time
    pub last_connected: Option<u64>,
    
    /// Connection latency in milliseconds
    pub latency_ms: Option<u64>,
    
    /// Number of active connections
    pub active_connections: u32,
    
    /// Maximum connections allowed
    pub max_connections: u32,
    
    /// Error count in last hour
    pub error_count: u32,
    
    /// Database version
    pub version: Option<String>,
}

/// SQLite connection manager
pub struct SqliteManager {
    pool: Option<SqlitePool>,
    config: DatabaseConfig,
    health: Arc<RwLock<DatabaseHealth>>,
}

impl SqliteManager {
    /// Create new SQLite manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            pool: None,
            config,
            health: Arc::new(RwLock::new(DatabaseHealth {
                status: ConnectionStatus::Disconnected,
                last_connected: None,
                latency_ms: None,
                active_connections: 0,
                max_connections: 10, // Default for SQLite
                error_count: 0,
                version: None,
            })),
        }
    }

    /// Connect to SQLite database
    pub async fn connect(&mut self) -> Result<()> {
        let start_time = Instant::now();
        
        // Update status to connecting
        {
            let mut health = self.health.write().await;
            health.status = ConnectionStatus::Connecting;
        }

        // Create connection pool
        let pool = SqlitePool::connect(&format!("sqlite:{}", self.config.sqlite_path)).await
            .map_err(|e| SymbioteError::database(format!("Failed to connect to SQLite: {}", e)))?;

        // Test connection
        let version = sqlx::query_scalar::<_, String>("SELECT sqlite_version()")
            .fetch_one(&pool)
            .await
            .map_err(|e| SymbioteError::database(format!("Failed to query SQLite version: {}", e)))?;

        let latency = start_time.elapsed().as_millis() as u64;

        // Update health status
        {
            let mut health = self.health.write().await;
            health.status = ConnectionStatus::Connected;
            health.last_connected = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            health.latency_ms = Some(latency);
            health.version = Some(version);
        }

        self.pool = Some(pool);
        tracing::info!("Connected to SQLite database in {}ms", latency);
        Ok(())
    }

    /// Get database pool
    pub fn pool(&self) -> Result<&SqlitePool> {
        self.pool.as_ref().ok_or_else(|| SymbioteError::database("SQLite not connected"))
    }

    /// Get health status
    pub async fn health(&self) -> DatabaseHealth {
        self.health.read().await.clone()
    }

    /// Disconnect from database
    pub async fn disconnect(&mut self) {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        
        let mut health = self.health.write().await;
        health.status = ConnectionStatus::Disconnected;
        health.active_connections = 0;
    }
}

/// Neo4j connection manager
pub struct Neo4jManager {
    graph: Option<Arc<Graph>>,
    config: DatabaseConfig,
    health: Arc<RwLock<DatabaseHealth>>,
}

impl Neo4jManager {
    /// Create new Neo4j manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            graph: None,
            config,
            health: Arc::new(RwLock::new(DatabaseHealth {
                status: ConnectionStatus::Disconnected,
                last_connected: None,
                latency_ms: None,
                active_connections: 0,
                max_connections: 100, // Default for Neo4j
                error_count: 0,
                version: None,
            })),
        }
    }

    /// Connect to Neo4j database
    pub async fn connect(&mut self) -> Result<()> {
        let start_time = Instant::now();
        
        // Update status to connecting
        {
            let mut health = self.health.write().await;
            health.status = ConnectionStatus::Connecting;
        }

        // Create Neo4j connection
        let config = ConfigBuilder::default()
            .uri(&self.config.neo4j_uri)
            .user(&self.config.neo4j_user)
            .password(&self.config.neo4j_password)
            .build()
            .map_err(|e| SymbioteError::database(format!("Failed to build Neo4j config: {}", e)))?;

        let graph = Graph::connect(config).await
            .map_err(|e| SymbioteError::database(format!("Failed to connect to Neo4j: {}", e)))?;

        // Test connection with a simple query
        let mut result = graph.execute(neo4rs::query("CALL dbms.components() YIELD name, versions, edition")).await
            .map_err(|e| SymbioteError::database(format!("Failed to test Neo4j connection: {}", e)))?;

        let version = if let Some(row) = result.next().await.map_err(|e| SymbioteError::database(format!("Failed to read Neo4j version: {}", e)))? {
            row.get::<String>("versions").unwrap_or_else(|_| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };

        let latency = start_time.elapsed().as_millis() as u64;

        // Update health status
        {
            let mut health = self.health.write().await;
            health.status = ConnectionStatus::Connected;
            health.last_connected = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            health.latency_ms = Some(latency);
            health.version = Some(version);
        }

        self.graph = Some(Arc::new(graph));
        tracing::info!("Connected to Neo4j database in {}ms", latency);
        Ok(())
    }

    /// Get graph connection
    pub fn graph(&self) -> Result<Arc<Graph>> {
        self.graph.as_ref().cloned().ok_or_else(|| SymbioteError::database("Neo4j not connected"))
    }

    /// Get health status
    pub async fn health(&self) -> DatabaseHealth {
        self.health.read().await.clone()
    }

    /// Disconnect from database
    pub async fn disconnect(&mut self) {
        self.graph = None;
        
        let mut health = self.health.write().await;
        health.status = ConnectionStatus::Disconnected;
        health.active_connections = 0;
    }
}

/// Qdrant connection manager
pub struct QdrantManager {
    client: Option<QdrantClient>,
    config: DatabaseConfig,
    health: Arc<RwLock<DatabaseHealth>>,
}

impl QdrantManager {
    /// Create new Qdrant manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            client: None,
            config,
            health: Arc::new(RwLock::new(DatabaseHealth {
                status: ConnectionStatus::Disconnected,
                last_connected: None,
                latency_ms: None,
                active_connections: 0,
                max_connections: 50, // Default for Qdrant
                error_count: 0,
                version: None,
            })),
        }
    }

    /// Connect to Qdrant database
    pub async fn connect(&mut self) -> Result<()> {
        let start_time = Instant::now();
        
        // Update status to connecting
        {
            let mut health = self.health.write().await;
            health.status = ConnectionStatus::Connecting;
        }

        // Create Qdrant client configuration
        let mut client_config = QdrantClientConfig::from_url(&self.config.qdrant_uri);
        
        if let Some(api_key) = &self.config.qdrant_api_key {
            client_config = client_config.with_api_key(api_key);
        }

        let client = QdrantClient::new(Some(client_config))
            .map_err(|e| SymbioteError::database(format!("Failed to create Qdrant client: {}", e)))?;

        // Test connection
        let health_check = client.health_check().await
            .map_err(|e| SymbioteError::database(format!("Qdrant health check failed: {}", e)))?;

        let latency = start_time.elapsed().as_millis() as u64;

        // Update health status
        {
            let mut health = self.health.write().await;
            health.status = ConnectionStatus::Connected;
            health.last_connected = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            health.latency_ms = Some(latency);
            health.version = Some(health_check.version.unwrap_or_else(|| "Unknown".to_string()));
        }

        self.client = Some(client);
        tracing::info!("Connected to Qdrant database in {}ms", latency);
        Ok(())
    }

    /// Get Qdrant client
    pub fn client(&self) -> Result<&QdrantClient> {
        self.client.as_ref().ok_or_else(|| SymbioteError::database("Qdrant not connected"))
    }

    /// Get health status
    pub async fn health(&self) -> DatabaseHealth {
        self.health.read().await.clone()
    }

    /// Disconnect from database
    pub async fn disconnect(&mut self) {
        self.client = None;
        
        let mut health = self.health.write().await;
        health.status = ConnectionStatus::Disconnected;
        health.active_connections = 0;
    }
}

/// Multi-database manager coordinating all database connections
pub struct MultiDatabaseManager {
    sqlite: SqliteManager,
    neo4j: Neo4jManager,
    qdrant: QdrantManager,
    config: DatabaseConfig,
}

impl MultiDatabaseManager {
    /// Create new multi-database manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            sqlite: SqliteManager::new(config.clone()),
            neo4j: Neo4jManager::new(config.clone()),
            qdrant: QdrantManager::new(config.clone()),
            config,
        }
    }

    /// Connect to all databases
    pub async fn connect_all(&mut self) -> Result<()> {
        tracing::info!("Connecting to all databases...");

        // Connect to SQLite
        if let Err(e) = self.sqlite.connect().await {
            tracing::error!("Failed to connect to SQLite: {}", e);
        }

        // Connect to Neo4j
        if let Err(e) = self.neo4j.connect().await {
            tracing::error!("Failed to connect to Neo4j: {}", e);
        }

        // Connect to Qdrant
        if let Err(e) = self.qdrant.connect().await {
            tracing::error!("Failed to connect to Qdrant: {}", e);
        }

        tracing::info!("Database connection attempts completed");
        Ok(())
    }

    /// Get SQLite pool
    pub fn sqlite(&self) -> Result<&SqlitePool> {
        self.sqlite.pool()
    }

    /// Get Neo4j graph
    pub fn neo4j(&self) -> Result<Arc<Graph>> {
        self.neo4j.graph()
    }

    /// Get Qdrant client
    pub fn qdrant(&self) -> Result<&QdrantClient> {
        self.qdrant.client()
    }

    /// Get health status for all databases
    pub async fn health_status(&self) -> HashMap<String, DatabaseHealth> {
        let mut status = HashMap::new();
        status.insert("sqlite".to_string(), self.sqlite.health().await);
        status.insert("neo4j".to_string(), self.neo4j.health().await);
        status.insert("qdrant".to_string(), self.qdrant.health().await);
        status
    }

    /// Disconnect from all databases
    pub async fn disconnect_all(&mut self) {
        tracing::info!("Disconnecting from all databases...");
        self.sqlite.disconnect().await;
        self.neo4j.disconnect().await;
        self.qdrant.disconnect().await;
        tracing::info!("Disconnected from all databases");
    }

    /// Check if all databases are connected
    pub async fn all_connected(&self) -> bool {
        let health = self.health_status().await;
        health.values().all(|h| h.status == ConnectionStatus::Connected)
    }

    /// Get connection summary
    pub async fn connection_summary(&self) -> String {
        let health = self.health_status().await;
        let connected_count = health.values().filter(|h| h.status == ConnectionStatus::Connected).count();
        format!("{}/{} databases connected", connected_count, health.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_manager_creation() {
        let config = DatabaseConfig::default();
        let manager = MultiDatabaseManager::new(config);
        
        // Test that managers are created
        assert!(manager.sqlite.pool.is_none());
        assert!(manager.neo4j.graph.is_none());
        assert!(manager.qdrant.client.is_none());
    }

    #[tokio::test]
    async fn test_health_status() {
        let config = DatabaseConfig::default();
        let manager = MultiDatabaseManager::new(config);
        
        let health = manager.health_status().await;
        assert_eq!(health.len(), 3);
        assert!(health.contains_key("sqlite"));
        assert!(health.contains_key("neo4j"));
        assert!(health.contains_key("qdrant"));
        
        // All should be disconnected initially
        for (_, health) in health {
            assert_eq!(health.status, ConnectionStatus::Disconnected);
        }
    }

    #[tokio::test]
    async fn test_connection_summary() {
        let config = DatabaseConfig::default();
        let manager = MultiDatabaseManager::new(config);
        
        let summary = manager.connection_summary().await;
        assert_eq!(summary, "0/3 databases connected");
    }
}
