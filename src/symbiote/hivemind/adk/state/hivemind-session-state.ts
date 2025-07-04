/**
 * Hivemind Session State
 * 
 * Thread-safe shared state management for multi-agent coordination
 */

import { EventEmitter } from 'events';
import { Logger } from '../../../utils/logger';

export interface StateChange {
  key: string;
  value: any;
  previousValue: any;
  agentId: string;
  timestamp: Date;
}

export interface StateSnapshot {
  state: Map<string, any>;
  version: number;
  timestamp: Date;
}

export class HivemindSessionState extends EventEmitter {
  private logger = new Logger('HivemindSessionState');
  private state = new Map<string, any>();
  private locks = new Map<string, string>(); // key -> agentId
  private version = 0;
  private history: StateChange[] = [];
  private snapshots: StateSnapshot[] = [];
  
  constructor(private readonly sessionId: string) {
    super();
  }
  
  /**
   * Get value from state
   */
  async get(key: string): Promise<any> {
    return this.state.get(key);
  }
  
  /**
   * Set value in state with optional locking
   */
  async set(key: string, value: any, agentId?: string): Promise<void> {
    // Check if key is locked by another agent
    const lockHolder = this.locks.get(key);
    if (lockHolder && lockHolder !== agentId) {
      throw new Error(`Key ${key} is locked by agent ${lockHolder}`);
    }
    
    const previousValue = this.state.get(key);
    this.state.set(key, value);
    this.version++;
    
    // Record change
    const change: StateChange = {
      key,
      value,
      previousValue,
      agentId: agentId || 'system',
      timestamp: new Date()
    };
    
    this.history.push(change);
    
    // Emit change event
    this.emit('state-changed', change);
    
    this.logger.debug(`State updated: ${key} by ${agentId || 'system'}`);
  }
  
  /**
   * Delete key from state
   */
  async delete(key: string, agentId?: string): Promise<void> {
    const lockHolder = this.locks.get(key);
    if (lockHolder && lockHolder !== agentId) {
      throw new Error(`Key ${key} is locked by agent ${lockHolder}`);
    }
    
    const previousValue = this.state.get(key);
    this.state.delete(key);
    this.version++;
    
    const change: StateChange = {
      key,
      value: undefined,
      previousValue,
      agentId: agentId || 'system',
      timestamp: new Date()
    };
    
    this.history.push(change);
    this.emit('state-deleted', change);
  }
  
  /**
   * Get multiple values
   */
  async getMultiple(keys: string[]): Promise<Map<string, any>> {
    const result = new Map<string, any>();
    for (const key of keys) {
      result.set(key, this.state.get(key));
    }
    return result;
  }
  
  /**
   * Set multiple values atomically
   */
  async setMultiple(updates: Map<string, any>, agentId?: string): Promise<void> {
    // Check all locks first
    for (const key of updates.keys()) {
      const lockHolder = this.locks.get(key);
      if (lockHolder && lockHolder !== agentId) {
        throw new Error(`Key ${key} is locked by agent ${lockHolder}`);
      }
    }
    
    // Apply all updates
    const changes: StateChange[] = [];
    for (const [key, value] of updates) {
      const previousValue = this.state.get(key);
      this.state.set(key, value);
      
      changes.push({
        key,
        value,
        previousValue,
        agentId: agentId || 'system',
        timestamp: new Date()
      });
    }
    
    this.version++;
    this.history.push(...changes);
    
    // Emit batch update event
    this.emit('state-batch-updated', { changes, agentId });
  }
  
  /**
   * Lock a key for exclusive access
   */
  async lock(key: string, agentId: string): Promise<boolean> {
    const currentLock = this.locks.get(key);
    if (currentLock && currentLock !== agentId) {
      return false;
    }
    
    this.locks.set(key, agentId);
    this.emit('key-locked', { key, agentId });
    return true;
  }
  
  /**
   * Unlock a key
   */
  async unlock(key: string, agentId: string): Promise<void> {
    const currentLock = this.locks.get(key);
    if (currentLock === agentId) {
      this.locks.delete(key);
      this.emit('key-unlocked', { key, agentId });
    }
  }
  
  /**
   * Get all keys matching a pattern
   */
  async getKeys(pattern?: RegExp): Promise<string[]> {
    const keys = Array.from(this.state.keys());
    if (pattern) {
      return keys.filter(key => pattern.test(key));
    }
    return keys;
  }
  
  /**
   * Create a snapshot of current state
   */
  async createSnapshot(): Promise<StateSnapshot> {
    const snapshot: StateSnapshot = {
      state: new Map(this.state),
      version: this.version,
      timestamp: new Date()
    };
    
    this.snapshots.push(snapshot);
    this.emit('snapshot-created', snapshot);
    
    return snapshot;
  }
  
  /**
   * Restore from snapshot
   */
  async restoreSnapshot(version: number): Promise<void> {
    const snapshot = this.snapshots.find(s => s.version === version);
    if (!snapshot) {
      throw new Error(`Snapshot version ${version} not found`);
    }
    
    this.state = new Map(snapshot.state);
    this.version = snapshot.version;
    
    this.emit('snapshot-restored', snapshot);
  }
  
  /**
   * Get state history
   */
  getHistory(limit?: number): StateChange[] {
    if (limit) {
      return this.history.slice(-limit);
    }
    return [...this.history];
  }
  
  /**
   * Get changes by agent
   */
  getChangesByAgent(agentId: string): StateChange[] {
    return this.history.filter(change => change.agentId === agentId);
  }
  
  /**
   * Clear all state
   */
  async clear(): Promise<void> {
    this.state.clear();
    this.locks.clear();
    this.version = 0;
    this.history = [];
    this.snapshots = [];
    
    this.emit('state-cleared');
  }
  
  /**
   * Export state as JSON
   */
  toJSON(): any {
    return {
      sessionId: this.sessionId,
      version: this.version,
      state: Object.fromEntries(this.state),
      locks: Object.fromEntries(this.locks),
      historyCount: this.history.length,
      snapshotCount: this.snapshots.length
    };
  }
  
  /**
   * Get state metrics
   */
  getMetrics(): any {
    return {
      keyCount: this.state.size,
      lockCount: this.locks.size,
      version: this.version,
      historySize: this.history.length,
      snapshotCount: this.snapshots.length
    };
  }
}