/**
 * Testing Specialist Agent
 * 
 * Specializes in test creation, test strategies, and quality assurance
 */

import { BaseSpecialistAgent } from './base-specialist';
import { AgentSpecialty, HivemindTask, TaskResult } from '../types';
import * as path from 'path';
import * as fs from 'fs/promises';

export class TestingSpecialistAgent extends BaseSpecialistAgent {
  constructor(id: string) {
    super(
      id,
      'Testing Specialist',
      AgentSpecialty.Testing,
      [
        'unit_testing',
        'integration_testing',
        'e2e_testing',
        'test_automation',
        'tdd',
        'bdd',
        'performance_testing',
        'security_testing',
        'test_coverage',
        'mocking',
        'test_data_generation',
        'regression_testing'
      ]
    );
  }
  
  /**
   * Configure agent with testing-specific prompts
   */
  protected async configureAgent(): Promise<void> {
    if (!this.agent) return;
    
    const systemPrompt = `You are a Testing and Quality Assurance Specialist with expertise in:
- Unit testing (Jest, Mocha, Vitest, JUnit, pytest)
- Integration testing and API testing
- End-to-end testing (Cypress, Playwright, Selenium)
- Test-Driven Development (TDD) and Behavior-Driven Development (BDD)
- Test automation frameworks and CI/CD integration
- Performance testing (JMeter, K6, Artillery)
- Security testing and vulnerability scanning
- Code coverage analysis and reporting
- Mock creation and test data generation
- Test strategy and test plan development

When working on testing tasks:
1. Write comprehensive tests that cover happy paths and edge cases
2. Follow AAA pattern (Arrange, Act, Assert)
3. Create descriptive test names that explain what is being tested
4. Use appropriate mocking and stubbing techniques
5. Ensure tests are isolated and independent
6. Aim for high code coverage but focus on meaningful tests
7. Include performance and security considerations
8. Make tests maintainable and easy to understand

Always provide complete, working test code with clear explanations.`;
    
    // Update agent configuration
    if ('setSystemPrompt' in this.agent) {
      await (this.agent as any).setSystemPrompt(systemPrompt);
    }
  }
  
  /**
   * Perform testing-specific task
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
      // Analyze test environment
      const testInfo = await this.analyzeTestEnvironment(task.context.projectPath!);
      
      // Analyze code to test if applicable
      let codeAnalysis = null;
      if (task.context.files && task.context.files.length > 0) {
        codeAnalysis = await this.analyzeCodeToTest(task.context.files[0]);
      }
      
      // Build context from memories
      const memoryContext = this.buildMemoryContext(memories);
      
      // Create the prompt based on task type
      const prompt = this.buildTaskPrompt(task, testInfo, codeAnalysis, memoryContext);
      
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
        testInfo
      );
      
      result.success = true;
      result.output = taskOutput;
      result.filesModified = taskOutput.filesModified || [];
      result.filesCreated = taskOutput.filesCreated || [];
      result.testsAdded = taskOutput.testsAdded || 0;
      
      // Add coverage report if available
      if (taskOutput.coverage) {
        result.output.coverage = taskOutput.coverage;
      }
      
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
   * Analyze test environment
   */
  private async analyzeTestEnvironment(projectPath: string): Promise<any> {
    const info: any = {
      framework: 'unknown',
      testRunner: null,
      coverageTool: null,
      e2eFramework: null,
      mockingLibrary: null,
      assertionLibrary: null,
      testFilePattern: null,
      hasTypeScript: false
    };
    
    try {
      // Check package.json
      const packageJsonPath = path.join(projectPath, 'package.json');
      const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
      const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
      
      // Detect test runner
      if (deps.jest) {
        info.testRunner = 'jest';
        info.testFilePattern = '**/*.{test,spec}.{js,jsx,ts,tsx}';
      } else if (deps.vitest) {
        info.testRunner = 'vitest';
        info.testFilePattern = '**/*.{test,spec}.{js,jsx,ts,tsx}';
      } else if (deps.mocha) {
        info.testRunner = 'mocha';
        info.testFilePattern = '**/*.{test,spec}.js';
      } else if (deps['@playwright/test']) {
        info.testRunner = 'playwright';
      }
      
      // Detect coverage tool
      if (deps.nyc) info.coverageTool = 'nyc';
      else if (deps['c8']) info.coverageTool = 'c8';
      else if (info.testRunner === 'jest') info.coverageTool = 'jest';
      else if (info.testRunner === 'vitest') info.coverageTool = 'vitest';
      
      // Detect E2E framework
      if (deps.cypress) info.e2eFramework = 'cypress';
      else if (deps.playwright || deps['@playwright/test']) info.e2eFramework = 'playwright';
      else if (deps.puppeteer) info.e2eFramework = 'puppeteer';
      else if (deps['selenium-webdriver']) info.e2eFramework = 'selenium';
      
      // Detect mocking library
      if (deps.sinon) info.mockingLibrary = 'sinon';
      else if (deps['@testing-library/jest-dom']) info.mockingLibrary = 'jest';
      else if (deps.nock) info.mockingLibrary = 'nock';
      
      // Detect assertion library  
      if (deps.chai) info.assertionLibrary = 'chai';
      else if (deps.expect) info.assertionLibrary = 'expect';
      else if (info.testRunner === 'jest') info.assertionLibrary = 'jest';
      
      // Check for TypeScript
      info.hasTypeScript = !!deps.typescript;
      
      // Check test scripts
      if (packageJson.scripts) {
        info.testScript = packageJson.scripts.test || null;
        info.coverageScript = packageJson.scripts.coverage || null;
        info.e2eScript = packageJson.scripts.e2e || packageJson.scripts['test:e2e'] || null;
      }
      
    } catch (error) {
      this.logger.warn('Could not analyze test environment', error);
    }
    
    return info;
  }
  
  /**
   * Analyze code to test
   */
  private async analyzeCodeToTest(filePath: string): Promise<any> {
    const analysis = {
      fileType: '',
      exports: [] as string[],
      classes: [] as string[],
      functions: [] as string[],
      complexity: 'low'
    };
    
    try {
      const content = await fs.readFile(filePath, 'utf8');
      
      // Determine file type
      const ext = path.extname(filePath);
      if (['.js', '.jsx', '.ts', '.tsx'].includes(ext)) {
        analysis.fileType = 'javascript';
        
        // Extract exports
        const exportRegex = /export\s+(?:default\s+)?(?:class|function|const|let|var)\s+(\w+)/g;
        let match;
        while ((match = exportRegex.exec(content)) !== null) {
          analysis.exports.push(match[1]);
        }
        
        // Extract classes
        const classRegex = /class\s+(\w+)/g;
        while ((match = classRegex.exec(content)) !== null) {
          analysis.classes.push(match[1]);
        }
        
        // Extract functions
        const functionRegex = /(?:function|const|let|var)\s+(\w+)\s*=?\s*(?:\([^)]*\)\s*=>|function)/g;
        while ((match = functionRegex.exec(content)) !== null) {
          analysis.functions.push(match[1]);
        }
        
        // Estimate complexity
        const lines = content.split('\n').length;
        if (lines > 500) analysis.complexity = 'high';
        else if (lines > 200) analysis.complexity = 'medium';
        
      } else if (['.py'].includes(ext)) {
        analysis.fileType = 'python';
        // Python analysis would go here
      }
      
    } catch (error) {
      this.logger.warn('Could not analyze code file', error);
    }
    
    return analysis;
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
    testInfo: any,
    codeAnalysis: any,
    memoryContext: string
  ): string {
    const basePrompt = `
Test Environment:
- Test Runner: ${testInfo.testRunner || 'Not detected'}
- Coverage Tool: ${testInfo.coverageTool || 'Not detected'}
- E2E Framework: ${testInfo.e2eFramework || 'Not detected'}
- Mocking Library: ${testInfo.mockingLibrary || 'Not detected'}
- TypeScript: ${testInfo.hasTypeScript ? 'Yes' : 'No'}

${codeAnalysis ? `Code to Test:
- File Type: ${codeAnalysis.fileType}
- Exports: ${codeAnalysis.exports.join(', ') || 'None'}
- Classes: ${codeAnalysis.classes.join(', ') || 'None'}
- Functions: ${codeAnalysis.functions.join(', ') || 'None'}
- Complexity: ${codeAnalysis.complexity}
` : ''}

Task: ${task.title}
Description: ${task.description}

Files Context:
${task.context.files?.join('\n') || 'No specific files'}

${task.context.codeContext || ''}
${memoryContext}

Please complete this testing task following the project's conventions and best practices.
Provide complete test code with clear explanations.
`;
    
    // Add task-specific instructions
    switch (task.type) {
      case 'create_unit_tests':
        return basePrompt + `
Create comprehensive unit tests:
- Test all exported functions/methods
- Cover happy paths and edge cases
- Test error conditions
- Use appropriate mocking for dependencies
- Aim for >80% code coverage
- Include descriptive test names
- Group related tests using describe blocks`;
        
      case 'create_integration_tests':
        return basePrompt + `
Create integration tests:
- Test component interactions
- Test API endpoints with real requests
- Test database operations
- Use test database/environment
- Clean up test data after tests
- Test error scenarios
- Verify response formats`;
        
      case 'create_e2e_tests':
        return basePrompt + `
Create end-to-end tests:
- Test complete user workflows
- Use page object pattern
- Handle async operations properly
- Include waits and retries
- Test on multiple viewports/browsers
- Capture screenshots on failure
- Use data-testid attributes`;
        
      case 'improve_test_coverage':
        return basePrompt + `
Improve test coverage:
- Identify uncovered code paths
- Add tests for missing scenarios
- Focus on critical business logic
- Test boundary conditions
- Add tests for error handling
- Ensure branch coverage
- Update existing tests if needed`;
        
      case 'create_test_data':
        return basePrompt + `
Create test data and fixtures:
- Generate realistic test data
- Create reusable fixtures
- Include edge case data
- Provide data factories/builders
- Handle different data states
- Consider data relationships
- Make data easily maintainable`;
        
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
    testInfo: any
  ): Promise<any> {
    const result = {
      filesModified: [] as string[],
      filesCreated: [] as string[],
      testsAdded: 0,
      issues: [] as string[],
      summary: '',
      coverage: null as any,
      testSuites: [] as string[]
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
            }
            
            // Count tests
            const testCount = this.countTests(block.content, testInfo.testRunner);
            result.testsAdded += testCount;
            
            // Track test suites
            if (testCount > 0) {
              result.testSuites.push(filePath);
            }
            
          } catch (error) {
            result.issues.push(`Failed to write ${block.filename}: ${error.message}`);
          }
        }
      }
      
      // Extract coverage info if mentioned
      const coverageMatch = output.match(/coverage[:\s]+(\d+)%/i);
      if (coverageMatch) {
        result.coverage = {
          percentage: parseInt(coverageMatch[1]),
          details: this.extractCoverageDetails(output)
        };
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
   * Count tests in code
   */
  private countTests(code: string, testRunner: string): number {
    let count = 0;
    
    if (testRunner === 'jest' || testRunner === 'vitest') {
      // Count it() and test() calls
      const testRegex = /(?:it|test)\s*\(['"`]/g;
      const matches = code.match(testRegex);
      count = matches ? matches.length : 0;
    } else if (testRunner === 'mocha') {
      // Count it() calls
      const testRegex = /it\s*\(['"`]/g;
      const matches = code.match(testRegex);
      count = matches ? matches.length : 0;
    } else if (testRunner === 'playwright') {
      // Count test() calls
      const testRegex = /test\s*\(['"`]/g;
      const matches = code.match(testRegex);
      count = matches ? matches.length : 0;
    }
    
    return count;
  }
  
  /**
   * Extract coverage details
   */
  private extractCoverageDetails(output: string): any {
    const details: any = {};
    
    // Try to extract different coverage metrics
    const statementsMatch = output.match(/statements[:\s]+(\d+)%/i);
    if (statementsMatch) details.statements = parseInt(statementsMatch[1]);
    
    const branchesMatch = output.match(/branches[:\s]+(\d+)%/i);
    if (branchesMatch) details.branches = parseInt(branchesMatch[1]);
    
    const functionsMatch = output.match(/functions[:\s]+(\d+)%/i);
    if (functionsMatch) details.functions = parseInt(functionsMatch[1]);
    
    const linesMatch = output.match(/lines[:\s]+(\d+)%/i);
    if (linesMatch) details.lines = parseInt(linesMatch[1]);
    
    return Object.keys(details).length > 0 ? details : null;
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
