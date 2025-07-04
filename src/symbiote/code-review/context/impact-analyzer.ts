/**
 * Impact Analyzer - Analyzes the impact of code changes and issues
 */

import * as vscode from 'vscode';
import { Logger } from '../../utils/logger';
import { ReviewIssue, ReviewSeverity } from '../types';
import { Neo4jConnectionManager } from '../../knowledge/neo4j/connection-manager';

export interface ImpactAnalysis {
  description: string;
  affectedFiles: string[];
  affectedComponents: string[];
  affectedFunctions: string[];
  dependentPackages: string[];
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  performanceImpact: number; // -1 to 1
  securityImpact: number; // -1 to 1
  maintainabilityImpact: number; // -1 to 1
  testCoverageImpact: number; // -1 to 1
  estimatedEffort: number; // hours
  propagationPath: PropagationNode[];
}

export interface PropagationNode {
  file: string;
  function?: string;
  type: 'direct' | 'indirect' | 'transitive';
  impact: 'high' | 'medium' | 'low';
}

export class ImpactAnalyzer {
  private logger = new Logger('ImpactAnalyzer');
  private neo4j?: Neo4jConnectionManager;
  private impactCache = new Map<string, ImpactAnalysis>();
  
  constructor(neo4j?: Neo4jConnectionManager) {
    this.neo4j = neo4j;
  }
  
  /**
   * Analyze impact of an issue
   */
  async analyzeImpact(issue: ReviewIssue): Promise<ImpactAnalysis> {
    const cacheKey = `${issue.id}_${issue.location.uri.fsPath}`;
    
    // Check cache
    if (this.impactCache.has(cacheKey)) {
      return this.impactCache.get(cacheKey)!;
    }
    
    try {
      this.logger.info(`Analyzing impact of issue: ${issue.title}`);
      
      // Base impact analysis
      const impact: ImpactAnalysis = {
        description: this.generateImpactDescription(issue),
        affectedFiles: [issue.location.uri.fsPath],
        affectedComponents: [],
        affectedFunctions: [],
        dependentPackages: [],
        riskLevel: this.calculateRiskLevel(issue),
        performanceImpact: this.estimatePerformanceImpact(issue),
        securityImpact: this.estimateSecurityImpact(issue),
        maintainabilityImpact: this.estimateMaintainabilityImpact(issue),
        testCoverageImpact: this.estimateTestCoverageImpact(issue),
        estimatedEffort: issue.effort || 1,
        propagationPath: []
      };
      
      // Enhanced analysis with Neo4j if available
      if (this.neo4j) {
        const enhancedImpact = await this.performGraphAnalysis(issue);
        Object.assign(impact, enhancedImpact);
      } else {
        // Fallback to file-based analysis
        const localImpact = await this.performLocalAnalysis(issue);
        Object.assign(impact, localImpact);
      }
      
      // Adjust risk level based on propagation
      if (impact.affectedFiles.length > 5 || impact.affectedComponents.length > 3) {
        impact.riskLevel = this.increaseRiskLevel(impact.riskLevel);
      }
      
      // Cache the result
      this.impactCache.set(cacheKey, impact);
      
      return impact;
      
    } catch (error) {
      this.logger.error('Impact analysis failed', error);
      
      // Return basic impact analysis
      return {
        description: `Unable to fully analyze impact: ${error.message}`,
        affectedFiles: [issue.location.uri.fsPath],
        affectedComponents: [],
        affectedFunctions: [],
        dependentPackages: [],
        riskLevel: 'medium',
        performanceImpact: 0,
        securityImpact: 0,
        maintainabilityImpact: -0.1,
        testCoverageImpact: 0,
        estimatedEffort: issue.effort || 1,
        propagationPath: []
      };
    }
  }
  
  /**
   * Perform graph-based analysis using Neo4j
   */
  private async performGraphAnalysis(issue: ReviewIssue): Promise<Partial<ImpactAnalysis>> {
    if (!this.neo4j) {
      return {};
    }
    
    try {
      // Find all files that depend on this file
      const dependencyQuery = `
        MATCH (f:File {path: $filePath})
        MATCH (f)<-[:IMPORTS]-(dependent:File)
        WITH dependent
        MATCH (dependent)<-[:IMPORTS*0..2]-(transitive:File)
        RETURN DISTINCT transitive.path as path, 
               size((transitive)<-[:IMPORTS*0..2]-()) as dependencyDepth
        ORDER BY dependencyDepth
        LIMIT 50
      `;
      
      const dependencies = await this.neo4j.query(dependencyQuery, {
        filePath: issue.location.uri.fsPath
      });
      
      const affectedFiles = dependencies.map(d => d.path);
      
      // Find affected components
      const componentQuery = `
        MATCH (f:File {path: $filePath})-[:CONTAINS]->(fn:Function)
        WHERE fn.line >= $startLine AND fn.line <= $endLine
        MATCH (fn)<-[:CALLS*1..3]-(caller:Function)<-[:CONTAINS]-(callerFile:File)
        WITH DISTINCT callerFile, caller
        MATCH (callerFile)-[:BELONGS_TO]->(c:Component)
        RETURN DISTINCT c.name as component, 
               collect(DISTINCT caller.name) as functions,
               collect(DISTINCT callerFile.path) as files
        LIMIT 20
      `;
      
      const components = await this.neo4j.query(componentQuery, {
        filePath: issue.location.uri.fsPath,
        startLine: issue.location.range.start.line,
        endLine: issue.location.range.end.line
      });
      
      const affectedComponents = components.map(c => c.component);
      const affectedFunctions = components.flatMap(c => c.functions);
      
      // Build propagation path
      const propagationPath = await this.buildPropagationPath(
        issue.location.uri.fsPath,
        affectedFiles,
        dependencies
      );
      
      // Find dependent packages
      const packageQuery = `
        MATCH (f:File {path: $filePath})
        MATCH (f)-[:EXPORTS]->(symbol:Symbol)
        MATCH (symbol)<-[:USES]-(consumer:File)
        MATCH (consumer)-[:BELONGS_TO]->(pkg:Package)
        WHERE pkg.name <> (f)-[:BELONGS_TO]->(:Package).name
        RETURN DISTINCT pkg.name as package
      `;
      
      const packages = await this.neo4j.query(packageQuery, {
        filePath: issue.location.uri.fsPath
      });
      
      const dependentPackages = packages.map(p => p.package);
      
      return {
        affectedFiles,
        affectedComponents,
        affectedFunctions,
        dependentPackages,
        propagationPath
      };
      
    } catch (error) {
      this.logger.error('Graph analysis failed', error);
      return {};
    }
  }
  
  /**
   * Perform local file-based analysis
   */
  private async performLocalAnalysis(issue: ReviewIssue): Promise<Partial<ImpactAnalysis>> {
    const affectedFiles: string[] = [issue.location.uri.fsPath];
    const affectedFunctions: string[] = [];
    
    try {
      // Search for files that import this file
      const fileName = vscode.workspace.asRelativePath(issue.location.uri);
      const searchPattern = fileName.replace(/\.(ts|js|tsx|jsx)$/, '');
      
      const files = await vscode.workspace.findFiles(
        '**/*.{js,ts,jsx,tsx}',
        '**/node_modules/**'
      );
      
      for (const file of files) {
        if (file.fsPath === issue.location.uri.fsPath) continue;
        
        try {
          const document = await vscode.workspace.openTextDocument(file);
          const content = document.getText();
          
          // Check if this file imports the affected file
          if (content.includes(searchPattern)) {
            affectedFiles.push(file.fsPath);
            
            // Try to find affected functions
            const functionPattern = /(?:function|const|let|var)\s+(\w+)/g;
            let match;
            while ((match = functionPattern.exec(content)) !== null) {
              if (content.substring(match.index - 100, match.index).includes(searchPattern)) {
                affectedFunctions.push(match[1]);
              }
            }
          }
        } catch (error) {
          // Skip files that can't be read
        }
      }
      
      // Build simple propagation path
      const propagationPath: PropagationNode[] = affectedFiles.slice(1, 6).map(file => ({
        file,
        type: 'direct' as const,
        impact: 'medium' as const
      }));
      
      return {
        affectedFiles,
        affectedFunctions,
        propagationPath
      };
      
    } catch (error) {
      this.logger.error('Local analysis failed', error);
      return {};
    }
  }
  
  /**
   * Generate impact description
   */
  private generateImpactDescription(issue: ReviewIssue): string {
    const categoryDescriptions = {
      security: 'This security issue could expose sensitive data or create vulnerabilities',
      performance: 'This performance issue could degrade user experience',
      'best-practices': 'This code quality issue could affect maintainability',
      maintainability: 'This issue makes the code harder to understand and modify',
      accessibility: 'This accessibility issue could prevent some users from using the feature',
      testing: 'This testing issue reduces confidence in code correctness',
      documentation: 'This documentation issue makes the code harder to use',
      dependencies: 'This dependency issue could affect stability or security',
      architecture: 'This architectural issue could affect system scalability',
      style: 'This style issue affects code consistency'
    };
    
    const baseDescription = categoryDescriptions[issue.category] || 
      'This issue could affect code quality';
    
    const severityAddition = {
      critical: ' and requires immediate attention',
      error: ' and should be fixed soon',
      warning: ' and should be addressed',
      info: ' but has minimal impact'
    };
    
    return baseDescription + severityAddition[issue.severity];
  }
  
  /**
   * Calculate risk level
   */
  private calculateRiskLevel(issue: ReviewIssue): 'low' | 'medium' | 'high' | 'critical' {
    // Base risk on severity
    const severityRisk = {
      [ReviewSeverity.Critical]: 'critical',
      [ReviewSeverity.Error]: 'high',
      [ReviewSeverity.Warning]: 'medium',
      [ReviewSeverity.Info]: 'low'
    };
    
    let risk = severityRisk[issue.severity] as 'low' | 'medium' | 'high' | 'critical';
    
    // Adjust based on category
    if (issue.category === 'security' && risk !== 'critical') {
      risk = this.increaseRiskLevel(risk);
    }
    
    if (issue.category === 'performance' && issue.location.uri.fsPath.includes('critical')) {
      risk = this.increaseRiskLevel(risk);
    }
    
    return risk;
  }
  
  /**
   * Estimate performance impact
   */
  private estimatePerformanceImpact(issue: ReviewIssue): number {
    if (issue.category !== 'performance') {
      return 0;
    }
    
    const impactMap = {
      [ReviewSeverity.Critical]: -0.8,
      [ReviewSeverity.Error]: -0.5,
      [ReviewSeverity.Warning]: -0.3,
      [ReviewSeverity.Info]: -0.1
    };
    
    return impactMap[issue.severity];
  }
  
  /**
   * Estimate security impact
   */
  private estimateSecurityImpact(issue: ReviewIssue): number {
    if (issue.category !== 'security') {
      return 0;
    }
    
    const impactMap = {
      [ReviewSeverity.Critical]: -1.0,
      [ReviewSeverity.Error]: -0.7,
      [ReviewSeverity.Warning]: -0.4,
      [ReviewSeverity.Info]: -0.2
    };
    
    return impactMap[issue.severity];
  }
  
  /**
   * Estimate maintainability impact
   */
  private estimateMaintainabilityImpact(issue: ReviewIssue): number {
    const categoryImpact = {
      'best-practices': -0.3,
      'maintainability': -0.5,
      'documentation': -0.2,
      'style': -0.1,
      'architecture': -0.4
    };
    
    const baseImpact = categoryImpact[issue.category] || -0.1;
    
    // Adjust based on severity
    const severityMultiplier = {
      [ReviewSeverity.Critical]: 2.0,
      [ReviewSeverity.Error]: 1.5,
      [ReviewSeverity.Warning]: 1.0,
      [ReviewSeverity.Info]: 0.5
    };
    
    return baseImpact * severityMultiplier[issue.severity];
  }
  
  /**
   * Estimate test coverage impact
   */
  private estimateTestCoverageImpact(issue: ReviewIssue): number {
    if (issue.category === 'testing') {
      const impactMap = {
        [ReviewSeverity.Critical]: -0.5,
        [ReviewSeverity.Error]: -0.3,
        [ReviewSeverity.Warning]: -0.2,
        [ReviewSeverity.Info]: -0.1
      };
      return impactMap[issue.severity];
    }
    
    // Other issues might indirectly affect test coverage
    if (issue.title.toLowerCase().includes('test') || 
        issue.description.toLowerCase().includes('test')) {
      return -0.1;
    }
    
    return 0;
  }
  
  /**
   * Build propagation path
   */
  private async buildPropagationPath(
    originFile: string,
    affectedFiles: string[],
    dependencies: any[]
  ): Promise<PropagationNode[]> {
    const path: PropagationNode[] = [];
    
    // Add direct dependencies
    for (const dep of dependencies.slice(0, 3)) {
      path.push({
        file: dep.path,
        type: dep.dependencyDepth === 0 ? 'direct' : 'indirect',
        impact: dep.dependencyDepth === 0 ? 'high' : 'medium'
      });
    }
    
    // Add transitive dependencies
    for (const dep of dependencies.slice(3, 6)) {
      path.push({
        file: dep.path,
        type: 'transitive',
        impact: 'low'
      });
    }
    
    return path;
  }
  
  /**
   * Increase risk level
   */
  private increaseRiskLevel(
    current: 'low' | 'medium' | 'high' | 'critical'
  ): 'low' | 'medium' | 'high' | 'critical' {
    const levels = ['low', 'medium', 'high', 'critical'];
    const currentIndex = levels.indexOf(current);
    return levels[Math.min(currentIndex + 1, levels.length - 1)] as any;
  }
  
  /**
   * Clear impact cache
   */
  clearCache(): void {
    this.impactCache.clear();
  }
}
