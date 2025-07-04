/**
 * Loop Refiner Agent Pattern
 * 
 * Iteratively refines results until quality criteria are met
 */

import { LoopAgent, Tool } from '@google/adk';
import { ADKHivemindAgent, ADKHivemindConfig } from '../core/adk-hivemind-base';
import { HivemindTask, TaskResult, AgentSpecialty } from '../../types';
import { Logger } from '../../../utils/logger';

export interface RefinementStep {
  agent: ADKHivemindAgent;
  role: 'generator' | 'evaluator' | 'refiner';
  evaluationCriteria?: EvaluationCriteria;
}

export interface EvaluationCriteria {
  name: string;
  description: string;
  threshold: number;
  evaluator: (result: any) => Promise<number>; // Returns score 0-1
}

export interface LoopRefinerConfig extends ADKHivemindConfig {
  steps: RefinementStep[];
  maxIterations: number;
  minIterations?: number;
  exitCriteria: EvaluationCriteria[];
  improvementThreshold?: number; // Min improvement between iterations
  aggregateScores?: boolean; // Combine all criteria scores
}

export class LoopRefinerAgent extends ADKHivemindAgent {
  private steps: RefinementStep[];
  private maxIterations: number;
  private minIterations: number;
  private exitCriteria: EvaluationCriteria[];
  private improvementThreshold: number;
  private aggregateScores: boolean;
  private iterationHistory: any[] = [];
  
  constructor(config: LoopRefinerConfig) {
    super({
      ...config,
      specialty: AgentSpecialty.General,
      systemPrompt: config.systemPrompt || `You are an iterative refinement coordinator that progressively improves results through multiple iterations.
      
Refinement steps:
${config.steps.map(step => 
  `- ${step.agent.name} (${step.role}): ${step.agent.capabilities.join(', ')}`
).join('\n')}

Exit criteria:
${config.exitCriteria.map(c => 
  `- ${c.name}: ${c.description} (threshold: ${c.threshold})`
).join('\n')}

Your role is to:
1. Execute refinement loops
2. Evaluate results against criteria
3. Decide when to continue or stop
4. Track improvement progress`
    });
    
    this.steps = config.steps;
    this.maxIterations = config.maxIterations;
    this.minIterations = config.minIterations || 1;
    this.exitCriteria = config.exitCriteria;
    this.improvementThreshold = config.improvementThreshold || 0.05;
    this.aggregateScores = config.aggregateScores ?? true;
    
    // Add loop control tools
    this.addLoopTools();
  }
  
  /**
   * Add loop-specific tools
   */
  private addLoopTools(): void {
    this.tools.push(new Tool({
      name: 'evaluate_result',
      description: 'Evaluate a result against exit criteria',
      parameters: {
        result: { type: 'any', required: true },
        criteriaName: { type: 'string', required: false }
      },
      handler: async (params: any) => {
        return this.evaluateResult(params.result, params.criteriaName);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'should_continue',
      description: 'Determine if refinement should continue',
      parameters: {
        currentIteration: { type: 'number', required: true },
        scores: { type: 'object', required: true }
      },
      handler: async (params: any) => {
        return this.shouldContinue(params.currentIteration, params.scores);
      }
    }));
    
    this.tools.push(new Tool({
      name: 'generate_refinement_prompt',
      description: 'Generate prompt for next refinement iteration',
      parameters: {
        previousResult: { type: 'any', required: true },
        evaluation: { type: 'object', required: true }
      },
      handler: async (params: any) => {
        return this.generateRefinementPrompt(params.previousResult, params.evaluation);
      }
    }));
  }
  
  /**
   * Execute iterative refinement
   */
  public async executeTask(task: HivemindTask): Promise<TaskResult> {
    const startTime = Date.now();
    const loopId = `loop_${task.id}`;
    
    try {
      // Initialize loop state
      const loopState = {
        taskId: task.id,
        iterations: [],
        currentIteration: 0,
        bestResult: null,
        bestScore: 0,
        status: 'running'
      };
      
      await this.writeState(loopId, loopState);
      
      let currentResult: any = null;
      let previousScores: any = {};
      
      // Execute refinement loop
      for (let iteration = 0; iteration < this.maxIterations; iteration++) {
        loopState.currentIteration = iteration;
        
        const iterationData = {
          iteration,
          startTime: new Date(),
          steps: [],
          scores: {},
          improvement: 0
        };
        
        // Execute each step in the refinement process
        for (const step of this.steps) {
          const stepStartTime = Date.now();
          
          let stepInput: HivemindTask;
          if (step.role === 'generator' && iteration === 0) {
            // First iteration - use original task
            stepInput = task;
          } else if (step.role === 'refiner') {
            // Refiner gets previous result and evaluation
            stepInput = {
              ...task,
              description: `Refine the following result based on evaluation feedback:\n\nPrevious Result: ${JSON.stringify(currentResult)}\n\nEvaluation: ${JSON.stringify(iterationData.scores)}`,
              context: {
                ...task.context,
                previousResult: currentResult,
                evaluation: iterationData.scores,
                iteration
              }
            };
          } else if (step.role === 'evaluator') {
            // Evaluator checks the current result
            stepInput = {
              ...task,
              description: `Evaluate the following result:\n\n${JSON.stringify(currentResult)}`,
              context: {
                ...task.context,
                resultToEvaluate: currentResult,
                criteria: this.exitCriteria.map(c => ({
                  name: c.name,
                  description: c.description,
                  threshold: c.threshold
                }))
              }
            };
          } else {
            // Default - pass current state
            stepInput = {
              ...task,
              context: {
                ...task.context,
                currentResult,
                iteration
              }
            };
          }
          
          // Execute step
          const stepResult = await step.agent.executeTask(stepInput);
          
          iterationData.steps.push({
            agent: step.agent.id,
            role: step.role,
            duration: Date.now() - stepStartTime,
            success: stepResult.success,
            output: stepResult.output
          });
          
          // Update current result
          if (step.role === 'generator' || step.role === 'refiner') {
            currentResult = stepResult.output;
          }
        }
        
        // Evaluate current result
        const evaluation = await this.evaluateAllCriteria(currentResult);
        iterationData.scores = evaluation.scores;
        
        // Calculate improvement
        if (iteration > 0) {
          const currentScore = evaluation.overallScore;
          const previousScore = previousScores.overallScore || 0;
          iterationData.improvement = currentScore - previousScore;
        }
        
        // Update best result if improved
        if (evaluation.overallScore > loopState.bestScore) {
          loopState.bestResult = currentResult;
          loopState.bestScore = evaluation.overallScore;
        }
        
        // Store iteration data
        iterationData.endTime = new Date();
        loopState.iterations.push(iterationData);
        await this.writeState(loopId, loopState);
        
        // Check exit conditions
        const shouldContinue = await this.shouldContinue(iteration + 1, evaluation);
        
        if (!shouldContinue) {
          this.logger.info(`Exiting refinement loop after ${iteration + 1} iterations`);
          break;
        }
        
        previousScores = evaluation.scores;
      }
      
      // Create final result
      loopState.status = 'completed';
      await this.writeState(loopId, loopState);
      
      const finalResult: TaskResult = {
        taskId: task.id,
        success: loopState.bestScore >= this.getMinAcceptableScore(),
        output: {
          bestResult: loopState.bestResult,
          bestScore: loopState.bestScore,
          iterations: loopState.iterations.length,
          finalScores: loopState.iterations[loopState.iterations.length - 1]?.scores,
          improvementHistory: loopState.iterations.map(i => ({
            iteration: i.iteration,
            score: i.scores.overallScore || 0,
            improvement: i.improvement
          }))
        },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
      
      // Aggregate metrics from all iterations
      loopState.iterations.forEach(iteration => {
        iteration.steps.forEach((step: any) => {
          if (step.output?.filesModified) {
            finalResult.filesModified.push(...step.output.filesModified);
          }
          if (step.output?.filesCreated) {
            finalResult.filesCreated.push(...step.output.filesCreated);
          }
        });
      });
      
      // Remove duplicates
      finalResult.filesModified = [...new Set(finalResult.filesModified)];
      finalResult.filesCreated = [...new Set(finalResult.filesCreated)];
      
      // Store in history
      this.iterationHistory.push({
        taskId: task.id,
        loopId,
        iterations: loopState.iterations.length,
        bestScore: loopState.bestScore,
        success: finalResult.success
      });
      
      return finalResult;
      
    } catch (error) {
      this.logger.error(`Loop refinement failed`, error);
      
      return {
        taskId: task.id,
        success: false,
        output: { error: error.message },
        filesModified: [],
        filesCreated: [],
        testsAdded: 0,
        issuesFound: [`Loop refinement error: ${error.message}`],
        performanceMetrics: {
          duration: Date.now() - startTime,
          tokensUsed: 0,
          cost: 0
        }
      };
    }
  }
  
  /**
   * Evaluate result against all criteria
   */
  private async evaluateAllCriteria(result: any): Promise<any> {
    const scores: any = {};
    let totalScore = 0;
    
    for (const criterion of this.exitCriteria) {
      try {
        const score = await criterion.evaluator(result);
        scores[criterion.name] = {
          score,
          threshold: criterion.threshold,
          passed: score >= criterion.threshold
        };
        totalScore += score;
      } catch (error) {
        this.logger.error(`Failed to evaluate criterion ${criterion.name}`, error);
        scores[criterion.name] = {
          score: 0,
          threshold: criterion.threshold,
          passed: false,
          error: error.message
        };
      }
    }
    
    const overallScore = this.aggregateScores 
      ? totalScore / this.exitCriteria.length 
      : Math.min(...Object.values(scores).map((s: any) => s.score));
    
    return {
      scores,
      overallScore,
      allPassed: Object.values(scores).every((s: any) => s.passed)
    };
  }
  
  /**
   * Determine if refinement should continue
   */
  private async shouldContinue(iteration: number, evaluation: any): Promise<boolean> {
    // Check minimum iterations
    if (iteration < this.minIterations) {
      return true;
    }
    
    // Check maximum iterations
    if (iteration >= this.maxIterations) {
      return false;
    }
    
    // Check if all criteria are met
    if (evaluation.allPassed) {
      return false;
    }
    
    // Check improvement threshold
    if (iteration > 1) {
      const history = await this.readState(`loop_${this.id}_history`);
      if (history && history.length > 1) {
        const lastImprovement = history[history.length - 1].improvement || 0;
        if (Math.abs(lastImprovement) < this.improvementThreshold) {
          this.logger.info('Improvement below threshold, stopping refinement');
          return false;
        }
      }
    }
    
    return true;
  }
  
  /**
   * Evaluate result (tool function)
   */
  private async evaluateResult(result: any, criteriaName?: string): Promise<any> {
    if (criteriaName) {
      const criterion = this.exitCriteria.find(c => c.name === criteriaName);
      if (criterion) {
        const score = await criterion.evaluator(result);
        return {
          criterion: criteriaName,
          score,
          threshold: criterion.threshold,
          passed: score >= criterion.threshold
        };
      }
    }
    
    return this.evaluateAllCriteria(result);
  }
  
  /**
   * Generate refinement prompt
   */
  private async generateRefinementPrompt(previousResult: any, evaluation: any): Promise<string> {
    const failedCriteria = Object.entries(evaluation.scores)
      .filter(([_, score]: any) => !score.passed)
      .map(([name, score]: any) => `- ${name}: ${score.score}/${score.threshold}`);
    
    return `Please refine the previous result to improve the following criteria:

${failedCriteria.join('\n')}

Previous result:
${JSON.stringify(previousResult, null, 2)}

Focus on addressing the areas that did not meet the threshold.`;
  }
  
  /**
   * Get minimum acceptable score
   */
  private getMinAcceptableScore(): number {
    if (this.aggregateScores) {
      return this.exitCriteria.reduce((sum, c) => sum + c.threshold, 0) / this.exitCriteria.length;
    }
    return Math.min(...this.exitCriteria.map(c => c.threshold));
  }
  
  /**
   * Build task prompt
   */
  protected buildTaskPrompt(task: HivemindTask, memories: any[]): string {
    return `Execute iterative refinement for this task:

Task: ${task.title}
Description: ${task.description}

Refine the result through multiple iterations until quality criteria are met.
Maximum iterations: ${this.maxIterations}`;
  }
  
  /**
   * Get iteration history
   */
  public getIterationHistory(): any[] {
    return [...this.iterationHistory];
  }
  
  /**
   * Add evaluation criterion
   */
  public addCriterion(criterion: EvaluationCriteria): void {
    this.exitCriteria.push(criterion);
  }
  
  /**
   * Remove evaluation criterion
   */
  public removeCriterion(name: string): void {
    this.exitCriteria = this.exitCriteria.filter(c => c.name !== name);
  }
}