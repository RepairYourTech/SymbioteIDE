/**
 * History Tracker - Tracks code review history and actions
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';
import { Logger } from '../../utils/logger';
import { CodeReview, ReviewHistory } from '../types';

export interface ReviewAction {
  action: 'accept' | 'reject' | 'modify' | 'defer';
  issueId: string;
  suggestionId?: string;
  timestamp: Date;
  comment?: string;
}

export interface HistoryEntry {
  reviewId: string;
  review: CodeReview;
  actions: ReviewAction[];
  timestamp: Date;
}

export class HistoryTracker {
  private logger = new Logger('HistoryTracker');
  private historyPath: string;
  private history: Map<string, HistoryEntry> = new Map();
  private initialized = false;
  
  constructor() {
    // Store history in workspace state directory
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
      this.historyPath = path.join(
        workspaceFolder.uri.fsPath,
        '.vscode',
        'symbiote',
        'review-history'
      );
    } else {
      this.historyPath = path.join(process.cwd(), '.review-history');
    }
  }
  
  /**
   * Initialize history tracker
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }
    
    try {
      // Ensure history directory exists
      await fs.mkdir(this.historyPath, { recursive: true });
      
      // Load existing history
      await this.loadHistory();
      
      this.initialized = true;
      this.logger.info('History tracker initialized');
      
    } catch (error) {
      this.logger.error('Failed to initialize history tracker', error);
      throw error;
    }
  }
  
  /**
   * Add a review to history
   */
  async addReview(review: CodeReview): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }
    
    try {
      const entry: HistoryEntry = {
        reviewId: review.id,
        review: this.sanitizeReview(review),
        actions: [],
        timestamp: new Date()
      };
      
      this.history.set(review.id, entry);
      
      // Save to disk
      await this.saveReviewToDisk(entry);
      
      // Cleanup old entries
      await this.cleanupOldEntries();
      
      this.logger.info(`Added review ${review.id} to history`);
      
    } catch (error) {
      this.logger.error('Failed to add review to history', error);
    }
  }
  
  /**
   * Record an action on a review
   */
  async recordAction(
    reviewId: string,
    action: ReviewAction
  ): Promise<void> {
    const entry = this.history.get(reviewId);
    if (!entry) {
      this.logger.warn(`Review ${reviewId} not found in history`);
      return;
    }
    
    try {
      entry.actions.push(action);
      
      // Update on disk
      await this.saveReviewToDisk(entry);
      
      // Update analytics
      await this.updateAnalytics(entry, action);
      
      this.logger.info(`Recorded ${action.action} action for review ${reviewId}`);
      
    } catch (error) {
      this.logger.error('Failed to record action', error);
    }
  }
  
  /**
   * Get review by ID
   */
  async getReview(reviewId: string): Promise<CodeReview | null> {
    const entry = this.history.get(reviewId);
    if (entry) {
      return entry.review;
    }
    
    // Try to load from disk
    try {
      const filePath = path.join(this.historyPath, `${reviewId}.json`);
      const data = await fs.readFile(filePath, 'utf-8');
      const loaded: HistoryEntry = JSON.parse(data);
      
      // Restore dates
      loaded.timestamp = new Date(loaded.timestamp);
      loaded.review.startTime = new Date(loaded.review.startTime);
      if (loaded.review.endTime) {
        loaded.review.endTime = new Date(loaded.review.endTime);
      }
      
      this.history.set(reviewId, loaded);
      return loaded.review;
      
    } catch (error) {
      return null;
    }
  }
  
  /**
   * Get review history
   */
  async getHistory(options?: {
    limit?: number;
    offset?: number;
    startDate?: Date;
    endDate?: Date;
  }): Promise<ReviewHistory[]> {
    const allEntries = Array.from(this.history.values());
    
    // Filter by date range
    let filtered = allEntries;
    if (options?.startDate || options?.endDate) {
      filtered = allEntries.filter(entry => {
        const date = entry.timestamp;
        if (options.startDate && date < options.startDate) return false;
        if (options.endDate && date > options.endDate) return false;
        return true;
      });
    }
    
    // Sort by timestamp (newest first)
    filtered.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
    
    // Apply pagination
    const offset = options?.offset || 0;
    const limit = options?.limit || 100;
    const paginated = filtered.slice(offset, offset + limit);
    
    // Convert to ReviewHistory format
    return paginated.map(entry => this.toReviewHistory(entry));
  }
  
  /**
   * Get reviews in a time period
   */
  async getReviewsInPeriod(
    startDate: Date,
    endDate: Date
  ): Promise<CodeReview[]> {
    const entries = await this.getHistory({ startDate, endDate });
    const reviews: CodeReview[] = [];
    
    for (const entry of entries) {
      const review = await this.getReview(entry.reviewId);
      if (review) {
        reviews.push(review);
      }
    }
    
    return reviews;
  }
  
  /**
   * Get analytics data
   */
  async getAnalytics(): Promise<{
    totalReviews: number;
    acceptanceRate: number;
    averageIssuesPerReview: number;
    averageTimeToFix: number;
    topIssueTypes: Array<{ type: string; count: number }>;
  }> {
    const entries = Array.from(this.history.values());
    
    let totalIssues = 0;
    let acceptedSuggestions = 0;
    let totalSuggestions = 0;
    let fixTimes: number[] = [];
    const issueTypes = new Map<string, number>();
    
    for (const entry of entries) {
      totalIssues += entry.review.issues.length;
      
      // Count issue types
      for (const issue of entry.review.issues) {
        const count = issueTypes.get(issue.category) || 0;
        issueTypes.set(issue.category, count + 1);
      }
      
      // Count accepted suggestions
      for (const action of entry.actions) {
        if (action.suggestionId) {
          totalSuggestions++;
          if (action.action === 'accept') {
            acceptedSuggestions++;
            
            // Calculate time to fix
            if (entry.review.startTime) {
              const timeDiff = action.timestamp.getTime() - 
                entry.review.startTime.getTime();
              fixTimes.push(timeDiff / 1000 / 60); // minutes
            }
          }
        }
      }
    }
    
    // Calculate averages
    const avgIssues = entries.length > 0 ? totalIssues / entries.length : 0;
    const acceptanceRate = totalSuggestions > 0 ? 
      acceptedSuggestions / totalSuggestions : 0;
    const avgTimeToFix = fixTimes.length > 0 ?
      fixTimes.reduce((a, b) => a + b, 0) / fixTimes.length : 0;
    
    // Get top issue types
    const topIssueTypes = Array.from(issueTypes.entries())
      .map(([type, count]) => ({ type, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5);
    
    return {
      totalReviews: entries.length,
      acceptanceRate,
      averageIssuesPerReview: avgIssues,
      averageTimeToFix,
      topIssueTypes
    };
  }
  
  /**
   * Load history from disk
   */
  private async loadHistory(): Promise<void> {
    try {
      const files = await fs.readdir(this.historyPath);
      const jsonFiles = files.filter(f => f.endsWith('.json'));
      
      for (const file of jsonFiles) {
        try {
          const filePath = path.join(this.historyPath, file);
          const data = await fs.readFile(filePath, 'utf-8');
          const entry: HistoryEntry = JSON.parse(data);
          
          // Restore dates
          entry.timestamp = new Date(entry.timestamp);
          entry.review.startTime = new Date(entry.review.startTime);
          if (entry.review.endTime) {
            entry.review.endTime = new Date(entry.review.endTime);
          }
          
          for (const action of entry.actions) {
            action.timestamp = new Date(action.timestamp);
          }
          
          this.history.set(entry.reviewId, entry);
          
        } catch (error) {
          this.logger.warn(`Failed to load history file ${file}`, error);
        }
      }
      
      this.logger.info(`Loaded ${this.history.size} reviews from history`);
      
    } catch (error) {
      this.logger.warn('Failed to load history', error);
    }
  }
  
  /**
   * Save review to disk
   */
  private async saveReviewToDisk(entry: HistoryEntry): Promise<void> {
    const filePath = path.join(this.historyPath, `${entry.reviewId}.json`);
    const data = JSON.stringify(entry, null, 2);
    await fs.writeFile(filePath, data, 'utf-8');
  }
  
  /**
   * Cleanup old entries
   */
  private async cleanupOldEntries(): Promise<void> {
    const maxAge = 90 * 24 * 60 * 60 * 1000; // 90 days
    const now = Date.now();
    const toDelete: string[] = [];
    
    for (const [reviewId, entry] of this.history) {
      if (now - entry.timestamp.getTime() > maxAge) {
        toDelete.push(reviewId);
      }
    }
    
    for (const reviewId of toDelete) {
      this.history.delete(reviewId);
      const filePath = path.join(this.historyPath, `${reviewId}.json`);
      try {
        await fs.unlink(filePath);
      } catch (error) {
        // File might not exist
      }
    }
    
    if (toDelete.length > 0) {
      this.logger.info(`Cleaned up ${toDelete.length} old history entries`);
    }
  }
  
  /**
   * Update analytics
   */
  private async updateAnalytics(
    entry: HistoryEntry,
    action: ReviewAction
  ): Promise<void> {
    // This could update a separate analytics database
    // For now, just log
    this.logger.debug('Analytics updated', {
      reviewId: entry.reviewId,
      action: action.action,
      issueId: action.issueId
    });
  }
  
  /**
   * Sanitize review for storage
   */
  private sanitizeReview(review: CodeReview): CodeReview {
    // Remove large data that can be reconstructed
    return {
      ...review,
      files: review.files.map(f => vscode.Uri.file(f.fsPath)), // Ensure URIs are serializable
      issues: review.issues.map(issue => ({
        ...issue,
        location: {
          ...issue.location,
          uri: vscode.Uri.file(issue.location.uri.fsPath),
          snippet: issue.location.snippet?.substring(0, 200) // Limit snippet size
        }
      })),
      suggestions: review.suggestions.map(s => ({
        ...s,
        code: s.code.substring(0, 1000) // Limit code size
      }))
    };
  }
  
  /**
   * Convert to ReviewHistory format
   */
  private toReviewHistory(entry: HistoryEntry): ReviewHistory {
    const acceptedCount = entry.actions.filter(a => 
      a.action === 'accept' && a.suggestionId
    ).length;
    
    const fixTimes = entry.actions
      .filter(a => a.action === 'accept')
      .map(a => {
        if (entry.review.startTime) {
          return a.timestamp.getTime() - entry.review.startTime.getTime();
        }
        return 0;
      })
      .filter(t => t > 0);
    
    const avgTimeToFix = fixTimes.length > 0 ?
      fixTimes.reduce((a, b) => a + b, 0) / fixTimes.length / 1000 / 60 : 0; // minutes
    
    return {
      reviewId: entry.reviewId,
      timestamp: entry.timestamp,
      files: entry.review.files.map(f => f.fsPath),
      issuesFound: entry.review.issues.length,
      issuesFixed: acceptedCount,
      timeToFix: avgTimeToFix,
      userActions: entry.actions
    };
  }
  
  /**
   * Export history to file
   */
  async exportHistory(outputPath: string): Promise<void> {
    const entries = Array.from(this.history.values());
    const exportData = {
      version: '1.0',
      exportDate: new Date(),
      entries: entries.map(e => ({
        ...e,
        review: this.sanitizeReview(e.review)
      }))
    };
    
    await fs.writeFile(outputPath, JSON.stringify(exportData, null, 2), 'utf-8');
    this.logger.info(`Exported ${entries.length} reviews to ${outputPath}`);
  }
  
  /**
   * Import history from file
   */
  async importHistory(inputPath: string): Promise<void> {
    try {
      const data = await fs.readFile(inputPath, 'utf-8');
      const importData = JSON.parse(data);
      
      for (const entry of importData.entries) {
        // Restore dates
        entry.timestamp = new Date(entry.timestamp);
        entry.review.startTime = new Date(entry.review.startTime);
        if (entry.review.endTime) {
          entry.review.endTime = new Date(entry.review.endTime);
        }
        
        for (const action of entry.actions) {
          action.timestamp = new Date(action.timestamp);
        }
        
        // Restore URIs
        entry.review.files = entry.review.files.map((f: any) => 
          vscode.Uri.file(typeof f === 'string' ? f : f.fsPath)
        );
        
        for (const issue of entry.review.issues) {
          issue.location.uri = vscode.Uri.file(
            typeof issue.location.uri === 'string' ? 
              issue.location.uri : issue.location.uri.fsPath
          );
        }
        
        this.history.set(entry.reviewId, entry);
        await this.saveReviewToDisk(entry);
      }
      
      this.logger.info(`Imported ${importData.entries.length} reviews`);
      
    } catch (error) {
      this.logger.error('Failed to import history', error);
      throw error;
    }
  }
  
  /**
   * Shutdown the history tracker
   */
  async shutdown(): Promise<void> {
    // Save any pending changes
    for (const entry of this.history.values()) {
      await this.saveReviewToDisk(entry);
    }
    
    this.history.clear();
    this.initialized = false;
    
    this.logger.info('History tracker shut down');
  }
}
