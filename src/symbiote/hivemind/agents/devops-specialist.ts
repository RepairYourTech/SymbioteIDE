/**
 * DevOps Specialist Agent
 * 
 * Specializes in CI/CD, containerization, infrastructure, and deployment
 */

import { BaseSpecialistAgent } from './base-specialist';
import { AgentSpecialty, HivemindTask, TaskResult } from '../types';
import * as path from 'path';
import * as fs from 'fs/promises';

export class DevOpsSpecialistAgent extends BaseSpecialistAgent {
  constructor(id: string) {
    super(
      id,
      'DevOps Specialist',
      AgentSpecialty.DevOps,
      [
        'ci_cd_pipeline',
        'containerization',
        'kubernetes',
        'infrastructure_as_code',
        'deployment_automation',
        'monitoring_setup',
        'logging_configuration',
        'cloud_services',
        'database_setup',
        'load_balancing',
        'auto_scaling',
        'backup_recovery'
      ]
    );
  }
  
  /**
   * Configure agent with DevOps-specific prompts
   */
  protected async configureAgent(): Promise<void> {
    if (!this.agent) return;
    
    const systemPrompt = `You are a DevOps Specialist with expertise in:
- CI/CD pipelines (Jenkins, GitHub Actions, GitLab CI, CircleCI)
- Containerization (Docker, Podman)
- Container orchestration (Kubernetes, Docker Swarm)
- Infrastructure as Code (Terraform, CloudFormation, Pulumi)
- Cloud platforms (AWS, Azure, GCP)
- Configuration management (Ansible, Chef, Puppet)
- Monitoring and observability (Prometheus, Grafana, ELK stack)
- Version control and branching strategies
- Database management and scaling
- Security best practices for infrastructure
- Disaster recovery and backup strategies

When working on DevOps tasks:
1. Follow infrastructure as code principles
2. Implement proper monitoring and alerting
3. Ensure high availability and scalability
4. Use secure secrets management
5. Implement proper logging and observability
6. Create reproducible environments
7. Document all configurations and processes
8. Include automated tests for infrastructure

Always provide complete, working configurations with clear explanations.`;
    
    // Update agent configuration
    if ('setSystemPrompt' in this.agent) {
      await (this.agent as any).setSystemPrompt(systemPrompt);
    }
  }
  
  /**
   * Perform DevOps-specific task
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
      // Analyze infrastructure context
      const infraInfo = await this.analyzeInfrastructure(task.context.projectPath!);
      
      // Build context from memories
      const memoryContext = this.buildMemoryContext(memories);
      
      // Create the prompt based on task type
      const prompt = this.buildTaskPrompt(task, infraInfo, memoryContext);
      
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
        infraInfo
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
   * Analyze infrastructure
   */
  private async analyzeInfrastructure(projectPath: string): Promise<any> {
    const info: any = {
      containerized: false,
      orchestration: null,
      cicdPlatform: null,
      cloudProvider: null,
      infrastructureAsCode: null,
      monitoring: null,
      databases: [],
      services: [],
      hasDockerfile: false,
      hasKubernetes: false,
      hasCI: false
    };
    
    try {
      // Check for Docker
      try {
        await fs.access(path.join(projectPath, 'Dockerfile'));
        info.hasDockerfile = true;
        info.containerized = true;
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'docker-compose.yml'));
        info.containerized = true;
        info.orchestration = 'docker-compose';
      } catch {}
      
      // Check for Kubernetes
      try {
        await fs.access(path.join(projectPath, 'k8s'));
        info.hasKubernetes = true;
        info.orchestration = 'kubernetes';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'helm'));
        info.hasKubernetes = true;
        info.orchestration = 'helm';
      } catch {}
      
      // Check for CI/CD
      try {
        await fs.access(path.join(projectPath, '.github/workflows'));
        info.hasCI = true;
        info.cicdPlatform = 'github-actions';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, '.gitlab-ci.yml'));
        info.hasCI = true;
        info.cicdPlatform = 'gitlab-ci';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'Jenkinsfile'));
        info.hasCI = true;
        info.cicdPlatform = 'jenkins';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, '.circleci'));
        info.hasCI = true;
        info.cicdPlatform = 'circleci';
      } catch {}
      
      // Check for IaC
      try {
        await fs.access(path.join(projectPath, 'terraform'));
        info.infrastructureAsCode = 'terraform';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'cloudformation'));
        info.infrastructureAsCode = 'cloudformation';
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'ansible'));
        info.infrastructureAsCode = 'ansible';
      } catch {}
      
      // Check package.json for scripts
      try {
        const packageJsonPath = path.join(projectPath, 'package.json');
        const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
        
        if (packageJson.scripts) {
          if (packageJson.scripts.build) info.services.push('build');
          if (packageJson.scripts.start) info.services.push('app');
          if (packageJson.scripts.test) info.services.push('test');
        }
        
        // Check for database dependencies
        const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
        if (deps.pg || deps.postgresql) info.databases.push('postgresql');
        if (deps.mysql || deps.mysql2) info.databases.push('mysql');
        if (deps.mongodb || deps.mongoose) info.databases.push('mongodb');
        if (deps.redis) info.databases.push('redis');
      } catch {}
      
    } catch (error) {
      this.logger.warn('Could not analyze infrastructure', error);
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
    infraInfo: any,
    memoryContext: string
  ): string {
    const basePrompt = `
Infrastructure Context:
- Containerized: ${infraInfo.containerized ? 'Yes' : 'No'}
- Orchestration: ${infraInfo.orchestration || 'None'}
- CI/CD Platform: ${infraInfo.cicdPlatform || 'None detected'}
- Infrastructure as Code: ${infraInfo.infrastructureAsCode || 'None'}
- Databases: ${infraInfo.databases.join(', ') || 'None'}
- Services: ${infraInfo.services.join(', ') || 'None'}

Task: ${task.title}
Description: ${task.description}

Files Context:
${task.context.files?.join('\n') || 'No specific files'}

${task.context.codeContext || ''}
${memoryContext}

Please complete this DevOps task following best practices and industry standards.
Provide complete, production-ready configurations with clear documentation.
`;
    
    // Add task-specific instructions
    switch (task.type) {
      case 'create_dockerfile':
        return basePrompt + `
Create an optimized Dockerfile:
- Use multi-stage builds for smaller images
- Follow security best practices (non-root user)
- Optimize layer caching
- Include health checks
- Use specific base image versions
- Copy only necessary files
- Set proper environment variables
- Document build arguments`;
        
      case 'setup_ci_cd':
        return basePrompt + `
Set up CI/CD pipeline:
- Define build, test, and deploy stages
- Implement proper caching strategies
- Add security scanning (SAST/DAST)
- Include code quality checks
- Set up environment-specific deployments
- Implement proper secrets management
- Add notifications for failures
- Include rollback mechanisms`;
        
      case 'kubernetes_deployment':
        return basePrompt + `
Create Kubernetes deployment:
- Define deployment with proper replicas
- Create service for load balancing
- Add ConfigMaps for configuration
- Use Secrets for sensitive data
- Implement proper health checks
- Set resource limits and requests
- Add horizontal pod autoscaling
- Include network policies`;
        
      case 'infrastructure_as_code':
        return basePrompt + `
Implement infrastructure as code:
- Define all resources declaratively
- Use modules for reusability
- Implement proper state management
- Include variable definitions
- Add output values
- Implement proper tagging
- Include documentation
- Add destroy safeguards`;
        
      case 'monitoring_setup':
        return basePrompt + `
Set up monitoring and alerting:
- Configure metrics collection
- Set up log aggregation
- Create dashboards
- Define alert rules
- Implement SLIs/SLOs
- Add distributed tracing
- Configure retention policies
- Document runbooks`;
        
      case 'database_setup':
        return basePrompt + `
Set up database infrastructure:
- Configure high availability
- Implement backup strategies
- Set up replication
- Configure connection pooling
- Implement monitoring
- Add automated maintenance
- Configure security settings
- Document recovery procedures`;
        
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
    infraInfo: any
  ): Promise<any> {
    const result = {
      filesModified: [] as string[],
      filesCreated: [] as string[],
      testsAdded: 0,
      issues: [] as string[],
      summary: '',
      configurations: [] as string[],
      scripts: [] as string[]
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
              
              // Count infrastructure tests
              if (block.filename.includes('test') || 
                  block.filename.includes('spec')) {
                result.testsAdded++;
              }
            }
            
            // Track configurations
            if (block.filename.match(/\.(yml|yaml|json|tf|hcl)$/)) {
              result.configurations.push(filePath);
            }
            
            // Track scripts
            if (block.filename.match(/\.(sh|bash|ps1)$/) ||
                block.filename.includes('Jenkinsfile')) {
              result.scripts.push(filePath);
              
              // Make scripts executable on Unix systems
              if (process.platform !== 'win32') {
                try {
                  await fs.chmod(filePath, 0o755);
                } catch {}
              }
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