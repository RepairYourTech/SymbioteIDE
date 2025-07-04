/**
 * ADK-Enhanced Hivemind Controller
 * 
 * Orchestrates multi-agent collaboration using Google ADK patterns
 */

import { Orchestrator, LLMAgent } from '@google/adk';
import { Logger } from '../../utils/logger';
import { EventEmitter } from 'events';
import { 
  HivemindTask, 
  TaskResult, 
  AgentSpecialty,
  HivemindConfig,
  TaskPriority,
  TaskStatus 
} from '../types';
import { MemoryManager } from '../../memory/unified-context-api';
import { RedisQueueManager } from '../../queue/redis-queue-manager';

// ADK Components
import { ADKHivemindAgent } from './core/adk-hivemind-base';
import { HivemindSessionState } from './state/hivemind-session-state';
import { LLMDrivenRouter } from './routing/llm-router';

// ADK Patterns
import { CoordinatorAgent } from './patterns/coordinator-agent';
import { PipelineAgent } from './patterns/pipeline-agent';
import { ParallelGatherAgent } from './patterns/parallel-gather-agent';
import { LoopRefinerAgent } from './patterns/loop-refiner-agent';
import { HierarchicalDecomposerAgent } from './patterns/hierarchical-decomposer-agent';
import { ReviewCritiqueAgent } from './patterns/review-critique-agent';
import { HumanInLoopAgent } from './patterns/human-in-loop-agent';

// Specialist Agents (to be migrated)
import { FrontendSpecialistAgent } from '../agents/frontend-specialist';

export interface ADKHivemindControllerConfig extends HivemindConfig {
  enablePatterns?: string[]; // Which ADK patterns to enable
  routingStrategy?: 'llm' | 'rule-based' | 'hybrid';
  humanInLoopEnabled?: boolean;
  reviewEnabled?: boolean;
  hierarchicalDepth?: number;
}

export class ADKHivemindController extends EventEmitter {
  private logger = new Logger('ADKHivemindController');
  private config: ADKHivemindControllerConfig;
  private orchestrator: Orchestrator;
  private sessionState: HivemindSessionState;
  private memoryManager?: MemoryManager;
  private queueManager?: RedisQueueManager;
  
  // Core components
  private router: LLMDrivenRouter;
  private agents: Map<string, ADKHivemindAgent> = new Map();
  private patternAgents: Map<string, ADKHivemindAgent> = new Map();
  
  // Metrics
  private metrics = {
    tasksProcessed: 0,
    successRate: 0,
    averageDuration: 0,
    patternUsage: new Map<string, number>()
  };
  
  constructor(config: ADKHivemindControllerConfig) {
    super();
    this.config = config;
    
    // Initialize session state
    this.sessionState = new HivemindSessionState();
    
    // Initialize orchestrator
    this.orchestrator = new Orchestrator({
      name: 'Hivemind ADK Orchestrator',
      description: 'Orchestrates multi-agent collaboration for complex tasks'
    });
    
    // Initialize components
    this.initializeRouter();
    this.initializeSpecialistAgents();
    this.initializePatternAgents();
    this.setupEventHandlers();
  }
  
  /**
   * Initialize the routing system
   */
  private initializeRouter(): void {
    const routerContext = {
      availableAgents: new Map(),
      systemConstraints: {
        maxConcurrentAgents: this.config.maxConcurrentAgents || 5,
        preferredPatterns: this.config.enablePatterns,
        costOptimization: true
      }
    };
    
    this.router = new LLMDrivenRouter(routerContext, true);
  }
  
  /**
   * Initialize specialist agents
   */
  private async initializeSpecialistAgents(): Promise<void> {
    // Frontend Specialist
    const frontendAgent = new FrontendSpecialistAgent({
      id: 'frontend-specialist',
      name: 'Frontend Specialist',
      memoryManager: this.memoryManager,
      sessionState: this.sessionState
    });
    this.agents.set('frontend-specialist', frontendAgent);
    
    // Backend Specialist (placeholder - to be migrated)
    const backendAgent = new ADKHivemindAgent({
      id: 'backend-specialist',
      name: 'Backend Specialist',
      specialty: AgentSpecialty.Backend,
      capabilities: ['api_development', 'database_design', 'authentication'],
      memoryManager: this.memoryManager,
      sessionState: this.sessionState
    });
    this.agents.set('backend-specialist', backendAgent);
    
    // Add more specialists as they are migrated...
    
    // Update router with available agents
    this.updateRouterAgents();
  }
  
  /**
   * Initialize ADK pattern agents
   */
  private async initializePatternAgents(): Promise<void> {
    const enabledPatterns = this.config.enablePatterns || [
      'coordinator', 'pipeline', 'parallel', 'loop', 'hierarchical'
    ];
    
    // Coordinator Pattern
    if (enabledPatterns.includes('coordinator')) {
      const coordinator = new CoordinatorAgent({
        id: 'coordinator',
        name: 'Task Coordinator',
        subAgents: this.agents,
        routingStrategy: this.config.routingStrategy || 'hybrid',
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('coordinator', coordinator);
    }
    
    // Pipeline Pattern
    if (enabledPatterns.includes('pipeline')) {
      const pipeline = new PipelineAgent({
        id: 'pipeline',
        name: 'Sequential Pipeline',
        steps: [], // Will be configured per task
        continueOnError: false,
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('pipeline', pipeline);
    }
    
    // Parallel Pattern
    if (enabledPatterns.includes('parallel')) {
      const parallel = new ParallelGatherAgent({
        id: 'parallel',
        name: 'Parallel Executor',
        parallelTasks: [], // Will be configured per task
        gatherStrategy: { type: 'best' },
        maxConcurrency: this.config.maxConcurrentAgents,
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('parallel', parallel);
    }
    
    // Loop Refiner Pattern
    if (enabledPatterns.includes('loop')) {
      const loopRefiner = new LoopRefinerAgent({
        id: 'loop-refiner',
        name: 'Iterative Refiner',
        steps: [], // Will be configured per task
        maxIterations: 5,
        exitCriteria: ReviewCritiqueAgent.createDefaultCriteria(),
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('loop', loopRefiner);
    }
    
    // Hierarchical Pattern
    if (enabledPatterns.includes('hierarchical')) {
      const hierarchical = new HierarchicalDecomposerAgent({
        id: 'hierarchical',
        name: 'Task Decomposer',
        decompositionStrategy: {
          maxDepth: this.config.hierarchicalDepth || 3,
          minTaskComplexity: 3,
          decompositionThreshold: 7
        },
        subAgents: this.agents,
        autoAssignAgents: true,
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('hierarchical', hierarchical);
    }
    
    // Review Pattern
    if (this.config.reviewEnabled) {
      const reviewer = new ReviewCritiqueAgent({
        id: 'reviewer',
        name: 'Quality Reviewer',
        reviewCriteria: ReviewCritiqueAgent.createDefaultCriteria(),
        passingThreshold: 80,
        provideSuggestions: true,
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('review', reviewer);
    }
    
    // Human-in-the-Loop Pattern
    if (this.config.humanInLoopEnabled) {
      const humanInLoop = new HumanInLoopAgent({
        id: 'human-in-loop',
        name: 'Human Collaborator',
        interactionPoints: [
          {
            id: 'approval',
            type: 'approval',
            title: 'Approve Action',
            description: 'Approve critical actions',
            required: true
          },
          {
            id: 'guidance',
            type: 'guidance',
            title: 'Provide Guidance',
            description: 'Guide AI decision making',
            required: false
          }
        ],
        fallbackBehavior: 'wait',
        showProgress: true,
        memoryManager: this.memoryManager,
        sessionState: this.sessionState
      });
      this.patternAgents.set('human', humanInLoop);
    }
  }
  
  /**
   * Update router with available agents
   */
  private updateRouterAgents(): void {
    const agentProfiles = new Map();
    
    // Add specialist agents
    for (const [id, agent] of this.agents) {
      agentProfiles.set(id, {
        id,
        name: agent.name,
        specialty: agent.specialty,
        capabilities: agent.capabilities,
        performance: {
          successRate: 0.9, // Would track actual performance
          averageDuration: 60000,
          specialtyMatch: new Map()
        },
        currentLoad: 0,
        availability: true
      });
    }
    
    // Add pattern agents
    for (const [id, agent] of this.patternAgents) {
      agentProfiles.set(id, {
        id,
        name: agent.name,
        specialty: agent.specialty,
        capabilities: agent.capabilities,
        performance: {
          successRate: 0.95,
          averageDuration: 120000,
          specialtyMatch: new Map()
        },
        currentLoad: 0,
        availability: true
      });
    }
    
    // Update router context
    this.router = new LLMDrivenRouter({
      availableAgents: agentProfiles,
      systemConstraints: {
        maxConcurrentAgents: this.config.maxConcurrentAgents || 5,
        preferredPatterns: this.config.enablePatterns,
        costOptimization: true
      }
    }, true);
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Session state events
    this.sessionState.on('stateChanged', (data) => {
      this.logger.debug('Session state changed', data);
      this.emit('stateChanged', data);
    });
    
    // Agent events
    for (const agent of this.agents.values()) {
      agent.on('taskStarted', (data) => {
        this.emit('agentTaskStarted', { agentId: agent.id, ...data });
      });
      
      agent.on('taskCompleted', (data) => {
        this.emit('agentTaskCompleted', { agentId: agent.id, ...data });
      });
    }
  }
  
  /**
   * Execute a task using appropriate pattern and agents
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      this.logger.info(`Executing task ${task.id}: ${task.title}`);
      
      // Route task to determine execution pattern
      const routing = await this.router.route(task);
      this.logger.info(`Routing decision: ${routing.executionPattern} with agents: ${routing.targetAgents.join(', ')}`);
      
      // Track pattern usage
      this.metrics.patternUsage.set(
        routing.executionPattern,
        (this.metrics.patternUsage.get(routing.executionPattern) || 0) + 1
      );
      
      // Execute based on pattern
      let result: TaskResult;
      
      switch (routing.executionPattern) {
        case 'sequential':
          result = await this.executeSequential(task, routing.targetAgents);
          break;
          
        case 'parallel':
          result = await this.executeParallel(task, routing.targetAgents);
          break;
          
        case 'loop':
          result = await this.executeLoop(task, routing.targetAgents);
          break;
          
        case 'hierarchical':
          result = await this.executeHierarchical(task);
          break;
          
        case 'conditional':
          result = await this.executeConditional(task, routing);
          break;
          
        default:
          // Default to coordinator pattern
          result = await this.executeCoordinated(task);
      }
      
      // Apply review if enabled
      if (this.config.reviewEnabled && result.success) {
        result = await this.applyReview(task, result);
      }
      
      // Update metrics
      this.updateMetrics(result, Date.now() - startTime);
      
      // Store result in memory if available
      if (this.memoryManager) {
        await this.memoryManager.remember({
          content: JSON.stringify({ task, result }),
          metadata: {
            taskId: task.id,
            pattern: routing.executionPattern,
            success: result.success
          }
        });
      }
      
      // Update routing outcome
      this.router.updateOutcome(task.id, result.success, Date.now() - startTime);
      
      this.emit('taskCompleted', { task, result });
      
      return result;
      
    } catch (error) {
      this.logger.error(`Task execution failed`, error);
      
      const errorResult: TaskResult = {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Execution error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
      
      this.emit('taskFailed', { task, error });
      
      return errorResult;
    }
  }
  
  /**
   * Execute sequential pattern
   */
  private async executeSequential(
    task: HivemindTask,
    agentIds: string[]
  ): Promise<TaskResult> {
    const pipelineAgent = this.patternAgents.get('pipeline') as PipelineAgent;
    if (!pipelineAgent) {
      throw new Error('Pipeline pattern not available');
    }
    
    // Configure pipeline steps
    const steps = agentIds.map((agentId, index) => {
      const agent = this.agents.get(agentId) || this.patternAgents.get(agentId);
      if (!agent) throw new Error(`Agent ${agentId} not found`);
      
      return {
        agentId,
        agent,
        name: `Step ${index + 1}: ${agent.name}`,
        description: `Execute with ${agent.name}`
      };
    });
    
    // Update pipeline configuration
    pipelineAgent.steps = steps;
    
    return pipelineAgent.executeTask(task);
  }
  
  /**
   * Execute parallel pattern
   */
  private async executeParallel(
    task: HivemindTask,
    agentIds: string[]
  ): Promise<TaskResult> {
    const parallelAgent = this.patternAgents.get('parallel') as ParallelGatherAgent;
    if (!parallelAgent) {
      throw new Error('Parallel pattern not available');
    }
    
    // Configure parallel tasks
    const parallelTasks = agentIds.map(agentId => {
      const agent = this.agents.get(agentId) || this.patternAgents.get(agentId);
      if (!agent) throw new Error(`Agent ${agentId} not found`);
      
      return {
        agentId,
        agent,
        task: {},
        weight: 1
      };
    });
    
    // Update parallel configuration
    parallelAgent.parallelTasks = parallelTasks;
    
    return parallelAgent.executeTask(task);
  }
  
  /**
   * Execute loop pattern
   */
  private async executeLoop(
    task: HivemindTask,
    agentIds: string[]
  ): Promise<TaskResult> {
    const loopAgent = this.patternAgents.get('loop') as LoopRefinerAgent;
    if (!loopAgent) {
      throw new Error('Loop pattern not available');
    }
    
    // Configure loop steps
    const steps = agentIds.map(agentId => {
      const agent = this.agents.get(agentId) || this.patternAgents.get(agentId);
      if (!agent) throw new Error(`Agent ${agentId} not found`);
      
      return {
        agent,
        role: agentId.includes('review') ? 'evaluator' : 
              agentId.includes('refine') ? 'refiner' : 'generator'
      };
    });
    
    // Update loop configuration
    loopAgent.steps = steps;
    
    return loopAgent.executeTask(task);
  }
  
  /**
   * Execute hierarchical pattern
   */
  private async executeHierarchical(task: HivemindTask): Promise<TaskResult> {
    const hierarchicalAgent = this.patternAgents.get('hierarchical') as HierarchicalDecomposerAgent;
    if (!hierarchicalAgent) {
      throw new Error('Hierarchical pattern not available');
    }
    
    return hierarchicalAgent.executeTask(task);
  }
  
  /**
   * Execute conditional pattern
   */
  private async executeConditional(
    task: HivemindTask,
    routing: any
  ): Promise<TaskResult> {
    const coordinatorAgent = this.patternAgents.get('coordinator') as CoordinatorAgent;
    if (!coordinatorAgent) {
      throw new Error('Coordinator pattern not available');
    }
    
    // Coordinator handles conditional logic
    return coordinatorAgent.executeTask(task);
  }
  
  /**
   * Execute coordinated pattern (default)
   */
  private async executeCoordinated(task: HivemindTask): Promise<TaskResult> {
    const coordinatorAgent = this.patternAgents.get('coordinator') as CoordinatorAgent;
    if (!coordinatorAgent) {
      // Fallback to first available agent
      const firstAgent = this.agents.values().next().value;
      if (!firstAgent) {
        throw new Error('No agents available');
      }
      return firstAgent.executeTask(task);
    }
    
    return coordinatorAgent.executeTask(task);
  }
  
  /**
   * Apply review to task result
   */
  private async applyReview(
    task: HivemindTask,
    result: TaskResult
  ): Promise<TaskResult> {
    const reviewAgent = this.patternAgents.get('review') as ReviewCritiqueAgent;
    if (!reviewAgent) {
      return result;
    }
    
    // Create review task
    const reviewTask: HivemindTask = {
      id: `${task.id}_review`,
      type: 'review',
      title: `Review: ${task.title}`,
      description: 'Review task output for quality and correctness',
      requiredSpecialties: [AgentSpecialty.General],
      priority: task.priority,
      status: TaskStatus.Pending,
      dependencies: [],
      assignedAgents: ['reviewer'],
      estimatedComplexity: 3,
      context: {
        originalTask: task,
        outputToReview: result.output
      },
      createdAt: new Date()
    };
    
    const reviewResult = await reviewAgent.executeTask(reviewTask);
    
    // Merge review into original result
    if (!reviewResult.success) {
      result.success = false;
      result.issuesFound.push(...reviewResult.issuesFound);
    }
    
    result.output.reviewResult = reviewResult.output;
    
    return result;
  }
  
  /**
   * Update metrics
   */
  private updateMetrics(result: TaskResult, duration: number): void {
    this.metrics.tasksProcessed++;
    
    const oldSuccessRate = this.metrics.successRate;
    const successCount = oldSuccessRate * (this.metrics.tasksProcessed - 1);
    this.metrics.successRate = (successCount + (result.success ? 1 : 0)) / this.metrics.tasksProcessed;
    
    const oldAvgDuration = this.metrics.averageDuration;
    const totalDuration = oldAvgDuration * (this.metrics.tasksProcessed - 1);
    this.metrics.averageDuration = (totalDuration + duration) / this.metrics.tasksProcessed;
  }
  
  /**
   * Get controller metrics
   */
  public getMetrics(): any {
    return {
      ...this.metrics,
      patternUsage: Object.fromEntries(this.metrics.patternUsage),
      agentCount: this.agents.size,
      patternCount: this.patternAgents.size
    };
  }
  
  /**
   * Get agent by ID
   */
  public getAgent(agentId: string): ADKHivemindAgent | undefined {
    return this.agents.get(agentId) || this.patternAgents.get(agentId);
  }
  
  /**
   * List all agents
   */
  public listAgents(): Array<{ id: string; name: string; type: string }> {
    const agents: Array<{ id: string; name: string; type: string }> = [];
    
    for (const [id, agent] of this.agents) {
      agents.push({ id, name: agent.name, type: 'specialist' });
    }
    
    for (const [id, agent] of this.patternAgents) {
      agents.push({ id, name: agent.name, type: 'pattern' });
    }
    
    return agents;
  }
  
  /**
   * Initialize with external services
   */
  public async initialize(
    memoryManager?: MemoryManager,
    queueManager?: RedisQueueManager
  ): Promise<void> {
    this.memoryManager = memoryManager;
    this.queueManager = queueManager;
    
    // Reinitialize agents with services
    await this.initializeSpecialistAgents();
    await this.initializePatternAgents();
    
    this.logger.info('ADK Hivemind Controller initialized');
  }
  
  /**
   * Shutdown controller
   */
  public async shutdown(): Promise<void> {
    // Clean up agents
    for (const agent of this.agents.values()) {
      if ('shutdown' in agent) {
        await (agent as any).shutdown();
      }
    }
    
    for (const agent of this.patternAgents.values()) {
      if ('shutdown' in agent) {
        await (agent as any).shutdown();
      }
    }
    
    this.logger.info('ADK Hivemind Controller shut down');
  }
}