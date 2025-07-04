/**
 * Graph Sync Processor - Handles Neo4j graph synchronization jobs
 */

import { Job } from 'bullmq';
import { BaseProcessor } from './base-processor';
import { 
  GraphSyncJobData, 
  JobResult, 
  JobType 
} from '../types';
import { Neo4jConnectionManager } from '../../neo4j/connection-manager';
import { RedisManager } from '../../redis/redis-manager';

export class GraphSyncProcessor extends BaseProcessor<GraphSyncJobData> {
  private neo4jManager?: Neo4jConnectionManager;
  private redisManager?: RedisManager;

  constructor(queueManager: any) {
    super(queueManager, 'GraphSyncProcessor');
  }

  /**
   * Initialize services
   */
  private async initializeServices(): Promise<void> {
    if (!this.neo4jManager) {
      this.neo4jManager = Neo4jConnectionManager.getInstance();
      await this.neo4jManager.connect();
      
      this.redisManager = RedisManager.getInstance();
    }
  }

  /**
   * Process graph sync job
   */
  async process(job: Job<GraphSyncJobData>): Promise<JobResult> {
    try {
      this.validateJobData(job.data);
      await this.initializeServices();

      switch (job.data.type) {
        case JobType.SyncGraphNode:
          return await this.syncGraphNode(job);
          
        case JobType.UpdateRelationships:
          return await this.updateRelationships(job);
          
        case JobType.AnalyzeDependencies:
          return await this.analyzeDependencies(job);
          
        default:
          throw new Error(`Unknown job type: ${job.data.type}`);
      }
    } catch (error: any) {
      return this.handleFailure(job, error);
    }
  }

  /**
   * Sync a graph node
   */
  private async syncGraphNode(job: Job<GraphSyncJobData>): Promise<JobResult> {
    const { nodeId, nodeType } = job.data;
    
    if (!nodeId || !nodeType) {
      return this.createErrorResult(
        'MISSING_PARAMS',
        'Node ID and type are required'
      );
    }
    
    await this.updateProgress(job, 0, 'Fetching node data');
    
    const session = this.neo4jManager!.getSession();
    
    try {
      // Get node data
      const result = await session.run(
        `
        MATCH (n:${nodeType} {id: $nodeId})
        OPTIONAL MATCH (n)-[r]-(connected)
        RETURN n, collect(DISTINCT {
          relationship: type(r),
          direction: CASE 
            WHEN startNode(r) = n THEN 'outgoing'
            ELSE 'incoming'
          END,
          connectedNode: connected,
          properties: properties(r)
        }) as relationships
        `,
        { nodeId }
      );
      
      if (result.records.length === 0) {
        return this.createErrorResult(
          'NODE_NOT_FOUND',
          `Node ${nodeId} of type ${nodeType} not found`
        );
      }
      
      await this.updateProgress(job, 30, 'Processing relationships');
      
      const node = result.records[0].get('n');
      const relationships = result.records[0].get('relationships');
      
      // Analyze relationships
      const stats = {
        incoming: relationships.filter((r: any) => r.direction === 'incoming').length,
        outgoing: relationships.filter((r: any) => r.direction === 'outgoing').length,
        types: new Set(relationships.map((r: any) => r.relationship)).size
      };
      
      await this.updateProgress(job, 60, 'Updating cache');
      
      // Cache node data
      const cacheKey = `graph:node:${nodeType}:${nodeId}`;
      await this.redisManager!.set(cacheKey, {
        id: nodeId,
        type: nodeType,
        properties: node.properties,
        relationships: relationships.map((r: any) => ({
          type: r.relationship,
          direction: r.direction,
          targetId: r.connectedNode.properties.id,
          targetType: r.connectedNode.labels[0]
        })),
        stats,
        lastSynced: new Date().toISOString()
      }, 3600); // 1 hour cache
      
      await this.updateProgress(job, 90, 'Updating metrics');
      
      // Update sync metrics
      await this.updateSyncMetrics(nodeType, 'synced');
      
      await this.updateProgress(job, 100, 'Complete');
      
      return this.createSuccessResult({
        nodeId,
        nodeType,
        relationshipCount: relationships.length,
        stats
      });
      
    } finally {
      await session.close();
    }
  }

  /**
   * Update relationships
   */
  private async updateRelationships(job: Job<GraphSyncJobData>): Promise<JobResult> {
    const { nodeId, nodeType, relationships } = job.data;
    
    if (!nodeId || !relationships || relationships.length === 0) {
      return this.createErrorResult(
        'MISSING_PARAMS',
        'Node ID and relationships are required'
      );
    }
    
    await this.updateProgress(job, 0, 'Starting relationship updates');
    
    const session = this.neo4jManager!.getSession();
    const tx = session.beginTransaction();
    
    try {
      // First, ensure the source node exists
      const nodeCheck = await tx.run(
        `MATCH (n {id: $nodeId}) RETURN n`,
        { nodeId }
      );
      
      if (nodeCheck.records.length === 0) {
        throw new Error(`Source node ${nodeId} not found`);
      }
      
      let created = 0;
      let updated = 0;
      let errors = 0;
      
      // Process each relationship
      for (let i = 0; i < relationships.length; i++) {
        const rel = relationships[i];
        
        try {
          // Check if relationship exists
          const existingRel = await tx.run(
            `
            MATCH (source {id: $sourceId})-[r:${rel.type}]->(target {id: $targetId})
            RETURN r
            `,
            { sourceId: nodeId, targetId: rel.targetId }
          );
          
          if (existingRel.records.length > 0) {
            // Update existing relationship
            await tx.run(
              `
              MATCH (source {id: $sourceId})-[r:${rel.type}]->(target {id: $targetId})
              SET r.lastUpdated = datetime(),
                  r.strength = coalesce(r.strength, 0) + 1
              `,
              { sourceId: nodeId, targetId: rel.targetId }
            );
            updated++;
          } else {
            // Create new relationship
            await tx.run(
              `
              MATCH (source {id: $sourceId}), (target {id: $targetId})
              CREATE (source)-[r:${rel.type} {
                createdAt: datetime(),
                lastUpdated: datetime(),
                strength: 1
              }]->(target)
              `,
              { sourceId: nodeId, targetId: rel.targetId }
            );
            created++;
          }
        } catch (error) {
          this.logger.error(`Failed to process relationship`, {
            source: nodeId,
            target: rel.targetId,
            type: rel.type,
            error
          });
          errors++;
        }
        
        const progress = (i + 1) / relationships.length * 80;
        await this.updateProgress(
          job,
          progress,
          `Processed ${i + 1}/${relationships.length} relationships`
        );
      }
      
      await tx.commit();
      
      await this.updateProgress(job, 90, 'Invalidating cache');
      
      // Invalidate cache
      await this.invalidateNodeCache(nodeId, nodeType);
      
      await this.updateProgress(job, 100, 'Complete');
      
      return this.createSuccessResult({
        nodeId,
        created,
        updated,
        errors,
        total: relationships.length
      });
      
    } catch (error) {
      await tx.rollback();
      throw error;
    } finally {
      await session.close();
    }
  }

  /**
   * Analyze dependencies
   */
  private async analyzeDependencies(job: Job<GraphSyncJobData>): Promise<JobResult> {
    const { nodeId, nodeType = 'File' } = job.data;
    
    if (!nodeId) {
      return this.createErrorResult(
        'MISSING_NODE_ID',
        'Node ID is required for dependency analysis'
      );
    }
    
    await this.updateProgress(job, 0, 'Analyzing dependencies');
    
    const session = this.neo4jManager!.getSession();
    
    try {
      // Get direct dependencies
      await this.updateProgress(job, 10, 'Finding direct dependencies');
      
      const directDeps = await session.run(
        `
        MATCH (n:${nodeType} {id: $nodeId})-[:IMPORTS|DEPENDS_ON|REFERENCES]->(dep)
        RETURN DISTINCT dep.id as id, dep.path as path, labels(dep)[0] as type
        `,
        { nodeId }
      );
      
      // Get transitive dependencies (up to 3 levels)
      await this.updateProgress(job, 30, 'Finding transitive dependencies');
      
      const transitiveDeps = await session.run(
        `
        MATCH (n:${nodeType} {id: $nodeId})-[:IMPORTS|DEPENDS_ON|REFERENCES*1..3]->(dep)
        WHERE dep.id <> $nodeId
        RETURN DISTINCT dep.id as id, dep.path as path, labels(dep)[0] as type,
               length(shortestPath((n)-[:IMPORTS|DEPENDS_ON|REFERENCES*]->(dep))) as distance
        ORDER BY distance
        `,
        { nodeId }
      );
      
      // Find circular dependencies
      await this.updateProgress(job, 50, 'Checking for circular dependencies');
      
      const circularDeps = await session.run(
        `
        MATCH (n:${nodeType} {id: $nodeId})-[:IMPORTS|DEPENDS_ON|REFERENCES*]->(dep)-[:IMPORTS|DEPENDS_ON|REFERENCES*]->(n)
        RETURN DISTINCT dep.id as id, dep.path as path
        `,
        { nodeId }
      );
      
      // Analyze impact (what depends on this node)
      await this.updateProgress(job, 70, 'Analyzing impact');
      
      const impactAnalysis = await session.run(
        `
        MATCH (dependent)-[:IMPORTS|DEPENDS_ON|REFERENCES*1..2]->(n:${nodeType} {id: $nodeId})
        RETURN DISTINCT dependent.id as id, dependent.path as path, labels(dependent)[0] as type,
               length(shortestPath((dependent)-[:IMPORTS|DEPENDS_ON|REFERENCES*]->(n))) as distance
        ORDER BY distance
        `,
        { nodeId }
      );
      
      await this.updateProgress(job, 90, 'Storing analysis results');
      
      // Compile results
      const analysis = {
        nodeId,
        nodeType,
        directDependencies: directDeps.records.map(r => ({
          id: r.get('id'),
          path: r.get('path'),
          type: r.get('type')
        })),
        transitiveDependencies: transitiveDeps.records.map(r => ({
          id: r.get('id'),
          path: r.get('path'),
          type: r.get('type'),
          distance: r.get('distance').toNumber()
        })),
        circularDependencies: circularDeps.records.map(r => ({
          id: r.get('id'),
          path: r.get('path')
        })),
        impactedNodes: impactAnalysis.records.map(r => ({
          id: r.get('id'),
          path: r.get('path'),
          type: r.get('type'),
          distance: r.get('distance').toNumber()
        })),
        metrics: {
          directDependencyCount: directDeps.records.length,
          transitiveDependencyCount: transitiveDeps.records.length,
          hasCircularDependencies: circularDeps.records.length > 0,
          impactRadius: impactAnalysis.records.length
        },
        analyzedAt: new Date().toISOString()
      };
      
      // Cache analysis
      const cacheKey = `graph:analysis:${nodeType}:${nodeId}`;
      await this.redisManager!.set(cacheKey, analysis, 3600 * 24); // 24 hour cache
      
      // Store in Neo4j for querying
      await session.run(
        `
        MATCH (n:${nodeType} {id: $nodeId})
        SET n.lastAnalyzed = datetime(),
            n.directDependencyCount = $directCount,
            n.hasCircularDependencies = $hasCircular,
            n.impactRadius = $impactRadius
        `,
        {
          nodeId,
          directCount: analysis.metrics.directDependencyCount,
          hasCircular: analysis.metrics.hasCircularDependencies,
          impactRadius: analysis.metrics.impactRadius
        }
      );
      
      await this.updateProgress(job, 100, 'Complete');
      
      return this.createSuccessResult(analysis);
      
    } finally {
      await session.close();
    }
  }

  /**
   * Invalidate node cache
   */
  private async invalidateNodeCache(nodeId: string, nodeType?: string): Promise<void> {
    const patterns = [
      `graph:node:*:${nodeId}`,
      `graph:analysis:*:${nodeId}`
    ];
    
    if (nodeType) {
      patterns.push(`graph:node:${nodeType}:${nodeId}`);
      patterns.push(`graph:analysis:${nodeType}:${nodeId}`);
    }
    
    for (const pattern of patterns) {
      const keys = await this.redisManager!.keys(pattern);
      if (keys.length > 0) {
        await this.redisManager!.del(...keys);
      }
    }
  }

  /**
   * Update sync metrics
   */
  private async updateSyncMetrics(nodeType: string, action: string): Promise<void> {
    const key = `graph:metrics:${nodeType}`;
    const field = `${action}:${new Date().toISOString().split('T')[0]}`;
    
    await this.redisManager!.hincrby(key, field, 1);
  }

  /**
   * Validate job data
   */
  protected validateJobData(data: GraphSyncJobData): void {
    if (!data.type) {
      throw new Error('Job type is required');
    }
    
    if (data.type === JobType.UpdateRelationships && !data.relationships) {
      throw new Error('Relationships are required for update job');
    }
  }

  /**
   * Custom retry logic
   */
  protected shouldRetry(error: Error, job: Job<GraphSyncJobData>): boolean {
    // Retry on connection errors
    if (error.message.includes('connection') || error.message.includes('ECONNREFUSED')) {
      return true;
    }
    
    // Don't retry on data errors
    if (error.message.includes('not found') || error.message.includes('MISSING')) {
      return false;
    }
    
    return super.shouldRetry(error, job);
  }
}