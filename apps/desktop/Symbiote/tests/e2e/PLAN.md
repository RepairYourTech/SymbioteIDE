# tests/e2e: Detailed Plan (v0.1)

Goals
- Playwright-based E2E validating UI flows across IDE, Trading, Workflow, Approvals, and Assistant Hub.

Structure
- projects/
  - assistant_hub.spec.ts
  - ide_flows.spec.ts
  - trading_flows.spec.ts
  - workflow_flows.spec.ts
- fixtures/: seed data, mock exchange sandbox endpoints (non-destructive)

Acceptance
- Critical flows pass on Win/macOS/Linux; trading kill-switch <30s; approvals enforced; no secret leakage in logs

