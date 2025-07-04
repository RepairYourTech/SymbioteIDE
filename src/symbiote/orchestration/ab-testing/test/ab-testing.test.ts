/**
 * A/B Testing Framework Tests
 */

import { 
  ExperimentManager,
  ExperimentConfig,
  ExperimentStatus,
  AllocationMethod,
  MetricType,
  AggregationType,
  TrafficSplitter,
  MetricsCollector,
  StatisticalAnalyzer
} from '../index';

import { ModelRegistry } from '../../model-registry';
import { OrchestrationEngine } from '../../orchestration-engine';
import { ABTestingIntegration } from '../ab-testing-integration';
import { AITask, TaskType, TaskResult } from '../../interfaces';

describe('A/B Testing Framework', () => {
  let modelRegistry: ModelRegistry;
  let experimentManager: ExperimentManager;
  let orchestrationEngine: OrchestrationEngine;
  let abTesting: ABTestingIntegration;

  beforeEach(() => {
    modelRegistry = new ModelRegistry();
    experimentManager = new ExperimentManager(modelRegistry);
    orchestrationEngine = new OrchestrationEngine();
    abTesting = new ABTestingIntegration(orchestrationEngine, experimentManager);
  });

  describe('Experiment Management', () => {
    it('should create and manage experiments', () => {
      const experiment = experimentManager.createExperiment({
        name: 'GPT-4 vs Claude Comparison',
        description: 'Compare performance of GPT-4 and Claude on code generation',
        startTime: new Date(),
        variants: [
          {
            id: 'gpt4',
            name: 'GPT-4',
            modelId: 'gpt-4',
            weight: 50
          },
          {
            id: 'claude',
            name: 'Claude 3 Opus',
            modelId: 'claude-3-opus-20240229',
            weight: 50
          }
        ],
        trafficAllocation: {
          method: AllocationMethod.Random
        },
        metrics: [
          {
            id: 'latency',
            name: 'Response Latency',
            type: MetricType.Latency,
            aggregation: AggregationType.Average,
            unit: 'ms',
            higherIsBetter: false,
            confidenceLevel: 0.95
          }
        ]
      });

      expect(experiment.id).toBeDefined();
      expect(experiment.status).toBe(ExperimentStatus.Draft);
      expect(experiment.variants).toHaveLength(2);
    });

    it('should start and complete experiments', () => {
      const experiment = experimentManager.createExperiment({
        name: 'Test Experiment',
        description: 'Test',
        startTime: new Date(),
        variants: [
          { id: 'a', name: 'A', modelId: 'gpt-4', weight: 50 },
          { id: 'b', name: 'B', modelId: 'claude-3-opus-20240229', weight: 50 }
        ],
        trafficAllocation: { method: AllocationMethod.Random },
        metrics: [{
          id: 'success',
          name: 'Success Rate',
          type: MetricType.SuccessRate,
          aggregation: AggregationType.Rate,
          higherIsBetter: true
        }]
      });

      experimentManager.startExperiment(experiment.id);
      expect(experimentManager.getExperiment(experiment.id).status).toBe(ExperimentStatus.Running);

      const result = experimentManager.completeExperiment(experiment.id);
      expect(result.status).toBe(ExperimentStatus.Completed);
    });
  });

  describe('Traffic Splitting', () => {
    let splitter: TrafficSplitter;

    beforeEach(() => {
      splitter = new TrafficSplitter();
    });

    it('should split traffic according to weights', () => {
      const experiment: ExperimentConfig = {
        id: 'test-exp',
        name: 'Test',
        description: 'Test',
        startTime: new Date(),
        status: ExperimentStatus.Running,
        variants: [
          { id: 'a', name: 'A', modelId: 'model-a', weight: 30 },
          { id: 'b', name: 'B', modelId: 'model-b', weight: 70 }
        ],
        trafficAllocation: { method: AllocationMethod.Random },
        metrics: []
      };

      const assignments: Record<string, number> = { a: 0, b: 0 };
      const iterations = 10000;

      for (let i = 0; i < iterations; i++) {
        const task: AITask = {
          id: `task-${i}`,
          type: TaskType.General,
          prompt: 'Test prompt'
        };

        const variant = splitter.assignVariant(experiment, task);
        if (variant) {
          assignments[variant.id]++;
        }
      }

      const aPercentage = (assignments.a / iterations) * 100;
      const bPercentage = (assignments.b / iterations) * 100;

      expect(aPercentage).toBeCloseTo(30, 1);
      expect(bPercentage).toBeCloseTo(70, 1);
    });

    it('should provide deterministic assignment', () => {
      const experiment: ExperimentConfig = {
        id: 'test-exp',
        name: 'Test',
        description: 'Test',
        startTime: new Date(),
        status: ExperimentStatus.Running,
        variants: [
          { id: 'a', name: 'A', modelId: 'model-a', weight: 50 },
          { id: 'b', name: 'B', modelId: 'model-b', weight: 50 }
        ],
        trafficAllocation: { 
          method: AllocationMethod.Deterministic,
          seed: 'test-seed'
        },
        metrics: []
      };

      const task: AITask = {
        id: 'same-task',
        type: TaskType.General,
        prompt: 'Test prompt'
      };

      // Should get same assignment for same task
      const variant1 = splitter.assignVariant(experiment, task);
      const variant2 = splitter.assignVariant(experiment, task);
      
      expect(variant1?.id).toBe(variant2?.id);
    });

    it('should handle user-based assignment', () => {
      const experiment: ExperimentConfig = {
        id: 'test-exp',
        name: 'Test',
        description: 'Test',
        startTime: new Date(),
        status: ExperimentStatus.Running,
        variants: [
          { id: 'a', name: 'A', modelId: 'model-a', weight: 50 },
          { id: 'b', name: 'B', modelId: 'model-b', weight: 50 }
        ],
        trafficAllocation: { method: AllocationMethod.UserBased },
        metrics: []
      };

      const userId = 'user-123';
      const variants = new Set<string>();

      // Same user should always get same variant
      for (let i = 0; i < 10; i++) {
        const task: AITask = {
          id: `task-${i}`,
          type: TaskType.General,
          prompt: 'Different prompts each time'
        };

        const variant = splitter.assignVariant(experiment, task, { userId });
        if (variant) {
          variants.add(variant.id);
        }
      }

      expect(variants.size).toBe(1);
    });
  });

  describe('Metrics Collection', () => {
    let collector: MetricsCollector;

    beforeEach(() => {
      collector = new MetricsCollector();
    });

    it('should collect metrics from task results', () => {
      const task: AITask = {
        id: 'test-task',
        type: TaskType.CodeGeneration,
        prompt: 'Test'
      };

      const result: TaskResult = {
        id: 'result-1',
        taskId: 'test-task',
        status: 'success',
        content: 'Generated code',
        model: 'gpt-4',
        usage: {
          promptTokens: 100,
          completionTokens: 200,
          totalTokens: 300
        },
        cost: {
          amount: 0.015,
          currency: 'USD',
          breakdown: {} as any
        },
        latency: 2500,
        timestamp: Date.now()
      };

      const metricConfigs = [
        {
          id: 'latency',
          name: 'Latency',
          type: MetricType.Latency,
          aggregation: AggregationType.Average,
          higherIsBetter: false
        },
        {
          id: 'cost',
          name: 'Cost',
          type: MetricType.Cost,
          aggregation: AggregationType.Average,
          higherIsBetter: false
        },
        {
          id: 'tokens',
          name: 'Tokens',
          type: MetricType.TokenUsage,
          aggregation: AggregationType.Sum,
          higherIsBetter: false
        }
      ];

      const metrics = collector.collectMetrics(task, result, metricConfigs);

      expect(metrics.latency).toBe(2500);
      expect(metrics.cost).toBe(0.015);
      expect(metrics.tokens).toBe(300);
    });
  });

  describe('Statistical Analysis', () => {
    let analyzer: StatisticalAnalyzer;

    beforeEach(() => {
      analyzer = new StatisticalAnalyzer();
    });

    it('should perform statistical analysis', () => {
      const variantResults = [
        {
          variantId: 'a',
          variantName: 'Variant A',
          sampleSize: 100,
          metrics: [
            {
              metricId: 'latency',
              metricName: 'Latency',
              value: 1000,
              standardDeviation: 200,
              samples: 100,
              confidenceInterval: {
                lower: 960,
                upper: 1040,
                confidence: 0.95
              }
            }
          ],
          errors: 2,
          errorRate: 0.02
        },
        {
          variantId: 'b',
          variantName: 'Variant B',
          sampleSize: 100,
          metrics: [
            {
              metricId: 'latency',
              metricName: 'Latency',
              value: 1200,
              standardDeviation: 250,
              samples: 100,
              confidenceInterval: {
                lower: 1150,
                upper: 1250,
                confidence: 0.95
              }
            }
          ],
          errors: 5,
          errorRate: 0.05
        }
      ];

      const metricConfigs = [
        {
          id: 'latency',
          name: 'Latency',
          type: MetricType.Latency,
          aggregation: AggregationType.Average,
          higherIsBetter: false,
          confidenceLevel: 0.95,
          metadata: { isPrimary: true }
        }
      ];

      const analysis = analyzer.analyze(variantResults, metricConfigs);

      expect(analysis).toBeDefined();
      expect(analysis?.tests).toHaveLength(1);
      expect(analysis?.tests[0].testType).toBe('t-test');
      // Variant A should win due to lower latency
      if (analysis?.winner) {
        expect(analysis.winner).toBe('a');
      }
    });
  });

  describe('Integration with Orchestration Engine', () => {
    it('should create model comparison experiment', async () => {
      const experiment = await abTesting.createModelComparison({
        name: 'Model Performance Test',
        description: 'Compare different models',
        baselineModel: 'gpt-3.5-turbo',
        challengerModels: ['gpt-4', 'claude-3-sonnet-20240229'],
        trafficSplit: [34, 33, 33],
        duration: 24 * 60 * 60 * 1000 // 24 hours
      });

      expect(experiment.variants).toHaveLength(3);
      expect(experiment.variants[0].weight).toBe(34);
      expect(experiment.variants[1].weight).toBe(33);
      expect(experiment.variants[2].weight).toBe(33);
      expect(experiment.metrics.length).toBeGreaterThan(0);
    });

    it('should create prompt variation experiment', async () => {
      const experiment = await abTesting.createPromptExperiment({
        name: 'Prompt Optimization',
        description: 'Test different prompt styles',
        modelId: 'gpt-4',
        baselinePrompt: 'You are a helpful assistant.',
        promptVariations: [
          {
            name: 'Expert Style',
            systemPrompt: 'You are an expert software engineer with 20 years of experience.',
            temperature: 0.7
          },
          {
            name: 'Concise Style',
            systemPrompt: 'You are a concise and efficient coding assistant. Be brief.',
            temperature: 0.5
          }
        ],
        sampleSize: 100
      });

      expect(experiment.variants).toHaveLength(3);
      expect(experiment.variants[0].id).toBe('baseline');
      expect(experiment.variants[1].modelConfig?.temperature).toBe(0.7);
      expect(experiment.variants[2].modelConfig?.temperature).toBe(0.5);
    });
  });
});

// Run tests if this file is executed directly
if (require.main === module) {
  console.log('Running A/B testing framework tests...');
  // In a real environment, we would use Jest or another test runner
}