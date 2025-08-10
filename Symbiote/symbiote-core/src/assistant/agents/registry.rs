//! # Agent Registry
//! 
//! Registry for managing all specialized agents available to SYMBIOTE.

use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry of all specialized agents
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, Box<dyn SpecializedAgent>>>>,
    capabilities: Arc<RwLock<HashMap<String, Vec<String>>>>, // capability -> agent_ids
}

impl AgentRegistry {
    /// Create new agent registry
    pub fn new() -> Self {
        let registry = Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Register default agents
        tokio::spawn({
            let registry = registry.clone();
            async move {
                let _ = registry.register_default_agents().await;
            }
        });
        
        registry
    }

    /// Register a specialized agent
    pub async fn register_agent(&self, agent: Box<dyn SpecializedAgent>) -> Result<()> {
        let agent_id = Uuid::new_v4().to_string();
        let agent_type = agent.agent_type().to_string();
        let capabilities = agent.capabilities();
        
        // Store agent
        {
            let mut agents = self.agents.write().await;
            agents.insert(agent_id.clone(), agent);
        }
        
        // Update capability mappings
        {
            let mut cap_map = self.capabilities.write().await;
            for capability in capabilities {
                cap_map.entry(capability)
                    .or_insert_with(Vec::new)
                    .push(agent_id.clone());
            }
        }
        
        Ok(())
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: &str) -> Result<Box<dyn SpecializedAgent>> {
        let agents = self.agents.read().await;
        // For now, we'll return an error since we can't clone trait objects without Clone trait
        // In a real implementation, we'd need to restructure this to use Arc from the start
        // or implement a clone method on the SpecializedAgent trait
        Err(SymbioteError::NotFound(format!("Agent cloning not implemented: {}", agent_id)))
    }

    /// Get agents by capability
    pub async fn get_agents_by_capability(&self, capability: &str) -> Vec<String> {
        let cap_map = self.capabilities.read().await;
        cap_map.get(capability).cloned().unwrap_or_default()
    }

    /// Get all available agents
    pub async fn get_available_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        let mut agent_infos = Vec::new();
        
        for (id, agent) in agents.iter() {
            agent_infos.push(AgentInfo {
                id: id.clone(),
                agent_type: agent.agent_type().to_string(),
                capabilities: agent.capabilities(),
                status: agent.get_status().await,
                load: agent.get_load().await,
            });
        }
        
        agent_infos
    }

    /// Get all capabilities
    pub async fn get_capabilities(&self) -> Vec<AgentCapability> {
        let agents = self.agents.read().await;
        let mut capabilities = Vec::new();
        
        for agent in agents.values() {
            // Would get detailed capability info from agent
            for cap_name in agent.capabilities() {
                capabilities.push(AgentCapability {
                    name: cap_name.clone(),
                    description: format!("Capability provided by {}", agent.display_name()),
                    parameters: vec![], // Would be populated by agent
                    examples: vec![], // Would be populated by agent
                });
            }
        }
        
        capabilities
    }

    /// Find best agent for task
    pub async fn find_best_agent_for_task(&self, task: &Task) -> Result<String> {
        let agents = self.agents.read().await;
        
        // Find agents that can handle the task
        let mut candidates = Vec::new();
        for (id, agent) in agents.iter() {
            if agent.can_handle_task(task).await {
                let load = agent.get_load().await;
                candidates.push((id.clone(), load));
            }
        }
        
        if candidates.is_empty() {
            return Err(SymbioteError::NotFound("No agent can handle this task".to_string()));
        }
        
        // Select agent with lowest load
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        Ok(candidates[0].0.clone())
    }

    /// Register default agents
    async fn register_default_agents(&self) -> Result<()> {
        let default_agents = create_default_agents();
        
        for agent in default_agents {
            self.register_agent(agent).await?;
        }
        
        Ok(())
    }

    /// Remove agent
    pub async fn remove_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id);
        
        // Clean up capability mappings
        let mut cap_map = self.capabilities.write().await;
        for agent_ids in cap_map.values_mut() {
            agent_ids.retain(|id| id != agent_id);
        }
        
        Ok(())
    }

    /// Get agent count
    pub async fn get_agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Check if agent exists
    pub async fn has_agent(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(agent_id)
    }

    /// Get agents by type
    pub async fn get_agents_by_type(&self, agent_type: &str) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.iter()
            .filter(|(_, agent)| agent.agent_type() == agent_type)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Update agent configuration
    pub async fn update_agent_config(&self, agent_id: &str, config: AgentConfig) -> Result<()> {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(agent_id) {
            agent.update_config(config).await?;
        } else {
            return Err(SymbioteError::NotFound(format!("Agent not found: {}", agent_id)));
        }
        Ok(())
    }

    /// Get agent statistics
    pub async fn get_agent_statistics(&self) -> AgentStatistics {
        let agents = self.agents.read().await;
        let mut stats = AgentStatistics {
            total_agents: agents.len(),
            available_agents: 0,
            busy_agents: 0,
            offline_agents: 0,
            error_agents: 0,
            total_capabilities: 0,
            agent_types: HashMap::new(),
        };
        
        for agent in agents.values() {
            match agent.get_status().await {
                AgentStatus::Available => stats.available_agents += 1,
                AgentStatus::Busy => stats.busy_agents += 1,
                AgentStatus::Offline => stats.offline_agents += 1,
                AgentStatus::Error => stats.error_agents += 1,
            }
            
            let agent_type = agent.agent_type().to_string();
            *stats.agent_types.entry(agent_type).or_insert(0) += 1;
            
            stats.total_capabilities += agent.capabilities().len();
        }
        
        stats
    }
}

/// Agent statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub total_agents: usize,
    pub available_agents: usize,
    pub busy_agents: usize,
    pub offline_agents: usize,
    pub error_agents: usize,
    pub total_capabilities: usize,
    pub agent_types: HashMap<String, usize>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
