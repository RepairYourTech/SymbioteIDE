/**
 * MCP Security Layer
 * 
 * Comprehensive security system for Model Context Protocol servers
 */

// Core types
export * from './types';

// Security Manager
export { SecurityManager } from './security-manager';
export type { SecurityManagerOptions } from './security-manager';

// Sandbox components
export { SandboxManager } from './sandbox/sandbox-manager';
export { ProcessSandbox } from './sandbox/process-sandbox';
export { NetworkProxy } from './sandbox/network-proxy';
export { ResourceMonitor } from './sandbox/resource-monitor';
export { FilesystemGuard } from './sandbox/filesystem-guard';

// Permission system
export { PermissionManager } from './permissions/permission-manager';
export type { 
  PermissionManagerOptions,
  PermissionCheckResult 
} from './permissions/permission-manager';

// Security scanning
export { SecurityScanner } from './scanner/security-scanner';
export type { SecurityScannerOptions } from './scanner/security-scanner';

// Trust management
export { TrustManager } from './trust/trust-manager';
export type { 
  TrustManagerOptions,
  ServerTrustProfile,
  TrustEvent,
  TrustRestrictions,
  TrustMetrics 
} from './trust/trust-manager';

// Audit system
export { SecurityAuditor } from './audit/security-auditor';
export type { 
  AuditOptions,
  AuditSummary,
  AlertThresholds 
} from './audit/security-auditor';

// Integration
export { SecureMCPServerManager } from './integration/secure-server-manager';
export type { SecureServerOptions } from './integration/secure-server-manager';

// Extension
export { MCPSecurityExtension } from './extension/security-extension';

// UI components
export { SecurityPanelProvider } from './ui/security-panel-provider';

/**
 * Create and initialize MCP Security Extension
 */
export async function createMCPSecurityExtension(
  context: any,
  serverManager: any
): Promise<MCPSecurityExtension> {
  const { MCPSecurityExtension } = await import('./extension/security-extension');
  const extension = new MCPSecurityExtension(context, serverManager);
  await extension.activate();
  return extension;
}

/**
 * Security presets for common scenarios
 */
export const SecurityPresets = {
  /**
   * High security preset - for untrusted servers
   */
  highSecurity: {
    sandboxConfig: {
      isolationLevel: 'strict' as const,
      resourceLimits: {
        maxMemoryMB: 256,
        maxCpuPercent: 20,
        maxFileHandles: 100,
        maxThreads: 10,
        maxExecutionTime: 300,
        maxDiskIOMBps: 10
      },
      filesystem: {
        enabled: true,
        allowedReadPaths: [],
        allowedWritePaths: [],
        blockedPaths: ['*'],
        maxFileSizeMB: 10,
        tempDirOnly: true
      },
      network: {
        enabled: false,
        allowedProtocols: [],
        allowedHosts: [],
        blockedHosts: ['*'],
        blockLocalhost: true,
        allowedPorts: [],
        maxRequestSizeMB: 1,
        rateLimit: {
          maxRequests: 10,
          windowSeconds: 60
        }
      },
      process: {
        allowSpawn: false,
        allowedExecutables: [],
        envVarRestrictions: {
          blockAll: true,
          allowedVars: [],
          blockedVars: []
        }
      },
      monitoring: {
        enabled: true,
        logLevel: 'verbose' as const,
        auditActions: true,
        alertOnViolation: true,
        behavioralAnalysis: true
      }
    },
    defaultTrustLevel: 'untrusted' as const
  },

  /**
   * Medium security preset - for verified servers
   */
  mediumSecurity: {
    sandboxConfig: {
      isolationLevel: 'basic' as const,
      resourceLimits: {
        maxMemoryMB: 1024,
        maxCpuPercent: 50,
        maxFileHandles: 500,
        maxThreads: 50,
        maxExecutionTime: 1800,
        maxDiskIOMBps: 50
      },
      filesystem: {
        enabled: true,
        allowedReadPaths: ['./'],
        allowedWritePaths: ['./tmp', './output'],
        blockedPaths: ['/etc/*', '/System/*', '~/.ssh/*'],
        maxFileSizeMB: 100,
        tempDirOnly: false
      },
      network: {
        enabled: true,
        allowedProtocols: ['http', 'https'],
        allowedHosts: [],
        blockedHosts: ['localhost', '127.0.0.1', '::1'],
        blockLocalhost: true,
        allowedPorts: [80, 443],
        maxRequestSizeMB: 10,
        rateLimit: {
          maxRequests: 100,
          windowSeconds: 60
        }
      },
      process: {
        allowSpawn: false,
        allowedExecutables: [],
        envVarRestrictions: {
          blockAll: false,
          allowedVars: ['PATH', 'HOME', 'USER'],
          blockedVars: ['AWS_*', 'GOOGLE_*', 'AZURE_*']
        }
      },
      monitoring: {
        enabled: true,
        logLevel: 'info' as const,
        auditActions: true,
        alertOnViolation: true,
        behavioralAnalysis: false
      }
    },
    defaultTrustLevel: 'verified' as const
  },

  /**
   * Low security preset - for trusted servers
   */
  lowSecurity: {
    sandboxConfig: {
      isolationLevel: 'none' as const,
      resourceLimits: {
        maxMemoryMB: 4096,
        maxCpuPercent: 80,
        maxFileHandles: 1000,
        maxThreads: 100,
        maxExecutionTime: 3600,
        maxDiskIOMBps: 100
      },
      filesystem: {
        enabled: true,
        allowedReadPaths: ['*'],
        allowedWritePaths: ['*'],
        blockedPaths: ['/etc/shadow', '/etc/sudoers'],
        maxFileSizeMB: 1000,
        tempDirOnly: false
      },
      network: {
        enabled: true,
        allowedProtocols: ['http', 'https', 'ws', 'wss'],
        allowedHosts: ['*'],
        blockedHosts: [],
        blockLocalhost: false,
        allowedPorts: [],
        maxRequestSizeMB: 100,
        rateLimit: {
          maxRequests: 1000,
          windowSeconds: 60
        }
      },
      process: {
        allowSpawn: true,
        allowedExecutables: ['node', 'python', 'ruby'],
        envVarRestrictions: {
          blockAll: false,
          allowedVars: [],
          blockedVars: []
        }
      },
      monitoring: {
        enabled: true,
        logLevel: 'warning' as const,
        auditActions: false,
        alertOnViolation: false,
        behavioralAnalysis: false
      }
    },
    defaultTrustLevel: 'trusted' as const
  }
};

/**
 * Security utilities
 */
export const SecurityUtils = {
  /**
   * Generate secure server ID
   */
  generateServerId(): string {
    const crypto = require('crypto');
    return `mcp-${crypto.randomBytes(8).toString('hex')}`;
  },

  /**
   * Validate server configuration
   */
  validateServerConfig(config: any): { valid: boolean; errors: string[] } {
    const errors: string[] = [];
    
    if (!config.serverId) {
      errors.push('Server ID is required');
    }
    
    if (!config.command && !config.url) {
      errors.push('Either command or URL is required');
    }
    
    if (config.command && !Array.isArray(config.args)) {
      errors.push('Args must be an array when using command');
    }
    
    return {
      valid: errors.length === 0,
      errors
    };
  },

  /**
   * Sanitize server name
   */
  sanitizeServerName(name: string): string {
    return name
      .replace(/[^a-zA-Z0-9-_. ]/g, '')
      .trim()
      .substring(0, 50);
  }
};