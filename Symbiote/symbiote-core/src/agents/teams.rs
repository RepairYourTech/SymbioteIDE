//! # Team Management System
//! 
//! Manages preset teams and dynamic team creation for Symbiotes.
//! Teams can be specialized for different development stacks and task types.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Team manager for organizing Symbiotes into effective teams
#[derive(Debug)]
pub struct TeamManager {
    /// Preset teams for common scenarios
    preset_teams: HashMap<String, Team>,
    
    /// Dynamic teams created for specific tasks
    dynamic_teams: HashMap<String, Team>,
    
    /// Team templates for different scenarios
    team_templates: HashMap<TeamType, TeamTemplate>,
    
    /// Team performance history
    performance_history: HashMap<String, TeamPerformance>,
    
    /// Team formation strategies
    formation_strategies: HashMap<String, FormationStrategy>,
}

impl TeamManager {
    pub fn new() -> Self {
        Self {
            preset_teams: HashMap::new(),
            dynamic_teams: HashMap::new(),
            team_templates: HashMap::new(),
            performance_history: HashMap::new(),
            formation_strategies: Self::default_formation_strategies(),
        }
    }

    /// Initialize preset teams for common development scenarios
    pub async fn initialize_presets(&mut self) -> Result<()> {
        // Create preset teams for different stacks
        self.create_rust_development_team().await?;
        self.create_fullstack_web_team().await?;
        self.create_testing_team().await?;
        self.create_devops_team().await?;
        self.create_debugging_team().await?;
        self.create_code_review_team().await?;

        // Initialize team templates
        self.initialize_team_templates().await?;

        tracing::info!("Team Manager initialized with {} preset teams", self.preset_teams.len());
        Ok(())
    }

    /// Get all available teams
    pub async fn get_available_teams(&self) -> Result<Vec<TeamInfo>> {
        let mut teams = Vec::new();

        // Add preset teams
        for team in self.preset_teams.values() {
            teams.push(TeamInfo::from_team(team));
        }

        // Add active dynamic teams
        for team in self.dynamic_teams.values() {
            if team.status == TeamStatus::Active {
                teams.push(TeamInfo::from_team(team));
            }
        }

        Ok(teams)
    }

    /// Create a dynamic team for a specific task
    pub async fn create_dynamic_team(
        &mut self,
        task_analysis: &TaskAnalysis,
        available_symbiotes: &[SymbioteInfo],
    ) -> Result<Team> {
        let team_id = format!("dynamic-{}", Uuid::new_v4());
        
        // Determine optimal team composition
        let composition = self.determine_team_composition(task_analysis, available_symbiotes).await?;
        
        // Select formation strategy
        let strategy = self.select_formation_strategy(task_analysis).await?;
        
        // Create the team
        let team = Team {
            id: team_id.clone(),
            name: format!("Dynamic Team for {:?}", task_analysis.complexity),
            team_type: TeamType::Dynamic,
            members: composition.members,
            leader: composition.leader,
            specializations: composition.specializations,
            capabilities: composition.capabilities,
            coordination_strategy: strategy.coordination_strategy,
            communication_protocol: strategy.communication_protocol,
            status: TeamStatus::Forming,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        // Store the dynamic team
        self.dynamic_teams.insert(team_id, team.clone());

        Ok(team)
    }

    /// Find the best preset team for a task
    pub async fn find_best_preset_team(&self, task_analysis: &TaskAnalysis) -> Result<Option<Team>> {
        let mut best_team = None;
        let mut best_score = 0.0;

        for team in self.preset_teams.values() {
            let compatibility_score = self.calculate_team_compatibility(team, task_analysis).await?;
            
            if compatibility_score > best_score {
                best_score = compatibility_score;
                best_team = Some(team.clone());
            }
        }

        // Only return if compatibility is above threshold
        if best_score > 0.7 {
            Ok(best_team)
        } else {
            Ok(None)
        }
    }

    /// Activate a team for execution
    pub async fn activate_team(&mut self, team_id: &str) -> Result<()> {
        if let Some(team) = self.preset_teams.get_mut(team_id) {
            team.status = TeamStatus::Active;
        } else if let Some(team) = self.dynamic_teams.get_mut(team_id) {
            team.status = TeamStatus::Active;
        } else {
            return Err(SymbioteError::context(format!("Team not found: {}", team_id)));
        }

        Ok(())
    }

    /// Deactivate a team after task completion
    pub async fn deactivate_team(&mut self, team_id: &str, performance: TeamPerformance) -> Result<()> {
        // Store performance history
        self.performance_history.insert(team_id.to_string(), performance);

        // Update team status
        if let Some(team) = self.preset_teams.get_mut(team_id) {
            team.status = TeamStatus::Inactive;
        } else if let Some(team) = self.dynamic_teams.get_mut(team_id) {
            team.status = TeamStatus::Completed;
        }

        Ok(())
    }

    /// Get team performance metrics
    pub async fn get_team_performance(&self, team_id: &str) -> Result<Option<TeamPerformance>> {
        Ok(self.performance_history.get(team_id).cloned())
    }

    // Private helper methods

    async fn create_rust_development_team(&mut self) -> Result<()> {
        let team = Team {
            id: "rust-dev-team".to_string(),
            name: "Rust Development Team".to_string(),
            team_type: TeamType::Preset,
            members: vec![
                "rust-specialist".to_string(),
                "testing-specialist".to_string(),
                "debugging-specialist".to_string(),
            ],
            leader: Some("rust-specialist".to_string()),
            specializations: vec![DevelopmentStack::Rust],
            capabilities: vec![
                TeamCapability::CodeDevelopment,
                TeamCapability::Testing,
                TeamCapability::Debugging,
                TeamCapability::Performance,
            ],
            coordination_strategy: CoordinationStrategy::Hierarchical,
            communication_protocol: CommunicationProtocol::Structured,
            status: TeamStatus::Inactive,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        self.preset_teams.insert(team.id.clone(), team);
        Ok(())
    }

    async fn create_fullstack_web_team(&mut self) -> Result<()> {
        let team = Team {
            id: "fullstack-web-team".to_string(),
            name: "Full-Stack Web Development Team".to_string(),
            team_type: TeamType::Preset,
            members: vec![
                "react-specialist".to_string(),
                "javascript-specialist".to_string(),
                "testing-specialist".to_string(),
            ],
            leader: Some("react-specialist".to_string()),
            specializations: vec![
                DevelopmentStack::React,
                DevelopmentStack::JavaScript,
                DevelopmentStack::NodeJs,
            ],
            capabilities: vec![
                TeamCapability::FrontendDevelopment,
                TeamCapability::BackendDevelopment,
                TeamCapability::Testing,
                TeamCapability::Deployment,
            ],
            coordination_strategy: CoordinationStrategy::Collaborative,
            communication_protocol: CommunicationProtocol::Agile,
            status: TeamStatus::Inactive,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        self.preset_teams.insert(team.id.clone(), team);
        Ok(())
    }

    async fn create_testing_team(&mut self) -> Result<()> {
        let team = Team {
            id: "testing-team".to_string(),
            name: "Quality Assurance Team".to_string(),
            team_type: TeamType::Preset,
            members: vec![
                "testing-specialist".to_string(),
                "debugging-specialist".to_string(),
            ],
            leader: Some("testing-specialist".to_string()),
            specializations: vec![], // Works with all stacks
            capabilities: vec![
                TeamCapability::Testing,
                TeamCapability::QualityAssurance,
                TeamCapability::Performance,
            ],
            coordination_strategy: CoordinationStrategy::Parallel,
            communication_protocol: CommunicationProtocol::Structured,
            status: TeamStatus::Inactive,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        self.preset_teams.insert(team.id.clone(), team);
        Ok(())
    }

    async fn create_devops_team(&mut self) -> Result<()> {
        let team = Team {
            id: "devops-team".to_string(),
            name: "DevOps Team".to_string(),
            team_type: TeamType::Preset,
            members: vec![
                "deployment-specialist".to_string(),
                "monitoring-specialist".to_string(),
            ],
            leader: Some("deployment-specialist".to_string()),
            specializations: vec![
                DevelopmentStack::Docker,
                DevelopmentStack::Kubernetes,
                DevelopmentStack::AWS,
            ],
            capabilities: vec![
                TeamCapability::Deployment,
                TeamCapability::Infrastructure,
                TeamCapability::Monitoring,
                TeamCapability::Security,
            ],
            coordination_strategy: CoordinationStrategy::Pipeline,
            communication_protocol: CommunicationProtocol::Structured,
            status: TeamStatus::Inactive,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        self.preset_teams.insert(team.id.clone(), team);
        Ok(())
    }

    async fn create_debugging_team(&mut self) -> Result<()> {
        let team = Team {
            id: "debugging-team".to_string(),
            name: "Debugging Specialists".to_string(),
            team_type: TeamType::Preset,
            members: vec![
                "debugging-specialist".to_string(),
                "rust-specialist".to_string(), // For stack-specific debugging
            ],
            leader: Some("debugging-specialist".to_string()),
            specializations: vec![], // Works with all stacks
            capabilities: vec![
                TeamCapability::Debugging,
                TeamCapability::ErrorAnalysis,
                TeamCapability::Performance,
            ],
            coordination_strategy: CoordinationStrategy::Sequential,
            communication_protocol: CommunicationProtocol::Collaborative,
            status: TeamStatus::Inactive,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        self.preset_teams.insert(team.id.clone(), team);
        Ok(())
    }

    async fn create_code_review_team(&mut self) -> Result<()> {
        let team = Team {
            id: "code-review-team".to_string(),
            name: "Code Review Team".to_string(),
            team_type: TeamType::Preset,
            members: vec![
                "rust-specialist".to_string(),
                "testing-specialist".to_string(),
                "security-specialist".to_string(),
            ],
            leader: Some("rust-specialist".to_string()),
            specializations: vec![], // Works with all stacks
            capabilities: vec![
                TeamCapability::CodeReview,
                TeamCapability::QualityAssurance,
                TeamCapability::Security,
                TeamCapability::BestPractices,
            ],
            coordination_strategy: CoordinationStrategy::Collaborative,
            communication_protocol: CommunicationProtocol::Structured,
            status: TeamStatus::Inactive,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            performance_metrics: TeamMetrics::new(),
            metadata: HashMap::new(),
        };

        self.preset_teams.insert(team.id.clone(), team);
        Ok(())
    }

    async fn initialize_team_templates(&mut self) -> Result<()> {
        // Create templates for different team types
        self.team_templates.insert(TeamType::Development, TeamTemplate {
            name: "Development Team Template".to_string(),
            required_roles: vec![
                TeamRole::Developer,
                TeamRole::Tester,
            ],
            optional_roles: vec![
                TeamRole::Reviewer,
                TeamRole::Specialist,
            ],
            min_size: 2,
            max_size: 5,
            coordination_strategy: CoordinationStrategy::Collaborative,
        });

        self.team_templates.insert(TeamType::Analysis, TeamTemplate {
            name: "Analysis Team Template".to_string(),
            required_roles: vec![
                TeamRole::Analyst,
            ],
            optional_roles: vec![
                TeamRole::Specialist,
                TeamRole::Reviewer,
            ],
            min_size: 1,
            max_size: 3,
            coordination_strategy: CoordinationStrategy::Parallel,
        });

        Ok(())
    }

    async fn determine_team_composition(
        &self,
        task_analysis: &TaskAnalysis,
        available_symbiotes: &[SymbioteInfo],
    ) -> Result<TeamComposition> {
        let mut composition = TeamComposition {
            members: Vec::new(),
            leader: None,
            specializations: Vec::new(),
            capabilities: Vec::new(),
        };

        // Select Symbiotes based on required skills
        for skill in &task_analysis.required_skills {
            if let Some(symbiote) = self.find_best_symbiote_for_skill(skill, available_symbiotes).await? {
                if !composition.members.contains(&symbiote.id) {
                    composition.members.push(symbiote.id);
                }
            }
        }

        // Select a leader (most experienced Symbiote)
        if let Some(leader_id) = composition.members.first() {
            composition.leader = Some(leader_id.clone());
        }

        // Determine team specializations and capabilities
        for member_id in &composition.members {
            if let Some(symbiote) = available_symbiotes.iter().find(|s| s.id == *member_id) {
                composition.specializations.extend(symbiote.specializations.clone());
                // Convert symbiote capabilities to team capabilities
                for capability in &symbiote.capabilities {
                    let team_capability = self.convert_to_team_capability(capability);
                    if !composition.capabilities.contains(&team_capability) {
                        composition.capabilities.push(team_capability);
                    }
                }
            }
        }

        Ok(composition)
    }

    async fn find_best_symbiote_for_skill(
        &self,
        skill: &SymbioteSkill,
        available_symbiotes: &[SymbioteInfo],
    ) -> Result<Option<SymbioteInfo>> {
        let mut best_symbiote = None;
        let mut best_score = 0.0;

        for symbiote in available_symbiotes {
            if symbiote.skills.contains(skill) {
                // Calculate score based on success rate and experience
                let score = symbiote.success_rate + (symbiote.tasks_completed as f64 / 1000.0);
                
                if score > best_score {
                    best_score = score;
                    best_symbiote = Some(symbiote.clone());
                }
            }
        }

        Ok(best_symbiote)
    }

    fn convert_to_team_capability(&self, symbiote_capability: &SymbioteCapability) -> TeamCapability {
        match symbiote_capability {
            SymbioteCapability::FileRead | SymbioteCapability::FileWrite => TeamCapability::FileManagement,
            SymbioteCapability::CommandExecution => TeamCapability::Automation,
            SymbioteCapability::GitOperations => TeamCapability::VersionControl,
            SymbioteCapability::PackageManagement => TeamCapability::DependencyManagement,
            SymbioteCapability::NetworkAccess => TeamCapability::Integration,
            SymbioteCapability::DatabaseAccess => TeamCapability::DataManagement,
            SymbioteCapability::APIIntegration => TeamCapability::Integration,
            SymbioteCapability::DataAnalysis => TeamCapability::Analysis,
            SymbioteCapability::MachineLearning => TeamCapability::AI,
            _ => TeamCapability::General,
        }
    }

    async fn select_formation_strategy(&self, task_analysis: &TaskAnalysis) -> Result<FormationStrategy> {
        let strategy_name = match task_analysis.complexity {
            TaskComplexity::Simple => "simple",
            TaskComplexity::Moderate => "moderate",
            TaskComplexity::Complex => "complex",
            TaskComplexity::VeryComplex => "very_complex",
            TaskComplexity::Expert => "expert",
        };

        Ok(self.formation_strategies.get(strategy_name)
            .cloned()
            .unwrap_or_else(|| FormationStrategy::default()))
    }

    async fn calculate_team_compatibility(&self, team: &Team, task_analysis: &TaskAnalysis) -> Result<f64> {
        let mut score = 0.0;

        // Check capability compatibility
        for capability in &team.capabilities {
            if self.is_capability_relevant(capability, task_analysis) {
                score += 0.3;
            }
        }

        // Check specialization compatibility
        if let Some(required_stack) = task_analysis.required_skills.iter()
            .find_map(|skill| match skill {
                SymbioteSkill::StackSpecific(stack) => Some(stack),
                _ => None,
            }) {
            if team.specializations.contains(required_stack) {
                score += 0.4;
            }
        }

        // Factor in team performance history
        if let Some(performance) = self.performance_history.get(&team.id) {
            score += performance.success_rate * 0.3;
        } else {
            score += 0.15; // Default score for teams without history
        }

        Ok(score.min(1.0))
    }

    fn is_capability_relevant(&self, capability: &TeamCapability, task_analysis: &TaskAnalysis) -> bool {
        // Determine if a team capability is relevant for the task
        match capability {
            TeamCapability::CodeDevelopment => matches!(task_analysis.complexity, TaskComplexity::Moderate | TaskComplexity::Complex | TaskComplexity::VeryComplex),
            TeamCapability::Testing => task_analysis.required_skills.contains(&SymbioteSkill::TestGeneration),
            TeamCapability::Debugging => task_analysis.required_skills.contains(&SymbioteSkill::Debugging),
            TeamCapability::Analysis => task_analysis.required_skills.contains(&SymbioteSkill::CodeAnalysis),
            _ => true, // Most capabilities are generally useful
        }
    }

    fn default_formation_strategies() -> HashMap<String, FormationStrategy> {
        let mut strategies = HashMap::new();

        strategies.insert("simple".to_string(), FormationStrategy {
            coordination_strategy: CoordinationStrategy::Sequential,
            communication_protocol: CommunicationProtocol::Simple,
            team_size_preference: TeamSizePreference::Small,
        });

        strategies.insert("moderate".to_string(), FormationStrategy {
            coordination_strategy: CoordinationStrategy::Collaborative,
            communication_protocol: CommunicationProtocol::Structured,
            team_size_preference: TeamSizePreference::Medium,
        });

        strategies.insert("complex".to_string(), FormationStrategy {
            coordination_strategy: CoordinationStrategy::Hierarchical,
            communication_protocol: CommunicationProtocol::Structured,
            team_size_preference: TeamSizePreference::Large,
        });

        strategies.insert("very_complex".to_string(), FormationStrategy {
            coordination_strategy: CoordinationStrategy::Adaptive,
            communication_protocol: CommunicationProtocol::Agile,
            team_size_preference: TeamSizePreference::Large,
        });

        strategies
    }
}

/// Team structure and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub team_type: TeamType,
    pub members: Vec<SymbioteId>,
    pub leader: Option<SymbioteId>,
    pub specializations: Vec<DevelopmentStack>,
    pub capabilities: Vec<TeamCapability>,
    pub coordination_strategy: CoordinationStrategy,
    pub communication_protocol: CommunicationProtocol,
    pub status: TeamStatus,
    pub created_at: u64,
    pub performance_metrics: TeamMetrics,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of teams
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TeamType {
    Preset,
    Dynamic,
    Development,
    Analysis,
    Testing,
    Debugging,
    DevOps,
    Custom(String),
}

/// Team capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamCapability {
    CodeDevelopment,
    FrontendDevelopment,
    BackendDevelopment,
    Testing,
    QualityAssurance,
    Debugging,
    ErrorAnalysis,
    Performance,
    Security,
    CodeReview,
    BestPractices,
    Deployment,
    Infrastructure,
    Monitoring,
    Analysis,
    FileManagement,
    VersionControl,
    DependencyManagement,
    Integration,
    DataManagement,
    Automation,
    AI,
    General,
}

/// Communication protocols for teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationProtocol {
    Simple,
    Structured,
    Agile,
    Collaborative,
    Hierarchical,
}

/// Team status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamStatus {
    Inactive,
    Forming,
    Active,
    Paused,
    Completed,
    Disbanded,
}

/// Team performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub success_rate: f64,
    pub average_completion_time: f64,
    pub collaboration_score: f64,
    pub efficiency_score: f64,
}

impl TeamMetrics {
    pub fn new() -> Self {
        Self {
            tasks_completed: 0,
            tasks_failed: 0,
            success_rate: 1.0,
            average_completion_time: 0.0,
            collaboration_score: 1.0,
            efficiency_score: 1.0,
        }
    }
}

/// Team information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInfo {
    pub id: String,
    pub name: String,
    pub team_type: TeamType,
    pub member_count: usize,
    pub specializations: Vec<DevelopmentStack>,
    pub capabilities: Vec<TeamCapability>,
    pub status: TeamStatus,
    pub success_rate: f64,
    pub tasks_completed: u64,
}

impl TeamInfo {
    pub fn from_team(team: &Team) -> Self {
        Self {
            id: team.id.clone(),
            name: team.name.clone(),
            team_type: team.team_type.clone(),
            member_count: team.members.len(),
            specializations: team.specializations.clone(),
            capabilities: team.capabilities.clone(),
            status: team.status.clone(),
            success_rate: team.performance_metrics.success_rate,
            tasks_completed: team.performance_metrics.tasks_completed,
        }
    }
}

/// Team composition for dynamic team creation
#[derive(Debug, Clone)]
pub struct TeamComposition {
    pub members: Vec<SymbioteId>,
    pub leader: Option<SymbioteId>,
    pub specializations: Vec<DevelopmentStack>,
    pub capabilities: Vec<TeamCapability>,
}

/// Team template for creating teams
#[derive(Debug, Clone)]
pub struct TeamTemplate {
    pub name: String,
    pub required_roles: Vec<TeamRole>,
    pub optional_roles: Vec<TeamRole>,
    pub min_size: usize,
    pub max_size: usize,
    pub coordination_strategy: CoordinationStrategy,
}

/// Team roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamRole {
    Leader,
    Developer,
    Tester,
    Reviewer,
    Analyst,
    Specialist,
    Coordinator,
}

/// Team formation strategy
#[derive(Debug, Clone)]
pub struct FormationStrategy {
    pub coordination_strategy: CoordinationStrategy,
    pub communication_protocol: CommunicationProtocol,
    pub team_size_preference: TeamSizePreference,
}

impl Default for FormationStrategy {
    fn default() -> Self {
        Self {
            coordination_strategy: CoordinationStrategy::Collaborative,
            communication_protocol: CommunicationProtocol::Structured,
            team_size_preference: TeamSizePreference::Medium,
        }
    }
}

/// Team size preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamSizePreference {
    Small,  // 1-2 members
    Medium, // 3-4 members
    Large,  // 5+ members
}

/// Team performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPerformance {
    pub team_id: String,
    pub task_id: String,
    pub success_rate: f64,
    pub completion_time: u64,
    pub quality_score: f64,
    pub collaboration_effectiveness: f64,
    pub resource_efficiency: f64,
    pub member_satisfaction: f64,
    pub timestamp: u64,
}
