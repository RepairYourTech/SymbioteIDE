/**
 * Network Proxy
 * 
 * Intercepts and controls network access for sandboxed processes
 */

import { EventEmitter } from 'events';
import * as net from 'net';
import * as http from 'http';
import * as https from 'https';
import * as url from 'url';
import { NetworkRestrictions, RateLimit } from '../types';

export interface ProxyConfig {
  serverId: string;
  port: number;
  env: Record<string, string>;
}

export interface NetworkMetrics {
  requestsPerSecond: number;
  bytesPerSecond: number;
  activeConnections: number;
  blockedRequests: number;
  totalRequests: number;
}

interface RequestRecord {
  timestamp: Date;
  bytes: number;
}

export class NetworkProxy extends EventEmitter {
  private proxies: Map<string, ProxyServer> = new Map();
  private nextPort: number = 30000;
  
  /**
   * Create proxy for server
   */
  async createProxy(
    serverId: string,
    restrictions: NetworkRestrictions
  ): Promise<ProxyConfig> {
    // Find available port
    const port = await this.findAvailablePort();
    
    // Create proxy server
    const proxy = new ProxyServer(serverId, port, restrictions);
    await proxy.start();
    
    this.proxies.set(serverId, proxy);
    
    // Setup event forwarding
    proxy.on('request-blocked', (event) => {
      this.emit('request-blocked', { serverId, ...event });
    });
    
    proxy.on('suspicious-activity', (event) => {
      this.emit('suspicious-activity', { serverId, ...event });
    });
    
    // Return proxy configuration
    return {
      serverId,
      port,
      env: {
        HTTP_PROXY: `http://localhost:${port}`,
        HTTPS_PROXY: `http://localhost:${port}`,
        http_proxy: `http://localhost:${port}`,
        https_proxy: `http://localhost:${port}`,
        NO_PROXY: 'localhost,127.0.0.1',
        no_proxy: 'localhost,127.0.0.1'
      }
    };
  }
  
  /**
   * Update network restrictions
   */
  async updateRestrictions(
    serverId: string,
    restrictions: NetworkRestrictions
  ): Promise<void> {
    const proxy = this.proxies.get(serverId);
    if (proxy) {
      proxy.updateRestrictions(restrictions);
    }
  }
  
  /**
   * Remove proxy
   */
  async removeProxy(serverId: string): Promise<void> {
    const proxy = this.proxies.get(serverId);
    if (proxy) {
      await proxy.stop();
      this.proxies.delete(serverId);
    }
  }
  
  /**
   * Check if access is allowed
   */
  checkAccess(serverId: string, targetUrl: string): boolean {
    const proxy = this.proxies.get(serverId);
    if (!proxy) {
      return false;
    }
    
    return proxy.checkAccess(targetUrl);
  }
  
  /**
   * Get network metrics
   */
  async getMetrics(serverId: string): Promise<NetworkMetrics> {
    const proxy = this.proxies.get(serverId);
    if (!proxy) {
      return {
        requestsPerSecond: 0,
        bytesPerSecond: 0,
        activeConnections: 0,
        blockedRequests: 0,
        totalRequests: 0
      };
    }
    
    return proxy.getMetrics();
  }
  
  /**
   * Find available port
   */
  private async findAvailablePort(): Promise<number> {
    const checkPort = (port: number): Promise<boolean> => {
      return new Promise((resolve) => {
        const server = net.createServer();
        
        server.once('error', () => resolve(false));
        server.once('listening', () => {
          server.close();
          resolve(true);
        });
        
        server.listen(port);
      });
    };
    
    while (this.nextPort < 40000) {
      const port = this.nextPort++;
      if (await checkPort(port)) {
        return port;
      }
    }
    
    throw new Error('No available ports');
  }
}

/**
 * Individual proxy server
 */
class ProxyServer extends EventEmitter {
  private server: http.Server | null = null;
  private connections: Set<net.Socket> = new Set();
  private requestHistory: RequestRecord[] = [];
  private blockedCount: number = 0;
  private totalCount: number = 0;
  private rateLimitBuckets: Map<string, number[]> = new Map();
  
  constructor(
    private serverId: string,
    private port: number,
    private restrictions: NetworkRestrictions
  ) {
    super();
  }
  
  /**
   * Start proxy server
   */
  async start(): Promise<void> {
    this.server = http.createServer((req, res) => {
      this.handleRequest(req, res);
    });
    
    // Handle CONNECT for HTTPS
    this.server.on('connect', (req, clientSocket, head) => {
      this.handleConnect(req, clientSocket, head);
    });
    
    // Track connections
    this.server.on('connection', (socket) => {
      this.connections.add(socket);
      socket.on('close', () => {
        this.connections.delete(socket);
      });
    });
    
    return new Promise((resolve, reject) => {
      this.server!.once('error', reject);
      this.server!.listen(this.port, () => {
        resolve();
      });
    });
  }
  
  /**
   * Stop proxy server
   */
  async stop(): Promise<void> {
    // Close all connections
    for (const socket of this.connections) {
      socket.destroy();
    }
    
    // Close server
    if (this.server) {
      return new Promise((resolve) => {
        this.server!.close(() => resolve());
      });
    }
  }
  
  /**
   * Update restrictions
   */
  updateRestrictions(restrictions: NetworkRestrictions): void {
    this.restrictions = restrictions;
  }
  
  /**
   * Check if URL access is allowed
   */
  checkAccess(targetUrl: string): boolean {
    if (!this.restrictions.enabled) {
      return false;
    }
    
    const parsed = url.parse(targetUrl);
    const hostname = parsed.hostname || '';
    const port = parseInt(parsed.port || '80', 10);
    const protocol = parsed.protocol || 'http:';
    
    // Check protocol
    if (!this.restrictions.allowedProtocols.includes(protocol.replace(':', '') as any)) {
      return false;
    }
    
    // Check localhost
    if (this.restrictions.blockLocalhost) {
      if (['localhost', '127.0.0.1', '::1'].includes(hostname)) {
        return false;
      }
    }
    
    // Check blocked hosts
    for (const blocked of this.restrictions.blockedHosts) {
      if (this.matchesHost(hostname, blocked)) {
        return false;
      }
    }
    
    // Check allowed hosts
    if (this.restrictions.allowedHosts.length > 0) {
      let allowed = false;
      for (const allowedHost of this.restrictions.allowedHosts) {
        if (allowedHost === '*' || this.matchesHost(hostname, allowedHost)) {
          allowed = true;
          break;
        }
      }
      if (!allowed) {
        return false;
      }
    }
    
    // Check ports
    if (this.restrictions.allowedPorts && this.restrictions.allowedPorts.length > 0) {
      if (!this.restrictions.allowedPorts.includes(port)) {
        return false;
      }
    }
    
    // Check rate limit
    if (this.restrictions.rateLimit) {
      if (!this.checkRateLimit(hostname, this.restrictions.rateLimit)) {
        return false;
      }
    }
    
    return true;
  }
  
  /**
   * Get metrics
   */
  getMetrics(): NetworkMetrics {
    // Clean old records
    const now = Date.now();
    const oneSecondAgo = now - 1000;
    
    this.requestHistory = this.requestHistory.filter(r =>
      r.timestamp.getTime() > oneSecondAgo
    );
    
    // Calculate metrics
    const requestsPerSecond = this.requestHistory.length;
    const bytesPerSecond = this.requestHistory.reduce((sum, r) => sum + r.bytes, 0);
    
    return {
      requestsPerSecond,
      bytesPerSecond,
      activeConnections: this.connections.size,
      blockedRequests: this.blockedCount,
      totalRequests: this.totalCount
    };
  }
  
  /**
   * Handle HTTP request
   */
  private handleRequest(req: http.IncomingMessage, res: http.ServerResponse): void {
    this.totalCount++;
    
    const targetUrl = req.url || '';
    
    // Check if allowed
    if (!this.checkAccess(targetUrl)) {
      this.blockedCount++;
      this.emit('request-blocked', {
        url: targetUrl,
        method: req.method,
        headers: req.headers
      });
      
      res.writeHead(403, { 'Content-Type': 'text/plain' });
      res.end('Forbidden: Network access denied by security policy');
      return;
    }
    
    // Record request
    this.recordRequest(0); // Size will be updated later
    
    // Parse target URL
    const parsed = url.parse(targetUrl);
    const options = {
      hostname: parsed.hostname,
      port: parsed.port || 80,
      path: parsed.path,
      method: req.method,
      headers: this.filterHeaders(req.headers)
    };
    
    // Make request
    const proxyReq = http.request(options, (proxyRes) => {
      res.writeHead(proxyRes.statusCode || 200, proxyRes.headers);
      
      let bytesTransferred = 0;
      proxyRes.on('data', (chunk) => {
        bytesTransferred += chunk.length;
        res.write(chunk);
      });
      
      proxyRes.on('end', () => {
        res.end();
        this.recordRequest(bytesTransferred);
      });
    });
    
    proxyReq.on('error', (err) => {
      res.writeHead(502, { 'Content-Type': 'text/plain' });
      res.end('Bad Gateway');
      
      this.emit('proxy-error', { error: err, url: targetUrl });
    });
    
    // Check request size limit
    let requestSize = 0;
    req.on('data', (chunk) => {
      requestSize += chunk.length;
      
      if (requestSize > this.restrictions.maxRequestSizeMB * 1024 * 1024) {
        req.destroy();
        proxyReq.destroy();
        res.writeHead(413, { 'Content-Type': 'text/plain' });
        res.end('Request Entity Too Large');
        return;
      }
      
      proxyReq.write(chunk);
    });
    
    req.on('end', () => {
      proxyReq.end();
    });
  }
  
  /**
   * Handle CONNECT (HTTPS)
   */
  private handleConnect(
    req: http.IncomingMessage,
    clientSocket: net.Socket,
    head: Buffer
  ): void {
    this.totalCount++;
    
    const targetUrl = `https://${req.url}`;
    
    // Check if allowed
    if (!this.checkAccess(targetUrl)) {
      this.blockedCount++;
      this.emit('request-blocked', {
        url: targetUrl,
        method: 'CONNECT',
        headers: req.headers
      });
      
      clientSocket.write('HTTP/1.1 403 Forbidden\r\n\r\n');
      clientSocket.destroy();
      return;
    }
    
    // Parse host and port
    const [hostname, portStr] = (req.url || '').split(':');
    const port = parseInt(portStr || '443', 10);
    
    // Connect to target
    const serverSocket = net.connect(port, hostname, () => {
      clientSocket.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      serverSocket.write(head);
      
      // Pipe data
      serverSocket.pipe(clientSocket);
      clientSocket.pipe(serverSocket);
      
      // Track bytes
      let bytesTransferred = 0;
      serverSocket.on('data', (chunk) => {
        bytesTransferred += chunk.length;
      });
      
      serverSocket.on('close', () => {
        this.recordRequest(bytesTransferred);
      });
    });
    
    serverSocket.on('error', () => {
      clientSocket.write('HTTP/1.1 502 Bad Gateway\r\n\r\n');
      clientSocket.destroy();
    });
    
    clientSocket.on('error', () => {
      serverSocket.destroy();
    });
  }
  
  /**
   * Filter headers
   */
  private filterHeaders(headers: http.IncomingHttpHeaders): http.OutgoingHttpHeaders {
    const filtered = { ...headers };
    
    // Remove proxy headers
    delete filtered['proxy-authorization'];
    delete filtered['proxy-connection'];
    
    // Add identification
    filtered['x-mcp-sandbox'] = this.serverId;
    
    return filtered;
  }
  
  /**
   * Match hostname pattern
   */
  private matchesHost(hostname: string, pattern: string): boolean {
    // Handle wildcards
    if (pattern.includes('*')) {
      const regex = pattern
        .replace(/\./g, '\\.')
        .replace(/\*/g, '.*');
      return new RegExp(`^${regex}$`).test(hostname);
    }
    
    // Handle IP ranges (simplified)
    if (pattern.includes('/')) {
      // Basic CIDR check
      return this.matchesCIDR(hostname, pattern);
    }
    
    return hostname === pattern;
  }
  
  /**
   * Basic CIDR matching
   */
  private matchesCIDR(ip: string, cidr: string): boolean {
    // Very basic implementation
    // In production, use a proper IP library
    const [network, bits] = cidr.split('/');
    
    if (!net.isIP(ip) || !net.isIP(network)) {
      return false;
    }
    
    // Convert to binary for comparison
    const ipBinary = this.ipToBinary(ip);
    const networkBinary = this.ipToBinary(network);
    const maskBits = parseInt(bits, 10);
    
    return ipBinary.substring(0, maskBits) === networkBinary.substring(0, maskBits);
  }
  
  /**
   * Convert IP to binary string
   */
  private ipToBinary(ip: string): string {
    return ip.split('.')
      .map(octet => parseInt(octet, 10).toString(2).padStart(8, '0'))
      .join('');
  }
  
  /**
   * Check rate limit
   */
  private checkRateLimit(hostname: string, limit: RateLimit): boolean {
    const now = Date.now();
    const windowStart = now - (limit.windowSeconds * 1000);
    
    // Get or create bucket
    let bucket = this.rateLimitBuckets.get(hostname) || [];
    
    // Remove old entries
    bucket = bucket.filter(time => time > windowStart);
    
    // Check if limit exceeded
    if (bucket.length >= limit.maxRequests) {
      // Check burst allowance
      if (limit.burstAllowance && bucket.length < limit.maxRequests + limit.burstAllowance) {
        // Allow burst
      } else {
        return false;
      }
    }
    
    // Add current request
    bucket.push(now);
    this.rateLimitBuckets.set(hostname, bucket);
    
    return true;
  }
  
  /**
   * Record request for metrics
   */
  private recordRequest(bytes: number): void {
    this.requestHistory.push({
      timestamp: new Date(),
      bytes
    });
    
    // Keep only recent history (last 10 seconds)
    const cutoff = Date.now() - 10000;
    this.requestHistory = this.requestHistory.filter(r =>
      r.timestamp.getTime() > cutoff
    );
  }
}