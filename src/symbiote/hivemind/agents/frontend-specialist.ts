/**
 * Frontend Specialist Agent
 * 
 * ADK-powered agent specializing in UI/UX, React, Vue, Angular, and frontend development
 */

import { Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../adk/core/adk-hivemind-base';
import { AgentSpecialty, HivemindTask, TaskResult } from '../types';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';

export interface FrontendSpecialistConfig extends ADKHivemindConfig {
  projectPath?: string;
}

export class FrontendSpecialistAgent extends ADKHivemindAgent {
  private projectInfo: any = null;
  
  constructor(config: FrontendSpecialistConfig) {
    super({
      ...config,
      id: config.id || 'frontend-specialist',
      name: config.name || 'Frontend Specialist',
      specialty: AgentSpecialty.Frontend,
      capabilities: [
        'react_development',
        'vue_development',
        'angular_development',
        'ui_design',
        'responsive_design',
        'component_creation',
        'state_management',
        'styling',
        'accessibility',
        'performance_optimization'
      ],
      systemPrompt: config.systemPrompt || `You are a Frontend Development Specialist with expertise in:
- React, Vue, Angular, and modern JavaScript frameworks
- Component-based architecture and design patterns
- State management (Redux, Vuex, MobX, Context API)
- CSS, SASS, Tailwind, and modern styling approaches
- Responsive design and mobile-first development
- Performance optimization and lazy loading
- Accessibility (WCAG) and SEO best practices
- Modern build tools (Webpack, Vite, etc.)

When working on frontend tasks:
1. Follow framework best practices and conventions
2. Create reusable, modular components
3. Ensure responsive design across devices
4. Implement proper error handling and loading states
5. Consider accessibility from the start
6. Optimize for performance (bundle size, rendering)
7. Write clean, maintainable code with proper TypeScript types
8. Include unit tests for components

Always provide complete, working code that follows the project's existing patterns.`
    });
    
    // Add frontend-specific tools
    this.addFrontendTools();
  }
  
  /**
   * Add frontend-specific tools
   */
  private addFrontendTools(): void {
    // Project analysis tool
    this.tools.push(new Tool({
      name: 'analyze_project',
      description: 'Analyze project structure and frontend setup',
      parameters: {
        projectPath: { type: 'string', required: true }
      },
      handler: async ({ projectPath }) => {
        return this.analyzeProjectStructure(projectPath);
      }
    }));
    
    // Component generation tool
    this.tools.push(new Tool({
      name: 'generate_component',
      description: 'Generate a new frontend component',
      parameters: {
        name: { type: 'string', required: true },
        type: { type: 'string', required: true },
        path: { type: 'string', required: true },
        props: { type: 'object', required: false }
      },
      handler: async (params) => {
        return this.generateComponent(params);
      }
    }));
    
    // Performance analysis tool
    this.tools.push(new Tool({
      name: 'analyze_performance',
      description: 'Analyze frontend performance issues',
      parameters: {
        filePath: { type: 'string', required: true }
      },
      handler: async ({ filePath }) => {
        return this.analyzePerformance(filePath);
      }
    }));
    
    // Accessibility audit tool
    this.tools.push(new Tool({
      name: 'audit_accessibility',
      description: 'Audit component for accessibility issues',
      parameters: {
        componentPath: { type: 'string', required: true }
      },
      handler: async ({ componentPath }) => {
        return this.auditAccessibility(componentPath);
      }
    }));
  }
  
  /**
   * Execute frontend task
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
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
      // Analyze project structure
      const projectInfo = await this.analyzeProjectStructure(task.context.projectPath!);
      
      // Build context from memories
      const memoryContext = this.buildMemoryContext(memories);
      
      // Create the prompt based on task type
      const prompt = this.buildTaskPrompt(task, projectInfo, memoryContext);
      
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
        projectInfo
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
   * Analyze project structure for frontend context
   */
  private async analyzeProjectStructure(projectPath: string): Promise<any> {
    const info: any = {
      framework: 'unknown',
      hasTypeScript: false,
      styling: 'css',
      stateManagement: null,
      componentPath: null,
      testingFramework: null
    };
    
    try {
      // Check package.json
      const packageJsonPath = path.join(projectPath, 'package.json');
      const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
      
      const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
      
      // Detect framework
      if (deps.react) {
        info.framework = 'react';
        info.componentPath = await this.findComponentPath(projectPath, 'react');
      } else if (deps.vue) {
        info.framework = 'vue';
        info.componentPath = await this.findComponentPath(projectPath, 'vue');
      } else if (deps['@angular/core']) {
        info.framework = 'angular';
        info.componentPath = await this.findComponentPath(projectPath, 'angular');
      }
      
      // Detect TypeScript
      info.hasTypeScript = !!deps.typescript;
      
      // Detect styling
      if (deps.tailwindcss) info.styling = 'tailwind';
      else if (deps.sass || deps['node-sass']) info.styling = 'sass';
      else if (deps['styled-components']) info.styling = 'styled-components';
      else if (deps.emotion) info.styling = 'emotion';
      
      // Detect state management
      if (deps.redux) info.stateManagement = 'redux';
      else if (deps.mobx) info.stateManagement = 'mobx';
      else if (deps.vuex) info.stateManagement = 'vuex';
      else if (deps.pinia) info.stateManagement = 'pinia';
      
      // Detect testing
      if (deps.jest) info.testingFramework = 'jest';
      else if (deps.vitest) info.testingFramework = 'vitest';
      else if (deps.mocha) info.testingFramework = 'mocha';
      
    } catch (error) {
      this.logger.warn('Could not analyze project structure', error);
    }
    
    return info;
  }
  
  /**
   * Find component directory
   */
  private async findComponentPath(
    projectPath: string,
    framework: string
  ): Promise<string | null> {
    const possiblePaths = [
      'src/components',
      'components',
      'src/app/components',
      'app/components',
      'src/views/components'
    ];
    
    for (const p of possiblePaths) {
      try {
        const fullPath = path.join(projectPath, p);
        const stats = await fs.stat(fullPath);
        if (stats.isDirectory()) {
          return fullPath;
        }
      } catch {}
    }
    
    return null;
  }
  
  /**
   * Generate component
   */
  private async generateComponent(params: any): Promise<any> {
    const { name, type, path, props } = params;
    
    // Determine framework-specific template
    const template = this.getComponentTemplate(type, props);
    
    return {
      name,
      path: `${path}/${name}.tsx`,
      content: template,
      testPath: `${path}/${name}.test.tsx`
    };
  }
  
  /**
   * Analyze performance
   */
  private async analyzePerformance(filePath: string): Promise<any> {
    const issues: string[] = [];
    const suggestions: string[] = [];
    
    try {
      const content = await fs.readFile(filePath, 'utf8');
      
      // Check for common performance issues
      if (content.includes('componentDidUpdate') && !content.includes('shouldComponentUpdate')) {
        issues.push('Missing shouldComponentUpdate optimization');
        suggestions.push('Add shouldComponentUpdate or use React.memo');
      }
      
      if (content.match(/useState.*\[\]/g)) {
        issues.push('Empty dependency array in hooks');
        suggestions.push('Review hook dependencies');
      }
      
      if (content.includes('import * as')) {
        issues.push('Using wildcard imports');
        suggestions.push('Use named imports to reduce bundle size');
      }
      
    } catch (error) {
      issues.push(`Could not analyze file: ${error.message}`);
    }
    
    return { issues, suggestions };
  }
  
  /**
   * Audit accessibility
   */
  private async auditAccessibility(componentPath: string): Promise<any> {
    const issues: string[] = [];
    const suggestions: string[] = [];
    
    try {
      const content = await fs.readFile(componentPath, 'utf8');
      
      // Basic accessibility checks
      if (content.includes('<img') && !content.includes('alt=')) {
        issues.push('Images missing alt attributes');
      }
      
      if (content.includes('onClick') && !content.includes('onKeyDown')) {
        issues.push('Click handlers without keyboard support');
        suggestions.push('Add onKeyDown handlers for keyboard accessibility');
      }
      
      if (!content.includes('aria-') && content.includes('role=')) {
        suggestions.push('Consider adding ARIA attributes for better screen reader support');
      }
      
    } catch (error) {
      issues.push(`Could not audit file: ${error.message}`);
    }
    
    return { issues, suggestions };
  }
  
  /**
   * Get component template
   */
  private getComponentTemplate(type: string, props?: any): string {
    // Basic React functional component template
    return `import React from 'react';

interface ${type}Props {
  ${props ? Object.entries(props).map(([key, value]) => `${key}: ${value};`).join('\n  ') : '// Add props here'}
}

export const ${type}: React.FC<${type}Props> = (props) => {
  return (
    <div>
      {/* Component implementation */}
    </div>
  );
};`;
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    // Get project info from context or analyze if needed
    const projectInfo = this.projectInfo || {
      framework: 'unknown',
      hasTypeScript: false,
      styling: 'css',
      stateManagement: null
    };
    
    const basePrompt = `
Project Context:
- Framework: ${projectInfo.framework}
- TypeScript: ${projectInfo.hasTypeScript ? 'Yes' : 'No'}
- Styling: ${projectInfo.styling}
- State Management: ${projectInfo.stateManagement || 'None'}
- Component Path: ${projectInfo.componentPath || 'Not found'}

Task: ${task.title}
Description: ${task.description}

Files Context:
${task.context.files?.join('\n') || 'No specific files'}

${task.context.codeContext || ''}

Please complete this frontend task following the project's conventions and best practices.
Provide the complete implementation with clear explanations.
`;
    
    // Add task-specific instructions
    switch (task.type) {
      case 'create_component':
        return basePrompt + `
Create a reusable component with:
- Proper TypeScript types/interfaces
- Props validation
- Responsive design
- Accessibility features
- Unit tests
- Storybook story (if applicable)`;
        
      case 'implement_ui':
        return basePrompt + `
Implement the UI with:
- Pixel-perfect design implementation
- Responsive breakpoints
- Smooth animations/transitions
- Loading and error states
- Accessibility compliance`;
        
      case 'optimize_performance':
        return basePrompt + `
Optimize frontend performance:
- Identify performance bottlenecks
- Implement code splitting
- Optimize bundle size
- Add lazy loading
- Improve rendering performance
- Measure improvements`;
        
      case 'fix_bug':
        return basePrompt + `
Fix the frontend bug:
- Identify root cause
- Implement proper fix
- Add tests to prevent regression
- Ensure no side effects
- Update documentation if needed`;
        
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
    projectInfo: any
  ): Promise<any> {
    const result = {
      filesModified: [] as string[],
      filesCreated: [] as string[],
      testsAdded: 0,
      issues: [] as string[],
      summary: ''
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
              if (block.filename.includes('.test.') || block.filename.includes('.spec.')) {
                result.testsAdded++;
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
