/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

export interface ExtensionCompatibilityResult {
  extensionId: string;
  name: string;
  version: string;
  compatible: boolean;
  issues: CompatibilityIssue[];
  apiUsage: APIUsage[];
  recommendations: string[];
}

export interface CompatibilityIssue {
  severity: 'error' | 'warning' | 'info';
  category: string;
  message: string;
  location?: string;
}

export interface APIUsage {
  api: string;
  count: number;
  supported: boolean;
}

export class ExtensionCompatibilityTester {
  private readonly knownIncompatibleAPIs = [
    'vscode.proposed',
    'vscode.experimental'
  ];

  private readonly criticalExtensions = [
    'ms-python.python',
    'ms-vscode.cpptools',
    'ms-dotnettools.csharp',
    'golang.go',
    'rust-lang.rust-analyzer',
    'dbaeumer.vscode-eslint',
    'esbenp.prettier-vscode',
    'eamodio.gitlens',
    'ms-vscode-remote.remote-containers',
    'ms-vscode-remote.remote-ssh',
    'ms-vscode-remote.remote-wsl',
    'GitHub.copilot',
    'ms-toolsai.jupyter',
    'ritwickdey.LiveServer',
    'formulahendry.code-runner'
  ];

  async testExtension(extensionId: string): Promise<ExtensionCompatibilityResult> {
    const extension = vscode.extensions.getExtension(extensionId);
    
    if (!extension) {
      return {
        extensionId,
        name: 'Unknown',
        version: 'Unknown',
        compatible: false,
        issues: [{
          severity: 'error',
          category: 'installation',
          message: 'Extension not found'
        }],
        apiUsage: [],
        recommendations: ['Install the extension first']
      };
    }

    const issues: CompatibilityIssue[] = [];
    const apiUsage: APIUsage[] = [];
    const recommendations: string[] = [];

    // Check activation
    try {
      await extension.activate();
    } catch (error) {
      issues.push({
        severity: 'error',
        category: 'activation',
        message: `Failed to activate: ${error.message}`
      });
    }

    // Check package.json
    const packageJson = extension.packageJSON;
    this.checkPackageJson(packageJson, issues, apiUsage);

    // Check contribution points
    this.checkContributions(packageJson.contributes || {}, issues);

    // Check API usage
    if (extension.isActive) {
      this.checkAPIUsage(extension, apiUsage, issues);
    }

    // Generate recommendations
    this.generateRecommendations(issues, recommendations);

    return {
      extensionId: extension.id,
      name: packageJson.displayName || packageJson.name,
      version: packageJson.version,
      compatible: issues.filter(i => i.severity === 'error').length === 0,
      issues,
      apiUsage,
      recommendations
    };
  }

  async testAllExtensions(): Promise<ExtensionCompatibilityResult[]> {
    const results: ExtensionCompatibilityResult[] = [];
    
    for (const extension of vscode.extensions.all) {
      // Skip built-in extensions
      if (extension.packageJSON.isBuiltin) {
        continue;
      }
      
      const result = await this.testExtension(extension.id);
      results.push(result);
    }
    
    return results;
  }

  async testCriticalExtensions(): Promise<ExtensionCompatibilityResult[]> {
    const results: ExtensionCompatibilityResult[] = [];
    
    for (const extensionId of this.criticalExtensions) {
      const result = await this.testExtension(extensionId);
      results.push(result);
    }
    
    return results;
  }

  private checkPackageJson(
    packageJson: any,
    issues: CompatibilityIssue[],
    apiUsage: APIUsage[]
  ): void {
    // Check engine compatibility
    if (packageJson.engines?.vscode) {
      const requiredVersion = packageJson.engines.vscode;
      const currentVersion = vscode.version;
      
      if (!this.isVersionCompatible(currentVersion, requiredVersion)) {
        issues.push({
          severity: 'warning',
          category: 'version',
          message: `Requires VS Code ${requiredVersion}, current is ${currentVersion}`
        });
      }
    }

    // Check for proposed API usage
    if (packageJson.enableProposedApi || packageJson.enabledApiProposals) {
      issues.push({
        severity: 'warning',
        category: 'api',
        message: 'Uses proposed APIs which may not be stable'
      });
      
      apiUsage.push({
        api: 'proposed',
        count: 1,
        supported: false
      });
    }

    // Check activation events
    if (packageJson.activationEvents) {
      for (const event of packageJson.activationEvents) {
        if (event.startsWith('onCommand:')) {
          const command = event.substring('onCommand:'.length);
          apiUsage.push({
            api: `command:${command}`,
            count: 1,
            supported: true
          });
        }
      }
    }
  }

  private checkContributions(
    contributes: any,
    issues: CompatibilityIssue[]
  ): void {
    // Check commands
    if (contributes.commands) {
      for (const command of contributes.commands) {
        if (!command.command || !command.title) {
          issues.push({
            severity: 'warning',
            category: 'contribution',
            message: 'Invalid command contribution'
          });
        }
      }
    }

    // Check views
    if (contributes.views) {
      for (const viewContainer in contributes.views) {
        if (!['explorer', 'scm', 'debug', 'test'].includes(viewContainer)) {
          issues.push({
            severity: 'info',
            category: 'contribution',
            message: `Custom view container: ${viewContainer}`
          });
        }
      }
    }

    // Check custom editors
    if (contributes.customEditors) {
      issues.push({
        severity: 'info',
        category: 'contribution',
        message: 'Extension provides custom editors'
      });
    }
  }

  private checkAPIUsage(
    extension: vscode.Extension<any>,
    apiUsage: APIUsage[],
    issues: CompatibilityIssue[]
  ): void {
    // This is a simplified check - in reality, we'd need to analyze the extension's code
    const exports = extension.exports;
    
    if (exports) {
      apiUsage.push({
        api: 'exports',
        count: 1,
        supported: true
      });
    }

    // Check for known incompatible APIs
    for (const api of this.knownIncompatibleAPIs) {
      // This would require actual code analysis
      // For now, we'll just flag if mentioned in package.json
      if (JSON.stringify(extension.packageJSON).includes(api)) {
        issues.push({
          severity: 'warning',
          category: 'api',
          message: `May use incompatible API: ${api}`
        });
      }
    }
  }

  private generateRecommendations(
    issues: CompatibilityIssue[],
    recommendations: string[]
  ): void {
    const hasErrors = issues.some(i => i.severity === 'error');
    const hasWarnings = issues.some(i => i.severity === 'warning');

    if (hasErrors) {
      recommendations.push('Extension has compatibility errors that need to be resolved');
    }

    if (hasWarnings) {
      recommendations.push('Review warnings and test extension functionality thoroughly');
    }

    if (issues.some(i => i.message.includes('proposed API'))) {
      recommendations.push('Consider creating a compatibility layer for proposed APIs');
    }

    if (!hasErrors && !hasWarnings) {
      recommendations.push('Extension appears to be fully compatible');
    }
  }

  private isVersionCompatible(current: string, required: string): boolean {
    // Simplified version check - in production, use semver
    const parseVersion = (v: string) => {
      const match = v.match(/(\d+)\.(\d+)\.(\d+)/);
      if (!match) return { major: 0, minor: 0, patch: 0 };
      return {
        major: parseInt(match[1]),
        minor: parseInt(match[2]),
        patch: parseInt(match[3])
      };
    };

    const currentVer = parseVersion(current);
    const requiredVer = parseVersion(required.replace(/[\^~]/, ''));

    if (currentVer.major < requiredVer.major) return false;
    if (currentVer.major > requiredVer.major) return true;
    if (currentVer.minor < requiredVer.minor) return false;
    if (currentVer.minor > requiredVer.minor) return true;
    return currentVer.patch >= requiredVer.patch;
  }

  async generateReport(results: ExtensionCompatibilityResult[]): Promise<string> {
    const report = {
      timestamp: new Date().toISOString(),
      totalExtensions: results.length,
      compatible: results.filter(r => r.compatible).length,
      incompatible: results.filter(r => !r.compatible).length,
      criticalIssues: results.filter(r => 
        r.issues.some(i => i.severity === 'error')
      ).length,
      results: results
    };

    const reportPath = path.join(__dirname, 'compatibility-report.json');
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));

    return reportPath;
  }
}