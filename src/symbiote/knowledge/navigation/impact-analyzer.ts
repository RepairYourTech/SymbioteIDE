/**
 * Impact Analyzer
 * 
 * Analyze the impact of code changes and breaking changes
 */

import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import {
  GraphNode,
  NodeType,
  RelationType,
  FileNodeProperties,
  ClassNodeProperties,
  FunctionNodeProperties,
  VariableNodeProperties
} from '../types/graph-types';

export interface BreakingChange {
  type: 'signature' | 'removal' | 'rename' | 'type' | 'visibility' | 'behavior';
  description: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  location: {
    file: string;
    line: number;
    nodeId: string;
  };
  affects: string[];  // Node IDs that are affected
}

export interface ImpactedEntity {
  nodeId: string;
  name: string;
  type: NodeType;
  filePath: string;
  line?: number;
  impactType: 'direct' | 'transitive';
  changeRequired: boolean;
  changeDescription?: string;
}

export interface BreakingChangeAnalysis {
  entity: {
    id: string;
    name: string;
    type: NodeType;
    filePath: string;
  };
  breakingChanges: BreakingChange[];
  impactedEntities: ImpactedEntity[];
  affectedFiles: number;
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  recommendations: string[];
}

export interface RefactoringImpact {
  safeToRefactor: boolean;
  dependencies: ImpactedEntity[];
  tests: ImpactedEntity[];
  publicAPI: boolean;
  usageCount: number;
  complexityScore: number;
  recommendations: string[];
}

export class ImpactAnalyzer {
  constructor(private connectionManager: Neo4jConnectionManager) {}
  
  /**
   * Analyze breaking changes for an entity
   */
  async analyzeBreakingChanges(
    nodeId: string,
    databaseId: string,
    proposedChanges?: {
      type?: string;
      signature?: string;
      visibility?: string;
    }
  ): Promise<BreakingChangeAnalysis> {
    // Get entity details
    const entityResult = await this.connectionManager.executeQuery(
      databaseId,
      'MATCH (n {id: $id}) RETURN n, labels(n) as labels',
      { id: nodeId }
    );
    
    if (entityResult.records.length === 0) {
      throw new Error('Entity not found');
    }
    
    const node = entityResult.records[0].get('n');
    const labels = entityResult.records[0].get('labels');
    const nodeType = labels[0] as NodeType;
    const properties = node.properties;
    
    const entity = {
      id: nodeId,
      name: properties.name,
      type: nodeType,
      filePath: properties.filePath || properties.path
    };
    
    // Analyze different types of breaking changes
    const breakingChanges: BreakingChange[] = [];
    const impactedEntities: ImpactedEntity[] = [];
    
    // Check for different types of impacts based on node type
    switch (nodeType) {
      case NodeType.FUNCTION:
      case NodeType.METHOD:
        await this.analyzeFunctionBreakingChanges(
          nodeId,
          databaseId,
          properties,
          proposedChanges,
          breakingChanges,
          impactedEntities
        );
        break;
        
      case NodeType.CLASS:
      case NodeType.INTERFACE:
        await this.analyzeClassBreakingChanges(
          nodeId,
          databaseId,
          properties,
          proposedChanges,
          breakingChanges,
          impactedEntities
        );
        break;
        
      case NodeType.VARIABLE:
      case NodeType.CONSTANT:
        await this.analyzeVariableBreakingChanges(
          nodeId,
          databaseId,
          properties,
          proposedChanges,
          breakingChanges,
          impactedEntities
        );
        break;
    }
    
    // Get all affected files
    const affectedFiles = new Set(
      impactedEntities.map(e => e.filePath).filter(Boolean)
    ).size;
    
    // Calculate risk level
    const riskLevel = this.calculateRiskLevel(
      breakingChanges,
      impactedEntities.length,
      affectedFiles
    );
    
    // Generate recommendations
    const recommendations = this.generateRecommendations(
      nodeType,
      breakingChanges,
      impactedEntities
    );
    
    return {
      entity,
      breakingChanges,
      impactedEntities,
      affectedFiles,
      riskLevel,
      recommendations
    };
  }
  
  /**
   * Analyze refactoring impact
   */
  async analyzeRefactoringImpact(
    nodeId: string,
    databaseId: string,
    refactoringType: 'rename' | 'extract' | 'inline' | 'move'
  ): Promise<RefactoringImpact> {
    // Get all dependencies
    const dependencyQuery = `
      MATCH (n {id: $id})<-[:REFERENCES|CALLS|USES|IMPORTS|EXTENDS|IMPLEMENTS]-(dep)
      RETURN dep, labels(dep) as labels, type(r) as relType
    `;
    
    const testQuery = `
      MATCH (n {id: $id})<-[:TESTS]-(test)
      RETURN test, labels(test) as labels
    `;
    
    const [depResult, testResult] = await Promise.all([
      this.connectionManager.executeQuery(databaseId, dependencyQuery, { id: nodeId }),
      this.connectionManager.executeQuery(databaseId, testQuery, { id: nodeId })
    ]);
    
    const dependencies: ImpactedEntity[] = [];
    const tests: ImpactedEntity[] = [];
    
    // Process dependencies
    depResult.records.forEach(record => {
      const node = record.get('dep');
      const labels = record.get('labels');
      const relType = record.get('relType');
      
      dependencies.push({
        nodeId: node.properties.id,
        name: node.properties.name,
        type: labels[0] as NodeType,
        filePath: node.properties.filePath || node.properties.path,
        line: node.properties.startLine || node.properties.line,
        impactType: 'direct',
        changeRequired: true,
        changeDescription: `Update ${relType.toLowerCase()} reference`
      });
    });
    
    // Process tests
    testResult.records.forEach(record => {
      const node = record.get('test');
      const labels = record.get('labels');
      
      tests.push({
        nodeId: node.properties.id,
        name: node.properties.name,
        type: labels[0] as NodeType,
        filePath: node.properties.filePath || node.properties.path,
        line: node.properties.startLine || node.properties.line,
        impactType: 'direct',
        changeRequired: true,
        changeDescription: 'Update test to reflect refactoring'
      });
    });
    
    // Check if part of public API
    const publicAPIQuery = `
      MATCH (n {id: $id})
      WHERE exists(n.isExported) AND n.isExported = true
      OR exists(n.visibility) AND n.visibility = 'public'
      RETURN count(n) > 0 as isPublic
    `;
    
    const publicResult = await this.connectionManager.executeQuery(
      databaseId,
      publicAPIQuery,
      { id: nodeId }
    );
    
    const publicAPI = publicResult.records[0]?.get('isPublic') || false;
    
    // Calculate complexity score
    const complexityScore = this.calculateRefactoringComplexity(
      refactoringType,
      dependencies.length,
      tests.length,
      publicAPI
    );
    
    // Determine if safe to refactor
    const safeToRefactor = complexityScore < 7 && !publicAPI;
    
    // Generate recommendations
    const recommendations = this.generateRefactoringRecommendations(
      refactoringType,
      dependencies,
      tests,
      publicAPI,
      complexityScore
    );
    
    return {
      safeToRefactor,
      dependencies,
      tests,
      publicAPI,
      usageCount: dependencies.length,
      complexityScore,
      recommendations
    };
  }
  
  /**
   * Find cascading impacts
   */
  async findCascadingImpacts(
    nodeId: string,
    databaseId: string,
    maxDepth: number = 3
  ): Promise<{
    levels: ImpactedEntity[][];
    totalImpact: number;
    criticalPaths: string[][];
  }> {
    const levels: ImpactedEntity[][] = [];
    const visited = new Set<string>();
    const criticalPaths: string[][] = [];
    
    // BFS to find cascading impacts
    let currentLevel = [nodeId];
    let depth = 0;
    
    while (currentLevel.length > 0 && depth < maxDepth) {
      const nextLevel: string[] = [];
      const levelEntities: ImpactedEntity[] = [];
      
      for (const currentId of currentLevel) {
        if (visited.has(currentId)) continue;
        visited.add(currentId);
        
        // Find all entities that depend on current
        const query = `
          MATCH (n {id: $id})<-[:REFERENCES|CALLS|USES|IMPORTS|EXTENDS|IMPLEMENTS]-(dep)
          RETURN dep, labels(dep) as labels
        `;
        
        const result = await this.connectionManager.executeQuery(
          databaseId,
          query,
          { id: currentId }
        );
        
        result.records.forEach(record => {
          const node = record.get('dep');
          const labels = record.get('labels');
          const depId = node.properties.id;
          
          if (!visited.has(depId)) {
            nextLevel.push(depId);
            
            levelEntities.push({
              nodeId: depId,
              name: node.properties.name,
              type: labels[0] as NodeType,
              filePath: node.properties.filePath || node.properties.path,
              line: node.properties.startLine || node.properties.line,
              impactType: depth === 0 ? 'direct' : 'transitive',
              changeRequired: depth === 0
            });
          }
        });
      }
      
      if (levelEntities.length > 0) {
        levels.push(levelEntities);
      }
      
      currentLevel = nextLevel;
      depth++;
    }
    
    // Find critical paths (paths that affect many entities)
    if (levels.length > 1) {
      // Simplified critical path detection
      const impactCounts = new Map<string, number>();
      
      levels.forEach((level, index) => {
        level.forEach(entity => {
          const count = impactCounts.get(entity.nodeId) || 0;
          impactCounts.set(entity.nodeId, count + (levels.length - index));
        });
      });
      
      // Get top critical nodes
      const criticalNodes = Array.from(impactCounts.entries())
        .sort((a, b) => b[1] - a[1])
        .slice(0, 5)
        .map(([nodeId]) => nodeId);
      
      // Build paths (simplified)
      criticalNodes.forEach(criticalNode => {
        criticalPaths.push([nodeId, criticalNode]);
      });
    }
    
    const totalImpact = levels.reduce((sum, level) => sum + level.length, 0);
    
    return {
      levels,
      totalImpact,
      criticalPaths
    };
  }
  
  /**
   * Analyze test coverage impact
   */
  async analyzeTestCoverageImpact(
    nodeId: string,
    databaseId: string
  ): Promise<{
    directTests: ImpactedEntity[];
    indirectTests: ImpactedEntity[];
    uncoveredDependencies: ImpactedEntity[];
    coverageRatio: number;
  }> {
    // Find direct tests
    const directTestQuery = `
      MATCH (n {id: $id})<-[:TESTS]-(test)
      RETURN test, labels(test) as labels
    `;
    
    // Find indirect tests (tests that test entities depending on this)
    const indirectTestQuery = `
      MATCH (n {id: $id})<-[:REFERENCES|CALLS|USES]-(dep)<-[:TESTS]-(test)
      RETURN DISTINCT test, labels(test) as labels
    `;
    
    // Find dependencies without tests
    const uncoveredQuery = `
      MATCH (n {id: $id})<-[:REFERENCES|CALLS|USES]-(dep)
      WHERE NOT exists((dep)<-[:TESTS]-())
      RETURN dep, labels(dep) as labels
    `;
    
    const [directResult, indirectResult, uncoveredResult] = await Promise.all([
      this.connectionManager.executeQuery(databaseId, directTestQuery, { id: nodeId }),
      this.connectionManager.executeQuery(databaseId, indirectTestQuery, { id: nodeId }),
      this.connectionManager.executeQuery(databaseId, uncoveredQuery, { id: nodeId })
    ]);
    
    const directTests = this.mapToImpactedEntities(directResult, 'direct');
    const indirectTests = this.mapToImpactedEntities(indirectResult, 'transitive');
    const uncoveredDependencies = this.mapToImpactedEntities(uncoveredResult, 'direct');
    
    // Calculate coverage ratio
    const totalDependencies = uncoveredDependencies.length + 
      (directTests.length > 0 ? 1 : 0) + 
      indirectTests.length;
    
    const coveredDependencies = (directTests.length > 0 ? 1 : 0) + indirectTests.length;
    
    const coverageRatio = totalDependencies > 0 
      ? coveredDependencies / totalDependencies 
      : 0;
    
    return {
      directTests,
      indirectTests,
      uncoveredDependencies,
      coverageRatio
    };
  }
  
  // Private helper methods
  
  private async analyzeFunctionBreakingChanges(
    nodeId: string,
    databaseId: string,
    properties: FunctionNodeProperties,
    proposedChanges: any,
    breakingChanges: BreakingChange[],
    impactedEntities: ImpactedEntity[]
  ): Promise<void> {
    // Check signature changes
    if (proposedChanges?.signature && proposedChanges.signature !== properties.signature) {
      breakingChanges.push({
        type: 'signature',
        description: `Function signature changed from "${properties.signature}" to "${proposedChanges.signature}"`,
        severity: 'high',
        location: {
          file: properties.filePath,
          line: properties.startLine,
          nodeId
        },
        affects: []
      });
    }
    
    // Find all callers
    const callerQuery = `
      MATCH (n {id: $id})<-[r:CALLS]-(caller)
      RETURN caller, labels(caller) as labels, r
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      callerQuery,
      { id: nodeId }
    );
    
    result.records.forEach(record => {
      const caller = record.get('caller');
      const labels = record.get('labels');
      const relationship = record.get('r');
      
      impactedEntities.push({
        nodeId: caller.properties.id,
        name: caller.properties.name,
        type: labels[0] as NodeType,
        filePath: caller.properties.filePath || caller.properties.path,
        line: caller.properties.startLine || caller.properties.line,
        impactType: 'direct',
        changeRequired: true,
        changeDescription: 'Update function call to match new signature'
      });
      
      // Track affected nodes in breaking changes
      breakingChanges.forEach(bc => {
        if (bc.type === 'signature') {
          bc.affects.push(caller.properties.id);
        }
      });
    });
  }
  
  private async analyzeClassBreakingChanges(
    nodeId: string,
    databaseId: string,
    properties: ClassNodeProperties,
    proposedChanges: any,
    breakingChanges: BreakingChange[],
    impactedEntities: ImpactedEntity[]
  ): Promise<void> {
    // Check for inheritance breaking changes
    const inheritanceQuery = `
      MATCH (n {id: $id})<-[:EXTENDS|IMPLEMENTS]-(child)
      RETURN child, labels(child) as labels
    `;
    
    const instantiationQuery = `
      MATCH (n {id: $id})<-[:INSTANTIATES]-(user)
      RETURN user, labels(user) as labels
    `;
    
    const [inheritanceResult, instantiationResult] = await Promise.all([
      this.connectionManager.executeQuery(databaseId, inheritanceQuery, { id: nodeId }),
      this.connectionManager.executeQuery(databaseId, instantiationQuery, { id: nodeId })
    ]);
    
    // Process inheritance impacts
    inheritanceResult.records.forEach(record => {
      const child = record.get('child');
      const labels = record.get('labels');
      
      impactedEntities.push({
        nodeId: child.properties.id,
        name: child.properties.name,
        type: labels[0] as NodeType,
        filePath: child.properties.filePath || child.properties.path,
        line: child.properties.startLine || child.properties.line,
        impactType: 'direct',
        changeRequired: true,
        changeDescription: 'Update inheritance to reflect parent changes'
      });
    });
    
    // Process instantiation impacts
    instantiationResult.records.forEach(record => {
      const user = record.get('user');
      const labels = record.get('labels');
      
      impactedEntities.push({
        nodeId: user.properties.id,
        name: user.properties.name,
        type: labels[0] as NodeType,
        filePath: user.properties.filePath || user.properties.path,
        line: user.properties.startLine || user.properties.line,
        impactType: 'direct',
        changeRequired: true,
        changeDescription: 'Update class instantiation'
      });
    });
    
    // Check for interface contract changes
    if (properties.isAbstract || properties.isInterface) {
      breakingChanges.push({
        type: 'behavior',
        description: 'Changes to abstract class or interface affect all implementations',
        severity: 'critical',
        location: {
          file: properties.filePath,
          line: properties.startLine,
          nodeId
        },
        affects: impactedEntities.map(e => e.nodeId)
      });
    }
  }
  
  private async analyzeVariableBreakingChanges(
    nodeId: string,
    databaseId: string,
    properties: VariableNodeProperties,
    proposedChanges: any,
    breakingChanges: BreakingChange[],
    impactedEntities: ImpactedEntity[]
  ): Promise<void> {
    // Check type changes
    if (proposedChanges?.type && proposedChanges.type !== properties.type) {
      breakingChanges.push({
        type: 'type',
        description: `Variable type changed from "${properties.type}" to "${proposedChanges.type}"`,
        severity: 'medium',
        location: {
          file: properties.filePath,
          line: properties.line,
          nodeId
        },
        affects: []
      });
    }
    
    // Find all references
    const referenceQuery = `
      MATCH (n {id: $id})<-[:REFERENCES|USES]-(ref)
      RETURN ref, labels(ref) as labels
    `;
    
    const result = await this.connectionManager.executeQuery(
      databaseId,
      referenceQuery,
      { id: nodeId }
    );
    
    result.records.forEach(record => {
      const ref = record.get('ref');
      const labels = record.get('labels');
      
      impactedEntities.push({
        nodeId: ref.properties.id,
        name: ref.properties.name,
        type: labels[0] as NodeType,
        filePath: ref.properties.filePath || ref.properties.path,
        line: ref.properties.startLine || ref.properties.line,
        impactType: 'direct',
        changeRequired: proposedChanges?.type !== undefined,
        changeDescription: proposedChanges?.type ? 'Update to handle new variable type' : undefined
      });
    });
  }
  
  private calculateRiskLevel(
    breakingChanges: BreakingChange[],
    impactedCount: number,
    affectedFiles: number
  ): 'low' | 'medium' | 'high' | 'critical' {
    // Check for critical severity changes
    if (breakingChanges.some(bc => bc.severity === 'critical')) {
      return 'critical';
    }
    
    // Calculate risk score
    let riskScore = 0;
    
    // Breaking changes contribute to risk
    breakingChanges.forEach(bc => {
      switch (bc.severity) {
        case 'high': riskScore += 3; break;
        case 'medium': riskScore += 2; break;
        case 'low': riskScore += 1; break;
      }
    });
    
    // Scale of impact contributes to risk
    if (impactedCount > 50) riskScore += 3;
    else if (impactedCount > 20) riskScore += 2;
    else if (impactedCount > 5) riskScore += 1;
    
    if (affectedFiles > 20) riskScore += 2;
    else if (affectedFiles > 10) riskScore += 1;
    
    // Determine risk level
    if (riskScore >= 8) return 'critical';
    if (riskScore >= 5) return 'high';
    if (riskScore >= 2) return 'medium';
    return 'low';
  }
  
  private generateRecommendations(
    nodeType: NodeType,
    breakingChanges: BreakingChange[],
    impactedEntities: ImpactedEntity[]
  ): string[] {
    const recommendations: string[] = [];
    
    // General recommendations
    if (breakingChanges.length > 0) {
      recommendations.push('Consider deprecating the old API before removing it');
      recommendations.push('Document all breaking changes in release notes');
    }
    
    if (impactedEntities.length > 10) {
      recommendations.push('Consider creating a migration guide for affected code');
      recommendations.push('Phase the changes over multiple releases');
    }
    
    // Type-specific recommendations
    switch (nodeType) {
      case NodeType.FUNCTION:
      case NodeType.METHOD:
        if (breakingChanges.some(bc => bc.type === 'signature')) {
          recommendations.push('Consider function overloading to maintain backward compatibility');
          recommendations.push('Add parameter validation to handle old calling patterns');
        }
        break;
        
      case NodeType.CLASS:
      case NodeType.INTERFACE:
        recommendations.push('Consider using the adapter pattern for compatibility');
        if (impactedEntities.some(e => e.type === NodeType.CLASS)) {
          recommendations.push('Ensure all child classes are updated');
        }
        break;
    }
    
    // Test recommendations
    const testCount = impactedEntities.filter(e => 
      e.name.includes('test') || e.name.includes('spec')
    ).length;
    
    if (testCount === 0) {
      recommendations.push('Add tests before making breaking changes');
    } else {
      recommendations.push(`Update ${testCount} test files to reflect changes`);
    }
    
    return recommendations;
  }
  
  private generateRefactoringRecommendations(
    refactoringType: string,
    dependencies: ImpactedEntity[],
    tests: ImpactedEntity[],
    publicAPI: boolean,
    complexityScore: number
  ): string[] {
    const recommendations: string[] = [];
    
    // Complexity-based recommendations
    if (complexityScore > 8) {
      recommendations.push('Consider breaking this refactoring into smaller steps');
      recommendations.push('Create a detailed refactoring plan before proceeding');
    }
    
    // Public API recommendations
    if (publicAPI) {
      recommendations.push('This is part of the public API - consider deprecation first');
      recommendations.push('Notify API consumers about upcoming changes');
    }
    
    // Test recommendations
    if (tests.length === 0) {
      recommendations.push('Add tests before refactoring to ensure behavior is preserved');
    } else {
      recommendations.push(`Update ${tests.length} test files after refactoring`);
    }
    
    // Type-specific recommendations
    switch (refactoringType) {
      case 'rename':
        recommendations.push('Use find-and-replace with regex for consistent renaming');
        recommendations.push('Update documentation and comments');
        break;
        
      case 'extract':
        recommendations.push('Ensure extracted code is cohesive and reusable');
        recommendations.push('Consider the single responsibility principle');
        break;
        
      case 'move':
        recommendations.push('Update import statements in all dependent files');
        recommendations.push('Consider impact on module boundaries');
        break;
    }
    
    // Dependency recommendations
    if (dependencies.length > 20) {
      recommendations.push('Use automated refactoring tools when available');
      recommendations.push('Run tests frequently during refactoring');
    }
    
    return recommendations;
  }
  
  private calculateRefactoringComplexity(
    refactoringType: string,
    dependencyCount: number,
    testCount: number,
    isPublicAPI: boolean
  ): number {
    let complexity = 0;
    
    // Base complexity by type
    switch (refactoringType) {
      case 'rename': complexity = 2; break;
      case 'extract': complexity = 4; break;
      case 'inline': complexity = 3; break;
      case 'move': complexity = 5; break;
    }
    
    // Scale by dependencies
    if (dependencyCount > 50) complexity += 4;
    else if (dependencyCount > 20) complexity += 3;
    else if (dependencyCount > 10) complexity += 2;
    else if (dependencyCount > 5) complexity += 1;
    
    // Public API adds significant complexity
    if (isPublicAPI) complexity += 3;
    
    // Lack of tests increases risk
    if (testCount === 0) complexity += 2;
    
    return Math.min(complexity, 10); // Cap at 10
  }
  
  private mapToImpactedEntities(
    result: any,
    impactType: 'direct' | 'transitive'
  ): ImpactedEntity[] {
    return result.records.map((record: any) => {
      const node = record.get(record.keys[0]);
      const labels = record.get('labels');
      
      return {
        nodeId: node.properties.id,
        name: node.properties.name,
        type: labels[0] as NodeType,
        filePath: node.properties.filePath || node.properties.path,
        line: node.properties.startLine || node.properties.line,
        impactType,
        changeRequired: impactType === 'direct'
      };
    });
  }
}