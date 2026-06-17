// WP-KERNEL-009 / MT-244 — embed asset resolution tests (fail-closed).
//
// Proves the typed media-embed resolver: real-shaped metadata resolves to a
// typed content URL; absolute paths / traversal / schemes / empty refs are
// rejected CLIENT-SIDE before any request (mirror of backend MT-152); 404,
// 403, 5xx, malformed bodies, and network failures map to typed error kinds;
// a mime/kind mismatch is fail-closed; album/slideshow sequences resolve
// per-item. Uses an injected fetch returning real Response objects — no
// network, no mocks of the resolver itself.

import { describe, it, expect, vi } from "vitest";
import {
  assetContentUrl,
  assetMetadataUrl,
  assetTierContentUrl,
  assetTierRetryUrl,
  assetTiersUrl,
  collectionRefId,
  collectionUrl,
  isMediaEmbedKind,
  mimeMatchesEmbedKind,
  parseAssetRefList,
  resolveAssetTiers,
  resolveCollectionMembers,
  resolveCollectionSequence,
  resolveEmbedAsset,
  resolveEmbedSequence,
  retryAssetTier,
  validateAssetRef,
  MAX_SEQUENCE_ITEMS,
  type EmbedAssetMetadata,
} from "./embed_assets";

const WS = "ws-test";

function metadata(overrides: Partial<EmbedAssetMetadata> = {}): EmbedAssetMetadata {
  return {
    asset_id: "0198d2f0-0000-7000-8000-000000000001",
    workspace_id: WS,
    kind: "image",
    mime: "image/png",
    original_filename: "proof.png",
    content_hash: "abc123",
    size_bytes: 1024,
    width: 4,
    height: 4,
    ...overrides,
  };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function contextWith(response: Response | Error) {
  const fetchImpl = vi.fn(async () => {
    if (response instanceof Error) throw response;
    return response;
  });
  return { context: { workspaceId: WS, apiBaseUrl: "http://127.0.0.1:9", fetchImpl }, fetchImpl };
}

describe("media embed kind classification (MT-244)", () => {
  it("classifies exactly the media kinds as embeds", () => {
    expect(isMediaEmbedKind("images")).toBe(true);
    expect(isMediaEmbedKind("video")).toBe(true);
    expect(isMediaEmbedKind("album")).toBe(true);
    expect(isMediaEmbedKind("slideshow")).toBe(true);
    for (const kind of ["note", "file", "folder", "project", "spec", "wp", "symbol", "unknown"]) {
      expect(isMediaEmbedKind(kind)).toBe(false);
    }
  });

  it("matches mime families per embed kind", () => {
    expect(mimeMatchesEmbedKind("images", "image/png")).toBe(true);
    expect(mimeMatchesEmbedKind("images", "video/webm")).toBe(false);
    expect(mimeMatchesEmbedKind("video", "video/webm")).toBe(true);
    expect(mimeMatchesEmbedKind("video", "image/png")).toBe(false);
    expect(mimeMatchesEmbedKind("slideshow", "image/jpeg")).toBe(true);
    expect(mimeMatchesEmbedKind("album", "application/pdf")).toBe(false);
  });
});

describe("asset ref validation (fail-closed, backend MT-152 mirror)", () => {
  it("accepts UUID-shaped and opaque ids", () => {
    expect(validateAssetRef("0198d2f0-0000-7000-8000-000000000001")).toBeNull();
    expect(validateAssetRef("asset_01.v2")).toBeNull();
  });

  it("rejects absolute Windows paths as absolute_path_rejected", () => {
    expect(validateAssetRef("C:\\secrets\\img.png")?.errorKind).toBe("absolute_path_rejected");
    expect(validateAssetRef("D:/media/clip.mp4")?.errorKind).toBe("absolute_path_rejected");
  });

  it("rejects leading-slash and UNC paths as absolute_path_rejected", () => {
    expect(validateAssetRef("/etc/passwd")?.errorKind).toBe("absolute_path_rejected");
    expect(validateAssetRef("\\\\nas\\share\\img.png")?.errorKind).toBe("absolute_path_rejected");
  });

  it("rejects traversal and separators as traversal_rejected", () => {
    expect(validateAssetRef("a/../b")?.errorKind).toBe("traversal_rejected");
    expect(validateAssetRef("media/img.png")?.errorKind).toBe("traversal_rejected");
    expect(validateAssetRef("..")?.errorKind).toBe("traversal_rejected");
  });

  it("rejects schemes (http/javascript/file) as scheme_rejected", () => {
    expect(validateAssetRef("https://evil.example/x.png")?.errorKind).toBe("scheme_rejected");
    expect(validateAssetRef("javascript:alert(1)")?.errorKind).toBe("scheme_rejected");
    expect(validateAssetRef("file:./x")?.errorKind).toBe("scheme_rejected");
  });

  it("rejects empty and malformed ids", () => {
    expect(validateAssetRef("   ")?.errorKind).toBe("empty_ref");
    expect(validateAssetRef("a b")?.errorKind).toBe("invalid_ref");
    expect(validateAssetRef("x".repeat(300))?.errorKind).toBe("invalid_ref");
  });
});

describe("resolveEmbedAsset (typed results, no throws)", () => {
  it("resolves real metadata to a typed content URL", async () => {
    const { context, fetchImpl } = contextWith(jsonResponse(metadata()));
    const result = await resolveEmbedAsset("images", "0198d2f0-0000-7000-8000-000000000001", context);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.asset.mime).toBe("image/png");
      expect(result.contentUrl).toBe(
        assetContentUrl("http://127.0.0.1:9", WS, "0198d2f0-0000-7000-8000-000000000001"),
      );
    }
    expect(fetchImpl).toHaveBeenCalledWith(
      assetMetadataUrl("http://127.0.0.1:9", WS, "0198d2f0-0000-7000-8000-000000000001"),
    );
  });

  it("fails closed with no_workspace when no workspace context is bound", async () => {
    const result = await resolveEmbedAsset("images", "asset-1", { workspaceId: "" });
    expect(result).toMatchObject({ ok: false, errorKind: "no_workspace" });
  });

  it("rejects invalid refs before any network call", async () => {
    const { context, fetchImpl } = contextWith(jsonResponse(metadata()));
    const result = await resolveEmbedAsset("images", "C:\\evil.png", context);
    expect(result).toMatchObject({ ok: false, errorKind: "absolute_path_rejected" });
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("maps 404 → not_found, 403 → forbidden, 500 → server_error", async () => {
    for (const [status, errorKind] of [
      [404, "not_found"],
      [403, "forbidden"],
      [500, "server_error"],
    ] as const) {
      const { context } = contextWith(new Response("nope", { status }));
      const result = await resolveEmbedAsset("images", "asset-1", context);
      expect(result).toMatchObject({ ok: false, errorKind });
    }
  });

  it("maps fetch rejection → network_error and bad JSON → server_error", async () => {
    const rejected = await resolveEmbedAsset("images", "asset-1", contextWith(new Error("ECONNREFUSED")).context);
    expect(rejected).toMatchObject({ ok: false, errorKind: "network_error" });

    const badJson = await resolveEmbedAsset(
      "images",
      "asset-1",
      contextWith(new Response("<html>not json</html>", { status: 200 })).context,
    );
    expect(badJson).toMatchObject({ ok: false, errorKind: "server_error" });

    const missingFields = await resolveEmbedAsset(
      "images",
      "asset-1",
      contextWith(jsonResponse({ hello: "world" })).context,
    );
    expect(missingFields).toMatchObject({ ok: false, errorKind: "server_error" });
  });

  it("fails closed on mime/kind mismatch (video asset in an images embed)", async () => {
    const { context } = contextWith(jsonResponse(metadata({ mime: "video/webm" })));
    const result = await resolveEmbedAsset("images", "asset-1", context);
    expect(result).toMatchObject({ ok: false, errorKind: "kind_mismatch" });
  });
});

describe("album/slideshow sequences (closest real surface: asset-id list)", () => {
  it("parses ordered comma-separated asset refs", () => {
    expect(parseAssetRefList("a, b ,c")).toEqual(["a", "b", "c"]);
    expect(parseAssetRefList(" , ")).toEqual([]);
  });

  it("resolves members independently — a broken member is a typed per-item error", async () => {
    const fetchImpl = vi.fn(async (url: string) => {
      if (url.includes("asset-bad")) return new Response("nope", { status: 404 });
      return jsonResponse(metadata({ asset_id: "asset-good" }));
    });
    const result = await resolveEmbedSequence("slideshow", "asset-good,asset-bad", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect("items" in result).toBe(true);
    if ("items" in result) {
      expect(result.items).toHaveLength(2);
      expect(result.items[0].resolution.ok).toBe(true);
      expect(result.items[1].resolution).toMatchObject({ ok: false, errorKind: "not_found" });
    }
  });

  it("treats an empty sequence as a typed empty_ref error", async () => {
    const result = await resolveEmbedSequence("album", " , ", { workspaceId: WS });
    expect(result).toMatchObject({ ok: false, errorKind: "empty_ref" });
  });

  it("fails closed on an oversized sequence without fanning out requests (DoS guard)", async () => {
    const fetchImpl = vi.fn(async () => jsonResponse(metadata()));
    const refs = Array.from({ length: MAX_SEQUENCE_ITEMS + 1 }, (_, i) => `asset-${i}`).join(",");
    const result = await resolveEmbedSequence("album", refs, {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(result).toMatchObject({ ok: false, errorKind: "invalid_ref" });
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});

describe("MT-259 tier content URLs", () => {
  it("appends ?tier= for non-full tiers and omits it for full", () => {
    const base = "http://127.0.0.1:9";
    expect(assetTierContentUrl(base, WS, "a-1", "thumb")).toBe(
      `${assetContentUrl(base, WS, "a-1")}?tier=thumb`,
    );
    expect(assetTierContentUrl(base, WS, "a-1", "preview")).toBe(
      `${assetContentUrl(base, WS, "a-1")}?tier=preview`,
    );
    // full tier serves the original (no tier query) so video can Range-seek it.
    expect(assetTierContentUrl(base, WS, "a-1", "full")).toBe(
      assetContentUrl(base, WS, "a-1"),
    );
  });

  it("builds the per-asset tiers endpoint URL", () => {
    const base = "http://127.0.0.1:9";
    expect(assetTiersUrl(base, WS, "a-1")).toBe(
      `${assetMetadataUrl(base, WS, "a-1")}/tiers`,
    );
  });
});

describe("MT-259 backend collection list-source (GAP-LM-244a)", () => {
  it("builds the collection URL", () => {
    const base = "http://127.0.0.1:9";
    expect(collectionUrl(base, WS, "col-1")).toBe(
      `${base}/workspaces/${WS}/loom/collections/col-1`,
    );
  });

  it("resolves ordered members from the backend collection (not comma-split)", async () => {
    const ordered = ["asset-c", "asset-a", "asset-b"];
    const fetchImpl = vi.fn(async () =>
      jsonResponse({ collection_id: "col-1", title: "Album", members: ordered }),
    );
    const result = await resolveCollectionMembers("col-1", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.collection.members).toEqual(ordered);
    expect(fetchImpl).toHaveBeenCalledWith(collectionUrl("http://127.0.0.1:9", WS, "col-1"));
  });

  it("resolveCollectionSequence enumerates members from backend order then resolves each", async () => {
    const ordered = ["asset-2", "asset-1"];
    const fetchImpl = vi.fn(async (url: string) => {
      if (url.includes("/loom/collections/")) {
        return jsonResponse({ collection_id: "col-9", title: null, members: ordered });
      }
      // each member metadata resolves as an image
      const id = url.split("/assets/")[1];
      return jsonResponse(metadata({ asset_id: id }));
    });
    const result = await resolveCollectionSequence("album", "col-9", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.items.map((i) => i.refValue)).toEqual(ordered);
      expect(result.items.every((i) => i.resolution.ok)).toBe(true);
    }
  });

  it("fails closed (typed not_found) when the collection is missing", async () => {
    const fetchImpl = vi.fn(async () => jsonResponse({}, 404));
    const result = await resolveCollectionMembers("col-x", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(result).toMatchObject({ ok: false, errorKind: "not_found" });
  });

  it("rejects a collection ref with path traversal before any request", async () => {
    const fetchImpl = vi.fn(async () => jsonResponse({}));
    const result = await resolveCollectionMembers("../etc/passwd", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(result).toMatchObject({ ok: false, errorKind: "traversal_rejected" });
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});

describe("MT-259 live-path wiring (the repoint that makes the helpers non-dead)", () => {
  it("resolveEmbedAsset returns tier URLs the view loads before the original", async () => {
    const { context } = contextWith(jsonResponse(metadata({ asset_id: "a-7" })));
    const result = await resolveEmbedAsset("images", "a-7", context);
    expect(result.ok).toBe(true);
    if (result.ok) {
      // The grid loads thumbUrl; click upgrades to contentUrl (full original).
      expect(result.thumbUrl).toBe(assetTierContentUrl("http://127.0.0.1:9", WS, "a-7", "thumb"));
      expect(result.previewUrl).toBe(
        assetTierContentUrl("http://127.0.0.1:9", WS, "a-7", "preview"),
      );
      expect(result.posterUrl).toBe(
        assetTierContentUrl("http://127.0.0.1:9", WS, "a-7", "poster"),
      );
      expect(result.contentUrl).toBe(assetContentUrl("http://127.0.0.1:9", WS, "a-7"));
      // The thumb URL is NOT the original — proves the grid renders the tier.
      expect(result.thumbUrl).not.toBe(result.contentUrl);
    }
  });

  it("collectionRefId detects the collection: ref and ignores comma lists", () => {
    expect(collectionRefId("collection:col-1")).toBe("col-1");
    expect(collectionRefId("  collection:col-2 ")).toBe("col-2");
    expect(collectionRefId("a-1,a-2,a-3")).toBeNull();
    expect(collectionRefId("collection:")).toBeNull();
  });

  it("resolveEmbedSequence routes a collection: ref to the BACKEND (not comma-split)", async () => {
    const ordered = ["asset-2", "asset-1"];
    const seen: string[] = [];
    const fetchImpl = vi.fn(async (url: string) => {
      seen.push(url);
      if (url.includes("/loom/collections/")) {
        return jsonResponse({ collection_id: "col-9", title: null, members: ordered });
      }
      const id = url.split("/assets/")[1];
      return jsonResponse(metadata({ asset_id: id }));
    });
    const result = await resolveEmbedSequence("album", "collection:col-9", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect("items" in result).toBe(true);
    if ("items" in result) {
      // Order is server-owned; membership came from the backend collection.
      expect(result.items.map((i) => i.refValue)).toEqual(ordered);
    }
    // It actually hit the collection endpoint (the repoint), not just metadata.
    expect(seen.some((u) => u.includes("/loom/collections/col-9"))).toBe(true);
  });

  it("resolveEmbedSequence still comma-splits a legacy asset-id list", async () => {
    const fetchImpl = vi.fn(async (url: string) => {
      const id = url.split("/assets/")[1];
      return jsonResponse(metadata({ asset_id: id }));
    });
    const result = await resolveEmbedSequence("album", "a-1,a-2", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect("items" in result).toBe(true);
    if ("items" in result) expect(result.items.map((i) => i.refValue)).toEqual(["a-1", "a-2"]);
    // No collection request for a legacy list.
    expect(fetchImpl.mock.calls.every(([u]) => !String(u).includes("/loom/collections/"))).toBe(
      true,
    );
  });

  it("resolveAssetTiers reads the preview_status surface (pending/ready/failed)", async () => {
    const fetchImpl = vi.fn(async () =>
      jsonResponse({
        tiers: [
          { tier: "thumb", status: "ready", tier_asset_id: "t-1", content_hash: "h", failure_reason: null, attempt_count: 0 },
          { tier: "poster", status: "failed", tier_asset_id: null, content_hash: null, failure_reason: "no_video_decoder_bundled", attempt_count: 2 },
        ],
      }),
    );
    const tiers = await resolveAssetTiers("a-1", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(tiers.map((t) => t.tier)).toEqual(["thumb", "poster"]);
    expect(tiers.find((t) => t.tier === "poster")?.status).toBe("failed");
    expect(fetchImpl).toHaveBeenCalledWith(assetTiersUrl("http://127.0.0.1:9", WS, "a-1"));
  });

  it("resolveAssetTiers fails closed (empty list) on transport failure", async () => {
    const fetchImpl = vi.fn(async () => {
      throw new Error("backend unreachable");
    });
    const tiers = await resolveAssetTiers("a-1", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(tiers).toEqual([]);
  });

  it("retryAssetTier POSTs the retry endpoint and returns ok", async () => {
    const fetchImpl = vi.fn(async (_url: string, init?: { method?: string }) => {
      expect(init?.method).toBe("POST");
      return jsonResponse({ tier: "poster", status: "pending", attempt_count: 1, requeued: true });
    });
    const ok = await retryAssetTier("a-1", "poster", {
      workspaceId: WS,
      apiBaseUrl: "http://127.0.0.1:9",
      fetchImpl,
    });
    expect(ok).toBe(true);
    expect(fetchImpl).toHaveBeenCalledWith(
      assetTierRetryUrl("http://127.0.0.1:9", WS, "a-1", "poster"),
      { method: "POST" },
    );
  });
});
