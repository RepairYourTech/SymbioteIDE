/**
 * Task Analyzer Unit Tests
 */

import { TestRunner, expect, createMockFunction } from '../utils/test-framework';
import { TaskAnalyzer } from '../../task-analyzer';
import { AITask } from '../../types';

const runner = new TestRunner();

runner.describe('TaskAnalyzer', () => {
  let analyzer: TaskAnalyzer;
  
  runner.beforeEach(() => {
    analyzer = new TaskAnalyzer();
  });
  
  runner.it('should analyze simple chat task', async () => {
    const task: AITask = {
      id: 'task-1',
      type: 'chat',
      prompt: 'Hello, how are you?',
      context: {}
    };
    
    const analysis = await analyzer.analyzeTask(task);
    
    expect(analysis.complexity).toBe('simple');
    expect(analysis.estimatedTokens).toBeGreaterThan(0);
    expect(analysis.estimatedTokens).toBeLessThan(100);
    expect(analysis.requiredCapabilities).toContain('chat');
    expect(analysis.suggestedModelTier).toBe('basic');
  });
  
  runner.it('should analyze complex code generation task', async () => {
    const task: AITask = {
      id: 'task-2',
      type: 'code_generation',
      prompt: 'Create a complete REST API with authentication, database models, and testing suite for a blog application',
      context: {
        files: ['schema.sql', 'requirements.txt'],
        variables: { framework: 'express', database: 'postgresql' }
      }
    };
    
    const analysis = await analyzer.analyzeTask(task);
    
    expect(analysis.complexity).toBe('complex');
    expect(analysis.estimatedTokens).toBeGreaterThan(1000);
    expect(analysis.requiredCapabilities).toContain('code');
    expect(analysis.suggestedModelTier).toBe('advanced');
  });
  
  runner.it('should detect code-related keywords', async () => {
    const task: AITask = {
      id: 'task-3',
      type: 'chat',
      prompt: 'Explain this Python function and suggest improvements',
      context: {
        files: ['main.py']
      }
    };
    
    const analysis = await analyzer.analyzeTask(task);
    
    expect(analysis.requiredCapabilities).toContain('code');
    expect(analysis.detectedPatterns).toContain('code_analysis');
  });
  
  runner.it('should analyze task with history context', async () => {
    const task: AITask = {
      id: 'task-4',
      type: 'chat',
      prompt: 'Continue the discussion',
      context: {
        history: [
          { role: 'user', content: 'Tell me about quantum computing' },
          { role: 'assistant', content: 'Quantum computing is...' },
          { role: 'user', content: 'How does it differ from classical computing?' },
          { role: 'assistant', content: 'The main differences are...' }
        ]
      }
    };
    
    const analysis = await analyzer.analyzeTask(task);
    
    expect(analysis.contextSize).toBeGreaterThan(100);
    expect(analysis.suggestedModelTier).toBe('standard');
  });
  
  runner.it('should detect function calling requirements', async () => {
    const task: AITask = {
      id: 'task-5',
      type: 'function_calling',
      prompt: 'Get the current weather in San Francisco',
      context: {},
      constraints: {
        requiredCapabilities: ['function']
      }
    };
    
    const analysis = await analyzer.analyzeTask(task);
    
    expect(analysis.requiredCapabilities).toContain('function');
    expect(analysis.detectedPatterns).toContain('function_call');
  });
  
  runner.it('should calculate token estimates accurately', async () => {
    const task: AITask = {
      id: 'task-6',
      type: 'chat',
      prompt: 'This is a test prompt with exactly known length for testing token estimation',
      context: {
        files: ['file1.txt', 'file2.txt'],
        variables: {
          var1: 'value1',
          var2: 'value2'
        }
      }
    };
    
    const analysis = await analyzer.analyzeTask(task);
    
    // Rough estimate: ~4 characters per token
    const expectedTokens = Math.ceil(
      (task.prompt.length + JSON.stringify(task.context).length) / 4
    );
    
    expect(Math.abs(analysis.estimatedTokens - expectedTokens)).toBeLessThan(50);
  });
  
  runner.it('should suggest model tier based on complexity', async () => {
    const tasks: Array<[AITask, string]> = [
      [
        { id: '1', type: 'chat', prompt: 'Hi', context: {} },
        'basic'
      ],
      [
        { 
          id: '2', 
          type: 'chat', 
          prompt: 'Explain quantum mechanics in detail',
          context: { variables: { depth: 'advanced' } }
        },
        'standard'
      ],
      [
        {
          id: '3',
          type: 'code_generation',
          prompt: 'Build a distributed microservices architecture',
          context: { files: Array(10).fill('file.txt') }
        },
        'advanced'
      ]
    ];
    
    for (const [task, expectedTier] of tasks) {
      const analysis = await analyzer.analyzeTask(task);
      expect(analysis.suggestedModelTier).toBe(expectedTier);
    }
  });
  
  runner.it('should detect urgency indicators', async () => {
    const urgentTask: AITask = {
      id: 'urgent-1',
      type: 'chat',
      prompt: 'URGENT: Fix the production bug immediately',
      context: {},
      metadata: {
        priority: 'high'
      }
    };
    
    const analysis = await analyzer.analyzeTask(urgentTask);
    
    expect(analysis.urgency).toBe('high');
    expect(analysis.detectedPatterns).toContain('urgent');
  });
  
  runner.it('should handle edge cases gracefully', async () => {
    const edgeCases: AITask[] = [
      { id: '1', type: 'chat', prompt: '', context: {} },
      { id: '2', type: 'unknown' as any, prompt: 'test', context: {} },
      { id: '3', type: 'chat', prompt: 'test', context: null as any }
    ];
    
    for (const task of edgeCases) {
      const analysis = await analyzer.analyzeTask(task);
      expect(analysis).toBeTruthy();
      expect(analysis.complexity).toBeTruthy();
      expect(analysis.estimatedTokens).toBeGreaterThan(0);
    }
  });
});

export default runner;