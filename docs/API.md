# SymbioteIDE API Reference

This document provides a comprehensive reference for the SymbioteIDE API.

## Table of Contents

- [Agent API](#agent-api)
- [Orchestration API](#orchestration-api)
- [MCP API](#mcp-api)
- [Memory API](#memory-api)
- [Knowledge API](#knowledge-api)
- [Terminal API](#terminal-api)

## Agent API

### Overview

The Agent API allows you to create and manage AI agents for various development tasks.

### IAgentAPI

```typescript
interface IAgentAPI {
  createAgent(config: AgentConfig): Promise<Agent>;
  startAgent(agentId: string): Promise<void>;
  stopAgent(agentId: string): Promise<void>;
  destroyAgent(agentId: string): Promise<void>;
  sendMessage(agentId: string, message: AgentMessage): Promise<void>;
  listAgents(): Promise<Agent[]>;
  getAgent(agentId: string): Promise<Agent | undefined>;
  getAgentStatus(agentId: string): Promise<AgentStatus>;
  createWorkflow(config: WorkflowConfig): Promise<Workflow>;
  executeWorkflow(workflowId: string, input: any): Promise<WorkflowResult>;
}
```

### Creating an Agent

```typescript
const agent = await agentAPI.createAgent({
  name: 'Code Reviewer',
  type: AgentType.Review,
  model: 'claude-3-sonnet',
  tools: ['eslint', 'prettier', 'security-scan'],
  permissions: {
    fileSystem: {
      read: ['src/**/*'],
      write: [],
      delete: [],
      execute: []
    },
    network: {
      allowedHosts: ['api.github.com'],
      allowedPorts: [443],
      allowedProtocols: ['https']
    }
  }
});
```

### Agent Types

- `AgentType.Development` - Code generation and implementation
- `AgentType.Testing` - Test creation and execution
- `AgentType.Documentation` - Documentation generation
- `AgentType.Review` - Code review and analysis
- `AgentType.Research` - Information gathering
- `AgentType.Custom` - Custom agent types

### Agent Messages

```typescript
// Send a command to an agent
await agentAPI.sendMessage(agent.id, {
  type: MessageType.Command,
  content: {
    action: 'review',
    files: ['src/index.ts'],
    rules: ['no-console', 'security']
  },
  priority: MessagePriority.High
});

// Listen for responses
agentAPI.onMessage.subscribe((event: AgentMessageEvent) => {
  if (event.message.type === MessageType.Result) {
    console.log('Review complete:', event.message.content);
  }
});
```

## Orchestration API

### Overview

The Orchestration API manages AI model selection, routing, and execution.

### IOrchestrationAPI

```typescript
interface IOrchestrationAPI {
  selectModel(task: AITask): Promise<ModelSelection>;
  routeTask(task: AITask): Promise<TaskRoute>;
  executeTask(task: AITask, options?: ExecutionOptions): Promise<TaskResult>;
  executeParallel(tasks: AITask[]): Promise<TaskResult[]>;
  executePipeline(pipeline: TaskPipeline): Promise<PipelineResult>;
  listModels(): Promise<AIModel[]>;
  getMetrics(): Promise<OrchestrationMetrics>;
  getCostEstimate(task: AITask): Promise<CostEstimate>;
}
```

### Task Execution

```typescript
// Simple task execution
const result = await orchestrationAPI.executeTask({
  id: 'task-1',
  type: TaskType.CodeGeneration,
  prompt: 'Create a React hook for local storage',
  context: {
    language: 'typescript',
    framework: 'react'
  },
  constraints: {
    maxTokens: 1000,
    maxCost: 0.10,
    temperature: 0.7
  }
});

// Parallel execution
const results = await orchestrationAPI.executeParallel([
  { type: TaskType.CodeReview, prompt: 'Review auth.ts' },
  { type: TaskType.Documentation, prompt: 'Document auth.ts' },
  { type: TaskType.Testing, prompt: 'Generate tests for auth.ts' }
]);
```

### Model Selection

```typescript
const selection = await orchestrationAPI.selectModel({
  type: TaskType.CodeGeneration,
  constraints: {
    maxCost: 0.05,
    requiredCapabilities: ['code-generation', 'typescript']
  }
});

console.log(`Selected: ${selection.primary.name}`);
console.log(`Estimated cost: $${selection.estimatedCost}`);
```

## MCP API

### Overview

The MCP API provides Model Context Protocol client functionality.

### IMcpAPI

```typescript
interface IMcpAPI {
  connectServer(config: McpServerConfig): Promise<McpConnection>;
  disconnectServer(serverId: string): Promise<void>;
  listServers(): Promise<McpServer[]>;
  listTools(serverId?: string): Promise<McpTool[]>;
  executeTool(toolId: string, params: any): Promise<McpToolResult>;
  listResources(serverId?: string): Promise<McpResource[]>;
  readResource(resourceUri: string): Promise<McpResourceContent>;
}
```

### Connecting to MCP Servers

```typescript
// Connect to a server
const connection = await mcpAPI.connectServer({
  id: 'my-tools',
  name: 'My Tool Server',
  type: 'stdio',
  command: 'my-mcp-server',
  args: ['--port', '7777'],
  permissions: {
    tools: {
      allowed: '*',
      rateLimit: {
        requests: 100,
        window: 60
      }
    }
  }
});

// List available tools
const tools = await mcpAPI.listTools('my-tools');
tools.forEach(tool => {
  console.log(`${tool.name}: ${tool.description}`);
});
```

### Executing Tools

```typescript
const result = await mcpAPI.executeTool('my-tools/generate-code', {
  language: 'python',
  description: 'FastAPI endpoint for user creation'
});

if (result.success) {
  console.log('Generated code:', result.output);
}
```

## Memory API

### Overview

The Memory API provides persistent memory capabilities using Mem0.

### IMemoryAPI

```typescript
interface IMemoryAPI {
  createMemory(memory: MemoryInput): Promise<Memory>;
  getMemory(id: string): Promise<Memory | null>;
  searchMemories(query: MemorySearchQuery): Promise<MemorySearchResult>;
  updateMemory(id: string, updates: Partial<MemoryInput>): Promise<Memory>;
  deleteMemory(id: string): Promise<void>;
  getAnalytics(): Promise<MemoryAnalytics>;
}
```

### Storing Memories

```typescript
// Store a code pattern
const memory = await memoryAPI.createMemory({
  type: MemoryType.CodePattern,
  content: {
    pattern: 'React hook for API calls',
    code: 'const useAPI = (url) => { ... }',
    language: 'typescript'
  },
  metadata: {
    project: 'my-app',
    tags: ['react', 'hooks', 'api']
  },
  importance: 0.8
});

// Store a preference
await memoryAPI.createMemory({
  type: MemoryType.UserPreference,
  content: {
    preference: 'indent_size',
    value: 2
  }
});
```

### Searching Memories

```typescript
const results = await memoryAPI.searchMemories({
  query: 'react hooks',
  filters: {
    type: MemoryType.CodePattern,
    tags: ['react']
  },
  limit: 10
});

results.memories.forEach(memory => {
  console.log(`${memory.content.pattern} (relevance: ${memory.score})`);
});
```

## Knowledge API

### Overview

The Knowledge API integrates Neo4j and Qdrant for code intelligence.

### IKnowledgeAPI

```typescript
interface IKnowledgeAPI {
  // Graph operations
  createNode(node: GraphNode): Promise<string>;
  createRelationship(rel: GraphRelationship): Promise<string>;
  queryGraph(query: GraphQuery): Promise<GraphResult>;
  
  // Vector operations
  indexCode(code: CodeToIndex): Promise<IndexResult>;
  searchSimilar(query: SimilarityQuery): Promise<SimilarityResult>;
  
  // Unified operations
  getEntity(id: string): Promise<KnowledgeEntity>;
  findRelated(id: string, options?: RelationOptions): Promise<KnowledgeEntity[]>;
}
```

### Graph Operations

```typescript
// Create a function node
const nodeId = await knowledgeAPI.createNode({
  type: NodeType.Function,
  properties: {
    name: 'calculateTotal',
    file: 'src/utils.ts',
    line: 42,
    complexity: 5
  }
});

// Create relationship
await knowledgeAPI.createRelationship({
  type: RelationType.Calls,
  sourceId: nodeId,
  targetId: 'other-function-id',
  properties: {
    count: 3
  }
});
```

### Semantic Search

```typescript
// Find similar code
const similar = await knowledgeAPI.searchSimilar({
  code: 'function authenticate(user, password) { ... }',
  limit: 5,
  threshold: 0.8
});

similar.results.forEach(result => {
  console.log(`${result.file}:${result.line} - ${result.score}`);
});
```

## Terminal API

### Overview

The Terminal API provides AI-enhanced terminal functionality.

### ITerminalAPI

```typescript
interface ITerminalAPI {
  createSession(config: TerminalConfig): Promise<TerminalSession>;
  executeCommand(sessionId: string, command: string): Promise<CommandResult>;
  translateNaturalLanguage(input: string): Promise<CommandTranslation>;
  getSuggestions(context: SuggestionContext): Promise<CommandSuggestion[]>;
  analyzeError(error: TerminalError): Promise<ErrorAnalysis>;
}
```

### Natural Language Commands

```typescript
// Translate natural language to commands
const translation = await terminalAPI.translateNaturalLanguage(
  "Show me all TypeScript files modified in the last week"
);

console.log(translation.command); 
// find . -name "*.ts" -mtime -7

// Execute with confirmation
if (translation.riskLevel === 'safe') {
  await terminalAPI.executeCommand(sessionId, translation.command);
}
```

### Error Analysis

```typescript
// Analyze command errors
const analysis = await terminalAPI.analyzeError({
  command: 'npm run build',
  exitCode: 1,
  stderr: 'Error: Cannot find module...'
});

console.log('Diagnosis:', analysis.diagnosis);
console.log('Fix:', analysis.suggestedFix);

// Apply fix automatically
if (analysis.autoFixAvailable) {
  await terminalAPI.executeCommand(sessionId, analysis.autoFixCommand);
}
```

### Smart Suggestions

```typescript
// Get command suggestions
const suggestions = await terminalAPI.getSuggestions({
  currentDirectory: '/src',
  recentCommands: ['git status', 'git add .'],
  currentInput: 'git c'
});

suggestions.forEach(suggestion => {
  console.log(`${suggestion.command} - ${suggestion.description}`);
});
```

## Error Handling

All API methods follow consistent error handling:

```typescript
try {
  const result = await api.method();
} catch (error) {
  if (error.code === 'RATE_LIMITED') {
    // Handle rate limiting
  } else if (error.code === 'PERMISSION_DENIED') {
    // Handle permission issues
  } else {
    // Handle other errors
  }
}
```

## Events

Most APIs provide event emitters for real-time updates:

```typescript
// Agent events
agentAPI.onMessage((event) => { ... });
agentAPI.onStatusChange((event) => { ... });

// Orchestration events
orchestrationAPI.onMetricsUpdate((event) => { ... });
orchestrationAPI.onCostAlert((event) => { ... });

// MCP events
mcpAPI.onMessage((event) => { ... });
mcpAPI.onError((event) => { ... });
```

## Best Practices

1. **Always handle errors**: Use try-catch blocks
2. **Set appropriate timeouts**: Prevent hanging operations
3. **Use constraints**: Limit costs and resources
4. **Clean up resources**: Call dispose methods when done
5. **Monitor usage**: Track metrics and costs

## Examples

See the [examples directory](../examples/) for complete working examples of:
- Agent workflows
- Multi-model orchestration
- MCP server integration
- Memory persistence
- Knowledge graph queries