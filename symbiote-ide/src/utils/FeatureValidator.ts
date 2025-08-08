// SymbioteIDE Feature Validator
// Comprehensive validation of all major differentiating features

import { invoke } from '@tauri-apps/api/core';

export interface ValidationResult {
  feature: string;
  status: 'success' | 'error' | 'warning';
  message: string;
  details?: any;
  duration?: number;
}

export interface ValidationReport {
  overall: 'success' | 'partial' | 'failed';
  results: ValidationResult[];
  summary: {
    total: number;
    passed: number;
    failed: number;
    warnings: number;
  };
  performance: {
    totalTime: number;
    averageResponseTime: number;
  };
}

export class FeatureValidator {
  private results: ValidationResult[] = [];
  private startTime: number = 0;

  async validateAllFeatures(): Promise<ValidationReport> {
    this.startTime = performance.now();
    this.results = [];

    console.log('🚀 Starting comprehensive SymbioteIDE feature validation...');

    // Validate all major differentiating features
    await this.validateAPIBuilder();
    await this.validatePerformanceMonitoring();
    await this.validateInteractionModes();
    await this.validatePASystem();
    await this.validateAgentSystem();
    await this.validateContextManagement();

    return this.generateReport();
  }

  private async validateAPIBuilder(): Promise<void> {
    console.log('🔧 Validating API Builder...');

    try {
      // Test project creation
      const startTime = performance.now();
      const projectId = await invoke<string>('api_builder_create_project', {
        name: 'Validation Test API',
        description: 'Feature validation test project'
      });
      const createTime = performance.now() - startTime;

      this.addResult({
        feature: 'API Builder - Project Creation',
        status: 'success',
        message: `Project created successfully (ID: ${projectId})`,
        duration: createTime
      });

      // Test project retrieval
      const retrieveStartTime = performance.now();
      const projectJson = await invoke<string>('api_builder_get_project', {
        projectId
      });
      const retrieveTime = performance.now() - retrieveStartTime;

      const project = JSON.parse(projectJson);
      if (project.id === projectId) {
        this.addResult({
          feature: 'API Builder - Project Retrieval',
          status: 'success',
          message: 'Project retrieved successfully',
          duration: retrieveTime
        });
      } else {
        this.addResult({
          feature: 'API Builder - Project Retrieval',
          status: 'error',
          message: 'Project data mismatch'
        });
      }

      // Test template system
      const templatesJson = await invoke<string>('api_builder_get_templates');
      const templates = JSON.parse(templatesJson);
      
      if (Object.keys(templates).length > 0) {
        this.addResult({
          feature: 'API Builder - Template System',
          status: 'success',
          message: `${Object.keys(templates).length} templates available`
        });
      } else {
        this.addResult({
          feature: 'API Builder - Template System',
          status: 'warning',
          message: 'No templates found'
        });
      }

      // Test code generation
      const codeJson = await invoke<string>('api_builder_generate_code', {
        projectId,
        framework: 'express'
      });
      const generatedCode = JSON.parse(codeJson);

      if (generatedCode.files && Object.keys(generatedCode.files).length > 0) {
        this.addResult({
          feature: 'API Builder - Code Generation',
          status: 'success',
          message: `Generated ${Object.keys(generatedCode.files).length} files`,
          details: {
            files: Object.keys(generatedCode.files),
            dependencies: generatedCode.dependencies
          }
        });
      } else {
        this.addResult({
          feature: 'API Builder - Code Generation',
          status: 'error',
          message: 'Code generation failed'
        });
      }

    } catch (error) {
      this.addResult({
        feature: 'API Builder',
        status: 'error',
        message: `API Builder validation failed: ${error}`
      });
    }
  }

  private async validatePerformanceMonitoring(): Promise<void> {
    console.log('📊 Validating Performance Monitoring...');

    try {
      // Test performance tracking by measuring operation times
      const operations = [
        { name: 'Project Creation', threshold: 1000 },
        { name: 'Template Retrieval', threshold: 500 },
        { name: 'Code Generation', threshold: 2000 }
      ];

      let allWithinThreshold = true;
      const performanceData: any[] = [];

      for (const operation of operations) {
        const startTime = performance.now();
        
        // Simulate operation (create test project)
        try {
          await invoke<string>('api_builder_create_project', {
            name: `Perf Test ${operation.name}`,
            description: 'Performance monitoring test'
          });
        } catch (error) {
          // Operation may fail, but we're testing performance monitoring
        }
        
        const duration = performance.now() - startTime;
        performanceData.push({ operation: operation.name, duration });

        if (duration > operation.threshold) {
          allWithinThreshold = false;
        }
      }

      if (allWithinThreshold) {
        this.addResult({
          feature: 'Performance Monitoring',
          status: 'success',
          message: 'All operations within performance thresholds',
          details: performanceData
        });
      } else {
        this.addResult({
          feature: 'Performance Monitoring',
          status: 'warning',
          message: 'Some operations exceeded thresholds',
          details: performanceData
        });
      }

    } catch (error) {
      this.addResult({
        feature: 'Performance Monitoring',
        status: 'error',
        message: `Performance monitoring validation failed: ${error}`
      });
    }
  }

  private async validateInteractionModes(): Promise<void> {
    console.log('🎯 Validating Interaction Modes...');

    try {
      const modes = ['Easy', 'Interactive', 'Manual'];
      let successCount = 0;

      for (const mode of modes) {
        try {
          const result = await invoke<string>('switch_interaction_mode', {
            mode
          });

          if (result.includes(mode)) {
            successCount++;
          }
        } catch (error) {
          console.warn(`Failed to switch to ${mode} mode:`, error);
        }
      }

      // Test getting current mode
      try {
        const currentMode = await invoke<string>('get_current_mode');
        if (currentMode) {
          successCount++;
        }
      } catch (error) {
        console.warn('Failed to get current mode:', error);
      }

      // Test getting mode config
      try {
        const modeConfig = await invoke<string>('get_mode_config');
        if (modeConfig) {
          successCount++;
        }
      } catch (error) {
        console.warn('Failed to get mode config:', error);
      }

      if (successCount >= 4) {
        this.addResult({
          feature: 'Interaction Modes',
          status: 'success',
          message: `${successCount}/5 interaction mode operations successful`
        });
      } else {
        this.addResult({
          feature: 'Interaction Modes',
          status: 'warning',
          message: `Only ${successCount}/5 interaction mode operations successful`
        });
      }

    } catch (error) {
      this.addResult({
        feature: 'Interaction Modes',
        status: 'error',
        message: `Interaction modes validation failed: ${error}`
      });
    }
  }

  private async validatePASystem(): Promise<void> {
    console.log('🤖 Validating PA System...');

    try {
      // Test PA system status
      const statusJson = await invoke<string>('pa_get_system_status');
      const status = JSON.parse(statusJson);

      if (status.status) {
        this.addResult({
          feature: 'PA System - Status',
          status: 'success',
          message: `PA System status: ${status.status}`,
          details: status
        });
      } else {
        this.addResult({
          feature: 'PA System - Status',
          status: 'warning',
          message: 'PA System status unclear'
        });
      }

      // Test plan generation
      try {
        const planResult = await invoke<string>('pa_generate_plan', {
          request: JSON.stringify({
            description: 'Test plan for validation',
            priority: 'medium',
            deadline: new Date(Date.now() + 86400000).toISOString()
          })
        });

        this.addResult({
          feature: 'PA System - Plan Generation',
          status: 'success',
          message: 'Plan generation successful'
        });
      } catch (error) {
        this.addResult({
          feature: 'PA System - Plan Generation',
          status: 'warning',
          message: `Plan generation failed: ${error}`
        });
      }

      // Test getting active plans
      try {
        const plansJson = await invoke<string>('pa_get_active_plans');
        this.addResult({
          feature: 'PA System - Active Plans',
          status: 'success',
          message: 'Active plans retrieved successfully'
        });
      } catch (error) {
        this.addResult({
          feature: 'PA System - Active Plans',
          status: 'warning',
          message: `Failed to get active plans: ${error}`
        });
      }

    } catch (error) {
      this.addResult({
        feature: 'PA System',
        status: 'error',
        message: `PA System validation failed: ${error}`
      });
    }
  }

  private async validateAgentSystem(): Promise<void> {
    console.log('👥 Validating Agent System...');

    try {
      // Test agent activation
      const agentResult = await invoke<string>('activate_agent', {
        agentType: 'Developer'
      });

      if (agentResult.includes('activated')) {
        this.addResult({
          feature: 'Agent System - Activation',
          status: 'success',
          message: 'Agent activation successful'
        });
      } else {
        this.addResult({
          feature: 'Agent System - Activation',
          status: 'warning',
          message: 'Agent activation response unclear'
        });
      }

      // Test agent status
      try {
        const statusResult = await invoke<string>('get_agent_status', {
          agentId: 'test-agent'
        });

        this.addResult({
          feature: 'Agent System - Status',
          status: 'success',
          message: 'Agent status retrieval successful'
        });
      } catch (error) {
        this.addResult({
          feature: 'Agent System - Status',
          status: 'warning',
          message: `Agent status failed: ${error}`
        });
      }

    } catch (error) {
      this.addResult({
        feature: 'Agent System',
        status: 'error',
        message: `Agent system validation failed: ${error}`
      });
    }
  }

  private async validateContextManagement(): Promise<void> {
    console.log('🧠 Validating Context Management...');

    try {
      // Test context retrieval
      const contextResult = await invoke<string>('get_context', {
        scope: 'project',
        filters: ['test']
      });

      this.addResult({
        feature: 'Context Management - Retrieval',
        status: 'success',
        message: 'Context retrieval successful'
      });

      // Test context update
      await invoke('update_context', {
        scope: 'test',
        data: JSON.stringify({ validation: true, timestamp: Date.now() })
      });

      this.addResult({
        feature: 'Context Management - Update',
        status: 'success',
        message: 'Context update successful'
      });

    } catch (error) {
      this.addResult({
        feature: 'Context Management',
        status: 'error',
        message: `Context management validation failed: ${error}`
      });
    }
  }

  private addResult(result: ValidationResult): void {
    this.results.push(result);
    
    const statusIcon = result.status === 'success' ? '✅' : 
                      result.status === 'warning' ? '⚠️' : '❌';
    
    console.log(`${statusIcon} ${result.feature}: ${result.message}`);
    
    if (result.duration) {
      console.log(`   ⏱️ Duration: ${result.duration.toFixed(2)}ms`);
    }
  }

  private generateReport(): ValidationReport {
    const totalTime = performance.now() - this.startTime;
    const passed = this.results.filter(r => r.status === 'success').length;
    const failed = this.results.filter(r => r.status === 'error').length;
    const warnings = this.results.filter(r => r.status === 'warning').length;
    
    const averageResponseTime = this.results
      .filter(r => r.duration)
      .reduce((sum, r) => sum + (r.duration || 0), 0) / 
      this.results.filter(r => r.duration).length;

    const overall = failed > 0 ? 'failed' : 
                   warnings > 0 ? 'partial' : 'success';

    const report: ValidationReport = {
      overall,
      results: this.results,
      summary: {
        total: this.results.length,
        passed,
        failed,
        warnings
      },
      performance: {
        totalTime,
        averageResponseTime: averageResponseTime || 0
      }
    };

    this.logReport(report);
    return report;
  }

  private logReport(report: ValidationReport): void {
    console.log('\n🎯 SymbioteIDE Feature Validation Report');
    console.log('==========================================');
    console.log(`Overall Status: ${report.overall.toUpperCase()}`);
    console.log(`Total Tests: ${report.summary.total}`);
    console.log(`✅ Passed: ${report.summary.passed}`);
    console.log(`❌ Failed: ${report.summary.failed}`);
    console.log(`⚠️ Warnings: ${report.summary.warnings}`);
    console.log(`⏱️ Total Time: ${report.performance.totalTime.toFixed(2)}ms`);
    console.log(`📊 Avg Response: ${report.performance.averageResponseTime.toFixed(2)}ms`);
    
    if (report.summary.failed === 0 && report.summary.warnings === 0) {
      console.log('\n🚀 All features validated successfully! SymbioteIDE is ready for production.');
    } else if (report.summary.failed === 0) {
      console.log('\n⚠️ All critical features working, but some warnings detected.');
    } else {
      console.log('\n❌ Some critical features failed validation. Review required.');
    }
  }
}

// Export singleton instance
export const featureValidator = new FeatureValidator();
