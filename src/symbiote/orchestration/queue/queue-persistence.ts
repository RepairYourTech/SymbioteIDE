/**
 * Queue Persistence Implementation
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { 
  QueuePersistence, 
  QueuedRequest, 
  DeadLetterEntry,
  RequestStatus,
  RequestPriority
} from './queue-interfaces';
import { AITask, TaskResult, ModelProfile } from '../interfaces';

export class FileSystemQueuePersistence implements QueuePersistence {
  private basePath: string;
  private queuePath: string;
  private deadLetterPath: string;
  private readonly filePrefix = 'queue';
  private readonly deadLetterPrefix = 'deadletter';

  constructor(basePath: string = '.symbiote/queue') {
    this.basePath = basePath;
    this.queuePath = path.join(basePath, 'active');
    this.deadLetterPath = path.join(basePath, 'deadletter');
  }

  /**
   * Initialize persistence directories
   */
  async initialize(): Promise<void> {
    await fs.mkdir(this.queuePath, { recursive: true });
    await fs.mkdir(this.deadLetterPath, { recursive: true });
  }

  /**
   * Save queue state to disk
   */
  async save(requests: QueuedRequest[]): Promise<void> {
    await this.initialize();

    // Group requests by priority for organized storage
    const requestsByPriority = new Map<RequestPriority, QueuedRequest[]>();
    
    for (const request of requests) {
      const priority = request.priority;
      if (!requestsByPriority.has(priority)) {
        requestsByPriority.set(priority, []);
      }
      requestsByPriority.get(priority)!.push(request);
    }

    // Save each priority group to a separate file
    for (const [priority, priorityRequests] of requestsByPriority) {
      const filename = `${this.filePrefix}_priority_${priority}.json`;
      const filepath = path.join(this.queuePath, filename);
      
      const data = {
        version: '1.0',
        priority,
        timestamp: new Date().toISOString(),
        count: priorityRequests.length,
        requests: priorityRequests.map(req => this.serializeRequest(req))
      };

      await fs.writeFile(filepath, JSON.stringify(data, null, 2), 'utf-8');
    }

    // Clean up files for priorities with no requests
    await this.cleanupEmptyPriorityFiles(requestsByPriority);
  }

  /**
   * Load queue state from disk
   */
  async load(): Promise<QueuedRequest[]> {
    await this.initialize();

    const requests: QueuedRequest[] = [];
    
    try {
      const files = await fs.readdir(this.queuePath);
      const queueFiles = files.filter(f => f.startsWith(this.filePrefix) && f.endsWith('.json'));

      for (const file of queueFiles) {
        const filepath = path.join(this.queuePath, file);
        const content = await fs.readFile(filepath, 'utf-8');
        const data = JSON.parse(content);

        if (data.version === '1.0' && Array.isArray(data.requests)) {
          for (const serialized of data.requests) {
            try {
              const request = this.deserializeRequest(serialized);
              requests.push(request);
            } catch (error) {
              console.error(`Failed to deserialize request: ${error}`);
            }
          }
        }
      }
    } catch (error) {
      if ((error as any).code !== 'ENOENT') {
        throw error;
      }
    }

    return requests;
  }

  /**
   * Save dead letter entry
   */
  async saveDeadLetter(entry: DeadLetterEntry): Promise<void> {
    await this.initialize();

    const filename = `${this.deadLetterPrefix}_${entry.request.id}_${Date.now()}.json`;
    const filepath = path.join(this.deadLetterPath, filename);

    const data = {
      version: '1.0',
      timestamp: entry.timestamp.toISOString(),
      reason: entry.reason,
      attempts: entry.attempts,
      error: entry.lastError ? {
        message: entry.lastError.message,
        stack: entry.lastError.stack,
        name: entry.lastError.name
      } : null,
      request: this.serializeRequest(entry.request)
    };

    await fs.writeFile(filepath, JSON.stringify(data, null, 2), 'utf-8');
  }

  /**
   * Load dead letter entries
   */
  async loadDeadLetter(): Promise<DeadLetterEntry[]> {
    await this.initialize();

    const entries: DeadLetterEntry[] = [];
    
    try {
      const files = await fs.readdir(this.deadLetterPath);
      const deadLetterFiles = files.filter(f => 
        f.startsWith(this.deadLetterPrefix) && f.endsWith('.json')
      );

      for (const file of deadLetterFiles) {
        const filepath = path.join(this.deadLetterPath, file);
        const content = await fs.readFile(filepath, 'utf-8');
        const data = JSON.parse(content);

        if (data.version === '1.0') {
          const entry: DeadLetterEntry = {
            request: this.deserializeRequest(data.request),
            reason: data.reason,
            timestamp: new Date(data.timestamp),
            attempts: data.attempts,
            lastError: data.error ? new Error(data.error.message) : undefined
          };
          entries.push(entry);
        }
      }
    } catch (error) {
      if ((error as any).code !== 'ENOENT') {
        throw error;
      }
    }

    return entries;
  }

  /**
   * Clear all persisted data
   */
  async clear(): Promise<void> {
    try {
      // Clear active queue
      const queueFiles = await fs.readdir(this.queuePath);
      for (const file of queueFiles) {
        if (file.startsWith(this.filePrefix)) {
          await fs.unlink(path.join(this.queuePath, file));
        }
      }

      // Clear dead letter queue
      const deadLetterFiles = await fs.readdir(this.deadLetterPath);
      for (const file of deadLetterFiles) {
        if (file.startsWith(this.deadLetterPrefix)) {
          await fs.unlink(path.join(this.deadLetterPath, file));
        }
      }
    } catch (error) {
      if ((error as any).code !== 'ENOENT') {
        throw error;
      }
    }
  }

  /**
   * Archive old dead letter entries
   */
  async archiveOldDeadLetters(daysOld: number = 30): Promise<number> {
    await this.initialize();

    const archivePath = path.join(this.basePath, 'archive');
    await fs.mkdir(archivePath, { recursive: true });

    const cutoffTime = Date.now() - (daysOld * 24 * 60 * 60 * 1000);
    let archivedCount = 0;

    try {
      const files = await fs.readdir(this.deadLetterPath);
      
      for (const file of files) {
        if (!file.startsWith(this.deadLetterPrefix)) continue;

        const filepath = path.join(this.deadLetterPath, file);
        const stats = await fs.stat(filepath);

        if (stats.mtimeMs < cutoffTime) {
          const archiveFilepath = path.join(archivePath, file);
          await fs.rename(filepath, archiveFilepath);
          archivedCount++;
        }
      }
    } catch (error) {
      if ((error as any).code !== 'ENOENT') {
        throw error;
      }
    }

    return archivedCount;
  }

  /**
   * Get persistence statistics
   */
  async getStats(): Promise<{
    activeRequests: number;
    deadLetterEntries: number;
    oldestRequest?: Date;
    totalSize: number;
  }> {
    let activeRequests = 0;
    let deadLetterEntries = 0;
    let oldestRequest: Date | undefined;
    let totalSize = 0;

    try {
      // Count active requests
      const queueFiles = await fs.readdir(this.queuePath);
      for (const file of queueFiles) {
        if (file.startsWith(this.filePrefix)) {
          const filepath = path.join(this.queuePath, file);
          const stats = await fs.stat(filepath);
          totalSize += stats.size;

          const content = await fs.readFile(filepath, 'utf-8');
          const data = JSON.parse(content);
          activeRequests += data.count || 0;

          if (data.requests && data.requests.length > 0) {
            for (const req of data.requests) {
              const enqueuedAt = new Date(req.enqueuedAt);
              if (!oldestRequest || enqueuedAt < oldestRequest) {
                oldestRequest = enqueuedAt;
              }
            }
          }
        }
      }

      // Count dead letter entries
      const deadLetterFiles = await fs.readdir(this.deadLetterPath);
      for (const file of deadLetterFiles) {
        if (file.startsWith(this.deadLetterPrefix)) {
          deadLetterEntries++;
          const filepath = path.join(this.deadLetterPath, file);
          const stats = await fs.stat(filepath);
          totalSize += stats.size;
        }
      }
    } catch (error) {
      if ((error as any).code !== 'ENOENT') {
        throw error;
      }
    }

    return {
      activeRequests,
      deadLetterEntries,
      oldestRequest,
      totalSize
    };
  }

  /**
   * Serialize request for storage
   */
  private serializeRequest(request: QueuedRequest): any {
    return {
      id: request.id,
      task: request.task,
      priority: request.priority,
      enqueuedAt: request.enqueuedAt.toISOString(),
      startedAt: request.startedAt?.toISOString(),
      completedAt: request.completedAt?.toISOString(),
      attempts: request.attempts,
      status: request.status,
      assignedModel: request.assignedModel,
      result: request.result,
      error: request.error ? {
        message: request.error.message,
        stack: request.error.stack,
        name: request.error.name
      } : undefined,
      metadata: request.metadata
    };
  }

  /**
   * Deserialize request from storage
   */
  private deserializeRequest(data: any): QueuedRequest {
    return {
      id: data.id,
      task: data.task as AITask,
      priority: data.priority as RequestPriority,
      enqueuedAt: new Date(data.enqueuedAt),
      startedAt: data.startedAt ? new Date(data.startedAt) : undefined,
      completedAt: data.completedAt ? new Date(data.completedAt) : undefined,
      attempts: data.attempts,
      status: data.status as RequestStatus,
      assignedModel: data.assignedModel as ModelProfile | undefined,
      result: data.result as TaskResult | undefined,
      error: data.error ? new Error(data.error.message) : undefined,
      metadata: data.metadata
    };
  }

  /**
   * Clean up empty priority files
   */
  private async cleanupEmptyPriorityFiles(
    currentPriorities: Map<RequestPriority, QueuedRequest[]>
  ): Promise<void> {
    try {
      const files = await fs.readdir(this.queuePath);
      const priorityFiles = files.filter(f => 
        f.startsWith(this.filePrefix) && f.includes('priority')
      );

      for (const file of priorityFiles) {
        const match = file.match(/priority_(\d+)/);
        if (match) {
          const priority = parseInt(match[1]) as RequestPriority;
          if (!currentPriorities.has(priority) || currentPriorities.get(priority)!.length === 0) {
            await fs.unlink(path.join(this.queuePath, file));
          }
        }
      }
    } catch (error) {
      // Ignore cleanup errors
    }
  }
}

/**
 * In-Memory Queue Persistence (for testing or when persistence is disabled)
 */
export class InMemoryQueuePersistence implements QueuePersistence {
  private requests: QueuedRequest[] = [];
  private deadLetterEntries: DeadLetterEntry[] = [];

  async save(requests: QueuedRequest[]): Promise<void> {
    this.requests = [...requests];
  }

  async load(): Promise<QueuedRequest[]> {
    return [...this.requests];
  }

  async saveDeadLetter(entry: DeadLetterEntry): Promise<void> {
    this.deadLetterEntries.push(entry);
  }

  async loadDeadLetter(): Promise<DeadLetterEntry[]> {
    return [...this.deadLetterEntries];
  }

  async clear(): Promise<void> {
    this.requests = [];
    this.deadLetterEntries = [];
  }
}