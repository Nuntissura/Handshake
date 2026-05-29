import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { AdapterCapabilities } from "./types";

type LoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; adapters: AdapterCapabilities[] };

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Failed to load sandbox adapters";
}

function boolLabel(value: boolean): string {
  return value ? "yes" : "no";
}

export function SandboxAdapterList() {
  const [state, setState] = useState<LoadState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    async function loadAdapters() {
      try {
        const adapters = await invoke<AdapterCapabilities[]>("kernel_sandbox_list_adapters");
        if (active) {
          setState({ status: "ready", adapters });
        }
      } catch (error) {
        if (active) {
          setState({ status: "error", message: errorMessage(error) });
        }
      }
    }

    void loadAdapters();

    return () => {
      active = false;
    };
  }, []);

  if (state.status === "loading") {
    return (
      <section aria-live="polite" data-state="loading" data-testid="sandbox-adapter-list">
        <p data-testid="sandbox-adapter-list.loading">Loading sandbox adapters...</p>
      </section>
    );
  }

  if (state.status === "error") {
    return (
      <section data-state="error" data-testid="sandbox-adapter-list">
        <p role="alert" data-testid="sandbox-adapter-list.error">
          Sandbox adapters unavailable: {state.message}
        </p>
      </section>
    );
  }

  if (state.adapters.length === 0) {
    return (
      <section data-state="empty" data-testid="sandbox-adapter-list">
        <p data-testid="sandbox-adapter-list.empty">No sandbox adapters registered.</p>
      </section>
    );
  }

  return (
    <section
      data-state="ready"
      data-testid="sandbox-adapter-list"
      aria-labelledby="sandbox-adapter-list-title"
    >
      <h3 id="sandbox-adapter-list-title">Sandbox Adapters</h3>
      <table data-testid="sandbox-adapter-list.table">
        <thead>
          <tr>
            <th>Adapter</th>
            <th>Isolation tier</th>
            <th>Filesystem isolation</th>
            <th>Network isolation</th>
            <th>GPU passthrough</th>
            <th>Stdio throughput</th>
            <th>Win32 fidelity</th>
            <th>Portable</th>
            <th>Nested virt</th>
            <th>Snapshot</th>
          </tr>
        </thead>
        <tbody>
          {state.adapters.map((adapter) => (
            <tr
              key={adapter.adapter_id}
              data-adapter-id={adapter.adapter_id}
              data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}`}
            >
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.adapter_id`}>
                <code>{adapter.adapter_id}</code>
              </td>
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.isolation_tier`}>
                {adapter.isolation_tier}
              </td>
              <td
                data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.filesystem_isolation_strength`}
              >
                {adapter.filesystem_isolation_strength}
              </td>
              <td
                data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.network_isolation_strength`}
              >
                {adapter.network_isolation_strength}
              </td>
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.gpu_passthrough`}>
                {adapter.gpu_passthrough}
              </td>
              <td
                data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.stdio_throughput_class`}
              >
                {adapter.stdio_throughput_class}
              </td>
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.win32_native_fidelity`}>
                {boolLabel(adapter.win32_native_fidelity)}
              </td>
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.cross_machine_portable`}>
                {boolLabel(adapter.cross_machine_portable)}
              </td>
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.requires_nested_virt`}>
                {boolLabel(adapter.requires_nested_virt)}
              </td>
              <td data-testid={`sandbox-adapter-list.row.${adapter.adapter_id}.supports_snapshot`}>
                {boolLabel(adapter.supports_snapshot)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
