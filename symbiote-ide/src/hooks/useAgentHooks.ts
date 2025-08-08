// Agent Hooks System - Standardized API for agents to interact with IDE
// Provides a unified interface for agents to trigger tools, actions, and IDE features

import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Hook types and interfaces
export interface AgentHookContext {
  agentId: string;
  agentType: string;
  permissions: AgentPermission[];
  context: Record<string, any>;
}

export interface AgentPermission {
  action: string;
  scope: 'global' | 'project' | 'file' | 'selection';
  level: 'read' | 'write' | 'execute' | 'admin';
}

export interface HookResult<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  metadata?: Record<string, any>;
}

export interface FileOperation {
  type: 'read' | 'write' | 'create' | 'delete' | 'move' | 'copy';
  path: string;
  content?: string;
  destination?: string;
}

export interface TerminalOperation {
  command: string;
  workingDirectory?: string;
  environment?: Record<string, string>;
  timeout?: number;
}

export interface UIOperation {
  type: 'notification' | 'modal' | 'panel' | 'highlight' | 'focus';
  target?: string;
  data: Record<string, any>;
}

// Main Agent Hooks System
export const useAgentHooks = (context: AgentHookContext) => {
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [lastResult, setLastResult] = useState<HookResult | null>(null);
  const executionQueue = useRef<Array<() => Promise<any>>>([]);

  // Permission checking
  const hasPermission = useCallback((action: string, scope: string, level: string): boolean => {
    return context.permissions.some(perm => 
      perm.action === action && 
      perm.scope === scope && 
      perm.level === level
    );
  }, [context.permissions]);

  // File System Hooks
  const useFileSystem = () => ({
    readFile: useCallback(async (path: string): Promise<HookResult<string>> => {
      if (!hasPermission('file.read', 'file', 'read')) {
        return { success: false, error: 'Permission denied: file.read' };
      }

      try {
        const content = await invoke<string>('agent_read_file', { 
          agentId: context.agentId, 
          path 
        });
        return { success: true, data: content };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    writeFile: useCallback(async (path: string, content: string): Promise<HookResult> => {
      if (!hasPermission('file.write', 'file', 'write')) {
        return { success: false, error: 'Permission denied: file.write' };
      }

      try {
        await invoke('agent_write_file', { 
          agentId: context.agentId, 
          path, 
          content 
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    createFile: useCallback(async (path: string, content: string = ''): Promise<HookResult> => {
      if (!hasPermission('file.create', 'file', 'write')) {
        return { success: false, error: 'Permission denied: file.create' };
      }

      try {
        await invoke('agent_create_file', { 
          agentId: context.agentId, 
          path, 
          content 
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    deleteFile: useCallback(async (path: string): Promise<HookResult> => {
      if (!hasPermission('file.delete', 'file', 'write')) {
        return { success: false, error: 'Permission denied: file.delete' };
      }

      try {
        await invoke('agent_delete_file', { 
          agentId: context.agentId, 
          path 
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    listDirectory: useCallback(async (path: string): Promise<HookResult<string[]>> => {
      if (!hasPermission('file.list', 'project', 'read')) {
        return { success: false, error: 'Permission denied: file.list' };
      }

      try {
        const files = await invoke<string[]>('agent_list_directory', { 
          agentId: context.agentId, 
          path 
        });
        return { success: true, data: files };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  });

  // Terminal Hooks
  const useTerminal = () => ({
    executeCommand: useCallback(async (operation: TerminalOperation): Promise<HookResult<string>> => {
      if (!hasPermission('terminal.execute', 'global', 'execute')) {
        return { success: false, error: 'Permission denied: terminal.execute' };
      }

      try {
        const result = await invoke<string>('agent_execute_command', {
          agentId: context.agentId,
          command: operation.command,
          workingDirectory: operation.workingDirectory,
          environment: operation.environment,
          timeout: operation.timeout || 30000,
        });
        return { success: true, data: result };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    createTerminalSession: useCallback(async (sessionName: string): Promise<HookResult<string>> => {
      if (!hasPermission('terminal.create', 'global', 'write')) {
        return { success: false, error: 'Permission denied: terminal.create' };
      }

      try {
        const sessionId = await invoke<string>('agent_create_terminal_session', {
          agentId: context.agentId,
          sessionName,
        });
        return { success: true, data: sessionId };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  });

  // UI Interaction Hooks
  const useUI = () => ({
    showNotification: useCallback(async (message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info'): Promise<HookResult> => {
      if (!hasPermission('ui.notify', 'global', 'write')) {
        return { success: false, error: 'Permission denied: ui.notify' };
      }

      try {
        await invoke('agent_show_notification', {
          agentId: context.agentId,
          message,
          type,
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    openPanel: useCallback(async (panelName: string, data?: Record<string, any>): Promise<HookResult> => {
      if (!hasPermission('ui.panel', 'global', 'write')) {
        return { success: false, error: 'Permission denied: ui.panel' };
      }

      try {
        await invoke('agent_open_panel', {
          agentId: context.agentId,
          panelName,
          data,
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    highlightCode: useCallback(async (filePath: string, startLine: number, endLine: number): Promise<HookResult> => {
      if (!hasPermission('ui.highlight', 'file', 'read')) {
        return { success: false, error: 'Permission denied: ui.highlight' };
      }

      try {
        await invoke('agent_highlight_code', {
          agentId: context.agentId,
          filePath,
          startLine,
          endLine,
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  });

  // Git Hooks
  const useGit = () => ({
    getStatus: useCallback(async (): Promise<HookResult<any>> => {
      if (!hasPermission('git.status', 'project', 'read')) {
        return { success: false, error: 'Permission denied: git.status' };
      }

      try {
        const status = await invoke('agent_git_status', {
          agentId: context.agentId,
        });
        return { success: true, data: status };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    commit: useCallback(async (message: string, files?: string[]): Promise<HookResult> => {
      if (!hasPermission('git.commit', 'project', 'write')) {
        return { success: false, error: 'Permission denied: git.commit' };
      }

      try {
        await invoke('agent_git_commit', {
          agentId: context.agentId,
          message,
          files,
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    createBranch: useCallback(async (branchName: string): Promise<HookResult> => {
      if (!hasPermission('git.branch', 'project', 'write')) {
        return { success: false, error: 'Permission denied: git.branch' };
      }

      try {
        await invoke('agent_git_create_branch', {
          agentId: context.agentId,
          branchName,
        });
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  });

  return {
    // Core hooks
    isExecuting,
    lastResult,
    hasPermission,
    
    // Feature hooks
    fileSystem: useFileSystem(),
    terminal: useTerminal(),
    ui: useUI(),
    git: useGit(),
    
    // Utility functions
    executeWithQueue: useCallback(async (operation: () => Promise<any>) => {
      executionQueue.current.push(operation);
      if (!isExecuting) {
        setIsExecuting(true);
        while (executionQueue.current.length > 0) {
          const op = executionQueue.current.shift();
          if (op) {
            try {
              const result = await op();
              setLastResult(result);
            } catch (error) {
              setLastResult({ success: false, error: String(error) });
            }
          }
        }
        setIsExecuting(false);
      }
    }, [isExecuting]),
  };
};

export default useAgentHooks;

// Example usage for different agent types
export const AgentHookExamples = {
  // Developer Agent Example
  developerAgent: `
    // Developer agent using hooks to implement a feature
    const developerAgent = async (context: AgentHookContext) => {
      const hooks = useDeveloperHooks(context);

      // Read existing code
      const fileResult = await hooks.fileSystem.readFile('./src/components/Button.tsx');
      if (!fileResult.success) {
        await hooks.ui.showNotification('Failed to read file', 'error');
        return;
      }

      // Generate improved code
      const codeResult = await hooks.generateCode(
        'Add TypeScript props interface and improve accessibility',
        'typescript'
      );

      if (codeResult.success) {
        // Write improved code
        await hooks.fileSystem.writeFile('./src/components/Button.tsx', codeResult.data);

        // Generate tests
        await hooks.generateTests('./src/components/Button.tsx', 'unit');

        // Show success notification
        await hooks.ui.showNotification('Component improved successfully!', 'success');
      }
    };
  `,

  // Tester Agent Example
  testerAgent: `
    // Tester agent using hooks to run comprehensive testing
    const testerAgent = async (context: AgentHookContext) => {
      const hooks = useTesterHooks(context);

      // Generate tests for all components
      const files = await hooks.fileSystem.listDirectory('./src/components');

      for (const file of files.data || []) {
        if (file.endsWith('.tsx')) {
          // Generate unit tests
          await hooks.generateTests(file, 'unit');

          // Generate integration tests
          await hooks.generateTests(file, 'integration');
        }
      }

      // Run all tests
      const testResults = await hooks.runTests('./tests');

      // Analyze coverage
      const coverage = await hooks.analyzeCoverage('./');

      // Security scan
      const security = await hooks.securityScan('./');

      // Report results
      await hooks.ui.showNotification(
        \`Tests: \${testResults.data.passed}/\${testResults.data.total} passed, Coverage: \${coverage.data.percentage}%\`,
        testResults.data.passed === testResults.data.total ? 'success' : 'warning'
      );
    };
  `,

  // DevOps Agent Example
  devopsAgent: `
    // DevOps agent using hooks for deployment pipeline
    const devopsAgent = async (context: AgentHookContext) => {
      const hooks = useDevOpsHooks(context);

      // Check Git status
      const gitStatus = await hooks.git.getStatus();

      if (gitStatus.data.modified.length > 0) {
        await hooks.ui.showNotification('Uncommitted changes detected', 'warning');
        return;
      }

      // Run tests before deployment
      const testResults = await hooks.terminal.executeCommand('npm test');

      if (!testResults.success) {
        await hooks.ui.showNotification('Tests failed, deployment aborted', 'error');
        return;
      }

      // Build application
      await hooks.terminal.executeCommand('npm run build');

      // Deploy to staging
      const deployment = await hooks.deployApplication('staging', {
        branch: 'main',
        environment: 'staging',
        healthCheck: true,
      });

      if (deployment.success) {
        // Monitor health after deployment
        await hooks.monitorHealth(['api', 'frontend', 'database']);

        await hooks.ui.showNotification('Deployment successful!', 'success');
      }
    };
  `,
};
