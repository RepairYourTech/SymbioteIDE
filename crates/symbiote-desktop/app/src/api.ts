import { invoke } from "@tauri-apps/api/core";

import { parseDemoOutcome, type DemoOutcome } from "./outcome";

export type { DemoOutcome, WorktreeEvidence } from "./outcome";
export { parseDemoOutcome } from "./outcome";

export function beginSession(
  stateDir: string,
  reservationBase: string,
  repository: string,
): Promise<string> {
  return invoke<string>("begin_session", {
    reservationBase,
    repository,
    stateDir,
  });
}

export function startDemo(): Promise<string> {
  return invoke<string>("start_demo");
}

export function readJournal(): Promise<number> {
  return invoke<number>("read_journal");
}

export async function finishDemo(dispatchId: string): Promise<DemoOutcome> {
  const payload = await invoke<string>("finish_demo", { dispatchId });
  return parseDemoOutcome(payload);
}

export function journalPosition(): Promise<number> {
  return invoke<number>("journal_position");
}

export function stopSession(): Promise<string> {
  return invoke<string>("stop_session");
}

/** Opens the untrusted Preview window for a finished run's output. The
 * window carries no Tauri IPC: worker content can never invoke commands. */
export function openPreview(
  dispatchId: string,
  report: string,
  worktree: string,
  files: string[],
): Promise<string> {
  return invoke<string>("open_preview", {
    dispatchId,
    files,
    report,
    worktree,
  });
}
