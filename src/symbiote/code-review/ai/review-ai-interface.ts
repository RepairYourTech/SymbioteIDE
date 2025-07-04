/**
 * Review AI Interface - Interface for AI-powered code review operations
 */

import { Logger } from '../../utils/logger';
import { ReviewConfig } from '../types';
import { OrchestrationEngine } from '../../orchestration/orchestration-engine';

export interface AIResult {
  output: string;
  modelUsed?: string;
  cost?: number;
  confidence?: number;
  metadata?: Record<string, any>;
}

export interface AIRequest {
  prompt: string;
  context?: string;
  maxTokens?: number;
  temperature?: number;
  model?: string;
  responseFormat?: 'text' | 'json';
}

export class ReviewAIInterface {
  private logger = new Logger('ReviewAIInterface');
  private orchestrator?: OrchestrationEngine;
  private config: ReviewConfig;
  private lastResult?: AIResult;
  
  constructor(orchestrator?: OrchestrationEngine, config?: ReviewConfig) {
    this.orchestrator = orchestrator;
    this.config = config || this.getDefaultConfig();
  }
  
  /**
   * Send request to AI
   */
  async request(request: AIRequest): Promise<AIResult> {
    try {
      if (this.orchestrator) {
        // Use orchestration engine
        const response = await this.orchestrator.query({
          prompt: request.prompt,
          context: request.context,
          options: {
            maxTokens: request.maxTokens || 2000,
            temperature: request.temperature || 0.3,
            model: request.model || this.selectModel(request),
            stream: false
          }
        });
        
        const result: AIResult = {
          output: response.content,
          modelUsed: response.model,
          cost: response.cost,
          confidence: response.confidence,
          metadata: response.metadata
        };
        
        this.lastResult = result;
        return result;
        
      } else {
        // Fallback to mock response for testing
        return this.mockResponse(request);
      }
      
    } catch (error) {
      this.logger.error('AI request failed', error);
      throw error;
    }
  }
  
  /**
   * Analyze code with AI
   */
  async analyzeCode(
    code: string,
    language: string,
    context?: string
  ): Promise<{
    issues: any[];
    suggestions: string[];
    complexity: number;
    summary: string;
  }> {
    const prompt = `Analyze the following ${language} code for issues and improvements:

\`\`\`${language}
${code}
\`\`\`

${context ? `Context: ${context}\n\n` : ''}
Provide analysis in JSON format with:
- issues: array of {title, description, severity, line}
- suggestions: array of improvement suggestions
- complexity: cyclomatic complexity score (1-10)
- summary: brief summary of code quality`;
    
    const result = await this.request({
      prompt,
      responseFormat: 'json',
      model: this.config.modelPreferences.general
    });
    
    try {
      return JSON.parse(result.output);
    } catch (error) {
      this.logger.warn('Failed to parse AI response as JSON', error);
      
      // Fallback response
      return {
        issues: [],
        suggestions: [result.output],
        complexity: 5,
        summary: 'Analysis completed'
      };
    }
  }
  
  /**
   * Generate fix suggestion
   */
  async generateFix(
    issue: any,
    code: string,
    language: string
  ): Promise<{
    code: string;
    explanation: string;
    confidence: number;
  }> {
    const prompt = `Fix the following issue in ${language} code:

Issue: ${issue.title}
Description: ${issue.description}

Original code:
\`\`\`${language}
${code}
\`\`\`

Provide a fix in JSON format with:
- code: the fixed code
- explanation: why this fix works
- confidence: confidence level (0-1)`;
    
    const result = await this.request({
      prompt,
      responseFormat: 'json',
      model: this.config.modelPreferences.general,
      temperature: 0.2 // Lower temperature for more consistent fixes
    });
    
    try {
      return JSON.parse(result.output);
    } catch (error) {
      // Fallback to extracting code from response
      const codeMatch = result.output.match(/```[\w]*\n([\s\S]*?)```/);
      return {
        code: codeMatch ? codeMatch[1] : code,
        explanation: result.output,
        confidence: 0.5
      };
    }
  }
  
  /**
   * Explain code issue
   */
  async explainIssue(
    issue: any,
    code: string,
    language: string
  ): Promise<string> {
    const prompt = `Explain this code issue in detail:

Issue: ${issue.title}
Category: ${issue.category}
Severity: ${issue.severity}

Code context:
\`\`\`${language}
${code}
\`\`\`

Provide a clear explanation of:
1. What the issue is
2. Why it's a problem
3. Potential consequences
4. How to fix it
5. Best practices to prevent it`;
    
    const result = await this.request({
      prompt,
      model: this.config.modelPreferences.general
    });
    
    return result.output;
  }
  
  /**
   * Check security vulnerabilities
   */
  async checkSecurity(
    code: string,
    language: string
  ): Promise<{
    vulnerabilities: any[];
    riskLevel: string;
    recommendations: string[];
  }> {
    const prompt = `Analyze this ${language} code for security vulnerabilities:

\`\`\`${language}
${code}
\`\`\`

Check for:
- Injection vulnerabilities (SQL, XSS, etc.)
- Authentication/authorization issues
- Sensitive data exposure
- Cryptographic weaknesses
- Input validation problems

Provide results in JSON format with:
- vulnerabilities: array of {type, severity, description, line}
- riskLevel: overall risk (low/medium/high/critical)
- recommendations: security best practices`;
    
    const result = await this.request({
      prompt,
      responseFormat: 'json',
      model: this.config.modelPreferences.security
    });
    
    try {
      return JSON.parse(result.output);
    } catch (error) {
      return {
        vulnerabilities: [],
        riskLevel: 'unknown',
        recommendations: [result.output]
      };
    }
  }
  
  /**
   * Analyze performance
   */
  async analyzePerformance(
    code: string,
    language: string
  ): Promise<{
    bottlenecks: any[];
    optimizations: any[];
    metrics: Record<string, number>;
  }> {
    const prompt = `Analyze this ${language} code for performance:

\`\`\`${language}
${code}
\`\`\`

Identify:
- Performance bottlenecks
- Inefficient algorithms or data structures
- Memory leaks or excessive allocations
- Optimization opportunities

Provide results in JSON format with:
- bottlenecks: array of {issue, impact, location}
- optimizations: array of {suggestion, expectedImprovement}
- metrics: {complexity, memoryUsage, timeComplexity}`;
    
    const result = await this.request({
      prompt,
      responseFormat: 'json',
      model: this.config.modelPreferences.performance
    });
    
    try {
      return JSON.parse(result.output);
    } catch (error) {
      return {
        bottlenecks: [],
        optimizations: [],
        metrics: {}
      };
    }
  }
  
  /**
   * Learn team patterns
   */
  async learnPatterns(
    codeExamples: Array<{ code: string; metadata: any }>
  ): Promise<{
    patterns: any[];
    conventions: any[];
    antiPatterns: any[];
  }> {
    const prompt = `Analyze these code examples to learn team patterns:

${codeExamples.map((ex, i) => `Example ${i + 1}:
\`\`\`
${ex.code.substring(0, 500)}...
\`\`\`
Metadata: ${JSON.stringify(ex.metadata)}`).join('\n\n')}

Identify:
- Common coding patterns and styles
- Naming conventions
- Architectural patterns
- Anti-patterns to avoid

Provide results in JSON format with:
- patterns: array of identified patterns
- conventions: coding conventions used
- antiPatterns: patterns to avoid`;
    
    const result = await this.request({
      prompt,
      responseFormat: 'json',
      model: this.config.modelPreferences.general,
      maxTokens: 3000
    });
    
    try {
      return JSON.parse(result.output);
    } catch (error) {
      return {
        patterns: [],
        conventions: [],
        antiPatterns: []
      };
    }
  }
  
  /**
   * Generate PR review
   */
  async reviewPullRequest(
    changes: Array<{ file: string; diff: string }>,
    context: string
  ): Promise<{
    summary: string;
    issues: any[];
    suggestions: any[];
    approved: boolean;
  }> {
    const prompt = `Review this pull request:

${context}

Changes:
${changes.map(c => `File: ${c.file}\n\`\`\`diff\n${c.diff}\n\`\`\``).join('\n\n')}

Provide a comprehensive review in JSON format with:
- summary: overall assessment
- issues: array of {severity, file, line, description}
- suggestions: improvement suggestions
- approved: boolean recommendation`;
    
    const result = await this.request({
      prompt,
      responseFormat: 'json',
      model: this.config.modelPreferences.general,
      maxTokens: 4000
    });
    
    try {
      return JSON.parse(result.output);
    } catch (error) {
      return {
        summary: result.output,
        issues: [],
        suggestions: [],
        approved: false
      };
    }
  }
  
  /**
   * Get last result
   */
  async getLastResult(): Promise<AIResult | undefined> {
    return this.lastResult;
  }
  
  /**
   * Select model based on request
   */
  private selectModel(request: AIRequest): string {
    // Select model based on content
    if (request.prompt.toLowerCase().includes('security')) {
      return this.config.modelPreferences.security;
    }
    
    if (request.prompt.toLowerCase().includes('performance')) {
      return this.config.modelPreferences.performance;
    }
    
    return this.config.modelPreferences.general;
  }
  
  /**
   * Mock response for testing
   */
  private async mockResponse(request: AIRequest): Promise<AIResult> {
    await new Promise(resolve => setTimeout(resolve, 100)); // Simulate delay
    
    const mockResponses: Record<string, any> = {
      analyze: {
        issues: [
          {
            title: 'Potential null reference',
            description: 'Variable may be null',
            severity: 'warning',
            line: 10
          }
        ],
        suggestions: ['Add null check before usage'],
        complexity: 3,
        summary: 'Code is generally well-written with minor issues'
      },
      fix: {
        code: 'if (variable != null) { /* use variable */ }',
        explanation: 'Added null check to prevent null reference exception',
        confidence: 0.9
      },
      security: {
        vulnerabilities: [],
        riskLevel: 'low',
        recommendations: ['Consider using parameterized queries']
      },
      performance: {
        bottlenecks: [],
        optimizations: [{ suggestion: 'Use caching', expectedImprovement: '20%' }],
        metrics: { complexity: 5, memoryUsage: 100, timeComplexity: 1 }
      }
    };
    
    // Determine response type
    let responseKey = 'analyze';
    if (request.prompt.includes('Fix')) responseKey = 'fix';
    if (request.prompt.includes('security')) responseKey = 'security';
    if (request.prompt.includes('performance')) responseKey = 'performance';
    
    const response = mockResponses[responseKey];
    const output = request.responseFormat === 'json' ? 
      JSON.stringify(response) : 
      'Mock analysis completed successfully';
    
    const result: AIResult = {
      output,
      modelUsed: 'mock',
      cost: 0,
      confidence: 0.8
    };
    
    this.lastResult = result;
    return result;
  }
  
  /**
   * Get default config
   */
  private getDefaultConfig(): ReviewConfig {
    return {
      severityThresholds: {},
      enabledRules: [],
      disabledRules: [],
      modelPreferences: {
        security: 'gpt-4',
        performance: 'gpt-4',
        general: 'gpt-3.5-turbo'
      },
      maxConcurrentAnalysis: 5,
      cacheEnabled: true,
      cacheDuration: 3600,
      gitIntegration: true,
      ciIntegration: false,
      prReviewEnabled: true
    };
  }
  
  /**
   * Update config
   */
  updateConfig(config: ReviewConfig): void {
    this.config = config;
  }
}
