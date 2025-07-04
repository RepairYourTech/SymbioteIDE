/**
 * Advanced Debugging AI Types
 */

import { ADKAgentConfig } from '../adk/types';

export interface DebugSession {
  id: string;
  projectId: string;
  userId: string;
  startTime: Date;
  endTime?: Date;
  status: 'active' | 'paused' | 'completed' | 'failed';
  error?: ErrorContext;
  analysis?: DebugAnalysis;
  fixes?: DebugFix[];
  breakpoints?: Breakpoint[];
  memory?: DebugMemory;
}

export interface ErrorContext {
  type: 'runtime' | 'compile' | 'logic' | 'performance' | 'memory';
  message: string;
  stack?: string;
  file?: string;
  line?: number;
  column?: number;
  code?: string;
  language?: string;
  framework?: string;
  environment?: Record<string, any>;
  reproducible?: boolean;
  frequency?: number;
  firstOccurrence?: Date;
  lastOccurrence?: Date;
}

export interface DebugAnalysis {
  rootCause: string;
  explanation: string;
  impact: {
    severity: 'low' | 'medium' | 'high' | 'critical';
    affectedFiles: string[];
    affectedFunctions: string[];
    userImpact: string;
  };
  patterns: ErrorPattern[];
  suggestions: string[];
  confidence: number;
  metadata?: Record<string, any>;
}

export interface ErrorPattern {
  id: string;
  name: string;
  description: string;
  category: string;
  examples: string[];
  frequency: number;
  lastSeen: Date;
  solutions: string[];
}

export interface DebugFix {
  id: string;
  description: string;
  code: string;
  explanation: string;
  confidence: number;
  impact: 'minimal' | 'moderate' | 'significant';
  testCoverage?: TestCoverage;
  alternativeFixes?: DebugFix[];
  appliedAt?: Date;
  success?: boolean;
}

export interface TestCoverage {
  hasTests: boolean;
  testCount: number;
  coverage: number;
  suggestedTests?: string[];
}

export interface Breakpoint {
  id: string;
  file: string;
  line: number;
  condition?: string;
  hitCount?: number;
  logMessage?: string;
  enabled: boolean;
  type: 'standard' | 'conditional' | 'logpoint' | 'exception';
  reason: string;
  confidence: number;
  suggestedBy: 'user' | 'ai';
}

export interface VariableState {
  name: string;
  value: any;
  type: string;
  scope: 'local' | 'closure' | 'global';
  mutated: boolean;
  accessCount: number;
  suspiciousValue?: boolean;
  suggestion?: string;
}

export interface StackFrame {
  id: string;
  functionName: string;
  file: string;
  line: number;
  column: number;
  variables: VariableState[];
  isUserCode: boolean;
  isAsync: boolean;
  hasError: boolean;
  analysis?: string;
}

export interface DebugMemory {
  episodic: EpisodicMemory[];
  semantic: SemanticMemory[];
  working: WorkingMemory;
  patterns: DebugPattern[];
}

export interface EpisodicMemory {
  sessionId: string;
  timestamp: Date;
  error: ErrorContext;
  solution?: DebugFix;
  success: boolean;
  duration: number;
  steps: DebugStep[];
}

export interface SemanticMemory {
  concept: string;
  description: string;
  examples: string[];
  relatedErrors: string[];
  bestPractices: string[];
}

export interface WorkingMemory {
  currentError: ErrorContext;
  investigatedFiles: string[];
  testedHypotheses: string[];
  ruledOutCauses: string[];
  viableFixOptions: DebugFix[];
}

export interface DebugPattern {
  id: string;
  name: string;
  description: string;
  errorSignature: string;
  commonCauses: string[];
  solutions: string[];
  frequency: number;
  successRate: number;
}

export interface DebugStep {
  id: string;
  timestamp: Date;
  action: string;
  agentId: string;
  input: any;
  output: any;
  duration: number;
  success: boolean;
}

// Agent-specific types
export interface DebugAgentRequest {
  sessionId: string;
  error: ErrorContext;
  code?: string;
  history?: DebugStep[];
  preferences?: DebugPreferences;
}

export interface DebugAgentResponse {
  agentId: string;
  action: string;
  result: any;
  confidence: number;
  reasoning?: string;
  nextSteps?: string[];
}

export interface DebugPreferences {
  autoFix: boolean;
  explainInDetail: boolean;
  preferredLanguage?: string;
  testingFramework?: string;
  codeStyle?: Record<string, any>;
  performanceThreshold?: number;
}

// UI Types
export interface DebugPanelState {
  activeSession?: DebugSession;
  recentErrors: ErrorContext[];
  suggestedBreakpoints: Breakpoint[];
  pendingFixes: DebugFix[];
  debugHistory: EpisodicMemory[];
}

export interface DebugCommand {
  command: 'analyze' | 'fix' | 'explain' | 'test' | 'addBreakpoint' | 'inspect';
  target?: string;
  options?: Record<string, any>;
}

// Agent Configurations
export interface DebugAgentConfigs {
  controller: ADKAgentConfig;
  errorAnalyzer: ADKAgentConfig;
  stackNavigator: ADKAgentConfig;
  variableInspector: ADKAgentConfig;
  fixGenerator: ADKAgentConfig;
}

// Metrics
export interface DebugMetrics {
  totalSessions: number;
  successfulFixes: number;
  averageTimeToFix: number;
  commonErrorTypes: Record<string, number>;
  agentPerformance: Record<string, AgentMetrics>;
}

export interface AgentMetrics {
  invocations: number;
  successRate: number;
  averageResponseTime: number;
  averageConfidence: number;
}