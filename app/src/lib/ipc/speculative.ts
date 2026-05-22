import { invoke } from "@tauri-apps/api/core";

// MT-109 SpeculativeMode mirrors src/backend/handshake_core/src/model_runtime/types.rs.
// serde(rename_all = "snake_case", tag = "mode") emits tagged enum
// objects: { mode: "ngram", lookback, max_draft } etc.
export type SpeculativeMode =
  | { mode: "ngram"; lookback: number; maxDraft: number }
  | { mode: "draft_model"; draftId: string; maxDraft: number }
  | { mode: "eagle3"; maxDraft: number };

export type SpeculativeModeValidation =
  | "disabled_ok"
  | { accepted: { mode: SpeculativeMode } };

export interface SpecSetModeRequest {
  modelId: string;
  mode: SpeculativeMode | null;
}

export interface SpecSetModeResult {
  modelId: string;
  eventType: string;
  previousMode: SpeculativeMode | null;
  currentMode: SpeculativeMode | null;
}

export interface SpecGetModeRequest {
  modelId: string;
}

export interface SpecGetModeResult {
  modelId: string;
  currentMode: SpeculativeMode | null;
}

export interface SpecValidateRequest {
  modelId: string;
  mode: SpeculativeMode | null;
}

export interface SpecValidateResult {
  modelId: string;
  validation: SpeculativeModeValidation;
}

export async function specSetMode(
  request: SpecSetModeRequest,
): Promise<SpecSetModeResult> {
  return await invoke<SpecSetModeResult>(
    "kernel_model_runtime_spec_set_mode",
    { request },
  );
}

export async function specGetMode(
  request: SpecGetModeRequest,
): Promise<SpecGetModeResult> {
  return await invoke<SpecGetModeResult>(
    "kernel_model_runtime_spec_get_mode",
    { request },
  );
}

export async function specValidate(
  request: SpecValidateRequest,
): Promise<SpecValidateResult> {
  return await invoke<SpecValidateResult>(
    "kernel_model_runtime_spec_validate",
    { request },
  );
}

// AC-INFER-LAB-UI-TOGGLES helper: shape the visible mode set per the
// model's declared capabilities. Eagle3 is always visible-but-disabled
// per operator E-4 ("post-merge dep upgrade after llama.cpp PR #18039").
export type SpecModeOption =
  | { kind: "none" }
  | { kind: "ngram" }
  | { kind: "draft_model" }
  | { kind: "eagle3_deferred"; note: string };

export function specModeOptions(
  supportsSpeculativeDraft: boolean,
  supportsEagle3: boolean,
): SpecModeOption[] {
  const options: SpecModeOption[] = [{ kind: "none" }];
  if (supportsSpeculativeDraft) {
    options.push({ kind: "ngram" });
    options.push({ kind: "draft_model" });
  }
  if (supportsEagle3) {
    options.push({ kind: "ngram" });
  } else {
    options.push({
      kind: "eagle3_deferred",
      note: "Eagle3 deferred until llama.cpp PR #18039 merges (operator E-4).",
    });
  }
  return options;
}
