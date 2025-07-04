/**
 * Code Indexing Processor - Handles code indexing jobs
 */

import { Job } from 'bullmq';
import * as fs from 'fs/promises';
import * as path from 'path';
import { BaseProcessor } from './base-processor';
import { 
  CodeIndexingJobData, 
  JobResult, 
  JobType,
  QueueName 
} from '../types';
import { Neo4jConnectionManager } from '../../neo4j/connection-manager';
import { CodeIngestionService } from '../../neo4j/code-ingestion';
import { QdrantManager } from '../../search/qdrant/qdrant-manager';
import { EmbeddingService } from '../../search/qdrant/embedding-service';

export class CodeIndexingProcessor extends BaseProcessor<CodeIndexingJobData> {
  private neo4jManager?: Neo4jConnectionManager;
  private ingestionService?: CodeIngestionService;
  private qdrantManager?: QdrantManager;
  private embeddingService?: EmbeddingService;

  constructor(queueManager: any) {
    super(queueManager, 'CodeIndexingProcessor');
  }

  /**
   * Initialize services
   */
  private async initializeServices(): Promise<void> {
    if (!this.neo4jManager) {
      this.neo4jManager = Neo4jConnectionManager.getInstance();
      await this.neo4jManager.connect();
      
      this.ingestionService = new CodeIngestionService(this.neo4jManager);
      
      this.qdrantManager = new QdrantManager({
        url: process.env.QDRANT_URL || 'http://localhost:6333',
        apiKey: process.env.QDRANT_API_KEY
      });
      await this.qdrantManager.initialize();
      
      this.embeddingService = new EmbeddingService({
        provider: process.env.EMBEDDING_PROVIDER || 'openai',
        apiKey: process.env.OPENAI_API_KEY || '',
        model: process.env.EMBEDDING_MODEL || 'text-embedding-3-small',
        dimensions: parseInt(process.env.EMBEDDING_DIMENSIONS || '1536')
      });
    }
  }

  /**
   * Process indexing job
   */
  async process(job: Job<CodeIndexingJobData>): Promise<JobResult> {
    try {
      this.validateJobData(job.data);
      await this.initializeServices();

      switch (job.data.type) {
        case JobType.IndexFile:
          return await this.indexFile(job);
          
        case JobType.IndexDirectory:
          return await this.indexDirectory(job);
          
        case JobType.IndexProject:
          return await this.indexProject(job);
          
        case JobType.UpdateIndex:
          return await this.updateIndex(job);
          
        case JobType.RemoveFromIndex:
          return await this.removeFromIndex(job);
          
        default:
          throw new Error(`Unknown job type: ${job.data.type}`);
      }
    } catch (error: any) {
      return this.handleFailure(job, error);
    }
  }

  /**
   * Index a single file
   */
  private async indexFile(job: Job<CodeIndexingJobData>): Promise<JobResult> {
    const { path: filePath, language } = job.data;
    
    await this.updateProgress(job, 0, 'Reading file');
    
    // Check if file exists
    try {
      await fs.access(filePath);
    } catch {
      return this.createErrorResult(
        'FILE_NOT_FOUND',
        `File not found: ${filePath}`
      );
    }

    // Read file content
    const content = await fs.readFile(filePath, 'utf-8');
    const fileName = path.basename(filePath);
    const fileExt = path.extname(filePath);
    
    await this.updateProgress(job, 20, 'Parsing AST');
    
    // Index in Neo4j
    if (this.ingestionService) {
      await this.ingestionService.indexFile(
        filePath,
        content,
        language || this.detectLanguage(fileExt)
      );
    }
    
    await this.updateProgress(job, 50, 'Generating embeddings');
    
    // Generate and store embeddings
    if (this.embeddingService && this.qdrantManager) {
      const chunks = this.chunkCode(content, filePath);
      const embeddings = [];
      
      for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i];
        const embedding = await this.embeddingService.generateEmbedding(chunk.content);
        
        embeddings.push({
          id: `${filePath}_chunk_${i}`,
          vector: embedding,
          metadata: {
            filePath,
            fileName,
            language: language || this.detectLanguage(fileExt),
            chunkIndex: i,
            totalChunks: chunks.length,
            startLine: chunk.startLine,
            endLine: chunk.endLine,
            content: chunk.content,
            indexedAt: new Date().toISOString()
          }
        });
        
        await this.updateProgress(
          job, 
          50 + (40 * (i + 1) / chunks.length),
          `Processing chunk ${i + 1}/${chunks.length}`
        );
      }
      
      // Store embeddings
      await this.qdrantManager.upsertBatch(embeddings);
    }
    
    await this.updateProgress(job, 90, 'Finalizing');
    
    // Schedule graph sync
    await this.queueManager.addJob(
      QueueName.GraphSync,
      'sync-file-relationships',
      {
        type: JobType.UpdateRelationships,
        nodeId: filePath,
        nodeType: 'file'
      }
    );
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      filePath,
      language: language || this.detectLanguage(fileExt),
      linesIndexed: content.split('\n').length,
      chunksCreated: this.chunkCode(content, filePath).length
    });
  }

  /**
   * Index a directory
   */
  private async indexDirectory(job: Job<CodeIndexingJobData>): Promise<JobResult> {
    const { path: dirPath, options } = job.data;
    
    await this.updateProgress(job, 0, 'Scanning directory');
    
    // Get all files in directory
    const files = await this.scanDirectory(dirPath, options);
    const totalFiles = files.length;
    
    if (totalFiles === 0) {
      return this.createSuccessResult({
        directory: dirPath,
        filesIndexed: 0
      });
    }
    
    await this.updateProgress(job, 10, `Found ${totalFiles} files to index`);
    
    // Create sub-jobs for each file
    const subJobs = files.map(file => ({
      name: 'index-file',
      data: {
        type: JobType.IndexFile,
        path: file.path,
        language: file.language,
        projectId: job.data.projectId
      }
    }));
    
    // Add jobs to queue
    await this.queueManager.addBulkJobs(QueueName.CodeIndexing, subJobs);
    
    await this.updateProgress(job, 100, 'Directory scan complete');
    
    return this.createSuccessResult({
      directory: dirPath,
      filesQueued: totalFiles,
      options
    });
  }

  /**
   * Index entire project
   */
  private async indexProject(job: Job<CodeIndexingJobData>): Promise<JobResult> {
    const { path: projectPath, projectId } = job.data;
    
    await this.updateProgress(job, 0, 'Initializing project indexing');
    
    // Create project node in Neo4j
    if (this.neo4jManager) {
      const session = this.neo4jManager.getSession();
      try {
        await session.run(
          `
          MERGE (p:Project {id: $projectId})
          SET p.path = $path,
              p.name = $name,
              p.indexedAt = datetime(),
              p.status = 'indexing'
          `,
          {
            projectId: projectId || projectPath,
            path: projectPath,
            name: path.basename(projectPath)
          }
        );
      } finally {
        await session.close();
      }
    }
    
    await this.updateProgress(job, 10, 'Scanning project structure');
    
    // Get all directories
    const directories = await this.getProjectDirectories(projectPath);
    
    // Create directory indexing jobs
    const dirJobs = directories.map(dir => ({
      name: 'index-directory',
      data: {
        type: JobType.IndexDirectory,
        path: dir,
        projectId: projectId || projectPath,
        options: job.data.options
      }
    }));
    
    await this.queueManager.addBulkJobs(QueueName.CodeIndexing, dirJobs);
    
    await this.updateProgress(job, 90, 'Updating project status');
    
    // Update project status
    if (this.neo4jManager) {
      const session = this.neo4jManager.getSession();
      try {
        await session.run(
          `
          MATCH (p:Project {id: $projectId})
          SET p.status = 'indexed',
              p.lastIndexed = datetime()
          `,
          { projectId: projectId || projectPath }
        );
      } finally {
        await session.close();
      }
    }
    
    await this.updateProgress(job, 100, 'Project indexing initiated');
    
    return this.createSuccessResult({
      projectPath,
      projectId: projectId || projectPath,
      directoriesQueued: directories.length
    });
  }

  /**
   * Update index for a file
   */
  private async updateIndex(job: Job<CodeIndexingJobData>): Promise<JobResult> {
    // Re-index the file
    return this.indexFile(job);
  }

  /**
   * Remove file from index
   */
  private async removeFromIndex(job: Job<CodeIndexingJobData>): Promise<JobResult> {
    const { path: filePath } = job.data;
    
    await this.updateProgress(job, 0, 'Removing from Neo4j');
    
    // Remove from Neo4j
    if (this.neo4jManager) {
      const session = this.neo4jManager.getSession();
      try {
        await session.run(
          `
          MATCH (f:File {path: $path})
          DETACH DELETE f
          `,
          { path: filePath }
        );
      } finally {
        await session.close();
      }
    }
    
    await this.updateProgress(job, 50, 'Removing from Qdrant');
    
    // Remove from Qdrant
    if (this.qdrantManager) {
      // Find all chunks for this file
      const searchResult = await this.qdrantManager.client.scroll('code', {
        filter: {
          must: [
            {
              key: 'filePath',
              match: { value: filePath }
            }
          ]
        },
        limit: 1000
      });
      
      if (searchResult.points.length > 0) {
        const ids = searchResult.points.map(p => p.id);
        await this.qdrantManager.client.delete('code', {
          points: ids
        });
      }
    }
    
    await this.updateProgress(job, 100, 'Complete');
    
    return this.createSuccessResult({
      filePath,
      removed: true
    });
  }

  /**
   * Chunk code for embedding
   */
  private chunkCode(
    content: string,
    filePath: string
  ): Array<{ content: string; startLine: number; endLine: number }> {
    const lines = content.split('\n');
    const chunks: Array<{ content: string; startLine: number; endLine: number }> = [];
    const chunkSize = 50; // Lines per chunk
    const overlap = 10; // Overlapping lines
    
    for (let i = 0; i < lines.length; i += chunkSize - overlap) {
      const startLine = i;
      const endLine = Math.min(i + chunkSize, lines.length);
      const chunkLines = lines.slice(startLine, endLine);
      
      chunks.push({
        content: chunkLines.join('\n'),
        startLine: startLine + 1,
        endLine
      });
      
      if (endLine >= lines.length) break;
    }
    
    return chunks;
  }

  /**
   * Scan directory for code files
   */
  private async scanDirectory(
    dirPath: string,
    options?: any
  ): Promise<Array<{ path: string; language: string }>> {
    const files: Array<{ path: string; language: string }> = [];
    const codeExtensions = [
      '.js', '.jsx', '.ts', '.tsx', '.py', '.java', '.cpp', '.c',
      '.cs', '.rb', '.go', '.rs', '.php', '.swift', '.kt', '.scala'
    ];
    
    const entries = await fs.readdir(dirPath, { withFileTypes: true });
    
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);
      
      if (entry.isDirectory()) {
        // Skip node_modules, .git, etc.
        if (!this.shouldSkipDirectory(entry.name, options)) {
          const subFiles = await this.scanDirectory(fullPath, options);
          files.push(...subFiles);
        }
      } else if (entry.isFile()) {
        const ext = path.extname(entry.name);
        if (codeExtensions.includes(ext)) {
          files.push({
            path: fullPath,
            language: this.detectLanguage(ext)
          });
        }
      }
    }
    
    return files;
  }

  /**
   * Get project directories
   */
  private async getProjectDirectories(projectPath: string): Promise<string[]> {
    const directories: string[] = [projectPath];
    
    const scanDir = async (dir: string) => {
      const entries = await fs.readdir(dir, { withFileTypes: true });
      
      for (const entry of entries) {
        if (entry.isDirectory() && !this.shouldSkipDirectory(entry.name)) {
          const fullPath = path.join(dir, entry.name);
          directories.push(fullPath);
          await scanDir(fullPath);
        }
      }
    };
    
    await scanDir(projectPath);
    return directories;
  }

  /**
   * Check if directory should be skipped
   */
  private shouldSkipDirectory(name: string, options?: any): boolean {
    const skipDirs = [
      'node_modules', '.git', '.svn', '.hg', 'dist', 'build',
      'coverage', '.next', '.nuxt', '__pycache__', 'venv'
    ];
    
    if (options?.includeTests === false && name.includes('test')) {
      return true;
    }
    
    return skipDirs.includes(name) || name.startsWith('.');
  }

  /**
   * Detect language from file extension
   */
  private detectLanguage(ext: string): string {
    const languageMap: { [key: string]: string } = {
      '.js': 'javascript',
      '.jsx': 'javascript',
      '.ts': 'typescript',
      '.tsx': 'typescript',
      '.py': 'python',
      '.java': 'java',
      '.cpp': 'cpp',
      '.c': 'c',
      '.cs': 'csharp',
      '.rb': 'ruby',
      '.go': 'go',
      '.rs': 'rust',
      '.php': 'php',
      '.swift': 'swift',
      '.kt': 'kotlin',
      '.scala': 'scala'
    };
    
    return languageMap[ext] || 'unknown';
  }

  /**
   * Validate job data
   */
  protected validateJobData(data: CodeIndexingJobData): void {
    if (!data.path) {
      throw new Error('Path is required for code indexing job');
    }
    
    if (!data.type) {
      throw new Error('Job type is required');
    }
  }
}