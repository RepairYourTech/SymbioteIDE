# Ultimate Trader Agent Team (Plan-Only)

Status: Draft v0.1
Scope: Multi-agent trading system purpose, roles, connectors, flows, safety/compliance, hooks, slash commands, observability, acceptance, roadmap.

## Purpose in Symbiote
- Convert natural-language goals ("I put $100 in Coinbase, make me money") into safe, compliant, auditable trading outcomes
- Full lifecycle: Strategy → Simulation → Paper → Live → Monitoring → Adaptation → Reporting, under budgets/policies/approvals
- Orchestrated by the Personal Assistant; integrates with workflows, connectors, vault/egress, security, telemetry

## Roles (Agents)
- Strategy Research/Composer Agent
  - Drafts/adapts strategies; cites evidence; parameterizes constraints (budget, risk, time horizon)
- Backtest/Simulation Agent
  - Historical/regime tests; realistic fees/slippage/latency; walk-forward, cross-validation; overfit detectors
- Risk Manager Agent
  - Enforces guardrails: exposure, VaR, max drawdown, leverage, stop logic, position sizing; circuit breakers
- Execution Agent
  - Venue-aware routing (CEX/DEX), TWAP/VWAP/POV; partial fills; slippage/MEV protection; cancel/replace
- Portfolio/Rebalance Agent
  - Target allocations, drift handling; schedule/event-driven; tax-aware (future)
- Exchange/Wallet Agent
  - Exchange APIs (Binance, Coinbase, Kraken), WalletConnect/Ledger/MetaMask; custody ops, approvals
- DeFi Ops Agent
  - Swaps, LPing, staking, lending (Uniswap/Aave); route selection via quotes/sims; gas optimization
- Compliance/Policy Agent
  - KYC/region rules, token allow/deny lists, sanctions screening hooks, reporting
- Telemetry/Reporting Agent
  - P&L, risk metrics, slippage, incident reports; budgets; alerts; investor-grade PDFs

## Connectors & Data
- Market Data: exchange websockets/REST; DEX aggregators; on-chain RPC (Alchemy/Infura) via Egress policies
- Trading Venues: Binance, Coinbase, Kraken; Uniswap/Aave; bridges (future)
- Custody: WalletConnect, MetaMask, Ledger; key ops via Vault Daemon; no raw keys in agents
- Storage: SQLite for runs/metrics; time-series (local) for candles/trades; optional cloud sinks

## Flows
- Plan → Sim → Paper → Live
  1) Plan: Strategy Research/Composer proposes strategy + constraints
  2) Sim: Backtest/Simulation evaluates; Risk Manager signs off with thresholds
  3) Paper: Execution routes simulated orders; Portfolio agent manages allocations
  4) Live: With approvals + budgets; Compliance verifies jurisdiction/tokens
- Monitoring & Adaptation
  - Telemetry/Reporting tracks P&L, risk; Risk Manager adjusts sizes/stops; incidents trigger circuit_breaker

## Safety & Compliance
- Budgets: per-strategy/workspace; soft/hard caps; auto de-risking
- Approvals: live deploy, leverage, large order %, contract approvals require HiL
- Vault/Egress: SecretHandles only; TLS pinning; allowlists for venues and RPCs
- Isolation: workspace-scoped wallets, caches; optional hard isolation processes
- Kill Switch: one-tap pause/close positions; auto trigger on policy breaches

## Hooks
- strategy_proposed, backtest_completed, promotion_ready, order_submitted, order_filled, risk_breach, circuit_breaker, incident_resolved

## Slash Commands
- /trade plan "btc momentum" budget:100 risk:low horizon:1w
- /trade simulate strategy:alpha params:…
- /trade promote strategy:alpha —paper→live
- /trade go live strategy:alpha size:25%
- /trade kill strategy:alpha
- /trade report period:7d

## Observability & Budgets
- Metrics: P&L, Sharpe, max DD, exposure, slippage, fees; order life-cycle timings
- Budgets/costs: per strategy/workspace; alerts for breaches; anomaly detection
- Traces: decision lineage (inputs→decision→orders→fills); “no secrets” default

## Acceptance Criteria
- No live deploy without passing backtest thresholds; approvals captured and immutable
- Incident response (kill switch) < 30s; circuit breaker halts new risk when breached
- Order execution adheres to size/price/slippage constraints; reconciliation consistent with venue fills
- Full audit with redaction; reproducible backtests and paper/live parity within tolerance

## Roadmap
- v0.1: CEX (Coinbase, Binance), basic DeFi swaps (Uniswap), Strategy→Sim→Paper→Live, budgets/approvals/kill switch, P&L/risk dashboards
- v0.2: Advanced routing (TWAP/VWAP/POV), staking/lending (Aave), compliance expansions, auto-hedging
- v0.3: Multi-venue smart order routing, cross-chain strategies, tax-aware rebalancing, investor reporting suite

