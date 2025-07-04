/**
 * Security Auditor
 * 
 * Comprehensive security logging and reporting for MCP servers
 */

import { EventEmitter } from 'events';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import { 
  AuditEntry, 
  SecurityIncident,
  SecurityScanResult,
  PermissionRequest,
  PermissionDecision
} from '../types';

export interface AuditOptions {
  dataDirectory: string;
  rotationSizeMB?: number;
  retentionDays?: number;
  enableRealTimeAlerts?: boolean;
  alertThresholds?: AlertThresholds;
}

export interface AlertThresholds {
  failedPermissionsPerMinute: number;
  criticalFindingsPerScan: number;
  resourceLimitViolationsPerHour: number;
  suspiciousActivitiesPerDay: number;
}

export interface AuditSummary {
  serverId: string;
  period: {
    start: Date;
    end: Date;
  };
  totals: {
    events: number;
    permissionRequests: number;
    permissionsDenied: number;
    securityScans: number;
    criticalFindings: number;
    incidents: number;
  };
  topIssues: Array<{
    type: string;
    count: number;
    severity: string;
  }>;
  recommendations: string[];
}

interface LogFile {
  path: string;
  stream: fs.WriteStream;
  size: number;
  created: Date;
}

export class SecurityAuditor extends EventEmitter {
  private options: Required<AuditOptions>;
  private logFiles: Map<string, LogFile> = new Map();
  private incidents: Map<string, SecurityIncident[]> = new Map();
  private recentEvents: Map<string, AuditEntry[]> = new Map();
  private alertCounts: Map<string, Map<string, number>> = new Map();
  
  constructor(options: AuditOptions) {
    super();
    
    this.options = {
      rotationSizeMB: 100,
      retentionDays: 30,
      enableRealTimeAlerts: true,
      alertThresholds: {
        failedPermissionsPerMinute: 10,
        criticalFindingsPerScan: 3,
        resourceLimitViolationsPerHour: 20,
        suspiciousActivitiesPerDay: 50
      },
      ...options
    };
  }
  
  /**
   * Initialize auditor
   */
  async initialize(): Promise<void> {
    // Create data directory structure
    await fs.promises.mkdir(this.options.dataDirectory, { recursive: true });
    await fs.promises.mkdir(path.join(this.options.dataDirectory, 'logs'), { recursive: true });
    await fs.promises.mkdir(path.join(this.options.dataDirectory, 'incidents'), { recursive: true });
    await fs.promises.mkdir(path.join(this.options.dataDirectory, 'reports'), { recursive: true });
    
    // Clean up old logs
    await this.cleanupOldLogs();
    
    // Start monitoring loop
    this.startMonitoring();
  }
  
  /**
   * Log audit entry
   */
  async logEntry(entry: AuditEntry): Promise<void> {
    // Add to recent events
    const serverEvents = this.recentEvents.get(entry.serverId) || [];
    serverEvents.push(entry);
    
    // Keep only last 1000 events in memory
    if (serverEvents.length > 1000) {
      serverEvents.shift();
    }
    
    this.recentEvents.set(entry.serverId, serverEvents);
    
    // Write to log file
    await this.writeToLog(entry);
    
    // Check for alerts
    if (this.options.enableRealTimeAlerts) {
      await this.checkAlerts(entry);
    }
    
    // Check for incidents
    await this.detectIncidents(entry);
    
    // Emit event
    this.emit('audit-logged', entry);
  }
  
  /**
   * Log permission request
   */
  async logPermissionRequest(
    request: PermissionRequest,
    decision: PermissionDecision | null,
    result: { allowed: boolean; reason?: string }
  ): Promise<void> {
    const entry: AuditEntry = {
      id: crypto.randomUUID(),
      timestamp: new Date(),
      serverId: request.serverId,
      serverName: request.serverName,
      category: 'permission',
      action: `${request.type}_request`,
      resource: request.resource,
      result: result.allowed ? 'allowed' : 'denied',
      metadata: {
        requestId: request.id,
        permissionType: request.type,
        decision: decision,
        reason: result.reason
      }
    };
    
    await this.logEntry(entry);
  }
  
  /**
   * Log security scan
   */
  async logSecurityScan(result: SecurityScanResult): Promise<void> {
    const entry: AuditEntry = {
      id: crypto.randomUUID(),
      timestamp: result.timestamp,
      serverId: result.serverId,
      serverName: '', // Would be filled from server info
      category: 'scan',
      action: 'security_scan',
      resource: result.serverPath,
      result: result.passed ? 'passed' : 'failed',
      metadata: {
        score: result.score,
        findings: result.findings.length,
        criticalFindings: result.findings.filter(f => f.severity === 'critical').length,
        highFindings: result.findings.filter(f => f.severity === 'high').length
      }
    };
    
    await this.logEntry(entry);
  }
  
  /**
   * Log security incident
   */
  async logIncident(incident: SecurityIncident): Promise<void> {
    // Store incident
    const serverIncidents = this.incidents.get(incident.serverId) || [];
    serverIncidents.push(incident);
    this.incidents.set(incident.serverId, serverIncidents);
    
    // Write to incident file
    const incidentFile = path.join(
      this.options.dataDirectory,
      'incidents',
      `incident-${incident.id}.json`
    );
    
    await fs.promises.writeFile(
      incidentFile,
      JSON.stringify(incident, null, 2)
    );
    
    // Log as audit entry
    const entry: AuditEntry = {
      id: crypto.randomUUID(),
      timestamp: incident.timestamp,
      serverId: incident.serverId,
      serverName: incident.serverName,
      category: 'incident',
      action: incident.type,
      resource: incident.details.resource || '',
      result: 'incident',
      metadata: {
        incidentId: incident.id,
        severity: incident.severity,
        resolved: incident.resolved
      }
    };
    
    await this.logEntry(entry);
    
    // Emit alert
    this.emit('security-incident', incident);
  }
  
  /**
   * Generate audit summary
   */
  async generateSummary(
    serverId: string,
    startDate: Date,
    endDate: Date
  ): Promise<AuditSummary> {
    const events = await this.queryEvents(serverId, startDate, endDate);
    
    const summary: AuditSummary = {
      serverId,
      period: { start: startDate, end: endDate },
      totals: {
        events: events.length,
        permissionRequests: 0,
        permissionsDenied: 0,
        securityScans: 0,
        criticalFindings: 0,
        incidents: 0
      },
      topIssues: [],
      recommendations: []
    };
    
    // Analyze events
    const issueCounts = new Map<string, { count: number; severity: string }>();
    
    for (const event of events) {
      // Count totals
      if (event.category === 'permission') {
        summary.totals.permissionRequests++;
        if (event.result === 'denied') {
          summary.totals.permissionsDenied++;
        }
      } else if (event.category === 'scan') {
        summary.totals.securityScans++;
        summary.totals.criticalFindings += event.metadata?.criticalFindings || 0;
      } else if (event.category === 'incident') {
        summary.totals.incidents++;
      }
      
      // Track issues
      if (event.result === 'denied' || event.result === 'failed' || event.result === 'incident') {
        const key = `${event.category}:${event.action}`;
        const issue = issueCounts.get(key) || { count: 0, severity: 'medium' };
        issue.count++;
        
        if (event.metadata?.severity) {
          issue.severity = event.metadata.severity;
        }
        
        issueCounts.set(key, issue);
      }
    }
    
    // Get top issues
    summary.topIssues = Array.from(issueCounts.entries())
      .map(([type, data]) => ({ type, ...data }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 10);
    
    // Generate recommendations
    summary.recommendations = this.generateRecommendations(summary);
    
    // Save report
    const reportFile = path.join(
      this.options.dataDirectory,
      'reports',
      `audit-summary-${serverId}-${startDate.toISOString().split('T')[0]}.json`
    );
    
    await fs.promises.writeFile(
      reportFile,
      JSON.stringify(summary, null, 2)
    );
    
    return summary;
  }
  
  /**
   * Query audit events
   */
  async queryEvents(
    serverId: string,
    startDate?: Date,
    endDate?: Date,
    filters?: {
      category?: string;
      action?: string;
      result?: string;
    }
  ): Promise<AuditEntry[]> {
    const events: AuditEntry[] = [];
    
    // Read from log files
    const logDir = path.join(this.options.dataDirectory, 'logs');
    const files = await fs.promises.readdir(logDir);
    
    for (const file of files) {
      if (!file.startsWith(`audit-${serverId}-`)) continue;
      
      const filePath = path.join(logDir, file);
      const content = await fs.promises.readFile(filePath, 'utf8');
      const lines = content.trim().split('\n');
      
      for (const line of lines) {
        if (!line) continue;
        
        try {
          const entry = JSON.parse(line) as AuditEntry;
          
          // Apply filters
          if (startDate && entry.timestamp < startDate) continue;
          if (endDate && entry.timestamp > endDate) continue;
          if (filters?.category && entry.category !== filters.category) continue;
          if (filters?.action && entry.action !== filters.action) continue;
          if (filters?.result && entry.result !== filters.result) continue;
          
          events.push(entry);
        } catch {
          // Skip invalid lines
        }
      }
    }
    
    return events.sort((a, b) => 
      a.timestamp.getTime() - b.timestamp.getTime()
    );
  }
  
  /**
   * Get recent incidents
   */
  getIncidents(serverId?: string): SecurityIncident[] {
    if (serverId) {
      return this.incidents.get(serverId) || [];
    }
    
    const allIncidents: SecurityIncident[] = [];
    for (const incidents of this.incidents.values()) {
      allIncidents.push(...incidents);
    }
    
    return allIncidents.sort((a, b) => 
      b.timestamp.getTime() - a.timestamp.getTime()
    );
  }
  
  /**
   * Resolve incident
   */
  async resolveIncident(
    incidentId: string,
    resolution: string,
    resolvedBy: string
  ): Promise<void> {
    // Find incident
    let incident: SecurityIncident | null = null;
    
    for (const [serverId, incidents] of this.incidents) {
      const found = incidents.find(i => i.id === incidentId);
      if (found) {
        incident = found;
        break;
      }
    }
    
    if (!incident) {
      throw new Error('Incident not found');
    }
    
    // Update incident
    incident.resolved = true;
    incident.resolution = resolution;
    incident.resolvedBy = resolvedBy;
    incident.resolvedAt = new Date();
    
    // Update incident file
    const incidentFile = path.join(
      this.options.dataDirectory,
      'incidents',
      `incident-${incident.id}.json`
    );
    
    await fs.promises.writeFile(
      incidentFile,
      JSON.stringify(incident, null, 2)
    );
    
    // Log resolution
    const entry: AuditEntry = {
      id: crypto.randomUUID(),
      timestamp: new Date(),
      serverId: incident.serverId,
      serverName: incident.serverName,
      category: 'incident',
      action: 'incident_resolved',
      resource: '',
      result: 'resolved',
      metadata: {
        incidentId: incident.id,
        resolution,
        resolvedBy
      }
    };
    
    await this.logEntry(entry);
  }
  
  /**
   * Export audit logs
   */
  async exportLogs(
    serverId: string,
    format: 'json' | 'csv',
    outputPath: string
  ): Promise<void> {
    const events = await this.queryEvents(serverId);
    
    if (format === 'json') {
      await fs.promises.writeFile(
        outputPath,
        JSON.stringify(events, null, 2)
      );
    } else if (format === 'csv') {
      const csv = this.eventsToCSV(events);
      await fs.promises.writeFile(outputPath, csv);
    }
  }
  
  /**
   * Write entry to log file
   */
  private async writeToLog(entry: AuditEntry): Promise<void> {
    const logKey = `${entry.serverId}-${new Date().toISOString().split('T')[0]}`;
    let logFile = this.logFiles.get(logKey);
    
    // Create or rotate log file
    if (!logFile || logFile.size > this.options.rotationSizeMB * 1024 * 1024) {
      if (logFile) {
        logFile.stream.end();
      }
      
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const logPath = path.join(
        this.options.dataDirectory,
        'logs',
        `audit-${logKey}-${timestamp}.jsonl`
      );
      
      const stream = fs.createWriteStream(logPath, { flags: 'a' });
      logFile = {
        path: logPath,
        stream,
        size: 0,
        created: new Date()
      };
      
      this.logFiles.set(logKey, logFile);
    }
    
    // Write entry
    const line = JSON.stringify(entry) + '\n';
    logFile.stream.write(line);
    logFile.size += Buffer.byteLength(line);
  }
  
  /**
   * Check for alerts
   */
  private async checkAlerts(entry: AuditEntry): Promise<void> {
    const now = Date.now();
    const serverId = entry.serverId;
    
    // Initialize alert counts
    if (!this.alertCounts.has(serverId)) {
      this.alertCounts.set(serverId, new Map());
    }
    
    const counts = this.alertCounts.get(serverId)!;
    
    // Track different alert types
    if (entry.category === 'permission' && entry.result === 'denied') {
      this.incrementAlertCount(counts, 'failed-permissions', now, 60000); // 1 minute window
      
      const count = counts.get('failed-permissions') || 0;
      if (count >= this.options.alertThresholds!.failedPermissionsPerMinute) {
        this.emit('security-alert', {
          type: 'excessive-permission-denials',
          serverId,
          count,
          threshold: this.options.alertThresholds!.failedPermissionsPerMinute,
          window: '1 minute'
        });
      }
    }
  }
  
  /**
   * Increment alert count
   */
  private incrementAlertCount(
    counts: Map<string, number>,
    key: string,
    now: number,
    windowMs: number
  ): void {
    const windowKey = `${key}-${Math.floor(now / windowMs)}`;
    counts.set(windowKey, (counts.get(windowKey) || 0) + 1);
    
    // Clean old windows
    for (const [k, _] of counts) {
      const [_, window] = k.split('-');
      if (parseInt(window) < Math.floor(now / windowMs) - 1) {
        counts.delete(k);
      }
    }
    
    // Update current count
    counts.set(key, counts.get(windowKey) || 0);
  }
  
  /**
   * Detect incidents
   */
  private async detectIncidents(entry: AuditEntry): Promise<void> {
    // Pattern-based incident detection
    const patterns = [
      {
        condition: (e: AuditEntry) => 
          e.category === 'scan' && 
          e.metadata?.criticalFindings > 0,
        type: 'critical_vulnerabilities',
        severity: 'critical'
      },
      {
        condition: (e: AuditEntry) => 
          e.category === 'resource' && 
          e.action === 'limit_exceeded' &&
          e.metadata?.duration > 60000,
        type: 'sustained_resource_abuse',
        severity: 'high'
      },
      {
        condition: (e: AuditEntry) => 
          e.category === 'network' && 
          e.action === 'suspicious_connection',
        type: 'suspicious_network_activity',
        severity: 'high'
      }
    ];
    
    for (const pattern of patterns) {
      if (pattern.condition(entry)) {
        const incident: SecurityIncident = {
          id: crypto.randomUUID(),
          timestamp: new Date(),
          serverId: entry.serverId,
          serverName: entry.serverName,
          type: pattern.type,
          severity: pattern.severity as any,
          description: `Detected ${pattern.type} from audit logs`,
          details: {
            triggerEvent: entry,
            resource: entry.resource
          },
          resolved: false
        };
        
        await this.logIncident(incident);
      }
    }
  }
  
  /**
   * Generate recommendations
   */
  private generateRecommendations(summary: AuditSummary): string[] {
    const recommendations: string[] = [];
    
    // High permission denial rate
    if (summary.totals.permissionRequests > 0) {
      const denialRate = summary.totals.permissionsDenied / summary.totals.permissionRequests;
      if (denialRate > 0.3) {
        recommendations.push(
          `High permission denial rate (${(denialRate * 100).toFixed(1)}%). Review and adjust security policies.`
        );
      }
    }
    
    // Critical findings
    if (summary.totals.criticalFindings > 0) {
      recommendations.push(
        `Found ${summary.totals.criticalFindings} critical security findings. Immediate remediation required.`
      );
    }
    
    // Incidents
    if (summary.totals.incidents > 5) {
      recommendations.push(
        `Multiple security incidents (${summary.totals.incidents}) detected. Consider stricter security measures.`
      );
    }
    
    // Top issues
    for (const issue of summary.topIssues.slice(0, 3)) {
      if (issue.count > 10) {
        recommendations.push(
          `Frequent ${issue.type} occurrences (${issue.count}). Investigate root cause.`
        );
      }
    }
    
    return recommendations;
  }
  
  /**
   * Convert events to CSV
   */
  private eventsToCSV(events: AuditEntry[]): string {
    const headers = [
      'Timestamp',
      'Server ID',
      'Category',
      'Action',
      'Resource',
      'Result',
      'Metadata'
    ];
    
    const rows = events.map(e => [
      e.timestamp.toISOString(),
      e.serverId,
      e.category,
      e.action,
      e.resource,
      e.result,
      JSON.stringify(e.metadata || {})
    ]);
    
    return [headers, ...rows]
      .map(row => row.map(cell => `"${cell}"`).join(','))
      .join('\n');
  }
  
  /**
   * Clean up old logs
   */
  private async cleanupOldLogs(): Promise<void> {
    const logDir = path.join(this.options.dataDirectory, 'logs');
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - this.options.retentionDays);
    
    try {
      const files = await fs.promises.readdir(logDir);
      
      for (const file of files) {
        const filePath = path.join(logDir, file);
        const stats = await fs.promises.stat(filePath);
        
        if (stats.mtime < cutoffDate) {
          await fs.promises.unlink(filePath);
        }
      }
    } catch {
      // Directory might not exist yet
    }
  }
  
  /**
   * Start monitoring loop
   */
  private startMonitoring(): void {
    // Periodic cleanup
    setInterval(() => {
      this.cleanupOldLogs();
    }, 86400000); // Daily
    
    // Periodic summaries
    setInterval(() => {
      this.generatePeriodicSummaries();
    }, 3600000); // Hourly
  }
  
  /**
   * Generate periodic summaries
   */
  private async generatePeriodicSummaries(): Promise<void> {
    const now = new Date();
    const oneHourAgo = new Date(now.getTime() - 3600000);
    
    // Generate summaries for active servers
    for (const serverId of this.recentEvents.keys()) {
      try {
        const summary = await this.generateSummary(serverId, oneHourAgo, now);
        
        // Emit summary event
        this.emit('hourly-summary', summary);
      } catch (error) {
        console.error(`Failed to generate summary for ${serverId}:`, error);
      }
    }
  }
}