/**
 * MCP Task Provider
 * 
 * Provides VS Code tasks for MCP operations
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { MCPServerManager } from '../server-manager';
import { MCPServerConfig } from '../types';

interface MCPTaskDefinition extends vscode.TaskDefinition {
  type: 'mcp';
  operation: 'start' | 'stop' | 'restart' | 'test' | 'build' | 'custom';
  serverId?: string;
  command?: string;
  args?: string[];
}

export class MCPTaskProvider implements vscode.TaskProvider {
  private static taskType = 'mcp';
  private serverManager: MCPServerManager;
  private tasks: vscode.Task[] = [];
  
  constructor(serverManager: MCPServerManager) {
    this.serverManager = serverManager;
  }
  
  /**
   * Provide tasks
   */
  async provideTasks(token?: vscode.CancellationToken): Promise<vscode.Task[]> {
    return this.getTasks();
  }
  
  /**
   * Resolve task
   */
  async resolveTask(
    task: vscode.Task,
    token?: vscode.CancellationToken
  ): Promise<vscode.Task | undefined> {
    const definition = task.definition as MCPTaskDefinition;
    if (definition.type !== MCPTaskProvider.taskType) {
      return undefined;
    }
    
    return this.createTask(definition);
  }
  
  /**
   * Get all available tasks
   */
  private async getTasks(): Promise<vscode.Task[]> {
    const tasks: vscode.Task[] = [];
    const servers = this.serverManager.getServers();
    
    // Create tasks for each server
    for (const [serverId, server] of servers) {
      // Start server task
      tasks.push(this.createTask({
        type: MCPTaskProvider.taskType,
        operation: 'start',
        serverId
      }));
      
      // Stop server task
      tasks.push(this.createTask({
        type: MCPTaskProvider.taskType,
        operation: 'stop',
        serverId
      }));
      
      // Restart server task
      tasks.push(this.createTask({
        type: MCPTaskProvider.taskType,
        operation: 'restart',
        serverId
      }));
      
      // Test server task (if applicable)
      if (server.testCommand) {
        tasks.push(this.createTask({
          type: MCPTaskProvider.taskType,
          operation: 'test',
          serverId,
          command: server.testCommand
        }));
      }
      
      // Build server task (if applicable)
      if (server.buildCommand) {
        tasks.push(this.createTask({
          type: MCPTaskProvider.taskType,
          operation: 'build',
          serverId,
          command: server.buildCommand
        }));
      }
    }
    
    // Add general MCP tasks
    tasks.push(this.createTask({
      type: MCPTaskProvider.taskType,
      operation: 'start',
      serverId: '*' // Start all servers
    }));
    
    tasks.push(this.createTask({
      type: MCPTaskProvider.taskType,
      operation: 'stop',
      serverId: '*' // Stop all servers
    }));
    
    this.tasks = tasks;
    return tasks;
  }
  
  /**
   * Create a task from definition
   */
  private createTask(definition: MCPTaskDefinition): vscode.Task {
    const { operation, serverId, command, args } = definition;
    
    // Determine task name and command
    let taskName: string;
    let shellCommand: string;
    
    switch (operation) {
      case 'start':
        taskName = serverId === '*' 
          ? 'MCP: Start All Servers'
          : `MCP: Start ${this.getServerName(serverId)}`;
        shellCommand = this.getStartCommand(serverId);
        break;
        
      case 'stop':
        taskName = serverId === '*'
          ? 'MCP: Stop All Servers'
          : `MCP: Stop ${this.getServerName(serverId)}`;
        shellCommand = this.getStopCommand(serverId);
        break;
        
      case 'restart':
        taskName = `MCP: Restart ${this.getServerName(serverId)}`;
        shellCommand = this.getRestartCommand(serverId);
        break;
        
      case 'test':
        taskName = `MCP: Test ${this.getServerName(serverId)}`;
        shellCommand = command || 'npm test';
        break;
        
      case 'build':
        taskName = `MCP: Build ${this.getServerName(serverId)}`;
        shellCommand = command || 'npm run build';
        break;
        
      case 'custom':
        taskName = `MCP: Custom - ${command}`;
        shellCommand = command || 'echo "No command specified"';
        break;
        
      default:
        taskName = 'MCP: Unknown Operation';
        shellCommand = 'echo "Unknown operation"';
    }
    
    // Create shell execution
    const execution = new vscode.ShellExecution(shellCommand, {
      cwd: this.getWorkingDirectory(serverId)
    });
    
    // Create task
    const task = new vscode.Task(
      definition,
      vscode.TaskScope.Workspace,
      taskName,
      'MCP',
      execution,
      ['$mcp-server']
    );
    
    // Set task group
    if (operation === 'build') {
      task.group = vscode.TaskGroup.Build;
    } else if (operation === 'test') {
      task.group = vscode.TaskGroup.Test;
    }
    
    // Set presentation options
    task.presentationOptions = {
      reveal: vscode.TaskRevealKind.Always,
      echo: true,
      focus: false,
      panel: vscode.TaskPanelKind.Shared,
      showReuseMessage: false,
      clear: false
    };
    
    return task;
  }
  
  /**
   * Get server name
   */
  private getServerName(serverId?: string): string {
    if (!serverId || serverId === '*') return 'All Servers';
    
    const server = this.serverManager.getServer(serverId);
    return server?.name || serverId;
  }
  
  /**
   * Get working directory for server
   */
  private getWorkingDirectory(serverId?: string): string | undefined {
    if (!serverId || serverId === '*') {
      return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    }
    
    const server = this.serverManager.getServer(serverId);
    if (server?.workingDirectory) {
      return path.resolve(
        vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '',
        server.workingDirectory
      );
    }
    
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  }
  
  /**
   * Get start command for server
   */
  private getStartCommand(serverId?: string): string {
    if (serverId === '*') {
      // Start all servers
      return 'echo "Starting all MCP servers..." && npm run mcp:start:all';
    }
    
    const server = this.serverManager.getServer(serverId);
    if (!server) return `echo "Server ${serverId} not found"`;
    
    if (server.command) {
      const args = server.args?.join(' ') || '';
      return `${server.command} ${args}`.trim();
    }
    
    return `npm run mcp:start:${serverId}`;
  }
  
  /**
   * Get stop command for server
   */
  private getStopCommand(serverId?: string): string {
    if (serverId === '*') {
      // Stop all servers
      return 'echo "Stopping all MCP servers..." && npm run mcp:stop:all';
    }
    
    return `npm run mcp:stop:${serverId}`;
  }
  
  /**
   * Get restart command for server
   */
  private getRestartCommand(serverId?: string): string {
    const stopCmd = this.getStopCommand(serverId);
    const startCmd = this.getStartCommand(serverId);
    return `${stopCmd} && sleep 2 && ${startCmd}`;
  }
  
  /**
   * Register custom task commands
   */
  async registerCommands(context: vscode.ExtensionContext): Promise<void> {
    // Command to run custom MCP task
    context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.runTask', async () => {
        const tasks = await this.getTasks();
        
        const items = tasks.map(task => ({
          label: task.name,
          description: task.detail,
          task
        }));
        
        const selected = await vscode.window.showQuickPick(items, {
          placeHolder: 'Select MCP task to run'
        });
        
        if (selected) {
          await vscode.tasks.executeTask(selected.task);
        }
      })
    );
    
    // Command to create custom MCP task
    context.subscriptions.push(
      vscode.commands.registerCommand('symbiote.mcp.createTask', async () => {
        const command = await vscode.window.showInputBox({
          prompt: 'Enter command to run',
          placeHolder: 'npm run custom-script'
        });
        
        if (!command) return;
        
        const name = await vscode.window.showInputBox({
          prompt: 'Enter task name',
          placeHolder: 'My Custom Task'
        });
        
        if (!name) return;
        
        const task = this.createTask({
          type: MCPTaskProvider.taskType,
          operation: 'custom',
          command
        });
        
        task.name = `MCP: ${name}`;
        await vscode.tasks.executeTask(task);
      })
    );
  }
}

/**
 * MCP Problem Matcher
 * 
 * Detects problems in MCP server output
 */
export const mcpProblemMatcher: vscode.ProblemMatcher = {
  owner: 'mcp',
  pattern: {
    regexp: '^\\[(.+?)\\]\\s+(ERROR|WARN|WARNING):\\s+(.+)$',
    file: 1,
    severity: 2,
    message: 3
  },
  fileLocation: ['relative', '${workspaceFolder}']
};