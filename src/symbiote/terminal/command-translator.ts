/**
 * Command Translator
 * 
 * Translates natural language to shell commands across different shells
 */

import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { AITask, TaskType } from '../orchestration/interfaces';
import { CommandAlternative } from '../../types/terminal-api';

export interface TranslationRequest {
  naturalLanguage: string;
  targetShell: ShellType;
  context?: TranslationContext;
}

export interface TranslationContext {
  workingDirectory?: string;
  platform?: 'windows' | 'macos' | 'linux';
  availableCommands?: string[];
  userPreferences?: UserPreferences;
}

export interface UserPreferences {
  verbosity?: 'minimal' | 'normal' | 'verbose';
  safety?: 'strict' | 'normal' | 'permissive';
  style?: 'compact' | 'readable' | 'documented';
}

export interface TranslationResult {
  command: string;
  shell: ShellType;
  explanation: string;
  confidence: number;
  alternatives?: CommandAlternative[];
  crossShellVariants?: ShellVariant[];
  parameters?: CommandParameter[];
  warnings?: string[];
}

export interface ShellVariant {
  shell: ShellType;
  command: string;
  notes?: string;
}

export interface CommandParameter {
  name: string;
  value: string;
  description: string;
  optional: boolean;
}

export type ShellType = 'bash' | 'zsh' | 'fish' | 'powershell' | 'cmd' | 'sh';

export class CommandTranslator {
  private orchestrationEngine: OrchestrationEngine;
  private shellProfiles: Map<ShellType, ShellProfile>;
  private commonCommands: Map<string, CommonCommand>;
  
  constructor(orchestrationEngine: OrchestrationEngine) {
    this.orchestrationEngine = orchestrationEngine;
    this.shellProfiles = this.initializeShellProfiles();
    this.commonCommands = this.initializeCommonCommands();
  }
  
  /**
   * Translate natural language to shell command
   */
  async translate(request: TranslationRequest): Promise<TranslationResult> {
    // Check for common command patterns first
    const commonMatch = this.matchCommonCommand(request.naturalLanguage);
    if (commonMatch) {
      return this.translateCommonCommand(commonMatch, request);
    }
    
    // Use AI for complex translations
    return await this.translateWithAI(request);
  }
  
  /**
   * Validate a command for a specific shell
   */
  validateCommand(command: string, shell: ShellType): ValidationResult {
    const profile = this.shellProfiles.get(shell);
    if (!profile) {
      return {
        valid: false,
        errors: [`Unknown shell type: ${shell}`]
      };
    }
    
    const errors: string[] = [];
    const warnings: string[] = [];
    
    // Check for shell-specific syntax
    if (!profile.validateSyntax(command)) {
      errors.push(`Invalid syntax for ${shell}`);
    }
    
    // Check for dangerous patterns
    const safety = this.checkCommandSafety(command, shell);
    if (!safety.safe) {
      warnings.push(...safety.warnings);
    }
    
    return {
      valid: errors.length === 0,
      errors,
      warnings
    };
  }
  
  /**
   * Convert command between shells
   */
  async convertShell(
    command: string, 
    fromShell: ShellType, 
    toShell: ShellType
  ): Promise<string> {
    if (fromShell === toShell) {
      return command;
    }
    
    // Try direct conversion rules
    const directConversion = this.directConvert(command, fromShell, toShell);
    if (directConversion) {
      return directConversion;
    }
    
    // Use AI for complex conversions
    const aiTask: AITask = {
      id: `shell-convert-${Date.now()}`,
      type: TaskType.Translation,
      prompt: `Convert this ${fromShell} command to ${toShell}:\n\n${command}\n\nProvide only the converted command.`,
      constraints: {
        maxTokens: 200,
        temperature: 0.2
      }
    };
    
    const result = await this.orchestrationEngine.executeTask(aiTask);
    
    if (result.status === 'success' && result.content) {
      return result.content.trim();
    }
    
    throw new Error(`Failed to convert command from ${fromShell} to ${toShell}`);
  }
  
  // Private methods
  
  private initializeShellProfiles(): Map<ShellType, ShellProfile> {
    const profiles = new Map<ShellType, ShellProfile>();
    
    profiles.set('bash', {
      name: 'Bash',
      syntaxPatterns: {
        variable: '$VAR or ${VAR}',
        conditional: 'if [ condition ]; then ... fi',
        loop: 'for i in ...; do ... done',
        function: 'function_name() { ... }'
      },
      builtins: ['cd', 'echo', 'export', 'source', 'alias'],
      validateSyntax: (cmd) => !cmd.includes('$PSItem'),
      features: ['arrays', 'functions', 'job-control']
    });
    
    profiles.set('powershell', {
      name: 'PowerShell',
      syntaxPatterns: {
        variable: '$VAR',
        conditional: 'if ($condition) { ... }',
        loop: 'foreach ($item in ...) { ... }',
        function: 'function Function-Name { ... }'
      },
      builtins: ['Get-ChildItem', 'Set-Location', 'Write-Host'],
      validateSyntax: (cmd) => !cmd.includes('${') || cmd.includes('${env:'),
      features: ['objects', 'cmdlets', '.NET-integration']
    });
    
    profiles.set('cmd', {
      name: 'Command Prompt',
      syntaxPatterns: {
        variable: '%VAR%',
        conditional: 'if condition ( ... )',
        loop: 'for %%i in (...) do ...',
        function: ':label ... goto :eof'
      },
      builtins: ['dir', 'cd', 'echo', 'set'],
      validateSyntax: (cmd) => !cmd.includes('$'),
      features: ['batch-scripting', 'limited']
    });
    
    profiles.set('zsh', {
      name: 'Z Shell',
      syntaxPatterns: {
        variable: '$VAR or ${VAR}',
        conditional: 'if [[ condition ]]; then ... fi',
        loop: 'for i in ...; do ... done',
        function: 'function_name() { ... }'
      },
      builtins: ['cd', 'echo', 'export', 'source', 'alias', 'setopt'],
      validateSyntax: (cmd) => true,
      features: ['arrays', 'functions', 'extended-globbing', 'themes']
    });
    
    profiles.set('fish', {
      name: 'Fish Shell',
      syntaxPatterns: {
        variable: '$VAR',
        conditional: 'if condition; ... end',
        loop: 'for i in ...; ... end',
        function: 'function name; ... end'
      },
      builtins: ['cd', 'echo', 'set', 'source'],
      validateSyntax: (cmd) => !cmd.includes('${'),
      features: ['autosuggestions', 'web-config', 'sane-scripting']
    });
    
    return profiles;
  }
  
  private initializeCommonCommands(): Map<string, CommonCommand> {
    const commands = new Map<string, CommonCommand>();
    
    // File operations
    commands.set('list_files', {
      patterns: ['list files', 'show files', 'ls', 'dir'],
      translations: {
        bash: 'ls -la',
        zsh: 'ls -la',
        fish: 'ls -la',
        powershell: 'Get-ChildItem -Force',
        cmd: 'dir /a'
      }
    });
    
    commands.set('create_directory', {
      patterns: ['create directory', 'make dir', 'mkdir'],
      translations: {
        bash: 'mkdir -p {name}',
        zsh: 'mkdir -p {name}',
        fish: 'mkdir -p {name}',
        powershell: 'New-Item -ItemType Directory -Path {name}',
        cmd: 'mkdir {name}'
      }
    });
    
    commands.set('delete_file', {
      patterns: ['delete file', 'remove file', 'rm'],
      translations: {
        bash: 'rm {file}',
        zsh: 'rm {file}',
        fish: 'rm {file}',
        powershell: 'Remove-Item {file}',
        cmd: 'del {file}'
      }
    });
    
    commands.set('copy_file', {
      patterns: ['copy file', 'cp'],
      translations: {
        bash: 'cp {source} {dest}',
        zsh: 'cp {source} {dest}',
        fish: 'cp {source} {dest}',
        powershell: 'Copy-Item {source} {dest}',
        cmd: 'copy {source} {dest}'
      }
    });
    
    // Navigation
    commands.set('change_directory', {
      patterns: ['change directory', 'go to', 'cd'],
      translations: {
        bash: 'cd {path}',
        zsh: 'cd {path}',
        fish: 'cd {path}',
        powershell: 'Set-Location {path}',
        cmd: 'cd {path}'
      }
    });
    
    commands.set('show_path', {
      patterns: ['current directory', 'where am i', 'pwd'],
      translations: {
        bash: 'pwd',
        zsh: 'pwd',
        fish: 'pwd',
        powershell: 'Get-Location',
        cmd: 'cd'
      }
    });
    
    // Process management
    commands.set('list_processes', {
      patterns: ['list processes', 'show processes', 'ps'],
      translations: {
        bash: 'ps aux',
        zsh: 'ps aux',
        fish: 'ps aux',
        powershell: 'Get-Process',
        cmd: 'tasklist'
      }
    });
    
    commands.set('kill_process', {
      patterns: ['kill process', 'stop process', 'terminate'],
      translations: {
        bash: 'kill {pid}',
        zsh: 'kill {pid}',
        fish: 'kill {pid}',
        powershell: 'Stop-Process -Id {pid}',
        cmd: 'taskkill /PID {pid}'
      }
    });
    
    // Environment
    commands.set('set_variable', {
      patterns: ['set variable', 'export', 'define'],
      translations: {
        bash: 'export {name}={value}',
        zsh: 'export {name}={value}',
        fish: 'set -x {name} {value}',
        powershell: '$env:{name} = "{value}"',
        cmd: 'set {name}={value}'
      }
    });
    
    commands.set('show_variables', {
      patterns: ['show variables', 'environment', 'env'],
      translations: {
        bash: 'env',
        zsh: 'env',
        fish: 'env',
        powershell: 'Get-ChildItem env:',
        cmd: 'set'
      }
    });
    
    return commands;
  }
  
  private matchCommonCommand(input: string): CommonCommand | null {
    const lowerInput = input.toLowerCase();
    
    for (const [, command] of this.commonCommands) {
      for (const pattern of command.patterns) {
        if (lowerInput.includes(pattern)) {
          return command;
        }
      }
    }
    
    return null;
  }
  
  private translateCommonCommand(
    command: CommonCommand, 
    request: TranslationRequest
  ): TranslationResult {
    const shellCommand = command.translations[request.targetShell];
    if (!shellCommand) {
      throw new Error(`No translation available for ${request.targetShell}`);
    }
    
    // Extract parameters from natural language
    const params = this.extractParameters(request.naturalLanguage);
    let finalCommand = shellCommand;
    
    for (const [key, value] of Object.entries(params)) {
      finalCommand = finalCommand.replace(`{${key}}`, value);
    }
    
    // Generate cross-shell variants
    const variants: ShellVariant[] = [];
    for (const [shell, cmd] of Object.entries(command.translations)) {
      if (shell !== request.targetShell) {
        let variantCmd = cmd;
        for (const [key, value] of Object.entries(params)) {
          variantCmd = variantCmd.replace(`{${key}}`, value);
        }
        variants.push({
          shell: shell as ShellType,
          command: variantCmd
        });
      }
    }
    
    return {
      command: finalCommand,
      shell: request.targetShell,
      explanation: `${command.patterns[0]} in ${request.targetShell}`,
      confidence: 0.95,
      crossShellVariants: variants
    };
  }
  
  private async translateWithAI(request: TranslationRequest): Promise<TranslationResult> {
    const prompt = this.buildTranslationPrompt(request);
    
    const aiTask: AITask = {
      id: `translate-${Date.now()}`,
      type: TaskType.Translation,
      prompt,
      context: {
        language: 'shell',
        framework: request.targetShell
      },
      constraints: {
        maxTokens: 500,
        temperature: 0.3
      }
    };
    
    const result = await this.orchestrationEngine.executeTask(aiTask);
    
    if (result.status === 'success' && result.content) {
      return this.parseTranslationResponse(result.content, request);
    }
    
    throw new Error('Failed to translate command');
  }
  
  private buildTranslationPrompt(request: TranslationRequest): string {
    const context = request.context || {};
    
    return `Translate this natural language request to a ${request.targetShell} command:

"${request.naturalLanguage}"

Context:
- Target Shell: ${request.targetShell}
- Platform: ${context.platform || 'unknown'}
- Working Directory: ${context.workingDirectory || 'unknown'}

Requirements:
1. Provide the exact command for ${request.targetShell}
2. Include parameter explanations
3. Suggest alternatives if applicable
4. Note any warnings or caveats

Format as JSON:
{
  "command": "exact command",
  "explanation": "what it does",
  "parameters": [{"name": "param", "value": "value", "description": "desc", "optional": false}],
  "alternatives": [{"command": "alt", "description": "when to use"}],
  "warnings": ["any warnings"]
}`;
  }
  
  private parseTranslationResponse(
    response: string, 
    request: TranslationRequest
  ): TranslationResult {
    try {
      const parsed = JSON.parse(response);
      
      return {
        command: parsed.command,
        shell: request.targetShell,
        explanation: parsed.explanation,
        confidence: 0.8,
        parameters: parsed.parameters,
        alternatives: parsed.alternatives,
        warnings: parsed.warnings
      };
    } catch {
      // Fallback parsing
      const lines = response.split('\n');
      const commandLine = lines.find(l => l.trim() && !l.startsWith('#'));
      
      if (commandLine) {
        return {
          command: commandLine.trim(),
          shell: request.targetShell,
          explanation: 'Translated from natural language',
          confidence: 0.6
        };
      }
      
      throw new Error('Could not parse translation response');
    }
  }
  
  private directConvert(
    command: string, 
    fromShell: ShellType, 
    toShell: ShellType
  ): string | null {
    // Simple conversion rules
    const conversions: Record<string, Record<string, string>> = {
      'ls': {
        'bash:powershell': 'Get-ChildItem',
        'bash:cmd': 'dir'
      },
      'pwd': {
        'bash:powershell': 'Get-Location',
        'bash:cmd': 'cd'
      },
      'mkdir': {
        'bash:powershell': 'New-Item -ItemType Directory',
        'cmd:bash': 'mkdir -p'
      }
    };
    
    const baseCommand = command.split(' ')[0];
    const conversionKey = `${fromShell}:${toShell}`;
    
    if (conversions[baseCommand]?.[conversionKey]) {
      return command.replace(baseCommand, conversions[baseCommand][conversionKey]);
    }
    
    return null;
  }
  
  private checkCommandSafety(command: string, shell: ShellType): SafetyCheckResult {
    const warnings: string[] = [];
    let safe = true;
    
    // Universal dangerous patterns
    const dangerousPatterns = [
      { pattern: /rm\s+-rf\s+\//, warning: 'Deleting root directory' },
      { pattern: /chmod\s+777/, warning: 'Setting world-writable permissions' },
      { pattern: />\/dev\/null\s+2>&1/, warning: 'Discarding all output' }
    ];
    
    for (const { pattern, warning } of dangerousPatterns) {
      if (pattern.test(command)) {
        warnings.push(warning);
        safe = false;
      }
    }
    
    // Shell-specific checks
    if (shell === 'powershell') {
      if (command.includes('-Force') && command.includes('Remove-Item')) {
        warnings.push('Force deleting items');
        safe = false;
      }
    }
    
    return { safe, warnings };
  }
  
  private extractParameters(input: string): Record<string, string> {
    const params: Record<string, string> = {};
    
    // Extract quoted strings
    const quotes = input.match(/"([^"]+)"|'([^']+)'/g);
    if (quotes) {
      quotes.forEach((q, i) => {
        const value = q.slice(1, -1);
        params[`param${i}`] = value;
        params.name = params.name || value;
        params.path = params.path || value;
        params.file = params.file || value;
        params.source = params.source || value;
        if (i === 1) params.dest = value;
      });
    }
    
    // Extract numbers
    const numbers = input.match(/\b\d+\b/g);
    if (numbers) {
      params.pid = numbers[0];
      params.port = numbers[0];
    }
    
    // Extract paths
    const paths = input.match(/\/[\w/]+|C:\\[\w\\]+/g);
    if (paths) {
      params.path = paths[0];
    }
    
    return params;
  }
}

interface ShellProfile {
  name: string;
  syntaxPatterns: {
    variable: string;
    conditional: string;
    loop: string;
    function: string;
  };
  builtins: string[];
  validateSyntax: (command: string) => boolean;
  features: string[];
}

interface CommonCommand {
  patterns: string[];
  translations: Record<ShellType, string>;
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings?: string[];
}

interface SafetyCheckResult {
  safe: boolean;
  warnings: string[];
}