/**
 * Learning Engine
 * 
 * Learns from command patterns and improves suggestions over time
 */

import {
  TerminalCommand,
  CommandPattern,
  CommandSuggestion,
  CommandCategory
} from '../../types/terminal-api';

interface LearnedPattern {
  pattern: string;
  frequency: number;
  contexts: Map<string, number>; // context -> count
  successRate: number;
  averageDuration: number;
  lastUsed: Date;
  category: CommandCategory;
}

interface SessionLearning {
  sessionId: string;
  patterns: Map<string, LearnedPattern>;
  commandSequences: CommandSequence[];
  contextualSuggestions: Map<string, string[]>; // context -> suggestions
}

interface CommandSequence {
  commands: string[];
  frequency: number;
  context: string;
  success: boolean;
}

export class LearningEngine {
  private sessionLearnings: Map<string, SessionLearning> = new Map();
  private globalPatterns: Map<string, LearnedPattern> = new Map();
  private sequenceTrie: SequenceTrie = new SequenceTrie();
  
  /**
   * Record a command execution
   */
  recordCommand(sessionId: string, command: string): void {
    const learning = this.getOrCreateSessionLearning(sessionId);
    const baseCommand = this.extractBaseCommand(command);
    const context = this.extractContext(command);
    
    // Update pattern frequency
    const pattern = learning.patterns.get(baseCommand) || this.createPattern(baseCommand);
    pattern.frequency++;
    pattern.lastUsed = new Date();
    pattern.contexts.set(context, (pattern.contexts.get(context) || 0) + 1);
    
    learning.patterns.set(baseCommand, pattern);
    
    // Update global patterns
    this.updateGlobalPattern(baseCommand, pattern);
    
    // Update sequence trie
    this.sequenceTrie.addCommand(sessionId, command);
  }
  
  /**
   * Record command result
   */
  recordResult(sessionId: string, result: TerminalCommand): void {
    const learning = this.getOrCreateSessionLearning(sessionId);
    const baseCommand = this.extractBaseCommand(result.command);
    
    const pattern = learning.patterns.get(baseCommand);
    if (pattern) {
      // Update success rate
      const success = result.exit_code === 0;
      const totalExecutions = pattern.frequency;
      const currentSuccesses = pattern.successRate * (totalExecutions - 1);
      pattern.successRate = (currentSuccesses + (success ? 1 : 0)) / totalExecutions;
      
      // Update average duration
      if (result.duration) {
        const currentTotal = pattern.averageDuration * (totalExecutions - 1);
        pattern.averageDuration = (currentTotal + result.duration) / totalExecutions;
      }
    }
  }
  
  /**
   * Get suggestions based on learning
   */
  getSuggestions(context: string, sessionId: string): CommandSuggestion[] {
    const suggestions: CommandSuggestion[] = [];
    const learning = this.sessionLearnings.get(sessionId);
    
    if (!learning) {
      return suggestions;
    }
    
    // Get sequence-based suggestions
    const sequenceSuggestions = this.sequenceTrie.getSuggestions(sessionId, 3);
    sequenceSuggestions.forEach(cmd => {
      suggestions.push({
        command: cmd.command,
        description: 'Based on your command history',
        confidence: cmd.confidence,
        category: this.categorizeCommand(cmd.command)
      });
    });
    
    // Get context-based suggestions
    const contextSuggestions = learning.contextualSuggestions.get(context) || [];
    contextSuggestions.forEach(cmd => {
      suggestions.push({
        command: cmd,
        description: 'Frequently used in this context',
        confidence: 0.7,
        category: this.categorizeCommand(cmd)
      });
    });
    
    // Get pattern-based suggestions
    const relevantPatterns = Array.from(learning.patterns.values())
      .filter(p => p.contexts.has(context) && p.successRate > 0.7)
      .sort((a, b) => b.frequency - a.frequency)
      .slice(0, 5);
    
    relevantPatterns.forEach(pattern => {
      suggestions.push({
        command: pattern.pattern,
        description: `Used ${pattern.frequency} times`,
        confidence: pattern.successRate,
        category: pattern.category
      });
    });
    
    return suggestions;
  }
  
  /**
   * Analyze patterns for a session
   */
  analyzePatterns(sessionId: string): CommandPattern[] {
    const learning = this.sessionLearnings.get(sessionId);
    if (!learning) {
      return [];
    }
    
    const patterns: CommandPattern[] = [];
    
    // Analyze command sequences
    const sequences = this.findFrequentSequences(learning.commandSequences);
    sequences.forEach(seq => {
      patterns.push({
        pattern: seq.commands.join(' → '),
        frequency: seq.frequency,
        typical_sequence: seq.commands,
        context: seq.context,
        automation_potential: this.calculateAutomationPotential(seq)
      });
    });
    
    // Analyze individual commands
    learning.patterns.forEach((pattern, command) => {
      if (pattern.frequency > 5) {
        patterns.push({
          pattern: command,
          frequency: pattern.frequency,
          typical_sequence: [command],
          context: Array.from(pattern.contexts.keys()).join(', '),
          automation_potential: pattern.frequency > 10 ? 0.8 : 0.5
        });
      }
    });
    
    return patterns;
  }
  
  /**
   * Learn from command history
   */
  async learn(sessionId: string, history: TerminalCommand[]): Promise<void> {
    const learning = this.getOrCreateSessionLearning(sessionId);
    
    // Build command sequences
    for (let i = 0; i < history.length - 1; i++) {
      const sequence: CommandSequence = {
        commands: [],
        frequency: 1,
        context: this.extractContext(history[i].command),
        success: true
      };
      
      // Build sequence of 2-5 commands
      for (let j = 0; j < 5 && i + j < history.length; j++) {
        sequence.commands.push(history[i + j].command);
        if (history[i + j].exit_code !== 0) {
          sequence.success = false;
        }
      }
      
      // Add to sequences
      this.addOrUpdateSequence(learning.commandSequences, sequence);
    }
    
    // Update contextual suggestions
    this.updateContextualSuggestions(learning, history);
  }
  
  /**
   * Export learned patterns
   */
  exportPatterns(): CommandPattern[] {
    const patterns: CommandPattern[] = [];
    
    this.globalPatterns.forEach((pattern, command) => {
      patterns.push({
        pattern: command,
        frequency: pattern.frequency,
        typical_sequence: [command],
        context: 'global',
        automation_potential: this.calculatePatternAutomation(pattern)
      });
    });
    
    return patterns;
  }
  
  /**
   * Import patterns
   */
  async importPatterns(patterns: CommandPattern[]): Promise<void> {
    patterns.forEach(pattern => {
      const learnedPattern: LearnedPattern = {
        pattern: pattern.pattern,
        frequency: pattern.frequency,
        contexts: new Map([['imported', pattern.frequency]]),
        successRate: 0.9, // Assume imported patterns are good
        averageDuration: 0,
        lastUsed: new Date(),
        category: this.categorizeCommand(pattern.pattern)
      };
      
      this.globalPatterns.set(pattern.pattern, learnedPattern);
    });
  }
  
  // Private methods
  
  private getOrCreateSessionLearning(sessionId: string): SessionLearning {
    if (!this.sessionLearnings.has(sessionId)) {
      this.sessionLearnings.set(sessionId, {
        sessionId,
        patterns: new Map(),
        commandSequences: [],
        contextualSuggestions: new Map()
      });
    }
    
    return this.sessionLearnings.get(sessionId)!;
  }
  
  private createPattern(command: string): LearnedPattern {
    return {
      pattern: command,
      frequency: 0,
      contexts: new Map(),
      successRate: 1,
      averageDuration: 0,
      lastUsed: new Date(),
      category: this.categorizeCommand(command)
    };
  }
  
  private extractBaseCommand(command: string): string {
    // Extract the base command without arguments
    const parts = command.trim().split(/\s+/);
    return parts[0];
  }
  
  private extractContext(command: string): string {
    // Simple context extraction - in production, would be more sophisticated
    if (command.includes('git')) return 'git';
    if (command.includes('npm') || command.includes('node')) return 'node';
    if (command.includes('docker')) return 'docker';
    if (command.includes('cd') || command.includes('ls')) return 'navigation';
    return 'general';
  }
  
  private categorizeCommand(command: string): CommandCategory {
    const cmd = command.toLowerCase();
    
    if (cmd.includes('git')) return 'git';
    if (cmd.includes('docker')) return 'docker';
    if (cmd.includes('kubectl') || cmd.includes('k8s')) return 'kubernetes';
    if (cmd.includes('npm') || cmd.includes('yarn') || cmd.includes('node')) return 'development';
    if (cmd.includes('ls') || cmd.includes('cd') || cmd.includes('mkdir')) return 'file_management';
    if (cmd.includes('ps') || cmd.includes('kill') || cmd.includes('top')) return 'process_control';
    if (cmd.includes('curl') || cmd.includes('wget') || cmd.includes('ping')) return 'network';
    if (cmd.includes('mysql') || cmd.includes('psql') || cmd.includes('mongo')) return 'database';
    
    return 'development';
  }
  
  private updateGlobalPattern(command: string, pattern: LearnedPattern): void {
    const global = this.globalPatterns.get(command) || this.createPattern(command);
    
    global.frequency += 1;
    global.lastUsed = pattern.lastUsed;
    pattern.contexts.forEach((count, context) => {
      global.contexts.set(context, (global.contexts.get(context) || 0) + 1);
    });
    
    this.globalPatterns.set(command, global);
  }
  
  private findFrequentSequences(sequences: CommandSequence[]): CommandSequence[] {
    // Find sequences that appear more than once
    const sequenceMap = new Map<string, CommandSequence>();
    
    sequences.forEach(seq => {
      const key = seq.commands.join('|');
      const existing = sequenceMap.get(key);
      
      if (existing) {
        existing.frequency++;
      } else {
        sequenceMap.set(key, { ...seq });
      }
    });
    
    return Array.from(sequenceMap.values())
      .filter(seq => seq.frequency > 1)
      .sort((a, b) => b.frequency - a.frequency);
  }
  
  private calculateAutomationPotential(sequence: CommandSequence): number {
    let potential = 0;
    
    // High frequency increases potential
    if (sequence.frequency > 10) potential += 0.3;
    else if (sequence.frequency > 5) potential += 0.2;
    else if (sequence.frequency > 2) potential += 0.1;
    
    // Success rate affects potential
    if (sequence.success) potential += 0.2;
    
    // Longer sequences have higher potential
    if (sequence.commands.length > 3) potential += 0.3;
    else if (sequence.commands.length > 2) potential += 0.2;
    
    // Specific contexts increase potential
    if (['git', 'docker', 'deployment'].includes(sequence.context)) {
      potential += 0.2;
    }
    
    return Math.min(potential, 1.0);
  }
  
  private calculatePatternAutomation(pattern: LearnedPattern): number {
    let potential = 0;
    
    if (pattern.frequency > 20) potential += 0.4;
    else if (pattern.frequency > 10) potential += 0.3;
    else if (pattern.frequency > 5) potential += 0.2;
    
    if (pattern.successRate > 0.9) potential += 0.3;
    else if (pattern.successRate > 0.8) potential += 0.2;
    
    if (pattern.contexts.size > 1) potential += 0.1;
    
    return Math.min(potential, 1.0);
  }
  
  private addOrUpdateSequence(sequences: CommandSequence[], newSeq: CommandSequence): void {
    const existing = sequences.find(seq => 
      seq.commands.length === newSeq.commands.length &&
      seq.commands.every((cmd, i) => cmd === newSeq.commands[i])
    );
    
    if (existing) {
      existing.frequency++;
    } else {
      sequences.push(newSeq);
    }
  }
  
  private updateContextualSuggestions(learning: SessionLearning, history: TerminalCommand[]): void {
    // Group commands by context
    const contextGroups = new Map<string, string[]>();
    
    history.forEach(cmd => {
      const context = this.extractContext(cmd.command);
      if (!contextGroups.has(context)) {
        contextGroups.set(context, []);
      }
      contextGroups.get(context)!.push(cmd.command);
    });
    
    // Find most common commands per context
    contextGroups.forEach((commands, context) => {
      const frequency = new Map<string, number>();
      
      commands.forEach(cmd => {
        frequency.set(cmd, (frequency.get(cmd) || 0) + 1);
      });
      
      const topCommands = Array.from(frequency.entries())
        .sort((a, b) => b[1] - a[1])
        .slice(0, 5)
        .map(([cmd]) => cmd);
      
      learning.contextualSuggestions.set(context, topCommands);
    });
  }
}

/**
 * Trie structure for command sequence prediction
 */
class SequenceTrie {
  private root: TrieNode = new TrieNode();
  private sessionNodes: Map<string, TrieNode> = new Map();
  
  addCommand(sessionId: string, command: string): void {
    if (!this.sessionNodes.has(sessionId)) {
      this.sessionNodes.set(sessionId, this.root);
    }
    
    const currentNode = this.sessionNodes.get(sessionId)!;
    const nextNode = currentNode.addChild(command);
    this.sessionNodes.set(sessionId, nextNode);
  }
  
  getSuggestions(sessionId: string, count: number): Array<{command: string; confidence: number}> {
    const currentNode = this.sessionNodes.get(sessionId) || this.root;
    
    return currentNode.getTopChildren(count).map(child => ({
      command: child.command,
      confidence: child.frequency / currentNode.totalFrequency
    }));
  }
}

class TrieNode {
  command: string = '';
  frequency: number = 0;
  totalFrequency: number = 0;
  children: Map<string, TrieNode> = new Map();
  
  addChild(command: string): TrieNode {
    if (!this.children.has(command)) {
      this.children.set(command, new TrieNode());
    }
    
    const child = this.children.get(command)!;
    child.command = command;
    child.frequency++;
    this.totalFrequency++;
    
    return child;
  }
  
  getTopChildren(count: number): TrieNode[] {
    return Array.from(this.children.values())
      .sort((a, b) => b.frequency - a.frequency)
      .slice(0, count);
  }
}