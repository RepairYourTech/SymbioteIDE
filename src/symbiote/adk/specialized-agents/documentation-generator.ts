/**
 * Documentation Generator Agent
 * 
 * Creates comprehensive technical documentation
 */

import { ADKAgentConfig, ADKAgentType } from '../types';
import { ProviderType, Capability } from '../../orchestration/interfaces';

export class DocumentationGeneratorAgent {
  static getConfig(): ADKAgentConfig {
    return {
      id: 'documentation-generator',
      name: 'Documentation Generator',
      type: ADKAgentType.LLM,
      description: 'Creates comprehensive technical documentation',
      systemPrompt: `You are a technical documentation expert skilled in creating clear, comprehensive documentation. Your expertise includes:

1. Documentation Types
   - API documentation (OpenAPI/Swagger)
   - Code documentation (JSDoc, TSDoc)
   - Architecture documentation (ADRs)
   - User guides and tutorials
   - README files
   
2. Documentation Standards
   - Clear and concise writing
   - Consistent formatting
   - Proper examples and use cases
   - Visual aids (diagrams, screenshots)
   - Accessibility considerations
   
3. Documentation Tools
   - Markdown formatting
   - Documentation generators (TypeDoc, JSDoc)
   - API documentation tools
   - Diagram tools (Mermaid, PlantUML)
   
4. Best Practices
   - Keep documentation up-to-date
   - Include code examples
   - Explain the "why" not just "how"
   - Progressive disclosure
   - Searchable content
   
5. Audience Awareness
   - Developer documentation
   - End-user documentation
   - API consumers
   - System administrators
   - Business stakeholders

When generating documentation:
- Use clear, simple language
- Include practical examples
- Organize content logically
- Add navigation aids
- Maintain version history`,
      model: {
        id: 'claude-3-sonnet',
        name: 'Claude 3 Sonnet',
        displayName: 'Claude 3 Sonnet',
        provider: ProviderType.Anthropic,
        contextWindow: 200000,
        maxOutputTokens: 4096,
        costPerToken: {
          input: 0.000003,
          output: 0.000015,
          currency: 'USD'
        },
        capabilities: [Capability.CreativeWriting, Capability.CodeGeneration],
        supportsBatch: true,
        supportsStreaming: true,
        supportsTools: true,
        supportsVision: false,
        lastUpdated: new Date('2024-03-04')
      },
      temperature: 0.6,
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
          name: 'generate_api_docs',
          description: 'Generate API documentation from code',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { code: string; format: string }) => {
              return {
                documentation: '# API Documentation\n\n...',
                format: params.format,
                endpoints: 10
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                code: { type: 'string' },
                format: { type: 'string', enum: ['markdown', 'openapi', 'html'] }
              },
              required: ['code', 'format']
            }
          }
        },
        {
          name: 'create_readme',
          description: 'Create comprehensive README file',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { projectInfo: any }) => {
              return {
                readme: '# Project Name\n\n## Overview\n\n...',
                sections: ['Overview', 'Installation', 'Usage', 'API', 'Contributing']
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                projectInfo: { type: 'object' }
              },
              required: ['projectInfo']
            }
          }
        }
      ]
    };
  }
}