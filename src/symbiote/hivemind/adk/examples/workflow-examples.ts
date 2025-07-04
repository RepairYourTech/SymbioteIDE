/**
 * ADK Hivemind Workflow Examples
 * 
 * Demonstrates various multi-agent patterns using Google ADK
 */

import { 
  HivemindTask, 
  TaskPriority, 
  AgentSpecialty,
  TaskStatus 
} from '../../types';
import { ADKHivemindController } from '../hivemind-adk-controller';
import { MemoryManager } from '../../../memory/mem0/memory-manager';

/**
 * Example 1: Sequential Pipeline for Feature Development
 * 
 * Frontend → Backend → Testing → Documentation
 */
export async function sequentialFeatureDevelopment(
  controller: ADKHivemindController
): Promise<void> {
  const task: HivemindTask = {
    id: 'feature_001',
    type: 'feature_development',
    title: 'Implement User Profile Feature',
    description: 'Create a complete user profile feature with UI, API, and tests',
    requiredSpecialties: [
      AgentSpecialty.Frontend,
      AgentSpecialty.Backend,
      AgentSpecialty.Testing
    ],
    priority: TaskPriority.High,
    status: TaskStatus.Pending,
    dependencies: [],
    assignedAgents: [],
    estimatedComplexity: 8,
    context: {
      files: [
        'src/components/UserProfile.tsx',
        'src/api/users.ts',
        'src/tests/userProfile.test.ts'
      ],
      requirements: `
        1. Create React component for user profile display/edit
        2. Implement REST API endpoints for profile CRUD
        3. Add authentication and authorization
        4. Write comprehensive tests
        5. Document API endpoints
      `
    },
    createdAt: new Date()
  };
  
  console.log('\n🚀 Starting Sequential Feature Development');
  console.log('Expected flow: Frontend → Backend → Testing → Documentation\n');
  
  const result = await controller.executeTask(task);
  
  console.log('\n✅ Sequential Pipeline Complete');
  console.log(`Success: ${result.success}`);
  console.log(`Files created: ${result.filesCreated.length}`);
  console.log(`Tests added: ${result.testsAdded}`);
}

/**
 * Example 2: Parallel Execution for Code Review
 * 
 * Security + Performance + Code Quality agents run in parallel
 */
export async function parallelCodeReview(
  controller: ADKHivemindController
): Promise<void> {
  const task: HivemindTask = {
    id: 'review_001',
    type: 'code_review',
    title: 'Comprehensive Code Review',
    description: 'Review recent changes for security, performance, and quality',
    requiredSpecialties: [
      AgentSpecialty.Security,
      AgentSpecialty.Performance,
      AgentSpecialty.General
    ],
    priority: TaskPriority.High,
    status: TaskStatus.Pending,
    dependencies: [],
    assignedAgents: [],
    estimatedComplexity: 6,
    context: {
      files: [
        'src/auth/authentication.ts',
        'src/api/dataProcessing.ts',
        'src/utils/helpers.ts'
      ],
      codeContext: 'Recent PR #123 changes'
    },
    createdAt: new Date()
  };
  
  console.log('\n🚀 Starting Parallel Code Review');
  console.log('Security, Performance, and Quality checks run simultaneously\n');
  
  const result = await controller.executeTask(task);
  
  console.log('\n✅ Parallel Review Complete');
  console.log(`Issues found: ${result.issuesFound.length}`);
  result.issuesFound.forEach(issue => console.log(`  - ${issue}`));
}

/**
 * Example 3: Hierarchical Task Decomposition
 * 
 * Complex refactoring broken into subtasks
 */
export async function hierarchicalRefactoring(
  controller: ADKHivemindController
): Promise<void> {
  const task: HivemindTask = {
    id: 'refactor_001',
    type: 'architecture_refactor',
    title: 'Refactor Monolith to Microservices',
    description: 'Break down monolithic application into microservices',
    requiredSpecialties: [
      AgentSpecialty.Backend,
      AgentSpecialty.DevOps,
      AgentSpecialty.Testing
    ],
    priority: TaskPriority.High,
    status: TaskStatus.Pending,
    dependencies: [],
    assignedAgents: [],
    estimatedComplexity: 10, // High complexity triggers decomposition
    context: {
      projectStructure: `
        /src
          /api (monolithic API)
          /services (business logic)
          /database (data layer)
      `,
      targetArchitecture: `
        - User Service
        - Product Service
        - Order Service
        - Notification Service
      `
    },
    createdAt: new Date()
  };
  
  console.log('\n🚀 Starting Hierarchical Refactoring');
  console.log('Complex task will be decomposed into manageable subtasks\n');
  
  const result = await controller.executeTask(task);
  
  console.log('\n✅ Hierarchical Execution Complete');
  console.log(`Task tree depth: ${result.output.levels || 'unknown'}`);
  console.log(`Total subtasks: ${result.output.totalTasks || 'unknown'}`);
}

/**
 * Example 4: Loop Refinement for Quality
 * 
 * Iterative improvement until quality criteria met
 */
export async function loopRefinementOptimization(
  controller: ADKHivemindController
): Promise<void> {
  const task: HivemindTask = {
    id: 'optimize_001',
    type: 'performance_optimization',
    title: 'Optimize Database Query Performance',
    description: 'Improve query performance to meet < 100ms requirement',
    requiredSpecialties: [
      AgentSpecialty.Backend,
      AgentSpecialty.Performance
    ],
    priority: TaskPriority.High,
    status: TaskStatus.Pending,
    dependencies: [],
    assignedAgents: [],
    estimatedComplexity: 7,
    context: {
      files: ['src/database/queries.ts'],
      performanceTarget: {
        currentLatency: '500ms',
        targetLatency: '100ms',
        criticalQueries: [
          'getUserData',
          'getProductList',
          'processOrder'
        ]
      }
    },
    createdAt: new Date()
  };
  
  console.log('\n🚀 Starting Loop Refinement Optimization');
  console.log('Will iterate until performance target is met\n');
  
  const result = await controller.executeTask(task);
  
  console.log('\n✅ Loop Refinement Complete');
  console.log(`Iterations: ${result.output.iterations || 'unknown'}`);
  console.log(`Final score: ${result.output.bestScore || 'unknown'}%`);
}

/**
 * Example 5: Human-in-the-Loop for Critical Decisions
 * 
 * Deployment with human approval gates
 */
export async function humanInLoopDeployment(
  controller: ADKHivemindController
): Promise<void> {
  const task: HivemindTask = {
    id: 'deploy_001',
    type: 'deployment',
    title: 'Production Deployment v2.0',
    description: 'Deploy new version to production with safety checks',
    requiredSpecialties: [
      AgentSpecialty.DevOps,
      AgentSpecialty.Testing
    ],
    priority: TaskPriority.Critical,
    status: TaskStatus.Pending,
    dependencies: [],
    assignedAgents: [],
    estimatedComplexity: 9,
    context: {
      requiresApproval: true, // Triggers human interaction
      deploymentSteps: [
        'Run final test suite',
        'Build production artifacts',
        'Deploy to staging',
        'Run smoke tests',
        'Deploy to production (requires approval)',
        'Monitor metrics'
      ],
      rollbackPlan: 'Automated rollback on error metrics'
    },
    createdAt: new Date()
  };
  
  console.log('\n🚀 Starting Human-in-the-Loop Deployment');
  console.log('Human approval required at critical steps\n');
  
  const result = await controller.executeTask(task);
  
  console.log('\n✅ Deployment Complete');
  console.log(`Human interactions: ${result.output.humanInteractions ? Object.keys(result.output.humanInteractions).length : 0}`);
}

/**
 * Example 6: Complex Multi-Pattern Workflow
 * 
 * Combines multiple patterns for a complex task
 */
export async function multiPatternMigration(
  controller: ADKHivemindController
): Promise<void> {
  const task: HivemindTask = {
    id: 'migration_001',
    type: 'database_migration',
    title: 'Migrate from PostgreSQL to MongoDB',
    description: 'Complete database migration with zero downtime',
    requiredSpecialties: [
      AgentSpecialty.Backend,
      AgentSpecialty.DevOps,
      AgentSpecialty.Testing,
      AgentSpecialty.Performance
    ],
    priority: TaskPriority.Critical,
    status: TaskStatus.Pending,
    dependencies: [],
    assignedAgents: [],
    estimatedComplexity: 10,
    context: {
      requiresApproval: true,
      migrationStrategy: `
        1. Hierarchical: Decompose into schema, data, and code migration
        2. Pipeline: Execute migration steps sequentially
        3. Parallel: Run data migration in parallel batches
        4. Loop: Verify data integrity iteratively
        5. Human: Approve before final cutover
      `,
      constraints: {
        zeroDowntime: true,
        dataIntegrity: 'critical',
        rollbackRequired: true
      }
    },
    createdAt: new Date()
  };
  
  console.log('\n🚀 Starting Multi-Pattern Migration');
  console.log('Combines hierarchical, pipeline, parallel, loop, and human patterns\n');
  
  const result = await controller.executeTask(task);
  
  console.log('\n✅ Migration Complete');
  console.log(`Success: ${result.success}`);
  console.log(`Pattern used: ${result.output.pattern || 'multiple'}`);
}

/**
 * Run all examples
 */
export async function runAllExamples(
  controller: ADKHivemindController
): Promise<void> {
  console.log('=== ADK Hivemind Workflow Examples ===\n');
  
  const examples = [
    { name: 'Sequential Feature Development', fn: sequentialFeatureDevelopment },
    { name: 'Parallel Code Review', fn: parallelCodeReview },
    { name: 'Hierarchical Refactoring', fn: hierarchicalRefactoring },
    { name: 'Loop Refinement Optimization', fn: loopRefinementOptimization },
    { name: 'Human-in-the-Loop Deployment', fn: humanInLoopDeployment },
    { name: 'Multi-Pattern Migration', fn: multiPatternMigration }
  ];
  
  for (const example of examples) {
    console.log(`\n${'='.repeat(50)}`);
    console.log(`Running: ${example.name}`);
    console.log('='.repeat(50));
    
    try {
      await example.fn(controller);
    } catch (error) {
      console.error(`Failed: ${error.message}`);
    }
    
    // Brief pause between examples
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
  
  console.log('\n=== All Examples Complete ===');
  
  // Show final metrics
  const metrics = controller.getMetrics();
  console.log('\nController Metrics:');
  console.log(`Total tasks: ${metrics.tasksProcessed}`);
  console.log(`Success rate: ${(metrics.successRate * 100).toFixed(1)}%`);
  console.log(`Pattern usage:`);
  Object.entries(metrics.patternUsage).forEach(([pattern, count]) => {
    console.log(`  - ${pattern}: ${count}`);
  });
}

/**
 * Example VS Code command to run workflows
 */
export async function createWorkflowCommands(): Promise<void> {
  // This would be registered in extension.ts
  const commands = [
    {
      command: 'hivemind.example.sequential',
      title: 'Hivemind: Run Sequential Pipeline Example',
      handler: async (controller: ADKHivemindController) => {
        await sequentialFeatureDevelopment(controller);
      }
    },
    {
      command: 'hivemind.example.parallel',
      title: 'Hivemind: Run Parallel Execution Example',
      handler: async (controller: ADKHivemindController) => {
        await parallelCodeReview(controller);
      }
    },
    {
      command: 'hivemind.example.hierarchical',
      title: 'Hivemind: Run Hierarchical Decomposition Example',
      handler: async (controller: ADKHivemindController) => {
        await hierarchicalRefactoring(controller);
      }
    },
    {
      command: 'hivemind.example.all',
      title: 'Hivemind: Run All ADK Examples',
      handler: async (controller: ADKHivemindController) => {
        await runAllExamples(controller);
      }
    }
  ];
  
  console.log('Workflow commands ready for registration');
}