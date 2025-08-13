# infra/docker: Detailed Plan (v0.1)

Goals
- Local stacks for Qdrant, Neo4j, Postgres, emulators (connectors) with safe defaults.

Contents
- compose files per profile; healthchecks; network isolation; resource caps

Acceptance
- One-command dev up; teardown clean; profiles respected; no external egress by default

