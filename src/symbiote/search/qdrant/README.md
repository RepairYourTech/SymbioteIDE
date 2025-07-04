# Qdrant Semantic Search System

This directory contains the Qdrant vector database integration for SymbioteIDE, providing semantic search capabilities that complement the Neo4j graph-based code intelligence.

## Overview

The Qdrant system enables AI agents to understand code semantically by:
- Converting code into high-dimensional vector embeddings
- Storing and indexing these embeddings for fast similarity search
- Finding semantically similar code patterns across the codebase
- Combining with Neo4j structural data for comprehensive understanding

## Architecture

### Core Components

1. **QdrantConnectionManager** (`connection-manager.ts`)
   - Manages connections to Qdrant instances
   - Handles reconnection and health checks
   - Provides connection pooling and retry logic

2. **EmbeddingService** (`embedding-service.ts`)
   - Generates embeddings using various providers (OpenAI, Anthropic, local)
   - Handles batch processing and caching
   - Enhances code with context for better embeddings

3. **QdrantManager** (`qdrant-manager.ts`)
   - Main interface for semantic search operations
   - Manages collections and vector operations
   - Provides search and similarity functions

4. **CodeIndexer** (`code-indexer.ts`)
   - Processes codebases to create embeddings
   - Supports multiple programming languages
   - Handles incremental updates and file watching

5. **AIContextBuilder** (`ai-context-builder.ts`)
   - Builds comprehensive context for AI agents
   - Combines Qdrant semantic search with Neo4j structural data
   - Ranks and filters results by relevance

## How It Works

### 1. Code Embedding Process

```typescript
// Code is processed through multiple steps:
1. Parse code file to extract entities (functions, classes, etc.)
2. Enhance each entity with metadata (name, type, docstring)
3. Generate embedding vector using AI model
4. Store in Qdrant with searchable metadata
```

### 2. Semantic Search Flow

```typescript
// When AI agents need code context:
1. Query is converted to embedding vector
2. Similar vectors are found in Qdrant
3. Results are enhanced with Neo4j graph data
4. Context is ranked by relevance
5. Formatted context returned to AI agent
```

### 3. Integration with Neo4j

The system maintains bidirectional links between:
- Qdrant embeddings (semantic similarity)
- Neo4j nodes (structural relationships)

This allows queries like:
- "Find code similar to this function that imports React"
- "Show me all error handling patterns in authentication modules"
- "Find implementations similar to this that are called by controllers"

## Usage

### Basic Setup

```typescript
import { QdrantManager, CodeIndexer, AIContextBuilder } from './qdrant';

// Initialize Qdrant connection
const qdrantManager = new QdrantManager({
  host: 'localhost',
  port: 6333
});

await qdrantManager.connect();

// Create code indexer
const indexer = new CodeIndexer(qdrantManager);

// Index a codebase
await indexer.indexCodebase({
  rootPath: '/path/to/project',
  includes: ['**/*.ts', '**/*.js'],
  excludes: ['**/node_modules/**']
});
```

### AI Agent Context Building

```typescript
// Create context builder for AI agents
const contextBuilder = new AIContextBuilder(qdrantManager, neo4jService);

// Update current context
contextBuilder.updateContext({
  taskDescription: 'Add authentication to user endpoints',
  currentFile: 'src/controllers/userController.ts'
});

// Find relevant code
const context = await contextBuilder.findRelevantCode(
  'implement JWT authentication'
);

// Use context in AI prompt
const prompt = `
Task: ${contextBuilder.taskDescription}
Current file: ${contextBuilder.currentFile}

Relevant code context:
${context.summary}

Primary examples:
${context.primary.map(r => r.code).join('\n---\n')}
`;
```

### Semantic Search Examples

```typescript
// Natural language search
const results = await qdrantManager.search({
  query: 'error handling in async functions',
  queryType: 'natural_language',
  filters: {
    language: ['typescript', 'javascript'],
    type: [CodeEntityType.Function]
  },
  limit: 10
});

// Code similarity search
const similar = await qdrantManager.searchSimilar(entityId, 10);

// Pattern discovery
const patterns = await contextBuilder.findSimilarPatterns(codeSnippet);
```

## Configuration

### Embedding Models

The system supports multiple embedding providers:

```typescript
// OpenAI (recommended for quality)
const embeddingConfig = {
  provider: EmbeddingProvider.OpenAI,
  model: 'text-embedding-3-large',
  dimensions: 1536,
  maxTokens: 8192
};

// Google (good balance of quality and cost)
const googleConfig = {
  provider: EmbeddingProvider.Google,
  model: 'text-embedding-004',  // Latest Google embedding model
  dimensions: 768,  // Supports 256, 512, or 768
  maxTokens: 8192,
  options: {
    apiKey: process.env.GOOGLE_API_KEY
  }
};

// Local model (for privacy)
const localConfig = {
  provider: EmbeddingProvider.Local,
  model: 'sentence-transformers/all-mpnet-base-v2',
  dimensions: 768
};
```

### Google Embedding Models

Google offers several embedding models:
- `text-embedding-004` - Latest model with best performance
- `textembedding-gecko@003` - Previous generation model
- `textembedding-gecko-multilingual@001` - Multi-language support

Features:
- Variable dimensions (256, 512, 768)
- Batch embedding support
- Cost-effective pricing
- High quality for semantic search

### Collection Configuration

```typescript
const collectionConfig = {
  name: 'code_embeddings',
  vectorSize: 1536,
  distance: 'Cosine', // Best for semantic similarity
  optimizers: {
    memmap_threshold: 20000,
    indexing_threshold: 10000
  }
};
```

## Performance Optimization

### 1. Batch Processing
- Process files in batches to reduce API calls
- Use parallel processing for local embeddings
- Cache embeddings to avoid recomputation

### 2. Incremental Updates
- Only re-index changed files
- Use file watchers for real-time updates
- Maintain consistency with Neo4j

### 3. Search Optimization
- Use filters to reduce search space
- Implement relevance scoring
- Cache frequent queries

## Integration Points

### 1. Orchestration Engine
The orchestration engine uses the context builder to provide relevant code context to AI models:

```typescript
class OrchestrationEngine {
  async buildPromptContext(task: string): Promise<string> {
    const codeContext = await this.contextBuilder.findRelevantCode(task);
    return this.formatContext(codeContext);
  }
}
```

### 2. Chat Interface
The chat interface uses semantic search to find relevant code when users ask questions:

```typescript
class ChatHandler {
  async handleCodeQuestion(question: string): Promise<Response> {
    const context = await this.contextBuilder.findRelevantCode(question);
    return this.generateResponse(question, context);
  }
}
```

### 3. MCP Tools
MCP tools can use semantic search to find similar implementations:

```typescript
class MCPTool {
  async findSimilarImplementations(code: string): Promise<any[]> {
    const patterns = await this.contextBuilder.findSimilarPatterns(code);
    return this.formatPatterns(patterns);
  }
}
```

## Best Practices

### 1. Embedding Quality
- Include rich metadata (docstrings, comments, types)
- Use appropriate chunking for large files
- Maintain consistent naming conventions

### 2. Search Relevance
- Combine semantic and structural search
- Use context-aware ranking
- Filter by recent activity

### 3. Performance
- Index during off-peak hours
- Use incremental updates
- Monitor embedding costs

## Troubleshooting

### Common Issues

1. **Poor Search Results**
   - Check embedding model configuration
   - Verify metadata is being included
   - Adjust similarity threshold

2. **Slow Indexing**
   - Reduce batch size
   - Use local embedding model
   - Enable parallel processing

3. **Memory Issues**
   - Configure Qdrant memory limits
   - Use collection optimization
   - Implement pagination

## Future Enhancements

1. **Multi-Modal Embeddings**
   - Include diagram understanding
   - Process documentation images
   - Support video tutorials

2. **Advanced Patterns**
   - Automatic pattern extraction
   - Design pattern recognition
   - Anti-pattern detection

3. **Mem0 Integration**
   - Persistent pattern memory
   - Team knowledge sharing
   - Learning from corrections