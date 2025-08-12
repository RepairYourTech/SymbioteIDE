# Agent Communication Protocol & Context Routing Rules
## AI Master Tool - Strict Agent Boundaries

**Version:** 1.0  
**Date:** January 2025  
**Status:** Critical Specification

---

## Core Principles

### 1. Output Sanctity Rule
**No agent may modify another agent's output unless explicitly designated as an editor/reviewer**

### 2. Context Minimalism Rule  
**Agents only pass specifically requested information, nothing more**

### 3. Role Clarity Rule
**Each agent has a clearly defined role and may not exceed it**

---

## Agent Communication Protocol

### 1. Strict Output Protection

```typescript
interface AgentOutputProtocol {
  // Every agent output is tagged and protected
  createOutput(agent: Agent, content: string): ProtectedOutput {
    return {
      id: generateUUID(),
      agentId: agent.id,
      agentRole: agent.role,
      content: content,
      metadata: {
        model: agent.model,
        timestamp: Date.now(),
        version: 1,
        checksum: sha256(content)
      },
      permissions: {
        canRead: ["*"], // Anyone can read
        canModify: agent.role === "editor" ? ["editor", "reviewer"] : [agent.id], // Only self or designated editors
        canDelete: ["orchestrator"], // Only orchestrator can remove
        canAnnotate: ["*"] // Anyone can add comments, not modify
      }
    };
  }
}

class AgentBoundaryEnforcer {
  // Enforce output protection
  processAgentInteraction(
    sourceAgent: Agent, 
    targetAgent: Agent, 
    output: ProtectedOutput
  ): ProcessedOutput {
    // Check if target agent has permission to modify
    if (targetAgent.attemptingToModify(output)) {
      if (!output.permissions.canModify.includes(targetAgent.role)) {
        throw new Error(
          `VIOLATION: ${targetAgent.id} cannot modify output from ${sourceAgent.id}. ` +
          `Only roles [${output.permissions.canModify}] are allowed.`
        );
      }
    }
    
    // If agent is allowed to read but not modify
    if (output.permissions.canRead.includes(targetAgent.role)) {
      return {
        ...output,
        accessMode: "READ_ONLY",
        warning: "This content is protected. You may analyze but not modify."
      };
    }
  }
}
```

### 2. Context Request Protocol

```typescript
interface ContextRequestProtocol {
  // Agents must explicitly request what they need
  requestContext(
    requestingAgent: Agent,
    contextNeeded: ContextRequest
  ): ContextResponse {
    return {
      requestId: generateUUID(),
      requestingAgent: requestingAgent.id,
      specification: {
        what: contextNeeded.dataTypes, // ["code", "tests", "documentation"]
        scope: contextNeeded.scope,     // ["current_file", "related_files"]
        depth: contextNeeded.depth,     // "summary" | "detailed" | "full"
        maxTokens: contextNeeded.maxTokens // Respect agent's context limit
      },
      timestamp: Date.now()
    };
  }
  
  // Context router only provides what was asked for
  routeContext(request: ContextRequest, availableContext: FullContext): RoutedContext {
    const filtered = {};
    
    // Only include requested data types
    for (const dataType of request.specification.what) {
      if (availableContext[dataType]) {
        filtered[dataType] = this.extractRequestedScope(
          availableContext[dataType],
          request.specification.scope,
          request.specification.depth
        );
      }
    }
    
    // Enforce token limit
    return this.truncateToTokenLimit(filtered, request.specification.maxTokens);
  }
}
```

### 3. Agent Role Definitions

```typescript
enum AgentRole {
  ORCHESTRATOR = "orchestrator",    // Can read all, modify none
  ARCHITECT = "architect",          // Creates designs, cannot modify code
  CODER = "coder",                 // Creates code, cannot modify designs
  REVIEWER = "reviewer",           // Can annotate, cannot modify
  EDITOR = "editor",               // ONLY role that can modify others' work
  TESTER = "tester",               // Creates tests, cannot modify code
  DOCUMENTER = "documenter",       // Creates docs, cannot modify code
  ANALYZER = "analyzer",           // Reads and reports, modifies nothing
}

interface RolePermissions {
  [AgentRole.ORCHESTRATOR]: {
    canCreate: ["plans", "task_assignments"],
    canModify: ["own_outputs"],
    canRead: ["everything"],
    mustNotModify: ["agent_outputs"]
  },
  
  [AgentRole.CODER]: {
    canCreate: ["code", "unit_tests"],
    canModify: ["own_code"],
    canRead: ["requirements", "designs", "related_code"],
    mustNotModify: ["other_agent_code", "designs", "documentation"]
  },
  
  [AgentRole.REVIEWER]: {
    canCreate: ["reviews", "annotations", "suggestions"],
    canModify: ["own_reviews"],
    canRead: ["everything"],
    mustNotModify: ["code", "designs", "tests"] // Can only annotate!
  },
  
  [AgentRole.EDITOR]: {
    canCreate: ["edited_versions"],
    canModify: ["designated_content"], // Only with explicit permission
    canRead: ["everything"],
    requiresPermission: true // Must have explicit permission to edit
  }
}
```

## Intelligent Context Routing

### 1. Context Request Specification

```typescript
class IntelligentContextRouter {
  // Agents must specify exactly what they need
  routeContextToAgent(
    agent: Agent,
    task: Task,
    availableContext: GlobalContext
  ): RoutedContext {
    // Analyze what the agent actually needs for this task
    const requirements = this.analyzeTaskRequirements(agent, task);
    
    // Build minimal context package
    const contextPackage = {
      essential: this.extractEssentials(requirements, availableContext),
      requested: this.extractRequested(agent.contextRequest, availableContext),
      prohibited: this.filterProhibited(agent.role),
      metadata: {
        totalTokens: 0,
        breakdown: {}
      }
    };
    
    // Validate package size
    if (contextPackage.totalTokens > agent.contextLimit) {
      return this.intelligentlyTruncate(contextPackage, agent.contextLimit);
    }
    
    return contextPackage;
  }
  
  // Prevent context bloat
  private filterUnnecessaryContext(
    context: RawContext,
    agentRole: AgentRole
  ): FilteredContext {
    const rules = {
      [AgentRole.CODER]: {
        remove: ["historical_discussions", "unrelated_modules", "documentation_drafts"],
        keep: ["current_module", "interfaces", "types", "tests"]
      },
      [AgentRole.TESTER]: {
        remove: ["implementation_details", "design_discussions", "documentation"],
        keep: ["interfaces", "expected_behavior", "edge_cases"]
      },
      [AgentRole.DOCUMENTER]: {
        remove: ["test_implementations", "internal_discussions", "draft_code"],
        keep: ["public_apis", "examples", "architecture"]
      }
    };
    
    return this.applyFilterRules(context, rules[agentRole]);
  }
}
```

### 2. Context Routing Examples

```typescript
// GOOD: Minimal, focused context routing
class GoodContextRouting {
  async routeToCodeReviewer(code: Code): Promise<ReviewContext> {
    return {
      // Only what the reviewer needs
      code: code.content,
      interfaces: code.publicInterfaces,
      tests: code.hasTests ? "Tests exist" : "No tests", // Just status, not full tests
      complexity: code.complexityMetrics,
      // NOT included: design docs, meeting notes, other modules
    };
  }
  
  async routeToTester(implementation: Implementation): Promise<TestContext> {
    return {
      // Only what the tester needs
      publicAPI: implementation.publicMethods,
      expectedBehavior: implementation.specifications,
      edgeCases: implementation.knownEdgeCases,
      // NOT included: implementation details, private methods, algorithms
    };
  }
}

// BAD: Passing everything
class BadContextRouting {
  async routeToCodeReviewer(project: EntireProject): Promise<ReviewContext> {
    return {
      everything: project, // 🚫 BAD! Reviewer doesn't need everything
      allDiscussions: project.discussions, // 🚫 Irrelevant
      allHistoricalCode: project.gitHistory, // 🚫 Too much
      everyDocument: project.allDocs // 🚫 Context overflow
    };
  }
}
```

### 3. Context Usage Tracking

```typescript
class ContextUsageTracker {
  trackAgentContextUsage(agent: Agent): ContextMetrics {
    return {
      requested: agent.contextRequests,
      received: agent.contextReceived,
      actuallyUsed: agent.contextAccessed, // What they actually looked at
      efficiency: agent.contextAccessed / agent.contextReceived,
      
      warnings: [
        agent.efficiency < 0.3 && "Agent receiving too much unused context",
        agent.contextReceived > agent.contextLimit * 0.9 && "Near context limit",
        agent.unusedDataTypes.length > 0 && `Never used: ${agent.unusedDataTypes}`
      ].filter(Boolean)
    };
  }
}
```

## Implementation Rules

### 1. Agent Boundary Enforcement

```typescript
class AgentBoundaryRules {
  static rules = {
    // Rule 1: No unsolicited modifications
    noUnsolicitedEdits: {
      enforce: (agent: Agent, output: Output) => {
        if (agent.isModifying(output) && !agent.hasEditPermission(output)) {
          throw new BoundaryViolation(
            `${agent.id} cannot modify ${output.owner}'s output`
          );
        }
      }
    },
    
    // Rule 2: No context pollution
    noContextPollution: {
      enforce: (agent: Agent, context: Context) => {
        const requested = agent.lastContextRequest;
        const provided = context.contents;
        
        for (const item of provided) {
          if (!requested.includes(item.type)) {
            throw new ContextViolation(
              `${item.type} was not requested by ${agent.id}`
            );
          }
        }
      }
    },
    
    // Rule 3: Role boundaries
    respectRoleBoundaries: {
      enforce: (agent: Agent, action: Action) => {
        const allowed = RolePermissions[agent.role].canCreate;
        if (!allowed.includes(action.type)) {
          throw new RoleViolation(
            `${agent.role} cannot perform ${action.type}`
          );
        }
      }
    }
  };
}
```

### 2. Context Request Templates

```typescript
// Predefined context request templates for common scenarios
const CONTEXT_REQUEST_TEMPLATES = {
  codeReview: {
    what: ["code", "interfaces", "complexity_metrics"],
    scope: ["current_file", "directly_related"],
    depth: "detailed",
    exclude: ["tests", "documentation", "history"]
  },
  
  unitTestCreation: {
    what: ["interfaces", "specifications", "examples"],
    scope: ["current_module"],
    depth: "full",
    exclude: ["implementation", "private_methods"]
  },
  
  documentation: {
    what: ["public_api", "examples", "architecture"],
    scope: ["current_module", "related_modules"],
    depth: "summary",
    exclude: ["tests", "private_implementation"]
  },
  
  bugFix: {
    what: ["error_trace", "related_code", "recent_changes"],
    scope: ["error_context", "call_stack"],
    depth: "detailed",
    exclude: ["unrelated_modules", "documentation"]
  }
};
```

### 3. Violation Handling

```typescript
class ViolationHandler {
  handleViolation(violation: Violation): Resolution {
    switch (violation.type) {
      case "UNAUTHORIZED_MODIFICATION":
        return {
          action: "BLOCK",
          message: `Agent ${violation.agent} cannot modify ${violation.target}'s output`,
          suggestion: "Request an Editor agent to make changes",
          logSeverity: "ERROR"
        };
        
      case "CONTEXT_OVERFLOW":
        return {
          action: "TRUNCATE",
          message: `Agent ${violation.agent} requested too much context`,
          suggestion: "Specify more focused context requirements",
          logSeverity: "WARNING"
        };
        
      case "ROLE_BOUNDARY_EXCEEDED":
        return {
          action: "REDIRECT",
          message: `Agent ${violation.agent} attempting action outside role`,
          suggestion: `Delegate to appropriate agent role`,
          logSeverity: "ERROR"
        };
    }
  }
}
```

## Configuration Examples

### Good Configuration

```yaml
good_agent_config:
  orchestrator:
    model: "gemini-2.5-pro"
    role: "orchestrator"
    permissions:
      - read: "*"
      - create: ["plans", "assignments"]
      - modify: ["own_outputs"]
    
  agents:
    architect:
      model: "claude-opus-4"
      role: "architect"
      permissions:
        - create: ["designs", "specifications"]
        - modify: ["own_designs"]
        - request_context: ["requirements", "constraints"]
      
    coder:
      model: "claude-sonnet-4"
      role: "coder"
      permissions:
        - create: ["code"]
        - modify: ["own_code"]
        - request_context: ["designs", "interfaces", "types"]
      output_protection: "ENFORCED" # No one can modify without permission
      
    reviewer:
      model: "deepseek-v3"
      role: "reviewer"
      permissions:
        - create: ["reviews", "annotations"]
        - read: ["code", "tests"]
        - annotate: ["*"]
        - modify: [] # Cannot modify anything!
```

### Bad Configuration

```yaml
bad_agent_config:
  orchestrator:
    model: "gpt-4"
    role: "orchestrator"
    permissions:
      - modify: ["*"] # 🚫 Orchestrator shouldn't modify agent outputs!
    
  agents:
    coder:
      model: "claude-sonnet-4"
      role: "coder"
      permissions:
        - modify: ["*"] # 🚫 Coder shouldn't modify everything!
        
    reviewer:
      model: "gemini-flash"
      role: "reviewer"
      permissions:
        - modify: ["code"] # 🚫 Reviewer shouldn't modify code!
        - receives_context: ["entire_project"] # 🚫 Too much context!
```

## Monitoring & Metrics

```typescript
interface AgentBoundaryMetrics {
  violationCount: {
    unauthorizedModifications: number,
    contextOverflows: number,
    roleBoundaryViolations: number
  },
  
  contextEfficiency: {
    averageRequestSize: number,
    averageUsedPercentage: number,
    wastedTokens: number
  },
  
  agentBehavior: {
    respectsBoundaries: boolean,
    efficientContextUse: boolean,
    properRoleExecution: boolean
  }
}
```

## Summary

These strict rules ensure:

1. **Quality Preservation**: Agents can't accidentally degrade each other's work
2. **Context Efficiency**: Only necessary information is passed between agents
3. **Clear Responsibilities**: Each agent knows exactly what it can and cannot do
4. **Reduced Errors**: Fewer context overflows and quality issues
5. **Better Performance**: Less wasted computation on unnecessary context

The system enforces these boundaries automatically, preventing common orchestration problems before they occur.

## Enhanced with Context Discovery System

**Important**: While agents receive minimal context by default, they have powerful discovery tools when more is needed. See [Agent Context Discovery System](agent-context-discovery-system.md) for:

### Discovery Tools Available to All Agents

```typescript
// When an agent needs more context
interface ContextDiscoveryTools {
  // Search the comprehensive codebase
  queryCodebase: (query: string) => Promise<CodebaseResults>;
  
  // Access project memory and patterns
  consultMemory: (need: MemoryQuery) => Promise<MemoryResults>;
  
  // Request help from Context Discovery Agent
  askContextAgent: (specification: DetailedNeed) => Promise<EnrichedContext>;
  
  // Access other agents' cached contexts
  checkCachedContexts: (criteria: CacheCriteria) => Promise<CachedContexts>;
  
  // Validate assumptions
  validateAssumptions: (assumptions: Assumptions) => Promise<Validation>;
}

// Example usage
class AgentWithDiscovery {
  async executeTask(task: Task) {
    // Start with minimal context
    let context = this.requestMinimalContext(task);
    
    // Need more? Use discovery tools
    if (this.needsMoreContext()) {
      // Search codebase
      const related = await this.tools.queryCodebase(
        `implementations similar to ${task.type}`
      );
      
      // Check what other agents know
      const cached = await this.tools.checkCachedContexts({
        phase: "architecture",
        relevant: task.requirements
      });
      
      // Still need more? Ask the Context Agent
      if (this.stillInsufficient()) {
        const enriched = await this.tools.askContextAgent({
          current: context,
          missing: this.identifyGaps(),
          task: task
        });
      }
    }
    
    return this.executeWithSufficientContext(context);
  }
}
```

### Checkpoint Integration

All context requests and discoveries are automatically checkpointed. See [Checkpoint & Rollback System](checkpoint-rollback-system.md) for:
- Micro-checkpoints of every context request
- Instant rollback if wrong context causes issues
- Quality gates ensuring 100% standards
- Complete audit trail of context flow

This ensures agents can be minimalist by default while having unlimited ability to discover what they need, all while maintaining perfect quality standards.