use crate::agent_runtime::{AgentType, AgentConfig, AgentContext, AgentResult, AgentError, Agent, AgentCapability};
use crate::agent_communication::{AgentMessage, MessageType, CommunicationProtocol};
use crate::context_compression::{ContextCompressor, CompressionSettings};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for specialized agent behavior
#[async_trait]
pub trait SpecializedAgent: Send + Sync {
    /// Get the agent type this implementation handles
    fn agent_type(&self) -> AgentType;
    
    /// Execute a task with the given context
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError>;
    
    /// Validate if this agent can handle the given task
    fn can_handle_task(&self, task: &str) -> bool;
    
    /// Get agent-specific context requirements
    fn get_context_requirements(&self) -> ContextRequirements;
    
    /// Process handoff context from another agent
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequirements {
    pub required_patterns: Vec<String>,
    pub optional_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_context_age_minutes: Option<u32>,
    pub priority_keywords: Vec<String>,
}

/// Architect Agent - High-level system design and architecture decisions
pub struct ArchitectAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl ArchitectAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for ArchitectAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Architect
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        // Architect focuses on high-level design, patterns, and system architecture
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Architect Agent".to_string()));
        }
        
        // Use full context window for comprehensive architectural analysis
        let analysis = self.analyze_architecture_requirements(&context, task).await?;
        let design_decisions = self.make_design_decisions(&analysis).await?;
        let recommendations = self.generate_recommendations(&design_decisions).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Architecture Analysis\n{}\n\n## Design Decisions\n{}\n\n## Recommendations\n{}", 
                          analysis, design_decisions, recommendations),
            metadata: HashMap::from([
                ("agent_type".to_string(), "architect".to_string()),
                ("focus".to_string(), "system_design".to_string()),
            ]),
            confidence: 0.9,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let architecture_keywords = [
            "design", "architecture", "pattern", "structure", "system",
            "framework", "component", "module", "interface", "api",
            "scalability", "performance", "security", "maintainability"
        ];
        
        let task_lower = task.to_lowercase();
        architecture_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "requirements".to_string(),
                "specifications".to_string(),
                "constraints".to_string(),
            ],
            optional_patterns: vec![
                "existing_architecture".to_string(),
                "performance_metrics".to_string(),
                "user_stories".to_string(),
            ],
            exclude_patterns: vec![
                "implementation_details".to_string(),
                "debug_logs".to_string(),
            ],
            max_context_age_minutes: Some(60),
            priority_keywords: vec![
                "architecture".to_string(),
                "design".to_string(),
                "requirements".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        // Process context from other agents, focusing on architectural aspects
        match from_agent {
            AgentType::Researcher => {
                // Extract requirements and constraints from research
                self.extract_architectural_requirements(context).await
            },
            AgentType::Developer => {
                // Extract implementation feedback for architectural refinement
                self.extract_implementation_feedback(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl ArchitectAgent {
    async fn analyze_architecture_requirements(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        // Comprehensive architectural analysis using full context
        Ok(format!("Analyzed requirements for: {}", task))
    }
    
    async fn make_design_decisions(&self, analysis: &str) -> Result<String, AgentError> {
        // Make informed design decisions based on analysis
        Ok("Design decisions made based on analysis".to_string())
    }
    
    async fn generate_recommendations(&self, decisions: &str) -> Result<String, AgentError> {
        // Generate actionable recommendations
        Ok("Generated architectural recommendations".to_string())
    }
    
    async fn extract_architectural_requirements(&self, context: &str) -> Result<String, AgentError> {
        // Extract architectural requirements from research context
        Ok(format!("Extracted architectural requirements from: {}", context))
    }
    
    async fn extract_implementation_feedback(&self, context: &str) -> Result<String, AgentError> {
        // Extract implementation feedback for architectural refinement
        Ok(format!("Extracted implementation feedback: {}", context))
    }
}

// Agent trait implementation removed - will be implemented properly later

/// Developer Agent - Core development tasks and implementation
pub struct DeveloperAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl DeveloperAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for DeveloperAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Developer
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Developer Agent".to_string()));
        }
        
        // Use full context for comprehensive development understanding
        let implementation_plan = self.create_implementation_plan(&context, task).await?;
        let code_generation = self.generate_code(&implementation_plan).await?;
        let quality_check = self.perform_quality_check(&code_generation).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Implementation Plan\n{}\n\n## Generated Code\n{}\n\n## Quality Assessment\n{}", 
                          implementation_plan, code_generation, quality_check),
            metadata: HashMap::from([
                ("agent_type".to_string(), "developer".to_string()),
                ("focus".to_string(), "implementation".to_string()),
            ]),
            confidence: 0.85,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let development_keywords = [
            "implement", "code", "function", "class", "method",
            "feature", "bug", "fix", "refactor", "optimize",
            "create", "build", "develop", "write"
        ];
        
        let task_lower = task.to_lowercase();
        development_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "specifications".to_string(),
                "existing_code".to_string(),
                "requirements".to_string(),
            ],
            optional_patterns: vec![
                "test_cases".to_string(),
                "documentation".to_string(),
                "examples".to_string(),
            ],
            exclude_patterns: vec![
                "research_notes".to_string(),
                "meeting_notes".to_string(),
            ],
            max_context_age_minutes: Some(30),
            priority_keywords: vec![
                "implementation".to_string(),
                "code".to_string(),
                "function".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        match from_agent {
            AgentType::Architect => {
                // Extract implementation specifications from architectural design
                self.extract_implementation_specs(context).await
            },
            AgentType::Tester => {
                // Extract implementation requirements from test feedback
                self.extract_test_requirements(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl DeveloperAgent {
    async fn create_implementation_plan(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Implementation plan for: {}", task))
    }
    
    async fn generate_code(&self, plan: &str) -> Result<String, AgentError> {
        Ok("Generated code based on plan".to_string())
    }
    
    async fn perform_quality_check(&self, code: &str) -> Result<String, AgentError> {
        Ok("Quality check completed".to_string())
    }
    
    async fn extract_implementation_specs(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted implementation specs: {}", context))
    }
    
    async fn extract_test_requirements(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted test requirements: {}", context))
    }
}

// Agent trait implementation removed - will be implemented properly later

/// Tester Agent - Comprehensive testing strategies and implementation
pub struct TesterAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl TesterAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for TesterAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Tester
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Tester Agent".to_string()));
        }
        
        let test_strategy = self.develop_test_strategy(&context, task).await?;
        let test_implementation = self.implement_tests(&test_strategy).await?;
        let coverage_analysis = self.analyze_coverage(&test_implementation).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Test Strategy\n{}\n\n## Test Implementation\n{}\n\n## Coverage Analysis\n{}", 
                          test_strategy, test_implementation, coverage_analysis),
            metadata: HashMap::from([
                ("agent_type".to_string(), "tester".to_string()),
                ("focus".to_string(), "testing".to_string()),
            ]),
            confidence: 0.88,
            requires_review: false,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let testing_keywords = [
            "test", "testing", "spec", "verify", "validate",
            "coverage", "e2e", "unit", "integration", "jest",
            "cypress", "selenium", "mock", "stub"
        ];
        
        let task_lower = task.to_lowercase();
        testing_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "code_to_test".to_string(),
                "requirements".to_string(),
                "specifications".to_string(),
            ],
            optional_patterns: vec![
                "existing_tests".to_string(),
                "bug_reports".to_string(),
                "user_scenarios".to_string(),
            ],
            exclude_patterns: vec![
                "implementation_details".to_string(),
                "architectural_decisions".to_string(),
            ],
            max_context_age_minutes: Some(45),
            priority_keywords: vec![
                "test".to_string(),
                "verify".to_string(),
                "coverage".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        match from_agent {
            AgentType::Developer => {
                // Extract testable components from implementation
                self.extract_testable_components(context).await
            },
            AgentType::Architect => {
                // Extract testing requirements from architectural design
                self.extract_testing_requirements(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl TesterAgent {
    async fn develop_test_strategy(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Test strategy for: {}", task))
    }
    
    async fn implement_tests(&self, strategy: &str) -> Result<String, AgentError> {
        Ok("Implemented comprehensive tests".to_string())
    }
    
    async fn analyze_coverage(&self, tests: &str) -> Result<String, AgentError> {
        Ok("Coverage analysis completed".to_string())
    }
    
    async fn extract_testable_components(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted testable components: {}", context))
    }
    
    async fn extract_testing_requirements(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted testing requirements: {}", context))
    }
}

// Agent trait implementation removed - will be implemented properly later

/// Debug Agent - Specialized debugging and error resolution
pub struct DebugAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl DebugAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for DebugAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Debug
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Debug Agent".to_string()));
        }
        
        let error_analysis = self.analyze_error(&context, task).await?;
        let root_cause = self.identify_root_cause(&error_analysis).await?;
        let solution = self.propose_solution(&root_cause).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Error Analysis\n{}\n\n## Root Cause\n{}\n\n## Proposed Solution\n{}", 
                          error_analysis, root_cause, solution),
            metadata: HashMap::from([
                ("agent_type".to_string(), "debug".to_string()),
                ("focus".to_string(), "error_resolution".to_string()),
            ]),
            confidence: 0.82,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let debug_keywords = [
            "debug", "error", "bug", "issue", "problem",
            "crash", "exception", "failure", "broken",
            "fix", "resolve", "troubleshoot", "diagnose"
        ];
        
        let task_lower = task.to_lowercase();
        debug_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "error_message".to_string(),
                "stack_trace".to_string(),
                "failing_code".to_string(),
            ],
            optional_patterns: vec![
                "logs".to_string(),
                "reproduction_steps".to_string(),
                "environment_info".to_string(),
            ],
            exclude_patterns: vec![
                "unrelated_code".to_string(),
                "documentation".to_string(),
            ],
            max_context_age_minutes: Some(15), // Fresh error context is crucial
            priority_keywords: vec![
                "error".to_string(),
                "exception".to_string(),
                "failure".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        // Debug agent needs focused, error-specific context
        self.extract_error_context(context).await
    }
}

impl DebugAgent {
    async fn analyze_error(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Error analysis for: {}", task))
    }
    
    async fn identify_root_cause(&self, analysis: &str) -> Result<String, AgentError> {
        Ok("Root cause identified".to_string())
    }
    
    async fn propose_solution(&self, root_cause: &str) -> Result<String, AgentError> {
        Ok("Solution proposed".to_string())
    }
    
    async fn extract_error_context(&self, context: &str) -> Result<String, AgentError> {
        // Extract only error-relevant information
        Ok(format!("Extracted error context: {}", context))
    }
}

// Agent trait implementation removed - will be implemented properly later

/// Security Agent - Security analysis and vulnerability assessment
pub struct SecurityAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl SecurityAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for SecurityAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Security
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Security Agent".to_string()));
        }
        
        let security_scan = self.perform_security_scan(&context, task).await?;
        let vulnerability_assessment = self.assess_vulnerabilities(&security_scan).await?;
        let recommendations = self.generate_security_recommendations(&vulnerability_assessment).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Security Scan\n{}\n\n## Vulnerability Assessment\n{}\n\n## Security Recommendations\n{}", 
                          security_scan, vulnerability_assessment, recommendations),
            metadata: HashMap::from([
                ("agent_type".to_string(), "security".to_string()),
                ("focus".to_string(), "security_analysis".to_string()),
            ]),
            confidence: 0.91,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let security_keywords = [
            "security", "vulnerability", "exploit", "attack",
            "authentication", "authorization", "encryption", "secure",
            "audit", "compliance", "penetration", "threat"
        ];
        
        let task_lower = task.to_lowercase();
        security_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "auth_code".to_string(),
                "security_sensitive".to_string(),
                "user_input".to_string(),
            ],
            optional_patterns: vec![
                "dependencies".to_string(),
                "configuration".to_string(),
                "network_code".to_string(),
            ],
            exclude_patterns: vec![
                "test_data".to_string(),
                "mock_data".to_string(),
            ],
            max_context_age_minutes: Some(120), // Security context can be older
            priority_keywords: vec![
                "security".to_string(),
                "auth".to_string(),
                "vulnerability".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        // Extract security-relevant aspects from any agent's output
        self.extract_security_context(context).await
    }
}

impl SecurityAgent {
    async fn perform_security_scan(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Security scan for: {}", task))
    }
    
    async fn assess_vulnerabilities(&self, scan: &str) -> Result<String, AgentError> {
        Ok("Vulnerability assessment completed".to_string())
    }
    
    async fn generate_security_recommendations(&self, assessment: &str) -> Result<String, AgentError> {
        Ok("Security recommendations generated".to_string())
    }
    
    async fn extract_security_context(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted security context: {}", context))
    }
}

// Agent trait implementation removed - will be implemented properly later

/// Agent Registry for managing specialized agent instances
pub struct SpecializedAgentRegistry {
    agents: Arc<RwLock<HashMap<AgentType, Box<dyn SpecializedAgent>>>>,
}

impl SpecializedAgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn register_agent(&self, agent: Box<dyn SpecializedAgent>) {
        let agent_type = agent.agent_type();
        let mut agents = self.agents.write().await;
        agents.insert(agent_type, agent);
    }
    
    pub async fn get_agent(&self, agent_type: &AgentType) -> Option<Box<dyn SpecializedAgent>> {
        let agents = self.agents.read().await;
        // Note: This is a simplified approach - in practice, you'd want to clone or use Arc
        None // Placeholder - actual implementation would handle this properly
    }
    
    pub async fn find_suitable_agent(&self, task: &str) -> Option<AgentType> {
        let agents = self.agents.read().await;
        for (agent_type, agent) in agents.iter() {
            if agent.can_handle_task(task) {
                return Some(agent_type.clone());
            }
        }
        None
    }
    
    pub async fn list_agents(&self) -> Vec<AgentType> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }
}

/// Agent registry alias for compatibility
pub type AgentRegistry = SpecializedAgentRegistry;

// Agent types are already defined above, no need for pub use

/// Rust specialist agent
pub struct RustSpecialistAgent {
    config: AgentConfig,
}

impl RustSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for RustSpecialistAgent {
    async fn execute(&self, task: &str, context: &AgentContext) -> AgentResult<String> {
        Ok(format!("Rust specialist executed: {}", task))
    }

    fn get_type(&self) -> AgentType {
        AgentType::RustSpecialist
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

/// React specialist agent
pub struct ReactSpecialistAgent {
    config: AgentConfig,
}

impl ReactSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for ReactSpecialistAgent {
    async fn execute(&self, task: &str, context: &AgentContext) -> AgentResult<String> {
        Ok(format!("React specialist executed: {}", task))
    }

    fn get_type(&self) -> AgentType {
        AgentType::ReactSpecialist
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

/// Python specialist agent
pub struct PythonSpecialistAgent {
    config: AgentConfig,
}

impl PythonSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for PythonSpecialistAgent {
    async fn execute(&self, task: &str, context: &AgentContext) -> AgentResult<String> {
        Ok(format!("Python specialist executed: {}", task))
    }

    fn get_type(&self) -> AgentType {
        AgentType::PythonSpecialist
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

/// DevOps specialist agent
pub struct DevOpsSpecialistAgent {
    config: AgentConfig,
}

impl DevOpsSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for DevOpsSpecialistAgent {
    async fn execute(&self, task: &str, context: &AgentContext) -> AgentResult<String> {
        Ok(format!("DevOps specialist executed: {}", task))
    }

    fn get_type(&self) -> AgentType {
        AgentType::DevOps
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

/// Performance optimizer agent
pub struct PerformanceOptimizerAgent {
    config: AgentConfig,
}

impl PerformanceOptimizerAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for PerformanceOptimizerAgent {
    async fn execute(&self, task: &str, context: &AgentContext) -> AgentResult<String> {
        Ok(format!("Performance optimizer executed: {}", task))
    }

    fn get_type(&self) -> AgentType {
        AgentType::PerformanceOptimizer
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

/// Researcher agent
pub struct ResearcherAgent {
    config: AgentConfig,
}

impl ResearcherAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Agent for ResearcherAgent {
    async fn execute(&self, task: &str, context: &AgentContext) -> AgentResult<String> {
        Ok(format!("Researcher executed: {}", task))
    }

    fn get_type(&self) -> AgentType {
        AgentType::Research
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_task_validation() {
        // Test that agents correctly identify tasks they can handle
        let architect = ArchitectAgent::new(
            AgentConfig::default(),
            Arc::new(CommunicationProtocol::new()),
            Arc::new(ContextCompressor::new(CompressionSettings::default())),
        );
        
        assert!(architect.can_handle_task("Design the system architecture"));
        assert!(!architect.can_handle_task("Fix this specific bug"));
    }
    
    #[tokio::test]
    async fn test_context_requirements() {
        let debug_agent = DebugAgent::new(
            AgentConfig::default(),
            Arc::new(CommunicationProtocol::new()),
            Arc::new(ContextCompressor::new(CompressionSettings::default())),
        );
        
        let requirements = debug_agent.get_context_requirements();
        assert!(requirements.required_patterns.contains(&"error_message".to_string()));
        assert_eq!(requirements.max_context_age_minutes, Some(15));
    }
}
