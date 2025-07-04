/**
 * Graph Ingestion Pipeline
 * 
 * Manages the process of parsing code and ingesting it into the Neo4j graph
 */

import { EventEmitter } from 'events';
import * as path from 'path';
import * as fs from 'fs/promises';
import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import { TypeScriptParser } from '../parsers/typescript-parser';
import {
  GraphNode,
  GraphRelationship,
  GraphUpdate,
  BatchGraphUpdate,
  IndexingStatus,
  GraphError,
  GraphErrorCode
} from '../types/graph-types';

export interface IngestionConfig {
  batchSize: number;
  parallelWorkers: number;
  fileExtensions: string[];
  excludePatterns: string[];
  maxFileSize: number;
  enableProgress: boolean;
}

export interface IngestionProgress {
  totalFiles: number;
  processedFiles: number;
  failedFiles: number;
  totalNodes: number;
  totalRelationships: number;
  currentFile?: string;
  estimatedTimeRemaining?: number;
}

export interface IngestionResult {
  success: boolean;
  filesProcessed: number;
  nodesCreated: number;
  relationshipsCreated: number;
  errors: IngestionError[];
  duration: number;
}

export interface IngestionError {
  file: string;
  error: string;
  timestamp: Date;
}

export class GraphIngestionPipeline extends EventEmitter {
  private connectionManager: Neo4jConnectionManager;
  private parser: TypeScriptParser;
  private config: IngestionConfig;
  private isRunning: boolean = false;
  private abortController: AbortController | null = null;
  
  constructor(
    connectionManager: Neo4jConnectionManager,
    config: Partial<IngestionConfig> = {}
  ) {
    super();
    
    this.connectionManager = connectionManager;
    this.parser = new TypeScriptParser();
    
    this.config = {
      batchSize: config.batchSize || 100,
      parallelWorkers: config.parallelWorkers || 4,
      fileExtensions: config.fileExtensions || ['.ts', '.tsx', '.js', '.jsx'],
      excludePatterns: config.excludePatterns || ['node_modules', '.git', 'dist', 'build'],
      maxFileSize: config.maxFileSize || 10 * 1024 * 1024, // 10MB
      enableProgress: config.enableProgress ?? true
    };
  }
  
  /**
   * Ingest a single project
   */
  async ingestProject(
    projectPath: string,
    databaseId: string,
    options?: {
      fullReindex?: boolean;
      watch?: boolean;
    }
  ): Promise<IngestionResult> {
    if (this.isRunning) {
      throw new GraphError(
        'Ingestion already in progress',
        GraphErrorCode.INVALID_OPERATION
      );
    }
    
    const startTime = Date.now();
    this.isRunning = true;
    this.abortController = new AbortController();
    
    const result: IngestionResult = {
      success: false,
      filesProcessed: 0,
      nodesCreated: 0,
      relationshipsCreated: 0,
      errors: [],
      duration: 0
    };
    
    try {
      // Clear existing data if full reindex
      if (options?.fullReindex) {
        await this.clearProjectData(databaseId);
      }
      
      // Find all relevant files
      const files = await this.findProjectFiles(projectPath);
      
      if (files.length === 0) {
        throw new GraphError(
          'No files found to process',
          GraphErrorCode.INVALID_OPERATION
        );
      }
      
      // Initialize progress tracking
      const progress: IngestionProgress = {
        totalFiles: files.length,
        processedFiles: 0,
        failedFiles: 0,
        totalNodes: 0,
        totalRelationships: 0
      };
      
      if (this.config.enableProgress) {
        this.emit('progress', progress);
      }
      
      // Process files in batches
      const batches = this.createBatches(files, this.config.batchSize);
      
      for (const batch of batches) {
        if (this.abortController.signal.aborted) {
          break;
        }
        
        const batchResults = await this.processBatch(
          batch,
          databaseId,
          projectPath,
          progress
        );
        
        result.filesProcessed += batchResults.filesProcessed;
        result.nodesCreated += batchResults.nodesCreated;
        result.relationshipsCreated += batchResults.relationshipsCreated;
        result.errors.push(...batchResults.errors);
        
        if (this.config.enableProgress) {
          this.emit('progress', progress);
        }
      }
      
      // Create cross-file relationships
      await this.createCrossFileRelationships(databaseId);
      
      // Update statistics
      await this.connectionManager.updateStatistics(databaseId);
      
      result.success = true;
      result.duration = Date.now() - startTime;
      
      this.emit('complete', result);
      
      // Set up file watching if requested
      if (options?.watch) {
        this.setupFileWatching(projectPath, databaseId);
      }
      
    } catch (error) {
      result.errors.push({
        file: projectPath,
        error: error.toString(),
        timestamp: new Date()
      });
      
      this.emit('error', error);
      
    } finally {
      this.isRunning = false;
      this.abortController = null;
    }
    
    return result;
  }
  
  /**
   * Ingest a single file
   */
  async ingestFile(
    filePath: string,
    databaseId: string,
    projectPath: string
  ): Promise<{
    nodes: number;
    relationships: number;
    errors: IngestionError[];
  }> {
    try {
      // Parse the file
      const parseResult = await this.parser.parseFile(filePath);
      
      if (parseResult.errors.length > 0) {
        return {
          nodes: 0,
          relationships: 0,
          errors: parseResult.errors.map(e => ({
            file: e.file,
            error: e.message,
            timestamp: new Date()
          }))
        };
      }
      
      // Prepare batch update
      const updates: GraphUpdate[] = [];
      
      // Add nodes
      for (const node of parseResult.nodes) {
        updates.push({
          type: 'create',
          entityType: 'node',
          data: node,
          metadata: {
            source: 'file-watcher',
            timestamp: new Date()
          }
        });
      }
      
      // Add relationships
      for (const relationship of parseResult.relationships) {
        updates.push({
          type: 'create',
          entityType: 'relationship',
          data: relationship,
          metadata: {
            source: 'file-watcher',
            timestamp: new Date()
          }
        });
      }
      
      // Apply updates to graph
      await this.applyBatchUpdate(databaseId, {
        updates,
        transactional: true,
        validateBeforeApply: true
      });
      
      return {
        nodes: parseResult.nodes.length,
        relationships: parseResult.relationships.length,
        errors: []
      };
      
    } catch (error) {
      return {
        nodes: 0,
        relationships: 0,
        errors: [{
          file: filePath,
          error: error.toString(),
          timestamp: new Date()
        }]
      };
    }
  }
  
  /**
   * Stop the ingestion process
   */
  stop(): void {
    if (this.abortController) {
      this.abortController.abort();
    }
  }
  
  /**
   * Get current indexing status
   */
  getIndexingStatus(databaseId: string): IndexingStatus {
    // This would be tracked in the database
    return {
      status: this.isRunning ? 'indexing' : 'idle',
      lastIndexed: new Date()
    };
  }
  
  private async findProjectFiles(projectPath: string): Promise<string[]> {
    const files: string[] = [];
    
    const walk = async (dir: string) => {
      const entries = await fs.readdir(dir, { withFileTypes: true });
      
      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);
        
        // Check exclusion patterns
        if (this.config.excludePatterns.some(pattern => fullPath.includes(pattern))) {
          continue;
        }
        
        if (entry.isDirectory()) {
          await walk(fullPath);
        } else if (entry.isFile()) {
          const ext = path.extname(entry.name);
          if (this.config.fileExtensions.includes(ext)) {
            files.push(fullPath);
          }
        }
      }
    };
    
    await walk(projectPath);
    return files;
  }
  
  private createBatches<T>(items: T[], batchSize: number): T[][] {
    const batches: T[][] = [];
    
    for (let i = 0; i < items.length; i += batchSize) {
      batches.push(items.slice(i, i + batchSize));
    }
    
    return batches;
  }
  
  private async processBatch(
    files: string[],
    databaseId: string,
    projectPath: string,
    progress: IngestionProgress
  ): Promise<{
    filesProcessed: number;
    nodesCreated: number;
    relationshipsCreated: number;
    errors: IngestionError[];
  }> {
    const results = {
      filesProcessed: 0,
      nodesCreated: 0,
      relationshipsCreated: 0,
      errors: [] as IngestionError[]
    };
    
    // Process files in parallel
    const promises = files.map(async (file) => {
      try {
        progress.currentFile = file;
        
        const result = await this.ingestFile(file, databaseId, projectPath);
        
        results.filesProcessed++;
        results.nodesCreated += result.nodes;
        results.relationshipsCreated += result.relationships;
        results.errors.push(...result.errors);
        
        progress.processedFiles++;
        progress.totalNodes += result.nodes;
        progress.totalRelationships += result.relationships;
        
        if (result.errors.length > 0) {
          progress.failedFiles++;
        }
        
      } catch (error) {
        results.errors.push({
          file,
          error: error.toString(),
          timestamp: new Date()
        });
        progress.failedFiles++;
      }
    });
    
    await Promise.all(promises);
    
    return results;
  }
  
  private async applyBatchUpdate(
    databaseId: string,
    batch: BatchGraphUpdate
  ): Promise<void> {
    await this.connectionManager.executeTransaction(databaseId, async (tx) => {
      for (const update of batch.updates) {
        if (update.type === 'create' && update.entityType === 'node') {
          const node = update.data as GraphNode;
          const labels = node.labels.join(':');
          
          await tx.run(`
            CREATE (n:${labels})
            SET n = $properties
          `, { properties: node.properties });
          
        } else if (update.type === 'create' && update.entityType === 'relationship') {
          const rel = update.data as GraphRelationship;
          
          await tx.run(`
            MATCH (a {id: $startId})
            MATCH (b {id: $endId})
            CREATE (a)-[r:${rel.type}]->(b)
            SET r = $properties
          `, {
            startId: rel.startNodeId,
            endId: rel.endNodeId,
            properties: rel.properties
          });
        }
      }
    });
  }
  
  private async clearProjectData(databaseId: string): Promise<void> {
    await this.connectionManager.executeQuery(
      databaseId,
      'MATCH (n) WHERE NOT n:SchemaVersion DETACH DELETE n'
    );
  }
  
  private async createCrossFileRelationships(databaseId: string): Promise<void> {
    // Create relationships based on imports/exports
    await this.connectionManager.executeQuery(databaseId, `
      // Match imports and exports
      MATCH (file:File)-[:CONTAINS]->(entity)
      WHERE entity:Class OR entity:Function OR entity:Interface
      WITH file, entity
      MATCH (otherFile:File)-[:IMPORTS]->(entity)
      WHERE file <> otherFile
      MERGE (otherFile)-[:DEPENDS_ON]->(file)
    `);
    
    // Create module relationships
    await this.connectionManager.executeQuery(databaseId, `
      // Create module nodes from files
      MATCH (f:File)
      WITH f, 
           split(f.path, '/') as parts,
           replace(f.path, f.name, '') as dirPath
      MERGE (m:Module {path: dirPath})
      MERGE (m)-[:CONTAINS]->(f)
    `);
  }
  
  private setupFileWatching(projectPath: string, databaseId: string): void {
    // This would integrate with VS Code's file watcher
    // For now, we'll emit an event
    this.emit('watch-enabled', { projectPath, databaseId });
  }
}