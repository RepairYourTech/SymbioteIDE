/**
 * Dashboard Data Collector
 * 
 * Collects metrics and data from various SymbioteIDE components
 */

import * as os from 'os';
import { 
  DashboardState,
  SystemStatus,
  AgentStatus,
  MCPServerStatus,
  MemoryMetrics,
  OrchestrationMetrics,
  ModelUsageStats,
  CostMetrics,
  ActivityEntry,
  ExecutionRecord,
  UserSettings,
  APIKeyStatus,
  FeatureFlag
} from '../types/dashboard-types';
import { OrchestrationEngine } from '../../orchestration';
import { MCPServerManager } from '../../mcp/mcp-server-manager';
import { MemoryManager } from '../../memory/unified-context-api';
import { ADKClient } from '../../adk/adk-client';
import { CredentialManager } from '../../credentials';
import { Logger } from '../../utils/logger';
import * as vscode from 'vscode';

export class DashboardDataCollector {
  private logger = new Logger('DashboardDataCollector');
  private startTime = Date.now();
  
  constructor(
    private orchestrationEngine: OrchestrationEngine,
    private mcpServerManager: MCPServerManager,
    private memoryManager: MemoryManager,
    private adkClient: ADKClient,
    private credentialManager: CredentialManager
  ) {}
  
  /**
   * Collect full dashboard state
   */
  async collectFullState(): Promise<DashboardState> {
    const [
      systemStatus,
      agents,
      mcpServers,
      memoryUsage,
      orchestrationMetrics,
      modelUsage,
      costTracking,
      activityLog,
      executionHistory,
      userSettings,
      apiKeys,
      featureFlags
    ] = await Promise.all([
      this.getSystemStatus(),
      this.getAgentStatus(),
      this.getMCPServerStatus(),
      this.getMemoryMetrics(),
      this.getOrchestrationMetrics(),
      this.getModelUsageStats(),
      this.getCostMetrics(),
      this.getActivityLog(),
      this.getExecutionHistory(),
      this.getUserSettings(),
      this.getAPIKeyStatus(),
      this.getFeatureFlags()
    ]);
    
    return {
      systemStatus,
      agents,
      mcpServers,
      memoryUsage,
      orchestrationMetrics,
      modelUsage,
      costTracking,
      activityLog,
      executionHistory,
      userSettings,
      apiKeys,
      featureFlags
    };
  }
  
  /**
   * Collect partial state for specific sections
   */
  async collectPartialState(sections: string[]): Promise<Partial<DashboardState>> {
    const state: Partial<DashboardState> = {};
    
    const collectors: Record<string, () => Promise<any>> = {
      systemStatus: () => this.getSystemStatus(),
      agents: () => this.getAgentStatus(),
      mcpServers: () => this.getMCPServerStatus(),
      memoryUsage: () => this.getMemoryMetrics(),
      orchestrationMetrics: () => this.getOrchestrationMetrics(),
      modelUsage: () => this.getModelUsageStats(),
      costTracking: () => this.getCostMetrics(),
      activityLog: () => this.getActivityLog(),
      executionHistory: () => this.getExecutionHistory(),
      userSettings: () => this.getUserSettings(),
      apiKeys: () => this.getAPIKeyStatus(),
      featureFlags: () => this.getFeatureFlags()
    };
    
    await Promise.all(
      sections.map(async (section) => {
        if (collectors[section]) {
          (state as any)[section] = await collectors[section]();
        }
      })
    );
    
    return state;
  }
  
  /**
   * Get system status
   */
  private async getSystemStatus(): Promise<SystemStatus> {
    const cpus = os.cpus();
    const totalMemory = os.totalmem();
    const freeMemory = os.freemem();
    const usedMemory = totalMemory - freeMemory;
    
    // Calculate CPU usage
    const cpuUsage = cpus.reduce((acc, cpu) => {
      const total = Object.values(cpu.times).reduce((a, b) => a + b, 0);
      const idle = cpu.times.idle;
      return acc + ((total - idle) / total);
    }, 0) / cpus.length * 100;
    
    return {
      version: vscode.extensions.getExtension('symbiote.ide')?.packageJSON.version || '0.1.0',
      uptime: Date.now() - this.startTime,
      status: 'healthy', // TODO: Implement actual health checks
      lastUpdated: new Date(),
      resources: {
        cpu: Math.round(cpuUsage),
        memory: {
          used: usedMemory,
          total: totalMemory,
          percentage: Math.round((usedMemory / totalMemory) * 100)
        },
        disk: {
          used: 0, // TODO: Implement disk usage
          total: 0,
          percentage: 0
        }
      }
    };
  }
  
  /**
   * Get agent status
   */
  private async getAgentStatus(): Promise<AgentStatus[]> {
    try {
      const agentList = await this.adkClient.listAgents();
      const agents: AgentStatus[] = [];
      
      for (const agent of agentList.agents) {
        // Get metrics from orchestration engine
        const metrics = await this.orchestrationEngine.getAgentMetrics(agent.id);
        
        agents.push({
          id: agent.id,
          name: agent.name,
          type: agent.type,
          status: agent.status as any,
          provider: 'google-adk',
          model: 'gemini-1.5-pro', // TODO: Get from agent config
          lastActivity: metrics?.lastActivity,
          metrics: {
            totalExecutions: metrics?.totalExecutions || 0,
            successRate: metrics?.successRate || 0,
            averageResponseTime: metrics?.averageResponseTime || 0
          },
          capabilities: ['streaming', 'tools', 'memory']
        });
      }
      
      return agents;
    } catch (error) {
      this.logger.error('Failed to get agent status:', error);
      return [];
    }
  }
  
  /**
   * Get MCP server status
   */
  private async getMCPServerStatus(): Promise<MCPServerStatus[]> {
    const servers = this.mcpServerManager.getServers();
    const statuses: MCPServerStatus[] = [];
    
    for (const [id, server] of servers) {
      const health = await this.mcpServerManager.checkServerHealth(id);
      const capabilities = await this.mcpServerManager.getServerCapabilities(id);
      
      statuses.push({
        id,
        name: server.name || id,
        url: server.command || 'stdio',
        status: health ? 'connected' : 'disconnected',
        transport: server.transport || 'stdio',
        lastPing: new Date(),
        tools: capabilities?.tools?.length || 0,
        resources: capabilities?.resources?.length || 0,
        prompts: capabilities?.prompts?.length || 0
      });
    }
    
    return statuses;
  }
  
  /**
   * Get memory metrics
   */
  private async getMemoryMetrics(): Promise<MemoryMetrics> {
    const stats = await this.memoryManager.getStats();
    
    return {
      shortTerm: {
        entries: stats.shortTerm?.entries || 0,
        sizeBytes: stats.shortTerm?.sizeBytes || 0
      },
      longTerm: {
        entries: stats.longTerm?.entries || 0,
        sizeBytes: stats.longTerm?.sizeBytes || 0
      },
      vectorStore: {
        documents: stats.vectorStore?.documents || 0,
        vectors: stats.vectorStore?.vectors || 0,
        dimensions: stats.vectorStore?.dimensions || 384
      },
      cacheHitRate: stats.cacheHitRate || 0
    };
  }
  
  /**
   * Get orchestration metrics
   */
  private async getOrchestrationMetrics(): Promise<OrchestrationMetrics> {
    const metrics = await this.orchestrationEngine.getMetrics();
    const queueStats = await this.orchestrationEngine.getQueueStats();
    const rateLimits = await this.orchestrationEngine.getRateLimits();
    
    return {
      activeRequests: queueStats.active || 0,
      queueLength: queueStats.waiting || 0,
      averageLatency: metrics.averageLatency || 0,
      throughput: {
        requestsPerMinute: metrics.requestsPerMinute || 0,
        tokensPerMinute: metrics.tokensPerMinute || 0
      },
      errorRate: metrics.errorRate || 0,
      rateLimits: Object.entries(rateLimits).map(([provider, limit]) => ({
        provider,
        used: limit.used || 0,
        limit: limit.limit || 0,
        resetAt: new Date(limit.resetAt || Date.now())
      }))
    };
  }
  
  /**
   * Get model usage statistics
   */
  private async getModelUsageStats(): Promise<ModelUsageStats[]> {
    const usage = await this.orchestrationEngine.getModelUsage();
    
    return Object.entries(usage).map(([modelKey, stats]) => {
      const [provider, model] = modelKey.split(':');
      return {
        model,
        provider,
        requests: stats.requests || 0,
        tokensUsed: {
          input: stats.inputTokens || 0,
          output: stats.outputTokens || 0,
          total: (stats.inputTokens || 0) + (stats.outputTokens || 0)
        },
        cost: stats.cost || 0,
        averageLatency: stats.averageLatency || 0,
        errorRate: stats.errorRate || 0
      };
    });
  }
  
  /**
   * Get cost metrics
   */
  private async getCostMetrics(): Promise<CostMetrics> {
    const costs = await this.orchestrationEngine.getCostBreakdown();
    const now = new Date();
    const startOfMonth = new Date(now.getFullYear(), now.getMonth(), 1);
    
    // Calculate projections
    const daysInMonth = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
    const daysPassed = now.getDate();
    const dailyAverage = costs.total / daysPassed;
    
    return {
      currentPeriod: {
        start: startOfMonth,
        end: now,
        totalCost: costs.total || 0,
        costByProvider: costs.byProvider || {},
        costByModel: costs.byModel || {}
      },
      projection: {
        daily: dailyAverage,
        weekly: dailyAverage * 7,
        monthly: dailyAverage * daysInMonth
      },
      budget: {
        limit: 100, // TODO: Get from config
        used: costs.total || 0,
        percentage: Math.min(100, ((costs.total || 0) / 100) * 100),
        alertThreshold: 80
      }
    };
  }
  
  /**
   * Get activity log
   */
  private async getActivityLog(limit = 100): Promise<ActivityEntry[]> {
    // TODO: Implement actual activity log retrieval
    return [
      {
        id: '1',
        timestamp: new Date(),
        type: 'system',
        severity: 'info',
        source: 'Dashboard',
        message: 'Dashboard initialized'
      }
    ];
  }
  
  /**
   * Get execution history
   */
  private async getExecutionHistory(limit = 50): Promise<ExecutionRecord[]> {
    // TODO: Implement actual execution history
    return [];
  }
  
  /**
   * Get user settings
   */
  private async getUserSettings(): Promise<UserSettings> {
    const config = vscode.workspace.getConfiguration('symbiote');
    
    return {
      theme: 'auto',
      dashboard: {
        refreshInterval: config.get('dashboard.refreshInterval', 5000),
        defaultView: config.get('dashboard.defaultView', 'overview'),
        widgets: []
      },
      notifications: {
        enabled: config.get('notifications.enabled', true),
        types: config.get('notifications.types', ['error', 'warning']),
        channels: config.get('notifications.channels', ['vscode'])
      },
      telemetry: {
        enabled: config.get('telemetry.enabled', true),
        level: config.get('telemetry.level', 'standard')
      }
    };
  }
  
  /**
   * Get API key status
   */
  private async getAPIKeyStatus(): Promise<APIKeyStatus[]> {
    const providers = [
      'anthropic',
      'openai',
      'google',
      'mistral',
      'perplexity'
    ];
    
    const statuses: APIKeyStatus[] = [];
    
    for (const provider of providers) {
      const configured = await this.credentialManager.hasCredential(`${provider}_api_key`);
      statuses.push({
        provider,
        configured,
        valid: configured, // TODO: Implement validation
        lastValidated: configured ? new Date() : undefined
      });
    }
    
    return statuses;
  }
  
  /**
   * Get feature flags
   */
  private async getFeatureFlags(): Promise<FeatureFlag[]> {
    const config = vscode.workspace.getConfiguration('symbiote.features');
    
    return [
      {
        id: 'experimental-agents',
        name: 'Experimental Agents',
        description: 'Enable experimental agent types',
        enabled: config.get('experimentalAgents', false),
        category: 'agents',
        experimental: true
      },
      {
        id: 'advanced-caching',
        name: 'Advanced Caching',
        description: 'Enable provider-specific context caching',
        enabled: config.get('advancedCaching', true),
        category: 'performance'
      },
      {
        id: 'real-time-collaboration',
        name: 'Real-time Collaboration',
        description: 'Enable real-time collaboration features',
        enabled: config.get('realTimeCollaboration', false),
        category: 'collaboration',
        experimental: true
      }
    ];
  }
  
  /**
   * Get specific data
   */
  async getSpecificData(dataType: string, params?: any): Promise<any> {
    switch (dataType) {
      case 'executionDetails':
        return this.getExecutionDetails(params.executionId);
      case 'agentDetails':
        return this.getAgentDetails(params.agentId);
      case 'costHistory':
        return this.getCostHistory(params.days || 30);
      default:
        throw new Error(`Unknown data type: ${dataType}`);
    }
  }
  
  private async getExecutionDetails(executionId: string): Promise<any> {
    // TODO: Implement execution details retrieval
    return {};
  }
  
  private async getAgentDetails(agentId: string): Promise<any> {
    // TODO: Implement agent details retrieval
    return {};
  }
  
  private async getCostHistory(days: number): Promise<any> {
    // TODO: Implement cost history retrieval
    return [];
  }
  
  /**
   * Action handlers
   */
  async toggleAgent(agentId: string, enabled: boolean): Promise<void> {
    if (enabled) {
      // Start agent
      this.logger.info(`Starting agent: ${agentId}`);
      // TODO: Implement agent start
    } else {
      // Stop agent
      this.logger.info(`Stopping agent: ${agentId}`);
      await this.adkClient.deleteAgent(agentId);
    }
  }
  
  async clearCache(cacheType: string): Promise<void> {
    switch (cacheType) {
      case 'all':
        await this.memoryManager.clearAll();
        await this.orchestrationEngine.clearCache();
        break;
      case 'memory':
        await this.memoryManager.clearMemory();
        break;
      case 'context':
        await this.orchestrationEngine.clearContextCache();
        break;
      case 'results':
        await this.orchestrationEngine.clearResultCache();
        break;
    }
  }
  
  async restartMCPServer(serverId: string): Promise<void> {
    await this.mcpServerManager.restartServer(serverId);
  }
}