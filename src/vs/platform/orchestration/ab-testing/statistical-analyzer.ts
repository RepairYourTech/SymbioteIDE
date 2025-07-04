/**
 * Statistical Analyzer for A/B Testing
 */

import {
  VariantResult,
  MetricResult,
  MetricConfig,
  StatisticalAnalysis,
  StatisticalTest,
  PowerAnalysis,
  MetricType
} from './experiment';

export class StatisticalAnalyzer {
  /**
   * Perform statistical analysis on experiment results
   */
  analyze(
    variants: VariantResult[],
    metricConfigs: MetricConfig[]
  ): StatisticalAnalysis | undefined {
    if (variants.length < 2) {
      return undefined;
    }

    // Find primary metric
    const primaryMetricConfig = metricConfigs.find(m => m.metadata?.isPrimary) || metricConfigs[0];
    const tests: StatisticalTest[] = [];

    // Perform tests for each metric
    for (const config of metricConfigs) {
      const metricResults = variants.map(v => 
        v.metrics.find(m => m.metricId === config.id)
      ).filter(Boolean) as MetricResult[];

      if (metricResults.length < 2) continue;

      const test = this.performStatisticalTest(metricResults, config);
      if (test) {
        tests.push(test);
      }
    }

    // Determine winner based on primary metric
    const primaryTest = tests.find(t => t.metric === primaryMetricConfig.id);
    let winner: string | undefined;
    let confidence: number | undefined;
    let effectSize: number | undefined;

    if (primaryTest && primaryTest.significant) {
      const primaryResults = variants.map(v => ({
        variantId: v.variantId,
        metric: v.metrics.find(m => m.metricId === primaryMetricConfig.id)
      })).filter(r => r.metric) as Array<{ variantId: string; metric: MetricResult }>;

      // Find best performing variant
      const sorted = primaryResults.sort((a, b) => {
        const diff = a.metric.value - b.metric.value;
        return primaryMetricConfig.higherIsBetter ? -diff : diff;
      });

      if (sorted.length > 0) {
        winner = sorted[0].variantId;
        confidence = 1 - (primaryTest.pValue || 0.05);
        
        // Calculate effect size
        if (sorted.length >= 2) {
          effectSize = this.calculateEffectSize(
            sorted[0].metric,
            sorted[1].metric
          );
        }
      }
    }

    // Perform power analysis
    const powerAnalysis = this.performPowerAnalysis(
      variants,
      primaryMetricConfig
    );

    return {
      primaryMetric: primaryMetricConfig.id,
      winner,
      confidence,
      pValue: primaryTest?.pValue,
      effectSize,
      powerAnalysis,
      tests
    };
  }

  /**
   * Perform appropriate statistical test
   */
  private performStatisticalTest(
    results: MetricResult[],
    config: MetricConfig
  ): StatisticalTest | null {
    // Skip if insufficient samples
    const minSamples = config.minimumSampleSize || 30;
    if (results.some(r => r.samples < minSamples)) {
      return null;
    }

    // Choose test based on metric type and data
    if (config.type === MetricType.SuccessRate || config.aggregation === 'rate') {
      return this.performProportionTest(results, config);
    } else if (results.length === 2) {
      return this.performTTest(results[0], results[1], config);
    } else {
      return this.performANOVA(results, config);
    }
  }

  /**
   * Perform two-sample t-test
   */
  private performTTest(
    resultA: MetricResult,
    resultB: MetricResult,
    config: MetricConfig
  ): StatisticalTest {
    // Calculate test statistic
    const meanDiff = resultA.value - resultB.value;
    const pooledVariance = this.calculatePooledVariance(
      resultA,
      resultB
    );
    const standardError = Math.sqrt(
      pooledVariance * (1 / resultA.samples + 1 / resultB.samples)
    );
    const tStatistic = meanDiff / standardError;
    const df = resultA.samples + resultB.samples - 2;

    // Calculate p-value (simplified)
    const pValue = this.calculatePValue(Math.abs(tStatistic), df);

    return {
      name: 'Two-Sample T-Test',
      metric: config.id,
      testType: 't-test',
      pValue,
      statistic: tStatistic,
      degreesOfFreedom: df,
      significant: pValue < (config.confidenceLevel ? 1 - config.confidenceLevel : 0.05)
    };
  }

  /**
   * Perform ANOVA test
   */
  private performANOVA(
    results: MetricResult[],
    config: MetricConfig
  ): StatisticalTest {
    // Calculate overall mean
    const totalSamples = results.reduce((sum, r) => sum + r.samples, 0);
    const overallMean = results.reduce((sum, r) => sum + r.value * r.samples, 0) / totalSamples;

    // Calculate between-group sum of squares
    const ssBetween = results.reduce((sum, r) => 
      sum + r.samples * Math.pow(r.value - overallMean, 2), 0
    );

    // Calculate within-group sum of squares (approximate)
    const ssWithin = results.reduce((sum, r) => {
      const variance = r.standardDeviation ? Math.pow(r.standardDeviation, 2) : 0;
      return sum + variance * (r.samples - 1);
    }, 0);

    // Calculate F-statistic
    const dfBetween = results.length - 1;
    const dfWithin = totalSamples - results.length;
    const msBetween = ssBetween / dfBetween;
    const msWithin = ssWithin / dfWithin;
    const fStatistic = msBetween / msWithin;

    // Calculate p-value (simplified)
    const pValue = this.calculateFPValue(fStatistic, dfBetween, dfWithin);

    return {
      name: 'One-Way ANOVA',
      metric: config.id,
      testType: 'anova',
      pValue,
      statistic: fStatistic,
      degreesOfFreedom: dfBetween,
      significant: pValue < (config.confidenceLevel ? 1 - config.confidenceLevel : 0.05)
    };
  }

  /**
   * Perform proportion test (Chi-square)
   */
  private performProportionTest(
    results: MetricResult[],
    config: MetricConfig
  ): StatisticalTest {
    // Build contingency table
    const successes = results.map(r => Math.round(r.value * r.samples));
    const failures = results.map((r, i) => r.samples - successes[i]);
    const totalSuccesses = successes.reduce((a, b) => a + b, 0);
    const totalFailures = failures.reduce((a, b) => a + b, 0);
    const totalSamples = results.reduce((sum, r) => sum + r.samples, 0);

    // Calculate expected values
    const expectedSuccesses = results.map(r => 
      (r.samples * totalSuccesses) / totalSamples
    );
    const expectedFailures = results.map(r => 
      (r.samples * totalFailures) / totalSamples
    );

    // Calculate chi-square statistic
    let chiSquare = 0;
    for (let i = 0; i < results.length; i++) {
      chiSquare += Math.pow(successes[i] - expectedSuccesses[i], 2) / expectedSuccesses[i];
      chiSquare += Math.pow(failures[i] - expectedFailures[i], 2) / expectedFailures[i];
    }

    const df = results.length - 1;
    const pValue = this.calculateChiSquarePValue(chiSquare, df);

    return {
      name: 'Chi-Square Test',
      metric: config.id,
      testType: 'chi-square',
      pValue,
      statistic: chiSquare,
      degreesOfFreedom: df,
      significant: pValue < (config.confidenceLevel ? 1 - config.confidenceLevel : 0.05)
    };
  }

  /**
   * Calculate effect size (Cohen's d)
   */
  private calculateEffectSize(
    resultA: MetricResult,
    resultB: MetricResult
  ): number {
    const meanDiff = Math.abs(resultA.value - resultB.value);
    const pooledStdDev = Math.sqrt(
      ((resultA.samples - 1) * Math.pow(resultA.standardDeviation || 0, 2) +
       (resultB.samples - 1) * Math.pow(resultB.standardDeviation || 0, 2)) /
      (resultA.samples + resultB.samples - 2)
    );
    
    return pooledStdDev > 0 ? meanDiff / pooledStdDev : 0;
  }

  /**
   * Perform power analysis
   */
  private performPowerAnalysis(
    variants: VariantResult[],
    primaryMetric: MetricConfig
  ): PowerAnalysis | undefined {
    if (variants.length < 2) return undefined;

    const results = variants.map(v => 
      v.metrics.find(m => m.metricId === primaryMetric.id)
    ).filter(Boolean) as MetricResult[];

    if (results.length < 2) return undefined;

    // Calculate current effect size
    const effectSize = this.calculateEffectSize(results[0], results[1]);
    
    // Estimate power (simplified)
    const averageSampleSize = results.reduce((sum, r) => sum + r.samples, 0) / results.length;
    const currentPower = this.estimatePower(effectSize, averageSampleSize);
    
    // Calculate required sample size for 80% power
    const targetPower = 0.8;
    const requiredSampleSize = this.calculateRequiredSampleSize(
      effectSize,
      targetPower,
      primaryMetric.confidenceLevel || 0.95
    );

    // Estimate duration based on current rate
    const currentRate = variants.reduce((sum, v) => sum + v.sampleSize, 0) / 
      variants.length; // per variant
    const additionalSamplesNeeded = Math.max(0, requiredSampleSize - averageSampleSize);
    const expectedDuration = additionalSamplesNeeded * (24 * 60 * 60 * 1000) / currentRate; // ms

    return {
      currentPower,
      requiredSampleSize,
      expectedDuration
    };
  }

  /**
   * Calculate pooled variance
   */
  private calculatePooledVariance(
    resultA: MetricResult,
    resultB: MetricResult
  ): number {
    const varA = Math.pow(resultA.standardDeviation || 0, 2);
    const varB = Math.pow(resultB.standardDeviation || 0, 2);
    
    return ((resultA.samples - 1) * varA + (resultB.samples - 1) * varB) /
           (resultA.samples + resultB.samples - 2);
  }

  /**
   * Calculate p-value for t-statistic (simplified)
   */
  private calculatePValue(tStatistic: number, df: number): number {
    // This is a simplified approximation
    // In production, use a proper statistical library
    const x = df / (df + tStatistic * tStatistic);
    const a = df / 2;
    const b = 0.5;
    
    // Incomplete beta function approximation
    const p = Math.pow(x, a) * Math.pow(1 - x, b);
    return 2 * p; // Two-tailed
  }

  /**
   * Calculate p-value for F-statistic (simplified)
   */
  private calculateFPValue(fStatistic: number, df1: number, df2: number): number {
    // Simplified approximation
    const x = df2 / (df2 + df1 * fStatistic);
    return Math.pow(x, df2 / 2);
  }

  /**
   * Calculate p-value for chi-square (simplified)
   */
  private calculateChiSquarePValue(chiSquare: number, df: number): number {
    // Simplified approximation using gamma function
    const x = chiSquare / 2;
    const k = df / 2;
    
    // Incomplete gamma function approximation
    let sum = 0;
    let term = 1;
    for (let i = 0; i < 20; i++) {
      sum += term;
      term *= x / (k + i);
    }
    
    return Math.exp(-x) * Math.pow(x, k) * sum / this.gamma(k);
  }

  /**
   * Gamma function approximation
   */
  private gamma(z: number): number {
    // Stirling's approximation
    return Math.sqrt(2 * Math.PI / z) * Math.pow(z / Math.E, z);
  }

  /**
   * Estimate statistical power
   */
  private estimatePower(effectSize: number, sampleSize: number): number {
    // Simplified power calculation
    const z = effectSize * Math.sqrt(sampleSize / 2);
    const power = 1 - this.normalCDF(-z + 1.96) + this.normalCDF(-z - 1.96);
    return Math.min(0.99, Math.max(0.01, power));
  }

  /**
   * Calculate required sample size
   */
  private calculateRequiredSampleSize(
    effectSize: number,
    targetPower: number,
    confidence: number
  ): number {
    if (effectSize === 0) return Infinity;
    
    const alpha = 1 - confidence;
    const zAlpha = this.getZScore(confidence);
    const zBeta = this.getZScore(targetPower);
    
    const n = 2 * Math.pow((zAlpha + zBeta) / effectSize, 2);
    return Math.ceil(n);
  }

  /**
   * Normal CDF approximation
   */
  private normalCDF(x: number): number {
    const a1 = 0.254829592;
    const a2 = -0.284496736;
    const a3 = 1.421413741;
    const a4 = -1.453152027;
    const a5 = 1.061405429;
    const p = 0.3275911;

    const sign = x < 0 ? -1 : 1;
    x = Math.abs(x) / Math.sqrt(2);

    const t = 1 / (1 + p * x);
    const y = 1 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-x * x);

    return 0.5 * (1 + sign * y);
  }

  /**
   * Get z-score for confidence level
   */
  private getZScore(confidence: number): number {
    const zScores: Record<number, number> = {
      0.80: 1.282,
      0.90: 1.645,
      0.95: 1.96,
      0.99: 2.576
    };
    
    // Find closest match
    const closest = Object.keys(zScores)
      .map(k => parseFloat(k))
      .reduce((prev, curr) => 
        Math.abs(curr - confidence) < Math.abs(prev - confidence) ? curr : prev
      );
    
    return zScores[closest] || 1.96;
  }
}