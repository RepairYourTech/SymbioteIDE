# SymbioteIDE Quick Start Guide

Welcome to SymbioteIDE! This guide will get you up and running in minutes.

## Installation

### Download

Download the latest release for your platform:

- **Windows**: [SymbioteIDE-win-x64.exe](#)
- **macOS Intel**: [SymbioteIDE-mac-x64.dmg](#)
- **macOS Apple Silicon**: [SymbioteIDE-mac-arm64.dmg](#)
- **Linux**: [SymbioteIDE-linux-x64.AppImage](#)

### First Launch

1. Run the installer for your platform
2. Launch SymbioteIDE
3. You'll be prompted to configure your AI providers

## Initial Configuration

### Step 1: Add Your API Keys

SymbioteIDE uses a BYOK (Bring Your Own Key) approach. Add at least one provider:

1. Open Settings (`Ctrl/Cmd + ,`)
2. Search for "SymbioteIDE: API Keys"
3. Add your keys:

```json
{
  "symbiote.ai.providers": [
    {
      "id": "openai",
      "apiKey": "sk-...",
      "enabled": true
    },
    {
      "id": "anthropic", 
      "apiKey": "sk-ant-...",
      "enabled": true
    }
  ]
}
```

### Step 2: Configure MCP Servers (Optional)

To use MCP servers:

1. Create `.mcp.json` in your project root
2. Add server configurations:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"]
    }
  }
}
```

### Step 3: Choose Your Workflow

SymbioteIDE supports multiple AI-powered workflows:

## Basic Usage

### AI Chat Interface

Open the AI chat panel:
- **Command**: `Ctrl/Cmd + Shift + I`
- **View** → **AI Chat**

Ask questions or request code:
```
> Generate a React component for a todo list
> Explain this function
> Find security issues in my code
```

### Natural Language Terminal

Use natural language in the terminal:
```
> "Show me all Python files modified today"
> "Run tests for the auth module"
> "Install dependencies for React development"
```

### Smart Code Completion

SymbioteIDE automatically selects the best AI model for completions based on:
- Language
- File type
- Context complexity
- Cost constraints

## Key Features

### 1. Multi-Model Orchestration

SymbioteIDE automatically routes your requests to the optimal AI model:

```typescript
// This comment triggers code generation with the best model
// @ai: Create a function to validate email addresses
```

### 2. Agent Workflows

Create automated workflows with agents:

1. Open Command Palette (`Ctrl/Cmd + Shift + P`)
2. Run "SymbioteIDE: Create Agent"
3. Choose a template (e.g., "Code Reviewer")
4. Configure and run

### 3. Persistent Memory

SymbioteIDE remembers your preferences and context:

- Code patterns you use
- Common fixes you apply
- Project-specific knowledge

### 4. Knowledge Graph

Navigate code intelligently:

- **Go to Definition**: Enhanced with AI understanding
- **Find References**: Semantic search across codebase
- **Explain Symbol**: Get AI-powered explanations

## Keyboard Shortcuts

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Open AI Chat | `Ctrl+Shift+I` | `Cmd+Shift+I` |
| Quick AI Action | `Ctrl+K` | `Cmd+K` |
| Run Agent | `Ctrl+Shift+A` | `Cmd+Shift+A` |
| AI Terminal | `Ctrl+`` ` | `Cmd+`` ` |
| Smart Complete | `Ctrl+Space` | `Cmd+Space` |

## Common Workflows

### 1. Code Review Workflow

```bash
# 1. Stage your changes
git add .

# 2. Run AI review
symbiote review --comprehensive

# 3. Apply suggested fixes
symbiote apply-fixes
```

### 2. Test Generation

Right-click on any function and select:
- "Generate Unit Tests"
- "Generate Integration Tests"
- "Generate Test Cases"

### 3. Documentation

Select code and:
- Press `Ctrl/Cmd + D` for inline docs
- Use "Generate Documentation" from context menu

### 4. Refactoring

1. Select code to refactor
2. Press `Ctrl/Cmd + Shift + R`
3. Choose refactoring type
4. Review AI suggestions

## Tips and Tricks

### 1. Cost Management

Set cost limits in settings:
```json
{
  "symbiote.ai.costLimits": {
    "perSession": 5.00,
    "perDay": 20.00,
    "warningThreshold": 0.8
  }
}
```

### 2. Model Preferences

Configure model routing:
```json
{
  "symbiote.ai.routing": {
    "code-generation": ["claude-3-opus", "gpt-4"],
    "code-review": ["claude-3-sonnet", "gpt-4"],
    "general": ["gpt-3.5-turbo"]
  }
}
```

### 3. Privacy Settings

Control data sharing:
```json
{
  "symbiote.privacy": {
    "shareAcrossProjects": false,
    "anonymizeData": true,
    "localMemoryOnly": true
  }
}
```

## Troubleshooting

### AI Features Not Working

1. Check API keys in settings
2. Verify internet connection
3. Check status bar for errors

### Extension Compatibility

Most VS Code extensions work out of the box. If you encounter issues:

1. Check our [compatibility list](./extension-compatibility.md)
2. Report issues on GitHub

### Performance Issues

1. Reduce concurrent AI requests in settings
2. Enable GPU acceleration for terminal
3. Clear cache: Command → "Clear SymbioteIDE Cache"

## Next Steps

- Read the [full documentation](./README.md)
- Explore [agent templates](../templates/)
- Join our [Discord community](#)
- Watch [video tutorials](#)

## Getting Help

- **In-app**: Type "help" in AI chat
- **Documentation**: Press `F1` → "SymbioteIDE: Help"
- **Community**: Join our Discord server
- **Issues**: GitHub issue tracker

Welcome to the future of AI-powered development! 🚀