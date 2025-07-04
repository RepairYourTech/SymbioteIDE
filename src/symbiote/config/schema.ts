/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

export interface SymbioteConfiguration {
  version: string;
  ai: AIConfiguration;
  agents: AgentConfiguration;
  mcp: MCPConfiguration;
  memory: MemoryConfiguration;
  knowledge: KnowledgeConfiguration;
  terminal: TerminalConfiguration;
}

export interface AIConfiguration {
  orchestration: {
    enabled: boolean;
    defaultModel: string;
    modelRouting: ModelRoutingConfig[];
    fallbackChain: string[];
    maxRetries: number;
    timeout: number;
  };
  providers: AIProviderConfig[];
  costLimits: {
    perSession: number;
    perDay: number;
    perMonth: number;
    warningThreshold: number;
  };
}

export interface ModelRoutingConfig {
  taskType: 'code-generation' | 'code-review' | 'documentation' | 'debugging' | 'testing' | 'general';
  preferredModels: string[];
  requiredCapabilities?: string[];
  maxCost?: number;
}

export interface AIProviderConfig {
  id: string;
  name: string;
  type: 'openai' | 'anthropic' | 'google' | 'local' | 'custom';
  apiKeySource: 'env' | 'keychain' | 'config';
  apiKeyName?: string;
  baseUrl?: string;
  models: AIModelConfig[];
  enabled: boolean;
}

export interface AIModelConfig {
  id: string;
  name: string;
  capabilities: string[];
  contextWindow: number;
  costPer1kTokens: {
    input: number;
    output: number;
  };
  maxTokens: number;
  temperature?: number;
  topP?: number;
}

export interface AgentConfiguration {
  enabled: boolean;
  workspace: string;
  templates: string;
  maxConcurrentAgents: number;
  defaultTimeout: number;
  sandboxing: {
    enabled: boolean;
    isolationLevel: 'none' | 'process' | 'container';
    resourceLimits: {
      memory: string;
      cpu: number;
      diskSpace: string;
    };
  };
  marketplace: {
    enabled: boolean;
    trustedPublishers: string[];
    autoUpdate: boolean;
  };
}

export interface MCPConfiguration {
  enabled: boolean;
  servers: MCPServerConfig[];
  security: {
    allowedTools: string[];
    blockedTools: string[];
    requireSignature: boolean;
    sandboxLevel: 'none' | 'restricted' | 'strict';
  };
  communication: {
    protocol: 'stdio' | 'websocket' | 'http';
    timeout: number;
    maxMessageSize: number;
  };
}

export interface MCPServerConfig {
  id: string;
  name: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  workingDirectory?: string;
  autoStart: boolean;
  permissions: MCPPermissions;
}

export interface MCPPermissions {
  fileSystem: {
    read: string[];
    write: string[];
    execute: string[];
  };
  network: {
    allowed: string[];
    blocked: string[];
  };
  system: {
    allowSpawn: boolean;
    allowEnvAccess: boolean;
    allowSystemInfo: boolean;
  };
}

export interface MemoryConfiguration {
  provider: 'mem0' | 'local' | 'custom';
  persistence: {
    enabled: boolean;
    location: string;
    encryption: boolean;
    maxSize: string;
  };
  sync: {
    enabled: boolean;
    interval: number;
    conflictResolution: 'latest' | 'merge' | 'manual';
  };
  privacy: {
    shareAcrossProjects: boolean;
    shareAcrossWorkspaces: boolean;
    anonymizeData: boolean;
    retentionDays: number;
  };
}

export interface KnowledgeConfiguration {
  neo4j: {
    enabled: boolean;
    uri: string;
    auth: {
      type: 'basic' | 'bearer' | 'custom';
      credentials: string;
    };
    indexing: {
      automatic: boolean;
      languages: string[];
      excludePaths: string[];
    };
  };
  qdrant: {
    enabled: boolean;
    url: string;
    apiKey?: string;
    collection: string;
    embedding: {
      model: string;
      dimensions: number;
      batchSize: number;
    };
  };
  search: {
    hybrid: boolean;
    reranking: boolean;
    maxResults: number;
    minScore: number;
  };
}

export interface TerminalConfiguration {
  ai: {
    enabled: boolean;
    naturalLanguage: boolean;
    commandSuggestions: boolean;
    errorDiagnosis: boolean;
    autoFix: boolean;
  };
  rendering: {
    gpu: boolean;
    fontLigatures: boolean;
    theme: string;
  };
  shell: {
    default: string;
    env: Record<string, string>;
  };
}

// Configuration validation
export function validateConfiguration(config: Partial<SymbioteConfiguration>): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Validate AI configuration
  if (config.ai) {
    if (!config.ai.providers || config.ai.providers.length === 0) {
      errors.push('At least one AI provider must be configured');
    }
    
    if (config.ai.orchestration?.enabled && !config.ai.orchestration.defaultModel) {
      errors.push('Default model must be specified when orchestration is enabled');
    }
  }

  // Validate MCP configuration
  if (config.mcp?.enabled && (!config.mcp.servers || config.mcp.servers.length === 0)) {
    warnings.push('MCP is enabled but no servers are configured');
  }

  // Validate memory configuration
  if (config.memory?.persistence?.enabled && !config.memory.persistence.location) {
    errors.push('Persistence location must be specified when enabled');
  }

  // Validate knowledge configuration
  if (config.knowledge) {
    if (config.knowledge.neo4j?.enabled && !config.knowledge.neo4j.uri) {
      errors.push('Neo4j URI must be specified when enabled');
    }
    
    if (config.knowledge.qdrant?.enabled && !config.knowledge.qdrant.url) {
      errors.push('Qdrant URL must be specified when enabled');
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings
  };
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// Default configuration
export const defaultConfiguration: SymbioteConfiguration = {
  version: '1.0.0',
  ai: {
    orchestration: {
      enabled: true,
      defaultModel: 'gpt-4',
      modelRouting: [
        {
          taskType: 'code-generation',
          preferredModels: ['claude-3-opus', 'gpt-4'],
          requiredCapabilities: ['code-generation', 'long-context']
        },
        {
          taskType: 'code-review',
          preferredModels: ['claude-3-sonnet', 'gpt-4'],
          requiredCapabilities: ['code-analysis']
        }
      ],
      fallbackChain: ['gpt-4', 'claude-3-sonnet', 'gpt-3.5-turbo'],
      maxRetries: 3,
      timeout: 60000
    },
    providers: [],
    costLimits: {
      perSession: 10,
      perDay: 50,
      perMonth: 500,
      warningThreshold: 0.8
    }
  },
  agents: {
    enabled: true,
    workspace: '.symbiote-agents',
    templates: 'templates',
    maxConcurrentAgents: 5,
    defaultTimeout: 300000,
    sandboxing: {
      enabled: true,
      isolationLevel: 'process',
      resourceLimits: {
        memory: '1GB',
        cpu: 2,
        diskSpace: '5GB'
      }
    },
    marketplace: {
      enabled: true,
      trustedPublishers: ['symbiote-ide', 'microsoft', 'anthropic'],
      autoUpdate: false
    }
  },
  mcp: {
    enabled: true,
    servers: [],
    security: {
      allowedTools: ['*'],
      blockedTools: [],
      requireSignature: false,
      sandboxLevel: 'restricted'
    },
    communication: {
      protocol: 'stdio',
      timeout: 30000,
      maxMessageSize: 10485760 // 10MB
    }
  },
  memory: {
    provider: 'mem0',
    persistence: {
      enabled: true,
      location: '.symbiote-memory',
      encryption: true,
      maxSize: '1GB'
    },
    sync: {
      enabled: true,
      interval: 300000, // 5 minutes
      conflictResolution: 'latest'
    },
    privacy: {
      shareAcrossProjects: false,
      shareAcrossWorkspaces: false,
      anonymizeData: true,
      retentionDays: 90
    }
  },
  knowledge: {
    neo4j: {
      enabled: false,
      uri: 'bolt://localhost:7687',
      auth: {
        type: 'basic',
        credentials: 'env:NEO4J_AUTH'
      },
      indexing: {
        automatic: true,
        languages: ['javascript', 'typescript', 'python', 'java'],
        excludePaths: ['node_modules', '.git', 'dist', 'build']
      }
    },
    qdrant: {
      enabled: false,
      url: 'http://localhost:6333',
      collection: 'symbiote-code',
      embedding: {
        model: 'text-embedding-ada-002',
        dimensions: 1536,
        batchSize: 100
      }
    },
    search: {
      hybrid: true,
      reranking: true,
      maxResults: 20,
      minScore: 0.7
    }
  },
  terminal: {
    ai: {
      enabled: true,
      naturalLanguage: true,
      commandSuggestions: true,
      errorDiagnosis: true,
      autoFix: false
    },
    rendering: {
      gpu: true,
      fontLigatures: true,
      theme: 'symbiote-dark'
    },
    shell: {
      default: process.platform === 'win32' ? 'powershell' : 'bash',
      env: {}
    }
  }
};