use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub agent_type: String,
    pub name: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub context_scope: String,
    pub created_at: DateTime<Utc>,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub agent_id: String,
    pub description: String,
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: TaskStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

pub struct AgentManager {
    agents: HashMap<String, Agent>,
    tasks: HashMap<String, AgentTask>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            tasks: HashMap::new(),
        }
    }

    pub fn create_agent(&mut self, agent_type: &str, config: &str) -> Result<String> {
        let agent_id = Uuid::new_v4().to_string();
        
        let (name, role, capabilities) = match agent_type {
            "architect" => (
                "Architect Agent".to_string(),
                "System architecture and design planning".to_string(),
                vec![
                    "system_design".to_string(),
                    "architecture_planning".to_string(),
                    "technology_selection".to_string(),
                    "pattern_recognition".to_string(),
                ]
            ),
            "developer" => (
                "Developer Agent".to_string(),
                "Code implementation and development".to_string(),
                vec![
                    "code_generation".to_string(),
                    "refactoring".to_string(),
                    "debugging".to_string(),
                    "optimization".to_string(),
                ]
            ),
            "tester" => (
                "Tester Agent".to_string(),
                "Testing and quality assurance".to_string(),
                vec![
                    "test_generation".to_string(),
                    "test_execution".to_string(),
                    "quality_analysis".to_string(),
                    "coverage_analysis".to_string(),
                ]
            ),
            "reviewer" => (
                "Reviewer Agent".to_string(),
                "Code review and quality assessment".to_string(),
                vec![
                    "code_review".to_string(),
                    "security_analysis".to_string(),
                    "performance_analysis".to_string(),
                    "best_practices".to_string(),
                ]
            ),
            "orchestrator" => (
                "Orchestrator Agent".to_string(),
                "Multi-agent coordination and task management".to_string(),
                vec![
                    "task_coordination".to_string(),
                    "agent_management".to_string(),
                    "workflow_orchestration".to_string(),
                    "context_management".to_string(),
                ]
            ),
            _ => return Err(anyhow!("Unknown agent type: {}", agent_type))
        };

        let agent = Agent {
            id: agent_id.clone(),
            agent_type: agent_type.to_string(),
            name,
            role,
            capabilities,
            context_scope: "project".to_string(), // Default scope
            created_at: Utc::now(),
            status: AgentStatus::Idle,
        };

        self.agents.insert(agent_id.clone(), agent);
        Ok(agent_id)
    }

    pub async fn execute_task(&mut self, agent_id: &str, task_description: &str) -> Result<String> {
        let agent = self.agents.get_mut(agent_id)
            .ok_or_else(|| anyhow!("Agent not found: {}", agent_id))?;

        // Check if agent is available
        match agent.status {
            AgentStatus::Working => return Err(anyhow!("Agent is currently busy")),
            AgentStatus::Blocked => return Err(anyhow!("Agent is blocked")),
            AgentStatus::Error(ref msg) => return Err(anyhow!("Agent in error state: {}", msg)),
            AgentStatus::Idle => {}
        }

        // Create task
        let task_id = Uuid::new_v4().to_string();
        let task = AgentTask {
            id: task_id.clone(),
            agent_id: agent_id.to_string(),
            description: task_description.to_string(),
            context: None, // Will be populated by context manager
            created_at: Utc::now(),
            status: TaskStatus::Pending,
            result: None,
        };

        self.tasks.insert(task_id.clone(), task);

        // Update agent status
        agent.status = AgentStatus::Working;

        // Execute task based on agent type
        let result = self.execute_agent_task_internal(agent, task_description).await?;

        // Update task and agent status
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result.clone());
        }

        agent.status = AgentStatus::Idle;

        Ok(result)
    }

    async fn execute_agent_task_internal(&self, agent: &Agent, task: &str) -> Result<String> {
        // This is where we would integrate with the AI provider
        // For now, return a mock response based on agent type
        
        match agent.agent_type.as_str() {
            "architect" => {
                Ok(format!("Architecture analysis for: {}\n\nRecommended approach:\n1. Modular design\n2. Clear separation of concerns\n3. Scalable architecture patterns", task))
            },
            "developer" => {
                Ok(format!("Code implementation for: {}\n\n```rust\n// Generated code structure\npub fn {}() {{\n    // Implementation here\n}}\n```", task.replace(" ", "_")))
            },
            "tester" => {
                Ok(format!("Test strategy for: {}\n\n1. Unit tests\n2. Integration tests\n3. End-to-end tests\n4. Performance tests", task))
            },
            "reviewer" => {
                Ok(format!("Code review for: {}\n\nFindings:\n✅ Code structure is good\n⚠️  Consider error handling\n✅ Performance looks optimal", task))
            },
            "orchestrator" => {
                Ok(format!("Task coordination for: {}\n\nExecution plan:\n1. Break down into subtasks\n2. Assign to appropriate agents\n3. Monitor progress\n4. Integrate results", task))
            },
            _ => Err(anyhow!("Unknown agent type for execution"))
        }
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<&Agent> {
        self.agents.get(agent_id)
    }

    pub fn list_agents(&self) -> Vec<&Agent> {
        self.agents.values().collect()
    }

    pub fn get_task(&self, task_id: &str) -> Option<&AgentTask> {
        self.tasks.get(task_id)
    }

    pub fn list_tasks_for_agent(&self, agent_id: &str) -> Vec<&AgentTask> {
        self.tasks.values()
            .filter(|task| task.agent_id == agent_id)
            .collect()
    }
}

// Global agent manager instance
static mut AGENT_MANAGER: Option<AgentManager> = None;

pub fn create_agent(agent_type: &str, config: &str) -> Result<String> {
    unsafe {
        if AGENT_MANAGER.is_none() {
            AGENT_MANAGER = Some(AgentManager::new());
        }

        let manager = AGENT_MANAGER.as_mut().unwrap();
        manager.create_agent(agent_type, config)
    }
}

pub async fn execute_task(agent_id: &str, task: &str) -> Result<String> {
    unsafe {
        if AGENT_MANAGER.is_none() {
            AGENT_MANAGER = Some(AgentManager::new());
        }

        let manager = AGENT_MANAGER.as_mut().unwrap();
        manager.execute_task(agent_id, task).await
    }
}
