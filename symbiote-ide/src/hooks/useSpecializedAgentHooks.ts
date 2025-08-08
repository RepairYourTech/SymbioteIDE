// Specialized Agent Hooks - Role-specific hooks for different agent types
// Provides specialized functionality for Architect, Developer, Tester, and other agent roles

import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAgentHooks, AgentHookContext, HookResult } from './useAgentHooks';

// Architect Agent Hooks
export const useArchitectHooks = (context: AgentHookContext) => {
  const baseHooks = useAgentHooks(context);

  return {
    ...baseHooks,
    
    // Architecture analysis
    analyzeCodebase: useCallback(async (projectPath: string): Promise<HookResult<any>> => {
      try {
        const analysis = await invoke('agent_analyze_codebase', {
          agentId: context.agentId,
          projectPath,
        });
        return { success: true, data: analysis };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    generateArchitectureDiagram: useCallback(async (analysisData: any): Promise<HookResult<string>> => {
      try {
        const diagramPath = await invoke<string>('agent_generate_architecture_diagram', {
          agentId: context.agentId,
          analysisData,
        });
        return { success: true, data: diagramPath };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    suggestRefactoring: useCallback(async (filePath: string): Promise<HookResult<any>> => {
      try {
        const suggestions = await invoke('agent_suggest_refactoring', {
          agentId: context.agentId,
          filePath,
        });
        return { success: true, data: suggestions };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    designPattern: useCallback(async (pattern: string, context: any): Promise<HookResult<string>> => {
      try {
        const implementation = await invoke<string>('agent_design_pattern', {
          agentId: context.agentId,
          pattern,
          context,
        });
        return { success: true, data: implementation };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  };
};

// Developer Agent Hooks
export const useDeveloperHooks = (context: AgentHookContext) => {
  const baseHooks = useAgentHooks(context);

  return {
    ...baseHooks,
    
    // Code generation and modification
    generateCode: useCallback(async (specification: string, language: string): Promise<HookResult<string>> => {
      try {
        const code = await invoke<string>('agent_generate_code', {
          agentId: context.agentId,
          specification,
          language,
        });
        return { success: true, data: code };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    refactorCode: useCallback(async (filePath: string, refactoringType: string): Promise<HookResult<string>> => {
      try {
        const refactoredCode = await invoke<string>('agent_refactor_code', {
          agentId: context.agentId,
          filePath,
          refactoringType,
        });
        return { success: true, data: refactoredCode };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    fixBug: useCallback(async (filePath: string, bugDescription: string): Promise<HookResult<string>> => {
      try {
        const fixedCode = await invoke<string>('agent_fix_bug', {
          agentId: context.agentId,
          filePath,
          bugDescription,
        });
        return { success: true, data: fixedCode };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    optimizePerformance: useCallback(async (filePath: string): Promise<HookResult<any>> => {
      try {
        const optimizations = await invoke('agent_optimize_performance', {
          agentId: context.agentId,
          filePath,
        });
        return { success: true, data: optimizations };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    addDocumentation: useCallback(async (filePath: string, docType: string): Promise<HookResult<string>> => {
      try {
        const documentation = await invoke<string>('agent_add_documentation', {
          agentId: context.agentId,
          filePath,
          docType,
        });
        return { success: true, data: documentation };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  };
};

// Tester Agent Hooks
export const useTesterHooks = (context: AgentHookContext) => {
  const baseHooks = useAgentHooks(context);

  return {
    ...baseHooks,
    
    // Test generation and execution
    generateTests: useCallback(async (filePath: string, testType: string): Promise<HookResult<string>> => {
      try {
        const tests = await invoke<string>('agent_generate_tests', {
          agentId: context.agentId,
          filePath,
          testType,
        });
        return { success: true, data: tests };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    runTests: useCallback(async (testPath: string): Promise<HookResult<any>> => {
      try {
        const results = await invoke('agent_run_tests', {
          agentId: context.agentId,
          testPath,
        });
        return { success: true, data: results };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    analyzeCoverage: useCallback(async (projectPath: string): Promise<HookResult<any>> => {
      try {
        const coverage = await invoke('agent_analyze_coverage', {
          agentId: context.agentId,
          projectPath,
        });
        return { success: true, data: coverage };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    performanceTest: useCallback(async (endpoint: string, config: any): Promise<HookResult<any>> => {
      try {
        const results = await invoke('agent_performance_test', {
          agentId: context.agentId,
          endpoint,
          config,
        });
        return { success: true, data: results };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    securityScan: useCallback(async (projectPath: string): Promise<HookResult<any>> => {
      try {
        const vulnerabilities = await invoke('agent_security_scan', {
          agentId: context.agentId,
          projectPath,
        });
        return { success: true, data: vulnerabilities };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  };
};

// Reviewer Agent Hooks
export const useReviewerHooks = (context: AgentHookContext) => {
  const baseHooks = useAgentHooks(context);

  return {
    ...baseHooks,
    
    // Code review and quality analysis
    reviewCode: useCallback(async (filePath: string, reviewType: string): Promise<HookResult<any>> => {
      try {
        const review = await invoke('agent_review_code', {
          agentId: context.agentId,
          filePath,
          reviewType,
        });
        return { success: true, data: review };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    checkCompliance: useCallback(async (projectPath: string, standards: string[]): Promise<HookResult<any>> => {
      try {
        const compliance = await invoke('agent_check_compliance', {
          agentId: context.agentId,
          projectPath,
          standards,
        });
        return { success: true, data: compliance };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    analyzeComplexity: useCallback(async (filePath: string): Promise<HookResult<any>> => {
      try {
        const complexity = await invoke('agent_analyze_complexity', {
          agentId: context.agentId,
          filePath,
        });
        return { success: true, data: complexity };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    suggestImprovements: useCallback(async (filePath: string): Promise<HookResult<any>> => {
      try {
        const suggestions = await invoke('agent_suggest_improvements', {
          agentId: context.agentId,
          filePath,
        });
        return { success: true, data: suggestions };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  };
};

// DevOps Agent Hooks
export const useDevOpsHooks = (context: AgentHookContext) => {
  const baseHooks = useAgentHooks(context);

  return {
    ...baseHooks,
    
    // Deployment and infrastructure
    deployApplication: useCallback(async (environment: string, config: any): Promise<HookResult<any>> => {
      try {
        const deployment = await invoke('agent_deploy_application', {
          agentId: context.agentId,
          environment,
          config,
        });
        return { success: true, data: deployment };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    setupCI: useCallback(async (projectPath: string, platform: string): Promise<HookResult<string>> => {
      try {
        const ciConfig = await invoke<string>('agent_setup_ci', {
          agentId: context.agentId,
          projectPath,
          platform,
        });
        return { success: true, data: ciConfig };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    monitorHealth: useCallback(async (services: string[]): Promise<HookResult<any>> => {
      try {
        const health = await invoke('agent_monitor_health', {
          agentId: context.agentId,
          services,
        });
        return { success: true, data: health };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),

    scaleResources: useCallback(async (service: string, scale: number): Promise<HookResult<any>> => {
      try {
        const result = await invoke('agent_scale_resources', {
          agentId: context.agentId,
          service,
          scale,
        });
        return { success: true, data: result };
      } catch (error) {
        return { success: false, error: String(error) };
      }
    }, []),
  };
};

// All hooks are already exported above
