/**
 * MCP Context Server
 * 
 * Exposes the unified Neo4j + Qdrant + Mem0 context system as MCP tools
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ErrorCode,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  McpError,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { Logger } from '../../utils/logger';
import { UnifiedContextAPI } from '../../memory/unified-context-api';
import { MemoryManager } from '../../memory/mem0/memory-manager';
import { QdrantManager } from '../../search/qdrant/qdrant-manager';
import { Neo4jConnectionManager } from '../../knowledge/neo4j/connection-manager';
import { RedisManager } from '../../redis';
import { Mem0Client } from '../../memory/mem0/client';
import { Neo4jService } from '../../memory/mem0/neo4j-service';
import {
  Memory,
  MemoryType,
  MemoryScope,
  MemorySearchRequest,
  TaskContext,
  Learning
} from '../../memory/mem0/types';

export interface ContextServerConfig {
  name?: string;
  version?: string;
  redisConfig?: any;
  qdrantConfig?: any;
  neo4jConfig?: any;
  mem0Config?: any;
}

export class MCPContextServer {
  private server: Server;
  private unifiedAPI?: UnifiedContextAPI;
  private logger = new Logger('MCPContextServer');
  private config: ContextServerConfig;
  
  constructor(config: ContextServerConfig = {}) {
    this.config = {
      name: config.name || 'symbiote-context',
      version: config.version || '1.0.0',
      ...config
    };
    
    this.server = new Server(
      {
        name: this.config.name,
        version: this.config.version,
      },
      {
        capabilities: {
          resources: {},
          tools: {}
        }
      }
    );
    
    this.setupHandlers();
  }
  
  /**
   * Initialize the context systems
   */
  private async initializeContextSystems(): Promise<void> {
    try {
      // Initialize Redis
      const redisManager = new RedisManager(this.config.redisConfig);
      await redisManager.connect();
      
      // Initialize Qdrant
      const qdrantManager = new QdrantManager(this.config.qdrantConfig);
      await qdrantManager.connect();
      
      // Initialize Neo4j
      const neo4jManager = new Neo4jConnectionManager(this.config.neo4jConfig);
      const neo4jService = new Neo4jService(neo4jManager);
      
      // Initialize Mem0
      const mem0Client = new Mem0Client(this.config.mem0Config);
      await mem0Client.connect();
      
      // Create memory manager
      const memoryManager = new MemoryManager(
        mem0Client,
        qdrantManager,
        neo4jService
      );
      await memoryManager.initialize();
      
      // Create unified API
      this.unifiedAPI = new UnifiedContextAPI({
        memoryManager,
        qdrantManager,
        neo4jManager,
        redisManager,
        enableRealtime: true
      });
      
      await this.unifiedAPI.initialize();
      
      this.logger.info('Context systems initialized');
      
    } catch (error) {
      this.logger.error('Failed to initialize context systems', error);
      throw error;
    }
  }
  
  /**
   * Set up MCP handlers
   */
  private setupHandlers(): void {
    // List available tools
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: [
        {
          name: 'search_context',
          description: 'Search for relevant context across code, memories, and knowledge graph',
          inputSchema: {
            type: 'object',
            properties: {
              query: {
                type: 'string',
                description: 'Search query or task description'
              },
              includeCode: {
                type: 'boolean',
                description: 'Include code search results (default: true)'
              },
              includeMemories: {
                type: 'boolean',
                description: 'Include memory search results (default: true)'
              },
              includeGraph: {
                type: 'boolean',
                description: 'Include knowledge graph insights (default: true)'
              },
              limit: {
                type: 'number',
                description: 'Maximum number of results per source'
              }
            },
            required: ['query']
          }
        },
        {
          name: 'store_memory',
          description: 'Store a memory for future reference',
          inputSchema: {
            type: 'object',
            properties: {
              content: {
                type: 'string',
                description: 'Memory content'
              },
              type: {
                type: 'string',
                enum: ['episodic', 'semantic', 'working', 'long_term'],
                description: 'Memory type'
              },
              scope: {
                type: 'string',
                enum: ['agent', 'project', 'team', 'global'],
                description: 'Memory scope'
              },
              metadata: {
                type: 'object',
                description: 'Additional metadata'
              }
            },
            required: ['content', 'type']
          }
        },
        {
          name: 'search_memories',
          description: 'Search for specific memories',
          inputSchema: {
            type: 'object',
            properties: {
              query: {
                type: 'string',
                description: 'Search query'
              },
              type: {
                type: 'array',
                items: {
                  type: 'string',
                  enum: ['episodic', 'semantic', 'working', 'long_term']
                },
                description: 'Filter by memory types'
              },
              limit: {
                type: 'number',
                description: 'Maximum results'
              }
            }
          }
        },
        {
          name: 'learn_from_task',
          description: 'Record learning from task execution',
          inputSchema: {
            type: 'object',
            properties: {
              taskDescription: {
                type: 'string',
                description: 'Description of the task'
              },
              outcome: {
                type: 'string',
                enum: ['success', 'failure'],
                description: 'Task outcome'
              },
              solution: {
                type: 'string',
                description: 'Solution (if successful)'
              },
              error: {
                type: 'string',
                description: 'Error message (if failed)'
              },
              duration: {
                type: 'number',
                description: 'Task duration in milliseconds'
              },
              filesModified: {
                type: 'array',
                items: { type: 'string' },
                description: 'Files that were modified'
              }
            },
            required: ['taskDescription', 'outcome', 'duration']
          }
        },
        {
          name: 'find_similar_code',
          description: 'Find similar code implementations',
          inputSchema: {
            type: 'object',
            properties: {
              code: {
                type: 'string',
                description: 'Code snippet to find similar implementations for'
              },
              language: {
                type: 'string',
                description: 'Programming language'
              },
              limit: {
                type: 'number',
                description: 'Maximum results'
              }
            },
            required: ['code']
          }
        },
        {
          name: 'get_file_context',
          description: 'Get comprehensive context for a specific file',
          inputSchema: {
            type: 'object',
            properties: {
              filePath: {
                type: 'string',
                description: 'Path to the file'
              }
            },
            required: ['filePath']
          }
        },
        {
          name: 'get_team_knowledge',
          description: 'Get shared team knowledge and best practices',
          inputSchema: {
            type: 'object',
            properties: {
              teamId: {
                type: 'string',
                description: 'Team identifier (optional)'
              }
            }
          }
        },
        {
          name: 'share_memory',
          description: 'Share a memory with the team',
          inputSchema: {
            type: 'object',
            properties: {
              memoryId: {
                type: 'string',
                description: 'ID of memory to share'
              },
              teamId: {
                type: 'string',
                description: 'Team to share with (optional)'
              }
            },
            required: ['memoryId']
          }
        }
      ]
    }));
    
    // Handle tool calls
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      if (!this.unifiedAPI) {
        await this.initializeContextSystems();
      }
      
      const { name, arguments: args } = request.params;
      
      try {
        switch (name) {
          case 'search_context':
            return await this.handleSearchContext(args);
            
          case 'store_memory':
            return await this.handleStoreMemory(args);
            
          case 'search_memories':
            return await this.handleSearchMemories(args);
            
          case 'learn_from_task':
            return await this.handleLearnFromTask(args);
            
          case 'find_similar_code':
            return await this.handleFindSimilarCode(args);
            
          case 'get_file_context':
            return await this.handleGetFileContext(args);
            
          case 'get_team_knowledge':
            return await this.handleGetTeamKnowledge(args);
            
          case 'share_memory':
            return await this.handleShareMemory(args);
            
          default:
            throw new McpError(
              ErrorCode.MethodNotFound,
              `Unknown tool: ${name}`
            );
        }
      } catch (error: any) {
        this.logger.error(`Tool execution failed: ${name}`, error);
        throw new McpError(
          ErrorCode.InternalError,
          error.message || 'Tool execution failed'
        );
      }
    });
    
    // List resources (memories, patterns, etc.)
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => ({
      resources: [
        {
          uri: 'memory://recent',
          name: 'Recent Memories',
          description: 'Recently accessed memories',
          mimeType: 'application/json'
        },
        {
          uri: 'memory://patterns',
          name: 'Code Patterns',
          description: 'Learned code patterns',
          mimeType: 'application/json'
        },
        {
          uri: 'memory://team-knowledge',
          name: 'Team Knowledge',
          description: 'Shared team knowledge and best practices',
          mimeType: 'application/json'
        }
      ]
    }));
    
    // Read resources
    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;
      
      if (!this.unifiedAPI) {
        await this.initializeContextSystems();
      }
      
      try {
        switch (uri) {
          case 'memory://recent':
            return await this.getRecentMemories();
            
          case 'memory://patterns':
            return await this.getCodePatterns();
            
          case 'memory://team-knowledge':
            return await this.getTeamKnowledgeResource();
            
          default:
            throw new McpError(
              ErrorCode.InvalidRequest,
              `Unknown resource: ${uri}`
            );
        }
      } catch (error: any) {
        throw new McpError(
          ErrorCode.InternalError,
          error.message || 'Resource read failed'
        );
      }
    });
  }
  
  /**
   * Start the server
   */
  async run(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    this.logger.info('MCP Context Server started');
  }
  
  // Tool handlers
  
  private async handleSearchContext(args: any): Promise<any> {
    const result = await this.unifiedAPI!.search({
      query: args.query,
      includeCode: args.includeCode !== false,
      includeMemories: args.includeMemories !== false,
      includeGraph: args.includeGraph !== false,
      limit: args.limit
    });
    
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(result, null, 2)
        }
      ]
    };
  }
  
  private async handleStoreMemory(args: any): Promise<any> {
    const memory: Memory = {
      id: `mem_${Date.now()}`,
      type: args.type as MemoryType || MemoryType.Semantic,
      scope: args.scope as MemoryScope || MemoryScope.Agent,
      content: args.content,
      metadata: {
        agentId: 'mcp-client',
        confidence: 0.8,
        importance: 0.7,
        shareable: true,
        ...args.metadata
      },
      createdAt: new Date(),
      updatedAt: new Date(),
      accessedAt: new Date(),
      accessCount: 0
    };
    
    const id = await this.unifiedAPI!['memoryManager'].store(memory);
    
    return {
      content: [
        {
          type: 'text',
          text: `Memory stored with ID: ${id}`
        }
      ]
    };
  }
  
  private async handleSearchMemories(args: any): Promise<any> {
    const request: MemorySearchRequest = {
      query: args.query,
      type: args.type,
      limit: args.limit || 10
    };
    
    const results = await this.unifiedAPI!['memoryManager'].search(request);
    
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(results, null, 2)
        }
      ]
    };
  }
  
  private async handleLearnFromTask(args: any): Promise<any> {
    const context: TaskContext = {
      taskId: `task_${Date.now()}`,
      description: args.taskDescription,
      type: 'development'
    };
    
    const learnings = await this.unifiedAPI!.learn({
      task: context,
      outcome: args.outcome,
      solution: args.solution,
      error: args.error,
      duration: args.duration,
      filesModified: args.filesModified
    });
    
    return {
      content: [
        {
          type: 'text',
          text: `Stored ${learnings.length} learnings from task execution`
        }
      ]
    };
  }
  
  private async handleFindSimilarCode(args: any): Promise<any> {
    const result = await this.unifiedAPI!.findSimilarImplementations(
      args.code,
      args.language
    );
    
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(result, null, 2)
        }
      ]
    };
  }
  
  private async handleGetFileContext(args: any): Promise<any> {
    const result = await this.unifiedAPI!.getFileContext(args.filePath);
    
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(result, null, 2)
        }
      ]
    };
  }
  
  private async handleGetTeamKnowledge(args: any): Promise<any> {
    const result = await this.unifiedAPI!.getTeamKnowledge(args.teamId);
    
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(result, null, 2)
        }
      ]
    };
  }
  
  private async handleShareMemory(args: any): Promise<any> {
    await this.unifiedAPI!.shareMemory(args.memoryId, args.teamId);
    
    return {
      content: [
        {
          type: 'text',
          text: `Memory ${args.memoryId} shared with team`
        }
      ]
    };
  }
  
  // Resource handlers
  
  private async getRecentMemories(): Promise<any> {
    const memories = await this.unifiedAPI!['memoryManager'].search({
      limit: 20
    });
    
    return {
      contents: [
        {
          uri: 'memory://recent',
          mimeType: 'application/json',
          text: JSON.stringify(memories, null, 2)
        }
      ]
    };
  }
  
  private async getCodePatterns(): Promise<any> {
    // This would fetch learned patterns
    const patterns = [];
    
    return {
      contents: [
        {
          uri: 'memory://patterns',
          mimeType: 'application/json',
          text: JSON.stringify(patterns, null, 2)
        }
      ]
    };
  }
  
  private async getTeamKnowledgeResource(): Promise<any> {
    const knowledge = await this.unifiedAPI!.getTeamKnowledge();
    
    return {
      contents: [
        {
          uri: 'memory://team-knowledge',
          mimeType: 'application/json',
          text: JSON.stringify(knowledge, null, 2)
        }
      ]
    };
  }
}

// Main entry point
if (require.main === module) {
  const server = new MCPContextServer({
    redisConfig: {
      host: process.env.REDIS_HOST || 'localhost',
      port: parseInt(process.env.REDIS_PORT || '6379')
    },
    qdrantConfig: {
      host: process.env.QDRANT_HOST || 'localhost',
      port: parseInt(process.env.QDRANT_PORT || '6333')
    },
    neo4jConfig: {
      globalGraphConfig: {
        context: 'global',
        uri: process.env.NEO4J_URI || 'bolt://localhost:7687',
        username: process.env.NEO4J_USERNAME || 'neo4j',
        password: process.env.NEO4J_PASSWORD || 'password'
      }
    },
    mem0Config: {
      apiKey: process.env.MEM0_API_KEY,
      baseUrl: process.env.MEM0_BASE_URL
    }
  });
  
  server.run().catch(console.error);
}