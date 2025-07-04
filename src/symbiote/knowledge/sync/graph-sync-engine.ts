/**
 * Graph Synchronization Engine
 * 
 * Keeps the Neo4j graph in sync with file system changes in real-time
 */

import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import * as path from 'path';
import { Neo4jConnectionManager } from '../neo4j/connection-manager';
import { GraphIngestionPipeline } from '../ingestion/ingestion-pipeline';
import { TypeScriptParser } from '../parsers/typescript-parser';
import {
  GraphUpdate,
  BatchGraphUpdate,
  NodeType,
  RelationType,
  GraphError,
  GraphErrorCode,
  ChangeInfo
} from '../types/graph-types';

export interface SyncConfig {
  debounceDelay: number;
  batchUpdates: boolean;
  autoReconnect: boolean;
  trackGitChanges: boolean;
}

export interface FileChangeEvent {
  type: 'created' | 'modified' | 'deleted' | 'renamed';
  uri: vscode.Uri;
  oldUri?: vscode.Uri; // For renames
}

export interface SyncStatus {
  isActive: boolean;
  projectsWatching: Map<string, ProjectSyncStatus>;
  totalChangesProcessed: number;
  lastSyncTime?: Date;
}

export interface ProjectSyncStatus {
  projectId: string;
  databaseId: string;
  workspacePath: string;
  isActive: boolean;
  pendingChanges: number;
  lastSync?: Date;
}

export class GraphSyncEngine extends EventEmitter {
  private connectionManager: Neo4jConnectionManager;
  private ingestionPipeline: GraphIngestionPipeline;
  private parser: TypeScriptParser;
  private config: SyncConfig;
  
  private fileWatchers: Map<string, vscode.FileSystemWatcher> = new Map();
  private pendingChanges: Map<string, Set<FileChangeEvent>> = new Map();
  private debounceTimers: Map<string, NodeJS.Timeout> = new Map();
  private projectStatus: Map<string, ProjectSyncStatus> = new Map();
  
  constructor(
    connectionManager: Neo4jConnectionManager,
    config: Partial<SyncConfig> = {}
  ) {
    super();
    
    this.connectionManager = connectionManager;
    this.ingestionPipeline = new GraphIngestionPipeline(connectionManager);
    this.parser = new TypeScriptParser();
    
    this.config = {
      debounceDelay: config.debounceDelay || 500,
      batchUpdates: config.batchUpdates ?? true,
      autoReconnect: config.autoReconnect ?? true,
      trackGitChanges: config.trackGitChanges ?? true
    };
  }
  
  /**
   * Start watching a project for changes
   */
  async startWatching(
    projectId: string,
    workspacePath: string,
    databaseId: string
  ): Promise<void> {
    // Check if already watching
    if (this.fileWatchers.has(projectId)) {
      return;
    }
    
    // Create project status
    const status: ProjectSyncStatus = {
      projectId,
      databaseId,
      workspacePath,
      isActive: true,
      pendingChanges: 0
    };
    
    this.projectStatus.set(projectId, status);
    this.pendingChanges.set(projectId, new Set());
    
    // Create file watcher
    const pattern = new vscode.RelativePattern(
      workspacePath,
      '**/*.{ts,tsx,js,jsx}'
    );
    
    const watcher = vscode.workspace.createFileSystemWatcher(
      pattern,
      false, // ignoreCreateEvents
      false, // ignoreChangeEvents
      false  // ignoreDeleteEvents
    );
    
    // Set up event handlers
    watcher.onDidCreate(uri => this.handleFileChange({
      type: 'created',
      uri
    }, projectId));
    
    watcher.onDidChange(uri => this.handleFileChange({
      type: 'modified',
      uri
    }, projectId));
    
    watcher.onDidDelete(uri => this.handleFileChange({
      type: 'deleted',
      uri
    }, projectId));
    
    this.fileWatchers.set(projectId, watcher);
    
    // Set up git integration if enabled
    if (this.config.trackGitChanges) {
      this.setupGitIntegration(projectId, workspacePath);
    }
    
    this.emit('watching-started', { projectId, workspacePath });
  }
  
  /**
   * Stop watching a project
   */
  async stopWatching(projectId: string): Promise<void> {
    const watcher = this.fileWatchers.get(projectId);
    if (watcher) {
      watcher.dispose();
      this.fileWatchers.delete(projectId);
    }
    
    // Clear pending changes
    this.pendingChanges.delete(projectId);
    
    // Clear debounce timer
    const timer = this.debounceTimers.get(projectId);
    if (timer) {
      clearTimeout(timer);
      this.debounceTimers.delete(projectId);
    }
    
    // Update status
    const status = this.projectStatus.get(projectId);
    if (status) {
      status.isActive = false;
    }
    
    this.emit('watching-stopped', { projectId });
  }
  
  /**
   * Stop watching all projects
   */
  async stopAll(): Promise<void> {
    const projectIds = Array.from(this.fileWatchers.keys());
    await Promise.all(projectIds.map(id => this.stopWatching(id)));
  }
  
  /**
   * Get sync status
   */
  getSyncStatus(): SyncStatus {
    let totalChanges = 0;
    let lastSyncTime: Date | undefined;
    
    for (const status of this.projectStatus.values()) {
      totalChanges += status.pendingChanges;
      if (status.lastSync && (!lastSyncTime || status.lastSync > lastSyncTime)) {
        lastSyncTime = status.lastSync;
      }
    }
    
    return {
      isActive: this.fileWatchers.size > 0,
      projectsWatching: new Map(this.projectStatus),
      totalChangesProcessed: totalChanges,
      lastSyncTime
    };
  }
  
  /**
   * Force sync for a project
   */
  async forceSync(projectId: string): Promise<void> {
    const changes = this.pendingChanges.get(projectId);
    if (changes && changes.size > 0) {
      await this.processPendingChanges(projectId);
    }
  }
  
  private handleFileChange(event: FileChangeEvent, projectId: string): void {
    // Add to pending changes
    const changes = this.pendingChanges.get(projectId);
    if (!changes) return;
    
    changes.add(event);
    
    // Update status
    const status = this.projectStatus.get(projectId);
    if (status) {
      status.pendingChanges = changes.size;
    }
    
    // Debounce processing
    if (this.config.batchUpdates) {
      this.debounceProcessing(projectId);
    } else {
      this.processPendingChanges(projectId);
    }
  }
  
  private debounceProcessing(projectId: string): void {
    // Clear existing timer
    const existingTimer = this.debounceTimers.get(projectId);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }
    
    // Set new timer
    const timer = setTimeout(() => {
      this.processPendingChanges(projectId);
    }, this.config.debounceDelay);
    
    this.debounceTimers.set(projectId, timer);
  }
  
  private async processPendingChanges(projectId: string): Promise<void> {
    const changes = this.pendingChanges.get(projectId);
    const status = this.projectStatus.get(projectId);
    
    if (!changes || changes.size === 0 || !status) {
      return;
    }
    
    // Get changes and clear pending
    const changeArray = Array.from(changes);
    changes.clear();
    status.pendingChanges = 0;
    
    try {
      // Group changes by type
      const created: vscode.Uri[] = [];
      const modified: vscode.Uri[] = [];
      const deleted: vscode.Uri[] = [];
      const renamed: Array<{ oldUri: vscode.Uri; newUri: vscode.Uri }> = [];
      
      for (const change of changeArray) {
        switch (change.type) {
          case 'created':
            created.push(change.uri);
            break;
          case 'modified':
            modified.push(change.uri);
            break;
          case 'deleted':
            deleted.push(change.uri);
            break;
          case 'renamed':
            if (change.oldUri) {
              renamed.push({ oldUri: change.oldUri, newUri: change.uri });
            }
            break;
        }
      }
      
      // Process each type of change
      const updates: GraphUpdate[] = [];
      
      // Handle created files
      for (const uri of created) {
        const fileUpdates = await this.handleFileCreated(uri, status.databaseId);
        updates.push(...fileUpdates);
      }
      
      // Handle modified files
      for (const uri of modified) {
        const fileUpdates = await this.handleFileModified(uri, status.databaseId);
        updates.push(...fileUpdates);
      }
      
      // Handle deleted files
      for (const uri of deleted) {
        const fileUpdates = await this.handleFileDeleted(uri, status.databaseId);
        updates.push(...fileUpdates);
      }
      
      // Handle renamed files
      for (const { oldUri, newUri } of renamed) {
        const fileUpdates = await this.handleFileRenamed(oldUri, newUri, status.databaseId);
        updates.push(...fileUpdates);
      }
      
      // Apply updates to graph
      if (updates.length > 0) {
        await this.applyGraphUpdates(status.databaseId, updates);
        
        // Update statistics
        await this.connectionManager.updateStatistics(status.databaseId);
      }
      
      // Update status
      status.lastSync = new Date();
      
      // Emit sync event
      this.emit('sync-completed', {
        projectId,
        changesProcessed: changeArray.length,
        updatesApplied: updates.length
      });
      
    } catch (error) {
      this.emit('sync-error', {
        projectId,
        error
      });
      
      // Re-add changes to pending if failed
      changeArray.forEach(change => changes.add(change));
      status.pendingChanges = changes.size;
    }
  }
  
  private async handleFileCreated(uri: vscode.Uri, databaseId: string): Promise<GraphUpdate[]> {
    const updates: GraphUpdate[] = [];
    
    try {
      // Parse the new file
      const parseResult = await this.parser.parseFile(uri.fsPath);
      
      // Create updates for nodes
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
      
      // Create updates for relationships
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
      
      // Track the change
      const changeInfo: ChangeInfo = {
        entityId: parseResult.nodes[0]?.id || uri.fsPath,
        changeType: 'created',
        timestamp: new Date(),
        description: `File created: ${path.basename(uri.fsPath)}`
      };
      
      this.emit('file-indexed', { uri, nodeCount: parseResult.nodes.length });
      
    } catch (error) {
      console.error(`Failed to parse created file ${uri.fsPath}:`, error);
    }
    
    return updates;
  }
  
  private async handleFileModified(uri: vscode.Uri, databaseId: string): Promise<GraphUpdate[]> {
    const updates: GraphUpdate[] = [];
    
    try {
      // Delete existing nodes for this file
      await this.deleteFileNodes(uri.fsPath, databaseId);
      
      // Re-parse and create new nodes
      const createUpdates = await this.handleFileCreated(uri, databaseId);
      updates.push(...createUpdates);
      
      // Track the change
      const changeInfo: ChangeInfo = {
        entityId: uri.fsPath,
        changeType: 'modified',
        timestamp: new Date(),
        description: `File modified: ${path.basename(uri.fsPath)}`
      };
      
      this.emit('file-updated', { uri });
      
    } catch (error) {
      console.error(`Failed to handle modified file ${uri.fsPath}:`, error);
    }
    
    return updates;
  }
  
  private async handleFileDeleted(uri: vscode.Uri, databaseId: string): Promise<GraphUpdate[]> {
    const updates: GraphUpdate[] = [];
    
    try {
      // Get all nodes for this file
      const result = await this.connectionManager.executeQuery(
        databaseId,
        `
        MATCH (f:File {path: $path})-[:CONTAINS*]->(n)
        RETURN n.id as id
        `,
        { path: uri.fsPath }
      );
      
      // Create delete updates
      for (const record of result.records) {
        updates.push({
          type: 'delete',
          entityType: 'node',
          data: { id: record.get('id') } as any,
          metadata: {
            source: 'file-watcher',
            timestamp: new Date()
          }
        });
      }
      
      // Delete the file node itself
      await this.deleteFileNodes(uri.fsPath, databaseId);
      
      // Track the change
      const changeInfo: ChangeInfo = {
        entityId: uri.fsPath,
        changeType: 'deleted',
        timestamp: new Date(),
        description: `File deleted: ${path.basename(uri.fsPath)}`
      };
      
      this.emit('file-deleted', { uri });
      
    } catch (error) {
      console.error(`Failed to handle deleted file ${uri.fsPath}:`, error);
    }
    
    return updates;
  }
  
  private async handleFileRenamed(
    oldUri: vscode.Uri,
    newUri: vscode.Uri,
    databaseId: string
  ): Promise<GraphUpdate[]> {
    const updates: GraphUpdate[] = [];
    
    try {
      // Update file node path
      await this.connectionManager.executeQuery(
        databaseId,
        `
        MATCH (f:File {path: $oldPath})
        SET f.path = $newPath, f.name = $newName, f.updatedAt = datetime()
        `,
        {
          oldPath: oldUri.fsPath,
          newPath: newUri.fsPath,
          newName: path.basename(newUri.fsPath)
        }
      );
      
      // Update all contained nodes' file paths
      await this.connectionManager.executeQuery(
        databaseId,
        `
        MATCH (f:File {path: $newPath})-[:CONTAINS*]->(n)
        SET n.filePath = $newPath, n.updatedAt = datetime()
        `,
        { newPath: newUri.fsPath }
      );
      
      this.emit('file-renamed', { oldUri, newUri });
      
    } catch (error) {
      console.error(`Failed to handle renamed file ${oldUri.fsPath} -> ${newUri.fsPath}:`, error);
    }
    
    return updates;
  }
  
  private async deleteFileNodes(filePath: string, databaseId: string): Promise<void> {
    await this.connectionManager.executeQuery(
      databaseId,
      `
      MATCH (f:File {path: $path})
      OPTIONAL MATCH (f)-[:CONTAINS*]->(n)
      DETACH DELETE n, f
      `,
      { path: filePath }
    );
  }
  
  private async applyGraphUpdates(databaseId: string, updates: GraphUpdate[]): Promise<void> {
    const batch: BatchGraphUpdate = {
      updates,
      transactional: true,
      validateBeforeApply: false // Skip validation for performance
    };
    
    await this.connectionManager.executeTransaction(databaseId, async (tx) => {
      for (const update of updates) {
        if (update.type === 'create' && update.entityType === 'node') {
          const node = update.data as any;
          const labels = Array.isArray(node.labels) ? node.labels.join(':') : node.labels;
          
          await tx.run(`
            CREATE (n:${labels})
            SET n = $properties
          `, { properties: node.properties });
          
        } else if (update.type === 'create' && update.entityType === 'relationship') {
          const rel = update.data as any;
          
          await tx.run(`
            MATCH (a {id: $startId})
            MATCH (b {id: $endId})
            CREATE (a)-[r:${rel.type}]->(b)
            SET r = $properties
          `, {
            startId: rel.startNodeId,
            endId: rel.endNodeId,
            properties: rel.properties || {}
          });
          
        } else if (update.type === 'delete' && update.entityType === 'node') {
          const node = update.data as any;
          
          await tx.run(`
            MATCH (n {id: $id})
            DETACH DELETE n
          `, { id: node.id });
        }
      }
    });
  }
  
  private setupGitIntegration(projectId: string, workspacePath: string): void {
    // Watch for git operations
    const gitPattern = new vscode.RelativePattern(
      workspacePath,
      '.git/**/*'
    );
    
    const gitWatcher = vscode.workspace.createFileSystemWatcher(
      gitPattern,
      false,
      false,
      false
    );
    
    gitWatcher.onDidChange(async (uri) => {
      // Handle git operations like branch switches, merges, etc.
      if (uri.fsPath.endsWith('HEAD') || uri.fsPath.endsWith('index')) {
        // Trigger a full resync after git operations
        this.emit('git-operation-detected', { projectId });
      }
    });
    
    // Store the git watcher with the project
    // (Would need to modify the structure to support multiple watchers per project)
  }
}