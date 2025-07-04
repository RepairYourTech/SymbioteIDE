# MCP Context Server

The MCP Context Server exposes the unified Neo4j + Qdrant + Mem0 context system as an MCP server, allowing any MCP-compatible AI tool to access comprehensive code understanding, persistent memory, and team knowledge.

## Overview

This server combines three powerful systems:
- **Neo4j**: Graph database for structural code relationships
- **Qdrant**: Vector database for semantic code search
- **Mem0**: Persistent inter-agent memory system

## Features

### Available Tools

1. **search_context** - Search across all context sources
   - Combines code search, memory search, and graph insights
   - Returns unified context with relevance scoring

2. **store_memory** - Store memories for future reference
   - Episodic, semantic, working, or long-term memory types
   - Agent, project, team, or global scope

3. **search_memories** - Search specific memories
   - Filter by type, scope, and metadata
   - Semantic similarity search

4. **learn_from_task** - Record learnings from task execution
   - Automatically extracts patterns from successes/failures
   - Updates team knowledge base

5. **find_similar_code** - Find similar implementations
   - Semantic code similarity search
   - Returns patterns and examples

6. **get_file_context** - Get comprehensive file context
   - All references, usages, and related memories
   - Graph relationships and dependencies

7. **get_team_knowledge** - Access shared team knowledge
   - Coding conventions and best practices
   - Common patterns and known issues

8. **share_memory** - Share memories with team
   - Make local discoveries available to all agents
   - Build collective intelligence

### Available Resources

- `memory://recent` - Recently accessed memories
- `memory://patterns` - Learned code patterns
- `memory://team-knowledge` - Team knowledge base

## Setup

### Prerequisites

1. **Redis** - For distributed caching and pub/sub
   ```bash
   docker run -d -p 6379:6379 redis:alpine
   ```

2. **Qdrant** - For semantic search
   ```bash
   docker run -d -p 6333:6333 qdrant/qdrant
   ```

3. **Neo4j** - For graph database
   ```bash
   docker run -d \
     -p 7474:7474 -p 7687:7687 \
     -e NEO4J_AUTH=neo4j/password \
     neo4j:latest
   ```

4. **Mem0** - Sign up at https://mem0.ai for API key

### Environment Variables

```bash
# Redis
REDIS_HOST=localhost
REDIS_PORT=6379

# Qdrant
QDRANT_HOST=localhost
QDRANT_PORT=6333

# Neo4j
NEO4J_URI=bolt://localhost:7687
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=password

# Mem0
MEM0_API_KEY=your-api-key
MEM0_BASE_URL=https://api.mem0.ai

# Embedding Models (choose one)
OPENAI_API_KEY=your-key        # For OpenAI embeddings
GOOGLE_API_KEY=your-key        # For Google embeddings
```

### Installation

```bash
cd src/symbiote/mcp/servers
npm install
npm run build
```

## Usage

### As MCP Server

Add to your MCP client configuration (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "symbiote-context": {
      "command": "node",
      "args": ["/path/to/symbiote/mcp/servers/context-server.js"],
      "env": {
        "REDIS_HOST": "localhost",
        "QDRANT_HOST": "localhost",
        "NEO4J_URI": "bolt://localhost:7687",
        "NEO4J_USERNAME": "neo4j",
        "NEO4J_PASSWORD": "password",
        "MEM0_API_KEY": "your-key",
        "OPENAI_API_KEY": "your-key"
      }
    }
  }
}
```

### Direct Usage

```bash
# Start the server
npm start

# Or with custom config
REDIS_HOST=myredis.com npm start
```

## Examples

### Search for Context

```javascript
// MCP tool call
{
  "tool": "search_context",
  "arguments": {
    "query": "implement authentication middleware",
    "includeCode": true,
    "includeMemories": true,
    "includeGraph": true,
    "limit": 10
  }
}
```

### Store a Learning

```javascript
{
  "tool": "learn_from_task",
  "arguments": {
    "taskDescription": "Add JWT authentication to API endpoints",
    "outcome": "success",
    "solution": "Used passport-jwt middleware with RS256 tokens",
    "duration": 3600000,
    "filesModified": [
      "src/middleware/auth.ts",
      "src/routes/api.ts"
    ]
  }
}
```

### Find Similar Code

```javascript
{
  "tool": "find_similar_code",
  "arguments": {
    "code": "async function validateUser(token: string): Promise<User> { ... }",
    "language": "typescript",
    "limit": 5
  }
}
```

## Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────┐
│   MCP Client    │────▶│ Context      │────▶│   Redis     │
│ (Claude, etc.)  │     │ Server       │     │  Pub/Sub    │
└─────────────────┘     └──────────────┘     └─────────────┘
                               │
                    ┌──────────┴───────────┐
                    │                      │
              ┌─────▼─────┐         ┌─────▼─────┐
              │  Unified  │         │   Memory  │
              │    API    │         │  Manager  │
              └─────┬─────┘         └─────┬─────┘
                    │                      │
        ┌───────────┼───────────┬─────────┼────────┐
        │           │           │         │        │
   ┌────▼───┐  ┌───▼────┐  ┌──▼───┐  ┌──▼───┐   │
   │ Qdrant │  │ Neo4j  │  │ Mem0 │  │Redis │   │
   │(Search)│  │(Graph) │  │(Mem) │  │Cache │   │
   └────────┘  └────────┘  └──────┘  └──────┘   │
```

## Performance

- **Caching**: Redis caches frequent queries (5min TTL)
- **Parallel Search**: All three systems queried in parallel
- **Smart Routing**: Only queries relevant systems based on request
- **Batch Operations**: Supports batch memory storage
- **Real-time Updates**: Pub/sub for instant memory sync

## Security

- All connections support TLS
- API keys stored in environment variables
- Memory access controlled by scope
- Rate limiting via Redis

## Troubleshooting

### Connection Issues
- Verify all services are running
- Check firewall rules
- Test connections individually

### Performance Issues
- Monitor Redis memory usage
- Check Qdrant index optimization
- Review Neo4j query performance

### Memory Issues
- Set appropriate TTLs
- Implement memory decay policies
- Monitor storage usage

## Contributing

See [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](../../../../LICENSE) for details.