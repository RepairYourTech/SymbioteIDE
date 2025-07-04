/**
 * Performance Specialist Agent
 * 
 * Specializes in performance optimization, profiling, and scalability
 */

import { BaseSpecialistAgent } from './base-specialist';
import { AgentSpecialty, HivemindTask, TaskResult } from '../types';
import * as path from 'path';
import * as fs from 'fs/promises';

export class PerformanceSpecialistAgent extends BaseSpecialistAgent {
  constructor(id: string) {
    super(
      id,
      'Performance Specialist',
      AgentSpecialty.Performance,
      [
        'performance_profiling',
        'optimization',
        'caching_strategies',
        'database_optimization',
        'frontend_performance',
        'backend_performance',
        'memory_optimization',
        'load_testing',
        'bottleneck_analysis',
        'scalability_planning',
        'cdn_configuration',
        'async_processing'
      ]
    );
  }
  
  /**
   * Configure agent with performance-specific prompts
   */
  protected async configureAgent(): Promise<void> {
    if (!this.agent) return;
    
    const systemPrompt = `You are a Performance Optimization Specialist with expertise in:
- Performance profiling and analysis
- Frontend optimization (bundle size, rendering, lazy loading)
- Backend optimization (database queries, caching, async processing)
- Memory management and leak detection
- Load testing and stress testing
- Scalability architecture and patterns
- CDN configuration and edge computing
- Database query optimization and indexing
- Caching strategies (Redis, Memcached, browser cache)
- Performance monitoring and alerting
- Web Vitals and Core Web Vitals optimization

When working on performance tasks:
1. Always measure before and after optimization
2. Focus on the biggest bottlenecks first
3. Consider the trade-offs (memory vs CPU, latency vs throughput)
4. Implement proper caching strategies
5. Optimize for the common case
6. Use appropriate data structures and algorithms
7. Document performance improvements
8. Include performance tests and benchmarks

Provide optimized code with clear performance metrics and explanations.`;
    
    // Update agent configuration
    if ('setSystemPrompt' in this.agent) {
      await (this.agent as any).setSystemPrompt(systemPrompt);
    }
  }
  
  /**
   * Perform performance-specific task
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
      // Analyze performance context
      const perfInfo = await this.analyzePerformanceContext(task.context.projectPath!);
      
      // Perform performance analysis if needed
      let analysisResults = null;
      if (task.type === 'performance_profiling' || task.type === 'bottleneck_analysis') {
        analysisResults = await this.performPerformanceAnalysis(task.context.files || []);
      }
      
      // Build context from memories
      const memoryContext = this.buildMemoryContext(memories);
      
      // Create the prompt based on task type
      const prompt = this.buildTaskPrompt(task, perfInfo, analysisResults, memoryContext);
      
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
        perfInfo
      );
      
      result.success = true;
      result.output = taskOutput;
      result.filesModified = taskOutput.filesModified || [];
      result.filesCreated = taskOutput.filesCreated || [];
      result.testsAdded = taskOutput.testsAdded || 0;
      
      // Add performance improvements
      if (taskOutput.improvements) {
        result.output.performanceGains = taskOutput.improvements;
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
   * Analyze performance context
   */
  private async analyzePerformanceContext(projectPath: string): Promise<any> {
    const info: any = {
      framework: 'unknown',
      bundler: null,
      caching: [],
      cdn: null,
      monitoring: [],
      optimization: {
        minification: false,
        compression: false,
        lazyLoading: false,
        codeSplitting: false
      },
      database: {
        type: null,
        hasIndexes: false,
        hasCaching: false
      }
    };
    
    try {
      // Check package.json
      const packageJsonPath = path.join(projectPath, 'package.json');
      const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
      const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
      
      // Detect framework
      if (deps.react) info.framework = 'react';
      else if (deps.vue) info.framework = 'vue';
      else if (deps['@angular/core']) info.framework = 'angular';
      else if (deps.express) info.framework = 'express';
      else if (deps.next) info.framework = 'nextjs';
      
      // Detect bundler
      if (deps.webpack) info.bundler = 'webpack';
      else if (deps.vite) info.bundler = 'vite';
      else if (deps.rollup) info.bundler = 'rollup';
      else if (deps.parcel) info.bundler = 'parcel';
      else if (deps.esbuild) info.bundler = 'esbuild';
      
      // Detect caching
      if (deps.redis) info.caching.push('redis');
      if (deps.memcached) info.caching.push('memcached');
      if (deps['node-cache']) info.caching.push('node-cache');
      
      // Detect monitoring
      if (deps['@sentry/node'] || deps['@sentry/browser']) info.monitoring.push('sentry');
      if (deps['newrelic']) info.monitoring.push('newrelic');
      if (deps['@opentelemetry/api']) info.monitoring.push('opentelemetry');
      
      // Detect optimization tools
      if (deps.terser || deps['terser-webpack-plugin']) info.optimization.minification = true;
      if (deps['compression-webpack-plugin']) info.optimization.compression = true;
      
      // Check for performance scripts
      if (packageJson.scripts) {
        if (packageJson.scripts['build:prod'] || packageJson.scripts['build:production']) {
          info.optimization.minification = true;
        }
        if (packageJson.scripts.analyze || packageJson.scripts['bundle-analyze']) {
          info.optimization.bundleAnalysis = true;
        }
      }
      
      // Check webpack config for optimizations
      try {
        const webpackConfig = await fs.readFile(path.join(projectPath, 'webpack.config.js'), 'utf8');
        if (webpackConfig.includes('splitChunks')) info.optimization.codeSplitting = true;
        if (webpackConfig.includes('lazy') || webpackConfig.includes('import(')) info.optimization.lazyLoading = true;
      } catch {}
      
    } catch (error) {
      this.logger.warn('Could not analyze performance context', error);
    }
    
    return info;
  }
  
  /**
   * Perform performance analysis
   */
  private async performPerformanceAnalysis(files: string[]): Promise<any> {
    const analysis = {
      bottlenecks: [],
      opportunities: [],
      metrics: {}
    };
    
    for (const file of files) {
      try {
        const content = await fs.readFile(file, 'utf8');
        const ext = path.extname(file);
        
        // Check for common performance issues
        
        // JavaScript/TypeScript files
        if (['.js', '.jsx', '.ts', '.tsx'].includes(ext)) {
          // Check for synchronous operations
          if (content.match(/readFileSync|writeFileSync/)) {
            analysis.bottlenecks.push({
              file,
              type: 'sync-io',
              description: 'Synchronous file operations detected'
            });
          }
          
          // Check for inefficient loops
          if (content.match(/for\s*\(.*in\s+/)) {
            analysis.opportunities.push({
              file,
              type: 'loop-optimization',
              description: 'for...in loop can be optimized'
            });
          }
          
          // Check for multiple array operations
          if (content.match(/\.filter\(.*\)\.map\(.*\)\.reduce\(/)) {
            analysis.opportunities.push({
              file,
              type: 'array-chain',
              description: 'Multiple array operations can be combined'
            });
          }
          
          // Check bundle size indicators
          const importCount = (content.match(/import\s+/g) || []).length;
          if (importCount > 20) {
            analysis.bottlenecks.push({
              file,
              type: 'bundle-size',
              description: `High number of imports (${importCount})`
            });
          }
        }
        
        // CSS files
        if (['.css', '.scss', '.less'].includes(ext)) {
          // Check for unused selectors (simplified)
          const selectorCount = (content.match(/[.#][a-zA-Z][\w-]*/g) || []).length;
          if (selectorCount > 500) {
            analysis.opportunities.push({
              file,
              type: 'css-optimization',
              description: `Large CSS file with ${selectorCount} selectors`
            });
          }
        }
        
        // Database queries
        if (content.includes('SELECT') || content.includes('find(') || content.includes('aggregate(')) {
          // Check for N+1 queries
          if (content.match(/for.*await.*\.(find|findOne|query)/)) {
            analysis.bottlenecks.push({
              file,
              type: 'n+1-query',
              description: 'Potential N+1 query problem'
            });
          }
          
          // Check for missing indexes
          if (content.match(/WHERE|find\({[^}]*\}/) && !content.includes('index')) {
            analysis.opportunities.push({
              file,
              type: 'database-index',
              description: 'Query may benefit from database index'
            });
          }
        }
        
      } catch (error) {
        this.logger.warn(`Could not analyze file ${file}`, error);
      }
    }
    
    // Calculate metrics
    analysis.metrics = {
      bottleneckCount: analysis.bottlenecks.length,
      opportunityCount: analysis.opportunities.length,
      estimatedImpact: analysis.bottlenecks.length * 10 + analysis.opportunities.length * 5
    };
    
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
    perfInfo: any,
    analysisResults: any,
    memoryContext: string
  ): string {
    const basePrompt = `
Performance Context:
- Framework: ${perfInfo.framework}
- Bundler: ${perfInfo.bundler || 'None detected'}
- Caching: ${perfInfo.caching.join(', ') || 'None'}
- Monitoring: ${perfInfo.monitoring.join(', ') || 'None'}
- Optimizations: 
  - Minification: ${perfInfo.optimization.minification ? 'Yes' : 'No'}
  - Code Splitting: ${perfInfo.optimization.codeSplitting ? 'Yes' : 'No'}
  - Lazy Loading: ${perfInfo.optimization.lazyLoading ? 'Yes' : 'No'}

${analysisResults ? `Performance Analysis:
- Bottlenecks Found: ${analysisResults.bottleneckCount}
- Optimization Opportunities: ${analysisResults.opportunityCount}
${analysisResults.bottlenecks.map((b: any) => `  - ${b.type}: ${b.description} in ${b.file}`).join('\n')}
` : ''}

Task: ${task.title}
Description: ${task.description}

Files Context:
${task.context.files?.join('\n') || 'No specific files'}

${task.context.codeContext || ''}
${memoryContext}

Please complete this performance optimization task with measurable improvements.
Provide optimized code with performance metrics and benchmarks.
`;
    
    // Add task-specific instructions
    switch (task.type) {
      case 'frontend_optimization':
        return basePrompt + `
Optimize frontend performance:
- Reduce bundle size (tree shaking, code splitting)
- Implement lazy loading for components/routes
- Optimize images (WebP, lazy loading, sizing)
- Minimize render blocking resources
- Implement efficient caching strategies
- Optimize critical rendering path
- Measure Core Web Vitals improvements
- Add performance monitoring`;
        
      case 'backend_optimization':
        return basePrompt + `
Optimize backend performance:
- Profile and identify bottlenecks
- Optimize database queries and add indexes
- Implement caching layers (Redis/Memcached)
- Use connection pooling
- Implement async/parallel processing
- Optimize API response times
- Add request/response compression
- Include performance benchmarks`;
        
      case 'database_optimization':
        return basePrompt + `
Optimize database performance:
- Analyze slow queries
- Add appropriate indexes
- Optimize query structure
- Implement query caching
- Use database-specific optimizations
- Implement connection pooling
- Add query monitoring
- Document performance gains`;
        
      case 'caching_strategy':
        return basePrompt + `
Implement caching strategy:
- Identify cacheable data
- Choose appropriate caching layers
- Implement cache invalidation
- Set proper TTL values
- Add cache warming if needed
- Monitor cache hit rates
- Handle cache failures gracefully
- Document caching policies`;
        
      case 'load_testing':
        return basePrompt + `
Perform load testing:
- Set up load testing framework
- Define realistic test scenarios
- Gradually increase load
- Monitor system metrics
- Identify breaking points
- Document bottlenecks
- Provide optimization recommendations
- Create performance baseline`;
        
      case 'memory_optimization':
        return basePrompt + `
Optimize memory usage:
- Profile memory usage
- Identify memory leaks
- Optimize data structures
- Implement object pooling if needed
- Use streams for large data
- Implement garbage collection tuning
- Add memory monitoring
- Document memory improvements`;
        
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
    perfInfo: any
  ): Promise<any> {
    const result = {
      filesModified: [] as string[],
      filesCreated: [] as string[],
      testsAdded: 0,
      issues: [] as string[],
      improvements: [] as any[],
      benchmarks: [] as any[],
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
              
              // Count performance tests
              if (block.filename.includes('benchmark') || 
                  block.filename.includes('perf') ||
                  block.filename.includes('.test.')) {
                result.testsAdded++;
              }
            }
            
          } catch (error) {
            result.issues.push(`Failed to write ${block.filename}: ${error.message}`);
          }
        }
      }
      
      // Extract performance improvements
      const improvementMatches = output.match(/improvement:\s*([^\n]+)/gi);
      if (improvementMatches) {
        improvementMatches.forEach(match => {
          const improvement = match.replace(/improvement:\s*/i, '');
          // Try to extract metrics
          const metricMatch = improvement.match(/(\d+)%?\s*(faster|slower|reduction|improvement)/i);
          if (metricMatch) {
            result.improvements.push({
              description: improvement,
              metric: parseInt(metricMatch[1]),
              type: metricMatch[2].toLowerCase()
            });
          } else {
            result.improvements.push({
              description: improvement,
              metric: null,
              type: 'general'
            });
          }
        });
      }
      
      // Extract benchmarks
      const benchmarkMatches = output.match(/benchmark:\s*([^\n]+)/gi);
      if (benchmarkMatches) {
        result.benchmarks = benchmarkMatches.map(m => 
          m.replace(/benchmark:\s*/i, '')
        );
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