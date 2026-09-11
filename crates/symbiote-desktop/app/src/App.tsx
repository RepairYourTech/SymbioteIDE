import { useState, type ChangeEvent, type ReactElement } from "react";

import {
  beginSession,
  finishDemo,
  journalPosition,
  openPreview,
  readJournal,
  startDemo,
  stopSession,
} from "./api";
import type { DemoOutcome } from "./outcome";

const DEFAULT_STATE_DIR = "/tmp/symbiote-desktop-demo/state";
const DEFAULT_RESERVATION_BASE = "/tmp/symbiote-desktop-demo/worktrees";
const DEFAULT_REPOSITORY = "/tmp/symbiote-desktop-demo/repo";

const FIXTURE_NOTICE =
  "The model turn in this demonstration is the operator's explicitly labeled fixture transport. A live one requires explicit authorization for credentials and billing — nothing is spent here.";

function formatLogLine(action: string, result: string): string {
  return `${new Date().toLocaleTimeString()}  ${action}: ${result}`;
}

function prependLog(existing: string, action: string, result: string): string {
  return `${formatLogLine(action, result)}\n${existing}`;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function formatOutcome(outcome: DemoOutcome): string {
  return `state=${outcome.task_state} report=${JSON.stringify(outcome.report)} produced=${outcome.worktree.worktree}`;
}

function inputValue(event: ChangeEvent<HTMLInputElement>): string {
  return event.target.value;
}

async function runAction(
  busy: boolean,
  setBusy: (next: boolean) => void,
  setLog: (update: (existing: string) => string) => void,
  action: string,
  work: () => Promise<string>,
): Promise<void> {
  if (busy) {
    return;
  }
  setBusy(true);
  try {
    const result = await work();
    setLog((existing) => prependLog(existing, action, result));
  } catch (error) {
    setLog((existing) => prependLog(existing, action, `FAILED ${errorMessage(error)}`));
  } finally {
    setBusy(false);
  }
}

export function App(): ReactElement {
  const [stateDir, setStateDir] = useState(DEFAULT_STATE_DIR);
  const [reservationBase, setReservationBase] = useState(DEFAULT_RESERVATION_BASE);
  const [repository, setRepository] = useState(DEFAULT_REPOSITORY);
  const [dispatchId, setDispatchId] = useState("");
  const [outcome, setOutcome] = useState<DemoOutcome | null>(null);
  const [log, setLog] = useState("");
  const [busy, setBusy] = useState(false);

  return (
    <main>
      <h1>Symbiote</h1>
      <p className="notice">{FIXTURE_NOTICE}</p>
      <fieldset>
        <legend>Open a repository</legend>
        <label htmlFor="state-dir">
          State directory
          <input
            id="state-dir"
            value={stateDir}
            disabled={busy}
            onChange={(event) => setStateDir(inputValue(event))}
          />
        </label>
        <label htmlFor="reservation-base">
          Worktree base
          <input
            id="reservation-base"
            value={reservationBase}
            disabled={busy}
            onChange={(event) => setReservationBase(inputValue(event))}
          />
        </label>
        <label htmlFor="repository">
          Repository
          <input
            id="repository"
            value={repository}
            disabled={busy}
            onChange={(event) => setRepository(inputValue(event))}
          />
        </label>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void runAction(busy, setBusy, setLog, "begin", () =>
              beginSession(stateDir, reservationBase, repository),
            );
          }}
        >
          Begin session
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void runAction(busy, setBusy, setLog, "stop", stopSession);
          }}
        >
          Stop
        </button>
      </fieldset>
      <fieldset>
        <legend>Demonstration</legend>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void runAction(busy, setBusy, setLog, "start-demo", async () => {
              const started = await startDemo();
              setDispatchId(started);
              return `dispatch ${started}`;
            });
          }}
        >
          Describe task &amp; start dispatch
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void runAction(busy, setBusy, setLog, "read-journal", async () =>
              String(await readJournal()),
            );
          }}
        >
          Read evidence trail
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void runAction(busy, setBusy, setLog, "finish-demo", async () => {
              const finished = await finishDemo(dispatchId);
              setDispatchId(finished.dispatch_id);
              setOutcome(finished);
              return formatOutcome(finished);
            });
          }}
        >
          Run &amp; read evidence
        </button>
        <button
          type="button"
          disabled={busy || outcome === null}
          onClick={() => {
            if (outcome === null) {
              return;
            }
            void runAction(busy, setBusy, setLog, "preview", () =>
              openPreview(
                outcome.report ?? "",
                outcome.worktree.worktree,
                [
                  ...outcome.worktree.untracked,
                  ...outcome.worktree.uncommitted,
                ],
              ),
            );
          }}
        >
          Preview run output
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void runAction(busy, setBusy, setLog, "journal-position", async () =>
              String(await journalPosition()),
            );
          }}
        >
          Journal position
        </button>
      </fieldset>
      <pre className="log" aria-live="polite">
        {log}
      </pre>
    </main>
  );
}
