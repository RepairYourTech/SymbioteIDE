# SymbioteIDE Development Guide

This guide provides detailed information for developers working on SymbioteIDE.

## Development Environment Setup

### System Requirements

- **OS**: Windows 10+, macOS 11+, Ubuntu 20.04+
- **RAM**: 8GB minimum, 16GB recommended
- **Disk**: 10GB free space
- **Node.js**: 18.15.0 or higher
- **npm**: 9.0.0 or higher

### Initial Setup

1. **Clone and Install**
   ```bash
   git clone https://github.com/symbiote-ide/symbiote-ide.git
   cd symbiote-ide
   npm run setup
   npm install
   ```

2. **Configure API Keys**
   ```bash
   cp .env.example .env
   # Edit .env with your API keys
   ```

3. **Install VS Code Extensions**
   - ESLint
   - Prettier
   - TypeScript and JavaScript Language Features

## Development Commands

### Essential Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start full development environment |
| `npm run dev:quick` | Start SymbioteIDE modules only |
| `npm run compile` | Compile all TypeScript |
| `npm run watch` | Watch mode compilation |
| `npm test` | Run test suite |
| `npm run lint` | Run linter with auto-fix |
| `npm run format` | Format code with Prettier |

### Build Commands

| Command | Description |
|---------|-------------|
| `npm run build` | Production build |
| `npm run package` | Package for current platform |
| `npm run package:all` | Package for all platforms |
| `npm run release` | Create release build |

### Utility Commands

| Command | Description |
|---------|-------------|
| `npm run clean` | Clean build artifacts |
| `npm run typecheck` | Run TypeScript type checking |
| `npm run update-branding` | Update branding across codebase |
| `npm run generate-icons` | Generate icon files |

## Architecture Deep Dive

### Module Architecture

```
┌─────────────────────────────────────────┐
│           SymbioteIDE Shell             │
├─────────────────────────────────────────┤
│         Orchestration Engine            │
├────────────┬────────────┬──────────────┤
│   Agents   │    MCP     │   Memory     │
├────────────┼────────────┼──────────────┤
│ Knowledge  │  Terminal  │   Branding   │
├────────────┴────────────┴──────────────┤
│          VS Code Core (Fork)            │
└─────────────────────────────────────────┘
```

### Key Components

#### 1. Orchestration Engine (`src/symbiote/orchestration/`)

Manages AI model selection and routing:

```typescript
// Example usage
const engine = new OrchestrationEngine();
const result = await engine.executeTask({
  type: TaskType.CodeGeneration,
  prompt: "Create a React component",
  constraints: {
    maxCost: 0.10,
    maxLatency: 5000
  }
});
```

#### 2. MCP Client (`src/symbiote/mcp/`)

Implements Model Context Protocol:

```typescript
// Example MCP server connection
const client = new MCPClient();
await client.connect({
  id: 'my-server',
  type: 'stdio',
  command: 'my-mcp-server'
});
```

#### 3. Agent System (`src/symbiote/agents/`)

Manages AI agents:

```typescript
// Example agent creation
const agent = await agentManager.createAgent({
  name: 'Code Reviewer',
  type: AgentType.Review,
  tools: ['eslint', 'prettier']
});
```

### Data Flow

1. **User Input** → IDE Interface
2. **Task Creation** → Orchestration Engine
3. **Model Selection** → Based on task type and constraints
4. **Execution** → Via appropriate provider
5. **Result Processing** → Knowledge graph update
6. **Response** → Back to user

## Debugging

### VS Code Launch Configurations

1. **Launch SymbioteIDE** - Full application debugging
2. **Debug Tests** - Test debugging with breakpoints
3. **Debug MCP Server** - MCP communication debugging
4. **Debug Web Server** - Web version debugging

### Common Debugging Scenarios

#### Debugging Orchestration Issues
```typescript
// Enable debug logging
process.env.DEBUG = 'symbiote:orchestration:*';
```

#### Debugging MCP Communication
```typescript
// Use MCP debug mode
const client = new MCPClient({ debug: true });
```

#### Memory Leak Detection
```bash
# Run with memory profiling
node --inspect --max-old-space-size=4096 ./out/main.js
```

## Testing Strategy

### Test Structure

```
src/
├── component.ts
├── component.test.ts    # Unit tests
└── __tests__/
    └── component.integration.test.ts
```

### Writing Tests

#### Unit Test Example
```typescript
describe('OrchestrationEngine', () => {
  let engine: OrchestrationEngine;

  beforeEach(() => {
    engine = new OrchestrationEngine();
  });

  it('should select cheapest model within constraints', async () => {
    const result = await engine.selectModel({
      type: TaskType.General,
      constraints: { maxCost: 0.05 }
    });
    
    expect(result.estimatedCost).toBeLessThanOrEqual(0.05);
  });
});
```

#### Integration Test Example
```typescript
describe('Agent Integration', () => {
  it('should complete code review workflow', async () => {
    const agent = await createTestAgent();
    const result = await agent.review({
      files: ['test.ts'],
      rules: ['no-console']
    });
    
    expect(result.issues).toHaveLength(1);
  });
});
```

### Test Coverage Requirements

- Minimum 60% overall coverage
- 80% coverage for critical paths
- 100% coverage for API interfaces

## Performance Optimization

### Compilation Performance

```json
// tsconfig optimizations
{
  "compilerOptions": {
    "incremental": true,
    "tsBuildInfoFile": ".build/cache"
  }
}
```

### Runtime Performance

1. **Lazy Loading**: Load modules on demand
2. **Caching**: Use memory and disk caching
3. **Parallel Processing**: Utilize worker threads
4. **Debouncing**: Limit API calls

### Memory Management

- Monitor with `process.memoryUsage()`
- Implement cleanup in `dispose()` methods
- Use WeakMap for object associations
- Clear caches periodically

## Troubleshooting

### Common Issues

#### Build Failures

```bash
# Clean and rebuild
npm run clean
npm install
npm run compile
```

#### Extension Compatibility

```bash
# Test extension compatibility
node scripts/test-extension-compatibility.js
```

#### Development Server Issues

```bash
# Reset development environment
rm -rf node_modules .build out
npm install
npm run dev
```

### Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| SYM001 | Orchestration timeout | Increase timeout or check model availability |
| SYM002 | MCP connection failed | Verify server configuration |
| SYM003 | Memory limit exceeded | Increase memory allocation |
| SYM004 | Knowledge graph error | Check Neo4j/Qdrant connection |

## Advanced Topics

### Adding Custom MCP Servers

1. Create server implementation
2. Add to `.mcp.json`
3. Implement client handler
4. Add tests

### Extending the Agent System

1. Define new agent type
2. Implement agent class
3. Create agent template
4. Register with factory

### Custom AI Providers

1. Implement provider interface
2. Add to provider registry
3. Configure in settings
4. Add cost calculations

## Resources

- [API Documentation](./API.md)
- [Architecture Diagrams](./ARCHITECTURE.md)
- [Testing Guide](./TESTING.md)
- [Security Guide](./SECURITY.md)

## Getting Help

- GitHub Issues: Bug reports and feature requests
- GitHub Discussions: Questions and ideas
- Discord: Real-time chat with developers
- Documentation: Comprehensive guides and references