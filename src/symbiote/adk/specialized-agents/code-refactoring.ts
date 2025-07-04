/**
 * Code Refactoring Agent
 * 
 * Specializes in code refactoring and modernization
 */

import { ADKAgentConfig, ADKAgentType } from '../types';
import { ProviderType, Capability } from '../../orchestration/interfaces';

export class CodeRefactoringAgent {
  static getConfig(): ADKAgentConfig {
    return {
      id: 'code-refactoring',
      name: 'Code Refactoring Agent',
      type: ADKAgentType.LLM,
      description: 'Specializes in code refactoring and modernization',
      systemPrompt: `You are a code refactoring expert focused on improving code quality and maintainability. Your expertise includes:

1. Refactoring Patterns
   - Extract method/function
   - Extract variable
   - Inline method/variable
   - Move method/field
   - Pull up/Push down
   - Extract interface
   - Replace conditional with polymorphism
   
2. Code Smells Detection
   - Long methods
   - Large classes
   - Duplicate code
   - Dead code
   - Feature envy
   - Data clumps
   - Primitive obsession
   
3. Design Patterns
   - Creational patterns (Factory, Builder, Singleton)
   - Structural patterns (Adapter, Decorator, Facade)
   - Behavioral patterns (Observer, Strategy, Command)
   - Functional patterns
   
4. Modern Language Features
   - ES6+ JavaScript features
   - TypeScript migrations
   - Async/await patterns
   - Functional programming
   - Type safety improvements
   
5. Code Quality
   - Readability improvements
   - Naming conventions
   - Code organization
   - Dependency management
   - Test coverage

When refactoring:
- Preserve functionality
- Improve one aspect at a time
- Ensure tests pass
- Document changes
- Consider performance impact`,
      model: {
        id: 'claude-3-opus',
        name: 'Claude 3 Opus',
        displayName: 'Claude 3 Opus',
        provider: ProviderType.Anthropic,
        contextWindow: 200000,
        maxOutputTokens: 4096,
        costPerToken: {
          input: 0.000015,
          output: 0.000075,
          currency: 'USD'
        },
        capabilities: [Capability.LongContext, Capability.CodeGeneration, Capability.CodeAnalysis],
        supportsBatch: true,
        supportsStreaming: true,
        supportsTools: true,
        supportsVision: false,
        lastUpdated: new Date('2024-03-04')
      },
      temperature: 0.2,
      capabilities: {
        streaming: true,
        multiModal: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true
      },
      tools: [
        {
          name: 'analyze_code_smells',
          description: 'Detect code smells and refactoring opportunities',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { code: string; language: string }) => {
              return {
                codeSmells: [
                  {
                    type: 'Long Method',
                    location: 'line 25-150',
                    severity: 'High',
                    suggestion: 'Extract into smaller methods'
                  }
                ],
                refactoringOpportunities: 5
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                code: { type: 'string' },
                language: { type: 'string' }
              },
              required: ['code', 'language']
            }
          }
        },
        {
          name: 'apply_refactoring',
          description: 'Apply specific refactoring pattern to code',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { code: string; pattern: string; target: string }) => {
              return {
                refactoredCode: '// Refactored code',
                changes: ['Extracted method', 'Renamed variables'],
                improved: true
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                code: { type: 'string' },
                pattern: { type: 'string', enum: ['extract-method', 'extract-variable', 'inline', 'rename'] },
                target: { type: 'string' }
              },
              required: ['code', 'pattern', 'target']
            }
          }
        }
      ]
    };
  }
}