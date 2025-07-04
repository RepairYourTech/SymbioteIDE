# Google AI ADK Integration

This module provides integration with the official Google AI Agent Development Kit (ADK) for SymbioteIDE.

## Overview

Google ADK is a Python-based framework for building AI agents. Since SymbioteIDE is built with TypeScript/VS Code, we use a bridge architecture:

```
VS Code Extension (TypeScript)
    ↓
ADK Client (TypeScript)
    ↓ HTTP/WebSocket
ADK Bridge Server (Python)
    ↓
Google ADK (Python)
    ↓
Google AI APIs (Gemini, etc.)
```

## Architecture

### Components

1. **ADK Client** (`adk-client.ts`) - TypeScript client that communicates with the Python bridge
2. **ADK Bridge Server** (`python-bridge/adk_server.py`) - FastAPI server that wraps Google ADK
3. **Agent Factory** (`agent-factory.ts`) - Creates and manages agents
4. **Specialized Agents** (`specialized-agents/`) - Pre-configured agent templates

### Key Features

- **Real Google ADK**: Uses the official `google-adk` Python package
- **Full ADK Features**: Access to all ADK capabilities including:
  - LLM Agents with Gemini models
  - Tool integration
  - Memory systems
  - Agent orchestration
  - A2A (Agent-to-Agent) communication
- **TypeScript Integration**: Seamless integration with VS Code extension
- **MCP Tool Support**: ADK agents can use MCP tools
- **Visual Builder**: Agents can be created visually in the Agent Builder UI

## Installation

1. Install Python dependencies:
```bash
pip install google-adk
```

2. Start the ADK bridge server:
```bash
python src/symbiote/adk/python-bridge/adk_server.py
```

3. The TypeScript client will automatically connect to the server.

## Usage

### Creating an Agent

```typescript
import { ADKClient } from './adk-client';
import { ADKAgentType } from './types';

const client = new ADKClient();

// Create a code review agent
const agent = await client.createADKAgent({
  id: 'code-reviewer',
  name: 'Code Review Agent',
  type: ADKAgentType.LLM,
  description: 'Reviews code for quality and best practices',
  systemPrompt: `You are an expert code reviewer. Analyze code for:
    - Code quality and readability
    - Performance issues
    - Security vulnerabilities
    - Best practices`,
  model: {
    id: 'gemini-1.5-pro',
    provider: 'google'
  },
  temperature: 0.3,
  capabilities: {
    streaming: true,
    toolUse: true,
    memoryAccess: true
  }
});

// Execute a task
const result = await agent.execute('Review this TypeScript file for issues', {
  file: '/path/to/file.ts',
  content: fileContent
});
```

### Using with Visual Agent Builder

Agents created in the Visual Agent Builder UI are automatically integrated with Google ADK:

1. Drag an "AI Agent" node onto the canvas
2. Select "Google ADK" as the provider
3. Configure the agent type (e.g., "code-reviewer")
4. Connect tools and set parameters
5. Export as standalone agent

### Available Models

Google ADK supports all Gemini models:

- **Gemini 1.5 Pro**: Best for complex reasoning (2M token context)
- **Gemini 1.5 Flash**: Fast and efficient (1M token context)
- **Gemini 2.0 Flash**: Experimental with real-time capabilities

### Integration with A2A Protocol

ADK agents can communicate with each other using the A2A protocol:

```typescript
// Agent 1: Analyzer
const analyzer = await client.createADKAgent({
  id: 'analyzer',
  name: 'Code Analyzer',
  capabilities: { a2aProtocol: true }
});

// Agent 2: Optimizer
const optimizer = await client.createADKAgent({
  id: 'optimizer', 
  name: 'Code Optimizer',
  capabilities: { a2aProtocol: true }
});

// Orchestrate agents
const result = await client.orchestrate(
  'Analyze and optimize this codebase',
  ['analyzer', 'optimizer']
);
```

## API Reference

See the [Google ADK documentation](https://google.github.io/adk-docs/) for detailed API reference.

## Environment Variables

- `GOOGLE_API_KEY` - Your Google AI API key
- `ADK_SERVER_URL` - ADK bridge server URL (default: http://localhost:8888)
- `PYTHON_PATH` - Path to Python executable (default: python3)