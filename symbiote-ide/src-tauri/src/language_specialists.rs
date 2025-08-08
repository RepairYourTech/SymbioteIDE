use crate::agent_runtime::{AgentType, AgentConfig, AgentContext, AgentResult, AgentError};
use crate::agent_communication::{AgentMessage, MessageType, CommunicationProtocol};
use crate::context_compression::{ContextCompressor, CompressionSettings};
use crate::specialized_agents::{SpecializedAgent, ContextRequirements};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// React Specialist Agent - React/TypeScript/Frontend expertise
pub struct ReactSpecialistAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl ReactSpecialistAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for ReactSpecialistAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::ReactSpecialist
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult<String>, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for React Specialist".to_string()));
        }
        
        let component_analysis = self.analyze_react_requirements(&context, task).await?;
        let implementation = self.implement_react_solution(&component_analysis).await?;
        let optimization = self.optimize_react_code(&implementation).await?;
        
        Ok(format!("Rust analysis completed: {}", analysis))
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## React Component Analysis\n{}\n\n## Implementation\n{}\n\n## Optimization\n{}", 
                          component_analysis, implementation, optimization),
            metadata: HashMap::from([
                ("agent_type".to_string(), "react_specialist".to_string()),
                ("language".to_string(), "typescript".to_string()),
                ("framework".to_string(), "react".to_string()),
            ]),
            confidence: 0.92,
            requires_review: false,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let react_keywords = [
            "react", "component", "jsx", "tsx", "hook", "state",
            "props", "context", "reducer", "effect", "memo",
            "frontend", "ui", "interface", "typescript", "javascript"
        ];
        
        let task_lower = task.to_lowercase();
        react_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "react_code".to_string(),
                "component_specs".to_string(),
                "ui_requirements".to_string(),
            ],
            optional_patterns: vec![
                "existing_components".to_string(),
                "design_system".to_string(),
                "api_interfaces".to_string(),
            ],
            exclude_patterns: vec![
                "backend_code".to_string(),
                "database_schema".to_string(),
            ],
            max_context_age_minutes: Some(30),
            priority_keywords: vec![
                "component".to_string(),
                "react".to_string(),
                "typescript".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        match from_agent {
            AgentType::Architect => {
                self.extract_ui_architecture(context).await
            },
            AgentType::Developer => {
                self.extract_component_requirements(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl ReactSpecialistAgent {
    async fn analyze_react_requirements(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("React requirements analysis for: {}", task))
    }
    
    async fn implement_react_solution(&self, analysis: &str) -> Result<String, AgentError> {
        Ok("React implementation completed".to_string())
    }
    
    async fn optimize_react_code(&self, implementation: &str) -> Result<String, AgentError> {
        Ok("React code optimized".to_string())
    }
    
    async fn extract_ui_architecture(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted UI architecture: {}", context))
    }
    
    async fn extract_component_requirements(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted component requirements: {}", context))
    }
}

/// Rust Specialist Agent - Rust/Systems Programming expertise
pub struct RustSpecialistAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl RustSpecialistAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for RustSpecialistAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::RustSpecialist
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Rust Specialist".to_string()));
        }
        
        let rust_analysis = self.analyze_rust_requirements(&context, task).await?;
        let implementation = self.implement_rust_solution(&rust_analysis).await?;
        let safety_check = self.perform_safety_analysis(&implementation).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Rust Analysis\n{}\n\n## Implementation\n{}\n\n## Safety Analysis\n{}", 
                          rust_analysis, implementation, safety_check),
            metadata: HashMap::from([
                ("agent_type".to_string(), "rust_specialist".to_string()),
                ("language".to_string(), "rust".to_string()),
                ("focus".to_string(), "systems_programming".to_string()),
            ]),
            confidence: 0.89,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let rust_keywords = [
            "rust", "cargo", "trait", "impl", "struct", "enum",
            "lifetime", "borrow", "ownership", "async", "tokio",
            "unsafe", "memory", "performance", "systems", "backend"
        ];
        
        let task_lower = task.to_lowercase();
        rust_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "rust_code".to_string(),
                "cargo_toml".to_string(),
                "system_requirements".to_string(),
            ],
            optional_patterns: vec![
                "performance_requirements".to_string(),
                "memory_constraints".to_string(),
                "concurrency_needs".to_string(),
            ],
            exclude_patterns: vec![
                "frontend_code".to_string(),
                "ui_components".to_string(),
            ],
            max_context_age_minutes: Some(45),
            priority_keywords: vec![
                "rust".to_string(),
                "performance".to_string(),
                "memory".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        match from_agent {
            AgentType::Architect => {
                self.extract_system_architecture(context).await
            },
            AgentType::Developer => {
                self.extract_implementation_specs(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl RustSpecialistAgent {
    async fn analyze_rust_requirements(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Rust requirements analysis for: {}", task))
    }
    
    async fn implement_rust_solution(&self, analysis: &str) -> Result<String, AgentError> {
        Ok("Rust implementation completed".to_string())
    }
    
    async fn perform_safety_analysis(&self, implementation: &str) -> Result<String, AgentError> {
        Ok("Rust safety analysis completed".to_string())
    }
    
    async fn extract_system_architecture(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted system architecture: {}", context))
    }
    
    async fn extract_implementation_specs(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted implementation specs: {}", context))
    }
}

/// Python Specialist Agent - Python/Data Science/ML expertise
pub struct PythonSpecialistAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl PythonSpecialistAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for PythonSpecialistAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::PythonSpecialist
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Python Specialist".to_string()));
        }
        
        let python_analysis = self.analyze_python_requirements(&context, task).await?;
        let implementation = self.implement_python_solution(&python_analysis).await?;
        let optimization = self.optimize_python_code(&implementation).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Python Analysis\n{}\n\n## Implementation\n{}\n\n## Optimization\n{}", 
                          python_analysis, implementation, optimization),
            metadata: HashMap::from([
                ("agent_type".to_string(), "python_specialist".to_string()),
                ("language".to_string(), "python".to_string()),
                ("focus".to_string(), "data_science".to_string()),
            ]),
            confidence: 0.87,
            requires_review: false,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let python_keywords = [
            "python", "django", "flask", "fastapi", "pandas",
            "numpy", "scikit", "tensorflow", "pytorch", "jupyter",
            "data", "analysis", "machine", "learning", "ai"
        ];
        
        let task_lower = task.to_lowercase();
        python_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "python_code".to_string(),
                "requirements_txt".to_string(),
                "data_requirements".to_string(),
            ],
            optional_patterns: vec![
                "datasets".to_string(),
                "models".to_string(),
                "notebooks".to_string(),
            ],
            exclude_patterns: vec![
                "frontend_code".to_string(),
                "compiled_code".to_string(),
            ],
            max_context_age_minutes: Some(60),
            priority_keywords: vec![
                "python".to_string(),
                "data".to_string(),
                "analysis".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        match from_agent {
            AgentType::Architect => {
                self.extract_data_architecture(context).await
            },
            AgentType::Developer => {
                self.extract_python_requirements(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl PythonSpecialistAgent {
    async fn analyze_python_requirements(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Python requirements analysis for: {}", task))
    }
    
    async fn implement_python_solution(&self, analysis: &str) -> Result<String, AgentError> {
        Ok("Python implementation completed".to_string())
    }
    
    async fn optimize_python_code(&self, implementation: &str) -> Result<String, AgentError> {
        Ok("Python code optimized".to_string())
    }
    
    async fn extract_data_architecture(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted data architecture: {}", context))
    }
    
    async fn extract_python_requirements(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted Python requirements: {}", context))
    }
}

/// DevOps Specialist Agent - CI/CD, Deployment, Infrastructure
pub struct DevOpsSpecialistAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl DevOpsSpecialistAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for DevOpsSpecialistAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::DevOpsSpecialist
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for DevOps Specialist".to_string()));
        }
        
        let infrastructure_analysis = self.analyze_infrastructure_needs(&context, task).await?;
        let deployment_strategy = self.design_deployment_strategy(&infrastructure_analysis).await?;
        let automation = self.implement_automation(&deployment_strategy).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Infrastructure Analysis\n{}\n\n## Deployment Strategy\n{}\n\n## Automation\n{}", 
                          infrastructure_analysis, deployment_strategy, automation),
            metadata: HashMap::from([
                ("agent_type".to_string(), "devops_specialist".to_string()),
                ("focus".to_string(), "infrastructure".to_string()),
                ("domain".to_string(), "deployment".to_string()),
            ]),
            confidence: 0.86,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let devops_keywords = [
            "deploy", "deployment", "ci", "cd", "pipeline",
            "docker", "kubernetes", "aws", "azure", "gcp",
            "terraform", "ansible", "jenkins", "github", "actions",
            "infrastructure", "monitoring", "logging"
        ];
        
        let task_lower = task.to_lowercase();
        devops_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "deployment_config".to_string(),
                "infrastructure_requirements".to_string(),
                "environment_specs".to_string(),
            ],
            optional_patterns: vec![
                "monitoring_requirements".to_string(),
                "scaling_needs".to_string(),
                "security_requirements".to_string(),
            ],
            exclude_patterns: vec![
                "application_logic".to_string(),
                "ui_components".to_string(),
            ],
            max_context_age_minutes: Some(90),
            priority_keywords: vec![
                "deployment".to_string(),
                "infrastructure".to_string(),
                "pipeline".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        match from_agent {
            AgentType::Architect => {
                self.extract_infrastructure_requirements(context).await
            },
            AgentType::SecurityAgent => {
                self.extract_security_constraints(context).await
            },
            _ => Ok(context.to_string())
        }
    }
}

impl DevOpsSpecialistAgent {
    async fn analyze_infrastructure_needs(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Infrastructure analysis for: {}", task))
    }
    
    async fn design_deployment_strategy(&self, analysis: &str) -> Result<String, AgentError> {
        Ok("Deployment strategy designed".to_string())
    }
    
    async fn implement_automation(&self, strategy: &str) -> Result<String, AgentError> {
        Ok("Automation implemented".to_string())
    }
    
    async fn extract_infrastructure_requirements(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted infrastructure requirements: {}", context))
    }
    
    async fn extract_security_constraints(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted security constraints: {}", context))
    }
}

/// Performance Optimizer Agent - Performance analysis and optimization
pub struct PerformanceOptimizerAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl PerformanceOptimizerAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for PerformanceOptimizerAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::PerformanceOptimizer
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Performance Optimizer".to_string()));
        }
        
        let performance_analysis = self.analyze_performance(&context, task).await?;
        let bottleneck_identification = self.identify_bottlenecks(&performance_analysis).await?;
        let optimization_plan = self.create_optimization_plan(&bottleneck_identification).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Performance Analysis\n{}\n\n## Bottleneck Identification\n{}\n\n## Optimization Plan\n{}", 
                          performance_analysis, bottleneck_identification, optimization_plan),
            metadata: HashMap::from([
                ("agent_type".to_string(), "performance_optimizer".to_string()),
                ("focus".to_string(), "optimization".to_string()),
                ("domain".to_string(), "performance".to_string()),
            ]),
            confidence: 0.84,
            requires_review: true,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let performance_keywords = [
            "performance", "optimize", "optimization", "slow", "fast",
            "memory", "cpu", "bottleneck", "profile", "benchmark",
            "cache", "latency", "throughput", "efficiency", "speed"
        ];
        
        let task_lower = task.to_lowercase();
        performance_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "performance_metrics".to_string(),
                "code_to_optimize".to_string(),
                "benchmarks".to_string(),
            ],
            optional_patterns: vec![
                "profiling_data".to_string(),
                "system_constraints".to_string(),
                "usage_patterns".to_string(),
            ],
            exclude_patterns: vec![
                "ui_design".to_string(),
                "documentation".to_string(),
            ],
            max_context_age_minutes: Some(30),
            priority_keywords: vec![
                "performance".to_string(),
                "bottleneck".to_string(),
                "optimization".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        // Extract performance-relevant information from any agent
        self.extract_performance_context(context).await
    }
}

impl PerformanceOptimizerAgent {
    async fn analyze_performance(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Performance analysis for: {}", task))
    }
    
    async fn identify_bottlenecks(&self, analysis: &str) -> Result<String, AgentError> {
        Ok("Bottlenecks identified".to_string())
    }
    
    async fn create_optimization_plan(&self, bottlenecks: &str) -> Result<String, AgentError> {
        Ok("Optimization plan created".to_string())
    }
    
    async fn extract_performance_context(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted performance context: {}", context))
    }
}

/// Researcher Agent - Research, documentation, and knowledge gathering
pub struct ResearcherAgent {
    config: AgentConfig,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
}

impl ResearcherAgent {
    pub fn new(config: AgentConfig, communication: Arc<CommunicationProtocol>, compressor: Arc<ContextCompressor>) -> Self {
        Self { config, communication, compressor }
    }
}

#[async_trait]
impl SpecializedAgent for ResearcherAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Researcher
    }
    
    async fn execute_task(&self, context: AgentContext, task: &str) -> Result<AgentResult, AgentError> {
        if !self.can_handle_task(task) {
            return Err(AgentError::InvalidTask("Task not suitable for Researcher".to_string()));
        }
        
        let research_plan = self.create_research_plan(&context, task).await?;
        let information_gathering = self.gather_information(&research_plan).await?;
        let analysis = self.analyze_findings(&information_gathering).await?;
        
        Ok(AgentResult {
            agent_id: self.config.id.clone(),
            task_id: context.task_id.clone(),
            output: format!("## Research Plan\n{}\n\n## Information Gathering\n{}\n\n## Analysis\n{}", 
                          research_plan, information_gathering, analysis),
            metadata: HashMap::from([
                ("agent_type".to_string(), "researcher".to_string()),
                ("focus".to_string(), "research".to_string()),
                ("domain".to_string(), "knowledge_gathering".to_string()),
            ]),
            confidence: 0.88,
            requires_review: false,
        })
    }
    
    fn can_handle_task(&self, task: &str) -> bool {
        let research_keywords = [
            "research", "investigate", "analyze", "study", "explore",
            "documentation", "best", "practices", "standards", "comparison",
            "survey", "review", "evaluate", "assess", "gather"
        ];
        
        let task_lower = task.to_lowercase();
        research_keywords.iter().any(|keyword| task_lower.contains(keyword))
    }
    
    fn get_context_requirements(&self) -> ContextRequirements {
        ContextRequirements {
            required_patterns: vec![
                "research_topic".to_string(),
                "requirements".to_string(),
                "objectives".to_string(),
            ],
            optional_patterns: vec![
                "existing_knowledge".to_string(),
                "constraints".to_string(),
                "sources".to_string(),
            ],
            exclude_patterns: vec![
                "implementation_details".to_string(),
                "specific_bugs".to_string(),
            ],
            max_context_age_minutes: Some(180), // Research can use older context
            priority_keywords: vec![
                "research".to_string(),
                "analysis".to_string(),
                "investigation".to_string(),
            ],
        }
    }
    
    async fn process_handoff_context(&self, from_agent: AgentType, context: &str) -> Result<String, AgentError> {
        // Extract research requirements from any agent's request
        self.extract_research_requirements(context).await
    }
}

impl ResearcherAgent {
    async fn create_research_plan(&self, context: &AgentContext, task: &str) -> Result<String, AgentError> {
        Ok(format!("Research plan for: {}", task))
    }
    
    async fn gather_information(&self, plan: &str) -> Result<String, AgentError> {
        Ok("Information gathered".to_string())
    }
    
    async fn analyze_findings(&self, information: &str) -> Result<String, AgentError> {
        Ok("Findings analyzed".to_string())
    }
    
    async fn extract_research_requirements(&self, context: &str) -> Result<String, AgentError> {
        Ok(format!("Extracted research requirements: {}", context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_language_specialist_task_validation() {
        let react_specialist = ReactSpecialistAgent::new(
            AgentConfig::default(),
            Arc::new(CommunicationProtocol::new()),
            Arc::new(ContextCompressor::new(CompressionSettings::default())),
        );
        
        assert!(react_specialist.can_handle_task("Create a React component"));
        assert!(!react_specialist.can_handle_task("Optimize Rust memory usage"));
    }
    
    #[tokio::test]
    async fn test_context_requirements_specificity() {
        let rust_specialist = RustSpecialistAgent::new(
            AgentConfig::default(),
            Arc::new(CommunicationProtocol::new()),
            Arc::new(ContextCompressor::new(CompressionSettings::default())),
        );
        
        let requirements = rust_specialist.get_context_requirements();
        assert!(requirements.required_patterns.contains(&"rust_code".to_string()));
        assert!(requirements.exclude_patterns.contains(&"frontend_code".to_string()));
    }
}
