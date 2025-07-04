/**
 * TypeScript Kernel
 * 
 * Executes TypeScript code with type checking and transpilation
 */

import * as ts from 'typescript';
import { JavaScriptKernel } from './javascript-kernel';
import {
  CellLanguage,
  ExecuteRequest,
  ExecuteResult,
  CellOutput,
  ExecutionError
} from '../types';

export class TypeScriptKernel extends JavaScriptKernel {
  public name = 'TypeScript';
  public language = CellLanguage.TypeScript;
  
  private compilerOptions: ts.CompilerOptions = {
    target: ts.ScriptTarget.ES2020,
    module: ts.ModuleKind.CommonJS,
    jsx: ts.JsxEmit.React,
    esModuleInterop: true,
    allowSyntheticDefaultImports: true,
    strict: false,
    skipLibCheck: true,
    forceConsistentCasingInFileNames: true,
    resolveJsonModule: true,
    isolatedModules: true,
    noEmit: false,
    lib: ['ES2020', 'DOM', 'DOM.Iterable']
  };
  
  constructor() {
    super();
    this.id = `ts-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
  
  async execute(request: ExecuteRequest): Promise<ExecuteResult> {
    const startTime = Date.now();
    const outputs: CellOutput[] = [];
    let error: ExecutionError | undefined;
    
    try {
      // Transpile TypeScript to JavaScript
      const transpileResult = this.transpileTypeScript(request.code, request.cellId);
      
      // Check for transpilation errors
      if (transpileResult.diagnostics && transpileResult.diagnostics.length > 0) {
        const diagnosticMessages = this.formatDiagnostics(transpileResult.diagnostics);
        
        // Add warnings for non-error diagnostics
        diagnosticMessages.forEach(msg => {
          if (msg.severity === 'warning') {
            outputs.push({
              type: 'text',
              data: `[TS Warning] ${msg.message}`
            });
          }
        });
        
        // Check if there are any errors
        const hasErrors = diagnosticMessages.some(msg => msg.severity === 'error');
        if (hasErrors) {
          const errorMessages = diagnosticMessages
            .filter(msg => msg.severity === 'error')
            .map(msg => msg.message)
            .join('\n');
          
          throw new Error(`TypeScript compilation failed:\n${errorMessages}`);
        }
      }
      
      // Execute the transpiled JavaScript
      const jsRequest: ExecuteRequest = {
        ...request,
        code: transpileResult.outputText,
        language: CellLanguage.JavaScript
      };
      
      return await super.execute(jsRequest);
      
    } catch (err: any) {
      error = {
        name: err.name || 'TypeScriptError',
        message: err.message || String(err),
        stack: err.stack,
        lineNumber: err.lineNumber,
        columnNumber: err.columnNumber
      };
      
      outputs.push({
        type: 'error',
        data: error
      });
      
      const endTime = Date.now();
      
      return {
        cellId: request.cellId,
        executionCount: this.executionCount,
        outputs,
        status: 'error',
        error,
        timing: {
          startTime,
          endTime,
          duration: endTime - startTime
        }
      };
    }
  }
  
  /**
   * Transpile TypeScript code to JavaScript
   */
  private transpileTypeScript(code: string, cellId: string): ts.TranspileOutput {
    // Add React import if JSX is detected
    if (this.containsJSX(code) && !code.includes('import React')) {
      code = `import React from 'react';\n${code}`;
    }
    
    return ts.transpileModule(code, {
      compilerOptions: this.compilerOptions,
      fileName: `cell-${cellId}.tsx`,
      reportDiagnostics: true
    });
  }
  
  /**
   * Check if code contains JSX
   */
  private containsJSX(code: string): boolean {
    return /<[A-Z]\w*/.test(code) || /<\//.test(code);
  }
  
  /**
   * Format TypeScript diagnostics
   */
  private formatDiagnostics(diagnostics: ts.Diagnostic[]): Array<{
    severity: 'error' | 'warning' | 'info';
    message: string;
    line?: number;
    column?: number;
  }> {
    return diagnostics.map(diagnostic => {
      let severity: 'error' | 'warning' | 'info';
      
      switch (diagnostic.category) {
        case ts.DiagnosticCategory.Error:
          severity = 'error';
          break;
        case ts.DiagnosticCategory.Warning:
          severity = 'warning';
          break;
        default:
          severity = 'info';
      }
      
      const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n');
      
      let line: number | undefined;
      let column: number | undefined;
      
      if (diagnostic.file && diagnostic.start !== undefined) {
        const { line: lineNumber, character } = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start);
        line = lineNumber + 1;
        column = character + 1;
      }
      
      return { severity, message, line, column };
    });
  }
  
  /**
   * Update compiler options
   */
  setCompilerOptions(options: Partial<ts.CompilerOptions>): void {
    this.compilerOptions = {
      ...this.compilerOptions,
      ...options
    };
  }
  
  /**
   * Get current compiler options
   */
  getCompilerOptions(): ts.CompilerOptions {
    return { ...this.compilerOptions };
  }
}