import { invoke } from "@tauri-apps/api/core";

// MT-129: Frontend IPC wrappers for the Cloud Lane Tauri commands.
// Every call round-trips through the production `OsKeychainSecretsVault`
// + `ConsentGate` types in the backend. No placeholder data; every
// store/rotate/delete/list call goes through the real Tauri IPC layer
// and hits the OS keychain.

export type CloudLaneKind = "openai_byok" | "anthropic_byok" | "official_cli";

export interface CloudLaneSummary {
  laneId: string;
  kind: CloudLaneKind;
  displayName: string;
  modelName: string;
  hasSecret: boolean;
  enabled: boolean;
  registeredAtUtc: string;
  secretUpdatedAtUtc: string | null;
}

export interface KeyMetadata {
  lane: string;
  hasSecret: boolean;
  updatedAtUtc: string | null;
}

export interface StoreReceipt {
  lane: string;
  action: string;
  operatorSignature: string;
  updatedAtUtc: string;
  eventType: string;
}

export interface ConsentReceipt {
  sessionId: string;
  lane: string;
  decision: string;
  decidedAtUtc: string;
  eventType: string;
}

export interface RegisterCloudLaneRequest {
  kind: CloudLaneKind;
  laneId: string;
  modelName: string;
}

export interface StoreApiKeyRequest {
  lane: string;
  secret: string;
  operatorSignature: string;
}

// ---------------------------------------------------------------------------
// Tauri command wrappers.
// ---------------------------------------------------------------------------

export async function listCloudLanes(): Promise<CloudLaneSummary[]> {
  return await invoke<CloudLaneSummary[]>("list_cloud_lanes");
}

export async function registerCloudLane(
  request: RegisterCloudLaneRequest,
): Promise<CloudLaneSummary> {
  return await invoke<CloudLaneSummary>("register_cloud_lane", { request });
}

export async function removeCloudLane(
  laneId: string,
  operatorSignature: string,
): Promise<void> {
  await invoke<void>("remove_cloud_lane", { laneId, operatorSignature });
}

export async function toggleCloudLane(
  laneId: string,
  enabled: boolean,
  operatorSignature: string,
): Promise<CloudLaneSummary> {
  return await invoke<CloudLaneSummary>("toggle_cloud_lane", {
    laneId,
    enabled,
    operatorSignature,
  });
}

export async function storeApiKey(
  request: StoreApiKeyRequest,
): Promise<StoreReceipt> {
  // NEVER log the request - the `secret` field contains the
  // operator's API key. The backend wraps it in `SecretString` on
  // arrival and the receipt does not echo it back.
  return await invoke<StoreReceipt>("store_api_key", { request });
}

export async function rotateApiKey(
  request: StoreApiKeyRequest,
): Promise<StoreReceipt> {
  return await invoke<StoreReceipt>("rotate_api_key", { request });
}

export async function deleteApiKey(
  lane: string,
  operatorSignature: string,
): Promise<StoreReceipt> {
  return await invoke<StoreReceipt>("delete_api_key", {
    lane,
    operatorSignature,
  });
}

export async function listStoredKeys(): Promise<KeyMetadata[]> {
  return await invoke<KeyMetadata[]>("list_stored_keys");
}

export async function grantConsent(
  sessionId: string,
  lane: string,
): Promise<ConsentReceipt> {
  return await invoke<ConsentReceipt>("grant_consent", { sessionId, lane });
}

export async function denyConsent(
  sessionId: string,
  lane: string,
): Promise<ConsentReceipt> {
  return await invoke<ConsentReceipt>("deny_consent", { sessionId, lane });
}

// ---------------------------------------------------------------------------
// Format helpers for the UI.
// ---------------------------------------------------------------------------

/** Format an RFC3339 timestamp as YYYY-MM-DD for the "stored on" indicator. */
export function formatStoredOnDate(updatedAtUtc: string | null): string {
  if (!updatedAtUtc) return "never";
  // Defensive parse: take only the date portion. The backend emits
  // chrono's `to_rfc3339` which is always `YYYY-MM-DDTHH:MM:SS...`.
  const dateMatch = updatedAtUtc.match(/^\d{4}-\d{2}-\d{2}/);
  return dateMatch ? dateMatch[0] : updatedAtUtc;
}

/** Tag for the per-lane consent prompt error path. The backend
 * surfaces a `ConsentDenied { session, lane }` error string when an
 * operator-denied (session, lane) pair is retried; the frontend
 * detects it via this prefix to know when to re-open the modal. */
export const CONSENT_REQUIRED_ERROR_PREFIX = "consent gate error:";
