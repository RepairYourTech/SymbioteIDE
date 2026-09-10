// The desktop shell's UI wiring: every button calls one typed command.
// The commands are the controller's; no capability is granted here.
const { invoke } = window.__TAURI__.core;
const log = (line) => {
  const el = document.getElementById("log");
  el.textContent = `${new Date().toLocaleTimeString()}  ${line}\n${el.textContent}`;
};
const run = (id, action) => {
  document.getElementById(id).addEventListener("click", async () => {
    try {
      log(`${id}: ${await action()}`);
    } catch (error) {
      log(`${id}: FAILED ${error}`);
    }
  });
};
const value = (id) => document.getElementById(id).value;
run("begin", () =>
  invoke("begin_session", {
    stateDir: value("state-dir"),
    reservationBase: value("reservation-base"),
    repository: value("repository"),
  })
);
run("read-journal", () => invoke("read_journal"));
run("finish-demo", async () => {
  const outcome = JSON.parse(await invoke("finish_demo", { dispatchId: window._dispatchId ?? "" }));
  window._dispatchId = outcome.dispatch_id;
  return `state=${outcome.task_state} report=${JSON.stringify(outcome.report)} produced=${outcome.worktree.worktree}`;
});
run("journal-position", () => invoke("journal_position"));
run("stop", () => invoke("stop_session"));
// start_demo's result (the dispatch id) feeds finish_demo.
run("start-demo", async () => {
  const dispatchId = await invoke("start_demo");
  window._dispatchId = dispatchId;
  return `dispatch ${dispatchId}`;
});
