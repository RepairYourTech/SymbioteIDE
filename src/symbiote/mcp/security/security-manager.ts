/**
 * MCP Security Manager
 * 
 * Central authority for MCP security operations
 */

import { EventEmitter } from 'events';
import * as path from 'path';
import {
  SecurityPolicy,
  SecurityRule,
  SandboxConfig,
  PermissionRequest,
  PermissionType,
  PermissionDecision,
  SecurityAuditEntry,
  SecurityMetrics,
  TrustLevel,
  SecurityScanResult
} from './types';
import { SandboxManager } from './sandbox/sandbox-manager';
import { PermissionManager } from './permissions/permission-manager';
import { SecurityScanner } from './scanner/security-scanner';
import { SecurityAuditor } from './audit/security-auditor';
import { TrustManager } from './trust/trust-manager';
import { McpServerDefinition } from '../mcp-types';

export interface SecurityManagerOptions {
  /**
   * Default security policy
   */
  defaultPolicy?: SecurityPolicy;
  
  /**
   * Enable security scanning
   */
  enableScanning?: boolean;
  
  /**
   * Enable audit logging
   */
  enableAuditing?: boolean;
  
  /**
   * Security data directory
   */
  dataDirectory?: string;
  
  /**
   * User interaction handler
   */
  interactionHandler?: ISecurityInteractionHandler;
}

export interface ISecurityInteractionHandler {
  /**
   * Request user permission
   */
  requestPermission(request: PermissionRequest): Promise<PermissionDecision>;
  
  /**
   * Show security alert
   */
  showAlert(alert: SecurityAlert): Promise<void>;
  
  /**
   * Get user confirmation
   */
  confirm(message: string, options?: ConfirmOptions): Promise<boolean>;
}

export interface SecurityAlert {
  severity: 'info' | 'warning' | 'error' | 'critical';
  title: string;
  message: string;
  serverId?: string;
  actions?: string[];
}

export interface ConfirmOptions {
  title?: string;
  detail?: string;
  defaultButton?: 'yes' | 'no';
}

export class SecurityManager extends EventEmitter {
  private sandboxManager: SandboxManager;
  private permissionManager: PermissionManager;
  private securityScanner: SecurityScanner;
  private securityAuditor: SecurityAuditor;
  private trustManager: TrustManager;
  private options: Required<SecurityManagerOptions>;
  private serverConfigs: Map<string, SandboxConfig> = new Map();
  
  constructor(options: SecurityManagerOptions = {}) {
    super();
    
    this.options = {
      defaultPolicy: this.createDefaultPolicy(),
      enableScanning: true,
      enableAuditing: true,
      dataDirectory: path.join(process.cwd(), '.symbiote', 'security'),
      interactionHandler: this.createDefaultInteractionHandler(),
      ...options
    };
    
    // Initialize components
    this.sandboxManager = new SandboxManager();
    this.permissionManager = new PermissionManager({
      dataDirectory: path.join(this.options.dataDirectory, 'permissions'),
      defaultPolicy: this.options.defaultPolicy
    });
    this.securityScanner = new SecurityScanner({
      dataDirectory: path.join(this.options.dataDirectory, 'scanner')
    });
    this.securityAuditor = new SecurityAuditor({
      dataDirectory: path.join(this.options.dataDirectory, 'audit')
    });
    this.trustManager = new TrustManager({
      dataDirectory: path.join(this.options.dataDirectory, 'trust')
    });
    
    this.setupEventHandlers();
  }
  
  /**
   * Initialize security manager
   */
  async initialize(): Promise<void> {
    await Promise.all([
      this.permissionManager.initialize(),
      this.securityScanner.initialize(),
      this.securityAuditor.initialize(),
      this.trustManager.initialize()
    ]);
    
    this.emit('initialized');
  }
  
  /**
   * Create sandboxed environment for server
   */
  async createSandbox(
    serverId: string,
    definition: McpServerDefinition
  ): Promise<SandboxConfig> {
    // Get trust level
    const trustLevel = await this.trustManager.getTrustLevel(serverId);
    
    // Create sandbox config based on trust
    const sandboxConfig = this.createSandboxConfig(serverId, definition, trustLevel);
    
    // Store config
    this.serverConfigs.set(serverId, sandboxConfig);
    
    // Audit
    if (this.options.enableAuditing) {
      await this.securityAuditor.logEntry({
        id: this.generateId(),
        timestamp: new Date(),
        serverId,
        serverName: definition.command,
        type: 'permission',
        severity: 'low',
        operation: 'create_sandbox',
        result: 'allowed',
        details: {
          trustLevel: trustLevel.level,
          isolationLevel: sandboxConfig.isolationLevel
        }
      });
    }
    
    return sandboxConfig;
  }
  
  /**
   * Check permission for operation
   */
  async checkPermission(
    serverId: string,
    type: PermissionType,
    resource: string,
    metadata?: Record<string, any>
  ): Promise<boolean> {
    // Create permission request
    const request: PermissionRequest = {
      id: this.generateId(),
      serverId,
      serverName: this.getServerName(serverId),
      timestamp: new Date(),
      type,
      resource,
      operation: type,
      metadata,
      status: 'pending'
    };
    
    // Check with permission manager
    const decision = await this.permissionManager.checkPermission(request);
    
    // If denied, return false
    if (!decision.allowed) {
      await this.handlePermissionDenied(request, decision);
      return false;
    }
    
    // If requires user interaction
    if (decision.requiresInteraction) {
      const userDecision = await this.options.interactionHandler.requestPermission(request);
      
      // Store decision
      await this.permissionManager.storeDecision(request, userDecision);
      
      if (!userDecision.allowed) {
        await this.handlePermissionDenied(request, userDecision);
        return false;
      }
    }
    
    // Audit allowed permission
    if (this.options.enableAuditing) {
      await this.securityAuditor.logEntry({
        id: this.generateId(),
        timestamp: new Date(),
        serverId,
        serverName: this.getServerName(serverId),
        type: 'permission',
        severity: 'low',
        operation: type,
        resource,
        result: 'allowed',
        details: { metadata }
      });
    }
    
    return true;
  }
  
  /**
   * Scan server for security issues
   */
  async scanServer(
    serverId: string,
    serverPath: string
  ): Promise<SecurityScanResult> {
    if (!this.options.enableScanning) {
      return {
        serverId,
        serverPath,
        timestamp: new Date(),
        passed: true,
        findings: [],
        score: 100,
        recommendations: []
      };
    }
    
    const result = await this.securityScanner.scan(serverId, serverPath);
    
    // Update trust level based on scan
    await this.trustManager.updateFromScan(serverId, result);
    
    // Alert on critical findings
    const criticalFindings = result.findings.filter(f => f.severity === 'critical');
    if (criticalFindings.length > 0) {
      await this.handleCriticalFindings(serverId, criticalFindings);
    }
    
    return result;
  }
  
  /**
   * Report security violation
   */
  async reportViolation(
    serverId: string,
    violation: {
      type: string;
      resource?: string;
      details: Record<string, any>;
      severity: 'low' | 'medium' | 'high' | 'critical';
    }
  ): Promise<void> {
    // Log to audit
    const entry: SecurityAuditEntry = {
      id: this.generateId(),
      timestamp: new Date(),
      serverId,
      serverName: this.getServerName(serverId),
      type: 'violation',
      severity: violation.severity,
      operation: violation.type,
      resource: violation.resource,
      result: 'denied',
      details: violation.details
    };
    
    await this.securityAuditor.logEntry(entry);
    
    // Update trust level
    await this.trustManager.reportViolation(serverId, violation);
    
    // Handle based on severity
    if (violation.severity === 'critical' || violation.severity === 'high') {
      await this.handleSevereViolation(serverId, violation);
    }
    
    this.emit('violation', { serverId, violation });
  }
  
  /**
   * Get security metrics for server
   */
  async getMetrics(serverId: string): Promise<SecurityMetrics> {
    return this.sandboxManager.getMetrics(serverId);
  }
  
  /**
   * Get trust level for server
   */
  async getTrustLevel(serverId: string): Promise<TrustLevel> {
    return this.trustManager.getTrustLevel(serverId);
  }
  
  /**
   * Set trust level for server
   */
  async setTrustLevel(
    serverId: string,
    level: TrustLevel['level'],
    reason: string
  ): Promise<void> {
    await this.trustManager.setTrustLevel(serverId, level, reason);
    
    // Update sandbox config if server is running
    if (this.serverConfigs.has(serverId)) {
      const definition = await this.getServerDefinition(serverId);
      const trustLevel = await this.trustManager.getTrustLevel(serverId);
      const newConfig = this.createSandboxConfig(serverId, definition, trustLevel);
      this.serverConfigs.set(serverId, newConfig);
      
      // Apply new config
      await this.sandboxManager.updateConfig(serverId, newConfig);
    }
  }
  
  /**
   * Get audit logs
   */
  async getAuditLogs(
    filter?: {
      serverId?: string;
      startDate?: Date;
      endDate?: Date;
      type?: SecurityAuditEntry['type'];
      severity?: SecurityAuditEntry['severity'];
    }
  ): Promise<SecurityAuditEntry[]> {
    return this.securityAuditor.queryLogs(filter);
  }
  
  /**
   * Export security report
   */
  async exportSecurityReport(serverId?: string): Promise<SecurityReport> {
    const report: SecurityReport = {
      timestamp: new Date(),
      servers: []
    };
    
    const serverIds = serverId ? [serverId] : Array.from(this.serverConfigs.keys());
    
    for (const id of serverIds) {
      const trustLevel = await this.trustManager.getTrustLevel(id);
      const violations = await this.securityAuditor.queryLogs({
        serverId: id,
        type: 'violation'
      });
      const deniedPermissions = await this.securityAuditor.queryLogs({
        serverId: id,
        type: 'permission',
        result: 'denied' as any
      });
      
      report.servers.push({
        serverId: id,
        trustLevel,
        violationCount: violations.length,
        deniedPermissionCount: deniedPermissions.length,
        lastScan: await this.getLastScanDate(id),
        recommendations: await this.getSecurityRecommendations(id)
      });
    }
    
    return report;
  }
  
  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Permission manager events
    this.permissionManager.on('permission-denied', (event) => {
      this.emit('permission-denied', event);
    });
    
    // Scanner events
    this.securityScanner.on('threat-detected', (event) => {
      this.emit('threat-detected', event);
    });
    
    // Auditor events
    this.securityAuditor.on('anomaly-detected', (event) => {
      this.emit('anomaly-detected', event);
    });
  }
  
  /**
   * Create default security policy
   */
  private createDefaultPolicy(): SecurityPolicy {
    return {
      id: 'default',
      name: 'Default Security Policy',
      description: 'Default security policy for MCP servers',
      rules: [
        // Block access to sensitive files
        {
          id: 'block-env-files',
          type: 'filesystem',
          action: 'deny',
          resource: '**/.env*',
          metadata: { reason: 'Environment files may contain secrets' }
        },
        {
          id: 'block-git',
          type: 'filesystem',
          action: 'deny',
          resource: '**/.git/**',
          metadata: { reason: 'Git directories may contain sensitive data' }
        },
        {
          id: 'block-ssh',
          type: 'filesystem',
          action: 'deny',
          resource: '**/.ssh/**',
          metadata: { reason: 'SSH keys are sensitive' }
        },
        // Network restrictions
        {
          id: 'block-localhost',
          type: 'network',
          action: 'deny',
          resource: 'localhost',
          metadata: { reason: 'Prevent local network access by default' }
        },
        {
          id: 'block-private-ips',
          type: 'network',
          action: 'deny',
          resource: '10.0.0.0/8,172.16.0.0/12,192.168.0.0/16',
          metadata: { reason: 'Prevent private network access by default' }
        }
      ],
      defaultAction: 'prompt',
      priority: 0,
      enabled: true,
      createdAt: new Date(),
      updatedAt: new Date()
    };
  }
  
  /**
   * Create sandbox config based on trust level
   */
  private createSandboxConfig(
    serverId: string,
    definition: McpServerDefinition,
    trustLevel: TrustLevel
  ): SandboxConfig {
    const baseConfig: SandboxConfig = {
      isolationLevel: 'strict',
      resourceLimits: {
        maxCpuPercent: 50,
        maxMemoryMB: 512,
        maxDiskIOMBps: 10,
        maxExecutionTime: 300, // 5 minutes
        maxFileHandles: 100,
        maxThreads: 10
      },
      filesystem: {
        allowedReadPaths: [definition.cwd || process.cwd()],
        allowedWritePaths: [],
        blockedPaths: [
          '**/.env*',
          '**/.git/**',
          '**/.ssh/**',
          '**/node_modules/**/package.json'
        ],
        maxFileSizeMB: 10,
        blockHiddenFiles: true,
        blockSymlinks: true
      },
      network: {
        enabled: false,
        allowedHosts: [],
        blockedHosts: ['localhost', '127.0.0.1', '0.0.0.0'],
        allowedProtocols: ['https'],
        blockLocalhost: true,
        maxRequestSizeMB: 1,
        rateLimit: {
          maxRequests: 100,
          windowSeconds: 60
        }
      },
      process: {
        allowSpawn: false,
        envVarRestrictions: {
          blockAll: false,
          blocked: ['PATH', 'HOME', 'USER', 'SSH_*', 'AWS_*', 'GITHUB_*']
        }
      },
      monitoring: {
        enabled: true,
        logOperations: true,
        alertOnSuspicious: true,
        metricsInterval: 1000,
        behavioralAnalysis: true
      }
    };
    
    // Adjust based on trust level
    switch (trustLevel.level) {
      case 'verified':
        baseConfig.isolationLevel = 'basic';
        baseConfig.resourceLimits.maxCpuPercent = 80;
        baseConfig.resourceLimits.maxMemoryMB = 2048;
        baseConfig.filesystem.allowedWritePaths = [definition.cwd || process.cwd()];
        baseConfig.network.enabled = true;
        baseConfig.network.allowedHosts = ['*'];
        baseConfig.process.allowSpawn = true;
        break;
        
      case 'trusted':
        baseConfig.isolationLevel = 'basic';
        baseConfig.resourceLimits.maxCpuPercent = 70;
        baseConfig.resourceLimits.maxMemoryMB = 1024;
        baseConfig.network.enabled = true;
        break;
        
      case 'standard':
        baseConfig.resourceLimits.maxCpuPercent = 60;
        baseConfig.resourceLimits.maxMemoryMB = 768;
        break;
        
      case 'restricted':
        baseConfig.resourceLimits.maxCpuPercent = 40;
        baseConfig.resourceLimits.maxMemoryMB = 256;
        baseConfig.monitoring.behavioralAnalysis = true;
        break;
        
      case 'untrusted':
        // Keep strict defaults
        break;
    }
    
    return baseConfig;
  }
  
  /**
   * Handle permission denied
   */
  private async handlePermissionDenied(
    request: PermissionRequest,
    decision: PermissionDecision
  ): Promise<void> {
    // Audit
    if (this.options.enableAuditing) {
      await this.securityAuditor.logEntry({
        id: this.generateId(),
        timestamp: new Date(),
        serverId: request.serverId,
        serverName: request.serverName,
        type: 'permission',
        severity: 'medium',
        operation: request.type,
        resource: request.resource,
        result: 'denied',
        details: {
          reason: decision.conditions
        }
      });
    }
    
    this.emit('permission-denied', { request, decision });
  }
  
  /**
   * Handle critical security findings
   */
  private async handleCriticalFindings(
    serverId: string,
    findings: any[]
  ): Promise<void> {
    const alert: SecurityAlert = {
      severity: 'critical',
      title: 'Critical Security Issues Found',
      message: `${findings.length} critical security issues found in server ${serverId}`,
      serverId,
      actions: ['Disable Server', 'View Details', 'Ignore']
    };
    
    await this.options.interactionHandler.showAlert(alert);
    
    // Consider disabling the server
    const shouldDisable = await this.options.interactionHandler.confirm(
      `Disable server ${serverId} due to critical security issues?`,
      {
        title: 'Security Warning',
        detail: findings.map(f => f.title).join('\n'),
        defaultButton: 'yes'
      }
    );
    
    if (shouldDisable) {
      this.emit('server-disabled', { serverId, reason: 'security' });
    }
  }
  
  /**
   * Handle severe violation
   */
  private async handleSevereViolation(
    serverId: string,
    violation: any
  ): Promise<void> {
    const alert: SecurityAlert = {
      severity: violation.severity,
      title: 'Security Violation Detected',
      message: `Server ${serverId} violated security policy: ${violation.type}`,
      serverId,
      actions: ['Terminate', 'Restrict', 'View Details']
    };
    
    await this.options.interactionHandler.showAlert(alert);
    
    // Reduce trust level
    const currentTrust = await this.trustManager.getTrustLevel(serverId);
    if (currentTrust.level !== 'untrusted') {
      await this.setTrustLevel(serverId, 'restricted', `Security violation: ${violation.type}`);
    }
  }
  
  /**
   * Create default interaction handler
   */
  private createDefaultInteractionHandler(): ISecurityInteractionHandler {
    return {
      async requestPermission(request: PermissionRequest): Promise<PermissionDecision> {
        // Default: deny all
        return {
          allowed: false,
          timestamp: new Date(),
          remember: false,
          scope: 'session'
        };
      },
      
      async showAlert(alert: SecurityAlert): Promise<void> {
        console.error('[Security Alert]', alert);
      },
      
      async confirm(message: string, options?: ConfirmOptions): Promise<boolean> {
        console.log('[Security Confirm]', message, options);
        return false;
      }
    };
  }
  
  /**
   * Helper methods
   */
  private generateId(): string {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }
  
  private getServerName(serverId: string): string {
    // TODO: Get actual server name from registry
    return serverId;
  }
  
  private async getServerDefinition(serverId: string): Promise<McpServerDefinition> {
    // TODO: Get from server registry
    return {
      command: 'unknown',
      args: []
    };
  }
  
  private async getLastScanDate(serverId: string): Promise<Date | null> {
    // TODO: Get from scanner
    return null;
  }
  
  private async getSecurityRecommendations(serverId: string): Promise<string[]> {
    const recommendations: string[] = [];
    const trustLevel = await this.trustManager.getTrustLevel(serverId);
    
    if (trustLevel.level === 'untrusted') {
      recommendations.push('Run security scan to verify server integrity');
      recommendations.push('Review server permissions and capabilities');
    }
    
    return recommendations;
  }
}

export interface SecurityReport {
  timestamp: Date;
  servers: Array<{
    serverId: string;
    trustLevel: TrustLevel;
    violationCount: number;
    deniedPermissionCount: number;
    lastScan: Date | null;
    recommendations: string[];
  }>;
}