/**
 * Security Scanner
 * 
 * Scans MCP servers for security vulnerabilities and malicious patterns
 */

import { EventEmitter } from 'events';
import * as path from 'path';
import * as fs from 'fs';
import * as crypto from 'crypto';
import { SecurityScanResult, SecurityFinding, ThreatPattern } from '../types';

export interface SecurityScannerOptions {
  dataDirectory: string;
  enableDependencyScanning?: boolean;
  enableBehavioralAnalysis?: boolean;
  customPatterns?: ThreatPattern[];
}

export class SecurityScanner extends EventEmitter {
  private options: Required<SecurityScannerOptions>;
  private threatPatterns: Map<string, ThreatPattern> = new Map();
  private knownVulnerabilities: Map<string, any> = new Map();
  private scanHistory: Map<string, SecurityScanResult[]> = new Map();
  
  constructor(options: SecurityScannerOptions) {
    super();
    
    this.options = {
      enableDependencyScanning: true,
      enableBehavioralAnalysis: true,
      customPatterns: [],
      ...options
    };
    
    this.loadBuiltInPatterns();
    this.loadCustomPatterns();
  }
  
  /**
   * Initialize scanner
   */
  async initialize(): Promise<void> {
    // Create data directory
    await fs.promises.mkdir(this.options.dataDirectory, { recursive: true });
    
    // Load vulnerability database
    await this.loadVulnerabilityDatabase();
    
    // Load scan history
    await this.loadScanHistory();
  }
  
  /**
   * Scan MCP server
   */
  async scan(serverId: string, serverPath: string): Promise<SecurityScanResult> {
    const startTime = Date.now();
    const findings: SecurityFinding[] = [];
    
    try {
      // Check if path exists
      const stats = await fs.promises.stat(serverPath);
      
      if (stats.isDirectory()) {
        // Scan directory
        await this.scanDirectory(serverPath, findings);
      } else {
        // Scan single file
        await this.scanFile(serverPath, serverPath, findings);
      }
      
      // Scan dependencies if package.json exists
      if (this.options.enableDependencyScanning) {
        await this.scanDependencies(serverPath, findings);
      }
      
      // Calculate security score
      const score = this.calculateScore(findings);
      
      // Generate recommendations
      const recommendations = this.generateRecommendations(findings);
      
      const result: SecurityScanResult = {
        serverId,
        serverPath,
        timestamp: new Date(),
        passed: findings.filter(f => f.severity === 'critical' || f.severity === 'high').length === 0,
        findings,
        score,
        recommendations
      };
      
      // Store scan result
      await this.storeScanResult(serverId, result);
      
      // Emit events for critical findings
      const criticalFindings = findings.filter(f => f.severity === 'critical');
      if (criticalFindings.length > 0) {
        this.emit('critical-findings', { serverId, findings: criticalFindings });
      }
      
      return result;
      
    } catch (error) {
      return {
        serverId,
        serverPath,
        timestamp: new Date(),
        passed: false,
        findings: [{
          type: 'vulnerability',
          severity: 'high',
          title: 'Scan Error',
          description: `Failed to scan server: ${error.message}`,
          recommendation: 'Fix the error and rescan'
        }],
        score: 0,
        recommendations: ['Fix scan errors before proceeding']
      };
    }
  }
  
  /**
   * Scan directory recursively
   */
  private async scanDirectory(
    dirPath: string,
    findings: SecurityFinding[]
  ): Promise<void> {
    const entries = await fs.promises.readdir(dirPath, { withFileTypes: true });
    
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);
      
      // Skip common non-code directories
      if (entry.isDirectory()) {
        if (['node_modules', '.git', 'dist', 'build'].includes(entry.name)) {
          continue;
        }
        await this.scanDirectory(fullPath, findings);
      } else {
        // Scan files
        if (this.shouldScanFile(entry.name)) {
          await this.scanFile(fullPath, dirPath, findings);
        }
      }
    }
  }
  
  /**
   * Scan individual file
   */
  private async scanFile(
    filePath: string,
    basePath: string,
    findings: SecurityFinding[]
  ): Promise<void> {
    try {
      const content = await fs.promises.readFile(filePath, 'utf8');
      const relativePath = path.relative(basePath, filePath);
      
      // Check against threat patterns
      for (const pattern of this.threatPatterns.values()) {
        await this.checkPattern(content, relativePath, pattern, findings);
      }
      
      // Check for hardcoded secrets
      await this.checkForSecrets(content, relativePath, findings);
      
      // Check for dangerous functions
      await this.checkDangerousFunctions(content, relativePath, findings);
      
      // Check for suspicious URLs
      await this.checkSuspiciousUrls(content, relativePath, findings);
      
    } catch (error) {
      // Ignore read errors (permissions, etc)
    }
  }
  
  /**
   * Check content against threat pattern
   */
  private async checkPattern(
    content: string,
    filePath: string,
    pattern: ThreatPattern,
    findings: SecurityFinding[]
  ): Promise<void> {
    const lines = content.split('\n');
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      
      for (const patternStr of pattern.patterns) {
        const regex = new RegExp(patternStr, 'gi');
        const matches = line.matchAll(regex);
        
        for (const match of matches) {
          findings.push({
            type: 'malicious_code',
            severity: pattern.severity,
            title: pattern.name,
            description: pattern.description,
            location: {
              file: filePath,
              line: i + 1,
              column: match.index || 0
            },
            evidence: line.trim(),
            recommendation: pattern.mitigation,
            references: pattern.references
          });
        }
      }
    }
  }
  
  /**
   * Check for hardcoded secrets
   */
  private async checkForSecrets(
    content: string,
    filePath: string,
    findings: SecurityFinding[]
  ): Promise<void> {
    const secretPatterns = [
      {
        name: 'API Key',
        pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*['"]([a-zA-Z0-9_-]{20,})['"]|sk-[a-zA-Z0-9]{48}/gi,
        severity: 'critical' as const
      },
      {
        name: 'Private Key',
        pattern: /-----BEGIN (?:RSA |EC )?PRIVATE KEY-----/gi,
        severity: 'critical' as const
      },
      {
        name: 'AWS Credentials',
        pattern: /(?:aws_access_key_id|aws_secret_access_key)\s*[:=]\s*['"]([a-zA-Z0-9/+=]{20,})['"]|AKIA[0-9A-Z]{16}/gi,
        severity: 'critical' as const
      },
      {
        name: 'Database URL',
        pattern: /(?:mongodb|postgres|mysql|redis):\/\/[^:]+:[^@]+@[^\s]+/gi,
        severity: 'high' as const
      }
    ];
    
    for (const secretPattern of secretPatterns) {
      const matches = content.matchAll(secretPattern.pattern);
      
      for (const match of matches) {
        const line = content.substring(0, match.index).split('\n').length;
        
        findings.push({
          type: 'vulnerability',
          severity: secretPattern.severity,
          title: `Hardcoded ${secretPattern.name}`,
          description: `Found potential ${secretPattern.name} in source code`,
          location: {
            file: filePath,
            line,
            column: match.index || 0
          },
          evidence: this.sanitizeEvidence(match[0]),
          recommendation: `Remove hardcoded ${secretPattern.name} and use environment variables or secure credential storage`
        });
      }
    }
  }
  
  /**
   * Check for dangerous functions
   */
  private async checkDangerousFunctions(
    content: string,
    filePath: string,
    findings: SecurityFinding[]
  ): Promise<void> {
    const dangerousFunctions = [
      {
        name: 'eval()',
        pattern: /\beval\s*\(/gi,
        severity: 'high' as const,
        description: 'eval() can execute arbitrary code and is a security risk'
      },
      {
        name: 'Function constructor',
        pattern: /new\s+Function\s*\(/gi,
        severity: 'high' as const,
        description: 'Function constructor can execute arbitrary code'
      },
      {
        name: 'child_process.exec',
        pattern: /child_process\.(exec|execSync)\s*\(/gi,
        severity: 'high' as const,
        description: 'exec() is vulnerable to command injection'
      },
      {
        name: 'fs.chmod with 777',
        pattern: /fs\.chmod[Sync]*\s*\([^,]+,\s*0?777/gi,
        severity: 'medium' as const,
        description: 'Setting permissions to 777 is insecure'
      }
    ];
    
    for (const func of dangerousFunctions) {
      const matches = content.matchAll(func.pattern);
      
      for (const match of matches) {
        const line = content.substring(0, match.index).split('\n').length;
        
        findings.push({
          type: 'suspicious_pattern',
          severity: func.severity,
          title: `Dangerous function: ${func.name}`,
          description: func.description,
          location: {
            file: filePath,
            line,
            column: match.index || 0
          },
          evidence: match[0],
          recommendation: `Replace ${func.name} with a safer alternative`
        });
      }
    }
  }
  
  /**
   * Check for suspicious URLs
   */
  private async checkSuspiciousUrls(
    content: string,
    filePath: string,
    findings: SecurityFinding[]
  ): Promise<void> {
    const urlPattern = /https?:\/\/[^\s'"]+/gi;
    const suspiciousDomains = [
      'pastebin.com',
      'bit.ly',
      'tinyurl.com',
      'goo.gl',
      'ow.ly',
      'is.gd',
      'buff.ly'
    ];
    
    const matches = content.matchAll(urlPattern);
    
    for (const match of matches) {
      const url = match[0];
      
      for (const domain of suspiciousDomains) {
        if (url.includes(domain)) {
          const line = content.substring(0, match.index).split('\n').length;
          
          findings.push({
            type: 'suspicious_pattern',
            severity: 'medium',
            title: 'Suspicious URL',
            description: `URL to suspicious domain: ${domain}`,
            location: {
              file: filePath,
              line,
              column: match.index || 0
            },
            evidence: url,
            recommendation: 'Verify the purpose of this URL and ensure it is legitimate'
          });
        }
      }
    }
  }
  
  /**
   * Scan dependencies
   */
  private async scanDependencies(
    serverPath: string,
    findings: SecurityFinding[]
  ): Promise<void> {
    const packageJsonPath = path.join(serverPath, 'package.json');
    
    try {
      const packageJson = JSON.parse(
        await fs.promises.readFile(packageJsonPath, 'utf8')
      );
      
      const allDeps = {
        ...packageJson.dependencies,
        ...packageJson.devDependencies
      };
      
      for (const [name, version] of Object.entries(allDeps)) {
        // Check for known vulnerable packages
        if (this.knownVulnerabilities.has(name)) {
          const vuln = this.knownVulnerabilities.get(name);
          
          findings.push({
            type: 'dependency_issue',
            severity: vuln.severity,
            title: `Vulnerable dependency: ${name}`,
            description: vuln.description,
            location: {
              file: 'package.json'
            },
            recommendation: `Update ${name} to version ${vuln.fixedVersion} or higher`,
            cve: vuln.cve,
            references: vuln.references
          });
        }
        
        // Check for suspicious package names (typosquatting)
        if (this.isSuspiciousPackageName(name)) {
          findings.push({
            type: 'dependency_issue',
            severity: 'high',
            title: `Suspicious package name: ${name}`,
            description: 'This package name is similar to a popular package and could be a typosquatting attempt',
            location: {
              file: 'package.json'
            },
            recommendation: 'Verify this is the correct package name'
          });
        }
      }
    } catch {
      // No package.json or invalid
    }
  }
  
  /**
   * Check if package name is suspicious
   */
  private isSuspiciousPackageName(name: string): boolean {
    const popularPackages = [
      'express', 'react', 'lodash', 'axios', 'mongoose',
      'body-parser', 'nodemon', 'babel', 'webpack', 'eslint'
    ];
    
    // Check for one-character differences
    for (const popular of popularPackages) {
      if (this.isTyposquatting(name, popular)) {
        return true;
      }
    }
    
    return false;
  }
  
  /**
   * Check if name is typosquatting
   */
  private isTyposquatting(name: string, target: string): boolean {
    if (name === target) return false;
    
    // Check Levenshtein distance
    const distance = this.levenshteinDistance(name, target);
    
    // If very similar (1-2 character difference), it's suspicious
    return distance > 0 && distance <= 2;
  }
  
  /**
   * Calculate Levenshtein distance
   */
  private levenshteinDistance(a: string, b: string): number {
    const matrix = [];
    
    for (let i = 0; i <= b.length; i++) {
      matrix[i] = [i];
    }
    
    for (let j = 0; j <= a.length; j++) {
      matrix[0][j] = j;
    }
    
    for (let i = 1; i <= b.length; i++) {
      for (let j = 1; j <= a.length; j++) {
        if (b.charAt(i - 1) === a.charAt(j - 1)) {
          matrix[i][j] = matrix[i - 1][j - 1];
        } else {
          matrix[i][j] = Math.min(
            matrix[i - 1][j - 1] + 1,
            matrix[i][j - 1] + 1,
            matrix[i - 1][j] + 1
          );
        }
      }
    }
    
    return matrix[b.length][a.length];
  }
  
  /**
   * Calculate security score
   */
  private calculateScore(findings: SecurityFinding[]): number {
    let score = 100;
    
    for (const finding of findings) {
      switch (finding.severity) {
        case 'critical':
          score -= 30;
          break;
        case 'high':
          score -= 20;
          break;
        case 'medium':
          score -= 10;
          break;
        case 'low':
          score -= 5;
          break;
      }
    }
    
    return Math.max(0, score);
  }
  
  /**
   * Generate recommendations
   */
  private generateRecommendations(findings: SecurityFinding[]): string[] {
    const recommendations = new Set<string>();
    
    // Add specific recommendations based on findings
    const hasSecrets = findings.some(f => f.title.includes('Hardcoded'));
    if (hasSecrets) {
      recommendations.add('Use environment variables or secure credential storage for sensitive data');
    }
    
    const hasDangerousFunctions = findings.some(f => f.title.includes('Dangerous function'));
    if (hasDangerousFunctions) {
      recommendations.add('Review and replace dangerous functions with safer alternatives');
    }
    
    const hasVulnerableDeps = findings.some(f => f.type === 'dependency_issue');
    if (hasVulnerableDeps) {
      recommendations.add('Update vulnerable dependencies to their latest secure versions');
    }
    
    const hasCritical = findings.some(f => f.severity === 'critical');
    if (hasCritical) {
      recommendations.add('Address critical security issues immediately before using this server');
    }
    
    // General recommendations
    if (findings.length === 0) {
      recommendations.add('No security issues found. Continue monitoring for updates');
    } else {
      recommendations.add('Implement security best practices and regular scanning');
    }
    
    return Array.from(recommendations);
  }
  
  /**
   * Should scan file based on extension
   */
  private shouldScanFile(filename: string): boolean {
    const extensions = [
      '.js', '.ts', '.jsx', '.tsx', '.json',
      '.py', '.rb', '.php', '.java', '.cs',
      '.go', '.rs', '.cpp', '.c', '.h',
      '.sh', '.bash', '.ps1', '.yml', '.yaml'
    ];
    
    return extensions.some(ext => filename.endsWith(ext));
  }
  
  /**
   * Sanitize evidence for display
   */
  private sanitizeEvidence(evidence: string): string {
    // Mask actual secret values
    return evidence
      .replace(/['"][a-zA-Z0-9_-]{20,}['"]/g, '"***REDACTED***"')
      .replace(/sk-[a-zA-Z0-9]{48}/g, 'sk-***REDACTED***')
      .replace(/AKIA[0-9A-Z]{16}/g, 'AKIA***REDACTED***');
  }
  
  /**
   * Load built-in threat patterns
   */
  private loadBuiltInPatterns(): void {
    const patterns: ThreatPattern[] = [
      {
        id: 'reverse-shell',
        name: 'Reverse Shell',
        description: 'Attempts to establish reverse shell connection',
        severity: 'critical',
        patterns: [
          'nc\\.traditional.*-e.*\\/bin\\/(ba)?sh',
          'bash.*-i.*>&.*\\/dev\\/tcp',
          'python.*socket.*connect.*shell'
        ],
        indicators: ['socket', 'connect', 'shell', 'exec'],
        mitigation: 'Remove reverse shell code immediately'
      },
      {
        id: 'crypto-miner',
        name: 'Cryptocurrency Miner',
        description: 'Code that mines cryptocurrency',
        severity: 'critical',
        patterns: [
          'stratum\\+tcp:\\/\\/',
          'cryptonight',
          'monero.*wallet',
          'xmrig'
        ],
        indicators: ['miner', 'hash', 'nonce', 'proof-of-work'],
        mitigation: 'Remove cryptocurrency mining code'
      },
      {
        id: 'data-exfiltration',
        name: 'Data Exfiltration',
        description: 'Attempts to send data to external servers',
        severity: 'high',
        patterns: [
          'fetch.*process\\.env',
          'axios.*post.*\\/etc\\/passwd',
          'fs\\.readFileSync.*\\.ssh'
        ],
        indicators: ['upload', 'exfiltrate', 'steal', 'send'],
        mitigation: 'Review and remove data exfiltration code'
      }
    ];
    
    for (const pattern of patterns) {
      this.threatPatterns.set(pattern.id, pattern);
    }
  }
  
  /**
   * Load custom patterns
   */
  private loadCustomPatterns(): void {
    for (const pattern of this.options.customPatterns) {
      this.threatPatterns.set(pattern.id, pattern);
    }
  }
  
  /**
   * Load vulnerability database
   */
  private async loadVulnerabilityDatabase(): Promise<void> {
    // In a real implementation, this would load from a CVE database
    // For now, we'll use a minimal set of known vulnerabilities
    
    this.knownVulnerabilities.set('lodash', {
      severity: 'high',
      description: 'Prototype pollution vulnerability',
      fixedVersion: '4.17.21',
      cve: 'CVE-2020-8203',
      references: ['https://nvd.nist.gov/vuln/detail/CVE-2020-8203']
    });
    
    this.knownVulnerabilities.set('minimist', {
      severity: 'medium',
      description: 'Prototype pollution vulnerability',
      fixedVersion: '1.2.6',
      cve: 'CVE-2021-44906',
      references: ['https://nvd.nist.gov/vuln/detail/CVE-2021-44906']
    });
  }
  
  /**
   * Store scan result
   */
  private async storeScanResult(
    serverId: string,
    result: SecurityScanResult
  ): Promise<void> {
    const history = this.scanHistory.get(serverId) || [];
    history.push(result);
    
    // Keep last 10 scans
    if (history.length > 10) {
      history.shift();
    }
    
    this.scanHistory.set(serverId, history);
    
    // Persist to disk
    const historyFile = path.join(
      this.options.dataDirectory,
      `scan-history-${serverId}.json`
    );
    
    await fs.promises.writeFile(
      historyFile,
      JSON.stringify(history, null, 2)
    );
  }
  
  /**
   * Load scan history
   */
  private async loadScanHistory(): Promise<void> {
    try {
      const files = await fs.promises.readdir(this.options.dataDirectory);
      
      for (const file of files) {
        if (file.startsWith('scan-history-') && file.endsWith('.json')) {
          const serverId = file.replace('scan-history-', '').replace('.json', '');
          const content = await fs.promises.readFile(
            path.join(this.options.dataDirectory, file),
            'utf8'
          );
          
          this.scanHistory.set(serverId, JSON.parse(content));
        }
      }
    } catch {
      // No history yet
    }
  }
}