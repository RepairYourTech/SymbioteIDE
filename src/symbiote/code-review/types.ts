/**
 * Core types for AI-Powered Code Review System
 */

import { Position, Range, Uri } from 'vscode';

/**
 * Severity levels for code review issues
 */
export enum ReviewSeverity {
  Info = 'info',
  Warning = 'warning',
  Error = 'error',
  Critical = 'critical'
}

/**
 * Categories of code review issues
 */
export enum ReviewCategory {
  Security = 'security',
  Performance = 'performance',
  BestPractices = 'best-practices',
  Maintainability = 'maintainability',
  Accessibility = 'accessibility',
  Testing = 'testing',
  Documentation = 'documentation',
  Dependencies = 'dependencies',
  Architecture = 'architecture',
  Style = 'style'
}

/**
 * Status of a code review
 */
export enum ReviewStatus {
  Pending = 'pending',
  InProgress = 'in-progress',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled'
}

/**
 * Type of code change being reviewed
 */
export enum ChangeType {
  Addition = 'addition',
  Modification = 'modification',
  Deletion = 'deletion',
  Rename = 'rename',
  Move = 'move'
}

/**
 * Represents a code location
 */
export interface CodeLocation {
  uri: Uri;
  range: Range;
  snippet?: string;
  context?: {
    before: string;
    after: string;
  };
}

/**
 * Represents a code review issue
 */
export interface ReviewIssue {
  id: string;
  category: ReviewCategory;
  severity: ReviewSeverity;
  title: string;
  description: string;
  location: CodeLocation;
  rule?: string; // Rule or pattern that was violated
  impact?: string; // Impact of the issue
  effort?: number; // Estimated effort to fix (story points)
  tags?: string[];
  metadata?: Record<string, any>;
}

/**
 * Represents a suggested fix for an issue
 */
export interface ReviewSuggestion {
  id: string;
  issueId: string;
  title: string;
  description: string;
  code: string; // Suggested code change
  explanation: string; // Why this fix is recommended
  confidence: number; // 0-1 confidence score
  alternatives?: ReviewSuggestion[]; // Alternative fixes
  estimatedImpact?: {
    performance?: number; // -1 to 1 (-1 worse, 1 better)
    security?: number;
    maintainability?: number;
    testCoverage?: number;
  };
  references?: string[]; // Links to documentation or examples
}

/**
 * Code quality metrics
 */
export interface CodeQualityMetrics {
  complexity: number; // Cyclomatic complexity
  maintainability: number; // 0-100 maintainability index
  testCoverage: number; // 0-100 percentage
  duplicateCode: number; // Percentage of duplicate code
  technicalDebt: number; // Estimated hours
  securityScore: number; // 0-100 security score
  performanceScore: number; // 0-100 performance score
  accessibilityScore: number; // 0-100 accessibility score
}

/**
 * Represents a complete code review
 */
export interface CodeReview {
  id: string;
  projectUri: Uri;
  files: Uri[];
  status: ReviewStatus;
  startTime: Date;
  endTime?: Date;
  issues: ReviewIssue[];
  suggestions: ReviewSuggestion[];
  metrics: CodeQualityMetrics;
  summary: string;
  modelUsed?: string; // AI model used for review
  cost?: number; // Cost of AI usage
  metadata?: {
    branch?: string;
    commit?: string;
    pullRequest?: string;
    author?: string;
    reviewers?: string[];
  };
}

/**
 * Review configuration
 */
export interface ReviewConfig {
  // Severity thresholds
  severityThresholds: {
    [ReviewCategory.Security]: ReviewSeverity;
    [ReviewCategory.Performance]: ReviewSeverity;
    [ReviewCategory.BestPractices]: ReviewSeverity;
    [key: string]: ReviewSeverity;
  };
  
  // Rules to enable/disable
  enabledRules: string[];
  disabledRules: string[];
  
  // Custom rules
  customRules?: CustomRule[];
  
  // AI model preferences
  modelPreferences: {
    security: string;
    performance: string;
    general: string;
  };
  
  // Performance settings
  maxConcurrentAnalysis: number;
  cacheEnabled: boolean;
  cacheDuration: number; // seconds
  
  // Integration settings
  gitIntegration: boolean;
  ciIntegration: boolean;
  prReviewEnabled: boolean;
}

/**
 * Custom review rule
 */
export interface CustomRule {
  id: string;
  name: string;
  description: string;
  category: ReviewCategory;
  severity: ReviewSeverity;
  pattern?: string; // Regex pattern
  ast?: any; // AST pattern
  semantic?: string; // Semantic search query
  handler?: (code: string, context: any) => ReviewIssue | null;
}

/**
 * Team-specific standards learned from codebase
 */
export interface TeamStandards {
  namingConventions: {
    variables: string[];
    functions: string[];
    classes: string[];
    files: string[];
  };
  architecturePatterns: string[];
  commonLibraries: Array<{
    name: string;
    version: string;
    usage: string[];
  }>;
  codePatterns: Array<{
    pattern: string;
    frequency: number;
    description: string;
  }>;
  testingPatterns: {
    framework: string;
    structure: string;
    coverage: number;
  };
}

/**
 * Review request
 */
export interface ReviewRequest {
  files?: Uri[]; // Specific files to review
  scope?: 'file' | 'folder' | 'workspace' | 'changes'; // Review scope
  categories?: ReviewCategory[]; // Specific categories to check
  depth?: 'quick' | 'standard' | 'deep'; // Review depth
  compareWith?: string; // Branch or commit to compare with
  includeContext?: boolean; // Include surrounding code context
  generateReport?: boolean; // Generate detailed report
}

/**
 * Review progress event
 */
export interface ReviewProgress {
  reviewId: string;
  phase: 'analyzing' | 'detecting' | 'suggesting' | 'summarizing';
  current: number;
  total: number;
  currentFile?: Uri;
  message?: string;
}

/**
 * Review result summary
 */
export interface ReviewSummary {
  totalIssues: number;
  issuesBySeverity: Record<ReviewSeverity, number>;
  issuesByCategory: Record<ReviewCategory, number>;
  topIssues: ReviewIssue[];
  qualityTrend: 'improving' | 'stable' | 'declining';
  recommendations: string[];
}

/**
 * AI model performance metrics for reviews
 */
export interface ModelPerformance {
  modelId: string;
  accuracy: number; // Based on user acceptance of suggestions
  speed: number; // Average ms per file
  cost: number; // Average cost per review
  issueDetectionRate: Record<ReviewCategory, number>;
  falsePositiveRate: number;
  userSatisfaction: number; // 0-5 rating
}

/**
 * Review history entry
 */
export interface ReviewHistory {
  reviewId: string;
  timestamp: Date;
  files: string[];
  issuesFound: number;
  issuesFixed: number;
  timeToFix: number; // Average time to fix issues
  userActions: Array<{
    action: 'accept' | 'reject' | 'modify';
    issueId: string;
    timestamp: Date;
  }>;
}

/**
 * Technical debt item
 */
export interface TechnicalDebtItem {
  id: string;
  type: 'code-smell' | 'outdated-dependency' | 'missing-tests' | 'poor-performance' | 'security-risk';
  location: CodeLocation;
  description: string;
  effort: number; // Story points to fix
  priority: 'low' | 'medium' | 'high' | 'critical';
  age: number; // Days since introduced
  impact: string[];
  suggestedFix?: ReviewSuggestion;
}

/**
 * Review analytics
 */
export interface ReviewAnalytics {
  period: {
    start: Date;
    end: Date;
  };
  totalReviews: number;
  totalIssues: number;
  issuesTrend: Array<{
    date: Date;
    count: number;
  }>;
  topIssueCategories: Array<{
    category: ReviewCategory;
    count: number;
    trend: 'increasing' | 'stable' | 'decreasing';
  }>;
  codeQualityScore: number;
  technicalDebtHours: number;
  modelPerformance: ModelPerformance[];
}