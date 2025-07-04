/**
 * Code Review Repository
 */

import { BaseRepository } from './base-repository';

export interface CodeReviewRecord {
  id: string;
  project_id?: string;
  session_id?: string;
  review_type: string;
  files: string[];
  issues_found: number;
  suggestions_made: number;
  suggestions_accepted: number;
  review_data: any;
  started_at: Date;
  completed_at?: Date;
  duration_ms?: number;
  metadata: Record<string, any>;
}

export class CodeReviewRepository extends BaseRepository<CodeReviewRecord> {
  constructor() {
    super('code_reviews');
  }
  
  /**
   * Find reviews by project
   */
  async findByProject(projectId: string): Promise<CodeReviewRecord[]> {
    return this.findWhere({ project_id: projectId }, {
      orderBy: 'started_at DESC'
    });
  }
  
  /**
   * Update suggestion acceptance
   */
  async acceptSuggestion(reviewId: string): Promise<void> {
    const review = await this.findById(reviewId);
    if (!review) return;
    
    await this.update(reviewId, {
      suggestions_accepted: review.suggestions_accepted + 1
    });
  }
}