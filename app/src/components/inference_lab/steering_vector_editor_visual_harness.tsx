// Real-component mount harness for MT-098 Playwright visual coverage.
//
// This mounts the shipped <SteeringVectorEditor> into Chromium. The only
// stand-in is deterministic Tauri IPC via mockIPC so the component can load
// registered vectors and drive the live generateAb IPC shape offline.
import "../../App.css";

import { createRoot } from "react-dom/client";
import { mockIPC } from "@tauri-apps/api/mocks";

import { SteeringVectorEditor } from "./SteeringVectorEditor";

declare global {
  interface Window {
    __HS_STEERING_GENERATE_AB_REQUEST__?: unknown;
  }
}

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";
const AFTER_VECTOR_ID = "019a1b2c-0000-7000-8000-000000000001";
const BEFORE_VECTOR_ID = "019a1b2c-0000-7000-8000-000000000002";

window.__HS_STEERING_GENERATE_AB_REQUEST__ = null;

mockIPC((cmd: string, args?: unknown) => {
  const payload = args as { request?: unknown } | undefined;
  switch (cmd) {
    case "kernel_model_runtime_steering_list_vectors":
      return [
        {
          vectorId: AFTER_VECTOR_ID,
          name: "calm-tone",
          layer: 14,
          hookPoint: "resid_stream",
          intensity: 1.5,
          description: "after vector",
        },
        {
          vectorId: BEFORE_VECTOR_ID,
          name: "direct-tone",
          layer: 12,
          hookPoint: "resid_stream",
          intensity: 0.75,
          description: "before vector",
        },
      ];
    case "kernel_model_runtime_steering_set_active":
      return {
        activeIds: ((payload?.request as { vectorIds?: string[] } | undefined)?.vectorIds ?? []),
        eventType: "FR-EVT-LLM-INFER-STEER-ACTIVE",
      };
    case "kernel_model_runtime_steering_unregister":
      return { eventType: "FR-EVT-LLM-INFER-STEER-UNREGISTER" };
    case "kernel_model_runtime_steering_capture":
      return {
        tokensSeen: 2,
        activationsByLayer: [{ layer: 12, activations: [[0.1, 0.2, 0.3]] }],
        eventType: "FR-EVT-LLM-INFER-STEER-CAPTURE",
      };
    case "kernel_model_runtime_steering_register_vector":
      return {
        vectorId: "019a1b2c-0000-7000-8000-000000000003",
        eventType: "FR-EVT-LLM-INFER-STEER-REGISTER",
      };
    case "kernel_model_runtime_steering_generate_ab": {
      const request = (payload?.request ?? {}) as {
        prompts?: string[];
        activeVectorIds?: string[];
        inactiveVectorIds?: string[];
      };
      window.__HS_STEERING_GENERATE_AB_REQUEST__ = request;
      return {
        comparisons: (request.prompts ?? []).map((prompt) => ({
          prompt,
          inactiveCompletion: `before:${(request.inactiveVectorIds ?? []).join(",")}:${prompt}`,
          activeCompletion: `after:${(request.activeVectorIds ?? []).join(",")}:${prompt}`,
        })),
        activeVectorIds: request.activeVectorIds ?? [],
        inactiveVectorIds: request.inactiveVectorIds ?? [],
        eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
      };
    }
    default:
      return null;
  }
});

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(
    <SteeringVectorEditor
      modelId={MODEL_ID}
      capabilities={{
        supportsLora: false,
        supportsKvPrefixCache: false,
        supportsKvQuantization: "none",
        supportsActivationSteering: true,
        supportsSubquadratic: false,
        supportsSpeculativeDraft: false,
        supportsEagle3: false,
      }}
      nLayers={32}
    />,
  );
}

export {};
