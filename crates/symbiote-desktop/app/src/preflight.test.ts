import assert from "node:assert/strict";
import { test } from "node:test";

import {
  observationSummary,
  parsePreflightReport,
  withheldSurfaces,
} from "./preflight.ts";

function sample(overrides: Record<string, unknown> = {}): string {
  return JSON.stringify({
    status: "ready_for_preflight",
    profile_id: "engineer",
    checks: [
      {
        prerequisite: "runtime_surfaces",
        result: "satisfied",
      },
    ],
    activation_pending: ["environment_resolution"],
    candidates: [
      {
        profile: "engineer",
        profile_revision: 0,
        model: "coding-model",
        runtime: "native_symbiote",
        observation: {
          kind: "observed",
          surfaces: {
            surfaces: [
              {
                surface: "tools_and_mcp",
                preventive: {},
                carried: {
                  tools: { kind: "declared" },
                },
              },
              {
                surface: "hooks_and_events",
                preventive: {},
                carried: {
                  hooks: { kind: "absent" },
                },
              },
            ],
          },
        },
      },
      {
        profile: "fallback-engineer",
        profile_revision: 0,
        model: "coding-model",
        runtime: "external_harness",
        observation: { kind: "not_declared" },
      },
    ],
    ...overrides,
  });
}

test("parsePreflightReport reads candidates and keeps empty observations visible", () => {
  const report = parsePreflightReport(sample());
  assert.equal(report.status, "ready_for_preflight");
  assert.equal(report.candidates.length, 2);
  assert.deepEqual(withheldSurfaces(report.candidates[0]!), ["hooks_and_events"]);
  assert.equal(
    observationSummary(report.candidates[0]!),
    "withheld: hooks_and_events",
  );
  assert.deepEqual(withheldSurfaces(report.candidates[1]!), []);
  assert.match(observationSummary(report.candidates[1]!), /empty/);
});

test("parsePreflightReport rejects malformed candidates and observations", () => {
  assert.throws(() => parsePreflightReport("[]"), /not an object/);
  assert.throws(
    () => parsePreflightReport(sample({ status: "started" })),
    /invalid status/,
  );
  assert.throws(
    () =>
      parsePreflightReport(
        sample({
          candidates: [{ profile: "missing-the-rest" }],
        }),
      ),
    /invalid candidates/,
  );
  assert.throws(
    () =>
      parsePreflightReport(
        sample({
          candidates: [
            {
              profile: "engineer",
              profile_revision: 0,
              model: "coding-model",
              runtime: "native_symbiote",
              observation: { kind: "observed", surfaces: { surfaces: [{}] } },
            },
          ],
        }),
      ),
    /invalid candidates/,
  );
});
