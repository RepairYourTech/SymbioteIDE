/**
 * Security Analyzer - Detects security vulnerabilities and issues
 */

import * as vscode from 'vscode';
import { Logger } from '../../utils/logger';
import {
  ReviewIssue,
  ReviewCategory,
  ReviewSeverity,
  CodeLocation,
  ReviewConfig
} from '../types';

export class SecurityAnalyzer {
  private logger = new Logger('SecurityAnalyzer');
  private config: ReviewConfig;
  
  // Common vulnerability patterns
  private vulnerabilityPatterns = [
    // SQL Injection
    {
      pattern: /(?:SELECT|INSERT|UPDATE|DELETE)\s+.*\s+WHERE\s+.*\+|\$\{|\$\(/gi,
      title: 'Potential SQL Injection',
      description: 'SQL query appears to use string concatenation or template literals',
      severity: ReviewSeverity.Critical,
      rule: 'no-sql-injection'
    },
    // XSS vulnerabilities
    {
      pattern: /\.innerHTML\s*=\s*[^'"]*(?:\$\{|\+)/g,
      title: 'Potential XSS via innerHTML',
      description: 'Using innerHTML with dynamic content can lead to XSS attacks',
      severity: ReviewSeverity.Critical,
      rule: 'no-unsafe-innerhtml'
    },
    {
      pattern: /document\.write\s*\(/g,
      title: 'Unsafe document.write usage',
      description: 'document.write can be exploited for XSS attacks',
      severity: ReviewSeverity.Error,
      rule: 'no-document-write'
    },
    // Code injection
    {
      pattern: /\beval\s*\(/g,
      title: 'Use of eval()',
      description: 'eval() can execute arbitrary code and is a security risk',
      severity: ReviewSeverity.Critical,
      rule: 'no-eval'
    },
    {
      pattern: /new\s+Function\s*\(/g,
      title: 'Dynamic function creation',
      description: 'Creating functions dynamically can lead to code injection',
      severity: ReviewSeverity.Error,
      rule: 'no-new-func'
    },
    // Hardcoded secrets
    {
      pattern: /(?:password|secret|api[_-]?key|token)\s*[:=]\s*['"][^'"]{8,}['"]/gi,
      title: 'Hardcoded credential detected',
      description: 'Credentials should not be hardcoded in source code',
      severity: ReviewSeverity.Critical,
      rule: 'no-hardcoded-secrets'
    },
    // Path traversal
    {
      pattern: /(?:readFile|readdir|unlink|rmdir).*\.\.[\/\\]/g,
      title: 'Potential path traversal',
      description: 'File operations with .. can lead to path traversal attacks',
      severity: ReviewSeverity.Error,
      rule: 'no-path-traversal'
    },
    // Command injection
    {
      pattern: /(?:exec|spawn|execFile)\s*\([^)]*\$\{|\+/g,
      title: 'Potential command injection',
      description: 'Executing system commands with user input is dangerous',
      severity: ReviewSeverity.Critical,
      rule: 'no-command-injection'
    },
    // Weak crypto
    {
      pattern: /\b(?:MD5|SHA1)\s*\(/gi,
      title: 'Weak cryptographic algorithm',
      description: 'MD5 and SHA1 are considered weak for security purposes',
      severity: ReviewSeverity.Warning,
      rule: 'no-weak-crypto'
    },
    // CORS issues
    {
      pattern: /Access-Control-Allow-Origin['"]?\s*:\s*['"]\*/g,
      title: 'Overly permissive CORS',
      description: 'Allowing all origins in CORS can be a security risk',
      severity: ReviewSeverity.Warning,
      rule: 'no-wildcard-cors'
    }
  ];
  
  // Language-specific checks
  private languageChecks = {
    javascript: [
      {
        check: (content: string) => content.includes('dangerouslySetInnerHTML'),
        title: 'React dangerouslySetInnerHTML usage',
        description: 'Using dangerouslySetInnerHTML can lead to XSS if not properly sanitized',
        severity: ReviewSeverity.Error,
        rule: 'react-no-danger'
      }
    ],
    python: [
      {
        check: (content: string) => /pickle\.loads?\s*\(/.test(content),
        title: 'Unsafe pickle usage',
        description: 'Unpickling untrusted data can execute arbitrary code',
        severity: ReviewSeverity.Critical,
        rule: 'no-unsafe-pickle'
      },
      {
        check: (content: string) => /input\s*\(/.test(content) && /exec\s*\(/.test(content),
        title: 'User input to exec',
        description: 'Executing user input is a critical security vulnerability',
        severity: ReviewSeverity.Critical,
        rule: 'no-input-exec'
      }
    ],
    java: [
      {
        check: (content: string) => /Runtime\.getRuntime\(\)\.exec/.test(content),
        title: 'Runtime.exec usage',
        description: 'Executing system commands can lead to command injection',
        severity: ReviewSeverity.Error,
        rule: 'no-runtime-exec'
      }
    ]
  };
  
  constructor(config: ReviewConfig) {
    this.config = config;
  }
  
  /**
   * Analyze file for security issues
   */
  async analyze(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    const lines = content.split('\n');
    
    try {
      // Check vulnerability patterns
      for (const vulnerabilityPattern of this.vulnerabilityPatterns) {
        if (this.isRuleDisabled(vulnerabilityPattern.rule)) {
          continue;
        }
        
        const matches = content.matchAll(vulnerabilityPattern.pattern);
        for (const match of matches) {
          const issue = this.createIssueFromMatch(
            file,
            content,
            match,
            vulnerabilityPattern
          );
          if (issue) {
            issues.push(issue);
          }
        }
      }
      
      // Language-specific checks
      const language = this.detectLanguage(file.fsPath);
      const langChecks = this.languageChecks[language] || [];
      
      for (const check of langChecks) {
        if (this.isRuleDisabled(check.rule)) {
          continue;
        }
        
        if (check.check(content)) {
          // Find specific occurrences
          const issue = this.createGeneralIssue(
            file,
            check.title,
            check.description,
            check.severity,
            check.rule
          );
          issues.push(issue);
        }
      }
      
      // Advanced security checks
      issues.push(...await this.performAdvancedChecks(file, content, context));
      
      // Check dependencies for vulnerabilities
      if (context.dependencies) {
        issues.push(...await this.checkDependencyVulnerabilities(file, context.dependencies));
      }
      
      // OWASP Top 10 checks
      issues.push(...await this.checkOWASPTop10(file, content, context));
      
      this.logger.info(`Found ${issues.length} security issues in ${file.fsPath}`);
      
    } catch (error) {
      this.logger.error('Security analysis failed', error);
    }
    
    return issues;
  }
  
  /**
   * Create issue from regex match
   */
  private createIssueFromMatch(
    file: vscode.Uri,
    content: string,
    match: RegExpMatchArray,
    pattern: any
  ): ReviewIssue | null {
    const position = this.getPositionFromIndex(content, match.index || 0);
    if (!position) return null;
    
    const line = content.split('\n')[position.line];
    const range = new vscode.Range(
      position,
      new vscode.Position(position.line, line.length)
    );
    
    return {
      id: `sec_${file.fsPath}_${position.line}_${Date.now()}`,
      category: ReviewCategory.Security,
      severity: pattern.severity,
      title: pattern.title,
      description: pattern.description,
      location: {
        uri: file,
        range,
        snippet: line.trim(),
        context: {
          before: position.line > 0 ? content.split('\n')[position.line - 1] : '',
          after: position.line < content.split('\n').length - 1 ? 
            content.split('\n')[position.line + 1] : ''
        }
      },
      rule: pattern.rule,
      impact: this.getSecurityImpact(pattern.severity),
      effort: this.getEffortEstimate(pattern.severity),
      tags: ['security', pattern.rule],
      metadata: {
        pattern: pattern.pattern.source,
        matchedText: match[0]
      }
    };
  }
  
  /**
   * Create general security issue
   */
  private createGeneralIssue(
    file: vscode.Uri,
    title: string,
    description: string,
    severity: ReviewSeverity,
    rule: string
  ): ReviewIssue {
    return {
      id: `sec_${file.fsPath}_${Date.now()}`,
      category: ReviewCategory.Security,
      severity,
      title,
      description,
      location: {
        uri: file,
        range: new vscode.Range(0, 0, 0, 0), // File-level issue
        snippet: ''
      },
      rule,
      impact: this.getSecurityImpact(severity),
      effort: this.getEffortEstimate(severity),
      tags: ['security', rule]
    };
  }
  
  /**
   * Perform advanced security checks
   */
  private async performAdvancedChecks(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    // Check for unsafe regex patterns (ReDoS)
    const unsafeRegexPatterns = [
      /\/\([^)]*\+\)\+/g, // Nested quantifiers
      /\/\([^)]*\|[^)]*\)\*/g, // Alternation with star
    ];
    
    for (const pattern of unsafeRegexPatterns) {
      if (pattern.test(content)) {
        issues.push(this.createGeneralIssue(
          file,
          'Potential ReDoS vulnerability',
          'Regular expression may be vulnerable to catastrophic backtracking',
          ReviewSeverity.Warning,
          'no-unsafe-regex'
        ));
      }
    }
    
    // Check for timing attacks in comparisons
    if (/password\s*===?\s*/.test(content) || /secret\s*===?\s*/.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Potential timing attack',
        'Direct string comparison of secrets can be vulnerable to timing attacks',
        ReviewSeverity.Warning,
        'secure-compare'
      ));
    }
    
    // Check for insufficient randomness
    if (/Math\.random\(\)/.test(content) && 
        (/token|session|password|key/i.test(content))) {
      issues.push(this.createGeneralIssue(
        file,
        'Weak random number generation',
        'Math.random() is not cryptographically secure',
        ReviewSeverity.Error,
        'crypto-random'
      ));
    }
    
    return issues;
  }
  
  /**
   * Check dependencies for known vulnerabilities
   */
  private async checkDependencyVulnerabilities(
    file: vscode.Uri,
    dependencies: any[]
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    // Known vulnerable packages (simplified - real implementation would use a vulnerability DB)
    const vulnerablePackages = {
      'lodash': { below: '4.17.21', severity: ReviewSeverity.Error },
      'minimist': { below: '1.2.6', severity: ReviewSeverity.Critical },
      'axios': { below: '0.21.2', severity: ReviewSeverity.Error }
    };
    
    for (const dep of dependencies) {
      const vuln = vulnerablePackages[dep.name];
      if (vuln && this.isVersionBelow(dep.version, vuln.below)) {
        issues.push(this.createGeneralIssue(
          file,
          `Vulnerable dependency: ${dep.name}`,
          `${dep.name}@${dep.version} has known vulnerabilities. Update to ${vuln.below} or later.`,
          vuln.severity,
          'vulnerable-dependency'
        ));
      }
    }
    
    return issues;
  }
  
  /**
   * Check for OWASP Top 10 vulnerabilities
   */
  private async checkOWASPTop10(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    // A03:2021 – Injection (already covered in patterns)
    
    // A01:2021 – Broken Access Control
    if (/\/api\//.test(file.fsPath) && !content.includes('auth') && !content.includes('permission')) {
      issues.push(this.createGeneralIssue(
        file,
        'Missing authentication check',
        'API endpoint appears to lack authentication checks',
        ReviewSeverity.Error,
        'broken-access-control'
      ));
    }
    
    // A02:2021 – Cryptographic Failures
    if (/password|secret/i.test(content) && !(/bcrypt|scrypt|argon2/i.test(content))) {
      issues.push(this.createGeneralIssue(
        file,
        'Weak password hashing',
        'Use strong hashing algorithms like bcrypt, scrypt, or argon2',
        ReviewSeverity.Error,
        'weak-crypto'
      ));
    }
    
    // A05:2021 – Security Misconfiguration
    if (/DEBUG\s*=\s*true/i.test(content) || /\.env\.development/.test(content)) {
      issues.push(this.createGeneralIssue(
        file,
        'Debug mode enabled',
        'Debug mode should be disabled in production',
        ReviewSeverity.Warning,
        'security-misconfiguration'
      ));
    }
    
    // A07:2021 – Identification and Authentication Failures
    if (/session|cookie/i.test(content) && !(/httponly|secure|samesite/i.test(content))) {
      issues.push(this.createGeneralIssue(
        file,
        'Insecure session configuration',
        'Session cookies should use httpOnly, secure, and sameSite flags',
        ReviewSeverity.Warning,
        'insecure-session'
      ));
    }
    
    return issues;
  }
  
  /**
   * Get position from string index
   */
  private getPositionFromIndex(content: string, index: number): vscode.Position | null {
    let line = 0;
    let character = 0;
    
    for (let i = 0; i < index && i < content.length; i++) {
      if (content[i] === '\n') {
        line++;
        character = 0;
      } else {
        character++;
      }
    }
    
    return new vscode.Position(line, character);
  }
  
  /**
   * Detect language from file path
   */
  private detectLanguage(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const langMap: Record<string, string> = {
      'js': 'javascript',
      'jsx': 'javascript',
      'ts': 'javascript',
      'tsx': 'javascript',
      'py': 'python',
      'java': 'java',
      'rb': 'ruby',
      'php': 'php',
      'go': 'go',
      'rs': 'rust',
      'cpp': 'cpp',
      'c': 'c',
      'cs': 'csharp'
    };
    return langMap[ext || ''] || 'unknown';
  }
  
  /**
   * Check if rule is disabled
   */
  private isRuleDisabled(rule: string): boolean {
    return this.config.disabledRules.includes(rule);
  }
  
  /**
   * Get security impact description
   */
  private getSecurityImpact(severity: ReviewSeverity): string {
    const impacts = {
      [ReviewSeverity.Critical]: 'Critical security vulnerability that could lead to system compromise',
      [ReviewSeverity.Error]: 'Serious security issue that could expose sensitive data or functionality',
      [ReviewSeverity.Warning]: 'Potential security concern that should be addressed',
      [ReviewSeverity.Info]: 'Security best practice recommendation'
    };
    return impacts[severity];
  }
  
  /**
   * Get effort estimate for fixing
   */
  private getEffortEstimate(severity: ReviewSeverity): number {
    const efforts = {
      [ReviewSeverity.Critical]: 8,
      [ReviewSeverity.Error]: 5,
      [ReviewSeverity.Warning]: 3,
      [ReviewSeverity.Info]: 1
    };
    return efforts[severity];
  }
  
  /**
   * Compare versions
   */
  private isVersionBelow(current: string, target: string): boolean {
    const currentParts = current.split('.').map(Number);
    const targetParts = target.split('.').map(Number);
    
    for (let i = 0; i < targetParts.length; i++) {
      if (currentParts[i] < targetParts[i]) return true;
      if (currentParts[i] > targetParts[i]) return false;
    }
    
    return false;
  }
  
  /**
   * Update configuration
   */
  updateConfig(config: ReviewConfig): void {
    this.config = config;
  }
}
