use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use reqwest::Client;
use tokio::sync::RwLock;
use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::codebase_intelligence::{CodeEntity, Relationship, RelationshipType, RelationshipMetadata, ContextQuery};

pub type Neo4jGraphResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Neo4j Knowledge Graph Implementation for Codebase Relationships
#[derive(Debug, Clone)]
pub struct Neo4jGraph {
    driver: Arc<Neo4jDriver>,
    schema: GraphSchema,
    query_cache: Arc<RwLock<QueryCache>>,
    relationship_analyzer: Arc<RelationshipAnalyzer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jConfig {
    pub uri: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: usize,
    pub connection_timeout_ms: u64,
    pub query_timeout_ms: u64,
    pub enable_encryption: bool,
}

#[derive(Debug, Clone)]
pub struct Neo4jDriver {
    config: Neo4jConfig,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSchema {
    pub node_types: Vec<NodeType>,
    pub relationship_types: Vec<RelationshipTypeSchema>,
    pub indices: Vec<GraphIndex>,
    pub constraints: Vec<GraphConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeType {
    pub label: String,
    pub properties: Vec<PropertySchema>,
    pub required_properties: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    pub name: String,
    pub property_type: PropertyType,
    pub indexed: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    List(Box<PropertyType>),
    Map,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTypeSchema {
    pub name: String,
    pub from_labels: Vec<String>,
    pub to_labels: Vec<String>,
    pub properties: Vec<PropertySchema>,
    pub cardinality: Cardinality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphIndex {
    pub name: String,
    pub label: String,
    pub properties: Vec<String>,
    pub index_type: IndexType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    FullText,
    Vector,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConstraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub label: String,
    pub properties: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    Unique,
    NodeKey,
    Exists,
    NodePropertyType,
    RelationshipPropertyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CypherQuery {
    pub query: String,
    pub parameters: HashMap<String, CypherValue>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CypherValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<CypherValue>),
    Map(HashMap<String, CypherValue>),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub records: Vec<Record>,
    pub summary: QuerySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub fields: HashMap<String, CypherValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySummary {
    pub query_type: String,
    pub counters: QueryCounters,
    pub execution_time_ms: u64,
    pub result_available_after_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCounters {
    pub nodes_created: u64,
    pub nodes_deleted: u64,
    pub relationships_created: u64,
    pub relationships_deleted: u64,
    pub properties_set: u64,
    pub labels_added: u64,
    pub labels_removed: u64,
    pub indexes_added: u64,
    pub indexes_removed: u64,
    pub constraints_added: u64,
    pub constraints_removed: u64,
}

#[derive(Debug, Clone)]
pub struct RelationshipAnalyzer {
    patterns: Vec<RelationshipPattern>,
    extractors: Vec<RelationshipExtractor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipPattern {
    pub name: String,
    pub pattern_type: PatternType,
    pub cypher_template: String,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    CallGraph,
    ImportDependency,
    Inheritance,
    Composition,
    Association,
    DataFlow,
    ControlFlow,
    Temporal,
    Semantic,
}

#[derive(Debug, Clone)]
pub struct RelationshipExtractor {
    pub name: String,
    pub language_specific: bool,
    pub supported_languages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct QueryCache {
    cache: HashMap<String, CachedQueryResult>,
    max_size: usize,
    ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct CachedQueryResult {
    pub result: QueryResult,
    pub timestamp: DateTime<Utc>,
    pub hit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResult {
    pub entities: Vec<CodeEntity>,
    pub relationships: Vec<Relationship>,
    pub paths: Vec<GraphPath>,
    pub metadata: GraphResultMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPath {
    pub nodes: Vec<String>,
    pub relationships: Vec<String>,
    pub length: usize,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResultMetadata {
    pub query_type: String,
    pub execution_time_ms: u64,
    pub nodes_traversed: usize,
    pub relationships_evaluated: usize,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub entity_id: String,
    pub direct_dependents: usize,
    pub total_affected: usize,
    pub impact_score: f32,
    pub critical_paths: Vec<GraphPath>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Neo4jGraph {
    pub async fn new(config: Neo4jConfig) -> Result<Self> {
        let driver = Arc::new(Neo4jDriver::new(config).await?);
        let schema = GraphSchema::default();
        let query_cache = Arc::new(RwLock::new(QueryCache::new(1000, 3600)));
        let relationship_analyzer = Arc::new(RelationshipAnalyzer::new());

        let graph = Self {
            driver,
            schema,
            query_cache,
            relationship_analyzer,
        };

        graph.initialize_schema().await?;
        Ok(graph)
    }

    pub async fn store_graph(&self, entities: &[CodeEntity], relationships: &[Relationship]) -> Result<()> {
        self.create_entity_nodes(entities).await?;
        self.create_relationships(relationships).await?;
        self.update_indices().await?;
        Ok(())
    }

    pub async fn structural_search(&self, query: &ContextQuery) -> Result<Vec<GraphResult>> {
        let cypher = self.build_structural_query(query)?;
        let result = self.execute_query(cypher).await?;
        self.parse_graph_results(result).await
    }

    pub async fn exploratory_search(&self, query: &ContextQuery) -> Result<Vec<GraphResult>> {
        let cypher = self.build_exploratory_query(query)?;
        let result = self.execute_query(cypher).await?;
        self.parse_graph_results(result).await
    }

    pub async fn analytical_search(&self, query: &ContextQuery) -> Result<Vec<GraphResult>> {
        let cypher = self.build_analytical_query(query)?;
        let result = self.execute_query(cypher).await?;
        self.parse_graph_results(result).await
    }

    pub async fn semantic_search(&self, query: &ContextQuery) -> Result<Vec<GraphResult>> {
        let cypher = self.build_semantic_query(query)?;
        let result = self.execute_query(cypher).await?;
        self.parse_graph_results(result).await
    }

    pub async fn find_related_entities(&self, entity_id: &str, depth: usize) -> Result<Vec<CodeEntity>> {
        let cypher = CypherQuery {
            query: format!(
                "MATCH (start:Entity {{id: $entity_id}})-[*1..{}]-(related:Entity) 
                 RETURN DISTINCT related 
                 ORDER BY related.importance DESC 
                 LIMIT 50",
                depth
            ),
            parameters: {
                let mut params = HashMap::new();
                params.insert("entity_id".to_string(), CypherValue::String(entity_id.to_string()));
                params
            },
            timeout_ms: Some(5000),
        };

        let result = self.execute_query(cypher).await?;
        self.parse_entities_from_result(result).await
    }

    pub async fn get_id_for_entity(&self, entity_id: &str) -> Result<String> {
        Ok(entity_id.to_string())
    }

    pub async fn add_vector_reference(&self, graph_id: &str, vector_id: &str) -> Result<()> {
        let cypher = CypherQuery {
            query: "MATCH (e:Entity {id: $graph_id}) SET e.vector_id = $vector_id".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("graph_id".to_string(), CypherValue::String(graph_id.to_string()));
                params.insert("vector_id".to_string(), CypherValue::String(vector_id.to_string()));
                params
            },
            timeout_ms: Some(1000),
        };

        self.execute_query(cypher).await?;
        Ok(())
    }

    pub async fn get_access_count(&self, entity_id: &str) -> Result<u32> {
        let cypher = CypherQuery {
            query: "MATCH (e:Entity {id: $entity_id}) RETURN COALESCE(e.access_count, 0) as count".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("entity_id".to_string(), CypherValue::String(entity_id.to_string()));
                params
            },
            timeout_ms: Some(1000),
        };

        let result = self.execute_query(cypher).await?;
        if let Some(record) = result.records.first() {
            if let Some(CypherValue::Integer(count)) = record.fields.get("count") {
                return Ok(*count as u32);
            }
        }
        Ok(0)
    }

    pub async fn create_performance_indices(&self) -> Result<()> {
        let indices = vec![
            "CREATE INDEX entity_id_index IF NOT EXISTS FOR (e:Entity) ON (e.id)",
            "CREATE INDEX entity_type_index IF NOT EXISTS FOR (e:Entity) ON (e.entity_type)",
            "CREATE INDEX entity_language_index IF NOT EXISTS FOR (e:Entity) ON (e.language)",
            "CREATE FULLTEXT INDEX entity_content_index IF NOT EXISTS FOR (e:Entity) ON (e.content, e.docstring)",
        ];

        for index_query in indices {
            let cypher = CypherQuery {
                query: index_query.to_string(),
                parameters: HashMap::new(),
                timeout_ms: Some(30000),
            };
            self.execute_query(cypher).await?;
        }
        Ok(())
    }

    pub async fn precompute_paths(&self) -> Result<()> {
        let patterns = vec![
            "MATCH (f:Function)-[:CALLS]->(g:Function) MERGE (f)-[:CALL_CHAIN {length: 1}]->(g)",
            "MATCH (f:Function)-[:CALLS*2]->(g:Function) MERGE (f)-[:CALL_CHAIN {length: 2}]->(g)",
        ];

        for pattern in patterns {
            let cypher = CypherQuery {
                query: pattern.to_string(),
                parameters: HashMap::new(),
                timeout_ms: Some(60000),
            };
            self.execute_query(cypher).await?;
        }
        Ok(())
    }

    // Helper methods
    async fn initialize_schema(&self) -> Result<()> {
        let constraints = vec![
            "CREATE CONSTRAINT entity_id_unique IF NOT EXISTS FOR (e:Entity) REQUIRE e.id IS UNIQUE",
        ];

        for constraint in constraints {
            let cypher = CypherQuery {
                query: constraint.to_string(),
                parameters: HashMap::new(),
                timeout_ms: Some(10000),
            };
            self.execute_query(cypher).await?;
        }
        Ok(())
    }

    async fn create_entity_nodes(&self, entities: &[CodeEntity]) -> Result<()> {
        for entity in entities {
            let cypher = CypherQuery {
                query: "MERGE (e:Entity {id: $id})
                        SET e.name = $name,
                            e.entity_type = $entity_type,
                            e.file_path = $file_path,
                            e.language = $language,
                            e.content = $content,
                            e.complexity = $complexity".to_string(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("id".to_string(), CypherValue::String(entity.id.clone()));
                    params.insert("name".to_string(), CypherValue::String(entity.name.clone()));
                    params.insert("entity_type".to_string(), CypherValue::String(format!("{:?}", entity.entity_type)));
                    params.insert("file_path".to_string(), CypherValue::String(entity.file_path.clone()));
                    params.insert("language".to_string(), CypherValue::String(entity.language.clone()));
                    params.insert("content".to_string(), CypherValue::String(entity.content.clone()));
                    params.insert("complexity".to_string(), CypherValue::Integer(entity.metadata.complexity as i64));
                    params
                },
                timeout_ms: Some(5000),
            };
            self.execute_query(cypher).await?;
        }
        Ok(())
    }

    async fn create_relationships(&self, relationships: &[Relationship]) -> Result<()> {
        for relationship in relationships {
            let cypher = CypherQuery {
                query: format!(
                    "MATCH (from:Entity {{id: $from_id}}), (to:Entity {{id: $to_id}})
                     MERGE (from)-[r:{} {{relationship_type: $rel_type}}]->(to)
                     SET r.strength = $strength",
                    self.relationship_type_to_string(&relationship.relationship_type)
                ),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("from_id".to_string(), CypherValue::String(relationship.from.clone()));
                    params.insert("to_id".to_string(), CypherValue::String(relationship.to.clone()));
                    params.insert("rel_type".to_string(), CypherValue::String(format!("{:?}", relationship.relationship_type)));
                    params.insert("strength".to_string(), CypherValue::Float(relationship.strength as f64));
                    params
                },
                timeout_ms: Some(5000),
            };
            self.execute_query(cypher).await?;
        }
        Ok(())
    }

    fn relationship_type_to_string(&self, rel_type: &RelationshipType) -> String {
        match rel_type {
            RelationshipType::Calls => "CALLS",
            RelationshipType::CalledBy => "CALLED_BY",
            RelationshipType::Imports => "IMPORTS",
            RelationshipType::ImportedBy => "IMPORTED_BY",
            RelationshipType::Inherits => "INHERITS",
            RelationshipType::InheritedBy => "INHERITED_BY",
            RelationshipType::Implements => "IMPLEMENTS",
            RelationshipType::ImplementedBy => "IMPLEMENTED_BY",
            RelationshipType::Uses => "USES",
            RelationshipType::UsedBy => "USED_BY",
            RelationshipType::Contains => "CONTAINS",
            RelationshipType::ContainedBy => "CONTAINED_BY",
            RelationshipType::Similar => "SIMILAR",
            RelationshipType::Related => "RELATED",
            RelationshipType::Duplicate => "DUPLICATE",
            RelationshipType::Alternative => "ALTERNATIVE",
            RelationshipType::ModifiedTogether => "MODIFIED_TOGETHER",
            RelationshipType::CreatedTogether => "CREATED_TOGETHER",
            RelationshipType::TestedTogether => "TESTED_TOGETHER",
            RelationshipType::LayerAbove => "LAYER_ABOVE",
            RelationshipType::LayerBelow => "LAYER_BELOW",
            RelationshipType::SameLayer => "SAME_LAYER",
            RelationshipType::CrossCutting => "CROSS_CUTTING",
        }.to_string()
    }

    async fn execute_query(&self, query: CypherQuery) -> Result<QueryResult> {
        let cache_key = format!("{:?}", query);
        {
            let cache = self.query_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.is_valid() {
                    return Ok(cached.result.clone());
                }
            }
        }

        let result = self.driver.execute(query.clone()).await?;

        {
            let mut cache = self.query_cache.write().await;
            cache.insert(cache_key, CachedQueryResult {
                result: result.clone(),
                timestamp: Utc::now(),
                hit_count: 1,
            });
        }

        Ok(result)
    }

    fn build_structural_query(&self, _query: &ContextQuery) -> Result<CypherQuery> {
        Ok(CypherQuery {
            query: "MATCH (n:Entity) RETURN n LIMIT 10".to_string(),
            parameters: HashMap::new(),
            timeout_ms: Some(5000),
        })
    }

    fn build_exploratory_query(&self, _query: &ContextQuery) -> Result<CypherQuery> {
        Ok(CypherQuery {
            query: "MATCH (n:Entity)-[r]->(m:Entity) RETURN n, r, m LIMIT 10".to_string(),
            parameters: HashMap::new(),
            timeout_ms: Some(5000),
        })
    }

    fn build_analytical_query(&self, _query: &ContextQuery) -> Result<CypherQuery> {
        Ok(CypherQuery {
            query: "MATCH (n:Entity) RETURN n.entity_type, count(n) as count".to_string(),
            parameters: HashMap::new(),
            timeout_ms: Some(5000),
        })
    }

    fn build_semantic_query(&self, _query: &ContextQuery) -> Result<CypherQuery> {
        Ok(CypherQuery {
            query: "MATCH (n:Entity) WHERE n.content CONTAINS $search_term RETURN n LIMIT 10".to_string(),
            parameters: HashMap::new(),
            timeout_ms: Some(5000),
        })
    }

    async fn parse_graph_results(&self, _result: QueryResult) -> Result<Vec<GraphResult>> {
        Ok(vec![])
    }

    async fn parse_entities_from_result(&self, _result: QueryResult) -> Result<Vec<CodeEntity>> {
        Ok(vec![])
    }

    async fn update_indices(&self) -> Result<()> {
        Ok(())
    }

    // Task-specific methods
    pub async fn create_node(&self, node_id: &str, node_type: &str, properties: &[(&str, &str)]) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("node_id".to_string(), node_id.to_string());
        params.insert("node_type".to_string(), node_type.to_string());

        for (key, value) in properties {
            params.insert(key.to_string(), value.to_string());
        }

        let query = CypherQuery {
            query: format!("CREATE (n:{} {{id: $node_id}}) SET n += $properties", node_type),
            parameters: params,
            timeout_ms: Some(5000),
        };

        self.driver.execute(query).await?;
        Ok(())
    }

    pub async fn update_node(&self, node_id: &str, properties: &[(&str, &str)]) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("node_id".to_string(), node_id.to_string());

        for (key, value) in properties {
            params.insert(key.to_string(), value.to_string());
        }

        let query = CypherQuery {
            query: "MATCH (n {id: $node_id}) SET n += $properties".to_string(),
            parameters: params,
            timeout_ms: Some(5000),
        };

        self.driver.execute(query).await?;
        Ok(())
    }

    pub async fn delete_node(&self, node_id: &str) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("node_id".to_string(), node_id.to_string());

        let query = CypherQuery {
            query: "MATCH (n {id: $node_id}) DETACH DELETE n".to_string(),
            parameters: params,
            timeout_ms: Some(5000),
        };

        self.driver.execute(query).await?;
        Ok(())
    }

    pub async fn find_related_nodes(&self, node_id: &str, max_depth: usize) -> Result<Vec<String>> {
        let mut params = HashMap::new();
        params.insert("node_id".to_string(), node_id.to_string());
        params.insert("max_depth".to_string(), max_depth.to_string());

        let query = CypherQuery {
            query: format!("MATCH (n {{id: $node_id}})-[*1..{}]-(related) RETURN DISTINCT related.id", max_depth),
            parameters: params,
            timeout_ms: Some(10000),
        };

        let _result = self.driver.execute(query).await?;
        // For now, return empty list - would parse actual results in real implementation
        Ok(vec![])
    }

    pub async fn create_performance_indices(&self) -> Result<()> {
        // Stub implementation - would create actual indices in real implementation
        Ok(())
    }

    pub async fn precompute_paths(&self) -> Result<()> {
        // Stub implementation - would precompute common paths in real implementation
        Ok(())
    }
}

// Supporting implementations
impl Neo4jDriver {
    pub async fn new(config: Neo4jConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.connection_timeout_ms))
            .build()?;

        Ok(Self { config, client })
    }

    pub async fn execute(&self, _query: CypherQuery) -> Result<QueryResult> {
        Ok(QueryResult {
            records: vec![],
            summary: QuerySummary {
                query_type: "READ".to_string(),
                counters: QueryCounters {
                    nodes_created: 0,
                    nodes_deleted: 0,
                    relationships_created: 0,
                    relationships_deleted: 0,
                    properties_set: 0,
                    labels_added: 0,
                    labels_removed: 0,
                    indexes_added: 0,
                    indexes_removed: 0,
                    constraints_added: 0,
                    constraints_removed: 0,
                },
                execution_time_ms: 10,
                result_available_after_ms: 5,
            },
        })
    }
}

impl GraphSchema {
    pub fn default() -> Self {
        Self {
            node_types: vec![],
            relationship_types: vec![],
            indices: vec![],
            constraints: vec![],
        }
    }
}

impl RelationshipAnalyzer {
    pub fn new() -> Self {
        Self {
            patterns: vec![],
            extractors: vec![],
        }
    }
}

impl QueryCache {
    pub fn new(max_size: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            ttl_seconds,
        }
    }

    pub fn get(&self, key: &str) -> Option<&CachedQueryResult> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: String, value: CachedQueryResult) {
        if self.cache.len() >= self.max_size {
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(key, value);
    }
}

impl CachedQueryResult {
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.timestamp);
        age.num_seconds() < 3600
    }
}
