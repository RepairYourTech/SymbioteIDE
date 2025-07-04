/**
 * Intelligent Handoff Example
 * 
 * Demonstrates intelligent context management with Google AI ADK agents
 */

import { TaskManager } from '../task-manager';
import { 
  TaskType,
  TaskPriority,
  HandoffType,
  ContextScope
} from '../types';
import { getADKOrchestrator } from '../../adk/orchestrator';
import { Logger } from '../../utils/logger';

const logger = new Logger('IntelligentHandoffExample');

async function runExample() {
  // Initialize task manager with intelligent handoffs
  const taskManager = new TaskManager({
    redis: true,
    neo4j: true,
    useIntelligentHandoffs: true
  });
  
  await taskManager.initialize();
  
  // Initialize ADK orchestrator
  const adkOrchestrator = getADKOrchestrator();
  await adkOrchestrator.initialize();
  
  // Example: Complex research task that requires multiple model handoffs
  await demonstrateComplexHandoff(taskManager);
  
  // Example: Large context optimization
  await demonstrateLargeContextOptimization(taskManager);
  
  // Example: Multi-agent workflow with context bubbles
  await demonstrateMultiAgentWorkflow(taskManager);
}

/**
 * Demonstrate complex handoff between models with different context sizes
 */
async function demonstrateComplexHandoff(taskManager: TaskManager) {
  logger.info('=== Complex Handoff Example ===');
  
  // Create a research task that starts with Gemini 1.5 Pro (1M context)
  const researchTask = await taskManager.createTask({
    title: 'Analyze Large Codebase',
    description: 'Analyze entire codebase structure and patterns',
    type: TaskType.Research,
    priority: TaskPriority.High,
    contextScope: ContextScope.Shared,
    contextData: {
      codebase: {
        // Simulate large codebase data (would be actual code in practice)
        files: Array(1000).fill(null).map((_, i) => ({
          path: `/src/module${i}/file${i}.ts`,
          content: `// Large file content ${i}...`.repeat(100),
          ast: { /* Large AST data */ },
          dependencies: [ /* Dependencies */ ]
        })),
        totalSize: '500MB',
        language: 'TypeScript'
      }
    },
    estimatedDuration: 3600000, // 1 hour
    createdBy: 'system'
  });
  
  // Create summarization task with GPT-4 (32k context)
  const summarizeTask = await taskManager.createTask({
    title: 'Summarize Analysis Results',
    description: 'Create concise summary of codebase analysis',
    type: TaskType.Task,
    priority: TaskPriority.High,
    parentId: researchTask.id,
    dependencies: [{
      taskId: researchTask.id,
      type: DependencyType.Blocking,
      required: true
    }],
    contextScope: ContextScope.Inherited,
    createdBy: 'system'
  });
  
  // Create documentation task with Claude 3 Haiku (200k context)
  const documentTask = await taskManager.createTask({
    title: 'Generate Documentation',
    description: 'Create comprehensive documentation from analysis',
    type: TaskType.Documentation,
    priority: TaskPriority.Medium,
    parentId: researchTask.id,
    dependencies: [{
      taskId: summarizeTask.id,
      type: DependencyType.Blocking,
      required: true
    }],
    contextScope: ContextScope.Inherited,
    createdBy: 'system'
  });
  
  // Create handoff from Gemini to GPT-4
  const handoff1 = await taskManager.createHandoff({
    type: HandoffType.AgentToAgent,
    fromTaskId: researchTask.id,
    toTaskId: summarizeTask.id,
    fromAgentId: 'gemini-analyzer',
    toAgentId: 'gpt4-summarizer',
    fromModelId: 'gemini-1.5-pro',
    toModelId: 'gpt-4-32k',
    data: {
      analysis: 'Large analysis results from codebase scan...',
      patterns: ['Pattern 1', 'Pattern 2', 'Pattern 3'],
      metrics: { files: 1000, lines: 500000, complexity: 'high' }
    },
    preservePriority: ['patterns', 'metrics', 'key_findings']
  });
  
  logger.info(`Created handoff 1: ${handoff1.id}`);
  logger.info(`Original tokens: ${handoff1.metadata?.originalTokenCount}`);
  logger.info(`Optimized tokens: ${handoff1.metadata?.optimizedTokenCount}`);
  
  // Create handoff from GPT-4 to Claude
  const handoff2 = await taskManager.createHandoff({
    type: HandoffType.AgentToAgent,
    fromTaskId: summarizeTask.id,
    toTaskId: documentTask.id,
    fromAgentId: 'gpt4-summarizer',
    toAgentId: 'claude-documenter',
    fromModelId: 'gpt-4-32k',
    toModelId: 'claude-3-haiku',
    data: {
      summary: 'Concise summary of findings...',
      recommendations: ['Rec 1', 'Rec 2', 'Rec 3'],
      actionItems: ['Action 1', 'Action 2']
    },
    preservePriority: ['summary', 'actionItems']
  });
  
  logger.info(`Created handoff 2: ${handoff2.id}`);
  
  // Complete handoffs
  await taskManager.completeHandoff(handoff1.id, {
    success: true,
    optimizationStrategies: ['summarization', 'selective_retention']
  });
  
  await taskManager.completeHandoff(handoff2.id, {
    success: true,
    optimizationStrategies: ['progressive_reduction']
  });
}

/**
 * Demonstrate large context optimization
 */
async function demonstrateLargeContextOptimization(taskManager: TaskManager) {
  logger.info('=== Large Context Optimization Example ===');
  
  // Simulate a task with embeddings that need optimization
  const embeddingTask = await taskManager.createTask({
    title: 'Process Large Embeddings',
    description: 'Process and analyze large embedding dataset',
    type: TaskType.Task,
    priority: TaskPriority.High,
    contextData: {
      embeddings: Array(10000).fill(null).map(() => 
        Array(1536).fill(null).map(() => Math.random())
      ),
      metadata: {
        model: 'text-embedding-ada-002',
        dimensions: 1536,
        count: 10000
      }
    },
    createdBy: 'system'
  });
  
  // Create analysis task with smaller model
  const analysisTask = await taskManager.createTask({
    title: 'Analyze Embedding Patterns',
    description: 'Find patterns in embeddings',
    type: TaskType.Task,
    priority: TaskPriority.Medium,
    dependencies: [{
      taskId: embeddingTask.id,
      type: DependencyType.Blocking,
      required: true
    }],
    createdBy: 'system'
  });
  
  // Create handoff with embedding optimization
  const handoff = await taskManager.createHandoff({
    type: HandoffType.ToolToAgent,
    fromTaskId: embeddingTask.id,
    toTaskId: analysisTask.id,
    fromModelId: 'embedding-processor',
    toModelId: 'gpt-3.5-turbo',
    data: {
      embeddings: taskManager.getTask(embeddingTask.id)?.context.data.embeddings,
      metadata: taskManager.getTask(embeddingTask.id)?.context.data.metadata
    },
    preservePriority: ['metadata', 'summary_statistics']
  });
  
  logger.info(`Embedding handoff created: ${handoff.id}`);
  logger.info(`Compression ratio: ${handoff.metadata?.compressionRatio}`);
}

/**
 * Demonstrate multi-agent workflow with context bubbles
 */
async function demonstrateMultiAgentWorkflow(taskManager: TaskManager) {
  logger.info('=== Multi-Agent Workflow Example ===');
  
  // Create main task
  const mainTask = await taskManager.createTask({
    title: 'Build Feature: User Analytics Dashboard',
    description: 'Implement complete user analytics dashboard',
    type: TaskType.Epic,
    priority: TaskPriority.High,
    createdBy: 'system'
  });
  
  // Create subtasks for different agents
  const designTask = await taskManager.createTask({
    title: 'Design Dashboard UI',
    type: TaskType.Task,
    parentId: mainTask.id,
    assignee: {
      type: AssigneeType.Agent,
      id: 'design-agent',
      name: 'UI Design Agent'
    },
    createdBy: 'system'
  });
  
  const backendTask = await taskManager.createTask({
    title: 'Implement Analytics API',
    type: TaskType.Task,
    parentId: mainTask.id,
    assignee: {
      type: AssigneeType.Agent,
      id: 'backend-agent',
      name: 'Backend Development Agent'
    },
    createdBy: 'system'
  });
  
  const frontendTask = await taskManager.createTask({
    title: 'Build Dashboard Components',
    type: TaskType.Task,
    parentId: mainTask.id,
    assignee: {
      type: AssigneeType.Agent,
      id: 'frontend-agent',
      name: 'Frontend Development Agent'
    },
    dependencies: [{
      taskId: designTask.id,
      type: DependencyType.Blocking,
      required: true
    }],
    createdBy: 'system'
  });
  
  // Create context bubble for shared design system
  const designBubble = await taskManager.createContextBubble({
    name: 'Design System Context',
    taskIds: [designTask.id, frontendTask.id],
    isolation: BubbleIsolation.Partial,
    sharedData: {
      colors: {
        primary: '#007bff',
        secondary: '#6c757d',
        success: '#28a745'
      },
      components: {
        button: { borderRadius: '4px', padding: '8px 16px' },
        card: { boxShadow: '0 2px 4px rgba(0,0,0,0.1)' }
      },
      typography: {
        heading: { fontFamily: 'Inter', fontWeight: 600 },
        body: { fontFamily: 'Inter', fontWeight: 400 }
      }
    },
    permissions: {
      read: [designTask.id, frontendTask.id, backendTask.id],
      write: [designTask.id],
      execute: []
    }
  });
  
  logger.info(`Created design bubble: ${designBubble.id}`);
  
  // Create API contract bubble
  const apiBubble = await taskManager.createContextBubble({
    name: 'API Contract Context',
    taskIds: [backendTask.id, frontendTask.id],
    isolation: BubbleIsolation.Partial,
    sharedData: {
      endpoints: {
        analytics: {
          getUserStats: 'GET /api/analytics/users/:id/stats',
          getTeamStats: 'GET /api/analytics/teams/:id/stats',
          getProjectStats: 'GET /api/analytics/projects/:id/stats'
        }
      },
      schemas: {
        UserStats: {
          totalActions: 'number',
          lastActive: 'Date',
          engagement: 'number'
        }
      }
    }
  });
  
  logger.info(`Created API bubble: ${apiBubble.id}`);
  
  // Create handoffs between tasks
  const designToFrontendHandoff = await taskManager.createHandoff({
    type: HandoffType.AgentToAgent,
    fromTaskId: designTask.id,
    toTaskId: frontendTask.id,
    fromAgentId: 'design-agent',
    toAgentId: 'frontend-agent',
    fromModelId: 'gpt-4-vision',
    toModelId: 'claude-3-sonnet',
    data: {
      designs: {
        dashboard: 'Dashboard layout specifications...',
        components: ['Chart', 'Table', 'Filter', 'DatePicker'],
        mockups: ['mockup1.png', 'mockup2.png']
      },
      requirements: {
        responsive: true,
        accessibility: 'WCAG 2.1 AA',
        performance: 'LCP < 2.5s'
      }
    },
    preservePriority: ['components', 'requirements']
  });
  
  logger.info(`Design to Frontend handoff: ${designToFrontendHandoff.id}`);
  
  // Get analytics
  const analytics = await taskManager.getAnalytics();
  logger.info('Workflow analytics:', analytics);
}

// Import additional types
import { DependencyType, AssigneeType, BubbleIsolation } from '../types';

// Run the example
if (require.main === module) {
  runExample()
    .then(() => logger.info('Example completed successfully'))
    .catch(error => logger.error('Example failed:', error))
    .finally(() => process.exit(0));
}

export { runExample };