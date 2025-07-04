# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build & Compile
```bash
npm run compile          # Quick TypeScript compilation with SWC
npm run watch           # Watch mode with auto-recompilation
npm run build           # Full production build (includes ADK setup)
npm run start           # Launch SymbioteIDE in development mode
npm run start:dev       # Start with watch mode enabled
./scripts/code.sh       # Alternative launch method
```

### Testing
```bash
npm test                # Run all tests
npm run test-node       # Unit tests with Jest
npm run test-browser    # Browser tests with Playwright
npm run test-extension  # VS Code extension tests

# Run specific test file
npx jest path/to/test.spec.ts
```

### Distribution
```bash
npm run dist           # Create installers for all platforms
npm run dist:win       # Windows installer
npm run dist:mac       # macOS installer
npm run dist:linux     # Linux installer
```

## Architecture Overview

SymbioteIDE is a standalone application forked from VS Code, with AI capabilities deeply integrated at the platform level. The codebase merges VS Code's foundation with custom AI features:

- `src/vs/*` - Core VS Code functionality (upstream code - modify carefully to maintain compatibility)
- `src/symbiote/*` - AI enhancements and SymbioteIDE-specific features (our primary development area)

This is a full fork, not extensions - SymbioteIDE ships as its own branded application with custom installers while maintaining compatibility with VS Code extensions.

### Key Architectural Components

#### AI & Orchestration Layer
- **Orchestration Engine** (`src/symbiote/orchestration/`) - Routes requests to appropriate AI models based on context and capabilities
- **ADK (Agent Development Kit)** (`src/symbiote/adk/`) - Framework for creating specialized AI agents. Key files:
  - `agent-orchestrator.ts` - Manages agent lifecycle and coordination
  - `agent-factory.ts` - Creates agents from templates
  - `specialized-agents/` - Pre-built agents for specific tasks
- **MCP Integration** (`src/symbiote/mcp/`) - Model Context Protocol for standardized AI communication

#### Memory & Knowledge Systems
- **Memory System** (`src/symbiote/memory/mem0/`) - Persistent context using Mem0
- **Knowledge Graph** (`src/symbiote/neo4j/`) - Code relationships and dependencies
- **Vector Search** (`src/symbiote/memory/qdrant/`) - Semantic code search

#### Communication Infrastructure
- **WebSocket Server** (`src/symbiote/websocket/`) - Real-time bidirectional communication
- **A2A Protocol** (`src/symbiote/a2a/`) - Agent-to-agent communication protocol
- **Queue System** (`src/symbiote/queue/`) - Redis-based task queue for async operations

#### User Interfaces
- **Chat Interface** (`src/symbiote/chat/`) - Multimodal AI chat integrated into VS Code
- **Agent Builder** (`src/symbiote/agent-builder/`) - Visual interface for creating custom agents
- **AI Browser** (`src/symbiote/browser/`) - Enhanced webview with AI capabilities

### Extension Integration Points

When extending SymbioteIDE functionality:

1. **VS Code Integration**: Use standard VS Code extension APIs in `src/symbiote/extension.ts`
2. **AI Features**: Implement through ADK agents or orchestration engine
3. **WebSocket Events**: Register handlers in `websocket-server.ts` for real-time features
4. **MCP Tools**: Add new tools in `src/symbiote/mcp/client/`

### Development Guidelines

- **TypeScript**: Strict mode enabled, use explicit types
- **Async/Await**: Preferred over callbacks and promises
- **Error Handling**: All AI operations should have proper error boundaries
- **Testing**: Minimum 60% coverage, mock external services
- **Code Style**: Tabs for indentation, PascalCase for types, camelCase for functions

### Common Development Tasks

#### Adding a New AI Agent
1. Create agent in `src/symbiote/adk/specialized-agents/`
2. Register in `agent-factory.ts`
3. Add orchestration rules if needed
4. Write tests in corresponding `__tests__` directory

#### Integrating External AI Services
1. Add service client in appropriate directory
2. Create MCP adapter if using Model Context Protocol
3. Update orchestration engine configuration
4. Add credentials handling in `src/symbiote/credentials/`

#### Debugging AI Features
- Use VS Code's built-in debugger with launch configurations
- Enable WebSocket debug logging: `DEBUG=symbiote:*`
- Check orchestration logs in `src/symbiote/orchestration/logs/`
- Use the AI debugging panel for multi-agent traces

### Environment Setup

Required services for full functionality:
- Redis (for queue and caching)
- Neo4j (for knowledge graph)
- Qdrant (for vector search)
- Supabase (for authentication)

These can be configured in environment variables or use the default Docker setup.