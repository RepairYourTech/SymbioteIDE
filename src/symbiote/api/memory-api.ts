/**
 * Memory API - Memory management API interface
 */

export interface MemoryAPI {
  store(key: string, value: any, metadata?: Record<string, any>): Promise<void>;
  retrieve(key: string): Promise<any>;
  search(query: string, options?: MemorySearchOptions): Promise<MemorySearchResult[]>;
  delete(key: string): Promise<void>;
  update(key: string, value: any, metadata?: Record<string, any>): Promise<void>;
  list(options?: MemoryListOptions): Promise<MemoryItem[]>;
}

export interface MemorySearchOptions {
  limit?: number;
  offset?: number;
  filters?: Record<string, any>;
  similarity?: number;
  includeMetadata?: boolean;
}

export interface MemorySearchResult {
  key: string;
  value: any;
  score: number;
  metadata?: Record<string, any>;
}

export interface MemoryListOptions {
  limit?: number;
  offset?: number;
  prefix?: string;
  includeMetadata?: boolean;
}

export interface MemoryItem {
  key: string;
  value: any;
  createdAt: Date;
  updatedAt: Date;
  metadata?: Record<string, any>;
}

export class MemoryAPIImpl implements MemoryAPI {
  private storage: Map<string, MemoryItem> = new Map();

  async store(key: string, value: any, metadata?: Record<string, any>): Promise<void> {
    const now = new Date();
    this.storage.set(key, {
      key,
      value,
      createdAt: now,
      updatedAt: now,
      metadata
    });
  }

  async retrieve(key: string): Promise<any> {
    const item = this.storage.get(key);
    return item?.value || null;
  }

  async search(query: string, options?: MemorySearchOptions): Promise<MemorySearchResult[]> {
    const results: MemorySearchResult[] = [];
    
    for (const [key, item] of this.storage) {
      // Simple string matching for now
      if (JSON.stringify(item.value).toLowerCase().includes(query.toLowerCase())) {
        results.push({
          key,
          value: item.value,
          score: 1.0,
          metadata: options?.includeMetadata ? item.metadata : undefined
        });
      }
    }

    // Apply limit
    if (options?.limit) {
      return results.slice(0, options.limit);
    }

    return results;
  }

  async delete(key: string): Promise<void> {
    this.storage.delete(key);
  }

  async update(key: string, value: any, metadata?: Record<string, any>): Promise<void> {
    const existing = this.storage.get(key);
    if (existing) {
      existing.value = value;
      existing.updatedAt = new Date();
      if (metadata) {
        existing.metadata = { ...existing.metadata, ...metadata };
      }
    } else {
      await this.store(key, value, metadata);
    }
  }

  async list(options?: MemoryListOptions): Promise<MemoryItem[]> {
    let items = Array.from(this.storage.values());

    // Apply prefix filter
    if (options?.prefix) {
      items = items.filter(item => item.key.startsWith(options.prefix!));
    }

    // Apply offset and limit
    const offset = options?.offset || 0;
    const limit = options?.limit || items.length;
    
    return items.slice(offset, offset + limit);
  }
}

export default MemoryAPIImpl;