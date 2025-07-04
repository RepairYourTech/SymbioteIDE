/**
 * Review and Critique Agent Pattern
 * 
 * Provides review, feedback, and quality assurance for task outputs
 */

import { LLMAgent, Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty } from '../../types';
import { Logger } from '../../../utils/logger';

export interface ReviewCriteria {
  name: string;
  description: string;
  weight: number;
  evaluator: (output: any) => Promise<ReviewScore>;
}

export interface ReviewScore {
  score: number; // 0-100
  feedback: string;
  issues: string[];
  suggestions: string[];
}

export interface ReviewConfig extends ADKHivemindConfig {
  reviewCriteria: ReviewCriteria[];
  passingThreshold?: number;
  provideSuggestions?: boolean;
  maxRevisionRounds?: number;
  originalAgent?: ADKHivemindAgent; // Agent that produced the work
}

export interface ReviewResult {
  overallScore: number;
  passed: boolean;
  criteriaScores: Map<string, ReviewScore>;
  summary: string;
  requiredRevisions: string[];
  optionalSuggestions: string[];
}

export class ReviewCritiqueAgent extends ADKHivemindAgent {
  private reviewCriteria: ReviewCriteria[];
  private passingThreshold: number;
  private provideSuggestions: boolean;
  private maxRevisionRounds: number;
  private originalAgent?: ADKHivemindAgent;
  private reviewHistory: any[] = [];
  
  constructor(config: ReviewConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are a code review and quality assurance specialist.
      
Your responsibilities:
1. Review code and documentation for quality
2. Check adherence to best practices and standards
3. Identify bugs, security issues, and performance problems
4. Provide constructive feedback and suggestions
5. Ensure code meets acceptance criteria

Review criteria:
${config.reviewCriteria.map(c => 
  `- ${c.name} (weight: ${c.weight}): ${c.description}`
).join('\n')}

Passing threshold: ${config.passingThreshold || 80}%

Focus on:
- Correctness and functionality
- Code quality and maintainability
- Performance and scalability
- Security and best practices
- Documentation and tests

Provide specific, actionable feedback with examples when possible.`
    });
    
    this.reviewCriteria = config.reviewCriteria;
    this.passingThreshold = config.passingThreshold || 80;
    this.provideSuggestions = config.provideSuggestions ?? true;
    this.maxRevisionRounds = config.maxRevisionRounds || 3;
    this.originalAgent = config.originalAgent;
    
    // Add review tools
    this.addReviewTools();
  }
  
  /**
   * Add review-specific tools
   */
  private addReviewTools(): void {
    this.tools.push(new Tool({
      name: 'review_code',
      description: 'Review code for quality and issues',
      parameters: {
        code: { type: 'string', required: true },
        language: { type: 'string', required: true },
        context: { type: 'object', required: false }
      },
      handler: async (params: any) => {
        return this.reviewCode(params.code, params.language, params.context);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'check_best_practices',
      description: 'Check code against best practices',
      parameters: {
        code: { type: 'string', required: true },
        framework: { type: 'string', required: false }
      },
      handler: async (params: any) => {
        return this.checkBestPractices(params.code, params.framework);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'security_audit',
      description: 'Audit code for security vulnerabilities',
      parameters: {
        code: { type: 'string', required: true },
        sensitivePatterns: { type: 'array', required: false }
      },
      handler: async (params: any) => {
        return this.securityAudit(params.code, params.sensitivePatterns);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'suggest_improvements',
      description: 'Suggest code improvements',
      parameters: {
        code: { type: 'string', required: true },
        focusAreas: { type: 'array', required: false }
      },
      handler: async (params: any) => {
        return this.suggestImprovements(params.code, params.focusAreas);
      }
    }));
  }
  
  /**
   * Execute review task
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    const reviewId = `review_${task.id}`;
    
    try {
      // Initialize review state
      const reviewState = {
        taskId: task.id,
        rounds: [],
        currentRound: 0,
        finalScore: 0,
        status: 'reviewing'
      };
      
      await this.writeState(reviewId, reviewState);
      
      // Extract output to review
      const outputToReview = task.context.outputToReview || task.context.previousOutput;
      if (!outputToReview) {
        throw new Error('No output provided for review');
      }
      
      // Perform iterative review if needed
      let reviewResult: ReviewResult | null = null;
      let currentOutput = outputToReview;
      
      for (let round = 0; round < this.maxRevisionRounds; round++) {
        reviewState.currentRound = round + 1;
        
        // Perform review
        reviewResult = await this.performReview(currentOutput, task);
        
        // Store round results
        reviewState.rounds.push({
          round: round + 1,
          score: reviewResult.overallScore,
          passed: reviewResult.passed,
          timestamp: new Date()
        });
        
        await this.writeState(reviewId, reviewState);
        
        // Check if passed
        if (reviewResult.passed) {
          this.logger.info(`Review passed on round ${round + 1}`);
          break;
        }
        
        // If original agent available and revisions needed, request revision
        if (this.originalAgent && round < this.maxRevisionRounds - 1) {
          currentOutput = await this.requestRevision(
            task,
            currentOutput,
            reviewResult
          );
        } else {
          break;
        }
      }
      
      // Finalize review
      reviewState.finalScore = reviewResult?.overallScore || 0;
      reviewState.status = 'completed';
      await this.writeState(reviewId, reviewState);
      
      // Create task result
      const result: TaskResult = {
        taskId: task.id,
        success: reviewResult?.passed || false,
        output: {
          reviewResult,
          rounds: reviewState.rounds.length,
          finalScore: reviewResult?.overallScore || 0,
          summary: reviewResult?.summary || 'Review incomplete'
        },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: reviewResult?.requiredRevisions || [],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
      
      // Store in history
      this.reviewHistory.push({
        taskId: task.id,
        reviewId,
        rounds: reviewState.rounds.length,
        finalScore: reviewResult?.overallScore || 0,
        passed: reviewResult?.passed || false
      });
      
      return result;
      
    } catch (error) {
      this.logger.error(`Review failed`, error);
      
      return {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Review error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
    }
  }
  
  /**
   * Perform comprehensive review
   */
  private async performReview(
    output: any,
    task: HivemindTask
  ): Promise<ReviewResult> {
    const criteriaScores = new Map<string, ReviewScore>();
    let totalWeightedScore = 0;
    let totalWeight = 0;
    
    // Evaluate each criterion
    for (const criterion of this.reviewCriteria) {
      try {
        const score = await criterion.evaluator(output);
        criteriaScores.set(criterion.name, score);
        totalWeightedScore += score.score * criterion.weight;
        totalWeight += criterion.weight;
      } catch (error) {
        this.logger.error(`Failed to evaluate criterion ${criterion.name}`, error);
        criteriaScores.set(criterion.name, {
          score: 0,
          feedback: 'Evaluation failed',
          issues: [`Error: ${error.message}`],
          suggestions: []
        });
      }
    }
    
    // Calculate overall score
    const overallScore = totalWeight > 0 ? totalWeightedScore / totalWeight : 0;
    const passed = overallScore >= this.passingThreshold;
    
    // Compile feedback
    const requiredRevisions: string[] = [];
    const optionalSuggestions: string[] = [];
    
    for (const [criterionName, score] of criteriaScores) {
      if (score.score < this.passingThreshold) {
        requiredRevisions.push(...score.issues);
      }
      if (this.provideSuggestions) {
        optionalSuggestions.push(...score.suggestions);
      }
    }
    
    // Generate summary using LLM
    const summary = await this.generateReviewSummary(
      output,
      criteriaScores,
      overallScore,
      passed
    );
    
    return {
      overallScore,
      passed,
      criteriaScores,
      summary,
      requiredRevisions,
      optionalSuggestions
    };
  }
  
  /**
   * Generate review summary
   */
  private async generateReviewSummary(
    output: any,
    criteriaScores: Map<string, ReviewScore>,
    overallScore: number,
    passed: boolean
  ): Promise<string> {
    const prompt = `Generate a concise review summary:

Overall Score: ${overallScore.toFixed(1)}%
Status: ${passed ? 'PASSED' : 'NEEDS REVISION'}

Criteria Results:
${Array.from(criteriaScores.entries()).map(([name, score]) => 
  `- ${name}: ${score.score}% - ${score.feedback}`
).join('\n')}

Provide a 2-3 sentence summary of the review results.`;

    const result = await this.run(prompt);
    return result.output;
  }
  
  /**
   * Request revision from original agent
   */
  private async requestRevision(
    task: HivemindTask,
    currentOutput: any,
    reviewResult: ReviewResult
  ): Promise<any> {
    if (!this.originalAgent) {
      return currentOutput;
    }
    
    // Create revision task
    const revisionTask: HivemindTask = {
      ...task,
      id: `${task.id}_revision`,
      title: `Revise: ${task.title}`,
      description: `Revise the output based on review feedback:\n\n${reviewResult.requiredRevisions.join('\n')}`,
      context: {
        ...task.context,
        previousOutput: currentOutput,
        reviewFeedback: reviewResult,
        revisionRound: true
      }
    };
    
    // Execute revision
    const revisionResult = await this.originalAgent.executeTask(revisionTask);
    
    return revisionResult.output;
  }
  
  /**
   * Review code
   */
  private async reviewCode(
    code: string,
    language: string,
    context?: any
  ): Promise<ReviewScore> {
    const prompt = `Review this ${language} code:

\`\`\`${language}
${code}
\`\`\`

${context ? `Context: ${JSON.stringify(context)}` : ''}

Evaluate:
1. Correctness and functionality
2. Code quality and readability
3. Error handling
4. Performance considerations
5. Security concerns

Provide a score (0-100) and specific feedback.`;

    const result = await this.run(prompt);
    
    // Parse LLM response
    try {
      const parsed = JSON.parse(result.output);
      return {
        score: parsed.score || 0,
        feedback: parsed.feedback || '',
        issues: parsed.issues || [],
        suggestions: parsed.suggestions || []
      };
    } catch {
      return {
        score: 50,
        feedback: result.output,
        issues: [],
        suggestions: []
      };
    }
  }
  
  /**
   * Check best practices
   */
  private async checkBestPractices(
    code: string,
    framework?: string
  ): Promise<any> {
    const practices = {
      violations: [] as string[],
      suggestions: [] as string[],
      score: 100
    };
    
    // Common checks
    if (code.includes('console.log') || code.includes('print(')) {
      practices.violations.push('Debug statements found in code');
      practices.score -= 10;
    }
    
    if (!code.includes('try') && !code.includes('catch')) {
      practices.suggestions.push('Consider adding error handling');
      practices.score -= 5;
    }
    
    if (code.includes('var ')) {
      practices.violations.push('Using var instead of let/const');
      practices.score -= 5;
    }
    
    // Framework-specific checks
    if (framework === 'react' && code.includes('componentWillMount')) {
      practices.violations.push('Using deprecated lifecycle methods');
      practices.score -= 10;
    }
    
    return practices;
  }
  
  /**
   * Security audit
   */
  private async securityAudit(
    code: string,
    sensitivePatterns?: string[]
  ): Promise<any> {
    const vulnerabilities: string[] = [];
    const risks: string[] = [];
    
    // Common security checks
    if (code.includes('eval(')) {
      vulnerabilities.push('Use of eval() - potential code injection');
    }
    
    if (code.match(/password\s*=\s*["'][^"']+["']/i)) {
      vulnerabilities.push('Hardcoded password detected');
    }
    
    if (code.includes('innerHTML')) {
      risks.push('Use of innerHTML - potential XSS vulnerability');
    }
    
    if (code.match(/SELECT.*FROM.*WHERE/i) && !code.includes('?')) {
      vulnerabilities.push('Potential SQL injection - use parameterized queries');
    }
    
    // Check for sensitive patterns
    if (sensitivePatterns) {
      for (const pattern of sensitivePatterns) {
        if (code.includes(pattern)) {
          vulnerabilities.push(`Sensitive pattern found: ${pattern}`);
        }
      }
    }
    
    return {
      vulnerabilities,
      risks,
      severity: vulnerabilities.length > 0 ? 'high' : 
                risks.length > 0 ? 'medium' : 'low'
    };
  }
  
  /**
   * Suggest improvements
   */
  private async suggestImprovements(
    code: string,
    focusAreas?: string[]
  ): Promise<string[]> {
    const suggestions: string[] = [];
    
    // Performance suggestions
    if (!focusAreas || focusAreas.includes('performance')) {
      if (code.includes('.forEach') && code.includes('async')) {
        suggestions.push('Consider using Promise.all() for parallel async operations');
      }
      
      if (code.match(/for.*in/)) {
        suggestions.push('Consider using for...of instead of for...in for better performance');
      }
    }
    
    // Readability suggestions
    if (!focusAreas || focusAreas.includes('readability')) {
      if (code.split('\n').some(line => line.length > 100)) {
        suggestions.push('Consider breaking long lines for better readability');
      }
      
      if (!code.includes('/**') && !code.includes('//')) {
        suggestions.push('Add comments to explain complex logic');
      }
    }
    
    // Modern syntax suggestions
    if (!focusAreas || focusAreas.includes('modern')) {
      if (code.includes('function(')) {
        suggestions.push('Consider using arrow functions for cleaner syntax');
      }
      
      if (code.includes('Object.assign')) {
        suggestions.push('Consider using spread operator instead of Object.assign');
      }
    }
    
    return suggestions;
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    const outputInfo = task.context.outputToReview 
      ? 'Output to review provided in context' 
      : 'No specific output provided';
    
    return `Perform a comprehensive review of the task output:

Task: ${task.title}
Description: ${task.description}
${outputInfo}

Apply all review criteria and provide detailed feedback.
Focus on identifying issues that must be fixed and suggestions for improvement.`;
  }
  
  /**
   * Get review history
   */
  public getReviewHistory(): any[] {
    return [...this.reviewHistory];
  }
  
  /**
   * Create default review criteria
   */
  public static createDefaultCriteria(): ReviewCriteria[] {
    return [
      {
        name: 'Functionality',
        description: 'Code works correctly and meets requirements',
        weight: 30,
        evaluator: async (output) => ({
          score: 85, // Would be determined by actual testing
          feedback: 'Basic functionality appears correct',
          issues: [],
          suggestions: ['Add edge case handling']
        })
      },
      {
        name: 'Code Quality',
        description: 'Code is clean, readable, and maintainable',
        weight: 25,
        evaluator: async (output) => ({
          score: 80,
          feedback: 'Code is generally well-structured',
          issues: ['Some functions are too long'],
          suggestions: ['Extract complex logic into helper functions']
        })
      },
      {
        name: 'Testing',
        description: 'Adequate test coverage and quality',
        weight: 20,
        evaluator: async (output) => ({
          score: 70,
          feedback: 'Basic tests present but coverage could be improved',
          issues: ['Missing edge case tests'],
          suggestions: ['Add integration tests']
        })
      },
      {
        name: 'Documentation',
        description: 'Code is well-documented',
        weight: 15,
        evaluator: async (output) => ({
          score: 75,
          feedback: 'Documentation exists but could be more detailed',
          issues: ['Missing API documentation'],
          suggestions: ['Add examples to documentation']
        })
      },
      {
        name: 'Performance',
        description: 'Code is optimized and efficient',
        weight: 10,
        evaluator: async (output) => ({
          score: 90,
          feedback: 'No obvious performance issues',
          issues: [],
          suggestions: ['Consider caching for repeated operations']
        })
      }
    ];
  }
}