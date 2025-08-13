# infra/ci: Detailed Plan (v0.1)

Goals
- Lint/build/test matrix; artifact retention; signing/SBOM; trading disclaimers ack for live tests.

Contents
- workflows: lint (md, rustfmt, clippy), build (features matrix), test (unit/integration/e2e), release (signing)

Acceptance
- All branches green; feature gates observed; no live trading in CI; artifacts available with retention policy

