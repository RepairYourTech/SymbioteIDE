/**
 * Result Aggregator
 * 
 * Aggregates and synthesizes results from multiple agents
 */

import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import {
  HivemindTask,
  TaskResult,
  AggregatedResult,
  ConflictType
} from './types';

export class ResultAggregator extends EventEmitter {
  private logger = new Logger('ResultAggregator');
  
  constructor() {
    super();
  }
  
  /**
   * Aggregate results from multiple agents
   */
  async aggregateResults(
    originalTask: HivemindTask,
    results: TaskResult[]
  ): Promise<AggregatedResult> {
    this.logger.info(`Aggregating ${results.length} results for task ${originalTask.id}`);
    
    const aggregated: AggregatedResult = {
      taskId: originalTask.id,
      success: false,
      aggregatedOutput: {},
      filesModified: [],
      filesCreated: [],
      testsAdded: 0,
      issuesFound: [],
      performanceGains: [],
      conflictsResolved: 0
    };
    
    try {
      // Check overall success
      aggregated.success = this.determineOverallSuccess(results);
      
      // Aggregate file changes
      const fileChanges = this.aggregateFileChanges(results);
      aggregated.filesModified = fileChanges.modified;
      aggregated.filesCreated = fileChanges.created;
      
      // Aggregate test counts
      aggregated.testsAdded = results.reduce((sum, r) => sum + r.testsAdded, 0);
      
      // Aggregate issues
      aggregated.issuesFound = this.aggregateIssues(results);
      
      // Aggregate outputs
      aggregated.aggregatedOutput = this.aggregateOutputs(results);
      
      // Extract performance gains
      aggregated.performanceGains = this.extractPerformanceGains(results);
      
      // Count resolved conflicts
      aggregated.conflictsResolved = this.countResolvedConflicts(results);
      
      // Generate summary
      aggregated.summary = this.generateSummary(originalTask, results, aggregated);
      
      this.emit('aggregation-complete', aggregated);
      
      return aggregated;
      
    } catch (error) {
      this.logger.error('Failed to aggregate results', error);
      throw error;
    }
  }
  
  /**
   * Determine overall success
   */
  private determineOverallSuccess(results: TaskResult[]): boolean {
    if (results.length === 0) return false;
    
    // Calculate success rate
    const successCount = results.filter(r => r.success).length;
    const successRate = successCount / results.length;
    
    // Consider task successful if > 80% of subtasks succeeded
    return successRate > 0.8;
  }
  
  /**
   * Aggregate file changes
   */
  private aggregateFileChanges(results: TaskResult[]): {
    modified: string[];
    created: string[];
  } {
    const modifiedSet = new Set<string>();
    const createdSet = new Set<string>();
    
    results.forEach(result => {
      result.filesModified.forEach(file => modifiedSet.add(file));
      result.filesCreated.forEach(file => createdSet.add(file));
    });
    
    // If a file was created by one agent and modified by another,
    // it should only appear in created
    modifiedSet.forEach(file => {
      if (createdSet.has(file)) {
        modifiedSet.delete(file);
      }
    });
    
    return {
      modified: Array.from(modifiedSet).sort(),
      created: Array.from(createdSet).sort()
    };
  }
  
  /**
   * Aggregate issues
   */
  private aggregateIssues(results: TaskResult[]): string[] {
    const issueMap = new Map<string, number>();
    
    // Count occurrences of each issue
    results.forEach(result => {
      result.issuesFound.forEach(issue => {
        const count = issueMap.get(issue) || 0;
        issueMap.set(issue, count + 1);
      });
    });
    
    // Sort by frequency and return unique issues
    return Array.from(issueMap.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([issue]) => issue);
  }
  
  /**
   * Aggregate outputs
   */
  private aggregateOutputs(results: TaskResult[]): any {
    const aggregated: any = {
      byAgent: {},
      combined: {},
      statistics: {
        totalDuration: 0,
        averageDuration: 0,
        totalTokensUsed: 0,
        totalCost: 0
      }
    };
    
    // Collect outputs by agent
    results.forEach(result => {
      const agentId = this.extractAgentId(result);
      aggregated.byAgent[agentId] = result.output;
      
      // Update statistics
      aggregated.statistics.totalDuration += result.performanceMetrics.duration;
      aggregated.statistics.totalTokensUsed += result.performanceMetrics.tokensUsed;
      aggregated.statistics.totalCost += result.performanceMetrics.cost;
    });
    
    // Calculate averages
    if (results.length > 0) {
      aggregated.statistics.averageDuration = 
        aggregated.statistics.totalDuration / results.length;
    }
    
    // Merge common output fields
    results.forEach(result => {
      Object.entries(result.output).forEach(([key, value]) => {
        if (key === 'error') return; // Skip errors
        
        if (!aggregated.combined[key]) {
          aggregated.combined[key] = [];
        }
        
        if (Array.isArray(value)) {
          aggregated.combined[key].push(...value);
        } else {
          aggregated.combined[key].push(value);
        }
      });
    });
    
    // Deduplicate arrays in combined output
    Object.keys(aggregated.combined).forEach(key => {
      if (Array.isArray(aggregated.combined[key])) {
        aggregated.combined[key] = [...new Set(aggregated.combined[key])];
      }
    });
    
    return aggregated;
  }
  
  /**
   * Extract agent ID from result
   */
  private extractAgentId(result: TaskResult): string {
    // This would be enhanced to properly track agent assignments
    return result.taskId.split('_')[0] || 'unknown';
  }
  
  /**
   * Extract performance gains
   */
  private extractPerformanceGains(results: TaskResult[]): any[] {
    const gains: any[] = [];
    
    results.forEach(result => {
      if (result.output.performanceGains) {
        gains.push(...result.output.performanceGains);
      }
      
      // Also check for improvements in output
      if (result.output.improvements) {
        result.output.improvements.forEach((improvement: any) => {
          if (improvement.metric) {
            gains.push({
              description: improvement.description,
              improvement: `${improvement.metric}${improvement.type}`,
              agent: this.extractAgentId(result)
            });
          }
        });
      }
    });
    
    return gains;
  }
  
  /**
   * Count resolved conflicts
   */
  private countResolvedConflicts(results: TaskResult[]): number {
    let count = 0;
    
    results.forEach(result => {
      if (result.output.conflictsResolved) {
        count += result.output.conflictsResolved;
      }
    });
    
    return count;
  }
  
  /**
   * Generate summary
   */
  private generateSummary(
    originalTask: HivemindTask,
    results: TaskResult[],
    aggregated: AggregatedResult
  ): string {
    const successCount = results.filter(r => r.success).length;
    const failureCount = results.length - successCount;
    
    let summary = `Completed task "${originalTask.title}" with ${results.length} subtasks. `;
    summary += `Success: ${successCount}, Failed: ${failureCount}. `;
    
    if (aggregated.filesCreated.length > 0) {
      summary += `Created ${aggregated.filesCreated.length} files. `;
    }
    
    if (aggregated.filesModified.length > 0) {
      summary += `Modified ${aggregated.filesModified.length} files. `;
    }
    
    if (aggregated.testsAdded > 0) {
      summary += `Added ${aggregated.testsAdded} tests. `;
    }
    
    if (aggregated.performanceGains.length > 0) {
      summary += `Achieved ${aggregated.performanceGains.length} performance improvements. `;
    }
    
    if (aggregated.conflictsResolved > 0) {
      summary += `Resolved ${aggregated.conflictsResolved} conflicts. `;
    }
    
    if (aggregated.issuesFound.length > 0) {
      summary += `Found ${aggregated.issuesFound.length} issues that need attention.`;
    }
    
    return summary;
  }
  
  /**
   * Merge results for re-aggregation
   */
  async mergeResults(
    existingResult: AggregatedResult,
    newResults: TaskResult[]
  ): Promise<AggregatedResult> {
    // Convert existing result back to task results format
    const existingAsResults: TaskResult[] = [{
      taskId: existingResult.taskId,
      success: existingResult.success,
      output: existingResult.aggregatedOutput,
      filesModified: existingResult.filesModified,
      filesCreated: existingResult.filesCreated,
      testsAdded: existingResult.testsAdded,
      issuesFound: existingResult.issuesFound,
      performanceMetrics: {
        duration: 0,
        tokensUsed: 0,
        cost: 0
      }
    }];
    
    // Combine with new results
    const allResults = [...existingAsResults, ...newResults];
    
    // Re-aggregate
    const task: HivemindTask = {
      id: existingResult.taskId,
      type: 'merged',
      title: 'Merged Task',
      description: 'Merged results from multiple executions',
      requiredSpecialties: [],
      priority: 'medium',
      status: 'completed',
      dependencies: [],
      assignedAgents: [],
      estimatedComplexity: 5,
      context: {},
      createdAt: new Date()
    };
    
    return this.aggregateResults(task, allResults);
  }
  
  /**
   * Generate detailed report
   */
  generateDetailedReport(
    task: HivemindTask,
    results: TaskResult[],
    aggregated: AggregatedResult
  ): string {
    let report = `# Hivemind Task Execution Report\n\n`;
    report += `## Task Information\n`;
    report += `- **ID**: ${task.id}\n`;
    report += `- **Title**: ${task.title}\n`;
    report += `- **Type**: ${task.type}\n`;
    report += `- **Priority**: ${task.priority}\n`;
    report += `- **Complexity**: ${task.estimatedComplexity}/10\n`;
    report += `- **Required Specialties**: ${task.requiredSpecialties.join(', ')}\n\n`;
    
    report += `## Execution Summary\n`;
    report += `- **Overall Success**: ${aggregated.success ? 'Yes' : 'No'}\n`;
    report += `- **Subtasks Executed**: ${results.length}\n`;
    report += `- **Success Rate**: ${(results.filter(r => r.success).length / results.length * 100).toFixed(1)}%\n`;
    report += `- **Total Duration**: ${aggregated.aggregatedOutput.statistics?.totalDuration || 0}ms\n\n`;
    
    report += `## File Changes\n`;
    if (aggregated.filesCreated.length > 0) {
      report += `### Files Created (${aggregated.filesCreated.length})\n`;
      aggregated.filesCreated.forEach(file => {
        report += `- ${file}\n`;
      });
      report += `\n`;
    }
    
    if (aggregated.filesModified.length > 0) {
      report += `### Files Modified (${aggregated.filesModified.length})\n`;
      aggregated.filesModified.forEach(file => {
        report += `- ${file}\n`;
      });
      report += `\n`;
    }
    
    if (aggregated.testsAdded > 0) {
      report += `### Tests Added\n`;
      report += `- **Total**: ${aggregated.testsAdded}\n\n`;
    }
    
    if (aggregated.performanceGains.length > 0) {
      report += `## Performance Improvements\n`;
      aggregated.performanceGains.forEach(gain => {
        report += `- ${gain.description}: **${gain.improvement}**\n`;
      });
      report += `\n`;
    }
    
    if (aggregated.issuesFound.length > 0) {
      report += `## Issues Found\n`;
      aggregated.issuesFound.forEach(issue => {
        report += `- ${issue}\n`;
      });
      report += `\n`;
    }
    
    report += `## Individual Agent Results\n`;
    results.forEach(result => {
      const agentId = this.extractAgentId(result);
      report += `### ${agentId}\n`;
      report += `- **Success**: ${result.success ? 'Yes' : 'No'}\n`;
      report += `- **Duration**: ${result.performanceMetrics.duration}ms\n`;
      
      if (result.output.summary) {
        report += `- **Summary**: ${result.output.summary}\n`;
      }
      
      report += `\n`;
    });
    
    return report;
  }
  
  /**
   * Export results to JSON
   */
  exportToJSON(
    task: HivemindTask,
    results: TaskResult[],
    aggregated: AggregatedResult
  ): string {
    const exportData = {
      task: {
        id: task.id,
        title: task.title,
        type: task.type,
        priority: task.priority,
        estimatedComplexity: task.estimatedComplexity,
        requiredSpecialties: task.requiredSpecialties
      },
      execution: {
        success: aggregated.success,
        summary: aggregated.summary,
        filesCreated: aggregated.filesCreated,
        filesModified: aggregated.filesModified,
        testsAdded: aggregated.testsAdded,
        performanceGains: aggregated.performanceGains,
        issuesFound: aggregated.issuesFound,
        conflictsResolved: aggregated.conflictsResolved
      },
      statistics: aggregated.aggregatedOutput.statistics,
      individualResults: results.map(r => ({
        taskId: r.taskId,
        success: r.success,
        duration: r.performanceMetrics.duration,
        filesModified: r.filesModified.length,
        filesCreated: r.filesCreated.length,
        testsAdded: r.testsAdded,
        issues: r.issuesFound.length
      }))
    };
    
    return JSON.stringify(exportData, null, 2);
  }
}