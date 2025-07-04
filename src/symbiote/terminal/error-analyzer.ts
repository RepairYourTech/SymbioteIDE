/**
 * Error Analyzer
 * 
 * Intelligent error analysis and solution suggestions for terminal errors
 */

import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { AITask, TaskType } from '../orchestration/interfaces';

export interface ErrorAnalysis {
  errorType: ErrorType;
  severity: ErrorSeverity;
  category: ErrorCategory;
  explanation: string;
  rootCause: string;
  suggestedFixes: Fix[];
  relatedErrors?: string[];
  documentation?: string[];
  confidence: number;
}

export interface Fix {
  command: string;
  description: string;
  confidence: number;
  riskLevel: 'safe' | 'moderate' | 'risky';
  explanation: string;
  prerequisites?: string[];
}

export enum ErrorType {
  CommandNotFound = 'command-not-found',
  PermissionDenied = 'permission-denied',
  FileNotFound = 'file-not-found',
  SyntaxError = 'syntax-error',
  NetworkError = 'network-error',
  DependencyError = 'dependency-error',
  ConfigurationError = 'configuration-error',
  RuntimeError = 'runtime-error',
  CompilationError = 'compilation-error',
  Unknown = 'unknown'
}

export enum ErrorSeverity {
  Low = 'low',
  Medium = 'medium',
  High = 'high',
  Critical = 'critical'
}

export enum ErrorCategory {
  System = 'system',
  Shell = 'shell',
  Programming = 'programming',
  Network = 'network',
  Security = 'security',
  Configuration = 'configuration',
  Application = 'application'
}

interface ErrorPattern {
  pattern: RegExp;
  type: ErrorType;
  category: ErrorCategory;
  severity: ErrorSeverity;
  extractor?: (match: RegExpMatchArray) => ExtractedInfo;
}

interface ExtractedInfo {
  command?: string;
  file?: string;
  permission?: string;
  missingDependency?: string;
  lineNumber?: number;
  columnNumber?: number;
  [key: string]: any;
}

export class ErrorAnalyzer {
  private orchestrationEngine: OrchestrationEngine;
  private errorPatterns: ErrorPattern[];
  private fixDatabase: Map<ErrorType, FixTemplate[]>;
  
  constructor(orchestrationEngine: OrchestrationEngine) {
    this.orchestrationEngine = orchestrationEngine;
    this.errorPatterns = this.initializeErrorPatterns();
    this.fixDatabase = this.initializeFixDatabase();
  }
  
  /**
   * Analyze an error message
   */
  async analyzeError(
    errorMessage: string,
    context?: {
      command?: string;
      shell?: string;
      workingDirectory?: string;
      platform?: string;
    }
  ): Promise<ErrorAnalysis> {
    // Try pattern matching first
    const patternMatch = this.matchErrorPattern(errorMessage);
    
    if (patternMatch) {
      const fixes = await this.generateFixes(
        patternMatch.type,
        patternMatch.extracted,
        context
      );
      
      return {
        errorType: patternMatch.type,
        severity: patternMatch.severity,
        category: patternMatch.category,
        explanation: await this.explainError(patternMatch, errorMessage),
        rootCause: this.identifyRootCause(patternMatch, errorMessage),
        suggestedFixes: fixes,
        confidence: 0.9
      };
    }
    
    // Use AI for unknown errors
    return await this.analyzeWithAI(errorMessage, context);
  }
  
  /**
   * Get quick fix for common errors
   */
  getQuickFix(errorType: ErrorType, info?: ExtractedInfo): Fix | null {
    const templates = this.fixDatabase.get(errorType);
    if (!templates || templates.length === 0) {
      return null;
    }
    
    const template = templates[0]; // Get highest priority fix
    return this.applyFixTemplate(template, info);
  }
  
  /**
   * Check if an error is recoverable
   */
  isRecoverable(errorType: ErrorType): boolean {
    const recoverableTypes = [
      ErrorType.CommandNotFound,
      ErrorType.FileNotFound,
      ErrorType.PermissionDenied,
      ErrorType.DependencyError,
      ErrorType.ConfigurationError
    ];
    
    return recoverableTypes.includes(errorType);
  }
  
  // Private methods
  
  private initializeErrorPatterns(): ErrorPattern[] {
    return [
      // Command not found
      {
        pattern: /command not found: (\w+)|'(\w+)' is not recognized|(\w+): command not found/i,
        type: ErrorType.CommandNotFound,
        category: ErrorCategory.Shell,
        severity: ErrorSeverity.Medium,
        extractor: (match) => ({
          command: match[1] || match[2] || match[3]
        })
      },
      
      // Permission denied
      {
        pattern: /permission denied|access is denied|operation not permitted/i,
        type: ErrorType.PermissionDenied,
        category: ErrorCategory.Security,
        severity: ErrorSeverity.High,
        extractor: (match) => {
          const fileMatch = match.input?.match(/['"]([^'"]+)['"]/);
          return {
            file: fileMatch?.[1]
          };
        }
      },
      
      // File not found
      {
        pattern: /no such file or directory|cannot find the file|file not found: ['"]?([^'"]+)['"]?/i,
        type: ErrorType.FileNotFound,
        category: ErrorCategory.System,
        severity: ErrorSeverity.Medium,
        extractor: (match) => ({
          file: match[1] || match.input?.match(/['"]([^'"]+)['"]/)?.[1]
        })
      },
      
      // Syntax errors
      {
        pattern: /syntax error|unexpected token|parsing error|invalid syntax/i,
        type: ErrorType.SyntaxError,
        category: ErrorCategory.Programming,
        severity: ErrorSeverity.High,
        extractor: (match) => {
          const lineMatch = match.input?.match(/line (\d+)/i);
          const colMatch = match.input?.match(/column (\d+)/i);
          return {
            lineNumber: lineMatch ? parseInt(lineMatch[1]) : undefined,
            columnNumber: colMatch ? parseInt(colMatch[1]) : undefined
          };
        }
      },
      
      // Network errors
      {
        pattern: /connection refused|network unreachable|timeout|could not resolve host/i,
        type: ErrorType.NetworkError,
        category: ErrorCategory.Network,
        severity: ErrorSeverity.High,
        extractor: (match) => {
          const hostMatch = match.input?.match(/host[:\s]+([^\s]+)/i);
          const portMatch = match.input?.match(/port[:\s]+(\d+)/i);
          return {
            host: hostMatch?.[1],
            port: portMatch?.[1]
          };
        }
      },
      
      // Dependency errors
      {
        pattern: /module not found|cannot find module|package .* is not installed|ImportError/i,
        type: ErrorType.DependencyError,
        category: ErrorCategory.Programming,
        severity: ErrorSeverity.High,
        extractor: (match) => {
          const moduleMatch = match.input?.match(/module[:\s]+['"]?([^'"]+)['"]?/i);
          const packageMatch = match.input?.match(/package[:\s]+['"]?([^'"]+)['"]?/i);
          return {
            missingDependency: moduleMatch?.[1] || packageMatch?.[1]
          };
        }
      },
      
      // Configuration errors
      {
        pattern: /configuration error|invalid configuration|config file not found/i,
        type: ErrorType.ConfigurationError,
        category: ErrorCategory.Configuration,
        severity: ErrorSeverity.Medium,
        extractor: (match) => {
          const fileMatch = match.input?.match(/file[:\s]+['"]?([^'"]+)['"]?/i);
          return {
            configFile: fileMatch?.[1]
          };
        }
      },
      
      // Compilation errors
      {
        pattern: /compilation failed|compiler error|build failed/i,
        type: ErrorType.CompilationError,
        category: ErrorCategory.Programming,
        severity: ErrorSeverity.High,
        extractor: () => ({})
      }
    ];
  }
  
  private initializeFixDatabase(): Map<ErrorType, FixTemplate[]> {
    const fixes = new Map<ErrorType, FixTemplate[]>();
    
    fixes.set(ErrorType.CommandNotFound, [
      {
        template: 'sudo apt-get install {command}',
        condition: (ctx) => ctx?.platform === 'linux',
        description: 'Install the missing command',
        riskLevel: 'safe'
      },
      {
        template: 'brew install {command}',
        condition: (ctx) => ctx?.platform === 'macos',
        description: 'Install the missing command using Homebrew',
        riskLevel: 'safe'
      },
      {
        template: 'npm install -g {command}',
        condition: () => true,
        description: 'Install as a global npm package',
        riskLevel: 'safe'
      }
    ]);
    
    fixes.set(ErrorType.PermissionDenied, [
      {
        template: 'sudo {lastCommand}',
        condition: (ctx) => ctx?.platform !== 'windows',
        description: 'Run with administrator privileges',
        riskLevel: 'moderate'
      },
      {
        template: 'chmod +x {file}',
        condition: (ctx, info) => ctx?.platform !== 'windows' && info?.file,
        description: 'Make file executable',
        riskLevel: 'safe'
      },
      {
        template: 'chmod 755 {file}',
        condition: (ctx, info) => ctx?.platform !== 'windows' && info?.file,
        description: 'Set standard permissions',
        riskLevel: 'safe'
      }
    ]);
    
    fixes.set(ErrorType.FileNotFound, [
      {
        template: 'touch {file}',
        condition: (ctx, info) => info?.file?.includes('.'),
        description: 'Create the missing file',
        riskLevel: 'safe'
      },
      {
        template: 'mkdir -p {file}',
        condition: (ctx, info) => info?.file && !info.file.includes('.'),
        description: 'Create the missing directory',
        riskLevel: 'safe'
      },
      {
        template: 'ls -la',
        condition: () => true,
        description: 'List files to check spelling',
        riskLevel: 'safe'
      }
    ]);
    
    fixes.set(ErrorType.DependencyError, [
      {
        template: 'npm install {missingDependency}',
        condition: (ctx) => ctx?.command?.includes('node') || ctx?.command?.includes('npm'),
        description: 'Install missing npm package',
        riskLevel: 'safe'
      },
      {
        template: 'pip install {missingDependency}',
        condition: (ctx) => ctx?.command?.includes('python'),
        description: 'Install missing Python package',
        riskLevel: 'safe'
      },
      {
        template: 'gem install {missingDependency}',
        condition: (ctx) => ctx?.command?.includes('ruby'),
        description: 'Install missing Ruby gem',
        riskLevel: 'safe'
      }
    ]);
    
    fixes.set(ErrorType.NetworkError, [
      {
        template: 'ping {host}',
        condition: (ctx, info) => info?.host,
        description: 'Test network connectivity',
        riskLevel: 'safe'
      },
      {
        template: 'nslookup {host}',
        condition: (ctx, info) => info?.host,
        description: 'Check DNS resolution',
        riskLevel: 'safe'
      },
      {
        template: 'netstat -an | grep {port}',
        condition: (ctx, info) => info?.port && ctx?.platform !== 'windows',
        description: 'Check if port is in use',
        riskLevel: 'safe'
      }
    ]);
    
    return fixes;
  }
  
  private matchErrorPattern(errorMessage: string): {
    type: ErrorType;
    category: ErrorCategory;
    severity: ErrorSeverity;
    extracted: ExtractedInfo;
  } | null {
    for (const pattern of this.errorPatterns) {
      const match = errorMessage.match(pattern.pattern);
      if (match) {
        return {
          type: pattern.type,
          category: pattern.category,
          severity: pattern.severity,
          extracted: pattern.extractor ? pattern.extractor(match) : {}
        };
      }
    }
    
    return null;
  }
  
  private async generateFixes(
    errorType: ErrorType,
    extracted: ExtractedInfo,
    context?: any
  ): Promise<Fix[]> {
    const fixes: Fix[] = [];
    const templates = this.fixDatabase.get(errorType) || [];
    
    for (const template of templates) {
      if (template.condition(context, extracted)) {
        const fix = this.applyFixTemplate(template, extracted, context);
        if (fix) {
          fixes.push(fix);
        }
      }
    }
    
    // Add AI-generated fixes if needed
    if (fixes.length < 3) {
      const aiFixes = await this.generateAIFixes(errorType, extracted, context);
      fixes.push(...aiFixes);
    }
    
    return fixes;
  }
  
  private applyFixTemplate(
    template: FixTemplate,
    info?: ExtractedInfo,
    context?: any
  ): Fix {
    let command = template.template;
    
    // Replace placeholders
    if (info) {
      for (const [key, value] of Object.entries(info)) {
        command = command.replace(`{${key}}`, String(value));
      }
    }
    
    if (context?.command) {
      command = command.replace('{lastCommand}', context.command);
    }
    
    return {
      command,
      description: template.description,
      confidence: 0.8,
      riskLevel: template.riskLevel,
      explanation: `This command will ${template.description.toLowerCase()}`,
      prerequisites: template.prerequisites
    };
  }
  
  private async explainError(
    match: { type: ErrorType; category: ErrorCategory },
    errorMessage: string
  ): Promise<string> {
    const explanations: Record<ErrorType, string> = {
      [ErrorType.CommandNotFound]: 'The command is not installed or not in your PATH',
      [ErrorType.PermissionDenied]: 'You lack the necessary permissions to perform this operation',
      [ErrorType.FileNotFound]: 'The specified file or directory does not exist',
      [ErrorType.SyntaxError]: 'There is a syntax error in your command or code',
      [ErrorType.NetworkError]: 'Network connectivity issue or remote service is unavailable',
      [ErrorType.DependencyError]: 'A required dependency or module is missing',
      [ErrorType.ConfigurationError]: 'Configuration file is missing or invalid',
      [ErrorType.CompilationError]: 'Code compilation failed due to errors',
      [ErrorType.RuntimeError]: 'An error occurred during program execution',
      [ErrorType.Unknown]: 'An unexpected error occurred'
    };
    
    return explanations[match.type] || explanations[ErrorType.Unknown];
  }
  
  private identifyRootCause(
    match: { type: ErrorType; extracted: ExtractedInfo },
    errorMessage: string
  ): string {
    // Simple root cause identification
    const causes: Record<ErrorType, string> = {
      [ErrorType.CommandNotFound]: `Command '${match.extracted.command}' is not installed`,
      [ErrorType.PermissionDenied]: 'Insufficient permissions for the operation',
      [ErrorType.FileNotFound]: `File '${match.extracted.file || 'unknown'}' does not exist`,
      [ErrorType.SyntaxError]: 'Invalid syntax in command or code',
      [ErrorType.NetworkError]: 'Network connection or DNS resolution failure',
      [ErrorType.DependencyError]: `Missing dependency: ${match.extracted.missingDependency}`,
      [ErrorType.ConfigurationError]: 'Invalid or missing configuration',
      [ErrorType.CompilationError]: 'Source code contains errors',
      [ErrorType.RuntimeError]: 'Runtime exception occurred',
      [ErrorType.Unknown]: 'Unable to determine root cause'
    };
    
    return causes[match.type] || causes[ErrorType.Unknown];
  }
  
  private async analyzeWithAI(
    errorMessage: string,
    context?: any
  ): Promise<ErrorAnalysis> {
    const prompt = `Analyze this terminal error and provide solutions:

Error: ${errorMessage}

Context:
- Command: ${context?.command || 'unknown'}
- Shell: ${context?.shell || 'unknown'}
- Directory: ${context?.workingDirectory || 'unknown'}
- Platform: ${context?.platform || 'unknown'}

Provide:
1. Error type and severity
2. Root cause explanation
3. Step-by-step fixes
4. Risk assessment for each fix

Format as JSON with errorType, severity, explanation, rootCause, and suggestedFixes array.`;
    
    const aiTask: AITask = {
      id: `error-analysis-${Date.now()}`,
      type: TaskType.Analysis,
      prompt,
      constraints: {
        maxTokens: 800,
        temperature: 0.3
      }
    };
    
    const result = await this.orchestrationEngine.executeTask(aiTask);
    
    if (result.status === 'success' && result.content) {
      try {
        const parsed = JSON.parse(result.content);
        return {
          errorType: parsed.errorType || ErrorType.Unknown,
          severity: parsed.severity || ErrorSeverity.Medium,
          category: ErrorCategory.Unknown,
          explanation: parsed.explanation || 'Error analysis failed',
          rootCause: parsed.rootCause || 'Unknown',
          suggestedFixes: parsed.suggestedFixes || [],
          confidence: 0.7
        };
      } catch {
        // Fallback
        return {
          errorType: ErrorType.Unknown,
          severity: ErrorSeverity.Medium,
          category: ErrorCategory.Unknown,
          explanation: 'Unable to analyze error',
          rootCause: 'Unknown',
          suggestedFixes: [],
          confidence: 0.3
        };
      }
    }
    
    throw new Error('Failed to analyze error with AI');
  }
  
  private async generateAIFixes(
    errorType: ErrorType,
    info: ExtractedInfo,
    context?: any
  ): Promise<Fix[]> {
    // Simplified AI fix generation
    return [];
  }
}

interface FixTemplate {
  template: string;
  condition: (context?: any, info?: ExtractedInfo) => boolean;
  description: string;
  riskLevel: 'safe' | 'moderate' | 'risky';
  prerequisites?: string[];
}