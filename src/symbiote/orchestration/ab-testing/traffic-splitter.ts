/**
 * Traffic Splitter for A/B Testing
 */

import * as crypto from 'crypto';
import {
  ExperimentConfig,
  ExperimentVariant,
  AllocationMethod,
  UserSegment
} from './experiment';
import { AITask } from '../interfaces';

export class TrafficSplitter {
  private assignmentCache: Map<string, string> = new Map();

  /**
   * Assign a variant to a task based on experiment configuration
   */
  assignVariant(
    experiment: ExperimentConfig,
    task: AITask,
    context?: {
      userId?: string;
      sessionId?: string;
      metadata?: Record<string, any>;
    }
  ): ExperimentVariant | null {
    // Check user segments first
    if (experiment.trafficAllocation.userSegments) {
      const segmentVariant = this.checkUserSegments(
        experiment.trafficAllocation.userSegments,
        experiment.variants,
        context
      );
      if (segmentVariant) {
        return segmentVariant;
      }
    }

    // Apply allocation method
    switch (experiment.trafficAllocation.method) {
      case AllocationMethod.Random:
        return this.randomAllocation(experiment.variants);
      
      case AllocationMethod.Deterministic:
        return this.deterministicAllocation(
          experiment.variants,
          task,
          experiment.trafficAllocation.seed
        );
      
      case AllocationMethod.UserBased:
        return this.userBasedAllocation(
          experiment.variants,
          context?.userId || 'anonymous',
          experiment.id
        );
      
      case AllocationMethod.SessionBased:
        return this.sessionBasedAllocation(
          experiment.variants,
          context?.sessionId || task.sessionId || 'default',
          experiment.id
        );
      
      case AllocationMethod.TaskBased:
        return this.taskBasedAllocation(
          experiment.variants,
          task.id,
          experiment.id
        );
      
      default:
        return this.randomAllocation(experiment.variants);
    }
  }

  /**
   * Check if user matches any segment criteria
   */
  private checkUserSegments(
    segments: UserSegment[],
    variants: ExperimentVariant[],
    context?: any
  ): ExperimentVariant | null {
    for (const segment of segments) {
      if (this.matchesSegmentCriteria(segment, context)) {
        return variants.find(v => v.id === segment.variantId) || null;
      }
    }
    return null;
  }

  /**
   * Check if context matches segment criteria
   */
  private matchesSegmentCriteria(
    segment: UserSegment,
    context?: any
  ): boolean {
    if (!context) return false;

    const criteria = segment.criteria;

    // Check user IDs
    if (criteria.userIds && context.userId) {
      if (!criteria.userIds.includes(context.userId)) {
        return false;
      }
    }

    // Check project IDs
    if (criteria.projectIds && context.projectId) {
      if (!criteria.projectIds.includes(context.projectId)) {
        return false;
      }
    }

    // Check tags
    if (criteria.tags && context.tags) {
      const contextTags = new Set(context.tags);
      const hasAllTags = criteria.tags.every(tag => contextTags.has(tag));
      if (!hasAllTags) {
        return false;
      }
    }

    // Check custom predicate
    if (criteria.customPredicate) {
      if (!criteria.customPredicate(context)) {
        return false;
      }
    }

    return true;
  }

  /**
   * Random allocation based on weights
   */
  private randomAllocation(variants: ExperimentVariant[]): ExperimentVariant {
    const random = Math.random() * 100;
    let cumulative = 0;

    for (const variant of variants) {
      cumulative += variant.weight;
      if (random < cumulative) {
        return variant;
      }
    }

    // Fallback to last variant
    return variants[variants.length - 1];
  }

  /**
   * Deterministic allocation based on hash
   */
  private deterministicAllocation(
    variants: ExperimentVariant[],
    task: AITask,
    seed?: string
  ): ExperimentVariant {
    const input = `${seed || 'default'}-${task.id}-${task.prompt}`;
    const hash = crypto.createHash('md5').update(input).digest('hex');
    const hashValue = parseInt(hash.substring(0, 8), 16);
    const normalized = (hashValue % 100) + 1;

    let cumulative = 0;
    for (const variant of variants) {
      cumulative += variant.weight;
      if (normalized <= cumulative) {
        return variant;
      }
    }

    return variants[variants.length - 1];
  }

  /**
   * User-based allocation (sticky per user)
   */
  private userBasedAllocation(
    variants: ExperimentVariant[],
    userId: string,
    experimentId: string
  ): ExperimentVariant {
    const cacheKey = `${experimentId}-user-${userId}`;
    
    // Check cache
    const cached = this.assignmentCache.get(cacheKey);
    if (cached) {
      const variant = variants.find(v => v.id === cached);
      if (variant) return variant;
    }

    // Generate deterministic assignment
    const hash = crypto.createHash('md5').update(cacheKey).digest('hex');
    const hashValue = parseInt(hash.substring(0, 8), 16);
    const normalized = (hashValue % 100) + 1;

    let cumulative = 0;
    for (const variant of variants) {
      cumulative += variant.weight;
      if (normalized <= cumulative) {
        this.assignmentCache.set(cacheKey, variant.id);
        return variant;
      }
    }

    const selected = variants[variants.length - 1];
    this.assignmentCache.set(cacheKey, selected.id);
    return selected;
  }

  /**
   * Session-based allocation (sticky per session)
   */
  private sessionBasedAllocation(
    variants: ExperimentVariant[],
    sessionId: string,
    experimentId: string
  ): ExperimentVariant {
    const cacheKey = `${experimentId}-session-${sessionId}`;
    
    // Check cache
    const cached = this.assignmentCache.get(cacheKey);
    if (cached) {
      const variant = variants.find(v => v.id === cached);
      if (variant) return variant;
    }

    // Generate deterministic assignment
    const hash = crypto.createHash('md5').update(cacheKey).digest('hex');
    const hashValue = parseInt(hash.substring(0, 8), 16);
    const normalized = (hashValue % 100) + 1;

    let cumulative = 0;
    for (const variant of variants) {
      cumulative += variant.weight;
      if (normalized <= cumulative) {
        this.assignmentCache.set(cacheKey, variant.id);
        return variant;
      }
    }

    const selected = variants[variants.length - 1];
    this.assignmentCache.set(cacheKey, selected.id);
    return selected;
  }

  /**
   * Task-based allocation (deterministic per task)
   */
  private taskBasedAllocation(
    variants: ExperimentVariant[],
    taskId: string,
    experimentId: string
  ): ExperimentVariant {
    const cacheKey = `${experimentId}-task-${taskId}`;
    
    // Check cache
    const cached = this.assignmentCache.get(cacheKey);
    if (cached) {
      const variant = variants.find(v => v.id === cached);
      if (variant) return variant;
    }

    // Generate deterministic assignment
    const hash = crypto.createHash('md5').update(cacheKey).digest('hex');
    const hashValue = parseInt(hash.substring(0, 8), 16);
    const normalized = (hashValue % 100) + 1;

    let cumulative = 0;
    for (const variant of variants) {
      cumulative += variant.weight;
      if (normalized <= cumulative) {
        this.assignmentCache.set(cacheKey, variant.id);
        return variant;
      }
    }

    const selected = variants[variants.length - 1];
    this.assignmentCache.set(cacheKey, selected.id);
    return selected;
  }

  /**
   * Clear assignment cache
   */
  clearCache(): void {
    this.assignmentCache.clear();
  }

  /**
   * Get cache size
   */
  getCacheSize(): number {
    return this.assignmentCache.size;
  }

  /**
   * Export cache for persistence
   */
  exportCache(): Record<string, string> {
    return Object.fromEntries(this.assignmentCache);
  }

  /**
   * Import cache from persistence
   */
  importCache(cache: Record<string, string>): void {
    this.assignmentCache = new Map(Object.entries(cache));
  }
}