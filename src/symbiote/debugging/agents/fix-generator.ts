/**
 * Fix Generator Agent - Creates and validates fixes for errors
 */

import { Logger } from '../../utils/logger';
import { 
  ErrorContext, 
  DebugAnalysis, 
  DebugFix, 
  TestCoverage,
  DebugPreferences,
  VariableState 
} from '../types';
import { VariableInspectionResult } from './variable-inspector';

export interface FixGenerationRequest {
  error: ErrorContext;
  code?: string;
  analysis?: DebugAnalysis;
  variableIssues?: VariableInspectionResult;
  preferences?: DebugPreferences;
  context?: any;
}

export interface FixGenerationResult {
  fixes: DebugFix[];
  primaryFix: DebugFix;
  alternativeFixes: DebugFix[];
  testSuggestions: TestSuggestion[];
  preventionStrategies: string[];
  confidence: number;
}

export interface TestSuggestion {
  type: 'unit' | 'integration' | 'e2e';
  description: string;
  framework?: string;
  code?: string;
  coverage: string[];
}

export class FixGeneratorAgent {
  private logger = new Logger('FixGeneratorAgent');
  private fixTemplates: Map<string, FixTemplate> = new Map();
  
  constructor() {
    this.initializeFixTemplates();
  }
  
  /**
   * Initialize fix templates for common error patterns
   */
  private initializeFixTemplates(): void {
    // Null/undefined fix templates
    this.fixTemplates.set('null-check', {
      pattern: /Cannot read property .+ of (null|undefined)/,
      generate: (context) => ({
        code: this.generateNullCheckFix(context),
        explanation: 'Added null/undefined check before property access',
        confidence: 0.9,
        impact: 'minimal'
      })
    });
    
    // Type mismatch fix templates
    this.fixTemplates.set('type-conversion', {
      pattern: /Type '.+' is not assignable to type '.+'/,
      generate: (context) => ({
        code: this.generateTypeConversionFix(context),
        explanation: 'Added type conversion to match expected type',
        confidence: 0.8,
        impact: 'minimal'
      })
    });
    
    // Async error fix templates
    this.fixTemplates.set('async-handling', {
      pattern: /Unhandled promise rejection/,
      generate: (context) => ({
        code: this.generateAsyncHandlingFix(context),
        explanation: 'Added proper async error handling',
        confidence: 0.85,
        impact: 'moderate'
      })
    });
    
    // Array method fix templates
    this.fixTemplates.set('array-safety', {
      pattern: /.+ is not a function|Cannot read property .+ of undefined/,
      generate: (context) => ({
        code: this.generateArraySafetyFix(context),
        explanation: 'Added array validation before method call',
        confidence: 0.85,
        impact: 'minimal'
      })
    });
  }
  
  /**
   * Generate fixes for the error
   */
  async generate(request: FixGenerationRequest): Promise<FixGenerationResult> {
    try {
      const { error, code, analysis, variableIssues, preferences } = request;
      
      // Generate multiple fix approaches
      const fixes = await this.generateMultipleFixes(request);
      
      // Rank fixes by confidence and impact
      const rankedFixes = this.rankFixes(fixes, preferences);
      
      // Select primary fix
      const primaryFix = rankedFixes[0];
      const alternativeFixes = rankedFixes.slice(1);
      
      // Generate test suggestions
      const testSuggestions = this.generateTestSuggestions(
        primaryFix,
        error,
        code
      );
      
      // Generate prevention strategies
      const preventionStrategies = this.generatePreventionStrategies(
        error,
        analysis,
        variableIssues
      );
      
      // Calculate overall confidence
      const confidence = this.calculateOverallConfidence(
        fixes,
        analysis,
        variableIssues
      );
      
      return {
        fixes: rankedFixes,
        primaryFix,
        alternativeFixes,
        testSuggestions,
        preventionStrategies,
        confidence
      };
      
    } catch (error) {
      this.logger.error('Fix generation failed', error);
      throw error;
    }
  }
  
  /**
   * Generate multiple fix approaches
   */
  private async generateMultipleFixes(
    request: FixGenerationRequest
  ): Promise<DebugFix[]> {
    const fixes: DebugFix[] = [];
    const { error, code, analysis, variableIssues } = request;
    
    // Template-based fixes
    this.fixTemplates.forEach((template, name) => {
      if (template.pattern.test(error.message)) {
        const fix = template.generate({
          error,
          code,
          analysis,
          variableIssues
        });
        
        fixes.push({
          id: `fix_${name}_${Date.now()}`,
          description: `Apply ${name} fix`,
          ...fix
        });
      }
    });
    
    // Context-aware fixes
    if (analysis?.patterns) {
      analysis.patterns.forEach(pattern => {
        const patternFixes = this.generatePatternBasedFixes(
          pattern,
          error,
          code
        );
        fixes.push(...patternFixes);
      });
    }
    
    // Variable-based fixes
    if (variableIssues) {
      const varFixes = this.generateVariableBasedFixes(variableIssues, code);
      fixes.push(...varFixes);
    }
    
    // Defensive programming fix (always include)
    fixes.push(this.generateDefensiveFix(error, code));
    
    // Refactoring fix (for complex issues)
    if (this.isComplexError(error, analysis)) {
      fixes.push(this.generateRefactoringFix(error, code, analysis));
    }
    
    return fixes;
  }
  
  /**
   * Generate null check fix
   */
  private generateNullCheckFix(context: any): string {
    const { error, code } = context;
    
    // Extract property and object from error
    const match = error.message.match(/Cannot read property '(\w+)' of (null|undefined)/);
    if (!match) return code || '';
    
    const property = match[1];
    const nullType = match[2];
    
    // Find the problematic line
    if (!code || !error.line) return code || '';
    
    const lines = code.split('\n');
    const errorLine = lines[error.line - 1];
    
    // Generate fix based on context
    const objectMatch = errorLine.match(/(\w+)\.${property}/);
    if (objectMatch) {
      const object = objectMatch[1];
      
      // Option 1: Optional chaining (modern)
      const modernFix = errorLine.replace(
        `${object}.${property}`,
        `${object}?.${property}`
      );
      
      // Option 2: Traditional null check
      const traditionalFix = `if (${object} != null) {\n  ${errorLine}\n}`;
      
      // Return modern fix as primary
      lines[error.line - 1] = modernFix;
      return lines.join('\n');
    }
    
    return code;
  }
  
  /**
   * Generate type conversion fix
   */
  private generateTypeConversionFix(context: any): string {
    const { error, code } = context;
    
    // Extract type information
    const match = error.message.match(/Type '(.+)' is not assignable to type '(.+)'/);
    if (!match) return code || '';
    
    const actualType = match[1];
    const expectedType = match[2];
    
    // Generate conversion based on types
    let conversion = '';
    
    if (expectedType.includes('number') && actualType.includes('string')) {
      conversion = 'Number()';
    } else if (expectedType.includes('string') && actualType.includes('number')) {
      conversion = 'String()';
    } else if (expectedType.includes('boolean')) {
      conversion = 'Boolean()';
    } else if (expectedType.includes('[]')) {
      conversion = 'Array.isArray() ? : []';
    }
    
    // Apply conversion (simplified)
    return code || `// Add type conversion: ${conversion}`;
  }
  
  /**
   * Generate async handling fix
   */
  private generateAsyncHandlingFix(context: any): string {
    const { error, code } = context;
    
    if (!code || !error.line) return code || '';
    
    const lines = code.split('\n');
    const errorLine = error.line - 1;
    
    // Find the async context
    let asyncStart = errorLine;
    for (let i = errorLine; i >= 0; i--) {
      if (lines[i].includes('async') || lines[i].includes('.then(')) {
        asyncStart = i;
        break;
      }
    }
    
    // Check if using async/await or promises
    const isAsyncAwait = lines[asyncStart].includes('async');
    
    if (isAsyncAwait) {
      // Wrap in try-catch
      const indent = lines[asyncStart].match(/^\s*/)?.[0] || '';
      const tryBlock = [
        `${indent}try {`,
        ...lines.slice(asyncStart, errorLine + 3).map(l => '  ' + l),
        `${indent}} catch (error) {`,
        `${indent}  console.error('Error:', error);`,
        `${indent}  // Handle error appropriately`,
        `${indent}}`
      ];
      
      // Replace lines
      lines.splice(asyncStart, errorLine - asyncStart + 3, ...tryBlock);
    } else {
      // Add .catch() to promise chain
      const promiseLine = lines[errorLine];
      if (!promiseLine.includes('.catch(')) {
        lines[errorLine] = promiseLine.trimEnd() + '\n  .catch(error => console.error("Error:", error));';
      }
    }
    
    return lines.join('\n');
  }
  
  /**
   * Generate array safety fix
   */
  private generateArraySafetyFix(context: any): string {
    const { error, code } = context;
    
    if (!code || !error.line) return code || '';
    
    const lines = code.split('\n');
    const errorLine = lines[error.line - 1];
    
    // Find array method call
    const methodMatch = errorLine.match(/(\w+)\.(map|filter|reduce|forEach|find)/);
    if (methodMatch) {
      const variable = methodMatch[1];
      const method = methodMatch[2];
      
      // Add array check
      const indent = errorLine.match(/^\s*/)?.[0] || '';
      const safeVersion = [
        `${indent}if (Array.isArray(${variable})) {`,
        `  ${errorLine}`,
        `${indent}} else {`,
        `${indent}  console.warn('${variable} is not an array');`,
        `${indent}  // Handle non-array case`,
        `${indent}}`
      ];
      
      lines.splice(error.line - 1, 1, ...safeVersion);
    }
    
    return lines.join('\n');
  }
  
  /**
   * Generate pattern-based fixes
   */
  private generatePatternBasedFixes(
    pattern: any,
    error: ErrorContext,
    code?: string
  ): DebugFix[] {
    const fixes: DebugFix[] = [];
    
    // Use pattern solutions
    if (pattern.solutions) {
      pattern.solutions.forEach((solution: string, index: number) => {
        fixes.push({
          id: `pattern_fix_${index}`,
          description: solution,
          code: this.applyPatternSolution(solution, code, error),
          explanation: `Based on ${pattern.name} pattern: ${solution}`,
          confidence: 0.7 + (index * 0.05), // Higher confidence for first solutions
          impact: 'moderate'
        });
      });
    }
    
    return fixes;
  }
  
  /**
   * Generate variable-based fixes
   */
  private generateVariableBasedFixes(
    variableIssues: VariableInspectionResult,
    code?: string
  ): DebugFix[] {
    const fixes: DebugFix[] = [];
    
    // Fix null/undefined variables
    variableIssues.nullUndefined.forEach(variable => {
      fixes.push({
        id: `var_fix_${variable.name}`,
        description: `Initialize ${variable.name} with default value`,
        code: this.generateVariableInitFix(variable, code),
        explanation: `Variable '${variable.name}' is ${variable.value}. Initializing with appropriate default.`,
        confidence: 0.8,
        impact: 'minimal'
      });
    });
    
    // Fix type mismatches
    variableIssues.typeMismatches.forEach(mismatch => {
      fixes.push({
        id: `type_fix_${mismatch.variable}`,
        description: `Convert ${mismatch.variable} to ${mismatch.expectedType}`,
        code: this.generateTypeFixCode(mismatch, code),
        explanation: `Type mismatch: ${mismatch.actualType} → ${mismatch.expectedType}`,
        confidence: 0.75,
        impact: 'minimal'
      });
    });
    
    return fixes;
  }
  
  /**
   * Generate defensive programming fix
   */
  private generateDefensiveFix(
    error: ErrorContext,
    code?: string
  ): DebugFix {
    const defensive = [
      '// Defensive programming approach',
      '// 1. Validate all inputs',
      '// 2. Check for null/undefined before access',
      '// 3. Use try-catch for error-prone operations',
      '// 4. Provide meaningful error messages',
      '',
      'function safeOperation(input) {',
      '  // Input validation',
      '  if (!input || typeof input !== "object") {',
      '    throw new Error("Invalid input: expected object");',
      '  }',
      '  ',
      '  try {',
      '    // Your operation here',
      '    return processData(input);',
      '  } catch (error) {',
      '    console.error("Operation failed:", error);',
      '    // Return safe default or rethrow',
      '    return null;',
      '  }',
      '}'
    ].join('\n');
    
    return {
      id: 'defensive_fix',
      description: 'Apply defensive programming principles',
      code: defensive,
      explanation: 'Comprehensive error prevention through validation and error handling',
      confidence: 0.7,
      impact: 'moderate'
    };
  }
  
  /**
   * Generate refactoring fix for complex issues
   */
  private generateRefactoringFix(
    error: ErrorContext,
    code?: string,
    analysis?: DebugAnalysis
  ): DebugFix {
    const refactoring = [
      '// Refactoring suggestion for better error handling',
      '',
      '// 1. Separate concerns',
      'class DataProcessor {',
      '  constructor(validator, errorHandler) {',
      '    this.validator = validator;',
      '    this.errorHandler = errorHandler;',
      '  }',
      '  ',
      '  async process(data) {',
      '    // Validation layer',
      '    const validation = this.validator.validate(data);',
      '    if (!validation.isValid) {',
      '      return this.errorHandler.handle(validation.errors);',
      '    }',
      '    ',
      '    // Processing with error boundaries',
      '    try {',
      '      const result = await this.transform(data);',
      '      return { success: true, data: result };',
      '    } catch (error) {',
      '      return this.errorHandler.handle(error);',
      '    }',
      '  }',
      '}',
      '',
      '// 2. Implement proper error handling',
      'class ErrorHandler {',
      '  handle(error) {',
      '    // Log error',
      '    console.error(error);',
      '    ',
      '    // Return user-friendly response',
      '    return {',
      '      success: false,',
      '      error: this.getUserMessage(error),',
      '      code: this.getErrorCode(error)',
      '    };',
      '  }',
      '}'
    ].join('\n');
    
    return {
      id: 'refactoring_fix',
      description: 'Refactor code for better error handling',
      code: refactoring,
      explanation: 'Architectural improvements to prevent similar errors',
      confidence: 0.6,
      impact: 'significant'
    };
  }
  
  /**
   * Rank fixes by confidence and impact
   */
  private rankFixes(
    fixes: DebugFix[],
    preferences?: DebugPreferences
  ): DebugFix[] {
    return fixes.sort((a, b) => {
      // Calculate scores
      let scoreA = a.confidence;
      let scoreB = b.confidence;
      
      // Adjust for impact preference
      if (preferences?.preferMinimalImpact) {
        scoreA *= a.impact === 'minimal' ? 1.2 : 0.8;
        scoreB *= b.impact === 'minimal' ? 1.2 : 0.8;
      }
      
      // Prefer fixes with test coverage
      if (a.testCoverage?.hasTests) scoreA *= 1.1;
      if (b.testCoverage?.hasTests) scoreB *= 1.1;
      
      return scoreB - scoreA;
    });
  }
  
  /**
   * Generate test suggestions
   */
  private generateTestSuggestions(
    fix: DebugFix,
    error: ErrorContext,
    code?: string
  ): TestSuggestion[] {
    const suggestions: TestSuggestion[] = [];
    
    // Unit test for the fix
    suggestions.push({
      type: 'unit',
      description: 'Test error condition is handled',
      framework: 'jest',
      code: this.generateUnitTest(fix, error),
      coverage: ['error handling', 'edge cases']
    });
    
    // Integration test if involves multiple components
    if (error.stack && error.stack.split('\n').length > 5) {
      suggestions.push({
        type: 'integration',
        description: 'Test component interaction',
        framework: 'jest',
        code: this.generateIntegrationTest(fix, error),
        coverage: ['component integration', 'data flow']
      });
    }
    
    // E2E test for user-facing errors
    if (error.type === 'runtime' && this.isUserFacing(error)) {
      suggestions.push({
        type: 'e2e',
        description: 'Test user workflow',
        framework: 'playwright',
        code: this.generateE2ETest(fix, error),
        coverage: ['user experience', 'error recovery']
      });
    }
    
    return suggestions;
  }
  
  /**
   * Generate unit test code
   */
  private generateUnitTest(fix: DebugFix, error: ErrorContext): string {
    return `
describe('${fix.description}', () => {
  it('should handle ${error.type} error', () => {
    // Arrange
    const input = /* setup test data */;
    
    // Act
    const result = functionUnderTest(input);
    
    // Assert
    expect(result).toBeDefined();
    expect(() => result.operation()).not.toThrow();
  });
  
  it('should handle null/undefined inputs', () => {
    expect(() => functionUnderTest(null)).not.toThrow();
    expect(() => functionUnderTest(undefined)).not.toThrow();
  });
});`;
  }
  
  /**
   * Generate integration test code
   */
  private generateIntegrationTest(fix: DebugFix, error: ErrorContext): string {
    return `
describe('Integration: ${fix.description}', () => {
  it('should handle error across components', async () => {
    // Setup
    const component1 = new Component1();
    const component2 = new Component2();
    
    // Execute
    const result = await component1.process(testData);
    const final = await component2.handle(result);
    
    // Verify
    expect(final.success).toBe(true);
    expect(final.errors).toHaveLength(0);
  });
});`;
  }
  
  /**
   * Generate E2E test code
   */
  private generateE2ETest(fix: DebugFix, error: ErrorContext): string {
    return `
test('E2E: ${fix.description}', async ({ page }) => {
  // Navigate to page
  await page.goto('/relevant-page');
  
  // Trigger error condition
  await page.click('#trigger-button');
  
  // Verify error is handled gracefully
  await expect(page.locator('.error-message')).not.toBeVisible();
  await expect(page.locator('#main-content')).toBeVisible();
  
  // Verify user can continue
  await page.click('#continue-button');
  await expect(page).toHaveURL('/next-page');
});`;
  }
  
  /**
   * Generate prevention strategies
   */
  private generatePreventionStrategies(
    error: ErrorContext,
    analysis?: DebugAnalysis,
    variableIssues?: VariableInspectionResult
  ): string[] {
    const strategies: string[] = [];
    
    // Type-based strategies
    if (error.type === 'runtime') {
      strategies.push(
        'Enable TypeScript strict mode for compile-time checks',
        'Add runtime type validation for external data',
        'Implement comprehensive error boundaries'
      );
    }
    
    // Pattern-based strategies
    if (analysis?.patterns) {
      analysis.patterns.forEach(pattern => {
        if (pattern.name === 'Null Reference Error') {
          strategies.push(
            'Use optional chaining (?.) throughout codebase',
            'Initialize all variables with appropriate defaults',
            'Add ESLint rules for null checking'
          );
        }
      });
    }
    
    // Variable-based strategies
    if (variableIssues?.memoryIssues.length) {
      strategies.push(
        'Implement memory profiling in CI/CD',
        'Add resource limits and monitoring',
        'Use weak references for caches'
      );
    }
    
    // General strategies
    strategies.push(
      'Add comprehensive test coverage',
      'Implement code review process',
      'Use static analysis tools',
      'Set up error monitoring and alerting'
    );
    
    return [...new Set(strategies)].slice(0, 5);
  }
  
  /**
   * Calculate overall confidence
   */
  private calculateOverallConfidence(
    fixes: DebugFix[],
    analysis?: DebugAnalysis,
    variableIssues?: VariableInspectionResult
  ): number {
    let confidence = 0.5;
    
    // Fix quality
    if (fixes.length > 0) {
      const avgFixConfidence = fixes.reduce((sum, f) => sum + f.confidence, 0) / fixes.length;
      confidence += avgFixConfidence * 0.3;
    }
    
    // Analysis quality
    if (analysis?.confidence) {
      confidence += analysis.confidence * 0.2;
    }
    
    // Variable analysis
    if (variableIssues && variableIssues.suspiciousValues.length === 0) {
      confidence += 0.1;
    }
    
    return Math.min(confidence, 0.95);
  }
  
  /**
   * Helper methods
   */
  
  private applyPatternSolution(
    solution: string,
    code?: string,
    error?: ErrorContext
  ): string {
    // This would apply the solution to the code
    // For now, return a comment with the solution
    return `// Apply solution: ${solution}\n${code || ''}`;
  }
  
  private generateVariableInitFix(
    variable: VariableState,
    code?: string
  ): string {
    const defaultValues: Record<string, string> = {
      array: '[]',
      object: '{}',
      string: '""',
      number: '0',
      boolean: 'false'
    };
    
    const defaultValue = defaultValues[variable.type] || 'null';
    return `${variable.name} = ${variable.name} || ${defaultValue}; // Initialize with default`;
  }
  
  private generateTypeFixCode(
    mismatch: any,
    code?: string
  ): string {
    const conversions: Record<string, Record<string, string>> = {
      string: {
        number: 'Number()',
        boolean: 'Boolean()'
      },
      number: {
        string: 'String()',
        boolean: 'Boolean()'
      }
    };
    
    const conversion = conversions[mismatch.actualType]?.[mismatch.expectedType] || '';
    return `${mismatch.variable} = ${conversion}(${mismatch.variable}); // Type conversion`;
  }
  
  private isComplexError(
    error: ErrorContext,
    analysis?: DebugAnalysis
  ): boolean {
    return (
      (analysis?.impact.severity === 'high' || analysis?.impact.severity === 'critical') ||
      (error.frequency && error.frequency > 5) ||
      (error.stack && error.stack.split('\n').length > 20)
    );
  }
  
  private isUserFacing(error: ErrorContext): boolean {
    // Check if error affects UI/UX
    return (
      error.message.includes('component') ||
      error.message.includes('render') ||
      error.message.includes('DOM') ||
      error.file?.includes('components/') ||
      error.file?.includes('pages/')
    );
  }
}

// Fix template interface
interface FixTemplate {
  pattern: RegExp;
  generate: (context: any) => Omit<DebugFix, 'id' | 'description'>;
}