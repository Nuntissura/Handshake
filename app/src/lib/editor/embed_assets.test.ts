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
  isMediaEmbedKind,
  mimeMatchesEmbedKind,
  parseAssetRefList,
  resolveEmbedAsset,
  resolveEmbedSequence,
  validateAssetRef,
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
});
