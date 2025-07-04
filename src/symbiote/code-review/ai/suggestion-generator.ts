/**
 * Suggestion Generator - Generates AI-powered fix suggestions
 */

import { Logger } from '../../utils/logger';
import { ReviewIssue, ReviewSuggestion, ReviewSeverity } from '../types';
import { ReviewAIInterface } from './review-ai-interface';

export interface SuggestionOptions {
  generateAlternatives?: boolean;
  includeExplanations?: boolean;
  considerImpact?: boolean;
  maxSuggestions?: number;
}

export class SuggestionGenerator {
  private logger = new Logger('SuggestionGenerator');
  private aiInterface: ReviewAIInterface;
  private suggestionCache = new Map<string, ReviewSuggestion[]>();
  
  constructor(aiInterface: ReviewAIInterface) {
    this.aiInterface = aiInterface;
  }
  
  /**
   * Generate suggestions for an issue
   */
  async generateSuggestions(
    issue: ReviewIssue,
    options: SuggestionOptions = {}
  ): Promise<ReviewSuggestion[]> {
    const cacheKey = `${issue.id}_${JSON.stringify(options)}`;
    
    // Check cache
    if (this.suggestionCache.has(cacheKey)) {
      return this.suggestionCache.get(cacheKey)!;
    }
    
    try {
      this.logger.info(`Generating suggestions for issue: ${issue.title}`);
      
      const suggestions: ReviewSuggestion[] = [];
      
      // Generate primary suggestion
      const primarySuggestion = await this.generatePrimarySuggestion(issue, options);
      if (primarySuggestion) {
        suggestions.push(primarySuggestion);
      }
      
      // Generate alternatives if requested
      if (options.generateAlternatives && suggestions.length > 0) {
        const alternatives = await this.generateAlternatives(
          issue,
          primarySuggestion!,
          options
        );
        suggestions.push(...alternatives);
      }
      
      // Limit number of suggestions
      const maxSuggestions = options.maxSuggestions || 3;
      const limitedSuggestions = suggestions.slice(0, maxSuggestions);
      
      // Cache results
      this.suggestionCache.set(cacheKey, limitedSuggestions);
      
      return limitedSuggestions;
      
    } catch (error) {
      this.logger.error('Failed to generate suggestions', error);
      return [];
    }
  }
  
  /**
   * Generate primary suggestion
   */
  private async generatePrimarySuggestion(
    issue: ReviewIssue,
    options: SuggestionOptions
  ): Promise<ReviewSuggestion | null> {
    try {
      // Get the code context
      const codeContext = issue.location.snippet || '';
      const beforeContext = issue.location.context?.before || '';
      const afterContext = issue.location.context?.after || '';
      
      const fullContext = `${beforeContext}\n${codeContext}\n${afterContext}`.trim();
      
      // Generate fix based on issue type
      const fixResult = await this.generateFixForIssueType(
        issue,
        fullContext,
        options
      );
      
      if (!fixResult) {
        return null;
      }
      
      // Create suggestion
      const suggestion: ReviewSuggestion = {
        id: `sug_${issue.id}_${Date.now()}`,
        issueId: issue.id,
        title: this.generateSuggestionTitle(issue),
        description: fixResult.description || 'Automated fix suggestion',
        code: fixResult.code,
        explanation: fixResult.explanation,
        confidence: fixResult.confidence || 0.7,
        references: fixResult.references || []
      };
      
      // Add impact analysis if requested
      if (options.considerImpact) {
        suggestion.estimatedImpact = await this.estimateImpact(
          issue,
          fixResult.code
        );
      }
      
      return suggestion;
      
    } catch (error) {
      this.logger.error('Failed to generate primary suggestion', error);
      return null;
    }
  }
  
  /**
   * Generate fix based on issue type
   */
  private async generateFixForIssueType(
    issue: ReviewIssue,
    context: string,
    options: SuggestionOptions
  ): Promise<{
    code: string;
    description: string;
    explanation: string;
    confidence: number;
    references?: string[];
  } | null> {
    const language = this.detectLanguage(issue.location.uri.fsPath);
    
    // Handle specific issue types with templates
    if (issue.rule) {
      const templateFix = this.getTemplateFix(issue.rule, context, language);
      if (templateFix) {
        return templateFix;
      }
    }
    
    // Use AI for complex fixes
    const aiResult = await this.aiInterface.generateFix(
      {
        title: issue.title,
        description: issue.description,
        category: issue.category,
        severity: issue.severity,
        rule: issue.rule
      },
      context,
      language
    );
    
    // Enhance with additional explanation if requested
    let explanation = aiResult.explanation;
    if (options.includeExplanations) {
      explanation = await this.enhanceExplanation(
        issue,
        aiResult.code,
        aiResult.explanation
      );
    }
    
    return {
      code: aiResult.code,
      description: `Fix for: ${issue.title}`,
      explanation,
      confidence: aiResult.confidence,
      references: this.getReferences(issue)
    };
  }
  
  /**
   * Get template fix for common issues
   */
  private getTemplateFix(
    rule: string,
    context: string,
    language: string
  ): any | null {
    const templates: Record<string, (context: string) => any> = {
      'no-console': (ctx) => ({
        code: ctx.replace(/console\.(log|error|warn|info|debug)/g, '// $&'),
        description: 'Comment out console statements',
        explanation: 'Console statements should be removed or replaced with proper logging in production',
        confidence: 0.95
      }),
      
      'no-var': (ctx) => ({
        code: ctx.replace(/\bvar\s+/g, 'let '),
        description: 'Replace var with let',
        explanation: 'Using let provides block scoping and prevents hoisting issues',
        confidence: 0.9
      }),
      
      'strict-equality': (ctx) => ({
        code: ctx.replace(/==/g, '===').replace(/!=/g, '!=='),
        description: 'Use strict equality operators',
        explanation: 'Strict equality prevents type coercion and unexpected behavior',
        confidence: 0.85
      }),
      
      'no-empty-catch': (ctx) => {
        const emptyMatch = ctx.match(/catch\s*\(([^)]*)\)\s*\{\s*\}/);
        if (emptyMatch) {
          const errorVar = emptyMatch[1] || 'error';
          const fixed = ctx.replace(
            /catch\s*\([^)]*\)\s*\{\s*\}/,
            `catch (${errorVar}) {\n    console.error('Error occurred:', ${errorVar});\n  }`
          );
          return {
            code: fixed,
            description: 'Add error handling to empty catch block',
            explanation: 'Empty catch blocks hide errors and make debugging difficult',
            confidence: 0.8
          };
        }
        return null;
      },
      
      'no-magic-numbers': (ctx) => {
        // Find magic numbers
        const numberMatch = ctx.match(/\b(\d+)\b/);
        if (numberMatch && numberMatch[1] !== '0' && numberMatch[1] !== '1') {
          const number = numberMatch[1];
          const constName = this.generateConstantName(number, ctx);
          const fixed = `const ${constName} = ${number};\n${ctx.replace(number, constName)}`;
          return {
            code: fixed,
            description: 'Extract magic number to named constant',
            explanation: 'Named constants improve code readability and maintainability',
            confidence: 0.7
          };
        }
        return null;
      }
    };
    
    const templateFn = templates[rule];
    if (templateFn) {
      try {
        return templateFn(context);
      } catch (error) {
        this.logger.warn(`Template fix failed for rule ${rule}`, error);
      }
    }
    
    return null;
  }
  
  /**
   * Generate alternatives
   */
  private async generateAlternatives(
    issue: ReviewIssue,
    primarySuggestion: ReviewSuggestion,
    options: SuggestionOptions
  ): Promise<ReviewSuggestion[]> {
    const alternatives: ReviewSuggestion[] = [];
    
    try {
      // Generate different approaches
      const approaches = await this.generateDifferentApproaches(
        issue,
        primarySuggestion.code
      );
      
      for (const approach of approaches) {
        const alternative: ReviewSuggestion = {
          id: `sug_alt_${issue.id}_${Date.now()}_${alternatives.length}`,
          issueId: issue.id,
          title: approach.title,
          description: approach.description,
          code: approach.code,
          explanation: approach.explanation,
          confidence: approach.confidence * 0.8, // Lower confidence for alternatives
          references: approach.references || []
        };
        
        if (options.considerImpact) {
          alternative.estimatedImpact = await this.estimateImpact(
            issue,
            approach.code
          );
        }
        
        alternatives.push(alternative);
      }
      
    } catch (error) {
      this.logger.error('Failed to generate alternatives', error);
    }
    
    return alternatives;
  }
  
  /**
   * Generate different approaches
   */
  private async generateDifferentApproaches(
    issue: ReviewIssue,
    primaryCode: string
  ): Promise<any[]> {
    const prompt = `Given this issue and primary fix, suggest 2 alternative approaches:

Issue: ${issue.title}
Primary fix:
\`\`\`
${primaryCode}
\`\`\`

Provide alternatives that:
1. Use a different technique or pattern
2. Have different trade-offs (performance vs readability)

Format as JSON array with: title, description, code, explanation, confidence`;
    
    try {
      const result = await this.aiInterface.request({
        prompt,
        responseFormat: 'json',
        maxTokens: 2000
      });
      
      const approaches = JSON.parse(result.output);
      return Array.isArray(approaches) ? approaches : [];
      
    } catch (error) {
      this.logger.warn('Failed to generate alternative approaches', error);
      return [];
    }
  }
  
  /**
   * Estimate impact of fix
   */
  private async estimateImpact(
    issue: ReviewIssue,
    fixCode: string
  ): Promise<ReviewSuggestion['estimatedImpact']> {
    // Simple heuristic-based estimation
    const impact: ReviewSuggestion['estimatedImpact'] = {
      performance: 0,
      security: 0,
      maintainability: 0,
      testCoverage: 0
    };
    
    // Performance impact
    if (issue.category === 'performance') {
      impact.performance = 0.3; // Assume moderate improvement
    } else if (fixCode.includes('async') || fixCode.includes('Promise')) {
      impact.performance = 0.1; // Async changes might help
    }
    
    // Security impact
    if (issue.category === 'security') {
      impact.security = this.getSecurityImpact(issue.severity);
    }
    
    // Maintainability impact
    if (issue.category === 'best-practices' || issue.category === 'maintainability') {
      impact.maintainability = 0.2;
    } else if (fixCode.length < issue.location.snippet?.length!) {
      impact.maintainability = 0.1; // Simpler code
    }
    
    // Test coverage impact
    if (fixCode.includes('test') || fixCode.includes('expect')) {
      impact.testCoverage = 0.1;
    }
    
    return impact;
  }
  
  /**
   * Enhance explanation
   */
  private async enhanceExplanation(
    issue: ReviewIssue,
    fixCode: string,
    basicExplanation: string
  ): Promise<string> {
    const prompt = `Enhance this fix explanation with:
- Why the original code was problematic
- How the fix addresses the issue
- Benefits of this approach
- Any trade-offs or considerations

Issue: ${issue.title}
Fix explanation: ${basicExplanation}

Provide a comprehensive but concise explanation.`;
    
    try {
      const result = await this.aiInterface.request({
        prompt,
        maxTokens: 500
      });
      
      return result.output;
    } catch (error) {
      return basicExplanation;
    }
  }
  
  /**
   * Generate suggestion title
   */
  private generateSuggestionTitle(issue: ReviewIssue): string {
    const actionMap: Record<string, string> = {
      'no-console': 'Remove console statements',
      'no-var': 'Replace var with let/const',
      'no-eval': 'Remove eval() usage',
      'no-empty-catch': 'Add error handling',
      'no-magic-numbers': 'Extract magic numbers',
      'strict-equality': 'Use strict equality',
      'no-hardcoded-secrets': 'Remove hardcoded credentials',
      'vulnerable-dependency': 'Update vulnerable dependency'
    };
    
    if (issue.rule && actionMap[issue.rule]) {
      return actionMap[issue.rule];
    }
    
    // Generate based on category
    const categoryPrefixes = {
      security: 'Fix security vulnerability',
      performance: 'Optimize performance',
      'best-practices': 'Improve code quality',
      maintainability: 'Enhance maintainability',
      accessibility: 'Fix accessibility issue',
      testing: 'Add test coverage',
      documentation: 'Update documentation',
      dependencies: 'Fix dependency issue',
      architecture: 'Refactor architecture',
      style: 'Fix style issue'
    };
    
    const prefix = categoryPrefixes[issue.category] || 'Fix';
    return `${prefix}: ${issue.title}`;
  }
  
  /**
   * Get references for issue
   */
  private getReferences(issue: ReviewIssue): string[] {
    const references: string[] = [];
    
    // Add rule-specific references
    const ruleReferences: Record<string, string[]> = {
      'no-eval': [
        'https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/eval#never_use_eval!'
      ],
      'no-hardcoded-secrets': [
        'https://owasp.org/www-community/vulnerabilities/Use_of_hard-coded_password'
      ],
      'strict-equality': [
        'https://developer.mozilla.org/en-US/docs/Web/JavaScript/Equality_comparisons_and_sameness'
      ]
    };
    
    if (issue.rule && ruleReferences[issue.rule]) {
      references.push(...ruleReferences[issue.rule]);
    }
    
    // Add category-specific references
    if (issue.category === 'security') {
      references.push('https://owasp.org/www-project-top-ten/');
    } else if (issue.category === 'performance') {
      references.push('https://web.dev/fast/');
    }
    
    return references;
  }
  
  /**
   * Generate constant name from number
   */
  private generateConstantName(number: string, context: string): string {
    // Try to infer meaning from context
    const contextLower = context.toLowerCase();
    
    if (contextLower.includes('timeout')) return `TIMEOUT_MS`;
    if (contextLower.includes('max')) return `MAX_VALUE`;
    if (contextLower.includes('min')) return `MIN_VALUE`;
    if (contextLower.includes('port')) return `PORT_NUMBER`;
    if (contextLower.includes('limit')) return `LIMIT`;
    if (contextLower.includes('size')) return `SIZE`;
    if (contextLower.includes('count')) return `COUNT`;
    
    // Common numbers
    const commonNumbers: Record<string, string> = {
      '60': 'SECONDS_PER_MINUTE',
      '3600': 'SECONDS_PER_HOUR',
      '86400': 'SECONDS_PER_DAY',
      '1000': 'MILLISECONDS_PER_SECOND',
      '1024': 'BYTES_PER_KB',
      '404': 'NOT_FOUND_STATUS',
      '200': 'OK_STATUS',
      '500': 'SERVER_ERROR_STATUS'
    };
    
    return commonNumbers[number] || `CONSTANT_${number}`;
  }
  
  /**
   * Get security impact based on severity
   */
  private getSecurityImpact(severity: ReviewSeverity): number {
    const impactMap = {
      [ReviewSeverity.Critical]: 1.0,
      [ReviewSeverity.Error]: 0.7,
      [ReviewSeverity.Warning]: 0.4,
      [ReviewSeverity.Info]: 0.2
    };
    return impactMap[severity];
  }
  
  /**
   * Detect language from file path
   */
  private detectLanguage(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const langMap: Record<string, string> = {
      'js': 'javascript',
      'jsx': 'javascript',
      'ts': 'typescript',
      'tsx': 'typescript',
      'py': 'python',
      'java': 'java',
      'rb': 'ruby',
      'php': 'php',
      'go': 'go',
      'rs': 'rust'
    };
    return langMap[ext || ''] || 'unknown';
  }
  
  /**
   * Clear suggestion cache
   */
  clearCache(): void {
    this.suggestionCache.clear();
  }
}
