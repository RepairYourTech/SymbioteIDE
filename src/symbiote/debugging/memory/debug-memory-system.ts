/**
 * Debug Memory System - Manages debugging knowledge and patterns
 */

import { Logger } from '../../utils/logger';
import { ADKClient } from '../../adk/adk-client';
import { 
  DebugMemory,
  EpisodicMemory,
  SemanticMemory,
  WorkingMemory,
  DebugPattern,
  DebugSession,
  ErrorContext,
  DebugFix
} from '../types';
import { getRepositories } from '../../database';
import { v4 as uuidv4 } from 'uuid';

export interface MemorySearchQuery {
  errorType?: string;
  errorMessage?: string;
  file?: string;
  framework?: string;
  limit?: number;
}

export interface PatternLearningResult {
  newPatterns: number;
  updatedPatterns: number;
  confidence: number;
}

export class DebugMemorySystem {
  private logger = new Logger('DebugMemorySystem');
  private adkClient: ADKClient;
  private workingMemory: Map<string, WorkingMemory> = new Map();
  private patterns: Map<string, DebugPattern> = new Map();
  
  constructor() {
    this.adkClient = new ADKClient();
    this.initializePatterns();
  }
  
  /**
   * Initialize with common debugging patterns
   */
  private async initializePatterns(): Promise<void> {
    try {
      // Load patterns from database
      const db = getRepositories();
      const storedPatterns = await db.codeReviews.findWhere({ 
        review_type: 'debug_pattern' 
      });
      
      // Convert to DebugPattern format
      storedPatterns.forEach(p => {
        if (p.review_data?.pattern) {
          this.patterns.set(p.review_data.pattern.id, p.review_data.pattern);
        }
      });
      
      // Add default patterns if none exist
      if (this.patterns.size === 0) {
        this.addDefaultPatterns();
      }
      
      this.logger.info(`Loaded ${this.patterns.size} debug patterns`);
      
    } catch (error) {
      this.logger.error('Failed to load patterns', error);
      this.addDefaultPatterns();
    }
  }
  
  /**
   * Add default debugging patterns
   */
  private addDefaultPatterns(): void {
    const defaultPatterns: DebugPattern[] = [
      {
        id: 'null-ref-001',
        name: 'Null Reference Pattern',
        description: 'Object property access on null/undefined',
        errorSignature: 'Cannot read property .* of (null|undefined)',
        commonCauses: [
          'Uninitialized variables',
          'Failed async operations',
          'Missing data validation',
          'Incorrect optional chaining'
        ],
        solutions: [
          'Add null checks before property access',
          'Use optional chaining operator',
          'Initialize with default values',
          'Validate data at boundaries'
        ],
        frequency: 0,
        successRate: 0.85
      },
      {
        id: 'async-error-001',
        name: 'Unhandled Async Error',
        description: 'Promise rejection without catch handler',
        errorSignature: 'Unhandled promise rejection',
        commonCauses: [
          'Missing catch blocks',
          'Forgotten await keywords',
          'Parallel promise failures',
          'Error in promise chain'
        ],
        solutions: [
          'Add try-catch to async functions',
          'Use .catch() on promise chains',
          'Implement global error handlers',
          'Use Promise.allSettled for parallel ops'
        ],
        frequency: 0,
        successRate: 0.9
      },
      {
        id: 'type-error-001',
        name: 'Type Mismatch Pattern',
        description: 'Operations on wrong data types',
        errorSignature: '.* is not a function|.* is not iterable',
        commonCauses: [
          'API response format changes',
          'Missing type validation',
          'Implicit type coercion',
          'Wrong assumptions about data'
        ],
        solutions: [
          'Add runtime type checking',
          'Use TypeScript strict mode',
          'Validate external data',
          'Add type guards'
        ],
        frequency: 0,
        successRate: 0.8
      }
    ];
    
    defaultPatterns.forEach(p => this.patterns.set(p.id, p));
  }
  
  /**
   * Store debugging session in memory
   */
  async storeSession(session: DebugSession): Promise<void> {
    try {
      // Create episodic memory
      const episodicMemory: EpisodicMemory = {
        sessionId: session.id,
        timestamp: new Date(),
        error: session.error!,
        solution: session.fixes?.find(f => f.success),
        success: session.status === 'completed' && session.fixes?.some(f => f.success) || false,
        duration: session.endTime ? 
          session.endTime.getTime() - session.startTime.getTime() : 0,
        steps: [] // Would be populated from session history
      };
      
      // Store in database
      const db = getRepositories();
      await db.codeReviews.create({
        review_type: 'debug_session',
        files: [session.error?.file || 'unknown'],
        issues_found: 1,
        suggestions_made: session.fixes?.length || 0,
        suggestions_accepted: session.fixes?.filter(f => f.success).length || 0,
        review_data: {
          episodicMemory,
          session
        },
        started_at: session.startTime,
        completed_at: session.endTime,
        duration_ms: episodicMemory.duration
      });
      
      // Update patterns based on session
      await this.updatePatterns(session);
      
      // Store in ADK memory for agent learning
      await this.storeInADKMemory(episodicMemory);
      
      this.logger.info(`Stored debug session ${session.id}`);
      
    } catch (error) {
      this.logger.error('Failed to store session', error);
    }
  }
  
  /**
   * Search episodic memory for similar errors
   */
  async searchEpisodicMemory(
    query: MemorySearchQuery
  ): Promise<EpisodicMemory[]> {
    try {
      const db = getRepositories();
      
      // Build search criteria
      const where: any = { review_type: 'debug_session' };
      
      // Search in stored sessions
      const results = await db.codeReviews.findWhere(where, {
        limit: query.limit || 10,
        orderBy: 'started_at DESC'
      });
      
      // Filter by similarity
      const episodes = results
        .map(r => r.review_data?.episodicMemory as EpisodicMemory)
        .filter(e => e && this.matchesQuery(e, query));
      
      return episodes;
      
    } catch (error) {
      this.logger.error('Failed to search episodic memory', error);
      return [];
    }
  }
  
  /**
   * Get semantic knowledge about error types
   */
  async getSemanticKnowledge(concept: string): Promise<SemanticMemory | null> {
    try {
      // Query ADK memory for semantic knowledge
      const knowledge = await this.adkClient.queryAgentMemory({
        type: 'semantic',
        query: concept,
        limit: 1
      });
      
      if (knowledge && knowledge.length > 0) {
        return knowledge[0] as SemanticMemory;
      }
      
      // Fallback to local patterns
      const pattern = Array.from(this.patterns.values())
        .find(p => p.name.toLowerCase().includes(concept.toLowerCase()));
      
      if (pattern) {
        return {
          concept: pattern.name,
          description: pattern.description,
          examples: pattern.commonCauses,
          relatedErrors: [pattern.errorSignature],
          bestPractices: pattern.solutions
        };
      }
      
      return null;
      
    } catch (error) {
      this.logger.error('Failed to get semantic knowledge', error);
      return null;
    }
  }
  
  /**
   * Get or create working memory for session
   */
  getWorkingMemory(sessionId: string): WorkingMemory {
    if (!this.workingMemory.has(sessionId)) {
      this.workingMemory.set(sessionId, {
        currentError: {} as ErrorContext,
        investigatedFiles: [],
        testedHypotheses: [],
        ruledOutCauses: [],
        viableFixOptions: []
      });
    }
    
    return this.workingMemory.get(sessionId)!;
  }
  
  /**
   * Update working memory
   */
  updateWorkingMemory(
    sessionId: string,
    updates: Partial<WorkingMemory>
  ): void {
    const memory = this.getWorkingMemory(sessionId);
    Object.assign(memory, updates);
  }
  
  /**
   * Learn new patterns from sessions
   */
  async learnPatterns(sessions: DebugSession[]): Promise<PatternLearningResult> {
    let newPatterns = 0;
    let updatedPatterns = 0;
    
    // Group sessions by error similarity
    const errorGroups = this.groupSessionsByError(sessions);
    
    for (const [signature, groupSessions] of errorGroups.entries()) {
      const successfulSessions = groupSessions.filter(s => 
        s.status === 'completed' && s.fixes?.some(f => f.success)
      );
      
      if (successfulSessions.length >= 3) {
        // Enough data to create/update pattern
        const pattern = this.extractPattern(signature, successfulSessions);
        
        if (this.patterns.has(pattern.id)) {
          this.updatePattern(pattern.id, successfulSessions);
          updatedPatterns++;
        } else {
          this.patterns.set(pattern.id, pattern);
          newPatterns++;
        }
      }
    }
    
    // Store updated patterns
    await this.persistPatterns();
    
    const confidence = this.calculateLearningConfidence(
      newPatterns,
      updatedPatterns,
      sessions.length
    );
    
    return {
      newPatterns,
      updatedPatterns,
      confidence
    };
  }
  
  /**
   * Get pattern recommendations for error
   */
  getPatternRecommendations(error: ErrorContext): DebugPattern[] {
    const recommendations: DebugPattern[] = [];
    
    for (const pattern of this.patterns.values()) {
      const regex = new RegExp(pattern.errorSignature, 'i');
      if (regex.test(error.message)) {
        recommendations.push(pattern);
      }
    }
    
    // Sort by success rate and frequency
    recommendations.sort((a, b) => {
      const scoreA = a.successRate * 0.7 + (a.frequency / 100) * 0.3;
      const scoreB = b.successRate * 0.7 + (b.frequency / 100) * 0.3;
      return scoreB - scoreA;
    });
    
    return recommendations.slice(0, 3);
  }
  
  /**
   * Store in ADK memory system
   */
  private async storeInADKMemory(memory: EpisodicMemory): Promise<void> {
    try {
      await this.adkClient.updateAgentMemory('debug-controller', {
        type: 'episodic',
        memory: {
          id: memory.sessionId,
          timestamp: memory.timestamp,
          data: memory,
          tags: ['debug', 'error', memory.error.type],
          metadata: {
            success: memory.success,
            duration: memory.duration,
            errorType: memory.error.type
          }
        }
      });
    } catch (error) {
      this.logger.error('Failed to store in ADK memory', error);
    }
  }
  
  /**
   * Update patterns based on session
   */
  private async updatePatterns(session: DebugSession): Promise<void> {
    if (!session.error || !session.fixes) return;
    
    const matchingPatterns = this.getPatternRecommendations(session.error);
    
    matchingPatterns.forEach(pattern => {
      pattern.frequency++;
      
      if (session.fixes?.some(f => f.success)) {
        // Update success rate
        const totalAttempts = pattern.frequency;
        const currentSuccesses = Math.round(pattern.successRate * (totalAttempts - 1));
        pattern.successRate = (currentSuccesses + 1) / totalAttempts;
        
        // Add successful solution if new
        const successfulFix = session.fixes.find(f => f.success);
        if (successfulFix && !pattern.solutions.includes(successfulFix.description)) {
          pattern.solutions.push(successfulFix.description);
        }
      }
    });
  }
  
  /**
   * Group sessions by error signature
   */
  private groupSessionsByError(
    sessions: DebugSession[]
  ): Map<string, DebugSession[]> {
    const groups = new Map<string, DebugSession[]>();
    
    sessions.forEach(session => {
      if (!session.error) return;
      
      // Create signature from error
      const signature = this.createErrorSignature(session.error);
      
      if (!groups.has(signature)) {
        groups.set(signature, []);
      }
      
      groups.get(signature)!.push(session);
    });
    
    return groups;
  }
  
  /**
   * Create error signature for grouping
   */
  private createErrorSignature(error: ErrorContext): string {
    // Remove specific values to create general pattern
    return error.message
      .replace(/'.+?'/g, "'*'")
      .replace(/".+?"/g, '"*"')
      .replace(/\d+/g, 'N')
      .replace(/\s+/g, ' ')
      .trim();
  }
  
  /**
   * Extract pattern from successful sessions
   */
  private extractPattern(
    signature: string,
    sessions: DebugSession[]
  ): DebugPattern {
    const id = `learned-${Date.now()}`;
    
    // Extract common causes
    const causes = new Set<string>();
    sessions.forEach(s => {
      if (s.analysis?.rootCause) {
        causes.add(s.analysis.rootCause);
      }
    });
    
    // Extract successful solutions
    const solutions = new Set<string>();
    sessions.forEach(s => {
      const successFix = s.fixes?.find(f => f.success);
      if (successFix) {
        solutions.add(successFix.description);
      }
    });
    
    return {
      id,
      name: `Learned Pattern: ${signature.substring(0, 50)}`,
      description: `Pattern learned from ${sessions.length} debugging sessions`,
      errorSignature: signature,
      commonCauses: Array.from(causes).slice(0, 5),
      solutions: Array.from(solutions).slice(0, 5),
      frequency: sessions.length,
      successRate: sessions.filter(s => s.fixes?.some(f => f.success)).length / sessions.length
    };
  }
  
  /**
   * Update existing pattern
   */
  private updatePattern(patternId: string, sessions: DebugSession[]): void {
    const pattern = this.patterns.get(patternId);
    if (!pattern) return;
    
    // Update frequency
    pattern.frequency += sessions.length;
    
    // Update success rate
    const newSuccesses = sessions.filter(s => s.fixes?.some(f => f.success)).length;
    const totalAttempts = pattern.frequency;
    const oldSuccesses = Math.round(pattern.successRate * (totalAttempts - sessions.length));
    pattern.successRate = (oldSuccesses + newSuccesses) / totalAttempts;
    
    // Add new solutions
    sessions.forEach(s => {
      const successFix = s.fixes?.find(f => f.success);
      if (successFix && !pattern.solutions.includes(successFix.description)) {
        pattern.solutions.push(successFix.description);
      }
    });
  }
  
  /**
   * Persist patterns to database
   */
  private async persistPatterns(): Promise<void> {
    try {
      const db = getRepositories();
      
      for (const pattern of this.patterns.values()) {
        await db.codeReviews.create({
          review_type: 'debug_pattern',
          files: ['pattern'],
          issues_found: 0,
          suggestions_made: pattern.solutions.length,
          suggestions_accepted: 0,
          review_data: { pattern },
          started_at: new Date(),
          completed_at: new Date()
        });
      }
      
    } catch (error) {
      this.logger.error('Failed to persist patterns', error);
    }
  }
  
  /**
   * Check if episode matches query
   */
  private matchesQuery(
    episode: EpisodicMemory,
    query: MemorySearchQuery
  ): boolean {
    if (query.errorType && episode.error.type !== query.errorType) {
      return false;
    }
    
    if (query.errorMessage) {
      const similarity = this.calculateSimilarity(
        episode.error.message.toLowerCase(),
        query.errorMessage.toLowerCase()
      );
      if (similarity < 0.6) return false;
    }
    
    if (query.file && episode.error.file !== query.file) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Calculate string similarity
   */
  private calculateSimilarity(str1: string, str2: string): number {
    const words1 = str1.split(/\s+/);
    const words2 = str2.split(/\s+/);
    const commonWords = words1.filter(w => words2.includes(w));
    
    return commonWords.length / Math.max(words1.length, words2.length);
  }
  
  /**
   * Calculate learning confidence
   */
  private calculateLearningConfidence(
    newPatterns: number,
    updatedPatterns: number,
    totalSessions: number
  ): number {
    if (totalSessions === 0) return 0;
    
    const learningRate = (newPatterns + updatedPatterns) / totalSessions;
    const patternDiversity = this.patterns.size / 10; // Normalize by expected patterns
    
    return Math.min(learningRate * 0.6 + patternDiversity * 0.4, 0.95);
  }
  
  /**
   * Clear working memory for session
   */
  clearWorkingMemory(sessionId: string): void {
    this.workingMemory.delete(sessionId);
  }
  
  /**
   * Get memory statistics
   */
  async getMemoryStats(): Promise<{
    episodicCount: number;
    patternCount: number;
    successRate: number;
    mostCommonErrors: Array<{ type: string; count: number }>;
  }> {
    const db = getRepositories();
    
    const sessions = await db.codeReviews.findWhere({ 
      review_type: 'debug_session' 
    });
    
    const errorTypes: Record<string, number> = {};
    let successfulSessions = 0;
    
    sessions.forEach(s => {
      const episode = s.review_data?.episodicMemory as EpisodicMemory;
      if (episode) {
        errorTypes[episode.error.type] = (errorTypes[episode.error.type] || 0) + 1;
        if (episode.success) successfulSessions++;
      }
    });
    
    const mostCommonErrors = Object.entries(errorTypes)
      .map(([type, count]) => ({ type, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5);
    
    return {
      episodicCount: sessions.length,
      patternCount: this.patterns.size,
      successRate: sessions.length > 0 ? successfulSessions / sessions.length : 0,
      mostCommonErrors
    };
  }
}