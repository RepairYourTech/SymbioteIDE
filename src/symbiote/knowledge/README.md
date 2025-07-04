# Neo4j Code Intelligence System

A comprehensive knowledge graph system that enables AI to perfectly understand codebases through Neo4j graph databases. The system maintains both per-project and global knowledge graphs, providing real-time code intelligence for AI operations.

## Architecture

### Multi-Graph Design

- **Per-Project Graphs**: Each VS Code workspace gets its own isolated Neo4j database
- **Global Knowledge Graph**: Shared graph for common libraries, frameworks, and patterns
- **Graph Federation**: Query across multiple graphs when needed
- **Automatic Provisioning**: New projects automatically get their own graph database

### Core Components

#### 1. Connection Manager (`neo4j/connection-manager.ts`)
Manages multiple Neo4j connections with:
- Connection pooling and lifecycle management
- Automatic database provisioning for new projects
- Health monitoring and auto-reconnection
- Support for both local and cloud Neo4j instances

#### 2. Schema Manager (`neo4j/schema-manager.ts`)
Defines and enforces the graph schema:
- **Node Types**: File, Class, Function, Method, Variable, Interface, Type, Module, Package
- **Relationship Types**: CONTAINS, IMPORTS, EXPORTS, CALLS, EXTENDS, IMPLEMENTS, REFERENCES, DEPENDS_ON
- Automatic constraint and index creation
- Schema versioning and migration support

#### 3. AST Parser (`parsers/typescript-parser.ts`)
Extracts code structure from source files:
- Full TypeScript/JavaScript AST parsing
- Accurate entity and relationship extraction
- Complexity analysis and metrics
- Support for modern language features

#### 4. Ingestion Pipeline (`ingestion/ingestion-pipeline.ts`)
Efficiently indexes entire codebases:
- Parallel file processing
- Batch operations for performance
- Progress tracking and cancellation
- Incremental updates support

#### 5. Sync Engine (`sync/graph-sync-engine.ts`)
Keeps graphs synchronized with code changes:
- Real-time file watching
- Debounced batch updates
- Git-aware synchronization
- Handles creates, updates, deletes, and renames

#### 6. AI Interface (`ai-integration/ai-graph-interface.ts`)
AI-friendly query and update interface:
- Natural language to Cypher conversion
- Context-aware query execution
- Impact analysis for changes
- Automatic graph updates from AI modifications

## Features

### For AI Understanding

#### Natural Language Queries
```typescript
const response = await knowledgeGraph.queryWithAI({
  query: "Show me all React components that use the useState hook",
  context: {
    currentFile: "/src/components/Dashboard.tsx",
    taskType: "analyze"
  },
  requirements: {
    includeImplementationDetails: true,
    includeUsageExamples: true,
    maxDepth: 2
  }
});
```

#### Code Context Retrieval
```typescript
// Get complete context for AI before making changes
const context = await knowledgeGraph.getAIContext(
  projectId,
  "/src/services/user.ts",
  150 // line number
);

// Returns:
// - Current entity (function/class/etc)
// - Parent context
// - Sibling entities
// - Dependencies
// - Usage examples
```

#### Impact Analysis
```typescript
// Before AI makes changes, analyze impact
const impact = await knowledgeGraph.analyzeChangeImpact(
  projectId,
  "UserService.authenticate", // entity ID
  "modify"
);

// Returns:
// - Affected files and entities
// - Risk level assessment
// - Suggested mitigation steps
```

### Real-Time Synchronization

The system automatically updates the graph when:
- Files are created, modified, or deleted
- Code is refactored or moved
- Dependencies are added or removed
- Git operations occur (branch switches, merges)

### Graph Schema

#### Code Entities (Nodes)

**File Node**
```cypher
(:File {
  id: String,
  path: String,
  name: String,
  extension: String,
  language: String,
  size: Integer,
  hash: String
})
```

**Class Node**
```cypher
(:Class {
  id: String,
  name: String,
  visibility: String,
  isAbstract: Boolean,
  superClass: String?,
  interfaces: String[],
  filePath: String,
  startLine: Integer,
  endLine: Integer
})
```

**Function/Method Node**
```cypher
(:Function/:Method {
  id: String,
  name: String,
  signature: String,
  parameters: ParameterInfo[],
  returnType: String?,
  isAsync: Boolean,
  complexity: Integer,
  filePath: String,
  startLine: Integer,
  endLine: Integer
})
```

#### Relationships

- `(File)-[:CONTAINS]->(Class/Function/Variable)`
- `(Class)-[:EXTENDS]->(Class)`
- `(Class)-[:IMPLEMENTS]->(Interface)`
- `(Function)-[:CALLS]->(Function)`
- `(Module)-[:IMPORTS]->(Module/Class/Function)`
- `(Package)-[:DEPENDS_ON]->(Package)`

### Query Examples

#### Find all implementations of an interface
```cypher
MATCH (i:Interface {name: "UserRepository"})
MATCH (c:Class)-[:IMPLEMENTS]->(i)
RETURN c
```

#### Trace call hierarchy
```cypher
MATCH path = (f1:Function {name: "processPayment"})-[:CALLS*1..3]->(f2:Function)
RETURN path
```

#### Find unused code
```cypher
MATCH (f:Function)
WHERE NOT (f)<-[:CALLS]-()
  AND NOT f.name STARTS WITH "test"
RETURN f
```

## Usage

### Initialize a Project

```typescript
import { createKnowledgeGraphSystem } from './symbiote/knowledge';

const knowledgeSystem = createKnowledgeGraphSystem(orchestrationEngine, {
  connectionConfig: {
    globalGraphConfig: {
      context: 'global',
      uri: 'bolt://localhost:7687',
      username: 'neo4j',
      password: 'password'
    }
  }
});

// Initialize project with full indexing and file watching
const projectContext = await knowledgeSystem.initializeProject(
  'my-project-id',
  '/path/to/workspace',
  {
    fullIndex: true,
    watchFiles: true
  }
);
```

### Query the Graph

```typescript
// AI-friendly natural language query
const response = await knowledgeSystem.queryWithAI({
  query: "Find all API endpoints that handle user authentication",
  context: { projectId: 'my-project-id' },
  requirements: {
    includeImplementationDetails: true,
    includeDependencies: true
  }
});

// Direct Cypher query
const result = await knowledgeSystem.executeGraphQuery({
  cypher: 'MATCH (c:Class)-[:EXTENDS*]->(base:Class {name: "BaseController"}) RETURN c',
  parameters: {}
});
```

### Update from AI Changes

```typescript
// After AI generates code changes
const updateResult = await knowledgeSystem.updateFromAI(
  'my-project-id',
  [
    {
      type: 'create',
      filePath: '/src/services/new-service.ts',
      code: generatedCode
    },
    {
      type: 'modify',
      entityId: 'existing-function-id',
      code: modifiedCode
    }
  ]
);
```

## Configuration

### Environment Variables

```bash
# Neo4j connection
NEO4J_URI=bolt://localhost:7687
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=your-password

# Optional: Neo4j Aura (cloud)
NEO4J_AURA_URI=neo4j+s://xxxxx.databases.neo4j.io
NEO4J_AURA_USERNAME=neo4j
NEO4J_AURA_PASSWORD=your-aura-password
```

### Performance Tuning

```typescript
const config = {
  ingestionConfig: {
    batchSize: 100,        // Files per batch
    parallelWorkers: 4,    // Parallel processing threads
    maxFileSize: 10485760  // Skip files larger than 10MB
  },
  syncConfig: {
    debounceDelay: 500,    // Milliseconds to wait before processing changes
    batchUpdates: true     // Group multiple changes together
  },
  aiConfig: {
    maxQueryDepth: 3,      // Maximum graph traversal depth
    maxResultNodes: 100    // Maximum nodes to return
  }
};
```

## Best Practices

### For AI Integration

1. **Always Query Before Modifying**: AI should query the graph to understand context before making changes
2. **Use Impact Analysis**: Check impact before making breaking changes
3. **Update Graph After Changes**: Ensure the graph reflects all AI-generated code
4. **Cache Frequent Queries**: The AI interface caches results for performance

### For Performance

1. **Use Specific Queries**: More specific queries perform better than broad searches
2. **Limit Traversal Depth**: Deep traversals can be expensive
3. **Index Hot Paths**: Frequently accessed code should be well-indexed
4. **Batch Updates**: Group multiple changes together

### For Accuracy

1. **Keep Parsers Updated**: Ensure parsers support latest language features
2. **Validate Graph Integrity**: Periodically check for orphaned nodes
3. **Handle Edge Cases**: Account for dynamic code, metaprogramming
4. **Track Confidence**: Mark AI-inferred relationships with confidence scores

## Troubleshooting

### Common Issues

#### Connection Failed
- Check Neo4j is running: `neo4j status`
- Verify credentials in environment variables
- Ensure firewall allows port 7687

#### Slow Queries
- Add indexes for frequently queried properties
- Reduce query complexity or depth
- Use query profiling: `PROFILE <query>`

#### Out of Sync
- Force resync: `knowledgeSystem.rebuildIndex(projectId)`
- Check file watcher is active
- Verify no permission issues

## Future Enhancements

- [ ] Support for more languages (Python, Java, Go)
- [ ] Vector embeddings for semantic search
- [ ] Cross-project dependency tracking
- [ ] Historical code evolution analysis
- [ ] Advanced pattern detection
- [ ] Integration with debugging data
- [ ] Performance profiling integration