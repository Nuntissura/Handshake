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
