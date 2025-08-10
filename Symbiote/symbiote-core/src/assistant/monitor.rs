//! # Agent Monitor
//! 
//! Monitoring system for tracking running agents, their progress, and results.
//! Provides the UI for users to see what agents are doing and select them for details.

use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent monitor for tracking running agents
#[derive(Debug, Clone)]
pub struct AgentMonitor {
    running_agents: Arc<RwLock<HashMap<String, RunningAgent>>>,
    agent_history: Arc<RwLock<Vec<AgentHistoryEntry>>>,
    performance_metrics: Arc<RwLock<AgentPerformanceMetrics>>,
}

impl AgentMonitor {
    /// Create new agent monitor
    pub fn new() -> Self {
        Self {
            running_agents: Arc::new(RwLock::new(HashMap::new())),
            agent_history: Arc::new(RwLock::new(Vec::new())),
            performance_metrics: Arc::new(RwLock::new(AgentPerformanceMetrics::default())),
        }
    }

    /// Start monitoring an agent
    pub async fn start_monitoring(&self, agent: RunningAgent) -> Result<()> {
        let mut running_agents = self.running_agents.write().await;
        running_agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    /// Update agent status
    pub async fn update_agent_status(&self, agent_id: &str, status: AgentExecutionStatus) -> Result<()> {
        let mut running_agents = self.running_agents.write().await;
        if let Some(agent) = running_agents.get_mut(agent_id) {
            agent.status = status;
            agent.last_update = Utc::now();
        }
        Ok(())
    }

    /// Update agent progress
    pub async fn update_agent_progress(&self, agent_id: &str, progress: f64, message: Option<String>) -> Result<()> {
        let mut running_agents = self.running_agents.write().await;
        if let Some(agent) = running_agents.get_mut(agent_id) {
            agent.progress = progress;
            agent.last_update = Utc::now();
            if let Some(msg) = message {
                agent.status_message = Some(msg);
            }
        }
        Ok(())
    }

    /// Complete agent execution
    pub async fn complete_agent(&self, agent_id: &str, result: AgentResult) -> Result<()> {
        let mut running_agents = self.running_agents.write().await;
        if let Some(mut agent) = running_agents.remove(agent_id) {
            agent.status = if result.success {
                AgentExecutionStatus::Completed
            } else {
                AgentExecutionStatus::Failed
            };
            agent.progress = 1.0;
            agent.completed_at = Some(Utc::now());
            agent.result = Some(result.clone());

            // Add to history
            let mut history = self.agent_history.write().await;
            history.push(AgentHistoryEntry {
                agent: agent.clone(),
                result,
                timestamp: Utc::now(),
            });

            // Update performance metrics
            let mut metrics = self.performance_metrics.write().await;
            metrics.total_executions += 1;
            if agent.status == AgentExecutionStatus::Completed {
                metrics.successful_executions += 1;
            } else {
                metrics.failed_executions += 1;
            }
            
            let execution_time = agent.completed_at.unwrap_or_else(Utc::now)
                .signed_duration_since(agent.started_at)
                .num_milliseconds() as u64;
            
            metrics.total_execution_time += execution_time;
            metrics.average_execution_time = metrics.total_execution_time / metrics.total_executions;
        }
        Ok(())
    }

    /// Get all running agents
    pub async fn get_running_agents(&self) -> Vec<RunningAgent> {
        let running_agents = self.running_agents.read().await;
        running_agents.values().cloned().collect()
    }

    /// Get agent details
    pub async fn get_agent_details(&self, agent_id: &str) -> Result<AgentDetails> {
        let running_agents = self.running_agents.read().await;
        if let Some(agent) = running_agents.get(agent_id) {
            Ok(AgentDetails {
                agent: agent.clone(),
                logs: self.get_agent_logs(agent_id).await,
                metrics: self.get_agent_metrics(agent_id).await,
            })
        } else {
            // Check history
            let history = self.agent_history.read().await;
            if let Some(entry) = history.iter().find(|e| e.agent.id == agent_id) {
                Ok(AgentDetails {
                    agent: entry.agent.clone(),
                    logs: self.get_agent_logs(agent_id).await,
                    metrics: self.get_agent_metrics(agent_id).await,
                })
            } else {
                Err(SymbioteError::NotFound(format!("Agent not found: {}", agent_id)))
            }
        }
    }

    /// Cancel a running agent
    pub async fn cancel_agent(&self, agent_id: &str) -> Result<()> {
        let mut running_agents = self.running_agents.write().await;
        if let Some(mut agent) = running_agents.get_mut(agent_id) {
            agent.status = AgentExecutionStatus::Cancelled;
            agent.completed_at = Some(Utc::now());
            // Would send cancellation signal to actual agent
        }
        Ok(())
    }

    /// Cancel all running agents
    pub async fn cancel_all_agents(&self) -> Result<()> {
        let mut running_agents = self.running_agents.write().await;
        for agent in running_agents.values_mut() {
            agent.status = AgentExecutionStatus::Cancelled;
            agent.completed_at = Some(Utc::now());
        }
        Ok(())
    }

    /// Get agent count
    pub async fn get_agent_count(&self) -> usize {
        let running_agents = self.running_agents.read().await;
        running_agents.len()
    }

    /// Get agent logs
    async fn get_agent_logs(&self, _agent_id: &str) -> Vec<AgentLogEntry> {
        // Mock logs (would be real logs in practice)
        vec![
            AgentLogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Agent started".to_string(),
                metadata: HashMap::new(),
            },
            AgentLogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Processing task".to_string(),
                metadata: HashMap::new(),
            },
        ]
    }

    /// Get agent metrics
    async fn get_agent_metrics(&self, _agent_id: &str) -> AgentMetrics {
        AgentMetrics {
            cpu_usage: 15.5,
            memory_usage: 128.0,
            network_usage: 1024,
            execution_time: 5000,
            tasks_completed: 1,
            success_rate: 1.0,
        }
    }

    /// Get overall performance metrics
    pub async fn get_performance_metrics(&self) -> AgentPerformanceMetrics {
        let metrics = self.performance_metrics.read().await;
        metrics.clone()
    }

    /// Get agent history
    pub async fn get_agent_history(&self, limit: Option<usize>) -> Vec<AgentHistoryEntry> {
        let history = self.agent_history.read().await;
        let limit = limit.unwrap_or(100);
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Clear agent history
    pub async fn clear_history(&self) -> Result<()> {
        let mut history = self.agent_history.write().await;
        history.clear();
        Ok(())
    }
}

/// Running agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAgent {
    pub id: String,
    pub agent_type: String,
    pub display_name: String,
    pub task_description: String,
    pub status: AgentExecutionStatus,
    pub progress: f64, // 0.0 to 1.0
    pub status_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub result: Option<AgentResult>,
    pub priority: u32,
}

/// Agent execution status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentExecutionStatus {
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Detailed agent information
#[derive(Debug, Clone)]
pub struct AgentDetails {
    pub agent: RunningAgent,
    pub logs: Vec<AgentLogEntry>,
    pub metrics: AgentMetrics,
}

/// Agent log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64, // MB
    pub network_usage: u64, // bytes
    pub execution_time: u64, // milliseconds
    pub tasks_completed: u32,
    pub success_rate: f64,
}

/// Agent history entry
#[derive(Debug, Clone)]
pub struct AgentHistoryEntry {
    pub agent: RunningAgent,
    pub result: AgentResult,
    pub timestamp: DateTime<Utc>,
}

/// Overall performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentPerformanceMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_execution_time: u64,
    pub average_execution_time: u64,
    pub peak_concurrent_agents: usize,
    pub current_concurrent_agents: usize,
}

impl RunningAgent {
    /// Create new running agent
    pub fn new(
        agent_type: String,
        display_name: String,
        task_description: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            agent_type,
            display_name,
            task_description,
            status: AgentExecutionStatus::Starting,
            progress: 0.0,
            status_message: None,
            started_at: now,
            last_update: now,
            completed_at: None,
            estimated_completion: None,
            result: None,
            priority: 1,
        }
    }

    /// Get execution duration
    pub fn get_execution_duration(&self) -> chrono::Duration {
        let end_time = self.completed_at.unwrap_or_else(Utc::now);
        end_time.signed_duration_since(self.started_at)
    }

    /// Check if agent is still running
    pub fn is_running(&self) -> bool {
        matches!(self.status, AgentExecutionStatus::Starting | AgentExecutionStatus::Running)
    }

    /// Get progress percentage
    pub fn get_progress_percentage(&self) -> u32 {
        (self.progress * 100.0) as u32
    }
}

impl Default for AgentMonitor {
    fn default() -> Self {
        Self::new()
    }
}
