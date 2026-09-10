export interface WorktreeEvidence {
  uncommitted: string[];
  untracked: string[];
  worktree: string;
}

export interface DemoOutcome {
  dispatch_id: string;
  journal_cursor: number;
  report: null | string;
  task_state: string;
  worktree: WorktreeEvidence;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isWorktreeEvidence(value: unknown): value is WorktreeEvidence {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.worktree === "string" &&
    isStringArray(value.uncommitted) &&
    isStringArray(value.untracked)
  );
}

export function parseDemoOutcome(payload: string): DemoOutcome {
  const parsed: unknown = JSON.parse(payload);
  if (!isRecord(parsed)) {
    throw new Error("demo outcome is not an object");
  }
  if (typeof parsed.dispatch_id !== "string") {
    throw new Error("demo outcome missing dispatch_id");
  }
  if (typeof parsed.journal_cursor !== "number" || !Number.isInteger(parsed.journal_cursor)) {
    throw new Error("demo outcome journal_cursor is not an integer");
  }
  if (parsed.report !== null && typeof parsed.report !== "string") {
    throw new Error("demo outcome report is not a string or null");
  }
  if (typeof parsed.task_state !== "string") {
    throw new Error("demo outcome missing task_state");
  }
  if (!isWorktreeEvidence(parsed.worktree)) {
    throw new Error("demo outcome missing worktree evidence");
  }
  return {
    dispatch_id: parsed.dispatch_id,
    journal_cursor: parsed.journal_cursor,
    report: parsed.report,
    task_state: parsed.task_state,
    worktree: parsed.worktree,
  };
}
