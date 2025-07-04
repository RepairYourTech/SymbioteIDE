/**
 * Review Manager - Manages the lifecycle of code reviews
 */

import * as vscode from 'vscode';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import {
  CodeReview,
  ReviewConfig,
  ReviewRequest,
  ReviewStatus,
  ReviewProgress,
  ReviewSummary,
  ReviewHistory,
  ReviewAnalytics,
  ReviewCategory,
  ReviewSeverity
} from './types';
import { ReviewEngine } from './review-engine';
import { ReviewCacheManager } from './cache/review-cache-manager';
import { HistoryTracker } from './context/history-tracker';
import { TeamRulesManager } from './config/team-rules';
import { ModelConfigManager } from './config/model-config';
import { Neo4jConnectionManager } from '../knowledge/neo4j/connection-manager';
import { QdrantManager } from '../search/qdrant/qdrant-manager';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';

export class ReviewManager extends EventEmitter {
  private logger = new Logger('ReviewManager');
  private engine: ReviewEngine;
  private cache: ReviewCacheManager;
  private history: HistoryTracker;
  private teamRules: TeamRulesManager;
  private modelConfig: ModelConfigManager;
  private activeReviews: Map<string, CodeReview> = new Map();
  private initialized = false;
  
  // Dependencies
  private neo4j?: Neo4jConnectionManager;
  private qdrant?: QdrantManager;
  private orchestrator?: OrchestrationEngine;
  
  // Default configuration
  private config: ReviewConfig = {
    severityThresholds: {
      [ReviewCategory.Security]: ReviewSeverity.Error,
      [ReviewCategory.Performance]: ReviewSeverity.Warning,
      [ReviewCategory.BestPractices]: ReviewSeverity.Warning,
      [ReviewCategory.Maintainability]: ReviewSeverity.Info,
      [ReviewCategory.Accessibility]: ReviewSeverity.Warning,
      [ReviewCategory.Testing]: ReviewSeverity.Warning,
      [ReviewCategory.Documentation]: ReviewSeverity.Info,
      [ReviewCategory.Dependencies]: ReviewSeverity.Warning,
      [ReviewCategory.Architecture]: ReviewSeverity.Warning,
      [ReviewCategory.Style]: ReviewSeverity.Info
    },
    enabledRules: [],
    disabledRules: [],
    modelPreferences: {
      security: 'claude-3.5-sonnet',
      performance: 'gpt-4-turbo',
      general: 'gemini-1.5-pro'
    },
    maxConcurrentAnalysis: 5,
    cacheEnabled: true,
    cacheDuration: 3600, // 1 hour
    gitIntegration: true,
    ciIntegration: false,
    prReviewEnabled: true
  };
  
  constructor(config?: Partial<ReviewConfig>) {
    super();
    if (config) {
      this.config = { ...this.config, ...config };
    }
    
    // Initialize components
    this.cache = new ReviewCacheManager(this.config.cacheDuration);
    this.history = new HistoryTracker();
    this.teamRules = new TeamRulesManager();
    this.modelConfig = new ModelConfigManager(this.config.modelPreferences);
  }
  
  /**
   * Initialize the review manager with external dependencies
   */
  async initialize(
    neo4j?: Neo4jConnectionManager,
    qdrant?: QdrantManager,
    orchestrator?: OrchestrationEngine
  ): Promise<void> {
    if (this.initialized) {
      return;
    }
    
    try {
      this.logger.info('Initializing Review Manager');
      
      // Set dependencies
      this.neo4j = neo4j;
      this.qdrant = qdrant;
      this.orchestrator = orchestrator;
      
      // Initialize engine
      this.engine = new ReviewEngine({
        config: this.config,
        neo4j: this.neo4j,
        qdrant: this.qdrant,
        orchestrator: this.orchestrator
      });
      
      // Initialize components
      await Promise.all([
        this.cache.initialize(),
        this.history.initialize(),
        this.teamRules.initialize(),
        this.modelConfig.initialize()
      ]);
      
      // Learn team standards if workspace is available
      if (vscode.workspace.workspaceFolders) {
        const workspaceRoot = vscode.workspace.workspaceFolders[0].uri;
        await this.learnTeamStandards(workspaceRoot);
      }
      
      // Set up event handlers
      this.setupEventHandlers();
      
      this.initialized = true;
      this.logger.info('Review Manager initialized successfully');
      
    } catch (error) {
      this.logger.error('Failed to initialize Review Manager', error);
      throw error;
    }
  }
  
  /**
   * Review files based on request
   */
  async reviewFiles(
    files: string[] | vscode.Uri[],
    options?: Partial<ReviewRequest>
  ): Promise<CodeReview> {
    if (!this.initialized) {
      await this.initialize();
    }
    
    const reviewId = this.generateReviewId();
    const uris = files.map(f => typeof f === 'string' ? vscode.Uri.file(f) : f);
    
    // Create review request
    const request: ReviewRequest = {
      files: uris,
      scope: options?.scope || 'file',
      categories: options?.categories || Object.values(ReviewCategory),
      depth: options?.depth || 'standard',
      compareWith: options?.compareWith,
      includeContext: options?.includeContext ?? true,
      generateReport: options?.generateReport ?? true
    };
    
    // Check cache
    const cacheKey = this.cache.generateKey(request);
    const cachedReview = await this.cache.get(cacheKey);
    if (cachedReview && this.config.cacheEnabled) {
      this.logger.info('Returning cached review', { reviewId: cachedReview.id });
      return cachedReview;
    }
    
    // Create new review
    const review: CodeReview = {
      id: reviewId,
      projectUri: vscode.workspace.workspaceFolders?.[0]?.uri || uris[0],
      files: uris,
      status: ReviewStatus.Pending,
      startTime: new Date(),
      issues: [],
      suggestions: [],
      metrics: {
        complexity: 0,
        maintainability: 0,
        testCoverage: 0,
        duplicateCode: 0,
        technicalDebt: 0,
        securityScore: 0,
        performanceScore: 0,
        accessibilityScore: 0
      },
      summary: ''
    };
    
    // Store active review
    this.activeReviews.set(reviewId, review);
    this.emit('reviewStarted', { reviewId, request });
    
    try {
      // Update status
      review.status = ReviewStatus.InProgress;
      this.emitProgress(reviewId, 'analyzing', 0, uris.length);
      
      // Perform review
      const result = await this.engine.reviewFiles(request, (progress) => {
        this.emitProgress(
          reviewId,
          progress.phase,
          progress.current,
          progress.total,
          progress.currentFile,
          progress.message
        );
      });
      
      // Update review with results
      review.issues = result.issues;
      review.suggestions = result.suggestions;
      review.metrics = result.metrics;
      review.summary = result.summary;
      review.modelUsed = result.modelUsed;
      review.cost = result.cost;
      review.endTime = new Date();
      review.status = ReviewStatus.Completed;
      
      // Cache result
      if (this.config.cacheEnabled) {
        await this.cache.set(cacheKey, review);
      }
      
      // Track history
      await this.history.addReview(review);
      
      // Emit completion
      this.emit('reviewCompleted', { reviewId, review });
      
      return review;
      
    } catch (error) {
      this.logger.error('Review failed', error);
      
      review.status = ReviewStatus.Failed;
      review.endTime = new Date();
      review.summary = `Review failed: ${error.message}`;
      
      this.emit('reviewFailed', { reviewId, error });
      
      throw error;
      
    } finally {
      this.activeReviews.delete(reviewId);
    }
  }
  
  /**
   * Review workspace
   */
  async reviewWorkspace(options?: Partial<ReviewRequest>): Promise<CodeReview> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
      throw new Error('No workspace folder found');
    }
    
    return this.reviewFiles([], {
      ...options,
      scope: 'workspace'
    });
  }
  
  /**
   * Review changes (git diff)
   */
  async reviewChanges(
    compareWith?: string,
    options?: Partial<ReviewRequest>
  ): Promise<CodeReview> {
    return this.reviewFiles([], {
      ...options,
      scope: 'changes',
      compareWith: compareWith || 'HEAD'
    });
  }
  
  /**
   * Get review by ID
   */
  async getReview(reviewId: string): Promise<CodeReview | null> {
    // Check active reviews
    const activeReview = this.activeReviews.get(reviewId);
    if (activeReview) {
      return activeReview;
    }
    
    // Check history
    return this.history.getReview(reviewId);
  }
  
  /**
   * Get review history
   */
  async getHistory(
    options?: {
      limit?: number;
      offset?: number;
      startDate?: Date;
      endDate?: Date;
    }
  ): Promise<ReviewHistory[]> {
    return this.history.getHistory(options);
  }
  
  /**
   * Get review analytics
   */
  async getAnalytics(
    startDate: Date,
    endDate: Date
  ): Promise<ReviewAnalytics> {
    const reviews = await this.history.getReviewsInPeriod(startDate, endDate);
    
    // Calculate analytics
    const analytics: ReviewAnalytics = {
      period: { start: startDate, end: endDate },
      totalReviews: reviews.length,
      totalIssues: reviews.reduce((sum, r) => sum + r.issues.length, 0),
      issuesTrend: this.calculateIssuesTrend(reviews),
      topIssueCategories: this.calculateTopCategories(reviews),
      codeQualityScore: this.calculateQualityScore(reviews),
      technicalDebtHours: this.calculateTechnicalDebt(reviews),
      modelPerformance: await this.modelConfig.getPerformanceMetrics()
    };
    
    return analytics;
  }
  
  /**
   * Cancel active review
   */
  async cancelReview(reviewId: string): Promise<void> {
    const review = this.activeReviews.get(reviewId);
    if (review) {
      review.status = ReviewStatus.Cancelled;
      review.endTime = new Date();
      this.activeReviews.delete(reviewId);
      this.emit('reviewCancelled', { reviewId });
    }
  }
  
  /**
   * Get review summary
   */
  async getReviewSummary(reviewId: string): Promise<ReviewSummary> {
    const review = await this.getReview(reviewId);
    if (!review) {
      throw new Error(`Review ${reviewId} not found`);
    }
    
    const summary: ReviewSummary = {
      totalIssues: review.issues.length,
      issuesBySeverity: this.groupIssuesBySeverity(review),
      issuesByCategory: this.groupIssuesByCategory(review),
      topIssues: review.issues
        .sort((a, b) => this.getSeverityWeight(b.severity) - this.getSeverityWeight(a.severity))
        .slice(0, 5),
      qualityTrend: await this.calculateQualityTrend(),
      recommendations: this.generateRecommendations(review)
    };
    
    return summary;
  }
  
  /**
   * Apply suggestion
   */
  async applySuggestion(
    suggestionId: string,
    options?: { preview?: boolean }
  ): Promise<void> {
    // Find suggestion in active or historical reviews
    let suggestion;
    let review;
    
    for (const activeReview of this.activeReviews.values()) {
      suggestion = activeReview.suggestions.find(s => s.id === suggestionId);
      if (suggestion) {
        review = activeReview;
        break;
      }
    }
    
    if (!suggestion) {
      const historicalReviews = await this.history.getHistory({ limit: 100 });
      for (const histReview of historicalReviews) {
        const fullReview = await this.getReview(histReview.reviewId);
        if (fullReview) {
          suggestion = fullReview.suggestions.find(s => s.id === suggestionId);
          if (suggestion) {
            review = fullReview;
            break;
          }
        }
      }
    }
    
    if (!suggestion || !review) {
      throw new Error(`Suggestion ${suggestionId} not found`);
    }
    
    // Find the issue
    const issue = review.issues.find(i => i.id === suggestion.issueId);
    if (!issue) {
      throw new Error(`Issue ${suggestion.issueId} not found`);
    }
    
    if (options?.preview) {
      // Show preview
      await vscode.window.showTextDocument(
        issue.location.uri,
        {
          preview: true,
          selection: issue.location.range
        }
      );
      
      // Show diff
      const edit = new vscode.WorkspaceEdit();
      edit.replace(issue.location.uri, issue.location.range, suggestion.code);
      await vscode.workspace.applyEdit(edit, { preview: true });
    } else {
      // Apply the suggestion
      const edit = new vscode.WorkspaceEdit();
      edit.replace(issue.location.uri, issue.location.range, suggestion.code);
      const success = await vscode.workspace.applyEdit(edit);
      
      if (success) {
        // Track acceptance
        await this.history.recordAction(review.id, {
          action: 'accept',
          issueId: issue.id,
          suggestionId: suggestion.id,
          timestamp: new Date()
        });
        
        this.emit('suggestionApplied', { reviewId: review.id, suggestionId });
      }
    }
  }
  
  /**
   * Update configuration
   */
  async updateConfig(config: Partial<ReviewConfig>): Promise<void> {
    this.config = { ...this.config, ...config };
    
    // Update components
    if (config.modelPreferences) {
      await this.modelConfig.updatePreferences(config.modelPreferences);
    }
    
    if (config.cacheDuration !== undefined) {
      this.cache.updateDuration(config.cacheDuration);
    }
    
    // Reinitialize engine with new config
    if (this.engine) {
      await this.engine.updateConfig(this.config);
    }
    
    this.emit('configUpdated', config);
  }
  
  /**
   * Learn team standards from codebase
   */
  private async learnTeamStandards(workspaceUri: vscode.Uri): Promise<void> {
    try {
      this.logger.info('Learning team standards from codebase');
      
      const standards = await this.teamRules.learnFromCodebase(workspaceUri.fsPath);
      
      // Apply learned standards to configuration
      const customRules = this.teamRules.generateRulesFromStandards(standards);
      await this.updateConfig({
        customRules: [...(this.config.customRules || []), ...customRules]
      });
      
      this.logger.info(`Learned ${customRules.length} custom rules from codebase`);
      
    } catch (error) {
      this.logger.warn('Failed to learn team standards', error);
    }
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Listen for file changes to invalidate cache
    vscode.workspace.onDidChangeTextDocument((event) => {
      this.cache.invalidateForFile(event.document.uri);
    });
    
    // Listen for configuration changes
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration('symbiote.codeReview')) {
        const config = vscode.workspace.getConfiguration('symbiote.codeReview');
        this.updateConfig(config as any);
      }
    });
  }
  
  /**
   * Emit progress event
   */
  private emitProgress(
    reviewId: string,
    phase: ReviewProgress['phase'],
    current: number,
    total: number,
    currentFile?: vscode.Uri,
    message?: string
  ): void {
    const progress: ReviewProgress = {
      reviewId,
      phase,
      current,
      total,
      currentFile,
      message
    };
    
    this.emit('reviewProgress', progress);
  }
  
  /**
   * Generate unique review ID
   */
  private generateReviewId(): string {
    return `review_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
  
  /**
   * Group issues by severity
   */
  private groupIssuesBySeverity(review: CodeReview): Record<ReviewSeverity, number> {
    const groups: Record<ReviewSeverity, number> = {
      [ReviewSeverity.Info]: 0,
      [ReviewSeverity.Warning]: 0,
      [ReviewSeverity.Error]: 0,
      [ReviewSeverity.Critical]: 0
    };
    
    review.issues.forEach(issue => {
      groups[issue.severity]++;
    });
    
    return groups;
  }
  
  /**
   * Group issues by category
   */
  private groupIssuesByCategory(review: CodeReview): Record<ReviewCategory, number> {
    const groups: Partial<Record<ReviewCategory, number>> = {};
    
    review.issues.forEach(issue => {
      groups[issue.category] = (groups[issue.category] || 0) + 1;
    });
    
    return groups as Record<ReviewCategory, number>;
  }
  
  /**
   * Get severity weight
   */
  private getSeverityWeight(severity: ReviewSeverity): number {
    const weights = {
      [ReviewSeverity.Info]: 1,
      [ReviewSeverity.Warning]: 2,
      [ReviewSeverity.Error]: 3,
      [ReviewSeverity.Critical]: 4
    };
    return weights[severity] || 0;
  }
  
  /**
   * Calculate issues trend
   */
  private calculateIssuesTrend(reviews: CodeReview[]): Array<{ date: Date; count: number }> {
    const trend: Array<{ date: Date; count: number }> = [];
    
    // Group by date
    const byDate = new Map<string, number>();
    reviews.forEach(review => {
      const dateKey = review.startTime.toISOString().split('T')[0];
      byDate.set(dateKey, (byDate.get(dateKey) || 0) + review.issues.length);
    });
    
    // Convert to array
    byDate.forEach((count, date) => {
      trend.push({ date: new Date(date), count });
    });
    
    // Sort by date
    trend.sort((a, b) => a.date.getTime() - b.date.getTime());
    
    return trend;
  }
  
  /**
   * Calculate top issue categories
   */
  private calculateTopCategories(
    reviews: CodeReview[]
  ): Array<{ category: ReviewCategory; count: number; trend: 'increasing' | 'stable' | 'decreasing' }> {
    const categoryCounts = new Map<ReviewCategory, number[]>();
    
    // Group reviews by week
    const weekGroups = this.groupReviewsByWeek(reviews);
    
    // Count issues by category per week
    weekGroups.forEach(weekReviews => {
      const weekCounts = new Map<ReviewCategory, number>();
      
      weekReviews.forEach(review => {
        review.issues.forEach(issue => {
          weekCounts.set(issue.category, (weekCounts.get(issue.category) || 0) + 1);
        });
      });
      
      // Add to category counts
      Object.values(ReviewCategory).forEach(category => {
        const counts = categoryCounts.get(category) || [];
        counts.push(weekCounts.get(category) || 0);
        categoryCounts.set(category, counts);
      });
    });
    
    // Calculate trends and totals
    const results: Array<{ category: ReviewCategory; count: number; trend: 'increasing' | 'stable' | 'decreasing' }> = [];
    
    categoryCounts.forEach((counts, category) => {
      const total = counts.reduce((sum, c) => sum + c, 0);
      const trend = this.calculateTrend(counts);
      
      results.push({ category, count: total, trend });
    });
    
    // Sort by count
    results.sort((a, b) => b.count - a.count);
    
    return results.slice(0, 5);
  }
  
  /**
   * Calculate quality score
   */
  private calculateQualityScore(reviews: CodeReview[]): number {
    if (reviews.length === 0) return 0;
    
    const scores = reviews.map(r => {
      const metrics = r.metrics;
      return (
        metrics.maintainability * 0.2 +
        metrics.securityScore * 0.2 +
        metrics.performanceScore * 0.2 +
        metrics.testCoverage * 0.2 +
        (100 - metrics.duplicateCode) * 0.1 +
        metrics.accessibilityScore * 0.1
      );
    });
    
    return scores.reduce((sum, s) => sum + s, 0) / scores.length;
  }
  
  /**
   * Calculate technical debt
   */
  private calculateTechnicalDebt(reviews: CodeReview[]): number {
    return reviews.reduce((sum, r) => sum + r.metrics.technicalDebt, 0);
  }
  
  /**
   * Calculate quality trend
   */
  private async calculateQualityTrend(): Promise<'improving' | 'stable' | 'declining'> {
    const recentReviews = await this.history.getHistory({ limit: 10 });
    if (recentReviews.length < 2) return 'stable';
    
    const scores = recentReviews.map(h => h.issuesFound);
    const trend = this.calculateTrend(scores);
    
    // Inverse because fewer issues = better quality
    if (trend === 'increasing') return 'declining';
    if (trend === 'decreasing') return 'improving';
    return 'stable';
  }
  
  /**
   * Generate recommendations
   */
  private generateRecommendations(review: CodeReview): string[] {
    const recommendations: string[] = [];
    
    // Based on issue categories
    const categoryGroups = this.groupIssuesByCategory(review);
    
    if (categoryGroups[ReviewCategory.Security] > 5) {
      recommendations.push('Consider a security audit - multiple security issues detected');
    }
    
    if (categoryGroups[ReviewCategory.Performance] > 3) {
      recommendations.push('Performance optimization needed - several performance issues found');
    }
    
    if (review.metrics.testCoverage < 60) {
      recommendations.push('Increase test coverage - currently below 60%');
    }
    
    if (review.metrics.technicalDebt > 40) {
      recommendations.push('Schedule technical debt reduction - over 40 hours accumulated');
    }
    
    if (review.metrics.duplicateCode > 10) {
      recommendations.push('Refactor duplicate code - over 10% duplication detected');
    }
    
    return recommendations;
  }
  
  /**
   * Group reviews by week
   */
  private groupReviewsByWeek(reviews: CodeReview[]): CodeReview[][] {
    const groups = new Map<string, CodeReview[]>();
    
    reviews.forEach(review => {
      const weekKey = this.getWeekKey(review.startTime);
      const group = groups.get(weekKey) || [];
      group.push(review);
      groups.set(weekKey, group);
    });
    
    return Array.from(groups.values());
  }
  
  /**
   * Get week key for date
   */
  private getWeekKey(date: Date): string {
    const year = date.getFullYear();
    const week = Math.ceil((date.getDate() - date.getDay() + 1) / 7);
    return `${year}-W${week}`;
  }
  
  /**
   * Calculate trend from numbers
   */
  private calculateTrend(values: number[]): 'increasing' | 'stable' | 'decreasing' {
    if (values.length < 2) return 'stable';
    
    // Simple linear regression
    const n = values.length;
    const sumX = (n * (n - 1)) / 2;
    const sumY = values.reduce((sum, v) => sum + v, 0);
    const sumXY = values.reduce((sum, v, i) => sum + v * i, 0);
    const sumX2 = (n * (n - 1) * (2 * n - 1)) / 6;
    
    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
    
    if (Math.abs(slope) < 0.1) return 'stable';
    return slope > 0 ? 'increasing' : 'decreasing';
  }
  
  /**
   * Shutdown the review manager
   */
  async shutdown(): Promise<void> {
    // Cancel active reviews
    for (const reviewId of this.activeReviews.keys()) {
      await this.cancelReview(reviewId);
    }
    
    // Shutdown components
    await Promise.all([
      this.cache.shutdown(),
      this.history.shutdown(),
      this.teamRules.shutdown(),
      this.modelConfig.shutdown()
    ]);
    
    this.removeAllListeners();
    this.initialized = false;
    
    this.logger.info('Review Manager shut down');
  }
}