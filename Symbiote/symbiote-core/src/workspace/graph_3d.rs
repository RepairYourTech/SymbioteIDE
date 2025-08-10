//! # 3D Workspace Knowledge Graph
//! 
//! 3D visualization and representation of workspace knowledge graphs,
//! showing relationships between files, conversations, tasks, goals, and all workspace entities.

use super::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 3D workspace knowledge graph for visualization
#[derive(Debug, Clone)]
pub struct Workspace3DGraph {
    pub workspace_id: String,
    pub nodes: HashMap<String, Graph3DNode>,
    pub edges: HashMap<String, Graph3DEdge>,
    pub clusters: HashMap<String, Graph3DCluster>,
    pub layout: Graph3DLayout,
    pub settings: Graph3DSettings,
    pub last_updated: DateTime<Utc>,
}

/// 3D graph node representing any workspace entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph3DNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub description: Option<String>,
    
    /// 3D position
    pub position: Vector3D,
    
    /// Visual properties
    pub size: f64,
    pub color: String,
    pub shape: NodeShape,
    pub opacity: f64,
    
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    pub importance: f64, // 0.0 to 1.0
    pub activity_level: f64, // Recent activity
    pub connections: Vec<String>, // Connected node IDs
    
    /// Entity-specific data
    pub entity_data: EntityData,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 3D graph edge representing relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph3DEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub edge_type: EdgeType,
    pub label: Option<String>,
    
    /// Visual properties
    pub thickness: f64,
    pub color: String,
    pub style: EdgeStyle,
    pub opacity: f64,
    
    /// Relationship strength
    pub weight: f64, // 0.0 to 1.0
    pub bidirectional: bool,
    
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    pub created_at: DateTime<Utc>,
}

/// 3D graph cluster for grouping related nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph3DCluster {
    pub id: String,
    pub name: String,
    pub cluster_type: ClusterType,
    pub node_ids: Vec<String>,
    
    /// 3D bounding box
    pub bounds: BoundingBox3D,
    
    /// Visual properties
    pub color: String,
    pub opacity: f64,
    pub show_boundary: bool,
    
    pub created_at: DateTime<Utc>,
}

/// 3D layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph3DLayout {
    pub layout_type: LayoutType,
    pub center: Vector3D,
    pub scale: f64,
    pub spacing: f64,
    
    /// Force-directed layout parameters
    pub force_strength: f64,
    pub repulsion_strength: f64,
    pub attraction_strength: f64,
    pub damping: f64,
    
    /// Animation settings
    pub animation_enabled: bool,
    pub animation_speed: f64,
    pub auto_rotate: bool,
    pub rotation_speed: f64,
}

/// 3D visualization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph3DSettings {
    /// Rendering settings
    pub show_labels: bool,
    pub show_edges: bool,
    pub show_clusters: bool,
    pub enable_physics: bool,
    pub enable_collision: bool,
    
    /// Interaction settings
    pub allow_node_dragging: bool,
    pub allow_zoom: bool,
    pub allow_rotation: bool,
    pub selection_enabled: bool,
    
    /// Filtering settings
    pub node_type_filters: Vec<NodeType>,
    pub edge_type_filters: Vec<EdgeType>,
    pub importance_threshold: f64,
    pub activity_threshold: f64,
    
    /// Performance settings
    pub max_nodes: usize,
    pub max_edges: usize,
    pub level_of_detail: bool,
    pub frustum_culling: bool,
}

/// Node types in the workspace graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    // Files and code
    File,
    Function,
    Class,
    Module,
    Symbol,
    
    // Conversations and communication
    Conversation,
    Message,
    Thread,
    
    // Tasks and project management
    Task,
    Project,
    Milestone,
    Goal,
    
    // Documentation and notes
    Note,
    Journal,
    Research,
    Documentation,
    
    // Workflows and automation
    Workflow,
    WorkflowNode,
    Trigger,
    Action,
    
    // Notebooks and data
    Notebook,
    Cell,
    Variable,
    Dataset,
    
    // Agents and AI
    Agent,
    AgentRule,
    Memory,
    
    // People and collaboration
    User,
    Team,
    Collaborator,
    
    // External resources
    URL,
    Repository,
    API,
    Service,
    
    // Abstract concepts
    Concept,
    Topic,
    Tag,
    Category,
}

/// Edge types representing relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeType {
    // Code relationships
    Imports,
    Exports,
    Calls,
    Inherits,
    Implements,
    References,
    
    // File relationships
    Contains,
    DependsOn,
    Includes,
    Links,
    
    // Conversation relationships
    Replies,
    Mentions,
    Discusses,
    
    // Task relationships
    Blocks,
    Subtask,
    RelatedTo,
    Assigned,
    
    // Workflow relationships
    Triggers,
    Executes,
    Follows,
    
    // Collaboration relationships
    CreatedBy,
    ModifiedBy,
    ReviewedBy,
    AssignedTo,
    
    // Semantic relationships
    Similar,
    Related,
    Tagged,
    Categorized,
    
    // Temporal relationships
    Before,
    After,
    During,
    
    // Generic relationships
    Uses,
    Produces,
    Consumes,
    Connects,
}

/// Cluster types for grouping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterType {
    FileSystem,
    Project,
    Feature,
    Conversation,
    Workflow,
    Team,
    Topic,
    TimeRange,
    Custom(String),
}

/// Node shapes for 3D visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeShape {
    Sphere,
    Cube,
    Cylinder,
    Cone,
    Pyramid,
    Octahedron,
    Torus,
    Custom(String),
}

/// Edge styles for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeStyle {
    Solid,
    Dashed,
    Dotted,
    Arrow,
    DoubleArrow,
    Curved,
    Straight,
}

/// Layout algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutType {
    ForceDirected,
    Hierarchical,
    Circular,
    Grid,
    Sphere,
    Helix,
    Tree,
    Custom(String),
}

/// 3D vector for positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// 3D bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox3D {
    pub min: Vector3D,
    pub max: Vector3D,
}

/// Entity-specific data for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityData {
    File {
        path: String,
        language: String,
        size: u64,
        last_modified: DateTime<Utc>,
    },
    Task {
        status: String,
        priority: String,
        due_date: Option<DateTime<Utc>>,
        progress: f64,
    },
    Conversation {
        participant_count: usize,
        message_count: usize,
        last_activity: DateTime<Utc>,
    },
    Goal {
        progress: f64,
        target_date: Option<DateTime<Utc>>,
        status: String,
    },
    Note {
        note_type: String,
        word_count: usize,
        tags: Vec<String>,
    },
    Agent {
        agent_type: String,
        status: String,
        last_activity: DateTime<Utc>,
    },
    Generic {
        data: HashMap<String, serde_json::Value>,
    },
}

impl Workspace3DGraph {
    /// Create new 3D graph for workspace
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            nodes: HashMap::new(),
            edges: HashMap::new(),
            clusters: HashMap::new(),
            layout: Graph3DLayout::default(),
            settings: Graph3DSettings::default(),
            last_updated: Utc::now(),
        }
    }

    /// Add node to the graph
    pub fn add_node(&mut self, node: Graph3DNode) {
        self.nodes.insert(node.id.clone(), node);
        self.last_updated = Utc::now();
    }

    /// Add edge to the graph
    pub fn add_edge(&mut self, edge: Graph3DEdge) {
        self.edges.insert(edge.id.clone(), edge);
        self.last_updated = Utc::now();
    }

    /// Create cluster from nodes
    pub fn create_cluster(&mut self, name: String, cluster_type: ClusterType, node_ids: Vec<String>) -> String {
        let cluster_id = Uuid::new_v4().to_string();
        
        // Calculate bounding box from node positions
        let bounds = self.calculate_bounding_box(&node_ids);
        
        let cluster = Graph3DCluster {
            id: cluster_id.clone(),
            name,
            cluster_type,
            node_ids,
            bounds,
            color: "#rgba(100, 100, 100, 0.3)".to_string(),
            opacity: 0.3,
            show_boundary: true,
            created_at: Utc::now(),
        };
        
        self.clusters.insert(cluster_id.clone(), cluster);
        self.last_updated = Utc::now();
        
        cluster_id
    }

    /// Update node position
    pub fn update_node_position(&mut self, node_id: &str, position: Vector3D) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.position = position;
            node.updated_at = Utc::now();
            self.last_updated = Utc::now();
        }
    }

    /// Get nodes by type
    pub fn get_nodes_by_type(&self, node_type: &NodeType) -> Vec<&Graph3DNode> {
        self.nodes.values()
            .filter(|node| &node.node_type == node_type)
            .collect()
    }

    /// Get edges by type
    pub fn get_edges_by_type(&self, edge_type: &EdgeType) -> Vec<&Graph3DEdge> {
        self.edges.values()
            .filter(|edge| &edge.edge_type == edge_type)
            .collect()
    }

    /// Find shortest path between nodes
    pub fn find_path(&self, start_id: &str, end_id: &str) -> Option<Vec<String>> {
        // Would implement pathfinding algorithm (Dijkstra, A*, etc.)
        None
    }

    /// Get node neighbors
    pub fn get_neighbors(&self, node_id: &str) -> Vec<&Graph3DNode> {
        let mut neighbors = Vec::new();
        
        for edge in self.edges.values() {
            if edge.source_id == node_id {
                if let Some(target) = self.nodes.get(&edge.target_id) {
                    neighbors.push(target);
                }
            } else if edge.target_id == node_id && edge.bidirectional {
                if let Some(source) = self.nodes.get(&edge.source_id) {
                    neighbors.push(source);
                }
            }
        }
        
        neighbors
    }

    /// Calculate graph statistics
    pub fn get_statistics(&self) -> Graph3DStatistics {
        Graph3DStatistics {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            cluster_count: self.clusters.len(),
            node_type_distribution: self.calculate_node_type_distribution(),
            edge_type_distribution: self.calculate_edge_type_distribution(),
            average_connections: self.calculate_average_connections(),
            graph_density: self.calculate_graph_density(),
            last_updated: self.last_updated,
        }
    }

    /// Apply force-directed layout
    pub fn apply_force_layout(&mut self, iterations: usize) {
        for _ in 0..iterations {
            self.apply_forces();
        }
        self.last_updated = Utc::now();
    }

    /// Filter graph by criteria
    pub fn filter(&self, filter: &GraphFilter) -> Workspace3DGraph {
        let mut filtered = self.clone();
        
        // Filter nodes
        filtered.nodes.retain(|_, node| {
            filter.node_types.is_empty() || filter.node_types.contains(&node.node_type)
        });
        
        // Filter edges
        filtered.edges.retain(|_, edge| {
            filter.edge_types.is_empty() || filter.edge_types.contains(&edge.edge_type)
        });
        
        filtered
    }

    // Helper methods
    fn calculate_bounding_box(&self, node_ids: &[String]) -> BoundingBox3D {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut max_z = f64::NEG_INFINITY;
        
        for node_id in node_ids {
            if let Some(node) = self.nodes.get(node_id) {
                min_x = min_x.min(node.position.x);
                min_y = min_y.min(node.position.y);
                min_z = min_z.min(node.position.z);
                max_x = max_x.max(node.position.x);
                max_y = max_y.max(node.position.y);
                max_z = max_z.max(node.position.z);
            }
        }
        
        BoundingBox3D {
            min: Vector3D { x: min_x, y: min_y, z: min_z },
            max: Vector3D { x: max_x, y: max_y, z: max_z },
        }
    }

    fn calculate_node_type_distribution(&self) -> HashMap<NodeType, usize> {
        let mut distribution = HashMap::new();
        for node in self.nodes.values() {
            *distribution.entry(node.node_type.clone()).or_insert(0) += 1;
        }
        distribution
    }

    fn calculate_edge_type_distribution(&self) -> HashMap<EdgeType, usize> {
        let mut distribution = HashMap::new();
        for edge in self.edges.values() {
            *distribution.entry(edge.edge_type.clone()).or_insert(0) += 1;
        }
        distribution
    }

    fn calculate_average_connections(&self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        
        let total_connections: usize = self.nodes.values()
            .map(|node| node.connections.len())
            .sum();
        
        total_connections as f64 / self.nodes.len() as f64
    }

    fn calculate_graph_density(&self) -> f64 {
        let node_count = self.nodes.len();
        if node_count < 2 {
            return 0.0;
        }
        
        let max_edges = node_count * (node_count - 1) / 2;
        self.edges.len() as f64 / max_edges as f64
    }

    fn apply_forces(&mut self) {
        // Would implement force-directed layout algorithm
        // This is a simplified placeholder
        for node in self.nodes.values_mut() {
            // Apply small random movement for demonstration
            node.position.x += (rand::random::<f64>() - 0.5) * 0.1;
            node.position.y += (rand::random::<f64>() - 0.5) * 0.1;
            node.position.z += (rand::random::<f64>() - 0.5) * 0.1;
        }
    }
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph3DStatistics {
    pub node_count: usize,
    pub edge_count: usize,
    pub cluster_count: usize,
    pub node_type_distribution: HashMap<NodeType, usize>,
    pub edge_type_distribution: HashMap<EdgeType, usize>,
    pub average_connections: f64,
    pub graph_density: f64,
    pub last_updated: DateTime<Utc>,
}

/// Graph filter for visualization
#[derive(Debug, Clone)]
pub struct GraphFilter {
    pub node_types: Vec<NodeType>,
    pub edge_types: Vec<EdgeType>,
    pub importance_threshold: Option<f64>,
    pub activity_threshold: Option<f64>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

// Default implementations
impl Default for Graph3DLayout {
    fn default() -> Self {
        Self {
            layout_type: LayoutType::ForceDirected,
            center: Vector3D { x: 0.0, y: 0.0, z: 0.0 },
            scale: 1.0,
            spacing: 10.0,
            force_strength: 1.0,
            repulsion_strength: 100.0,
            attraction_strength: 0.1,
            damping: 0.9,
            animation_enabled: true,
            animation_speed: 1.0,
            auto_rotate: false,
            rotation_speed: 0.1,
        }
    }
}

impl Default for Graph3DSettings {
    fn default() -> Self {
        Self {
            show_labels: true,
            show_edges: true,
            show_clusters: true,
            enable_physics: true,
            enable_collision: false,
            allow_node_dragging: true,
            allow_zoom: true,
            allow_rotation: true,
            selection_enabled: true,
            node_type_filters: Vec::new(),
            edge_type_filters: Vec::new(),
            importance_threshold: 0.0,
            activity_threshold: 0.0,
            max_nodes: 10000,
            max_edges: 50000,
            level_of_detail: true,
            frustum_culling: true,
        }
    }
}

impl Vector3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Vector3D) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
