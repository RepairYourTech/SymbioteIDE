/**
 * Security Specialist Agent
 * 
 * Specializes in security audits, vulnerability detection, and secure coding
 */

import { BaseSpecialistAgent } from './base-specialist';
import { AgentSpecialty, HivemindTask, TaskResult } from '../types';
import * as path from 'path';
import * as fs from 'fs/promises';

export class SecuritySpecialistAgent extends BaseSpecialistAgent {
  constructor(id: string) {
    super(
      id,
      'Security Specialist',
      AgentSpecialty.Security,
      [
        'security_audit',
        'vulnerability_assessment',
        'secure_coding',
        'authentication_security',
        'authorization_security',
        'encryption',
        'input_validation',
        'sql_injection_prevention',
        'xss_prevention',
        'csrf_protection',
        'security_headers',
        'penetration_testing'
      ]
    );
  }
  
  /**
   * Configure agent with security-specific prompts
   */
  protected async configureAgent(): Promise<void> {
    if (!this.agent) return;
    
    const systemPrompt = `You are a Security Specialist with expertise in:
- Application security and secure coding practices
- OWASP Top 10 vulnerabilities and prevention
- Authentication and authorization security
- Cryptography and encryption best practices
- Security auditing and vulnerability assessment
- Input validation and sanitization
- SQL injection, XSS, CSRF prevention
- Security headers and CSP
- Penetration testing methodologies
- Compliance (GDPR, PCI-DSS, HIPAA)

When working on security tasks:
1. Always prioritize security over convenience
2. Follow the principle of least privilege
3. Implement defense in depth
4. Use proven security libraries and patterns
5. Never store sensitive data in plain text
6. Always validate and sanitize user input
7. Implement proper error handling without information leakage
8. Add security tests and documentation

Provide secure code implementations with clear explanations of security measures.`;
    
    // Update agent configuration
    if ('setSystemPrompt' in this.agent) {
      await (this.agent as any).setSystemPrompt(systemPrompt);
    }
  }
  
  /**
   * Perform security-specific task
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
      // Analyze security context
      const securityInfo = await this.analyzeSecurityContext(task.context.projectPath!);
      
      // Perform security scan if needed
      let scanResults = null;
      if (task.type === 'security_audit' || task.type === 'vulnerability_scan') {
        scanResults = await this.performSecurityScan(task.context.files || []);
      }
      
      // Build context from memories
      const memoryContext = this.buildMemoryContext(memories);
      
      // Create the prompt based on task type
      const prompt = this.buildTaskPrompt(task, securityInfo, scanResults, memoryContext);
      
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
        securityInfo
      );
      
      result.success = true;
      result.output = taskOutput;
      result.filesModified = taskOutput.filesModified || [];
      result.filesCreated = taskOutput.filesCreated || [];
      result.testsAdded = taskOutput.testsAdded || 0;
      
      // Add security issues found
      if (taskOutput.vulnerabilities) {
        result.issuesFound = taskOutput.vulnerabilities.map((v: any) => 
          `${v.severity}: ${v.type} - ${v.description}`
        );
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
   * Analyze security context
   */
  private async analyzeSecurityContext(projectPath: string): Promise<any> {
    const info: any = {
      hasSecurityConfig: false,
      authenticationMethod: null,
      encryptionLibraries: [],
      securityMiddleware: [],
      sensitiveDataHandling: [],
      complianceRequirements: [],
      existingVulnerabilities: []
    };
    
    try {
      // Check package.json for security-related dependencies
      const packageJsonPath = path.join(projectPath, 'package.json');
      const packageJson = JSON.parse(await fs.readFile(packageJsonPath, 'utf8'));
      const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
      
      // Authentication libraries
      if (deps.passport) info.authenticationMethod = 'passport';
      else if (deps.jsonwebtoken) info.authenticationMethod = 'jwt';
      else if (deps['express-session']) info.authenticationMethod = 'session';
      else if (deps['@auth0/nextjs-auth0']) info.authenticationMethod = 'auth0';
      
      // Encryption libraries
      if (deps.bcrypt || deps.bcryptjs) info.encryptionLibraries.push('bcrypt');
      if (deps.crypto) info.encryptionLibraries.push('crypto');
      if (deps['node-forge']) info.encryptionLibraries.push('node-forge');
      
      // Security middleware
      if (deps.helmet) info.securityMiddleware.push('helmet');
      if (deps.cors) info.securityMiddleware.push('cors');
      if (deps['express-rate-limit']) info.securityMiddleware.push('rate-limiting');
      if (deps.csurf) info.securityMiddleware.push('csrf');
      
      // Check for security config files
      try {
        await fs.access(path.join(projectPath, '.env'));
        info.hasSecurityConfig = true;
      } catch {}
      
      try {
        await fs.access(path.join(projectPath, 'security.config.js'));
        info.hasSecurityConfig = true;
      } catch {}
      
    } catch (error) {
      this.logger.warn('Could not analyze security context', error);
    }
    
    return info;
  }
  
  /**
   * Perform security scan
   */
  private async performSecurityScan(files: string[]): Promise<any> {
    const scanResults = {
      vulnerabilities: [],
      warnings: [],
      recommendations: []
    };
    
    // This would integrate with actual security scanning tools
    // For now, we'll check for common patterns
    
    for (const file of files) {
      try {
        const content = await fs.readFile(file, 'utf8');
        
        // Check for hardcoded secrets
        if (content.match(/api[_-]?key\s*=\s*["'][^"']{10,}/i)) {
          scanResults.vulnerabilities.push({
            type: 'hardcoded-secret',
            severity: 'high',
            file,
            description: 'Possible hardcoded API key detected'
          });
        }
        
        // Check for SQL injection vulnerabilities
        if (content.match(/query\s*\(\s*['"`].*\$\{.*\}.*['"`]\s*\)/)) {
          scanResults.vulnerabilities.push({
            type: 'sql-injection',
            severity: 'critical',
            file,
            description: 'Possible SQL injection vulnerability'
          });
        }
        
        // Check for XSS vulnerabilities
        if (content.match(/innerHTML\s*=.*\$\{/) || content.match(/dangerouslySetInnerHTML/)) {
          scanResults.vulnerabilities.push({
            type: 'xss',
            severity: 'high',
            file,
            description: 'Possible XSS vulnerability'
          });
        }
        
        // Check for weak crypto
        if (content.match(/md5|sha1/i)) {
          scanResults.warnings.push({
            type: 'weak-crypto',
            file,
            description: 'Weak cryptographic algorithm detected'
          });
        }
        
      } catch (error) {
        this.logger.warn(`Could not scan file ${file}`, error);
      }
    }
    
    return scanResults;
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
    securityInfo: any,
    scanResults: any,
    memoryContext: string
  ): string {
    const basePrompt = `
Security Context:
- Authentication: ${securityInfo.authenticationMethod || 'None detected'}
- Encryption Libraries: ${securityInfo.encryptionLibraries.join(', ') || 'None'}
- Security Middleware: ${securityInfo.securityMiddleware.join(', ') || 'None'}
- Has Security Config: ${securityInfo.hasSecurityConfig ? 'Yes' : 'No'}

${scanResults ? `Security Scan Results:
- Vulnerabilities Found: ${scanResults.vulnerabilities.length}
- Warnings: ${scanResults.warnings.length}
${scanResults.vulnerabilities.map((v: any) => `  - ${v.severity}: ${v.type} in ${v.file}`).join('\n')}
` : ''}

Task: ${task.title}
Description: ${task.description}

Files Context:
${task.context.files?.join('\n') || 'No specific files'}

${task.context.codeContext || ''}
${memoryContext}

Please complete this security task following best practices and industry standards.
Provide secure implementations with detailed security considerations.
`;
    
    // Add task-specific instructions
    switch (task.type) {
      case 'security_audit':
        return basePrompt + `
Perform a comprehensive security audit:
- Review code for OWASP Top 10 vulnerabilities
- Check authentication and authorization
- Verify input validation and sanitization
- Review error handling and logging
- Check for secure communication (HTTPS, TLS)
- Assess data protection and encryption
- Create detailed report with findings and fixes`;
        
      case 'implement_auth':
        return basePrompt + `
Implement secure authentication:
- Use strong password hashing (bcrypt/argon2)
- Implement secure session management
- Add multi-factor authentication if needed
- Implement account lockout mechanisms
- Add proper logging and monitoring
- Include password reset functionality
- Add tests for auth flows`;
        
      case 'fix_vulnerability':
        return basePrompt + `
Fix the security vulnerability:
- Identify the exact vulnerability
- Implement proper fix following best practices
- Add input validation/sanitization
- Update error handling
- Add security tests
- Document the fix and prevention
- Ensure no regression`;
        
      case 'implement_encryption':
        return basePrompt + `
Implement encryption:
- Use appropriate encryption algorithms
- Implement secure key management
- Encrypt data at rest and in transit
- Add proper initialization vectors
- Implement secure random generation
- Add encryption/decryption utilities
- Include comprehensive tests`;
        
      case 'security_headers':
        return basePrompt + `
Implement security headers:
- Add Content Security Policy (CSP)
- Implement HSTS header
- Add X-Frame-Options
- Set X-Content-Type-Options
- Add Referrer-Policy
- Implement Feature-Policy
- Configure CORS properly`;
        
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
    securityInfo: any
  ): Promise<any> {
    const result = {
      filesModified: [] as string[],
      filesCreated: [] as string[],
      testsAdded: 0,
      vulnerabilities: [] as any[],
      fixes: [] as string[],
      recommendations: [] as string[],
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
              
              // Count security tests
              if (block.filename.includes('.test.') || 
                  block.filename.includes('.spec.') ||
                  block.filename.includes('security')) {
                result.testsAdded++;
              }
            }
            
            // Track security fixes
            if (block.filename.includes('fix') || 
                block.content.includes('// Security fix')) {
              result.fixes.push(filePath);
            }
            
          } catch (error) {
            result.vulnerabilities.push({
              type: 'file-write-error',
              severity: 'low',
              description: `Failed to write ${block.filename}: ${error.message}`
            });
          }
        }
      }
      
      // Extract vulnerabilities from output
      const vulnMatches = output.match(/vulnerability:\s*([^\n]+)/gi);
      if (vulnMatches) {
        vulnMatches.forEach(match => {
          result.vulnerabilities.push({
            type: 'detected',
            severity: 'medium',
            description: match.replace(/vulnerability:\s*/i, '')
          });
        });
      }
      
      // Extract recommendations
      const recMatches = output.match(/recommendation:\s*([^\n]+)/gi);
      if (recMatches) {
        result.recommendations = recMatches.map(m => 
          m.replace(/recommendation:\s*/i, '')
        );
      }
      
      // Extract summary
      result.summary = this.extractSummary(output);
      
    } catch (error) {
      this.logger.error('Failed to parse results', error);
      result.vulnerabilities.push({
        type: 'parse-error',
        severity: 'low',
        description: `Parse error: ${error.message}`
      });
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