/**
 * Agent Registry Module
 * 
 * Manages registration and discovery of AI agents
 */

import { EventEmitter } from 'events';

export interface AgentFactory {
  createAgent(type: string, config: any): Promise<any>;
  getAgentTypes(): string[];
}

export class AgentRegistryManager extends EventEmitter {
  private factories: Map<string, AgentFactory> = new Map();
  
  constructor() {
    super();
  }
  
  registerFactory(id: string, factory: AgentFactory): void {
    this.factories.set(id, factory);
    this.emit('factory-registered', id);
  }
  
  unregisterFactory(id: string): void {
    this.factories.delete(id);
    this.emit('factory-unregistered', id);
  }
  
  getFactory(id: string): AgentFactory | undefined {
    return this.factories.get(id);
  }
  
  getAllFactories(): Map<string, AgentFactory> {
    return new Map(this.factories);
  }
  
  async createAgent(factoryId: string, type: string, config: any): Promise<any> {
    const factory = this.factories.get(factoryId);
    if (!factory) {
      throw new Error(`Agent factory not found: ${factoryId}`);
    }
    return factory.createAgent(type, config);
  }
}