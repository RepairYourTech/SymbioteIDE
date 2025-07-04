/**
 * Conflict Resolver
 * 
 * Handles merge conflicts and overlapping work between agents
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import * as fs from 'fs/promises';
import * as path from 'path';
import { diffLines, createPatch } from 'diff';
import {
  ConflictResolution,
  ConflictType,
  ResolutionStrategy,
  HivemindTask
} from './types';

export interface ConflictDetection {
  hasConflict: boolean;
  type?: ConflictType;
  affectedFiles: string[];
  affectedTasks: string[];
  conflictDetails?: any;
}

export class ConflictResolver extends EventEmitter {
  private logger = new Logger('ConflictResolver');
  private fileVersions = new Map<string, Map<string, string>>(); // file -> taskId -> content
  private fileLocks = new Map<string, string>(); // file -> taskId
  private resolutionHistory: ConflictResolution[] = [];
  
  constructor() {
    super();
  }
  
  /**
   * Lock files for a task
   */
  async lockFiles(taskId: string, files: string[]): Promise<boolean> {
    const lockedFiles: string[] = [];
    
    try {
      for (const file of files) {
        const currentLock = this.fileLocks.get(file);
        
        if (currentLock && currentLock !== taskId) {
          // File is locked by another task
          this.logger.warn(`File ${file} is locked by task ${currentLock}`);
          
          // Unlock any files we already locked
          lockedFiles.forEach(f => this.fileLocks.delete(f));
          
          return false;
        }
        
        this.fileLocks.set(file, taskId);
        lockedFiles.push(file);
      }
      
      this.emit('files-locked', { taskId, files });
      return true;
      
    } catch (error) {
      this.logger.error('Failed to lock files', error);
      
      // Cleanup on error
      lockedFiles.forEach(f => this.fileLocks.delete(f));
      
      return false;
    }
  }
  
  /**
   * Unlock files for a task
   */
  unlockFiles(taskId: string, files: string[]): void {
    files.forEach(file => {
      if (this.fileLocks.get(file) === taskId) {
        this.fileLocks.delete(file);
      }
    });
    
    this.emit('files-unlocked', { taskId, files });
  }
  
  /**
   * Track file version for a task
   */
  async trackFileVersion(
    taskId: string,
    filePath: string,
    content: string
  ): Promise<void> {
    if (!this.fileVersions.has(filePath)) {
      this.fileVersions.set(filePath, new Map());
    }
    
    this.fileVersions.get(filePath)!.set(taskId, content);
  }
  
  /**
   * Detect conflicts between tasks
   */
  async detectConflicts(
    taskId: string,
    files: string[]
  ): Promise<ConflictDetection> {
    const conflicts: ConflictDetection = {
      hasConflict: false,
      affectedFiles: [],
      affectedTasks: []
    };
    
    for (const file of files) {
      // Check if file is locked by another task
      const lockingTask = this.fileLocks.get(file);
      if (lockingTask && lockingTask !== taskId) {
        conflicts.hasConflict = true;
        conflicts.type = ConflictType.ResourceLock;
        conflicts.affectedFiles.push(file);
        conflicts.affectedTasks.push(lockingTask);
        continue;
      }
      
      // Check for file edit conflicts
      const versions = this.fileVersions.get(file);
      if (versions && versions.size > 1) {
        // Multiple tasks have edited this file
        const otherTasks = Array.from(versions.keys()).filter(t => t !== taskId);
        
        if (otherTasks.length > 0) {
          conflicts.hasConflict = true;
          conflicts.type = ConflictType.FileEdit;
          conflicts.affectedFiles.push(file);
          conflicts.affectedTasks.push(...otherTasks);
          
          // Get conflict details
          if (!conflicts.conflictDetails) {
            conflicts.conflictDetails = {};
          }
          
          conflicts.conflictDetails[file] = await this.analyzeFileConflict(
            file,
            taskId,
            otherTasks[0]
          );
        }
      }
    }
    
    if (conflicts.hasConflict) {
      this.emit('conflict-detected', conflicts);
    }
    
    return conflicts;
  }
  
  /**
   * Analyze file conflict between two tasks
   */
  private async analyzeFileConflict(
    filePath: string,
    task1: string,
    task2: string
  ): Promise<any> {
    const versions = this.fileVersions.get(filePath);
    if (!versions) return null;
    
    const content1 = versions.get(task1);
    const content2 = versions.get(task2);
    
    if (!content1 || !content2) return null;
    
    // Get original content
    let originalContent = '';
    try {
      originalContent = await fs.readFile(filePath, 'utf8');
    } catch {}
    
    // Create diffs
    const diff1 = diffLines(originalContent, content1);
    const diff2 = diffLines(originalContent, content2);
    
    // Find overlapping changes
    const overlappingLines: number[] = [];
    let lineNumber = 0;
    
    diff1.forEach((part1, index1) => {
      if (part1.added || part1.removed) {
        diff2.forEach((part2, index2) => {
          if ((part2.added || part2.removed) && 
              Math.abs(index1 - index2) < 3) {
            overlappingLines.push(lineNumber);
          }
        });
      }
      lineNumber += part1.count || 0;
    });
    
    return {
      hasOverlap: overlappingLines.length > 0,
      overlappingLines,
      diff1: createPatch(filePath, originalContent, content1),
      diff2: createPatch(filePath, originalContent, content2),
      canAutoMerge: overlappingLines.length === 0
    };
  }
  
  /**
   * Resolve conflicts
   */
  async resolveConflicts(
    conflicts: ConflictDetection,
    strategy: ResolutionStrategy = ResolutionStrategy.Merge
  ): Promise<ConflictResolution> {
    const resolution: ConflictResolution = {
      conflictId: `conflict_${Date.now()}`,
      type: conflicts.type!,
      affectedFiles: conflicts.affectedFiles,
      affectedTasks: conflicts.affectedTasks,
      resolution: strategy
    };
    
    try {
      switch (conflicts.type) {
        case ConflictType.FileEdit:
          await this.resolveFileEditConflict(conflicts, strategy, resolution);
          break;
          
        case ConflictType.ResourceLock:
          await this.resolveResourceLockConflict(conflicts, strategy, resolution);
          break;
          
        case ConflictType.DependencyConflict:
          await this.resolveDependencyConflict(conflicts, strategy, resolution);
          break;
          
        case ConflictType.LogicalConflict:
          await this.resolveLogicalConflict(conflicts, strategy, resolution);
          break;
      }
      
      resolution.resolvedAt = new Date();
      this.resolutionHistory.push(resolution);
      
      this.emit('conflict-resolved', resolution);
      
      return resolution;
      
    } catch (error) {
      this.logger.error('Failed to resolve conflict', error);
      throw error;
    }
  }
  
  /**
   * Resolve file edit conflict
   */
  private async resolveFileEditConflict(
    conflicts: ConflictDetection,
    strategy: ResolutionStrategy,
    resolution: ConflictResolution
  ): Promise<void> {
    for (const file of conflicts.affectedFiles) {
      const conflictDetail = conflicts.conflictDetails?.[file];
      
      if (!conflictDetail) continue;
      
      switch (strategy) {
        case ResolutionStrategy.Merge:
          if (conflictDetail.canAutoMerge) {
            // Auto-merge non-conflicting changes
            await this.autoMergeFile(file, conflicts.affectedTasks);
            resolution.resolvedBy = 'auto-merge';
          } else {
            // Manual merge required
            await this.createMergeConflictFile(file, conflicts.affectedTasks);
            resolution.resolvedBy = 'manual-merge-required';
          }
          break;
          
        case ResolutionStrategy.Sequential:
          // Apply changes in sequence
          await this.applySequentialChanges(file, conflicts.affectedTasks);
          resolution.resolvedBy = 'sequential-application';
          break;
          
        case ResolutionStrategy.Retry:
          // Mark for retry
          this.emit('retry-required', {
            file,
            tasks: conflicts.affectedTasks
          });
          resolution.resolvedBy = 'retry-scheduled';
          break;
          
        case ResolutionStrategy.Manual:
          // Create conflict markers for manual resolution
          await this.createMergeConflictFile(file, conflicts.affectedTasks);
          resolution.resolvedBy = 'manual-intervention-required';
          break;
          
        case ResolutionStrategy.Abort:
          // Revert all changes
          await this.revertFileChanges(file);
          resolution.resolvedBy = 'changes-aborted';
          break;
      }
    }
  }
  
  /**
   * Auto-merge non-conflicting changes
   */
  private async autoMergeFile(
    filePath: string,
    taskIds: string[]
  ): Promise<void> {
    const versions = this.fileVersions.get(filePath);
    if (!versions) return;
    
    // Get original content
    let originalContent = '';
    try {
      originalContent = await fs.readFile(filePath, 'utf8');
    } catch {}
    
    // Apply all changes
    let mergedContent = originalContent;
    
    for (const taskId of taskIds) {
      const taskContent = versions.get(taskId);
      if (taskContent) {
        // Simple merge - in reality would use more sophisticated merging
        const diff = diffLines(originalContent, taskContent);
        
        let result = '';
        diff.forEach(part => {
          if (part.added) {
            result += part.value;
          } else if (!part.removed) {
            result += part.value;
          }
        });
        
        mergedContent = result;
      }
    }
    
    // Write merged content
    await fs.writeFile(filePath, mergedContent, 'utf8');
    
    this.logger.info(`Auto-merged ${filePath}`);
  }
  
  /**
   * Create merge conflict file
   */
  private async createMergeConflictFile(
    filePath: string,
    taskIds: string[]
  ): Promise<void> {
    const versions = this.fileVersions.get(filePath);
    if (!versions) return;
    
    // Get original content
    let originalContent = '';
    try {
      originalContent = await fs.readFile(filePath, 'utf8');
    } catch {}
    
    // Create conflict markers
    let conflictContent = originalContent;
    
    for (const taskId of taskIds) {
      const taskContent = versions.get(taskId);
      if (taskContent && taskContent !== originalContent) {
        conflictContent = `<<<<<<< Task: ${taskId}\n${taskContent}\n=======\n${originalContent}\n>>>>>>> Original\n`;
      }
    }
    
    // Write conflict file
    const conflictPath = `${filePath}.conflict`;
    await fs.writeFile(conflictPath, conflictContent, 'utf8');
    
    this.logger.warn(`Created conflict file: ${conflictPath}`);
  }
  
  /**
   * Apply changes sequentially
   */
  private async applySequentialChanges(
    filePath: string,
    taskIds: string[]
  ): Promise<void> {
    const versions = this.fileVersions.get(filePath);
    if (!versions) return;
    
    // Apply changes in order
    for (const taskId of taskIds) {
      const content = versions.get(taskId);
      if (content) {
        await fs.writeFile(filePath, content, 'utf8');
        this.logger.info(`Applied changes from task ${taskId} to ${filePath}`);
        
        // Small delay between applications
        await new Promise(resolve => setTimeout(resolve, 100));
      }
    }
  }
  
  /**
   * Revert file changes
   */
  private async revertFileChanges(filePath: string): Promise<void> {
    try {
      // Try to restore from git
      const { exec } = require('child_process');
      const util = require('util');
      const execPromise = util.promisify(exec);
      
      await execPromise(`git checkout -- ${filePath}`);
      
      this.logger.info(`Reverted ${filePath} to original state`);
    } catch (error) {
      this.logger.error(`Failed to revert ${filePath}`, error);
    }
  }
  
  /**
   * Resolve resource lock conflict
   */
  private async resolveResourceLockConflict(
    conflicts: ConflictDetection,
    strategy: ResolutionStrategy,
    resolution: ConflictResolution
  ): Promise<void> {
    switch (strategy) {
      case ResolutionStrategy.Sequential:
        // Queue the task for later execution
        this.emit('queue-task', {
          files: conflicts.affectedFiles,
          blockedBy: conflicts.affectedTasks
        });
        resolution.resolvedBy = 'queued-for-sequential-execution';
        break;
        
      case ResolutionStrategy.Retry:
        // Schedule retry with backoff
        setTimeout(() => {
          this.emit('retry-lock', {
            files: conflicts.affectedFiles
          });
        }, 5000);
        resolution.resolvedBy = 'retry-scheduled';
        break;
        
      default:
        // Force unlock (risky)
        conflicts.affectedFiles.forEach(file => {
          this.fileLocks.delete(file);
        });
        resolution.resolvedBy = 'force-unlocked';
        break;
    }
  }
  
  /**
   * Resolve dependency conflict
   */
  private async resolveDependencyConflict(
    conflicts: ConflictDetection,
    strategy: ResolutionStrategy,
    resolution: ConflictResolution
  ): Promise<void> {
    // Would implement dependency resolution logic
    resolution.resolvedBy = 'dependency-reordered';
  }
  
  /**
   * Resolve logical conflict
   */
  private async resolveLogicalConflict(
    conflicts: ConflictDetection,
    strategy: ResolutionStrategy,
    resolution: ConflictResolution
  ): Promise<void> {
    // Would implement logical conflict resolution
    resolution.resolvedBy = 'logical-resolution-applied';
  }
  
  /**
   * Get resolution history
   */
  getResolutionHistory(): ConflictResolution[] {
    return [...this.resolutionHistory];
  }
  
  /**
   * Clear file versions for completed tasks
   */
  clearTaskVersions(taskId: string): void {
    this.fileVersions.forEach(versions => {
      versions.delete(taskId);
    });
    
    // Remove empty version maps
    const emptyFiles: string[] = [];
    this.fileVersions.forEach((versions, file) => {
      if (versions.size === 0) {
        emptyFiles.push(file);
      }
    });
    
    emptyFiles.forEach(file => this.fileVersions.delete(file));
  }
}
