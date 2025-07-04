/**
 * Metrics Collector for A/B Testing
 */

import {
  MetricConfig,
  MetricType,
  MetricResult,
  AggregationType,
  ExperimentSample,
  ConfidenceInterval
} from './experiment';
import { AITask, TaskResult } from '../interfaces';

export class MetricsCollector {
  /**
   * Collect metrics from a task result
   */
  collectMetrics(
    task: AITask,
    result: TaskResult,
    metricConfigs: MetricConfig[]
  ): Record<string, number> {
    const metrics: Record<string, number> = {};

    for (const config of metricConfigs) {
      const value = this.extractMetricValue(task, result, config);
      if (value !== null) {
        metrics[config.id] = value;
      }
    }

    return metrics;
  }

  /**
   * Extract metric value based on type
   */
  private extractMetricValue(
    task: AITask,
    result: TaskResult,
    config: MetricConfig
  ): number | null {
    switch (config.type) {
      case MetricType.Latency:
        return result.latency;
      
      case MetricType.Cost:
        return result.cost.amount;
      
      case MetricType.TokenUsage:
        return result.usage.totalTokens;
      
      case MetricType.SuccessRate:
        return result.status === 'success' ? 1 : 0;
      
      case MetricType.UserSatisfaction:
        // This would need to be provided in metadata
        return result.metadata?.userSatisfaction || null;
      
      case MetricType.OutputQuality:
        // This would need custom evaluation
        return result.metadata?.qualityScore || null;
      
      case MetricType.CustomNumeric:
        return result.metadata?.[config.id] || null;
      
      case MetricType.CustomBoolean:
        const boolValue = result.metadata?.[config.id];
        return boolValue !== undefined ? (boolValue ? 1 : 0) : null;
      
      default:
        return null;
    }
  }

  /**
   * Aggregate metrics from samples
   */
  aggregateMetrics(
    samples: ExperimentSample[],
    metricConfigs: MetricConfig[]
  ): MetricResult[] {
    const results: MetricResult[] = [];

    for (const config of metricConfigs) {
      const values = samples
        .map(s => s.metrics[config.id])
        .filter(v => v !== undefined && v !== null);
      
      if (values.length === 0) {
        continue;
      }

      const result = this.calculateAggregation(values, config);
      results.push(result);
    }

    return results;
  }

  /**
   * Calculate aggregation based on type
   */
  private calculateAggregation(
    values: number[],
    config: MetricConfig
  ): MetricResult {
    let value: number;
    let standardDeviation: number | undefined;
    let confidenceInterval: ConfidenceInterval | undefined;

    switch (config.aggregation) {
      case AggregationType.Average:
        value = this.mean(values);
        standardDeviation = this.standardDeviation(values);
        confidenceInterval = this.calculateConfidenceInterval(
          values,
          config.confidenceLevel || 0.95
        );
        break;
      
      case AggregationType.Median:
        value = this.median(values);
        break;
      
      case AggregationType.Percentile:
        // Default to 95th percentile if not specified
        const percentile = config.metadata?.percentile || 95;
        value = this.percentile(values, percentile);
        break;
      
      case AggregationType.Sum:
        value = values.reduce((a, b) => a + b, 0);
        break;
      
      case AggregationType.Count:
        value = values.length;
        break;
      
      case AggregationType.Rate:
        const successes = values.filter(v => v > 0).length;
        value = successes / values.length;
        standardDeviation = Math.sqrt(value * (1 - value) / values.length);
        confidenceInterval = this.calculateProportionCI(
          successes,
          values.length,
          config.confidenceLevel || 0.95
        );
        break;
      
      default:
        value = this.mean(values);
    }

    return {
      metricId: config.id,
      metricName: config.name,
      value,
      standardDeviation,
      confidenceInterval,
      samples: values.length,
      outliers: this.countOutliers(values)
    };
  }

  /**
   * Calculate mean
   */
  private mean(values: number[]): number {
    return values.reduce((a, b) => a + b, 0) / values.length;
  }

  /**
   * Calculate median
   */
  private median(values: number[]): number {
    const sorted = [...values].sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    
    if (sorted.length % 2 === 0) {
      return (sorted[mid - 1] + sorted[mid]) / 2;
    }
    
    return sorted[mid];
  }

  /**
   * Calculate percentile
   */
  private percentile(values: number[], p: number): number {
    const sorted = [...values].sort((a, b) => a - b);
    const index = (p / 100) * (sorted.length - 1);
    const lower = Math.floor(index);
    const upper = Math.ceil(index);
    const weight = index % 1;
    
    if (lower === upper) {
      return sorted[lower];
    }
    
    return sorted[lower] * (1 - weight) + sorted[upper] * weight;
  }

  /**
   * Calculate standard deviation
   */
  private standardDeviation(values: number[]): number {
    const avg = this.mean(values);
    const squaredDiffs = values.map(v => Math.pow(v - avg, 2));
    const variance = this.mean(squaredDiffs);
    return Math.sqrt(variance);
  }

  /**
   * Calculate confidence interval
   */
  private calculateConfidenceInterval(
    values: number[],
    confidence: number
  ): ConfidenceInterval {
    const n = values.length;
    const mean = this.mean(values);
    const stdDev = this.standardDeviation(values);
    const standardError = stdDev / Math.sqrt(n);
    
    // Use t-distribution for small samples
    const tValue = this.getTValue(n - 1, confidence);
    const margin = tValue * standardError;
    
    return {
      lower: mean - margin,
      upper: mean + margin,
      confidence
    };
  }

  /**
   * Calculate confidence interval for proportions
   */
  private calculateProportionCI(
    successes: number,
    total: number,
    confidence: number
  ): ConfidenceInterval {
    const p = successes / total;
    const z = this.getZScore(confidence);
    const standardError = Math.sqrt(p * (1 - p) / total);
    const margin = z * standardError;
    
    return {
      lower: Math.max(0, p - margin),
      upper: Math.min(1, p + margin),
      confidence
    };
  }

  /**
   * Count outliers using IQR method
   */
  private countOutliers(values: number[]): number {
    if (values.length < 4) return 0;
    
    const q1 = this.percentile(values, 25);
    const q3 = this.percentile(values, 75);
    const iqr = q3 - q1;
    const lowerBound = q1 - 1.5 * iqr;
    const upperBound = q3 + 1.5 * iqr;
    
    return values.filter(v => v < lowerBound || v > upperBound).length;
  }

  /**
   * Get t-value for confidence interval
   */
  private getTValue(df: number, confidence: number): number {
    // Simplified t-value lookup for common confidence levels
    // In production, use a proper t-distribution library
    const alpha = 1 - confidence;
    
    if (df >= 30) {
      // Use z-score for large samples
      return this.getZScore(confidence);
    }
    
    // Common t-values for 95% confidence
    const t95: Record<number, number> = {
      1: 12.706,
      2: 4.303,
      3: 3.182,
      4: 2.776,
      5: 2.571,
      10: 2.228,
      20: 2.086,
      30: 2.042
    };
    
    return t95[df] || 2.0;
  }

  /**
   * Get z-score for confidence level
   */
  private getZScore(confidence: number): number {
    // Common z-scores
    const zScores: Record<number, number> = {
      0.90: 1.645,
      0.95: 1.96,
      0.99: 2.576
    };
    
    return zScores[confidence] || 1.96;
  }

  /**
   * Compare two metric results
   */
  compareMetrics(
    metricA: MetricResult,
    metricB: MetricResult,
    config: MetricConfig
  ): {
    difference: number;
    percentChange: number;
    significant: boolean;
    pValue?: number;
  } {
    const difference = metricA.value - metricB.value;
    const percentChange = (difference / metricB.value) * 100;
    
    // Check if confidence intervals overlap
    let significant = false;
    if (metricA.confidenceInterval && metricB.confidenceInterval) {
      significant = metricA.confidenceInterval.lower > metricB.confidenceInterval.upper ||
                   metricB.confidenceInterval.lower > metricA.confidenceInterval.upper;
    }
    
    return {
      difference,
      percentChange,
      significant
    };
  }
}