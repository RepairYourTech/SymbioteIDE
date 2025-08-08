// WebSocket handler for real-time PA system updates
use crate::pa_system::*;
use anyhow::Result;
use serde_json;
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

/// WebSocket handler for real-time PA system communication
pub struct PAWebSocketHandler {
    pub config: PASystemConfig,
    pub event_receiver: broadcast::Receiver<PASystemEvent>,
    pub clients: Arc<tokio::sync::RwLock<Vec<tokio_tungstenite::WebSocketStream<TcpStream>>>>,
}

impl PAWebSocketHandler {
    pub fn new(config: PASystemConfig, event_receiver: broadcast::Receiver<PASystemEvent>) -> Self {
        Self {
            config,
            event_receiver,
            clients: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
    
    /// Start WebSocket server for real-time updates
    pub async fn start_server(&mut self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.config.websocket_port);
        let listener = TcpListener::bind(&addr).await?;
        
        println!("PA WebSocket server listening on: {}", addr);
        
        // Spawn event broadcaster task
        self.spawn_event_broadcaster().await;
        
        // Accept WebSocket connections
        while let Ok((stream, addr)) = listener.accept().await {
            let clients = Arc::clone(&self.clients);
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, clients).await {
                    eprintln!("WebSocket connection error: {}", e);
                }
            });
        }
        
        Ok(())
    }
    
    /// Spawn task to broadcast PA system events to all connected clients
    async fn spawn_event_broadcaster(&mut self) {
        let mut event_receiver = self.event_receiver.resubscribe();
        let clients = Arc::clone(&self.clients);
        
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                let message = match serde_json::to_string(&event) {
                    Ok(msg) => Message::Text(msg),
                    Err(e) => {
                        eprintln!("Failed to serialize PA event: {}", e);
                        continue;
                    }
                };
                
                // Broadcast to all connected clients
                let mut clients_lock = clients.write().await;
                let mut disconnected_indices = Vec::new();
                
                for (i, client) in clients_lock.iter_mut().enumerate() {
                    if let Err(_) = client.send(message.clone()).await {
                        disconnected_indices.push(i);
                    }
                }
                
                // Remove disconnected clients
                for &i in disconnected_indices.iter().rev() {
                    clients_lock.remove(i);
                }
            }
        });
    }
    
    /// Broadcast event to all connected clients
    pub async fn broadcast_event(&self, event: PASystemEvent) -> Result<()> {
        let message = Message::Text(serde_json::to_string(&event)?);
        let mut clients = self.clients.write().await;
        let mut disconnected_indices = Vec::new();
        
        for (i, client) in clients.iter_mut().enumerate() {
            if let Err(_) = client.send(message.clone()).await {
                disconnected_indices.push(i);
            }
        }
        
        // Remove disconnected clients
        for &i in disconnected_indices.iter().rev() {
            clients.remove(i);
        }
        
        Ok(())
    }
}

/// Handle individual WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    clients: Arc<tokio::sync::RwLock<Vec<tokio_tungstenite::WebSocketStream<TcpStream>>>>,
) -> Result<()> {
    println!("New WebSocket connection from: {}", addr);
    
    let ws_stream = accept_async(stream).await?;
    
    // Add client to the list
    clients.write().await.push(ws_stream);
    
    Ok(())
}

/// WebSocket message types for PA system communication
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum PAWebSocketMessage {
    // Client to server messages
    CreateTask { title: String, description: String },
    UpdateTask { task_id: String, updates: serde_json::Value },
    GeneratePlan { request: PlanGenerationRequest },
    AssignAgent { task_id: String, agent_id: String },
    
    // Server to client messages (events)
    TaskCreated { task_id: String, task: serde_json::Value },
    TaskUpdated { task_id: String, changes: serde_json::Value },
    PlanGenerated { plan_id: String, plan: serde_json::Value },
    AgentStatusUpdate { agent_id: String, status: String },
    
    // System messages
    Connected,
    Error { message: String },
}

/// WebSocket client for connecting to PA system from frontend
pub struct PAWebSocketClient {
    pub url: String,
    pub event_sender: broadcast::Sender<PASystemEvent>,
}

impl PAWebSocketClient {
    pub fn new(url: String) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        Self {
            url,
            event_sender: event_tx,
        }
    }
    
    /// Connect to PA WebSocket server
    pub async fn connect(&self) -> Result<()> {
        // This would be implemented for frontend connection
        // For now, placeholder for the connection logic
        Ok(())
    }
    
    /// Send message to PA system
    pub async fn send_message(&self, message: PAWebSocketMessage) -> Result<()> {
        // Implementation for sending messages to PA system
        Ok(())
    }
}
