// Hook Registry System - Central registry for all agent hooks and capabilities
// Allows dynamic discovery and registration of hooks for different agent types

import { useState, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Hook registry types
export interface HookDefinition {
  name: string;
  category: string;
  description: string;
  parameters: HookParameter[];
  returnType: string;
  permissions: string[];
  agentTypes: string[];
  examples: HookExample[];
}

export interface HookParameter {
  name: string;
  type: string;
  required: boolean;
  description: string;
  defaultValue?: any;
}

export interface HookExample {
  title: string;
  description: string;
  code: string;
  expectedResult: string;
}

export interface RegisteredHook {
  id: string;
  definition: HookDefinition;
  implementation: Function;
  metadata: {
    registeredAt: Date;
    usageCount: number;
    lastUsed?: Date;
    version: string;
  };
}

export interface HookUsageStats {
  hookId: string;
  agentId: string;
  timestamp: Date;
  success: boolean;
  executionTime: number;
  error?: string;
}

// Main Hook Registry
export const useHookRegistry = () => {
  const [registeredHooks, setRegisteredHooks] = useState<Map<string, RegisteredHook>>(new Map());
  const [usageStats, setUsageStats] = useState<HookUsageStats[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const hookCache = useRef<Map<string, any>>(new Map());

  // Initialize registry with built-in hooks
  useEffect(() => {
    initializeBuiltInHooks();
  }, []);

  const initializeBuiltInHooks = useCallback(async () => {
    setIsLoading(true);
    try {
      // Register core file system hooks
      registerHook({
        id: 'fs.readFile',
        definition: {
          name: 'readFile',
          category: 'filesystem',
          description: 'Read content from a file',
          parameters: [
            {
              name: 'path',
              type: 'string',
              required: true,
              description: 'Path to the file to read',
            },
          ],
          returnType: 'Promise<HookResult<string>>',
          permissions: ['file.read'],
          agentTypes: ['*'],
          examples: [
            {
              title: 'Read a configuration file',
              description: 'Read package.json content',
              code: 'const result = await hooks.fileSystem.readFile("./package.json");',
              expectedResult: 'File content as string',
            },
          ],
        },
        implementation: async (path: string) => {
          // Implementation handled by useAgentHooks
          return { success: true, data: 'file content' };
        },
        metadata: {
          registeredAt: new Date(),
          usageCount: 0,
          version: '1.0.0',
        },
      });

      // Register terminal hooks
      registerHook({
        id: 'terminal.execute',
        definition: {
          name: 'executeCommand',
          category: 'terminal',
          description: 'Execute a command in the terminal',
          parameters: [
            {
              name: 'command',
              type: 'string',
              required: true,
              description: 'Command to execute',
            },
            {
              name: 'workingDirectory',
              type: 'string',
              required: false,
              description: 'Working directory for command execution',
            },
          ],
          returnType: 'Promise<HookResult<string>>',
          permissions: ['terminal.execute'],
          agentTypes: ['developer', 'devops', 'tester'],
          examples: [
            {
              title: 'Run npm install',
              description: 'Install project dependencies',
              code: 'const result = await hooks.terminal.executeCommand("npm install");',
              expectedResult: 'Command output as string',
            },
          ],
        },
        implementation: async (command: string, workingDirectory?: string) => {
          return { success: true, data: 'command output' };
        },
        metadata: {
          registeredAt: new Date(),
          usageCount: 0,
          version: '1.0.0',
        },
      });

      // Register UI hooks
      registerHook({
        id: 'ui.notification',
        definition: {
          name: 'showNotification',
          category: 'ui',
          description: 'Show a notification to the user',
          parameters: [
            {
              name: 'message',
              type: 'string',
              required: true,
              description: 'Notification message',
            },
            {
              name: 'type',
              type: 'string',
              required: false,
              description: 'Notification type (info, success, warning, error)',
              defaultValue: 'info',
            },
          ],
          returnType: 'Promise<HookResult>',
          permissions: ['ui.notify'],
          agentTypes: ['*'],
          examples: [
            {
              title: 'Show success notification',
              description: 'Notify user of successful operation',
              code: 'await hooks.ui.showNotification("Task completed!", "success");',
              expectedResult: 'Notification displayed to user',
            },
          ],
        },
        implementation: async (message: string, type: string = 'info') => {
          return { success: true };
        },
        metadata: {
          registeredAt: new Date(),
          usageCount: 0,
          version: '1.0.0',
        },
      });

      // Register Git hooks
      registerHook({
        id: 'git.status',
        definition: {
          name: 'getStatus',
          category: 'git',
          description: 'Get Git repository status',
          parameters: [],
          returnType: 'Promise<HookResult<GitStatus>>',
          permissions: ['git.status'],
          agentTypes: ['developer', 'reviewer', 'devops'],
          examples: [
            {
              title: 'Check repository status',
              description: 'Get current Git status',
              code: 'const status = await hooks.git.getStatus();',
              expectedResult: 'Git status object with changes, staged files, etc.',
            },
          ],
        },
        implementation: async () => {
          return { success: true, data: { modified: [], staged: [], untracked: [] } };
        },
        metadata: {
          registeredAt: new Date(),
          usageCount: 0,
          version: '1.0.0',
        },
      });

    } catch (error) {
      console.error('Failed to initialize built-in hooks:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Register a new hook
  const registerHook = useCallback((hook: RegisteredHook) => {
    setRegisteredHooks(prev => {
      const newMap = new Map(prev);
      newMap.set(hook.id, hook);
      return newMap;
    });
  }, []);

  // Unregister a hook
  const unregisterHook = useCallback((hookId: string) => {
    setRegisteredHooks(prev => {
      const newMap = new Map(prev);
      newMap.delete(hookId);
      return newMap;
    });
    hookCache.current.delete(hookId);
  }, []);

  // Get hooks by category
  const getHooksByCategory = useCallback((category: string): RegisteredHook[] => {
    return Array.from(registeredHooks.values()).filter(
      hook => hook.definition.category === category
    );
  }, [registeredHooks]);

  // Get hooks by agent type
  const getHooksByAgentType = useCallback((agentType: string): RegisteredHook[] => {
    return Array.from(registeredHooks.values()).filter(
      hook => hook.definition.agentTypes.includes('*') || hook.definition.agentTypes.includes(agentType)
    );
  }, [registeredHooks]);

  // Search hooks
  const searchHooks = useCallback((query: string): RegisteredHook[] => {
    const lowerQuery = query.toLowerCase();
    return Array.from(registeredHooks.values()).filter(hook =>
      hook.definition.name.toLowerCase().includes(lowerQuery) ||
      hook.definition.description.toLowerCase().includes(lowerQuery) ||
      hook.definition.category.toLowerCase().includes(lowerQuery)
    );
  }, [registeredHooks]);

  // Execute hook with usage tracking
  const executeHook = useCallback(async (
    hookId: string,
    agentId: string,
    parameters: any[]
  ): Promise<any> => {
    const hook = registeredHooks.get(hookId);
    if (!hook) {
      throw new Error(`Hook ${hookId} not found`);
    }

    const startTime = Date.now();
    let success = false;
    let error: string | undefined;
    let result: any;

    try {
      result = await hook.implementation(...parameters);
      success = true;

      // Update usage count
      hook.metadata.usageCount++;
      hook.metadata.lastUsed = new Date();
      
      // Update registry
      setRegisteredHooks(prev => {
        const newMap = new Map(prev);
        newMap.set(hookId, hook);
        return newMap;
      });

    } catch (err) {
      error = String(err);
      throw err;
    } finally {
      // Record usage stats
      const executionTime = Date.now() - startTime;
      const stats: HookUsageStats = {
        hookId,
        agentId,
        timestamp: new Date(),
        success,
        executionTime,
        error,
      };

      setUsageStats(prev => [...prev, stats]);

      // Persist stats to backend
      try {
        await invoke('record_hook_usage', { stats });
      } catch (persistError) {
        console.warn('Failed to persist hook usage stats:', persistError);
      }
    }

    return result;
  }, [registeredHooks]);

  // Get usage statistics
  const getUsageStats = useCallback((hookId?: string, agentId?: string) => {
    let filtered = usageStats;
    
    if (hookId) {
      filtered = filtered.filter(stat => stat.hookId === hookId);
    }
    
    if (agentId) {
      filtered = filtered.filter(stat => stat.agentId === agentId);
    }

    return {
      totalExecutions: filtered.length,
      successfulExecutions: filtered.filter(stat => stat.success).length,
      failedExecutions: filtered.filter(stat => !stat.success).length,
      averageExecutionTime: filtered.reduce((sum, stat) => sum + stat.executionTime, 0) / filtered.length || 0,
      recentExecutions: filtered.slice(-10),
    };
  }, [usageStats]);

  // Get hook documentation
  const getHookDocumentation = useCallback((hookId: string) => {
    const hook = registeredHooks.get(hookId);
    if (!hook) return null;

    return {
      ...hook.definition,
      metadata: hook.metadata,
      usageStats: getUsageStats(hookId),
    };
  }, [registeredHooks, getUsageStats]);

  // Export hook definitions for external tools
  const exportHookDefinitions = useCallback(() => {
    return Array.from(registeredHooks.values()).map(hook => ({
      id: hook.id,
      definition: hook.definition,
      metadata: hook.metadata,
    }));
  }, [registeredHooks]);

  return {
    // Registry management
    registeredHooks: Array.from(registeredHooks.values()),
    registerHook,
    unregisterHook,
    isLoading,

    // Hook discovery
    getHooksByCategory,
    getHooksByAgentType,
    searchHooks,
    getHookDocumentation,

    // Hook execution
    executeHook,

    // Analytics
    getUsageStats,
    usageStats,

    // Utilities
    exportHookDefinitions,
  };
};

export default useHookRegistry;
