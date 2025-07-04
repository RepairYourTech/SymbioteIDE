# SymbioteIDE

## A Revolutionary AI-Powered IDE Based on VS Code

SymbioteIDE is a next-generation development environment that leverages multiple AI models in specialized workspaces to create a truly intelligent coding experience. Built as a fork of Visual Studio Code, it extends the powerful VS Code foundation with advanced AI orchestration capabilities through the Model Context Protocol (MCP).

## Key Features

### Multi-Model Orchestration Engine
- Intelligent routing of tasks to specialized AI models
- Parallel processing with result synthesis
- Model performance monitoring and optimization
- Automatic fallback and error handling

### Agent Builder Framework
- Visual agent creation interface
- Pre-built agent templates for common workflows
- Multi-step workflow automation
- Agent marketplace for sharing and discovery

### AI-Powered Terminal
- Natural language command interpretation
- Intelligent command suggestions
- Context-aware auto-completion
- Error prediction and prevention

### Advanced Memory System (Mem0)
- Persistent context across sessions
- User preference learning
- Project-specific knowledge retention
- Intelligent context retrieval

### Knowledge Graph Integration
- Neo4j for relationship mapping
- Qdrant for vector similarity search
- Automatic code understanding and documentation
- Intelligent code navigation

### Advanced Debugging AI
- Multi-agent debugging system using Google ADK
- Pattern learning from past debugging sessions
- Automated fix generation and validation
- Browser integration for web debugging

### Docker Infrastructure Stack
- Local development environment setup
- Integrated services: Neo4j, Qdrant, Redis, PostgreSQL, RabbitMQ
- Optional Ollama integration for local LLMs
- Cross-platform support (Windows, WSL, macOS, Linux)

## Building SymbioteIDE

### Prerequisites
- Node.js 22.15.1 (use nvm to install)
- Git
- Python 3.x (for ADK bridge)
- Docker (for service stack)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/symbiote-ide/symbiote-ide.git
cd symbiote-ide

# Install Node.js version
nvm install
nvm use

# Install dependencies
npm install

# Compile VS Code and SymbioteIDE
npm run compile
npm run compile-symbiote

# Run in development mode
npm run watch

# Start SymbioteIDE
./scripts/code.sh
```

### Development

SymbioteIDE extends VS Code's architecture by integrating AI features directly into the core platform:

- **Orchestration Engine**: Located in `src/vs/platform/orchestration/`
- **ADK Integration**: Located in `src/vs/workbench/services/adk/`
- **Debugging AI**: Integrated into `src/vs/workbench/contrib/debug/`
- **Browser Features**: Enhanced in `src/vs/workbench/contrib/webview/`

## VS Code Foundation

SymbioteIDE is built on the robust foundation of [Visual Studio Code](https://code.visualstudio.com), inheriting all its powerful features while adding AI-powered enhancements. This ensures compatibility with existing VS Code extensions while providing next-generation AI capabilities.

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE.txt) file for details.

## Acknowledgments

- Built on [Visual Studio Code](https://github.com/microsoft/vscode)
- Integrates with leading AI models and services
- Uses the Model Context Protocol (MCP) for standardized AI integration

---

**SymbioteIDE** - Where AI and Development Converge