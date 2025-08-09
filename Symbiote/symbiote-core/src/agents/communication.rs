//! # Communication System for Symbiotes
//! 
//! Manages communication between Symbiotes, including message passing,
//! coordination protocols, and real-time collaboration.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

/// Communication hub for Symbiote coordination
#[derive(Debug)]
pub struct CommunicationHub {
    /// Message channels for Symbiotes
    channels: HashMap<SymbioteId, MessageChannel>,
    
    /// Broadcast channels for team communication
    broadcast_channels: HashMap<String, BroadcastChannel>,
    
    /// Message routing table
    routing_table: RoutingTable,
    
    /// Communication protocols
    protocols: HashMap<String, CommunicationProtocol>,
    
    /// Message history for debugging
    message_history: VecDeque<MessageRecord>,
    
    /// Performance metrics
    metrics: CommunicationMetrics,
}

impl CommunicationHub {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            broadcast_channels: HashMap::new(),
            routing_table: RoutingTable::new(),
            protocols: Self::default_protocols(),
            message_history: VecDeque::new(),
            metrics: CommunicationMetrics::new(),
        }
    }

    /// Register a Symbiote for communication
    pub async fn register_symbiote(&mut self, symbiote_id: SymbioteId) -> Result<()> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let channel = MessageChannel {
            symbiote_id: symbiote_id.clone(),
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            message_queue: VecDeque::new(),
            status: ChannelStatus::Active,
        };

        self.channels.insert(symbiote_id.clone(), channel);
        self.routing_table.add_route(symbiote_id.clone()).await?;

        tracing::debug!("Registered Symbiote for communication: {}", symbiote_id);
        Ok(())
    }

    /// Send a message between Symbiotes
    pub async fn send_message(
        &mut self,
        from: SymbioteId,
        to: SymbioteId,
        message: SymbioteMessage,
    ) -> Result<()> {
        // Create message record
        let record = MessageRecord {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.clone(),
            to: to.clone(),
            message: message.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            status: MessageStatus::Sent,
        };

        // Route the message
        if let Some(channel) = self.channels.get(&to) {
            channel.sender.send(message).map_err(|e| {
                SymbioteError::context(format!("Failed to send message: {}", e))
            })?;

            // Update metrics
            self.metrics.messages_sent += 1;
            
            // Store in history
            self.message_history.push_back(record);
            
            // Limit history size
            if self.message_history.len() > 1000 {
                self.message_history.pop_front();
            }
        } else {
            return Err(SymbioteError::context(format!("Symbiote not found: {}", to)));
        }

        Ok(())
    }

    /// Broadcast a message to a team
    pub async fn broadcast_to_team(
        &mut self,
        team_id: String,
        from: SymbioteId,
        message: SymbioteMessage,
    ) -> Result<()> {
        let members = if let Some(broadcast_channel) = self.broadcast_channels.get(&team_id) {
            broadcast_channel.members.clone()
        } else {
            Vec::new()
        };

        for member in members {
            if member != from {
                self.send_message(from.clone(), member, message.clone()).await?;
            }
        }

        Ok(())
    }

    /// Create a team communication channel
    pub async fn create_team_channel(
        &mut self,
        team_id: String,
        members: Vec<SymbioteId>,
        protocol: CommunicationProtocol,
    ) -> Result<()> {
        let broadcast_channel = BroadcastChannel {
            team_id: team_id.clone(),
            members,
            protocol,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            message_count: 0,
        };

        self.broadcast_channels.insert(team_id, broadcast_channel);
        Ok(())
    }

    /// Get communication metrics
    pub async fn get_metrics(&self) -> CommunicationMetrics {
        self.metrics.clone()
    }

    /// Get message history for debugging
    pub async fn get_message_history(&self, limit: Option<usize>) -> Vec<MessageRecord> {
        let limit = limit.unwrap_or(100);
        self.message_history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    fn default_protocols() -> HashMap<String, CommunicationProtocol> {
        let mut protocols = HashMap::new();
        
        protocols.insert("simple".to_string(), CommunicationProtocol::Simple);
        protocols.insert("structured".to_string(), CommunicationProtocol::Structured);
        protocols.insert("agile".to_string(), CommunicationProtocol::Agile);
        protocols.insert("collaborative".to_string(), CommunicationProtocol::Collaborative);
        protocols.insert("hierarchical".to_string(), CommunicationProtocol::Hierarchical);
        
        protocols
    }
}

/// Message channel for individual Symbiotes
#[derive(Debug)]
pub struct MessageChannel {
    pub symbiote_id: SymbioteId,
    pub sender: mpsc::UnboundedSender<SymbioteMessage>,
    pub receiver: Arc<RwLock<mpsc::UnboundedReceiver<SymbioteMessage>>>,
    pub message_queue: VecDeque<SymbioteMessage>,
    pub status: ChannelStatus,
}

/// Broadcast channel for team communication
#[derive(Debug, Clone)]
pub struct BroadcastChannel {
    pub team_id: String,
    pub members: Vec<SymbioteId>,
    pub protocol: CommunicationProtocol,
    pub created_at: u64,
    pub message_count: u64,
}

/// Channel status
#[derive(Debug, Clone)]
pub enum ChannelStatus {
    Active,
    Paused,
    Closed,
    Error,
}

/// Message between Symbiotes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteMessage {
    pub id: String,
    pub message_type: MessageType,
    pub content: MessageContent,
    pub priority: MessagePriority,
    pub requires_response: bool,
    pub correlation_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    TaskAssignment,
    TaskUpdate,
    TaskCompletion,
    ResourceRequest,
    ResourceResponse,
    Coordination,
    Status,
    Error,
    Custom(String),
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    Data(serde_json::Value),
    Task(SymbioteTask),
    Result(SymbioteResult),
    Status(SymbioteStatus),
    Error(String),
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Message record for history
#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: String,
    pub from: SymbioteId,
    pub to: SymbioteId,
    pub message: SymbioteMessage,
    pub timestamp: u64,
    pub status: MessageStatus,
}

/// Message status
#[derive(Debug, Clone)]
pub enum MessageStatus {
    Sent,
    Delivered,
    Read,
    Processed,
    Failed,
}

/// Routing table for message routing
#[derive(Debug)]
pub struct RoutingTable {
    routes: HashMap<SymbioteId, RouteInfo>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub async fn add_route(&mut self, symbiote_id: SymbioteId) -> Result<()> {
        let route_info = RouteInfo {
            symbiote_id: symbiote_id.clone(),
            last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            status: RouteStatus::Active,
            hop_count: 0,
        };

        self.routes.insert(symbiote_id, route_info);
        Ok(())
    }

    pub async fn remove_route(&mut self, symbiote_id: &SymbioteId) -> Result<()> {
        self.routes.remove(symbiote_id);
        Ok(())
    }

    pub async fn find_route(&self, symbiote_id: &SymbioteId) -> Option<&RouteInfo> {
        self.routes.get(symbiote_id)
    }
}

/// Route information
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub symbiote_id: SymbioteId,
    pub last_seen: u64,
    pub status: RouteStatus,
    pub hop_count: u32,
}

/// Route status
#[derive(Debug, Clone)]
pub enum RouteStatus {
    Active,
    Inactive,
    Unreachable,
}

/// Communication metrics
#[derive(Debug, Clone)]
pub struct CommunicationMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub average_latency_ms: f64,
    pub active_channels: u32,
    pub active_teams: u32,
    pub last_updated: u64,
}

impl CommunicationMetrics {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            messages_failed: 0,
            average_latency_ms: 0.0,
            active_channels: 0,
            active_teams: 0,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

/// Communication coordinator for managing protocols
#[derive(Debug)]
pub struct CommunicationCoordinator {
    active_protocols: HashMap<String, ProtocolInstance>,
    coordination_strategies: HashMap<String, CoordinationStrategy>,
}

impl CommunicationCoordinator {
    pub fn new() -> Self {
        Self {
            active_protocols: HashMap::new(),
            coordination_strategies: HashMap::new(),
        }
    }

    pub async fn coordinate_team_communication(
        &self,
        team_id: &str,
        protocol: &CommunicationProtocol,
    ) -> Result<()> {
        match protocol {
            CommunicationProtocol::Simple => self.coordinate_simple_protocol(team_id).await,
            CommunicationProtocol::Structured => self.coordinate_structured_protocol(team_id).await,
            CommunicationProtocol::Agile => self.coordinate_agile_protocol(team_id).await,
            CommunicationProtocol::Collaborative => self.coordinate_collaborative_protocol(team_id).await,
            CommunicationProtocol::Hierarchical => self.coordinate_hierarchical_protocol(team_id).await,
        }
    }

    async fn coordinate_simple_protocol(&self, team_id: &str) -> Result<()> {
        tracing::debug!("Coordinating simple protocol for team: {}", team_id);
        Ok(())
    }

    async fn coordinate_structured_protocol(&self, team_id: &str) -> Result<()> {
        tracing::debug!("Coordinating structured protocol for team: {}", team_id);
        Ok(())
    }

    async fn coordinate_agile_protocol(&self, team_id: &str) -> Result<()> {
        tracing::debug!("Coordinating agile protocol for team: {}", team_id);
        Ok(())
    }

    async fn coordinate_collaborative_protocol(&self, team_id: &str) -> Result<()> {
        tracing::debug!("Coordinating collaborative protocol for team: {}", team_id);
        Ok(())
    }

    async fn coordinate_hierarchical_protocol(&self, team_id: &str) -> Result<()> {
        tracing::debug!("Coordinating hierarchical protocol for team: {}", team_id);
        Ok(())
    }
}

/// Protocol instance for active communication
#[derive(Debug, Clone)]
pub struct ProtocolInstance {
    pub protocol_id: String,
    pub protocol_type: CommunicationProtocol,
    pub participants: Vec<SymbioteId>,
    pub started_at: u64,
    pub status: ProtocolStatus,
}

/// Protocol status
#[derive(Debug, Clone)]
pub enum ProtocolStatus {
    Starting,
    Active,
    Paused,
    Completed,
    Failed,
}

/// Message queue for handling message ordering
#[derive(Debug)]
pub struct MessageQueue {
    queue: VecDeque<QueuedMessage>,
    max_size: usize,
    processing: bool,
}

impl MessageQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
            processing: false,
        }
    }

    pub async fn enqueue(&mut self, message: SymbioteMessage, priority: MessagePriority) -> Result<()> {
        if self.queue.len() >= self.max_size {
            return Err(SymbioteError::context("Message queue is full".to_string()));
        }

        let queued_message = QueuedMessage {
            message,
            priority: priority.clone(),
            queued_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        // Insert based on priority
        let insert_pos = self.queue.iter()
            .position(|m| m.priority < priority)
            .unwrap_or(self.queue.len());

        self.queue.insert(insert_pos, queued_message);
        Ok(())
    }

    pub async fn dequeue(&mut self) -> Option<SymbioteMessage> {
        self.queue.pop_front().map(|qm| qm.message)
    }
}

/// Queued message with priority
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub message: SymbioteMessage,
    pub priority: MessagePriority,
    pub queued_at: u64,
}
