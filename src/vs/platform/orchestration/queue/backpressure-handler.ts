/**
 * Backpressure Handler Implementation
 */

import {
  BackpressureConfig,
  BackpressureStrategy,
  QueuedRequest,
  RequestPriority,
  QueueEvent,
  QueueEventType
} from './queue-interfaces';
import { EventEmitter } from 'events';

export interface BackpressureStatus {
  isActive: boolean;
  strategy: BackpressureStrategy;
  currentLoad: number;
  highWaterMark: number;
  lowWaterMark: number;
  droppedRequests: number;
  throttleDelay: number;
}

export class BackpressureHandler extends EventEmitter {
  private config: BackpressureConfig;
  private isBackpressureActive: boolean = false;
  private droppedRequestCount: number = 0;
  private throttleDelay: number = 0;
  private lastLoadCheck: number = Date.now();
  private loadHistory: number[] = [];
  private readonly loadHistorySize = 10;

  constructor(config: BackpressureConfig) {
    super();
    this.config = config;
  }

  /**
   * Check if backpressure should be applied based on current load
   */
  checkBackpressure(currentLoad: number, maxCapacity: number): boolean {
    const loadPercentage = (currentLoad / maxCapacity) * 100;
    this.updateLoadHistory(loadPercentage);

    // Apply hysteresis to prevent oscillation
    if (!this.isBackpressureActive && loadPercentage >= this.config.highWaterMark) {
      this.activateBackpressure();
      return true;
    } else if (this.isBackpressureActive && loadPercentage <= this.config.lowWaterMark) {
      this.deactivateBackpressure();
      return false;
    }

    return this.isBackpressureActive;
  }

  /**
   * Apply backpressure strategy to incoming request
   */
  async applyBackpressure(request: QueuedRequest): Promise<{
    accepted: boolean;
    reason?: string;
    delay?: number;
  }> {
    if (!this.isBackpressureActive) {
      return { accepted: true };
    }

    switch (this.config.strategy) {
      case BackpressureStrategy.Pause:
        return this.applyPauseStrategy(request);
      
      case BackpressureStrategy.Throttle:
        return this.applyThrottleStrategy(request);
      
      case BackpressureStrategy.Reject:
        return this.applyRejectStrategy(request);
      
      case BackpressureStrategy.Shed:
        return this.applyShedStrategy(request);
      
      default:
        return { accepted: true };
    }
  }

  /**
   * Get current backpressure status
   */
  getStatus(): BackpressureStatus {
    return {
      isActive: this.isBackpressureActive,
      strategy: this.config.strategy,
      currentLoad: this.getCurrentLoad(),
      highWaterMark: this.config.highWaterMark,
      lowWaterMark: this.config.lowWaterMark,
      droppedRequests: this.droppedRequestCount,
      throttleDelay: this.throttleDelay
    };
  }

  /**
   * Reset backpressure handler
   */
  reset(): void {
    this.isBackpressureActive = false;
    this.droppedRequestCount = 0;
    this.throttleDelay = 0;
    this.loadHistory = [];
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<BackpressureConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Pause strategy - delay acceptance of new requests
   */
  private async applyPauseStrategy(request: QueuedRequest): Promise<{
    accepted: boolean;
    reason?: string;
    delay?: number;
  }> {
    // Calculate delay based on load and priority
    const baseDelay = this.calculateBaseDelay();
    const priorityMultiplier = this.getPriorityMultiplier(request.priority);
    const delay = Math.floor(baseDelay * priorityMultiplier);

    if (delay > 0) {
      await this.sleep(delay);
    }

    return { 
      accepted: true, 
      delay 
    };
  }

  /**
   * Throttle strategy - slow down request acceptance
   */
  private async applyThrottleStrategy(request: QueuedRequest): Promise<{
    accepted: boolean;
    reason?: string;
    delay?: number;
  }> {
    // Implement adaptive throttling based on load
    const throttleDelay = this.calculateThrottleDelay();
    
    if (request.priority === RequestPriority.Critical) {
      // Critical requests get minimal delay
      const minDelay = Math.min(throttleDelay, 100);
      if (minDelay > 0) {
        await this.sleep(minDelay);
      }
      return { accepted: true, delay: minDelay };
    }

    // Apply full throttle delay for non-critical requests
    if (throttleDelay > 0) {
      await this.sleep(throttleDelay);
    }

    return { accepted: true, delay: throttleDelay };
  }

  /**
   * Reject strategy - reject new requests when overloaded
   */
  private applyRejectStrategy(request: QueuedRequest): {
    accepted: boolean;
    reason?: string;
  } {
    if (this.config.rejectOnFull) {
      // Accept only critical requests
      if (request.priority !== RequestPriority.Critical) {
        this.droppedRequestCount++;
        this.emitDroppedEvent(request, 'System overloaded - rejecting non-critical requests');
        return { 
          accepted: false, 
          reason: 'System overloaded - request rejected' 
        };
      }
    }

    return { accepted: true };
  }

  /**
   * Shed strategy - drop lowest priority requests
   */
  private applyShedStrategy(request: QueuedRequest): {
    accepted: boolean;
    reason?: string;
  } {
    // Calculate shed threshold based on load
    const loadFactor = this.getCurrentLoad() / 100;
    const shedThreshold = this.calculateShedThreshold(loadFactor);

    // Shed requests based on priority
    if (request.priority > shedThreshold) {
      this.droppedRequestCount++;
      this.emitDroppedEvent(request, 'Load shedding - priority too low');
      return { 
        accepted: false, 
        reason: 'Request shed due to system load' 
      };
    }

    return { accepted: true };
  }

  /**
   * Activate backpressure
   */
  private activateBackpressure(): void {
    this.isBackpressureActive = true;
    this.emit('event', {
      type: QueueEventType.BackpressureApplied,
      timestamp: new Date(),
      data: {
        strategy: this.config.strategy,
        load: this.getCurrentLoad()
      }
    } as QueueEvent);
  }

  /**
   * Deactivate backpressure
   */
  private deactivateBackpressure(): void {
    this.isBackpressureActive = false;
    this.throttleDelay = 0;
    this.emit('event', {
      type: QueueEventType.BackpressureReleased,
      timestamp: new Date(),
      data: {
        droppedRequests: this.droppedRequestCount,
        duration: Date.now() - this.lastLoadCheck
      }
    } as QueueEvent);
  }

  /**
   * Update load history for trend analysis
   */
  private updateLoadHistory(load: number): void {
    this.loadHistory.push(load);
    if (this.loadHistory.length > this.loadHistorySize) {
      this.loadHistory.shift();
    }
    this.lastLoadCheck = Date.now();
  }

  /**
   * Get current average load
   */
  private getCurrentLoad(): number {
    if (this.loadHistory.length === 0) {
      return 0;
    }
    const sum = this.loadHistory.reduce((a, b) => a + b, 0);
    return sum / this.loadHistory.length;
  }

  /**
   * Calculate base delay for pause strategy
   */
  private calculateBaseDelay(): number {
    const load = this.getCurrentLoad();
    const overload = Math.max(0, load - this.config.lowWaterMark);
    const range = this.config.highWaterMark - this.config.lowWaterMark;
    const factor = overload / range;
    
    // Exponential backoff: delay increases exponentially with load
    return Math.floor(100 * Math.pow(2, factor * 4)); // 100ms to ~1.6s
  }

  /**
   * Calculate throttle delay based on load
   */
  private calculateThrottleDelay(): number {
    const load = this.getCurrentLoad();
    const overloadFactor = (load - this.config.lowWaterMark) / 
                          (100 - this.config.lowWaterMark);
    
    // Adaptive throttling: increase delay as load increases
    this.throttleDelay = Math.floor(50 + (450 * overloadFactor)); // 50ms to 500ms
    return this.throttleDelay;
  }

  /**
   * Calculate shed threshold based on load
   */
  private calculateShedThreshold(loadFactor: number): RequestPriority {
    // As load increases, shed more requests (higher priority threshold)
    if (loadFactor > 0.95) {
      return RequestPriority.High; // Only accept Critical
    } else if (loadFactor > 0.9) {
      return RequestPriority.Normal; // Accept Critical and High
    } else if (loadFactor > 0.85) {
      return RequestPriority.Low; // Accept Critical, High, and Normal
    }
    return RequestPriority.Background; // Accept all
  }

  /**
   * Get priority multiplier for delays
   */
  private getPriorityMultiplier(priority: RequestPriority): number {
    switch (priority) {
      case RequestPriority.Critical:
        return 0.1; // 10% of base delay
      case RequestPriority.High:
        return 0.3; // 30% of base delay
      case RequestPriority.Normal:
        return 0.6; // 60% of base delay
      case RequestPriority.Low:
        return 1.0; // Full delay
      case RequestPriority.Background:
        return 1.5; // 150% of base delay
      default:
        return 1.0;
    }
  }

  /**
   * Emit dropped request event
   */
  private emitDroppedEvent(request: QueuedRequest, reason: string): void {
    this.emit('event', {
      type: QueueEventType.RequestDeadLettered,
      timestamp: new Date(),
      requestId: request.id,
      data: {
        reason,
        priority: request.priority,
        strategy: this.config.strategy
      }
    } as QueueEvent);
  }

  /**
   * Sleep utility
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

/**
 * Adaptive Backpressure Handler
 * Automatically adjusts strategy based on system conditions
 */
export class AdaptiveBackpressureHandler extends BackpressureHandler {
  private strategyHistory: { strategy: BackpressureStrategy; effectiveness: number }[] = [];
  private currentEffectiveness: number = 0;
  private requestsHandled: number = 0;
  private requestsDropped: number = 0;

  constructor(config: BackpressureConfig) {
    super(config);
  }

  /**
   * Override to track effectiveness and adapt strategy
   */
  async applyBackpressure(request: QueuedRequest): Promise<{
    accepted: boolean;
    reason?: string;
    delay?: number;
  }> {
    const result = await super.applyBackpressure(request);
    
    // Track effectiveness
    if (result.accepted) {
      this.requestsHandled++;
    } else {
      this.requestsDropped++;
    }
    
    // Periodically evaluate and adapt strategy
    if ((this.requestsHandled + this.requestsDropped) % 100 === 0) {
      this.evaluateAndAdaptStrategy();
    }
    
    return result;
  }

  /**
   * Evaluate current strategy effectiveness and adapt if needed
   */
  private evaluateAndAdaptStrategy(): void {
    const totalRequests = this.requestsHandled + this.requestsDropped;
    if (totalRequests === 0) return;

    // Calculate effectiveness (higher is better)
    // Balance between accepting requests and maintaining system stability
    const acceptanceRate = this.requestsHandled / totalRequests;
    const load = this.getStatus().currentLoad;
    const stabilityFactor = 1 - Math.abs(load - 75) / 25; // Optimal at 75% load
    
    this.currentEffectiveness = acceptanceRate * stabilityFactor;
    
    // Record strategy performance
    this.strategyHistory.push({
      strategy: this.getStatus().strategy,
      effectiveness: this.currentEffectiveness
    });
    
    // Keep only recent history
    if (this.strategyHistory.length > 10) {
      this.strategyHistory.shift();
    }
    
    // Adapt strategy if current one is underperforming
    if (this.currentEffectiveness < 0.7) {
      this.selectBetterStrategy();
    }
    
    // Reset counters
    this.requestsHandled = 0;
    this.requestsDropped = 0;
  }

  /**
   * Select a better strategy based on history
   */
  private selectBetterStrategy(): void {
    const strategyScores = new Map<BackpressureStrategy, number>();
    
    // Calculate average effectiveness for each strategy
    for (const entry of this.strategyHistory) {
      const current = strategyScores.get(entry.strategy) || 0;
      strategyScores.set(entry.strategy, current + entry.effectiveness);
    }
    
    // Find best performing strategy
    let bestStrategy = this.getStatus().strategy;
    let bestScore = 0;
    
    for (const [strategy, totalScore] of strategyScores) {
      const count = this.strategyHistory.filter(h => h.strategy === strategy).length;
      const avgScore = totalScore / count;
      
      if (avgScore > bestScore) {
        bestScore = avgScore;
        bestStrategy = strategy;
      }
    }
    
    // Switch to better strategy if different
    if (bestStrategy !== this.getStatus().strategy) {
      this.updateConfig({ strategy: bestStrategy });
      this.emit('strategyChanged', {
        from: this.getStatus().strategy,
        to: bestStrategy,
        reason: 'Adaptive optimization',
        effectiveness: bestScore
      });
    }
  }
}