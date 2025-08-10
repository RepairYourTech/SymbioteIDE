//! # Agent Rule System
//! 
//! User-accessible rule system for controlling agent behavior.
//! Every agent has 3 special features:
//! 1. POSITIVE rules (ALWAYS do this)
//! 2. NEGATIVE rules (NEVER do that)
//! 3. Custom instructions (fine-tune system prompt)

use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent rule manager for controlling agent behavior
#[derive(Debug, Clone)]
pub struct AgentRuleManager {
    /// Storage for agent rules by workspace_id -> agent_type -> user_id -> rules
    rules_storage: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, AgentRules>>>>>,
}

impl AgentRuleManager {
    /// Create new agent rule manager
    pub fn new() -> Self {
        Self {
            rules_storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get rules for specific workspace, agent and user
    pub async fn get_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str) -> Result<AgentRules> {
        let storage = self.rules_storage.read().await;

        if let Some(workspace_rules) = storage.get(workspace_id) {
            if let Some(agent_rules) = workspace_rules.get(agent_type) {
                if let Some(user_rules) = agent_rules.get(user_id) {
                    return Ok(user_rules.clone());
                }
            }
        }

        // Return default rules if none exist
        Ok(AgentRules::default_for_agent(agent_type))
    }

    /// Update rules for specific workspace, agent and user
    pub async fn update_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str, rules: AgentRules) -> Result<()> {
        let mut storage = self.rules_storage.write().await;

        storage.entry(workspace_id.to_string())
            .or_insert_with(HashMap::new)
            .entry(agent_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(user_id.to_string(), rules);

        Ok(())
    }

    /// Get all rules for a user across all agents in a workspace
    pub async fn get_workspace_user_rules(&self, workspace_id: &str, user_id: &str) -> Result<HashMap<String, AgentRules>> {
        let storage = self.rules_storage.read().await;
        let mut user_rules = HashMap::new();

        if let Some(workspace_rules) = storage.get(workspace_id) {
            for (agent_type, agent_rules) in workspace_rules.iter() {
                if let Some(rules) = agent_rules.get(user_id) {
                    user_rules.insert(agent_type.clone(), rules.clone());
                }
            }
        }

        Ok(user_rules)
    }

    /// Get all workspaces for a user
    pub async fn get_user_workspaces(&self, user_id: &str) -> Result<Vec<String>> {
        let storage = self.rules_storage.read().await;
        let mut workspaces = Vec::new();

        for (workspace_id, workspace_rules) in storage.iter() {
            for agent_rules in workspace_rules.values() {
                if agent_rules.contains_key(user_id) {
                    workspaces.push(workspace_id.clone());
                    break;
                }
            }
        }

        Ok(workspaces)
    }

    /// Reset rules for agent to defaults
    pub async fn reset_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str) -> Result<()> {
        let default_rules = AgentRules::default_for_agent(agent_type);
        self.update_rules(workspace_id, agent_type, user_id, default_rules).await
    }

    /// Export rules for workspace backup
    pub async fn export_workspace_rules(&self, workspace_id: &str, user_id: &str) -> Result<String> {
        let user_rules = self.get_workspace_user_rules(workspace_id, user_id).await?;
        serde_json::to_string_pretty(&user_rules)
            .map_err(|e| SymbioteError::Serialization(format!("Failed to export rules: {}", e)))
    }

    /// Import rules from backup for workspace
    pub async fn import_workspace_rules(&self, workspace_id: &str, user_id: &str, rules_json: &str) -> Result<()> {
        let user_rules: HashMap<String, AgentRules> = serde_json::from_str(rules_json)
            .map_err(|e| SymbioteError::Serialization(format!("Failed to import rules: {}", e)))?;

        for (agent_type, rules) in user_rules {
            self.update_rules(workspace_id, &agent_type, user_id, rules).await?;
        }

        Ok(())
    }

    /// Get workspace statistics
    pub async fn get_workspace_stats(&self, workspace_id: &str) -> Result<WorkspaceRuleStats> {
        let storage = self.rules_storage.read().await;
        let mut stats = WorkspaceRuleStats::default();

        if let Some(workspace_rules) = storage.get(workspace_id) {
            stats.total_agents = workspace_rules.len();
            for agent_rules in workspace_rules.values() {
                stats.total_rules += agent_rules.len();
                for rules in agent_rules.values() {
                    stats.active_rules += rules.positive_rules.iter().filter(|r| r.enabled).count();
                    stats.inactive_rules += rules.positive_rules.iter().filter(|r| !r.enabled).count();
                    stats.active_rules += rules.negative_rules.iter().filter(|r| r.enabled).count();
                    stats.inactive_rules += rules.negative_rules.iter().filter(|r| !r.enabled).count();
                }
            }
        }

        stats.last_updated = Utc::now();
        Ok(stats)
    }
}

/// Agent rules for controlling behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRules {
    /// Agent type this applies to
    pub agent_type: String,
    
    /// User ID these rules belong to
    pub user_id: String,
    
    /// POSITIVE rules - things the agent should ALWAYS do
    pub positive_rules: Vec<PositiveRule>,
    
    /// NEGATIVE rules - things the agent should NEVER do
    pub negative_rules: Vec<NegativeRule>,
    
    /// Custom instructions for fine-tuning the agent's system prompt
    pub custom_instructions: CustomInstructions,
    
    /// When these rules were created/updated
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    /// Whether these rules are active
    pub enabled: bool,
}

/// Positive rule - things the agent should ALWAYS do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositiveRule {
    pub id: String,
    pub description: String,
    pub rule_text: String,
    pub priority: u32, // Higher number = higher priority
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Negative rule - things the agent should NEVER do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegativeRule {
    pub id: String,
    pub description: String,
    pub rule_text: String,
    pub severity: RuleSeverity,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Custom instructions for fine-tuning agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomInstructions {
    /// Custom system prompt additions
    pub system_prompt_additions: String,
    
    /// Behavior modifications
    pub behavior_modifications: String,
    
    /// Response style preferences
    pub response_style: ResponseStyle,
    
    /// Communication preferences
    pub communication_preferences: CommunicationPreferences,
    
    /// Task-specific instructions
    pub task_instructions: HashMap<String, String>,
}

/// Rule severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleSeverity {
    /// Warning - log but continue
    Warning,
    /// Error - stop execution and report
    Error,
    /// Critical - immediately halt and escalate
    Critical,
}

/// Response style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStyle {
    pub verbosity: VerbosityLevel,
    pub tone: ToneStyle,
    pub format_preference: FormatPreference,
    pub include_reasoning: bool,
    pub include_confidence: bool,
}

/// Verbosity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerbosityLevel {
    Minimal,
    Concise,
    Normal,
    Detailed,
    Verbose,
}

/// Tone styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToneStyle {
    Professional,
    Casual,
    Friendly,
    Technical,
    Formal,
}

/// Format preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormatPreference {
    PlainText,
    Markdown,
    Structured,
    BulletPoints,
    Numbered,
}

/// Communication preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPreferences {
    pub ask_for_confirmation: bool,
    pub provide_alternatives: bool,
    pub explain_decisions: bool,
    pub show_progress: bool,
    pub notify_on_completion: bool,
}

impl AgentRules {
    /// Create default rules for an agent type
    pub fn default_for_agent(agent_type: &str) -> Self {
        let now = Utc::now();
        
        let (positive_rules, negative_rules) = match agent_type {
            "web_research" => (
                vec![
                    PositiveRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Always verify information from multiple sources".to_string(),
                        rule_text: "ALWAYS cross-reference information from at least 2 different sources before presenting findings".to_string(),
                        priority: 10,
                        enabled: true,
                        created_at: now,
                    },
                    PositiveRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Always include source URLs".to_string(),
                        rule_text: "ALWAYS provide source URLs for all information gathered".to_string(),
                        priority: 9,
                        enabled: true,
                        created_at: now,
                    },
                ],
                vec![
                    NegativeRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Never access inappropriate content".to_string(),
                        rule_text: "NEVER search for or access inappropriate, illegal, or harmful content".to_string(),
                        severity: RuleSeverity::Critical,
                        enabled: true,
                        created_at: now,
                    },
                ],
            ),
            "email" => (
                vec![
                    PositiveRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Always confirm before sending emails".to_string(),
                        rule_text: "ALWAYS ask for user confirmation before sending any email".to_string(),
                        priority: 10,
                        enabled: true,
                        created_at: now,
                    },
                ],
                vec![
                    NegativeRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Never send emails without explicit permission".to_string(),
                        rule_text: "NEVER send emails without explicit user permission and confirmation".to_string(),
                        severity: RuleSeverity::Critical,
                        enabled: true,
                        created_at: now,
                    },
                ],
            ),
            "crypto_trading" => (
                vec![
                    PositiveRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Always confirm trades before execution".to_string(),
                        rule_text: "ALWAYS require explicit user confirmation before executing any trade".to_string(),
                        priority: 10,
                        enabled: true,
                        created_at: now,
                    },
                ],
                vec![
                    NegativeRule {
                        id: Uuid::new_v4().to_string(),
                        description: "Never make trades without confirmation".to_string(),
                        rule_text: "NEVER execute trades without explicit user confirmation".to_string(),
                        severity: RuleSeverity::Critical,
                        enabled: true,
                        created_at: now,
                    },
                ],
            ),
            _ => (Vec::new(), Vec::new()),
        };
        
        Self {
            agent_type: agent_type.to_string(),
            user_id: "default".to_string(),
            positive_rules,
            negative_rules,
            custom_instructions: CustomInstructions::default(),
            created_at: now,
            updated_at: now,
            enabled: true,
        }
    }

    /// Apply rules to a system prompt
    pub fn apply_to_prompt(&self, base_prompt: &str) -> String {
        if !self.enabled {
            return base_prompt.to_string();
        }

        let mut enhanced_prompt = base_prompt.to_string();
        
        // Add positive rules
        if !self.positive_rules.is_empty() {
            enhanced_prompt.push_str("\n\n## POSITIVE RULES - ALWAYS DO THESE:\n");
            let mut sorted_positive = self.positive_rules.clone();
            sorted_positive.sort_by(|a, b| b.priority.cmp(&a.priority));
            
            for rule in sorted_positive.iter().filter(|r| r.enabled) {
                enhanced_prompt.push_str(&format!("- {}\n", rule.rule_text));
            }
        }
        
        // Add negative rules
        if !self.negative_rules.is_empty() {
            enhanced_prompt.push_str("\n## NEGATIVE RULES - NEVER DO THESE:\n");
            for rule in self.negative_rules.iter().filter(|r| r.enabled) {
                let severity_prefix = match rule.severity {
                    RuleSeverity::Critical => "🚨 CRITICAL",
                    RuleSeverity::Error => "❌ ERROR",
                    RuleSeverity::Warning => "⚠️ WARNING",
                };
                enhanced_prompt.push_str(&format!("- {} - {}\n", severity_prefix, rule.rule_text));
            }
        }
        
        // Add custom instructions
        if !self.custom_instructions.system_prompt_additions.is_empty() {
            enhanced_prompt.push_str("\n## CUSTOM INSTRUCTIONS:\n");
            enhanced_prompt.push_str(&self.custom_instructions.system_prompt_additions);
            enhanced_prompt.push('\n');
        }
        
        // Add behavior modifications
        if !self.custom_instructions.behavior_modifications.is_empty() {
            enhanced_prompt.push_str("\n## BEHAVIOR MODIFICATIONS:\n");
            enhanced_prompt.push_str(&self.custom_instructions.behavior_modifications);
            enhanced_prompt.push('\n');
        }
        
        // Add response style preferences
        enhanced_prompt.push_str(&format!("\n## RESPONSE STYLE:\n"));
        enhanced_prompt.push_str(&format!("- Verbosity: {:?}\n", self.custom_instructions.response_style.verbosity));
        enhanced_prompt.push_str(&format!("- Tone: {:?}\n", self.custom_instructions.response_style.tone));
        enhanced_prompt.push_str(&format!("- Format: {:?}\n", self.custom_instructions.response_style.format_preference));
        
        if self.custom_instructions.response_style.include_reasoning {
            enhanced_prompt.push_str("- ALWAYS include your reasoning process\n");
        }
        
        if self.custom_instructions.response_style.include_confidence {
            enhanced_prompt.push_str("- ALWAYS include confidence levels for your responses\n");
        }
        
        enhanced_prompt
    }

    /// Add a positive rule
    pub fn add_positive_rule(&mut self, description: String, rule_text: String, priority: u32) {
        let rule = PositiveRule {
            id: Uuid::new_v4().to_string(),
            description,
            rule_text,
            priority,
            enabled: true,
            created_at: Utc::now(),
        };
        
        self.positive_rules.push(rule);
        self.updated_at = Utc::now();
    }

    /// Add a negative rule
    pub fn add_negative_rule(&mut self, description: String, rule_text: String, severity: RuleSeverity) {
        let rule = NegativeRule {
            id: Uuid::new_v4().to_string(),
            description,
            rule_text,
            severity,
            enabled: true,
            created_at: Utc::now(),
        };
        
        self.negative_rules.push(rule);
        self.updated_at = Utc::now();
    }

    /// Remove a rule by ID
    pub fn remove_positive_rule(&mut self, rule_id: &str) {
        self.positive_rules.retain(|r| r.id != rule_id);
        self.updated_at = Utc::now();
    }

    pub fn remove_negative_rule(&mut self, rule_id: &str) {
        self.negative_rules.retain(|r| r.id != rule_id);
        self.updated_at = Utc::now();
    }

    /// Toggle rule enabled state
    pub fn toggle_positive_rule(&mut self, rule_id: &str) {
        if let Some(rule) = self.positive_rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = !rule.enabled;
            self.updated_at = Utc::now();
        }
    }

    pub fn toggle_negative_rule(&mut self, rule_id: &str) {
        if let Some(rule) = self.negative_rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = !rule.enabled;
            self.updated_at = Utc::now();
        }
    }
}

impl Default for CustomInstructions {
    fn default() -> Self {
        Self {
            system_prompt_additions: String::new(),
            behavior_modifications: String::new(),
            response_style: ResponseStyle::default(),
            communication_preferences: CommunicationPreferences::default(),
            task_instructions: HashMap::new(),
        }
    }
}

impl Default for ResponseStyle {
    fn default() -> Self {
        Self {
            verbosity: VerbosityLevel::Normal,
            tone: ToneStyle::Professional,
            format_preference: FormatPreference::Markdown,
            include_reasoning: false,
            include_confidence: false,
        }
    }
}

impl Default for CommunicationPreferences {
    fn default() -> Self {
        Self {
            ask_for_confirmation: true,
            provide_alternatives: true,
            explain_decisions: false,
            show_progress: true,
            notify_on_completion: true,
        }
    }
}

impl Default for AgentRuleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace rule statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceRuleStats {
    pub total_agents: usize,
    pub total_rules: usize,
    pub active_rules: usize,
    pub inactive_rules: usize,
    pub last_updated: DateTime<Utc>,
}
