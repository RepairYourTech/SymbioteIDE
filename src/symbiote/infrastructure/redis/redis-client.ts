/**
 * Redis Client - Redis connection and operations
 */

import Redis from 'ioredis';

export interface RedisConfig {
  host: string;
  port: number;
  password?: string;
  db?: number;
  keyPrefix?: string;
}

export class RedisClient {
  private static instance: RedisClient;
  private client: Redis | null = null;
  private config: RedisConfig | null = null;

  private constructor() {}

  static getInstance(): RedisClient {
    if (!RedisClient.instance) {
      RedisClient.instance = new RedisClient();
    }
    return RedisClient.instance;
  }

  async connect(config: RedisConfig): Promise<void> {
    if (this.client) {
      await this.disconnect();
    }

    this.config = config;
    this.client = new Redis({
      host: config.host,
      port: config.port,
      password: config.password,
      db: config.db,
      keyPrefix: config.keyPrefix
    });

    // Wait for connection
    await new Promise<void>((resolve, reject) => {
      this.client!.once('connect', () => resolve());
      this.client!.once('error', (err) => reject(err));
    });
  }

  async disconnect(): Promise<void> {
    if (this.client) {
      await this.client.quit();
      this.client = null;
    }
  }

  getClient(): Redis {
    if (!this.client) {
      throw new Error('Redis client not initialized');
    }
    return this.client;
  }

  async get(key: string): Promise<string | null> {
    const client = this.getClient();
    return client.get(key);
  }

  async set(key: string, value: string, ttl?: number): Promise<void> {
    const client = this.getClient();
    if (ttl) {
      await client.set(key, value, 'EX', ttl);
    } else {
      await client.set(key, value);
    }
  }

  async del(key: string): Promise<void> {
    const client = this.getClient();
    await client.del(key);
  }

  async exists(key: string): Promise<boolean> {
    const client = this.getClient();
    const result = await client.exists(key);
    return result === 1;
  }

  async hget(key: string, field: string): Promise<string | null> {
    const client = this.getClient();
    return client.hget(key, field);
  }

  async hset(key: string, field: string, value: string): Promise<void> {
    const client = this.getClient();
    await client.hset(key, field, value);
  }

  async hgetall(key: string): Promise<Record<string, string>> {
    const client = this.getClient();
    return client.hgetall(key);
  }

  async lpush(key: string, ...values: string[]): Promise<number> {
    const client = this.getClient();
    return client.lpush(key, ...values);
  }

  async rpop(key: string): Promise<string | null> {
    const client = this.getClient();
    return client.rpop(key);
  }

  async publish(channel: string, message: string): Promise<number> {
    const client = this.getClient();
    return client.publish(channel, message);
  }

  isConnected(): boolean {
    return this.client !== null && this.client.status === 'ready';
  }
}

export function createRedisClient(config: RedisConfig): RedisClient {
  const client = RedisClient.getInstance();
  client.connect(config);
  return client;
}

export default RedisClient;