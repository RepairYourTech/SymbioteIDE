/**
 * Cost Tracker - Tracks and manages AI model usage costs
 */

import {
  TaskType,
  ActualCost,
  CostMetrics,
  ModelProfile,
  TaskResult
} from './interfaces';

export interface CostLimit {
  amount: number;
  period: 'request' | 'hour' | 'day' | 'week' | 'month';
  currency: string;
}

export interface CostAlert {
  threshold: number; // percentage (0-1)
  callback: (metrics: CostMetrics) => void;
}

export interface CostRecord {
  timestamp: number;
  taskId: string;
  taskType: TaskType;
  modelId: string;
  provider: string;
  cost: ActualCost;
  duration: number;
  success: boolean;
}

export class CostTracker {
  private records: CostRecord[] = [];
  private limits: CostLimit[] = [];
  private alerts: CostAlert[] = [];
  private startTime: number = Date.now();

  constructor(limits?: CostLimit[]) {
    if (limits) {
      this.limits = limits;
    }
  }

  /**
   * Record a task execution cost
   */
  recordCost(result: TaskResult, model: ModelProfile, taskType: TaskType): void {
    const record: CostRecord = {
      timestamp: result.timestamp,
      taskId: result.taskId,
      taskType,
      modelId: model.id,
      provider: model.provider,
      cost: result.cost,
      duration: result.latency,
      success: result.status === 'success'
    };

    this.records.push(record);
    this.checkLimitsAndAlerts();
    this.cleanupOldRecords();
  }

  /**
   * Get current cost metrics
   */
  getMetrics(): CostMetrics {
    const now = Date.now();
    const hourAgo = now - 3600000;
    const dayAgo = now - 86400000;
    const weekAgo = now - 604800000;
    const monthAgo = now - 2592000000;

    // Calculate total cost
    const totalCost = this.records.reduce((sum, record) => 
      sum + record.cost.amount, 0
    );

    // Calculate cost per task type
    const costPerTask: Record<TaskType, number> = {} as any;
    for (const taskType of Object.values(TaskType)) {
      costPerTask[taskType] = this.records
        .filter(r => r.taskType === taskType)
        .reduce((sum, r) => sum + r.cost.amount, 0);
    }

    // Calculate cost per model
    const costPerModel: Record<string, number> = {};
    for (const record of this.records) {
      costPerModel[record.modelId] = (costPerModel[record.modelId] || 0) + record.cost.amount;
    }

    // Calculate budget utilization
    const budgetUtilization = this.calculateBudgetUtilization();

    // Project monthly cost based on recent usage
    const recentRecords = this.records.filter(r => r.timestamp > hourAgo);
    const recentCost = recentRecords.reduce((sum, r) => sum + r.cost.amount, 0);
    const projectedMonthlyCost = recentRecords.length > 0
      ? (recentCost / recentRecords.length) * 720 * 24 // Avg per request * requests per month
      : 0;

    return {
      totalCost,
      costPerTask,
      costPerModel,
      budgetUtilization,
      projectedMonthlyCost
    };
  }

  /**
   * Check if a cost would exceed limits
   */
  checkCostLimit(estimatedCost: number): { allowed: boolean; reason?: string } {
    for (const limit of this.limits) {
      const periodCost = this.getCostForPeriod(limit.period);
      if (periodCost + estimatedCost > limit.amount) {
        return {
          allowed: false,
          reason: `Would exceed ${limit.period}ly limit of ${limit.currency}${limit.amount}`
        };
      }
    }
    return { allowed: true };
  }

  /**
   * Add a cost limit
   */
  addLimit(limit: CostLimit): void {
    this.limits.push(limit);
  }

  /**
   * Add a cost alert
   */
  addAlert(alert: CostAlert): void {
    this.alerts.push(alert);
  }

  /**
   * Get cost breakdown for a period
   */
  getCostBreakdown(
    startTime: number,
    endTime: number = Date.now()
  ): {
    total: number;
    byModel: Record<string, number>;
    byTaskType: Record<string, number>;
    byProvider: Record<string, number>;
    successRate: number;
    averageCostPerTask: number;
    averageDuration: number;
  } {
    const periodRecords = this.records.filter(r => 
      r.timestamp >= startTime && r.timestamp <= endTime
    );

    const total = periodRecords.reduce((sum, r) => sum + r.cost.amount, 0);

    const byModel: Record<string, number> = {};
    const byTaskType: Record<string, number> = {};
    const byProvider: Record<string, number> = {};

    for (const record of periodRecords) {
      byModel[record.modelId] = (byModel[record.modelId] || 0) + record.cost.amount;
      byTaskType[record.taskType] = (byTaskType[record.taskType] || 0) + record.cost.amount;
      byProvider[record.provider] = (byProvider[record.provider] || 0) + record.cost.amount;
    }

    const successCount = periodRecords.filter(r => r.success).length;
    const successRate = periodRecords.length > 0 
      ? successCount / periodRecords.length 
      : 1;

    const averageCostPerTask = periodRecords.length > 0
      ? total / periodRecords.length
      : 0;

    const averageDuration = periodRecords.length > 0
      ? periodRecords.reduce((sum, r) => sum + r.duration, 0) / periodRecords.length
      : 0;

    return {
      total,
      byModel,
      byTaskType,
      byProvider,
      successRate,
      averageCostPerTask,
      averageDuration
    };
  }

  /**
   * Get spending rate analysis
   */
  getSpendingRate(): {
    currentRate: number; // per hour
    trend: 'increasing' | 'stable' | 'decreasing';
    projectedDailyCost: number;
    projectedMonthlyCost: number;
  } {
    const now = Date.now();
    const hourAgo = now - 3600000;
    const twoHoursAgo = now - 7200000;

    const lastHourCost = this.records
      .filter(r => r.timestamp > hourAgo)
      .reduce((sum, r) => sum + r.cost.amount, 0);

    const previousHourCost = this.records
      .filter(r => r.timestamp > twoHoursAgo && r.timestamp <= hourAgo)
      .reduce((sum, r) => sum + r.cost.amount, 0);

    const trend = lastHourCost > previousHourCost * 1.1 
      ? 'increasing' 
      : lastHourCost < previousHourCost * 0.9 
        ? 'decreasing' 
        : 'stable';

    return {
      currentRate: lastHourCost,
      trend,
      projectedDailyCost: lastHourCost * 24,
      projectedMonthlyCost: lastHourCost * 24 * 30
    };
  }

  /**
   * Get most expensive operations
   */
  getMostExpensive(limit: number = 10): {
    tasks: CostRecord[];
    models: Array<{ modelId: string; totalCost: number; count: number }>;
    taskTypes: Array<{ type: TaskType; totalCost: number; count: number }>;
  } {
    // Most expensive individual tasks
    const tasks = [...this.records]
      .sort((a, b) => b.cost.amount - a.cost.amount)
      .slice(0, limit);

    // Most expensive models overall
    const modelCosts = new Map<string, { totalCost: number; count: number }>();
    for (const record of this.records) {
      const existing = modelCosts.get(record.modelId) || { totalCost: 0, count: 0 };
      modelCosts.set(record.modelId, {
        totalCost: existing.totalCost + record.cost.amount,
        count: existing.count + 1
      });
    }

    const models = Array.from(modelCosts.entries())
      .map(([modelId, data]) => ({ modelId, ...data }))
      .sort((a, b) => b.totalCost - a.totalCost)
      .slice(0, limit);

    // Most expensive task types
    const typeCosts = new Map<TaskType, { totalCost: number; count: number }>();
    for (const record of this.records) {
      const existing = typeCosts.get(record.taskType) || { totalCost: 0, count: 0 };
      typeCosts.set(record.taskType, {
        totalCost: existing.totalCost + record.cost.amount,
        count: existing.count + 1
      });
    }

    const taskTypes = Array.from(typeCosts.entries())
      .map(([type, data]) => ({ type, ...data }))
      .sort((a, b) => b.totalCost - a.totalCost)
      .slice(0, limit);

    return { tasks, models, taskTypes };
  }

  /**
   * Export cost data
   */
  exportData(startTime?: number, endTime?: number): string {
    const records = startTime
      ? this.records.filter(r => r.timestamp >= startTime && r.timestamp <= (endTime || Date.now()))
      : this.records;

    return JSON.stringify({
      exportTime: Date.now(),
      startTime: startTime || this.startTime,
      endTime: endTime || Date.now(),
      records,
      summary: this.getCostBreakdown(startTime || this.startTime, endTime)
    }, null, 2);
  }

  /**
   * Import cost data
   */
  importData(json: string): void {
    const data = JSON.parse(json);
    if (data.records && Array.isArray(data.records)) {
      this.records.push(...data.records);
      this.records.sort((a, b) => a.timestamp - b.timestamp);
    }
  }

  /**
   * Reset cost tracking
   */
  reset(): void {
    this.records = [];
    this.startTime = Date.now();
  }

  // Private methods

  private getCostForPeriod(period: CostLimit['period']): number {
    const now = Date.now();
    let startTime: number;

    switch (period) {
      case 'request':
        return 0; // Per-request limit checked separately
      case 'hour':
        startTime = now - 3600000;
        break;
      case 'day':
        startTime = now - 86400000;
        break;
      case 'week':
        startTime = now - 604800000;
        break;
      case 'month':
        startTime = now - 2592000000;
        break;
    }

    return this.records
      .filter(r => r.timestamp >= startTime)
      .reduce((sum, r) => sum + r.cost.amount, 0);
  }

  private calculateBudgetUtilization(): number {
    if (this.limits.length === 0) return 0;

    const utilizations = this.limits.map(limit => {
      const spent = this.getCostForPeriod(limit.period);
      return spent / limit.amount;
    });

    return Math.max(...utilizations);
  }

  private checkLimitsAndAlerts(): void {
    const metrics = this.getMetrics();

    // Check alerts
    for (const alert of this.alerts) {
      if (metrics.budgetUtilization >= alert.threshold) {
        alert.callback(metrics);
      }
    }
  }

  private cleanupOldRecords(): void {
    // Keep records for the longest limit period plus buffer
    const maxRetention = Math.max(
      2592000000, // 30 days default
      ...this.limits.map(limit => {
        switch (limit.period) {
          case 'hour': return 86400000; // Keep 1 day
          case 'day': return 604800000; // Keep 1 week
          case 'week': return 2592000000; // Keep 30 days
          case 'month': return 7776000000; // Keep 90 days
          default: return 86400000;
        }
      })
    );

    const cutoff = Date.now() - maxRetention;
    this.records = this.records.filter(r => r.timestamp > cutoff);
  }
}