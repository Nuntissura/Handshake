// WP-KERNEL-009 / MT-244 — CKC embed asset resolution (fail-closed).
//
// Resolves the typed media embed kinds of the hsLink node (MT-163:
// [[HS_images:…]], [[video:…]], [[album:…]], [[HS_slideshow:…]]) against the
// REAL backend asset surface that exists today (src/backend api/loom.rs):
//   GET /workspaces/:workspace_id/assets/:asset_id            → Asset metadata
//   GET /workspaces/:workspace_id/assets/:asset_id/content    → raw bytes + mime
//   GET /workspaces/:workspace_id/assets/:asset_id/thumbnail  → thumbnail bytes
//
// Backend gap (reported per the MT-244 contract, NOT mocked around): there is
// no backend album/slideshow LIST-SOURCE entity yet (no /albums endpoint, no
// collection table). Per the contract, album/slideshow embeds therefore resolve
// against the closest real surface: the node's refValue carries an ordered
// comma-separated sequence of REAL asset ids, each resolved individually
// through the asset metadata endpoint above. When a backend album entity
// lands, `parseAssetRefList` is the single seam to repoint.
//
// Everything here is FAIL-CLOSED (MT-152 spirit, mirrored from
// storage/knowledge.rs + knowledge_document/embed.rs):
//   - absolute filesystem paths (drive letters, leading slashes, UNC) are
//     rejected client-side before any request is made,
//   - `..` traversal and path separators in asset ids are rejected,
//   - http(s)/file/javascript schemes are rejected for asset refs,
//   - a metadata mime that does not match the embed kind (e.g. a video asset
//     in an [[HS_images:…]] embed) is a typed kind_mismatch error,
//   - network/404/server failures are typed errors — never a blank render,
//     never substituted mock data.
//
// Pure logic + injectable fetch so the resolution paths are unit-testable in
// jsdom without a backend; the default base URL is the real Handshake REST
// base (app/src/lib/api.ts).

import { API_BASE_URL } from "../api";

/** The hsLink refKinds that render as media embeds (MT-161 inventory kinds). */
export const MEDIA_EMBED_REF_KINDS = ["images", "video", "album", "slideshow"] as const;

export type MediaEmbedRefKind = (typeof MEDIA_EMBED_REF_KINDS)[number];

/** True when an hsLink refKind renders as a media embed NodeView. */
export function isMediaEmbedKind(refKind: string): refKind is MediaEmbedRefKind {
  return (MEDIA_EMBED_REF_KINDS as readonly string[]).includes(refKind);
}

/** Mirror of the backend Asset row (storage/loom.rs `Asset`). */
export interface EmbedAssetMetadata {
  asset_id: string;
  workspace_id: string;
  kind: string;
  mime: string;
  original_filename: string | null;
  content_hash: string;
  size_bytes: number;
  width: number | null;
  height: number | null;
}

/** Typed reasons an embed cannot resolve. Every reason renders visibly. */
export type EmbedErrorKind =
  | "no_workspace" // editor mounted without a workspace context
  | "empty_ref" // refValue empty/whitespace
  | "absolute_path_rejected" // MT-152: absolute/UNC/drive-letter path shape
  | "traversal_rejected" // `..` or path separators in an asset id
  | "scheme_rejected" // http(s)/file/javascript/data scheme in an asset id
  | "invalid_ref" // otherwise malformed asset id
  | "not_found" // backend 404
  | "forbidden" // backend 401/403
  | "server_error" // backend 5xx / malformed body
  | "network_error" // fetch rejected (backend unreachable)
  | "kind_mismatch" // asset mime does not match the embed kind
  | "media_load_failed"; // browser could not decode/play the resolved bytes

export type EmbedResolution =
  | { ok: true; asset: EmbedAssetMetadata; contentUrl: string }
  | { ok: false; errorKind: EmbedErrorKind; detail: string };

/** Workspace + transport context media embed NodeViews resolve against. */
export interface EmbedResolverContext {
  /** Workspace owning the assets (RichDocument.workspace_id). */
  workspaceId: string;
  /** REST base; defaults to the Handshake backend base from api.ts. */
  apiBaseUrl?: string;
  /** Injectable fetch (tests/harness); defaults to the runtime fetch. */
  fetchImpl?: (url: string) => Promise<Response>;
}

const ASSET_ID_MAX_LENGTH = 256;
/** UUID-shaped and similar opaque ids; NO path separators, colons, or spaces. */
const ASSET_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

/**
 * Validates a single asset ref fail-closed (client-side mirror of the backend
 * MT-152 absolute-path rejection). Returns null when valid, else a typed error.
 */
export function validateAssetRef(
  refValue: string,
): { errorKind: EmbedErrorKind; detail: string } | null {
  const value = refValue.trim();
  if (value.length === 0) {
    return { errorKind: "empty_ref", detail: "embed reference is empty" };
  }
  // Drive letter (C:\…, C:/…) or scheme (http://, file:, javascript:): both
  // carry ':' which a real asset id never does.
  if (value.includes(":")) {
    const looksDriveLetter = /^[A-Za-z]:[\\/]/.test(value);
    return looksDriveLetter
      ? {
          errorKind: "absolute_path_rejected",
          detail: `absolute path '${value}' is forbidden: embeds are asset ids, never machine-local paths`,
        }
      : {
          errorKind: "scheme_rejected",
          detail: `'${value}' carries a scheme; media embeds resolve workspace asset ids only`,
        };
  }
  if (value.startsWith("/") || value.startsWith("\\")) {
    return {
      errorKind: "absolute_path_rejected",
      detail: `absolute path '${value}' is forbidden: embeds are asset ids, never machine-local paths`,
    };
  }
  if (value.includes("/") || value.includes("\\") || value.includes("..")) {
    return {
      errorKind: "traversal_rejected",
      detail: `'${value}' contains path separators or traversal; embeds are opaque asset ids`,
    };
  }
  if (value.length > ASSET_ID_MAX_LENGTH || !ASSET_ID_PATTERN.test(value)) {
    return {
      errorKind: "invalid_ref",
      detail: `'${value}' is not a valid asset id`,
    };
  }
  return null;
}

/**
 * Parses an album/slideshow refValue into its ordered asset-id sequence.
 * Closest-real-surface contract (see module header): the sequence IS the
 * node's refValue as a comma-separated list of real asset ids.
 */
export function parseAssetRefList(refValue: string): string[] {
  return refValue
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

/** URL of the asset metadata endpoint (api/loom.rs get_asset_metadata). */
export function assetMetadataUrl(baseUrl: string, workspaceId: string, assetId: string): string {
  return `${baseUrl}/workspaces/${encodeURIComponent(workspaceId)}/assets/${encodeURIComponent(assetId)}`;
}

/** URL of the asset content endpoint (api/loom.rs get_asset_content). */
export function assetContentUrl(baseUrl: string, workspaceId: string, assetId: string): string {
  return `${assetMetadataUrl(baseUrl, workspaceId, assetId)}/content`;
}

/** Mime family expected per media embed kind (kind_mismatch is fail-closed). */
export function mimeMatchesEmbedKind(kind: MediaEmbedRefKind, mime: string): boolean {
  const normalized = mime.toLowerCase();
  switch (kind) {
    case "video":
      return normalized.startsWith("video/");
    case "images":
    case "album":
    case "slideshow":
      return normalized.startsWith("image/");
  }
}

function defaultFetch(url: string): Promise<Response> {
  // Centralized transport exception mirroring api.ts ownership: embed assets
  // resolve against the same backend base; tests/harness inject their own.
  return globalThis.fetch(url);
}

function isEmbedAssetMetadata(value: unknown): value is EmbedAssetMetadata {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record.asset_id === "string" &&
    typeof record.workspace_id === "string" &&
    typeof record.mime === "string" &&
    typeof record.content_hash === "string"
  );
}

/**
 * Resolves ONE media asset ref fail-closed: validate the ref shape, fetch the
 * REAL asset metadata, check the mime family against the embed kind, and
 * return the typed content URL the <img>/<video> element loads. Every failure
 * is a typed error result — never a throw, never a silent blank.
 */
export async function resolveEmbedAsset(
  kind: MediaEmbedRefKind,
  refValue: string,
  context: EmbedResolverContext,
): Promise<EmbedResolution> {
  if (!context.workspaceId || context.workspaceId.trim().length === 0) {
    return {
      ok: false,
      errorKind: "no_workspace",
      detail: "no workspace context: media embeds resolve workspace assets and need a workspace id",
    };
  }
  const invalid = validateAssetRef(refValue);
  if (invalid) return { ok: false, ...invalid };

  const baseUrl = context.apiBaseUrl ?? API_BASE_URL;
  const fetchImpl = context.fetchImpl ?? defaultFetch;
  const assetId = refValue.trim();
  const metadataUrl = assetMetadataUrl(baseUrl, context.workspaceId, assetId);

  let response: Response;
  try {
    response = await fetchImpl(metadataUrl);
  } catch (error) {
    return {
      ok: false,
      errorKind: "network_error",
      detail: `asset metadata request failed: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
  if (response.status === 404) {
    return { ok: false, errorKind: "not_found", detail: `asset '${assetId}' not found in workspace '${context.workspaceId}'` };
  }
  if (response.status === 401 || response.status === 403) {
    return { ok: false, errorKind: "forbidden", detail: `asset '${assetId}' is not accessible (HTTP ${response.status})` };
  }
  if (!response.ok) {
    return { ok: false, errorKind: "server_error", detail: `asset metadata request returned HTTP ${response.status}` };
  }

  let metadata: unknown;
  try {
    metadata = await response.json();
  } catch (error) {
    return {
      ok: false,
      errorKind: "server_error",
      detail: `asset metadata body is not JSON: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
  if (!isEmbedAssetMetadata(metadata)) {
    return { ok: false, errorKind: "server_error", detail: "asset metadata body is missing required fields" };
  }
  if (!mimeMatchesEmbedKind(kind, metadata.mime)) {
    return {
      ok: false,
      errorKind: "kind_mismatch",
      detail: `asset '${assetId}' is '${metadata.mime}', which does not match the '${kind}' embed kind`,
    };
  }
  return { ok: true, asset: metadata, contentUrl: assetContentUrl(baseUrl, context.workspaceId, assetId) };
}

/** Per-item result of resolving an album/slideshow sequence. */
export interface EmbedSequenceItem {
  refValue: string;
  resolution: EmbedResolution;
}

/**
 * DoS guard (MT-244 adversarial review): a hostile/corrupt document could
 * carry a sequence refValue with thousands of comma-separated ids, fanning
 * out one metadata request each. Sequences cap fail-closed at this length.
 */
export const MAX_SEQUENCE_ITEMS = 100;

/**
 * Resolves an ordered album/slideshow asset sequence. Items resolve
 * independently so one broken member renders as a typed per-item error while
 * the rest of the sequence still shows (fail-closed per item, not all-or-
 * nothing blanking). An empty sequence is itself a typed error; an oversized
 * sequence fails closed (MAX_SEQUENCE_ITEMS request-fan-out guard).
 */
export async function resolveEmbedSequence(
  kind: MediaEmbedRefKind,
  refValue: string,
  context: EmbedResolverContext,
): Promise<{ items: EmbedSequenceItem[] } | { ok: false; errorKind: EmbedErrorKind; detail: string }> {
  const refs = parseAssetRefList(refValue);
  if (refs.length === 0) {
    return { ok: false, errorKind: "empty_ref", detail: "album/slideshow embed has no asset ids" };
  }
  if (refs.length > MAX_SEQUENCE_ITEMS) {
    return {
      ok: false,
      errorKind: "invalid_ref",
      detail: `sequence has ${refs.length} members; the maximum is ${MAX_SEQUENCE_ITEMS}`,
    };
  }
  const items = await Promise.all(
    refs.map(async (ref) => ({ refValue: ref, resolution: await resolveEmbedAsset(kind, ref, context) })),
  );
  return { items };
}
