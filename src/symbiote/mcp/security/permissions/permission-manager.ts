/**
 * Permission Manager
 * 
 * Manages permissions for MCP server operations
 */

import { EventEmitter } from 'events';
import * as path from 'path';
import * as fs from 'fs';
import {
  SecurityPolicy,
  SecurityRule,
  PermissionRequest,
  PermissionDecision,
  PermissionType,
  RuleCondition
} from '../types';

export interface PermissionManagerOptions {
  dataDirectory: string;
  defaultPolicy: SecurityPolicy;
  cacheDecisions?: boolean;
  decisionTTL?: number; // milliseconds
}

export interface PermissionCheckResult {
  allowed: boolean;
  requiresInteraction: boolean;
  reason?: string;
  matchedRule?: SecurityRule;
  conditions?: string[];
}

export interface StoredDecision {
  request: PermissionRequest;
  decision: PermissionDecision;
  expiresAt?: Date;
}

export class PermissionManager extends EventEmitter {
  private policies: Map<string, SecurityPolicy> = new Map();
  private decisions: Map<string, StoredDecision[]> = new Map();
  private options: Required<PermissionManagerOptions>;
  private decisionsFile: string;
  
  constructor(options: PermissionManagerOptions) {
    super();
    
    this.options = {
      cacheDecisions: true,
      decisionTTL: 3600000, // 1 hour default
      ...options
    };
    
    this.decisionsFile = path.join(this.options.dataDirectory, 'decisions.json');
    
    // Add default policy
    this.policies.set('default', this.options.defaultPolicy);
  }
  
  /**
   * Initialize permission manager
   */
  async initialize(): Promise<void> {
    // Create data directory
    await fs.promises.mkdir(this.options.dataDirectory, { recursive: true });
    
    // Load stored decisions
    await this.loadDecisions();
    
    // Load custom policies
    await this.loadPolicies();
  }
  
  /**
   * Check permission for request
   */
  async checkPermission(request: PermissionRequest): Promise<PermissionCheckResult> {
    // Check cached decisions first
    if (this.options.cacheDecisions) {
      const cached = this.getCachedDecision(request);
      if (cached) {
        return {
          allowed: cached.decision.allowed,
          requiresInteraction: false,
          reason: 'Cached decision',
          conditions: cached.decision.conditions
        };
      }
    }
    
    // Check against policies
    const result = await this.evaluatePolicies(request);
    
    // Log the check
    this.emit('permission-check', { request, result });
    
    return result;
  }
  
  /**
   * Store user decision
   */
  async storeDecision(
    request: PermissionRequest,
    decision: PermissionDecision
  ): Promise<void> {
    request.decision = decision;
    request.status = decision.allowed ? 'approved' : 'denied';
    
    // Store in memory
    const serverDecisions = this.decisions.get(request.serverId) || [];
    
    const stored: StoredDecision = {
      request,
      decision,
      expiresAt: decision.scope === 'session' ?
        new Date(Date.now() + this.options.decisionTTL) :
        undefined
    };
    
    serverDecisions.push(stored);
    this.decisions.set(request.serverId, serverDecisions);
    
    // Persist if permanent
    if (decision.scope === 'permanent') {
      await this.saveDecisions();
    }
    
    this.emit('decision-stored', { request, decision });
  }
  
  /**
   * Add custom policy
   */
  async addPolicy(policy: SecurityPolicy): Promise<void> {
    this.policies.set(policy.id, policy);
    await this.savePolicies();
    
    this.emit('policy-added', policy);
  }
  
  /**
   * Remove policy
   */
  async removePolicy(policyId: string): Promise<void> {
    if (policyId === 'default') {
      throw new Error('Cannot remove default policy');
    }
    
    this.policies.delete(policyId);
    await this.savePolicies();
    
    this.emit('policy-removed', policyId);
  }
  
  /**
   * Update policy
   */
  async updatePolicy(policy: SecurityPolicy): Promise<void> {
    this.policies.set(policy.id, policy);
    await this.savePolicies();
    
    this.emit('policy-updated', policy);
  }
  
  /**
   * Get all policies
   */
  getPolicies(): SecurityPolicy[] {
    return Array.from(this.policies.values())
      .sort((a, b) => b.priority - a.priority);
  }
  
  /**
   * Clear cached decisions
   */
  async clearDecisions(serverId?: string): Promise<void> {
    if (serverId) {
      this.decisions.delete(serverId);
    } else {
      this.decisions.clear();
    }
    
    await this.saveDecisions();
  }
  
  /**
   * Evaluate policies against request
   */
  private async evaluatePolicies(
    request: PermissionRequest
  ): Promise<PermissionCheckResult> {
    const policies = this.getPolicies().filter(p => p.enabled);
    
    for (const policy of policies) {
      for (const rule of policy.rules) {
        if (this.matchesRule(request, rule)) {
          const conditions = await this.evaluateConditions(request, rule);
          
          if (conditions.passed) {
            return {
              allowed: rule.action === 'allow',
              requiresInteraction: rule.action === 'prompt',
              reason: `Matched rule: ${rule.id}`,
              matchedRule: rule,
              conditions: conditions.messages
            };
          }
        }
      }
      
      // Check default action
      if (policy.defaultAction !== 'prompt') {
        return {
          allowed: policy.defaultAction === 'allow',
          requiresInteraction: false,
          reason: `Default policy action: ${policy.defaultAction}`
        };
      }
    }
    
    // No match, require interaction
    return {
      allowed: false,
      requiresInteraction: true,
      reason: 'No matching rule found'
    };
  }
  
  /**
   * Check if request matches rule
   */
  private matchesRule(request: PermissionRequest, rule: SecurityRule): boolean {
    // Check rule type
    if (!this.matchesRuleType(request.type, rule.type)) {
      return false;
    }
    
    // Check resource pattern
    if (!this.matchesResource(request.resource, rule.resource)) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Match permission type to rule type
   */
  private matchesRuleType(
    permissionType: PermissionType,
    ruleType: SecurityRule['type']
  ): boolean {
    const mapping = {
      [PermissionType.FileRead]: 'filesystem',
      [PermissionType.FileWrite]: 'filesystem',
      [PermissionType.FileDelete]: 'filesystem',
      [PermissionType.NetworkAccess]: 'network',
      [PermissionType.ProcessSpawn]: 'process',
      [PermissionType.SystemInfo]: 'resource',
      [PermissionType.ResourceAccess]: 'resource',
      [PermissionType.ToolExecution]: 'resource'
    };
    
    return mapping[permissionType] === ruleType;
  }
  
  /**
   * Match resource against pattern
   */
  private matchesResource(resource: string, pattern: string): boolean {
    // Handle glob patterns
    if (pattern.includes('*')) {
      const regex = this.globToRegex(pattern);
      return regex.test(resource);
    }
    
    // Handle comma-separated values (for network rules)
    if (pattern.includes(',')) {
      const values = pattern.split(',').map(v => v.trim());
      return values.some(v => this.matchesResource(resource, v));
    }
    
    // Exact match
    return resource === pattern;
  }
  
  /**
   * Convert glob pattern to regex
   */
  private globToRegex(glob: string): RegExp {
    const escaped = glob
      .replace(/[.+^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '.*')
      .replace(/\?/g, '.');
    
    return new RegExp(`^${escaped}$`);
  }
  
  /**
   * Evaluate rule conditions
   */
  private async evaluateConditions(
    request: PermissionRequest,
    rule: SecurityRule
  ): Promise<{ passed: boolean; messages: string[] }> {
    if (!rule.conditions || rule.conditions.length === 0) {
      return { passed: true, messages: [] };
    }
    
    const messages: string[] = [];
    
    for (const condition of rule.conditions) {
      const result = await this.evaluateCondition(request, condition);
      
      if (!result.passed) {
        messages.push(result.message);
        return { passed: false, messages };
      }
      
      messages.push(result.message);
    }
    
    return { passed: true, messages };
  }
  
  /**
   * Evaluate single condition
   */
  private async evaluateCondition(
    request: PermissionRequest,
    condition: RuleCondition
  ): Promise<{ passed: boolean; message: string }> {
    switch (condition.type) {
      case 'pattern':
        return this.evaluatePatternCondition(request, condition);
        
      case 'size':
        return this.evaluateSizeCondition(request, condition);
        
      case 'time':
        return this.evaluateTimeCondition(request, condition);
        
      case 'frequency':
        return this.evaluateFrequencyCondition(request, condition);
        
      default:
        return { passed: true, message: 'Unknown condition type' };
    }
  }
  
  /**
   * Evaluate pattern condition
   */
  private evaluatePatternCondition(
    request: PermissionRequest,
    condition: RuleCondition
  ): { passed: boolean; message: string } {
    const value = request.resource;
    let passed = false;
    
    switch (condition.operator) {
      case 'equals':
        passed = value === condition.value;
        break;
      case 'contains':
        passed = value.includes(condition.value);
        break;
      case 'startsWith':
        passed = value.startsWith(condition.value);
        break;
      case 'endsWith':
        passed = value.endsWith(condition.value);
        break;
      case 'matches':
        passed = new RegExp(condition.value).test(value);
        break;
    }
    
    return {
      passed,
      message: `Resource ${condition.operator} ${condition.value}: ${passed}`
    };
  }
  
  /**
   * Evaluate size condition
   */
  private async evaluateSizeCondition(
    request: PermissionRequest,
    condition: RuleCondition
  ): Promise<{ passed: boolean; message: string }> {
    // Get file size if applicable
    if (request.type === PermissionType.FileRead || 
        request.type === PermissionType.FileWrite) {
      try {
        const stats = await fs.promises.stat(request.resource);
        const sizeMB = stats.size / (1024 * 1024);
        
        let passed = false;
        switch (condition.operator) {
          case 'lessThan':
            passed = sizeMB < condition.value;
            break;
          case 'greaterThan':
            passed = sizeMB > condition.value;
            break;
        }
        
        return {
          passed,
          message: `File size ${sizeMB.toFixed(2)}MB ${condition.operator} ${condition.value}MB`
        };
      } catch {
        return { passed: true, message: 'Could not check file size' };
      }
    }
    
    return { passed: true, message: 'Size condition not applicable' };
  }
  
  /**
   * Evaluate time condition
   */
  private evaluateTimeCondition(
    request: PermissionRequest,
    condition: RuleCondition
  ): { passed: boolean; message: string } {
    const now = new Date();
    const hour = now.getHours();
    
    // Example: Only allow during business hours
    const [startHour, endHour] = condition.value;
    const passed = hour >= startHour && hour <= endHour;
    
    return {
      passed,
      message: `Current hour ${hour} within ${startHour}-${endHour}: ${passed}`
    };
  }
  
  /**
   * Evaluate frequency condition
   */
  private async evaluateFrequencyCondition(
    request: PermissionRequest,
    condition: RuleCondition
  ): Promise<{ passed: boolean; message: string }> {
    // Count recent requests of same type
    const recentRequests = await this.countRecentRequests(
      request.serverId,
      request.type,
      condition.value.windowSeconds * 1000
    );
    
    const passed = recentRequests < condition.value.maxCount;
    
    return {
      passed,
      message: `Request count ${recentRequests} < ${condition.value.maxCount}: ${passed}`
    };
  }
  
  /**
   * Count recent requests
   */
  private async countRecentRequests(
    serverId: string,
    type: PermissionType,
    windowMs: number
  ): Promise<number> {
    const decisions = this.decisions.get(serverId) || [];
    const cutoff = new Date(Date.now() - windowMs);
    
    return decisions.filter(d =>
      d.request.type === type &&
      d.request.timestamp > cutoff
    ).length;
  }
  
  /**
   * Get cached decision
   */
  private getCachedDecision(request: PermissionRequest): StoredDecision | null {
    const serverDecisions = this.decisions.get(request.serverId) || [];
    
    // Clean expired decisions
    const now = new Date();
    const valid = serverDecisions.filter(d =>
      !d.expiresAt || d.expiresAt > now
    );
    
    if (valid.length !== serverDecisions.length) {
      this.decisions.set(request.serverId, valid);
    }
    
    // Find matching decision
    return valid.find(d =>
      d.request.type === request.type &&
      d.request.resource === request.resource &&
      d.decision.remember
    ) || null;
  }
  
  /**
   * Load stored decisions
   */
  private async loadDecisions(): Promise<void> {
    try {
      const data = await fs.promises.readFile(this.decisionsFile, 'utf8');
      const stored = JSON.parse(data);
      
      // Convert back to Map
      for (const [serverId, decisions] of Object.entries(stored)) {
        this.decisions.set(serverId, decisions as StoredDecision[]);
      }
    } catch (error) {
      // File doesn't exist or is invalid
      this.decisions.clear();
    }
  }
  
  /**
   * Save decisions to disk
   */
  private async saveDecisions(): Promise<void> {
    // Convert Map to object for JSON
    const toSave: Record<string, StoredDecision[]> = {};
    
    for (const [serverId, decisions] of this.decisions) {
      // Only save permanent decisions
      const permanent = decisions.filter(d =>
        d.decision.scope === 'permanent'
      );
      
      if (permanent.length > 0) {
        toSave[serverId] = permanent;
      }
    }
    
    await fs.promises.writeFile(
      this.decisionsFile,
      JSON.stringify(toSave, null, 2)
    );
  }
  
  /**
   * Load custom policies
   */
  private async loadPolicies(): Promise<void> {
    const policiesFile = path.join(this.options.dataDirectory, 'policies.json');
    
    try {
      const data = await fs.promises.readFile(policiesFile, 'utf8');
      const policies = JSON.parse(data) as SecurityPolicy[];
      
      for (const policy of policies) {
        this.policies.set(policy.id, policy);
      }
    } catch {
      // No custom policies yet
    }
  }
  
  /**
   * Save policies to disk
   */
  private async savePolicies(): Promise<void> {
    const policiesFile = path.join(this.options.dataDirectory, 'policies.json');
    
    // Get all policies except default
    const toSave = Array.from(this.policies.values())
      .filter(p => p.id !== 'default');
    
    await fs.promises.writeFile(
      policiesFile,
      JSON.stringify(toSave, null, 2)
    );
  }
}