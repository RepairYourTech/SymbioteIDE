/**
 * Explanation Generator - Generates human-readable explanations for code review findings
 */

import { Logger } from '../../utils/logger';
import { ReviewAIInterface } from './review-ai-interface';
import { ReviewIssue, ReviewSeverity, CodeQualityMetrics } from '../types';

export interface ExplanationOptions {
  technicalLevel?: 'beginner' | 'intermediate' | 'expert';
  includeExamples?: boolean;
  language?: string;
}

export class ExplanationGenerator {
  private logger = new Logger('ExplanationGenerator');
  private aiInterface: ReviewAIInterface;
  
  constructor(aiInterface: ReviewAIInterface) {
    this.aiInterface = aiInterface;
  }
  
  /**
   * Generate review summary
   */
  async generateReviewSummary(data: {
    totalIssues: number;
    criticalIssues: number;
    errorIssues: number;
    warningIssues: number;
    infoIssues: number;
    suggestionsGenerated: number;
    metrics: CodeQualityMetrics;
    topCategories: Array<{ category: string; count: number }>;
    estimatedEffort: number;
  }): Promise<string> {
    try {
      const prompt = `Generate a concise executive summary for this code review:

Findings:
- Total issues: ${data.totalIssues}
- Critical: ${data.criticalIssues}
- Errors: ${data.errorIssues}
- Warnings: ${data.warningIssues}
- Info: ${data.infoIssues}
- Suggestions generated: ${data.suggestionsGenerated}

Top issue categories:
${data.topCategories.map(c => `- ${c.category}: ${c.count} issues`).join('\n')}

Code quality metrics:
- Complexity: ${data.metrics.complexity}
- Maintainability: ${data.metrics.maintainability}/100
- Security score: ${data.metrics.securityScore}/100
- Performance score: ${data.metrics.performanceScore}/100
- Test coverage: ${data.metrics.testCoverage}%
- Technical debt: ${data.metrics.technicalDebt} hours

Estimated effort to fix: ${data.estimatedEffort} story points

Provide a 3-4 paragraph summary covering:
1. Overall code quality assessment
2. Most significant issues found
3. Recommended priorities
4. Expected effort and impact`;
      
      const result = await this.aiInterface.request({
        prompt,
        maxTokens: 500,
        temperature: 0.3
      });
      
      return result.output;
      
    } catch (error) {
      this.logger.error('Failed to generate review summary', error);
      
      // Fallback summary
      return this.generateFallbackSummary(data);
    }
  }
  
  /**
   * Explain issue in detail
   */
  async explainIssue(
    issue: ReviewIssue,
    options: ExplanationOptions = {}
  ): Promise<string> {
    try {
      const technicalLevel = options.technicalLevel || 'intermediate';
      
      const prompt = `Explain this code issue for a ${technicalLevel} developer:

Issue: ${issue.title}
Category: ${issue.category}
Severity: ${issue.severity}
Description: ${issue.description}
${issue.rule ? `Rule: ${issue.rule}` : ''}
${issue.impact ? `Impact: ${issue.impact}` : ''}

Code context:
\`\`\`
${issue.location.snippet}
\`\`\`

Provide an explanation that includes:
1. What the issue is (in simple terms)
2. Why it matters
3. Potential consequences if not fixed
4. How to fix it
${options.includeExamples ? '5. Example of correct implementation' : ''}

Tailor the explanation for ${technicalLevel} level understanding.`;
      
      const result = await this.aiInterface.request({
        prompt,
        maxTokens: 800,
        temperature: 0.3
      });
      
      return result.output;
      
    } catch (error) {
      this.logger.error('Failed to explain issue', error);
      return issue.description;
    }
  }
  
  /**
   * Generate learning content
   */
  async generateLearningContent(
    issues: ReviewIssue[],
    options: ExplanationOptions = {}
  ): Promise<{
    lessons: Array<{
      topic: string;
      explanation: string;
      examples: string[];
      resources: string[];
    }>;
    recommendations: string[];
  }> {
    try {
      // Group issues by category
      const issuesByCategory = new Map<string, ReviewIssue[]>();
      for (const issue of issues) {
        const category = issue.category;
        if (!issuesByCategory.has(category)) {
          issuesByCategory.set(category, []);
        }
        issuesByCategory.get(category)!.push(issue);
      }
      
      // Generate lessons for top categories
      const lessons = [];
      const topCategories = Array.from(issuesByCategory.entries())
        .sort((a, b) => b[1].length - a[1].length)
        .slice(0, 3);
      
      for (const [category, categoryIssues] of topCategories) {
        const lesson = await this.generateLesson(
          category,
          categoryIssues,
          options
        );
        lessons.push(lesson);
      }
      
      // Generate recommendations
      const recommendations = await this.generateRecommendations(
        issues,
        issuesByCategory
      );
      
      return { lessons, recommendations };
      
    } catch (error) {
      this.logger.error('Failed to generate learning content', error);
      return { lessons: [], recommendations: [] };
    }
  }
  
  /**
   * Generate lesson for a category
   */
  private async generateLesson(
    category: string,
    issues: ReviewIssue[],
    options: ExplanationOptions
  ): Promise<any> {
    const issueExamples = issues.slice(0, 3).map(i => 
      `- ${i.title}: ${i.description}`
    ).join('\n');
    
    const prompt = `Create a learning lesson about ${category} issues:

Common issues found:
${issueExamples}

Generate a lesson in JSON format with:
- topic: lesson title
- explanation: comprehensive explanation of the concept
- examples: array of code examples (good vs bad)
- resources: array of learning resources/links

Target audience: ${options.technicalLevel || 'intermediate'} developers`;
    
    try {
      const result = await this.aiInterface.request({
        prompt,
        responseFormat: 'json',
        maxTokens: 1500
      });
      
      return JSON.parse(result.output);
    } catch (error) {
      return {
        topic: `Understanding ${category} issues`,
        explanation: `This lesson covers common ${category} problems found in your code.`,
        examples: [],
        resources: []
      };
    }
  }
  
  /**
   * Generate recommendations
   */
  private async generateRecommendations(
    issues: ReviewIssue[],
    issuesByCategory: Map<string, ReviewIssue[]>
  ): Promise<string[]> {
    const criticalCount = issues.filter(i => i.severity === ReviewSeverity.Critical).length;
    const errorCount = issues.filter(i => i.severity === ReviewSeverity.Error).length;
    
    const recommendations: string[] = [];
    
    // Priority recommendations
    if (criticalCount > 0) {
      recommendations.push(
        `Address ${criticalCount} critical issues immediately - these pose serious risks`
      );
    }
    
    if (errorCount > 5) {
      recommendations.push(
        'Consider implementing stricter code review processes to catch errors earlier'
      );
    }
    
    // Category-specific recommendations
    for (const [category, categoryIssues] of issuesByCategory) {
      if (categoryIssues.length > 10) {
        recommendations.push(
          `Focus on ${category} training - ${categoryIssues.length} issues found`
        );
      }
    }
    
    // Tool recommendations
    if (issuesByCategory.has('security')) {
      recommendations.push(
        'Consider integrating security scanning tools into your CI/CD pipeline'
      );
    }
    
    if (issuesByCategory.has('performance')) {
      recommendations.push(
        'Implement performance monitoring and profiling in development'
      );
    }
    
    return recommendations;
  }
  
  /**
   * Generate PR review comment
   */
  async generatePRComment(
    issues: ReviewIssue[],
    suggestions: number,
    metrics: CodeQualityMetrics
  ): Promise<string> {
    try {
      const severityCounts = {
        critical: issues.filter(i => i.severity === ReviewSeverity.Critical).length,
        error: issues.filter(i => i.severity === ReviewSeverity.Error).length,
        warning: issues.filter(i => i.severity === ReviewSeverity.Warning).length,
        info: issues.filter(i => i.severity === ReviewSeverity.Info).length
      };
      
      const prompt = `Generate a professional GitHub PR review comment:

Review Results:
- Total issues: ${issues.length}
- Critical: ${severityCounts.critical}
- Errors: ${severityCounts.error}
- Warnings: ${severityCounts.warning}
- Info: ${severityCounts.info}
- Automated suggestions: ${suggestions}

Code Quality:
- Security score: ${metrics.securityScore}/100
- Performance score: ${metrics.performanceScore}/100
- Maintainability: ${metrics.maintainability}/100

Top issues:
${issues.slice(0, 5).map(i => `- ${i.title} (${i.severity})`).join('\n')}

Generate a comment that:
1. Summarizes findings professionally
2. Highlights critical issues
3. Acknowledges good practices if scores are high
4. Provides actionable next steps
5. Uses GitHub markdown formatting`;
      
      const result = await this.aiInterface.request({
        prompt,
        maxTokens: 600,
        temperature: 0.3
      });
      
      return result.output;
      
    } catch (error) {
      this.logger.error('Failed to generate PR comment', error);
      return this.generateFallbackPRComment(issues, suggestions, metrics);
    }
  }
  
  /**
   * Generate technical debt report
   */
  async generateTechnicalDebtReport(
    issues: ReviewIssue[],
    metrics: CodeQualityMetrics
  ): Promise<string> {
    try {
      const debtByCategory = new Map<string, number>();
      let totalEffort = 0;
      
      for (const issue of issues) {
        const effort = issue.effort || 1;
        totalEffort += effort;
        debtByCategory.set(
          issue.category,
          (debtByCategory.get(issue.category) || 0) + effort
        );
      }
      
      const prompt = `Generate a technical debt report:

Total technical debt: ${metrics.technicalDebt} hours
Issues contributing to debt: ${issues.length}
Total effort to fix: ${totalEffort} story points

Debt by category:
${Array.from(debtByCategory.entries())
  .map(([cat, effort]) => `- ${cat}: ${effort} points`)
  .join('\n')}

Current quality metrics:
- Maintainability: ${metrics.maintainability}/100
- Test coverage: ${metrics.testCoverage}%
- Duplicate code: ${metrics.duplicateCode}%

Generate a report that includes:
1. Executive summary
2. Impact on development velocity
3. Prioritized remediation plan
4. Resource requirements
5. Expected ROI of addressing debt`;
      
      const result = await this.aiInterface.request({
        prompt,
        maxTokens: 1000,
        temperature: 0.3
      });
      
      return result.output;
      
    } catch (error) {
      this.logger.error('Failed to generate technical debt report', error);
      return 'Technical debt report generation failed';
    }
  }
  
  /**
   * Generate fallback summary
   */
  private generateFallbackSummary(data: any): string {
    const quality = data.metrics.maintainability >= 70 ? 'good' : 
                   data.metrics.maintainability >= 50 ? 'moderate' : 'poor';
    
    return `Code Review Summary

The code review identified ${data.totalIssues} issues across the codebase, with ${data.criticalIssues} critical issues requiring immediate attention.

Overall code quality is ${quality} with a maintainability score of ${data.metrics.maintainability}/100. The main areas of concern are in the ${data.topCategories[0]?.category || 'general'} category with ${data.topCategories[0]?.count || 0} issues.

The estimated effort to address all issues is ${data.estimatedEffort} story points. Priority should be given to critical security and performance issues.

${data.suggestionsGenerated} automated fix suggestions have been generated to help expedite the remediation process.`;
  }
  
  /**
   * Generate fallback PR comment
   */
  private generateFallbackPRComment(
    issues: ReviewIssue[],
    suggestions: number,
    metrics: CodeQualityMetrics
  ): string {
    const critical = issues.filter(i => i.severity === ReviewSeverity.Critical).length;
    const errors = issues.filter(i => i.severity === ReviewSeverity.Error).length;
    
    let status = '✅ Approved';
    let emoji = '👍';
    
    if (critical > 0) {
      status = '❌ Changes Required';
      emoji = '🚨';
    } else if (errors > 5) {
      status = '⚠️ Needs Attention';
      emoji = '⚠️';
    }
    
    return `## Code Review Results ${emoji}

**Status:** ${status}

### Summary
- 🔍 Total issues found: **${issues.length}**
- 🚨 Critical: **${critical}**
- ❌ Errors: **${errors}**
- ⚠️ Warnings: **${issues.filter(i => i.severity === ReviewSeverity.Warning).length}**
- ℹ️ Info: **${issues.filter(i => i.severity === ReviewSeverity.Info).length}**

### Code Quality Scores
- 🔒 Security: **${metrics.securityScore}/100**
- ⚡ Performance: **${metrics.performanceScore}/100**
- 🏗️ Maintainability: **${metrics.maintainability}/100**

### Top Issues
${issues.slice(0, 5).map((i, idx) => 
  `${idx + 1}. **${i.title}** (${i.severity}) - ${i.location.uri.path}`
).join('\n')}

### Automated Fixes
🤖 Generated **${suggestions}** automated fix suggestions

${critical > 0 ? '### ⚠️ Action Required\nPlease address critical issues before merging.' : 
  errors > 0 ? '### 📋 Recommended\nConsider fixing error-level issues to improve code quality.' :
  '### ✨ Good to merge\nOnly minor issues found.'}`;
  }
}
