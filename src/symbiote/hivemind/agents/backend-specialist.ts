/**
 * Backend Specialist Agent
 * 
 * Specializes in API development, server-side logic, and database operations
 */

import { BaseSpecialistAgent } from './base-specialist';
import { AgentSpecialty, HivemindTask, TaskResult } from '../types';
import * as path from 'path';
import * as fs from 'fs/promises';

export class BackendSpecialistAgent extends BaseSpecialistAgent {
  constructor(id: string) {
    super(
      id,
      'Backend Specialist',
      AgentSpecialty.Backend,
      [
        'api_development',
        'database_design',
        'authentication',
        'authorization',
        'microservices',
        'rest_api',
        'graphql',
        'websockets',
        'message_queues',
        'caching',
        'performance_optimization',
        'security'
      ]
    );
  }
  
  /**
   * Configure agent with backend-specific prompts
   */
  protected async configureAgent(): Promise<void> {
    if (!this.agent) return;
    
    const systemPrompt = `You are a Backend Development Specialist with expertise in:
- RESTful API design and GraphQL
- Database design (SQL and NoSQL)
- Authentication & Authorization (JWT, OAuth, etc.)
- Microservices architecture
- Message queues (RabbitMQ, Kafka, Redis)
- Caching strategies (Redis, Memcached)
- Performance optimization and scaling
- Security best practices
- Cloud services (AWS, Azure, GCP)
- Container orchestration (Docker, Kubernetes)

When working on backend tasks:
1. Follow RESTful principles and API design best practices
2. Implement proper error handling and validation
3. Ensure security (input validation, SQL injection prevention, etc.)
4. Design efficient database schemas and queries
5. Implement proper logging and monitoring
6. Consider scalability and performance from the start
7. Write clean, maintainable code with proper documentation
8. Include unit and integration tests

Always provide complete, working code that follows the project's existing patterns.`;
    
    // Update agent configuration
    if ('setSystemPrompt' in this.agent) {
      await (this.agent as any).setSystemPrompt(systemPrompt);
    }
  }
  
  /**
   * Perform backend-specific task
   */
  protected async performTask(
    task: HivemindTask,
    memories: any[]
  ): Promise<TaskResult> {
    const result: TaskResult = {
      taskId: task.id,
      success: false,
      output: {},
      filesModified: [],
      filesCreated: [],
      testsAdded: 0,
      issuesFound: [],
      performanceMetrics: {
        duration: 0,
        tokensUsed: 0,
        cost: 0
      }
    };
    
    const startTime = Date.now();
    
    try {
      // Analyze backend structure
      const backendInfo = await this.analyzeBackendStructure(task.context.projectPath!);
      
      // Build context from memories
      const memoryContext = this.buildMemoryContext(memories);
      
      // Create the prompt based on task type
      const prompt = this.buildTaskPrompt(task, backendInfo, memoryContext);
      
      // Execute with agent
      const response = await this.agent!.execute({
        messages: [
          { role: 'user', content: prompt }
        ]
      });
      
      // Parse and apply results
      const taskOutput = await this.parseAndApplyResults(
        response.output,
        task,
        backendInfo
      );
      
      result.success = true;
      result.output = taskOutput;
      result.filesModified = taskOutput.filesModified || [];
      result.filesCreated = taskOutput.filesCreated || [];
      result.testsAdded = taskOutput.testsAdded || 0;
      
      // Check for issues
      if (taskOutput.issues) {
        result.issuesFound = taskOutput.issues;
      }
      
    } catch (error) {
      this.logger.error('Task execution failed', error);
      result.success = false;
      result.output = { error: error.message };
      result.issuesFound.push(`Execution error: ${error.message}`);
    }
    
    result.performanceMetrics.duration = Date.now() - startTime;
    
    return result;
  }
  
  /**
   * Analyze backend project structure
   */
  private async analyzeBackendStructure(projectPath: string): Promise<any> {
    const info: any = {
      framework: 'unknown',
      language: 'javascript',
      hasTypeScript: false,
      database: null,
      orm: null,
      authentication: null,
      testingFramework: null,
      apiStyle: 'rest',
      containerized: false
    };
    
    try {
      // Check package.json for Node.js projects
      const packageJsonPath = path.join(projectPath, 'package.json');
      try {
        const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
        const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
        
        // Detect framework
        if (deps.express) info.framework = 'express';
        else if (deps.koa) info.framework = 'koa';
        else if (deps.fastify) info.framework = 'fastify';
        else if (deps['@nestjs/core']) info.framework = 'nestjs';
        else if (deps.next) info.framework = 'nextjs';
        
        // Detect language
        info.hasTypeScript = !!deps.typescript;
        info.language = info.hasTypeScript ? 'typescript' : 'javascript';
        
        // Detect database
        if (deps.mongoose) {
          info.database = 'mongodb';
          info.orm = 'mongoose';
        } else if (deps.pg || deps['pg-promise']) {
          info.database = 'postgresql';
        } else if (deps.mysql || deps.mysql2) {
          info.database = 'mysql';
        }
        
        // Detect ORM
        if (deps.typeorm) info.orm = 'typeorm';
        else if (deps.sequelize) info.orm = 'sequelize';
        else if (deps.prisma || deps['@prisma/client']) info.orm = 'prisma';
        
        // Detect authentication
        if (deps.passport) info.authentication = 'passport';
        else if (deps.jsonwebtoken) info.authentication = 'jwt';
        else if (deps['express-session']) info.authentication = 'session';
        
        // Detect API style
        if (deps.graphql || deps['apollo-server']) info.apiStyle = 'graphql';
        else if (deps['@grpc/grpc-js']) info.apiStyle = 'grpc';
        
        // Detect testing
        if (deps.jest) info.testingFramework = 'jest';
        else if (deps.mocha) info.testingFramework = 'mocha';
        else if (deps.vitest) info.testingFramework = 'vitest';
        
      } catch {}
      
      // Check for other language indicators
      try {
        await fs.access(path.join(projectPath, 'pom.xml'));
        info.language = 'java';
        info.framework = 'spring';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'requirements.txt'));
        info.language = 'python';
        // Check for Python frameworks
        const reqContent = await fs.readFile(path.join(projectPath, 'requirements.txt'), 'utf8');
        if (reqContent.includes('django')) info.framework = 'django';
        else if (reqContent.includes('flask')) info.framework = 'flask';
        else if (reqContent.includes('fastapi')) info.framework = 'fastapi';
      } catch {}
      
      // Check for Docker
      try {
        await fs.access(path.join(projectPath, 'Dockerfile'));
        info.containerized = true;
      } catch {}
      
    } catch (error) {
      this.logger.warn('Could not analyze backend structure', error);
    }
    
    return info;
  }
  
  /**
   * Build memory context
   */
  private buildMemoryContext(memories: any[]): string {
    if (memories.length === 0) return '';
    
    const relevantMemories = memories
      .filter(m => m.metadata?.success)
      .slice(0, 3)
      .map(m => {
        try {
          const content = JSON.parse(m.content);
          return `- ${content.task.title}: ${content.result.summary}`;
        } catch {
          return null;
        }
      })
      .filter(Boolean);
    
    if (relevantMemories.length === 0) return '';
    
    return `\n\nRelevant past experiences:\n${relevantMemories.join('\n')}`;
  }
  
  /**
   * Build task-specific prompt
   */
  private buildTaskPrompt(
    task: HivemindTask,
    backendInfo: any,
    memoryContext: string
  ): string {
    const basePrompt = `
Project Context:
- Language: ${backendInfo.language}
- Framework: ${backendInfo.framework}
- Database: ${backendInfo.database || 'None detected'}
- ORM: ${backendInfo.orm || 'None'}
- API Style: ${backendInfo.apiStyle}
- Authentication: ${backendInfo.authentication || 'None detected'}
- Containerized: ${backendInfo.containerized ? 'Yes' : 'No'}

Task: ${task.title}
Description: ${task.description}

Files Context:
${task.context.files?.join('\n') || 'No specific files'}

${task.context.codeContext || ''}
${memoryContext}

Please complete this backend task following the project's conventions and best practices.
Provide the complete implementation with clear explanations.
`;
    
    // Add task-specific instructions
    switch (task.type) {
      case 'create_api_endpoint':
        return basePrompt + `
Create the API endpoint with:
- Proper route definition
- Input validation and sanitization
- Error handling (400, 401, 403, 404, 500)
- Database operations if needed
- Response formatting
- API documentation
- Unit and integration tests`;
        
      case 'implement_auth':
        return basePrompt + `
Implement authentication/authorization:
- Secure password handling (if applicable)
- Token generation and validation
- Session management
- Role-based access control
- Security headers
- Rate limiting
- Tests for auth flows`;
        
      case 'optimize_database':
        return basePrompt + `
Optimize database operations:
- Analyze current queries
- Add appropriate indexes
- Optimize query structure
- Implement caching where appropriate
- Add database migrations
- Monitor query performance
- Document changes`;
        
      case 'implement_service':
        return basePrompt + `
Implement the service with:
- Clean service architecture
- Dependency injection
- Error handling and logging
- Transaction management
- Business logic validation
- Unit tests with mocking
- Service documentation`;
        
      case 'fix_bug':
        return basePrompt + `
Fix the backend bug:
- Identify root cause
- Implement proper fix
- Add logging for debugging
- Add tests to prevent regression
- Ensure no side effects
- Update API documentation if needed`;
        
      default:
        return basePrompt;
    }
  }
  
  /**
   * Parse and apply results
   */
  private async parseAndApplyResults(
    output: string,
    task: HivemindTask,
    backendInfo: any
  ): Promise<any> {
    const result = {
      filesModified: [] as string[],
      filesCreated: [] as string[],
      testsAdded: 0,
      issues: [] as string[],
      summary: '',
      apiEndpoints: [] as string[],
      databaseChanges: [] as string[]
    };
    
    try {
      // Extract code blocks
      const codeBlocks = this.extractCodeBlocks(output);
      
      // Apply each code block
      for (const block of codeBlocks) {
        if (block.filename) {
          const filePath = path.isAbsolute(block.filename) 
            ? block.filename 
            : path.join(task.context.projectPath!, block.filename);
          
          try {
            // Check if file exists
            const exists = await this.fileExists(filePath);
            
            if (exists) {
              // Modify existing file
              await fs.writeFile(filePath, block.content, 'utf8');
              result.filesModified.push(filePath);
            } else {
              // Create new file
              await this.ensureDirectoryExists(path.dirname(filePath));
              await fs.writeFile(filePath, block.content, 'utf8');
              result.filesCreated.push(filePath);
              
              // Count tests
              if (block.filename.includes('.test.') || 
                  block.filename.includes('.spec.') ||
                  block.filename.includes('test_')) {
                result.testsAdded++;
              }
            }
            
            // Track API endpoints
            if (block.content.includes('router.') || 
                block.content.includes('app.') ||
                block.content.includes('@Get') ||
                block.content.includes('@Post')) {
              const endpoints = this.extractEndpoints(block.content);
              result.apiEndpoints.push(...endpoints);
            }
            
            // Track database changes
            if (block.filename.includes('migration') ||
                block.filename.includes('schema') ||
                block.content.includes('CREATE TABLE') ||
                block.content.includes('ALTER TABLE')) {
              result.databaseChanges.push(block.filename);
            }
            
          } catch (error) {
            result.issues.push(`Failed to write ${block.filename}: ${error.message}`);
          }
        }
      }
      
      // Extract summary
      result.summary = this.extractSummary(output);
      
    } catch (error) {
      this.logger.error('Failed to parse results', error);
      result.issues.push(`Parse error: ${error.message}`);
    }
    
    return result;
  }
  
  /**
   * Extract API endpoints from code
   */
  private extractEndpoints(code: string): string[] {
    const endpoints: string[] = [];
    
    // Express/Node.js style
    const expressRegex = /(?:router|app)\.(get|post|put|patch|delete)\s*\(\s*['"`]([^'"`]+)['"`]/g;
    let match;
    while ((match = expressRegex.exec(code)) !== null) {
      endpoints.push(`${match[1].toUpperCase()} ${match[2]}`);
    }
    
    // NestJS style
    const nestRegex = /@(Get|Post|Put|Patch|Delete)\s*\(\s*['"`]([^'"`]+)['"`]\s*\)/g;
    while ((match = nestRegex.exec(code)) !== null) {
      endpoints.push(`${match[1].toUpperCase()} ${match[2]}`);
    }
    
    return endpoints;
  }
  
  /**
   * Extract code blocks from output
   */
  private extractCodeBlocks(output: string): Array<{
    filename?: string;
    language: string;
    content: string;
  }> {
    const blocks: any[] = [];
    const codeBlockRegex = /```(\w+)?(?:\s+([^\n]+))?\n([\s\S]*?)```/g;
    
    let match;
    while ((match = codeBlockRegex.exec(output)) !== null) {
      blocks.push({
        language: match[1] || 'text',
        filename: match[2]?.trim(),
        content: match[3].trim()
      });
    }
    
    return blocks;
  }
  
  /**
   * Extract summary from output
   */
  private extractSummary(output: string): string {
    // Remove code blocks
    const withoutCode = output.replace(/```[\s\S]*?```/g, '');
    
    // Get first paragraph or up to 200 characters
    const firstParagraph = withoutCode.split('\n\n')[0];
    
    return firstParagraph.length > 200 
      ? firstParagraph.substring(0, 197) + '...'
      : firstParagraph;
  }
  
  /**
   * Check if file exists
   */
  private async fileExists(filePath: string): Promise<boolean> {
    try {
      await fs.access(filePath);
      return true;
    } catch {
      return false;
    }
  }
  
  /**
   * Ensure directory exists
   */
  private async ensureDirectoryExists(dirPath: string): Promise<void> {
    try {
      await fs.mkdir(dirPath, { recursive: true });
    } catch (error) {
      if (error.code !== 'EEXIST') {
        throw error;
      }
    }
  }
}
