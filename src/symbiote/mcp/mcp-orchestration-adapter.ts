/**
 * MCP Orchestration Adapter
 * 
 * Integrates MCP server tools into the AI orchestration system
 */

import { ProviderAdapter } from '../orchestration/providers/provider-adapter';
import { 
  AITask, 
  TaskResult, 
  ModelProfile, 
  ExecutionOptions,
  TaskType,
  Capability,
  ModelAvailability,
  ComplexityLevel,
  QualityLevel
} from '../orchestration/interfaces';
import { McpClient } from './mcp-client';
import { McpTool, McpToolMapping, McpIntegrationConfig } from './mcp-types';

export class McpOrchestrationAdapter implements ProviderAdapter {
  private client: McpClient;
  private config: McpIntegrationConfig;
  private toolCache: Map<string, McpTool> = new Map();
  
  constructor(client: McpClient, config: McpIntegrationConfig) {
    this.client = client;
    this.config = config;
    
    // Subscribe to capability updates
    this.client.on('server-event', (event) => {
      if (event.type === 'capabilities-updated' && event.serverId === config.serverId) {
        this.updateToolCache();
      }
    });
    
    // Initial cache update
    this.updateToolCache();
  }
  
  /**
   * Get available models (MCP tools exposed as models)
   */
  async getAvailableModels(): Promise<ModelProfile[]> {
    const tools = await this.client.listTools(this.config.serverId);
    const models: ModelProfile[] = [];
    
    for (const tool of tools) {
      // Only include tools that are mapped or in default tools
      if (this.shouldIncludeTool(tool)) {
        models.push(this.toolToModelProfile(tool));
      }
    }
    
    return models;
  }
  
  /**
   * Execute a task using MCP tool
   */
  async execute(
    task: AITask,
    model: ModelProfile,
    options?: ExecutionOptions
  ): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      // Find the corresponding tool
      const tool = this.toolCache.get(model.id);
      if (!tool) {
        throw new Error(`Tool ${model.id} not found`);
      }
      
      // Prepare tool arguments from task
      const toolArgs = this.prepareToolArguments(task, tool);
      
      // Execute tool
      const result = await this.client.executeTool({
        serverId: this.config.serverId,
        toolName: tool.name,
        arguments: toolArgs,
        timeout: options?.timeout
      });
      
      if (!result.success) {
        throw new Error(result.error || 'Tool execution failed');
      }
      
      // Convert result to TaskResult
      return {
        id: `mcp-result-${Date.now()}`,
        taskId: task.id,
        status: 'success',
        content: this.formatToolResult(result.result),
        model: model.id,
        usage: {
          promptTokens: 0, // MCP tools don't report token usage
          completionTokens: 0,
          totalTokens: 0
        },
        cost: {
          amount: 0, // MCP tools are typically free
          currency: 'USD',
          breakdown: {
            inputTokens: 0,
            outputTokens: 0,
            inputCost: 0,
            outputCost: 0
          }
        },
        latency: result.duration,
        timestamp: Date.now(),
        metadata: {
          serverId: this.config.serverId,
          toolName: tool.name,
          mcpResult: result.result
        }
      };
      
    } catch (error) {
      return {
        id: `mcp-result-${Date.now()}`,
        taskId: task.id,
        status: 'failed',
        model: model.id,
        error: {
          code: 'MCP_EXECUTION_ERROR',
          message: error instanceof Error ? error.message : String(error),
          type: 'tool_error',
          retryable: true,
          details: { serverId: this.config.serverId, modelId: model.id }
        },
        usage: {
          promptTokens: 0,
          completionTokens: 0,
          totalTokens: 0
        },
        cost: {
          amount: 0,
          currency: 'USD',
          breakdown: {} as any
        },
        latency: Date.now() - startTime,
        timestamp: Date.now()
      };
    }
  }
  
  /**
   * Validate API key (not applicable for MCP)
   */
  async validateApiKey(apiKey: string): Promise<boolean> {
    // MCP servers don't use API keys directly
    return true;
  }
  
  /**
   * Test a model (tool)
   */
  async testModel(modelId: string): Promise<boolean> {
    try {
      const tool = this.toolCache.get(modelId);
      if (!tool) {
        return false;
      }
      
      // Try to execute with minimal arguments
      const result = await this.client.executeTool({
        serverId: this.config.serverId,
        toolName: tool.name,
        arguments: {},
        timeout: 5000
      });
      
      return result.success;
    } catch {
      return false;
    }
  }
  
  /**
   * Get model details
   */
  async getModelDetails(modelId: string): Promise<ModelProfile | null> {
    const tool = this.toolCache.get(modelId);
    if (!tool) {
      return null;
    }
    
    return this.toolToModelProfile(tool);
  }
  
  /**
   * Shutdown the adapter
   */
  async shutdown(): Promise<void> {
    // Cleanup if needed
    this.toolCache.clear();
  }
  
  // Private methods
  
  private async updateToolCache(): Promise<void> {
    const tools = await this.client.listTools(this.config.serverId);
    this.toolCache.clear();
    
    for (const tool of tools) {
      if (this.shouldIncludeTool(tool)) {
        const modelId = this.getModelId(tool);
        this.toolCache.set(modelId, tool);
      }
    }
  }
  
  private shouldIncludeTool(tool: McpTool): boolean {
    // Check if tool is in default tools
    if (this.config.defaultTools?.includes(tool.name)) {
      return true;
    }
    
    // Check if tool is mapped to any task type
    if (this.config.toolMapping) {
      for (const tools of Object.values(this.config.toolMapping)) {
        if (tools.includes(tool.name)) {
          return true;
        }
      }
    }
    
    // If no specific configuration, include all tools
    return !this.config.defaultTools && !this.config.toolMapping;
  }
  
  private toolToModelProfile(tool: McpTool): ModelProfile {
    const modelId = this.getModelId(tool);
    
    return {
      id: modelId,
      provider: `mcp-${this.config.serverId}`,
      name: tool.name,
      displayName: tool.description || tool.name,
      version: '1.0.0', // MCP doesn't version tools
      capabilities: this.getToolCapabilities(tool),
      contextWindow: 1000000, // Arbitrary large number
      maxOutputTokens: 1000000, // MCP tools don't have token limits
      costPerToken: {
        input: 0,
        output: 0
      },
      averageLatency: 1000, // Default estimate
      reliability: 0.95, // Default high reliability
      availability: {
        status: 'available',
        lastChecked: new Date()
      },
      specializations: this.getToolSpecializations(tool)
    };
  }
  
  private getModelId(tool: McpTool): string {
    return `mcp-${this.config.serverId}-${tool.name}`;
  }
  
  private getToolCapabilities(tool: McpTool): Capability[] {
    const capabilities: Capability[] = [];
    
    // Analyze tool name and description to infer capabilities
    const nameAndDesc = `${tool.name} ${tool.description || ''}`.toLowerCase();
    
    if (nameAndDesc.includes('code') || nameAndDesc.includes('program')) {
      capabilities.push(Capability.CodeGeneration);
    }
    if (nameAndDesc.includes('analyze') || nameAndDesc.includes('review')) {
      capabilities.push(Capability.CodeAnalysis);
    }
    if (nameAndDesc.includes('test')) {
      capabilities.push(Capability.Testing);
    }
    if (nameAndDesc.includes('doc') || nameAndDesc.includes('comment')) {
      capabilities.push(Capability.Documentation);
    }
    if (nameAndDesc.includes('debug') || nameAndDesc.includes('fix')) {
      capabilities.push(Capability.Debugging);
    }
    if (nameAndDesc.includes('refactor') || nameAndDesc.includes('improve')) {
      capabilities.push(Capability.Refactoring);
    }
    if (nameAndDesc.includes('explain') || nameAndDesc.includes('understand')) {
      capabilities.push(Capability.Explanation);
    }
    if (nameAndDesc.includes('complete') || nameAndDesc.includes('suggest')) {
      capabilities.push(Capability.Completion);
    }
    if (nameAndDesc.includes('translate')) {
      capabilities.push(Capability.Translation);
    }
    if (nameAndDesc.includes('creative') || nameAndDesc.includes('generate')) {
      capabilities.push(Capability.CreativeWriting);
    }
    if (nameAndDesc.includes('math') || nameAndDesc.includes('calculate')) {
      capabilities.push(Capability.Mathematics);
    }
    if (nameAndDesc.includes('data') || nameAndDesc.includes('analyze')) {
      capabilities.push(Capability.DataAnalysis);
    }
    
    // Default to natural language if no specific capability found
    if (capabilities.length === 0) {
      capabilities.push(Capability.NaturalLanguage);
    }
    
    return capabilities;
  }
  
  private getToolSpecializations(tool: McpTool): {
    taskTypes: TaskType[];
    languages?: string[];
    frameworks?: string[];
    complexityLevels: ComplexityLevel[];
    qualityLevels: QualityLevel[];
  } {
    const taskTypes: TaskType[] = [];
    
    // Map tool to task types based on tool mapping config
    if (this.config.toolMapping) {
      for (const [taskType, tools] of Object.entries(this.config.toolMapping)) {
        if (tools.includes(tool.name)) {
          taskTypes.push(taskType as TaskType);
        }
      }
    }
    
    // If no mapping, infer from capabilities
    if (taskTypes.length === 0) {
      const capabilities = this.getToolCapabilities(tool);
      
      if (capabilities.includes(Capability.CodeGeneration)) {
        taskTypes.push(TaskType.CodeGeneration);
      }
      if (capabilities.includes(Capability.CodeAnalysis)) {
        taskTypes.push(TaskType.CodeReview);
      }
      if (capabilities.includes(Capability.Refactoring)) {
        taskTypes.push(TaskType.Refactoring);
      }
      if (capabilities.includes(Capability.Documentation)) {
        taskTypes.push(TaskType.Documentation);
      }
      if (capabilities.includes(Capability.Testing)) {
        taskTypes.push(TaskType.Testing);
      }
      if (capabilities.includes(Capability.Debugging)) {
        taskTypes.push(TaskType.BugFix);
      }
      if (capabilities.includes(Capability.Translation)) {
        taskTypes.push(TaskType.Translation);
      }
      
      // Default to general if no specific type
      if (taskTypes.length === 0) {
        taskTypes.push(TaskType.General);
      }
    }
    
    return {
      taskTypes,
      complexityLevels: [
        ComplexityLevel.Low,
        ComplexityLevel.Medium,
        ComplexityLevel.High
      ],
      qualityLevels: [
        QualityLevel.Standard,
        QualityLevel.High,
        QualityLevel.Premium
      ]
    };
  }
  
  private prepareToolArguments(task: AITask, tool: McpTool): any {
    const args: any = {};
    
    // Add prompt/input
    if (tool.inputSchema?.properties?.prompt || 
        tool.inputSchema?.properties?.input ||
        tool.inputSchema?.properties?.query) {
      const key = tool.inputSchema.properties.prompt ? 'prompt' :
                  tool.inputSchema.properties.input ? 'input' : 'query';
      args[key] = task.prompt;
    }
    
    // Add context if supported
    if (tool.inputSchema?.properties?.context && task.context) {
      args.context = {
        files: task.context.files,
        language: task.context.language,
        framework: task.context.framework
      };
    }
    
    // Add constraints if supported
    if (tool.inputSchema?.properties?.maxTokens && task.constraints?.maxTokens) {
      args.maxTokens = task.constraints.maxTokens;
    }
    
    if (tool.inputSchema?.properties?.temperature && task.constraints?.temperature) {
      args.temperature = task.constraints.temperature;
    }
    
    // If no schema or empty schema, just pass the prompt
    if (!tool.inputSchema || Object.keys(args).length === 0) {
      return { prompt: task.prompt };
    }
    
    return args;
  }
  
  private formatToolResult(result: any): string {
    if (typeof result === 'string') {
      return result;
    }
    
    if (result.output) {
      return String(result.output);
    }
    
    if (result.content) {
      return String(result.content);
    }
    
    if (result.text) {
      return String(result.text);
    }
    
    if (result.message) {
      return String(result.message);
    }
    
    // Fallback to JSON
    return JSON.stringify(result, null, 2);
  }
}