/**
 * File Watcher for Qdrant Indexing
 * 
 * Monitors file changes and updates the Qdrant index in real-time
 */

import * as chokidar from 'chokidar';
import * as path from 'path';
import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import { CodeIndexer } from './code-indexer';
import { debounce } from 'lodash';

interface WatcherConfig {
  // Paths to watch
  paths: string[];
  ignored?: string[];
  
  // File patterns
  extensions?: string[];
  
  // Performance
  debounceMs?: number;
  batchSize?: number;
  
  // Features
  usePolling?: boolean;
  depth?: number;
}

interface FileChange {
  type: 'add' | 'change' | 'unlink';
  path: string;
  timestamp: Date;
}

export class QdrantFileWatcher extends EventEmitter {
  private watcher?: chokidar.FSWatcher;
  private indexer: CodeIndexer;
  private config: WatcherConfig;
  private logger = new Logger('QdrantFileWatcher');
  private pendingChanges = new Map<string, FileChange>();
  private processChanges: () => void;
  
  constructor(indexer: CodeIndexer, config: WatcherConfig) {
    super();
    
    this.indexer = indexer;
    this.config = {
      debounceMs: 1000,
      batchSize: 10,
      extensions: ['.js', '.jsx', '.ts', '.tsx', '.py', '.java', '.go'],
      ...config
    };
    
    // Create debounced change processor
    this.processChanges = debounce(
      this.processPendingChanges.bind(this),
      this.config.debounceMs
    );
  }
  
  /**
   * Start watching files
   */
  async start(): Promise<void> {
    if (this.watcher) {
      this.logger.warn('Watcher already started');
      return;
    }
    
    this.logger.info('Starting file watcher', {
      paths: this.config.paths,
      extensions: this.config.extensions
    });
    
    // Create watcher
    this.watcher = chokidar.watch(this.config.paths, {
      ignored: this.config.ignored || [
        /(^|[\/\\])\../,  // Hidden files
        '**/node_modules/**',
        '**/dist/**',
        '**/build/**',
        '**/.git/**'
      ],
      persistent: true,
      usePolling: this.config.usePolling,
      depth: this.config.depth,
      awaitWriteFinish: {
        stabilityThreshold: 500,
        pollInterval: 100
      }
    });
    
    // Set up event handlers
    this.watcher
      .on('add', (filePath) => this.handleFileAdd(filePath))
      .on('change', (filePath) => this.handleFileChange(filePath))
      .on('unlink', (filePath) => this.handleFileRemove(filePath))
      .on('error', (error) => this.handleError(error))
      .on('ready', () => this.handleReady());
  }
  
  /**
   * Stop watching files
   */
  async stop(): Promise<void> {
    if (!this.watcher) {
      return;
    }
    
    this.logger.info('Stopping file watcher');
    
    await this.watcher.close();
    this.watcher = undefined;
    
    // Process any remaining changes
    if (this.pendingChanges.size > 0) {
      await this.processPendingChanges();
    }
    
    this.emit('stopped');
  }
  
  /**
   * Handle file addition
   */
  private handleFileAdd(filePath: string): void {
    if (!this.shouldProcess(filePath)) {
      return;
    }
    
    this.logger.debug(`File added: ${filePath}`);
    
    this.pendingChanges.set(filePath, {
      type: 'add',
      path: filePath,
      timestamp: new Date()
    });
    
    this.processChanges();
  }
  
  /**
   * Handle file change
   */
  private handleFileChange(filePath: string): void {
    if (!this.shouldProcess(filePath)) {
      return;
    }
    
    this.logger.debug(`File changed: ${filePath}`);
    
    this.pendingChanges.set(filePath, {
      type: 'change',
      path: filePath,
      timestamp: new Date()
    });
    
    this.processChanges();
  }
  
  /**
   * Handle file removal
   */
  private handleFileRemove(filePath: string): void {
    if (!this.shouldProcess(filePath)) {
      return;
    }
    
    this.logger.debug(`File removed: ${filePath}`);
    
    this.pendingChanges.set(filePath, {
      type: 'unlink',
      path: filePath,
      timestamp: new Date()
    });
    
    this.processChanges();
  }
  
  /**
   * Handle watcher error
   */
  private handleError(error: Error): void {
    this.logger.error('File watcher error', error);
    this.emit('error', error);
  }
  
  /**
   * Handle watcher ready
   */
  private handleReady(): void {
    this.logger.info('File watcher ready');
    this.emit('ready');
  }
  
  /**
   * Check if file should be processed
   */
  private shouldProcess(filePath: string): boolean {
    // Check extension
    const ext = path.extname(filePath).toLowerCase();
    if (!this.config.extensions?.includes(ext)) {
      return false;
    }
    
    // Additional filters can be added here
    return true;
  }
  
  /**
   * Process pending changes
   */
  private async processPendingChanges(): Promise<void> {
    if (this.pendingChanges.size === 0) {
      return;
    }
    
    const changes = Array.from(this.pendingChanges.values());
    this.pendingChanges.clear();
    
    this.logger.info(`Processing ${changes.length} file changes`);
    
    // Group changes by type
    const additions = changes.filter(c => c.type === 'add');
    const modifications = changes.filter(c => c.type === 'change');
    const deletions = changes.filter(c => c.type === 'unlink');
    
    try {
      // Process deletions first
      for (const change of deletions) {
        await this.indexer.removeFile(change.path);
        this.emit('file-removed', change.path);
      }
      
      // Process additions and modifications in batches
      const toIndex = [...additions, ...modifications];
      const batchSize = this.config.batchSize || 10;
      
      for (let i = 0; i < toIndex.length; i += batchSize) {
        const batch = toIndex.slice(i, i + batchSize);
        
        await Promise.all(
          batch.map(async (change) => {
            try {
              if (change.type === 'add') {
                await this.indexer.indexFile(change.path);
                this.emit('file-indexed', change.path);
              } else {
                await this.indexer.updateFile(change.path);
                this.emit('file-updated', change.path);
              }
            } catch (error) {
              this.logger.error(`Failed to process ${change.path}`, error);
              this.emit('processing-error', { path: change.path, error });
            }
          })
        );
      }
      
      this.emit('batch-completed', {
        additions: additions.length,
        modifications: modifications.length,
        deletions: deletions.length
      });
      
    } catch (error) {
      this.logger.error('Failed to process changes', error);
      this.emit('error', error);
    }
  }
  
  /**
   * Get watcher statistics
   */
  getStats(): {
    watching: boolean;
    pendingChanges: number;
    watchedPaths: string[];
  } {
    return {
      watching: !!this.watcher,
      pendingChanges: this.pendingChanges.size,
      watchedPaths: this.config.paths
    };
  }
  
  /**
   * Force process specific files
   */
  async reindexFiles(filePaths: string[]): Promise<void> {
    this.logger.info(`Force reindexing ${filePaths.length} files`);
    
    for (const filePath of filePaths) {
      if (this.shouldProcess(filePath)) {
        this.pendingChanges.set(filePath, {
          type: 'change',
          path: filePath,
          timestamp: new Date()
        });
      }
    }
    
    await this.processPendingChanges();
  }
}