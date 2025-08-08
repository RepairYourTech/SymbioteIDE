use crate::agent_runtime::{AgentType, AgentConfig, AgentContext, AgentResult, AgentError};
use crate::agent_communication::{AgentMessage, MessageType, CommunicationProtocol};
use crate::context_compression::{ContextCompressor, CompressionSettings};
// use crate::specialized_agents::{SpecializedAgent, ContextRequirements}; // Disabled

// Stub types
pub struct SpecializedAgent;
pub struct ContextRequirements;
use crate::context_manager::ContextManager;
use crate::codebase_intelligence::CodebaseIntelligence;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent-specific context profile for intelligent context retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextProfile {
    pub agent_type: AgentType,
    pub max_context_tokens: usize,
    pub context_priorities: Vec<ContextPriority>,
    pub semantic_filters: Vec<String>,
    pub exclusion_patterns: Vec<String>,
    pub freshness_requirements: Option<u32>, // minutes
    pub relationship_depth: u8, // for graph traversal
    pub vector_similarity_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPriority {
    pub pattern: String,
    pub weight: f32,
    pub required: bool,
}

/// Intelligent context retrieval system for agents
pub struct AgentContextRetriever {
    context_manager: Arc<ContextManager>,
    codebase_intelligence: Arc<CodebaseIntelligence>,
    context_compressor: Arc<ContextCompressor>,
    agent_profiles: Arc<RwLock<HashMap<AgentType, AgentContextProfile>>>,
}

impl AgentContextRetriever {
    pub fn new(
        context_manager: Arc<ContextManager>,
        codebase_intelligence: Arc<CodebaseIntelligence>,
        context_compressor: Arc<ContextCompressor>,
    ) -> Self {
        Self {
            context_manager,
            codebase_intelligence,
            context_compressor,
            agent_profiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize with default agent context profiles
    pub async fn initialize_default_profiles(&self) -> Result<(), AgentError> {
        let mut profiles = self.agent_profiles.write().await;
        
        // Architect Agent Profile
        profiles.insert(AgentType::Architect, AgentContextProfile {
            agent_type: AgentType::Architect,
            max_context_tokens: 200000, // Large context for comprehensive analysis
            context_priorities: vec![
                ContextPriority { pattern: "requirements".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "architecture".to_string(), weight: 0.9, required: false },
                ContextPriority { pattern: "constraints".to_string(), weight: 0.8, required: false },
                ContextPriority { pattern: "specifications".to_string(), weight: 0.8, required: false },
                ContextPriority { pattern: "design_patterns".to_string(), weight: 0.7, required: false },
            ],
            semantic_filters: vec![
                "system_design".to_string(),
                "architecture_decision".to_string(),
                "component_interface".to_string(),
                "scalability".to_string(),
            ],
            exclusion_patterns: vec![
                "implementation_details".to_string(),
                "debug_logs".to_string(),
                "test_data".to_string(),
            ],
            freshness_requirements: Some(60),
            relationship_depth: 3,
            vector_similarity_threshold: 0.7,
        });
        
        // Developer Agent Profile
        profiles.insert(AgentType::Developer, AgentContextProfile {
            agent_type: AgentType::Developer,
            max_context_tokens: 32000,
            context_priorities: vec![
                ContextPriority { pattern: "implementation".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "code_structure".to_string(), weight: 0.9, required: false },
                ContextPriority { pattern: "api_interface".to_string(), weight: 0.8, required: false },
                ContextPriority { pattern: "dependencies".to_string(), weight: 0.7, required: false },
            ],
            semantic_filters: vec![
                "function_implementation".to_string(),
                "class_definition".to_string(),
                "module_structure".to_string(),
                "code_logic".to_string(),
            ],
            exclusion_patterns: vec![
                "architectural_decisions".to_string(),
                "research_notes".to_string(),
            ],
            freshness_requirements: Some(30),
            relationship_depth: 2,
            vector_similarity_threshold: 0.8,
        });
        
        // Debug Agent Profile
        profiles.insert(AgentType::DebugAgent, AgentContextProfile {
            agent_type: AgentType::DebugAgent,
            max_context_tokens: 16000,
            context_priorities: vec![
                ContextPriority { pattern: "error_context".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "stack_trace".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "failing_code".to_string(), weight: 0.9, required: true },
                ContextPriority { pattern: "recent_changes".to_string(), weight: 0.8, required: false },
            ],
            semantic_filters: vec![
                "error_message".to_string(),
                "exception_handling".to_string(),
                "bug_report".to_string(),
                "failure_point".to_string(),
            ],
            exclusion_patterns: vec![
                "unrelated_code".to_string(),
                "documentation".to_string(),
                "test_cases".to_string(),
            ],
            freshness_requirements: Some(5), // Very fresh context for debugging
            relationship_depth: 1,
            vector_similarity_threshold: 0.9,
        });
        
        // React Specialist Profile
        profiles.insert(AgentType::ReactSpecialist, AgentContextProfile {
            agent_type: AgentType::ReactSpecialist,
            max_context_tokens: 24000,
            context_priorities: vec![
                ContextPriority { pattern: "react_component".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "ui_requirements".to_string(), weight: 0.9, required: false },
                ContextPriority { pattern: "component_props".to_string(), weight: 0.8, required: false },
                ContextPriority { pattern: "state_management".to_string(), weight: 0.8, required: false },
            ],
            semantic_filters: vec![
                "jsx_component".to_string(),
                "react_hook".to_string(),
                "frontend_logic".to_string(),
                "ui_interaction".to_string(),
            ],
            exclusion_patterns: vec![
                "backend_code".to_string(),
                "database_schema".to_string(),
                "server_logic".to_string(),
            ],
            freshness_requirements: Some(45),
            relationship_depth: 2,
            vector_similarity_threshold: 0.8,
        });
        
        // Security Agent Profile
        profiles.insert(AgentType::SecurityAgent, AgentContextProfile {
            agent_type: AgentType::SecurityAgent,
            max_context_tokens: 48000,
            context_priorities: vec![
                ContextPriority { pattern: "security_sensitive".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "authentication".to_string(), weight: 0.9, required: false },
                ContextPriority { pattern: "user_input".to_string(), weight: 0.9, required: false },
                ContextPriority { pattern: "data_access".to_string(), weight: 0.8, required: false },
            ],
            semantic_filters: vec![
                "auth_code".to_string(),
                "permission_check".to_string(),
                "input_validation".to_string(),
                "encryption".to_string(),
            ],
            exclusion_patterns: vec![
                "test_data".to_string(),
                "mock_credentials".to_string(),
            ],
            freshness_requirements: Some(120),
            relationship_depth: 3,
            vector_similarity_threshold: 0.75,
        });
        
        // Add profiles for other agents...
        self.add_remaining_agent_profiles(&mut profiles).await;
        
        Ok(())
    }
    
    /// Retrieve intelligent context for a specific agent and task
    pub async fn get_agent_context(
        &self,
        agent_type: &AgentType,
        task_description: &str,
        additional_context: Option<&str>,
    ) -> Result<AgentContext, AgentError> {
        let profile = self.get_agent_profile(agent_type).await?;
        
        // Step 1: Semantic search using vector store
        let vector_context = self.retrieve_vector_context(
            &profile,
            task_description,
            additional_context,
        ).await?;
        
        // Step 2: Graph traversal for relationships
        let graph_context = self.retrieve_graph_context(
            &profile,
            task_description,
            &vector_context,
        ).await?;
        
        // Step 3: Combine and prioritize context
        let combined_context = self.combine_contexts(
            &profile,
            vector_context,
            graph_context,
        ).await?;
        
        // Step 4: Apply agent-specific filtering
        let filtered_context = self.apply_agent_filters(
            &profile,
            combined_context,
        ).await?;
        
        // Step 5: Ensure context fits within token limits
        let final_context = self.optimize_context_size(
            &profile,
            filtered_context,
        ).await?;
        
        Ok(AgentContext {
            task_id: uuid::Uuid::new_v4().to_string(),
            agent_type: agent_type.clone(),
            context_data: final_context,
            max_tokens: profile.max_context_tokens,
            priority: 5, // Default priority
        })
    }
    
    /// Retrieve context optimized for agent handoffs
    pub async fn get_handoff_context(
        &self,
        from_agent: &AgentType,
        to_agent: &AgentType,
        original_context: &str,
        task_result: &AgentResult<String>,
    ) -> Result<String, AgentError> {
        let from_profile = self.get_agent_profile(from_agent).await?;
        let to_profile = self.get_agent_profile(to_agent).await?;
        
        // Extract relevant information for the receiving agent
        let relevant_context = self.extract_relevant_context(
            &from_profile,
            &to_profile,
            original_context,
            task_result,
        ).await?;
        
        // Apply compression optimized for the receiving agent
        let compressed_context = self.context_compressor
            .compress_for_handoff(
                from_agent.clone(),
                to_agent.clone(),
                &relevant_context,
                to_profile.max_context_tokens,
            )
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;
        
        Ok(compressed_context)
    }
    
    /// Update agent context profile
    pub async fn update_agent_profile(
        &self,
        agent_type: AgentType,
        profile: AgentContextProfile,
    ) -> Result<(), AgentError> {
        let mut profiles = self.agent_profiles.write().await;
        profiles.insert(agent_type, profile);
        Ok(())
    }
    
    /// Get context statistics for monitoring
    pub async fn get_context_stats(&self, agent_type: &AgentType) -> Result<ContextStats, AgentError> {
        let profile = self.get_agent_profile(agent_type).await?;
        
        // Get statistics from codebase intelligence
        let vector_stats = self.codebase_intelligence
            .get_vector_store_stats()
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;
        
        let graph_stats = self.codebase_intelligence
            .get_graph_stats()
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;
        
        Ok(ContextStats {
            agent_type: agent_type.clone(),
            max_context_tokens: profile.max_context_tokens,
            average_context_utilization: 0.75, // Placeholder
            vector_chunks_available: vector_stats.total_chunks,
            graph_nodes_available: graph_stats.total_nodes,
            context_freshness_score: 0.85, // Placeholder
        })
    }
    
    /// Private helper methods
    
    async fn get_agent_profile(&self, agent_type: &AgentType) -> Result<AgentContextProfile, AgentError> {
        let profiles = self.agent_profiles.read().await;
        profiles.get(agent_type)
            .cloned()
            .ok_or_else(|| AgentError::AgentNotFound(format!("{:?}", agent_type)))
    }
    
    async fn retrieve_vector_context(
        &self,
        profile: &AgentContextProfile,
        task_description: &str,
        additional_context: Option<&str>,
    ) -> Result<Vec<String>, AgentError> {
        let query = if let Some(additional) = additional_context {
            format!("{} {}", task_description, additional)
        } else {
            task_description.to_string()
        };
        
        // Use semantic filters to enhance the query
        let enhanced_query = self.enhance_query_with_filters(&query, &profile.semantic_filters);
        
        // Retrieve from vector store
        let chunks = self.codebase_intelligence
            .semantic_search(
                &enhanced_query,
                50, // Max chunks
                profile.vector_similarity_threshold,
            )
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;
        
        Ok(chunks.into_iter().map(|chunk| chunk.content).collect())
    }
    
    async fn retrieve_graph_context(
        &self,
        profile: &AgentContextProfile,
        task_description: &str,
        vector_context: &[String],
    ) -> Result<Vec<String>, AgentError> {
        // Extract entities from vector context for graph traversal
        let entities = self.extract_entities_from_context(vector_context).await?;
        
        // Traverse graph to find related entities
        let related_entities = self.codebase_intelligence
            .find_related_entities(
                &entities,
                profile.relationship_depth,
            )
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;
        
        // Convert entities back to context strings
        let graph_context = self.entities_to_context_strings(&related_entities).await?;
        
        Ok(graph_context)
    }
    
    async fn combine_contexts(
        &self,
        profile: &AgentContextProfile,
        vector_context: Vec<String>,
        graph_context: Vec<String>,
    ) -> Result<String, AgentError> {
        let mut combined = Vec::new();
        
        // Add vector context with priority weighting
        for context in vector_context {
            if self.matches_priority_patterns(&context, &profile.context_priorities) {
                combined.push(context);
            }
        }
        
        // Add graph context
        combined.extend(graph_context);
        
        // Remove duplicates and sort by relevance
        combined.sort();
        combined.dedup();
        
        Ok(combined.join("\n\n"))
    }
    
    async fn apply_agent_filters(
        &self,
        profile: &AgentContextProfile,
        context: String,
    ) -> Result<String, AgentError> {
        let mut filtered_lines = Vec::new();
        
        for line in context.lines() {
            // Check exclusion patterns
            let should_exclude = profile.exclusion_patterns.iter()
                .any(|pattern| line.to_lowercase().contains(&pattern.to_lowercase()));
            
            if !should_exclude {
                filtered_lines.push(line);
            }
        }
        
        Ok(filtered_lines.join("\n"))
    }
    
    async fn optimize_context_size(
        &self,
        profile: &AgentContextProfile,
        context: String,
    ) -> Result<String, AgentError> {
        let estimated_tokens = context.len() / 4; // Rough estimation
        
        if estimated_tokens <= profile.max_context_tokens {
            return Ok(context);
        }
        
        // Apply intelligent truncation while preserving priority content
        let compressed = self.context_compressor
            .compress_with_priorities(
                &context,
                profile.max_context_tokens,
                &profile.context_priorities.iter()
                    .map(|p| p.pattern.clone())
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;
        
        Ok(compressed)
    }
    
    async fn extract_relevant_context(
        &self,
        from_profile: &AgentContextProfile,
        to_profile: &AgentContextProfile,
        original_context: &str,
        task_result: &AgentResult<String>,
    ) -> Result<String, AgentError> {
        let mut relevant_parts = Vec::new();
        
        // Extract parts relevant to the receiving agent
        for priority in &to_profile.context_priorities {
            if priority.required {
                // Find content matching this priority pattern
                for line in original_context.lines() {
                    if line.to_lowercase().contains(&priority.pattern.to_lowercase()) {
                        relevant_parts.push(line.to_string());
                    }
                }
            }
        }
        
        // Add task result output
        relevant_parts.push(format!("Previous Task Result:\n{}", task_result.output));
        
        Ok(relevant_parts.join("\n"))
    }
    
    async fn add_remaining_agent_profiles(&self, profiles: &mut HashMap<AgentType, AgentContextProfile>) {
        // Add profiles for remaining agents (Tester, Rust Specialist, etc.)
        // This is a simplified version - in practice, each would have detailed profiles
        
        profiles.insert(AgentType::Tester, AgentContextProfile {
            agent_type: AgentType::Tester,
            max_context_tokens: 28000,
            context_priorities: vec![
                ContextPriority { pattern: "test_requirements".to_string(), weight: 1.0, required: true },
                ContextPriority { pattern: "code_to_test".to_string(), weight: 0.9, required: true },
            ],
            semantic_filters: vec!["test_case".to_string(), "verification".to_string()],
            exclusion_patterns: vec!["implementation_details".to_string()],
            freshness_requirements: Some(30),
            relationship_depth: 2,
            vector_similarity_threshold: 0.8,
        });
        
        // Add other agent profiles...
    }
    
    fn enhance_query_with_filters(&self, query: &str, filters: &[String]) -> String {
        let mut enhanced = query.to_string();
        for filter in filters {
            enhanced.push_str(&format!(" {}", filter));
        }
        enhanced
    }
    
    async fn extract_entities_from_context(&self, context: &[String]) -> Result<Vec<String>, AgentError> {
        // Extract code entities (functions, classes, etc.) from context
        let mut entities = Vec::new();
        
        for chunk in context {
            // Simple entity extraction - in practice, this would use the parser
            for line in chunk.lines() {
                if line.contains("function ") || line.contains("class ") || line.contains("struct ") {
                    entities.push(line.to_string());
                }
            }
        }
        
        Ok(entities)
    }
    
    async fn entities_to_context_strings(&self, entities: &[String]) -> Result<Vec<String>, AgentError> {
        // Convert entities back to context strings
        Ok(entities.to_vec())
    }
    
    fn matches_priority_patterns(&self, context: &str, priorities: &[ContextPriority]) -> bool {
        priorities.iter().any(|priority| {
            context.to_lowercase().contains(&priority.pattern.to_lowercase())
        })
    }
}

/// Context statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub agent_type: AgentType,
    pub max_context_tokens: usize,
    pub average_context_utilization: f32,
    pub vector_chunks_available: u64,
    pub graph_nodes_available: u64,
    pub context_freshness_score: f32,
}

/// Agent context integration manager
pub struct AgentContextIntegration {
    context_retriever: Arc<AgentContextRetriever>,
    communication_protocol: Arc<CommunicationProtocol>,
}

impl AgentContextIntegration {
    pub fn new(
        context_manager: Arc<ContextManager>,
        codebase_intelligence: Arc<CodebaseIntelligence>,
        context_compressor: Arc<ContextCompressor>,
        communication_protocol: Arc<CommunicationProtocol>,
    ) -> Self {
        let context_retriever = Arc::new(AgentContextRetriever::new(
            context_manager,
            codebase_intelligence,
            context_compressor,
        ));
        
        Self {
            context_retriever,
            communication_protocol,
        }
    }
    
    /// Initialize the integration system
    pub async fn initialize(&self) -> Result<(), AgentError> {
        self.context_retriever.initialize_default_profiles().await?;
        Ok(())
    }
    
    /// Get context retriever for use by agents
    pub fn get_context_retriever(&self) -> Arc<AgentContextRetriever> {
        self.context_retriever.clone()
    }
    
    /// Process agent context request with full intelligence
    pub async fn process_context_request(
        &self,
        agent_type: AgentType,
        task_description: String,
        additional_context: Option<String>,
    ) -> Result<AgentContext, AgentError> {
        self.context_retriever
            .get_agent_context(
                &agent_type,
                &task_description,
                additional_context.as_deref(),
            )
            .await
    }
    
    /// Process agent handoff with intelligent context transformation
    pub async fn process_agent_handoff(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        original_context: String,
        task_result: AgentResult<String>,
    ) -> Result<String, AgentError> {
        self.context_retriever
            .get_handoff_context(
                &from_agent,
                &to_agent,
                &original_context,
                &task_result,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_context_retrieval() {
        // Test context retrieval for different agent types
        // This would require mock implementations of the dependencies
    }
    
    #[tokio::test]
    async fn test_context_handoff_optimization() {
        // Test that handoff context is properly optimized for receiving agent
    }
}
