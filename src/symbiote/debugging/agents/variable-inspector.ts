/**
 * Variable State Inspector Agent - Examines variable states and mutations
 */

import { Logger } from '../../utils/logger';
import { VariableState, ErrorContext, DebugAnalysis } from '../types';

export interface VariableInspectionResult {
  variables: VariableState[];
  suspiciousValues: VariableState[];
  nullUndefined: VariableState[];
  typeMismatches: TypeMismatch[];
  mutations: VariableMutation[];
  memoryIssues: MemoryIssue[];
  recommendations: string[];
  analysis: string;
}

export interface TypeMismatch {
  variable: string;
  expectedType: string;
  actualType: string;
  location: string;
  severity: 'warning' | 'error';
}

export interface VariableMutation {
  variable: string;
  beforeValue: any;
  afterValue: any;
  location: string;
  timestamp: number;
  causedBy: string;
}

export interface MemoryIssue {
  type: 'leak' | 'circular' | 'excessive' | 'retained';
  variable: string;
  description: string;
  impact: string;
}

export class VariableInspectorAgent {
  private logger = new Logger('VariableInspectorAgent');
  private suspiciousPatterns: Map<string, (value: any) => boolean> = new Map();
  
  constructor() {
    this.initializeSuspiciousPatterns();
  }
  
  /**
   * Initialize patterns for detecting suspicious values
   */
  private initializeSuspiciousPatterns(): void {
    // Null/undefined checks
    this.suspiciousPatterns.set('null-undefined', 
      (value) => value === null || value === undefined
    );
    
    // Empty arrays/objects that might cause issues
    this.suspiciousPatterns.set('empty-collection',
      (value) => (Array.isArray(value) && value.length === 0) ||
                 (typeof value === 'object' && value !== null && Object.keys(value).length === 0)
    );
    
    // NaN values
    this.suspiciousPatterns.set('nan-value',
      (value) => typeof value === 'number' && isNaN(value)
    );
    
    // Infinity values
    this.suspiciousPatterns.set('infinity-value',
      (value) => value === Infinity || value === -Infinity
    );
    
    // Very large arrays/strings (potential memory issues)
    this.suspiciousPatterns.set('large-collection',
      (value) => (Array.isArray(value) && value.length > 10000) ||
                 (typeof value === 'string' && value.length > 1000000)
    );
    
    // Circular references (basic check)
    this.suspiciousPatterns.set('potential-circular',
      (value) => {
        try {
          JSON.stringify(value);
          return false;
        } catch (e) {
          return true;
        }
      }
    );
  }
  
  /**
   * Inspect variables for issues
   */
  async inspect(params: {
    error: ErrorContext;
    code?: string;
    analysis?: DebugAnalysis;
    suspectedVariables?: string[];
    executionContext?: any;
  }): Promise<VariableInspectionResult> {
    try {
      const { error, code, analysis, suspectedVariables = [] } = params;
      
      // Extract variables from various sources
      const variables = await this.extractVariables(error, code, params.executionContext);
      
      // Check for suspicious values
      const suspiciousValues = this.identifySuspiciousValues(variables);
      
      // Check for null/undefined specifically
      const nullUndefined = this.findNullUndefined(variables, suspectedVariables);
      
      // Detect type mismatches
      const typeMismatches = await this.detectTypeMismatches(variables, code);
      
      // Track mutations
      const mutations = this.trackMutations(variables, params.executionContext);
      
      // Detect memory issues
      const memoryIssues = this.detectMemoryIssues(variables);
      
      // Generate recommendations
      const recommendations = this.generateRecommendations({
        suspiciousValues,
        nullUndefined,
        typeMismatches,
        mutations,
        memoryIssues,
        error,
        analysis
      });
      
      // Generate comprehensive analysis
      const analysisText = this.generateAnalysis({
        variables,
        suspiciousValues,
        nullUndefined,
        typeMismatches,
        mutations,
        memoryIssues
      });
      
      return {
        variables,
        suspiciousValues,
        nullUndefined,
        typeMismatches,
        mutations,
        memoryIssues,
        recommendations,
        analysis: analysisText
      };
      
    } catch (error) {
      this.logger.error('Variable inspection failed', error);
      throw error;
    }
  }
  
  /**
   * Extract variables from error context and code
   */
  private async extractVariables(
    error: ErrorContext,
    code?: string,
    executionContext?: any
  ): Promise<VariableState[]> {
    const variables: VariableState[] = [];
    
    // Extract from error message
    const errorVars = this.extractVariablesFromError(error);
    variables.push(...errorVars);
    
    // Extract from code context
    if (code && error.line) {
      const codeVars = this.extractVariablesFromCode(code, error.line);
      variables.push(...codeVars);
    }
    
    // Extract from execution context (if available from debugger)
    if (executionContext) {
      const contextVars = this.extractVariablesFromContext(executionContext);
      variables.push(...contextVars);
    }
    
    // Deduplicate by name
    const uniqueVars = new Map<string, VariableState>();
    variables.forEach(v => {
      if (!uniqueVars.has(v.name) || v.value !== undefined) {
        uniqueVars.set(v.name, v);
      }
    });
    
    return Array.from(uniqueVars.values());
  }
  
  /**
   * Extract variables mentioned in error message
   */
  private extractVariablesFromError(error: ErrorContext): VariableState[] {
    const variables: VariableState[] = [];
    const message = error.message;
    
    // Common patterns in error messages
    const patterns = [
      // Property access: "Cannot read property 'x' of undefined"
      /property '(\w+)' of (null|undefined)/g,
      // Variable name: "x is not defined"
      /(\w+) is not defined/g,
      // Type error: "x is not a function"
      /(\w+) is not a function/g,
      // Assignment: "Cannot set property 'x'"
      /Cannot set property '(\w+)'/g,
      // General variable reference
      /'(\w+)'/g,
    ];
    
    patterns.forEach(pattern => {
      let match;
      while ((match = pattern.exec(message)) !== null) {
        const varName = match[1];
        if (varName && !this.isKeyword(varName)) {
          variables.push({
            name: varName,
            value: match[2] || undefined, // e.g., "null" or "undefined"
            type: 'unknown',
            scope: 'unknown',
            mutated: false,
            accessCount: 1,
            suspiciousValue: true,
            suggestion: `Check if '${varName}' is properly initialized`
          });
        }
      }
    });
    
    return variables;
  }
  
  /**
   * Extract variables from code around error line
   */
  private extractVariablesFromCode(code: string, errorLine: number): VariableState[] {
    const variables: VariableState[] = [];
    const lines = code.split('\n');
    
    // Get context around error line
    const startLine = Math.max(0, errorLine - 10);
    const endLine = Math.min(lines.length, errorLine + 5);
    
    for (let i = startLine; i < endLine; i++) {
      const line = lines[i];
      
      // Variable declarations
      const varPatterns = [
        /(?:const|let|var)\s+(\w+)\s*=\s*(.+?)(?:;|$)/g,
        /(?:const|let|var)\s+\{([^}]+)\}\s*=/g,
        /(?:const|let|var)\s+\[([^\]]+)\]\s*=/g,
      ];
      
      varPatterns.forEach(pattern => {
        let match;
        while ((match = pattern.exec(line)) !== null) {
          const varName = match[1];
          const value = match[2];
          
          if (varName && !this.isKeyword(varName)) {
            variables.push({
              name: varName.trim(),
              value: this.parseValue(value),
              type: this.inferType(value),
              scope: this.inferScope(match[0]),
              mutated: false,
              accessCount: 0,
              suspiciousValue: false
            });
          }
        }
      });
      
      // Function parameters
      const funcPattern = /function\s*\w*\s*\(([^)]+)\)/g;
      let funcMatch;
      while ((funcMatch = funcPattern.exec(line)) !== null) {
        const params = funcMatch[1].split(',').map(p => p.trim());
        params.forEach(param => {
          if (param && !this.isKeyword(param)) {
            variables.push({
              name: param,
              value: undefined,
              type: 'parameter',
              scope: 'local',
              mutated: false,
              accessCount: 0,
              suspiciousValue: false
            });
          }
        });
      }
    }
    
    return variables;
  }
  
  /**
   * Extract variables from execution context
   */
  private extractVariablesFromContext(context: any): VariableState[] {
    const variables: VariableState[] = [];
    
    // This would integrate with debugger protocol
    // For now, parse if context is provided as object
    if (typeof context === 'object' && context !== null) {
      Object.entries(context).forEach(([name, value]) => {
        variables.push({
          name,
          value,
          type: this.getType(value),
          scope: 'execution',
          mutated: false,
          accessCount: 0,
          suspiciousValue: false
        });
      });
    }
    
    return variables;
  }
  
  /**
   * Identify suspicious values
   */
  private identifySuspiciousValues(variables: VariableState[]): VariableState[] {
    const suspicious: VariableState[] = [];
    
    variables.forEach(variable => {
      let isSuspicious = false;
      const suggestions: string[] = [];
      
      // Check against patterns
      this.suspiciousPatterns.forEach((checker, patternName) => {
        if (checker(variable.value)) {
          isSuspicious = true;
          
          switch (patternName) {
            case 'null-undefined':
              suggestions.push(`Variable '${variable.name}' is ${variable.value}. Add null check or initialize with default value.`);
              break;
            case 'empty-collection':
              suggestions.push(`Variable '${variable.name}' is empty. Check if this is expected or add validation.`);
              break;
            case 'nan-value':
              suggestions.push(`Variable '${variable.name}' is NaN. Check numeric operations and parsing.`);
              break;
            case 'infinity-value':
              suggestions.push(`Variable '${variable.name}' is Infinity. Check for division by zero.`);
              break;
            case 'large-collection':
              suggestions.push(`Variable '${variable.name}' is very large. Consider pagination or streaming.`);
              break;
            case 'potential-circular':
              suggestions.push(`Variable '${variable.name}' may have circular references.`);
              break;
          }
        }
      });
      
      if (isSuspicious) {
        variable.suspiciousValue = true;
        variable.suggestion = suggestions.join(' ');
        suspicious.push(variable);
      }
    });
    
    return suspicious;
  }
  
  /**
   * Find null/undefined variables
   */
  private findNullUndefined(
    variables: VariableState[],
    suspectedVariables: string[]
  ): VariableState[] {
    return variables.filter(v => 
      (v.value === null || v.value === undefined) ||
      suspectedVariables.includes(v.name)
    );
  }
  
  /**
   * Detect type mismatches
   */
  private async detectTypeMismatches(
    variables: VariableState[],
    code?: string
  ): Promise<TypeMismatch[]> {
    const mismatches: TypeMismatch[] = [];
    
    // This would integrate with TypeScript language service
    // For now, use heuristics
    
    variables.forEach(variable => {
      // Check for common type issues
      if (variable.name.endsWith('Count') || variable.name.endsWith('Index')) {
        if (typeof variable.value !== 'number' && variable.value !== undefined) {
          mismatches.push({
            variable: variable.name,
            expectedType: 'number',
            actualType: this.getType(variable.value),
            location: 'unknown',
            severity: 'warning'
          });
        }
      }
      
      if (variable.name.startsWith('is') || variable.name.startsWith('has')) {
        if (typeof variable.value !== 'boolean' && variable.value !== undefined) {
          mismatches.push({
            variable: variable.name,
            expectedType: 'boolean',
            actualType: this.getType(variable.value),
            location: 'unknown',
            severity: 'warning'
          });
        }
      }
      
      // Array method on non-array
      if (code && code.includes(`${variable.name}.map(`) || 
          code && code.includes(`${variable.name}.filter(`)) {
        if (!Array.isArray(variable.value) && variable.value !== undefined) {
          mismatches.push({
            variable: variable.name,
            expectedType: 'array',
            actualType: this.getType(variable.value),
            location: 'unknown',
            severity: 'error'
          });
        }
      }
    });
    
    return mismatches;
  }
  
  /**
   * Track variable mutations
   */
  private trackMutations(
    variables: VariableState[],
    executionContext?: any
  ): VariableMutation[] {
    const mutations: VariableMutation[] = [];
    
    // This would integrate with debugger stepping
    // For now, detect potential mutations from variable names and values
    
    variables.forEach(variable => {
      // Check if variable appears to have been mutated
      if (variable.name.includes('prev') || 
          variable.name.includes('old') ||
          variable.name.includes('original')) {
        
        // Look for corresponding current variable
        const currentVarName = variable.name
          .replace('prev', '')
          .replace('old', '')
          .replace('original', '');
        
        const currentVar = variables.find(v => 
          v.name === currentVarName || 
          v.name === 'new' + currentVarName
        );
        
        if (currentVar && currentVar.value !== variable.value) {
          mutations.push({
            variable: currentVarName,
            beforeValue: variable.value,
            afterValue: currentVar.value,
            location: 'unknown',
            timestamp: Date.now(),
            causedBy: 'state update'
          });
        }
      }
    });
    
    return mutations;
  }
  
  /**
   * Detect memory issues
   */
  private detectMemoryIssues(variables: VariableState[]): MemoryIssue[] {
    const issues: MemoryIssue[] = [];
    
    variables.forEach(variable => {
      // Large arrays or objects
      if (Array.isArray(variable.value) && variable.value.length > 10000) {
        issues.push({
          type: 'excessive',
          variable: variable.name,
          description: `Array contains ${variable.value.length} elements`,
          impact: 'High memory usage, potential performance degradation'
        });
      }
      
      // Very large strings
      if (typeof variable.value === 'string' && variable.value.length > 1000000) {
        issues.push({
          type: 'excessive',
          variable: variable.name,
          description: `String is ${(variable.value.length / 1000000).toFixed(2)}MB`,
          impact: 'High memory usage, consider streaming or chunking'
        });
      }
      
      // Potential circular references
      try {
        JSON.stringify(variable.value);
      } catch (e) {
        issues.push({
          type: 'circular',
          variable: variable.name,
          description: 'Contains circular references',
          impact: 'Cannot be serialized, may cause memory leaks'
        });
      }
      
      // Check for potential memory leaks (heuristic)
      if (variable.name.includes('cache') || 
          variable.name.includes('store') ||
          variable.name.includes('pool')) {
        
        if (typeof variable.value === 'object' && 
            variable.value !== null &&
            Object.keys(variable.value).length > 1000) {
          
          issues.push({
            type: 'leak',
            variable: variable.name,
            description: 'Large cache/store object that may grow unbounded',
            impact: 'Potential memory leak if not properly managed'
          });
        }
      }
    });
    
    return issues;
  }
  
  /**
   * Generate recommendations
   */
  private generateRecommendations(data: {
    suspiciousValues: VariableState[];
    nullUndefined: VariableState[];
    typeMismatches: TypeMismatch[];
    mutations: VariableMutation[];
    memoryIssues: MemoryIssue[];
    error?: ErrorContext;
    analysis?: DebugAnalysis;
  }): string[] {
    const recommendations: string[] = [];
    
    // Null/undefined recommendations
    if (data.nullUndefined.length > 0) {
      recommendations.push(
        'Add null/undefined checks before accessing properties',
        'Use optional chaining (?.) for safe property access',
        'Initialize variables with appropriate default values'
      );
    }
    
    // Type mismatch recommendations
    if (data.typeMismatches.length > 0) {
      recommendations.push(
        'Enable TypeScript strict mode for better type checking',
        'Add runtime type validation for external data',
        'Use type guards before operations'
      );
    }
    
    // Memory issue recommendations
    if (data.memoryIssues.length > 0) {
      const hasLeak = data.memoryIssues.some(i => i.type === 'leak');
      const hasCircular = data.memoryIssues.some(i => i.type === 'circular');
      
      if (hasLeak) {
        recommendations.push(
          'Implement proper cleanup and disposal patterns',
          'Use WeakMap/WeakSet for object references',
          'Add memory limits to caches and pools'
        );
      }
      
      if (hasCircular) {
        recommendations.push(
          'Break circular references before serialization',
          'Use object IDs instead of direct references',
          'Implement custom toJSON methods'
        );
      }
    }
    
    // Mutation recommendations
    if (data.mutations.length > 0) {
      recommendations.push(
        'Consider using immutable data structures',
        'Implement proper state management patterns',
        'Add mutation tracking for debugging'
      );
    }
    
    return [...new Set(recommendations)]; // Remove duplicates
  }
  
  /**
   * Generate analysis text
   */
  private generateAnalysis(data: {
    variables: VariableState[];
    suspiciousValues: VariableState[];
    nullUndefined: VariableState[];
    typeMismatches: TypeMismatch[];
    mutations: VariableMutation[];
    memoryIssues: MemoryIssue[];
  }): string {
    let analysis = '## Variable State Analysis\n\n';
    
    // Summary
    analysis += `**Total Variables Inspected**: ${data.variables.length}\n`;
    analysis += `**Suspicious Values Found**: ${data.suspiciousValues.length}\n`;
    analysis += `**Null/Undefined**: ${data.nullUndefined.length}\n`;
    analysis += `**Type Mismatches**: ${data.typeMismatches.length}\n\n`;
    
    // Suspicious values
    if (data.suspiciousValues.length > 0) {
      analysis += '### Suspicious Values\n';
      data.suspiciousValues.forEach(v => {
        analysis += `- **${v.name}**: ${v.suggestion || 'Requires attention'}\n`;
      });
      analysis += '\n';
    }
    
    // Type mismatches
    if (data.typeMismatches.length > 0) {
      analysis += '### Type Mismatches\n';
      data.typeMismatches.forEach(m => {
        analysis += `- **${m.variable}**: Expected ${m.expectedType}, got ${m.actualType}\n`;
      });
      analysis += '\n';
    }
    
    // Memory issues
    if (data.memoryIssues.length > 0) {
      analysis += '### Memory Concerns\n';
      data.memoryIssues.forEach(i => {
        analysis += `- **${i.variable}** (${i.type}): ${i.description}\n`;
      });
      analysis += '\n';
    }
    
    // Mutations
    if (data.mutations.length > 0) {
      analysis += '### Detected Mutations\n';
      data.mutations.forEach(m => {
        analysis += `- **${m.variable}**: Changed from ${JSON.stringify(m.beforeValue)} to ${JSON.stringify(m.afterValue)}\n`;
      });
      analysis += '\n';
    }
    
    return analysis;
  }
  
  /**
   * Helper: Check if string is a keyword
   */
  private isKeyword(str: string): boolean {
    const keywords = [
      'function', 'var', 'let', 'const', 'if', 'else', 'for', 'while',
      'return', 'break', 'continue', 'class', 'extends', 'new', 'this',
      'true', 'false', 'null', 'undefined', 'typeof', 'instanceof'
    ];
    return keywords.includes(str);
  }
  
  /**
   * Helper: Parse value from string
   */
  private parseValue(valueStr: string): any {
    if (!valueStr) return undefined;
    
    const trimmed = valueStr.trim();
    
    // Common literals
    if (trimmed === 'null') return null;
    if (trimmed === 'undefined') return undefined;
    if (trimmed === 'true') return true;
    if (trimmed === 'false') return false;
    
    // Numbers
    if (/^-?\d+(\.\d+)?$/.test(trimmed)) {
      return parseFloat(trimmed);
    }
    
    // Strings
    if (trimmed.startsWith('"') || trimmed.startsWith("'")) {
      return trimmed.slice(1, -1);
    }
    
    // Arrays
    if (trimmed.startsWith('[')) {
      return []; // Simplified
    }
    
    // Objects
    if (trimmed.startsWith('{')) {
      return {}; // Simplified
    }
    
    return trimmed;
  }
  
  /**
   * Helper: Infer type from value string
   */
  private inferType(valueStr: string): string {
    if (!valueStr) return 'unknown';
    
    const trimmed = valueStr.trim();
    
    if (trimmed === 'null' || trimmed === 'undefined') return trimmed;
    if (trimmed === 'true' || trimmed === 'false') return 'boolean';
    if (/^-?\d+(\.\d+)?$/.test(trimmed)) return 'number';
    if (trimmed.startsWith('"') || trimmed.startsWith("'")) return 'string';
    if (trimmed.startsWith('[')) return 'array';
    if (trimmed.startsWith('{')) return 'object';
    if (trimmed.includes('=>') || trimmed.includes('function')) return 'function';
    
    return 'unknown';
  }
  
  /**
   * Helper: Infer scope from declaration
   */
  private inferScope(declaration: string): 'local' | 'closure' | 'global' {
    if (declaration.includes('var')) return 'global';
    if (declaration.includes('const') || declaration.includes('let')) return 'local';
    return 'local';
  }
  
  /**
   * Helper: Get JavaScript type
   */
  private getType(value: any): string {
    if (value === null) return 'null';
    if (value === undefined) return 'undefined';
    if (Array.isArray(value)) return 'array';
    return typeof value;
  }
}