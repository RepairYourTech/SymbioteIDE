/**
 * Security Auditor Agent
 * 
 * Performs security analysis and vulnerability detection
 */

import { ADKAgentConfig, ADKAgentType } from '../types';
import { ProviderType, Capability } from '../../orchestration/interfaces';

export class SecurityAuditorAgent {
  static getConfig(): ADKAgentConfig {
    return {
      id: 'security-auditor',
      name: 'Security Auditor',
      type: ADKAgentType.LLM,
      description: 'Performs security analysis and vulnerability detection',
      systemPrompt: `You are a cybersecurity expert specializing in application security. Your expertise includes:

1. Vulnerability Detection
   - OWASP Top 10 vulnerabilities
   - SQL injection, XSS, CSRF
   - Authentication/authorization flaws
   - Insecure dependencies
   - Code injection risks
   
2. Security Best Practices
   - Input validation and sanitization
   - Secure session management
   - Encryption and hashing
   - Secure API design
   - Least privilege principle
   
3. Compliance & Standards
   - GDPR, HIPAA, PCI-DSS
   - Security frameworks (NIST, ISO 27001)
   - Industry-specific requirements
   
4. Security Testing
   - Static analysis (SAST)
   - Dynamic analysis (DAST)
   - Dependency scanning
   - Penetration testing strategies
   
5. Incident Response
   - Security logging
   - Monitoring strategies
   - Incident detection
   - Response procedures

When auditing:
- Prioritize by severity (Critical, High, Medium, Low)
- Provide specific remediation steps
- Include code examples for fixes
- Reference security standards
- Consider false positives`,
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
        capabilities: [Capability.CodeAnalysis, Capability.Reasoning],
        supportsBatch: true,
        supportsStreaming: true,
        supportsTools: true,
        supportsVision: true,
        lastUpdated: new Date('2024-01-25')
      },
      temperature: 0.1,
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
          name: 'scan_vulnerabilities',
          description: 'Scan code for security vulnerabilities',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { code: string; language: string }) => {
              return {
                vulnerabilities: [
                  {
                    type: 'SQL Injection',
                    severity: 'High',
                    line: 42,
                    description: 'Unsanitized user input in SQL query',
                    remediation: 'Use parameterized queries'
                  }
                ],
                score: 7.5
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
          name: 'check_dependencies',
          description: 'Check for vulnerable dependencies',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { packageFile: string }) => {
              return {
                totalDependencies: 150,
                vulnerableDependencies: 3,
                criticalVulnerabilities: 1,
                updates: [
                  { package: 'lodash', current: '4.17.15', recommended: '4.17.21' }
                ]
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                packageFile: { type: 'string' }
              },
              required: ['packageFile']
            }
          }
        }
      ]
    };
  }
}