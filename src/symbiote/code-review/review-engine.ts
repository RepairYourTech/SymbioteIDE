/**
 * Review Engine - Core engine for performing code reviews
 */

import * as vscode from 'vscode';
import { Logger } from '../utils/logger';
import {
  ReviewRequest,
  ReviewIssue,
  ReviewSuggestion,
  CodeQualityMetrics,
  ReviewProgress,
  ReviewConfig,
  ReviewCategory
} from './types';
import { Neo4jConnectionManager } from '../knowledge/neo4j/connection-manager';
import { QdrantManager } from '../search/qdrant/qdrant-manager';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';

// Analyzers
import { SecurityAnalyzer } from './analyzers/security-analyzer';
import { PerformanceAnalyzer } from './analyzers/performance-analyzer';
import { BestPracticesAnalyzer } from './analyzers/best-practices-analyzer';
import { DependencyAnalyzer } from './analyzers/dependency-analyzer';

// Context builders
import { ReviewContextBuilder } from './context/review-context-builder';
import { ImpactAnalyzer } from './context/impact-analyzer';

// AI components
import { ReviewAIInterface } from './ai/review-ai-interface';
import { SuggestionGenerator } from './ai/suggestion-generator';
import { ExplanationGenerator } from './ai/explanation-generator';

// Semantic components
import { SimilarityFinder } from './semantic/similarity-finder';
import { PatternMatcher } from './semantic/pattern-matcher';

export interface ReviewEngineConfig {
  config: ReviewConfig;
  neo4j?: Neo4jConnectionManager;
  qdrant?: QdrantManager;
  orchestrator?: OrchestrationEngine;
}

export interface ReviewResult {
  issues: ReviewIssue[];
  suggestions: ReviewSuggestion[];
  metrics: CodeQualityMetrics;
  summary: string;
  modelUsed?: string;
  cost?: number;
}

export class ReviewEngine {
  private logger = new Logger('ReviewEngine');
  private config: ReviewConfig;
  
  // Dependencies
  private neo4j?: Neo4jConnectionManager;
  private qdrant?: QdrantManager;
  private orchestrator?: OrchestrationEngine;
  
  // Analyzers
  private securityAnalyzer: SecurityAnalyzer;
  private performanceAnalyzer: PerformanceAnalyzer;
  private bestPracticesAnalyzer: BestPracticesAnalyzer;
  private dependencyAnalyzer: DependencyAnalyzer;
  
  // Context and AI
  private contextBuilder: ReviewContextBuilder;
  private impactAnalyzer: ImpactAnalyzer;
  private aiInterface: ReviewAIInterface;
  private suggestionGenerator: SuggestionGenerator;
  private explanationGenerator: ExplanationGenerator;
  
  // Semantic components
  private similarityFinder: SimilarityFinder;
  private patternMatcher: PatternMatcher;
  
  constructor(engineConfig: ReviewEngineConfig) {
    this.config = engineConfig.config;
    this.neo4j = engineConfig.neo4j;
    this.qdrant = engineConfig.qdrant;
    this.orchestrator = engineConfig.orchestrator;
    
    // Initialize analyzers
    this.securityAnalyzer = new SecurityAnalyzer(this.config);
    this.performanceAnalyzer = new PerformanceAnalyzer(this.config);
    this.bestPracticesAnalyzer = new BestPracticesAnalyzer(this.config);
    this.dependencyAnalyzer = new DependencyAnalyzer(this.config);
    
    // Initialize context builders
    this.contextBuilder = new ReviewContextBuilder(this.neo4j, this.qdrant);
    this.impactAnalyzer = new ImpactAnalyzer(this.neo4j);
    
    // Initialize AI components
    this.aiInterface = new ReviewAIInterface(this.orchestrator, this.config);
    this.suggestionGenerator = new SuggestionGenerator(this.aiInterface);
    this.explanationGenerator = new ExplanationGenerator(this.aiInterface);
    
    // Initialize semantic components
    this.similarityFinder = new SimilarityFinder(this.qdrant);
    this.patternMatcher = new PatternMatcher(this.qdrant);
  }
  
  /**
   * Review files based on request
   */
  async reviewFiles(
    request: ReviewRequest,
    progressCallback?: (progress: ReviewProgress) => void
  ): Promise<ReviewResult> {
    const startTime = Date.now();
    const issues: ReviewIssue[] = [];
    const suggestions: ReviewSuggestion[] = [];
    let totalCost = 0;
    const modelsUsed: Set<string> = new Set();
    
    try {
      // Get files to review
      const files = await this.getFilesToReview(request);
      const totalFiles = files.length;
      
      this.logger.info(`Starting review of ${totalFiles} files`);
      
      // Phase 1: Analysis
      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        progressCallback?.({
          reviewId: '',
          phase: 'analyzing',
          current: i + 1,
          total: totalFiles,
          currentFile: file,
          message: `Analyzing ${file.fsPath}`
        });
        
        // Read file content
        const content = await this.readFileContent(file);
        if (!content) continue;
        
        // Build context for the file
        const context = await this.contextBuilder.buildContext(file, {
          includeReferences: request.includeContext,
          includeDependencies: true,
          includeHistory: true
        });
        
        // Run analyzers based on categories
        const fileIssues = await this.runAnalyzers(file, content, context, request.categories);
        issues.push(...fileIssues);
      }
      
      // Phase 2: Pattern Detection
      progressCallback?.({
        reviewId: '',
        phase: 'detecting',
        current: 0,
        total: issues.length,
        message: 'Detecting patterns and similar issues'
      });
      
      // Find similar issues across codebase
      const enrichedIssues = await this.enrichIssuesWithSimilarities(issues);
      
      // Detect vulnerability patterns
      const patternIssues = await this.detectPatterns(files, request);
      issues.push(...patternIssues);
      
      // Phase 3: Suggestion Generation
      progressCallback?.({
        reviewId: '',
        phase: 'suggesting',
        current: 0,
        total: enrichedIssues.length,
        message: 'Generating fix suggestions'
      });
      
      // Generate suggestions for issues
      for (let i = 0; i < enrichedIssues.length; i++) {
        const issue = enrichedIssues[i];
        progressCallback?.({
          reviewId: '',
          phase: 'suggesting',
          current: i + 1,
          total: enrichedIssues.length,
          message: `Generating suggestions for ${issue.title}`
        });
        
        const issueSuggestions = await this.suggestionGenerator.generateSuggestions(issue, {
          generateAlternatives: request.depth === 'deep',
          includeExplanations: true,
          considerImpact: request.depth !== 'quick'
        });
        
        suggestions.push(...issueSuggestions);
        
        // Track AI usage
        if (issueSuggestions.length > 0) {
          const result = await this.aiInterface.getLastResult();
          if (result) {
            totalCost += result.cost || 0;
            modelsUsed.add(result.modelUsed || 'unknown');
          }
        }
      }
      
      // Phase 4: Impact Analysis
      if (request.depth !== 'quick') {
        progressCallback?.({
          reviewId: '',
          phase: 'analyzing',
          current: 0,
          total: 1,
          message: 'Analyzing impact of issues'
        });
        
        // Analyze impact of issues
        await this.analyzeImpact(enrichedIssues, suggestions);
      }
      
      // Phase 5: Summary Generation
      progressCallback?.({
        reviewId: '',
        phase: 'summarizing',
        current: 0,
        total: 1,
        message: 'Generating review summary'
      });
      
      // Calculate metrics
      const metrics = await this.calculateMetrics(files, enrichedIssues, suggestions);
      
      // Generate summary
      const summary = await this.generateSummary(enrichedIssues, suggestions, metrics);
      
      // Track summary generation cost
      const summaryResult = await this.aiInterface.getLastResult();
      if (summaryResult) {
        totalCost += summaryResult.cost || 0;
        modelsUsed.add(summaryResult.modelUsed || 'unknown');
      }
      
      const result: ReviewResult = {
        issues: enrichedIssues,
        suggestions,
        metrics,
        summary,
        modelUsed: Array.from(modelsUsed).join(', '),
        cost: totalCost
      };
      
      this.logger.info(`Review completed in ${Date.now() - startTime}ms`, {
        issuesFound: enrichedIssues.length,
        suggestionsGenerated: suggestions.length,
        cost: totalCost
      });
      
      return result;
      
    } catch (error) {
      this.logger.error('Review failed', error);
      throw error;
    }
  }
  
  /**
   * Get files to review based on request
   */
  private async getFilesToReview(request: ReviewRequest): Promise<vscode.Uri[]> {
    if (request.files && request.files.length > 0) {
      return request.files;
    }
    
    switch (request.scope) {
      case 'workspace':
        return this.getWorkspaceFiles();
        
      case 'folder':
        // TODO: Implement folder scope
        return [];
        
      case 'changes':
        return this.getChangedFiles(request.compareWith);
        
      default:
        return [];
    }
  }
  
  /**
   * Get all workspace files
   */
  private async getWorkspaceFiles(): Promise<vscode.Uri[]> {
    const pattern = '**/*.{ts,js,tsx,jsx,py,java,go,rs,cpp,c,h,cs}';
    const excludePattern = '**/node_modules/**';
    
    const files = await vscode.workspace.findFiles(pattern, excludePattern);
    return files;
  }
  
  /**
   * Get changed files from git
   */
  private async getChangedFiles(compareWith?: string): Promise<vscode.Uri[]> {
    // TODO: Implement git integration to get changed files
    return [];
  }
  
  /**
   * Read file content
   */
  private async readFileContent(file: vscode.Uri): Promise<string | null> {
    try {
      const document = await vscode.workspace.openTextDocument(file);
      return document.getText();
    } catch (error) {
      this.logger.warn(`Failed to read file ${file.fsPath}`, error);
      return null;
    }
  }
  
  /**
   * Run analyzers on file
   */
  private async runAnalyzers(
    file: vscode.Uri,
    content: string,
    context: any,
    categories?: ReviewCategory[]
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    const categoriesToRun = categories || Object.values(ReviewCategory);
    
    // Run analyzers in parallel
    const analyzerPromises: Promise<ReviewIssue[]>[] = [];
    
    if (categoriesToRun.includes(ReviewCategory.Security)) {
      analyzerPromises.push(this.securityAnalyzer.analyze(file, content, context));
    }
    
    if (categoriesToRun.includes(ReviewCategory.Performance)) {
      analyzerPromises.push(this.performanceAnalyzer.analyze(file, content, context));
    }
    
    if (categoriesToRun.includes(ReviewCategory.BestPractices)) {
      analyzerPromises.push(this.bestPracticesAnalyzer.analyze(file, content, context));
    }
    
    if (categoriesToRun.includes(ReviewCategory.Dependencies)) {
      analyzerPromises.push(this.dependencyAnalyzer.analyze(file, content, context));
    }
    
    const results = await Promise.all(analyzerPromises);
    results.forEach(result => issues.push(...result));
    
    return issues;
  }
  
  /**
   * Enrich issues with similarity information
   */
  private async enrichIssuesWithSimilarities(issues: ReviewIssue[]): Promise<ReviewIssue[]> {
    if (!this.qdrant) {
      return issues;
    }
    
    const enrichedIssues: ReviewIssue[] = [];
    
    for (const issue of issues) {
      try {
        // Find similar issues
        const similarIssues = await this.similarityFinder.findSimilarIssues(issue);
        
        // Enrich issue with similarity data
        const enriched = {
          ...issue,
          metadata: {
            ...issue.metadata,
            similarIssues: similarIssues.map(s => ({
              id: s.id,
              similarity: s.score,
              location: s.payload.location
            }))
          }
        };
        
        enrichedIssues.push(enriched);
      } catch (error) {
        this.logger.warn('Failed to enrich issue with similarities', error);
        enrichedIssues.push(issue);
      }
    }
    
    return enrichedIssues;
  }
  
  /**
   * Detect patterns across files
   */
  private async detectPatterns(
    files: vscode.Uri[],
    request: ReviewRequest
  ): Promise<ReviewIssue[]> {
    if (!this.qdrant || request.depth === 'quick') {
      return [];
    }
    
    try {
      const patterns = await this.patternMatcher.detectPatterns(files, {
        categories: request.categories,
        includeKnownVulnerabilities: true,
        includeAntiPatterns: true
      });
      
      return patterns;
    } catch (error) {
      this.logger.warn('Pattern detection failed', error);
      return [];
    }
  }
  
  /**
   * Analyze impact of issues
   */
  private async analyzeImpact(
    issues: ReviewIssue[],
    suggestions: ReviewSuggestion[]
  ): Promise<void> {
    if (!this.neo4j) {
      return;
    }
    
    for (const issue of issues) {
      try {
        const impact = await this.impactAnalyzer.analyzeImpact(issue);
        
        // Update issue with impact data
        issue.impact = impact.description;
        issue.metadata = {
          ...issue.metadata,
          impactedFiles: impact.affectedFiles,
          impactedComponents: impact.affectedComponents,
          riskLevel: impact.riskLevel
        };
        
        // Update related suggestions
        const relatedSuggestions = suggestions.filter(s => s.issueId === issue.id);
        for (const suggestion of relatedSuggestions) {
          suggestion.estimatedImpact = {
            performance: impact.performanceImpact,
            security: impact.securityImpact,
            maintainability: impact.maintainabilityImpact,
            testCoverage: impact.testCoverageImpact
          };
        }
      } catch (error) {
        this.logger.warn('Impact analysis failed for issue', error);
      }
    }
  }
  
  /**
   * Calculate code quality metrics
   */
  private async calculateMetrics(
    files: vscode.Uri[],
    issues: ReviewIssue[],
    suggestions: ReviewSuggestion[]
  ): Promise<CodeQualityMetrics> {
    // Basic metrics calculation
    const metrics: CodeQualityMetrics = {
      complexity: 0,
      maintainability: 100,
      testCoverage: 0,
      duplicateCode: 0,
      technicalDebt: 0,
      securityScore: 100,
      performanceScore: 100,
      accessibilityScore: 100
    };
    
    // Calculate based on issues
    issues.forEach(issue => {
      // Reduce scores based on issue severity and category
      const severityImpact = this.getSeverityImpact(issue.severity);
      
      switch (issue.category) {
        case ReviewCategory.Security:
          metrics.securityScore -= severityImpact;
          break;
        case ReviewCategory.Performance:
          metrics.performanceScore -= severityImpact;
          break;
        case ReviewCategory.Accessibility:
          metrics.accessibilityScore -= severityImpact;
          break;
        case ReviewCategory.Maintainability:
          metrics.maintainability -= severityImpact;
          break;
      }
      
      // Add to technical debt
      metrics.technicalDebt += issue.effort || 1;
    });
    
    // Ensure scores are within bounds
    metrics.securityScore = Math.max(0, metrics.securityScore);
    metrics.performanceScore = Math.max(0, metrics.performanceScore);
    metrics.accessibilityScore = Math.max(0, metrics.accessibilityScore);
    metrics.maintainability = Math.max(0, metrics.maintainability);
    
    // TODO: Calculate actual complexity, test coverage, and duplicate code metrics
    
    return metrics;
  }
  
  /**
   * Generate review summary
   */
  private async generateSummary(
    issues: ReviewIssue[],
    suggestions: ReviewSuggestion[],
    metrics: CodeQualityMetrics
  ): Promise<string> {
    const summaryData = {
      totalIssues: issues.length,
      criticalIssues: issues.filter(i => i.severity === 'critical').length,
      errorIssues: issues.filter(i => i.severity === 'error').length,
      warningIssues: issues.filter(i => i.severity === 'warning').length,
      infoIssues: issues.filter(i => i.severity === 'info').length,
      suggestionsGenerated: suggestions.length,
      metrics,
      topCategories: this.getTopCategories(issues),
      estimatedEffort: issues.reduce((sum, i) => sum + (i.effort || 0), 0)
    };
    
    return this.explanationGenerator.generateReviewSummary(summaryData);
  }
  
  /**
   * Get severity impact score
   */
  private getSeverityImpact(severity: string): number {
    const impacts: Record<string, number> = {
      critical: 25,
      error: 15,
      warning: 5,
      info: 1
    };
    return impacts[severity] || 0;
  }
  
  /**
   * Get top issue categories
   */
  private getTopCategories(issues: ReviewIssue[]): Array<{ category: string; count: number }> {
    const counts = new Map<string, number>();
    
    issues.forEach(issue => {
      counts.set(issue.category, (counts.get(issue.category) || 0) + 1);
    });
    
    return Array.from(counts.entries())
      .map(([category, count]) => ({ category, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 3);
  }
  
  /**
   * Update configuration
   */
  async updateConfig(config: ReviewConfig): Promise<void> {
    this.config = config;
    
    // Update analyzers
    this.securityAnalyzer.updateConfig(config);
    this.performanceAnalyzer.updateConfig(config);
    this.bestPracticesAnalyzer.updateConfig(config);
    this.dependencyAnalyzer.updateConfig(config);
    
    // Update AI interface
    this.aiInterface.updateConfig(config);
  }
}