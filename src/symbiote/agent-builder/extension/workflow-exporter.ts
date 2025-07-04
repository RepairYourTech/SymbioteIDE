/**
 * Workflow Exporter
 * 
 * Exports visual workflows to standalone executable code
 */

import {
  AgentWorkflow,
  WorkflowNode,
  WorkflowEdge,
  ExportFormat,
  WorkflowExport,
  AgentNodeConfig,
  ToolNodeConfig,
  ConditionNodeConfig,
  LoopNodeConfig
} from '../types/workflow-types';

export class WorkflowExporter {
  /**
   * Export workflow to specified format
   */
  async export(
    workflow: AgentWorkflow,
    format: ExportFormat,
    options?: any
  ): Promise<WorkflowExport> {
    switch (format) {
      case 'typescript':
        return this.exportToTypeScript(workflow, options);
        
      case 'javascript':
        return this.exportToJavaScript(workflow, options);
        
      case 'python':
        return this.exportToPython(workflow, options);
        
      case 'docker':
        return this.exportToDocker(workflow, options);
        
      case 'serverless':
        return this.exportToServerless(workflow, options);
        
      case 'cli':
        return this.exportToCLI(workflow, options);
        
      default:
        throw new Error(`Unsupported export format: ${format}`);
    }
  }
  
  /**
   * Export to TypeScript
   */
  private async exportToTypeScript(
    workflow: AgentWorkflow,
    options?: any
  ): Promise<WorkflowExport> {
    const className = this.toPascalCase(workflow.name);
    
    const code = `/**
 * ${workflow.name}
 * ${workflow.description || 'Generated workflow agent'}
 * 
 * Generated on ${new Date().toISOString()}
 * Version: ${workflow.version}
 */

import { Agent, AgentFactory, AgentOrchestrator } from '@symbiote/adk';
import { Logger } from '@symbiote/utils';

export interface ${className}Input {
  [key: string]: any;
}

export interface ${className}Output {
  [key: string]: any;
}

export class ${className}Agent {
  private agentFactory: AgentFactory;
  private orchestrator: AgentOrchestrator;
  private logger: Logger;
  
  constructor() {
    this.agentFactory = new AgentFactory();
    this.orchestrator = new AgentOrchestrator(this.agentFactory);
    this.logger = new Logger('${className}Agent');
  }
  
  async execute(input: ${className}Input): Promise<${className}Output> {
    this.logger.info('Starting workflow execution');
    
    try {
      ${this.generateTypeScriptExecutionCode(workflow)}
      
      return output;
    } catch (error) {
      this.logger.error('Workflow execution failed', error);
      throw error;
    }
  }
  
  ${this.generateTypeScriptHelperMethods(workflow)}
}

// Export as default
export default ${className}Agent;

// Export factory function
export function create${className}Agent(): ${className}Agent {
  return new ${className}Agent();
}
`;
    
    return {
      format: 'typescript',
      code,
      dependencies: [
        '@symbiote/adk',
        '@symbiote/utils'
      ],
      runtime: {
        node: '>=16.0.0',
        requirements: []
      },
      metadata: {
        generated: new Date(),
        version: workflow.version,
        checksum: this.generateChecksum(code)
      }
    };
  }
  
  /**
   * Export to JavaScript
   */
  private async exportToJavaScript(
    workflow: AgentWorkflow,
    options?: any
  ): Promise<WorkflowExport> {
    // Similar to TypeScript but without types
    const tsExport = await this.exportToTypeScript(workflow, options);
    
    // Strip TypeScript syntax (simplified)
    const jsCode = tsExport.code
      .replace(/: \w+(\[\])?/g, '') // Remove type annotations
      .replace(/interface \w+ {[^}]+}/g, '') // Remove interfaces
      .replace(/export interface/g, '// interface')
      .replace(/<\w+>/g, ''); // Remove generics
    
    return {
      ...tsExport,
      format: 'javascript',
      code: jsCode
    };
  }
  
  /**
   * Export to Python
   */
  private async exportToPython(
    workflow: AgentWorkflow,
    options?: any
  ): Promise<WorkflowExport> {
    const className = this.toSnakeCase(workflow.name);
    
    const code = `"""
${workflow.name}
${workflow.description || 'Generated workflow agent'}

Generated on ${new Date().toISOString()}
Version: ${workflow.version}
"""

import asyncio
import logging
from typing import Dict, Any
from symbiote_adk import AgentFactory, AgentOrchestrator

logger = logging.getLogger(__name__)

class ${this.toPascalCase(workflow.name)}Agent:
    """Workflow agent implementation"""
    
    def __init__(self):
        self.agent_factory = AgentFactory()
        self.orchestrator = AgentOrchestrator(self.agent_factory)
    
    async def execute(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """Execute the workflow"""
        logger.info("Starting workflow execution")
        
        try:
            ${this.generatePythonExecutionCode(workflow)}
            
            return output
        except Exception as e:
            logger.error(f"Workflow execution failed: {e}")
            raise
    
    ${this.generatePythonHelperMethods(workflow)}

# Usage example
if __name__ == "__main__":
    agent = ${this.toPascalCase(workflow.name)}Agent()
    
    # Example input
    input_data = {
        "example": "data"
    }
    
    # Run the workflow
    result = asyncio.run(agent.execute(input_data))
    print(f"Result: {result}")
`;
    
    return {
      format: 'python',
      code,
      dependencies: [
        'symbiote-adk',
        'asyncio',
        'logging'
      ],
      runtime: {
        python: '>=3.8',
        requirements: ['symbiote-adk>=1.0.0']
      },
      metadata: {
        generated: new Date(),
        version: workflow.version,
        checksum: this.generateChecksum(code)
      }
    };
  }
  
  /**
   * Export to Docker
   */
  private async exportToDocker(
    workflow: AgentWorkflow,
    options?: any
  ): Promise<WorkflowExport> {
    const workflowName = this.toKebabCase(workflow.name);
    
    const dockerfile = `# ${workflow.name} Dockerfile
FROM node:18-alpine

# Install dependencies
RUN apk add --no-cache python3 make g++

# Create app directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy workflow code
COPY . .

# Build TypeScript
RUN npm run build

# Expose port
EXPOSE 3000

# Environment variables
ENV NODE_ENV=production
ENV WORKFLOW_NAME=${workflowName}

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s \\
  CMD node healthcheck.js

# Run the workflow
CMD ["node", "dist/index.js"]
`;
    
    const dockerCompose = `version: '3.8'

services:
  ${workflowName}:
    build: .
    container_name: ${workflowName}
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
      - REDIS_URL=redis://redis:6379
    depends_on:
      - redis
    networks:
      - workflow-network

  redis:
    image: redis:7-alpine
    container_name: ${workflowName}-redis
    restart: unless-stopped
    networks:
      - workflow-network

networks:
  workflow-network:
    driver: bridge
`;
    
    return {
      format: 'docker',
      code: dockerfile + '\n\n---\n# docker-compose.yml\n' + dockerCompose,
      dependencies: [],
      runtime: {
        docker: '>=20.10',
        requirements: []
      },
      metadata: {
        generated: new Date(),
        version: workflow.version,
        checksum: this.generateChecksum(dockerfile)
      }
    };
  }
  
  /**
   * Export to Serverless (AWS Lambda)
   */
  private async exportToServerless(
    workflow: AgentWorkflow,
    options?: any
  ): Promise<WorkflowExport> {
    const functionName = this.toKebabCase(workflow.name);
    
    const serverlessYml = `service: ${functionName}

provider:
  name: aws
  runtime: nodejs18.x
  stage: \${opt:stage, 'dev'}
  region: \${opt:region, 'us-east-1'}
  memorySize: 1024
  timeout: 300

functions:
  ${functionName}:
    handler: handler.execute
    events:
      - http:
          path: /execute
          method: POST
          cors: true
      - eventBridge:
          schedule: rate(5 minutes)
    environment:
      NODE_ENV: production
      WORKFLOW_NAME: ${workflow.name}

plugins:
  - serverless-plugin-typescript
  - serverless-offline

custom:
  serverless-offline:
    httpPort: 3000
`;
    
    const handlerCode = `import { APIGatewayProxyHandler } from 'aws-lambda';
import { ${this.toPascalCase(workflow.name)}Agent } from './workflow';

const agent = new ${this.toPascalCase(workflow.name)}Agent();

export const execute: APIGatewayProxyHandler = async (event) => {
  try {
    const input = JSON.parse(event.body || '{}');
    const result = await agent.execute(input);
    
    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*'
      },
      body: JSON.stringify({
        success: true,
        result
      })
    };
  } catch (error) {
    console.error('Execution failed:', error);
    
    return {
      statusCode: 500,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*'
      },
      body: JSON.stringify({
        success: false,
        error: error.message
      })
    };
  }
};
`;
    
    return {
      format: 'serverless',
      code: serverlessYml + '\n\n---\n// handler.ts\n' + handlerCode,
      dependencies: [
        'aws-lambda',
        'serverless',
        'serverless-plugin-typescript',
        'serverless-offline'
      ],
      runtime: {
        node: '>=18.0.0',
        requirements: []
      },
      metadata: {
        generated: new Date(),
        version: workflow.version,
        checksum: this.generateChecksum(serverlessYml)
      }
    };
  }
  
  /**
   * Export to CLI
   */
  private async exportToCLI(
    workflow: AgentWorkflow,
    options?: any
  ): Promise<WorkflowExport> {
    const cliName = this.toKebabCase(workflow.name);
    
    const cliCode = `#!/usr/bin/env node
/**
 * ${workflow.name} CLI
 * ${workflow.description || 'Generated workflow CLI'}
 */

import { Command } from 'commander';
import { ${this.toPascalCase(workflow.name)}Agent } from './workflow';
import chalk from 'chalk';
import ora from 'ora';

const program = new Command();
const agent = new ${this.toPascalCase(workflow.name)}Agent();

program
  .name('${cliName}')
  .description('${workflow.description || 'Execute workflow from command line'}')
  .version('${workflow.version}')
  .option('-i, --input <json>', 'Input data as JSON string')
  .option('-f, --file <path>', 'Input data from JSON file')
  .option('-o, --output <path>', 'Output results to file')
  .option('-v, --verbose', 'Verbose output')
  .action(async (options) => {
    const spinner = ora('Executing workflow...').start();
    
    try {
      // Parse input
      let input = {};
      
      if (options.input) {
        input = JSON.parse(options.input);
      } else if (options.file) {
        const fs = require('fs');
        input = JSON.parse(fs.readFileSync(options.file, 'utf-8'));
      }
      
      // Execute workflow
      const result = await agent.execute(input);
      
      spinner.succeed(chalk.green('Workflow executed successfully'));
      
      // Output results
      if (options.output) {
        const fs = require('fs');
        fs.writeFileSync(options.output, JSON.stringify(result, null, 2));
        console.log(chalk.blue(\`Results saved to \${options.output}\`));
      } else {
        console.log(chalk.cyan('\\nResults:'));
        console.log(JSON.stringify(result, null, 2));
      }
      
    } catch (error) {
      spinner.fail(chalk.red('Workflow execution failed'));
      console.error(chalk.red(error.message));
      
      if (options.verbose) {
        console.error(error.stack);
      }
      
      process.exit(1);
    }
  });

program.parse(process.argv);
`;
    
    const packageJson = `{
  "name": "${cliName}-cli",
  "version": "${workflow.version}",
  "description": "${workflow.description || 'Workflow CLI'}",
  "main": "dist/index.js",
  "bin": {
    "${cliName}": "./dist/cli.js"
  },
  "scripts": {
    "build": "tsc",
    "start": "node dist/cli.js",
    "dev": "ts-node src/cli.ts"
  },
  "dependencies": {
    "commander": "^11.0.0",
    "chalk": "^5.3.0",
    "ora": "^7.0.1",
    "@symbiote/adk": "^1.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }
}`;
    
    return {
      format: 'cli',
      code: cliCode + '\n\n---\n// package.json\n' + packageJson,
      dependencies: [
        'commander',
        'chalk',
        'ora',
        '@symbiote/adk'
      ],
      runtime: {
        node: '>=16.0.0',
        requirements: []
      },
      metadata: {
        generated: new Date(),
        version: workflow.version,
        checksum: this.generateChecksum(cliCode)
      }
    };
  }
  
  /**
   * Generate TypeScript execution code
   */
  private generateTypeScriptExecutionCode(workflow: AgentWorkflow): string {
    const lines: string[] = [];
    
    // Create context
    lines.push('const context = new Map<string, any>();');
    lines.push('context.set("$input", input);');
    lines.push('');
    
    // Add variables
    workflow.variables.forEach(v => {
      lines.push(`context.set("${v.name}", ${JSON.stringify(v.value)});`);
    });
    lines.push('');
    
    // Generate node execution code
    // This is simplified - real implementation would handle graph traversal
    workflow.nodes.forEach(node => {
      lines.push(`// Execute ${node.data.label}`);
      
      switch (node.type) {
        case 'agent':
          const agentConfig = node.data.config as AgentNodeConfig;
          lines.push(`const ${node.id}_agent = await this.agentFactory.createSpecializedAgent('${agentConfig.agentType}');`);
          lines.push(`const ${node.id}_result = await ${node.id}_agent.execute(context.get("$input"));`);
          lines.push(`context.set("${node.id}", ${node.id}_result);`);
          break;
          
        case 'condition':
          const condConfig = node.data.config as ConditionNodeConfig;
          lines.push(`const ${node.id}_condition = ${condConfig.expression};`);
          lines.push(`if (${node.id}_condition) {`);
          lines.push(`  // True branch`);
          lines.push(`} else {`);
          lines.push(`  // False branch`);
          lines.push(`}`);
          break;
          
        default:
          lines.push(`// TODO: Implement ${node.type} node`);
      }
      
      lines.push('');
    });
    
    // Return output
    lines.push('const output = context.get("$output") || context.get("$input");');
    
    return lines.join('\n      ');
  }
  
  /**
   * Generate TypeScript helper methods
   */
  private generateTypeScriptHelperMethods(workflow: AgentWorkflow): string {
    return `
  private async createAgent(type: string): Promise<Agent> {
    return await this.agentFactory.createSpecializedAgent(type);
  }
  
  private evaluateCondition(expression: string, context: any): boolean {
    try {
      const func = new Function('context', \`return \${expression}\`);
      return func(context);
    } catch (error) {
      this.logger.error('Failed to evaluate condition', error);
      return false;
    }
  }`;
  }
  
  /**
   * Generate Python execution code
   */
  private generatePythonExecutionCode(workflow: AgentWorkflow): string {
    const lines: string[] = [];
    
    lines.push('context = {"$input": input_data}');
    lines.push('');
    
    workflow.nodes.forEach(node => {
      lines.push(f`# Execute ${node.data.label}`);
      
      switch (node.type) {
        case 'agent':
          const agentConfig = node.data.config as AgentNodeConfig;
          lines.push(`${node.id}_agent = await self.agent_factory.create_specialized_agent("${agentConfig.agentType}")`);
          lines.push(`${node.id}_result = await ${node.id}_agent.execute(context["$input"])`);
          lines.push(`context["${node.id}"] = ${node.id}_result`);
          break;
          
        default:
          lines.push(`# TODO: Implement ${node.type} node`);
      }
      
      lines.push('');
    });
    
    lines.push('output = context.get("$output", context["$input"])');
    
    return lines.map(line => '        ' + line).join('\n');
  }
  
  /**
   * Generate Python helper methods
   */
  private generatePythonHelperMethods(workflow: AgentWorkflow): string {
    return `
    async def create_agent(self, agent_type: str):
        """Create an agent by type"""
        return await self.agent_factory.create_specialized_agent(agent_type)
    
    def evaluate_condition(self, expression: str, context: Dict[str, Any]) -> bool:
        """Evaluate a condition expression"""
        try:
            return eval(expression, {"context": context})
        except Exception as e:
            logger.error(f"Failed to evaluate condition: {e}")
            return False`;
  }
  
  // Utility methods
  
  private toPascalCase(str: string): string {
    return str
      .replace(/[-_\s]+(.)?/g, (_, c) => c ? c.toUpperCase() : '')
      .replace(/^(.)/, (_, c) => c.toUpperCase());
  }
  
  private toSnakeCase(str: string): string {
    return str
      .replace(/\s+/g, '_')
      .replace(/([A-Z])/g, '_$1')
      .toLowerCase()
      .replace(/^_/, '');
  }
  
  private toKebabCase(str: string): string {
    return str
      .replace(/\s+/g, '-')
      .replace(/([A-Z])/g, '-$1')
      .toLowerCase()
      .replace(/^-/, '');
  }
  
  private generateChecksum(content: string): string {
    // Simple checksum - in production use crypto
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      const char = content.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash).toString(16);
  }
  
  private f(str: string): string {
    return str; // Python f-string placeholder
  }
}