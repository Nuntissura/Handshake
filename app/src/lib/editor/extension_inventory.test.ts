// WP-KERNEL-009 / MT-161 — TiptapExtensionInventory tests.
//
// Proves the inventory is internally consistent and stays aligned with the
// lower-level extension_set (MT-021) contract: a no-context model relying on
// this inventory to know which editor primitives exist gets a truthful map.

import { describe, it, expect } from "vitest";
import {
  WP009_EDITOR_PRIMITIVES,
  WP009_EDITOR_PRIMITIVE_NAMES,
  WP009_WIKILINK_KINDS,
  WP009_WIKILINK_KIND_BY_PREFIX,
  WP009_EDITOR_BACKEND_BLOCK_KINDS,
  WP009_INVENTORY_SCHEMA_VERSION,
  editorPrimitive,
  inventoryMissingRequiredNodes,
} from "./extension_inventory";
import { WP009_RICH_DOCUMENT_SCHEMA_VERSION } from "../tiptap/extension_set";

describe("WP009 editor extension inventory (MT-161)", () => {
  it("is non-empty and has unique primitive names", () => {
    expect(WP009_EDITOR_PRIMITIVES.length).toBeGreaterThan(10);
    const names = WP009_EDITOR_PRIMITIVE_NAMES;
    expect(new Set(names).size).toBe(names.length);
  });

  it("covers every required schema node declared by the extension set (no drift)", () => {
    expect(inventoryMissingRequiredNodes()).toEqual([]);
  });

  it("targets the same schema version the extension set owns", () => {
    expect(WP009_INVENTORY_SCHEMA_VERSION).toBe(WP009_RICH_DOCUMENT_SCHEMA_VERSION);
  });

  it("declares the embedded Monaco code block node mapped to the backend code block kind", () => {
    const codeNode = editorPrimitive("monacoCodeBlock");
    expect(codeNode).toBeDefined();
    expect(codeNode?.engine).toBe("tiptap");
    expect(codeNode?.kind).toBe("node");
    expect(codeNode?.backendBlockKind).toBe("code");
    expect(codeNode?.implementedBy).toBe("MT-165");
  });

  it("declares the typed wikilink node and both Monaco surfaces", () => {
    expect(editorPrimitive("hsLink")?.backendBlockKind).toBe("link");
    expect(editorPrimitive("monaco:languages")?.engine).toBe("monaco");
    expect(editorPrimitive("monaco:workers")?.engine).toBe("monaco");
  });

  it("enumerates the full [[kind:value]] wikilink family with backend ref kinds", () => {
    const prefixes = WP009_WIKILINK_KINDS.map((k) => k.prefix);
    for (const required of ["note", "file", "folder", "project", "spec", "wp", "symbol", "album", "video"]) {
      expect(prefixes).toContain(required);
    }
    // Lookup map is case-insensitive on prefix.
    expect(WP009_WIKILINK_KIND_BY_PREFIX.get("wp")?.backendRefKind).toBe("wp");
    expect(WP009_WIKILINK_KIND_BY_PREFIX.get("file")?.backendRefKind).toBe("file");
  });

  it("exposes the editor's backend block-kind coverage including code and link", () => {
    expect(WP009_EDITOR_BACKEND_BLOCK_KINDS).toContain("code");
    expect(WP009_EDITOR_BACKEND_BLOCK_KINDS).toContain("link");
    expect(WP009_EDITOR_BACKEND_BLOCK_KINDS).toContain("paragraph");
    // No nulls leak into the coverage list.
    expect(WP009_EDITOR_BACKEND_BLOCK_KINDS.every((k) => typeof k === "string")).toBe(true);
  });
});
