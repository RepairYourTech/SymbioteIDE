//! # Agent Builder System
//! 
//! The Agent Builder is a specialized Symbiote that can create new Symbiotes
//! based on user specifications. It supports both programmatic creation and
//! visual agent building through the Symbiote IDE interface.

use super::*;
use crate::{Result, SymbioteError, UserId};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent Builder - A specialized Symbiote that creates other Symbiotes
#[derive(Debug)]
pub struct AgentBuilder {
    /// Builder configuration
    config: BuilderConfig,
    
    /// Template library for common agent patterns
    template_library: TemplateLibrary,
    
    /// Validation engine for agent definitions
    validator: AgentValidator,
    
    /// Code generation engine for agent implementation
    code_generator: CodeGenerator,
    
    /// Testing framework for new agents
    test_framework: AgentTestFramework,
    
    /// Registry of created agents
    created_agents: HashMap<SymbioteId, CreatedAgentInfo>,
    
    /// Builder metrics
    metrics: BuilderMetrics,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            config: BuilderConfig::default(),
            template_library: TemplateLibrary::new(),
            validator: AgentValidator::new(),
            code_generator: CodeGenerator::new(),
            test_framework: AgentTestFramework::new(),
            created_agents: HashMap::new(),
            metrics: BuilderMetrics::new(),
        }
    }

    /// Create a new Symbiote from a definition
    pub async fn create_symbiote(&mut self, definition: SymbioteDefinition, user_id: UserId) -> Result<SymbioteId> {
        tracing::info!("Creating new Symbiote: {} for user: {}", definition.name, user_id);

        // Validate the definition
        self.validator.validate_definition(&definition).await?;

        // Generate the Symbiote implementation
        let implementation = self.code_generator.generate_implementation(&definition).await?;

        // Create the Symbiote instance
        let mut symbiote = Symbiote::new(definition.clone());
        
        // Apply the generated implementation
        self.apply_implementation(&mut symbiote, implementation).await?;

        // Test the new Symbiote
        let test_results = self.test_framework.test_symbiote(&symbiote).await?;
        
        if !test_results.passed {
            return Err(SymbioteError::context(format!(
                "Symbiote failed validation tests: {:?}", 
                test_results.failures
            )));
        }

        // Store creation info
        let creation_info = CreatedAgentInfo {
            id: symbiote.id.clone(),
            name: symbiote.name.clone(),
            created_by: user_id,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            definition: definition.clone(),
            test_results,
            usage_count: 0,
            last_used: None,
        };

        self.created_agents.insert(symbiote.id.clone(), creation_info);

        // Update metrics
        self.metrics.agents_created += 1;
        self.metrics.last_creation = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        tracing::info!("Successfully created Symbiote: {}", symbiote.id);
        Ok(symbiote.id)
    }

    /// Create a Symbiote from a visual definition (from the Visual Agent Builder)
    pub async fn create_from_visual_definition(
        &mut self,
        visual_definition: VisualAgentDefinition,
        user_id: UserId,
    ) -> Result<SymbioteId> {
        // Convert visual definition to standard definition
        let definition = self.convert_visual_to_standard(visual_definition).await?;
        
        // Create the Symbiote using standard process
        self.create_symbiote(definition, user_id).await
    }

    /// Get a created Symbiote by ID
    pub async fn get_symbiote(&self, symbiote_id: &SymbioteId) -> Result<Symbiote> {
        if let Some(creation_info) = self.created_agents.get(symbiote_id) {
            // Recreate the Symbiote from its definition
            let symbiote = Symbiote::new(creation_info.definition.clone());
            Ok(symbiote)
        } else {
            Err(SymbioteError::context(format!("Symbiote not found: {}", symbiote_id)))
        }
    }

    /// Get available templates
    pub async fn get_available_templates(&self) -> Result<Vec<AgentTemplate>> {
        self.template_library.get_all_templates().await
    }

    /// Create a Symbiote from a template
    pub async fn create_from_template(
        &mut self,
        template_id: &str,
        customizations: TemplateCustomizations,
        user_id: UserId,
    ) -> Result<SymbioteId> {
        // Get the template
        let template = self.template_library.get_template(template_id).await?;
        
        // Apply customizations to create definition
        let definition = self.apply_template_customizations(template, customizations).await?;
        
        // Create the Symbiote
        self.create_symbiote(definition, user_id).await
    }

    /// Clone an existing Symbiote with modifications
    pub async fn clone_symbiote(
        &mut self,
        source_id: &SymbioteId,
        modifications: SymbioteModifications,
        user_id: UserId,
    ) -> Result<SymbioteId> {
        // Get the source Symbiote
        let source_info = self.created_agents.get(source_id)
            .ok_or_else(|| SymbioteError::context(format!("Source Symbiote not found: {}", source_id)))?;

        // Apply modifications to create new definition
        let mut new_definition = source_info.definition.clone();
        self.apply_modifications(&mut new_definition, modifications).await?;
        
        // Update metadata
        new_definition.name = format!("{} (Clone)", new_definition.name);
        new_definition.version = "1.0.0".to_string();
        new_definition.created_by = user_id.clone();

        // Create the new Symbiote
        self.create_symbiote(new_definition, user_id).await
    }

    /// Get builder statistics
    pub async fn get_statistics(&self) -> Result<BuilderStatistics> {
        Ok(BuilderStatistics {
            total_agents_created: self.metrics.agents_created,
            active_agents: self.created_agents.len() as u64,
            templates_available: self.template_library.template_count().await?,
            success_rate: self.calculate_success_rate().await?,
            popular_templates: self.get_popular_templates().await?,
            recent_creations: self.get_recent_creations().await?,
        })
    }

    /// Export a Symbiote definition for sharing
    pub async fn export_symbiote(&self, symbiote_id: &SymbioteId) -> Result<ExportedSymbiote> {
        let creation_info = self.created_agents.get(symbiote_id)
            .ok_or_else(|| SymbioteError::context(format!("Symbiote not found: {}", symbiote_id)))?;

        Ok(ExportedSymbiote {
            definition: creation_info.definition.clone(),
            metadata: ExportMetadata {
                exported_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                exported_by: creation_info.created_by.clone(),
                original_creator: creation_info.created_by.clone(),
                version: creation_info.definition.version.clone(),
                compatibility_version: "1.0.0".to_string(),
            },
        })
    }

    /// Import a Symbiote definition
    pub async fn import_symbiote(
        &mut self,
        exported_symbiote: ExportedSymbiote,
        user_id: UserId,
    ) -> Result<SymbioteId> {
        // Validate compatibility
        self.validator.validate_compatibility(&exported_symbiote.metadata).await?;
        
        // Create the Symbiote from imported definition
        let mut definition = exported_symbiote.definition;
        definition.created_by = user_id.clone();
        
        self.create_symbiote(definition, user_id).await
    }

    // Private helper methods

    async fn apply_implementation(&self, symbiote: &mut Symbiote, implementation: AgentImplementation) -> Result<()> {
        // Apply the generated implementation to the Symbiote
        // This would involve setting up the Symbiote's behavior, capabilities, etc.
        
        // For now, this is a placeholder
        tracing::debug!("Applied implementation to Symbiote: {}", symbiote.id);
        Ok(())
    }

    async fn convert_visual_to_standard(&self, visual_def: VisualAgentDefinition) -> Result<SymbioteDefinition> {
        // Convert visual definition to standard definition
        let mut definition = SymbioteDefinition {
            name: visual_def.name,
            description: visual_def.description,
            version: "1.0.0".to_string(),
            created_by: visual_def.created_by,
            skills: Vec::new(),
            capabilities: Vec::new(),
            specializations: Vec::new(),
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        // Convert visual components to skills and capabilities
        for component in visual_def.components {
            match component.component_type {
                VisualComponentType::SkillNode => {
                    if let Some(skill) = self.parse_skill_from_component(&component).await? {
                        definition.skills.push(skill);
                    }
                }
                VisualComponentType::CapabilityNode => {
                    if let Some(capability) = self.parse_capability_from_component(&component).await? {
                        definition.capabilities.push(capability);
                    }
                }
                VisualComponentType::SpecializationNode => {
                    if let Some(specialization) = self.parse_specialization_from_component(&component).await? {
                        definition.specializations.push(specialization);
                    }
                }
                _ => {}
            }
        }

        Ok(definition)
    }

    async fn parse_skill_from_component(&self, component: &VisualComponent) -> Result<Option<SymbioteSkill>> {
        // Parse skill from visual component
        match component.properties.get("skill_type") {
            Some(skill_value) => {
                match skill_value.as_str() {
                    Some("programming") => Ok(Some(SymbioteSkill::Programming)),
                    Some("code_generation") => Ok(Some(SymbioteSkill::CodeGeneration)),
                    Some("code_analysis") => Ok(Some(SymbioteSkill::CodeAnalysis)),
                    Some("debugging") => Ok(Some(SymbioteSkill::Debugging)),
                    Some("testing") => Ok(Some(SymbioteSkill::TestGeneration)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn parse_capability_from_component(&self, component: &VisualComponent) -> Result<Option<SymbioteCapability>> {
        // Parse capability from visual component
        match component.properties.get("capability_type") {
            Some(capability_value) => {
                match capability_value.as_str() {
                    Some("file_read") => Ok(Some(SymbioteCapability::FileRead)),
                    Some("file_write") => Ok(Some(SymbioteCapability::FileWrite)),
                    Some("command_execution") => Ok(Some(SymbioteCapability::CommandExecution)),
                    Some("network_access") => Ok(Some(SymbioteCapability::NetworkAccess)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn parse_specialization_from_component(&self, component: &VisualComponent) -> Result<Option<DevelopmentStack>> {
        // Parse specialization from visual component
        match component.properties.get("stack") {
            Some(stack_value) => {
                match stack_value.as_str() {
                    Some("rust") => Ok(Some(DevelopmentStack::Rust)),
                    Some("javascript") => Ok(Some(DevelopmentStack::JavaScript)),
                    Some("python") => Ok(Some(DevelopmentStack::Python)),
                    Some("react") => Ok(Some(DevelopmentStack::React)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn apply_template_customizations(
        &self,
        template: AgentTemplate,
        customizations: TemplateCustomizations,
    ) -> Result<SymbioteDefinition> {
        let mut definition = template.base_definition;
        
        // Apply name customization
        if let Some(name) = customizations.name {
            definition.name = name;
        }
        
        // Apply description customization
        if let Some(description) = customizations.description {
            definition.description = description;
        }
        
        // Apply skill customizations
        if let Some(additional_skills) = customizations.additional_skills {
            definition.skills.extend(additional_skills);
        }
        
        // Apply capability customizations
        if let Some(additional_capabilities) = customizations.additional_capabilities {
            definition.capabilities.extend(additional_capabilities);
        }
        
        // Apply configuration customizations
        if let Some(config_overrides) = customizations.configuration_overrides {
            for (key, value) in config_overrides {
                definition.configuration.custom_settings.insert(key, value);
            }
        }

        Ok(definition)
    }

    async fn apply_modifications(
        &self,
        definition: &mut SymbioteDefinition,
        modifications: SymbioteModifications,
    ) -> Result<()> {
        // Apply modifications to the definition
        if let Some(name) = modifications.name {
            definition.name = name;
        }
        
        if let Some(description) = modifications.description {
            definition.description = description;
        }
        
        if let Some(skills_to_add) = modifications.skills_to_add {
            definition.skills.extend(skills_to_add);
        }
        
        if let Some(skills_to_remove) = modifications.skills_to_remove {
            definition.skills.retain(|skill| !skills_to_remove.contains(skill));
        }
        
        if let Some(capabilities_to_add) = modifications.capabilities_to_add {
            definition.capabilities.extend(capabilities_to_add);
        }
        
        if let Some(capabilities_to_remove) = modifications.capabilities_to_remove {
            definition.capabilities.retain(|cap| !capabilities_to_remove.contains(cap));
        }

        Ok(())
    }

    async fn calculate_success_rate(&self) -> Result<f64> {
        if self.metrics.agents_created == 0 {
            return Ok(1.0);
        }
        
        let successful_agents = self.created_agents.values()
            .filter(|info| info.test_results.passed)
            .count();
        
        Ok(successful_agents as f64 / self.metrics.agents_created as f64)
    }

    async fn get_popular_templates(&self) -> Result<Vec<String>> {
        // Return list of popular template IDs
        Ok(vec![
            "rust-developer".to_string(),
            "web-developer".to_string(),
            "tester".to_string(),
        ])
    }

    async fn get_recent_creations(&self) -> Result<Vec<RecentCreation>> {
        let mut recent: Vec<_> = self.created_agents.values()
            .map(|info| RecentCreation {
                id: info.id.clone(),
                name: info.name.clone(),
                created_at: info.created_at,
                created_by: info.created_by.clone(),
            })
            .collect();
        
        recent.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        recent.truncate(10); // Return last 10
        
        Ok(recent)
    }
}

/// Builder configuration
#[derive(Debug, Clone)]
pub struct BuilderConfig {
    pub max_agents_per_user: u32,
    pub validation_timeout_seconds: u64,
    pub test_timeout_seconds: u64,
    pub enable_advanced_features: bool,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            max_agents_per_user: 100,
            validation_timeout_seconds: 30,
            test_timeout_seconds: 60,
            enable_advanced_features: true,
        }
    }
}

/// Visual agent definition from the Visual Agent Builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualAgentDefinition {
    pub name: String,
    pub description: String,
    pub created_by: UserId,
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub layout: VisualLayout,
}

/// Visual component in the agent builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualComponent {
    pub id: String,
    pub component_type: VisualComponentType,
    pub position: Position,
    pub properties: HashMap<String, serde_json::Value>,
}

/// Types of visual components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisualComponentType {
    SkillNode,
    CapabilityNode,
    SpecializationNode,
    ConfigurationNode,
    InputNode,
    OutputNode,
    ProcessingNode,
    DecisionNode,
}

/// Visual connection between components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualConnection {
    pub id: String,
    pub source_component: String,
    pub target_component: String,
    pub connection_type: ConnectionType,
}

/// Types of connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    DataFlow,
    ControlFlow,
    Dependency,
    Trigger,
}

/// Visual layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualLayout {
    pub canvas_size: Size,
    pub zoom_level: f32,
    pub viewport: Viewport,
}

/// Position in the visual builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Size dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// Viewport information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Template customizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCustomizations {
    pub name: Option<String>,
    pub description: Option<String>,
    pub additional_skills: Option<Vec<SymbioteSkill>>,
    pub additional_capabilities: Option<Vec<SymbioteCapability>>,
    pub configuration_overrides: Option<HashMap<String, serde_json::Value>>,
}

/// Symbiote modifications for cloning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteModifications {
    pub name: Option<String>,
    pub description: Option<String>,
    pub skills_to_add: Option<Vec<SymbioteSkill>>,
    pub skills_to_remove: Option<Vec<SymbioteSkill>>,
    pub capabilities_to_add: Option<Vec<SymbioteCapability>>,
    pub capabilities_to_remove: Option<Vec<SymbioteCapability>>,
    pub configuration_changes: Option<HashMap<String, serde_json::Value>>,
}

/// Information about a created agent
#[derive(Debug, Clone)]
pub struct CreatedAgentInfo {
    pub id: SymbioteId,
    pub name: String,
    pub created_by: UserId,
    pub created_at: u64,
    pub definition: SymbioteDefinition,
    pub test_results: AgentTestResults,
    pub usage_count: u64,
    pub last_used: Option<u64>,
}

/// Builder metrics
#[derive(Debug, Clone)]
pub struct BuilderMetrics {
    pub agents_created: u64,
    pub agents_failed: u64,
    pub last_creation: u64,
    pub total_build_time: u64,
}

impl BuilderMetrics {
    pub fn new() -> Self {
        Self {
            agents_created: 0,
            agents_failed: 0,
            last_creation: 0,
            total_build_time: 0,
        }
    }
}

/// Builder statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderStatistics {
    pub total_agents_created: u64,
    pub active_agents: u64,
    pub templates_available: u64,
    pub success_rate: f64,
    pub popular_templates: Vec<String>,
    pub recent_creations: Vec<RecentCreation>,
}

/// Recent creation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentCreation {
    pub id: SymbioteId,
    pub name: String,
    pub created_at: u64,
    pub created_by: UserId,
}

/// Exported Symbiote for sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSymbiote {
    pub definition: SymbioteDefinition,
    pub metadata: ExportMetadata,
}

/// Export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub exported_at: u64,
    pub exported_by: UserId,
    pub original_creator: UserId,
    pub version: String,
    pub compatibility_version: String,
}

/// Agent implementation generated by the code generator
#[derive(Debug, Clone)]
pub struct AgentImplementation {
    pub behavior_code: String,
    pub configuration: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
}

/// Agent test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTestResults {
    pub passed: bool,
    pub total_tests: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub failures: Vec<TestFailure>,
    pub execution_time_ms: u64,
}

/// Test failure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}
