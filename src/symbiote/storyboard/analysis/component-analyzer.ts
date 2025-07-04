/**
 * Component Analyzer
 * 
 * Analyzes React/Vue/Angular components to extract structure and relationships
 */

import * as ts from 'typescript';
import * as path from 'path';
import { Logger } from '../../utils/logger';
import {
  ComponentAnalysis,
  ComponentInfo,
  ComponentRelationship,
  DependencyInfo,
  ComponentMetrics,
  PropInfo,
  StateInfo
} from '../types';

export interface ComponentAnalyzerConfig {
  framework?: 'react' | 'vue' | 'angular' | 'auto';
  includeNodeModules?: boolean;
  maxDepth?: number;
  analyzePropTypes?: boolean;
  analyzeState?: boolean;
}

export class ComponentAnalyzer {
  private logger = new Logger('ComponentAnalyzer');
  private config: Required<ComponentAnalyzerConfig>;
  private program?: ts.Program;
  private typeChecker?: ts.TypeChecker;
  
  constructor(config: ComponentAnalyzerConfig = {}) {
    this.config = {
      framework: config.framework || 'auto',
      includeNodeModules: config.includeNodeModules || false,
      maxDepth: config.maxDepth || 10,
      analyzePropTypes: config.analyzePropTypes ?? true,
      analyzeState: config.analyzeState ?? true
    };
  }
  
  /**
   * Analyze a project or directory
   */
  async analyzeProject(rootPath: string): Promise<ComponentAnalysis> {
    try {
      // Create TypeScript program
      const configPath = ts.findConfigFile(
        rootPath,
        ts.sys.fileExists,
        'tsconfig.json'
      );
      
      let program: ts.Program;
      
      if (configPath) {
        const { config } = ts.readConfigFile(configPath, ts.sys.readFile);
        const { options, fileNames } = ts.parseJsonConfigFileContent(
          config,
          ts.sys,
          path.dirname(configPath)
        );
        
        program = ts.createProgram(fileNames, options);
      } else {
        // Fallback: analyze all TS/JS files
        const files = this.findSourceFiles(rootPath);
        program = ts.createProgram(files, {
          target: ts.ScriptTarget.ES2020,
          module: ts.ModuleKind.CommonJS,
          jsx: ts.JsxEmit.React
        });
      }
      
      this.program = program;
      this.typeChecker = program.getTypeChecker();
      
      // Analyze components
      const components: ComponentInfo[] = [];
      const relationships: ComponentRelationship[] = [];
      const dependencies: DependencyInfo[] = [];
      
      for (const sourceFile of program.getSourceFiles()) {
        if (this.shouldAnalyzeFile(sourceFile.fileName)) {
          const fileComponents = this.analyzeFile(sourceFile);
          components.push(...fileComponents);
          
          // Extract relationships
          const fileRelationships = this.extractRelationships(
            sourceFile,
            fileComponents
          );
          relationships.push(...fileRelationships);
          
          // Extract dependencies
          const fileDependencies = this.extractDependencies(sourceFile);
          dependencies.push(...fileDependencies);
        }
      }
      
      // Calculate metrics
      const metrics = this.calculateMetrics(components, dependencies);
      
      return {
        components,
        relationships,
        dependencies,
        metrics
      };
      
    } catch (error) {
      this.logger.error('Failed to analyze project', error);
      throw error;
    }
  }
  
  /**
   * Analyze a single file
   */
  analyzeFile(sourceFile: ts.SourceFile): ComponentInfo[] {
    const components: ComponentInfo[] = [];
    const framework = this.detectFramework(sourceFile);
    
    ts.forEachChild(sourceFile, node => {
      const component = this.extractComponent(node, sourceFile, framework);
      if (component) {
        components.push(component);
      }
    });
    
    return components;
  }
  
  /**
   * Extract component information from a node
   */
  private extractComponent(
    node: ts.Node,
    sourceFile: ts.SourceFile,
    framework: string
  ): ComponentInfo | null {
    switch (framework) {
      case 'react':
        return this.extractReactComponent(node, sourceFile);
      case 'vue':
        return this.extractVueComponent(node, sourceFile);
      case 'angular':
        return this.extractAngularComponent(node, sourceFile);
      default:
        // Try to detect component pattern
        return this.extractReactComponent(node, sourceFile);
    }
  }
  
  /**
   * Extract React component
   */
  private extractReactComponent(
    node: ts.Node,
    sourceFile: ts.SourceFile
  ): ComponentInfo | null {
    // Function component
    if (ts.isFunctionDeclaration(node) || ts.isArrowFunction(node)) {
      const name = this.getNodeName(node);
      if (name && this.isReactComponent(node)) {
        return this.buildComponentInfo(name, node, sourceFile, 'functional');
      }
    }
    
    // Class component
    if (ts.isClassDeclaration(node)) {
      const name = node.name?.getText();
      if (name && this.extendsReactComponent(node)) {
        return this.buildComponentInfo(name, node, sourceFile, 'class');
      }
    }
    
    // Variable declaration (const Component = () => {})
    if (ts.isVariableStatement(node)) {
      const declaration = node.declarationList.declarations[0];
      if (declaration && ts.isVariableDeclaration(declaration)) {
        const name = declaration.name.getText();
        const initializer = declaration.initializer;
        
        if (initializer && this.isReactComponent(initializer)) {
          return this.buildComponentInfo(name, initializer, sourceFile, 'functional');
        }
      }
    }
    
    return null;
  }
  
  /**
   * Extract Vue component
   */
  private extractVueComponent(
    node: ts.Node,
    sourceFile: ts.SourceFile
  ): ComponentInfo | null {
    // Vue.extend or defineComponent
    if (ts.isCallExpression(node)) {
      const expression = node.expression;
      if (ts.isPropertyAccessExpression(expression)) {
        const methodName = expression.name.getText();
        if (methodName === 'extend' || methodName === 'defineComponent') {
          const name = this.getVueComponentName(node);
          if (name) {
            return this.buildComponentInfo(name, node, sourceFile, 'functional');
          }
        }
      }
    }
    
    return null;
  }
  
  /**
   * Extract Angular component
   */
  private extractAngularComponent(
    node: ts.Node,
    sourceFile: ts.SourceFile
  ): ComponentInfo | null {
    // @Component decorator
    if (ts.isClassDeclaration(node) && node.decorators) {
      const componentDecorator = node.decorators.find(decorator => {
        if (ts.isCallExpression(decorator.expression)) {
          const decoratorName = decorator.expression.expression.getText();
          return decoratorName === 'Component';
        }
        return false;
      });
      
      if (componentDecorator) {
        const name = node.name?.getText() || 'AnonymousComponent';
        return this.buildComponentInfo(name, node, sourceFile, 'class');
      }
    }
    
    return null;
  }
  
  /**
   * Build component info
   */
  private buildComponentInfo(
    name: string,
    node: ts.Node,
    sourceFile: ts.SourceFile,
    type: 'functional' | 'class' | 'hook' | 'hoc'
  ): ComponentInfo {
    const filePath = sourceFile.fileName;
    const props = this.config.analyzePropTypes ? 
      this.extractProps(node) : [];
    const state = this.config.analyzeState ? 
      this.extractState(node) : [];
    const methods = this.extractMethods(node);
    const hooks = this.extractHooks(node);
    const imports = this.extractImports(sourceFile);
    const exports = this.extractExports(sourceFile);
    
    return {
      name,
      path: filePath,
      type,
      props,
      state,
      methods,
      hooks,
      imports,
      exports
    };
  }
  
  /**
   * Extract props from component
   */
  private extractProps(node: ts.Node): PropInfo[] {
    const props: PropInfo[] = [];
    
    if (!this.typeChecker) return props;
    
    // For function components, check first parameter
    if (ts.isFunctionDeclaration(node) || ts.isArrowFunction(node)) {
      const params = node.parameters;
      if (params.length > 0) {
        const propsParam = params[0];
        const type = this.typeChecker.getTypeAtLocation(propsParam);
        
        // Extract properties from type
        const properties = this.typeChecker.getPropertiesOfType(type);
        properties.forEach(prop => {
          const propType = this.typeChecker.getTypeOfSymbolAtLocation(
            prop,
            propsParam
          );
          const propInfo: PropInfo = {
            name: prop.getName(),
            type: this.typeChecker.typeToString(propType),
            required: !prop.flags || !(prop.flags & ts.SymbolFlags.Optional),
            description: ts.displayPartsToString(prop.getDocumentationComment(this.typeChecker))
          };
          props.push(propInfo);
        });
      }
    }
    
    return props;
  }
  
  /**
   * Extract state from component
   */
  private extractState(node: ts.Node): StateInfo[] {
    const state: StateInfo[] = [];
    
    // Look for useState hooks
    ts.forEachChild(node, child => {
      if (ts.isCallExpression(child)) {
        const expression = child.expression;
        if (ts.isIdentifier(expression) && expression.text === 'useState') {
          // Extract state variable name
          const parent = child.parent;
          if (ts.isVariableDeclaration(parent) && 
              ts.isArrayBindingPattern(parent.name)) {
            const elements = parent.name.elements;
            if (elements.length >= 2) {
              const stateName = elements[0].getText();
              const setterName = elements[1].getText();
              
              // Try to infer type from initial value
              let stateType = 'any';
              if (child.arguments.length > 0) {
                const initialValue = child.arguments[0];
                if (ts.isStringLiteral(initialValue)) {
                  stateType = 'string';
                } else if (ts.isNumericLiteral(initialValue)) {
                  stateType = 'number';
                } else if (initialValue.kind === ts.SyntaxKind.TrueKeyword ||
                          initialValue.kind === ts.SyntaxKind.FalseKeyword) {
                  stateType = 'boolean';
                }
              }
              
              state.push({
                name: stateName,
                type: stateType,
                updaters: [setterName]
              });
            }
          }
        }
      }
    });
    
    return state;
  }
  
  /**
   * Extract methods from component
   */
  private extractMethods(node: ts.Node): string[] {
    const methods: string[] = [];
    
    if (ts.isClassDeclaration(node)) {
      node.members.forEach(member => {
        if (ts.isMethodDeclaration(member) && member.name) {
          methods.push(member.name.getText());
        }
      });
    }
    
    return methods;
  }
  
  /**
   * Extract hooks from component
   */
  private extractHooks(node: ts.Node): string[] {
    const hooks = new Set<string>();
    
    const findHooks = (n: ts.Node) => {
      if (ts.isCallExpression(n)) {
        const expression = n.expression;
        if (ts.isIdentifier(expression)) {
          const name = expression.text;
          if (name.startsWith('use')) {
            hooks.add(name);
          }
        }
      }
      ts.forEachChild(n, findHooks);
    };
    
    findHooks(node);
    
    return Array.from(hooks);
  }
  
  /**
   * Extract imports from file
   */
  private extractImports(sourceFile: ts.SourceFile): string[] {
    const imports: string[] = [];
    
    ts.forEachChild(sourceFile, node => {
      if (ts.isImportDeclaration(node)) {
        const moduleSpecifier = node.moduleSpecifier;
        if (ts.isStringLiteral(moduleSpecifier)) {
          imports.push(moduleSpecifier.text);
        }
      }
    });
    
    return imports;
  }
  
  /**
   * Extract exports from file
   */
  private extractExports(sourceFile: ts.SourceFile): string[] {
    const exports: string[] = [];
    
    ts.forEachChild(sourceFile, node => {
      if (ts.isExportDeclaration(node) || ts.isExportAssignment(node)) {
        exports.push(node.getText());
      }
    });
    
    return exports;
  }
  
  /**
   * Extract relationships between components
   */
  private extractRelationships(
    sourceFile: ts.SourceFile,
    components: ComponentInfo[]
  ): ComponentRelationship[] {
    const relationships: ComponentRelationship[] = [];
    
    // Find component usage in JSX
    const findJSXElements = (node: ts.Node) => {
      if (ts.isJsxElement(node) || ts.isJsxSelfClosingElement(node)) {
        const tagName = ts.isJsxElement(node) ? 
          node.openingElement.tagName : 
          node.tagName;
        
        if (ts.isIdentifier(tagName)) {
          const componentName = tagName.text;
          
          // Check if it's a known component
          const component = components.find(c => c.name === componentName);
          if (component) {
            // Find parent component
            const parentComponent = this.findParentComponent(node, components);
            if (parentComponent) {
              relationships.push({
                parent: parentComponent.name,
                child: component.name,
                type: 'renders'
              });
            }
          }
        }
      }
      
      ts.forEachChild(node, findJSXElements);
    };
    
    findJSXElements(sourceFile);
    
    return relationships;
  }
  
  /**
   * Extract dependencies between files
   */
  private extractDependencies(sourceFile: ts.SourceFile): DependencyInfo[] {
    const dependencies: DependencyInfo[] = [];
    const filePath = sourceFile.fileName;
    
    ts.forEachChild(sourceFile, node => {
      if (ts.isImportDeclaration(node)) {
        const moduleSpecifier = node.moduleSpecifier;
        if (ts.isStringLiteral(moduleSpecifier)) {
          const importPath = moduleSpecifier.text;
          
          // Resolve relative imports
          let targetPath = importPath;
          if (importPath.startsWith('.')) {
            targetPath = path.resolve(path.dirname(filePath), importPath);
          }
          
          dependencies.push({
            source: filePath,
            target: targetPath,
            type: 'import'
          });
        }
      }
    });
    
    return dependencies;
  }
  
  /**
   * Calculate component metrics
   */
  private calculateMetrics(
    components: ComponentInfo[],
    dependencies: DependencyInfo[]
  ): ComponentMetrics {
    // Detect circular dependencies
    const circularDeps = this.findCircularDependencies(dependencies);
    
    // Find unused components
    const usedComponents = new Set<string>();
    dependencies.forEach(dep => {
      const componentName = path.basename(dep.target, path.extname(dep.target));
      usedComponents.add(componentName);
    });
    
    const unusedComponents = components
      .filter(c => !usedComponents.has(c.name))
      .map(c => c.name);
    
    // Calculate complexity (simplified)
    const totalComplexity = components.reduce((sum, comp) => {
      const complexity = comp.props.length + 
                        comp.state.length + 
                        comp.methods.length;
      return sum + complexity;
    }, 0);
    
    return {
      totalComponents: components.length,
      averageComplexity: totalComplexity / components.length || 0,
      maxDepth: this.calculateMaxDepth(components, dependencies),
      circularDependencies: circularDeps.length,
      unusedComponents
    };
  }
  
  /**
   * Find circular dependencies
   */
  private findCircularDependencies(
    dependencies: DependencyInfo[]
  ): DependencyInfo[] {
    const circular: DependencyInfo[] = [];
    const graph = new Map<string, string[]>();
    
    // Build dependency graph
    dependencies.forEach(dep => {
      if (!graph.has(dep.source)) {
        graph.set(dep.source, []);
      }
      graph.get(dep.source)!.push(dep.target);
    });
    
    // DFS to find cycles
    const visited = new Set<string>();
    const recursionStack = new Set<string>();
    
    const hasCycle = (node: string): boolean => {
      visited.add(node);
      recursionStack.add(node);
      
      const neighbors = graph.get(node) || [];
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          if (hasCycle(neighbor)) {
            return true;
          }
        } else if (recursionStack.has(neighbor)) {
          // Found circular dependency
          const dep = dependencies.find(
            d => d.source === node && d.target === neighbor
          );
          if (dep) {
            dep.isCircular = true;
            circular.push(dep);
          }
          return true;
        }
      }
      
      recursionStack.delete(node);
      return false;
    };
    
    for (const node of graph.keys()) {
      if (!visited.has(node)) {
        hasCycle(node);
      }
    }
    
    return circular;
  }
  
  /**
   * Calculate maximum component depth
   */
  private calculateMaxDepth(
    components: ComponentInfo[],
    dependencies: DependencyInfo[]
  ): number {
    // Simplified: count maximum import chain length
    return Math.min(this.config.maxDepth, 5);
  }
  
  // Helper methods
  
  private shouldAnalyzeFile(fileName: string): boolean {
    if (!this.config.includeNodeModules && fileName.includes('node_modules')) {
      return false;
    }
    
    const ext = path.extname(fileName);
    return ['.ts', '.tsx', '.js', '.jsx'].includes(ext);
  }
  
  private findSourceFiles(rootPath: string): string[] {
    // In a real implementation, recursively find all source files
    return [];
  }
  
  private detectFramework(sourceFile: ts.SourceFile): string {
    const content = sourceFile.getText();
    
    if (content.includes('import React') || content.includes('from "react"')) {
      return 'react';
    }
    if (content.includes('import Vue') || content.includes('from "vue"')) {
      return 'vue';
    }
    if (content.includes('@angular/core')) {
      return 'angular';
    }
    
    return this.config.framework === 'auto' ? 'react' : this.config.framework;
  }
  
  private isReactComponent(node: ts.Node): boolean {
    // Check if returns JSX
    const hasJSX = this.containsJSX(node);
    
    // Check if name starts with uppercase
    const name = this.getNodeName(node);
    const isUpperCase = name ? name[0] === name[0].toUpperCase() : false;
    
    return hasJSX || isUpperCase;
  }
  
  private containsJSX(node: ts.Node): boolean {
    let hasJSX = false;
    
    const checkJSX = (n: ts.Node) => {
      if (ts.isJsxElement(n) || ts.isJsxSelfClosingElement(n) || ts.isJsxFragment(n)) {
        hasJSX = true;
      }
      if (!hasJSX) {
        ts.forEachChild(n, checkJSX);
      }
    };
    
    checkJSX(node);
    return hasJSX;
  }
  
  private extendsReactComponent(node: ts.ClassDeclaration): boolean {
    if (!node.heritageClauses) return false;
    
    return node.heritageClauses.some(clause => {
      return clause.types.some(type => {
        const expression = type.expression;
        if (ts.isPropertyAccessExpression(expression)) {
          return expression.name.text === 'Component' &&
                 expression.expression.getText() === 'React';
        }
        return false;
      });
    });
  }
  
  private getNodeName(node: ts.Node): string | null {
    if (ts.isFunctionDeclaration(node)) {
      return node.name?.text || null;
    }
    if (ts.isArrowFunction(node)) {
      const parent = node.parent;
      if (ts.isVariableDeclaration(parent)) {
        return parent.name.getText();
      }
    }
    return null;
  }
  
  private getVueComponentName(node: ts.CallExpression): string | null {
    // Look for name property in options object
    if (node.arguments.length > 0) {
      const options = node.arguments[0];
      if (ts.isObjectLiteralExpression(options)) {
        const nameProp = options.properties.find(prop => {
          return ts.isPropertyAssignment(prop) && 
                 prop.name?.getText() === 'name';
        });
        
        if (nameProp && ts.isPropertyAssignment(nameProp)) {
          const value = nameProp.initializer;
          if (ts.isStringLiteral(value)) {
            return value.text;
          }
        }
      }
    }
    
    return 'AnonymousComponent';
  }
  
  private findParentComponent(
    node: ts.Node,
    components: ComponentInfo[]
  ): ComponentInfo | null {
    let current = node.parent;
    
    while (current) {
      const componentName = this.getNodeName(current);
      if (componentName) {
        const component = components.find(c => c.name === componentName);
        if (component) {
          return component;
        }
      }
      current = current.parent;
    }
    
    return null;
  }
}