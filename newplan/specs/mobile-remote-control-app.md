# Symbiote Mobile Remote Control App (Plan-Only)

## Purpose
A companion mobile app that acts as a secure remote control and status console for the main Symbiote app. It does not duplicate the IDE; it orchestrates, approves, monitors, and triggers flows.

## Goals
- Secure out-of-band approvals (HiL) and notifications
- Start/stop agents, workflows, trades; view status and logs
- Switch "vibes" and permission profiles (ZeroTrust ↔ Full Auto)
- Quick actions: run saved commands, benchmarks, refresh model catalog
- Privacy-first; minimal data on device; push-based, not polling

## Architecture
- Client: Native (proposed) or cross‑platform; minimal UI; offline-capable
- Backend: Same auth and org model as website; uses Symbiote API with scoped tokens
- Realtime: WebSocket/EventSource for live status; push notifications for approvals/alerts
- Security: Device binding, passkeys, per-device revoke; end-to-end encryption for approval payloads

## Core Features
- Approvals & Alerts
  - Approve/reject high‑risk tool calls, deployments, crypto trades, data exfil attempts
  - Policy-aware: show rationale, diff, cost/risk estimates
  - Push notifications with deep links to action
- Agent/Workflow Control
  - Start/stop/pause/resume; set max budget/time; view live logs and metrics
  - Switch vibe and permission profile for the active workspace
  - Trigger saved workflows and one‑shot actions
- Status & Observability
  - Dashboards: agent performance, costs, token usage, P&L, build health
  - Incident mode: red alerts, one-tap kill switch
- Context & Catalog
  - Search models and providers; apply Compare Panel presets; set default model set for a workspace
  - MCP controls: enable/disable servers/tools; reload

## Authentication & Security
- Auth: Passkeys + OAuth; org invites; MFA
- Device binding: register device; sign approval requests; per-device revoke
- Scoped tokens: minimal privileges; time‑boxed approvals
- E2EE approvals payloads (optional) to avoid server seeing content of sensitive approvals

## Data Model (high-level)
- devices (user_id, device_public_key, platform, push_token, approved_scopes)
- approval_requests (id, type, payload_ref, expires_at, status)
- device_approvals (device_id, approval_request_id, decision, signed_at)
- notifications (user_id, message, severity, link, ack)

## Flows
- Enroll device → bind to account with passkey → grant scopes
- Receive push → open approval → view diff/cost → Approve/Reject (sign)
- Control workflow → set limits → monitor logs and status
- Toggle permission profile or vibe

## Integration with Main App
- Main app publishes approval_requests to backend; backend multiplexes to devices via push and WS
- Device responses (signed) update approval_requests; main app unblocks/block actions accordingly
- Settings changes (vibe/profile, MCP toggles) synced and audited

## Acceptance Criteria
- Device enrollment with passkeys and secure binding
- Approvals: receive push, open, view details, sign decision; main app reacts within seconds
- Control: start/stop agents/workflows; change vibe/profile; observe live status
- Security: device revoke invalidates tokens; approvals are E2E-encrypted (if enabled)
- Playwright (later) with mobile emulation + backend contract tests

## Roadmap
- Native integrations: biometrics for approvals, Siri/Assistant voice commands
- Offline queue: store-and-forward approvals; retry with backoff
- Geofencing-based policies (optional)

