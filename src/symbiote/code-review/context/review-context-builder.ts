/**
 * Review Context Builder - Builds context for code review analysis
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { Logger } from '../../utils/logger';
import { Neo4jConnectionManager } from '../../knowledge/neo4j/connection-manager';
import { QdrantManager } from '../../search/qdrant/qdrant-manager';

export interface ReviewContext {
  file: vscode.Uri;
  language: string;
  imports: string[];
  exports: string[];
  dependencies: string[];
  references: FileReference[];
  history: GitHistory;
  relatedFiles: vscode.Uri[];
  codeStructure: CodeStructure;
  testCoverage?: number;
  metadata: Record<string, any>;
}

export interface FileReference {
  file: vscode.Uri;
  type: 'import' | 'export' | 'reference';
  line: number;
  symbol?: string;
}

export interface GitHistory {
  lastModified: Date;
  lastModifiedBy: string;
  recentChanges: number;
  hotspots: CodeHotspot[];
}

export interface CodeHotspot {
  lines: [number, number];
  changeFrequency: number;
  lastChanged: Date;
}

export interface CodeStructure {
  classes: ClassInfo[];
  functions: FunctionInfo[];
  complexity: number;
  loc: number;
  comments: number;
}

export interface ClassInfo {
  name: string;
  line: number;
  methods: string[];
  properties: string[];
}

export interface FunctionInfo {
  name: string;
  line: number;
  parameters: string[];
  complexity: number;
  async: boolean;
}

export interface ContextBuildOptions {
  includeReferences?: boolean;
  includeDependencies?: boolean;
  includeHistory?: boolean;
  maxDepth?: number;
  includeTests?: boolean;
}

export class ReviewContextBuilder {
  private logger = new Logger('ReviewContextBuilder');
  private neo4j?: Neo4jConnectionManager;
  private qdrant?: QdrantManager;
  private contextCache = new Map<string, ReviewContext>();
  
  constructor(neo4j?: Neo4jConnectionManager, qdrant?: QdrantManager) {
    this.neo4j = neo4j;
    this.qdrant = qdrant;
  }
  
  /**
   * Build context for a file
   */
  async buildContext(
    file: vscode.Uri,
    options: ContextBuildOptions = {}
  ): Promise<ReviewContext> {
    const cacheKey = `${file.fsPath}_${JSON.stringify(options)}`;
    
    // Check cache
    if (this.contextCache.has(cacheKey)) {
      return this.contextCache.get(cacheKey)!;
    }
    
    try {
      this.logger.info(`Building context for ${file.fsPath}`);
      
      const content = await this.readFile(file);
      if (!content) {
        throw new Error('Unable to read file');
      }
      
      // Build basic context
      const context: ReviewContext = {
        file,
        language: this.detectLanguage(file.fsPath),
        imports: await this.extractImports(content, file.fsPath),
        exports: await this.extractExports(content, file.fsPath),
        dependencies: [],
        references: [],
        history: await this.getGitHistory(file),
        relatedFiles: [],
        codeStructure: await this.analyzeCodeStructure(content, file.fsPath),
        metadata: {}
      };
      
      // Add optional context
      if (options.includeReferences) {
        context.references = await this.findReferences(file, context);
        context.relatedFiles = await this.findRelatedFiles(file, context);
      }
      
      if (options.includeDependencies) {
        context.dependencies = await this.analyzeDependencies(file, context);
      }
      
      if (options.includeHistory && this.neo4j) {
        context.history = await this.getDetailedHistory(file);
      }
      
      if (options.includeTests) {
        context.testCoverage = await this.getTestCoverage(file);
      }
      
      // Get semantic context from Qdrant if available
      if (this.qdrant) {
        const semanticContext = await this.getSemanticContext(file, content);
        context.metadata.semanticContext = semanticContext;
      }
      
      // Cache the context
      this.contextCache.set(cacheKey, context);
      
      return context;
      
    } catch (error) {
      this.logger.error('Failed to build context', error);
      throw error;
    }
  }
  
  /**
   * Read file content
   */
  private async readFile(file: vscode.Uri): Promise<string | null> {
    try {
      const document = await vscode.workspace.openTextDocument(file);
      return document.getText();
    } catch (error) {
      this.logger.error('Failed to read file', error);
      return null;
    }
  }
  
  /**
   * Extract imports from file
   */
  private async extractImports(content: string, filePath: string): Promise<string[]> {
    const imports: string[] = [];
    const language = this.detectLanguage(filePath);
    
    switch (language) {
      case 'javascript':
      case 'typescript':
        // ES6 imports
        const es6Imports = content.match(/import\s+.*?from\s+['"]([^'"]+)['"]/g) || [];
        for (const imp of es6Imports) {
          const match = imp.match(/from\s+['"]([^'"]+)['"]/);;
          if (match) imports.push(match[1]);
        }
        
        // CommonJS requires
        const requireImports = content.match(/require\s*\(['"]([^'"]+)['"]/g) || [];
        for (const req of requireImports) {
          const match = req.match(/require\s*\(['"]([^'"]+)['"]/);;
          if (match) imports.push(match[1]);
        }
        break;
        
      case 'python':
        // Python imports
        const pythonImports = content.match(/(?:from\s+\S+\s+)?import\s+.+/g) || [];
        for (const imp of pythonImports) {
          const fromMatch = imp.match(/from\s+(\S+)\s+import/);
          if (fromMatch) {
            imports.push(fromMatch[1]);
          } else {
            const importMatch = imp.match(/import\s+([\w.]+)/);
            if (importMatch) imports.push(importMatch[1]);
          }
        }
        break;
        
      case 'java':
        // Java imports
        const javaImports = content.match(/import\s+[\w.]+;/g) || [];
        for (const imp of javaImports) {
          const match = imp.match(/import\s+([\w.]+);/);
          if (match) imports.push(match[1]);
        }
        break;
    }
    
    return [...new Set(imports)]; // Remove duplicates
  }
  
  /**
   * Extract exports from file
   */
  private async extractExports(content: string, filePath: string): Promise<string[]> {
    const exports: string[] = [];
    const language = this.detectLanguage(filePath);
    
    switch (language) {
      case 'javascript':
      case 'typescript':
        // Named exports
        const namedExports = content.match(/export\s+(?:const|let|var|function|class)\s+(\w+)/g) || [];
        for (const exp of namedExports) {
          const match = exp.match(/export\s+(?:const|let|var|function|class)\s+(\w+)/);
          if (match) exports.push(match[1]);
        }
        
        // Export statements
        const exportStatements = content.match(/export\s*\{([^}]+)\}/g) || [];
        for (const exp of exportStatements) {
          const match = exp.match(/export\s*\{([^}]+)\}/);
          if (match) {
            const symbols = match[1].split(',').map(s => s.trim().split(' ')[0]);
            exports.push(...symbols);
          }
        }
        
        // Default export
        if (/export\s+default/.test(content)) {
          exports.push('default');
        }
        break;
        
      case 'python':
        // Python __all__
        const allMatch = content.match(/__all__\s*=\s*\[([^\]]+)\]/);
        if (allMatch) {
          const symbols = allMatch[1].match(/['"]([^'"]+)['"]/g) || [];
          for (const sym of symbols) {
            const match = sym.match(/['"]([^'"]+)['"]/);
            if (match) exports.push(match[1]);
          }
        }
        
        // Top-level definitions
        const pythonDefs = content.match(/^(?:def|class)\s+(\w+)/gm) || [];
        for (const def of pythonDefs) {
          const match = def.match(/^(?:def|class)\s+(\w+)/);
          if (match && !match[1].startsWith('_')) {
            exports.push(match[1]);
          }
        }
        break;
    }
    
    return [...new Set(exports)];
  }
  
  /**
   * Find references to this file
   */
  private async findReferences(
    file: vscode.Uri,
    context: ReviewContext
  ): Promise<FileReference[]> {
    const references: FileReference[] = [];
    
    if (!context.exports.length) {
      return references;
    }
    
    // Search for references in workspace
    const searchPattern = context.exports.map(exp => exp).join('|');
    const files = await vscode.workspace.findFiles('**/*.{js,ts,jsx,tsx,py,java}', '**/node_modules/**');
    
    for (const searchFile of files) {
      if (searchFile.fsPath === file.fsPath) continue;
      
      try {
        const content = await this.readFile(searchFile);
        if (!content) continue;
        
        // Check if this file imports from our target file
        const relativeImport = path.relative(path.dirname(searchFile.fsPath), file.fsPath)
          .replace(/\\/g, '/')
          .replace(/\.(ts|js|tsx|jsx)$/, '');
        
        if (content.includes(relativeImport)) {
          // Find specific symbols
          for (const exp of context.exports) {
            const regex = new RegExp(`\\b${exp}\\b`, 'g');
            let match;
            while ((match = regex.exec(content)) !== null) {
              const line = content.substring(0, match.index).split('\n').length - 1;
              references.push({
                file: searchFile,
                type: 'reference',
                line,
                symbol: exp
              });
            }
          }
        }
      } catch (error) {
        // Skip files that can't be read
      }
    }
    
    return references;
  }
  
  /**
   * Find related files
   */
  private async findRelatedFiles(
    file: vscode.Uri,
    context: ReviewContext
  ): Promise<vscode.Uri[]> {
    const related: vscode.Uri[] = [];
    const baseName = path.basename(file.fsPath, path.extname(file.fsPath));
    const dir = path.dirname(file.fsPath);
    
    // Look for test files
    const testPatterns = [
      `${baseName}.test.*`,
      `${baseName}.spec.*`,
      `test_${baseName}.*`,
      `${baseName}_test.*`
    ];
    
    for (const pattern of testPatterns) {
      const testFiles = await vscode.workspace.findFiles(`**/${pattern}`, '**/node_modules/**');
      related.push(...testFiles);
    }
    
    // Look for related files (same name, different extension)
    const relatedPatterns = [
      `${baseName}.d.ts`, // TypeScript definitions
      `${baseName}.css`, // Styles
      `${baseName}.scss`,
      `${baseName}.module.css`,
      `${baseName}.stories.*` // Storybook stories
    ];
    
    for (const pattern of relatedPatterns) {
      const relatedFiles = await vscode.workspace.findFiles(`**/${pattern}`, '**/node_modules/**');
      related.push(...relatedFiles);
    }
    
    // Look for imported files
    for (const imp of context.imports) {
      if (imp.startsWith('.')) {
        const importPath = path.resolve(dir, imp);
        const importUri = vscode.Uri.file(importPath);
        if (await this.fileExists(importUri)) {
          related.push(importUri);
        }
      }
    }
    
    return [...new Set(related.map(f => f.fsPath))].map(f => vscode.Uri.file(f));
  }
  
  /**
   * Analyze dependencies
   */
  private async analyzeDependencies(
    file: vscode.Uri,
    context: ReviewContext
  ): Promise<string[]> {
    const dependencies: string[] = [];
    
    // External dependencies from imports
    for (const imp of context.imports) {
      if (!imp.startsWith('.') && !imp.startsWith('/')) {
        const packageName = imp.split('/')[0];
        if (!this.isBuiltinModule(packageName)) {
          dependencies.push(packageName);
        }
      }
    }
    
    // Look for package.json
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(file);
    if (workspaceFolder) {
      const packageJsonPath = path.join(workspaceFolder.uri.fsPath, 'package.json');
      const packageJsonUri = vscode.Uri.file(packageJsonPath);
      
      if (await this.fileExists(packageJsonUri)) {
        try {
          const packageContent = await this.readFile(packageJsonUri);
          if (packageContent) {
            const packageData = JSON.parse(packageContent);
            const allDeps = {
              ...packageData.dependencies,
              ...packageData.devDependencies
            };
            dependencies.push(...Object.keys(allDeps));
          }
        } catch (error) {
          // Ignore parse errors
        }
      }
    }
    
    return [...new Set(dependencies)];
  }
  
  /**
   * Get git history
   */
  private async getGitHistory(file: vscode.Uri): Promise<GitHistory> {
    // Simplified version - real implementation would use git commands
    return {
      lastModified: new Date(),
      lastModifiedBy: 'unknown',
      recentChanges: 0,
      hotspots: []
    };
  }
  
  /**
   * Get detailed history from Neo4j
   */
  private async getDetailedHistory(file: vscode.Uri): Promise<GitHistory> {
    if (!this.neo4j) {
      return this.getGitHistory(file);
    }
    
    try {
      // Query Neo4j for file history
      const query = `
        MATCH (f:File {path: $path})-[:CHANGED_IN]->(c:Commit)
        WITH f, c ORDER BY c.timestamp DESC
        RETURN f, collect(c) as commits
        LIMIT 100
      `;
      
      const result = await this.neo4j.query(query, { path: file.fsPath });
      
      if (result.length > 0) {
        const commits = result[0].commits;
        const recentCommits = commits.slice(0, 10);
        
        return {
          lastModified: new Date(commits[0].timestamp),
          lastModifiedBy: commits[0].author,
          recentChanges: recentCommits.length,
          hotspots: this.identifyHotspots(commits)
        };
      }
    } catch (error) {
      this.logger.error('Failed to get history from Neo4j', error);
    }
    
    return this.getGitHistory(file);
  }
  
  /**
   * Analyze code structure
   */
  private async analyzeCodeStructure(content: string, filePath: string): Promise<CodeStructure> {
    const language = this.detectLanguage(filePath);
    const lines = content.split('\n');
    
    const structure: CodeStructure = {
      classes: [],
      functions: [],
      complexity: 0,
      loc: lines.length,
      comments: 0
    };
    
    // Count comments
    for (const line of lines) {
      if (line.trim().startsWith('//') || line.trim().startsWith('#')) {
        structure.comments++;
      }
    }
    
    // Extract classes and functions based on language
    switch (language) {
      case 'javascript':
      case 'typescript':
        // Classes
        const classMatches = content.matchAll(/class\s+(\w+)\s*(?:extends\s+\w+\s*)?\{/g);
        for (const match of classMatches) {
          const className = match[1];
          const classLine = content.substring(0, match.index!).split('\n').length - 1;
          
          structure.classes.push({
            name: className,
            line: classLine,
            methods: this.extractClassMethods(content, className),
            properties: []
          });
        }
        
        // Functions
        const funcMatches = content.matchAll(/(?:function\s+(\w+)|const\s+(\w+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>)/g);
        for (const match of funcMatches) {
          const funcName = match[1] || match[2];
          const funcLine = content.substring(0, match.index!).split('\n').length - 1;
          
          structure.functions.push({
            name: funcName,
            line: funcLine,
            parameters: this.extractParameters(match[0]),
            complexity: this.calculateComplexity(content, match.index!),
            async: match[0].includes('async')
          });
        }
        break;
        
      case 'python':
        // Classes
        const pyClassMatches = content.matchAll(/class\s+(\w+)(?:\([^)]*\))?:/g);
        for (const match of pyClassMatches) {
          const className = match[1];
          const classLine = content.substring(0, match.index!).split('\n').length - 1;
          
          structure.classes.push({
            name: className,
            line: classLine,
            methods: this.extractPythonClassMethods(content, className),
            properties: []
          });
        }
        
        // Functions
        const pyFuncMatches = content.matchAll(/def\s+(\w+)\s*\([^)]*\)\s*:/g);
        for (const match of pyFuncMatches) {
          const funcName = match[1];
          const funcLine = content.substring(0, match.index!).split('\n').length - 1;
          
          structure.functions.push({
            name: funcName,
            line: funcLine,
            parameters: this.extractParameters(match[0]),
            complexity: this.calculateComplexity(content, match.index!),
            async: content.substring(match.index! - 20, match.index!).includes('async')
          });
        }
        break;
    }
    
    // Calculate overall complexity
    structure.complexity = structure.functions.reduce((sum, f) => sum + f.complexity, 0) / 
      (structure.functions.length || 1);
    
    return structure;
  }
  
  /**
   * Get test coverage
   */
  private async getTestCoverage(file: vscode.Uri): Promise<number | undefined> {
    // This would integrate with coverage tools
    // For now, return undefined
    return undefined;
  }
  
  /**
   * Get semantic context from Qdrant
   */
  private async getSemanticContext(file: vscode.Uri, content: string): Promise<any> {
    if (!this.qdrant) {
      return null;
    }
    
    try {
      // Get similar code snippets
      const embedding = await this.qdrant.createEmbedding(content.substring(0, 1000));
      const similar = await this.qdrant.search({
        vector: embedding,
        limit: 5,
        filter: {
          must: [
            {
              key: 'type',
              match: { value: 'code' }
            }
          ]
        }
      });
      
      return {
        similarFiles: similar.map(s => s.payload.file),
        patterns: similar.map(s => s.payload.pattern).filter(Boolean)
      };
    } catch (error) {
      this.logger.error('Failed to get semantic context', error);
      return null;
    }
  }
  
  /**
   * Helper methods
   */
  
  private detectLanguage(filePath: string): string {
    const ext = path.extname(filePath).toLowerCase();
    const langMap: Record<string, string> = {
      '.js': 'javascript',
      '.jsx': 'javascript',
      '.ts': 'typescript',
      '.tsx': 'typescript',
      '.py': 'python',
      '.java': 'java',
      '.rb': 'ruby',
      '.go': 'go',
      '.rs': 'rust',
      '.php': 'php',
      '.cpp': 'cpp',
      '.c': 'c',
      '.cs': 'csharp'
    };
    return langMap[ext] || 'unknown';
  }
  
  private async fileExists(uri: vscode.Uri): Promise<boolean> {
    try {
      await vscode.workspace.fs.stat(uri);
      return true;
    } catch {
      return false;
    }
  }
  
  private isBuiltinModule(moduleName: string): boolean {
    const builtins = [
      'fs', 'path', 'http', 'https', 'crypto', 'os', 'util',
      'stream', 'buffer', 'events', 'url', 'querystring',
      'child_process', 'cluster', 'net', 'dgram', 'dns'
    ];
    return builtins.includes(moduleName);
  }
  
  private extractClassMethods(content: string, className: string): string[] {
    const methods: string[] = [];
    const classRegex = new RegExp(`class\\s+${className}[^{]*\\{([^}]+class|[^}]+)\\}`, 's');
    const classMatch = content.match(classRegex);
    
    if (classMatch) {
      const classBody = classMatch[1];
      const methodMatches = classBody.matchAll(/(\w+)\s*\([^)]*\)\s*\{/g);
      for (const match of methodMatches) {
        methods.push(match[1]);
      }
    }
    
    return methods;
  }
  
  private extractPythonClassMethods(content: string, className: string): string[] {
    const methods: string[] = [];
    const lines = content.split('\n');
    let inClass = false;
    let classIndent = 0;
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      
      if (line.includes(`class ${className}`)) {
        inClass = true;
        classIndent = line.search(/\S/);
        continue;
      }
      
      if (inClass) {
        const lineIndent = line.search(/\S/);
        if (lineIndent !== -1 && lineIndent <= classIndent) {
          break; // Left the class
        }
        
        const methodMatch = line.match(/def\s+(\w+)\s*\(/);
        if (methodMatch) {
          methods.push(methodMatch[1]);
        }
      }
    }
    
    return methods;
  }
  
  private extractParameters(funcSignature: string): string[] {
    const paramMatch = funcSignature.match(/\(([^)]*)\)/);
    if (!paramMatch) return [];
    
    return paramMatch[1]
      .split(',')
      .map(p => p.trim())
      .filter(p => p && p !== '...')
      .map(p => p.split(/[:\s=]/)[0]); // Get parameter name only
  }
  
  private calculateComplexity(content: string, startIndex: number): number {
    // Simple cyclomatic complexity calculation
    const funcEnd = content.indexOf('}', startIndex);
    if (funcEnd === -1) return 1;
    
    const funcBody = content.substring(startIndex, funcEnd);
    let complexity = 1;
    
    // Count decision points
    const decisionPatterns = [
      /\bif\b/g,
      /\belse\s+if\b/g,
      /\bwhile\b/g,
      /\bfor\b/g,
      /\bcase\b/g,
      /\bcatch\b/g,
      /\?.*:/g // Ternary
    ];
    
    for (const pattern of decisionPatterns) {
      const matches = funcBody.match(pattern);
      if (matches) {
        complexity += matches.length;
      }
    }
    
    return complexity;
  }
  
  private identifyHotspots(commits: any[]): CodeHotspot[] {
    // Identify frequently changed code sections
    const hotspots: CodeHotspot[] = [];
    const changeMap = new Map<string, number>();
    
    // This is simplified - real implementation would analyze actual changed lines
    for (const commit of commits) {
      if (commit.changedLines) {
        for (const range of commit.changedLines) {
          const key = `${range.start}-${range.end}`;
          changeMap.set(key, (changeMap.get(key) || 0) + 1);
        }
      }
    }
    
    // Convert to hotspots
    for (const [range, frequency] of changeMap) {
      const [start, end] = range.split('-').map(Number);
      if (frequency > 3) { // More than 3 changes
        hotspots.push({
          lines: [start, end],
          changeFrequency: frequency,
          lastChanged: new Date()
        });
      }
    }
    
    return hotspots;
  }
  
  /**
   * Clear context cache
   */
  clearCache(): void {
    this.contextCache.clear();
  }
}
