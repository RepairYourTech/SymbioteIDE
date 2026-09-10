# symbiote-desktop

The Linux-first desktop shell: a Tauri 2 window over the headless
[`controller`](src/controller.rs). The workbench is a strict TypeScript/React
app in [`app/`](app/) that invokes the same typed commands the controller tests
drive against a real daemon.

The UI grants nothing. Worker completion remains evidence; Host verification
and independent review stay the completion gates. The demonstration's model
turn is the operator's explicitly labeled fixture transport. A live native or
Codex turn still requires explicit authorization for credentials and billing —
nothing is spent here.

```
cd crates/symbiote-desktop/app
npm ci --ignore-scripts
npm run typecheck
npm test
npm run build
```

`app/dist` is the committed frontend Tauri embeds. CI rebuilds it and fails if
the committed assets are stale. `withGlobalTauri` is off: the workbench uses
`@tauri-apps/api`, not `window.__TAURI__`.
