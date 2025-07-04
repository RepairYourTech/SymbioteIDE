/**
 * Best Practices Analyzer - Checks code against coding best practices and standards
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

export class BestPracticesAnalyzer {
  private logger = new Logger('BestPracticesAnalyzer');
  private config: ReviewConfig;
  
  // Best practice patterns
  private bestPracticePatterns = [
    // Variable naming
    {
      pattern: /\b[a-z]\b(?!\s*:|\s*\(|\s*=>)/g,
      title: 'Single letter variable name',
      description: 'Use descriptive variable names instead of single letters (except for loop counters)',
      severity: ReviewSeverity.Warning,
      rule: 'descriptive-names'
    },
    // Magic numbers
    {
      pattern: /(?<!\.|\d)(?:86400|3600|1440|365|404|200|500)(?!\d)/g,
      title: 'Magic number detected',
      description: 'Define magic numbers as named constants',
      severity: ReviewSeverity.Info,
      rule: 'no-magic-numbers'
    },
    // Console statements
    {
      pattern: /console\.(log|error|warn|info|debug)\s*\(/g,
      title: 'Console statement found',
      description: 'Remove console statements before production',
      severity: ReviewSeverity.Warning,
      rule: 'no-console'
    },
    // TODO comments
    {
      pattern: /\/\/\s*(?:TODO|FIXME|HACK|XXX)\s*:/gi,
      title: 'Unresolved TODO comment',
      description: 'Address TODO comments before merging',
      severity: ReviewSeverity.Info,
      rule: 'no-todo-comments'
    },
    // Commented code
    {
      pattern: /\/\/\s*(?:function|class|const|let|var|if|for|while)\s+/g,
      title: 'Commented out code',
      description: 'Remove commented out code - use version control instead',
      severity: ReviewSeverity.Warning,
      rule: 'no-commented-code'
    },
    // Empty catch blocks
    {
      pattern: /catch\s*\([^)]*\)\s*\{\s*\}/g,
      title: 'Empty catch block',
      description: 'Handle or log errors in catch blocks',
      severity: ReviewSeverity.Error,
      rule: 'no-empty-catch'
    },
    // Nested ternary
    {
      pattern: /\?[^:]*\?[^:]*:[^:]*:/g,
      title: 'Nested ternary operator',
      description: 'Nested ternary operators are hard to read. Use if-else instead',
      severity: ReviewSeverity.Warning,
      rule: 'no-nested-ternary'
    },
    // Long lines
    {
      pattern: /^.{120,}$/gm,
      title: 'Line too long',
      description: 'Lines should not exceed 120 characters',
      severity: ReviewSeverity.Info,
      rule: 'max-line-length'
    }
  ];
  
  // Code quality checks
  private qualityChecks = {
    // DRY principle
    duplicateThreshold: 50, // characters
    // Function parameters
    maxParams: 5,
    // Return statements
    maxReturns: 3,
    // Import count
    maxImports: 20
  };
  
  constructor(config: ReviewConfig) {
    this.config = config;
  }
  
  /**
   * Analyze file for best practice violations
   */
  async analyze(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    try {
      // Check best practice patterns
      for (const pattern of this.bestPracticePatterns) {
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
      
      // Check code organization
      issues.push(...this.checkCodeOrganization(file, content));
      
      // Check naming conventions
      issues.push(...this.checkNamingConventions(file, content));
      
      // Check error handling
      issues.push(...this.checkErrorHandling(file, content));
      
      // Check documentation
      issues.push(...this.checkDocumentation(file, content));
      
      // Check code duplication
      issues.push(...this.checkDuplication(file, content));
      
      // Language-specific best practices
      const language = this.detectLanguage(file.fsPath);
      issues.push(...this.checkLanguageSpecificPractices(file, content, language));
      
      // Check test coverage hints
      if (context.testCoverage !== undefined && context.testCoverage < 80) {
        issues.push(this.createGeneralIssue(
          file,
          'Low test coverage',
          `Test coverage is ${context.testCoverage}%. Aim for at least 80%`,
          ReviewSeverity.Warning,
          'low-test-coverage'
        ));
      }
      
      this.logger.info(`Found ${issues.length} best practice issues in ${file.fsPath}`);
      
    } catch (error) {
      this.logger.error('Best practices analysis failed', error);
    }
    
    return issues;
  }
  
  /**
   * Check code organization
   */
  private checkCodeOrganization(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const lines = content.split('\n');
    
    // Check import organization
    const importLines = lines.filter(line => line.trim().startsWith('import'));
    if (importLines.length > this.qualityChecks.maxImports) {
      issues.push(this.createGeneralIssue(
        file,
        'Too many imports',
        `File has ${importLines.length} imports. Consider splitting the module`,
        ReviewSeverity.Warning,
        'too-many-imports'
      ));
    }
    
    // Check for mixed concerns
    const hasUI = /(?:render|component|jsx|view)/i.test(content);
    const hasDB = /(?:query|database|sql|mongo|redis)/i.test(content);
    const hasAPI = /(?:fetch|axios|request|endpoint)/i.test(content);
    
    const concerns = [hasUI, hasDB, hasAPI].filter(Boolean).length;
    if (concerns > 1) {
      issues.push(this.createGeneralIssue(
        file,
        'Multiple concerns in single file',
        'Consider separating UI, data access, and API logic into different modules',
        ReviewSeverity.Warning,
        'separation-of-concerns'
      ));
    }
    
    // Check function organization
    const functions = this.extractFunctions(content);
    const publicFunctions = functions.filter(f => !f.name.startsWith('_') && !f.name.startsWith('private'));
    const privateFunctions = functions.filter(f => f.name.startsWith('_') || f.name.startsWith('private'));
    
    if (publicFunctions.length > 0 && privateFunctions.length > 0) {
      // Check if private functions come after public
      const lastPublicIndex = Math.max(...publicFunctions.map(f => f.line));
      const firstPrivateIndex = Math.min(...privateFunctions.map(f => f.line));
      
      if (firstPrivateIndex < lastPublicIndex) {
        issues.push(this.createGeneralIssue(
          file,
          'Mixed function visibility',
          'Group public functions together, followed by private functions',
          ReviewSeverity.Info,
          'function-organization'
        ));
      }
    }
    
    return issues;
  }
  
  /**
   * Check naming conventions
   */
  private checkNamingConventions(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const language = this.detectLanguage(file.fsPath);
    
    // JavaScript/TypeScript naming conventions
    if (['javascript', 'typescript'].includes(language)) {
      // Check for snake_case in JS (except for constants)
      const snakeCaseVars = content.match(/(?:let|var|const)\s+(\w*_\w+)(?!\s*=\s*[A-Z])/g);
      if (snakeCaseVars) {
        issues.push(this.createGeneralIssue(
          file,
          'Snake case variable names',
          'Use camelCase for JavaScript/TypeScript variables',
          ReviewSeverity.Warning,
          'naming-convention'
        ));
      }
      
      // Check class naming
      const classNames = content.match(/class\s+([a-z]\w*)/g);
      if (classNames) {
        issues.push(this.createGeneralIssue(
          file,
          'Class name not PascalCase',
          'Class names should start with uppercase letter',
          ReviewSeverity.Error,
          'class-naming'
        ));
      }
      
      // Check constant naming
      const constants = content.match(/const\s+[a-z_]+\s*=\s*(?:\d+|'[^']*'|"[^"]*")/g);
      if (constants && content.includes('export const')) {
        issues.push(this.createGeneralIssue(
          file,
          'Exported constants not UPPER_CASE',
          'Exported constants should use UPPER_SNAKE_CASE',
          ReviewSeverity.Info,
          'constant-naming'
        ));
      }
    }
    
    // Python naming conventions
    if (language === 'python') {
      // Check for camelCase in Python
      const camelCaseFuncs = content.match(/def\s+[a-z]+[A-Z]\w*\s*\(/g);
      if (camelCaseFuncs) {
        issues.push(this.createGeneralIssue(
          file,
          'CamelCase function names',
          'Use snake_case for Python function names',
          ReviewSeverity.Warning,
          'python-naming'
        ));
      }
    }
    
    return issues;
  }
  
  /**
   * Check error handling practices
   */
  private checkErrorHandling(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    // Check for error handling in async functions
    const asyncFuncs = content.match(/async\s+(?:function\s+)?\w*\s*\([^)]*\)\s*\{[^}]+\}/g) || [];
    
    for (const func of asyncFuncs) {
      if (!func.includes('try') && !func.includes('.catch')) {
        issues.push(this.createGeneralIssue(
          file,
          'Async function without error handling',
          'Async functions should handle errors with try-catch or .catch()',
          ReviewSeverity.Error,
          'async-error-handling'
        ));
        break; // Only report once per file
      }
    }
    
    // Check for Promise without catch
    if (/\.then\s*\([^)]+\)(?!\s*\.catch)/g.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Promise without error handling',
        'Promises should have .catch() for error handling',
        ReviewSeverity.Warning,
        'promise-catch'
      ));
    }
    
    // Check for generic error messages
    if (/["'](?:Error|Failed|Something went wrong)["']/g.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Generic error messages',
        'Use specific, actionable error messages',
        ReviewSeverity.Info,
        'specific-errors'
      ));
    }
    
    // Check for error swallowing
    if (/catch\s*\([^)]*\)\s*\{\s*(?:\/\/.*)?\s*\}/g.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Error swallowing',
        'Errors should be logged or re-thrown, not silently ignored',
        ReviewSeverity.Error,
        'error-swallowing'
      ));
    }
    
    return issues;
  }
  
  /**
   * Check documentation practices
   */
  private checkDocumentation(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const functions = this.extractFunctions(content);
    
    // Check for missing function documentation
    const exportedFunctions = functions.filter(f => 
      content.includes(`export function ${f.name}`) ||
      content.includes(`export const ${f.name}`) ||
      content.includes(`export async function ${f.name}`)
    );
    
    for (const func of exportedFunctions) {
      const funcIndex = content.indexOf(`function ${func.name}`);
      if (funcIndex === -1) continue;
      
      // Check if there's a comment before the function
      const beforeFunc = content.substring(Math.max(0, funcIndex - 200), funcIndex);
      if (!beforeFunc.includes('/**') && !beforeFunc.includes('//')) {
        issues.push(this.createFunctionIssue(
          file,
          func,
          'Missing function documentation',
          'Exported functions should have JSDoc comments',
          ReviewSeverity.Warning,
          'missing-jsdoc'
        ));
      }
    }
    
    // Check for outdated comments
    const todoComments = content.match(/\/\/\s*TODO.*\d{4}/g) || [];
    const currentYear = new Date().getFullYear();
    
    for (const comment of todoComments) {
      const yearMatch = comment.match(/\d{4}/);
      if (yearMatch && parseInt(yearMatch[0]) < currentYear - 1) {
        issues.push(this.createGeneralIssue(
          file,
          'Outdated TODO comment',
          'TODO comment is over a year old - address or remove it',
          ReviewSeverity.Warning,
          'outdated-todo'
        ));
      }
    }
    
    // Check for missing file header
    if (!content.startsWith('/**') && !content.startsWith('//')) {
      issues.push(this.createGeneralIssue(
        file,
        'Missing file header',
        'Files should start with a comment describing their purpose',
        ReviewSeverity.Info,
        'file-header'
      ));
    }
    
    return issues;
  }
  
  /**
   * Check for code duplication
   */
  private checkDuplication(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const lines = content.split('\n');
    const duplicates = new Map<string, number[]>();
    
    // Simple duplicate detection (would need more sophisticated algorithm)
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      
      // Skip empty lines, comments, and short lines
      if (line.length < this.qualityChecks.duplicateThreshold || 
          line.startsWith('//') || 
          line.startsWith('*') ||
          line === '{' || line === '}') {
        continue;
      }
      
      // Check for similar lines
      const similar = duplicates.get(line) || [];
      similar.push(i);
      duplicates.set(line, similar);
    }
    
    // Report duplicates
    for (const [line, occurrences] of duplicates) {
      if (occurrences.length > 2) {
        issues.push(this.createGeneralIssue(
          file,
          'Duplicate code detected',
          `Line "${line.substring(0, 50)}..." appears ${occurrences.length} times`,
          ReviewSeverity.Warning,
          'code-duplication'
        ));
        break; // Only report most significant
      }
    }
    
    // Check for similar functions
    const functions = this.extractFunctions(content);
    const funcBodies = new Map<string, string[]>();
    
    for (const func of functions) {
      const normalizedBody = func.body
        .replace(/\s+/g, ' ')
        .replace(/[a-zA-Z_]\w*/g, 'VAR') // Normalize variable names
        .trim();
      
      const similar = funcBodies.get(normalizedBody) || [];
      similar.push(func.name);
      funcBodies.set(normalizedBody, similar);
    }
    
    for (const [body, names] of funcBodies) {
      if (names.length > 1) {
        issues.push(this.createGeneralIssue(
          file,
          'Similar functions detected',
          `Functions ${names.join(', ')} have very similar implementations`,
          ReviewSeverity.Warning,
          'duplicate-functions'
        ));
      }
    }
    
    return issues;
  }
  
  /**
   * Check language-specific best practices
   */
  private checkLanguageSpecificPractices(
    file: vscode.Uri,
    content: string,
    language: string
  ): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    switch (language) {
      case 'javascript':
      case 'typescript':
        // Check for var usage
        if (/\bvar\s+/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Using var instead of let/const',
            'Use let or const instead of var for better scoping',
            ReviewSeverity.Warning,
            'no-var'
          ));
        }
        
        // Check for == instead of ===
        if (/[^=!]==[^=]/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Using == instead of ===',
            'Use === for strict equality comparison',
            ReviewSeverity.Warning,
            'strict-equality'
          ));
        }
        
        // Check for missing semicolons (if not using a formatter)
        const needsSemicolon = /[^;{}]\s*\n\s*[\(\[]/.test(content);
        if (needsSemicolon) {
          issues.push(this.createGeneralIssue(
            file,
            'Potential missing semicolon',
            'Missing semicolons can cause unexpected behavior',
            ReviewSeverity.Warning,
            'missing-semicolon'
          ));
        }
        break;
        
      case 'python':
        // Check for mutable default arguments
        if (/def\s+\w+\s*\([^)]*=\s*(?:\[|\{)/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Mutable default argument',
            'Avoid mutable default arguments in Python functions',
            ReviewSeverity.Error,
            'mutable-default-arg'
          ));
        }
        
        // Check for bare except
        if (/except\s*:/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Bare except clause',
            'Specify exception types in except clauses',
            ReviewSeverity.Warning,
            'bare-except'
          ));
        }
        break;
        
      case 'java':
        // Check for System.out.println
        if (/System\.out\.println/.test(content)) {
          issues.push(this.createGeneralIssue(
            file,
            'Using System.out.println',
            'Use a proper logging framework instead of System.out',
            ReviewSeverity.Warning,
            'use-logger'
          ));
        }
        break;
    }
    
    return issues;
  }
  
  /**
   * Extract functions from code
   */
  private extractFunctions(content: string): Array<{ name: string; body: string; line: number }> {
    const functions: Array<{ name: string; body: string; line: number }> = [];
    const lines = content.split('\n');
    
    const functionPatterns = [
      /function\s+(\w+)\s*\(/,
      /const\s+(\w+)\s*=\s*(?:async\s*)?(?:\([^)]*\)|\w+)\s*=>/,
      /(\w+)\s*\([^)]*\)\s*\{/, // Method in class
      /def\s+(\w+)\s*\(/ // Python
    ];
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      
      for (const pattern of functionPatterns) {
        const match = line.match(pattern);
        if (match) {
          functions.push({
            name: match[1],
            body: line, // Simplified - would need proper parsing
            line: i
          });
          break;
        }
      }
    }
    
    return functions;
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
      id: `bp_${file.fsPath}_${position.line}_${Date.now()}`,
      category: ReviewCategory.BestPractices,
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
      impact: this.getBestPracticeImpact(pattern.severity),
      effort: this.getEffortEstimate(pattern.severity),
      tags: ['best-practices', pattern.rule]
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
      id: `bp_${file.fsPath}_${Date.now()}`,
      category: ReviewCategory.BestPractices,
      severity,
      title,
      description,
      location: {
        uri: file,
        range: new vscode.Range(0, 0, 0, 0),
        snippet: ''
      },
      rule,
      impact: this.getBestPracticeImpact(severity),
      effort: this.getEffortEstimate(severity),
      tags: ['best-practices', rule]
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
      id: `bp_${file.fsPath}_${func.line}_${Date.now()}`,
      category: ReviewCategory.BestPractices,
      severity,
      title: `${title} in ${func.name}()`,
      description,
      location: {
        uri: file,
        range,
        snippet: func.body.trim()
      },
      rule,
      impact: this.getBestPracticeImpact(severity),
      effort: this.getEffortEstimate(severity),
      tags: ['best-practices', rule],
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
      'rs': 'rust',
      'cpp': 'cpp',
      'c': 'c',
      'cs': 'csharp'
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
   * Get best practice impact
   */
  private getBestPracticeImpact(severity: ReviewSeverity): string {
    const impacts = {
      [ReviewSeverity.Critical]: 'Critical code quality issue that needs immediate attention',
      [ReviewSeverity.Error]: 'Significant deviation from best practices',
      [ReviewSeverity.Warning]: 'Code quality issue that should be addressed',
      [ReviewSeverity.Info]: 'Minor code quality improvement suggestion'
    };
    return impacts[severity];
  }
  
  /**
   * Get effort estimate
   */
  private getEffortEstimate(severity: ReviewSeverity): number {
    const efforts = {
      [ReviewSeverity.Critical]: 3,
      [ReviewSeverity.Error]: 2,
      [ReviewSeverity.Warning]: 1,
      [ReviewSeverity.Info]: 0.5
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
