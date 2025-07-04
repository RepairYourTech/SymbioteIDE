# MCP Integration for SymbioteIDE

This module provides integration with the Model Context Protocol (MCP), allowing SymbioteIDE to connect to external MCP servers and use their tools within the AI orchestration system.

## Overview

MCP (Model Context Protocol) is a standard by Anthropic for connecting AI assistants to external tools and services. MCP servers are external programs that provide:
- **Tools**: Functions that can be called with arguments and return results
- **Resources**: Data sources that can be read and monitored
- **Prompts**: Pre-defined prompt templates with variable substitution

## Architecture

```
SymbioteIDE
├── MCP Integration
│   ├── Config Reader (.vscode/mcp.json)
│   ├── Server Manager (process lifecycle)
│   ├── MCP Client (communication)
│   └── Orchestration Adapter (AI integration)
└── AI Orchestration Engine
    └── MCP tools exposed as AI models
```

## Configuration

MCP servers are configured in `.vscode/mcp.json`:

```json
{
  "mcpServers": {
    "task-master-ai": {
      "command": "npx",
      "args": ["-y", "--package=task-master-ai", "task-master-ai"],
      "env": {
        "ANTHROPIC_API_KEY": "your-key-here"
      },
      "autoStart": true
    }
  }
}
```

## Usage

### Basic Usage

```typescript
import { McpIntegration } from './symbiote/mcp';
import { OrchestrationEngine } from './symbiote/orchestration';

// Initialize
const orchestration = new OrchestrationEngine();
const mcp = new McpIntegration(orchestration);
await mcp.initialize();

// List available tools
const tools = await mcp.listTools();

// Execute a tool directly
const result = await mcp.executeTool('task-master-ai', 'get_tasks', {});
```

### Orchestration Integration

MCP tools are automatically exposed as AI models in the orchestration engine:

```typescript
// Execute task using MCP tool through orchestration
const task = {
  id: 'task-1',
  type: TaskType.General,
  prompt: 'Get all pending tasks'
};

const result = await orchestration.executeTask(task);
```

### Server Management

```typescript
// Start/stop servers
await mcp.startServer('task-master-ai');
await mcp.stopServer('task-master-ai');

// Get server status
const servers = mcp.getAllServers();
```

## Components

### McpConfigReader
- Reads `.vscode/mcp.json` configuration files
- Watches for configuration changes
- Validates server definitions

### McpServerManager
- Manages MCP server process lifecycle
- Handles process crashes and restarts
- Monitors server health

### McpClient
- High-level API for MCP communication
- Tool/resource/prompt discovery
- Request/response handling

### McpOrchestrationAdapter
- Adapts MCP tools to orchestration engine models
- Maps task types to appropriate tools
- Handles tool execution and result formatting

## Error Handling

The integration includes comprehensive error handling:
- Process failures with automatic restart
- Connection timeouts
- Invalid server configurations
- Tool execution failures

## Security

- Servers run in isolated processes
- Environment variables are sandboxed
- No direct code execution
- JSON-RPC communication only

## Future Enhancements

- [ ] WebSocket transport support
- [ ] HTTP transport support  
- [ ] Tool result caching
- [ ] Advanced tool mapping strategies
- [ ] Performance metrics collection