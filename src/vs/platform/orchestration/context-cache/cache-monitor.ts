/**
 * Cache Monitor
 * 
 * Monitoring and alerting for the context cache system
 */

import { EventEmitter } from 'events';
import { ContextCacheManager } from './context-cache-manager';
import { MonitoringConfig, CacheStats } from './types';

interface CacheMetrics {
  timestamp: Date;
  provider: string;
  hitRate: number;
  evictionRate: number;
  errorRate: number;
  avgLatency: number;
  storageUsed: number;
  costSaved: number;
}

interface Alert {
  level: 'warning' | 'error' | 'info';
  message: string;
  metric: string;
  value: number;
  threshold: number;
  timestamp: Date;
}

export class CacheMonitor extends EventEmitter {
  private cacheManager: ContextCacheManager;
  private config?: MonitoringConfig;
  private metricsHistory: CacheMetrics[] = [];
  private intervalId?: NodeJS.Timeout;
  private startTime: Date;
  private eventCounts: Map<string, number>;
  
  constructor(cacheManager: ContextCacheManager, config?: MonitoringConfig) {
    super();
    this.cacheManager = cacheManager;
    this.config = config;
    this.startTime = new Date();
    this.eventCounts = new Map();
    
    if (config?.enabled) {
      this.start();
    }
  }
  
  /**
   * Start monitoring
   */
  start(): void {
    if (this.intervalId) {
      return; // Already running
    }
    
    // Set up event listeners
    this.setupEventListeners();
    
    // Start metrics collection interval
    const interval = this.config?.metricsInterval || 60; // Default 60 seconds
    this.intervalId = setInterval(() => {
      this.collectMetrics();
    }, interval * 1000);
    
    // Collect initial metrics
    this.collectMetrics();
  }
  
  /**
   * Stop monitoring
   */
  stop(): void {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = undefined;
    }
    
    // Remove event listeners
    this.removeAllListeners();
  }
  
  /**
   * Get current metrics
   */
  getCurrentMetrics(): Map<string, CacheMetrics> {
    const currentMetrics = new Map<string, CacheMetrics>();
    const stats = this.cacheManager.getStats() as Map<string, CacheStats>;
    
    stats.forEach((stat, provider) => {
      currentMetrics.set(provider, {
        timestamp: new Date(),
        provider,
        hitRate: stat.hitRate,
        evictionRate: this.calculateEvictionRate(provider),
        errorRate: this.calculateErrorRate(provider),
        avgLatency: stat.avgLatency,
        storageUsed: stat.storageUsed,
        costSaved: stat.costSaved
      });
    });
    
    return currentMetrics;
  }
  
  /**
   * Get metrics history
   */
  getMetricsHistory(
    provider?: string,
    startTime?: Date,
    endTime?: Date
  ): CacheMetrics[] {
    let history = this.metricsHistory;
    
    if (provider) {
      history = history.filter(m => m.provider === provider);
    }
    
    if (startTime) {
      history = history.filter(m => m.timestamp >= startTime);
    }
    
    if (endTime) {
      history = history.filter(m => m.timestamp <= endTime);
    }
    
    return history;
  }
  
  /**
   * Get aggregated metrics
   */
  getAggregatedMetrics(
    provider?: string,
    period: 'hour' | 'day' | 'week' = 'hour'
  ): {
    avgHitRate: number;
    totalCostSaved: number;
    totalRequests: number;
    peakStorageUsed: number;
  } {
    const now = new Date();
    const startTime = new Date(now);
    
    switch (period) {
      case 'hour':
        startTime.setHours(startTime.getHours() - 1);
        break;
      case 'day':
        startTime.setDate(startTime.getDate() - 1);
        break;
      case 'week':
        startTime.setDate(startTime.getDate() - 7);
        break;
    }
    
    const history = this.getMetricsHistory(provider, startTime, now);
    
    if (history.length === 0) {
      return {
        avgHitRate: 0,
        totalCostSaved: 0,
        totalRequests: 0,
        peakStorageUsed: 0
      };
    }
    
    const avgHitRate = history.reduce((sum, m) => sum + m.hitRate, 0) / history.length;
    const totalCostSaved = history[history.length - 1].costSaved - history[0].costSaved;
    const peakStorageUsed = Math.max(...history.map(m => m.storageUsed));
    
    // Calculate total requests from stats
    const stats = this.cacheManager.getStats(provider) as CacheStats;
    const totalRequests = stats?.totalRequests || 0;
    
    return {
      avgHitRate,
      totalCostSaved,
      totalRequests,
      peakStorageUsed
    };
  }
  
  /**
   * Generate cache report
   */
  generateReport(): string {
    const stats = this.cacheManager.getStats() as Map<string, CacheStats>;
    const uptime = (new Date().getTime() - this.startTime.getTime()) / 1000 / 60; // minutes
    
    let report = `Context Cache Report\n`;
    report += `====================\n\n`;
    report += `Uptime: ${uptime.toFixed(2)} minutes\n\n`;
    
    stats.forEach((stat, provider) => {
      report += `Provider: ${provider}\n`;
      report += `---------\n`;
      report += `Hit Rate: ${(stat.hitRate * 100).toFixed(2)}%\n`;
      report += `Total Requests: ${stat.totalRequests}\n`;
      report += `Hits: ${stat.hits}\n`;
      report += `Misses: ${stat.misses}\n`;
      report += `Cost Saved: $${stat.costSaved.toFixed(4)}\n`;
      report += `Storage Used: ${(stat.storageUsed / 1024 / 1024).toFixed(2)} MB\n`;
      report += `Avg Latency: ${stat.avgLatency.toFixed(2)} ms\n\n`;
    });
    
    // Add alerts summary
    const recentAlerts = this.getRecentAlerts();
    if (recentAlerts.length > 0) {
      report += `Recent Alerts\n`;
      report += `-------------\n`;
      recentAlerts.forEach(alert => {
        report += `[${alert.level.toUpperCase()}] ${alert.message}\n`;
      });
    }
    
    return report;
  }
  
  // Private methods
  
  private setupEventListeners(): void {
    // Track cache events
    this.cacheManager.on('hit', (key: string, provider: string) => {
      this.incrementEventCount(`${provider}:hit`);
    });
    
    this.cacheManager.on('miss', (key: string, provider: string) => {
      this.incrementEventCount(`${provider}:miss`);
    });
    
    this.cacheManager.on('evict', (key: string, reason: string) => {
      this.incrementEventCount('evictions');
    });
    
    this.cacheManager.on('error', (error: Error, operation: string) => {
      this.incrementEventCount('errors');
      this.emit('alert', {
        level: 'error',
        message: `Cache error in ${operation}: ${error.message}`,
        metric: 'error',
        value: 1,
        threshold: 0,
        timestamp: new Date()
      } as Alert);
    });
  }
  
  private collectMetrics(): void {
    const currentMetrics = this.getCurrentMetrics();
    
    currentMetrics.forEach((metrics, provider) => {
      // Store in history
      this.metricsHistory.push(metrics);
      
      // Check thresholds and emit alerts
      this.checkThresholds(metrics);
    });
    
    // Limit history size (keep last 24 hours at 1-minute intervals)
    const maxHistorySize = 24 * 60;
    if (this.metricsHistory.length > maxHistorySize) {
      this.metricsHistory = this.metricsHistory.slice(-maxHistorySize);
    }
  }
  
  private checkThresholds(metrics: CacheMetrics): void {
    if (!this.config?.alertThresholds) {
      return;
    }
    
    const thresholds = this.config.alertThresholds;
    
    // Check hit rate
    if (thresholds.hitRate !== undefined && metrics.hitRate < thresholds.hitRate) {
      this.emit('alert', {
        level: 'warning',
        message: `Low cache hit rate for ${metrics.provider}: ${(metrics.hitRate * 100).toFixed(2)}%`,
        metric: 'hitRate',
        value: metrics.hitRate,
        threshold: thresholds.hitRate,
        timestamp: new Date()
      } as Alert);
    }
    
    // Check eviction rate
    if (thresholds.evictionRate !== undefined && metrics.evictionRate > thresholds.evictionRate) {
      this.emit('alert', {
        level: 'warning',
        message: `High eviction rate for ${metrics.provider}: ${(metrics.evictionRate * 100).toFixed(2)}%`,
        metric: 'evictionRate',
        value: metrics.evictionRate,
        threshold: thresholds.evictionRate,
        timestamp: new Date()
      } as Alert);
    }
    
    // Check error rate
    if (thresholds.errorRate !== undefined && metrics.errorRate > thresholds.errorRate) {
      this.emit('alert', {
        level: 'error',
        message: `High error rate for ${metrics.provider}: ${(metrics.errorRate * 100).toFixed(2)}%`,
        metric: 'errorRate',
        value: metrics.errorRate,
        threshold: thresholds.errorRate,
        timestamp: new Date()
      } as Alert);
    }
  }
  
  private calculateEvictionRate(provider: string): number {
    const evictions = this.eventCounts.get('evictions') || 0;
    const sets = this.eventCounts.get(`${provider}:set`) || 0;
    
    if (sets === 0) return 0;
    return evictions / sets;
  }
  
  private calculateErrorRate(provider: string): number {
    const errors = this.eventCounts.get('errors') || 0;
    const stats = this.cacheManager.getStats(provider) as CacheStats;
    
    if (!stats || stats.totalRequests === 0) return 0;
    return errors / stats.totalRequests;
  }
  
  private incrementEventCount(event: string): void {
    const count = this.eventCounts.get(event) || 0;
    this.eventCounts.set(event, count + 1);
  }
  
  private getRecentAlerts(minutes: number = 5): Alert[] {
    const cutoff = new Date();
    cutoff.setMinutes(cutoff.getMinutes() - minutes);
    
    const alerts: Alert[] = [];
    
    this.listeners('alert').forEach(listener => {
      // This is a simplified version - in production you'd store alerts
    });
    
    return alerts;
  }
}