import assert from "node:assert/strict";
import { test } from "node:test";

import { parseDemoOutcome } from "./outcome.ts";

test("parseDemoOutcome accepts the controller's finish_demo payload", () => {
  const outcome = parseDemoOutcome(
    JSON.stringify({
      dispatch_id: "dispatch-1",
      journal_cursor: 7,
      report: "implemented the bounded change",
      task_state: "completion_requested",
      worktree: {
        uncommitted: ["produced.txt"],
        untracked: [],
        worktree: "/tmp/worktree",
      },
    }),
  );
  assert.equal(outcome.dispatch_id, "dispatch-1");
  assert.equal(outcome.journal_cursor, 7);
  assert.equal(outcome.report, "implemented the bounded change");
  assert.equal(outcome.task_state, "completion_requested");
  assert.equal(outcome.worktree.worktree, "/tmp/worktree");
  assert.deepEqual(outcome.worktree.uncommitted, ["produced.txt"]);
});

test("parseDemoOutcome rejects a payload that is not the DemoOutcome shape", () => {
  assert.throws(() => parseDemoOutcome("{}"), /missing dispatch_id/);
  assert.throws(() => parseDemoOutcome("[]"), /not an object/);
  assert.throws(
    () =>
      parseDemoOutcome(
        JSON.stringify({
          dispatch_id: "dispatch-1",
          journal_cursor: 1,
          report: 12,
          task_state: "completion_requested",
          worktree: { uncommitted: [], untracked: [], worktree: "/tmp" },
        }),
      ),
    /report is not a string or null/,
  );
});
