/**
 * Configuration Store - Handles versioning and persistence
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import * as crypto from 'crypto';
import {
  ConfigStore,
  ConfigVersion,
  OrchestrationConfig
} from './config-interfaces';

export class FileSystemConfigStore implements ConfigStore {
  private basePath: string;
  private currentFile: string;
  private versionsDir: string;

  constructor(basePath: string = '.symbiote/config') {
    this.basePath = basePath;
    this.currentFile = path.join(basePath, 'current.json');
    this.versionsDir = path.join(basePath, 'versions');
  }

  /**
   * Initialize store directories
   */
  async initialize(): Promise<void> {
    await fs.mkdir(this.versionsDir, { recursive: true });
  }

  /**
   * Save configuration with optional version info
   */
  async save(
    config: OrchestrationConfig,
    version?: ConfigVersion
  ): Promise<void> {
    await this.initialize();

    // Generate version info if not provided
    const versionInfo: ConfigVersion = version || {
      version: config.version || this.generateVersion(),
      timestamp: new Date(),
      author: process.env.USER || 'system',
      description: 'Configuration update',
      checksum: this.calculateChecksum(config)
    };

    // Save to versions directory
    const versionFile = path.join(
      this.versionsDir,
      `config-${versionInfo.version}-${versionInfo.timestamp.getTime()}.json`
    );

    const versionData = {
      version: versionInfo,
      config
    };

    await fs.writeFile(
      versionFile,
      JSON.stringify(versionData, null, 2),
      'utf-8'
    );

    // Update current config
    await fs.writeFile(
      this.currentFile,
      JSON.stringify({ version: versionInfo, config }, null, 2),
      'utf-8'
    );
  }

  /**
   * Load configuration by version or current
   */
  async load(version?: string): Promise<OrchestrationConfig> {
    if (!version) {
      // Load current config
      try {
        const content = await fs.readFile(this.currentFile, 'utf-8');
        const data = JSON.parse(content);
        return data.config;
      } catch (error) {
        // Return empty config if no current config exists
        return {};
      }
    }

    // Load specific version
    const versions = await this.listVersions();
    const targetVersion = versions.find(v => v.version === version);
    
    if (!targetVersion) {
      throw new Error(`Version ${version} not found`);
    }

    const files = await fs.readdir(this.versionsDir);
    const versionFile = files.find(f => 
      f.includes(`config-${version}`) && 
      f.includes(targetVersion.timestamp.getTime().toString())
    );

    if (!versionFile) {
      throw new Error(`Version file for ${version} not found`);
    }

    const content = await fs.readFile(
      path.join(this.versionsDir, versionFile),
      'utf-8'
    );
    const data = JSON.parse(content);
    return data.config;
  }

  /**
   * List all available versions
   */
  async listVersions(): Promise<ConfigVersion[]> {
    await this.initialize();

    const versions: ConfigVersion[] = [];
    
    try {
      const files = await fs.readdir(this.versionsDir);
      
      for (const file of files) {
        if (file.startsWith('config-') && file.endsWith('.json')) {
          const filepath = path.join(this.versionsDir, file);
          const content = await fs.readFile(filepath, 'utf-8');
          const data = JSON.parse(content);
          
          if (data.version) {
            versions.push({
              ...data.version,
              timestamp: new Date(data.version.timestamp)
            });
          }
        }
      }
    } catch (error) {
      // Return empty array if no versions exist
    }

    // Sort by timestamp descending
    return versions.sort((a, b) => 
      b.timestamp.getTime() - a.timestamp.getTime()
    );
  }

  /**
   * Rollback to a specific version
   */
  async rollback(version: string): Promise<OrchestrationConfig> {
    const config = await this.load(version);
    
    // Get version info
    const versions = await this.listVersions();
    const versionInfo = versions.find(v => v.version === version);
    
    if (!versionInfo) {
      throw new Error(`Version ${version} not found`);
    }

    // Save as current with rollback note
    await this.save(config, {
      ...versionInfo,
      version: this.generateVersion(),
      timestamp: new Date(),
      description: `Rollback to version ${version} (${versionInfo.description})`
    });

    return config;
  }

  /**
   * Delete a specific version
   */
  async delete(version: string): Promise<void> {
    const versions = await this.listVersions();
    const targetVersion = versions.find(v => v.version === version);
    
    if (!targetVersion) {
      throw new Error(`Version ${version} not found`);
    }

    // Don't allow deleting the current version
    const current = await this.getCurrentVersion();
    if (current && current.version === version) {
      throw new Error('Cannot delete current version');
    }

    const files = await fs.readdir(this.versionsDir);
    const versionFile = files.find(f => 
      f.includes(`config-${version}`) && 
      f.includes(targetVersion.timestamp.getTime().toString())
    );

    if (versionFile) {
      await fs.unlink(path.join(this.versionsDir, versionFile));
    }
  }

  /**
   * Get the current version info
   */
  async getCurrentVersion(): Promise<ConfigVersion | null> {
    try {
      const content = await fs.readFile(this.currentFile, 'utf-8');
      const data = JSON.parse(content);
      return data.version ? {
        ...data.version,
        timestamp: new Date(data.version.timestamp)
      } : null;
    } catch (error) {
      return null;
    }
  }

  /**
   * Export configuration to a file
   */
  async export(
    version: string,
    outputPath: string
  ): Promise<void> {
    const config = await this.load(version);
    const versions = await this.listVersions();
    const versionInfo = versions.find(v => v.version === version);

    const exportData = {
      version: versionInfo,
      config,
      exported: {
        timestamp: new Date(),
        host: process.env.HOSTNAME || 'unknown',
        user: process.env.USER || 'unknown'
      }
    };

    await fs.writeFile(
      outputPath,
      JSON.stringify(exportData, null, 2),
      'utf-8'
    );
  }

  /**
   * Import configuration from a file
   */
  async import(
    inputPath: string,
    description?: string
  ): Promise<OrchestrationConfig> {
    const content = await fs.readFile(inputPath, 'utf-8');
    const data = JSON.parse(content);

    if (!data.config) {
      throw new Error('Invalid import file: missing config');
    }

    // Save imported config with new version
    await this.save(data.config, {
      version: this.generateVersion(),
      timestamp: new Date(),
      author: process.env.USER || 'import',
      description: description || `Imported from ${path.basename(inputPath)}`,
      checksum: this.calculateChecksum(data.config)
    });

    return data.config;
  }

  /**
   * Clean up old versions
   */
  async cleanup(keepCount: number = 10): Promise<number> {
    const versions = await this.listVersions();
    
    if (versions.length <= keepCount) {
      return 0;
    }

    // Keep the most recent versions
    const toDelete = versions.slice(keepCount);
    let deletedCount = 0;

    for (const version of toDelete) {
      try {
        await this.delete(version.version);
        deletedCount++;
      } catch (error) {
        // Skip if cannot delete (e.g., current version)
      }
    }

    return deletedCount;
  }

  // Private helper methods

  private generateVersion(): string {
    const now = new Date();
    return `${now.getFullYear()}.${now.getMonth() + 1}.${now.getDate()}-${now.getHours()}${now.getMinutes()}${now.getSeconds()}`;
  }

  private calculateChecksum(config: OrchestrationConfig): string {
    const content = JSON.stringify(config, Object.keys(config).sort());
    return crypto.createHash('sha256').update(content).digest('hex').slice(0, 8);
  }
}

/**
 * In-memory configuration store for testing
 */
export class InMemoryConfigStore implements ConfigStore {
  private versions: Map<string, { version: ConfigVersion; config: OrchestrationConfig }> = new Map();
  private current: OrchestrationConfig = {};
  private currentVersion: ConfigVersion | null = null;

  async save(
    config: OrchestrationConfig,
    version?: ConfigVersion
  ): Promise<void> {
    const versionInfo: ConfigVersion = version || {
      version: this.generateVersion(),
      timestamp: new Date(),
      author: 'system',
      description: 'Configuration update',
      checksum: this.calculateChecksum(config)
    };

    this.versions.set(versionInfo.version, { version: versionInfo, config });
    this.current = config;
    this.currentVersion = versionInfo;
  }

  async load(version?: string): Promise<OrchestrationConfig> {
    if (!version) {
      return this.current;
    }

    const versionData = this.versions.get(version);
    if (!versionData) {
      throw new Error(`Version ${version} not found`);
    }

    return versionData.config;
  }

  async listVersions(): Promise<ConfigVersion[]> {
    return Array.from(this.versions.values())
      .map(v => v.version)
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
  }

  async rollback(version: string): Promise<OrchestrationConfig> {
    const config = await this.load(version);
    const versionInfo = this.versions.get(version)?.version;
    
    if (!versionInfo) {
      throw new Error(`Version ${version} not found`);
    }

    await this.save(config, {
      ...versionInfo,
      version: this.generateVersion(),
      timestamp: new Date(),
      description: `Rollback to version ${version}`
    });

    return config;
  }

  async delete(version: string): Promise<void> {
    if (this.currentVersion && this.currentVersion.version === version) {
      throw new Error('Cannot delete current version');
    }
    this.versions.delete(version);
  }

  private generateVersion(): string {
    return `v${Date.now()}`;
  }

  private calculateChecksum(config: OrchestrationConfig): string {
    return 'mock-checksum';
  }
}