/**
 * Security Manager Tests
 */

import { SecurityManager } from '../security-manager';
import { TrustLevel, PermissionType } from '../types';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

describe('SecurityManager', () => {
  let securityManager: SecurityManager;
  let tempDir: string;
  
  beforeEach(async () => {
    // Create temp directory
    tempDir = path.join(os.tmpdir(), `mcp-security-test-${Date.now()}`);
    await fs.promises.mkdir(tempDir, { recursive: true });
    
    // Initialize security manager
    securityManager = new SecurityManager({
      dataDirectory: tempDir,
      defaultPolicy: {
        id: 'test-default',
        name: 'Test Default Policy',
        description: 'Default policy for tests',
        enabled: true,
        priority: 0,
        rules: [],
        defaultAction: 'prompt',
        appliesTo: []
      },
      enableSandbox: true,
      enableScanning: true,
      enableAuditing: true
    });
    
    await securityManager.initialize();
  });
  
  afterEach(async () => {
    // Cleanup
    await securityManager.cleanup();
    await fs.promises.rm(tempDir, { recursive: true, force: true });
  });
  
  describe('Server Profile Management', () => {
    test('should initialize server profile', async () => {
      const serverId = 'test-server-1';
      const serverName = 'Test Server';
      
      await securityManager.initializeServerProfile(
        serverId,
        serverName,
        TrustLevel.Unknown
      );
      
      const trustLevel = securityManager.getTrustLevel(serverId);
      expect(trustLevel).toBe(TrustLevel.Unknown);
    });
    
    test('should update trust level', async () => {
      const serverId = 'test-server-2';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      await securityManager.setServerTrustLevel(
        serverId,
        TrustLevel.Verified,
        'Passed security review',
        'test-user'
      );
      
      const trustLevel = securityManager.getTrustLevel(serverId);
      expect(trustLevel).toBe(TrustLevel.Verified);
    });
  });
  
  describe('Permission System', () => {
    test('should check permissions based on policy', async () => {
      const serverId = 'test-server-3';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Verified
      );
      
      // Add a test policy
      await securityManager.addSecurityPolicy({
        id: 'test-policy',
        name: 'Test Policy',
        description: 'Test policy',
        enabled: true,
        priority: 100,
        rules: [
          {
            id: 'allow-temp-files',
            name: 'Allow Temp Files',
            type: 'filesystem',
            resource: '/tmp/*',
            action: 'allow',
            conditions: []
          }
        ],
        defaultAction: 'deny',
        appliesTo: []
      });
      
      const allowed = await securityManager.checkPermission(
        serverId,
        PermissionType.FileRead,
        '/tmp/test.txt'
      );
      
      expect(allowed).toBe(true);
      
      const denied = await securityManager.checkPermission(
        serverId,
        PermissionType.FileRead,
        '/etc/passwd'
      );
      
      expect(denied).toBe(false);
    });
    
    test('should handle permission requests with user interaction', async () => {
      const serverId = 'test-server-4';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      // Mock user decision
      const mockDecision = {
        allowed: true,
        remember: true,
        scope: 'session' as const,
        timestamp: new Date()
      };
      
      // Create permission request
      const request = await securityManager.createPermissionRequest(
        serverId,
        PermissionType.NetworkAccess,
        'https://api.example.com',
        'Access API endpoint'
      );
      
      // Handle decision
      await securityManager.handlePermissionDecision(
        request.id,
        mockDecision
      );
      
      // Check if decision is cached
      const allowed = await securityManager.checkPermission(
        serverId,
        PermissionType.NetworkAccess,
        'https://api.example.com'
      );
      
      expect(allowed).toBe(true);
    });
  });
  
  describe('Security Scanning', () => {
    test('should scan server directory', async () => {
      const serverId = 'test-server-5';
      const testServerPath = path.join(tempDir, 'test-server');
      
      // Create test server files
      await fs.promises.mkdir(testServerPath, { recursive: true });
      await fs.promises.writeFile(
        path.join(testServerPath, 'index.js'),
        'console.log("Hello World");'
      );
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      const result = await securityManager.scanServer(serverId, testServerPath);
      
      expect(result).toBeDefined();
      expect(result.passed).toBe(true);
      expect(result.score).toBeGreaterThan(0);
    });
    
    test('should detect security issues', async () => {
      const serverId = 'test-server-6';
      const testServerPath = path.join(tempDir, 'vulnerable-server');
      
      // Create vulnerable server files
      await fs.promises.mkdir(testServerPath, { recursive: true });
      await fs.promises.writeFile(
        path.join(testServerPath, 'vulnerable.js'),
        `
        const apiKey = "sk-1234567890abcdef";
        eval(userInput);
        require('child_process').exec(command);
        `
      );
      
      await securityManager.initializeServerProfile(
        serverId,
        'Vulnerable Server',
        TrustLevel.Unknown
      );
      
      const result = await securityManager.scanServer(serverId, testServerPath);
      
      expect(result.passed).toBe(false);
      expect(result.findings.length).toBeGreaterThan(0);
      expect(result.findings.some(f => f.severity === 'critical')).toBe(true);
    });
  });
  
  describe('Sandbox Management', () => {
    test('should create sandboxed process', async () => {
      const serverId = 'test-server-7';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Restricted
      );
      
      const config = await securityManager.getSandboxConfig(serverId);
      
      expect(config).toBeDefined();
      expect(config.isolationLevel).toBeDefined();
      expect(config.resourceLimits).toBeDefined();
      
      // Create sandbox (mock process)
      const sandbox = await securityManager.createSandbox(
        serverId,
        'echo',
        ['test'],
        config
      );
      
      expect(sandbox).toBeDefined();
      
      // Cleanup
      await securityManager.terminateSandbox(serverId);
    });
    
    test('should enforce resource limits', async () => {
      const serverId = 'test-server-8';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Untrusted
      );
      
      const config = await securityManager.getSandboxConfig(serverId);
      
      // Verify strict limits for untrusted server
      expect(config.resourceLimits.maxMemoryMB).toBeLessThanOrEqual(256);
      expect(config.resourceLimits.maxCpuPercent).toBeLessThanOrEqual(20);
      expect(config.network.enabled).toBe(false);
    });
  });
  
  describe('Audit System', () => {
    test('should log security events', async () => {
      const serverId = 'test-server-9';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      // Generate some events
      await securityManager.checkPermission(
        serverId,
        PermissionType.FileRead,
        '/tmp/test.txt'
      );
      
      await securityManager.logToolExecution(serverId, {
        toolName: 'test-tool',
        args: { test: true },
        result: 'success',
        duration: 100
      });
      
      // Query audit events
      const events = await securityManager.queryAuditEvents(serverId);
      
      expect(events.length).toBeGreaterThan(0);
      expect(events.some(e => e.category === 'permission')).toBe(true);
      expect(events.some(e => e.category === 'tool')).toBe(true);
    });
    
    test('should generate audit summary', async () => {
      const serverId = 'test-server-10';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      // Generate events
      for (let i = 0; i < 5; i++) {
        await securityManager.checkPermission(
          serverId,
          PermissionType.FileRead,
          `/tmp/test${i}.txt`
        );
      }
      
      const summary = await securityManager.generateAuditSummary(
        serverId,
        new Date(Date.now() - 3600000), // 1 hour ago
        new Date()
      );
      
      expect(summary).toBeDefined();
      expect(summary.totals.events).toBeGreaterThan(0);
      expect(summary.recommendations).toBeDefined();
    });
  });
  
  describe('Security Incidents', () => {
    test('should create and track incidents', async () => {
      const serverId = 'test-server-11';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      // Create incident
      const incident = {
        id: 'test-incident-1',
        timestamp: new Date(),
        serverId,
        serverName: 'Test Server',
        type: 'suspicious_activity',
        severity: 'high' as const,
        description: 'Test incident',
        details: { test: true },
        resolved: false
      };
      
      await securityManager.logIncident(incident);
      
      // Get incidents
      const incidents = await securityManager.getSecurityIncidents();
      
      expect(incidents.length).toBeGreaterThan(0);
      expect(incidents[0].id).toBe(incident.id);
      
      // Resolve incident
      await securityManager.resolveIncident(
        incident.id,
        'Test resolution',
        'test-user'
      );
      
      const resolved = await securityManager.getSecurityIncidents();
      expect(resolved[0].resolved).toBe(true);
    });
  });
  
  describe('Network Security', () => {
    test('should enforce network restrictions', async () => {
      const serverId = 'test-server-12';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Restricted
      );
      
      const allowed = await securityManager.checkNetworkAccess(
        serverId,
        'http://localhost:3000'
      );
      
      expect(allowed).toBe(false); // Localhost blocked for restricted servers
      
      const externalAllowed = await securityManager.checkNetworkAccess(
        serverId,
        'https://api.example.com'
      );
      
      // Should depend on policy
      expect(typeof externalAllowed).toBe('boolean');
    });
  });
  
  describe('Trust Management', () => {
    test('should update trust based on events', async () => {
      const serverId = 'test-server-13';
      
      await securityManager.initializeServerProfile(
        serverId,
        'Test Server',
        TrustLevel.Unknown
      );
      
      const initialScore = securityManager.getTrustScore(serverId);
      
      // Positive event - clean scan
      await securityManager.updateTrustFromScan({
        serverId,
        serverPath: '/test',
        timestamp: new Date(),
        passed: true,
        findings: [],
        score: 100,
        recommendations: []
      });
      
      const updatedScore = securityManager.getTrustScore(serverId);
      expect(updatedScore).toBeGreaterThan(initialScore);
      
      // Negative event - incident
      await securityManager.updateTrustFromIncident({
        id: 'test-incident',
        timestamp: new Date(),
        serverId,
        serverName: 'Test Server',
        type: 'malicious_activity',
        severity: 'critical',
        description: 'Test incident',
        details: {},
        resolved: false
      });
      
      const finalScore = securityManager.getTrustScore(serverId);
      expect(finalScore).toBeLessThan(updatedScore);
    });
  });
});