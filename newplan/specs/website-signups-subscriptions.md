# Symbiote Website: Signups, Authentication, Subscriptions (Plan-Only)

## Purpose
Provide a separate, clean plan for the public website handling user onboarding, authentication (consumer + orgs), subscriptions/billing, and account management — decoupled from the core Symbiote spec to keep the main plan uncluttered.

## Goals
- Fast, secure onboarding with Passkeys + OAuth
- Org and seat-based subscriptions with trials, coupons, and usage-based add‑ons
- Enterprise-ready (SSO/SAML, invoices, tax, regional data controls)
- Minimal surface area in repo: data lives in Supabase; code added later

## Personas
- Visitor: reads marketing, signs up
- User: joins/creates org, manages personal account
- Org Owner/Admin: manages seats, billing, policies
- Finance: invoices/receipts
- Support: refunds/credits

## Authentication
- Identity: Supabase Auth
  - Email + password, Magic Link, Passkeys (WebAuthn), OAuth (Google, GitHub, Microsoft, Apple)
  - MFA: TOTP and optionally SMS; recovery codes
  - Session: short‑lived access JWT + refresh; rotation and revocation lists
- Enterprise: SSO/SAML (Entra ID, Okta) — roadmap

## Account & Org Model
- Tables (Supabase; high‑level)
  - users (Supabase managed)
  - profiles (display name, avatar, preferences)
  - orgs (name, slug, owner_id, settings)
  - org_members (user_id, org_id, role: owner|admin|member|billing)
  - projects (org-scoped; optional for future portal integration)

## Plans & Entitlements
- Plan catalog
  - Free: limited features/quotas
  - Pro: single user, higher quotas
  - Team: seats, shared quotas, RBAC
  - Enterprise: custom, SSO/SAML, SLAs
- Entitlements (feature flags): per plan + overrides
  - Examples: Compare Panel, Model Ingestion, Local‑only mode, MCP agent, workflow concurrency
- Usage metrics (for metered billing): tokens, API calls, runs, storage, bandwidth

## Billing & Payments
- Processor: Stripe
  - Products/Prices: monthly/annual; seat‑based; trials; coupons; proration
  - Add‑ons: usage (per 1K tokens, per workflow run), extra seats
  - Taxes: Stripe Tax; customer location collection; reverse charge handling
  - Invoices & receipts; dunning (failed payments)
- Webhooks (secured by signature): checkout.session.completed, invoice.paid, customer.subscription.updated, usage.recorded
- Refunds/credits: controlled via admin tooling

## Compliance & Security
- Data: RLS on all org/user tables
- Secrets: Stripe keys in server; never in client
- PII: minimize; encryption at rest; audit logs for billing actions
- Regionalization (roadmap): project/org data residency flags

## Flows (UX)
- Signup → email confirm / passkey
- Create org (or join via invite)
- Checkout → select plan + seats + add‑ons → success → redirect to app
- Manage subscription: upgrade/downgrade, add seats, billing details, invoices
- Cancel: end of term or immediate (proration rules)
- Trial handling: auto‑end with reminders; card capture optional

## Data Model (Supabase sketch)
- subscriptions (org_id, status, plan, seats, period_start/end, stripe_customer_id, stripe_subscription_id)
- invoices (org_id, stripe_invoice_id, amount, currency, pdf_url)
- usage_events (org_id, metric, value, window_start/end, source)
- entitlements (org_id, feature_key, value, source: plan|override)
- audit_logs (actor, action, org_id, target, metadata, created_at)

## Admin
- Feature flags and entitlements editor
- Credits/adjustments
- Plan definitions editor (guarded)

## Integration with Symbiote
- Auth: share JWT; org membership claims
- Feature flags: delivered to app via /features endpoint or included in session claims
- Quotas: app records usage_events; nightly aggregation for billing

## Acceptance Criteria
- Signups with Passkeys + OAuth + MFA
- Org + seat management with invites
- Stripe checkout + webhooks → subscriptions table reflects truth within seconds
- RLS verified: users see only their org/account records
- Playwright E2E (later): signup → checkout → access gated feature → downgrade → lockout on expiry

## Roadmap (later)
- SSO/SAML, Enterprise invoicing, per‑region residency, SOC2 attestation


