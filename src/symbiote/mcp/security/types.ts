/**
 * MCP Security Types
 * 
 * Type definitions for MCP security and permission system
 */

export interface SecurityPolicy {
  id: string;
  name: string;
  description: string;
  rules: SecurityRule[];
  defaultAction: 'allow' | 'deny';
  priority: number;
  enabled: boolean;
  createdAt: Date;
  updatedAt: Date;
}

export interface SecurityRule {
  id: string;
  type: 'filesystem' | 'network' | 'process' | 'resource';
  action: 'allow' | 'deny' | 'prompt';
  resource: string;
  conditions?: RuleCondition[];
  metadata?: Record<string, any>;
}

export interface RuleCondition {
  type: 'pattern' | 'size' | 'time' | 'frequency';
  operator: 'equals' | 'contains' | 'startsWith' | 'endsWith' | 'matches' | 'lessThan' | 'greaterThan';
  value: any;
}

export interface SandboxConfig {
  /**
   * Process isolation level
   */
  isolationLevel: 'none' | 'basic' | 'strict';
  
  /**
   * Resource limits
   */
  resourceLimits: ResourceLimits;
  
  /**
   * Filesystem restrictions
   */
  filesystem: FilesystemRestrictions;
  
  /**
   * Network restrictions
   */
  network: NetworkRestrictions;
  
  /**
   * Process restrictions
   */
  process: ProcessRestrictions;
  
  /**
   * Monitoring configuration
   */
  monitoring: MonitoringConfig;
}

export interface ResourceLimits {
  /**
   * Maximum CPU usage percentage (0-100)
   */
  maxCpuPercent: number;
  
  /**
   * Maximum memory in MB
   */
  maxMemoryMB: number;
  
  /**
   * Maximum disk I/O in MB/s
   */
  maxDiskIOMBps: number;
  
  /**
   * Maximum execution time in seconds
   */
  maxExecutionTime: number;
  
  /**
   * Maximum number of file handles
   */
  maxFileHandles: number;
  
  /**
   * Maximum number of threads
   */
  maxThreads: number;
}

export interface FilesystemRestrictions {
  /**
   * Allowed read paths (glob patterns)
   */
  allowedReadPaths: string[];
  
  /**
   * Allowed write paths (glob patterns)
   */
  allowedWritePaths: string[];
  
  /**
   * Blocked paths (takes precedence over allowed)
   */
  blockedPaths: string[];
  
  /**
   * Maximum file size for operations
   */
  maxFileSizeMB: number;
  
  /**
   * Allowed file extensions
   */
  allowedExtensions?: string[];
  
  /**
   * Block access to hidden files
   */
  blockHiddenFiles: boolean;
  
  /**
   * Block access to symbolic links
   */
  blockSymlinks: boolean;
}

export interface NetworkRestrictions {
  /**
   * Network access enabled
   */
  enabled: boolean;
  
  /**
   * Allowed domains/IPs
   */
  allowedHosts: string[];
  
  /**
   * Blocked domains/IPs
   */
  blockedHosts: string[];
  
  /**
   * Allowed ports
   */
  allowedPorts?: number[];
  
  /**
   * Allowed protocols
   */
  allowedProtocols: ('http' | 'https' | 'ws' | 'wss')[];
  
  /**
   * Block localhost access
   */
  blockLocalhost: boolean;
  
  /**
   * Maximum request size in MB
   */
  maxRequestSizeMB: number;
  
  /**
   * Request rate limiting
   */
  rateLimit?: RateLimit;
}

export interface ProcessRestrictions {
  /**
   * Allow spawning child processes
   */
  allowSpawn: boolean;
  
  /**
   * Allowed executables (if spawn enabled)
   */
  allowedExecutables?: string[];
  
  /**
   * Environment variable restrictions
   */
  envVarRestrictions: {
    /**
     * Block all environment variables
     */
    blockAll: boolean;
    
    /**
     * Allowed environment variables
     */
    allowed?: string[];
    
    /**
     * Blocked environment variables
     */
    blocked?: string[];
  };
}

export interface MonitoringConfig {
  /**
   * Enable activity monitoring
   */
  enabled: boolean;
  
  /**
   * Log all operations
   */
  logOperations: boolean;
  
  /**
   * Alert on suspicious activity
   */
  alertOnSuspicious: boolean;
  
  /**
   * Metrics collection interval (ms)
   */
  metricsInterval: number;
  
  /**
   * Behavioral analysis
   */
  behavioralAnalysis: boolean;
}

export interface RateLimit {
  /**
   * Maximum requests per window
   */
  maxRequests: number;
  
  /**
   * Time window in seconds
   */
  windowSeconds: number;
  
  /**
   * Burst allowance
   */
  burstAllowance?: number;
}

export interface PermissionRequest {
  id: string;
  serverId: string;
  serverName: string;
  timestamp: Date;
  type: PermissionType;
  resource: string;
  operation: string;
  reason?: string;
  metadata?: Record<string, any>;
  status: 'pending' | 'approved' | 'denied' | 'timeout';
  decision?: PermissionDecision;
}

export enum PermissionType {
  FileRead = 'file_read',
  FileWrite = 'file_write',
  FileDelete = 'file_delete',
  NetworkAccess = 'network_access',
  ProcessSpawn = 'process_spawn',
  SystemInfo = 'system_info',
  ResourceAccess = 'resource_access',
  ToolExecution = 'tool_execution'
}

export interface PermissionDecision {
  allowed: boolean;
  timestamp: Date;
  userId?: string;
  remember: boolean;
  scope: 'session' | 'permanent';
  conditions?: string[];
}

export interface SecurityAuditEntry {
  id: string;
  timestamp: Date;
  serverId: string;
  serverName: string;
  type: 'permission' | 'violation' | 'anomaly' | 'error';
  severity: 'low' | 'medium' | 'high' | 'critical';
  operation: string;
  resource?: string;
  result: 'allowed' | 'denied' | 'error';
  details: Record<string, any>;
  stackTrace?: string;
  remediation?: string;
}

export interface SecurityMetrics {
  serverId: string;
  timestamp: Date;
  cpu: {
    usage: number;
    limit: number;
  };
  memory: {
    used: number;
    limit: number;
  };
  disk: {
    readMBps: number;
    writeMBps: number;
    limit: number;
  };
  network: {
    requestsPerSecond: number;
    bytesPerSecond: number;
    activeConnections: number;
  };
  violations: number;
  deniedOperations: number;
}

export interface ThreatPattern {
  id: string;
  name: string;
  description: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  patterns: string[];
  indicators: string[];
  mitigation: string;
  references?: string[];
}

export interface SecurityScanResult {
  serverId: string;
  serverPath: string;
  timestamp: Date;
  passed: boolean;
  findings: SecurityFinding[];
  score: number;
  recommendations: string[];
}

export interface SecurityFinding {
  type: 'vulnerability' | 'malicious_code' | 'suspicious_pattern' | 'dependency_issue';
  severity: 'low' | 'medium' | 'high' | 'critical';
  title: string;
  description: string;
  location?: {
    file: string;
    line?: number;
    column?: number;
  };
  evidence?: string;
  recommendation: string;
  cve?: string;
  references?: string[];
}

export interface TrustLevel {
  serverId: string;
  level: 'untrusted' | 'restricted' | 'standard' | 'trusted' | 'verified';
  score: number;
  factors: TrustFactor[];
  lastUpdated: Date;
}

export interface TrustFactor {
  type: 'signature' | 'behavior' | 'reputation' | 'manual' | 'scan';
  score: number;
  description: string;
  timestamp: Date;
}