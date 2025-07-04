/**
 * Stack Navigator Agent - Fast stack trace traversal and analysis
 */

import { Logger } from '../../utils/logger';
import { StackFrame, ErrorContext } from '../types';
import * as path from 'path';

export interface StackNavigationResult {
  frames: StackFrame[];
  userCodeFrames: StackFrame[];
  errorOrigin: StackFrame | null;
  executionPath: string[];
  suspiciousFrames: StackFrame[];
  asyncBoundaries: number[];
  analysis: string;
}

export class StackNavigatorAgent {
  private logger = new Logger('StackNavigatorAgent');
  private userCodePatterns: RegExp[] = [];
  private libraryPatterns: RegExp[] = [];
  
  constructor() {
    this.initializePatterns();
  }
  
  /**
   * Initialize code patterns for classification
   */
  private initializePatterns(): void {
    // Common user code patterns
    this.userCodePatterns = [
      /^src\//,
      /^app\//,
      /^components\//,
      /^pages\//,
      /^lib\//,
      /^utils\//,
      /^services\//,
      /^api\//,
      /^\.\//, // Relative paths
      /^\.\.\//, // Parent paths
    ];
    
    // Common library patterns
    this.libraryPatterns = [
      /node_modules/,
      /^\[.*\]$/, // Native code
      /<anonymous>/,
      /^internal\//,
      /^events\.js/,
      /^timers\.js/,
      /^net\.js/,
      /^_stream_/,
      /webpack:/,
      /\(native\)/,
    ];
  }
  
  /**
   * Navigate and analyze stack trace
   */
  async navigate(
    stack: string,
    code?: string,
    options?: {
      focusUserCode?: boolean;
      maxDepth?: number;
      includeAsync?: boolean;
    }
  ): Promise<StackNavigationResult> {
    try {
      // Parse stack trace into frames
      const frames = this.parseStackTrace(stack);
      
      // Classify frames
      const userCodeFrames = this.filterUserCodeFrames(frames);
      
      // Find error origin
      const errorOrigin = this.findErrorOrigin(frames, userCodeFrames);
      
      // Build execution path
      const executionPath = this.buildExecutionPath(frames, options?.focusUserCode);
      
      // Identify suspicious frames
      const suspiciousFrames = this.identifySuspiciousFrames(frames, code);
      
      // Find async boundaries
      const asyncBoundaries = this.findAsyncBoundaries(frames);
      
      // Generate analysis
      const analysis = this.generateAnalysis({
        frames,
        userCodeFrames,
        errorOrigin,
        suspiciousFrames,
        asyncBoundaries
      });
      
      return {
        frames,
        userCodeFrames,
        errorOrigin,
        executionPath,
        suspiciousFrames,
        asyncBoundaries,
        analysis
      };
      
    } catch (error) {
      this.logger.error('Stack navigation failed', error);
      throw error;
    }
  }
  
  /**
   * Parse stack trace into structured frames
   */
  private parseStackTrace(stack: string): StackFrame[] {
    const frames: StackFrame[] = [];
    const lines = stack.split('\n');
    
    // Common stack trace patterns
    const patterns = [
      // Node.js style: at functionName (file:line:col)
      /^\s*at\s+(?:(.+?)\s+\()?(.+?):(\d+):(\d+)\)?$/,
      // Chrome style: at file:line:col
      /^\s*at\s+(.+?):(\d+):(\d+)$/,
      // Firefox style: functionName@file:line:col
      /^(.+?)@(.+?):(\d+):(\d+)$/,
      // Simple format: file:line:col
      /^(.+?):(\d+):(\d+)$/
    ];
    
    lines.forEach((line, index) => {
      for (const pattern of patterns) {
        const match = line.match(pattern);
        if (match) {
          let functionName: string;
          let file: string;
          let lineNum: number;
          let column: number;
          
          if (match.length === 5) {
            // Has function name
            functionName = match[1] || '<anonymous>';
            file = match[2];
            lineNum = parseInt(match[3]);
            column = parseInt(match[4]);
          } else if (match.length === 4) {
            // No function name
            functionName = '<anonymous>';
            file = match[1];
            lineNum = parseInt(match[2]);
            column = parseInt(match[3]);
          } else {
            continue;
          }
          
          // Clean up function name
          functionName = this.cleanFunctionName(functionName);
          
          // Create frame
          const frame: StackFrame = {
            id: `frame_${index}`,
            functionName,
            file: this.normalizeFilePath(file),
            line: lineNum,
            column,
            variables: [], // Will be populated by Variable Inspector
            isUserCode: this.isUserCode(file),
            isAsync: this.isAsyncFrame(functionName, line),
            hasError: index === 0, // First frame usually has the error
            analysis: ''
          };
          
          frames.push(frame);
          break;
        }
      }
    });
    
    return frames;
  }
  
  /**
   * Clean up function name
   */
  private cleanFunctionName(name: string): string {
    return name
      .replace(/^Object\./, '')
      .replace(/^Module\./, '')
      .replace(/^exports\./, '')
      .replace(/\s+/g, ' ')
      .trim();
  }
  
  /**
   * Normalize file path
   */
  private normalizeFilePath(file: string): string {
    // Remove webpack prefixes
    file = file.replace(/^webpack:\/\/\//, '');
    file = file.replace(/^webpack-internal:\/\/\//, '');
    
    // Handle eval and anonymous
    if (file.includes('eval')) {
      const evalMatch = file.match(/eval at .+ \((.+?)\)/);
      if (evalMatch) {
        file = evalMatch[1];
      }
    }
    
    // Normalize path separators
    return path.normalize(file);
  }
  
  /**
   * Check if frame is user code
   */
  private isUserCode(file: string): boolean {
    // Check against library patterns first (exclusion)
    for (const pattern of this.libraryPatterns) {
      if (pattern.test(file)) {
        return false;
      }
    }
    
    // Check against user code patterns (inclusion)
    for (const pattern of this.userCodePatterns) {
      if (pattern.test(file)) {
        return true;
      }
    }
    
    // Default: consider it user code if no patterns match
    return !file.includes('/') || file.startsWith('./');
  }
  
  /**
   * Check if frame is async
   */
  private isAsyncFrame(functionName: string, line: string): boolean {
    const asyncIndicators = [
      'async',
      'await',
      'Promise',
      'then',
      'catch',
      'finally',
      'callback',
      'setTimeout',
      'setInterval',
      'nextTick',
      'microtask'
    ];
    
    const lowerName = functionName.toLowerCase();
    const lowerLine = line.toLowerCase();
    
    return asyncIndicators.some(indicator => 
      lowerName.includes(indicator) || lowerLine.includes(indicator)
    );
  }
  
  /**
   * Filter user code frames
   */
  private filterUserCodeFrames(frames: StackFrame[]): StackFrame[] {
    return frames.filter(frame => frame.isUserCode);
  }
  
  /**
   * Find error origin frame
   */
  private findErrorOrigin(
    frames: StackFrame[],
    userCodeFrames: StackFrame[]
  ): StackFrame | null {
    // Prefer first user code frame
    if (userCodeFrames.length > 0) {
      return userCodeFrames[0];
    }
    
    // Otherwise, first non-native frame
    const nonNativeFrame = frames.find(f => 
      !f.file.includes('[native]') && 
      !f.file.includes('<anonymous>')
    );
    
    return nonNativeFrame || frames[0] || null;
  }
  
  /**
   * Build execution path
   */
  private buildExecutionPath(
    frames: StackFrame[],
    focusUserCode?: boolean
  ): string[] {
    const framesToUse = focusUserCode ? 
      frames.filter(f => f.isUserCode) : 
      frames;
    
    return framesToUse.map(frame => {
      const location = `${frame.file}:${frame.line}`;
      if (frame.functionName !== '<anonymous>') {
        return `${frame.functionName} (${location})`;
      }
      return location;
    });
  }
  
  /**
   * Identify suspicious frames
   */
  private identifySuspiciousFrames(
    frames: StackFrame[],
    code?: string
  ): StackFrame[] {
    const suspicious: StackFrame[] = [];
    
    frames.forEach((frame, index) => {
      let isSuspicious = false;
      let reasons: string[] = [];
      
      // Check for recursive calls
      const sameFunctionFrames = frames.filter(f => 
        f.functionName === frame.functionName && 
        f.file === frame.file
      );
      
      if (sameFunctionFrames.length > 3) {
        isSuspicious = true;
        reasons.push('Potential infinite recursion');
      }
      
      // Check for error-prone patterns in function names
      const errorPronePatterns = [
        'eval',
        'Function',
        'setTimeout',
        'setInterval',
        'dangerouslySetInnerHTML'
      ];
      
      if (errorPronePatterns.some(pattern => 
        frame.functionName.includes(pattern)
      )) {
        isSuspicious = true;
        reasons.push('Uses error-prone patterns');
      }
      
      // Check for deep nesting (potential stack overflow)
      if (index > 100) {
        isSuspicious = true;
        reasons.push('Very deep call stack');
      }
      
      // Check for mixed async/sync boundaries
      if (index > 0 && frame.isAsync !== frames[index - 1].isAsync) {
        isSuspicious = true;
        reasons.push('Async/sync boundary crossing');
      }
      
      if (isSuspicious) {
        frame.analysis = reasons.join('; ');
        suspicious.push(frame);
      }
    });
    
    return suspicious;
  }
  
  /**
   * Find async boundaries in stack
   */
  private findAsyncBoundaries(frames: StackFrame[]): number[] {
    const boundaries: number[] = [];
    
    frames.forEach((frame, index) => {
      if (index > 0 && frame.isAsync !== frames[index - 1].isAsync) {
        boundaries.push(index);
      }
    });
    
    return boundaries;
  }
  
  /**
   * Generate stack analysis
   */
  private generateAnalysis(data: {
    frames: StackFrame[];
    userCodeFrames: StackFrame[];
    errorOrigin: StackFrame | null;
    suspiciousFrames: StackFrame[];
    asyncBoundaries: number[];
  }): string {
    const { frames, userCodeFrames, errorOrigin, suspiciousFrames, asyncBoundaries } = data;
    
    let analysis = '## Stack Trace Analysis\n\n';
    
    // Overview
    analysis += `**Total Frames**: ${frames.length}\n`;
    analysis += `**User Code Frames**: ${userCodeFrames.length}\n`;
    analysis += `**Library Frames**: ${frames.length - userCodeFrames.length}\n\n`;
    
    // Error origin
    if (errorOrigin) {
      analysis += `### Error Origin\n`;
      analysis += `- **Function**: ${errorOrigin.functionName}\n`;
      analysis += `- **Location**: ${errorOrigin.file}:${errorOrigin.line}:${errorOrigin.column}\n`;
      analysis += `- **Type**: ${errorOrigin.isUserCode ? 'User Code' : 'Library Code'}\n\n`;
    }
    
    // Call flow
    analysis += `### Execution Flow\n`;
    const topFrames = userCodeFrames.slice(0, 5);
    topFrames.forEach((frame, index) => {
      analysis += `${index + 1}. ${frame.functionName} at ${path.basename(frame.file)}:${frame.line}\n`;
    });
    
    if (userCodeFrames.length > 5) {
      analysis += `... and ${userCodeFrames.length - 5} more frames\n`;
    }
    analysis += '\n';
    
    // Suspicious frames
    if (suspiciousFrames.length > 0) {
      analysis += `### Suspicious Frames\n`;
      suspiciousFrames.forEach(frame => {
        analysis += `- ${frame.functionName}: ${frame.analysis}\n`;
      });
      analysis += '\n';
    }
    
    // Async analysis
    if (asyncBoundaries.length > 0) {
      analysis += `### Async Boundaries\n`;
      analysis += `Found ${asyncBoundaries.length} async/sync transitions in the stack.\n`;
      analysis += 'This may indicate promise handling issues or callback problems.\n\n';
    }
    
    // Recommendations
    analysis += `### Recommendations\n`;
    
    if (errorOrigin && errorOrigin.isUserCode) {
      analysis += `- Start debugging at ${path.basename(errorOrigin.file)}:${errorOrigin.line}\n`;
    }
    
    if (suspiciousFrames.length > 0) {
      analysis += `- Investigate suspicious patterns in the identified frames\n`;
    }
    
    if (frames.length > 50) {
      analysis += `- Deep call stack detected - check for infinite recursion\n`;
    }
    
    if (asyncBoundaries.length > 3) {
      analysis += `- Complex async flow - consider simplifying promise chains\n`;
    }
    
    return analysis;
  }
  
  /**
   * Get frame context
   */
  async getFrameContext(
    frame: StackFrame,
    code?: string
  ): Promise<{
    codeSnippet: string;
    localVariables: string[];
    possibleCauses: string[];
  }> {
    // This would integrate with the code analysis system
    // For now, return a structured response
    
    return {
      codeSnippet: `// Code at ${frame.file}:${frame.line}\n// [Code would be extracted here]`,
      localVariables: ['variable1', 'variable2'], // Would be extracted from code
      possibleCauses: this.analyzePossibleCauses(frame)
    };
  }
  
  /**
   * Analyze possible causes for a frame
   */
  private analyzePossibleCauses(frame: StackFrame): string[] {
    const causes: string[] = [];
    
    if (frame.hasError) {
      causes.push('This is where the error was thrown');
    }
    
    if (frame.functionName.includes('callback')) {
      causes.push('Callback function - check error handling');
    }
    
    if (frame.isAsync) {
      causes.push('Async function - verify promise handling');
    }
    
    if (frame.analysis) {
      causes.push(frame.analysis);
    }
    
    return causes;
  }
}