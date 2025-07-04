/**
 * Dependency Analyzer - Analyzes dependencies for vulnerabilities and issues
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { Logger } from '../../utils/logger';
import {
  ReviewIssue,
  ReviewCategory,
  ReviewSeverity,
  CodeLocation,
  ReviewConfig
} from '../types';

export interface DependencyInfo {
  name: string;
  version: string;
  type: 'production' | 'development' | 'peer' | 'optional';
  location: string;
}

export interface VulnerabilityInfo {
  severity: 'low' | 'moderate' | 'high' | 'critical';
  title: string;
  description: string;
  cve?: string;
  fixedIn?: string;
}

export class DependencyAnalyzer {
  private logger = new Logger('DependencyAnalyzer');
  private config: ReviewConfig;
  
  // Known vulnerable packages (simplified - real implementation would use a vulnerability DB)
  private vulnerablePackages: Record<string, Array<{ versions: string; vulnerability: VulnerabilityInfo }>> = {
    'lodash': [
      {
        versions: '<4.17.21',
        vulnerability: {
          severity: 'high',
          title: 'Prototype Pollution',
          description: 'Versions before 4.17.21 are vulnerable to Prototype Pollution',
          cve: 'CVE-2021-23337',
          fixedIn: '4.17.21'
        }
      }
    ],
    'axios': [
      {
        versions: '<0.21.2',
        vulnerability: {
          severity: 'high',
          title: 'Server-Side Request Forgery',
          description: 'Axios before 0.21.2 is vulnerable to SSRF',
          cve: 'CVE-2021-3749',
          fixedIn: '0.21.2'
        }
      }
    ],
    'minimist': [
      {
        versions: '<1.2.6',
        vulnerability: {
          severity: 'critical',
          title: 'Prototype Pollution',
          description: 'minimist before 1.2.6 is vulnerable to Prototype Pollution',
          cve: 'CVE-2021-44906',
          fixedIn: '1.2.6'
        }
      }
    ],
    'node-fetch': [
      {
        versions: '<2.6.7',
        vulnerability: {
          severity: 'high',
          title: 'Denial of Service',
          description: 'node-fetch before 2.6.7 is vulnerable to DoS',
          cve: 'CVE-2022-0235',
          fixedIn: '2.6.7'
        }
      }
    ]
  };
  
  // Deprecated packages
  private deprecatedPackages: Record<string, string> = {
    'request': 'Use axios, node-fetch, or built-in fetch instead',
    'node-uuid': 'Use uuid instead',
    'jade': 'Use pug instead',
    'bower': 'Use npm or yarn instead',
    'tslint': 'Use ESLint with TypeScript support instead'
  };
  
  // License compatibility issues
  private restrictedLicenses = ['GPL', 'AGPL', 'LGPL'];
  
  constructor(config: ReviewConfig) {
    this.config = config;
  }
  
  /**
   * Analyze file for dependency issues
   */
  async analyze(
    file: vscode.Uri,
    content: string,
    context: any
  ): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    const fileName = path.basename(file.fsPath);
    
    try {
      // Check package.json files
      if (fileName === 'package.json') {
        issues.push(...await this.analyzePackageJson(file, content));
      }
      
      // Check requirements.txt (Python)
      if (fileName === 'requirements.txt') {
        issues.push(...await this.analyzePythonRequirements(file, content));
      }
      
      // Check Gemfile (Ruby)
      if (fileName === 'Gemfile') {
        issues.push(...await this.analyzeGemfile(file, content));
      }
      
      // Check import statements
      issues.push(...this.analyzeImports(file, content));
      
      // Check for hardcoded versions
      issues.push(...this.checkHardcodedVersions(file, content));
      
      // Check for missing dependencies
      if (context.projectDependencies) {
        issues.push(...this.checkMissingDependencies(file, content, context.projectDependencies));
      }
      
      this.logger.info(`Found ${issues.length} dependency issues in ${file.fsPath}`);
      
    } catch (error) {
      this.logger.error('Dependency analysis failed', error);
    }
    
    return issues;
  }
  
  /**
   * Analyze package.json
   */
  private async analyzePackageJson(file: vscode.Uri, content: string): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    
    try {
      const packageData = JSON.parse(content);
      
      // Check dependencies
      const allDeps = {
        ...packageData.dependencies,
        ...packageData.devDependencies,
        ...packageData.peerDependencies,
        ...packageData.optionalDependencies
      };
      
      // Check for vulnerabilities
      for (const [name, version] of Object.entries(allDeps)) {
        const vulnInfo = this.checkVulnerability(name, version as string);
        if (vulnInfo) {
          issues.push(this.createVulnerabilityIssue(file, name, version as string, vulnInfo));
        }
        
        // Check for deprecated packages
        if (this.deprecatedPackages[name]) {
          issues.push(this.createDeprecationIssue(file, name, this.deprecatedPackages[name]));
        }
      }
      
      // Check for version patterns
      issues.push(...this.checkVersionPatterns(file, allDeps));
      
      // Check for duplicate dependencies
      issues.push(...this.checkDuplicateDependencies(file, packageData));
      
      // Check for missing fields
      issues.push(...this.checkPackageJsonCompleteness(file, packageData));
      
      // Check license
      if (packageData.license) {
        issues.push(...this.checkLicense(file, packageData.license));
      }
      
    } catch (error) {
      issues.push({
        id: `dep_${file.fsPath}_parse_error`,
        category: ReviewCategory.Dependencies,
        severity: ReviewSeverity.Error,
        title: 'Invalid package.json',
        description: `Failed to parse package.json: ${error.message}`,
        location: {
          uri: file,
          range: new vscode.Range(0, 0, 0, 0),
          snippet: ''
        },
        rule: 'valid-package-json',
        tags: ['dependencies', 'package.json']
      });
    }
    
    return issues;
  }
  
  /**
   * Check for vulnerabilities
   */
  private checkVulnerability(packageName: string, version: string): VulnerabilityInfo | null {
    const vulnerabilities = this.vulnerablePackages[packageName];
    if (!vulnerabilities) return null;
    
    for (const vuln of vulnerabilities) {
      if (this.isVersionMatch(version, vuln.versions)) {
        return vuln.vulnerability;
      }
    }
    
    return null;
  }
  
  /**
   * Check version patterns
   */
  private checkVersionPatterns(file: vscode.Uri, dependencies: Record<string, string>): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    for (const [name, version] of Object.entries(dependencies)) {
      // Check for wildcard versions
      if (version === '*' || version === 'latest') {
        issues.push({
          id: `dep_${file.fsPath}_wildcard_${name}`,
          category: ReviewCategory.Dependencies,
          severity: ReviewSeverity.Error,
          title: `Wildcard version for ${name}`,
          description: 'Using wildcard versions can lead to unexpected breaking changes',
          location: {
            uri: file,
            range: new vscode.Range(0, 0, 0, 0),
            snippet: `"${name}": "${version}"`
          },
          rule: 'no-wildcard-versions',
          tags: ['dependencies', 'version']
        });
      }
      
      // Check for Git URLs
      if (version.includes('git://') || version.includes('github.com')) {
        issues.push({
          id: `dep_${file.fsPath}_git_${name}`,
          category: ReviewCategory.Dependencies,
          severity: ReviewSeverity.Warning,
          title: `Git URL dependency: ${name}`,
          description: 'Git URL dependencies can be unstable and harder to cache',
          location: {
            uri: file,
            range: new vscode.Range(0, 0, 0, 0),
            snippet: `"${name}": "${version}"`
          },
          rule: 'no-git-dependencies',
          tags: ['dependencies', 'git']
        });
      }
      
      // Check for file: protocol
      if (version.startsWith('file:')) {
        issues.push({
          id: `dep_${file.fsPath}_local_${name}`,
          category: ReviewCategory.Dependencies,
          severity: ReviewSeverity.Warning,
          title: `Local file dependency: ${name}`,
          description: 'Local file dependencies are not portable',
          location: {
            uri: file,
            range: new vscode.Range(0, 0, 0, 0),
            snippet: `"${name}": "${version}"`
          },
          rule: 'no-file-dependencies',
          tags: ['dependencies', 'local']
        });
      }
    }
    
    return issues;
  }
  
  /**
   * Check for duplicate dependencies
   */
  private checkDuplicateDependencies(file: vscode.Uri, packageData: any): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const deps = packageData.dependencies || {};
    const devDeps = packageData.devDependencies || {};
    
    // Check for packages in both dependencies and devDependencies
    for (const name of Object.keys(deps)) {
      if (devDeps[name]) {
        issues.push({
          id: `dep_${file.fsPath}_duplicate_${name}`,
          category: ReviewCategory.Dependencies,
          severity: ReviewSeverity.Warning,
          title: `Duplicate dependency: ${name}`,
          description: `${name} is listed in both dependencies and devDependencies`,
          location: {
            uri: file,
            range: new vscode.Range(0, 0, 0, 0),
            snippet: ''
          },
          rule: 'no-duplicate-dependencies',
          tags: ['dependencies', 'duplicate']
        });
      }
    }
    
    return issues;
  }
  
  /**
   * Check package.json completeness
   */
  private checkPackageJsonCompleteness(file: vscode.Uri, packageData: any): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const requiredFields = ['name', 'version', 'description', 'license'];
    const recommendedFields = ['author', 'repository', 'keywords', 'engines'];
    
    // Check required fields
    for (const field of requiredFields) {
      if (!packageData[field]) {
        issues.push({
          id: `dep_${file.fsPath}_missing_${field}`,
          category: ReviewCategory.Dependencies,
          severity: ReviewSeverity.Warning,
          title: `Missing ${field} field`,
          description: `package.json should include a ${field} field`,
          location: {
            uri: file,
            range: new vscode.Range(0, 0, 0, 0),
            snippet: ''
          },
          rule: 'complete-package-json',
          tags: ['dependencies', 'package.json']
        });
      }
    }
    
    // Check recommended fields
    for (const field of recommendedFields) {
      if (!packageData[field]) {
        issues.push({
          id: `dep_${file.fsPath}_recommended_${field}`,
          category: ReviewCategory.Dependencies,
          severity: ReviewSeverity.Info,
          title: `Consider adding ${field}`,
          description: `Adding ${field} to package.json improves package discoverability`,
          location: {
            uri: file,
            range: new vscode.Range(0, 0, 0, 0),
            snippet: ''
          },
          rule: 'complete-package-json',
          tags: ['dependencies', 'package.json']
        });
      }
    }
    
    return issues;
  }
  
  /**
   * Check license compatibility
   */
  private checkLicense(file: vscode.Uri, license: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    if (this.restrictedLicenses.some(restricted => license.includes(restricted))) {
      issues.push({
        id: `dep_${file.fsPath}_license`,
        category: ReviewCategory.Dependencies,
        severity: ReviewSeverity.Warning,
        title: 'Potentially restrictive license',
        description: `${license} license may have implications for commercial use`,
        location: {
          uri: file,
          range: new vscode.Range(0, 0, 0, 0),
          snippet: `"license": "${license}"`
        },
        rule: 'license-compatibility',
        tags: ['dependencies', 'license']
      });
    }
    
    return issues;
  }
  
  /**
   * Analyze Python requirements
   */
  private async analyzePythonRequirements(file: vscode.Uri, content: string): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    const lines = content.split('\n');
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (!line || line.startsWith('#')) continue;
      
      // Parse requirement
      const match = line.match(/^([a-zA-Z0-9-_]+)([=<>~!]+)(.+)$/);
      if (match) {
        const [, packageName, operator, version] = match;
        
        // Check for == operator (pinned versions)
        if (operator !== '==') {
          issues.push({
            id: `dep_${file.fsPath}_unpinned_${packageName}`,
            category: ReviewCategory.Dependencies,
            severity: ReviewSeverity.Info,
            title: `Unpinned dependency: ${packageName}`,
            description: 'Consider pinning to specific version for reproducibility',
            location: {
              uri: file,
              range: new vscode.Range(i, 0, i, line.length),
              snippet: line
            },
            rule: 'pin-dependencies',
            tags: ['dependencies', 'python']
          });
        }
        
        // Check for known vulnerabilities
        const vulnInfo = this.checkVulnerability(packageName, version);
        if (vulnInfo) {
          issues.push(this.createVulnerabilityIssue(file, packageName, version, vulnInfo, i));
        }
      }
    }
    
    return issues;
  }
  
  /**
   * Analyze Ruby Gemfile
   */
  private async analyzeGemfile(file: vscode.Uri, content: string): Promise<ReviewIssue[]> {
    const issues: ReviewIssue[] = [];
    const lines = content.split('\n');
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (!line || line.startsWith('#')) continue;
      
      // Parse gem declaration
      const match = line.match(/^gem\s+['"]([^'"]+)['"](?:,\s*['"]([^'"]+)['"])?/);
      if (match) {
        const [, gemName, version] = match;
        
        if (!version) {
          issues.push({
            id: `dep_${file.fsPath}_unversioned_${gemName}`,
            category: ReviewCategory.Dependencies,
            severity: ReviewSeverity.Warning,
            title: `Unversioned gem: ${gemName}`,
            description: 'Specify version constraints for better dependency management',
            location: {
              uri: file,
              range: new vscode.Range(i, 0, i, line.length),
              snippet: line
            },
            rule: 'specify-gem-versions',
            tags: ['dependencies', 'ruby']
          });
        }
      }
    }
    
    return issues;
  }
  
  /**
   * Analyze import statements
   */
  private analyzeImports(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const language = this.detectLanguage(file.fsPath);
    
    // Check for missing error handling on dynamic imports
    if (['javascript', 'typescript'].includes(language)) {
      const dynamicImports = content.match(/import\s*\([^)]+\)/g) || [];
      
      for (const imp of dynamicImports) {
        if (!content.includes('.catch') && !content.includes('try')) {
          issues.push({
            id: `dep_${file.fsPath}_dynamic_import`,
            category: ReviewCategory.Dependencies,
            severity: ReviewSeverity.Warning,
            title: 'Dynamic import without error handling',
            description: 'Dynamic imports can fail and should have error handling',
            location: {
              uri: file,
              range: new vscode.Range(0, 0, 0, 0),
              snippet: imp
            },
            rule: 'handle-dynamic-imports',
            tags: ['dependencies', 'imports']
          });
          break;
        }
      }
      
      // Check for deprecated imports
      const deprecatedImports = [
        { pattern: /from\s+['"]request['"]/, replacement: 'axios or node-fetch' },
        { pattern: /from\s+['"]moment['"]/, replacement: 'date-fns or native Date' }
      ];
      
      for (const deprecated of deprecatedImports) {
        if (deprecated.pattern.test(content)) {
          issues.push({
            id: `dep_${file.fsPath}_deprecated_import`,
            category: ReviewCategory.Dependencies,
            severity: ReviewSeverity.Warning,
            title: 'Importing deprecated package',
            description: `Consider using ${deprecated.replacement} instead`,
            location: {
              uri: file,
              range: new vscode.Range(0, 0, 0, 0),
              snippet: ''
            },
            rule: 'no-deprecated-imports',
            tags: ['dependencies', 'deprecated']
          });
        }
      }
    }
    
    return issues;
  }
  
  /**
   * Check for hardcoded versions
   */
  private checkHardcodedVersions(file: vscode.Uri, content: string): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    
    // Check for hardcoded CDN URLs with versions
    const cdnPattern = /(?:unpkg\.com|jsdelivr\.net|cdnjs\.cloudflare\.com)\/[^@]+@[\d.]+/g;
    const cdnMatches = content.match(cdnPattern) || [];
    
    for (const match of cdnMatches) {
      issues.push({
        id: `dep_${file.fsPath}_hardcoded_cdn`,
        category: ReviewCategory.Dependencies,
        severity: ReviewSeverity.Info,
        title: 'Hardcoded CDN version',
        description: 'Consider using a package manager instead of hardcoded CDN URLs',
        location: {
          uri: file,
          range: new vscode.Range(0, 0, 0, 0),
          snippet: match
        },
        rule: 'no-hardcoded-cdn',
        tags: ['dependencies', 'cdn']
      });
    }
    
    return issues;
  }
  
  /**
   * Check for missing dependencies
   */
  private checkMissingDependencies(
    file: vscode.Uri,
    content: string,
    projectDependencies: string[]
  ): ReviewIssue[] {
    const issues: ReviewIssue[] = [];
    const language = this.detectLanguage(file.fsPath);
    
    if (['javascript', 'typescript'].includes(language)) {
      // Extract imports
      const importPattern = /(?:import|require)\s*(?:\()?['"]([^'"]+)['"]/g;
      const imports = new Set<string>();
      
      let match;
      while ((match = importPattern.exec(content)) !== null) {
        const importPath = match[1];
        if (!importPath.startsWith('.') && !importPath.startsWith('/')) {
          imports.add(importPath.split('/')[0]); // Get package name
        }
      }
      
      // Check if imported packages are in dependencies
      for (const imp of imports) {
        if (!projectDependencies.includes(imp) && !this.isBuiltinModule(imp)) {
          issues.push({
            id: `dep_${file.fsPath}_missing_${imp}`,
            category: ReviewCategory.Dependencies,
            severity: ReviewSeverity.Error,
            title: `Missing dependency: ${imp}`,
            description: `${imp} is imported but not listed in dependencies`,
            location: {
              uri: file,
              range: new vscode.Range(0, 0, 0, 0),
              snippet: ''
            },
            rule: 'missing-dependency',
            tags: ['dependencies', 'missing']
          });
        }
      }
    }
    
    return issues;
  }
  
  /**
   * Create vulnerability issue
   */
  private createVulnerabilityIssue(
    file: vscode.Uri,
    packageName: string,
    version: string,
    vulnerability: VulnerabilityInfo,
    line?: number
  ): ReviewIssue {
    const severityMap = {
      'low': ReviewSeverity.Info,
      'moderate': ReviewSeverity.Warning,
      'high': ReviewSeverity.Error,
      'critical': ReviewSeverity.Critical
    };
    
    return {
      id: `dep_${file.fsPath}_vuln_${packageName}_${vulnerability.cve || Date.now()}`,
      category: ReviewCategory.Dependencies,
      severity: severityMap[vulnerability.severity],
      title: `Vulnerable dependency: ${packageName}@${version}`,
      description: `${vulnerability.title}: ${vulnerability.description}`,
      location: {
        uri: file,
        range: line !== undefined ? 
          new vscode.Range(line, 0, line, 100) : 
          new vscode.Range(0, 0, 0, 0),
        snippet: `${packageName}@${version}`
      },
      rule: 'vulnerable-dependency',
      impact: `Security vulnerability (${vulnerability.severity}): ${vulnerability.cve || 'No CVE'}`,
      effort: 2,
      tags: ['dependencies', 'security', 'vulnerability'],
      metadata: {
        packageName,
        currentVersion: version,
        fixedVersion: vulnerability.fixedIn,
        cve: vulnerability.cve
      }
    };
  }
  
  /**
   * Create deprecation issue
   */
  private createDeprecationIssue(
    file: vscode.Uri,
    packageName: string,
    suggestion: string
  ): ReviewIssue {
    return {
      id: `dep_${file.fsPath}_deprecated_${packageName}`,
      category: ReviewCategory.Dependencies,
      severity: ReviewSeverity.Warning,
      title: `Deprecated package: ${packageName}`,
      description: suggestion,
      location: {
        uri: file,
        range: new vscode.Range(0, 0, 0, 0),
        snippet: packageName
      },
      rule: 'no-deprecated-packages',
      effort: 3,
      tags: ['dependencies', 'deprecated']
    };
  }
  
  /**
   * Check if version matches pattern
   */
  private isVersionMatch(version: string, pattern: string): boolean {
    // Simple version comparison - real implementation would use semver
    if (pattern.startsWith('<')) {
      const targetVersion = pattern.substring(1);
      return this.compareVersions(version, targetVersion) < 0;
    }
    
    if (pattern.startsWith('<=')) {
      const targetVersion = pattern.substring(2);
      return this.compareVersions(version, targetVersion) <= 0;
    }
    
    return false;
  }
  
  /**
   * Compare versions (simplified)
   */
  private compareVersions(v1: string, v2: string): number {
    const cleanV1 = v1.replace(/[^\d.]/g, '');
    const cleanV2 = v2.replace(/[^\d.]/g, '');
    
    const parts1 = cleanV1.split('.').map(Number);
    const parts2 = cleanV2.split('.').map(Number);
    
    for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
      const p1 = parts1[i] || 0;
      const p2 = parts2[i] || 0;
      
      if (p1 < p2) return -1;
      if (p1 > p2) return 1;
    }
    
    return 0;
  }
  
  /**
   * Check if module is built-in
   */
  private isBuiltinModule(moduleName: string): boolean {
    const builtins = [
      'fs', 'path', 'http', 'https', 'crypto', 'os', 'util',
      'stream', 'buffer', 'events', 'url', 'querystring',
      'child_process', 'cluster', 'net', 'dgram', 'dns',
      'readline', 'repl', 'vm', 'assert', 'module', 'process'
    ];
    return builtins.includes(moduleName);
  }
  
  /**
   * Detect language
   */
  private detectLanguage(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const langMap: Record<string, string> = {
      'js': 'javascript',
      'jsx': 'javascript',
      'ts': 'typescript',
      'tsx': 'typescript',
      'py': 'python',
      'rb': 'ruby',
      'java': 'java',
      'go': 'go',
      'rs': 'rust',
      'php': 'php'
    };
    return langMap[ext || ''] || 'unknown';
  }
  
  /**
   * Update configuration
   */
  updateConfig(config: ReviewConfig): void {
    this.config = config;
  }
}
