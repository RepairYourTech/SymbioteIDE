/**
 * TypeScript/JavaScript AST Parser
 * 
 * Parses TypeScript and JavaScript files to extract code entities and relationships
 */

import * as ts from 'typescript';
import * as path from 'path';
import * as crypto from 'crypto';
import {
  NodeType,
  RelationType,
  FileNodeProperties,
  ClassNodeProperties,
  FunctionNodeProperties,
  VariableNodeProperties,
  GraphNode,
  GraphRelationship,
  BaseNodeProperties,
  BaseRelationshipProperties,
  ParameterInfo
} from '../types/graph-types';

export interface ParseResult {
  nodes: GraphNode[];
  relationships: GraphRelationship[];
  errors: ParseError[];
}

export interface ParseError {
  file: string;
  line: number;
  column: number;
  message: string;
}

export interface ParserConfig {
  includePrivateMembers: boolean;
  includeComments: boolean;
  includeTypeInfo: boolean;
  maxFileSize: number;
}

export class TypeScriptParser {
  private config: ParserConfig;
  private program: ts.Program | null = null;
  private typeChecker: ts.TypeChecker | null = null;
  
  constructor(config: Partial<ParserConfig> = {}) {
    this.config = {
      includePrivateMembers: config.includePrivateMembers ?? true,
      includeComments: config.includeComments ?? true,
      includeTypeInfo: config.includeTypeInfo ?? true,
      maxFileSize: config.maxFileSize || 10 * 1024 * 1024 // 10MB
    };
  }
  
  /**
   * Parse a single file
   */
  async parseFile(filePath: string, content?: string): Promise<ParseResult> {
    const nodes: GraphNode[] = [];
    const relationships: GraphRelationship[] = [];
    const errors: ParseError[] = [];
    
    try {
      // Create source file
      const sourceFile = content
        ? ts.createSourceFile(filePath, content, ts.ScriptTarget.Latest, true)
        : ts.createSourceFile(filePath, await this.readFile(filePath), ts.ScriptTarget.Latest, true);
      
      // Create file node
      const fileNode = this.createFileNode(filePath, sourceFile);
      nodes.push(fileNode);
      
      // Parse the AST
      const context: ParseContext = {
        sourceFile,
        filePath,
        nodes,
        relationships,
        errors,
        currentScope: fileNode.id,
        importMap: new Map()
      };
      
      this.visitNode(sourceFile, context);
      
      // Process imports/exports
      this.processImportsExports(context);
      
    } catch (error) {
      errors.push({
        file: filePath,
        line: 0,
        column: 0,
        message: `Failed to parse file: ${error}`
      });
    }
    
    return { nodes, relationships, errors };
  }
  
  /**
   * Parse a TypeScript project
   */
  async parseProject(projectPath: string, tsConfigPath?: string): Promise<ParseResult> {
    const nodes: GraphNode[] = [];
    const relationships: GraphRelationship[] = [];
    const errors: ParseError[] = [];
    
    try {
      // Load TypeScript configuration
      const configPath = tsConfigPath || path.join(projectPath, 'tsconfig.json');
      const config = ts.readConfigFile(configPath, ts.sys.readFile);
      
      if (config.error) {
        throw new Error(`Failed to read tsconfig: ${config.error.messageText}`);
      }
      
      const parsedConfig = ts.parseJsonConfigFileContent(
        config.config,
        ts.sys,
        projectPath
      );
      
      // Create program
      this.program = ts.createProgram({
        rootNames: parsedConfig.fileNames,
        options: parsedConfig.options
      });
      
      this.typeChecker = this.program.getTypeChecker();
      
      // Parse each source file
      for (const sourceFile of this.program.getSourceFiles()) {
        // Skip declaration files and node_modules
        if (sourceFile.isDeclarationFile || sourceFile.fileName.includes('node_modules')) {
          continue;
        }
        
        const fileResult = await this.parseSourceFile(sourceFile);
        nodes.push(...fileResult.nodes);
        relationships.push(...fileResult.relationships);
        errors.push(...fileResult.errors);
      }
      
      // Create cross-file relationships
      this.createCrossFileRelationships(nodes, relationships);
      
    } catch (error) {
      errors.push({
        file: projectPath,
        line: 0,
        column: 0,
        message: `Failed to parse project: ${error}`
      });
    } finally {
      this.program = null;
      this.typeChecker = null;
    }
    
    return { nodes, relationships, errors };
  }
  
  private async parseSourceFile(sourceFile: ts.SourceFile): Promise<ParseResult> {
    const nodes: GraphNode[] = [];
    const relationships: GraphRelationship[] = [];
    const errors: ParseError[] = [];
    
    // Create file node
    const fileNode = this.createFileNode(sourceFile.fileName, sourceFile);
    nodes.push(fileNode);
    
    const context: ParseContext = {
      sourceFile,
      filePath: sourceFile.fileName,
      nodes,
      relationships,
      errors,
      currentScope: fileNode.id,
      importMap: new Map()
    };
    
    this.visitNode(sourceFile, context);
    this.processImportsExports(context);
    
    return { nodes, relationships, errors };
  }
  
  private visitNode(node: ts.Node, context: ParseContext): void {
    switch (node.kind) {
      case ts.SyntaxKind.ClassDeclaration:
        this.visitClassDeclaration(node as ts.ClassDeclaration, context);
        break;
        
      case ts.SyntaxKind.InterfaceDeclaration:
        this.visitInterfaceDeclaration(node as ts.InterfaceDeclaration, context);
        break;
        
      case ts.SyntaxKind.FunctionDeclaration:
        this.visitFunctionDeclaration(node as ts.FunctionDeclaration, context);
        break;
        
      case ts.SyntaxKind.MethodDeclaration:
      case ts.SyntaxKind.MethodSignature:
        this.visitMethodDeclaration(node as ts.MethodDeclaration, context);
        break;
        
      case ts.SyntaxKind.VariableStatement:
        this.visitVariableStatement(node as ts.VariableStatement, context);
        break;
        
      case ts.SyntaxKind.TypeAliasDeclaration:
        this.visitTypeAliasDeclaration(node as ts.TypeAliasDeclaration, context);
        break;
        
      case ts.SyntaxKind.EnumDeclaration:
        this.visitEnumDeclaration(node as ts.EnumDeclaration, context);
        break;
        
      case ts.SyntaxKind.ImportDeclaration:
        this.visitImportDeclaration(node as ts.ImportDeclaration, context);
        break;
        
      case ts.SyntaxKind.ExportDeclaration:
        this.visitExportDeclaration(node as ts.ExportDeclaration, context);
        break;
    }
    
    // Continue traversing
    ts.forEachChild(node, child => this.visitNode(child, context));
  }
  
  private visitClassDeclaration(node: ts.ClassDeclaration, context: ParseContext): void {
    if (!node.name) return;
    
    const className = node.name.text;
    const classId = this.generateNodeId(NodeType.CLASS, context.filePath, className);
    
    // Create class node
    const classNode: GraphNode<ClassNodeProperties> = {
      id: classId,
      labels: [NodeType.CLASS],
      properties: {
        id: classId,
        name: className,
        visibility: this.getVisibility(node),
        isAbstract: this.hasModifier(node, ts.SyntaxKind.AbstractKeyword),
        isInterface: false,
        superClass: this.getSuperClass(node),
        interfaces: this.getInterfaces(node),
        filePath: context.filePath,
        startLine: this.getLineNumber(node.getStart(), context.sourceFile),
        endLine: this.getLineNumber(node.getEnd(), context.sourceFile),
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    context.nodes.push(classNode);
    
    // Create contains relationship
    this.createRelationship(
      context.currentScope,
      classId,
      RelationType.CONTAINS,
      {},
      context
    );
    
    // Create inheritance relationships
    if (classNode.properties.superClass) {
      this.createRelationship(
        classId,
        classNode.properties.superClass,
        RelationType.EXTENDS,
        {},
        context
      );
    }
    
    // Create interface implementation relationships
    for (const interfaceName of classNode.properties.interfaces) {
      this.createRelationship(
        classId,
        interfaceName,
        RelationType.IMPLEMENTS,
        {},
        context
      );
    }
    
    // Visit class members
    const previousScope = context.currentScope;
    context.currentScope = classId;
    
    node.members.forEach(member => this.visitNode(member, context));
    
    context.currentScope = previousScope;
  }
  
  private visitInterfaceDeclaration(node: ts.InterfaceDeclaration, context: ParseContext): void {
    const interfaceName = node.name.text;
    const interfaceId = this.generateNodeId(NodeType.INTERFACE, context.filePath, interfaceName);
    
    // Create interface node
    const interfaceNode: GraphNode = {
      id: interfaceId,
      labels: [NodeType.INTERFACE],
      properties: {
        id: interfaceId,
        name: interfaceName,
        filePath: context.filePath,
        startLine: this.getLineNumber(node.getStart(), context.sourceFile),
        endLine: this.getLineNumber(node.getEnd(), context.sourceFile),
        extends: this.getExtendedInterfaces(node),
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    context.nodes.push(interfaceNode);
    
    // Create contains relationship
    this.createRelationship(
      context.currentScope,
      interfaceId,
      RelationType.CONTAINS,
      {},
      context
    );
    
    // Visit interface members
    const previousScope = context.currentScope;
    context.currentScope = interfaceId;
    
    node.members.forEach(member => this.visitNode(member, context));
    
    context.currentScope = previousScope;
  }
  
  private visitFunctionDeclaration(node: ts.FunctionDeclaration, context: ParseContext): void {
    if (!node.name) return;
    
    const functionName = node.name.text;
    const functionId = this.generateNodeId(NodeType.FUNCTION, context.filePath, functionName);
    
    // Create function node
    const functionNode: GraphNode<FunctionNodeProperties> = {
      id: functionId,
      labels: [NodeType.FUNCTION],
      properties: {
        id: functionId,
        name: functionName,
        signature: this.getFunctionSignature(node),
        parameters: this.getParameters(node),
        returnType: this.getReturnType(node),
        visibility: this.getVisibility(node),
        isAsync: this.hasModifier(node, ts.SyntaxKind.AsyncKeyword),
        isGenerator: node.asteriskToken !== undefined,
        filePath: context.filePath,
        startLine: this.getLineNumber(node.getStart(), context.sourceFile),
        endLine: this.getLineNumber(node.getEnd(), context.sourceFile),
        complexity: this.calculateComplexity(node),
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    context.nodes.push(functionNode);
    
    // Create contains relationship
    this.createRelationship(
      context.currentScope,
      functionId,
      RelationType.CONTAINS,
      {},
      context
    );
    
    // Analyze function body for calls and references
    if (node.body) {
      this.analyzeFunctionBody(node.body, functionId, context);
    }
  }
  
  private visitMethodDeclaration(node: ts.MethodDeclaration | ts.MethodSignature, context: ParseContext): void {
    if (!node.name) return;
    
    const methodName = (node.name as ts.Identifier).text;
    const methodId = this.generateNodeId(NodeType.METHOD, context.filePath, `${context.currentScope}.${methodName}`);
    
    // Create method node
    const methodNode: GraphNode<FunctionNodeProperties> = {
      id: methodId,
      labels: [NodeType.METHOD],
      properties: {
        id: methodId,
        name: methodName,
        signature: this.getMethodSignature(node),
        parameters: this.getParameters(node),
        returnType: this.getReturnType(node),
        visibility: this.getVisibility(node),
        isAsync: this.hasModifier(node, ts.SyntaxKind.AsyncKeyword),
        isGenerator: false,
        filePath: context.filePath,
        startLine: this.getLineNumber(node.getStart(), context.sourceFile),
        endLine: this.getLineNumber(node.getEnd(), context.sourceFile),
        complexity: node.kind === ts.SyntaxKind.MethodDeclaration ? this.calculateComplexity(node) : 0,
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    context.nodes.push(methodNode);
    
    // Create contains relationship
    this.createRelationship(
      context.currentScope,
      methodId,
      RelationType.CONTAINS,
      {},
      context
    );
  }
  
  private visitVariableStatement(node: ts.VariableStatement, context: ParseContext): void {
    node.declarationList.declarations.forEach(declaration => {
      if (!ts.isIdentifier(declaration.name)) return;
      
      const varName = declaration.name.text;
      const varId = this.generateNodeId(NodeType.VARIABLE, context.filePath, `${context.currentScope}.${varName}`);
      
      // Create variable node
      const varNode: GraphNode<VariableNodeProperties> = {
        id: varId,
        labels: [NodeType.VARIABLE],
        properties: {
          id: varId,
          name: varName,
          type: this.getVariableType(declaration),
          isConstant: node.declarationList.flags === ts.NodeFlags.Const,
          scope: this.getVariableScope(context.currentScope),
          filePath: context.filePath,
          line: this.getLineNumber(declaration.getStart(), context.sourceFile),
          createdAt: new Date(),
          updatedAt: new Date(),
          version: 1
        }
      };
      
      context.nodes.push(varNode);
      
      // Create contains relationship
      this.createRelationship(
        context.currentScope,
        varId,
        RelationType.CONTAINS,
        {},
        context
      );
    });
  }
  
  private visitTypeAliasDeclaration(node: ts.TypeAliasDeclaration, context: ParseContext): void {
    const typeName = node.name.text;
    const typeId = this.generateNodeId(NodeType.TYPE, context.filePath, typeName);
    
    // Create type node
    const typeNode: GraphNode = {
      id: typeId,
      labels: [NodeType.TYPE],
      properties: {
        id: typeId,
        name: typeName,
        definition: node.type.getText(),
        isAlias: true,
        filePath: context.filePath,
        line: this.getLineNumber(node.getStart(), context.sourceFile),
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    context.nodes.push(typeNode);
    
    // Create contains relationship
    this.createRelationship(
      context.currentScope,
      typeId,
      RelationType.CONTAINS,
      {},
      context
    );
  }
  
  private visitEnumDeclaration(node: ts.EnumDeclaration, context: ParseContext): void {
    const enumName = node.name.text;
    const enumId = this.generateNodeId(NodeType.ENUM, context.filePath, enumName);
    
    // Create enum node
    const enumNode: GraphNode = {
      id: enumId,
      labels: [NodeType.ENUM],
      properties: {
        id: enumId,
        name: enumName,
        members: node.members.map(m => (m.name as ts.Identifier).text),
        filePath: context.filePath,
        startLine: this.getLineNumber(node.getStart(), context.sourceFile),
        endLine: this.getLineNumber(node.getEnd(), context.sourceFile),
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
    
    context.nodes.push(enumNode);
    
    // Create contains relationship
    this.createRelationship(
      context.currentScope,
      enumId,
      RelationType.CONTAINS,
      {},
      context
    );
  }
  
  private visitImportDeclaration(node: ts.ImportDeclaration, context: ParseContext): void {
    const moduleSpecifier = (node.moduleSpecifier as ts.StringLiteral).text;
    
    if (node.importClause) {
      const imports: ImportInfo = {
        module: moduleSpecifier,
        line: this.getLineNumber(node.getStart(), context.sourceFile),
        imports: []
      };
      
      // Default import
      if (node.importClause.name) {
        imports.imports.push({
          name: node.importClause.name.text,
          alias: node.importClause.name.text,
          isDefault: true
        });
      }
      
      // Named imports
      if (node.importClause.namedBindings) {
        if (ts.isNamedImports(node.importClause.namedBindings)) {
          node.importClause.namedBindings.elements.forEach(element => {
            imports.imports.push({
              name: element.name.text,
              alias: element.propertyName ? element.propertyName.text : element.name.text,
              isDefault: false
            });
          });
        } else if (ts.isNamespaceImport(node.importClause.namedBindings)) {
          imports.imports.push({
            name: '*',
            alias: node.importClause.namedBindings.name.text,
            isDefault: false
          });
        }
      }
      
      context.importMap.set(moduleSpecifier, imports);
    }
  }
  
  private visitExportDeclaration(node: ts.ExportDeclaration, context: ParseContext): void {
    // Handle exports - to be implemented
  }
  
  // Helper methods
  
  private createFileNode(filePath: string, sourceFile: ts.SourceFile): GraphNode<FileNodeProperties> {
    const fileId = this.generateNodeId(NodeType.FILE, filePath);
    const fileContent = sourceFile.getText();
    
    return {
      id: fileId,
      labels: [NodeType.FILE],
      properties: {
        id: fileId,
        name: path.basename(filePath),
        path: filePath,
        extension: path.extname(filePath),
        language: this.getLanguageFromExtension(path.extname(filePath)),
        size: fileContent.length,
        hash: crypto.createHash('sha256').update(fileContent).digest('hex'),
        encoding: 'utf-8',
        createdAt: new Date(),
        updatedAt: new Date(),
        version: 1
      }
    };
  }
  
  private generateNodeId(type: NodeType, ...parts: string[]): string {
    const combined = [type, ...parts].join(':');
    return crypto.createHash('sha256').update(combined).digest('hex').substring(0, 16);
  }
  
  private createRelationship(
    fromId: string,
    toId: string,
    type: RelationType,
    properties: any,
    context: ParseContext
  ): void {
    const relationshipId = this.generateNodeId(
      type as any,
      fromId,
      toId,
      Date.now().toString()
    );
    
    context.relationships.push({
      id: relationshipId,
      type,
      startNodeId: fromId,
      endNodeId: toId,
      properties: {
        ...properties,
        createdAt: new Date(),
        updatedAt: new Date()
      }
    });
  }
  
  private getLineNumber(pos: number, sourceFile: ts.SourceFile): number {
    return sourceFile.getLineAndCharacterOfPosition(pos).line + 1;
  }
  
  private getVisibility(node: ts.Node): 'public' | 'private' | 'protected' {
    if (this.hasModifier(node, ts.SyntaxKind.PrivateKeyword)) return 'private';
    if (this.hasModifier(node, ts.SyntaxKind.ProtectedKeyword)) return 'protected';
    return 'public';
  }
  
  private hasModifier(node: ts.Node, kind: ts.SyntaxKind): boolean {
    return node.modifiers?.some(m => m.kind === kind) || false;
  }
  
  private getSuperClass(node: ts.ClassDeclaration): string | undefined {
    if (!node.heritageClauses) return undefined;
    
    for (const clause of node.heritageClauses) {
      if (clause.token === ts.SyntaxKind.ExtendsKeyword) {
        return clause.types[0]?.expression.getText();
      }
    }
    
    return undefined;
  }
  
  private getInterfaces(node: ts.ClassDeclaration): string[] {
    const interfaces: string[] = [];
    
    if (!node.heritageClauses) return interfaces;
    
    for (const clause of node.heritageClauses) {
      if (clause.token === ts.SyntaxKind.ImplementsKeyword) {
        clause.types.forEach(type => {
          interfaces.push(type.expression.getText());
        });
      }
    }
    
    return interfaces;
  }
  
  private getExtendedInterfaces(node: ts.InterfaceDeclaration): string[] {
    const extended: string[] = [];
    
    if (!node.heritageClauses) return extended;
    
    for (const clause of node.heritageClauses) {
      clause.types.forEach(type => {
        extended.push(type.expression.getText());
      });
    }
    
    return extended;
  }
  
  private getParameters(node: ts.SignatureDeclaration): ParameterInfo[] {
    return node.parameters.map(param => ({
      name: (param.name as ts.Identifier).text,
      type: param.type?.getText(),
      defaultValue: param.initializer?.getText(),
      isOptional: !!param.questionToken,
      isRest: !!param.dotDotDotToken
    }));
  }
  
  private getReturnType(node: ts.SignatureDeclaration): string | undefined {
    return node.type?.getText();
  }
  
  private getFunctionSignature(node: ts.FunctionDeclaration): string {
    const params = this.getParameters(node).map(p => {
      let sig = p.name;
      if (p.isOptional) sig += '?';
      if (p.type) sig += `: ${p.type}`;
      return sig;
    }).join(', ');
    
    const returnType = this.getReturnType(node);
    return `${node.name!.text}(${params})${returnType ? `: ${returnType}` : ''}`;
  }
  
  private getMethodSignature(node: ts.MethodDeclaration | ts.MethodSignature): string {
    const name = (node.name as ts.Identifier).text;
    const params = this.getParameters(node).map(p => {
      let sig = p.name;
      if (p.isOptional) sig += '?';
      if (p.type) sig += `: ${p.type}`;
      return sig;
    }).join(', ');
    
    const returnType = this.getReturnType(node);
    return `${name}(${params})${returnType ? `: ${returnType}` : ''}`;
  }
  
  private getVariableType(node: ts.VariableDeclaration): string | undefined {
    if (node.type) {
      return node.type.getText();
    }
    
    // Try to infer type from initializer if type checker is available
    if (this.typeChecker && node.initializer) {
      const type = this.typeChecker.getTypeAtLocation(node.initializer);
      return this.typeChecker.typeToString(type);
    }
    
    return undefined;
  }
  
  private getVariableScope(scopeId: string): 'global' | 'module' | 'class' | 'function' | 'block' {
    // Simple heuristic based on scope ID pattern
    if (scopeId.includes(NodeType.CLASS)) return 'class';
    if (scopeId.includes(NodeType.FUNCTION) || scopeId.includes(NodeType.METHOD)) return 'function';
    if (scopeId.includes(NodeType.FILE)) return 'module';
    return 'block';
  }
  
  private calculateComplexity(node: ts.Node): number {
    let complexity = 1;
    
    const visit = (n: ts.Node) => {
      switch (n.kind) {
        case ts.SyntaxKind.IfStatement:
        case ts.SyntaxKind.ConditionalExpression:
        case ts.SyntaxKind.ForStatement:
        case ts.SyntaxKind.ForInStatement:
        case ts.SyntaxKind.ForOfStatement:
        case ts.SyntaxKind.WhileStatement:
        case ts.SyntaxKind.DoStatement:
        case ts.SyntaxKind.CatchClause:
        case ts.SyntaxKind.CaseClause:
          complexity++;
          break;
      }
      
      ts.forEachChild(n, visit);
    };
    
    visit(node);
    return complexity;
  }
  
  private analyzeFunctionBody(body: ts.Block, functionId: string, context: ParseContext): void {
    // Analyze function calls, variable references, etc.
    // This is a simplified version - real implementation would be more comprehensive
    
    const visit = (node: ts.Node) => {
      if (ts.isCallExpression(node)) {
        // Track function calls
        const callText = node.expression.getText();
        // Create CALLS relationship if we can resolve the target
      }
      
      ts.forEachChild(node, visit);
    };
    
    visit(body);
  }
  
  private processImportsExports(context: ParseContext): void {
    // Create import relationships
    context.importMap.forEach((importInfo, modulePath) => {
      importInfo.imports.forEach(imp => {
        this.createRelationship(
          context.nodes[0].id, // File node
          imp.name, // This would need to be resolved to actual node ID
          RelationType.IMPORTS,
          {
            importType: imp.isDefault ? 'default' : 'named',
            alias: imp.alias,
            line: importInfo.line,
            filePath: context.filePath
          },
          context
        );
      });
    });
  }
  
  private createCrossFileRelationships(nodes: GraphNode[], relationships: GraphRelationship[]): void {
    // Resolve cross-file references and create relationships
    // This would involve matching import names to exported entities
  }
  
  private getLanguageFromExtension(ext: string): string {
    switch (ext) {
      case '.ts':
      case '.tsx':
        return 'typescript';
      case '.js':
      case '.jsx':
        return 'javascript';
      default:
        return 'unknown';
    }
  }
  
  private async readFile(filePath: string): Promise<string> {
    const fs = await import('fs/promises');
    return fs.readFile(filePath, 'utf-8');
  }
}

// Internal types
interface ParseContext {
  sourceFile: ts.SourceFile;
  filePath: string;
  nodes: GraphNode[];
  relationships: GraphRelationship[];
  errors: ParseError[];
  currentScope: string;
  importMap: Map<string, ImportInfo>;
}

interface ImportInfo {
  module: string;
  line: number;
  imports: Array<{
    name: string;
    alias: string;
    isDefault: boolean;
  }>;
}