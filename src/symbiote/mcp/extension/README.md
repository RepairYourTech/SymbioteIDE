# MCP VS Code Extension Integration

This directory contains the VS Code extension integration for the Model Context Protocol (MCP) in SymbioteIDE.

## Overview

The MCP Extension provides a complete integration of MCP servers with VS Code, including:

- Server lifecycle management
- Security and permissions
- Language features (completion, hover, definitions, symbols)
- File system provider for MCP resources
- Task automation
- Rich UI with tree views and webview panels
- Diagnostic reporting
- Configuration management

## Architecture

### Core Components

1. **MCPExtension** (`mcp-extension.ts`)
   - Main extension class that coordinates all components
   - Handles activation/deactivation lifecycle
   - Manages event handlers and registrations

2. **Configuration Manager** (`configuration-manager.ts`)
   - Manages VS Code settings integration
   - Handles workspace and global configurations
   - Supports .mcp.json file format

3. **Command Registry** (`command-registry.ts`)
   - Implements all MCP-related commands
   - Provides user interactions and validations
   - Handles command execution with progress reporting

4. **Status Bar Manager** (`status-bar-manager.ts`)
   - Shows MCP status in VS Code status bar
   - Real-time server status updates
   - Quick access to common actions

5. **Tree Data Provider** (`tree-data-provider.ts`)
   - Hierarchical view of MCP servers
   - Shows tools, resources, and prompts
   - Supports refresh and real-time updates

6. **Panel Provider** (`panel-provider.ts`)
   - Rich webview UI for server management
   - Real-time server monitoring
   - Interactive server configuration

7. **Diagnostic Manager** (`diagnostic-manager.ts`)
   - Reports MCP-related errors and warnings
   - Integrates with VS Code Problems panel
   - Supports quick fixes

### Language Features

1. **Completion Provider** (`completion-provider.ts`)
   - Code completions from MCP servers
   - Context-aware suggestions
   - Multi-server aggregation

2. **Hover Provider** (`hover-provider.ts`)
   - Hover information from MCP servers
   - Rich markdown documentation
   - Multi-server results

3. **Definition Provider** (`definition-provider.ts`)
   - Go-to-definition functionality
   - Cross-server navigation
   - Location links support

4. **Symbol Providers** (`symbol-provider.ts`)
   - Document symbols (outline view)
   - Workspace symbols (search)
   - Hierarchical symbol support

### Additional Providers

1. **FileSystem Provider** (`filesystem-provider.ts`)
   - Access MCP resources as virtual files
   - URI scheme: `mcp://server-id/resource-uri`
   - Read-only access (write support planned)

2. **Task Provider** (`task-provider.ts`)
   - Automated tasks for MCP operations
   - Start/stop/restart servers
   - Custom build and test tasks

3. **Debug Adapter** (`debug-adapter.ts`)
   - Debug MCP server implementations (stub)
   - Planned for future implementation

4. **Language Client** (`language-client.ts`)
   - LSP integration for MCP servers (stub)
   - Planned for future implementation

## Commands

### Server Management
- `symbiote.mcp.startServer` - Start an MCP server
- `symbiote.mcp.stopServer` - Stop an MCP server
- `symbiote.mcp.restartServer` - Restart an MCP server
- `symbiote.mcp.addServer` - Add a new server configuration
- `symbiote.mcp.removeServer` - Remove a server
- `symbiote.mcp.editServer` - Edit server configuration

### Tools & Resources
- `symbiote.mcp.executeTool` - Execute an MCP tool
- `symbiote.mcp.listTools` - List available tools
- `symbiote.mcp.listResources` - List available resources
- `symbiote.mcp.readResource` - Read a resource

### UI Commands
- `symbiote.mcp.showPanel` - Show the MCP panel
- `symbiote.mcp.showServerInfo` - Show server information
- `symbiote.mcp.showLogs` - Show server logs
- `symbiote.mcp.quickAction` - Show quick actions

### Configuration
- `symbiote.mcp.openSettings` - Open MCP settings
- `symbiote.mcp.reloadConfiguration` - Reload configuration

### Development
- `symbiote.mcp.createServer` - Create new MCP server project
- `symbiote.mcp.testServer` - Test server implementation
- `symbiote.mcp.runTask` - Run MCP task
- `symbiote.mcp.createTask` - Create custom task

## Configuration

### VS Code Settings

```json
{
  "symbiote.mcp.autoStart": ["server-id-1", "server-id-2"],
  "symbiote.mcp.servers": {
    "my-server": {
      "name": "My MCP Server",
      "command": "node",
      "args": ["./my-server.js"],
      "env": {
        "API_KEY": "..."
      }
    }
  },
  "symbiote.mcp.security": {
    "defaultTrustLevel": "verified",
    "requirePermissions": true
  }
}
```

### Workspace Configuration (.mcp.json)

```json
{
  "mcpServers": {
    "project-server": {
      "name": "Project Server",
      "command": "npm",
      "args": ["run", "mcp-server"],
      "workingDirectory": "./server"
    }
  }
}
```

## Usage

### Basic Usage

1. **Add a Server**
   - Use Command Palette: `MCP: Add Server`
   - Or click the + button in the MCP Servers view

2. **Start a Server**
   - Click the play button next to the server
   - Or use Command Palette: `MCP: Start Server`

3. **Execute Tools**
   - Expand server in tree view
   - Click on a tool to execute
   - Or use Command Palette: `MCP: Execute Tool`

### Advanced Usage

1. **Multi-Server Setup**
   - Configure multiple servers for different capabilities
   - Servers are queried in parallel for language features
   - Results are aggregated and deduplicated

2. **Security Configuration**
   - Set trust levels for servers
   - Configure permissions for tools and resources
   - Enable sandboxing for untrusted servers

3. **Task Automation**
   - Create tasks for common MCP operations
   - Use in build pipelines
   - Integrate with VS Code task system

## Development

### Adding New Features

1. **New Provider**
   ```typescript
   export class MCPNewProvider implements vscode.SomeProvider {
     constructor(private serverManager: MCPServerManager) {}
     
     async provideSomething(): Promise<Something> {
       // Query MCP servers
       const servers = this.serverManager.getServers();
       // Aggregate results
       return aggregatedResults;
     }
   }
   ```

2. **New Command**
   ```typescript
   // In command-registry.ts
   async newCommand(param?: string): Promise<void> {
     // Implement command logic
     await vscode.window.withProgress({
       location: vscode.ProgressLocation.Notification,
       title: 'Executing command...'
     }, async (progress) => {
       // Command implementation
     });
   }
   ```

3. **Register in Extension**
   ```typescript
   // In mcp-extension.ts registerCommands()
   this.context.subscriptions.push(
     vscode.commands.registerCommand('symbiote.mcp.newCommand', async () => {
       await this.commandRegistry.newCommand();
     })
   );
   ```

### Testing

1. **Unit Tests**
   - Test individual providers
   - Mock VS Code API
   - Test command logic

2. **Integration Tests**
   - Test with real MCP servers
   - Test multi-server scenarios
   - Test error handling

3. **Manual Testing**
   - Use Extension Development Host
   - Test all commands
   - Verify UI updates

## Security

The extension integrates with the MCP Security system:

1. **Permission Checks**
   - All tool executions require permissions
   - Resource access is controlled
   - UI shows permission status

2. **Sandboxing**
   - Untrusted servers run in sandboxes
   - Resource limits enforced
   - Network access controlled

3. **Trust Management**
   - Servers have trust levels
   - Trust affects available features
   - Users can adjust trust levels

## Future Enhancements

1. **Language Server Protocol**
   - Full LSP integration
   - Semantic tokens
   - Code actions

2. **Debugging Support**
   - Debug MCP servers
   - Breakpoints in server code
   - Variable inspection

3. **Enhanced UI**
   - Graphical tool builders
   - Resource editors
   - Server monitoring dashboard

4. **Performance**
   - Connection pooling
   - Result caching
   - Lazy loading

## Troubleshooting

### Common Issues

1. **Server Won't Start**
   - Check command and args
   - Verify working directory
   - Check environment variables
   - View logs for errors

2. **No Completions/Hover**
   - Ensure server is running
   - Check server capabilities
   - Verify file associations
   - Check language selector

3. **Permission Denied**
   - Check security settings
   - Verify trust level
   - Grant required permissions
   - Check audit logs

### Debug Mode

Enable debug logging:
```json
{
  "symbiote.mcp.debug": true,
  "symbiote.mcp.logLevel": "debug"
}
```

View logs:
- Output panel: "MCP Extension"
- Server logs: "MCP: [Server Name]"
- Security logs: "MCP Security"