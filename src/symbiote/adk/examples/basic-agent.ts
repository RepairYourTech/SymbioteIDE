/**
 * Basic ADK Agent Example
 * 
 * Demonstrates creating and using an ADK agent with MCP tools and A2A protocol
 */

import { 
  getADKOrchestrator,
  ADKAgentType,
  AgentFactory,
  getOrchestrationIntegration
} from '../index';
import { ModelProfile, ProviderType, Capability } from '../../orchestration/interfaces';

async function createBasicAgent() {
  // Initialize orchestrator
  const orchestrator = getADKOrchestrator({
    mcpEnabled: true,
    cacheResults: true,
    redis: true
  });
  
  await orchestrator.initialize();

  // Define model profile
  const modelProfile: ModelProfile = {
    id: 'claude-3-sonnet',
    name: 'Claude 3 Sonnet',
    provider: ProviderType.Anthropic,
    displayName: 'Claude 3 Sonnet',
    capabilities: [
      Capability.CodeGeneration,
      Capability.NaturalLanguage,
      Capability.FunctionCalling
    ],
    contextWindow: 200000,
    maxOutputTokens: 4096,
    costPerToken: {
      input: 0.00001,
      output: 0.00003,
      currency: 'USD'
    },
    averageLatency: 2000,
    reliability: 0.95,
    specializations: ['code-generation', 'natural-language'],
    rateLimit: {
      requestsPerMinute: 60,
      tokensPerMinute: 100000
    },
    availability: {
      status: 'available' as const
    }
  };

  // Create agent factory
  const agentFactory = new AgentFactory();
  
  // Create agent using builder pattern
  const agentConfig = agentFactory.createBuilder()
    .withId('example-agent-1')
    .withName('Example Assistant')
    .withType(ADKAgentType.LLM)
    .withModel(modelProfile)
    .withSystemPrompt('You are a helpful AI assistant integrated with SymbioteIDE.')
    .withMemory({
      enabled: true,
      type: 'episodic',
      provider: 'mem0',
      scope: 'agent'
    })
    .withA2A({
      enabled: true,
      agentCard: {
        name: 'Example Assistant',
        description: 'A helpful AI assistant for coding tasks',
        version: '1.0.0',
        capabilities: ['text-generation', 'code-analysis', 'task-planning']
      },
      endpoints: {
        discovery: '/agents/example-assistant',
        task: '/agents/example-assistant/task',
        message: '/agents/example-assistant/message',
        artifact: '/agents/example-assistant/artifact'
      }
    })
    .build();

  // Create the agent
  const agentId = await orchestrator.createAgent(agentConfig);
  console.log(`Created agent: ${agentId}`);

  return { orchestrator, agentId };
}

async function executeSimpleTask() {
  const { orchestrator, agentId } = await createBasicAgent();

  // Execute a simple task
  const result = await orchestrator.executeAgent({
    agentId,
    input: {
      messages: [{
        role: 'user',
        content: 'What are the best practices for TypeScript development?'
      }]
    },
    context: {
      projectType: 'typescript',
      ide: 'symbioteide'
    }
  });

  console.log('Execution result:', result);
  
  // Shutdown
  await orchestrator.shutdown();
}

async function createWorkflowExample() {
  const orchestrator = getADKOrchestrator();
  await orchestrator.initialize();

  // Create a workflow with multiple steps
  const workflowId = await orchestrator.createWorkflow(
    'code-review-workflow',
    [
      {
        id: 'analyzer',
        name: 'Code Analyzer',
        type: ADKAgentType.LLM,
        description: 'Analyzes code for issues',
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: false,
          mcpTools: true
        },
        systemPrompt: 'Analyze the provided code for potential issues, bugs, and improvements.'
      },
      {
        id: 'reviewer',
        name: 'Code Reviewer',
        type: ADKAgentType.LLM,
        description: 'Provides detailed code review',
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: false,
          mcpTools: true
        },
        systemPrompt: 'Review the code analysis and provide constructive feedback.'
      },
      {
        id: 'suggester',
        name: 'Improvement Suggester',
        type: ADKAgentType.LLM,
        description: 'Suggests code improvements',
        capabilities: {
          streaming: false,
          multiModal: false,
          toolUse: true,
          memoryAccess: true,
          a2aProtocol: false,
          mcpTools: true
        },
        systemPrompt: 'Based on the review, suggest specific code improvements and refactoring.'
      }
    ],
    {
      description: 'Automated code review workflow',
      sequential: true
    }
  );

  console.log(`Created workflow: ${workflowId}`);
  
  // Execute workflow
  const result = await orchestrator.executeAgent({
    agentId: workflowId,
    input: {
      code: `
        function calculateTotal(items) {
          let total = 0;
          for (let i = 0; i < items.length; i++) {
            total = total + items[i].price * items[i].quantity;
          }
          return total;
        }
      `
    }
  });

  console.log('Workflow result:', result);
  
  await orchestrator.shutdown();
}

async function a2aInteractionExample() {
  const { orchestrator, agentId: agent1Id } = await createBasicAgent();
  
  // Create second agent
  const agentFactory = new AgentFactory();
  const agent2Config = agentFactory.createBuilder()
    .withId('example-agent-2')
    .withName('Code Expert')
    .withType(ADKAgentType.LLM)
    .withSystemPrompt('You are a code expert that helps with advanced programming concepts.')
    .withA2A({
      enabled: true,
      agentCard: {
        name: 'Code Expert',
        description: 'Expert in advanced programming concepts',
        version: '1.0.0',
        capabilities: ['code-expertise', 'architecture-design', 'performance-optimization']
      },
      endpoints: {
        discovery: '/agents/code-expert',
        task: '/agents/code-expert/task',
        message: '/agents/code-expert/message',
        artifact: '/agents/code-expert/artifact'
      }
    })
    .build();
  
  const agent2Id = await orchestrator.createAgent(agent2Config);
  
  // Agent 1 sends task to Agent 2
  const task = await orchestrator.executeA2ATask(
    agent1Id,
    'http://localhost:3000/agents/code-expert',
    {
      type: 'code-optimization',
      input: {
        code: 'function fibonacci(n) { return n <= 1 ? n : fibonacci(n-1) + fibonacci(n-2); }',
        requirements: 'Optimize this recursive fibonacci function'
      }
    }
  );
  
  console.log('A2A task created:', task);
  
  await orchestrator.shutdown();
}

async function orchestrationIntegrationExample() {
  const integration = getOrchestrationIntegration();
  await integration.initialize();
  
  // Create agent from orchestration model
  const agentId = await integration.createAgentFromModel('gpt-4', {
    name: 'GPT-4 Assistant',
    systemPrompt: 'You are a helpful coding assistant.',
    memory: {
      enabled: true,
      type: 'semantic',
      provider: 'mem0',
      scope: 'project'
    }
  });
  
  // Execute via orchestration interface
  const response = await integration.executeViaOrchestration(agentId, {
    model: 'gpt-4',
    messages: [{
      role: 'user',
      content: 'Explain the SOLID principles in software design'
    }],
    stream: false
  });
  
  console.log('Response:', response);
  
  await integration.shutdown();
}

// Export examples
export {
  createBasicAgent,
  executeSimpleTask,
  createWorkflowExample,
  a2aInteractionExample,
  orchestrationIntegrationExample
};

// Run example if executed directly
if (require.main === module) {
  (async () => {
    console.log('Running ADK examples...\n');
    
    console.log('1. Basic Agent Example:');
    await executeSimpleTask();
    
    console.log('\n2. Workflow Example:');
    await createWorkflowExample();
    
    console.log('\n3. A2A Interaction Example:');
    await a2aInteractionExample();
    
    console.log('\n4. Orchestration Integration Example:');
    await orchestrationIntegrationExample();
  })().catch(console.error);
}