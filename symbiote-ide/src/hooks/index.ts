// Agent Hooks System - Main Export File
// Centralized exports for all agent hooks and utilities

// Core hooks
export {
  useAgentHooks,
  AgentHookExamples,
} from './useAgentHooks';

export type {
  AgentHookContext,
  AgentPermission,
  HookResult,
  FileOperation,
  TerminalOperation,
  UIOperation,
} from './useAgentHooks';

// Specialized hooks
export {
  useArchitectHooks,
  useDeveloperHooks,
  useTesterHooks,
  useReviewerHooks,
  useDevOpsHooks,
} from './useSpecializedAgentHooks';

// Hook registry
export {
  useHookRegistry,
  type HookDefinition,
  type HookParameter,
  type HookExample,
  type RegisteredHook,
  type HookUsageStats,
} from './useHookRegistry';

// Import types for use in utilities
import type { AgentHookContext, HookResult } from './useAgentHooks';

// Hook utilities and helpers
export const HookUtils = {
  // Create agent context with default permissions
  createAgentContext: (
    agentId: string,
    agentType: string,
    customPermissions: string[] = []
  ): AgentHookContext => {
    const defaultPermissions = getDefaultPermissions(agentType);
    const allPermissions = [...defaultPermissions, ...customPermissions];
    
    return {
      agentId,
      agentType,
      permissions: allPermissions.map(perm => {
        const [action, scope, level] = perm.split('.');
        return {
          action,
          scope: scope as 'global' | 'project' | 'file' | 'selection',
          level: level as 'read' | 'write' | 'execute' | 'admin',
        };
      }),
      context: {},
    };
  },

  // Validate hook permissions
  validatePermissions: (
    context: AgentHookContext,
    requiredPermissions: string[]
  ): boolean => {
    return requiredPermissions.every(required => {
      const [action, scope, level] = required.split('.');
      return context.permissions.some(perm =>
        perm.action === action &&
        perm.scope === scope &&
        perm.level === level
      );
    });
  },

  // Get hook categories
  getHookCategories: (): string[] => [
    'filesystem',
    'terminal',
    'ui',
    'git',
    'testing',
    'deployment',
    'monitoring',
    'security',
    'documentation',
    'refactoring',
  ],

  // Get agent types
  getAgentTypes: (): string[] => [
    'architect',
    'developer',
    'tester',
    'reviewer',
    'devops',
    'security',
    'documentation',
    'project-manager',
  ],
};

// Default permissions for each agent type
function getDefaultPermissions(agentType: string): string[] {
  const permissionMap: Record<string, string[]> = {
    architect: [
      'file.project.read',
      'ui.global.write',
      'git.project.read',
    ],
    developer: [
      'file.project.write',
      'terminal.global.execute',
      'ui.global.write',
      'git.project.write',
    ],
    tester: [
      'file.project.read',
      'terminal.global.execute',
      'ui.global.write',
      'git.project.read',
    ],
    reviewer: [
      'file.project.read',
      'ui.global.write',
      'git.project.read',
    ],
    devops: [
      'file.project.write',
      'terminal.global.execute',
      'ui.global.write',
      'git.project.write',
    ],
    security: [
      'file.project.read',
      'terminal.global.execute',
      'ui.global.write',
    ],
    documentation: [
      'file.project.write',
      'ui.global.write',
      'git.project.read',
    ],
    'project-manager': [
      'file.project.read',
      'ui.global.write',
      'git.project.read',
    ],
  };

  return permissionMap[agentType] || ['file.project.read', 'ui.global.write'];
}

// Hook execution wrapper with error handling and logging
export const executeHookSafely = async <T>(
  hookFunction: () => Promise<HookResult<T>>,
  context: {
    agentId: string;
    hookName: string;
    parameters?: any;
  }
): Promise<HookResult<T>> => {
  const startTime = Date.now();
  
  try {
    console.log(`[Agent ${context.agentId}] Executing hook: ${context.hookName}`, context.parameters);
    
    const result = await hookFunction();
    const executionTime = Date.now() - startTime;
    
    console.log(`[Agent ${context.agentId}] Hook completed: ${context.hookName} (${executionTime}ms)`, {
      success: result.success,
      hasData: !!result.data,
      error: result.error,
    });
    
    return result;
  } catch (error) {
    const executionTime = Date.now() - startTime;
    const errorMessage = error instanceof Error ? error.message : String(error);
    
    console.error(`[Agent ${context.agentId}] Hook failed: ${context.hookName} (${executionTime}ms)`, {
      error: errorMessage,
      parameters: context.parameters,
    });
    
    return {
      success: false,
      error: errorMessage,
      metadata: {
        executionTime,
        timestamp: new Date().toISOString(),
      },
    };
  }
};

// Batch hook execution for complex workflows
export const executeBatchHooks = async (
  hooks: Array<{
    name: string;
    function: () => Promise<HookResult>;
    dependencies?: string[];
  }>,
  context: { agentId: string }
): Promise<Record<string, HookResult>> => {
  const results: Record<string, HookResult> = {};
  const executed = new Set<string>();
  const executing = new Set<string>();

  const executeHook = async (hookName: string): Promise<void> => {
    if (executed.has(hookName) || executing.has(hookName)) {
      return;
    }

    const hook = hooks.find(h => h.name === hookName);
    if (!hook) {
      throw new Error(`Hook ${hookName} not found`);
    }

    executing.add(hookName);

    // Execute dependencies first
    if (hook.dependencies) {
      for (const dep of hook.dependencies) {
        await executeHook(dep);
      }
    }

    // Execute the hook
    results[hookName] = await executeHookSafely(hook.function, {
      agentId: context.agentId,
      hookName,
    });

    executing.delete(hookName);
    executed.add(hookName);
  };

  // Execute all hooks
  for (const hook of hooks) {
    await executeHook(hook.name);
  }

  return results;
};

// Types are already exported above
