# Context - Hybrid Context Engine Plan

## Goals & Vision

The `context` crate provides a sophisticated hybrid context engine for Symbiote that combines multiple storage and retrieval technologies. It offers:

- **Hybrid Architecture**: SQLite + Qdrant + Neo4j for optimal performance and capabilities
- **Semantic Search**: Vector embeddings for semantic similarity and retrieval
- **Graph Relationships**: Knowledge graphs for complex relationship modeling
- **Fast Metadata**: SQLite for rapid metadata queries and indexing
- **Context Management**: Intelligent context window management and optimization
- **Multi-Modal Support**: Text, code, images, and structured data indexing
- **Real-Time Updates**: Incremental updates and live synchronization

This crate provides the intelligent context foundation that powers all AI interactions in Symbiote.

## UI Design Specifications

### Context Visualization Dashboard

#### Main Context Explorer
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🧠 Context Engine                            [🔍] [⚙️] [📊] [🔄]           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🎯 Context Overview                                                         │
│ ┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐     │
│ │ 📚 Items    │ 🔗 Relations│ 🎯 Vectors  │ 💾 Storage  │ ⚡ Speed     │     │
│ │  1.2M       │    847K     │    2.3M     │   15.7 GB   │   <50ms     │     │
│ └─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘     │
│                                                                             │
│ 🔍 Context Search                                                           │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Search: [How to deploy React app with Docker?                        ] │ │
│ │ Filters: [📁 All] [🔤 Text] [💻 Code] [🖼️ Images] [📊 Data]          │ │
│ │ Scope:   [🌐 Global] [📂 Project] [🕐 Recent] [⭐ Bookmarked]         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📊 Search Results (127 items, 0.043s)                                      │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 🎯 95% │ 📄 React Deployment Guide                    │ 2 days ago    │ │
│ │        │ Complete guide for deploying React apps...   │ [View] [Pin]  │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 🎯 92% │ 💻 docker-compose.yml template              │ 1 week ago    │ │
│ │        │ version: '3.8'\nservices:\n  app:...        │ [View] [Pin]  │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 🎯 89% │ 🔗 Related: Kubernetes deployment           │ 3 days ago    │ │
│ │        │ Alternative deployment using K8s...          │ [View] [Pin]  │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 🎯 87% │ 📊 Performance metrics for React apps       │ 5 days ago    │ │
│ │        │ Monitoring and optimization strategies...    │ [View] [Pin]  │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🌐 Context Graph View                                                       │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │                    ┌─[Docker]─┐                                         │ │
│ │                   ╱             ╲                                       │ │
│ │         [React]──╱               ╲──[Deployment]                        │ │
│ │            │                        │                                   │ │
│ │            │                        │                                   │ │
│ │      [Components]              [Production]                             │ │
│ │            │                        │                                   │ │
│ │            │                        │                                   │ │
│ │         [Hooks]                [Monitoring]                             │ │
│ │                                                                         │ │
│ │ [🔍 Focus] [📏 Layout] [🎨 Style] [📤 Export]                         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Context Detail View
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 📄 React Deployment Guide                                          [✕]     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📊 Context Metadata                                                         │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Type: Documentation          │ Size: 15.2 KB    │ Created: 2 days ago   │ │
│ │ Source: docs/deployment.md   │ Words: 2,847     │ Modified: 1 hour ago  │ │
│ │ Relevance: 95%              │ Tokens: 3,421    │ Views: 47             │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📝 Content Preview                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ # Deploying React Applications with Docker                             │ │
│ │                                                                         │ │
│ │ This guide covers the complete process of containerizing and           │ │
│ │ deploying React applications using Docker and Docker Compose.          │ │
│ │                                                                         │ │
│ │ ## Prerequisites                                                       │ │
│ │ - Node.js 18+                                                          │ │
│ │ - Docker Desktop                                                       │ │
│ │ - Basic React knowledge                                                │ │
│ │                                                                         │ │
│ │ ## Step 1: Create Dockerfile                                           │ │
│ │ ```dockerfile                                                          │ │
│ │ FROM node:18-alpine as build                                           │ │
│ │ WORKDIR /app                                                           │ │
│ │ COPY package*.json ./                                                  │ │
│ │ RUN npm ci --only=production                                           │ │
│ │ ...                                                                    │ │
│ │ ```                                                                    │ │
│ │                                                                         │ │
│ │ [📖 View Full Content] [✏️ Edit] [📋 Copy] [🔗 Share]                 │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔗 Related Context (8 items)                                               │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ • 🐳 Docker best practices (92% similar)                               │ │
│ │ • ⚙️ React build optimization (89% similar)                            │ │
│ │ • 🌐 Nginx configuration for React (87% similar)                       │ │
│ │ • 📊 Performance monitoring setup (84% similar)                        │ │
│ │ • 🔒 Security considerations (81% similar)                             │ │
│ │ [View All Related]                                                     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🏷️ Tags & Categories                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ [React] [Docker] [Deployment] [Frontend] [DevOps] [Production]         │ │
│ │ [+ Add Tag]                                                            │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📈 Usage Analytics                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Views: 47 (↑15% this week)    │ Relevance Score: 95%                   │ │
│ │ Last Accessed: 1 hour ago     │ Embedding Quality: High                │ │
│ │ Referenced By: 12 contexts    │ Update Frequency: Weekly               │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Context Graph Visualization
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🌐 Context Knowledge Graph                              [🔍] [⚙️] [📤]     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🎛️ Graph Controls                                                           │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Layout: [Force] [Hierarchical] [Circular] [Grid]                       │ │
│ │ Filter: [All] [Code] [Docs] [Images] [Recent] [High Relevance]         │ │
│ │ Depth:  [1] [2] [3] [All]     Nodes: 247    Edges: 1,834              │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🗺️ Interactive Graph                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │                                                                         │ │
│ │           ┌─[React]─┐                                                   │ │
│ │          ╱     │     ╲                                                  │ │
│ │    [JSX]╱      │      ╲[Hooks]                                         │ │
│ │       │        │        │                                              │ │
│ │       │    [Components] │                                              │ │
│ │       │        │        │                                              │ │
│ │   [Babel]──[Build]──[Webpack]                                          │ │
│ │       │        │        │                                              │ │
│ │       │    [Docker]─────┼─────[Deployment]                             │ │
│ │       │        │        │           │                                  │ │
│ │   [Node.js]    │    [Nginx]    [Production]                           │ │
│ │       │        │        │           │                                  │ │
│ │       └────[Package.json]      [Monitoring]                           │ │
│ │                                                                         │ │
│ │ 🎯 Selected: React (47 connections)                                    │ │
│ │ 📊 Centrality: 0.89  📈 PageRank: 0.12  🔗 Clustering: 0.67          │ │
│ │                                                                         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📊 Graph Analytics                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Most Connected: React (47), Docker (34), JavaScript (29)               │ │
│ │ Clusters: Frontend (89 nodes), Backend (67 nodes), DevOps (45 nodes)  │ │
│ │ Orphaned Nodes: 12    Bridge Nodes: 8    Critical Paths: 15           │ │
│ │ [📊 Detailed Analytics] [🔍 Find Patterns] [📤 Export Graph]          │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Context Management Interface
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⚙️ Context Engine Settings                                           [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🔧 Storage Configuration                                                    │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ SQLite Database:                                                        │ │
│ │ ├─ Path: /data/context/metadata.db                                     │ │
│ │ ├─ Size: 2.3 GB                                                        │ │
│ │ ├─ Status: ✅ Connected                                                │ │
│ │ └─ [🔧 Configure] [📊 Stats] [🧹 Optimize]                            │ │
│ │                                                                         │ │
│ │ Qdrant Vector Store:                                                    │ │
│ │ ├─ Endpoint: http://localhost:6333                                     │ │
│ │ ├─ Collections: 5 (2.1M vectors)                                       │ │
│ │ ├─ Status: ✅ Connected                                                │ │
│ │ └─ [🔧 Configure] [📊 Stats] [🧹 Optimize]                            │ │
│ │                                                                         │ │
│ │ Neo4j Graph Database:                                                   │ │
│ │ ├─ URI: bolt://localhost:7687                                          │ │
│ │ ├─ Nodes: 1.2M, Relationships: 847K                                   │ │
│ │ ├─ Status: ✅ Connected                                                │ │
│ │ └─ [🔧 Configure] [📊 Stats] [🧹 Optimize]                            │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🤖 Embedding Models                                                         │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Primary Model: text-embedding-ada-002                                  │ │
│ │ ├─ Dimensions: 1536                                                    │ │
│ │ ├─ Cost: $0.0001/1K tokens                                            │ │
│ │ ├─ Performance: 95% accuracy                                           │ │
│ │ └─ [Change Model] [Test Performance]                                   │ │
│ │                                                                         │ │
│ │ Fallback Model: sentence-transformers/all-MiniLM-L6-v2                │ │
│ │ ├─ Dimensions: 384                                                     │ │
│ │ ├─ Cost: Free (local)                                                  │ │
│ │ ├─ Performance: 87% accuracy                                           │ │
│ │ └─ [Configure] [Update Model]                                          │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔍 Search & Retrieval                                                       │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Max Results: [50        ] │ Similarity Threshold: [0.75    ]           │ │
│ │ Context Window: [8192   ] │ Reranking: ☑️ Enabled                     │ │
│ │ Cache TTL: [1 hour     ] │ Prefetch: ☑️ Enabled                      │ │
│ │ Hybrid Search: ☑️ Vector + Text + Graph                               │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📊 Performance Monitoring                                                   │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Average Query Time: 43ms  │ Cache Hit Rate: 78%                       │ │
│ │ Indexing Rate: 1.2K/min   │ Memory Usage: 2.1GB                       │ │
│ │ Error Rate: 0.02%         │ Uptime: 99.97%                            │ │
│ │ [📈 Detailed Metrics] [🚨 Set Alerts] [📋 Export Report]             │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│                                    [Cancel] [Save Settings]                │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Database Schema

### Context Storage Persistence

```sql
-- Documents and content storage
CREATE TABLE context_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id VARCHAR(255) NOT NULL UNIQUE,
    title VARCHAR(500),
    content TEXT NOT NULL,
    content_type VARCHAR(100), -- 'code', 'text', 'markdown', 'json', etc.
    language VARCHAR(50),
    file_path VARCHAR(1000),
    file_hash VARCHAR(64),
    size_bytes INTEGER,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    indexed_at TIMESTAMP,
    metadata JSONB
);

-- Document chunks for retrieval
CREATE TABLE context_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chunk_id VARCHAR(255) NOT NULL UNIQUE,
    document_id VARCHAR(255) REFERENCES context_documents(document_id),
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    start_line INTEGER,
    end_line INTEGER,
    start_char INTEGER,
    end_char INTEGER,
    chunk_type VARCHAR(100), -- 'function', 'class', 'paragraph', 'code_block'
    token_count INTEGER,
    embedding_model VARCHAR(100),
    embedding_version VARCHAR(50),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Vector embeddings (stored separately for performance)
CREATE TABLE context_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chunk_id VARCHAR(255) REFERENCES context_chunks(chunk_id),
    embedding_model VARCHAR(100) NOT NULL,
    embedding_version VARCHAR(50) NOT NULL,
    dimensions INTEGER NOT NULL,
    embedding_vector VECTOR(1536), -- Adjust dimensions as needed
    created_at TIMESTAMP DEFAULT NOW()
);

-- Document relationships and graph connections
CREATE TABLE context_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_document_id VARCHAR(255) REFERENCES context_documents(document_id),
    to_document_id VARCHAR(255) REFERENCES context_documents(document_id),
    relationship_type VARCHAR(100), -- 'imports', 'calls', 'references', 'similar', 'depends_on'
    strength DECIMAL(3,2) DEFAULT 1.0, -- 0.0 to 1.0
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(from_document_id, to_document_id, relationship_type)
);

-- Code symbols and definitions
CREATE TABLE context_symbols (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol_id VARCHAR(255) NOT NULL UNIQUE,
    document_id VARCHAR(255) REFERENCES context_documents(document_id),
    symbol_name VARCHAR(255) NOT NULL,
    symbol_type VARCHAR(100), -- 'function', 'class', 'variable', 'constant', 'interface'
    definition_line INTEGER,
    definition_char INTEGER,
    scope VARCHAR(255),
    visibility VARCHAR(50), -- 'public', 'private', 'protected', 'internal'
    signature TEXT,
    docstring TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Symbol references and usage
CREATE TABLE context_symbol_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol_id VARCHAR(255) REFERENCES context_symbols(symbol_id),
    document_id VARCHAR(255) REFERENCES context_documents(document_id),
    reference_line INTEGER,
    reference_char INTEGER,
    reference_type VARCHAR(100), -- 'call', 'import', 'assignment', 'read', 'write'
    context_lines TEXT, -- Surrounding code for context
    created_at TIMESTAMP DEFAULT NOW()
);

-- Search queries and results for analytics
CREATE TABLE context_search_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_id VARCHAR(255) NOT NULL UNIQUE,
    user_id VARCHAR(255),
    session_id VARCHAR(255),
    query_text TEXT NOT NULL,
    query_type VARCHAR(100), -- 'semantic', 'keyword', 'code', 'hybrid'
    filters JSONB,
    results_count INTEGER,
    execution_time_ms INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Search results for caching and analytics
CREATE TABLE context_search_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_id VARCHAR(255) REFERENCES context_search_queries(query_id),
    chunk_id VARCHAR(255) REFERENCES context_chunks(chunk_id),
    rank_position INTEGER,
    relevance_score DECIMAL(5,4),
    result_type VARCHAR(100), -- 'exact', 'semantic', 'related'
    created_at TIMESTAMP DEFAULT NOW()
);

-- Context collections for organization
CREATE TABLE context_collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    collection_type VARCHAR(100), -- 'workspace', 'project', 'custom'
    configuration JSONB,
    created_by VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Collection membership
CREATE TABLE context_collection_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id VARCHAR(255) REFERENCES context_collections(collection_id),
    document_id VARCHAR(255) REFERENCES context_documents(document_id),
    added_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(collection_id, document_id)
);

-- Context usage analytics
CREATE TABLE context_usage_analytics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(255),
    document_id VARCHAR(255) REFERENCES context_documents(document_id),
    chunk_id VARCHAR(255) REFERENCES context_chunks(chunk_id),
    access_type VARCHAR(100), -- 'search', 'browse', 'reference', 'edit'
    access_count INTEGER DEFAULT 1,
    last_accessed TIMESTAMP DEFAULT NOW(),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Indexing jobs and status
CREATE TABLE context_indexing_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id VARCHAR(255) NOT NULL UNIQUE,
    job_type VARCHAR(100), -- 'full_index', 'incremental', 'reindex', 'cleanup'
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed'
    workspace_path VARCHAR(1000),
    files_processed INTEGER DEFAULT 0,
    files_total INTEGER DEFAULT 0,
    chunks_created INTEGER DEFAULT 0,
    embeddings_generated INTEGER DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_context_documents_file_path ON context_documents(file_path);
CREATE INDEX idx_context_documents_content_type ON context_documents(content_type);
CREATE INDEX idx_context_documents_updated_at ON context_documents(updated_at);
CREATE INDEX idx_context_chunks_document_id ON context_chunks(document_id);
CREATE INDEX idx_context_chunks_chunk_type ON context_chunks(chunk_type);
CREATE INDEX idx_context_embeddings_chunk_id ON context_embeddings(chunk_id);
CREATE INDEX idx_context_embeddings_model ON context_embeddings(embedding_model, embedding_version);
CREATE INDEX idx_context_relationships_from ON context_relationships(from_document_id);
CREATE INDEX idx_context_relationships_to ON context_relationships(to_document_id);
CREATE INDEX idx_context_relationships_type ON context_relationships(relationship_type);
CREATE INDEX idx_context_symbols_document_id ON context_symbols(document_id);
CREATE INDEX idx_context_symbols_name ON context_symbols(symbol_name);
CREATE INDEX idx_context_symbols_type ON context_symbols(symbol_type);
CREATE INDEX idx_context_symbol_references_symbol ON context_symbol_references(symbol_id);
CREATE INDEX idx_context_search_queries_user ON context_search_queries(user_id);
CREATE INDEX idx_context_search_queries_created_at ON context_search_queries(created_at);
CREATE INDEX idx_context_usage_analytics_user ON context_usage_analytics(user_id);
CREATE INDEX idx_context_usage_analytics_document ON context_usage_analytics(document_id);
```

## Architecture & Design

### Core Modules

```
context/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── engine/               # Core context engine
│   │   ├── mod.rs
│   │   ├── hybrid.rs         # Hybrid storage coordinator
│   │   ├── indexing.rs       # Content indexing and processing
│   │   ├── retrieval.rs      # Context retrieval and ranking
│   │   └── optimization.rs   # Performance optimization
│   ├── storage/              # Storage backends
│   │   ├── mod.rs
│   │   ├── sqlite.rs         # SQLite metadata storage
│   │   ├── qdrant.rs         # Qdrant vector storage
│   │   ├── neo4j.rs          # Neo4j graph storage
│   │   └── coordinator.rs    # Storage coordination
│   ├── embeddings/           # Embedding management
│   │   ├── mod.rs
│   │   ├── generators.rs     # Embedding generation
│   │   ├── models.rs         # Embedding model management
│   │   ├── caching.rs        # Embedding caching
│   │   └── optimization.rs   # Embedding optimization
│   ├── graph/                # Graph operations
│   │   ├── mod.rs
│   │   ├── relationships.rs  # Relationship modeling
│   │   ├── traversal.rs      # Graph traversal algorithms
│   │   ├── analysis.rs       # Graph analysis and insights
│   │   └── visualization.rs  # Graph visualization support
│   ├── search/               # Search and retrieval
│   │   ├── mod.rs
│   │   ├── semantic.rs       # Semantic search
│   │   ├── hybrid.rs         # Hybrid search strategies
│   │   ├── ranking.rs        # Result ranking and scoring
│   │   └── filtering.rs      # Search filtering and faceting
│   ├── chunking/             # Content chunking
│   │   ├── mod.rs
│   │   ├── strategies.rs     # Chunking strategies
│   │   ├── text.rs           # Text chunking
│   │   ├── code.rs           # Code-aware chunking
│   │   └── multimodal.rs     # Multi-modal chunking
│   ├── indexing/             # Content indexing
│   │   ├── mod.rs
│   │   ├── pipeline.rs       # Indexing pipeline
│   │   ├── processors.rs     # Content processors
│   │   ├── extractors.rs     # Feature extractors
│   │   └── validators.rs     # Content validators
│   └── types/                # Context types
│       ├── mod.rs
│       ├── documents.rs      # Document types
│       ├── chunks.rs         # Chunk types
│       └── queries.rs        # Query types
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── basic_indexing.rs
    └── semantic_search.rs
```

### Key Design Principles

1. **Hybrid Optimization**: Leverage strengths of each storage technology
2. **Semantic Intelligence**: Deep understanding of content relationships
3. **Performance First**: Sub-second retrieval for large datasets
4. **Scalability**: Handle millions of documents and relationships
5. **Flexibility**: Support diverse content types and use cases

## Error Handling

### Context Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("Document not found: {document_id}")]
    DocumentNotFound { document_id: String },

    #[error("Chunk not found: {chunk_id}")]
    ChunkNotFound { chunk_id: String },

    #[error("Collection not found: {collection_id}")]
    CollectionNotFound { collection_id: String },

    #[error("Indexing failed: {file_path} - {reason}")]
    IndexingFailed { file_path: String, reason: String },

    #[error("Embedding generation failed: {model} - {error}")]
    EmbeddingFailed { model: String, error: String },

    #[error("Vector search failed: {operation} - {details}")]
    VectorSearchFailed { operation: String, details: String },

    #[error("Graph query failed: {query} - {error}")]
    GraphQueryFailed { query: String, error: String },

    #[error("Storage operation failed: {storage_type} - {operation} - {error}")]
    StorageFailed { storage_type: String, operation: String, error: String },

    #[error("Parsing failed: {file_path} - {language} - {error}")]
    ParsingFailed { file_path: String, language: String, error: String },

    #[error("Symbol resolution failed: {symbol} in {file_path}")]
    SymbolResolutionFailed { symbol: String, file_path: String },

    #[error("Dependency analysis failed: {file_path} - {reason}")]
    DependencyAnalysisFailed { file_path: String, reason: String },

    #[error("Context budget exceeded: requested {requested} tokens, limit {limit}")]
    BudgetExceeded { requested: usize, limit: usize },

    #[error("Chunking strategy failed: {strategy} - {content_type} - {error}")]
    ChunkingFailed { strategy: String, content_type: String, error: String },

    #[error("Search query invalid: {query} - {reason}")]
    InvalidQuery { query: String, reason: String },

    #[error("Configuration error: {setting} - {issue}")]
    ConfigurationError { setting: String, issue: String },

    #[error("Database connection failed: {database_type} - {error}")]
    DatabaseConnectionFailed { database_type: String, error: String },

    #[error("Migration failed: {migration} - {error}")]
    MigrationFailed { migration: String, error: String },

    #[error("Backup operation failed: {operation} - {path} - {error}")]
    BackupFailed { operation: String, path: String, error: String },

    #[error("Restore operation failed: {path} - {error}")]
    RestoreFailed { path: String, error: String },

    #[error("Cache operation failed: {operation} - {cache_type} - {error}")]
    CacheFailed { operation: String, cache_type: String, error: String },

    #[error("Serialization error: {data_type} - {error}")]
    SerializationError { data_type: String, error: String },

    #[error("Deserialization error: {data_type} - {error}")]
    DeserializationError { data_type: String, error: String },

    #[error("IO error: {operation} - {path} - {error}")]
    IoError { operation: String, path: String, error: String },

    #[error("Permission denied: {operation} - {resource}")]
    PermissionDenied { operation: String, resource: String },

    #[error("Resource limit exceeded: {resource} - {limit}")]
    ResourceLimitExceeded { resource: String, limit: String },

    #[error("Timeout error: {operation} timed out after {duration_ms}ms")]
    TimeoutError { operation: String, duration_ms: u64 },

    #[error("Concurrent access conflict: {resource} - {operation}")]
    ConcurrencyError { resource: String, operation: String },

    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    #[error("Integrity check failed: {check_type} - {details}")]
    IntegrityCheckFailed { check_type: String, details: String },

    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },

    #[error("Internal context error: {details}")]
    InternalError { details: String },
}

pub type ContextResult<T> = Result<T, ContextError>;

impl From<sqlx::Error> for ContextError {
    fn from(err: sqlx::Error) -> Self {
        ContextError::StorageFailed {
            storage_type: "sqlite".to_string(),
            operation: "database_operation".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ContextError {
    fn from(err: std::io::Error) -> Self {
        ContextError::IoError {
            operation: "io_operation".to_string(),
            path: "unknown".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ContextError {
    fn from(err: serde_json::Error) -> Self {
        ContextError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}
```

## APIs & Interfaces

### Context Engine

```rust
/// Hybrid context engine with codebase intelligence (Augment's Architecture)
pub struct SymbioteContextEngine {
    // Real-time file system monitoring
    file_watcher: FileWatcher,

    // Hybrid storage approach
    vector_store: QdrantClient,           // Qdrant for semantic embeddings (optional: can be swapped or disabled)
    graph_db: Option<Neo4jClient>,        // Optional: Neo4j for code relationships (degrades gracefully if None)
    sql_db: SqlitePool,                   // SQLite for structured metadata (always available, embedded)

    // Configurable embedding providers
    embedding_manager: EmbeddingProviderManager,  // Support all providers
    ast_analyzer: ASTAnalyzer,                     // AST-based structural analysis

    // Git integration
    commit_indexer: CommitIndexer,

    // Intelligent retrieval with ranking
    hybrid_retriever: HybridRetriever,
    context_ranker: ContextRanker,
    versioning: IndexVersioning,             // embedder/version params recorded; rolling re‑embed thresholds
    compaction: IndexCompactor,              // dedupe chunks; PQ compression; configurable HNSW defaults
    budget_planner: ContextBudgetPlanner,    // assemble ranges within token budgets with summarization

    // Legacy compatibility
    sqlite: SqliteStorage,
    qdrant: Option<QdrantStorage>,
    neo4j: Option<Neo4jStorage>,
    embeddings: EmbeddingManager,
    indexing_pipeline: IndexingPipeline,

    // Codebase Intelligence Components (Meta Glean-inspired)
    codebase_intelligence: CodebaseIntelligence,
    lsp_manager: LSPManager,
    ast_indexer: ASTIndexer,
    semantic_indexer: SemanticIndexer,
    dependency_tracker: DependencyTracker,

    // Real-time incremental updates
    file_watcher: FileWatcher,
    incremental_indexer: IncrementalIndexer,
    change_propagator: ChangePropagator,
    search_engine: SearchEngine,
    config: ContextConfig,
}

impl SymbioteContextEngine {
    pub async fn new(config: ContextConfig) -> ContextResult<Self>;

    /// Hybrid retrieval strategy combining SQL + Graph + Vector + Ranking Fusion
    pub async fn hybrid_retrieve(&self, query: &str, context: &RetrievalContext) -> ContextResult<Vec<ContextChunk>>;

    /// Range assembly with surgical retrieval and minimal line ranges
    pub async fn assemble_ranges(&self, chunks: &[ContextChunk], budget: TokenBudget) -> ContextResult<AssembledContext>;

    /// Budget-aware packing with token limits and summarization
    pub async fn pack_with_budget(&self, context: &AssembledContext, budget: TokenBudget) -> ContextResult<PackedContext>;

    /// Incremental indexing pipeline with git diff + FS watch
    pub async fn start_incremental_indexing(&mut self) -> ContextResult<()>;

    /// Code-aware chunking with symbol bodies/blocks and stable anchors
    pub async fn chunk_code(&self, content: &str, language: &str) -> ContextResult<Vec<CodeChunk>>;
}

impl ContextEngine {
    pub async fn new(config: ContextConfig) -> ContextResult<Self>;

    /// Codebase Intelligence Methods (Meta Glean-inspired)
    pub async fn handle_file_change(&self, file_path: &Path, change_type: ChangeType) -> ContextResult<()>;

    pub async fn index_codebase(&mut self, workspace_path: &Path) -> ContextResult<IndexingResult>;

    pub async fn get_symbol_definition(&self, symbol: &str, file_path: &Path) -> ContextResult<Vec<SymbolDefinition>>;

    pub async fn get_symbol_references(&self, symbol: &str, file_path: &Path) -> ContextResult<Vec<SymbolReference>>;

    pub async fn get_file_dependencies(&self, file_path: &Path) -> ContextResult<Vec<Dependency>>;

    pub async fn get_dependency_graph(&self, root_file: &Path) -> ContextResult<DependencyGraph>;

    pub async fn semantic_code_search(&self, query: &str, language: Option<Language>) -> ContextResult<Vec<CodeSearchResult>>;

    pub async fn find_similar_code(&self, code_snippet: &str) -> ContextResult<Vec<SimilarCodeMatch>>;

    pub async fn get_code_context(&self, file_path: &Path, line: u32, context_lines: u32) -> ContextResult<CodeContext>;

    pub async fn analyze_code_relationships(&self, file_path: &Path) -> ContextResult<CodeRelationships>;

    pub async fn get_ast_for_file(&self, file_path: &Path) -> ContextResult<AbstractSyntaxTree>;

    pub async fn get_semantic_tokens(&self, file_path: &Path) -> ContextResult<Vec<SemanticToken>>;

    pub async fn validate_code_integrity(&self, file_path: &Path) -> ContextResult<IntegrityReport>;

    pub async fn get_code_metrics(&self, file_path: &Path) -> ContextResult<CodeMetrics>;

    pub async fn track_symbol_usage(&self, symbol: &str) -> ContextResult<SymbolUsageStats>;

    pub async fn get_cross_language_dependencies(&self, workspace_path: &Path) -> ContextResult<CrossLanguageDependencies>;

    pub async fn optimize_indexing_performance(&mut self) -> ContextResult<OptimizationResult>;

    pub async fn get_indexing_statistics(&self) -> ContextResult<IndexingStatistics>;

    pub async fn index_document(&self, document: Document) -> ContextResult<DocumentId>;
    
    pub async fn index_documents(&self, documents: Vec<Document>) -> ContextResult<Vec<DocumentId>>;
    
    pub async fn update_document(&self, id: DocumentId, document: Document) -> ContextResult<()>;
    
    pub async fn delete_document(&self, id: DocumentId) -> ContextResult<()>;
    
    pub async fn search(&self, query: ContextQuery) -> ContextResult<SearchResults>;
    
    pub async fn get_related(&self, id: DocumentId, relation_type: RelationType) -> ContextResult<Vec<RelatedDocument>>;
    
    pub async fn build_context(&self, query: &str, max_tokens: usize) -> ContextResult<ContextWindow>;
    
    pub async fn get_document(&self, id: DocumentId) -> ContextResult<Option<Document>>;
    
    pub async fn get_statistics(&self) -> ContextResult<ContextStatistics>;

    pub async fn create_context_collection(&self, name: &str, config: CollectionConfig) -> ContextResult<CollectionId>;

    pub async fn delete_context_collection(&self, collection_id: CollectionId) -> ContextResult<()>;

    pub fn list_context_collections(&self) -> Vec<CollectionInfo>;

    pub async fn get_collection_stats(&self, collection_id: CollectionId) -> ContextResult<CollectionStats>;

    pub async fn backup_context_data(&self, backup_path: &str) -> ContextResult<BackupInfo>;

    pub async fn restore_context_data(&self, backup_path: &str) -> ContextResult<RestoreResult>;

    pub async fn optimize_storage(&self) -> ContextResult<OptimizationResult>;

    pub async fn rebuild_indexes(&self) -> ContextResult<RebuildResult>;

    pub async fn validate_data_integrity(&self) -> ContextResult<IntegrityReport>;

    pub async fn export_context_data(&self, format: ExportFormat, filter: ExportFilter) -> ContextResult<ExportResult>;

    pub async fn import_context_data(&self, data: &[u8], format: ImportFormat) -> ContextResult<ImportResult>;

    pub async fn create_context_snapshot(&self, name: &str) -> ContextResult<SnapshotId>;

    pub async fn restore_from_snapshot(&self, snapshot_id: SnapshotId) -> ContextResult<()>;

    pub fn list_snapshots(&self) -> Vec<SnapshotInfo>;

    pub async fn delete_snapshot(&self, snapshot_id: SnapshotId) -> ContextResult<()>;

    pub async fn get_embedding_model_info(&self) -> ContextResult<EmbeddingModelInfo>;

    pub async fn switch_embedding_model(&mut self, model_config: EmbeddingModelConfig) -> ContextResult<()>;

    pub async fn benchmark_embedding_models(&self, test_data: Vec<String>) -> ContextResult<BenchmarkResults>;

    pub async fn get_query_performance_stats(&self, timeframe: TimeRange) -> ContextResult<PerformanceStats>;

    pub async fn set_cache_policy(&mut self, policy: CachePolicy) -> ContextResult<()>;

    pub async fn clear_cache(&self, cache_type: CacheType) -> ContextResult<()>;

    pub async fn get_cache_stats(&self) -> ContextResult<CacheStats>;

    pub async fn create_context_view(&self, name: &str, query: ViewQuery) -> ContextResult<ViewId>;

    pub async fn update_context_view(&self, view_id: ViewId, query: ViewQuery) -> ContextResult<()>;

    pub async fn delete_context_view(&self, view_id: ViewId) -> ContextResult<()>;

    pub fn list_context_views(&self) -> Vec<ViewInfo>;

    pub async fn get_view_results(&self, view_id: ViewId, params: ViewParams) -> ContextResult<ViewResults>;

    pub async fn create_context_alert(&self, alert_config: AlertConfig) -> ContextResult<AlertId>;

    pub async fn update_context_alert(&self, alert_id: AlertId, config: AlertConfig) -> ContextResult<()>;

    pub async fn delete_context_alert(&self, alert_id: AlertId) -> ContextResult<()>;

    pub fn list_context_alerts(&self) -> Vec<AlertInfo>;

    pub async fn get_context_health(&self) -> ContextResult<HealthStatus>;

    pub async fn run_context_diagnostics(&self) -> ContextResult<DiagnosticsReport>;

    pub async fn get_storage_usage(&self) -> ContextResult<StorageUsage>;

    pub async fn estimate_storage_growth(&self, timeframe: Duration) -> ContextResult<GrowthEstimate>;

    pub async fn configure_auto_cleanup(&mut self, policy: CleanupPolicy) -> ContextResult<()>;

    pub async fn run_manual_cleanup(&self, criteria: CleanupCriteria) -> ContextResult<CleanupResult>;

    pub async fn get_context_lineage(&self, document_id: DocumentId) -> ContextResult<ContextLineage>;

    pub async fn trace_context_usage(&self, document_id: DocumentId) -> ContextResult<UsageTrace>;

    pub async fn analyze_context_patterns(&self, analysis_config: AnalysisConfig) -> ContextResult<PatternAnalysis>;

    pub async fn generate_context_insights(&self, insight_type: InsightType) -> ContextResult<ContextInsights>;

    pub async fn create_context_index(&self, index_config: IndexConfig) -> ContextResult<IndexId>;

    pub async fn drop_context_index(&self, index_id: IndexId) -> ContextResult<()>;

    pub fn list_context_indexes(&self) -> Vec<IndexInfo>;

    pub async fn reindex_collection(&self, collection_id: CollectionId) -> ContextResult<ReindexResult>;

    pub async fn get_index_usage_stats(&self, index_id: IndexId) -> ContextResult<IndexUsageStats>;
}
```

### Document Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Option<DocumentId>,
    pub content: DocumentContent,
    pub metadata: DocumentMetadata,
    pub relationships: Vec<DocumentRelationship>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentContent {
    Text { text: String },
    Code { 
        code: String, 
        language: String, 
        file_path: Option<String> 
    },
    Structured { 
        data: serde_json::Value, 
        schema: Option<String> 
    },
    Binary { 
        data: Vec<u8>, 
        mime_type: String 
    },
    Multimodal { 
        parts: Vec<ContentPart> 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub source: DocumentSource,
    pub language: Option<String>,
    pub size: usize,
    pub checksum: String,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentSource {
    File { path: String },
    Url { url: String },
    Database { table: String, id: String },
    Memory { session_id: String },
    Generated { generator: String },
    Custom { source_type: String, identifier: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRelationship {
    pub target_id: DocumentId,
    pub relationship_type: RelationType,
    pub strength: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationType {
    References,
    ReferencedBy,
    Similar,
    Depends,
    DependedBy,
    Contains,
    ContainedBy,
    Follows,
    Precedes,
    Custom(String),
}
```

### Search and Retrieval

```rust
#[derive(Debug, Clone)]
pub struct ContextQuery {
    pub text: String,
    pub query_type: QueryType,
    pub filters: Vec<SearchFilter>,
    pub max_results: usize,
    pub min_score: Option<f32>,
    pub include_metadata: bool,
    pub include_relationships: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Semantic,      // Vector similarity search
    Keyword,       // Traditional keyword search
    Hybrid,        // Combination of semantic and keyword
    Graph,         // Graph traversal query
    Structured,    // Structured data query
}

#[derive(Debug, Clone)]
pub enum SearchFilter {
    Tag(String),
    Source(DocumentSource),
    Language(String),
    DateRange { start: DateTime<Utc>, end: DateTime<Utc> },
    SizeRange { min: usize, max: usize },
    Custom { field: String, value: serde_json::Value },
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total_count: usize,
    pub query_time: Duration,
    pub facets: HashMap<String, Vec<FacetValue>>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
    pub highlights: Vec<Highlight>,
    pub explanation: Option<ScoreExplanation>,
}

#[derive(Debug, Clone)]
pub struct ContextWindow {
    pub content: String,
    pub token_count: usize,
    pub sources: Vec<ContextSource>,
    pub relevance_score: f32,
    pub build_time: Duration,
}

#[derive(Debug, Clone)]
pub struct ContextSource {
    pub document_id: DocumentId,
    pub chunk_id: Option<ChunkId>,
    pub relevance_score: f32,
    pub token_contribution: usize,
}
```

### Embedding Management

```rust
pub struct EmbeddingManager {
    generators: HashMap<String, Box<dyn EmbeddingGenerator>>,
    cache: EmbeddingCache,
    models: EmbeddingModelRegistry,
    config: EmbeddingConfig,
}

impl EmbeddingManager {
    pub fn new(config: EmbeddingConfig) -> Self;
    
    pub async fn generate_embedding(&self, content: &str, model: &str) -> ContextResult<Embedding>;
    
    pub async fn generate_embeddings(&self, contents: &[String], model: &str) -> ContextResult<Vec<Embedding>>;
    
    pub fn calculate_similarity(&self, a: &Embedding, b: &Embedding) -> f32;
    
    pub async fn find_similar(&self, embedding: &Embedding, threshold: f32) -> ContextResult<Vec<SimilarityMatch>>;
    
    pub fn get_model_info(&self, model: &str) -> Option<&EmbeddingModelInfo>;
    
    pub async fn warm_cache(&self, contents: &[String]) -> ContextResult<()>;
}

#[async_trait]
pub trait EmbeddingGenerator: Send + Sync {
    fn model_name(&self) -> &str;
    
    fn dimensions(&self) -> usize;
    
    fn max_input_length(&self) -> usize;
    
    async fn generate(&self, text: &str) -> ContextResult<Embedding>;
    
    async fn generate_batch(&self, texts: &[String]) -> ContextResult<Vec<Embedding>>;
}

pub type Embedding = Vec<f32>;

#[derive(Debug, Clone)]
pub struct SimilarityMatch {
    pub document_id: DocumentId,
    pub chunk_id: Option<ChunkId>,
    pub similarity: f32,
    pub embedding: Embedding,
}
```

### Storage Backends

```rust
pub struct SqliteStorage {
    pool: SqlitePool,
    schema_version: u32,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> ContextResult<Self>;
    
    pub async fn store_document(&self, document: &Document) -> ContextResult<DocumentId>;
    
    pub async fn get_document(&self, id: DocumentId) -> ContextResult<Option<Document>>;
    
    pub async fn search_metadata(&self, filters: &[SearchFilter]) -> ContextResult<Vec<DocumentId>>;
    
    pub async fn get_relationships(&self, id: DocumentId) -> ContextResult<Vec<DocumentRelationship>>;
    
    pub async fn update_statistics(&self, stats: &ContextStatistics) -> ContextResult<()>;
}

pub struct QdrantStorage {
    client: QdrantClient,
    collection_name: String,
    vector_size: usize,
}

impl QdrantStorage {
    pub async fn new(config: QdrantConfig) -> ContextResult<Self>;
    
    pub async fn store_embedding(&self, id: DocumentId, embedding: Embedding, metadata: PointMetadata) -> ContextResult<()>;
    
    pub async fn search_similar(&self, embedding: &Embedding, limit: usize, threshold: Option<f32>) -> ContextResult<Vec<ScoredPoint>>;
    
    pub async fn delete_embedding(&self, id: DocumentId) -> ContextResult<()>;
    
    pub async fn get_collection_info(&self) -> ContextResult<CollectionInfo>;
}

pub struct Neo4jStorage {
    driver: Driver,
    database: String,
}

impl Neo4jStorage {
    pub async fn new(config: Neo4jConfig) -> ContextResult<Self>;
    
    pub async fn create_node(&self, document: &Document) -> ContextResult<()>;
    
    pub async fn create_relationship(&self, from: DocumentId, to: DocumentId, rel_type: RelationType) -> ContextResult<()>;
    
    pub async fn find_related(&self, id: DocumentId, rel_type: RelationType, depth: u32) -> ContextResult<Vec<RelatedDocument>>;
    
    pub async fn analyze_graph(&self) -> ContextResult<GraphAnalysis>;
}
```

## Security Model

### Context Access Control

```rust
pub struct ContextSecurityManager {
    access_control: ContextAccessControl,
    data_classifier: ContextDataClassifier,
    audit_logger: ContextAuditLogger,
    encryption_manager: ContextEncryptionManager,
}

impl ContextSecurityManager {
    /// Validate user access to specific documents/collections
    pub async fn validate_access(&self, user_id: &str, operation: ContextOperation, resource: &str) -> ContextResult<AccessDecision>;

    /// Classify data sensitivity for context content
    pub fn classify_content(&self, content: &str, file_path: &str) -> DataClassification;

    /// Encrypt sensitive context data
    pub async fn encrypt_content(&self, content: &str, classification: DataClassification) -> ContextResult<EncryptedContent>;

    /// Decrypt context data for authorized access
    pub async fn decrypt_content(&self, encrypted_content: &EncryptedContent, user_context: &UserContext) -> ContextResult<String>;

    /// Log context operations for audit
    pub async fn log_context_operation(&self, operation: &ContextOperation, user_id: &str, result: &OperationResult) -> ContextResult<()>;

    /// Sanitize search queries for security
    pub async fn sanitize_query(&self, query: &str, user_context: &UserContext) -> ContextResult<SanitizedQuery>;

    /// Filter search results based on permissions
    pub async fn filter_results(&self, results: &[ContextChunk], user_context: &UserContext) -> ContextResult<Vec<ContextChunk>>;
}

#[derive(Debug, Clone)]
pub enum ContextOperation {
    Search { query: String, collection_id: Option<String> },
    Index { file_path: String, content_type: String },
    Retrieve { document_id: String },
    Update { document_id: String },
    Delete { document_id: String },
    CreateCollection { name: String },
    DeleteCollection { collection_id: String },
    Export { format: String, filter: String },
    Backup { destination: String },
}

#[derive(Debug, Clone)]
pub enum DataClassification {
    Public,       // No restrictions
    Internal,     // Basic access control
    Confidential, // Encrypted + access logging
    Restricted,   // Strongest encryption + approval required
}
```

### Content Privacy & PII Protection

```rust
pub struct ContextPrivacyManager {
    pii_detector: PIIDetector,
    content_anonymizer: ContentAnonymizer,
    retention_manager: RetentionManager,
    consent_tracker: ConsentTracker,
}

impl ContextPrivacyManager {
    /// Detect PII in context content before indexing
    pub async fn detect_pii(&self, content: &str, file_path: &str) -> ContextResult<PIIDetectionResult>;

    /// Anonymize or redact PII from content
    pub async fn anonymize_content(&self, content: &str, pii_types: &[PIIType]) -> ContextResult<AnonymizedContent>;

    /// Handle data retention policies
    pub async fn apply_retention_policy(&self, document_id: &str, policy: &RetentionPolicy) -> ContextResult<RetentionAction>;

    /// Track user consent for data processing
    pub async fn track_consent(&self, user_id: &str, consent_type: ConsentType, granted: bool) -> ContextResult<()>;

    /// Handle right to be forgotten requests
    pub async fn handle_deletion_request(&self, user_id: &str, scope: DeletionScope) -> ContextResult<DeletionResult>;

    /// Generate privacy compliance report
    pub async fn generate_privacy_report(&self, time_range: TimeRange) -> ContextResult<PrivacyReport>;
}

#[derive(Debug, Clone)]
pub enum PIIType {
    EmailAddress,
    PhoneNumber,
    SocialSecurityNumber,
    CreditCardNumber,
    PersonalName,
    Address,
    IPAddress,
    APIKey,
    Password,
    Custom(String),
}
```

## Integration Points

### Upstream Dependencies

```rust
/// Integration with other Symbiote crates
pub struct SymbioteIntegration {
    ai_client: AiClient,                    // For embeddings and semantic analysis
    storage_manager: StorageManager,        // For persistent data storage
    security_manager: SecurityManager,      // For access control and encryption
    vault_manager: VaultManager,           // For secure credential storage
}

impl SymbioteIntegration {
    /// Initialize context engine with Symbiote ecosystem
    pub async fn initialize_context_engine(&self, config: ContextConfig) -> ContextResult<ContextEngine>;

    /// Generate embeddings using AI crate
    pub async fn generate_embeddings(&self, content: &[String], model: &str) -> ContextResult<Vec<Embedding>>;

    /// Store context data using storage crate
    pub async fn persist_context_data(&self, data: &ContextData) -> ContextResult<()>;

    /// Validate permissions using security crate
    pub async fn validate_context_permissions(&self, user_id: &str, operation: &ContextOperation) -> ContextResult<bool>;

    /// Retrieve secure credentials from vault
    pub async fn get_database_credentials(&self, database_type: &str) -> ContextResult<DatabaseCredentials>;
}
```

### Downstream Consumers

```rust
/// Services that consume context engine
pub trait ContextConsumer {
    /// Receive context updates
    async fn on_context_updated(&self, update: ContextUpdate) -> ContextResult<()>;

    /// Handle context search results
    async fn on_search_results(&self, query: &str, results: &[ContextChunk]) -> ContextResult<()>;

    /// Process context insights
    async fn on_context_insights(&self, insights: &ContextInsights) -> ContextResult<()>;
}

/// Integration points for downstream services
#[derive(Debug, Clone)]
pub enum ContextUpdate {
    DocumentAdded { document_id: String, metadata: DocumentMetadata },
    DocumentUpdated { document_id: String, changes: Vec<Change> },
    DocumentDeleted { document_id: String },
    CollectionCreated { collection_id: String, name: String },
    CollectionDeleted { collection_id: String },
    IndexingCompleted { job_id: String, stats: IndexingStats },
}

/// Context engine event system
pub struct ContextEventBus {
    subscribers: HashMap<EventType, Vec<Box<dyn ContextConsumer>>>,
    event_queue: EventQueue,
    delivery_guarantees: DeliveryGuarantees,
}

impl ContextEventBus {
    /// Subscribe to context events
    pub async fn subscribe(&mut self, event_type: EventType, consumer: Box<dyn ContextConsumer>) -> ContextResult<SubscriptionId>;

    /// Publish context event
    pub async fn publish(&self, event: ContextEvent) -> ContextResult<()>;

    /// Unsubscribe from events
    pub async fn unsubscribe(&mut self, subscription_id: SubscriptionId) -> ContextResult<()>;
}
```

### External Service Integration

```rust
/// Integration with external services
pub struct ExternalServiceIntegration {
    git_integration: GitIntegration,
    lsp_integration: LSPIntegration,
    cloud_storage: CloudStorageIntegration,
    monitoring: MonitoringIntegration,
}

impl ExternalServiceIntegration {
    /// Integrate with Git for version control awareness
    pub async fn setup_git_integration(&mut self, repo_path: &str) -> ContextResult<()>;

    /// Connect to Language Server Protocol servers
    pub async fn setup_lsp_integration(&mut self, languages: &[Language]) -> ContextResult<()>;

    /// Configure cloud storage for backup/sync
    pub async fn setup_cloud_storage(&mut self, provider: CloudProvider, credentials: CloudCredentials) -> ContextResult<()>;

    /// Setup monitoring and observability
    pub async fn setup_monitoring(&mut self, config: MonitoringConfig) -> ContextResult<()>;
}
```

## Implementation Details

### Technology Stack

- **SQLite**: Fast metadata storage with FTS5 for text search
- **Qdrant**: Vector database for semantic similarity search
- **Neo4j**: Graph database for relationship modeling
- **Embedding Models**: OpenAI, Sentence Transformers, local models
- **Chunking**: Intelligent text and code chunking algorithms
- **Caching**: Multi-level caching for embeddings and results
- **Async Processing**: Tokio-based async processing pipeline

### Key Dependencies

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid", "json"] }
qdrant-client = "1.7"
neo4rs = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
tracing = "0.1"
moka = { version = "0.12", features = ["future"] }
tiktoken-rs = "0.5"
candle-core = "0.3"
candle-nn = "0.3"
candle-transformers = "0.3"
symbiote-core = { path = "../symbiote-core" }
symbiote-ai = { path = "../ai" }
symbiote-storage = { path = "../storage" }

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
```

### Chunking Strategies

```rust
pub trait ChunkingStrategy: Send + Sync {
    fn chunk(&self, content: &str) -> ContextResult<Vec<Chunk>>;
    
    fn optimal_chunk_size(&self) -> usize;
    
    fn overlap_size(&self) -> usize;
}

pub struct SemanticChunker {
    max_chunk_size: usize,
    overlap_size: usize,
    sentence_splitter: SentenceSplitter,
}

pub struct CodeAwareChunker {
    max_chunk_size: usize,
    language: String,
    preserve_functions: bool,
    preserve_classes: bool,
}

pub struct FixedSizeChunker {
    chunk_size: usize,
    overlap_size: usize,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: ChunkId,
    pub content: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub metadata: ChunkMetadata,
}
```

### Codebase Intelligence Components

```rust
/// Meta Glean-inspired codebase intelligence system
pub struct CodebaseIntelligence {
    lsp_manager: LSPManager,
    ast_indexer: ASTIndexer,
    semantic_indexer: SemanticIndexer,
    dependency_tracker: DependencyTracker,
    file_watcher: FileWatcher,
    incremental_indexer: IncrementalIndexer,
    change_propagator: ChangePropagator,
}

impl CodebaseIntelligence {
    pub async fn new(workspace_path: &Path) -> ContextResult<Self>;

    pub async fn index_workspace(&mut self, workspace_path: &Path) -> ContextResult<IndexingResult>;

    pub async fn handle_file_change(&self, file_path: &Path, change_type: ChangeType) -> ContextResult<()>;

    pub async fn get_symbol_info(&self, symbol: &str, file_path: &Path) -> ContextResult<SymbolInfo>;

    pub async fn find_references(&self, symbol: &str, file_path: &Path) -> ContextResult<Vec<Reference>>;

    pub async fn get_dependencies(&self, file_path: &Path) -> ContextResult<Vec<Dependency>>;
}

/// Language Server Protocol manager
pub struct LSPManager {
    servers: HashMap<Language, LSPServer>,
    client_capabilities: ClientCapabilities,
    workspace_folders: Vec<WorkspaceFolder>,
}

impl LSPManager {
    pub async fn new() -> ContextResult<Self>;

    pub async fn start_server(&mut self, language: Language, workspace_path: &Path) -> ContextResult<()>;

    pub async fn get_definition(&self, file_path: &Path, position: Position) -> ContextResult<Vec<Location>>;

    pub async fn get_references(&self, file_path: &Path, position: Position) -> ContextResult<Vec<Location>>;

    pub async fn get_hover_info(&self, file_path: &Path, position: Position) -> ContextResult<Option<Hover>>;

    pub async fn get_symbols(&self, file_path: &Path) -> ContextResult<Vec<DocumentSymbol>>;

    pub async fn get_diagnostics(&self, file_path: &Path) -> ContextResult<Vec<Diagnostic>>;
}

/// Abstract Syntax Tree indexer
pub struct ASTIndexer {
    parsers: HashMap<Language, TreeSitterParser>,
    ast_cache: LruCache<PathBuf, AST>,
    symbol_extractor: SymbolExtractor,
}

impl ASTIndexer {
    pub async fn new() -> ContextResult<Self>;

    pub async fn parse_file(&mut self, file_path: &Path) -> ContextResult<AST>;

    pub async fn extract_symbols(&self, ast: &AST, file_path: &Path) -> ContextResult<Vec<Symbol>>;

    pub async fn get_function_signatures(&self, file_path: &Path) -> ContextResult<Vec<FunctionSignature>>;

    pub async fn get_class_hierarchy(&self, file_path: &Path) -> ContextResult<ClassHierarchy>;

    pub async fn find_symbol_at_position(&self, file_path: &Path, position: Position) -> ContextResult<Option<Symbol>>;
}

/// Semantic indexer for code understanding
pub struct SemanticIndexer {
    embedding_model: EmbeddingModel,
    semantic_cache: SemanticCache,
    code_analyzer: CodeAnalyzer,
}

impl SemanticIndexer {
    pub async fn new(embedding_model: EmbeddingModel) -> ContextResult<Self>;

    pub async fn index_code_semantics(&mut self, file_path: &Path, code: &str) -> ContextResult<()>;

    pub async fn semantic_search(&self, query: &str, language: Option<Language>) -> ContextResult<Vec<SemanticMatch>>;

    pub async fn find_similar_functions(&self, function_code: &str) -> ContextResult<Vec<SimilarFunction>>;

    pub async fn analyze_code_intent(&self, code: &str) -> ContextResult<CodeIntent>;

    pub async fn get_code_embeddings(&self, code: &str) -> ContextResult<Vec<f32>>;
}

/// Dependency tracker for cross-file relationships
pub struct DependencyTracker {
    dependency_graph: DependencyGraph,
    import_resolver: ImportResolver,
    module_analyzer: ModuleAnalyzer,
}

impl DependencyTracker {
    pub async fn new() -> ContextResult<Self>;

    pub async fn track_file_dependencies(&mut self, file_path: &Path) -> ContextResult<()>;

    pub async fn get_dependencies(&self, file_path: &Path) -> ContextResult<Vec<Dependency>>;

    pub async fn get_dependents(&self, file_path: &Path) -> ContextResult<Vec<Dependent>>;

    pub async fn analyze_circular_dependencies(&self) -> ContextResult<Vec<CircularDependency>>;

    pub async fn get_dependency_chain(&self, from: &Path, to: &Path) -> ContextResult<Option<DependencyChain>>;
}

/// Incremental indexer for efficient updates
pub struct IncrementalIndexer {
    change_detector: ChangeDetector,
    fanout_calculator: FanoutCalculator,
    update_scheduler: UpdateScheduler,
}

impl IncrementalIndexer {
    pub async fn new() -> ContextResult<Self>;

    pub async fn handle_file_change(&self, file_path: &Path, change_type: ChangeType) -> ContextResult<UpdatePlan>;

    pub async fn calculate_fanout(&self, changed_file: &Path) -> ContextResult<Vec<PathBuf>>;

    pub async fn schedule_updates(&self, update_plan: UpdatePlan) -> ContextResult<()>;

    pub async fn get_update_status(&self) -> ContextResult<UpdateStatus>;
}

#[derive(Debug, Clone)]
pub struct SymbolDefinition {
    pub symbol: String,
    pub file_path: PathBuf,
    pub line: u32,
    pub column: u32,
    pub symbol_type: SymbolType,
    pub scope: String,
}

#[derive(Debug, Clone)]
pub struct CodeSearchResult {
    pub file_path: PathBuf,
    pub line: u32,
    pub column: u32,
    pub code_snippet: String,
    pub relevance_score: f64,
    pub context: CodeContext,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
    pub cycles: Vec<CircularDependency>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { old_path: PathBuf },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Other(String),
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Function,
    Class,
    Interface,
    Variable,
    Constant,
    Type,
    Module,
    Namespace,
}
```

## Testing Strategy

### Unit Tests

- **Storage Backends**: Test SQLite, Qdrant, and Neo4j operations
- **Embedding Generation**: Test embedding generation and caching
- **Chunking Strategies**: Test different chunking algorithms
- **Search Algorithms**: Test semantic and hybrid search
- **Graph Operations**: Test relationship modeling and traversal

### Integration Tests

- **End-to-End Indexing**: Test complete document indexing pipeline
- **Cross-Storage Consistency**: Test data consistency across storage backends
- **Performance**: Test search performance with large datasets
- **Concurrent Operations**: Test thread safety and concurrent access
- **Error Recovery**: Test error handling and recovery scenarios

### Performance Tests

- **Indexing Throughput**: Test document indexing performance
- **Search Latency**: Test search response times
- **Memory Usage**: Test memory consumption under load
- **Scalability**: Test performance with increasing dataset sizes

## Integration Points

### Upstream Dependencies

- **symbiote-core**: Uses error types, configuration, and async utilities
- **ai**: Uses embedding generation and AI provider integration
- **storage**: Uses SQLite storage backend

### Downstream Consumers

- **Agent Framework**: Context retrieval for agent reasoning
- **Assistant**: Context for personal AI assistant conversations
- **Workflow Engine**: Context for workflow execution
- **IDE Features**: Code context for development assistance
- **Memory System**: Integration with conversation and knowledge memory

### External Integrations

- **Vector Databases**: Qdrant, Pinecone, Weaviate
- **Graph Databases**: Neo4j, ArangoDB, Amazon Neptune
- **Search Engines**: Elasticsearch, OpenSearch
- **Embedding Services**: OpenAI, Cohere, Hugging Face

## Acceptance Criteria

### Functional Requirements

- [ ] Hybrid storage with SQLite + Qdrant + Neo4j
- [ ] Semantic search with vector embeddings
- [ ] Graph relationship modeling and traversal
- [ ] Intelligent content chunking strategies
- [ ] Real-time document indexing and updates
- [ ] Context window optimization for AI interactions
- [ ] Multi-modal content support

### Non-Functional Requirements

- [ ] Sub-second search response times for 1M+ documents
- [ ] 99.9% search accuracy for semantic queries
- [ ] Support for 10GB+ of indexed content
- [ ] Memory usage under 1GB for typical workloads
- [ ] Concurrent indexing and search operations
- [ ] Cross-platform compatibility

### Quality Gates

- [ ] All tests pass with 95%+ coverage
- [ ] Performance benchmarks meet targets
- [ ] Search accuracy meets quality thresholds
- [ ] Memory and CPU usage within limits
- [ ] Documentation complete with examples
- [ ] Integration tests pass for all storage backends

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with async support
- **Database Drivers**: SQLite, Qdrant, Neo4j client libraries
- **ML Libraries**: Candle or similar for local embeddings

### Runtime Dependencies

- **SQLite**: Local SQLite database file
- **Qdrant**: Optional Qdrant server for vector storage
- **Neo4j**: Optional Neo4j server for graph storage
- **Embedding Models**: Local or remote embedding generation
- **Memory**: Sufficient memory for embedding caches

### Development Prerequisites

- **Test Databases**: Local instances of Qdrant and Neo4j for testing
- **Sample Data**: Test datasets for indexing and search validation
- **Performance Tools**: Benchmarking and profiling tools
- **Documentation**: API documentation and usage examples

This context engine provides the intelligent, scalable, and performant foundation for all context-aware AI interactions in Symbiote, enabling sophisticated understanding and retrieval of relevant information across diverse content types and relationships.

## Enhanced Components (Master Plan Features)

### Advanced Context Engine Components

```rust
/// Commit indexer for git integration
pub struct CommitIndexer {
    git_analyzer: GitAnalyzer,
    commit_parser: CommitParser,
    diff_analyzer: DiffAnalyzer,
}

impl CommitIndexer {
    /// Git integration with commit lineage and file evolution
    pub async fn index_commit(&self, commit: &Commit) -> ContextResult<CommitIndex>;

    pub async fn analyze_file_evolution(&self, file_path: &Path) -> ContextResult<FileEvolution>;

    pub async fn track_author_connections(&self, commits: &[Commit]) -> ContextResult<AuthorGraph>;

    /// Branch/PR aware indexing
    pub async fn index_branch(&self, branch: &str) -> ContextResult<BranchIndex>;

    pub async fn index_pull_request(&self, pr: &PullRequest) -> ContextResult<PRIndex>;
}

/// Hybrid retriever with intelligent ranking
pub struct HybridRetriever {
    sql_querier: SQLQuerier,
    graph_traverser: GraphTraverser,
    vector_searcher: VectorSearcher,
    ranking_fusion: RankingFusion,
}

impl HybridRetriever {
    /// Hybrid retrieval strategy: SQL + Graph + Vector + Ranking Fusion
    pub async fn retrieve(&self, query: &str, context: &RetrievalContext) -> ContextResult<Vec<ContextChunk>>;

    /// Fast structured lookups (file paths, symbols)
    pub async fn sql_query(&self, query: &StructuredQuery) -> ContextResult<Vec<StructuredResult>>;

    /// Relationship-based context expansion
    pub async fn graph_traversal(&self, start_nodes: &[NodeId], depth: u32) -> ContextResult<Vec<GraphResult>>;

    /// Semantic similarity for relevant code
    pub async fn vector_search(&self, embedding: &Embedding, limit: usize) -> ContextResult<Vec<VectorResult>>;

    /// Combine results with intelligent scoring
    pub async fn rank_and_fuse(&self, results: &[SearchResult]) -> ContextResult<Vec<RankedResult>>;
}

/// Context ranker for intelligent scoring
pub struct ContextRanker {
    relevance_scorer: RelevanceScorer,
    freshness_scorer: FreshnessScorer,
    authority_scorer: AuthorityScorer,
    diversity_scorer: DiversityScorer,
}

impl ContextRanker {
    /// Intelligent context ranking and fusion
    pub async fn rank_context(&self, chunks: &[ContextChunk], query: &str) -> ContextResult<Vec<RankedChunk>>;

    pub async fn score_relevance(&self, chunk: &ContextChunk, query: &str) -> ContextResult<f64>;

    pub async fn score_freshness(&self, chunk: &ContextChunk) -> ContextResult<f64>;

    pub async fn score_authority(&self, chunk: &ContextChunk) -> ContextResult<f64>;

    pub async fn ensure_diversity(&self, chunks: &[RankedChunk]) -> ContextResult<Vec<RankedChunk>>;
}

/// Index versioning for embedder/version tracking
pub struct IndexVersioning {
    version_tracker: VersionTracker,
    embedding_tracker: EmbeddingTracker,
    threshold_manager: ThresholdManager,
}

impl IndexVersioning {
    /// Embedder/version params recorded; rolling re-embed thresholds
    pub async fn track_embedding_version(&self, chunk_id: &str, embedder: &str, version: &str) -> ContextResult<()>;

    pub async fn check_reembed_threshold(&self, chunk_id: &str) -> ContextResult<bool>;

    pub async fn schedule_reembedding(&self, chunks: &[String]) -> ContextResult<ReembedJob>;

    pub async fn get_version_stats(&self) -> ContextResult<VersionStats>;
}

/// Index compactor for optimization
pub struct IndexCompactor {
    deduplicator: ChunkDeduplicator,
    compressor: PQCompressor,
    hnsw_optimizer: HNSWOptimizer,
}

impl IndexCompactor {
    /// Dedupe chunks; PQ compression; configurable HNSW defaults
    pub async fn deduplicate_chunks(&self, chunks: &[ContextChunk]) -> ContextResult<Vec<ContextChunk>>;

    pub async fn compress_vectors(&self, vectors: &[Embedding]) -> ContextResult<Vec<CompressedEmbedding>>;

    pub async fn optimize_hnsw_index(&self, config: &HNSWConfig) -> ContextResult<OptimizedIndex>;

    pub async fn compact_index(&self, index: &VectorIndex) -> ContextResult<CompactedIndex>;
}

/// Context budget planner for token management
pub struct ContextBudgetPlanner {
    token_counter: TokenCounter,
    summarizer: ContextSummarizer,
    range_assembler: RangeAssembler,
}

impl ContextBudgetPlanner {
    /// Assemble ranges within token budgets with summarization
    pub async fn plan_context_budget(&self, chunks: &[ContextChunk], budget: TokenBudget) -> ContextResult<ContextPlan>;

    /// Range assembly with surgical retrieval and minimal line ranges
    pub async fn assemble_minimal_ranges(&self, chunks: &[ContextChunk]) -> ContextResult<AssembledRanges>;

    /// Budget-aware packing with token limits and summarization
    pub async fn pack_with_summarization(&self, context: &AssembledRanges, budget: TokenBudget) -> ContextResult<PackedContext>;

    /// Prepend dependency stubs for context
    pub async fn add_dependency_stubs(&self, context: &PackedContext) -> ContextResult<EnhancedContext>;
}

/// AST analyzer for structural analysis
pub struct ASTAnalyzer {
    parser_manager: ParserManager,
    symbol_extractor: SymbolExtractor,
    dependency_analyzer: DependencyAnalyzer,
}

impl ASTAnalyzer {
    /// AST-based structural analysis
    pub async fn analyze_ast(&self, code: &str, language: &str) -> ContextResult<ASTAnalysis>;

    pub async fn extract_symbols(&self, ast: &AST) -> ContextResult<Vec<Symbol>>;

    pub async fn analyze_dependencies(&self, ast: &AST) -> ContextResult<DependencyGraph>;

    /// Code-aware chunking with symbol bodies/blocks and stable anchors
    pub async fn chunk_by_symbols(&self, ast: &AST) -> ContextResult<Vec<SymbolChunk>>;

    pub async fn create_stable_anchors(&self, chunks: &[SymbolChunk]) -> ContextResult<Vec<AnchoredChunk>>;
}

/// Embedding provider manager for configurable providers
pub struct EmbeddingProviderManager {
    providers: HashMap<String, Box<dyn EmbeddingProvider>>,
    active_provider: String,
    fallback_providers: Vec<String>,
}

impl EmbeddingProviderManager {
    /// Multi-provider embeddings with user-configurable providers
    pub async fn embed_text(&self, text: &str) -> ContextResult<Embedding>;

    pub async fn embed_batch(&self, texts: &[String]) -> ContextResult<Vec<Embedding>>;

    pub async fn switch_provider(&mut self, provider: &str) -> ContextResult<()>;

    pub async fn add_provider(&mut self, name: String, provider: Box<dyn EmbeddingProvider>) -> ContextResult<()>;

    pub fn get_available_providers(&self) -> Vec<String>;
}

/// Incremental indexing pipeline
pub struct IncrementalIndexingPipeline {
    file_watcher: FileWatcher,
    git_watcher: GitWatcher,
    parser_pool: ParserPool,
    chunker: CodeAwareChunker,
    embedder: BatchEmbedder,
    graph_extractor: GraphExtractor,
    scheduler: IndexingScheduler,
}

impl IncrementalIndexingPipeline {
    /// Incremental indexing with git diff + FS watch, branch/PR aware
    pub async fn start_watching(&mut self) -> ContextResult<()>;

    /// Watchers: git diff + FS watch; branch/PR aware; backpressure on large repos
    pub async fn handle_file_change(&self, change: &FileChange) -> ContextResult<()>;

    /// Parsing: tree-sitter/LSP for AST, symbols, imports, references per language
    pub async fn parse_and_index(&self, file: &Path) -> ContextResult<IndexResult>;

    /// Chunking: code-aware splits (symbol bodies/blocks), stable anchors; no cross-symbol leakage
    pub async fn chunk_with_anchors(&self, content: &str, language: &str) -> ContextResult<Vec<AnchoredChunk>>;

    /// Embeddings: per chunk + symbol docstrings/tests; dedupe; rolling re-embed thresholds
    pub async fn embed_chunks(&self, chunks: &[AnchoredChunk]) -> ContextResult<Vec<EmbeddedChunk>>;

    /// Graph extraction: imports/calls/defines; test↔subject links; coverage overlay
    pub async fn extract_graph_relationships(&self, ast: &AST) -> ContextResult<GraphRelationships>;

    /// Schedules: background initial crawl; near-real-time incremental updates
    pub async fn schedule_background_crawl(&self, workspace: &Path) -> ContextResult<CrawlJob>;
}

/// Types for the enhanced context engine
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub max_tokens: usize,
    pub reserved_tokens: usize,
    pub summarization_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct ContextChunk {
    pub id: String,
    pub content: String,
    pub metadata: ChunkMetadata,
    pub embedding: Option<Embedding>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone)]
pub struct AssembledContext {
    pub chunks: Vec<ContextChunk>,
    pub ranges: Vec<LineRange>,
    pub dependencies: Vec<DependencyStub>,
    pub total_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct PackedContext {
    pub content: String,
    pub metadata: PackedMetadata,
    pub token_count: usize,
    pub summarized_sections: Vec<SummarizedSection>,
}

#[derive(Debug, Clone)]
pub struct RetrievalContext {
    pub workspace_id: String,
    pub query_type: QueryType,
    pub filters: Vec<ContextFilter>,
    pub preferences: RetrievalPreferences,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Semantic,
    Structural,
    Hybrid,
    Exact,
}
```
