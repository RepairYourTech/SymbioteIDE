/**
 * Hivemind API Implementation for VS Code
 * Enhanced with Google ADK integration
 */

import * as vscode from 'vscode';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { MemoryManager } from '../memory/mem0/memory-manager';
import { UnifiedContextAPI } from '../memory/unified-context-api';
import { RedisQueueManager } from '../queue/redis-queue-manager';
import {
  HivemindTask,
  TaskPriority,
  AgentSpecialty,
  TaskStatus
} from '../hivemind/types';
import { ADKHivemindController, ADKHivemindControllerConfig } from '../hivemind/adk/hivemind-adk-controller';
import { Logger } from '../utils/logger';

export interface HivemindAPI {
  /**
   * Initialize the Hivemind system
   */
  initialize(orchestrator: OrchestrationEngine, memoryManager?: MemoryManager | UnifiedContextAPI, queueManager?: RedisQueueManager): Promise<void>;
  
  /**
   * Create a new Hivemind session (deprecated - ADK manages sessions)
   */
  createSession(config: ADKHivemindControllerConfig): Promise<string>;
  
  /**
   * Execute a task with the Hivemind
   */
  executeTask(task: HivemindTaskRequest): Promise<HivemindTaskResult>;
  
  /**
   * Get controller metrics
   */
  getMetrics(): any;
  
  /**
   * List available agents
   */
  listAgents(): Array<{ id: string; name: string; type: string }>;
  
  /**
   * Get agent statistics
   */
  getAgentStats(): Map<string, any>;
  
  /**
   * Enable/disable ADK patterns
   */
  configurePatterns(patterns: string[]): void;
  
  /**
   * Shutdown the Hivemind
   */
  shutdown(): Promise<void>;
}

export interface HivemindTaskRequest {
  type: string;
  title: string;
  description: string;
  priority?: TaskPriority;
  requiredSpecialties?: AgentSpecialty[];
  estimatedComplexity?: number;
  files?: string[];
  codeContext?: string;
}

export interface HivemindTaskResult {
  success: boolean;
  summary: string;
  filesModified: string[];
  filesCreated: string[];
  testsAdded: number;
  issues: string[];
  duration: number;
  report?: string;
}

export interface HivemindSessionStatus {
  id: string;
  status: string;
  taskCount: number;
  completedTasks: number;
  failedTasks: number;
  duration: number;
}

export class HivemindAPIImpl implements HivemindAPI {
  private logger = new Logger('HivemindAPI');
  private adkController?: ADKHivemindController;
  private outputChannel: vscode.OutputChannel;
  private orchestrator?: OrchestrationEngine;
  private memoryManager?: MemoryManager | UnifiedContextAPI;
  private queueManager?: RedisQueueManager;
  
  constructor() {
    this.outputChannel = vscode.window.createOutputChannel('Hivemind ADK');
  }
  
  async initialize(
    orchestrator: OrchestrationEngine,
    memoryManager?: MemoryManager | UnifiedContextAPI,
    queueManager?: RedisQueueManager
  ): Promise<void> {
    try {
      this.logger.info('Initializing ADK-Enhanced Hivemind API');
      
      this.orchestrator = orchestrator;
      this.memoryManager = memoryManager;
      this.queueManager = queueManager;
      
      // Create ADK Hivemind controller with default config
      this.adkController = new ADKHivemindController({
        maxConcurrentAgents: 5,
        enablePatterns: ['coordinator', 'pipeline', 'parallel', 'loop', 'hierarchical'],
        routingStrategy: 'hybrid',
        humanInLoopEnabled: true,
        reviewEnabled: true,
        hierarchicalDepth: 3
      });
      
      // Initialize with services
      await this.adkController.initialize(
        memoryManager as MemoryManager,
        queueManager
      );
      
      // Set up event handlers
      this.setupEventHandlers();
      
      this.outputChannel.appendLine('✅ ADK-Enhanced Hivemind initialized successfully');
      this.outputChannel.appendLine(`   Available patterns: coordinator, pipeline, parallel, loop, hierarchical`);
      this.outputChannel.appendLine(`   Human-in-the-loop: enabled`);
      this.outputChannel.appendLine(`   Review system: enabled`);
      
    } catch (error) {
      this.logger.error('Failed to initialize ADK Hivemind', error);
      throw error;
    }
  }
  
  private setupEventHandlers(): void {
    if (!this.adkController) return;
    
    this.adkController.on('stateChanged', (data) => {
      this.outputChannel.appendLine(`📊 State changed: ${data.key} = ${JSON.stringify(data.value)}`);
    });
    
    this.adkController.on('taskCompleted', ({ task, result }) => {
      this.outputChannel.appendLine(`✅ Task completed: ${task.title}`);
      this.outputChannel.appendLine(`   Pattern used: ${result.output.pattern || 'unknown'}`);
    });
    
    this.adkController.on('taskFailed', ({ task, error }) => {
      this.outputChannel.appendLine(`❌ Task failed: ${task.title} - ${error.message}`);
    });
    
    this.adkController.on('agentTaskStarted', (data) => {
      this.outputChannel.appendLine(`🚀 Agent ${data.agentId} started task`);
    });
    
    this.adkController.on('agentTaskCompleted', (data) => {
      this.outputChannel.appendLine(`✨ Agent ${data.agentId} completed task`);
    });
  }
  
  async createSession(config: ADKHivemindControllerConfig): Promise<string> {
    // ADK manages sessions internally, return a placeholder ID
    const sessionId = `adk_session_${Date.now()}`;
    
    this.outputChannel.appendLine(`\n🚀 ADK Session Created: ${sessionId}`);
    this.outputChannel.appendLine(`   Patterns enabled: ${config.enablePatterns?.join(', ') || 'all'}`);
    this.outputChannel.appendLine(`   Routing strategy: ${config.routingStrategy || 'hybrid'}`);
    
    // Reconfigure controller if needed
    if (config.enablePatterns) {
      this.configurePatterns(config.enablePatterns);
    }
    
    return sessionId;
  }
  
  async executeTask(request: HivemindTaskRequest): Promise<HivemindTaskResult> {
    if (!this.adkController) {
      throw new Error('ADK Hivemind not initialized');
    }
    
    // Convert request to HivemindTask
    const task: HivemindTask = {
      id: `task_${Date.now()}`,
      type: request.type,
      title: request.title,
      description: request.description,
      priority: request.priority || TaskPriority.Medium,
      requiredSpecialties: request.requiredSpecialties || this.inferSpecialties(request),
      estimatedComplexity: request.estimatedComplexity || this.estimateComplexity(request),
      status: TaskStatus.Pending,
      dependencies: [],
      assignedAgents: [],
      context: {
        files: request.files,
        codeContext: request.codeContext,
        projectPath: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
        requiresApproval: request.type === 'deployment' || request.type === 'deletion',
        requiresInput: request.type === 'configuration'
      },
      createdAt: new Date()
    };
    
    this.outputChannel.appendLine(`\n🎯 Executing task: ${task.title}`);
    this.outputChannel.appendLine(`   Type: ${task.type}`);
    this.outputChannel.appendLine(`   Complexity: ${task.estimatedComplexity}/10`);
    this.outputChannel.appendLine(`   Specialties: ${task.requiredSpecialties.join(', ')}`);
    
    try {
      // Execute with progress
      const result = await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `ADK Hivemind: ${task.title}`,
        cancellable: true
      }, async (progress, token) => {
        progress.report({ increment: 0, message: 'Routing task to agents...' });
        
        // Handle cancellation
        token.onCancellationRequested(() => {
          this.logger.info('Task cancelled by user');
        });
        
        const adkResult = await this.adkController!.executeTask(task);
        
        progress.report({ increment: 100, message: 'Complete!' });
        
        return adkResult;
      });
      
      // Convert to API result
      const apiResult: HivemindTaskResult = {
        success: result.success,
        summary: result.output.summary || JSON.stringify(result.output),
        filesModified: result.filesModified,
        filesCreated: result.filesCreated,
        testsAdded: result.testsAdded,
        issues: result.issuesFound,
        duration: result.performanceMetrics.duration,
        report: this.generateADKReport(task, result)
      };
      
      // Show result summary
      if (result.success) {
        this.outputChannel.appendLine(`\n✅ Task completed successfully!`);
        this.outputChannel.appendLine(`   Files created: ${result.filesCreated.length}`);
        this.outputChannel.appendLine(`   Files modified: ${result.filesModified.length}`);
        this.outputChannel.appendLine(`   Tests added: ${result.testsAdded}`);
        this.outputChannel.appendLine(`   Duration: ${(result.performanceMetrics.duration / 1000).toFixed(2)}s`);
        
        vscode.window.showInformationMessage(
          `ADK Hivemind: ${task.title} completed successfully! ` +
          `Modified ${result.filesModified.length} files, created ${result.filesCreated.length} files.`
        );
      } else {
        this.outputChannel.appendLine(`\n❌ Task failed!`);
        result.issuesFound.forEach(issue => {
          this.outputChannel.appendLine(`   - ${issue}`);
        });
        
        vscode.window.showErrorMessage(
          `ADK Hivemind: ${task.title} failed. Check output for details.`
        );
      }
      
      return apiResult;
      
    } catch (error) {
      this.logger.error('Task execution failed', error);
      const errorMessage = error instanceof Error ? error.message : String(error);
      this.outputChannel.appendLine(`\n❌ Error: ${errorMessage}`);
      
      throw error;
    }
  }
  
  private inferSpecialties(request: HivemindTaskRequest): AgentSpecialty[] {
    const specialties: AgentSpecialty[] = [];
    const keywords = `${request.type} ${request.title} ${request.description}`.toLowerCase();
    
    if (keywords.includes('ui') || keywords.includes('component') || keywords.includes('frontend')) {
      specialties.push(AgentSpecialty.Frontend);
    }
    
    if (keywords.includes('api') || keywords.includes('backend') || keywords.includes('server')) {
      specialties.push(AgentSpecialty.Backend);
    }
    
    if (keywords.includes('test') || keywords.includes('spec')) {
      specialties.push(AgentSpecialty.Testing);
    }
    
    if (keywords.includes('security') || keywords.includes('auth') || keywords.includes('encrypt')) {
      specialties.push(AgentSpecialty.Security);
    }
    
    if (keywords.includes('deploy') || keywords.includes('docker') || keywords.includes('ci')) {
      specialties.push(AgentSpecialty.DevOps);
    }
    
    if (keywords.includes('performance') || keywords.includes('optimize') || keywords.includes('speed')) {
      specialties.push(AgentSpecialty.Performance);
    }
    
    // Default to general if no specialties inferred
    if (specialties.length === 0) {
      specialties.push(AgentSpecialty.General);
    }
    
    return specialties;
  }
  
  private estimateComplexity(request: HivemindTaskRequest): number {
    let complexity = 5; // Base complexity
    
    // Adjust based on type
    const complexTypes = ['architecture', 'refactor', 'migration', 'security_audit'];
    if (complexTypes.includes(request.type)) {
      complexity += 2;
    }
    
    // Adjust based on description length
    if (request.description.length > 500) {
      complexity += 1;
    }
    
    // Adjust based on file count
    if (request.files && request.files.length > 5) {
      complexity += 1;
    }
    
    // Adjust based on required specialties
    if (request.requiredSpecialties && request.requiredSpecialties.length > 2) {
      complexity += 1;
    }
    
    return Math.min(complexity, 10);
  }
  
  private generateADKReport(task: HivemindTask, result: any): string {
    let report = `# ADK Hivemind Task Report\n\n`;
    report += `**Task:** ${task.title}\n`;
    report += `**Type:** ${task.type}\n`;
    report += `**Status:** ${result.success ? 'Success' : 'Failed'}\n`;
    report += `**Duration:** ${(result.performanceMetrics.duration / 1000).toFixed(2)}s\n`;
    report += `**Pattern Used:** ${result.output.pattern || 'Unknown'}\n\n`;
    
    if (result.filesCreated.length > 0) {
      report += `## Files Created (${result.filesCreated.length})\n`;
      result.filesCreated.forEach((file: string) => {
        report += `- ${file}\n`;
      });
      report += `\n`;
    }
    
    if (result.filesModified.length > 0) {
      report += `## Files Modified (${result.filesModified.length})\n`;
      result.filesModified.forEach((file: string) => {
        report += `- ${file}\n`;
      });
      report += `\n`;
    }
    
    if (result.testsAdded > 0) {
      report += `## Tests Added: ${result.testsAdded}\n\n`;
    }
    
    if (result.output.agentResults) {
      report += `## Agent Results\n`;
      result.output.agentResults.forEach((r: any) => {
        report += `- **${r.agentId}**: ${r.success ? 'Success' : 'Failed'}\n`;
      });
      report += `\n`;
    }
    
    if (result.issuesFound.length > 0) {
      report += `## Issues Found\n`;
      result.issuesFound.forEach((issue: string) => {
        report += `- ${issue}\n`;
      });
    }
    
    if (result.output.reviewResult) {
      report += `\n## Review Results\n`;
      report += `- Overall Score: ${result.output.reviewResult.overallScore}%\n`;
      report += `- Passed: ${result.output.reviewResult.passed ? 'Yes' : 'No'}\n`;
    }
    
    return report;
  }
  
  getMetrics(): any {
    if (!this.adkController) {
      return {};
    }
    
    return this.adkController.getMetrics();
  }
  
  listAgents(): Array<{ id: string; name: string; type: string }> {
    if (!this.adkController) {
      return [];
    }
    
    return this.adkController.listAgents();
  }
  
  getAgentStats(): Map<string, any> {
    // ADK controller manages stats internally
    const stats = new Map<string, any>();
    
    if (this.adkController) {
      const agents = this.adkController.listAgents();
      const metrics = this.adkController.getMetrics();
      
      agents.forEach(agent => {
        stats.set(agent.id, {
          name: agent.name,
          type: agent.type,
          tasksProcessed: metrics.tasksProcessed || 0,
          successRate: metrics.successRate || 0
        });
      });
    }
    
    return stats;
  }
  
  configurePatterns(patterns: string[]): void {
    if (!this.adkController) {
      throw new Error('ADK Hivemind not initialized');
    }
    
    // This would require reinitializing the controller with new patterns
    this.outputChannel.appendLine(`\n🔧 Configuring patterns: ${patterns.join(', ')}`);
    this.outputChannel.appendLine(`   Note: Pattern changes will take effect on next initialization`);
  }
  
  async shutdown(): Promise<void> {
    if (!this.adkController) return;
    
    this.logger.info('Shutting down ADK Hivemind API');
    
    await this.adkController.shutdown();
    this.adkController = undefined;
    
    this.outputChannel.appendLine('\n🛑 ADK Hivemind shut down');
    this.outputChannel.dispose();
  }
}

// Export singleton instance
export const hivemindAPI = new HivemindAPIImpl();