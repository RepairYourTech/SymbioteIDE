/**
 * Performance Analyzer - Detects performance issues and optimization opportunities
 */

import * as vscode from 'vscode';
import { Logger } from '../../utils/logger';
import {
  ReviewIssue,
  ReviewCategory,
  ReviewSeverity,
  CodeLocation,
  ReviewConfig
} from '../types';

export class PerformanceAnalyzer {
  private logger = new Logger('PerformanceAnalyzer');
  private config: ReviewConfig;
  
  // Performance anti-patterns
  private performancePatterns = [
    // Inefficient loops
    {
      pattern: /\.forEach\s*\([^)]*async/g,
      title: 'Async operations in forEach',
      description: 'forEach does not handle async operations properly. Use for...of or Promise.all',
      severity: ReviewSeverity.Error,
      rule: 'no-async-foreach'
    },
    {
      pattern: /for\s*\([^;]+in\s+[^)]+\)/g,
      title: 'for...in loop usage',
      description: 'for...in loops are slower than for...of or traditional for loops',
      severity: ReviewSeverity.Warning,
      rule: 'prefer-for-of'
    },
    // DOM manipulation
    {
      pattern: /for\s*\([^)]+\)\s*\{[^}]*(?:appendChild|innerHTML|insertBefore)/g,
      title: 'DOM manipulation in loop',
      description: 'Manipulating DOM in a loop causes reflows. Use DocumentFragment or batch updates',
      severity: ReviewSeverity.Error,
      rule: 'no-dom-in-loop'
    },
    {
      pattern: /\.style\.[\w]+\s*=/g,
      title: 'Direct style manipulation',
      description: 'Multiple style changes cause reflows. Use CSS classes instead',
      severity: ReviewSeverity.Info,
      rule: 'prefer-css-classes'
    },
    // Memory leaks
    {
      pattern: /addEventListener[^)]+\)(?!.*removeEventListener)/g,
      title: 'Event listener without cleanup',
      description: 'Event listeners should be removed to prevent memory leaks',
      severity: ReviewSeverity.Warning,
      rule: 'cleanup-event-listeners'
    },
    {
      pattern: /setInterval\s*\(/g,
      title: 'setInterval usage',
      description: 'setInterval can cause memory leaks if not cleared properly',
      severity: ReviewSeverity.Warning,
      rule: 'prefer-settimeout'
    },
    // Inefficient operations
    {
      pattern: /JSON\.parse\s*\(\s*JSON\.stringify/g,
      title: 'Inefficient deep clone',
      description: 'JSON parse/stringify is slow for deep cloning. Use structured cloning or libraries',
      severity: ReviewSeverity.Warning,
      rule: 'efficient-clone'
    },
    {
      pattern: /\.filter\([^)]+\)\s*\.map\(/g,
      title: 'Chained array operations',
      description: 'Multiple array iterations can be combined for better performance',
      severity: ReviewSeverity.Info,
      rule: 'combine-array-operations'
    },
    // React specific
    {
      pattern: /(?<!React\.memo\()function\s+\w+\s*\([^)]*\)\s*\{[^}]*return\s*<|const\s+\w+\s*=\s*\([^)]*\)\s*=>\s*[^{]*</g,
      title: 'Component without memoization',
      description: 'Consider using React.memo for pure components to prevent unnecessary re-renders',
      severity: ReviewSeverity.Info,
      rule: 'consider-memo'
    }
  ];
  
  // Complexity thresholds
  private complexityThresholds = {
    cyclomatic: { warning: 10, error: 20 },
    cognitive: { warning: 15, error: 30 },
    nestingDepth: { warning: 4, error: 6 },
    functionLength: { warning: 50, error: 100 },
    fileLength: { warning: 300, error: 500 }
  };
  
  constructor(config: ReviewConfig) {
    this.config = config;
  }
  
  /**
   * Analyze file for performance issues
   */
  async analyze(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    try {
      // Check performance patterns
      for (const pattern of this.performancePatterns) {
        if (this.isRuleDisabled(pattern.rule)) {
          continue;
        }
        
        const matches = content.matchAll(pattern.pattern);
        for (const match of matches) {
          const issue = this.createIssueFromMatch(
            file,
            content,
            match,
            pattern
          );
          if (issue) {
            issues.push(issue);
          }
        }
      }
      
      // Analyze code complexity
      issues.push(...this.analyzeComplexity(file, content));
      
      // Check for performance bottlenecks
      issues.push(...this.checkBottlenecks(file, content, context));
      
      // Language-specific performance checks
      const language = this.detectLanguage(file.fsPath);
      issues.push(...this.performLanguageSpecificChecks(file, content, language));
      
      // Analyze resource usage
      issues.push(...this.analyzeResourceUsage(file, content));
      
      // Check for optimization opportunities
      issues.push(...this.findOptimizationOpportunities(file, content, context));
      
      this.logger.info(`Found ${issues.length} performance issues in ${file.fsPath}`);
      
    } catch (error) {
      this.logger.error('Performance analysis failed', error);
    }
    
    return issues;
  }
  
  /**
   * Analyze code complexity
   */
  private analyzeComplexity(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const lines = content.split('\n');
    
    // File length check
    if (lines.length > this.complexityThresholds.fileLength.error) {
      issues.push(this.createGeneralIssue(
        file,
        'File too long',
        `File has ${lines.length} lines. Consider splitting into smaller modules`,
        ReviewSeverity.Error,
        'file-too-long'
      ));
    } else if (lines.length > this.complexityThresholds.fileLength.warning) {
      issues.push(this.createGeneralIssue(
        file,
        'File is getting long',
        `File has ${lines.length} lines. Consider refactoring`,
        ReviewSeverity.Warning,
        'file-length-warning'
      ));
    }
    
    // Function complexity
    const functions = this.extractFunctions(content);
    for (const func of functions) {
      // Cyclomatic complexity
      const complexity = this.calculateCyclomaticComplexity(func.body);
      if (complexity > this.complexityThresholds.cyclomatic.error) {
        issues.push(this.createFunctionIssue(
          file,
          func,
          'High cyclomatic complexity',
          `Function has complexity of ${complexity}. Consider breaking it down`,
          ReviewSeverity.Error,
          'high-complexity'
        ));
      } else if (complexity > this.complexityThresholds.cyclomatic.warning) {
        issues.push(this.createFunctionIssue(
          file,
          func,
          'Moderate cyclomatic complexity',
          `Function has complexity of ${complexity}. Consider simplifying`,
          ReviewSeverity.Warning,
          'moderate-complexity'
        ));
      }
      
      // Function length
      const funcLines = func.body.split('\n').length;
      if (funcLines > this.complexityThresholds.functionLength.error) {
        issues.push(this.createFunctionIssue(
          file,
          func,
          'Function too long',
          `Function has ${funcLines} lines. Break it into smaller functions`,
          ReviewSeverity.Error,
          'function-too-long'
        ));
      }
      
      // Nesting depth
      const maxNesting = this.calculateMaxNesting(func.body);
      if (maxNesting > this.complexityThresholds.nestingDepth.error) {
        issues.push(this.createFunctionIssue(
          file,
          func,
          'Excessive nesting',
          `Function has nesting depth of ${maxNesting}. Refactor to reduce nesting`,
          ReviewSeverity.Error,
          'excessive-nesting'
        ));
      }
    }
    
    return issues;
  }
  
  /**
   * Check for performance bottlenecks
   */
  private checkBottlenecks(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    // N+1 query patterns
    if (/\.map\s*\([^)]*async[^)]*(?:find|fetch|query|select)/gi.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Potential N+1 query problem',
        'Async database operations inside map can cause N+1 queries. Use batch operations',
        ReviewSeverity.Error,
        'n-plus-one-query'
      ));
    }
    
    // Synchronous file operations
    if (/(?:readFileSync|writeFileSync|existsSync)/.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Synchronous file operation',
        'Synchronous file operations block the event loop. Use async versions',
        ReviewSeverity.Error,
        'no-sync-operations'
      ));
    }
    
    // Large data in memory
    if (/new\s+Array\s*\(\s*\d{6,}\s*\)/.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Large array allocation',
        'Creating very large arrays can cause memory issues. Consider streaming or pagination',
        ReviewSeverity.Warning,
        'large-memory-allocation'
      ));
    }
    
    // Inefficient string concatenation
    if (/(?:for|while)[^{]*\{[^}]*\+=/g.test(content) && /string/i.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'String concatenation in loop',
        'String concatenation in loops is inefficient. Use array.join() or template literals',
        ReviewSeverity.Warning,
        'inefficient-string-concat'
      ));
    }
    
    return Promise.resolve(issues);
  }
  
  /**
   * Perform language-specific checks
   */
  private performLanguageSpecificChecks(
    file: vscode.Uri,
    content: string,
    language: string
  ): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    switch (language) {
      case 'javascript':
      case 'typescript':
        // Check for inefficient array methods
        if (/\.map\([^)]+\)\.filter\([^)]+\)\.reduce\(/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Multiple array iterations',
            'Chaining map, filter, and reduce creates multiple iterations. Consider combining',
            ReviewSeverity.Warning,
            'multiple-array-iterations'
          ));
        }
        
        // Missing async/await optimization
        if (/Promise\.all\s*\(\s*\[/.test(content) && /\.map\s*\(\s*async/.test(content)) {
          // This is actually good, no issue
        } else if (/for\s*\([^)]+\)\s*\{[^}]*await/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Sequential async operations',
            'Consider using Promise.all for parallel async operations when possible',
            ReviewSeverity.Info,
            'sequential-async'
          ));
        }
        break;
        
      case 'python':
        // List comprehension vs loops
        if (/for\s+\w+\s+in\s+[^:]+:\s*\n\s*\w+\.append\(/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Use list comprehension',
            'List comprehensions are more efficient than appending in a loop',
            ReviewSeverity.Info,
            'prefer-list-comprehension'
          ));
        }
        break;
        
      case 'java':
        // String concatenation in loops
        if (/for\s*\([^)]+\)\s*\{[^}]*\+\s*=\s*"/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Use StringBuilder',
            'String concatenation in loops is inefficient. Use StringBuilder',
            ReviewSeverity.Warning,
            'use-stringbuilder'
          ));
        }
        break;
    }
    
    return issues;
  }
  
  /**
   * Analyze resource usage
   */
  private analyzeResourceUsage(
    file: vscode.Uri,
    content: string
  ): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    // Check for resource leaks
    const openPatterns = [
      { open: /(?:open|connect|createConnection)\s*\(/, close: /(?:close|disconnect|end)\s*\(/, resource: 'connection' },
      { open: /(?:createReadStream|createWriteStream)\s*\(/, close: /(?:close|end|destroy)\s*\(/, resource: 'stream' },
      { open: /new\s+Worker\s*\(/, close: /terminate\s*\(/, resource: 'worker' }
    ];
    
    for (const pattern of openPatterns) {
      const opens = (content.match(pattern.open) || []).length;
      const closes = (content.match(pattern.close) || []).length;
      
      if (opens > closes) {
        issues.push(this.createGeneralIssue(
          file,
          `Potential ${pattern.resource} leak`,
          `Found ${opens} ${pattern.resource} opens but only ${closes} closes`,
          ReviewSeverity.Warning,
          `${pattern.resource}-leak`
        ));
      }
    }
    
    // Check for unbounded caches
    if (/(?:cache|memo)\s*\[[^\]]+\]\s*=/.test(content) && 
        !/(?:delete|clear|evict|limit|max)/i.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Unbounded cache detected',
        'Cache appears to have no size limit or eviction policy',
        ReviewSeverity.Warning,
        'unbounded-cache'
      ));
    }
    
    return issues;
  }
  
  /**
   * Find optimization opportunities
   */
  private findOptimizationOpportunities(
    file: vscode.Uri,
    content: string,
    context: any
  ): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    // Suggest memoization for expensive computations
    const expensivePatterns = [
      /function\s+\w+\s*\([^)]+\)\s*\{[^}]*(?:for|while)[^}]*\}/g,
      /=>\s*\{[^}]*(?:reduce|sort|filter).*(?:reduce|sort|filter)/g
    ];
    
    for (const pattern of expensivePatterns) {
      if (pattern.test(content) && !/(?:memo|cache)/i.test(content)) {
        issues.push(this.createGeneralIssue(
          file,
          'Consider memoization',
          'This function appears to perform expensive computations. Consider memoization',
          ReviewSeverity.Info,
          'consider-memoization'
        ));
        break;
      }
    }
    
    // Suggest lazy loading
    if (/import\s+.*\s+from\s+['"]/.test(content) && 
        content.split('\n').filter(line => line.includes('import')).length > 10) {
      issues.push(this.createGeneralIssue(
        file,
        'Many imports detected',
        'Consider lazy loading some modules to improve initial load time',
        ReviewSeverity.Info,
        'consider-lazy-loading'
      ));
    }
    
    // Suggest debouncing/throttling
    if (/addEventListener\s*\(['"](?:scroll|resize|input|mousemove)['"]/.test(content) &&
        !/(?:debounce|throttle)/i.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'High-frequency event handler',
        'Consider debouncing or throttling this event handler',
        ReviewSeverity.Warning,
        'debounce-throttle'
      ));
    }
    
    return issues;
  }
  
  /**
   * Extract functions from code
   */
  private extractFunctions(content: string): Array<{ name: string; body: string; line: number }> {
    const functions: Array<{ name: string; body: string; line: number }> = [];
    const lines = content.split('\n');
    
    // Simple function extraction (would need proper AST parsing for accuracy)
    const functionRegex = /(?:function\s+(\w+)|const\s+(\w+)\s*=\s*(?:async\s*)?(?:\([^)]*\)|\w+)\s*=>)/;
    
    let inFunction = false;
    let currentFunction: { name: string; body: string; line: number } | null = null;
    let braceCount = 0;
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const match = line.match(functionRegex);
      
      if (match && !inFunction) {
        currentFunction = {
          name: match[1] || match[2] || 'anonymous',
          body: '',
          line: i
        };
        inFunction = true;
      }
      
      if (inFunction && currentFunction) {
        currentFunction.body += line + '\n';
        braceCount += (line.match(/\{/g) || []).length;
        braceCount -= (line.match(/\}/g) || []).length;
        
        if (braceCount === 0 && line.includes('}')) {
          functions.push(currentFunction);
          inFunction = false;
          currentFunction = null;
          braceCount = 0;
        }
      }
    }
    
    return functions;
  }
  
  /**
   * Calculate cyclomatic complexity
   */
  private calculateCyclomaticComplexity(code: string): number {
    let complexity = 1; // Base complexity
    
    // Count decision points
    const decisionPatterns = [
      /\bif\s*\(/g,
      /\belse\s+if\s*\(/g,
      /\bwhile\s*\(/g,
      /\bfor\s*\(/g,
      /\bcase\s+/g,
      /\bcatch\s*\(/g,
      /\?.*:/g, // Ternary operators
      /\|\|/g, // Logical OR
      /&&/g // Logical AND
    ];
    
    for (const pattern of decisionPatterns) {
      const matches = code.match(pattern);
      if (matches) {
        complexity += matches.length;
      }
    }
    
    return complexity;
  }
  
  /**
   * Calculate maximum nesting depth
   */
  private calculateMaxNesting(code: string): number {
    let maxDepth = 0;
    let currentDepth = 0;
    
    for (const char of code) {
      if (char === '{') {
        currentDepth++;
        maxDepth = Math.max(maxDepth, currentDepth);
      } else if (char === '}') {
        currentDepth--;
      }
    }
    
    return maxDepth;
  }
  
  /**
   * Create issue from match
   */
  private createIssueFromMatch(
    file: vscode.Uri,
    content: string,
    match: RegExpMatchArray,
    pattern: any
  ): ReviewIssue | null {
    const position = this.getPositionFromIndex(content, match.index || 0);
    if (!position) return null;
    
    const line = content.split('\n')[position.line];
    const range = new vscode.Range(
      position,
      new vscode.Position(position.line, line.length)
    );
    
    return {
      id: `perf_${file.fsPath}_${position.line}_${Date.now()}`,
      category: ReviewCategory.Performance,
      severity: pattern.severity,
      title: pattern.title,
      description: pattern.description,
      location: {
        uri: file,
        range,
        snippet: line.trim(),
        context: {
          before: position.line > 0 ? content.split('\n')[position.line - 1] : '',
          after: position.line < content.split('\n').length - 1 ? 
            content.split('\n')[position.line + 1] : ''
        }
      },
      rule: pattern.rule,
      impact: this.getPerformanceImpact(pattern.severity),
      effort: this.getEffortEstimate(pattern.severity),
      tags: ['performance', pattern.rule],
      metadata: {
        pattern: pattern.pattern.source,
        matchedText: match[0]
      }
    };
  }
  
  /**
   * Create general issue
   */
  private createGeneralIssue(
    file: vscode.Uri,
    title: string,
    description: string,
    severity: ReviewSeverity,
    rule: string
  ): ReviewIssue {
    return {
      id: `perf_${file.fsPath}_${Date.now()}`,
      category: ReviewCategory.Performance,
      severity,
      title,
      description,
      location: {
        uri: file,
        range: new vscode.Range(0, 0, 0, 0),
        snippet: ''
      },
      rule,
      impact: this.getPerformanceImpact(severity),
      effort: this.getEffortEstimate(severity),
      tags: ['performance', rule]
    };
  }
  
  /**
   * Create function-specific issue
   */
  private createFunctionIssue(
    file: vscode.Uri,
    func: { name: string; body: string; line: number },
    title: string,
    description: string,
    severity: ReviewSeverity,
    rule: string
  ): ReviewIssue {
    const range = new vscode.Range(
      new vscode.Position(func.line, 0),
      new vscode.Position(func.line, 100)
    );
    
    return {
      id: `perf_${file.fsPath}_${func.line}_${Date.now()}`,
      category: ReviewCategory.Performance,
      severity,
      title: `${title} in ${func.name}()`,
      description,
      location: {
        uri: file,
        range,
        snippet: func.body.split('\n')[0].trim()
      },
      rule,
      impact: this.getPerformanceImpact(severity),
      effort: this.getEffortEstimate(severity),
      tags: ['performance', rule, 'complexity'],
      metadata: {
        functionName: func.name
      }
    };
  }
  
  /**
   * Get position from index
   */
  private getPositionFromIndex(content: string, index: number): vscode.Position | null {
    let line = 0;
    let character = 0;
    
    for (let i = 0; i < index && i < content.length; i++) {
      if (content[i] === '\n') {
        line++;
        character = 0;
      } else {
        character++;
      }
    }
    
    return new vscode.Position(line, character);
  }
  
  /**
   * Detect language
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
   * Check if rule is disabled
   */
  private isRuleDisabled(rule: string): boolean {
    return this.config.disabledRules.includes(rule);
  }
  
  /**
   * Get performance impact
   */
  private getPerformanceImpact(severity: ReviewSeverity): string {
    const impacts = {
      [ReviewSeverity.Critical]: 'Severe performance degradation that affects user experience',
      [ReviewSeverity.Error]: 'Significant performance issue that should be addressed',
      [ReviewSeverity.Warning]: 'Moderate performance impact that could be optimized',
      [ReviewSeverity.Info]: 'Minor performance improvement opportunity'
    };
    return impacts[severity];
  }
  
  /**
   * Get effort estimate
   */
  private getEffortEstimate(severity: ReviewSeverity): number {
    const efforts = {
      [ReviewSeverity.Critical]: 5,
      [ReviewSeverity.Error]: 3,
      [ReviewSeverity.Warning]: 2,
      [ReviewSeverity.Info]: 1
    };
    return efforts[severity];
  }
  
  /**
   * Update configuration
   */
  updateConfig(config: ReviewConfig): void {
    this.config = config;
  }
}
