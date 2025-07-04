/**
 * MCP Security Integration Tests
 * 
 * End-to-end tests for MCP security system
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { MCPServerManager } from '../../mcp-server-manager';
import { SecurityManager } from '../security-manager';
import { TrustLevel, SecurityPolicy, PermissionType } from '../types';

describe('MCP Security Integration', () => {
  let context: vscode.ExtensionContext;
  let serverManager: MCPServerManager;
  let securityManager: SecurityManager;
  
  beforeEach(async () => {
    // Mock VS Code context
    context = {
      globalStorageUri: {
        fsPath: path.join(__dirname, 'test-storage')
      },
      subscriptions: []
    } as any;
    
    // Create server manager with security
    serverManager = new MCPServerManager(context, {
      security: {
        enableScanning: true,
        enableAuditing: true,
        defaultPolicy: SecurityPolicy.Strict
      }
    });
    
    await serverManager.initialize();
    securityManager = (serverManager as any).securityManager;
  });
  
  afterEach(async () => {
    await serverManager.dispose();
  });
  
  describe('Server Trust Management', () => {
    it('should scan server before registration', async () => {
      const mockServer = {
        command: 'node',
        args: ['malicious-server.js'],
        env: {
          ADMIN_PASSWORD: 'secret123'  // Should be detected
        }
      };
      
      const scanResult = await securityManager.scanServer(mockServer);
      
      expect(scanResult.severity).toBe('high');
      expect(scanResult.issues).toContainEqual(
        expect.objectContaining({
          type: 'sensitive_data',
          description: expect.stringContaining('password')
        })
      );
    });
    
    it('should enforce trust levels for operations', async () => {
      const serverId = 'test-server';
      
      // Set server as untrusted
      await securityManager.setServerTrust(serverId, TrustLevel.Untrusted);
      
      // Try to execute tool - should be denied
      const canExecute = await securityManager.checkPermission({
        serverName: serverId,
        permission: PermissionType.ExecuteTool,
        resource: 'dangerousTool',
        reason: 'Test execution'
      });
      
      expect(canExecute).toBe(false);
    });
  });
  
  describe('Permission System', () => {
    it('should prompt user for permissions', async () => {
      let promptShown = false;
      
      // Override user interaction handler
      (securityManager as any).options.userInteractionHandler = async (request: any) => {
        promptShown = true;
        expect(request.serverName).toBe('test-server');
        expect(request.permission).toBe(PermissionType.FileSystemWrite);
        return { allow: true, remember: false };
      };
      
      const result = await securityManager.checkPermission({
        serverName: 'test-server',
        permission: PermissionType.FileSystemWrite,
        resource: '/etc/passwd',
        reason: 'Write system file'
      });
      
      expect(promptShown).toBe(true);
      expect(result).toBe(true);
    });
    
    it('should remember permanent permissions', async () => {
      const serverId = 'test-server';
      
      // Grant permanent permission
      await securityManager.grantPermanentPermission(
        serverId,
        PermissionType.NetworkAccess
      );
      
      // Check permission - should not prompt
      let promptShown = false;
      (securityManager as any).options.userInteractionHandler = async () => {
        promptShown = true;
        return { allow: false, remember: false };
      };
      
      const result = await securityManager.checkPermission({
        serverName: serverId,
        permission: PermissionType.NetworkAccess,
        resource: 'https://api.example.com',
        reason: 'API access'
      });
      
      expect(promptShown).toBe(false);
      expect(result).toBe(true);
    });
  });
  
  describe('Sandbox Enforcement', () => {
    it('should start server in sandbox', async () => {
      const serverId = 'sandboxed-server';
      
      await serverManager.registerServer(serverId, {
        command: 'node',
        args: ['server.js']
      });
      
      // Mock process spawn
      let sandboxApplied = false;
      (securityManager as any).startSandboxedServer = async (id: string, config: any) => {
        sandboxApplied = true;
        expect(config.command).toBe('node');
        expect(config).toHaveProperty('filesystemRestrictions');
        expect(config).toHaveProperty('networkRestrictions');
        
        return {
          process: {
            pid: 1234,
            stdin: { write: jest.fn() },
            stdout: { on: jest.fn() },
            stderr: { on: jest.fn() },
            on: jest.fn(),
            kill: jest.fn()
          }
        };
      };
      
      await serverManager.startServer(serverId);
      expect(sandboxApplied).toBe(true);
    });
    
    it('should monitor resource usage', async () => {
      const serverId = 'resource-heavy-server';
      let violationReported = false;
      
      securityManager.on('violationReported', (event) => {
        if (event.type === 'high_cpu_usage') {
          violationReported = true;
        }
      });
      
      // Simulate high CPU usage
      (securityManager as any).getServerMetrics = async () => ({
        cpuUsage: 95,
        memoryUsage: 100 * 1024 * 1024,
        networkConnections: 5
      });
      
      // Trigger monitoring
      await (serverManager as any).performHealthCheck(serverId);
      
      expect(violationReported).toBe(true);
    });
  });
  
  describe('Audit Logging', () => {
    it('should audit tool executions', async () => {
      const serverId = 'audited-server';
      const toolName = 'writeFile';
      const args = { path: '/tmp/test.txt', content: 'test' };
      
      await securityManager.auditToolExecution(serverId, {
        toolName,
        arguments: args,
        timestamp: new Date()
      });
      
      const entries = await securityManager.getAuditLog({
        serverId,
        eventType: 'tool_execution'
      });
      
      expect(entries).toHaveLength(1);
      expect(entries[0]).toMatchObject({
        serverId,
        eventType: 'tool_execution',
        details: expect.objectContaining({
          toolName,
          arguments: args
        })
      });
    });
    
    it('should audit security violations', async () => {
      const serverId = 'violating-server';
      
      await securityManager.reportViolation(
        serverId,
        'unauthorized_access',
        {
          resource: '/etc/shadow',
          operation: 'read'
        }
      );
      
      const violations = await securityManager.getServerViolations(serverId);
      
      expect(violations).toHaveLength(1);
      expect(violations[0]).toMatchObject({
        type: 'unauthorized_access',
        details: expect.objectContaining({
          resource: '/etc/shadow'
        })
      });
    });
  });
  
  describe('Security Alerts', () => {
    it('should emit security alerts', async () => {
      let alertReceived: any = null;
      
      securityManager.on('securityAlert', (alert) => {
        alertReceived = alert;
      });
      
      // Trigger a security alert
      await securityManager.reportViolation(
        'malicious-server',
        'malware_detected',
        {
          file: 'virus.exe',
          signature: 'EICAR'
        }
      );
      
      expect(alertReceived).toBeTruthy();
      expect(alertReceived.serverName).toBe('malicious-server');
      expect(alertReceived.type).toBe('malware_detected');
    });
  });
});

// Helper to create test context
function createTestContext(): vscode.ExtensionContext {
  return {
    globalStorageUri: {
      fsPath: path.join(__dirname, 'test-storage')
    },
    subscriptions: [],
    extensionUri: vscode.Uri.file(__dirname),
    extensionPath: __dirname,
    globalState: {
      get: jest.fn(),
      update: jest.fn(),
      keys: jest.fn().mockReturnValue([])
    },
    workspaceState: {
      get: jest.fn(),
      update: jest.fn(),
      keys: jest.fn().mockReturnValue([])
    },
    secrets: {
      get: jest.fn(),
      store: jest.fn(),
      delete: jest.fn()
    }
  } as any;
}