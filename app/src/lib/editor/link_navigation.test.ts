// WP-KERNEL-009 / MT-245 (EXT-NAV-LINK-001) — typed hsLink resolution tests.
//
// resolveHsLinkTarget is the SINGLE pure resolver App.tsx and the offline proof
// harness both use: a typed link resolves to its real in-app surface; a link
// with no value, or a typed kind no surface owns, resolves to a typed `error`
// (never a silent no-op).

import { describe, expect, it } from "vitest";
import { resolveHsLinkTarget } from "./link_navigation";

describe("resolveHsLinkTarget (MT-245 EXT-NAV-LINK-001)", () => {
  it("routes known typed kinds to their owning surface", () => {
    expect(resolveHsLinkTarget({ refKind: "wp", refValue: "WP-KERNEL-009", label: "" })).toEqual({
      kind: "wp",
      refValue: "WP-KERNEL-009",
    });
    expect(resolveHsLinkTarget({ refKind: "mt", refValue: "MT-245", label: "" })).toEqual({
      kind: "mt",
      refValue: "MT-245",
    });
    expect(resolveHsLinkTarget({ refKind: "micro_task", refValue: "MT-9", label: "" }).kind).toBe("mt");
    expect(resolveHsLinkTarget({ refKind: "symbol", refValue: "Foo::bar", label: "" }).kind).toBe("symbol");
    expect(resolveHsLinkTarget({ refKind: "wiki_page", refValue: "Home", label: "" }).kind).toBe("wiki_page");
    expect(resolveHsLinkTarget({ refKind: "user_manual_page", refValue: "intro", label: "" }).kind).toBe(
      "user_manual",
    );
    expect(resolveHsLinkTarget({ refKind: "document", refValue: "doc-1", label: "" }).kind).toBe("document");
    expect(resolveHsLinkTarget({ refKind: "note", refValue: "n-1", label: "" }).kind).toBe("loom");
    // A KRD- note routes to the document workspace surface, not the loom block.
    expect(resolveHsLinkTarget({ refKind: "note", refValue: "KRD-7", label: "" }).kind).toBe("document");
  });

  it("normalizes whitespace and case in kind/value", () => {
    expect(resolveHsLinkTarget({ refKind: "  WP ", refValue: " WP-1 ", label: "" })).toEqual({
      kind: "wp",
      refValue: "WP-1",
    });
  });

  it("returns a typed visible error for an empty value", () => {
    const result = resolveHsLinkTarget({ refKind: "wp", refValue: "   ", label: "" });
    expect(result.kind).toBe("error");
    expect(result.kind === "error" && result.message).toMatch(/no target value/);
  });

  it("returns a typed visible error for a kind no local surface owns", () => {
    const result = resolveHsLinkTarget({ refKind: "ghost", refValue: "nowhere-123", label: "ghost" });
    expect(result.kind).toBe("error");
    expect(result.kind === "error" && result.message).toMatch(/ghost:nowhere-123/);
  });
});
