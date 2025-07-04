/**
 * Knowledge API - Knowledge graph API interface
 */

export interface KnowledgeAPI {
  addNode(node: KnowledgeNode): Promise<string>;
  getNode(id: string): Promise<KnowledgeNode | null>;
  updateNode(id: string, updates: Partial<KnowledgeNode>): Promise<void>;
  deleteNode(id: string): Promise<void>;
  
  addRelation(relation: KnowledgeRelation): Promise<string>;
  getRelations(nodeId: string, type?: string): Promise<KnowledgeRelation[]>;
  deleteRelation(id: string): Promise<void>;
  
  query(query: KnowledgeQuery): Promise<KnowledgeQueryResult>;
  search(text: string, options?: KnowledgeSearchOptions): Promise<KnowledgeNode[]>;
}

export interface KnowledgeNode {
  id?: string;
  type: string;
  name: string;
  properties: Record<string, any>;
  metadata?: {
    createdAt?: Date;
    updatedAt?: Date;
    source?: string;
    confidence?: number;
  };
}

export interface KnowledgeRelation {
  id?: string;
  type: string;
  sourceId: string;
  targetId: string;
  properties?: Record<string, any>;
  metadata?: {
    createdAt?: Date;
    confidence?: number;
  };
}

export interface KnowledgeQuery {
  pattern: string;
  parameters?: Record<string, any>;
  limit?: number;
}

export interface KnowledgeQueryResult {
  nodes: KnowledgeNode[];
  relations: KnowledgeRelation[];
  metadata?: {
    executionTime?: number;
    totalResults?: number;
  };
}

export interface KnowledgeSearchOptions {
  types?: string[];
  limit?: number;
  fuzzy?: boolean;
}

export class KnowledgeAPIImpl implements KnowledgeAPI {
  private nodes: Map<string, KnowledgeNode> = new Map();
  private relations: Map<string, KnowledgeRelation> = new Map();
  private nodeRelations: Map<string, Set<string>> = new Map();

  async addNode(node: KnowledgeNode): Promise<string> {
    const id = node.id || `node_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const now = new Date();
    
    const newNode: KnowledgeNode = {
      ...node,
      id,
      metadata: {
        ...node.metadata,
        createdAt: now,
        updatedAt: now
      }
    };

    this.nodes.set(id, newNode);
    return id;
  }

  async getNode(id: string): Promise<KnowledgeNode | null> {
    return this.nodes.get(id) || null;
  }

  async updateNode(id: string, updates: Partial<KnowledgeNode>): Promise<void> {
    const node = this.nodes.get(id);
    if (!node) {
      throw new Error(`Node ${id} not found`);
    }

    const updated: KnowledgeNode = {
      ...node,
      ...updates,
      id, // Preserve ID
      metadata: {
        ...node.metadata,
        ...updates.metadata,
        updatedAt: new Date()
      }
    };

    this.nodes.set(id, updated);
  }

  async deleteNode(id: string): Promise<void> {
    this.nodes.delete(id);
    
    // Delete associated relations
    const relations = this.nodeRelations.get(id);
    if (relations) {
      for (const relationId of relations) {
        this.relations.delete(relationId);
      }
      this.nodeRelations.delete(id);
    }
  }

  async addRelation(relation: KnowledgeRelation): Promise<string> {
    const id = relation.id || `rel_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    const newRelation: KnowledgeRelation = {
      ...relation,
      id,
      metadata: {
        ...relation.metadata,
        createdAt: new Date()
      }
    };

    this.relations.set(id, newRelation);
    
    // Track relations for nodes
    if (!this.nodeRelations.has(relation.sourceId)) {
      this.nodeRelations.set(relation.sourceId, new Set());
    }
    if (!this.nodeRelations.has(relation.targetId)) {
      this.nodeRelations.set(relation.targetId, new Set());
    }
    
    this.nodeRelations.get(relation.sourceId)!.add(id);
    this.nodeRelations.get(relation.targetId)!.add(id);
    
    return id;
  }

  async getRelations(nodeId: string, type?: string): Promise<KnowledgeRelation[]> {
    const relationIds = this.nodeRelations.get(nodeId);
    if (!relationIds) {
      return [];
    }

    const relations: KnowledgeRelation[] = [];
    
    for (const id of relationIds) {
      const relation = this.relations.get(id);
      if (relation && (!type || relation.type === type)) {
        relations.push(relation);
      }
    }

    return relations;
  }

  async deleteRelation(id: string): Promise<void> {
    const relation = this.relations.get(id);
    if (relation) {
      // Remove from node tracking
      const sourceRelations = this.nodeRelations.get(relation.sourceId);
      if (sourceRelations) {
        sourceRelations.delete(id);
      }
      
      const targetRelations = this.nodeRelations.get(relation.targetId);
      if (targetRelations) {
        targetRelations.delete(id);
      }
    }
    
    this.relations.delete(id);
  }

  async query(query: KnowledgeQuery): Promise<KnowledgeQueryResult> {
    // Simple pattern matching implementation
    const nodes: KnowledgeNode[] = [];
    const relations: KnowledgeRelation[] = [];
    
    // For now, just return all nodes and relations
    for (const node of this.nodes.values()) {
      nodes.push(node);
      if (nodes.length >= (query.limit || 100)) break;
    }

    return {
      nodes,
      relations,
      metadata: {
        totalResults: nodes.length + relations.length
      }
    };
  }

  async search(text: string, options?: KnowledgeSearchOptions): Promise<KnowledgeNode[]> {
    const results: KnowledgeNode[] = [];
    const searchText = text.toLowerCase();
    
    for (const node of this.nodes.values()) {
      // Check type filter
      if (options?.types && !options.types.includes(node.type)) {
        continue;
      }

      // Simple text search
      const nodeText = JSON.stringify(node).toLowerCase();
      if (nodeText.includes(searchText)) {
        results.push(node);
        
        if (results.length >= (options?.limit || 10)) {
          break;
        }
      }
    }

    return results;
  }
}

export default KnowledgeAPIImpl;