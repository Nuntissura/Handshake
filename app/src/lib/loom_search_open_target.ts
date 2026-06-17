import type { LoomGraphSearchHit } from "./api";

function metadataString(hit: LoomGraphSearchHit, key: string): string | null {
  if (!hit.metadata || typeof hit.metadata !== "object") return null;
  const value = (hit.metadata as Record<string, unknown>)[key];
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function blockDocumentId(hit: LoomGraphSearchHit): string | null {
  if (!hit.block || typeof hit.block !== "object") return null;
  const value = (hit.block as Record<string, unknown>).document_id;
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

export function documentIdFromLoomSearchHit(hit: LoomGraphSearchHit): string | null {
  const candidate =
    metadataString(hit, "rich_document_id") ??
    metadataString(hit, "document_id") ??
    blockDocumentId(hit) ??
    (hit.source_kind === "document" ? hit.ref_id.trim() : null);
  return candidate && candidate.startsWith("KRD-") ? candidate : null;
}

function userManualSlug(hit: LoomGraphSearchHit): string | null {
  if (hit.source_kind !== "user_manual_page") return null;
  return metadataString(hit, "page_slug") ?? hit.ref_id;
}

function firstIdMatch(value: string | null, pattern: RegExp): string | null {
  if (!value) return null;
  const match = value.match(pattern);
  return match?.[0] ?? null;
}

function workPacketId(hit: LoomGraphSearchHit): string | null {
  if (hit.source_kind !== "work_packet") return null;
  return (
    metadataString(hit, "work_packet_id") ??
    metadataString(hit, "wp_id") ??
    firstIdMatch(metadataString(hit, "entity_key"), /\bWP-[A-Za-z0-9-]+/i) ??
    firstIdMatch(hit.ref_id, /\bWP-[A-Za-z0-9-]+/i)
  );
}

function microTaskTarget(hit: LoomGraphSearchHit): { mtId: string; wpId?: string | null } | null {
  if (hit.source_kind !== "micro_task") return null;
  const mtId =
    metadataString(hit, "micro_task_id") ??
    metadataString(hit, "mt_id") ??
    firstIdMatch(metadataString(hit, "entity_key"), /\bMT-[A-Za-z0-9-]+/i) ??
    firstIdMatch(hit.ref_id, /\bMT-[A-Za-z0-9-]+/i);
  if (!mtId) return null;
  const wpId =
    metadataString(hit, "work_packet_id") ??
    metadataString(hit, "wp_id") ??
    metadataString(hit, "work_packet") ??
    firstIdMatch(metadataString(hit, "entity_key"), /\bWP-[A-Za-z0-9-]+/i);
  return { mtId, wpId };
}

export type LoomSearchOpenTarget =
  | { enabled: false; label: string; kind: "none" }
  | { enabled: true; label: string; kind: "code_symbol"; symbolEntityId: string }
  | { enabled: true; label: string; kind: "document"; documentId: string }
  | { enabled: true; label: string; kind: "loom_block"; blockId: string }
  | { enabled: true; label: string; kind: "micro_task"; target: { mtId: string; wpId?: string | null } }
  | { enabled: true; label: string; kind: "user_manual"; slug: string }
  | { enabled: true; label: string; kind: "wiki_page"; projectionId: string }
  | { enabled: true; label: string; kind: "work_packet"; wpId: string };

export function openTargetForLoomSearchHit(hit: LoomGraphSearchHit): LoomSearchOpenTarget {
  if (hit.source_kind === "wiki_page" && hit.ref_id.trim().length > 0) {
    return { enabled: true, label: "Open wiki page", kind: "wiki_page", projectionId: hit.ref_id.trim() };
  }
  const slug = userManualSlug(hit);
  if (slug) return { enabled: true, label: "Open UserManual page", kind: "user_manual", slug };
  const richDocumentId = documentIdFromLoomSearchHit(hit);
  if (richDocumentId) return { enabled: true, label: "Open document", kind: "document", documentId: richDocumentId };
  const documentId = hit.source_kind === "loom_block" ? blockDocumentId(hit) : null;
  if (documentId) return { enabled: true, label: "Open source document", kind: "document", documentId };
  if (hit.source_kind === "loom_block" && hit.ref_id.trim().length > 0) {
    return { enabled: true, label: "Open Loom block", kind: "loom_block", blockId: hit.ref_id.trim() };
  }
  if (hit.source_kind === "file" && hit.ref_id.trim().length > 0) {
    return { enabled: true, label: "Open file", kind: "loom_block", blockId: hit.ref_id.trim() };
  }
  if (hit.source_kind === "tag_hub" && hit.ref_id.trim().length > 0) {
    return { enabled: true, label: "Open tag hub", kind: "loom_block", blockId: hit.ref_id.trim() };
  }
  if (hit.source_kind === "symbol" && hit.ref_id.trim().length > 0) {
    return { enabled: true, label: "Open code symbol", kind: "code_symbol", symbolEntityId: hit.ref_id.trim() };
  }
  const wpId = workPacketId(hit);
  if (wpId) return { enabled: true, label: "Open Kernel DCC work packet", kind: "work_packet", wpId };
  const mtTarget = microTaskTarget(hit);
  if (mtTarget) return { enabled: true, label: "Open Kernel DCC microtask", kind: "micro_task", target: mtTarget };
  return { enabled: false, label: "No direct app target yet", kind: "none" };
}
