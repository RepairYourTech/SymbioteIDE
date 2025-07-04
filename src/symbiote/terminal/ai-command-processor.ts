/**
 * AI Command Processor
 * 
 * Processes natural language commands and provides intelligent assistance
 */

import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { 
  AITask, 
  TaskType,
  ExecutionOptions 
} from '../orchestration/interfaces';
import {
  CommandSuggestion,
  CommandAlternative,
  TerminalSafetyCheck,
  CommandCategory
} from '../../types/terminal-api';

export interface CommandContext {
  sessionId: string;
  workingDirectory: string;
  shell: string;
  history: string[];
  environment?: Record<string, string>;
}

export interface ProcessedCommand {
  command?: string;
  explanation?: string;
  confidence: number;
  alternatives?: CommandAlternative[];
  warnings?: string[];
  category?: CommandCategory;
}

export class AICommandProcessor {
  private orchestrationEngine: OrchestrationEngine;
  private commandPatterns: Map<string, CommandPattern>;
  private safetyRules: SafetyRule[];
  
  constructor(orchestrationEngine: OrchestrationEngine) {
    this.orchestrationEngine = orchestrationEngine;
    this.commandPatterns = this.initializeCommandPatterns();
    this.safetyRules = this.initializeSafetyRules();
  }
  
  /**
   * Process a natural language command
   */
  async processCommand(
    naturalLanguageInput: string, 
    context: CommandContext
  ): Promise<ProcessedCommand> {
    try {
      // First, check for direct command patterns
      const patternMatch = this.matchCommandPattern(naturalLanguageInput);
      if (patternMatch) {
        return this.applyPattern(patternMatch, naturalLanguageInput, context);
      }
      
      // Use AI for complex natural language processing
      const aiTask: AITask = {
        id: `terminal-cmd-${Date.now()}`,
        type: TaskType.Translation,
        prompt: this.buildCommandPrompt(naturalLanguageInput, context),
        context: {
          language: 'shell',
          framework: context.shell,
          projectType: 'terminal-command',
          additionalContext: {
            workingDirectory: context.workingDirectory,
            shell: context.shell,
            recentCommands: context.history.slice(-5)
          }
        },
        constraints: {
          maxTokens: 500,
          temperature: 0.3, // Lower temperature for more deterministic commands
          requiredCapabilities: []
        }
      };
      
      const result = await this.orchestrationEngine.executeTask(aiTask);
      
      if (result.status === 'success' && result.content) {
        return this.parseAIResponse(result.content, naturalLanguageInput, context);
      }
      
      return {
        confidence: 0,
        explanation: 'Failed to process command',
        warnings: ['Could not interpret the natural language input']
      };
      
    } catch (error) {
      console.error('Error processing command:', error);
      return {
        confidence: 0,
        explanation: 'Error processing command',
        warnings: [error instanceof Error ? error.message : 'Unknown error']
      };
    }
  }
  
  /**
   * Get command suggestions based on context
   */
  async getSuggestions(
    partialInput: string, 
    context: CommandContext
  ): Promise<CommandSuggestion[]> {
    const suggestions: CommandSuggestion[] = [];
    
    // Add pattern-based suggestions
    for (const [pattern, cmd] of this.commandPatterns) {
      if (pattern.toLowerCase().includes(partialInput.toLowerCase())) {
        suggestions.push({
          command: cmd.template,
          description: cmd.description,
          confidence: 0.8,
          category: cmd.category,
          examples: cmd.examples
        });
      }
    }
    
    // Add AI-based suggestions if needed
    if (suggestions.length < 5) {
      const aiSuggestions = await this.getAISuggestions(partialInput, context);
      suggestions.push(...aiSuggestions);
    }
    
    // Sort by confidence and limit
    return suggestions
      .sort((a, b) => b.confidence - a.confidence)
      .slice(0, 10);
  }
  
  /**
   * Check command safety
   */
  async checkSafety(command: string): Promise<TerminalSafetyCheck> {
    const warnings: string[] = [];
    const impacts: string[] = [];
    let riskLevel: 'safe' | 'caution' | 'dangerous' = 'safe';
    let requiresConfirmation = false;
    
    // Check against safety rules
    for (const rule of this.safetyRules) {
      if (rule.pattern.test(command)) {
        warnings.push(rule.warning);
        impacts.push(...rule.impacts);
        
        if (rule.level === 'dangerous') {
          riskLevel = 'dangerous';
          requiresConfirmation = true;
        } else if (rule.level === 'caution' && riskLevel !== 'dangerous') {
          riskLevel = 'caution';
        }
      }
    }
    
    // Suggest safer alternatives for dangerous commands
    const alternatives = riskLevel === 'dangerous' ? 
      this.getSaferAlternatives(command) : undefined;
    
    return {
      command,
      safe: riskLevel === 'safe',
      risk_level: riskLevel,
      warnings,
      potential_impacts: impacts,
      requires_confirmation: requiresConfirmation,
      safer_alternatives: alternatives
    };
  }
  
  /**
   * Explain an error message
   */
  async explainError(error: string, context: CommandContext): Promise<string> {
    const aiTask: AITask = {
      id: `terminal-error-${Date.now()}`,
      type: TaskType.Explanation,
      prompt: `Explain this terminal error and suggest fixes:\n\nError: ${error}\n\nContext:\n- Shell: ${context.shell}\n- Directory: ${context.workingDirectory}\n- Recent commands: ${context.history.slice(-3).join('\n')}`,
      constraints: {
        maxTokens: 300,
        temperature: 0.5
      }
    };
    
    const result = await this.orchestrationEngine.executeTask(aiTask);
    
    if (result.status === 'success' && result.content) {
      return result.content;
    }
    
    return 'Unable to explain this error. Please check the command syntax and try again.';
  }
  
  // Private methods
  
  private initializeCommandPatterns(): Map<string, CommandPattern> {
    const patterns = new Map<string, CommandPattern>();
    
    // File operations
    patterns.set('list files', {
      template: 'ls -la',
      description: 'List all files with details',
      category: 'file_management',
      examples: ['ls', 'ls -la', 'ls -lh']
    });
    
    patterns.set('create directory', {
      template: 'mkdir {name}',
      description: 'Create a new directory',
      category: 'file_management',
      examples: ['mkdir mydir', 'mkdir -p path/to/dir']
    });
    
    patterns.set('delete file', {
      template: 'rm {file}',
      description: 'Delete a file',
      category: 'file_management',
      examples: ['rm file.txt', 'rm -f file.txt']
    });
    
    // Git operations
    patterns.set('git status', {
      template: 'git status',
      description: 'Show git repository status',
      category: 'git',
      examples: ['git status', 'git status -s']
    });
    
    patterns.set('git commit', {
      template: 'git commit -m "{message}"',
      description: 'Commit changes with message',
      category: 'git',
      examples: ['git commit -m "feat: add feature"']
    });
    
    // Process operations
    patterns.set('list processes', {
      template: 'ps aux',
      description: 'List all running processes',
      category: 'process_control',
      examples: ['ps aux', 'ps -ef']
    });
    
    patterns.set('kill process', {
      template: 'kill {pid}',
      description: 'Terminate a process',
      category: 'process_control',
      examples: ['kill 1234', 'kill -9 1234']
    });
    
    // Network operations
    patterns.set('check port', {
      template: 'lsof -i :{port}',
      description: 'Check what is using a port',
      category: 'network',
      examples: ['lsof -i :3000', 'netstat -an | grep 3000']
    });
    
    return patterns;
  }
  
  private initializeSafetyRules(): SafetyRule[] {
    return [
      {
        pattern: /rm\s+-rf\s+\/|rm\s+-rf\s+~|rm\s+\/\*/,
        level: 'dangerous',
        warning: 'This command could delete important system files',
        impacts: ['Permanent data loss', 'System instability']
      },
      {
        pattern: /chmod\s+777|chmod\s+-R\s+777/,
        level: 'dangerous',
        warning: 'This gives full permissions to everyone',
        impacts: ['Security vulnerability', 'Unauthorized access']
      },
      {
        pattern: /curl.*\|\s*sh|wget.*\|\s*sh/,
        level: 'dangerous',
        warning: 'Executing remote scripts is dangerous',
        impacts: ['Malware installation', 'System compromise']
      },
      {
        pattern: /sudo\s+rm|sudo\s+chmod/,
        level: 'caution',
        warning: 'Using sudo with destructive commands',
        impacts: ['System file changes', 'Permission modifications']
      },
      {
        pattern: />\/dev\/null\s+2>&1/,
        level: 'caution',
        warning: 'Output is being discarded',
        impacts: ['Lost error messages', 'Difficult debugging']
      }
    ];
  }
  
  private matchCommandPattern(input: string): CommandPattern | null {
    const lowerInput = input.toLowerCase();
    
    for (const [pattern, cmd] of this.commandPatterns) {
      if (lowerInput.includes(pattern)) {
        return cmd;
      }
    }
    
    return null;
  }
  
  private applyPattern(
    pattern: CommandPattern, 
    input: string, 
    context: CommandContext
  ): ProcessedCommand {
    let command = pattern.template;
    
    // Simple parameter extraction (in real implementation, use NLP)
    const params = this.extractParameters(input, pattern);
    for (const [key, value] of Object.entries(params)) {
      command = command.replace(`{${key}}`, value);
    }
    
    return {
      command,
      explanation: pattern.description,
      confidence: 0.9,
      category: pattern.category,
      alternatives: pattern.examples?.map(ex => ({
        command: ex,
        description: `Example: ${ex}`,
        advantages: ['Common usage pattern'],
        when_to_use: 'Standard scenario'
      }))
    };
  }
  
  private buildCommandPrompt(input: string, context: CommandContext): string {
    return `Convert this natural language request to a shell command:

Request: "${input}"

Context:
- Current directory: ${context.workingDirectory}
- Shell: ${context.shell}
- Recent commands: ${context.history.slice(-3).join('\n')}

Provide:
1. The exact command to execute
2. A brief explanation
3. Any warnings if the command is potentially dangerous
4. Alternative commands if applicable

Format your response as JSON:
{
  "command": "the shell command",
  "explanation": "what it does",
  "confidence": 0.0-1.0,
  "warnings": ["any warnings"],
  "alternatives": [{"command": "alt", "description": "desc"}]
}`;
  }
  
  private parseAIResponse(
    aiResponse: string, 
    originalInput: string,
    context: CommandContext
  ): ProcessedCommand {
    try {
      // Try to parse as JSON first
      const parsed = JSON.parse(aiResponse);
      return {
        command: parsed.command,
        explanation: parsed.explanation,
        confidence: parsed.confidence || 0.7,
        warnings: parsed.warnings,
        alternatives: parsed.alternatives
      };
    } catch {
      // Fallback: extract command from plain text
      const lines = aiResponse.split('\n');
      const commandLine = lines.find(l => l.trim() && !l.startsWith('#'));
      
      if (commandLine) {
        return {
          command: commandLine.trim(),
          explanation: 'Interpreted from natural language',
          confidence: 0.6
        };
      }
      
      return {
        confidence: 0,
        explanation: 'Could not parse AI response'
      };
    }
  }
  
  private async getAISuggestions(
    partialInput: string, 
    context: CommandContext
  ): Promise<CommandSuggestion[]> {
    // Use AI to generate suggestions
    const aiTask: AITask = {
      id: `terminal-suggest-${Date.now()}`,
      type: TaskType.Completion,
      prompt: `Suggest shell commands for: "${partialInput}" in ${context.shell}`,
      constraints: {
        maxTokens: 200,
        temperature: 0.7
      }
    };
    
    const result = await this.orchestrationEngine.executeTask(aiTask);
    
    // Parse and return suggestions
    // This is simplified - real implementation would parse structured output
    return [];
  }
  
  private extractParameters(input: string, pattern: CommandPattern): Record<string, string> {
    // Simple parameter extraction - in real implementation use NLP
    const params: Record<string, string> = {};
    
    // Extract quoted strings
    const quoted = input.match(/"([^"]+)"/);
    if (quoted) {
      params.message = quoted[1];
      params.name = quoted[1];
      params.file = quoted[1];
    }
    
    // Extract numbers
    const numbers = input.match(/\d+/);
    if (numbers) {
      params.port = numbers[0];
      params.pid = numbers[0];
    }
    
    return params;
  }
  
  private getSaferAlternatives(command: string): string[] {
    const alternatives: string[] = [];
    
    if (command.includes('rm -rf')) {
      alternatives.push(
        'mv {file} /tmp/  # Move to tmp instead of deleting',
        'rm -i {file}     # Interactive mode asks for confirmation'
      );
    }
    
    if (command.includes('chmod 777')) {
      alternatives.push(
        'chmod 755 {file}  # Owner full, others read+execute',
        'chmod 644 {file}  # Owner read+write, others read only'
      );
    }
    
    return alternatives;
  }
}

interface CommandPattern {
  template: string;
  description: string;
  category: CommandCategory;
  examples?: string[];
}

interface SafetyRule {
  pattern: RegExp;
  level: 'caution' | 'dangerous';
  warning: string;
  impacts: string[];
}