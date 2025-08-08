use crate::task_manager::core::*;
use crate::agent_runtime::{AgentError, AgentResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// Agent coordination manager for multi-agent task collaboration
pub struct AgentCoordinationManager {
    active_coordinations: Arc<RwLock<HashMap<String, AgentCoordination>>>,
    agent_assignments: Arc<RwLock<HashMap<String, Vec<AgentAssignment>>>>, // task_id -> assignments
    coordination_events: broadcast::Sender<CoordinationEvent>,
}

/// Coordination events for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationEvent {
    AgentAssigned { task_id: String, agent_assignment: AgentAssignment },
    AgentStarted { task_id: String, agent_id: String },
    AgentProgress { task_id: String, agent_id: String, progress: String },
    AgentCompleted { task_id: String, agent_id: String, result: String },
    AgentHandoff { task_id: String, from_agent: String, to_agent: String, context: String },
    ConflictDetected { task_id: String, agents: Vec<String>, conflict_type: String },
    CoordinationStarted { coordination_id: String, task_id: String, agents: Vec<String> },
    CoordinationCompleted { coordination_id: String, task_id: String },
}

/// Agent task assignment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignmentRequest {
    pub task_id: String,
    pub agent_id: String,
    pub agent_type: String,
    pub model_info: ModelInfo,
    pub collaboration_mode: CollaborationMode,
    pub priority: AssignmentPriority,
    pub estimated_duration: Option<u32>, // minutes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssignmentPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Agent handoff request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandoffRequest {
    pub task_id: String,
    pub from_agent_id: String,
    pub to_agent_id: String,
    pub handoff_context: String,
    pub completion_status: HandoffCompletionStatus,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandoffCompletionStatus {
    Completed,
    PartiallyCompleted,
    Blocked,
    Failed,
}

impl AgentCoordinationManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        Self {
            active_coordinations: Arc::new(RwLock::new(HashMap::new())),
            agent_assignments: Arc::new(RwLock::new(HashMap::new())),
            coordination_events: sender,
        }
    }
    
    /// Assign an agent to a task
    pub async fn assign_agent(&self, request: AgentAssignmentRequest) -> Result<String, TaskManagerError> {
        let assignment_id = generate_id();
        let current_time = current_timestamp();
        
        // Check if task allows this collaboration mode
        let can_assign = self.can_assign_agent(&request).await?;
        if !can_assign {
            return Err(TaskManagerError::AssignmentError(
                "Task collaboration mode doesn't allow this assignment".to_string()
            ));
        }
        
        let assignment = AgentAssignment {
            agent_id: request.agent_id.clone(),
            agent_type: request.agent_type.clone(),
            model_info: request.model_info.clone(),
            assigned_at: current_time,
            status: AgentTaskStatus::Assigned,
            progress_notes: None,
            estimated_completion: request.estimated_duration.map(|d| current_time + (d as u64 * 60)),
        };
        
        // Add to assignments
        {
            let mut assignments = self.agent_assignments.write().await;
            assignments.entry(request.task_id.clone()).or_default().push(assignment.clone());
        }
        
        // Send event
        self.coordination_events.send(CoordinationEvent::AgentAssigned {
            task_id: request.task_id.clone(),
            agent_assignment: assignment,
        }).map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create coordination if multiple agents
        if request.collaboration_mode == CollaborationMode::Parallel {
            self.create_coordination_if_needed(&request.task_id).await?;
        }
        
        Ok(assignment_id)
    }
    
    /// Update agent status on a task
    pub async fn update_agent_status(
        &self,
        task_id: &str,
        agent_id: &str,
        status: AgentTaskStatus,
        progress_notes: Option<String>,
    ) -> Result<(), TaskManagerError> {
        {
            let mut assignments = self.agent_assignments.write().await;
            if let Some(task_assignments) = assignments.get_mut(task_id) {
                if let Some(assignment) = task_assignments.iter_mut().find(|a| a.agent_id == agent_id) {
                    assignment.status = status.clone();
                    assignment.progress_notes = progress_notes.clone();
                }
            }
        }
        
        // Send appropriate event
        match status {
            AgentTaskStatus::Working => {
                self.coordination_events.send(CoordinationEvent::AgentStarted {
                    task_id: task_id.to_string(),
                    agent_id: agent_id.to_string(),
                }).map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
            },
            AgentTaskStatus::Completed => {
                self.coordination_events.send(CoordinationEvent::AgentCompleted {
                    task_id: task_id.to_string(),
                    agent_id: agent_id.to_string(),
                    result: progress_notes.unwrap_or_default(),
                }).map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
            },
            _ => {
                if let Some(notes) = progress_notes {
                    self.coordination_events.send(CoordinationEvent::AgentProgress {
                        task_id: task_id.to_string(),
                        agent_id: agent_id.to_string(),
                        progress: notes,
                    }).map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle agent handoff
    pub async fn handoff_agent(&self, request: AgentHandoffRequest) -> Result<(), TaskManagerError> {
        // Update from agent status
        self.update_agent_status(
            &request.task_id,
            &request.from_agent_id,
            AgentTaskStatus::Handed_Off,
            Some(format!("Handed off to {}", request.to_agent_id)),
        ).await?;
        
        // Send handoff event
        self.coordination_events.send(CoordinationEvent::AgentHandoff {
            task_id: request.task_id.clone(),
            from_agent: request.from_agent_id,
            to_agent: request.to_agent_id,
            context: request.handoff_context,
        }).map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get active agents for a task
    pub async fn get_task_agents(&self, task_id: &str) -> Result<Vec<AgentAssignment>, TaskManagerError> {
        let assignments = self.agent_assignments.read().await;
        Ok(assignments.get(task_id).cloned().unwrap_or_default())
    }
    
    /// Get all active agents across all tasks
    pub async fn get_all_active_agents(&self) -> Result<HashMap<String, Vec<AgentAssignment>>, TaskManagerError> {
        let assignments = self.agent_assignments.read().await;
        Ok(assignments.clone())
    }
    
    /// Detect and resolve conflicts between agents
    pub async fn detect_conflicts(&self, task_id: &str) -> Result<Vec<AgentConflict>, TaskManagerError> {
        let assignments = self.agent_assignments.read().await;
        let mut conflicts = Vec::new();
        
        if let Some(task_assignments) = assignments.get(task_id) {
            // Check for multiple agents working on single-mode task
            let working_agents: Vec<_> = task_assignments.iter()
                .filter(|a| a.status == AgentTaskStatus::Working)
                .collect();
            
            if working_agents.len() > 1 {
                // This would need to check the actual task's collaboration mode
                // For now, we'll assume it's a conflict if more than one agent is working
                conflicts.push(AgentConflict {
                    conflict_type: ConflictType::MultipleWorkingAgents,
                    task_id: task_id.to_string(),
                    involved_agents: working_agents.iter().map(|a| a.agent_id.clone()).collect(),
                    description: "Multiple agents working on task simultaneously".to_string(),
                    severity: ConflictSeverity::Medium,
                    suggested_resolution: "Coordinate agent activities or switch to sequential mode".to_string(),
                });
            }
            
            // Check for resource conflicts (same model/provider)
            let mut model_usage: HashMap<String, Vec<String>> = HashMap::new();
            for assignment in task_assignments {
                let model_key = format!("{}:{}", assignment.model_info.provider, assignment.model_info.model_name);
                model_usage.entry(model_key).or_default().push(assignment.agent_id.clone());
            }
            
            for (model, agents) in model_usage {
                if agents.len() > 1 {
                    conflicts.push(AgentConflict {
                        conflict_type: ConflictType::ResourceContention,
                        task_id: task_id.to_string(),
                        involved_agents: agents,
                        description: format!("Multiple agents using same model: {}", model),
                        severity: ConflictSeverity::Low,
                        suggested_resolution: "Consider using different models or sequential execution".to_string(),
                    });
                }
            }
        }
        
        Ok(conflicts)
    }
    
    /// Subscribe to coordination events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<CoordinationEvent> {
        self.coordination_events.subscribe()
    }
    
    /// Generate agent activity summary for UI display
    pub async fn get_agent_activity_summary(&self, task_id: &str) -> Result<AgentActivitySummary, TaskManagerError> {
        let assignments = self.agent_assignments.read().await;
        
        if let Some(task_assignments) = assignments.get(task_id) {
            let total_agents = task_assignments.len();
            let active_agents = task_assignments.iter()
                .filter(|a| matches!(a.status, AgentTaskStatus::Working | AgentTaskStatus::Assigned))
                .count();
            let completed_agents = task_assignments.iter()
                .filter(|a| a.status == AgentTaskStatus::Completed)
                .count();
            
            let models_in_use: Vec<_> = task_assignments.iter()
                .filter(|a| a.status == AgentTaskStatus::Working)
                .map(|a| format!("{}:{}", a.model_info.provider, a.model_info.model_name))
                .collect();
            
            Ok(AgentActivitySummary {
                task_id: task_id.to_string(),
                total_agents,
                active_agents,
                completed_agents,
                models_in_use,
                last_activity: task_assignments.iter()
                    .map(|a| a.assigned_at)
                    .max()
                    .unwrap_or(0),
            })
        } else {
            Ok(AgentActivitySummary {
                task_id: task_id.to_string(),
                total_agents: 0,
                active_agents: 0,
                completed_agents: 0,
                models_in_use: vec![],
                last_activity: 0,
            })
        }
    }
    
    // Helper methods
    
    async fn can_assign_agent(&self, request: &AgentAssignmentRequest) -> Result<bool, TaskManagerError> {
        let assignments = self.agent_assignments.read().await;
        
        if let Some(existing_assignments) = assignments.get(&request.task_id) {
            match request.collaboration_mode {
                CollaborationMode::Single => {
                    // Only allow if no other agents are assigned or working
                    Ok(existing_assignments.iter().all(|a| 
                        matches!(a.status, AgentTaskStatus::Completed | AgentTaskStatus::Failed | AgentTaskStatus::Handed_Off)
                    ))
                },
                CollaborationMode::Parallel => Ok(true),
                CollaborationMode::Sequential => {
                    // Allow if no agent is currently working
                    Ok(existing_assignments.iter().all(|a| a.status != AgentTaskStatus::Working))
                },
                CollaborationMode::Review => Ok(true),
            }
        } else {
            Ok(true) // No existing assignments
        }
    }
    
    async fn create_coordination_if_needed(&self, task_id: &str) -> Result<(), TaskManagerError> {
        let assignments = self.agent_assignments.read().await;
        
        if let Some(task_assignments) = assignments.get(task_id) {
            if task_assignments.len() > 1 {
                let coordination_id = generate_id();
                let coordination = AgentCoordination {
                    coordination_id: coordination_id.clone(),
                    participating_agents: task_assignments.iter().map(|a| a.agent_id.clone()).collect(),
                    coordination_type: CoordinationType::Parallel_Work,
                    started_at: current_timestamp(),
                    status: CoordinationStatus::Active,
                    shared_context: HashMap::new(),
                };
                
                {
                    let mut coordinations = self.active_coordinations.write().await;
                    coordinations.insert(coordination_id.clone(), coordination);
                }
                
                self.coordination_events.send(CoordinationEvent::CoordinationStarted {
                    coordination_id,
                    task_id: task_id.to_string(),
                    agents: task_assignments.iter().map(|a| a.agent_id.clone()).collect(),
                }).map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
            }
        }
        
        Ok(())
    }
}

/// Agent conflict detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConflict {
    pub conflict_type: ConflictType,
    pub task_id: String,
    pub involved_agents: Vec<String>,
    pub description: String,
    pub severity: ConflictSeverity,
    pub suggested_resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    MultipleWorkingAgents,
    ResourceContention,
    SkillOverlap,
    DeadlockDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Agent activity summary for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivitySummary {
    pub task_id: String,
    pub total_agents: usize,
    pub active_agents: usize,
    pub completed_agents: usize,
    pub models_in_use: Vec<String>,
    pub last_activity: u64,
}
