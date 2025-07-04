/**
 * Graph Schema Manager
 * 
 * Manages the Neo4j graph schema for code intelligence
 */

import { Transaction } from 'neo4j-driver';
import { Neo4jConnectionManager } from './connection-manager';
import {
  NodeType,
  RelationType,
  GraphError,
  GraphErrorCode
} from '../types/graph-types';

export interface SchemaDefinition {
  nodeTypes: NodeTypeDefinition[];
  relationshipTypes: RelationshipTypeDefinition[];
  constraints: ConstraintDefinition[];
  indexes: IndexDefinition[];
}

export interface NodeTypeDefinition {
  type: NodeType;
  requiredProperties: string[];
  optionalProperties: string[];
  uniqueProperties?: string[];
}

export interface RelationshipTypeDefinition {
  type: RelationType;
  fromNodeTypes: NodeType[];
  toNodeTypes: NodeType[];
  requiredProperties?: string[];
  optionalProperties?: string[];
}

export interface ConstraintDefinition {
  name: string;
  type: 'unique' | 'exists' | 'node_key';
  nodeType: NodeType;
  properties: string[];
}

export interface IndexDefinition {
  name: string;
  type: 'btree' | 'fulltext' | 'vector';
  nodeTypes?: NodeType[];
  relationshipTypes?: RelationType[];
  properties: string[];
}

export class GraphSchemaManager {
  private connectionManager: Neo4jConnectionManager;
  private schemaVersion: string = '1.0.0';
  
  constructor(connectionManager: Neo4jConnectionManager) {
    this.connectionManager = connectionManager;
  }
  
  /**
   * Get the complete schema definition
   */
  getSchemaDefinition(): SchemaDefinition {
    return {
      nodeTypes: this.getNodeTypeDefinitions(),
      relationshipTypes: this.getRelationshipTypeDefinitions(),
      constraints: this.getConstraintDefinitions(),
      indexes: this.getIndexDefinitions()
    };
  }
  
  /**
   * Initialize schema for a database
   */
  async initializeSchema(databaseId: string): Promise<void> {
    try {
      await this.connectionManager.executeTransaction(databaseId, async (tx) => {
        // Create schema version node
        await tx.run(`
          MERGE (s:SchemaVersion {version: $version})
          SET s.updatedAt = datetime()
        `, { version: this.schemaVersion });
        
        // Create constraints
        await this.createConstraints(tx);
        
        // Create indexes
        await this.createIndexes(tx);
        
        // Create meta-schema for validation
        await this.createMetaSchema(tx);
      });
    } catch (error) {
      throw new GraphError(
        'Failed to initialize schema',
        GraphErrorCode.SCHEMA_VIOLATION,
        error
      );
    }
  }
  
  /**
   * Validate schema compliance
   */
  async validateSchema(databaseId: string): Promise<{ valid: boolean; issues: string[] }> {
    const issues: string[] = [];
    
    try {
      // Check schema version
      const versionResult = await this.connectionManager.executeQuery(
        databaseId,
        'MATCH (s:SchemaVersion) RETURN s.version as version'
      );
      
      if (versionResult.records.length === 0) {
        issues.push('Schema version not found');
      } else {
        const version = versionResult.records[0].get('version');
        if (version !== this.schemaVersion) {
          issues.push(`Schema version mismatch: expected ${this.schemaVersion}, found ${version}`);
        }
      }
      
      // Check constraints
      const constraintsResult = await this.connectionManager.executeQuery(
        databaseId,
        'SHOW CONSTRAINTS'
      );
      
      const expectedConstraints = this.getConstraintDefinitions();
      const existingConstraints = new Set(
        constraintsResult.records.map(r => r.get('name'))
      );
      
      for (const constraint of expectedConstraints) {
        if (!existingConstraints.has(constraint.name)) {
          issues.push(`Missing constraint: ${constraint.name}`);
        }
      }
      
      // Check indexes
      const indexesResult = await this.connectionManager.executeQuery(
        databaseId,
        'SHOW INDEXES'
      );
      
      const expectedIndexes = this.getIndexDefinitions();
      const existingIndexes = new Set(
        indexesResult.records.map(r => r.get('name'))
      );
      
      for (const index of expectedIndexes) {
        if (!existingIndexes.has(index.name)) {
          issues.push(`Missing index: ${index.name}`);
        }
      }
      
    } catch (error) {
      issues.push(`Schema validation error: ${error}`);
    }
    
    return {
      valid: issues.length === 0,
      issues
    };
  }
  
  /**
   * Migrate schema to new version
   */
  async migrateSchema(databaseId: string, targetVersion: string): Promise<void> {
    // TODO: Implement schema migration logic
    throw new Error('Schema migration not yet implemented');
  }
  
  private getNodeTypeDefinitions(): NodeTypeDefinition[] {
    return [
      {
        type: NodeType.PROJECT,
        requiredProperties: ['id', 'workspacePath', 'createdAt', 'updatedAt'],
        optionalProperties: ['name', 'description', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.FILE,
        requiredProperties: ['id', 'path', 'name', 'extension', 'createdAt', 'updatedAt'],
        optionalProperties: ['language', 'size', 'hash', 'encoding', 'metadata'],
        uniqueProperties: ['id', 'path']
      },
      {
        type: NodeType.CLASS,
        requiredProperties: ['id', 'name', 'filePath', 'startLine', 'endLine', 'createdAt', 'updatedAt'],
        optionalProperties: ['visibility', 'isAbstract', 'isInterface', 'superClass', 'interfaces', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.FUNCTION,
        requiredProperties: ['id', 'name', 'signature', 'filePath', 'startLine', 'endLine', 'createdAt', 'updatedAt'],
        optionalProperties: ['parameters', 'returnType', 'visibility', 'isAsync', 'isGenerator', 'complexity', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.METHOD,
        requiredProperties: ['id', 'name', 'signature', 'filePath', 'startLine', 'endLine', 'createdAt', 'updatedAt'],
        optionalProperties: ['parameters', 'returnType', 'visibility', 'isAsync', 'isGenerator', 'isStatic', 'complexity', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.VARIABLE,
        requiredProperties: ['id', 'name', 'filePath', 'line', 'createdAt', 'updatedAt'],
        optionalProperties: ['type', 'value', 'isConstant', 'scope', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.INTERFACE,
        requiredProperties: ['id', 'name', 'filePath', 'startLine', 'endLine', 'createdAt', 'updatedAt'],
        optionalProperties: ['extends', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.TYPE,
        requiredProperties: ['id', 'name', 'filePath', 'line', 'createdAt', 'updatedAt'],
        optionalProperties: ['definition', 'isAlias', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.MODULE,
        requiredProperties: ['id', 'name', 'path', 'createdAt', 'updatedAt'],
        optionalProperties: ['exports', 'imports', 'metadata'],
        uniqueProperties: ['id']
      },
      {
        type: NodeType.PACKAGE,
        requiredProperties: ['id', 'name', 'version', 'createdAt', 'updatedAt'],
        optionalProperties: ['registry', 'description', 'license', 'author', 'homepage', 'dependencies', 'metadata'],
        uniqueProperties: ['id']
      }
    ];
  }
  
  private getRelationshipTypeDefinitions(): RelationshipTypeDefinition[] {
    return [
      {
        type: RelationType.CONTAINS,
        fromNodeTypes: [NodeType.PROJECT, NodeType.DIRECTORY, NodeType.FILE, NodeType.CLASS, NodeType.MODULE],
        toNodeTypes: [NodeType.DIRECTORY, NodeType.FILE, NodeType.CLASS, NodeType.FUNCTION, NodeType.METHOD, NodeType.VARIABLE, NodeType.INTERFACE, NodeType.TYPE]
      },
      {
        type: RelationType.IMPORTS,
        fromNodeTypes: [NodeType.FILE, NodeType.MODULE, NodeType.CLASS],
        toNodeTypes: [NodeType.MODULE, NodeType.CLASS, NodeType.FUNCTION, NodeType.VARIABLE, NodeType.TYPE],
        optionalProperties: ['importType', 'importedNames', 'alias', 'line']
      },
      {
        type: RelationType.EXPORTS,
        fromNodeTypes: [NodeType.FILE, NodeType.MODULE],
        toNodeTypes: [NodeType.CLASS, NodeType.FUNCTION, NodeType.VARIABLE, NodeType.TYPE, NodeType.INTERFACE],
        optionalProperties: ['exportType', 'isDefault', 'alias']
      },
      {
        type: RelationType.CALLS,
        fromNodeTypes: [NodeType.FUNCTION, NodeType.METHOD],
        toNodeTypes: [NodeType.FUNCTION, NodeType.METHOD],
        optionalProperties: ['callCount', 'isAsync', 'parameters', 'line']
      },
      {
        type: RelationType.EXTENDS,
        fromNodeTypes: [NodeType.CLASS, NodeType.INTERFACE],
        toNodeTypes: [NodeType.CLASS, NodeType.INTERFACE]
      },
      {
        type: RelationType.IMPLEMENTS,
        fromNodeTypes: [NodeType.CLASS],
        toNodeTypes: [NodeType.INTERFACE]
      },
      {
        type: RelationType.REFERENCES,
        fromNodeTypes: [NodeType.FUNCTION, NodeType.METHOD, NodeType.CLASS],
        toNodeTypes: [NodeType.VARIABLE, NodeType.TYPE, NodeType.CLASS, NodeType.INTERFACE],
        optionalProperties: ['referenceType', 'line']
      },
      {
        type: RelationType.DEPENDS_ON,
        fromNodeTypes: [NodeType.PACKAGE, NodeType.MODULE, NodeType.FILE],
        toNodeTypes: [NodeType.PACKAGE, NodeType.MODULE],
        optionalProperties: ['versionConstraint', 'isDev', 'isOptional']
      },
      {
        type: RelationType.HAS_TYPE,
        fromNodeTypes: [NodeType.VARIABLE, NodeType.FUNCTION, NodeType.METHOD],
        toNodeTypes: [NodeType.TYPE, NodeType.CLASS, NodeType.INTERFACE]
      },
      {
        type: RelationType.RETURNS,
        fromNodeTypes: [NodeType.FUNCTION, NodeType.METHOD],
        toNodeTypes: [NodeType.TYPE, NodeType.CLASS, NodeType.INTERFACE]
      }
    ];
  }
  
  private getConstraintDefinitions(): ConstraintDefinition[] {
    const constraints: ConstraintDefinition[] = [];
    
    // Add unique constraints for all node types
    const nodeTypes = Object.values(NodeType);
    for (const nodeType of nodeTypes) {
      constraints.push({
        name: `${nodeType.toLowerCase()}_id_unique`,
        type: 'unique',
        nodeType,
        properties: ['id']
      });
    }
    
    // Add specific unique constraints
    constraints.push({
      name: 'file_path_unique',
      type: 'unique',
      nodeType: NodeType.FILE,
      properties: ['path']
    });
    
    return constraints;
  }
  
  private getIndexDefinitions(): IndexDefinition[] {
    return [
      // Basic property indexes
      {
        name: 'file_path_index',
        type: 'btree',
        nodeTypes: [NodeType.FILE],
        properties: ['path']
      },
      {
        name: 'file_extension_index',
        type: 'btree',
        nodeTypes: [NodeType.FILE],
        properties: ['extension']
      },
      {
        name: 'node_name_index',
        type: 'btree',
        nodeTypes: [NodeType.CLASS, NodeType.FUNCTION, NodeType.METHOD, NodeType.INTERFACE],
        properties: ['name']
      },
      {
        name: 'node_updated_index',
        type: 'btree',
        properties: ['updatedAt']
      },
      {
        name: 'package_name_version_index',
        type: 'btree',
        nodeTypes: [NodeType.PACKAGE],
        properties: ['name', 'version']
      },
      
      // Full-text search indexes
      {
        name: 'code_entity_search',
        type: 'fulltext',
        nodeTypes: [NodeType.CLASS, NodeType.FUNCTION, NodeType.METHOD, NodeType.INTERFACE, NodeType.TYPE],
        properties: ['name', 'description']
      },
      {
        name: 'file_content_search',
        type: 'fulltext',
        nodeTypes: [NodeType.FILE],
        properties: ['path', 'name']
      },
      {
        name: 'documentation_search',
        type: 'fulltext',
        nodeTypes: [NodeType.COMMENT, NodeType.DOCSTRING],
        properties: ['content']
      }
    ];
  }
  
  private async createConstraints(tx: Transaction): Promise<void> {
    const constraints = this.getConstraintDefinitions();
    
    for (const constraint of constraints) {
      try {
        const query = this.buildConstraintQuery(constraint);
        await tx.run(query);
      } catch (error) {
        // Constraint might already exist
        console.debug(`Constraint ${constraint.name} might already exist:`, error);
      }
    }
  }
  
  private async createIndexes(tx: Transaction): Promise<void> {
    const indexes = this.getIndexDefinitions();
    
    for (const index of indexes) {
      try {
        const query = this.buildIndexQuery(index);
        await tx.run(query);
      } catch (error) {
        // Index might already exist
        console.debug(`Index ${index.name} might already exist:`, error);
      }
    }
  }
  
  private async createMetaSchema(tx: Transaction): Promise<void> {
    // Store node type definitions
    const nodeTypes = this.getNodeTypeDefinitions();
    for (const nodeDef of nodeTypes) {
      await tx.run(`
        MERGE (nt:NodeTypeDefinition {type: $type})
        SET nt.requiredProperties = $requiredProperties,
            nt.optionalProperties = $optionalProperties,
            nt.uniqueProperties = $uniqueProperties,
            nt.updatedAt = datetime()
      `, {
        type: nodeDef.type,
        requiredProperties: nodeDef.requiredProperties,
        optionalProperties: nodeDef.optionalProperties,
        uniqueProperties: nodeDef.uniqueProperties || []
      });
    }
    
    // Store relationship type definitions
    const relTypes = this.getRelationshipTypeDefinitions();
    for (const relDef of relTypes) {
      await tx.run(`
        MERGE (rt:RelationshipTypeDefinition {type: $type})
        SET rt.fromNodeTypes = $fromNodeTypes,
            rt.toNodeTypes = $toNodeTypes,
            rt.requiredProperties = $requiredProperties,
            rt.optionalProperties = $optionalProperties,
            rt.updatedAt = datetime()
      `, {
        type: relDef.type,
        fromNodeTypes: relDef.fromNodeTypes,
        toNodeTypes: relDef.toNodeTypes,
        requiredProperties: relDef.requiredProperties || [],
        optionalProperties: relDef.optionalProperties || []
      });
    }
  }
  
  private buildConstraintQuery(constraint: ConstraintDefinition): string {
    const propList = constraint.properties.map(p => `n.${p}`).join(', ');
    
    switch (constraint.type) {
      case 'unique':
        return `CREATE CONSTRAINT ${constraint.name} IF NOT EXISTS
                FOR (n:${constraint.nodeType})
                REQUIRE ${propList} IS UNIQUE`;
      
      case 'exists':
        return `CREATE CONSTRAINT ${constraint.name} IF NOT EXISTS
                FOR (n:${constraint.nodeType})
                REQUIRE ${propList} IS NOT NULL`;
      
      case 'node_key':
        return `CREATE CONSTRAINT ${constraint.name} IF NOT EXISTS
                FOR (n:${constraint.nodeType})
                REQUIRE (${propList}) IS NODE KEY`;
      
      default:
        throw new Error(`Unknown constraint type: ${constraint.type}`);
    }
  }
  
  private buildIndexQuery(index: IndexDefinition): string {
    switch (index.type) {
      case 'btree':
        if (index.nodeTypes && index.nodeTypes.length > 0) {
          // Index on specific node types
          const labelList = index.nodeTypes.join('|');
          const propList = index.properties.map(p => `n.${p}`).join(', ');
          return `CREATE INDEX ${index.name} IF NOT EXISTS
                  FOR (n:${labelList})
                  ON (${propList})`;
        } else {
          // Index on all nodes
          const propList = index.properties.map(p => `n.${p}`).join(', ');
          return `CREATE INDEX ${index.name} IF NOT EXISTS
                  FOR (n)
                  ON (${propList})`;
        }
      
      case 'fulltext':
        if (index.nodeTypes && index.nodeTypes.length > 0) {
          const labelList = index.nodeTypes.join('|');
          const propList = index.properties.map(p => `n.${p}`).join(', n.');
          return `CREATE FULLTEXT INDEX ${index.name} IF NOT EXISTS
                  FOR (n:${labelList})
                  ON EACH [n.${propList}]`;
        } else {
          throw new Error('Fulltext indexes require node types');
        }
      
      case 'vector':
        // Vector indexes for future semantic search
        throw new Error('Vector indexes not yet supported');
      
      default:
        throw new Error(`Unknown index type: ${index.type}`);
    }
  }
}