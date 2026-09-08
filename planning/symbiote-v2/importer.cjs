'use strict';
throw new Error('RETIRED_STALE_IMPORTER: historical payload predates accepted amendments; see #470. No remote writes permitted.');

const REVISION = '2026-09-03-v2';
const PLAN_LABEL = 'symbiote:plan-v2';
const MARKER = (key) => `<!-- symbiote-plan-key: ${key} -->`;
const REVISION_MARKER = `<!-- symbiote-plan-revision: ${REVISION} -->`;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const coreIssues = require('./issues-core.cjs');
const productIssues = require('./issues-product.cjs');
const clientIssues = require('./issues-clients.cjs');
const harnessCatalog = require('./harnesses.cjs');

const epicTitles = {
  A: 'Product truth, competitor intelligence, positioning, and terminology',
  B: 'Projects, multi-root workspaces, persistence, and resource lifecycle',
  C: 'Linux Host daemon, typed control plane, Fabric, and distributed placement',
  D: 'Harness integration substrate and native configuration projection',
  E: 'Individual harness configuration contracts and adapters',
  F: 'AI-firm orchestration, Team roles, canonical tasks, sessions, and artifacts',
  G: 'System Graph and Context Broker foundation',
  H: 'Graph-powered engineering applications and Impact Closure',
  I: 'Deep Guidance, competitor-gap discovery, ambiguity auditing, remediation, and evolution',
  J: 'Delivery Architecture, Capability Closure, and production readiness',
  K: 'Linux-first desktop workbench, Lead chat/voice, editor, Preview, and artifacts UX',
  L: 'Git, automatic worktrees, GitHub Issues/PRs/Actions, review, and merge policy',
  M: 'MCP, skills, plugins, secrets, agent environments, and extension ecosystem',
  N: 'Continuous diagnostics, security, recovery, observability, and conformance',
  O: 'iOS/Android clients, Symbiote Link, remote access, and cross-platform distribution',
};

const programBase = `# Symbiote v2 Program\n\nSymbiote is an open-source AI software-development firm. The user is the Client; a project-selected Lead Orchestrator converses and reasons; Symbiote owns canonical projects, roles, tasks, events, artifacts, context, worktrees, diagnostics, delivery, permissions, and provenance.\n\n## Program invariants\n\n- Role-first routing: Task → Role → Harness Profile → eligible Host.\n- Linux-first Host/client architecture; Windows, macOS, iOS, and Android are required targets.\n- The System Graph is a software digital twin and source of impact evidence.\n- Deep Guidance and ambiguity remediation are native state-machine/graph capabilities.\n- Native harness configuration is projected non-destructively from Symbiote desired state.\n- Capability Closure, not a passing task list, defines completion.\n- GitHub projects default to issue → task worktree → PR → review/checks → merge.\n- Branded terminology is optional presentation over canonical technical concepts.\n`;

function epicBase(letter) {
  return `# Epic ${letter}: ${epicTitles[letter]}\n\nThis tracker owns the dependency-ordered work listed below. Child issues are generated and maintained by the Symbiote v2 planning importer. The epic is complete only when every child issue is closed with its stated verification evidence and all downstream integration gates pass.\n`;
}

const labelDefinitions = {
  [PLAN_LABEL]: { color: '5319E7', description: 'Managed by the Symbiote v2 planning program' },
  'type:architecture': { color: '0052CC', description: 'Architecture or domain contract' },
  'type:research': { color: '7057FF', description: 'Evidence-gathering and decision work' },
  'type:feature': { color: '0E8A16', description: 'Product capability implementation' },
  'type:integration': { color: '1D76DB', description: 'External/runtime integration' },
  'type:security': { color: 'B60205', description: 'Security, trust, permissions, or privacy' },
  'type:quality': { color: 'D93F0B', description: 'Diagnostics, verification, or conformance' },
  'type:ux': { color: 'C2E0C6', description: 'Client/desktop/mobile user experience' },
  'type:infrastructure': { color: '006B75', description: 'Host, network, deployment, or platform infrastructure' },
  'priority:P0': { color: 'B60205', description: 'Foundational/launch-blocking priority' },
  'priority:P1': { color: 'D93F0B', description: 'Important follow-on priority' },
  'priority:P2': { color: 'FBCA04', description: 'Planned breadth/extension priority' },
};
for (let wave = 0; wave <= 7; wave += 1) {
  labelDefinitions[`wave:${wave}`] = { color: 'BFD4F2', description: `Symbiote implementation wave ${wave}` };
}
for (const letter of Object.keys(epicTitles)) {
  labelDefinitions[`epic:${letter}`] = { color: 'D4C5F9', description: `Symbiote epic ${letter}` };
}

function buildHarnessIssues(catalog) {
  const issues = [];
  for (const h of catalog) {
    const contractScope = [
      `Identify the exact installed binary/package identity, supported versions, version probe, upgrade channel, and platform constraints for ${h.name}.`,
      `Document every native global/user/project/local/admin/managed/environment/CLI configuration scope and the complete precedence order.`,
      `Document native instruction/rule/memory files and discovery boundaries: ${h.instructions}.`,
      `Document native skills, custom agents/subagents, commands/prompts/workflows, hooks, MCP, plugins/extensions, permissions, sandbox/trust, and any compatibility imports: ${h.capabilities}.`,
      `Document authentication, multiple-account/profile behavior, model inventory/selection, usage or quota signals, session storage/resume/fork behavior, and which state is portable versus machine-local.`,
      `Evaluate every structured control surface—SDK, app server, RPC/JSONL, ACP, headless mode, wire protocol, or structured CLI—and explicitly reject terminal scraping when a stable structured surface exists. Candidate: ${h.integration}.`,
      `Create a field-ownership map describing which native keys/files Symbiote may manage, how unknown user content is preserved, how comments/order are retained where possible, and how detach/rollback restores prior state.`,
      `Define role isolation strategy when two Symbiote roles use the same harness but require different instructions, tools, secrets, permissions, or accounts.`,
      `Record security boundaries and secret-materialization behavior, including what the harness can read implicitly outside project configuration.`,
      `Produce a support verdict: Native, Deep, Generic, Import/Bridge, or Unsupported, with explicit limitations and re-evaluation triggers.`,
      ...h.specific,
    ];
    const contractAcceptance = [
      `A versioned ${h.name} Configuration Contract exists with exact paths, precedence, trust conditions, and ownership semantics; no path or behavior is inherited from another harness by analogy.`,
      'Fixtures cover an empty environment, an existing heavily customized environment, conflicting scopes, multiple roles/profiles, upgrade/migration, detach, and rollback.',
      'Round-trip tests prove Symbiote-managed changes preserve unknown keys, unmanaged files, comments/order when the native format permits, and user-owned content.',
      'Capability claims distinguish officially supported behavior, experimentally probed behavior, and unavailable behavior.',
      'Official sources, retrieval date, tested versions, unresolved questions, and deprecation risk are recorded.',
      'The resulting contract is sufficient for an adapter implementer to proceed without inventing native configuration behavior.',
      ...h.acceptance,
    ];
    issues.push({
      key: h.key,
      title: `Research and specify the native ${h.name} Configuration Contract`,
      epic: 'E', wave: h.wave, priority: h.priority, type: 'research', deps: h.deps || ['D03'],
      outcome: `Produce the authoritative, versioned native-environment contract Symbiote needs to safely configure, launch, observe, and isolate ${h.name}. This issue is deliberately independent: ${h.name} must be researched from its own official implementation and documentation rather than forced into a lowest-common-denominator adapter model.`,
      scope: contractScope,
      invariants: [
        'Do not overwrite complete native configuration files when a narrower merge is possible.',
        'Do not claim support for a capability merely because another harness exposes an analogous feature.',
        'Shared AGENTS.md or .agents resources are used only when officially discovered and semantically safe for every affected role.',
      ],
      acceptance: contractAcceptance,
      evidence: [
        'Official documentation/repository citations for every material claim.',
        'Machine-readable contract fixture plus human-readable support matrix.',
        'Probe transcript or automated fixture showing actual installed-version behavior.',
        'Independent review by someone not responsible for the original research.',
      ],
      refs: h.refs,
      labels: ['type:research'],
    });

    if (h.adapter) {
      issues.push({
        key: h.adapter.key,
        title: `Implement and qualify the native ${h.name} adapter`,
        epic: 'E', wave: h.adapter.wave, priority: h.adapter.priority, type: 'integration', deps: [h.key, 'D01', 'D02', 'D05', 'D08', ...(h.adapter.deps || [])],
        outcome: `Implement production-grade ${h.name} support against its approved Configuration Contract using ${h.integration}. The adapter must expose honest capabilities to Symbiote, maintain real session identity, reconcile native configuration safely, and degrade explicitly when the installed version lacks a feature.`,
        scope: [
          'Detection, version probing, installation/update guidance, authentication/profile discovery, model inventory, and health.',
          'Start, resume, send, stream, interrupt, cancel, fork where supported, approval/input response, and terminal/process ownership.',
          'Normalize structured events without discarding provider-specific evidence needed for debugging or UI rendering.',
          'Integrate native configuration projection for instructions, tools, skills, agents, commands, hooks, permissions, settings, and secrets exactly as allowed by the approved contract.',
          'Report context window/model metadata, token or usage signals where available, quota/auth/provider failures, and retry/fallback eligibility.',
          'Support task-owned working directories/worktrees and Host placement without silently changing harness-global state.',
          'Implement capability/version negotiation so unsupported UI/actions are disabled rather than simulated unreliably.',
          'Add deterministic conformance fixtures, recorded event streams, fault cases, and a golden real-session smoke test.',
          ...h.adapter.specific,
        ],
        invariants: [
          'No PTY scraping for capabilities available through the approved structured interface.',
          'Harness session state is linked to—but never substitutes for—the canonical Symbiote task/session state.',
          'Provider output cannot directly mutate canonical work state except through validated Symbiote commands/events.',
        ],
        acceptance: [
          'The adapter passes the shared Harness Adapter Conformance Kit on Linux.',
          'A real task can start, stream, request approval/input, modify an isolated worktree, resume after client restart, and complete with structured artifacts.',
          'Existing customized native configuration survives install, role assignment, update, drift repair, and detach.',
          'Authentication expiry, quota exhaustion, process crash, malformed event, cancellation, and Host reconnect have deterministic states and recovery behavior.',
          'Capability metadata presented to Team routing and UI matches observed installed-version behavior.',
          'Security review confirms secrets and permissions are limited to the assigned role/profile/project/Host.',
          ...h.adapter.acceptance,
        ],
        evidence: [
          'Conformance test output and golden session recording.',
          'Configuration before/after/rollback fixtures.',
          'Failure-injection results.',
          'Independent adapter review and support-matrix update.',
        ],
        refs: h.refs,
        labels: ['type:integration'],
      });
    }
  }
  return issues;
}

function normalizeIssue(raw) {
  return {
    invariants: [], acceptance: [], evidence: [], refs: [], labels: [], outOfScope: [],
    deps: [], wave: 0, priority: 'P1', type: 'feature', ...raw,
  };
}

function topologicalSort(specs) {
  const byKey = new Map(specs.map((s) => [s.key, s]));
  if (byKey.size !== specs.length) throw new Error('Duplicate issue keys detected');
  for (const spec of specs) {
    for (const dep of spec.deps || []) {
      if (!byKey.has(dep) && !dep.startsWith('EPIC-') && dep !== 'PROGRAM') {
        throw new Error(`${spec.key} depends on missing key ${dep}`);
      }
    }
  }
  const temporary = new Set();
  const permanent = new Set();
  const output = [];
  function visit(key) {
    if (permanent.has(key)) return;
    if (temporary.has(key)) throw new Error(`Dependency cycle at ${key}`);
    temporary.add(key);
    const spec = byKey.get(key);
    for (const dep of spec.deps || []) if (byKey.has(dep)) visit(dep);
    temporary.delete(key);
    permanent.add(key);
    output.push(spec);
  }
  for (const spec of specs) visit(spec.key);
  return output;
}

function dependencyLines(spec, numberByKey) {
  if (!spec.deps.length) return '- None.';
  return spec.deps.map((key) => {
    const number = numberByKey.get(key);
    return number ? `- [ ] #${number} — ${key}` : `- [ ] ${key} — number assigned during import`;
  }).join('\n');
}

function bulletList(values) {
  return values.length ? values.map((v) => `- ${v}`).join('\n') : '- None.';
}

function acceptanceList(values) {
  return values.length ? values.map((v) => `- [ ] ${v}`).join('\n') : '- [ ] Outcome demonstrated with repeatable evidence.';
}

function renderIssue(spec, epicNumber, numberByKey) {
  const commonAcceptance = [
    'All public schemas, states, permissions, failure modes, and lifecycle transitions introduced by this issue are documented and tested.',
    'No hidden provider-, platform-, or single-machine assumption is introduced; unsupported cases fail explicitly.',
    'Diagnostics, audit provenance, migration/compatibility behavior, and rollback or safe-disable behavior are implemented where applicable.',
    'Relevant architecture, contract, user, operator, and extension documentation is updated in the same change.',
    'Independent review finds no unresolved blocking ambiguity and verifies the current head rather than an earlier revision.',
  ];
  const commonEvidence = [
    'Automated tests and exact commands/results.',
    'Screenshots, recordings, traces, fixtures, or generated artifacts appropriate to the capability.',
    'A requirements-to-implementation trace showing every acceptance criterion is resolved.',
  ];
  const body = `${MARKER(spec.key)}\n${REVISION_MARKER}\n\nParent epic: ${epicNumber ? `#${epicNumber}` : `EPIC ${spec.epic}`} — **${epicTitles[spec.epic]}**\nWave: **${spec.wave}**\nPriority: **${spec.priority}**\nType: **${spec.type}**\n\n## Outcome\n\n${spec.outcome}\n\n## Dependencies\n\n${dependencyLines(spec, numberByKey)}\n\n## Required scope\n\n${bulletList(spec.scope)}\n\n## Non-negotiable invariants\n\n${bulletList(spec.invariants)}\n\n## Acceptance criteria\n\n${acceptanceList([...spec.acceptance, ...commonAcceptance])}\n\n## Required verification evidence\n\n${bulletList([...spec.evidence, ...commonEvidence])}\n\n## Primary references / research starting points\n\n${spec.refs.length ? spec.refs.map((r) => `- ${r}`).join('\n') : '- Use current primary/official documentation and repository sources; record retrieval date and version.'}\n\n## Rollout and rollback\n\n- Introduce migrations, feature flags, compatibility shims, or version gates when existing persisted state or native configuration may be affected.\n- A failed rollout must leave canonical Symbiote state and user-owned repositories/configuration recoverable.\n- Document the exact disable, detach, downgrade, or restoration procedure before marking complete.\n\n## Explicitly out of scope\n\n${bulletList(spec.outOfScope)}\n\n## Implementation discipline\n\n- Work issue-first in a task-owned branch/worktree once the Git subsystem exists.\n- Read and cite the governing architecture/configuration contract before coding.\n- Do not invent product behavior to satisfy an implementation shortcut; raise or create a blocking decision/ambiguity.\n- Repair diagnostics during execution, not as a later cleanup iteration.\n- Recompute graph impact and Capability Closure before requesting review.\n`;
  if (body.length > 65000) throw new Error(`${spec.key} body is too large: ${body.length}`);
  return body;
}

function replaceManagedSection(body, name, markdown) {
  const start = `<!-- symbiote-managed:${name}:start -->`;
  const end = `<!-- symbiote-managed:${name}:end -->`;
  const section = `${start}\n${markdown.trim()}\n${end}`;
  const pattern = new RegExp(`${start.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}[\\s\\S]*?${end.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`);
  if (pattern.test(body)) return body.replace(pattern, section);
  return `${body.trim()}\n\n${section}\n`;
}

function issueMarker(body) {
  const match = String(body || '').match(/<!--\s*symbiote-plan-key:\s*([^\s]+)\s*-->/i);
  return match ? match[1] : null;
}

async function ensureLabels(github, owner, repo) {
  for (const [name, value] of Object.entries(labelDefinitions)) {
    try {
      await github.rest.issues.createLabel({ owner, repo, name, color: value.color, description: value.description });
    } catch (error) {
      if (error.status !== 422) throw error;
    }
  }
}

async function listAllIssues(github, owner, repo) {
  return github.paginate(github.rest.issues.listForRepo, { owner, repo, state: 'all', per_page: 100 });
}

async function ensureTracker({ github, owner, repo, existingByKey, key, title, baseBody, labels }) {
  const existing = existingByKey.get(key);
  const desiredPrefix = `${MARKER(key)}\n${REVISION_MARKER}\n\n`;
  if (!existing) {
    const created = await github.rest.issues.create({ owner, repo, title, body: `${desiredPrefix}${baseBody}`, labels });
    existingByKey.set(key, created.data);
    await sleep(250);
    return created.data;
  }
  const revisionPattern = /<!--\s*symbiote-plan-revision:[^>]*-->/i;
  let body = String(existing.body || '');
  if (!body.includes(MARKER(key))) body = `${MARKER(key)}\n${body}`;
  body = revisionPattern.test(body) ? body.replace(revisionPattern, REVISION_MARKER) : `${REVISION_MARKER}\n${body}`;
  if (!body.trim()) body = `${desiredPrefix}${baseBody}`;
  await github.rest.issues.update({ owner, repo, issue_number: existing.number, title, body, labels, state: 'open' });
  existingByKey.set(key, { ...existing, title, body, labels: labels.map((name) => ({ name })) });
  await sleep(150);
  return existingByKey.get(key);
}

module.exports = async ({ github, context, core }) => {
  const { owner, repo } = context.repo;
  core.info(`Importing Symbiote plan ${REVISION} into ${owner}/${repo}`);
  await ensureLabels(github, owner, repo);

  const existingIssues = await listAllIssues(github, owner, repo);
  const existingByKey = new Map();
  for (const item of existingIssues) {
    const key = issueMarker(item.body);
    if (key) existingByKey.set(key, item);
  }

  const program = await ensureTracker({
    github, owner, repo, existingByKey, key: 'PROGRAM',
    title: '[PROGRAM] Symbiote v2 — AI Development Firm platform roadmap',
    baseBody: programBase,
    labels: [PLAN_LABEL, 'type:architecture', 'priority:P0', 'wave:0'],
  });

  const epicByLetter = new Map();
  for (const letter of Object.keys(epicTitles)) {
    const tracker = await ensureTracker({
      github, owner, repo, existingByKey, key: `EPIC-${letter}`,
      title: `[EPIC ${letter}] ${epicTitles[letter]}`,
      baseBody: epicBase(letter),
      labels: [PLAN_LABEL, `epic:${letter}`, 'type:architecture'],
    });
    epicByLetter.set(letter, tracker);
  }

  const specs = topologicalSort([
    ...coreIssues,
    ...buildHarnessIssues(harnessCatalog),
    ...productIssues,
    ...clientIssues,
  ].map(normalizeIssue));

  const numberByKey = new Map();
  for (const [key, item] of existingByKey.entries()) numberByKey.set(key, item.number);
  for (const [letter, item] of epicByLetter.entries()) numberByKey.set(`EPIC-${letter}`, item.number);
  numberByKey.set('PROGRAM', program.number);

  let createdCount = 0;
  let updatedCount = 0;
  for (const spec of specs) {
    const epic = epicByLetter.get(spec.epic);
    const body = renderIssue(spec, epic && epic.number, numberByKey);
    const labels = [...new Set([PLAN_LABEL, `epic:${spec.epic}`, `wave:${spec.wave}`, `priority:${spec.priority}`, `type:${spec.type}`, ...spec.labels])];
    const existing = existingByKey.get(spec.key);
    if (existing) {
      const existingLabels = (existing.labels || []).map((label) => typeof label === 'string' ? label : label.name).sort();
      const desiredLabels = [...labels].sort();
      const changed = existing.title !== `[${spec.key}] ${spec.title}` || existing.body !== body || JSON.stringify(existingLabels) !== JSON.stringify(desiredLabels) || existing.state !== 'open';
      if (changed) {
        await github.rest.issues.update({ owner, repo, issue_number: existing.number, title: `[${spec.key}] ${spec.title}`, body, labels, state: 'open' });
        updatedCount += 1;
      }
      numberByKey.set(spec.key, existing.number);
    } else {
      const created = await github.rest.issues.create({ owner, repo, title: `[${spec.key}] ${spec.title}`, body, labels });
      existingByKey.set(spec.key, created.data);
      numberByKey.set(spec.key, created.data.number);
      createdCount += 1;
    }
    await sleep(300);
  }

  // Re-render after every dependency has a concrete issue number.
  for (const spec of specs) {
    const existing = existingByKey.get(spec.key);
    const body = renderIssue(spec, epicByLetter.get(spec.epic).number, numberByKey);
    if (existing.body !== body) {
      const labels = [...new Set([PLAN_LABEL, `epic:${spec.epic}`, `wave:${spec.wave}`, `priority:${spec.priority}`, `type:${spec.type}`, ...spec.labels])];
      await github.rest.issues.update({ owner, repo, issue_number: existing.number, body, labels });
      existing.body = body;
      updatedCount += 1;
      await sleep(180);
    }
  }

  for (const letter of Object.keys(epicTitles)) {
    const epic = epicByLetter.get(letter);
    const children = specs.filter((spec) => spec.epic === letter);
    const checklist = `## Managed child issues (${children.length})\n\n${children.map((spec) => `- [ ] #${numberByKey.get(spec.key)} — ${spec.key}: ${spec.title} (Wave ${spec.wave}, ${spec.priority})`).join('\n')}`;
    const body = replaceManagedSection(epic.body || `${MARKER(`EPIC-${letter}`)}\n${REVISION_MARKER}\n\n${epicBase(letter)}`, 'children', checklist);
    await github.rest.issues.update({ owner, repo, issue_number: epic.number, body });
    await sleep(180);
  }

  const epicChecklist = `## Managed epics\n\n${Object.keys(epicTitles).map((letter) => `- [ ] #${epicByLetter.get(letter).number} — Epic ${letter}: ${epicTitles[letter]}`).join('\n')}\n\n## Managed atomic issues\n\n**${specs.length} atomic issues** across 15 epics. The epic trackers contain dependency-ordered child checklists.`;
  const programBody = replaceManagedSection(program.body || `${MARKER('PROGRAM')}\n${REVISION_MARKER}\n\n${programBase}`, 'epics', epicChecklist);
  await github.rest.issues.update({ owner, repo, issue_number: program.number, body: programBody });

  const summary = [
    `Revision: ${REVISION}`,
    `Program: #${program.number}`,
    `Epics: ${epicByLetter.size}`,
    `Atomic issues: ${specs.length}`,
    `Created this run: ${createdCount}`,
    `Updated this run: ${updatedCount}`,
  ].join('\n');
  core.summary.addHeading('Symbiote v2 issue import').addCodeBlock(summary).write();
  core.notice(summary);
};
