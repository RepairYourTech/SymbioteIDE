/**
 * Terminal API Types for AI-Powered Terminal Integration
 * Provides intelligent command execution and terminal management
 */

export interface TerminalConfig {
  shell?: string;
  workingDirectory?: string;
  env?: Record<string, string>;
  dimensions?: TerminalDimensions;
  options?: {
    convertEol?: boolean;
    fontFamily?: string;
    fontSize?: number;
    theme?: TerminalTheme;
    cursorBlink?: boolean;
    scrollback?: number;
  };
}

export interface TerminalDimensions {
  cols: number;
  rows: number;
}

export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  selection: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

export interface TerminalSession {
  id: string;
  name: string;
  shell: string;
  workingDirectory: string;
  status: 'active' | 'idle' | 'busy' | 'terminated';
  created_at: string;
  last_activity: string;
  dimensions: TerminalDimensions;
  buffer_size: number;
  ai_mode: AIAssistanceMode;
}

export interface AIAssistanceMode {
  enabled: boolean;
  level: 'none' | 'suggest' | 'assist' | 'autopilot';
  features: AITerminalFeatures;
}

export interface AITerminalFeatures {
  command_suggestions: boolean;
  error_explanation: boolean;
  command_completion: boolean;
  script_generation: boolean;
  output_analysis: boolean;
  workflow_automation: boolean;
  safety_checks: boolean;
}

export interface TerminalCommand {
  id: string;
  session_id: string;
  command: string;
  timestamp: string;
  working_directory: string;
  exit_code?: number;
  duration?: number;
  output?: TerminalOutput;
  ai_metadata?: AICommandMetadata;
}

export interface TerminalOutput {
  stdout: string;
  stderr: string;
  combined: string;
  truncated: boolean;
  size: number;
}

export interface AICommandMetadata {
  suggested: boolean;
  confidence: number;
  explanation?: string;
  alternatives?: CommandAlternative[];
  warnings?: string[];
  improvements?: string[];
  learned_patterns?: string[];
}

export interface CommandAlternative {
  command: string;
  description: string;
  advantages: string[];
  when_to_use: string;
}

export interface CommandSuggestion {
  command: string;
  description: string;
  confidence: number;
  category: CommandCategory;
  parameters?: CommandParameter[];
  examples?: string[];
  documentation_url?: string;
}

export type CommandCategory =
  | 'file_management'
  | 'process_control'
  | 'network'
  | 'development'
  | 'git'
  | 'docker'
  | 'kubernetes'
  | 'database'
  | 'monitoring'
  | 'security';

export interface CommandParameter {
  name: string;
  description: string;
  type: 'string' | 'number' | 'boolean' | 'path' | 'enum';
  required: boolean;
  default?: any;
  values?: string[];
}

export interface TerminalAutocomplete {
  position: number;
  prefix: string;
  suggestions: AutocompleteSuggestion[];
}

export interface AutocompleteSuggestion {
  text: string;
  type: 'command' | 'path' | 'argument' | 'option' | 'variable';
  description?: string;
  icon?: string;
  score: number;
}

export interface ScriptGenerationRequest {
  description: string;
  language?: 'bash' | 'powershell' | 'python' | 'node';
  context?: {
    platform?: string;
    shell?: string;
    available_tools?: string[];
  };
  requirements?: string[];
  examples?: string[];
}

export interface GeneratedScript {
  script: string;
  language: string;
  description: string;
  parameters: ScriptParameter[];
  dependencies: string[];
  estimated_duration?: number;
  risk_level: 'safe' | 'moderate' | 'dangerous';
  explanation: string;
  test_commands?: string[];
}

export interface ScriptParameter {
  name: string;
  description: string;
  type: string;
  required: boolean;
  default?: any;
  validation?: string;
}

export interface TerminalWorkflow {
  id: string;
  name: string;
  description: string;
  steps: WorkflowStep[];
  variables: Record<string, any>;
  triggers?: WorkflowTrigger[];
  tags: string[];
  created_at: string;
  last_run?: string;
}

export interface WorkflowStep {
  id: string;
  name: string;
  command: string;
  condition?: string;
  on_success?: string;
  on_failure?: string;
  retry?: {
    attempts: number;
    delay: number;
  };
  timeout?: number;
  ai_enhanced?: boolean;
}

export interface WorkflowTrigger {
  type: 'manual' | 'schedule' | 'file_change' | 'git_hook' | 'api';
  config: Record<string, any>;
}

export interface WorkflowExecution {
  id: string;
  workflow_id: string;
  status: 'running' | 'completed' | 'failed' | 'cancelled';
  started_at: string;
  completed_at?: string;
  steps: WorkflowStepExecution[];
  variables: Record<string, any>;
  trigger: string;
}

export interface WorkflowStepExecution {
  step_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';
  started_at?: string;
  completed_at?: string;
  command: string;
  output?: TerminalOutput;
  exit_code?: number;
  error?: string;
  retries?: number;
}

export interface TerminalAnalysis {
  session_id: string;
  period: {
    start: string;
    end: string;
  };
  statistics: TerminalStatistics;
  patterns: CommandPattern[];
  recommendations: string[];
  anomalies: TerminalAnomaly[];
}

export interface TerminalStatistics {
  total_commands: number;
  unique_commands: number;
  error_rate: number;
  average_duration: number;
  most_used_commands: Array<{
    command: string;
    count: number;
    average_duration: number;
  }>;
  peak_usage_times: Array<{
    hour: number;
    count: number;
  }>;
}

export interface CommandPattern {
  pattern: string;
  frequency: number;
  typical_sequence: string[];
  context: string;
  automation_potential: number;
}

export interface TerminalAnomaly {
  timestamp: string;
  command: string;
  type: 'unusual_command' | 'high_error_rate' | 'performance_degradation' | 'security_concern';
  description: string;
  severity: 'low' | 'medium' | 'high';
  recommendation: string;
}

export interface TerminalSafetyCheck {
  command: string;
  safe: boolean;
  risk_level: 'safe' | 'caution' | 'dangerous';
  warnings: string[];
  potential_impacts: string[];
  requires_confirmation: boolean;
  safer_alternatives?: string[];
}

export interface TerminalAPIClient {
  // Session management
  createSession(config?: TerminalConfig): Promise<TerminalSession>;
  getSession(id: string): Promise<TerminalSession>;
  listSessions(): Promise<TerminalSession[]>;
  terminateSession(id: string): Promise<void>;
  
  // Command execution
  executeCommand(session_id: string, command: string): Promise<TerminalCommand>;
  cancelCommand(command_id: string): Promise<void>;
  getCommandHistory(session_id: string, limit?: number): Promise<TerminalCommand[]>;
  
  // AI assistance
  getSuggestions(session_id: string, context: string): Promise<CommandSuggestion[]>;
  getAutocomplete(session_id: string, input: string, position: number): Promise<TerminalAutocomplete>;
  explainError(session_id: string, error: string): Promise<string>;
  generateScript(request: ScriptGenerationRequest): Promise<GeneratedScript>;
  checkSafety(command: string): Promise<TerminalSafetyCheck>;
  
  // Workflow management
  createWorkflow(workflow: Omit<TerminalWorkflow, 'id' | 'created_at'>): Promise<TerminalWorkflow>;
  getWorkflow(id: string): Promise<TerminalWorkflow>;
  updateWorkflow(id: string, updates: Partial<TerminalWorkflow>): Promise<TerminalWorkflow>;
  deleteWorkflow(id: string): Promise<void>;
  executeWorkflow(id: string, variables?: Record<string, any>): Promise<WorkflowExecution>;
  getWorkflowExecution(execution_id: string): Promise<WorkflowExecution>;
  
  // Terminal interaction
  sendInput(session_id: string, data: string): Promise<void>;
  resize(session_id: string, dimensions: TerminalDimensions): Promise<void>;
  getBuffer(session_id: string, lines?: number): Promise<string[]>;
  clear(session_id: string): Promise<void>;
  
  // Analytics and learning
  analyzeUsage(session_id: string, period?: { start: string; end: string }): Promise<TerminalAnalysis>;
  learnFromHistory(session_id: string): Promise<void>;
  exportLearnings(): Promise<CommandPattern[]>;
  importLearnings(patterns: CommandPattern[]): Promise<void>;
  
  // Configuration
  updateAIMode(session_id: string, mode: AIAssistanceMode): Promise<void>;
  setEnvironment(session_id: string, env: Record<string, string>): Promise<void>;
  changeDirectory(session_id: string, path: string): Promise<void>;
}

export interface TerminalEvent {
  type: TerminalEventType;
  session_id: string;
  timestamp: string;
  data: any;
}

export type TerminalEventType =
  | 'session_created'
  | 'session_terminated'
  | 'command_started'
  | 'command_completed'
  | 'output_received'
  | 'error_occurred'
  | 'workflow_started'
  | 'workflow_completed'
  | 'ai_suggestion'
  | 'safety_warning';

export interface TerminalWebSocketMessage {
  type: 'input' | 'output' | 'resize' | 'command' | 'event';
  session_id: string;
  data: any;
}

export interface TerminalShortcut {
  id: string;
  name: string;
  description: string;
  keys: string[];
  command?: string;
  action?: 'clear' | 'interrupt' | 'eof' | 'suspend' | 'resume';
  custom_handler?: string;
}

export interface TerminalPlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  hooks: {
    pre_command?: string;
    post_command?: string;
    on_error?: string;
    on_output?: string;
  };
  commands?: Record<string, string>;
  enabled: boolean;
}