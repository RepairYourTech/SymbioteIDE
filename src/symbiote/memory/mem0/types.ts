/**
 * Mem0 Memory System Types
 * 
 * Type definitions for the persistent inter-agent memory system
 */

/**
 * Memory types
 */
export enum MemoryType {
  Episodic = 'episodic',      // What happened - complete history
  Semantic = 'semantic',      // What was learned - patterns and insights
  Working = 'working',        // Current context - active task state
  LongTerm = 'long_term'      // Persistent knowledge - team conventions
}

/**
 * Memory scope
 */
export enum MemoryScope {
  Agent = 'agent',        // Individual agent memory
  Project = 'project',    // Project-specific memory
  Team = 'team',         // Team-shared memory
  Global = 'global'      // Organization-wide memory
}

/**
 * Base memory interface
 */
export interface Memory {
  id: string;
  type: MemoryType;
  scope: MemoryScope;
  content: string;
  metadata: MemoryMetadata;
  embedding?: number[];
  createdAt: Date;
  updatedAt: Date;
  accessedAt: Date;
  accessCount: number;
}

/**
 * Memory metadata
 */
export interface MemoryMetadata {
  // Identity
  agentId: string;
  userId?: string;
  teamId?: string;
  projectId?: string;
  sessionId?: string;
  
  // Task context
  taskId?: string;
  taskDescription?: string;
  taskType?: string;
  
  // Memory properties
  confidence: number;      // 0-1 confidence score
  importance: number;      // 0-1 importance score
  shareable: boolean;      // Can be shared with team
  
  // Code context
  language?: string;
  framework?: string;
  files?: string[];
  codeSnippets?: CodeSnippet[];
  
  // Learning context
  success?: boolean;
  errorMessage?: string;
  solution?: string;
  alternatives?: string[];
  
  // Relationships
  relatedMemories?: string[];
  parentMemory?: string;
  childMemories?: string[];
  
  // Neo4j integration
  graphNodeIds?: string[];
  
  // Qdrant integration
  vectorIds?: string[];
  
  // Custom metadata
  tags?: string[];
  labels?: Record<string, string>;
  extra?: Record<string, any>;
}

/**
 * Code snippet in memory
 */
export interface CodeSnippet {
  code: string;
  language: string;
  file?: string;
  startLine?: number;
  endLine?: number;
  purpose?: string;
}

/**
 * Agent memory - what an agent remembers
 */
export interface AgentMemory extends Memory {
  agentId: string;
  agentType: string;
  modelUsed?: string;
  promptUsed?: string;
  responseGenerated?: string;
  tokensUsed?: number;
  latency?: number;
}

/**
 * Task memory - memory about a specific task
 */
export interface TaskMemory extends Memory {
  taskId: string;
  taskDescription: string;
  approach: string;
  steps: TaskStep[];
  outcome: TaskOutcome;
  learnings: Learning[];
}

/**
 * Task step
 */
export interface TaskStep {
  order: number;
  action: string;
  result: string;
  success: boolean;
  duration?: number;
}

/**
 * Task outcome
 */
export interface TaskOutcome {
  success: boolean;
  result?: string;
  error?: string;
  metrics?: Record<string, number>;
}

/**
 * Learning extracted from memory
 */
export interface Learning {
  insight: string;
  type: 'pattern' | 'best_practice' | 'warning' | 'optimization';
  confidence: number;
  impact: 'high' | 'medium' | 'low';
  applicability: string[];  // Where this learning applies
}

/**
 * Memory search request
 */
export interface MemorySearchRequest {
  query?: string;
  type?: MemoryType[];
  scope?: MemoryScope[];
  agentId?: string;
  projectId?: string;
  teamId?: string;
  tags?: string[];
  dateRange?: {
    from?: Date;
    to?: Date;
  };
  limit?: number;
  includeEmbeddings?: boolean;
  minConfidence?: number;
  minImportance?: number;
}

/**
 * Memory search result
 */
export interface MemorySearchResult {
  memory: Memory;
  score: number;
  relevance: string;
  highlights?: string[];
}

/**
 * Task context for memory retrieval
 */
export interface TaskContext {
  taskId: string;
  description: string;
  type?: string;
  currentFile?: string;
  recentFiles?: string[];
  language?: string;
  framework?: string;
  previousAttempts?: number;
  constraints?: string[];
}

/**
 * Knowledge graph built from memories
 */
export interface KnowledgeGraph {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
  clusters: KnowledgeCluster[];
  insights: string[];
}

/**
 * Knowledge node
 */
export interface KnowledgeNode {
  id: string;
  type: 'concept' | 'pattern' | 'solution' | 'problem';
  label: string;
  description?: string;
  memories: string[];  // Memory IDs
  weight: number;
}

/**
 * Knowledge edge
 */
export interface KnowledgeEdge {
  source: string;
  target: string;
  type: 'leads_to' | 'similar_to' | 'opposite_of' | 'part_of' | 'requires';
  weight: number;
}

/**
 * Knowledge cluster
 */
export interface KnowledgeCluster {
  id: string;
  name: string;
  nodes: string[];
  theme: string;
  patterns: string[];
}

/**
 * Memory statistics
 */
export interface MemoryStats {
  totalMemories: number;
  byType: Record<MemoryType, number>;
  byScope: Record<MemoryScope, number>;
  totalAgents: number;
  totalProjects: number;
  averageConfidence: number;
  mostAccessedMemories: Memory[];
  recentMemories: Memory[];
  oldestMemories: Memory[];
}

/**
 * Memory decay configuration
 */
export interface MemoryDecayConfig {
  enabled: boolean;
  decayRate: number;         // How fast importance decreases
  minImportance: number;     // Minimum before removal
  consolidationThreshold: number;  // When to consolidate
  exemptTags?: string[];     // Tags that prevent decay
}

/**
 * Memory consolidation result
 */
export interface ConsolidationResult {
  originalMemories: string[];
  consolidatedMemory: Memory;
  extractedPatterns: Learning[];
  spaceSaved: number;
}

/**
 * Team knowledge
 */
export interface TeamKnowledge {
  teamId: string;
  conventions: CodingConvention[];
  bestPractices: BestPractice[];
  commonPatterns: CodePattern[];
  knownIssues: KnownIssue[];
  sharedLearnings: Learning[];
}

/**
 * Coding convention
 */
export interface CodingConvention {
  name: string;
  description: string;
  examples: CodeSnippet[];
  violations: CodeSnippet[];
  autoFixAvailable: boolean;
}

/**
 * Best practice
 */
export interface BestPractice {
  title: string;
  description: string;
  rationale: string;
  examples: CodeSnippet[];
  antiPatterns: CodeSnippet[];
  references: string[];
}

/**
 * Code pattern
 */
export interface CodePattern {
  name: string;
  description: string;
  useCase: string;
  implementation: CodeSnippet;
  variations: CodeSnippet[];
  frequency: number;
}

/**
 * Known issue
 */
export interface KnownIssue {
  issue: string;
  symptoms: string[];
  causes: string[];
  solutions: Solution[];
  preventions: string[];
}

/**
 * Solution
 */
export interface Solution {
  description: string;
  steps: string[];
  code?: CodeSnippet;
  successRate: number;
  caveats?: string[];
}

/**
 * Memory manager interface
 */
export interface IMemoryManager {
  // Store operations
  store(memory: Memory): Promise<string>;
  storeMany(memories: Memory[]): Promise<string[]>;
  
  // Retrieve operations
  get(id: string): Promise<Memory | null>;
  search(request: MemorySearchRequest): Promise<MemorySearchResult[]>;
  getRelated(memoryId: string, limit?: number): Promise<Memory[]>;
  
  // Update operations
  update(id: string, updates: Partial<Memory>): Promise<void>;
  incrementAccess(id: string): Promise<void>;
  
  // Delete operations
  delete(id: string): Promise<void>;
  deleteMany(ids: string[]): Promise<void>;
  
  // Learning operations
  extractLearnings(memoryIds: string[]): Promise<Learning[]>;
  consolidateMemories(memoryIds: string[]): Promise<ConsolidationResult>;
  
  // Team operations
  shareWithTeam(memoryId: string, teamId: string): Promise<void>;
  getTeamKnowledge(teamId: string): Promise<TeamKnowledge>;
  
  // Maintenance
  applyDecay(config: MemoryDecayConfig): Promise<number>;
  getStatistics(): Promise<MemoryStats>;
  optimize(): Promise<void>;
}

/**
 * Learning extractor interface
 */
export interface ILearningExtractor {
  extractFromTask(task: TaskMemory): Promise<Learning[]>;
  extractFromCode(before: string, after: string, context: TaskContext): Promise<Learning[]>;
  extractFromError(error: Error, context: TaskContext): Promise<Learning[]>;
  identifyPatterns(memories: Memory[]): Promise<CodePattern[]>;
}

/**
 * Memory event
 */
export interface MemoryEvent {
  type: 'created' | 'updated' | 'accessed' | 'shared' | 'consolidated' | 'deleted';
  memoryId: string;
  agentId?: string;
  timestamp: Date;
  details?: any;
}