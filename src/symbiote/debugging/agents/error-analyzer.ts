/**
 * Error Analyzer Agent - Deep error analysis and pattern recognition
 */

import { Logger } from '../../utils/logger';
import { 
  ErrorContext, 
  DebugAnalysis, 
  ErrorPattern,
  DebugAgentResponse 
} from '../types';

export class ErrorAnalyzerAgent {
  private logger = new Logger('ErrorAnalyzerAgent');
  private knownPatterns: Map<string, ErrorPattern> = new Map();
  
  constructor() {
    this.initializeKnownPatterns();
  }
  
  /**
   * Initialize known error patterns
   */
  private initializeKnownPatterns(): void {
    // Common JavaScript/TypeScript error patterns
    const patterns: ErrorPattern[] = [
      {
        id: 'null-reference',
        name: 'Null Reference Error',
        description: 'Attempting to access property of null or undefined',
        category: 'runtime',
        examples: [
          "Cannot read property 'x' of undefined",
          "Cannot read properties of null",
          "TypeError: Cannot destructure property"
        ],
        frequency: 0,
        lastSeen: new Date(),
        solutions: [
          'Add null/undefined checks before property access',
          'Use optional chaining (?.) operator',
          'Initialize variables with default values',
          'Use nullish coalescing (??) for fallbacks'
        ]
      },
      {
        id: 'type-mismatch',
        name: 'Type Mismatch Error',
        description: 'Type incompatibility in TypeScript',
        category: 'compile',
        examples: [
          "Type 'string' is not assignable to type 'number'",
          "Argument of type 'X' is not assignable to parameter of type 'Y'"
        ],
        frequency: 0,
        lastSeen: new Date(),
        solutions: [
          'Verify type annotations match actual values',
          'Use type guards for runtime type checking',
          'Consider using union types for flexibility',
          'Add proper type conversions'
        ]
      },
      {
        id: 'async-uncaught',
        name: 'Uncaught Promise Rejection',
        description: 'Unhandled promise rejection in async code',
        category: 'runtime',
        examples: [
          'UnhandledPromiseRejectionWarning',
          'Uncaught (in promise)',
          'Promise rejected with no catch block'
        ],
        frequency: 0,
        lastSeen: new Date(),
        solutions: [
          'Add .catch() to promise chains',
          'Use try-catch with async/await',
          'Set up global unhandled rejection handler',
          'Ensure all async functions have error handling'
        ]
      },
      {
        id: 'memory-leak',
        name: 'Memory Leak',
        description: 'Memory consumption growing over time',
        category: 'performance',
        examples: [
          'JavaScript heap out of memory',
          'Maximum call stack size exceeded',
          'FATAL ERROR: Reached heap limit'
        ],
        frequency: 0,
        lastSeen: new Date(),
        solutions: [
          'Remove event listeners when components unmount',
          'Clear timers and intervals',
          'Avoid circular references',
          'Use WeakMap/WeakSet for object references',
          'Profile memory usage to find leaks'
        ]
      },
      {
        id: 'race-condition',
        name: 'Race Condition',
        description: 'Timing-dependent bugs in concurrent code',
        category: 'concurrency',
        examples: [
          'State updated after component unmounted',
          'Stale closure accessing outdated values',
          'Concurrent modifications to shared state'
        ],
        frequency: 0,
        lastSeen: new Date(),
        solutions: [
          'Use proper state management',
          'Implement mutex/semaphore patterns',
          'Cancel async operations on cleanup',
          'Use immutable data structures'
        ]
      }
    ];
    
    patterns.forEach(p => this.knownPatterns.set(p.id, p));
  }
  
  /**
   * Analyze error and provide deep insights
   */
  async analyze(
    error: ErrorContext,
    code?: string,
    history?: any[]
  ): Promise<DebugAnalysis> {
    try {
      // Pattern matching
      const patterns = this.identifyPatterns(error);
      
      // Root cause analysis
      const rootCause = this.analyzeRootCause(error, patterns, code);
      
      // Impact assessment
      const impact = this.assessImpact(error, code);
      
      // Generate suggestions
      const suggestions = this.generateSuggestions(error, patterns, rootCause);
      
      // Calculate confidence
      const confidence = this.calculateConfidence(patterns, rootCause, history);
      
      const analysis: DebugAnalysis = {
        rootCause,
        explanation: this.generateExplanation(error, rootCause, patterns),
        impact,
        patterns,
        suggestions,
        confidence,
        metadata: {
          analyzedAt: new Date(),
          codeContext: code ? this.extractCodeContext(code, error) : undefined,
          historicalSimilarity: history ? this.findSimilarErrors(error, history) : undefined
        }
      };
      
      // Update pattern frequency
      patterns.forEach(p => {
        p.frequency++;
        p.lastSeen = new Date();
      });
      
      return analysis;
      
    } catch (error) {
      this.logger.error('Error analysis failed', error);
      throw error;
    }
  }
  
  /**
   * Identify matching error patterns
   */
  private identifyPatterns(error: ErrorContext): ErrorPattern[] {
    const patterns: ErrorPattern[] = [];
    const errorMessage = error.message.toLowerCase();
    const errorType = error.type;
    
    for (const pattern of this.knownPatterns.values()) {
      // Check if error matches pattern examples
      const matches = pattern.examples.some(example => 
        errorMessage.includes(example.toLowerCase()) ||
        this.fuzzyMatch(errorMessage, example.toLowerCase())
      );
      
      if (matches || pattern.category === errorType) {
        patterns.push(pattern);
      }
    }
    
    // Sort by relevance (frequency and recency)
    patterns.sort((a, b) => {
      const scoreA = a.frequency * 0.7 + (Date.now() - a.lastSeen.getTime()) * 0.3;
      const scoreB = b.frequency * 0.7 + (Date.now() - b.lastSeen.getTime()) * 0.3;
      return scoreB - scoreA;
    });
    
    return patterns;
  }
  
  /**
   * Analyze root cause
   */
  private analyzeRootCause(
    error: ErrorContext, 
    patterns: ErrorPattern[],
    code?: string
  ): string {
    // Start with pattern-based analysis
    if (patterns.length > 0) {
      const primaryPattern = patterns[0];
      
      switch (primaryPattern.id) {
        case 'null-reference':
          return this.analyzeNullReference(error, code);
          
        case 'type-mismatch':
          return this.analyzeTypeMismatch(error, code);
          
        case 'async-uncaught':
          return this.analyzeAsyncError(error, code);
          
        case 'memory-leak':
          return this.analyzeMemoryLeak(error, code);
          
        case 'race-condition':
          return this.analyzeRaceCondition(error, code);
          
        default:
          return this.genericRootCauseAnalysis(error, code);
      }
    }
    
    return this.genericRootCauseAnalysis(error, code);
  }
  
  /**
   * Analyze null reference errors
   */
  private analyzeNullReference(error: ErrorContext, code?: string): string {
    const propertyMatch = error.message.match(/property '(\w+)' of (null|undefined)/);
    
    if (propertyMatch) {
      const property = propertyMatch[1];
      const nullType = propertyMatch[2];
      
      return `Attempting to access property '${property}' on a ${nullType} value. ` +
             `This typically occurs when: ` +
             `1) An object is not initialized before use, ` +
             `2) An async operation returns ${nullType}, ` +
             `3) A function returns ${nullType} unexpectedly, or ` +
             `4) Optional data is accessed without checking existence.`;
    }
    
    return 'Null or undefined reference detected. Variable was not properly initialized or checked.';
  }
  
  /**
   * Analyze type mismatch errors
   */
  private analyzeTypeMismatch(error: ErrorContext, code?: string): string {
    const typeMatch = error.message.match(/Type '(.+)' is not assignable to type '(.+)'/);
    
    if (typeMatch) {
      const actualType = typeMatch[1];
      const expectedType = typeMatch[2];
      
      return `Type mismatch: Expected type '${expectedType}' but received '${actualType}'. ` +
             `This indicates a contract violation where the actual data structure differs from the expected interface. ` +
             `Common causes include API response changes, incorrect type assertions, or missing type conversions.`;
    }
    
    return 'Type incompatibility detected. The provided value does not match the expected type signature.';
  }
  
  /**
   * Analyze async errors
   */
  private analyzeAsyncError(error: ErrorContext, code?: string): string {
    if (error.message.includes('promise')) {
      return 'Unhandled promise rejection detected. An asynchronous operation failed without proper error handling. ' +
             'This can lead to silent failures and unpredictable application state. ' +
             'Ensure all promises have .catch() handlers or are wrapped in try-catch blocks.';
    }
    
    return 'Asynchronous operation failed. Missing error handling in async/await or promise chain.';
  }
  
  /**
   * Analyze memory leaks
   */
  private analyzeMemoryLeak(error: ErrorContext, code?: string): string {
    return 'Memory leak detected. The application is retaining references preventing garbage collection. ' +
           'Common causes: uncleaned event listeners, circular references, growing arrays/maps, ' +
           'or retained DOM references. Use memory profiling to identify the source.';
  }
  
  /**
   * Analyze race conditions
   */
  private analyzeRaceCondition(error: ErrorContext, code?: string): string {
    return 'Potential race condition detected. Multiple async operations are accessing shared state ' +
           'in an unpredictable order. This can cause intermittent bugs that are difficult to reproduce. ' +
           'Consider using proper synchronization, state management, or restructuring async flows.';
  }
  
  /**
   * Generic root cause analysis
   */
  private genericRootCauseAnalysis(error: ErrorContext, code?: string): string {
    const errorLocation = error.file ? `in ${error.file}:${error.line}` : 'in unknown location';
    
    return `${error.type} error occurred ${errorLocation}. ` +
           `The error "${error.message}" suggests a ${error.type} issue that needs investigation. ` +
           `Check the error context and surrounding code for potential causes.`;
  }
  
  /**
   * Assess error impact
   */
  private assessImpact(error: ErrorContext, code?: string): DebugAnalysis['impact'] {
    const severity = this.calculateSeverity(error);
    const affectedFiles = this.findAffectedFiles(error, code);
    const affectedFunctions = this.findAffectedFunctions(error, code);
    const userImpact = this.assessUserImpact(error, severity);
    
    return {
      severity,
      affectedFiles,
      affectedFunctions,
      userImpact
    };
  }
  
  /**
   * Calculate error severity
   */
  private calculateSeverity(error: ErrorContext): 'low' | 'medium' | 'high' | 'critical' {
    // Critical errors
    if (error.type === 'memory' || 
        error.message.includes('heap') ||
        error.message.includes('stack overflow')) {
      return 'critical';
    }
    
    // High severity
    if (error.type === 'runtime' &&
        (error.reproducible === true || error.frequency! > 10)) {
      return 'high';
    }
    
    // Medium severity
    if (error.type === 'logic' || error.type === 'performance') {
      return 'medium';
    }
    
    // Low severity
    return 'low';
  }
  
  /**
   * Find affected files
   */
  private findAffectedFiles(error: ErrorContext, code?: string): string[] {
    const files: string[] = [];
    
    if (error.file) {
      files.push(error.file);
    }
    
    // Extract files from stack trace
    if (error.stack) {
      const fileMatches = error.stack.matchAll(/at .+ \((.+):\d+:\d+\)/g);
      for (const match of fileMatches) {
        if (match[1] && !files.includes(match[1])) {
          files.push(match[1]);
        }
      }
    }
    
    return files;
  }
  
  /**
   * Find affected functions
   */
  private findAffectedFunctions(error: ErrorContext, code?: string): string[] {
    const functions: string[] = [];
    
    // Extract from stack trace
    if (error.stack) {
      const funcMatches = error.stack.matchAll(/at (\w+)[\s.]/g);
      for (const match of funcMatches) {
        if (match[1] && !functions.includes(match[1])) {
          functions.push(match[1]);
        }
      }
    }
    
    return functions;
  }
  
  /**
   * Assess user impact
   */
  private assessUserImpact(error: ErrorContext, severity: string): string {
    const impactMap: Record<string, Record<string, string>> = {
      critical: {
        runtime: 'Application crash or unresponsive state',
        memory: 'System instability and potential data loss',
        performance: 'Severe degradation making app unusable'
      },
      high: {
        runtime: 'Feature failure affecting user workflow',
        logic: 'Incorrect behavior leading to wrong results',
        performance: 'Noticeable delays impacting user experience'
      },
      medium: {
        runtime: 'Minor feature disruption with workarounds',
        logic: 'Edge case issues affecting some users',
        performance: 'Occasional slowdowns'
      },
      low: {
        runtime: 'Cosmetic issues or rare edge cases',
        compile: 'Development-time issues only',
        performance: 'Minor optimization opportunities'
      }
    };
    
    return impactMap[severity]?.[error.type] || 'Minimal user impact';
  }
  
  /**
   * Generate suggestions
   */
  private generateSuggestions(
    error: ErrorContext,
    patterns: ErrorPattern[],
    rootCause: string
  ): string[] {
    const suggestions: string[] = [];
    
    // Add pattern-based suggestions
    patterns.forEach(pattern => {
      suggestions.push(...pattern.solutions);
    });
    
    // Add context-specific suggestions
    if (error.type === 'runtime' && error.message.includes('undefined')) {
      suggestions.push(
        'Add defensive programming checks',
        'Use TypeScript strict null checks',
        'Implement proper error boundaries'
      );
    }
    
    if (error.type === 'performance') {
      suggestions.push(
        'Profile code to identify bottlenecks',
        'Consider memoization or caching',
        'Optimize algorithms and data structures'
      );
    }
    
    // Remove duplicates and limit
    return [...new Set(suggestions)].slice(0, 5);
  }
  
  /**
   * Calculate confidence score
   */
  private calculateConfidence(
    patterns: ErrorPattern[],
    rootCause: string,
    history?: any[]
  ): number {
    let confidence = 0.5; // Base confidence
    
    // Pattern match bonus
    if (patterns.length > 0) {
      confidence += 0.2;
      if (patterns[0].frequency > 5) {
        confidence += 0.1;
      }
    }
    
    // Historical similarity bonus
    if (history && history.length > 0) {
      confidence += 0.1;
    }
    
    // Root cause specificity bonus
    if (rootCause.length > 100 && rootCause.includes('typically occurs')) {
      confidence += 0.1;
    }
    
    return Math.min(confidence, 0.95);
  }
  
  /**
   * Generate human-readable explanation
   */
  private generateExplanation(
    error: ErrorContext,
    rootCause: string,
    patterns: ErrorPattern[]
  ): string {
    let explanation = `## Error Analysis\n\n`;
    explanation += `**Error Type**: ${error.type}\n`;
    explanation += `**Message**: ${error.message}\n\n`;
    
    explanation += `### Root Cause\n${rootCause}\n\n`;
    
    if (patterns.length > 0) {
      explanation += `### Identified Patterns\n`;
      patterns.forEach(p => {
        explanation += `- **${p.name}**: ${p.description}\n`;
      });
      explanation += '\n';
    }
    
    if (error.file) {
      explanation += `### Location\n`;
      explanation += `File: ${error.file}:${error.line}:${error.column || 0}\n\n`;
    }
    
    return explanation;
  }
  
  /**
   * Extract relevant code context
   */
  private extractCodeContext(code: string, error: ErrorContext): any {
    if (!error.line) return null;
    
    const lines = code.split('\n');
    const startLine = Math.max(0, error.line - 5);
    const endLine = Math.min(lines.length, error.line + 5);
    
    return {
      snippet: lines.slice(startLine, endLine).join('\n'),
      startLine,
      endLine,
      errorLine: error.line
    };
  }
  
  /**
   * Find similar historical errors
   */
  private findSimilarErrors(error: ErrorContext, history: any[]): any {
    const similar = history.filter(h => {
      return h.error && (
        h.error.type === error.type ||
        this.fuzzyMatch(h.error.message, error.message) ||
        h.error.file === error.file
      );
    });
    
    return {
      count: similar.length,
      lastOccurrence: similar[0]?.timestamp,
      commonFixes: this.extractCommonFixes(similar)
    };
  }
  
  /**
   * Extract common fixes from history
   */
  private extractCommonFixes(similarErrors: any[]): string[] {
    const fixes: Record<string, number> = {};
    
    similarErrors.forEach(err => {
      if (err.solution?.description) {
        fixes[err.solution.description] = (fixes[err.solution.description] || 0) + 1;
      }
    });
    
    return Object.entries(fixes)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3)
      .map(([fix]) => fix);
  }
  
  /**
   * Fuzzy string matching
   */
  private fuzzyMatch(str1: string, str2: string): boolean {
    // Simple fuzzy match - can be improved with Levenshtein distance
    const words1 = str1.toLowerCase().split(/\s+/);
    const words2 = str2.toLowerCase().split(/\s+/);
    
    const commonWords = words1.filter(w => words2.includes(w));
    return commonWords.length >= Math.min(words1.length, words2.length) * 0.5;
  }
}