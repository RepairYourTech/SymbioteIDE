# AI Model Selection Quick Reference
## AI Master Tool - Provider Selection Guide

**Version:** 1.0  
**Date:** January 2025  
**Purpose:** Quick reference for selecting optimal AI models

---

## Model Selection by Use Case

### 🎯 For Orchestration (Managing Other Models)

#### ✅ RECOMMENDED
| Model | Context | Cost/1M | Why Use |
|-------|---------|---------|---------|
| **Gemini 2.5 Pro** | 1M (2M soon) | $1.25/$10 | Best balance: huge context, multimodal, reasonable cost |
| **Qwen Turbo** | 1M | $0.05/$0.20 | Ultra-cheap with massive context |
| **Claude Opus 4** | 200K | $15/$75 | When quality matters more than cost |

#### ⚠️ USE WITH CAUTION
| Model | Context | Issue |
|-------|---------|-------|
| GPT-4 | 8K | Too small for orchestrating modern models |
| Doubao Lite | 256K | Too basic for complex orchestration |

#### ❌ AVOID
- Any model with <50K context for orchestration
- Budget models for complex multi-agent coordination

### 💻 For Code Generation

#### ✅ RECOMMENDED
| Model | Context | Cost/1M | Why Use |
|-------|---------|---------|---------|
| **Claude Sonnet 4** | 200K | $3/$15 | Best code quality, computer use |
| **DeepSeek V3** | 128K | $0.27/$1.10 | Excellent quality, 90% cheaper |
| **Kimi K2** | 130K | ~$1.07 | 71.6% SWE-bench score |

#### ⚠️ PROTECT OUTPUT
- When using Claude Sonnet 4, protect its output from modification by lesser models
- DeepSeek V3 output should also be protected when reviewed by budget models

### 🧠 For Reasoning & Analysis

#### ✅ RECOMMENDED
| Model | Context | Cost/1M | Why Use |
|-------|---------|---------|---------|
| **OpenAI o1** | 128K | $15/$60 | Best reasoning capability |
| **DeepSeek R1** | 128K | $0.55/$2.19 | Comparable to o1, much cheaper |
| **Gemini 2.0 Thinking** | Varies | Varies | Good reasoning with thinking mode |

### 🚀 For Speed-Critical Tasks

#### ✅ RECOMMENDED
| Model | Speed (t/s) | Cost/1M | Why Use |
|-------|-------------|---------|---------|
| **Groq (DeepSeek)** | 2,400 | Varies | Fastest available |
| **Cerebras (Llama 70B)** | 2,200 | $0.60 | Consistent speed |
| **Groq (Llama 8B)** | 750 | $0.05/$0.08 | Fast and cheap |

### 💰 For Budget-Conscious Tasks

#### ✅ RECOMMENDED
| Model | Context | Cost/1M | Why Use |
|-------|---------|---------|---------|
| **Doubao Lite** | 256K | $0.04 | Cheapest available |
| **Qwen Turbo** | 1M | $0.05/$0.20 | Massive context, ultra-cheap |
| **Local Ollama** | Varies | FREE | Zero cost, private |

### 🔒 For Privacy-Sensitive Tasks

#### ✅ RECOMMENDED
| Model | Type | Why Use |
|-------|------|---------|
| **Ollama** | Local | Completely private, no data leaves device |
| **LM Studio** | Local | GUI interface, easy model management |
| **Azure OpenAI** | Enterprise | Data zones, compliance features |
| **Bedrock VPC** | Enterprise | Isolated deployment |

## Context Window Compatibility Matrix

### ✅ Good Orchestrator-Worker Combinations

```yaml
good_combinations:
  - orchestrator: "gemini-2.5-pro" # 1M
    workers:
      - "claude-sonnet-4" # 200K - plenty of room
      - "deepseek-v3" # 128K - comfortable fit
      - "qwen-plus" # 128K - well matched
    
  - orchestrator: "claude-opus-4" # 200K
    workers:
      - "gpt-4o" # 128K - fits well
      - "deepseek-r1" # 128K - good match
      - "moonshot" # 130K - compatible
      
  - orchestrator: "qwen-turbo" # 1M
    workers:
      - ANY # Can handle anything!
```

### ❌ Bad Orchestrator-Worker Combinations

```yaml
bad_combinations:
  - orchestrator: "gpt-4" # 8K only!
    workers:
      - "gemini-2.5-pro" # 1M - massive mismatch
      - "qwen-turbo" # 1M - will lose context
    problem: "Orchestrator can't handle worker outputs"
    
  - orchestrator: "any-budget-model"
    workers:
      - "claude-opus-4" # Premium model
      - "openai-o1" # Reasoning model
    problem: "Orchestrator can't understand complex outputs"
```

## Quality Preservation Rules

### When to Protect Output

| Scenario | Rule | Example |
|----------|------|---------|
| Superior worker | NEVER modify | Claude Sonnet 4's code → protect from Gemini Flash |
| Specialized output | Preserve exactly | DeepSeek R1's reasoning → don't let budget model rewrite |
| Verified results | Lock output | Tested code → prevent any modifications |

### Quality Hierarchy (Highest to Lowest)

1. **Tier 1 - Premium**: Claude Opus 4, OpenAI o1
2. **Tier 2 - Professional**: Claude Sonnet 4, GPT-4.1, Gemini 2.5 Pro
3. **Tier 3 - Competent**: DeepSeek V3, Qwen Plus, Moonshot
4. **Tier 4 - Basic**: Gemini Flash, Qwen Turbo (for non-critical tasks)
5. **Tier 5 - Budget**: Doubao Lite, small local models

**Rule**: Never let lower-tier models modify higher-tier outputs!

## Cost Optimization Strategies

### Task-Based Routing

| Task Type | Recommended Models | Estimated Cost |
|-----------|-------------------|----------------|
| Simple Q&A | Doubao Lite | $0.00004/query |
| Code Review | DeepSeek V3 | $0.001/review |
| Complex Coding | Claude Sonnet 4 | $0.03/function |
| Documentation | Qwen Turbo | $0.0005/page |
| Reasoning | DeepSeek R1 | $0.002/problem |

### Monthly Budget Examples

```yaml
$50_budget:
  orchestrator: "qwen-turbo" # $0.05
  coder: "deepseek-v3" # $0.27
  reviewer: "deepseek-v3" # $0.27
  documenter: "doubao-lite" # $0.04
  
$500_budget:
  orchestrator: "gemini-2.5-pro" # $1.25
  coder: "claude-sonnet-4" # $3.00
  reviewer: "gpt-4o" # $2.50
  reasoner: "deepseek-r1" # $0.55
  
unlimited_budget:
  orchestrator: "claude-opus-4" # $15
  coder: "claude-sonnet-4" # $3.00
  reasoner: "openai-o1" # $15
  speed: "groq-premium" # Varies
```

## Quick Decision Tree

```
Need Orchestration?
├─ YES → Context Needed?
│  ├─ >500K → Gemini 2.5 Pro or Qwen Turbo
│  ├─ 100-500K → Claude Opus 4
│  └─ <100K → GPT-4o or Claude Sonnet
│
└─ NO → Task Type?
   ├─ Coding → Claude Sonnet 4 (best) or DeepSeek V3 (value)
   ├─ Reasoning → OpenAI o1 (best) or DeepSeek R1 (value)
   ├─ Speed Critical → Groq or Cerebras
   ├─ Budget → Doubao or Qwen Turbo
   └─ Privacy → Ollama or LM Studio
```

## Common Pitfalls to Avoid

1. **Using GPT-4 (8K) to orchestrate Gemini 2.5 (1M)** - Context overflow
2. **Letting Gemini Flash modify Claude Sonnet's code** - Quality degradation
3. **Using expensive models for simple tasks** - Waste of money
4. **Ignoring context window limits** - Data loss and errors
5. **Not protecting superior outputs** - Quality regression
6. **Mismatched speeds in parallel** - Bottlenecks (Cerebras waiting for GPT-4)

## Provider Switching Triggers

| Current Issue | Switch From | Switch To | Reason |
|--------------|-------------|-----------|---------|
| Context overflow | GPT-4 | Gemini 2.5 Pro | 125x more context |
| Too expensive | Claude Opus | DeepSeek V3 | 98% cheaper |
| Too slow | Any standard | Groq/Cerebras | 70x faster |
| Quality issues | Budget model | Claude/GPT-4 | Better output |
| Privacy concerns | Any cloud | Ollama | Local only |

---

**Remember**: The best configuration depends on your specific needs. Use this guide as a starting point, but always validate configurations before deployment!