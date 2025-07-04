/**
 * Tool Builder Agent
 * 
 * Specialized agent that can create tools for other agents
 */

import { ADKTool } from './types';
import { Logger } from '../utils/logger';

interface ToolSpecification {
  name: string;
  description: string;
  parameters: any;
  implementation: 'code' | 'api' | 'agent' | 'mcp';
  endpoint?: string;
  mcpServer?: string;
  mcpTool?: string;
}

export class ToolBuilderAgent {
  private logger = new Logger('ToolBuilderAgent');
  private toolRegistry = new Map<string, ADKTool>();
  
  constructor() {
    this.initializeCommonTools();
  }
  
  /**
   * Initialize common tool templates
   */
  private initializeCommonTools(): void {
    // File operations
    this.registerToolTemplate('file_reader', {
      name: 'Read File',
      description: 'Read contents of a file',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'File path to read' }
        },
        required: ['path']
      },
      implementation: async (params: { path: string }) => {
        const fs = await import('fs/promises');
        return await fs.readFile(params.path, 'utf-8');
      }
    });
    
    // Code analysis
    this.registerToolTemplate('code_analyzer', {
      name: 'Analyze Code',
      description: 'Analyze code for patterns, issues, or metrics',
      parameters: {
        type: 'object',
        properties: {
          code: { type: 'string', description: 'Code to analyze' },
          language: { type: 'string', description: 'Programming language' },
          checks: {
            type: 'array',
            items: { type: 'string' },
            description: 'Types of analysis to perform'
          }
        },
        required: ['code', 'language']
      },
      implementation: async (params: any) => {
        // Placeholder for code analysis
        return {
          language: params.language,
          lines: params.code.split('\n').length,
          complexity: 'medium',
          issues: []
        };
      }
    });
    
    // API caller
    this.registerToolTemplate('api_caller', {
      name: 'Call API',
      description: 'Make HTTP API calls',
      parameters: {
        type: 'object',
        properties: {
          url: { type: 'string', description: 'API endpoint URL' },
          method: { 
            type: 'string', 
            enum: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'],
            description: 'HTTP method' 
          },
          headers: { 
            type: 'object', 
            description: 'Request headers' 
          },
          body: { 
            type: 'object', 
            description: 'Request body' 
          }
        },
        required: ['url', 'method']
      },
      implementation: async (params: any) => {
        const response = await fetch(params.url, {
          method: params.method,
          headers: params.headers || {},
          body: params.body ? JSON.stringify(params.body) : undefined
        });
        
        return {
          status: response.status,
          data: await response.json()
        };
      }
    });
    
    // Database query
    this.registerToolTemplate('database_query', {
      name: 'Query Database',
      description: 'Execute database queries',
      parameters: {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'SQL query' },
          database: { type: 'string', description: 'Database name' },
          params: { 
            type: 'array', 
            description: 'Query parameters' 
          }
        },
        required: ['query']
      },
      implementation: async (params: any) => {
        // Placeholder - would connect to actual database
        return {
          rows: [],
          affected: 0
        };
      }
    });
    
    // Shell command
    this.registerToolTemplate('shell_command', {
      name: 'Execute Shell Command',
      description: 'Run shell commands',
      parameters: {
        type: 'object',
        properties: {
          command: { type: 'string', description: 'Command to execute' },
          cwd: { type: 'string', description: 'Working directory' },
          env: { type: 'object', description: 'Environment variables' }
        },
        required: ['command']
      },
      implementation: async (params: any) => {
        const { exec } = await import('child_process');
        const { promisify } = await import('util');
        const execAsync = promisify(exec);
        
        try {
          const result = await execAsync(params.command, {
            cwd: params.cwd,
            env: { ...process.env, ...params.env }
          });
          
          return {
            stdout: result.stdout,
            stderr: result.stderr
          };
        } catch (error: any) {
          return {
            error: error.message,
            stdout: error.stdout,
            stderr: error.stderr
          };
        }
      }
    });
  }
  
  /**
   * Create a new tool based on specification
   */
  async createTool(spec: ToolSpecification): Promise<ADKTool> {
    this.logger.info(`Creating tool: ${spec.name}`);
    
    try {
      let tool: ADKTool;
      
      switch (spec.implementation) {
        case 'code':
          tool = await this.createCodeTool(spec);
          break;
          
        case 'api':
          tool = await this.createAPITool(spec);
          break;
          
        case 'agent':
          tool = await this.createAgentTool(spec);
          break;
          
        case 'mcp':
          tool = await this.createMCPTool(spec);
          break;
          
        default:
          throw new Error(`Unknown implementation type: ${spec.implementation}`);
      }
      
      // Register tool
      this.toolRegistry.set(spec.name, tool);
      
      return tool;
      
    } catch (error) {
      this.logger.error(`Failed to create tool ${spec.name}`, error);
      throw error;
    }
  }
  
  /**
   * Create a tool from code implementation
   */
  private async createCodeTool(spec: ToolSpecification): Promise<ADKTool> {
    // Generate implementation based on description
    const implementation = await this.generateImplementation(spec);
    
    return {
      name: spec.name,
      description: spec.description,
      parameters: spec.parameters,
      execute: implementation
    } as ADKTool;
  }
  
  /**
   * Create a tool that calls an API
   */
  private async createAPITool(spec: ToolSpecification): Promise<ADKTool> {
    if (!spec.endpoint) {
      throw new Error('API endpoint required for API tools');
    }
    
    return {
      name: spec.name,
      description: spec.description,
      parameters: spec.parameters,
      execute: async (params: any) => {
        const response = await fetch(spec.endpoint!, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify(params)
        });
        
        if (!response.ok) {
          throw new Error(`API call failed: ${response.statusText}`);
        }
        
        return await response.json();
      }
    } as ADKTool;
  }
  
  /**
   * Create a tool that uses another agent
   */
  private async createAgentTool(spec: ToolSpecification): Promise<ADKTool> {
    return {
      name: spec.name,
      description: spec.description,
      parameters: spec.parameters,
      execute: async (params: any) => {
        // This would call another agent
        // Placeholder implementation
        return {
          result: 'Agent tool executed',
          params
        };
      }
    } as ADKTool;
  }
  
  /**
   * Create a tool from MCP server
   */
  private async createMCPTool(spec: ToolSpecification): Promise<ADKTool> {
    if (!spec.mcpServer || !spec.mcpTool) {
      throw new Error('MCP server and tool name required');
    }
    
    return {
      name: spec.name,
      description: spec.description,
      parameters: spec.parameters,
      execute: async (params: any) => {
        // This would call MCP tool
        // Placeholder implementation
        return {
          result: 'MCP tool executed',
          server: spec.mcpServer,
          tool: spec.mcpTool,
          params
        };
      }
    } as ADKTool;
  }
  
  /**
   * Generate implementation from description using AI
   */
  private async generateImplementation(spec: ToolSpecification): Promise<(params: any) => Promise<any>> {
    // In a real implementation, this would use an LLM to generate code
    // For now, return a placeholder
    
    return async (params: any) => {
      this.logger.debug(`Executing generated tool: ${spec.name}`, params);
      
      // Basic implementation based on common patterns
      if (spec.name.includes('read') || spec.name.includes('get')) {
        return { data: 'Read operation result', params };
      } else if (spec.name.includes('write') || spec.name.includes('create')) {
        return { success: true, id: Date.now(), params };
      } else if (spec.name.includes('delete') || spec.name.includes('remove')) {
        return { success: true, deleted: 1, params };
      } else {
        return { result: 'Operation completed', params };
      }
    };
  }
  
  /**
   * Create a composite tool from multiple tools
   */
  async createCompositeTool(spec: {
    name: string;
    description: string;
    tools: string[];
    flow: 'sequential' | 'parallel' | 'conditional';
    conditions?: Record<string, string>;
  }): Promise<ADKTool> {
    const tools = spec.tools.map(name => {
      const tool = this.toolRegistry.get(name);
      if (!tool) {
        throw new Error(`Tool not found: ${name}`);
      }
      return tool;
    });
    
    return {
      name: spec.name,
      description: spec.description,
      parameters: {
        type: 'object',
        properties: {
          input: { type: 'object', description: 'Input for composite tool' }
        }
      },
      execute: async (params: any) => {
        const results: any[] = [];
        
        if (spec.flow === 'sequential') {
          let input = params.input;
          
          for (const tool of tools) {
            const result = await tool.execute(input);
            results.push(result);
            input = result; // Pass result to next tool
          }
        } else if (spec.flow === 'parallel') {
          const promises = tools.map(tool => tool.execute(params.input));
          const parallelResults = await Promise.all(promises);
          results.push(...parallelResults);
        }
        
        return {
          flow: spec.flow,
          results
        };
      }
    } as ADKTool;
  }
  
  /**
   * Create a tool with retry logic
   */
  async createRetryableTool(spec: ToolSpecification & {
    maxRetries?: number;
    retryDelay?: number;
  }): Promise<ADKTool> {
    const baseTool = await this.createTool(spec);
    
    return {
      name: `${spec.name}_retryable`,
      description: `${spec.description} (with retry logic)`,
      parameters: baseTool.parameters,
      execute: async (params: any) => {
        const maxRetries = spec.maxRetries || 3;
        const retryDelay = spec.retryDelay || 1000;
        
        let lastError: any;
        
        for (let i = 0; i < maxRetries; i++) {
          try {
            return await baseTool.execute(params);
          } catch (error) {
            lastError = error;
            this.logger.warn(`Tool execution failed, retry ${i + 1}/${maxRetries}`, error);
            
            if (i < maxRetries - 1) {
              await new Promise(resolve => setTimeout(resolve, retryDelay * (i + 1)));
            }
          }
        }
        
        throw lastError;
      }
    } as ADKTool;
  }
  
  /**
   * Create a tool with caching
   */
  async createCachedTool(spec: ToolSpecification & {
    ttl?: number;
  }): Promise<ADKTool> {
    const baseTool = await this.createTool(spec);
    const cache = new Map<string, { result: any; timestamp: number }>();
    const ttl = spec.ttl || 60000; // 1 minute default
    
    return {
      name: `${spec.name}_cached`,
      description: `${spec.description} (with caching)`,
      parameters: baseTool.parameters,
      execute: async (params: any) => {
        const cacheKey = JSON.stringify(params);
        const cached = cache.get(cacheKey);
        
        if (cached && Date.now() - cached.timestamp < ttl) {
          this.logger.debug(`Cache hit for tool: ${spec.name}`);
          return cached.result;
        }
        
        const result = await baseTool.execute(params);
        cache.set(cacheKey, { result, timestamp: Date.now() });
        
        return result;
      }
    } as ADKTool;
  }
  
  /**
   * Create a tool with validation
   */
  async createValidatedTool(spec: ToolSpecification & {
    inputValidation?: (params: any) => boolean | string;
    outputValidation?: (result: any) => boolean | string;
  }): Promise<ADKTool> {
    const baseTool = await this.createTool(spec);
    
    return {
      name: `${spec.name}_validated`,
      description: `${spec.description} (with validation)`,
      parameters: baseTool.parameters,
      execute: async (params: any) => {
        // Input validation
        if (spec.inputValidation) {
          const validation = spec.inputValidation(params);
          if (validation !== true) {
            throw new Error(`Input validation failed: ${validation}`);
          }
        }
        
        const result = await baseTool.execute(params);
        
        // Output validation
        if (spec.outputValidation) {
          const validation = spec.outputValidation(result);
          if (validation !== true) {
            throw new Error(`Output validation failed: ${validation}`);
          }
        }
        
        return result;
      }
    } as ADKTool;
  }
  
  /**
   * Register a tool template
   */
  private registerToolTemplate(name: string, config: {
    name: string;
    description: string;
    parameters: any;
    implementation: (params: any) => Promise<any>;
  }): void {
    const tool: ADKTool = {
      name: config.name,
      description: config.description,
      parameters: config.parameters,
      execute: config.implementation
    };
    
    this.toolRegistry.set(name, tool);
  }
  
  /**
   * Get all registered tools
   */
  getRegisteredTools(): Map<string, ADKTool> {
    return new Map(this.toolRegistry);
  }
  
  /**
   * Get tool by name
   */
  getTool(name: string): ADKTool | undefined {
    return this.toolRegistry.get(name);
  }
}