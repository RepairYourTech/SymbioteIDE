/**
 * UX Optimization and Validation System for SymbioteIDE
 * Ensures optimal user experience across all components
 */

import React from 'react';

export interface UXMetrics {
  loadTime: number;
  interactionLatency: number;
  memoryUsage: number;
  errorRate: number;
  accessibilityScore: number;
  userSatisfactionScore: number;
}

export interface UXValidationResult {
  isValid: boolean;
  score: number;
  issues: UXIssue[];
  recommendations: UXRecommendation[];
}

export interface UXIssue {
  severity: 'low' | 'medium' | 'high' | 'critical';
  category: 'performance' | 'accessibility' | 'usability' | 'visual' | 'interaction';
  description: string;
  component: string;
  fix?: string;
}

export interface UXRecommendation {
  priority: 'low' | 'medium' | 'high';
  category: string;
  description: string;
  impact: string;
  effort: 'low' | 'medium' | 'high';
}

export class UXOptimizer {
  private metrics: UXMetrics = {
    loadTime: 0,
    interactionLatency: 0,
    memoryUsage: 0,
    errorRate: 0,
    accessibilityScore: 0,
    userSatisfactionScore: 0
  };

  private performanceObserver: PerformanceObserver | null = null;
  private interactionTimes: number[] = [];
  private errorCount = 0;
  private totalInteractions = 0;

  constructor() {
    this.initializePerformanceMonitoring();
    this.initializeAccessibilityMonitoring();
    this.initializeErrorTracking();
  }

  /**
   * Initialize performance monitoring
   */
  private initializePerformanceMonitoring(): void {
    if (typeof window !== 'undefined' && 'PerformanceObserver' in window) {
      this.performanceObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry) => {
          if (entry.entryType === 'navigation') {
            this.metrics.loadTime = entry.duration;
          } else if (entry.entryType === 'measure') {
            this.interactionTimes.push(entry.duration);
            this.updateInteractionLatency();
          }
        });
      });

      this.performanceObserver.observe({ 
        entryTypes: ['navigation', 'measure', 'paint'] 
      });
    }
  }

  /**
   * Initialize accessibility monitoring
   */
  private initializeAccessibilityMonitoring(): void {
    // Monitor for accessibility violations
    this.checkAccessibilityCompliance();
    
    // Set up periodic accessibility checks
    setInterval(() => {
      this.checkAccessibilityCompliance();
    }, 30000); // Check every 30 seconds
  }

  /**
   * Initialize error tracking
   */
  private initializeErrorTracking(): void {
    if (typeof window !== 'undefined') {
      window.addEventListener('error', (event) => {
        this.errorCount++;
        this.updateErrorRate();
        console.error('UX Error tracked:', event.error);
      });

      window.addEventListener('unhandledrejection', (event) => {
        this.errorCount++;
        this.updateErrorRate();
        console.error('UX Promise rejection tracked:', event.reason);
      });
    }
  }

  /**
   * Track user interaction timing
   */
  public trackInteraction(name: string, startTime: number): void {
    const endTime = performance.now();
    const duration = endTime - startTime;
    
    performance.mark(`${name}-start`);
    performance.mark(`${name}-end`);
    performance.measure(name, `${name}-start`, `${name}-end`);
    
    this.totalInteractions++;
    this.interactionTimes.push(duration);
    this.updateInteractionLatency();
  }

  /**
   * Update interaction latency metrics
   */
  private updateInteractionLatency(): void {
    if (this.interactionTimes.length > 0) {
      const sum = this.interactionTimes.reduce((a, b) => a + b, 0);
      this.metrics.interactionLatency = sum / this.interactionTimes.length;
    }
  }

  /**
   * Update error rate metrics
   */
  private updateErrorRate(): void {
    if (this.totalInteractions > 0) {
      this.metrics.errorRate = (this.errorCount / this.totalInteractions) * 100;
    }
  }

  /**
   * Check accessibility compliance
   */
  private checkAccessibilityCompliance(): void {
    let score = 100;
    const issues: string[] = [];

    // Check for missing alt text
    const images = document.querySelectorAll('img:not([alt])');
    if (images.length > 0) {
      score -= 10;
      issues.push(`${images.length} images missing alt text`);
    }

    // Check for missing form labels
    const inputs = document.querySelectorAll('input:not([aria-label]):not([aria-labelledby])');
    const unlabeledInputs = Array.from(inputs).filter(input => {
      const labels = document.querySelectorAll(`label[for="${input.id}"]`);
      return labels.length === 0;
    });
    if (unlabeledInputs.length > 0) {
      score -= 15;
      issues.push(`${unlabeledInputs.length} form inputs missing labels`);
    }

    // Check for proper heading hierarchy
    const headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
    let previousLevel = 0;
    let hierarchyIssues = 0;
    headings.forEach((heading) => {
      const level = parseInt(heading.tagName.charAt(1));
      if (level > previousLevel + 1) {
        hierarchyIssues++;
      }
      previousLevel = level;
    });
    if (hierarchyIssues > 0) {
      score -= 5;
      issues.push(`${hierarchyIssues} heading hierarchy violations`);
    }

    // Check for sufficient color contrast
    const elements = document.querySelectorAll('*');
    let contrastIssues = 0;
    elements.forEach((element) => {
      const styles = window.getComputedStyle(element);
      const color = styles.color;
      const backgroundColor = styles.backgroundColor;
      
      // Simple contrast check (would need more sophisticated implementation)
      if (color && backgroundColor && color !== 'rgba(0, 0, 0, 0)' && backgroundColor !== 'rgba(0, 0, 0, 0)') {
        // Simplified contrast ratio check
        if (this.getContrastRatio(color, backgroundColor) < 4.5) {
          contrastIssues++;
        }
      }
    });
    if (contrastIssues > 10) { // Only penalize if many issues
      score -= 10;
      issues.push(`${contrastIssues} potential color contrast issues`);
    }

    // Check for keyboard accessibility
    const focusableElements = document.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    let keyboardIssues = 0;
    focusableElements.forEach((element) => {
      const tabIndex = element.getAttribute('tabindex');
      if (tabIndex && parseInt(tabIndex) > 0) {
        keyboardIssues++;
      }
    });
    if (keyboardIssues > 0) {
      score -= 5;
      issues.push(`${keyboardIssues} positive tabindex values (anti-pattern)`);
    }

    this.metrics.accessibilityScore = Math.max(0, score);
  }

  /**
   * Simple contrast ratio calculation
   */
  private getContrastRatio(color1: string, color2: string): number {
    // Simplified implementation - would need proper color parsing
    // This is a placeholder for demonstration
    return 4.5; // Assume acceptable contrast for now
  }

  /**
   * Update memory usage metrics
   */
  public updateMemoryUsage(): void {
    if ('memory' in performance) {
      const memory = (performance as any).memory;
      this.metrics.memoryUsage = memory.usedJSHeapSize / (1024 * 1024); // MB
    }
  }

  /**
   * Validate overall UX quality
   */
  public validateUX(): UXValidationResult {
    this.updateMemoryUsage();
    
    const issues: UXIssue[] = [];
    const recommendations: UXRecommendation[] = [];
    let score = 100;

    // Performance validation
    if (this.metrics.loadTime > 3000) {
      score -= 20;
      issues.push({
        severity: 'high',
        category: 'performance',
        description: `Load time (${this.metrics.loadTime.toFixed(0)}ms) exceeds 3 seconds`,
        component: 'Application',
        fix: 'Optimize bundle size and implement code splitting'
      });
    }

    if (this.metrics.interactionLatency > 100) {
      score -= 15;
      issues.push({
        severity: 'medium',
        category: 'performance',
        description: `Average interaction latency (${this.metrics.interactionLatency.toFixed(0)}ms) exceeds 100ms`,
        component: 'UI Components',
        fix: 'Optimize event handlers and reduce re-renders'
      });
    }

    if (this.metrics.memoryUsage > 500) {
      score -= 10;
      issues.push({
        severity: 'medium',
        category: 'performance',
        description: `Memory usage (${this.metrics.memoryUsage.toFixed(0)}MB) exceeds 500MB`,
        component: 'Application',
        fix: 'Implement memory cleanup and optimize data structures'
      });
    }

    // Error rate validation
    if (this.metrics.errorRate > 1) {
      score -= 25;
      issues.push({
        severity: 'critical',
        category: 'usability',
        description: `Error rate (${this.metrics.errorRate.toFixed(1)}%) exceeds 1%`,
        component: 'Application',
        fix: 'Implement better error handling and user feedback'
      });
    }

    // Accessibility validation
    if (this.metrics.accessibilityScore < 90) {
      score -= 15;
      issues.push({
        severity: 'high',
        category: 'accessibility',
        description: `Accessibility score (${this.metrics.accessibilityScore}) below 90`,
        component: 'UI Components',
        fix: 'Address accessibility violations and improve ARIA support'
      });
    }

    // Generate recommendations
    if (this.metrics.loadTime > 2000) {
      recommendations.push({
        priority: 'high',
        category: 'Performance',
        description: 'Implement lazy loading for non-critical components',
        impact: 'Reduce initial load time by 30-50%',
        effort: 'medium'
      });
    }

    if (this.metrics.accessibilityScore < 95) {
      recommendations.push({
        priority: 'high',
        category: 'Accessibility',
        description: 'Conduct comprehensive accessibility audit',
        impact: 'Improve accessibility compliance and user experience',
        effort: 'medium'
      });
    }

    if (this.interactionTimes.length > 100) {
      recommendations.push({
        priority: 'medium',
        category: 'Performance',
        description: 'Implement interaction analytics dashboard',
        impact: 'Better visibility into user experience metrics',
        effort: 'low'
      });
    }

    return {
      isValid: score >= 80,
      score: Math.max(0, score),
      issues,
      recommendations
    };
  }

  /**
   * Get current UX metrics
   */
  public getMetrics(): UXMetrics {
    this.updateMemoryUsage();
    return { ...this.metrics };
  }

  /**
   * Generate UX report
   */
  public generateReport(): string {
    const validation = this.validateUX();
    const metrics = this.getMetrics();

    let report = `# SymbioteIDE UX Quality Report\n\n`;
    report += `**Overall Score:** ${validation.score}/100 ${validation.isValid ? '✅' : '❌'}\n\n`;
    
    report += `## Performance Metrics\n`;
    report += `- **Load Time:** ${metrics.loadTime.toFixed(0)}ms\n`;
    report += `- **Interaction Latency:** ${metrics.interactionLatency.toFixed(0)}ms\n`;
    report += `- **Memory Usage:** ${metrics.memoryUsage.toFixed(0)}MB\n`;
    report += `- **Error Rate:** ${metrics.errorRate.toFixed(1)}%\n`;
    report += `- **Accessibility Score:** ${metrics.accessibilityScore}/100\n\n`;

    if (validation.issues.length > 0) {
      report += `## Issues Found\n`;
      validation.issues.forEach((issue, index) => {
        report += `${index + 1}. **${issue.severity.toUpperCase()}** (${issue.category}): ${issue.description}\n`;
        if (issue.fix) {
          report += `   *Fix:* ${issue.fix}\n`;
        }
        report += `\n`;
      });
    }

    if (validation.recommendations.length > 0) {
      report += `## Recommendations\n`;
      validation.recommendations.forEach((rec, index) => {
        report += `${index + 1}. **${rec.priority.toUpperCase()}** - ${rec.description}\n`;
        report += `   *Impact:* ${rec.impact}\n`;
        report += `   *Effort:* ${rec.effort}\n\n`;
      });
    }

    return report;
  }

  /**
   * Cleanup resources
   */
  public destroy(): void {
    if (this.performanceObserver) {
      this.performanceObserver.disconnect();
    }
  }
}

// Global UX optimizer instance
export const uxOptimizer = new UXOptimizer();

// Helper functions for component integration
export const trackComponentInteraction = (componentName: string) => {
  const startTime = performance.now();
  return () => uxOptimizer.trackInteraction(componentName, startTime);
};

export const validateComponentUX = (componentName: string): UXValidationResult => {
  // Component-specific validation logic
  return uxOptimizer.validateUX();
};

export const optimizeComponentPerformance = (component: React.ComponentType) => {
  // HOC for performance optimization
  return React.memo(component);
};
