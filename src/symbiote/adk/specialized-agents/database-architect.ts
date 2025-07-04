/**
 * Database Architect Agent
 * 
 * Designs and optimizes database schemas and queries
 */

import { ADKAgentConfig, ADKAgentType } from '../types';
import { ProviderType, Capability } from '../../orchestration/interfaces';

export class DatabaseArchitectAgent {
  static getConfig(): ADKAgentConfig {
    return {
      id: 'database-architect',
      name: 'Database Architect',
      type: ADKAgentType.LLM,
      description: 'Designs and optimizes database schemas and queries',
      systemPrompt: `You are a database architecture expert with deep knowledge of:

1. Database Design
   - Normalization (1NF, 2NF, 3NF, BCNF)
   - Denormalization strategies
   - Entity-relationship modeling
   - Data modeling patterns
   - Schema versioning
   
2. Database Types
   - Relational (PostgreSQL, MySQL, SQL Server)
   - NoSQL (MongoDB, Cassandra, DynamoDB)
   - Graph (Neo4j, Amazon Neptune)
   - Time-series (InfluxDB, TimescaleDB)
   - Vector (Pinecone, Qdrant)
   
3. Performance Optimization
   - Query optimization
   - Index strategies
   - Partitioning and sharding
   - Connection pooling
   - Caching strategies
   
4. Data Integrity
   - Constraints and validations
   - Transactions and ACID
   - Consistency patterns
   - Backup and recovery
   - Data migration strategies
   
5. Scalability
   - Horizontal vs vertical scaling
   - Read replicas
   - Master-slave replication
   - Distributed databases
   - CAP theorem considerations

When designing databases:
- Consider data access patterns
- Plan for growth and scale
- Ensure data integrity
- Optimize for performance
- Document design decisions`,
      model: {
        id: 'gemini-1.5-pro',
        name: 'Gemini 1.5 Pro',
        displayName: 'Gemini 1.5 Pro',
        provider: ProviderType.Google,
        contextWindow: 1000000,
        maxOutputTokens: 1000000,
        costPerToken: {
          input: 0.00001,
          output: 0.00003,
          currency: 'USD'
        },
        capabilities: [Capability.LongContext, Capability.CodeAnalysis],
        supportsBatch: true,
        supportsStreaming: true,
        supportsTools: true,
        supportsVision: true,
        lastUpdated: new Date('2024-02-08')
      },
      temperature: 0.4,
      capabilities: {
        streaming: true,
        multiModal: false,
        toolUse: true,
        memoryAccess: true,
        a2aProtocol: true,
        mcpTools: true
      },
      tools: [
        {
          name: 'design_schema',
          description: 'Design database schema based on requirements',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { requirements: string; dbType: string }) => {
              return {
                schema: 'CREATE TABLE ...',
                tables: ['users', 'orders', 'products'],
                relationships: ['users->orders', 'orders->products']
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                requirements: { type: 'string' },
                dbType: { type: 'string', enum: ['postgresql', 'mysql', 'mongodb', 'neo4j'] }
              },
              required: ['requirements', 'dbType']
            }
          }
        },
        {
          name: 'optimize_query',
          description: 'Optimize database queries for performance',
          type: 'custom',
          implementation: {
            source: 'code',
            handler: async (params: { query: string; schema: string }) => {
              return {
                originalQuery: params.query,
                optimizedQuery: 'SELECT ...',
                improvements: ['Added index', 'Removed subquery'],
                estimatedSpeedup: '10x'
              };
            }
          },
          parameters: {
            schema: {
              type: 'object',
              properties: {
                query: { type: 'string' },
                schema: { type: 'string' }
              },
              required: ['query']
            }
          }
        }
      ]
    };
  }
}