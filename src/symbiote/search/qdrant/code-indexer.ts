/**
 * Code Indexer
 * 
 * Indexes codebase into Qdrant for semantic search
 */

import * as fs from 'fs';
import * as path from 'path';
import { glob } from 'glob';
import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import { QdrantManager } from './qdrant-manager';
import { Neo4jService } from '../../neo4j/neo4j-service';
import {
  CodeEmbeddingMetadata,
  CodeEntityType,
  BatchEmbeddingJob,
  ChunkingStrategy
} from './types';

interface IndexingOptions {
  // Paths
  rootPath: string;
  includes?: string[];
  excludes?: string[];
  
  // Processing
  batchSize?: number;
  parallel?: number;
  chunkingStrategy?: ChunkingStrategy;
  
  // Features
  includeTests?: boolean;
  includeComments?: boolean;
  includeDocstrings?: boolean;
  
  // Integration
  syncWithNeo4j?: boolean;
  fullReindex?: boolean;
}

interface FileProcessor {
  extensions: string[];
  process: (filePath: string, content: string) => Promise<CodeEmbeddingMetadata[]>;
}

export class CodeIndexer extends EventEmitter {
  private qdrantManager: QdrantManager;
  private neo4jService?: Neo4jService;
  private logger = new Logger('CodeIndexer');
  private processors = new Map<string, FileProcessor>();
  private currentJob?: BatchEmbeddingJob;
  
  constructor(
    qdrantManager: QdrantManager,
    neo4jService?: Neo4jService
  ) {
    super();
    
    this.qdrantManager = qdrantManager;
    this.neo4jService = neo4jService;
    
    // Register default processors
    this.registerDefaultProcessors();
  }
  
  /**
   * Index a codebase
   */
  async indexCodebase(options: IndexingOptions): Promise<BatchEmbeddingJob> {
    this.logger.info('Starting codebase indexing', {
      rootPath: options.rootPath,
      fullReindex: options.fullReindex
    });
    
    // Create job
    const job: BatchEmbeddingJob = {
      id: `index_${Date.now()}`,
      status: 'processing',
      totalEntities: 0,
      processedEntities: 0,
      failedEntities: 0,
      startTime: new Date(),
      errors: []
    };
    
    this.currentJob = job;
    this.emit('job-started', job);
    
    try {
      // Clear existing if full reindex
      if (options.fullReindex) {
        await this.clearExistingIndex(options.rootPath);
      }
      
      // Find all files to process
      const files = await this.findFiles(options);
      job.totalEntities = files.length;
      
      this.logger.info(`Found ${files.length} files to index`);
      
      // Process files in batches
      const batchSize = options.batchSize || 10;
      const results: CodeEmbeddingMetadata[] = [];
      
      for (let i = 0; i < files.length; i += batchSize) {
        const batch = files.slice(i, i + batchSize);
        const batchResults = await this.processBatch(batch, options);
        results.push(...batchResults);
        
        job.processedEntities = i + batch.length;
        this.emit('progress', {
          processed: job.processedEntities,
          total: job.totalEntities,
          percentage: (job.processedEntities / job.totalEntities) * 100
        });
      }
      
      // Create embeddings and store
      await this.createAndStoreEmbeddings(results, options);
      
      // Sync with Neo4j if enabled
      if (options.syncWithNeo4j && this.neo4jService) {
        await this.syncWithNeo4j(results);
      }
      
      // Complete job
      job.status = 'completed';
      job.endTime = new Date();
      
      this.logger.info('Codebase indexing completed', {
        totalFiles: files.length,
        totalEntities: results.length,
        duration: job.endTime.getTime() - job.startTime.getTime()
      });
      
      this.emit('job-completed', job);
      return job;
      
    } catch (error) {
      job.status = 'failed';
      job.endTime = new Date();
      job.errors?.push({
        entityId: 'general',
        error: error.message
      });
      
      this.logger.error('Codebase indexing failed', error);
      this.emit('job-failed', job);
      throw error;
      
    } finally {
      this.currentJob = undefined;
    }
  }
  
  /**
   * Index a single file
   */
  async indexFile(filePath: string): Promise<CodeEmbeddingMetadata[]> {
    this.logger.debug(`Indexing file: ${filePath}`);
    
    try {
      // Read file content
      const content = await fs.promises.readFile(filePath, 'utf-8');
      const ext = path.extname(filePath).toLowerCase();
      
      // Find processor
      const processor = this.findProcessor(ext);
      if (!processor) {
        this.logger.debug(`No processor for extension: ${ext}`);
        return [];
      }
      
      // Process file
      const entities = await processor.process(filePath, content);
      
      // Create embeddings and store
      const embeddings = await this.createEmbeddings(entities);
      await this.qdrantManager.batchUpsert(embeddings);
      
      // Sync with Neo4j
      if (this.neo4jService) {
        await this.syncFileWithNeo4j(filePath, entities);
      }
      
      this.logger.debug(`Indexed ${entities.length} entities from ${filePath}`);
      return entities;
      
    } catch (error) {
      this.logger.error(`Failed to index file: ${filePath}`, error);
      throw error;
    }
  }
  
  /**
   * Update file in index
   */
  async updateFile(filePath: string): Promise<void> {
    // Remove old entries
    await this.removeFile(filePath);
    
    // Re-index
    await this.indexFile(filePath);
  }
  
  /**
   * Remove file from index
   */
  async removeFile(filePath: string): Promise<void> {
    await this.qdrantManager.deleteByFilter({
      key: 'filePath',
      match: { value: filePath }
    });
    
    this.logger.debug(`Removed file from index: ${filePath}`);
  }
  
  /**
   * Find files to index
   */
  private async findFiles(options: IndexingOptions): Promise<string[]> {
    const includes = options.includes || ['**/*.{js,jsx,ts,tsx,py,java,go,rs,cpp,c,h,hpp}'];
    const excludes = options.excludes || [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/.git/**',
      '**/coverage/**',
      '**/*.min.js'
    ];
    
    const files: string[] = [];
    
    for (const pattern of includes) {
      const matches = await glob(pattern, {
        cwd: options.rootPath,
        ignore: excludes,
        absolute: true
      });
      files.push(...matches);
    }
    
    return [...new Set(files)]; // Remove duplicates
  }
  
  /**
   * Process a batch of files
   */
  private async processBatch(
    files: string[],
    options: IndexingOptions
  ): Promise<CodeEmbeddingMetadata[]> {
    const results: CodeEmbeddingMetadata[] = [];
    
    await Promise.all(
      files.map(async (filePath) => {
        try {
          const entities = await this.processFile(filePath, options);
          results.push(...entities);
        } catch (error) {
          this.logger.error(`Failed to process file: ${filePath}`, error);
          
          if (this.currentJob) {
            this.currentJob.failedEntities++;
            this.currentJob.errors?.push({
              entityId: filePath,
              error: error.message
            });
          }
        }
      })
    );
    
    return results;
  }
  
  /**
   * Process a single file
   */
  private async processFile(
    filePath: string,
    options: IndexingOptions
  ): Promise<CodeEmbeddingMetadata[]> {
    const content = await fs.promises.readFile(filePath, 'utf-8');
    const ext = path.extname(filePath).toLowerCase();
    
    const processor = this.findProcessor(ext);
    if (!processor) {
      return [];
    }
    
    const entities = await processor.process(filePath, content);
    
    // Filter based on options
    return entities.filter(entity => {
      if (!options.includeTests && entity.type === CodeEntityType.Test) {
        return false;
      }
      if (!options.includeComments && entity.type === CodeEntityType.Comment) {
        return false;
      }
      return true;
    });
  }
  
  /**
   * Create embeddings for entities
   */
  private async createEmbeddings(
    entities: CodeEmbeddingMetadata[]
  ): Promise<Array<{ vector: number[]; metadata: CodeEmbeddingMetadata }>> {
    const results = [];
    
    for (const entity of entities) {
      const code = await this.extractCodeForEntity(entity);
      const vector = await this.qdrantManager.embedCode(code, entity);
      
      results.push({ vector, metadata: entity });
    }
    
    return results;
  }
  
  /**
   * Create and store embeddings
   */
  private async createAndStoreEmbeddings(
    entities: CodeEmbeddingMetadata[],
    options: IndexingOptions
  ): Promise<void> {
    const batchSize = options.batchSize || 100;
    
    for (let i = 0; i < entities.length; i += batchSize) {
      const batch = entities.slice(i, i + batchSize);
      const embeddings = await this.createEmbeddings(batch);
      
      await this.qdrantManager.batchUpsert(embeddings);
      
      this.logger.debug(`Stored batch ${i / batchSize + 1}/${Math.ceil(entities.length / batchSize)}`);
    }
  }
  
  /**
   * Extract code for entity
   */
  private async extractCodeForEntity(entity: CodeEmbeddingMetadata): Promise<string> {
    // Read the specific lines from the file
    const content = await fs.promises.readFile(entity.filePath, 'utf-8');
    const lines = content.split('\n');
    
    const startLine = Math.max(0, entity.startLine - 1);
    const endLine = Math.min(lines.length, entity.endLine);
    
    return lines.slice(startLine, endLine).join('\n');
  }
  
  /**
   * Find processor for file extension
   */
  private findProcessor(extension: string): FileProcessor | undefined {
    for (const processor of this.processors.values()) {
      if (processor.extensions.includes(extension)) {
        return processor;
      }
    }
    return undefined;
  }
  
  /**
   * Register default processors
   */
  private registerDefaultProcessors(): void {
    // JavaScript/TypeScript processor
    this.registerProcessor('javascript', {
      extensions: ['.js', '.jsx', '.ts', '.tsx', '.mjs'],
      process: async (filePath, content) => {
        return this.processJavaScriptFile(filePath, content);
      }
    });
    
    // Python processor
    this.registerProcessor('python', {
      extensions: ['.py', '.pyw'],
      process: async (filePath, content) => {
        return this.processPythonFile(filePath, content);
      }
    });
    
    // Java processor
    this.registerProcessor('java', {
      extensions: ['.java'],
      process: async (filePath, content) => {
        return this.processJavaFile(filePath, content);
      }
    });
    
    // Add more processors as needed
  }
  
  /**
   * Register a file processor
   */
  registerProcessor(name: string, processor: FileProcessor): void {
    this.processors.set(name, processor);
  }
  
  /**
   * Process JavaScript/TypeScript file
   */
  private async processJavaScriptFile(
    filePath: string,
    content: string
  ): Promise<CodeEmbeddingMetadata[]> {
    const entities: CodeEmbeddingMetadata[] = [];
    const lines = content.split('\n');
    
    // Simple regex-based extraction (should use proper AST)
    const patterns = {
      function: /(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\([^)]*\)/g,
      class: /(?:export\s+)?class\s+(\w+)(?:\s+extends\s+\w+)?/g,
      method: /(?:async\s+)?(\w+)\s*\([^)]*\)\s*{/g,
      arrow: /(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\([^)]*\)\s*=>/g
    };
    
    // Extract functions
    let match;
    while ((match = patterns.function.exec(content)) !== null) {
      const name = match[1];
      const line = this.getLineNumber(content, match.index);
      
      entities.push({
        id: `${filePath}:function:${name}`,
        type: CodeEntityType.Function,
        name,
        filePath,
        startLine: line,
        endLine: this.findEndLine(lines, line),
        language: 'javascript',
        signature: match[0]
      });
    }
    
    // Extract classes
    patterns.class.lastIndex = 0;
    while ((match = patterns.class.exec(content)) !== null) {
      const name = match[1];
      const line = this.getLineNumber(content, match.index);
      
      entities.push({
        id: `${filePath}:class:${name}`,
        type: CodeEntityType.Class,
        name,
        filePath,
        startLine: line,
        endLine: this.findClassEndLine(lines, line),
        language: 'javascript',
        signature: match[0]
      });
    }
    
    return entities;
  }
  
  /**
   * Process Python file
   */
  private async processPythonFile(
    filePath: string,
    content: string
  ): Promise<CodeEmbeddingMetadata[]> {
    const entities: CodeEmbeddingMetadata[] = [];
    const lines = content.split('\n');
    
    // Simple regex-based extraction
    const patterns = {
      function: /^def\s+(\w+)\s*\([^)]*\):/gm,
      class: /^class\s+(\w+)(?:\([^)]*\))?:/gm
    };
    
    // Extract functions
    let match;
    while ((match = patterns.function.exec(content)) !== null) {
      const name = match[1];
      const line = this.getLineNumber(content, match.index);
      
      entities.push({
        id: `${filePath}:function:${name}`,
        type: CodeEntityType.Function,
        name,
        filePath,
        startLine: line,
        endLine: this.findPythonBlockEnd(lines, line),
        language: 'python',
        signature: match[0]
      });
    }
    
    // Extract classes
    patterns.class.lastIndex = 0;
    while ((match = patterns.class.exec(content)) !== null) {
      const name = match[1];
      const line = this.getLineNumber(content, match.index);
      
      entities.push({
        id: `${filePath}:class:${name}`,
        type: CodeEntityType.Class,
        name,
        filePath,
        startLine: line,
        endLine: this.findPythonBlockEnd(lines, line),
        language: 'python',
        signature: match[0]
      });
    }
    
    return entities;
  }
  
  /**
   * Process Java file
   */
  private async processJavaFile(
    filePath: string,
    content: string
  ): Promise<CodeEmbeddingMetadata[]> {
    // Similar to JavaScript processor but with Java syntax
    return [];
  }
  
  /**
   * Get line number from character index
   */
  private getLineNumber(content: string, index: number): number {
    return content.substring(0, index).split('\n').length;
  }
  
  /**
   * Find end line for a block
   */
  private findEndLine(lines: string[], startLine: number): number {
    let braceCount = 0;
    let inBlock = false;
    
    for (let i = startLine - 1; i < lines.length; i++) {
      const line = lines[i];
      
      for (const char of line) {
        if (char === '{') {
          braceCount++;
          inBlock = true;
        } else if (char === '}') {
          braceCount--;
          if (braceCount === 0 && inBlock) {
            return i + 1;
          }
        }
      }
    }
    
    return startLine + 1;
  }
  
  /**
   * Find end line for a class
   */
  private findClassEndLine(lines: string[], startLine: number): number {
    // Similar to findEndLine but handles class-specific patterns
    return this.findEndLine(lines, startLine);
  }
  
  /**
   * Find Python block end based on indentation
   */
  private findPythonBlockEnd(lines: string[], startLine: number): number {
    const baseIndent = lines[startLine - 1].match(/^\s*/)?.[0].length || 0;
    
    for (let i = startLine; i < lines.length; i++) {
      const line = lines[i];
      if (line.trim() === '') continue;
      
      const indent = line.match(/^\s*/)?.[0].length || 0;
      if (indent <= baseIndent) {
        return i;
      }
    }
    
    return lines.length;
  }
  
  /**
   * Clear existing index for a path
   */
  private async clearExistingIndex(rootPath: string): Promise<void> {
    await this.qdrantManager.deleteByFilter({
      key: 'filePath',
      match: {
        text: rootPath
      }
    });
  }
  
  /**
   * Sync with Neo4j
   */
  private async syncWithNeo4j(entities: CodeEmbeddingMetadata[]): Promise<void> {
    if (!this.neo4jService) return;
    
    // Link embeddings to Neo4j nodes
    for (const entity of entities) {
      try {
        // Find corresponding Neo4j node
        const neo4jNode = await this.neo4jService.findNodeByPath(
          entity.filePath,
          entity.name
        );
        
        if (neo4jNode) {
          entity.neo4jNodeId = neo4jNode.id;
          
          // Update Qdrant metadata
          await this.qdrantManager.updateMetadata(entity.id, {
            neo4jNodeId: neo4jNode.id
          });
        }
      } catch (error) {
        this.logger.warn(`Failed to sync entity with Neo4j: ${entity.id}`, error);
      }
    }
  }
  
  /**
   * Sync file with Neo4j
   */
  private async syncFileWithNeo4j(
    filePath: string,
    entities: CodeEmbeddingMetadata[]
  ): Promise<void> {
    if (!this.neo4jService) return;
    
    // Update Neo4j with Qdrant entity IDs
    for (const entity of entities) {
      try {
        await this.neo4jService.updateNodeMetadata(entity.neo4jNodeId!, {
          qdrantId: entity.id
        });
      } catch (error) {
        this.logger.warn(`Failed to update Neo4j node: ${entity.neo4jNodeId}`, error);
      }
    }
  }
}