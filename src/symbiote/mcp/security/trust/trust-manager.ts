/**
 * Trust Manager
 * 
 * Manages trust levels and reputation for MCP servers
 */

import { EventEmitter } from 'events';
import * as fs from 'fs';
import * as path from 'path';
import { 
  TrustLevel, 
  TrustScore, 
  SecurityScanResult,
  SecurityIncident,
  AuditEntry
} from '../types';

export interface TrustManagerOptions {
  dataDirectory: string;
  defaultTrustLevel?: TrustLevel;
  trustDecayRate?: number; // Points per day
  incidentImpact?: {
    critical: number;
    high: number;
    medium: number;
    low: number;
  };
}

export interface ServerTrustProfile {
  serverId: string;
  serverName: string;
  trustScore: TrustScore;
  history: TrustEvent[];
  restrictions: TrustRestrictions;
  lastUpdated: Date;
}

export interface TrustEvent {
  timestamp: Date;
  type: 'scan' | 'incident' | 'permission' | 'manual' | 'decay';
  impact: number;
  description: string;
  metadata?: any;
}

export interface TrustRestrictions {
  maxMemoryMB: number;
  maxCpuPercent: number;
  maxFileHandles: number;
  maxNetworkConnections: number;
  allowedFilesystemPaths: string[];
  allowedNetworkHosts: string[];
  requiresUserApproval: boolean;
}

export interface TrustMetrics {
  averageTrustScore: number;
  trustedServers: number;
  untrustedServers: number;
  restrictedServers: number;
  recentIncidents: number;
}

export class TrustManager extends EventEmitter {
  private options: Required<TrustManagerOptions>;
  private profiles: Map<string, ServerTrustProfile> = new Map();
  private trustFile: string;
  private decayInterval: NodeJS.Timer | null = null;
  
  constructor(options: TrustManagerOptions) {
    super();
    
    this.options = {
      defaultTrustLevel: TrustLevel.Unknown,
      trustDecayRate: 1, // 1 point per day
      incidentImpact: {
        critical: -30,
        high: -20,
        medium: -10,
        low: -5
      },
      ...options
    };
    
    this.trustFile = path.join(this.options.dataDirectory, 'trust-profiles.json');
  }
  
  /**
   * Initialize trust manager
   */
  async initialize(): Promise<void> {
    // Create data directory
    await fs.promises.mkdir(this.options.dataDirectory, { recursive: true });
    
    // Load existing profiles
    await this.loadProfiles();
    
    // Start trust decay process
    this.startTrustDecay();
  }
  
  /**
   * Get server trust profile
   */
  getTrustProfile(serverId: string): ServerTrustProfile | null {
    return this.profiles.get(serverId) || null;
  }
  
  /**
   * Get trust level
   */
  getTrustLevel(serverId: string): TrustLevel {
    const profile = this.profiles.get(serverId);
    return profile?.trustScore.level || this.options.defaultTrustLevel;
  }
  
  /**
   * Get trust score
   */
  getTrustScore(serverId: string): number {
    const profile = this.profiles.get(serverId);
    return profile?.trustScore.score || 50; // Default neutral score
  }
  
  /**
   * Initialize server profile
   */
  async initializeServer(
    serverId: string,
    serverName: string,
    initialTrust?: TrustLevel
  ): Promise<ServerTrustProfile> {
    // Check if already exists
    let profile = this.profiles.get(serverId);
    
    if (!profile) {
      // Create new profile
      const trustLevel = initialTrust || this.options.defaultTrustLevel;
      const score = this.trustLevelToScore(trustLevel);
      
      profile = {
        serverId,
        serverName,
        trustScore: {
          score,
          level: trustLevel,
          factors: {
            scanResults: 0,
            incidentHistory: 0,
            permissionCompliance: 0,
            uptime: 0,
            userFeedback: 0
          }
        },
        history: [{
          timestamp: new Date(),
          type: 'manual',
          impact: 0,
          description: 'Server profile created',
          metadata: { initialTrust: trustLevel }
        }],
        restrictions: this.getDefaultRestrictions(trustLevel),
        lastUpdated: new Date()
      };
      
      this.profiles.set(serverId, profile);
      await this.saveProfiles();
      
      this.emit('profile-created', profile);
    }
    
    return profile;
  }
  
  /**
   * Update trust based on security scan
   */
  async updateFromScan(result: SecurityScanResult): Promise<void> {
    const profile = await this.ensureProfile(result.serverId);
    
    // Calculate impact based on scan results
    let impact = 0;
    
    if (result.passed) {
      impact = 5; // Boost for clean scan
    } else {
      // Deduct based on findings
      const criticalCount = result.findings.filter(f => f.severity === 'critical').length;
      const highCount = result.findings.filter(f => f.severity === 'high').length;
      
      impact = -(criticalCount * 10 + highCount * 5);
    }
    
    // Update score
    await this.adjustTrust(result.serverId, impact, 'scan', 
      `Security scan: ${result.passed ? 'passed' : 'failed'} with score ${result.score}`,
      { scanResult: result }
    );
    
    // Update scan factor
    profile.trustScore.factors.scanResults = result.score / 20; // Normalize to 0-5
  }
  
  /**
   * Update trust based on incident
   */
  async updateFromIncident(incident: SecurityIncident): Promise<void> {
    const profile = await this.ensureProfile(incident.serverId);
    
    // Calculate impact based on severity
    const impact = this.options.incidentImpact[incident.severity];
    
    // Update score
    await this.adjustTrust(incident.serverId, impact, 'incident',
      `Security incident: ${incident.type} (${incident.severity})`,
      { incident }
    );
    
    // Update incident factor
    const recentIncidents = profile.history
      .filter(e => e.type === 'incident' && 
        e.timestamp > new Date(Date.now() - 30 * 24 * 60 * 60 * 1000))
      .length;
    
    profile.trustScore.factors.incidentHistory = Math.max(0, 5 - recentIncidents);
  }
  
  /**
   * Update trust based on permission compliance
   */
  async updateFromPermission(
    serverId: string,
    compliant: boolean,
    details?: string
  ): Promise<void> {
    const profile = await this.ensureProfile(serverId);
    
    // Small adjustments for permission behavior
    const impact = compliant ? 1 : -2;
    
    await this.adjustTrust(serverId, impact, 'permission',
      details || `Permission ${compliant ? 'compliance' : 'violation'}`
    );
    
    // Update compliance factor
    const recentPermissions = profile.history
      .filter(e => e.type === 'permission' && 
        e.timestamp > new Date(Date.now() - 7 * 24 * 60 * 60 * 1000));
    
    const compliantCount = recentPermissions.filter(e => e.impact > 0).length;
    const totalCount = recentPermissions.length || 1;
    
    profile.trustScore.factors.permissionCompliance = (compliantCount / totalCount) * 5;
  }
  
  /**
   * Manually adjust trust
   */
  async setTrustLevel(
    serverId: string,
    level: TrustLevel,
    reason: string,
    adjustedBy: string
  ): Promise<void> {
    const profile = await this.ensureProfile(serverId);
    const oldLevel = profile.trustScore.level;
    const newScore = this.trustLevelToScore(level);
    const impact = newScore - profile.trustScore.score;
    
    await this.adjustTrust(serverId, impact, 'manual',
      `Manual adjustment: ${oldLevel} → ${level}. Reason: ${reason}`,
      { adjustedBy, reason, oldLevel, newLevel: level }
    );
  }
  
  /**
   * Get trust restrictions
   */
  getTrustRestrictions(serverId: string): TrustRestrictions {
    const profile = this.profiles.get(serverId);
    return profile?.restrictions || this.getDefaultRestrictions(TrustLevel.Unknown);
  }
  
  /**
   * Update trust restrictions
   */
  async updateRestrictions(
    serverId: string,
    restrictions: Partial<TrustRestrictions>
  ): Promise<void> {
    const profile = await this.ensureProfile(serverId);
    
    profile.restrictions = {
      ...profile.restrictions,
      ...restrictions
    };
    
    profile.lastUpdated = new Date();
    await this.saveProfiles();
    
    this.emit('restrictions-updated', { serverId, restrictions: profile.restrictions });
  }
  
  /**
   * Get trust metrics
   */
  getTrustMetrics(): TrustMetrics {
    const profiles = Array.from(this.profiles.values());
    
    const metrics: TrustMetrics = {
      averageTrustScore: 0,
      trustedServers: 0,
      untrustedServers: 0,
      restrictedServers: 0,
      recentIncidents: 0
    };
    
    if (profiles.length === 0) {
      return metrics;
    }
    
    // Calculate metrics
    let totalScore = 0;
    
    for (const profile of profiles) {
      totalScore += profile.trustScore.score;
      
      switch (profile.trustScore.level) {
        case TrustLevel.Trusted:
          metrics.trustedServers++;
          break;
        case TrustLevel.Untrusted:
          metrics.untrustedServers++;
          break;
        case TrustLevel.Restricted:
          metrics.restrictedServers++;
          break;
      }
      
      // Count recent incidents
      metrics.recentIncidents += profile.history
        .filter(e => e.type === 'incident' &&
          e.timestamp > new Date(Date.now() - 7 * 24 * 60 * 60 * 1000))
        .length;
    }
    
    metrics.averageTrustScore = totalScore / profiles.length;
    
    return metrics;
  }
  
  /**
   * Export trust report
   */
  async exportTrustReport(outputPath: string): Promise<void> {
    const profiles = Array.from(this.profiles.values());
    const metrics = this.getTrustMetrics();
    
    const report = {
      generated: new Date(),
      metrics,
      profiles: profiles.map(p => ({
        serverId: p.serverId,
        serverName: p.serverName,
        trustLevel: p.trustScore.level,
        trustScore: p.trustScore.score,
        factors: p.trustScore.factors,
        recentEvents: p.history.slice(-10),
        restrictions: p.restrictions
      }))
    };
    
    await fs.promises.writeFile(
      outputPath,
      JSON.stringify(report, null, 2)
    );
  }
  
  /**
   * Adjust trust score
   */
  private async adjustTrust(
    serverId: string,
    impact: number,
    eventType: TrustEvent['type'],
    description: string,
    metadata?: any
  ): Promise<void> {
    const profile = await this.ensureProfile(serverId);
    
    // Apply impact
    const oldScore = profile.trustScore.score;
    profile.trustScore.score = Math.max(0, Math.min(100, oldScore + impact));
    
    // Update trust level
    const oldLevel = profile.trustScore.level;
    profile.trustScore.level = this.scoreToTrustLevel(profile.trustScore.score);
    
    // Add to history
    profile.history.push({
      timestamp: new Date(),
      type: eventType,
      impact,
      description,
      metadata
    });
    
    // Keep history size manageable
    if (profile.history.length > 1000) {
      profile.history = profile.history.slice(-500);
    }
    
    // Update restrictions if trust level changed
    if (oldLevel !== profile.trustScore.level) {
      profile.restrictions = this.getDefaultRestrictions(profile.trustScore.level);
      
      this.emit('trust-level-changed', {
        serverId,
        oldLevel,
        newLevel: profile.trustScore.level,
        score: profile.trustScore.score
      });
    }
    
    profile.lastUpdated = new Date();
    await this.saveProfiles();
    
    this.emit('trust-updated', {
      serverId,
      oldScore,
      newScore: profile.trustScore.score,
      impact,
      event: eventType
    });
  }
  
  /**
   * Ensure profile exists
   */
  private async ensureProfile(serverId: string): Promise<ServerTrustProfile> {
    let profile = this.profiles.get(serverId);
    
    if (!profile) {
      profile = await this.initializeServer(serverId, serverId);
    }
    
    return profile;
  }
  
  /**
   * Convert trust level to score
   */
  private trustLevelToScore(level: TrustLevel): number {
    switch (level) {
      case TrustLevel.Trusted:
        return 80;
      case TrustLevel.Verified:
        return 70;
      case TrustLevel.Unknown:
        return 50;
      case TrustLevel.Restricted:
        return 30;
      case TrustLevel.Untrusted:
        return 10;
      default:
        return 50;
    }
  }
  
  /**
   * Convert score to trust level
   */
  private scoreToTrustLevel(score: number): TrustLevel {
    if (score >= 80) return TrustLevel.Trusted;
    if (score >= 70) return TrustLevel.Verified;
    if (score >= 40) return TrustLevel.Unknown;
    if (score >= 20) return TrustLevel.Restricted;
    return TrustLevel.Untrusted;
  }
  
  /**
   * Get default restrictions for trust level
   */
  private getDefaultRestrictions(level: TrustLevel): TrustRestrictions {
    switch (level) {
      case TrustLevel.Trusted:
        return {
          maxMemoryMB: 2048,
          maxCpuPercent: 80,
          maxFileHandles: 1000,
          maxNetworkConnections: 100,
          allowedFilesystemPaths: ['*'],
          allowedNetworkHosts: ['*'],
          requiresUserApproval: false
        };
        
      case TrustLevel.Verified:
        return {
          maxMemoryMB: 1024,
          maxCpuPercent: 60,
          maxFileHandles: 500,
          maxNetworkConnections: 50,
          allowedFilesystemPaths: [],
          allowedNetworkHosts: [],
          requiresUserApproval: false
        };
        
      case TrustLevel.Unknown:
        return {
          maxMemoryMB: 512,
          maxCpuPercent: 40,
          maxFileHandles: 200,
          maxNetworkConnections: 20,
          allowedFilesystemPaths: [],
          allowedNetworkHosts: [],
          requiresUserApproval: true
        };
        
      case TrustLevel.Restricted:
        return {
          maxMemoryMB: 256,
          maxCpuPercent: 20,
          maxFileHandles: 100,
          maxNetworkConnections: 10,
          allowedFilesystemPaths: [],
          allowedNetworkHosts: [],
          requiresUserApproval: true
        };
        
      case TrustLevel.Untrusted:
        return {
          maxMemoryMB: 128,
          maxCpuPercent: 10,
          maxFileHandles: 50,
          maxNetworkConnections: 5,
          allowedFilesystemPaths: [],
          allowedNetworkHosts: [],
          requiresUserApproval: true
        };
    }
  }
  
  /**
   * Load profiles from disk
   */
  private async loadProfiles(): Promise<void> {
    try {
      const data = await fs.promises.readFile(this.trustFile, 'utf8');
      const saved = JSON.parse(data);
      
      // Convert dates
      for (const profile of saved) {
        profile.lastUpdated = new Date(profile.lastUpdated);
        for (const event of profile.history) {
          event.timestamp = new Date(event.timestamp);
        }
        
        this.profiles.set(profile.serverId, profile);
      }
    } catch {
      // No profiles yet
    }
  }
  
  /**
   * Save profiles to disk
   */
  private async saveProfiles(): Promise<void> {
    const profiles = Array.from(this.profiles.values());
    
    await fs.promises.writeFile(
      this.trustFile,
      JSON.stringify(profiles, null, 2)
    );
  }
  
  /**
   * Start trust decay process
   */
  private startTrustDecay(): void {
    // Apply trust decay daily
    this.decayInterval = setInterval(() => {
      this.applyTrustDecay();
    }, 24 * 60 * 60 * 1000); // Daily
  }
  
  /**
   * Apply trust decay
   */
  private async applyTrustDecay(): Promise<void> {
    const now = new Date();
    
    for (const [serverId, profile] of this.profiles) {
      // Only decay if no recent positive events
      const recentPositiveEvents = profile.history
        .filter(e => e.impact > 0 && 
          e.timestamp > new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000))
        .length;
      
      if (recentPositiveEvents === 0) {
        // Apply decay
        await this.adjustTrust(
          serverId,
          -this.options.trustDecayRate,
          'decay',
          'Trust decay due to inactivity'
        );
      }
    }
  }
  
  /**
   * Cleanup
   */
  async cleanup(): Promise<void> {
    if (this.decayInterval) {
      clearInterval(this.decayInterval);
    }
    
    await this.saveProfiles();
  }
}