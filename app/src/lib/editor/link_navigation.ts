// WP-KERNEL-009 iteration-3 (EXT-NAV-LINK-001) — typed link navigation intent.
//
// hsLink chips had NO click behavior at all (openOnClick:false on the Link
// mark, nothing replacing it): a typed [[wp:WP-KERNEL-009]] chip was a dead
// pixel. This module defines the NAVIGATION INTENT CONTRACT: clicking a chip
// dispatches a window-level CustomEvent carrying the typed ref. The workbench
// shell (MT-245/246/248 — tabs/split/document switcher) consumes this event and
// turns it into actual navigation. The event remains the stable machine-readable
// seam tests and tooling can assert on.

export const HS_LINK_NAVIGATE_EVENT = "hs:link-navigate" as const;

export interface HsLinkNavigateDetail {
  refKind: string;
  refValue: string;
  label: string;
}

/** Dispatches a typed navigation intent for a clicked hsLink chip. */
export function dispatchHsLinkNavigate(detail: HsLinkNavigateDetail): void {
  window.dispatchEvent(new CustomEvent<HsLinkNavigateDetail>(HS_LINK_NAVIGATE_EVENT, { detail }));
}

/** Typed subscription helper; returns the unsubscribe function. */
export function onHsLinkNavigate(
  listener: (detail: HsLinkNavigateDetail) => void,
): () => void {
  const handler = (event: Event) => {
    const custom = event as CustomEvent<HsLinkNavigateDetail>;
    if (custom.detail) listener(custom.detail);
  };
  window.addEventListener(HS_LINK_NAVIGATE_EVENT, handler);
  return () => window.removeEventListener(HS_LINK_NAVIGATE_EVENT, handler);
}

// WP-KERNEL-009 / MT-245 (EXT-NAV-LINK-001) — pure resolution of a typed hsLink
// to the in-app surface that owns it. App.tsx and the offline proof harness BOTH
// call this single function so the "typed link opens its real target; otherwise
// a typed visible error, never silent" contract is proven by exercising the same
// code, not a copy. The function is intentionally side-effect-free: callers map
// the returned target to the actual openX() pane action (or render the error).

const HS_LINK_LOOM_SOURCE_KINDS = new Set(["note", "loom_block", "file", "tag_hub", "journal"]);
const HS_LINK_DOCUMENT_SOURCE_KINDS = new Set(["document", "rich_document"]);

export type HsLinkTarget =
  | { kind: "document"; refValue: string }
  | { kind: "loom"; refValue: string }
  | { kind: "symbol"; refValue: string }
  | { kind: "wp"; refValue: string }
  | { kind: "mt"; refValue: string }
  | { kind: "wiki_page"; refValue: string }
  | { kind: "user_manual"; refValue: string }
  | { kind: "error"; message: string };

const normalizeHsLinkKind = (refKind: string) => refKind.trim().toLowerCase();
const normalizeHsLinkValue = (refValue: string) => refValue.trim();

const isHsLinkDocumentSource = (refKind: string, refValue: string) =>
  HS_LINK_DOCUMENT_SOURCE_KINDS.has(refKind) || (refKind === "note" && refValue.startsWith("KRD-"));

const isHsLinkLoomSource = (refKind: string, refValue: string) =>
  HS_LINK_LOOM_SOURCE_KINDS.has(refKind) && !(refKind === "note" && refValue.startsWith("KRD-"));

/**
 * Resolves a clicked hsLink to the in-app surface that owns it. A link with no
 * target value, or a typed kind no local surface owns, resolves to a typed
 * `error` — never a silent no-op.
 */
export function resolveHsLinkTarget(detail: HsLinkNavigateDetail): HsLinkTarget {
  const refKind = normalizeHsLinkKind(detail.refKind);
  const refValue = normalizeHsLinkValue(detail.refValue);
  if (!refValue) {
    return { kind: "error", message: `Cannot open ${detail.refKind || "link"}: the link has no target value.` };
  }
  if (isHsLinkDocumentSource(refKind, refValue)) return { kind: "document", refValue };
  if (isHsLinkLoomSource(refKind, refValue)) return { kind: "loom", refValue };
  if (refKind === "symbol") return { kind: "symbol", refValue };
  if (refKind === "wp") return { kind: "wp", refValue };
  if (refKind === "mt" || refKind === "micro_task") return { kind: "mt", refValue };
  if (refKind === "wiki_page") return { kind: "wiki_page", refValue };
  if (refKind === "user_manual" || refKind === "user_manual_page") return { kind: "user_manual", refValue };
  return {
    kind: "error",
    message: `Cannot resolve ${detail.refKind}:${detail.refValue} to a local Handshake surface.`,
  };
}
