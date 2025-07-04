/**
 * Dependency Analyzer Agent
 * 
 * Analyzes and manages project dependencies
 */

import { ADKAgentConfig, ADKAgentType } from '../types';
import { ProviderType, Capability } from '../../orchestration/interfaces';

export class DependencyAnalyzerAgent {
  static getConfig(): ADKAgentConfig {
    return {
      id: 'dependency-analyzer',
      name: 'Dependency Analyzer',
      type: ADKAgentType.LLM,
      description: 'Analyzes and manages project dependencies',
      systemPrompt: `You are a dependency management expert specializing in:

1. Dependency Analysis
   - Direct vs transitive dependencies
   - Version conflicts
   - Security vulnerabilities
   - License compatibility
   - Bundle size impact
   
2. Package Managers
   - npm/yarn/pnpm (JavaScript)
   - pip/poetry (Python)
   - Maven/Gradle (Java)
   - NuGet (.NET)
   - Cargo (Rust)
   
3. Version Management
   - Semantic versioning
   - Version ranges
   - Lock files
   - Update strategies
   - Breaking changes
   
4. Optimization
   - Removing unused dependencies
   - Finding lighter alternatives
   - Monorepo strategies
   - Dependency deduplication
   - Tree shaking
   
5. Security
   - CVE scanning
   - Security advisories
   - Automated updates
   - Supply chain security
   - License compliance

When analyzing dependencies:
- Identify security risks
- Suggest updates carefully
- Consider breaking changes
- Optimize bundle size
- Document decisions`,
      model: {
        id: 'gpt-4-turbo',
        name: 'GPT-4 Turbo',
        displayName: 'GPT-4 Turbo',
        provider: ProviderType.OpenAI,
        contextWindow: 128000,
        maxOutputTokens: 4096,
        costPerToken: {
          input: 0.00001,
          output: 0.00003,
          currency: 'USD'
        },
        capabilities: [Capability.CodeAnalysis, Capability.CodeGeneration],
        supportsBatch: true,
        supportsStreaming: true,
        supportsTools: true,
        supportsVision: true,
        lastUpdated: new Date('2024-01-25')
      },
      temperature: 0.3,
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
          name: 'analyze_dependencies',
          description: 'Analyze project dependencies for issues',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { packageFile: string; deep?: boolean }) => {
              return {
                totalDependencies: 150,
                directDependencies: 30,
                outdated: 12,
                vulnerabilities: {
                  critical: 1,
                  high: 2,
                  medium: 5,
                  low: 10
                },
                duplicates: ['lodash', 'moment']
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                packageFile: { type: 'string' },
                deep: { type: 'boolean', default: true }
              },
              required: ['packageFile']
            }
          }
        },
        {
          name: 'suggest_alternatives',
          description: 'Suggest lighter or better dependency alternatives',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { dependency: string; reason?: string }) => {
              return {
                current: params.dependency,
                alternatives: [
                  {
                    name: 'alternative1',
                    size: '10KB',
                    pros: ['Smaller', 'Faster'],
                    cons: ['Less features']
                  }
                ],
                recommendation: 'Consider switching to alternative1'
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                dependency: { type: 'string' },
                reason: { type: 'string' }
              },
              required: ['dependency']
            }
          }
        }
      ]
    };
  }
}