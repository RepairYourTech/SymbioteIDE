/**
 * Task Analyzer - Analyzes AI tasks to determine requirements and complexity
 */

import {
  AITask,
  TaskAnalysis,
  TaskType,
  ComplexityLevel,
  Capability,
  QualityLevel,
  UrgencyLevel
} from './interfaces';

export class TaskAnalyzer {
  private readonly codePatterns = {
    functionDefinition: /function\s+\w+|const\s+\w+\s*=\s*\(|class\s+\w+/,
    importStatement: /import\s+.+from|require\s*\(/,
    complexLogic: /if\s*\(.*\)\s*{[\s\S]*?}|for\s*\(|while\s*\(/,
    asyncCode: /async|await|Promise|then\s*\(/,
    errorHandling: /try\s*{|catch\s*\(|throw\s+/,
    typeDefinition: /interface\s+\w+|type\s+\w+|enum\s+\w+/
  };

  private readonly taskTypeKeywords: Record<TaskType, string[]> = {
    [TaskType.CodeGeneration]: ['create', 'generate', 'implement', 'build', 'write code'],
    [TaskType.CodeReview]: ['review', 'check', 'analyze code', 'find issues', 'audit'],
    [TaskType.CodeCompletion]: ['complete', 'finish', 'autocomplete', 'suggest'],
    [TaskType.Documentation]: ['document', 'explain', 'describe', 'comment', 'readme'],
    [TaskType.Testing]: ['test', 'unit test', 'integration test', 'test case', 'spec'],
    [TaskType.Refactoring]: ['refactor', 'improve', 'optimize', 'clean up', 'restructure'],
    [TaskType.BugFix]: ['fix', 'debug', 'resolve', 'bug', 'error', 'issue'],
    [TaskType.Security]: ['security', 'vulnerability', 'secure', 'auth', 'encrypt'],
    [TaskType.Performance]: ['performance', 'optimize', 'speed up', 'efficiency', 'latency'],
    [TaskType.General]: ['help', 'assist', 'question', 'how to', 'what is'],
    [TaskType.Research]: ['research', 'investigate', 'explore', 'find out', 'study'],
    [TaskType.Translation]: ['translate', 'convert', 'port', 'migrate', 'transform']
  };

  /**
   * Analyze a task to determine its requirements
   */
  async analyzeTask(task: AITask): Promise<TaskAnalysis> {
    const type = this.detectTaskType(task);
    const complexity = this.assessComplexity(task);
    const estimatedTokens = this.estimateTokens(task);
    const requiredCapabilities = this.determineRequiredCapabilities(task, type);
    const contextWindow = this.calculateRequiredContext(task, estimatedTokens);
    const qualityRequirement = this.determineQualityLevel(task, type);
    const urgency = task.priority || this.inferUrgency(task);
    const confidence = this.calculateConfidence(task, type);

    return {
      type,
      complexity,
      estimatedTokens,
      requiredCapabilities,
      contextWindow,
      qualityRequirement,
      urgency,
      confidence
    };
  }

  /**
   * Detect the task type from the prompt and context
   */
  private detectTaskType(task: AITask): TaskType {
    const prompt = task.prompt.toLowerCase();
    const context = JSON.stringify(task.context).toLowerCase();
    const combined = `${prompt} ${context}`;

    // Check explicit type if provided
    if (task.type) {
      return task.type;
    }

    // Score each task type based on keyword matches
    const scores: Record<TaskType, number> = {} as any;
    
    for (const [type, keywords] of Object.entries(this.taskTypeKeywords)) {
      scores[type as TaskType] = keywords.reduce((score, keyword) => {
        if (combined.includes(keyword)) {
          // Boost score if keyword is in the prompt itself
          return score + (prompt.includes(keyword) ? 2 : 1);
        }
        return score;
      }, 0);
    }

    // Special detection for code completion
    if (task.context?.files && task.context.files.length > 0) {
      const hasPartialCode = task.context.files.some(f => 
        f.content.trim().endsWith('//') || 
        f.content.trim().endsWith('/*') ||
        f.content.includes('// TODO') ||
        f.content.includes('...')
      );
      if (hasPartialCode) {
        scores[TaskType.CodeCompletion] += 5;
      }
    }

    // Find the type with the highest score
    let bestType = TaskType.General;
    let bestScore = 0;
    
    for (const [type, score] of Object.entries(scores)) {
      if (score > bestScore) {
        bestScore = score;
        bestType = type as TaskType;
      }
    }

    return bestType;
  }

  /**
   * Assess the complexity of the task
   */
  private assessComplexity(task: AITask): ComplexityLevel {
    let complexityScore = 0;

    // Check prompt length
    const promptLength = task.prompt.length;
    if (promptLength < 100) complexityScore += 1;
    else if (promptLength < 500) complexityScore += 2;
    else if (promptLength < 1000) complexityScore += 3;
    else complexityScore += 4;

    // Check context size
    if (task.context?.files) {
      const totalLines = task.context.files.reduce((sum, file) => {
        return sum + file.content.split('\n').length;
      }, 0);
      
      if (totalLines > 1000) complexityScore += 4;
      else if (totalLines > 500) complexityScore += 3;
      else if (totalLines > 100) complexityScore += 2;
      else complexityScore += 1;
    }

    // Check for complex patterns in code
    if (task.context?.files) {
      for (const file of task.context.files) {
        const content = file.content;
        
        // Check for complex code patterns
        if (this.codePatterns.complexLogic.test(content)) complexityScore += 1;
        if (this.codePatterns.asyncCode.test(content)) complexityScore += 1;
        if (this.codePatterns.errorHandling.test(content)) complexityScore += 1;
        if (this.codePatterns.typeDefinition.test(content)) complexityScore += 1;
        
        // Check for multiple functions/classes
        const functionMatches = content.match(this.codePatterns.functionDefinition);
        if (functionMatches && functionMatches.length > 5) complexityScore += 2;
      }
    }

    // Check for specific requirements
    if (task.constraints?.requiredCapabilities) {
      complexityScore += task.constraints.requiredCapabilities.length;
    }

    // Map score to complexity level
    if (complexityScore <= 3) return ComplexityLevel.Trivial;
    if (complexityScore <= 6) return ComplexityLevel.Simple;
    if (complexityScore <= 10) return ComplexityLevel.Moderate;
    if (complexityScore <= 15) return ComplexityLevel.Complex;
    return ComplexityLevel.Expert;
  }

  /**
   * Estimate the number of tokens needed
   */
  private estimateTokens(task: AITask): number {
    // Rough estimation: 1 token ≈ 4 characters
    let totalChars = task.prompt.length;

    // Add context
    if (task.context?.files) {
      totalChars += task.context.files.reduce((sum, file) => 
        sum + file.content.length, 0
      );
    }

    if (task.context?.previousMessages) {
      totalChars += task.context.previousMessages.reduce((sum, msg) => 
        sum + msg.content.length, 0
      );
    }

    // Add some overhead for system prompts and formatting
    const overhead = 500; // tokens
    const estimatedTokens = Math.ceil(totalChars / 4) + overhead;

    // Add expected output tokens based on task type
    const outputMultiplier = this.getOutputMultiplier(task.type || TaskType.General);
    
    return Math.ceil(estimatedTokens * outputMultiplier);
  }

  /**
   * Get output multiplier based on task type
   */
  private getOutputMultiplier(type: TaskType): number {
    switch (type) {
      case TaskType.CodeGeneration:
        return 2.5; // Expect significant code output
      case TaskType.Documentation:
        return 2.0; // Expect detailed documentation
      case TaskType.Testing:
        return 2.2; // Expect multiple test cases
      case TaskType.CodeReview:
        return 1.8; // Expect detailed analysis
      case TaskType.Refactoring:
        return 1.5; // Modified code similar to input
      case TaskType.CodeCompletion:
        return 1.2; // Usually short completions
      default:
        return 1.5; // Default multiplier
    }
  }

  /**
   * Determine required capabilities for the task
   */
  private determineRequiredCapabilities(task: AITask, type: TaskType): Capability[] {
    const capabilities: Set<Capability> = new Set();

    // Add capabilities based on task type
    switch (type) {
      case TaskType.CodeGeneration:
      case TaskType.CodeCompletion:
        capabilities.add(Capability.CodeGeneration);
        break;
      case TaskType.CodeReview:
      case TaskType.BugFix:
      case TaskType.Security:
      case TaskType.Refactoring:
        capabilities.add(Capability.CodeAnalysis);
        break;
      case TaskType.Documentation:
        capabilities.add(Capability.NaturalLanguage);
        capabilities.add(Capability.Summarization);
        break;
      case TaskType.Testing:
        capabilities.add(Capability.CodeGeneration);
        capabilities.add(Capability.Reasoning);
        break;
      case TaskType.Translation:
        capabilities.add(Capability.Translation);
        break;
      case TaskType.Research:
        capabilities.add(Capability.Reasoning);
        capabilities.add(Capability.NaturalLanguage);
        break;
      default:
        capabilities.add(Capability.NaturalLanguage);
    }

    // Add capabilities from constraints
    if (task.constraints?.requiredCapabilities) {
      task.constraints.requiredCapabilities.forEach(cap => capabilities.add(cap));
    }

    // Check for specific patterns requiring capabilities
    const combinedText = `${task.prompt} ${JSON.stringify(task.context)}`;
    
    if (combinedText.includes('math') || combinedText.includes('calculate')) {
      capabilities.add(Capability.Mathematics);
    }
    
    if (combinedText.includes('reason') || combinedText.includes('think')) {
      capabilities.add(Capability.Reasoning);
    }
    
    if (combinedText.includes('summarize') || combinedText.includes('summary')) {
      capabilities.add(Capability.Summarization);
    }

    // Check for long context needs
    const estimatedTokens = this.estimateTokens(task);
    if (estimatedTokens > 50000) {
      capabilities.add(Capability.LongContext);
    }

    return Array.from(capabilities);
  }

  /**
   * Calculate required context window
   */
  private calculateRequiredContext(task: AITask, estimatedTokens: number): number {
    // Add 20% buffer to estimated tokens
    const buffer = 1.2;
    const required = Math.ceil(estimatedTokens * buffer);
    
    // Round up to common context window sizes
    if (required <= 4096) return 4096;
    if (required <= 8192) return 8192;
    if (required <= 16384) return 16384;
    if (required <= 32768) return 32768;
    if (required <= 65536) return 65536;
    if (required <= 128000) return 128000;
    return 200000; // Maximum available
  }

  /**
   * Determine quality level requirement
   */
  private determineQualityLevel(task: AITask, type: TaskType): QualityLevel {
    // Check for explicit quality indicators
    const prompt = task.prompt.toLowerCase();
    
    if (prompt.includes('draft') || prompt.includes('quick')) {
      return QualityLevel.Draft;
    }
    
    if (prompt.includes('production') || prompt.includes('critical')) {
      return QualityLevel.Premium;
    }
    
    if (prompt.includes('high quality') || prompt.includes('thorough')) {
      return QualityLevel.High;
    }

    // Default by task type
    switch (type) {
      case TaskType.Security:
      case TaskType.BugFix:
        return QualityLevel.Premium; // Critical tasks need high quality
      case TaskType.CodeReview:
      case TaskType.Testing:
        return QualityLevel.High;
      case TaskType.CodeGeneration:
      case TaskType.Refactoring:
        return QualityLevel.Standard;
      case TaskType.Documentation:
      case TaskType.General:
        return QualityLevel.Standard;
      default:
        return QualityLevel.Standard;
    }
  }

  /**
   * Infer urgency from task content
   */
  private inferUrgency(task: AITask): UrgencyLevel {
    const prompt = task.prompt.toLowerCase();
    
    if (prompt.includes('urgent') || prompt.includes('asap') || prompt.includes('immediately')) {
      return UrgencyLevel.Critical;
    }
    
    if (prompt.includes('soon') || prompt.includes('quickly')) {
      return UrgencyLevel.High;
    }
    
    if (prompt.includes('when possible') || prompt.includes('low priority')) {
      return UrgencyLevel.Low;
    }
    
    return UrgencyLevel.Normal;
  }

  /**
   * Calculate confidence in the analysis
   */
  private calculateConfidence(task: AITask, detectedType: TaskType): number {
    let confidence = 0.5; // Base confidence

    // Increase confidence if task type was explicitly provided
    if (task.type === detectedType) {
      confidence += 0.3;
    }

    // Increase confidence based on keyword matches
    const prompt = task.prompt.toLowerCase();
    const keywords = this.taskTypeKeywords[detectedType];
    const matchCount = keywords.filter(keyword => prompt.includes(keyword)).length;
    confidence += Math.min(0.2, matchCount * 0.05);

    // Decrease confidence for ambiguous prompts
    if (prompt.length < 50) {
      confidence -= 0.1;
    }

    // Ensure confidence is between 0 and 1
    return Math.max(0, Math.min(1, confidence));
  }

  /**
   * Get analysis summary
   */
  getAnalysisSummary(analysis: TaskAnalysis): string {
    return `Task Type: ${analysis.type}
Complexity: ${analysis.complexity}
Estimated Tokens: ${analysis.estimatedTokens}
Required Context: ${analysis.contextWindow}
Quality Level: ${analysis.qualityRequirement}
Urgency: ${analysis.urgency}
Confidence: ${(analysis.confidence * 100).toFixed(1)}%
Capabilities: ${analysis.requiredCapabilities.join(', ')}`;
  }
}