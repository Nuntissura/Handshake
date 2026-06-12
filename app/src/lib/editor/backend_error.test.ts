// WP-KERNEL-009 / MT-174 — EditorBackendErrorStates (classification) tests.
//
// Proves save/load failures classify into the right typed error kind (so the
// editor renders an actionable inline message keyed on data-error-kind, never a
// blank screen), and that a schema-assertion failure becomes a schema error.

import { describe, it, expect } from "vitest";
import {
  classifySaveError,
  classifyLoadError,
  schemaMismatchError,
} from "./backend_error";

describe("backend error classification (MT-174)", () => {
  it("classifies an optimistic-concurrency conflict on save", () => {
    const e = classifySaveError(new Error("HSK-409 version conflict: expected_version 3 got 4"));
    expect(e.kind).toBe("conflict");
    expect(e.hint).toContain("Reload");
  });

  it("classifies a schema error on save", () => {
    expect(classifySaveError(new Error("schema_version mismatch")).kind).toBe("schema");
  });

  it("falls back to a generic save error", () => {
    const e = classifySaveError(new Error("network down"));
    expect(e.kind).toBe("save");
    expect(e.message).toBe("network down");
    expect(e.hint).toContain("kept locally");
  });

  it("classifies load failures (schema / projection / index / generic)", () => {
    expect(classifyLoadError(new Error("document schema newer")).kind).toBe("schema");
    expect(classifyLoadError(new Error("projection rebuild failed")).kind).toBe("projection");
    expect(classifyLoadError(new Error("index out of sync")).kind).toBe("index");
    expect(classifyLoadError(new Error("boom")).kind).toBe("load");
  });

  it("builds a schema error from an assertion reason", () => {
    const e = schemaMismatchError("Document schema rich_document_v9 is newer than this editor");
    expect(e.kind).toBe("schema");
    expect(e.message).toContain("newer than this editor");
  });

  it("accepts non-Error values without throwing", () => {
    expect(classifySaveError("plain string conflict").kind).toBe("conflict");
    expect(classifyLoadError(null).kind).toBe("load");
  });
});
