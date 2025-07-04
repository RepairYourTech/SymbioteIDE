/**
 * Debug Controller Agent - Orchestrates the debugging workflow
 */

import { ADKAgentConfig, ADKAgentType, AgentExecutionRequest, AgentExecutionResult } from '../../adk/types';
import { ADKClient } from '../../adk/adk-client';
import { Logger } from '../../utils/logger';
import { 
  DebugSession, 
  ErrorContext, 
  DebugAnalysis, 
  DebugFix,
  DebugAgentRequest,
  DebugAgentResponse,
  DebugStep,
  WorkingMemory
} from '../types';
import { v4 as uuidv4 } from 'uuid';

export class DebugControllerAgent {
  private logger = new Logger('DebugControllerAgent');
  private adkClient: ADKClient;
  private agentId: string;
  private subAgents: Map<string, string> = new Map(); // role -> agentId
  
  constructor(private config: ADKAgentConfig) {
    this.adkClient = new ADKClient();
    this.agentId = config.id;
  }
  
  /**
   * Initialize the controller and sub-agents
   */
  async initialize(): Promise<void> {
    try {
      // Create controller agent with Sequential type for step-by-step debugging
      await this.adkClient.createAgent({
        ...this.config,
        type: ADKAgentType.Sequential,
        systemPrompt: `You are the Debug Controller Agent orchestrating a debugging workflow.
        
Your responsibilities:
1. Coordinate sub-agents to analyze and fix errors
2. Maintain debugging context and memory
3. Ensure systematic investigation
4. Generate comprehensive solutions

Sub-agents available:
- Error Analyzer: Deep error analysis and pattern recognition
- Stack Navigator: Traverse and analyze stack traces
- Variable Inspector: Examine variable states and mutations
- Fix Generator: Create and validate fixes

Workflow:
1. Receive error context
2. Delegate to Error Analyzer for initial analysis
3. Use Stack Navigator to trace execution path
4. Employ Variable Inspector for state analysis
5. Generate fixes with Fix Generator
6. Validate and rank solutions

Always provide clear reasoning and maintain debugging history.`,
        capabilities: {
          streaming: true,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: true,
          mcpTools: true,
          multiModal: false
        }
      });
      
      // Initialize sub-agents
      await this.initializeSubAgents();
      
      this.logger.info('Debug Controller initialized with sub-agents');
      
    } catch (error) {
      this.logger.error('Failed to initialize Debug Controller', error);
      throw error;
    }
  }
  
  /**
   * Initialize specialized sub-agents
   */
  private async initializeSubAgents(): Promise<void> {
    // Error Analyzer Agent
    const errorAnalyzerId = await this.adkClient.createAgent({
      id: `${this.agentId}_error_analyzer`,
      name: 'Error Analyzer',
      type: ADKAgentType.LLM,
      model: { id: 'gpt-4', provider: 'openai' },
      systemPrompt: `You are an expert Error Analyzer agent.

Analyze errors to identify:
- Root cause of the error
- Error patterns and categories
- Similar historical errors
- Impact on the system
- Potential cascading effects

Provide structured analysis with confidence scores.`,
      temperature: 0.3,
      capabilities: {
        streaming: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: false,
        multiModal: false
      }
    });
    this.subAgents.set('errorAnalyzer', errorAnalyzerId);
    
    // Stack Navigator Agent
    const stackNavigatorId = await this.adkClient.createAgent({
      id: `${this.agentId}_stack_navigator`,
      name: 'Stack Navigator',
      type: ADKAgentType.LLM,
      model: { id: 'gemini-1.5-flash', provider: 'google' },
      systemPrompt: `You are a Stack Trace Navigator agent.

Your tasks:
- Parse and analyze stack traces
- Identify the exact error location
- Trace execution flow
- Highlight suspicious call patterns
- Find the originating user code

Focus on speed and accuracy in navigation.`,
      temperature: 0.1,
      capabilities: {
        streaming: false,
        toolUse: true,
        memoryAccess: false,
        a2aProtocol: true,
        mcpTools: false,
        multiModal: false
      }
    });
    this.subAgents.set('stackNavigator', stackNavigatorId);
    
    // Variable Inspector Agent
    const variableInspectorId = await this.adkClient.createAgent({
      id: `${this.agentId}_variable_inspector`,
      name: 'Variable Inspector',
      type: ADKAgentType.LLM,
      model: { id: 'claude-3-sonnet-20240229', provider: 'anthropic' },
      systemPrompt: `You are a Variable State Inspector agent.

Examine variable states to identify:
- Null/undefined values
- Type mismatches
- Unexpected mutations
- Memory leaks
- Race conditions
- Out-of-bounds access

Provide detailed state analysis with suspicious value detection.`,
      temperature: 0.2,
      capabilities: {
        streaming: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: false,
        multiModal: false
      }
    });
    this.subAgents.set('variableInspector', variableInspectorId);
    
    // Fix Generator Agent
    const fixGeneratorId = await this.adkClient.createAgent({
      id: `${this.agentId}_fix_generator`,
      name: 'Fix Generator',
      type: ADKAgentType.LLM,
      model: { id: 'gpt-4', provider: 'openai' },
      systemPrompt: `You are a Fix Generator agent.

Generate fixes that are:
- Correct and complete
- Well-tested
- Performance-conscious
- Following best practices
- Properly documented

Provide multiple fix alternatives with:
- Confidence scores
- Impact assessment
- Test suggestions
- Explanation of approach`,
      temperature: 0.4,
      capabilities: {
        streaming: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true,
        multiModal: false
      }
    });
    this.subAgents.set('fixGenerator', fixGeneratorId);
  }
  
  /**
   * Execute debugging workflow
   */
  async debug(request: DebugAgentRequest): Promise<DebugSession> {
    const session: DebugSession = {
      id: request.sessionId || uuidv4(),
      projectId: 'current',
      userId: 'current',
      startTime: new Date(),
      status: 'active',
      error: request.error,
      memory: {
        episodic: [],
        semantic: [],
        working: {
          currentError: request.error,
          investigatedFiles: [],
          testedHypotheses: [],
          ruledOutCauses: [],
          viableFixOptions: []
        },
        patterns: []
      }
    };
    
    const steps: DebugStep[] = [];
    
    try {
      // Step 1: Error Analysis
      const analysisStep = await this.executeStep(
        'errorAnalyzer',
        'analyze',
        {
          error: request.error,
          code: request.code,
          context: request.history
        },
        steps
      );
      
      const analysis = analysisStep.result as DebugAnalysis;
      session.analysis = analysis;
      
      // Step 2: Stack Navigation (if stack trace available)
      if (request.error.stack) {
        const stackStep = await this.executeStep(
          'stackNavigator',
          'navigate',
          {
            stack: request.error.stack,
            code: request.code,
            focusUserCode: true
          },
          steps
        );
        
        // Update working memory with investigated files
        const stackInfo = stackStep.result;
        if (stackInfo.userCodeFrames) {
          session.memory!.working.investigatedFiles.push(
            ...stackInfo.userCodeFrames.map((f: any) => f.file)
          );
        }
      }
      
      // Step 3: Variable Inspection
      const variableStep = await this.executeStep(
        'variableInspector',
        'inspect',
        {
          error: request.error,
          code: request.code,
          analysis: analysis,
          suspectedVariables: analysis.patterns?.map(p => p.name) || []
        },
        steps
      );
      
      const variableIssues = variableStep.result;
      
      // Step 4: Fix Generation
      const fixStep = await this.executeStep(
        'fixGenerator',
        'generate',
        {
          error: request.error,
          code: request.code,
          analysis: analysis,
          variableIssues: variableIssues,
          preferences: request.preferences
        },
        steps
      );
      
      const fixes = fixStep.result as DebugFix[];
      session.fixes = fixes;
      
      // Step 5: Orchestrate final recommendations
      const orchestrationResult = await this.orchestrate({
        session,
        steps,
        preferences: request.preferences
      });
      
      // Update session with orchestration results
      session.breakpoints = orchestrationResult.breakpoints;
      session.memory!.working.viableFixOptions = fixes;
      session.status = 'completed';
      session.endTime = new Date();
      
      return session;
      
    } catch (error) {
      this.logger.error('Debug workflow failed', error);
      session.status = 'failed';
      session.endTime = new Date();
      throw error;
    }
  }
  
  /**
   * Execute a step with a sub-agent
   */
  private async executeStep(
    agentRole: string,
    action: string,
    input: any,
    steps: DebugStep[]
  ): Promise<DebugStep> {
    const agentId = this.subAgents.get(agentRole);
    if (!agentId) {
      throw new Error(`Sub-agent ${agentRole} not found`);
    }
    
    const startTime = Date.now();
    
    try {
      const result = await this.adkClient.executeAgent(agentId, {
        action,
        input,
        context: {
          previousSteps: steps.slice(-3) // Last 3 steps for context
        }
      });
      
      const step: DebugStep = {
        id: uuidv4(),
        timestamp: new Date(),
        action: `${agentRole}.${action}`,
        agentId,
        input,
        output: result,
        duration: Date.now() - startTime,
        success: true
      };
      
      steps.push(step);
      return step;
      
    } catch (error) {
      const step: DebugStep = {
        id: uuidv4(),
        timestamp: new Date(),
        action: `${agentRole}.${action}`,
        agentId,
        input,
        output: { error: error instanceof Error ? error.message : 'Unknown error' },
        duration: Date.now() - startTime,
        success: false
      };
      
      steps.push(step);
      throw error;
    }
  }
  
  /**
   * Orchestrate final recommendations
   */
  private async orchestrate(context: {
    session: DebugSession;
    steps: DebugStep[];
    preferences?: any;
  }): Promise<{
    breakpoints: any[];
    summary: string;
    confidence: number;
  }> {
    const request: AgentExecutionRequest = {
      action: 'orchestrate',
      input: {
        analysis: context.session.analysis,
        fixes: context.session.fixes,
        steps: context.steps,
        preferences: context.preferences
      }
    };
    
    const result = await this.adkClient.executeAgent(this.agentId, request);
    
    return {
      breakpoints: result.breakpoints || [],
      summary: result.summary || 'Debugging complete',
      confidence: result.confidence || 0.8
    };
  }
  
  /**
   * Get debugging history
   */
  async getHistory(limit: number = 10): Promise<EpisodicMemory[]> {
    const memory = await this.adkClient.getAgentMemory(this.agentId, {
      type: 'episodic',
      limit
    });
    
    return memory.episodes || [];
  }
  
  /**
   * Learn from debugging session
   */
  async learn(session: DebugSession): Promise<void> {
    if (!session.fixes || session.fixes.length === 0) return;
    
    // Store successful patterns
    const successfulFix = session.fixes.find(f => f.success);
    if (successfulFix) {
      await this.adkClient.updateAgentMemory(this.agentId, {
        type: 'semantic',
        concept: `error_pattern_${session.error?.type}`,
        data: {
          error: session.error,
          solution: successfulFix,
          analysis: session.analysis,
          timestamp: new Date()
        }
      });
    }
  }
}