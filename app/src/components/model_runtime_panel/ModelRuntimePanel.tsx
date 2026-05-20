import { CloudLanePanel } from "./CloudLanePanel";

// MT-129: Parent route for the ModelRuntime control panel surfaces.
// Mounts CloudLanePanel under /model-runtime/cloud-lane (currently
// the only section; future MTs add Local-Adapter, KV-Cache,
// Steering surfaces alongside).
//
// The panel is mounted from App.tsx as the "model-runtime" view
// tab and renders the full CloudLanePanel which composes
// ApiKeyVault + ConsentPromptModal + lane registration.

export function ModelRuntimePanel() {
  return (
    <section
      className="model-runtime-parent"
      data-testid="model-runtime-panel"
      aria-labelledby="model-runtime-panel-title"
    >
      <header className="model-runtime-parent__header">
        <h1 id="model-runtime-panel-title">Model Runtime</h1>
        <p className="muted" data-testid="model-runtime-panel.note">
          Configure cloud and local lanes for the ModelRuntime
          orchestrator. Cloud lanes use one operator-managed API key
          stored in the OS keychain (MT-128); local lanes use the
          per-host artifact registry.
        </p>
      </header>
      <CloudLanePanel />
    </section>
  );
}
