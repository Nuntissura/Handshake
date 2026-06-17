// WP-KERNEL-009 / MT-244 — CKC embed NodeViews for the typed hsLink node.
//
// The React NodeView that makes the MT-163 typed [[kind:value]] nodes RENDER:
//   - [[HS_images:assetId]]      → a real <img> loading the backend asset bytes,
//   - [[video:assetId]]          → a real <video controls> for playback,
//   - [[album:id,id,…]] and [[HS_slideshow:id,id,…]] → sequenced image viewers
//     (prev/next + position) over an ordered set of real asset refs (the
//     closest real surface — see embed_assets.ts for the album list-source
//     backend gap report),
//   - every NON-media kind (note/file/folder/project/spec/wp/symbol/unknown)
//     keeps the exact MT-163 chip rendering (same selectors/classes/title) so
//     the existing link surface and its tests are unchanged.
//
// FAIL-CLOSED rendering states (never blank, never mock data):
//   - resolving:   data-testid="hs-embed-loading" visible progress text,
//   - resolved:    the real media element (data-testid="hs-embed-image" /
//                  "hs-embed-video" / "hs-embed-sequence"),
//   - failed:      data-testid="hs-embed-error" role="alert" with a typed
//                  data-error-kind (embed_assets.EmbedErrorKind) + detail text.
//
// The workspace/transport context arrives through the hsLink extension options
// (build_editor_extensions → HsLinkNode.configure({ embedContext })), so the
// document model and persistence stay byte-identical to MT-163 — rendering is
// the only thing this adds.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { NodeViewWrapper, type ReactNodeViewProps } from "@tiptap/react";
import {
  isMediaEmbedKind,
  resolveAssetTiers,
  resolveEmbedAsset,
  resolveEmbedSequence,
  retryAssetTier,
  type EmbedAssetTier,
  type EmbedErrorKind,
  type EmbedResolution,
  type EmbedResolverContext,
  type EmbedSequenceItem,
  type MediaEmbedRefKind,
} from "../lib/editor/embed_assets";
import { dispatchHsLinkNavigate } from "../lib/editor/link_navigation";
import { getLoomBlock, type LoomBlock } from "../lib/api";

type EmbedState =
  | { phase: "resolving" }
  | { phase: "resolved-single"; resolution: Extract<EmbedResolution, { ok: true }> }
  | { phase: "resolved-sequence"; items: EmbedSequenceItem[] }
  | { phase: "failed"; errorKind: EmbedErrorKind; detail: string };

type LoomPreviewState =
  | { phase: "loading" }
  | { phase: "resolved"; block: LoomBlock }
  | { phase: "failed"; detail: string };

/** The options surface the hsLink extension carries (see hs_link_node.ts). */
export interface HsLinkNodeOptions {
  embedContext: EmbedResolverContext | null;
}

const LOOM_LINK_PREVIEW_KINDS = new Set(["note", "file", "tag_hub", "loom_block", "journal"]);

function chipLabel(attrs: Record<string, unknown>): string {
  return String(attrs.label || `${attrs.refKind}:${attrs.refValue}`);
}

function canPreviewLoomLink(refKind: string): boolean {
  return LOOM_LINK_PREVIEW_KINDS.has(refKind);
}

export function HsLinkView(props: ReactNodeViewProps<HTMLElement>) {
  const { node, extension } = props;
  const refKind = String(node.attrs.refKind ?? "unknown");
  const refValue = String(node.attrs.refValue ?? "");
  const resolved = node.attrs.resolved !== false;
  const embedContext =
    (extension.options as HsLinkNodeOptions | undefined)?.embedContext ?? null;
  const [previewOpen, setPreviewOpen] = useState(false);

  if (!isMediaEmbedKind(refKind)) {
    const previewable =
      canPreviewLoomLink(refKind) &&
      Boolean(embedContext?.workspaceId) &&
      refValue.trim().length > 0;
    // Non-media kinds keep the exact MT-163 chip (selectors + classes + title).
    // Iteration-3 EXT-NAV-LINK-001: clicking the chip dispatches a typed
    // navigation INTENT (hs:link-navigate); the workbench shell
    // (MT-245/246/248) consumes it to perform the actual navigation.
    return (
      <NodeViewWrapper
        as="span"
        data-testid="hs-link"
        data-ref-kind={refKind}
        data-ref-value={refValue}
        data-label={String(node.attrs.label ?? "")}
        data-resolved={resolved ? "true" : "false"}
        data-navigable="true"
        data-previewable={previewable ? "true" : "false"}
        className={resolved ? "hs-link hs-link--resolved" : "hs-link hs-link--unresolved"}
        title={`${refKind}:${refValue} — click to open`}
        onMouseEnter={() => {
          if (previewable) setPreviewOpen(true);
        }}
        onMouseLeave={() => setPreviewOpen(false)}
        onFocus={() => {
          if (previewable) setPreviewOpen(true);
        }}
        onBlur={() => setPreviewOpen(false)}
        onClick={() =>
          dispatchHsLinkNavigate({
            refKind,
            refValue,
            label: chipLabel(node.attrs),
          })
        }
      >
        {chipLabel(node.attrs)}
        {previewOpen && previewable ? (
          <LoomLinkPreview workspaceId={embedContext!.workspaceId} blockId={refValue} />
        ) : null}
      </NodeViewWrapper>
    );
  }

  return (
    <NodeViewWrapper
      as="span"
      data-testid="hs-link"
      data-ref-kind={refKind}
      data-ref-value={refValue}
      data-label={String(node.attrs.label ?? "")}
      data-resolved={resolved ? "true" : "false"}
      className="hs-link hs-embed"
      title={`${refKind}:${refValue}`}
    >
      <MediaEmbed
        kind={refKind}
        refValue={refValue}
        label={chipLabel(node.attrs)}
        context={embedContext}
      />
    </NodeViewWrapper>
  );
}

function loomPreviewTitle(block: LoomBlock): string {
  return block.title?.trim() || block.original_filename?.trim() || block.block_id;
}

function loomPreviewExcerpt(block: LoomBlock): string {
  return (
    block.derived.full_text_index?.trim() ||
    block.original_filename?.trim() ||
    block.content_hash?.trim() ||
    block.block_id
  );
}

function loomPreviewCounts(block: LoomBlock): string {
  return [
    `${block.derived.tag_count} tags`,
    `${block.derived.mention_count} mentions`,
    `${block.derived.backlink_count} backlinks`,
  ].join(" · ");
}

function LoomLinkPreview({ workspaceId, blockId }: { workspaceId: string; blockId: string }) {
  const [state, setState] = useState<LoomPreviewState>({ phase: "loading" });
  const previewRef = useRef<HTMLSpanElement | null>(null);
  const requestKey = `${workspaceId}\0${blockId}`;
  const [lastRequestKey, setLastRequestKey] = useState(requestKey);
  if (lastRequestKey !== requestKey) {
    setLastRequestKey(requestKey);
    setState({ phase: "loading" });
  }

  useEffect(() => {
    let cancelled = false;
    getLoomBlock(workspaceId, blockId)
      .then((block) => {
        if (!cancelled) setState({ phase: "resolved", block });
      })
      .catch((error) => {
        if (!cancelled) {
          setState({
            phase: "failed",
            detail: error instanceof Error ? error.message : "Loom block preview failed",
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [workspaceId, blockId]);

  useLayoutEffect(() => {
    const node = previewRef.current;
    if (!node || typeof window === "undefined") return;

    const clampToViewport = () => {
      node.style.setProperty("--hs-link-preview-shift-x", "0px");
      const rect = node.getBoundingClientRect();
      const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
      const margin = 12;
      let shift = 0;
      if (rect.right > viewportWidth - margin) {
        shift -= rect.right - (viewportWidth - margin);
      }
      if (rect.left + shift < margin) {
        shift += margin - (rect.left + shift);
      }
      node.style.setProperty("--hs-link-preview-shift-x", `${Math.round(shift)}px`);
    };

    clampToViewport();
    window.addEventListener("resize", clampToViewport);
    window.addEventListener("scroll", clampToViewport, true);
    return () => {
      window.removeEventListener("resize", clampToViewport);
      window.removeEventListener("scroll", clampToViewport, true);
    };
  }, [state.phase, workspaceId, blockId]);

  if (state.phase === "loading") {
    return (
      <span
        ref={previewRef}
        className="hs-link-preview hs-link-preview--loading"
        data-testid="hs-link-preview-loading"
        contentEditable={false}
      >
        Loading preview…
      </span>
    );
  }

  if (state.phase === "failed") {
    return (
      <span
        ref={previewRef}
        className="hs-link-preview hs-link-preview--error"
        role="alert"
        data-testid="hs-link-preview-error"
        contentEditable={false}
      >
        Preview failed: {state.detail}
      </span>
    );
  }

  const { block } = state;
  return (
    <span
      ref={previewRef}
      className="hs-link-preview"
      role="tooltip"
      data-testid="hs-link-preview"
      data-preview-block-id={block.block_id}
      data-preview-content-type={block.content_type}
      contentEditable={false}
    >
      <strong className="hs-link-preview__title">{loomPreviewTitle(block)}</strong>
      <span className="hs-link-preview__excerpt">{loomPreviewExcerpt(block)}</span>
      <span className="hs-link-preview__meta">{loomPreviewCounts(block)}</span>
    </span>
  );
}

/**
 * The media-embed renderer for the typed hsLink media kinds. Exported so the
 * MT-259 offline Playwright harness can mount the REAL component (thumb-first
 * grid, video poster + Range src, preview_status retry surface) against a real
 * browser without standing up the full Tiptap editor.
 */
export function MediaEmbed({
  kind,
  refValue,
  label,
  context,
}: {
  kind: MediaEmbedRefKind;
  refValue: string;
  label: string;
  context: EmbedResolverContext | null;
}) {
  const [state, setState] = useState<EmbedState>({ phase: "resolving" });
  // Reset to the resolving state when the embed target changes, using the
  // render-adjust pattern (not a synchronous set-state inside the effect).
  const requestKey = `${kind} ${refValue} ${context?.workspaceId ?? ""}`;
  const [lastRequestKey, setLastRequestKey] = useState(requestKey);
  if (lastRequestKey !== requestKey) {
    setLastRequestKey(requestKey);
    setState({ phase: "resolving" });
  }

  useEffect(() => {
    let cancelled = false;
    const resolverContext: EmbedResolverContext = context ?? { workspaceId: "" };
    const run = async () => {
      if (kind === "album" || kind === "slideshow") {
        const result = await resolveEmbedSequence(kind, refValue, resolverContext);
        if (cancelled) return;
        if ("items" in result) {
          setState({ phase: "resolved-sequence", items: result.items });
        } else {
          setState({ phase: "failed", errorKind: result.errorKind, detail: result.detail });
        }
        return;
      }
      const result = await resolveEmbedAsset(kind, refValue, resolverContext);
      if (cancelled) return;
      if (result.ok) {
        setState({ phase: "resolved-single", resolution: result });
      } else {
        setState({ phase: "failed", errorKind: result.errorKind, detail: result.detail });
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [kind, refValue, context]);

  if (state.phase === "resolving") {
    return (
      <span className="hs-embed__loading" data-testid="hs-embed-loading" data-embed-kind={kind}>
        Resolving {kind} embed {refValue}…
      </span>
    );
  }

  if (state.phase === "failed") {
    return <EmbedError kind={kind} refValue={refValue} errorKind={state.errorKind} detail={state.detail} />;
  }

  if (state.phase === "resolved-single") {
    const { resolution } = state;
    if (kind === "video") {
      return (
        <span className="hs-embed__media" data-embed-kind="video">
          <video
            data-testid="hs-embed-video"
            data-asset-id={resolution.asset.asset_id}
            data-asset-mime={resolution.asset.mime}
            className="hs-embed__video"
            // MT-259: full-res src is the Range-capable content endpoint so the
            // browser seeks by issuing HTTP Range; poster loads the cheap poster
            // tier (falls back to nothing until the pyramid job marks it ready).
            src={resolution.contentUrl}
            poster={resolution.posterUrl}
            controls
            preload="metadata"
            onError={() =>
              setState({
                phase: "failed",
                errorKind: "media_load_failed",
                detail: `the browser could not load/play asset '${resolution.asset.asset_id}' (${resolution.asset.mime})`,
              })
            }
          />
          <span className="hs-embed__caption muted">
            {resolution.asset.original_filename ?? label}
          </span>
          <TierStatus assetId={resolution.asset.asset_id} context={context} />
        </span>
      );
    }
    return (
      <span className="hs-embed__media" data-embed-kind="images">
        <EmbedImage resolution={resolution} label={label} grid={false} />
        <TierStatus assetId={resolution.asset.asset_id} context={context} />
      </span>
    );
  }

  return <SequenceViewer kind={kind} items={state.items} context={context} />;
}

/**
 * MT-259 GAP-LM-009: a single image that loads the cheap THUMB tier first (so a
 * grid of hundreds scrolls fluidly), then loads the full-res original ONLY when
 * the viewer clicks/zooms. If the thumb tier is not yet generated the backend
 * returns 404 `tier_not_ready`; we degrade to the full-res original rather than
 * blanking (`onError` upgrades the src). `data-tier` records which tier is live
 * so a runtime proof can assert the grid renders the tier, not the original.
 */
function EmbedImage({
  resolution,
  label,
  grid,
}: {
  resolution: Extract<EmbedResolution, { ok: true }>;
  label: string;
  grid: boolean;
}) {
  const { asset, thumbUrl, contentUrl } = resolution;
  // The grid starts at thumb; a non-grid single embed starts at thumb too and
  // upgrades on click. Failure to load a tier degrades to the next-larger tier.
  const [src, setSrc] = useState(thumbUrl);
  const [tier, setTier] = useState<"thumb" | "full">("thumb");

  const upgradeToFull = () => {
    if (tier !== "full") {
      setSrc(contentUrl);
      setTier("full");
    }
  };

  return (
    <img
      data-testid={grid ? "hs-embed-grid-image" : "hs-embed-image"}
      data-asset-id={asset.asset_id}
      data-asset-mime={asset.mime}
      data-tier={tier}
      className="hs-embed__image"
      src={src}
      alt={asset.original_filename ?? label}
      loading="lazy"
      onClick={upgradeToFull}
      onError={() => {
        // Tier not generated yet (404 tier_not_ready) -> degrade to full-res
        // original so the viewer NEVER sees a blank; record the live tier.
        if (tier === "thumb") {
          setSrc(contentUrl);
          setTier("full");
        }
      }}
    />
  );
}

/**
 * MT-259 GAP-LM-009: the preview_status surface. Reads the per-asset tier rows
 * (list_asset_tiers) and shows pending/ready, and for a FAILED tier a visible
 * retry control that POSTs the retry endpoint (failed -> pending, attempt++,
 * requeues the real generation job). Never silent: a failed tier is always
 * shown with its reason and a retry button.
 */
function TierStatus({
  assetId,
  context,
}: {
  assetId: string;
  context: EmbedResolverContext | null;
}) {
  const [tiers, setTiers] = useState<EmbedAssetTier[] | null>(null);
  const [retrying, setRetrying] = useState<string | null>(null);

  const reload = useRef<() => void>(() => {});
  useEffect(() => {
    let cancelled = false;
    const ctx = context ?? { workspaceId: "" };
    const run = () => {
      void resolveAssetTiers(assetId, ctx).then((rows) => {
        if (!cancelled) setTiers(rows);
      });
    };
    reload.current = run;
    run();
    return () => {
      cancelled = true;
    };
  }, [assetId, context]);

  if (!tiers || tiers.length === 0) return null;
  const failed = tiers.filter((t) => t.status === "failed");
  const ready = tiers.filter((t) => t.status === "ready").length;
  const pending = tiers.filter((t) => t.status === "pending").length;

  return (
    <span
      className="hs-embed__tier-status"
      data-testid="hs-embed-tier-status"
      data-tier-ready={String(ready)}
      data-tier-pending={String(pending)}
      data-tier-failed={String(failed.length)}
      contentEditable={false}
    >
      {failed.map((t) => (
        <span
          key={t.tier}
          className="hs-embed__tier-failed"
          role="alert"
          data-testid={`hs-embed-tier-failed-${t.tier}`}
          data-failure-reason={t.failure_reason ?? ""}
          data-attempt-count={String(t.attempt_count)}
        >
          {t.tier} generation failed ({t.failure_reason ?? "unknown"})
          <button
            type="button"
            data-testid={`hs-embed-tier-retry-${t.tier}`}
            disabled={retrying === t.tier}
            onClick={() => {
              const ctx = context ?? { workspaceId: "" };
              setRetrying(t.tier);
              void retryAssetTier(assetId, t.tier, ctx).then((ok) => {
                setRetrying(null);
                if (ok) reload.current();
              });
            }}
          >
            Retry
          </button>
        </span>
      ))}
    </span>
  );
}

/**
 * Sequenced image viewer for albums and slideshows: one image at a time with
 * prev/next + position. A broken member renders its typed per-item error in
 * place while the rest of the sequence stays navigable.
 */
function SequenceViewer({
  kind,
  items,
  context,
}: {
  kind: MediaEmbedRefKind;
  items: EmbedSequenceItem[];
  context: EmbedResolverContext | null;
}) {
  const [index, setIndex] = useState(0);
  const clamped = Math.min(index, items.length - 1);
  const current = items[clamped];
  const resolvedCount = items.filter((item) => item.resolution.ok).length;

  if (resolvedCount === 0) {
    // Every member failed → the embed itself is a typed visible failure.
    const first = items[0];
    const firstError = first.resolution.ok
      ? { errorKind: "server_error" as EmbedErrorKind, detail: "unreachable" }
      : first.resolution;
    return (
      <EmbedError
        kind={kind}
        refValue={items.map((item) => item.refValue).join(",")}
        errorKind={firstError.errorKind}
        detail={`all ${items.length} member asset(s) failed to resolve; first: ${firstError.detail}`}
      />
    );
  }

  return (
    <span
      className="hs-embed__sequence"
      data-testid="hs-embed-sequence"
      data-embed-kind={kind}
      data-sequence-length={String(items.length)}
      data-sequence-index={String(clamped)}
      data-sequence-resolved={String(resolvedCount)}
    >
      <span className="hs-embed__sequence-frame">
        {current.resolution.ok ? (
          <EmbedImage
            resolution={current.resolution}
            label={current.refValue}
            grid={false}
          />
        ) : (
          <EmbedError
            kind={kind}
            refValue={current.refValue}
            errorKind={current.resolution.errorKind}
            detail={current.resolution.detail}
          />
        )}
      </span>
      {/* MT-259 GAP-LM-009: the thumbnail grid loads the cheap THUMB tier for
          EVERY member so scrolling hundreds of media never pulls full-res
          bytes; clicking a thumb jumps the frame to it. data-tier="thumb" on
          each grid image is the runtime assertion that the grid renders the
          tier, not the original. */}
      <span className="hs-embed__sequence-grid" data-testid="hs-embed-sequence-grid">
        {items.map((item, i) =>
          item.resolution.ok ? (
            <img
              key={item.refValue}
              data-testid="hs-embed-grid-image"
              data-asset-id={item.resolution.asset.asset_id}
              data-grid-index={String(i)}
              data-tier="thumb"
              className="hs-embed__grid-thumb"
              src={item.resolution.thumbUrl}
              alt={item.resolution.asset.original_filename ?? item.refValue}
              loading="lazy"
              onClick={() => setIndex(i)}
            />
          ) : null,
        )}
      </span>
      {current.resolution.ok ? (
        <TierStatus assetId={current.resolution.asset.asset_id} context={context} />
      ) : null}
      <span className="hs-embed__sequence-controls" contentEditable={false}>
        <button
          type="button"
          data-testid="hs-embed-sequence-prev"
          aria-label={`Previous ${kind} item`}
          disabled={clamped === 0}
          onClick={() => setIndex((value) => Math.max(0, value - 1))}
        >
          ‹
        </button>
        <span className="hs-embed__sequence-position" data-testid="hs-embed-sequence-position">
          {clamped + 1}/{items.length}
        </span>
        <button
          type="button"
          data-testid="hs-embed-sequence-next"
          aria-label={`Next ${kind} item`}
          disabled={clamped >= items.length - 1}
          onClick={() => setIndex((value) => Math.min(items.length - 1, value + 1))}
        >
          ›
        </button>
      </span>
    </span>
  );
}

/** The typed, visible, fail-closed embed error state (never blank). */
function EmbedError({
  kind,
  refValue,
  errorKind,
  detail,
}: {
  kind: string;
  refValue: string;
  errorKind: EmbedErrorKind;
  detail: string;
}) {
  return (
    <span
      className="hs-embed__error"
      role="alert"
      data-testid="hs-embed-error"
      data-embed-kind={kind}
      data-error-kind={errorKind}
    >
      <strong>Embed failed ({errorKind}):</strong> {kind}:{refValue} — {detail}
    </span>
  );
}
