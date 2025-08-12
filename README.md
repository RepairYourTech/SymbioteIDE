# Symbiote (Fresh Start)

This repository has been reset to adopt the new unified plan. All legacy code has been removed. The plan and specifications live under `newplan/` and serve as the single source of truth until implementation lands.

## What’s here
- newplan/
  - SYMBIOTE_UNIFIED_SPECIFICATION.md — master spec (MCP, Model Catalog, Compare Panel, Low‑VRAM mode, etc.)
  - SYMBIOTE_COMPLETE_BUILD_PLAN.md, SYMBIOTE_FROM_SCRATCH_PLAN.md
  - specs/ — migrations and technical addenda (e.g., Supabase schema)

## Source of truth for provider/models
- We use Supabase as the canonical database for provider/model metadata.
- Clients periodically sync this data to local SQLite for offline and fast queries.

Schema migration file:
- Path: `newplan/specs/migrations/2025-08-12_model_catalog.sql`
- Apply using the Supabase SQL Editor in project: `SymbioteIDE (apgzyopdlcpwjjyebgtm)`

## Near-term components (scaffold-first)
1) Model Catalog Service (OpenRouter‑first)
   - Dynamic ingestion from OpenRouter `/api/v1/models` → Supabase tables
   - Local runners (Ollama, llama.cpp, LM Studio, vLLM, KoboldCpp, LiteLLM, Docker Runner)
   - Quantization + device capabilities (VRAM, CUDA/ROCm/Metal, CPU ISA)
   - Uses/capabilities taxonomy (coding, debugging, vision, ASR/TTS, embeddings, etc.)
2) Compare Panel (All providers & models)
   - Filters: Uses, Local, Context window, Quantization, Device, Price, Latency, Policy
   - Sorting presets: Free First, Best Rated, Lowest Cost, Largest Context, Fastest Latency, Best Value
   - Actions: Use in workspace, Try now, Benchmark, Save/Export
3) Selection & Routing
   - Capability‑ and policy‑aware routing (OpenRouter fallbacks, price caps, data policy)
   - Low‑VRAM mode (4–6GB GPUs) presets and safety checks
4) MCP Settings & Agent
   - Hierarchical global/workspace/project settings, hot reload, on/off toggles
   - JSON import adapters (Claude Desktop, Continue.dev, OpenAI MCP manifests, generic)
   - “Setup MCP for me” guided agent with approvals and audit

## Local SQLite sync (planned)
- Strategy: pull changes from Supabase by `updated_at`, upsert into SQLite; support user‑driven “Refresh Models.”
- Schema will mirror the `models`, `model_ratings`, `model_combinations` tables (subset as needed).

## Testing & CI (planned)
- Playwright E2E for Compare Panel, routing, Low‑VRAM flows, and MCP setup UX
- Unit tests for mappers (OpenRouter → Supabase) and catalog queries
- Trace artifacts and flake budget < 2%

## Contributing
- Follow `newplan/SYMBIOTE_UNIFIED_SPECIFICATION.md` as the contract.
- Implementation PRs should include tests and, when relevant, Supabase migrations.

## License
- TBA (add once decided).

