//! # 3D Graph Builder
//! 
//! Builds 3D knowledge graphs from workspace data, integrating all components
//! including files, tasks, notes, conversations, goals, and code relationships.

use super::*;
use super::graph_3d::*;
use super::components::*;
use crate::codebase_index::*;

/// Builder for creating 3D workspace knowledge graphs
#[derive(Debug)]
pub struct Workspace3DGraphBuilder {
    workspace_id: String,
    graph: Workspace3DGraph,
    node_positions: HashMap<String, Vector3D>,
    layout_engine: LayoutEngine,
}

impl Workspace3DGraphBuilder {
    /// Create new graph builder
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id: workspace_id.clone(),
            graph: Workspace3DGraph::new(workspace_id),
            node_positions: HashMap::new(),
            layout_engine: LayoutEngine::new(),
        }
    }

    /// Build complete 3D graph from workspace context
    pub async fn build_from_workspace(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<Workspace3DGraph> {
        // Build nodes from all workspace components
        self.build_file_nodes(workspace).await?;
        self.build_task_nodes(workspace).await?;
        self.build_note_nodes(workspace).await?;
        self.build_conversation_nodes(workspace).await?;
        self.build_goal_nodes(workspace).await?;
        self.build_journal_nodes(workspace).await?;
        self.build_workflow_nodes(workspace).await?;
        self.build_agent_nodes(workspace).await?;
        self.build_codebase_nodes(workspace).await?;

        // Build relationships between nodes
        self.build_file_relationships(workspace).await?;
        self.build_task_relationships(workspace).await?;
        self.build_note_relationships(workspace).await?;
        self.build_conversation_relationships(workspace).await?;
        self.build_goal_relationships(workspace).await?;
        self.build_semantic_relationships(workspace).await?;
        self.build_temporal_relationships(workspace).await?;

        // Create clusters
        self.create_clusters(workspace).await?;

        // Apply layout
        self.apply_layout().await?;

        Ok(self.graph.clone())
    }

    /// Build file nodes from workspace files
    async fn build_file_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (file_id, file_context) in &workspace.files {
            let position = self.layout_engine.get_file_position(file_id);
            
            let node = Graph3DNode {
                id: file_id.clone(),
                node_type: NodeType::File,
                label: std::path::Path::new(&file_context.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&file_context.path)
                    .to_string(),
                description: Some(format!("File: {}", file_context.path)),
                position,
                size: self.calculate_file_size(&file_context),
                color: self.get_file_color(&file_context),
                shape: NodeShape::Cube,
                opacity: 0.8,
                metadata: HashMap::new(),
                importance: self.calculate_file_importance(&file_context),
                activity_level: self.calculate_file_activity(&file_context),
                connections: Vec::new(),
                entity_data: EntityData::File {
                    path: file_context.path.clone(),
                    language: file_context.language.clone(),
                    size: file_context.size.unwrap_or(0),
                    last_modified: chrono::DateTime::from_timestamp(file_context.last_modified as i64, 0)
                        .unwrap_or_else(|| chrono::Utc::now()),
                },
                created_at: chrono::DateTime::from_timestamp(file_context.last_modified as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
                updated_at: chrono::DateTime::from_timestamp(file_context.last_modified as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build task nodes from workspace tasks
    async fn build_task_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (task_id, task) in &workspace.tasks.tasks {
            let position = self.layout_engine.get_task_position(task_id);
            
            let node = Graph3DNode {
                id: task_id.clone(),
                node_type: NodeType::Task,
                label: task.title.clone(),
                description: task.description.clone(),
                position,
                size: self.calculate_task_size(task),
                color: self.get_task_color(task),
                shape: NodeShape::Sphere,
                opacity: 0.9,
                metadata: HashMap::new(),
                importance: self.calculate_task_importance(task),
                activity_level: self.calculate_task_activity(task),
                connections: Vec::new(),
                entity_data: EntityData::Task {
                    status: format!("{:?}", task.status),
                    priority: format!("{:?}", task.priority),
                    due_date: task.due_date,
                    progress: match task.status {
                        crate::workspace::components::TaskStatus::Done => 1.0,
                        _ => 0.5,
                    },
                },
                created_at: task.created_at,
                updated_at: task.updated_at,
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build note nodes from workspace notes
    async fn build_note_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (note_id, note) in &workspace.notes.notes {
            let position = self.layout_engine.get_note_position(note_id);
            
            let node = Graph3DNode {
                id: note_id.clone(),
                node_type: NodeType::Note,
                label: note.title.clone(),
                description: Some(note.content.chars().take(100).collect::<String>()),
                position,
                size: self.calculate_note_size(note),
                color: self.get_note_color(note),
                shape: NodeShape::Pyramid,
                opacity: 0.7,
                metadata: HashMap::new(),
                importance: self.calculate_note_importance(note),
                activity_level: self.calculate_note_activity(note),
                connections: Vec::new(),
                entity_data: EntityData::Note {
                    note_type: format!("{:?}", note.note_type),
                    word_count: note.content.split_whitespace().count(),
                    tags: note.tags.clone(),
                },
                created_at: note.created_at,
                updated_at: note.updated_at,
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build conversation nodes
    async fn build_conversation_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (conv_id, conversation) in &workspace.conversations {
            let position = self.layout_engine.get_conversation_position(conv_id);
            
            let node = Graph3DNode {
                id: conv_id.clone(),
                node_type: NodeType::Conversation,
                label: conversation.title.clone(),
                description: Some("Workspace conversation".to_string()),
                position,
                size: 5.0,
                color: "#4CAF50".to_string(),
                shape: NodeShape::Octahedron,
                opacity: 0.8,
                metadata: HashMap::new(),
                importance: 0.7,
                activity_level: 0.8,
                connections: Vec::new(),
                entity_data: EntityData::Conversation {
                    participant_count: conversation.participants.len(),
                    message_count: conversation.message_count.unwrap_or(0),
                    last_activity: chrono::DateTime::from_timestamp(conversation.last_activity as i64, 0)
                        .unwrap_or_else(|| chrono::Utc::now()),
                },
                created_at: chrono::DateTime::from_timestamp(conversation.created_at as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
                updated_at: chrono::DateTime::from_timestamp(conversation.last_activity as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build goal nodes from workspace goals
    async fn build_goal_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (goal_id, goal) in &workspace.goals.goals {
            let position = self.layout_engine.get_goal_position(goal_id);
            
            let node = Graph3DNode {
                id: goal_id.clone(),
                node_type: NodeType::Goal,
                label: goal.title.clone(),
                description: goal.description.clone(),
                position,
                size: self.calculate_goal_size(goal),
                color: self.get_goal_color(goal),
                shape: NodeShape::Cone,
                opacity: 0.9,
                metadata: HashMap::new(),
                importance: self.calculate_goal_importance(goal),
                activity_level: self.calculate_goal_activity(goal),
                connections: Vec::new(),
                entity_data: EntityData::Goal {
                    progress: goal.progress,
                    target_date: goal.target_date.map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc()),
                    status: format!("{:?}", goal.status),
                },
                created_at: goal.created_at,
                updated_at: goal.updated_at,
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build journal nodes
    async fn build_journal_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (date, entry) in &workspace.journal.daily_entries {
            let position = self.layout_engine.get_journal_position(&entry.id);
            
            let node = Graph3DNode {
                id: entry.id.clone(),
                node_type: NodeType::Journal,
                label: format!("Journal - {}", date.format("%Y-%m-%d")),
                description: Some(entry.title.clone()),
                position,
                size: 3.0,
                color: "#FF9800".to_string(),
                shape: NodeShape::Cylinder,
                opacity: 0.6,
                metadata: HashMap::new(),
                importance: 0.6,
                activity_level: 0.5,
                connections: Vec::new(),
                entity_data: EntityData::Generic {
                    data: HashMap::new(),
                },
                created_at: entry.created_at,
                updated_at: entry.updated_at,
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build workflow nodes
    async fn build_workflow_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (workflow_id, workflow) in &workspace.workflows {
            let position = self.layout_engine.get_workflow_position(workflow_id);
            
            let node = Graph3DNode {
                id: workflow_id.clone(),
                node_type: NodeType::Workflow,
                label: workflow.name.clone(),
                description: Some(workflow.description.clone().unwrap_or_default()),
                position,
                size: 6.0,
                color: "#9C27B0".to_string(),
                shape: NodeShape::Torus,
                opacity: 0.8,
                metadata: HashMap::new(),
                importance: 0.8,
                activity_level: 0.7,
                connections: Vec::new(),
                entity_data: EntityData::Generic {
                    data: HashMap::new(),
                },
                created_at: chrono::DateTime::from_timestamp(workflow.created_at as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
                updated_at: chrono::DateTime::from_timestamp(workflow.updated_at as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build agent nodes
    async fn build_agent_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (agent_id, agent) in &workspace.agents {
            let position = self.layout_engine.get_agent_position(agent_id);
            
            let node = Graph3DNode {
                id: agent_id.clone(),
                node_type: NodeType::Agent,
                label: agent.agent_type.clone(),
                description: Some("Workspace agent".to_string()),
                position,
                size: 4.0,
                color: "#2196F3".to_string(),
                shape: NodeShape::Octahedron,
                opacity: 0.9,
                metadata: HashMap::new(),
                importance: 0.9,
                activity_level: 0.8,
                connections: Vec::new(),
                entity_data: EntityData::Agent {
                    agent_type: agent.agent_type.clone(),
                    status: format!("{:?}", agent.status),
                    last_activity: agent.last_activity,
                },
                created_at: Utc::now(),
                updated_at: agent.last_activity,
            };

            self.graph.add_node(node);
        }
        Ok(())
    }

    /// Build codebase nodes from index
    async fn build_codebase_nodes(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        // Build symbol nodes from codebase index
        for (symbol_name, symbols) in &workspace.codebase_index.symbols {
            for symbol in symbols {
                let position = self.layout_engine.get_symbol_position(&symbol.name);
                
                let node = Graph3DNode {
                    id: format!("symbol_{}", symbol.name),
                    node_type: NodeType::Symbol,
                    label: symbol.name.clone(),
                    description: symbol.documentation.clone(),
                    position,
                    size: 2.0,
                    color: self.get_symbol_color(symbol),
                    shape: NodeShape::Sphere,
                    opacity: 0.6,
                    metadata: HashMap::new(),
                    importance: 0.5,
                    activity_level: 0.4,
                    connections: Vec::new(),
                    entity_data: EntityData::Generic {
                        data: HashMap::new(),
                    },
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                self.graph.add_node(node);
            }
        }
        Ok(())
    }

    /// Build relationships between components
    async fn build_task_relationships(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (task_id, task) in &workspace.tasks.tasks {
            // Task dependencies
            for dep_id in &task.dependencies {
                let edge = Graph3DEdge {
                    id: Uuid::new_v4().to_string(),
                    source_id: dep_id.clone(),
                    target_id: task_id.clone(),
                    edge_type: EdgeType::Blocks,
                    label: Some("blocks".to_string()),
                    thickness: 2.0,
                    color: "#FF5722".to_string(),
                    style: EdgeStyle::Arrow,
                    opacity: 0.8,
                    weight: 0.9,
                    bidirectional: false,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                };
                self.graph.add_edge(edge);
            }

            // Task subtasks
            for subtask_id in &task.subtasks {
                let edge = Graph3DEdge {
                    id: Uuid::new_v4().to_string(),
                    source_id: task_id.clone(),
                    target_id: subtask_id.clone(),
                    edge_type: EdgeType::Subtask,
                    label: Some("subtask".to_string()),
                    thickness: 1.5,
                    color: "#4CAF50".to_string(),
                    style: EdgeStyle::Solid,
                    opacity: 0.7,
                    weight: 0.8,
                    bidirectional: false,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                };
                self.graph.add_edge(edge);
            }
        }
        Ok(())
    }

    /// Build note relationships
    async fn build_note_relationships(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (note_id, note) in &workspace.notes.notes {
            // Note links
            for link in &note.links {
                let edge = Graph3DEdge {
                    id: Uuid::new_v4().to_string(),
                    source_id: note_id.clone(),
                    target_id: link.target_id.clone(),
                    edge_type: EdgeType::Links,
                    label: link.description.clone(),
                    thickness: 1.0,
                    color: "#9E9E9E".to_string(),
                    style: EdgeStyle::Dashed,
                    opacity: 0.6,
                    weight: 0.5,
                    bidirectional: true,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                };
                self.graph.add_edge(edge);
            }
        }
        Ok(())
    }

    /// Build goal relationships
    async fn build_goal_relationships(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        for (goal_id, goal) in &workspace.goals.goals {
            // Goal-task relationships
            for task_id in &goal.related_tasks {
                let edge = Graph3DEdge {
                    id: Uuid::new_v4().to_string(),
                    source_id: goal_id.clone(),
                    target_id: task_id.clone(),
                    edge_type: EdgeType::RelatedTo,
                    label: Some("supports".to_string()),
                    thickness: 2.0,
                    color: "#673AB7".to_string(),
                    style: EdgeStyle::Arrow,
                    opacity: 0.8,
                    weight: 0.9,
                    bidirectional: false,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                };
                self.graph.add_edge(edge);
            }
        }
        Ok(())
    }

    /// Create clusters for related nodes
    async fn create_clusters(&mut self, workspace: &EnhancedWorkspaceContext) -> Result<()> {
        // File system clusters
        let file_nodes: Vec<String> = self.graph.get_nodes_by_type(&NodeType::File)
            .iter().map(|n| n.id.clone()).collect();
        if !file_nodes.is_empty() {
            self.graph.create_cluster("Files".to_string(), ClusterType::FileSystem, file_nodes);
        }

        // Task clusters by project
        for (project_id, project) in &workspace.tasks.projects {
            let project_task_nodes: Vec<String> = project.task_ids.clone();
            if !project_task_nodes.is_empty() {
                self.graph.create_cluster(
                    project.name.clone(),
                    ClusterType::Project,
                    project_task_nodes
                );
            }
        }

        // Note clusters by notebook
        for (notebook_id, notebook) in &workspace.notes.notebooks {
            let notebook_note_nodes: Vec<String> = notebook.note_ids.clone();
            if !notebook_note_nodes.is_empty() {
                self.graph.create_cluster(
                    notebook.name.clone(),
                    ClusterType::Custom("notebook".to_string()),
                    notebook_note_nodes
                );
            }
        }

        Ok(())
    }

    /// Apply layout algorithm
    async fn apply_layout(&mut self) -> Result<()> {
        self.graph.apply_force_layout(100);
        Ok(())
    }

    // Helper methods for calculating node properties
    fn calculate_file_size(&self, file_context: &crate::context::FileContext) -> f64 {
        (file_context.size.unwrap_or(1000) as f64).log10().max(1.0)
    }

    fn get_file_color(&self, file_context: &crate::context::FileContext) -> String {
        match file_context.language.as_str() {
            "rust" => "#CE422B".to_string(),
            "javascript" => "#F7DF1E".to_string(),
            "typescript" => "#3178C6".to_string(),
            "python" => "#3776AB".to_string(),
            _ => "#757575".to_string(),
        }
    }

    fn calculate_file_importance(&self, file_context: &crate::context::FileContext) -> f64 {
        // Calculate based on file size, modification frequency, etc.
        0.7
    }

    fn calculate_file_activity(&self, file_context: &crate::context::FileContext) -> f64 {
        // Calculate based on recent modifications
        let now_timestamp = Utc::now().timestamp() as u64;
        let days_since_modified = (now_timestamp - file_context.last_modified) / (24 * 60 * 60);
        (1.0 / (days_since_modified as f64 + 1.0)).min(1.0)
    }

    fn calculate_task_size(&self, task: &Task) -> f64 {
        match task.priority {
            TaskPriority::Urgent => 8.0,
            TaskPriority::High => 6.0,
            TaskPriority::Medium => 4.0,
            TaskPriority::Low => 2.0,
        }
    }

    fn get_task_color(&self, task: &Task) -> String {
        match task.status {
            TaskStatus::Todo => "#2196F3".to_string(),
            TaskStatus::InProgress => "#FF9800".to_string(),
            TaskStatus::Review => "#9C27B0".to_string(),
            TaskStatus::Done => "#4CAF50".to_string(),
            TaskStatus::Cancelled => "#757575".to_string(),
        }
    }

    fn calculate_task_importance(&self, task: &Task) -> f64 {
        match task.priority {
            TaskPriority::Urgent => 1.0,
            TaskPriority::High => 0.8,
            TaskPriority::Medium => 0.6,
            TaskPriority::Low => 0.4,
        }
    }

    fn calculate_task_activity(&self, task: &Task) -> f64 {
        let days_since_updated = (Utc::now() - task.updated_at).num_days();
        (1.0 / (days_since_updated as f64 + 1.0)).min(1.0)
    }

    fn calculate_note_size(&self, note: &Note) -> f64 {
        (note.content.len() as f64 / 100.0).max(1.0).min(10.0)
    }

    fn get_note_color(&self, note: &Note) -> String {
        match note.note_type {
            NoteType::Text => "#607D8B".to_string(),
            NoteType::Markdown => "#795548".to_string(),
            NoteType::Code => "#FF5722".to_string(),
            NoteType::Meeting => "#3F51B5".to_string(),
            NoteType::Research => "#009688".to_string(),
            NoteType::Idea => "#FFEB3B".to_string(),
        }
    }

    fn calculate_note_importance(&self, note: &Note) -> f64 {
        // Calculate based on content length, tags, links
        let base_importance = (note.content.len() as f64 / 1000.0).min(1.0);
        let tag_bonus = note.tags.len() as f64 * 0.1;
        let link_bonus = note.links.len() as f64 * 0.1;
        (base_importance + tag_bonus + link_bonus).min(1.0)
    }

    fn calculate_note_activity(&self, note: &Note) -> f64 {
        let days_since_updated = (Utc::now() - note.updated_at).num_days();
        (1.0 / (days_since_updated as f64 + 1.0)).min(1.0)
    }

    fn calculate_goal_size(&self, goal: &Goal) -> f64 {
        match goal.priority {
            GoalPriority::Critical => 10.0,
            GoalPriority::High => 8.0,
            GoalPriority::Medium => 6.0,
            GoalPriority::Low => 4.0,
        }
    }

    fn get_goal_color(&self, goal: &Goal) -> String {
        match goal.status {
            GoalStatus::Draft => "#9E9E9E".to_string(),
            GoalStatus::Active => "#2196F3".to_string(),
            GoalStatus::Paused => "#FF9800".to_string(),
            GoalStatus::Completed => "#4CAF50".to_string(),
            GoalStatus::Cancelled => "#F44336".to_string(),
        }
    }

    fn calculate_goal_importance(&self, goal: &Goal) -> f64 {
        match goal.priority {
            GoalPriority::Critical => 1.0,
            GoalPriority::High => 0.8,
            GoalPriority::Medium => 0.6,
            GoalPriority::Low => 0.4,
        }
    }

    fn calculate_goal_activity(&self, goal: &Goal) -> f64 {
        // Calculate based on progress and recent updates
        let progress_factor = goal.progress;
        let days_since_updated = (Utc::now() - goal.updated_at).num_days();
        let recency_factor = (1.0 / (days_since_updated as f64 + 1.0)).min(1.0);
        (progress_factor + recency_factor) / 2.0
    }

    fn get_symbol_color(&self, symbol: &SymbolDefinition) -> String {
        match symbol.symbol_type {
            SymbolType::Function => "#4CAF50".to_string(),
            SymbolType::Class => "#2196F3".to_string(),
            SymbolType::Interface => "#9C27B0".to_string(),
            SymbolType::Variable => "#FF9800".to_string(),
            SymbolType::Constant => "#F44336".to_string(),
            _ => "#757575".to_string(),
        }
    }

    // Placeholder implementations
    async fn build_file_relationships(&mut self, _workspace: &EnhancedWorkspaceContext) -> Result<()> { Ok(()) }
    async fn build_conversation_relationships(&mut self, _workspace: &EnhancedWorkspaceContext) -> Result<()> { Ok(()) }
    async fn build_semantic_relationships(&mut self, _workspace: &EnhancedWorkspaceContext) -> Result<()> { Ok(()) }
    async fn build_temporal_relationships(&mut self, _workspace: &EnhancedWorkspaceContext) -> Result<()> { Ok(()) }
}

/// Layout engine for positioning nodes
#[derive(Debug)]
pub struct LayoutEngine {
    position_cache: HashMap<String, Vector3D>,
    layout_algorithm: LayoutAlgorithm,
}

#[derive(Debug)]
pub enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            position_cache: HashMap::new(),
            layout_algorithm: LayoutAlgorithm::ForceDirected,
        }
    }

    pub fn get_file_position(&mut self, file_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(file_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(file_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_task_position(&mut self, task_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(task_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(task_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_note_position(&mut self, note_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(note_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(note_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_conversation_position(&mut self, conv_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(conv_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(conv_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_goal_position(&mut self, goal_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(goal_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(goal_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_journal_position(&mut self, journal_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(journal_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(journal_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_workflow_position(&mut self, workflow_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(workflow_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(workflow_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_agent_position(&mut self, agent_id: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(agent_id) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(agent_id.to_string(), position.clone());
            position
        }
    }

    pub fn get_symbol_position(&mut self, symbol_name: &str) -> Vector3D {
        if let Some(position) = self.position_cache.get(symbol_name) {
            position.clone()
        } else {
            let position = self.generate_random_position();
            self.position_cache.insert(symbol_name.to_string(), position.clone());
            position
        }
    }

    fn generate_random_position(&self) -> Vector3D {
        Vector3D::new(
            (rand::random::<f64>() - 0.5) * 100.0,
            (rand::random::<f64>() - 0.5) * 100.0,
            (rand::random::<f64>() - 0.5) * 100.0,
        )
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}
