//! # Global Context - Central Context Store
//! 
//! The GlobalContext maintains the complete state of the Symbiote IDE,
//! including workspace, files, conversations, workflows, and agents.
//! 
//! Following Week 13-14 Context Management & Knowledge Graph implementation plan.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};


/// Global context containing all system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalContext {
    pub current_workspace: Option<WorkspaceContext>,
    pub open_files: HashMap<String, FileContext>,
    pub active_conversations: HashMap<String, ConversationContext>,
    pub running_workflows: HashMap<String, WorkflowContext>,
    pub agent_states: HashMap<String, AgentContext>,
    pub user_preferences: UserPreferences,
    pub session_info: SessionInfo,
    pub last_optimization: u64,
    pub compression_ratio: f64,
    pub version: u32,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self {
            current_workspace: None,
            open_files: HashMap::new(),
            active_conversations: HashMap::new(),
            running_workflows: HashMap::new(),
            agent_states: HashMap::new(),
            user_preferences: UserPreferences::default(),
            session_info: SessionInfo::new(),
            last_optimization: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            compression_ratio: 1.0,
            version: 1,
        }
    }

    /// Apply a context update to the global context
    pub fn apply_update(&mut self, update: ContextUpdate) -> Result<()> {
        match update.update_type {
            ContextUpdateType::FileOpened => {
                self.handle_file_opened(update)?;
            }
            ContextUpdateType::FileModified => {
                self.handle_file_modified(update)?;
            }
            ContextUpdateType::FileClosed => {
                self.handle_file_closed(update)?;
            }
            ContextUpdateType::WorkspaceChanged => {
                self.handle_workspace_changed(update)?;
            }
            ContextUpdateType::ConversationStarted => {
                self.handle_conversation_started(update)?;
            }
            ContextUpdateType::ConversationUpdated => {
                self.handle_conversation_updated(update)?;
            }
            ContextUpdateType::WorkflowStarted => {
                self.handle_workflow_started(update)?;
            }
            ContextUpdateType::WorkflowCompleted => {
                self.handle_workflow_completed(update)?;
            }
            ContextUpdateType::AgentStateChanged => {
                self.handle_agent_state_changed(update)?;
            }
            ContextUpdateType::UserAction => {
                self.handle_user_action(update)?;
            }
            ContextUpdateType::SystemEvent => {
                self.handle_system_event(update)?;
            }
        }

        self.version += 1;
        Ok(())
    }

    /// Get context specific to a system
    pub fn get_system_context(&self, system_id: &SystemId) -> Result<SystemContext> {
        // Filter context based on system needs
        let relevant_files = self.get_relevant_files_for_system(system_id);
        let relevant_conversations = self.get_relevant_conversations_for_system(system_id);
        let relevant_workflows = self.get_relevant_workflows_for_system(system_id);
        let relevant_agents = self.get_relevant_agents_for_system(system_id);

        Ok(SystemContext {
            system_id: system_id.clone(),
            workspace_context: self.current_workspace.clone(),
            file_contexts: relevant_files,
            conversation_contexts: relevant_conversations,
            workflow_contexts: relevant_workflows,
            agent_contexts: relevant_agents,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        })
    }

    /// Apply compression to reduce memory usage
    pub fn apply_compression(&mut self, compression: ContextCompression) -> Result<()> {
        // Compress old conversations
        for conversation_id in &compression.conversations_to_compress {
            if let Some(conversation) = self.active_conversations.get_mut(conversation_id) {
                // Compress conversation in place
                if conversation.messages.len() > 50 {
                    // Keep only the last 25 messages
                    let messages_to_keep = conversation.messages.split_off(conversation.messages.len() - 25);
                    conversation.messages = messages_to_keep;
                }
            }
        }

        // Compress old workflows
        for workflow_id in &compression.workflows_to_compress {
            if let Some(workflow) = self.running_workflows.get_mut(workflow_id) {
                // Compress workflow steps - keep only important ones
                workflow.steps.retain(|step| {
                    matches!(step.status, StepStatus::Failed | StepStatus::Completed)
                });
            }
        }

        // Compress agent memories
        for agent_id in &compression.agents_to_compress {
            if let Some(agent) = self.agent_states.get_mut(agent_id) {
                // Compress short-term memory
                while agent.memory.short_term.len() > 50 {
                    agent.memory.short_term.pop_front();
                }

                // Compress long-term memory - keep only high importance items
                agent.memory.long_term.retain(|_, item| item.importance > 0.7);
            }
        }

        self.compression_ratio = compression.compression_ratio;
        Ok(())
    }

    /// Apply optimization to improve performance
    pub fn apply_optimization(&mut self, optimization: ContextOptimization) -> Result<()> {
        // Apply file optimizations
        for (file_path, file_optimization) in optimization.file_optimizations {
            if let Some(file_context) = self.open_files.get_mut(&file_path) {
                // Apply file optimization directly
                if file_optimization.reduce_symbol_detail {
                    // Reduce symbol detail by keeping only public symbols
                    file_context.symbols.retain(|symbol| matches!(symbol.visibility, Visibility::Public));
                }
                // Other optimizations can be applied here
            }
        }

        // Apply conversation optimizations
        for (conversation_id, conversation_optimization) in optimization.conversation_optimizations {
            if let Some(conversation) = self.active_conversations.get_mut(&conversation_id) {
                if conversation_optimization.compress_old_messages {
                    // Compress old messages by keeping only recent ones
                    while conversation.messages.len() > 20 {
                        conversation.messages.pop_front();
                    }
                }
            }
        }

        // Apply agent optimizations
        for (agent_id, agent_optimization) in optimization.agent_optimizations {
            if let Some(agent) = self.agent_states.get_mut(&agent_id) {
                if agent_optimization.compress_memory {
                    // Compress agent memory
                    while agent.memory.short_term.len() > 100 {
                        agent.memory.short_term.pop_front();
                    }
                }
            }
        }

        self.last_optimization = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Ok(())
    }

    // Private helper methods for handling different update types
    fn handle_file_opened(&mut self, update: ContextUpdate) -> Result<()> {
        let file_path = update.data["path"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing file path in update".to_string()))?;
        
        let file_context = FileContext {
            path: file_path.to_string(),
            language: update.data["language"].as_str().unwrap_or("unknown").to_string(),
            content_hash: update.data["content_hash"].as_str().unwrap_or("").to_string(),
            symbols: vec![], // Will be populated by parser
            imports: vec![], // Will be populated by parser
            exports: vec![], // Will be populated by parser
            last_modified: update.timestamp,
            cursor_position: None,
            selection: None,
        };

        self.open_files.insert(file_path.to_string(), file_context);
        
        // Update workspace recent files
        if let Some(ref mut workspace) = self.current_workspace {
            workspace.recent_files.push_front(file_path.to_string());
            if workspace.recent_files.len() > 20 {
                workspace.recent_files.pop_back();
            }
        }

        Ok(())
    }

    fn handle_file_modified(&mut self, update: ContextUpdate) -> Result<()> {
        let file_path = update.data["path"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing file path in update".to_string()))?;
        
        if let Some(file_context) = self.open_files.get_mut(file_path) {
            file_context.last_modified = update.timestamp;
            if let Some(content_hash) = update.data["content_hash"].as_str() {
                file_context.content_hash = content_hash.to_string();
            }
            if let Some(cursor) = update.data["cursor_position"].as_object() {
                file_context.cursor_position = Some(Position {
                    line: cursor["line"].as_u64().unwrap_or(0) as u32,
                    column: cursor["column"].as_u64().unwrap_or(0) as u32,
                });
            }
        }

        Ok(())
    }

    fn handle_file_closed(&mut self, update: ContextUpdate) -> Result<()> {
        let file_path = update.data["path"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing file path in update".to_string()))?;
        
        self.open_files.remove(file_path);
        Ok(())
    }

    fn handle_workspace_changed(&mut self, update: ContextUpdate) -> Result<()> {
        let workspace_path = update.data["path"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing workspace path in update".to_string()))?;
        
        let workspace_context = WorkspaceContext {
            path: workspace_path.to_string(),
            name: update.data["name"].as_str().unwrap_or("Unknown").to_string(),
            project_type: ProjectType::Unknown, // Will be detected
            dependencies: vec![], // Will be populated
            configuration: HashMap::new(),
            recent_files: VecDeque::new(),
            git_info: None, // Will be populated if git repo
        };

        self.current_workspace = Some(workspace_context);
        self.open_files.clear(); // Clear files from previous workspace
        
        Ok(())
    }

    fn handle_conversation_started(&mut self, update: ContextUpdate) -> Result<()> {
        let conversation_id = update.data["id"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing conversation ID in update".to_string()))?;
        
        let conversation = ConversationContext {
            id: conversation_id.to_string(),
            title: update.data["title"].as_str().unwrap_or("New Conversation").to_string(),
            messages: VecDeque::new(),
            participants: vec![Participant::User("user".to_string()), Participant::AI("assistant".to_string())],
            context_files: vec![],
            created_at: update.timestamp,
            last_activity: update.timestamp,
        };

        self.active_conversations.insert(conversation_id.to_string(), conversation);
        Ok(())
    }

    fn handle_conversation_updated(&mut self, update: ContextUpdate) -> Result<()> {
        let conversation_id = update.data["id"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing conversation ID in update".to_string()))?;
        
        if let Some(conversation) = self.active_conversations.get_mut(conversation_id) {
            conversation.last_activity = update.timestamp;
            
            if let Some(message_data) = update.data["message"].as_object() {
                let message = Message {
                    id: Uuid::new_v4().to_string(),
                    sender: Participant::User("user".to_string()), // Simplified
                    content: message_data["content"].as_str().unwrap_or("").to_string(),
                    message_type: MessageType::Text,
                    timestamp: update.timestamp,
                    attachments: vec![],
                };
                
                conversation.messages.push_back(message);
                
                // Keep only last 100 messages
                if conversation.messages.len() > 100 {
                    conversation.messages.pop_front();
                }
            }
        }

        Ok(())
    }

    fn handle_workflow_started(&mut self, update: ContextUpdate) -> Result<()> {
        let workflow_id = update.data["id"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing workflow ID in update".to_string()))?;
        
        let workflow = WorkflowContext {
            id: workflow_id.to_string(),
            name: update.data["name"].as_str().unwrap_or("Unnamed Workflow").to_string(),
            status: WorkflowStatus::Running,
            steps: vec![], // Will be populated as workflow progresses
            current_step: None,
            variables: HashMap::new(),
            created_at: update.timestamp,
            started_at: Some(update.timestamp),
            completed_at: None,
        };

        self.running_workflows.insert(workflow_id.to_string(), workflow);
        Ok(())
    }

    fn handle_workflow_completed(&mut self, update: ContextUpdate) -> Result<()> {
        let workflow_id = update.data["id"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing workflow ID in update".to_string()))?;
        
        if let Some(workflow) = self.running_workflows.get_mut(workflow_id) {
            workflow.status = WorkflowStatus::Completed;
            workflow.completed_at = Some(update.timestamp);
        }

        Ok(())
    }

    fn handle_agent_state_changed(&mut self, update: ContextUpdate) -> Result<()> {
        let agent_id = update.data["id"].as_str()
            .ok_or_else(|| SymbioteError::context("Missing agent ID in update".to_string()))?;
        
        if let Some(agent) = self.agent_states.get_mut(agent_id) {
            if let Some(status_str) = update.data["status"].as_str() {
                agent.status = match status_str {
                    "idle" => AgentStatus::Idle,
                    "working" => AgentStatus::Working,
                    "waiting" => AgentStatus::Waiting,
                    "error" => AgentStatus::Error,
                    "offline" => AgentStatus::Offline,
                    _ => AgentStatus::Idle,
                };
            }
            
            if let Some(task) = update.data["current_task"].as_str() {
                agent.current_task = Some(task.to_string());
            }
        }

        Ok(())
    }

    fn handle_user_action(&mut self, _update: ContextUpdate) -> Result<()> {
        // Update user preferences or session info based on action
        self.session_info.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Ok(())
    }

    fn handle_system_event(&mut self, _update: ContextUpdate) -> Result<()> {
        // Handle system-level events
        Ok(())
    }

    // Helper methods for getting relevant context for systems
    fn get_relevant_files_for_system(&self, _system_id: &SystemId) -> HashMap<String, FileContext> {
        // For now, return all files. In the future, filter based on system needs
        self.open_files.clone()
    }

    fn get_relevant_conversations_for_system(&self, _system_id: &SystemId) -> HashMap<String, ConversationContext> {
        // For now, return all conversations. In the future, filter based on system needs
        self.active_conversations.clone()
    }

    fn get_relevant_workflows_for_system(&self, _system_id: &SystemId) -> HashMap<String, WorkflowContext> {
        // For now, return all workflows. In the future, filter based on system needs
        self.running_workflows.clone()
    }

    fn get_relevant_agents_for_system(&self, _system_id: &SystemId) -> HashMap<String, AgentContext> {
        // For now, return all agents. In the future, filter based on system needs
        self.agent_states.clone()
    }

    // Compression helper methods
    // Helper methods for context management
    fn calculate_memory_usage(&self) -> usize {
        let mut total = 0;

        // Calculate conversation memory
        for conversation in self.active_conversations.values() {
            total += conversation.messages.len() * 512; // Estimate 512 bytes per message
        }

        // Calculate workflow memory
        for workflow in self.running_workflows.values() {
            total += workflow.steps.len() * 1024; // Estimate 1KB per step
        }

        // Calculate agent memory
        for agent in self.agent_states.values() {
            total += agent.memory.short_term.len() * 256;
            total += agent.memory.long_term.len() * 512;
        }

        total
    }
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub language: String,
    pub auto_save: bool,
    pub ai_suggestions: bool,
    pub context_window_size: usize,
    pub optimization_frequency: OptimizationFrequency,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            auto_save: true,
            ai_suggestions: true,
            context_window_size: 10000,
            optimization_frequency: OptimizationFrequency::Moderate,
        }
    }
}

/// Optimization frequency settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationFrequency {
    Low,
    Moderate,
    High,
    Aggressive,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub started_at: u64,
    pub last_activity: u64,
    pub total_actions: u32,
    pub active_systems: Vec<SystemId>,
}

impl SessionInfo {
    pub fn new() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Self {
            session_id: Uuid::new_v4().to_string(),
            started_at: now,
            last_activity: now,
            total_actions: 0,
            active_systems: vec![],
        }
    }
}
