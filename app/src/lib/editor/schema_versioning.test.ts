// WP-KERNEL-009 / MT-162 — TiptapSchemaVersioning tests.
//
// Proves the editor's schema-version guard: current passes through, a synthetic
// older version migrates forward deterministically, a future version is refused
// (not down-converted), and an unknown version is a typed mismatch — so a load
// never feeds incompatible JSON into ProseMirror.

import { describe, it, expect } from "vitest";
import {
  assertEditorSchema,
  EDITOR_CURRENT_SCHEMA_VERSION,
  knownSchemaVersions,
  SCHEMA_MIGRATIONS,
  type SchemaMigration,
} from "./schema_versioning";
import type { JSONContentLike } from "../api";

const DOC: JSONContentLike = {
  type: "doc",
  content: [{ type: "paragraph", content: [{ type: "text", text: "hello" }] }],
};

describe("assertEditorSchema (MT-162)", () => {
  it("passes through a document already at the current schema version", () => {
    const result = assertEditorSchema(EDITOR_CURRENT_SCHEMA_VERSION, DOC);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.compatibility).toBe("current");
      expect(result.applied).toEqual([]);
      expect(result.content).toEqual(DOC);
    }
  });

  it("refuses (does not down-convert) a schema newer than the editor", () => {
    // current is rich_document_v1 → v9 is parseably-newer.
    const result = assertEditorSchema("rich_document_v9", DOC);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.compatibility).toBe("newer_than_editor");
      expect(result.reason).toContain("newer than this editor");
    }
  });

  it("returns a typed unknown-version mismatch when no migration path exists", () => {
    const result = assertEditorSchema("legacy_markdown_dump", DOC);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.compatibility).toBe("unknown_version");
    }
  });

  it("migrates forward through a registered chain (synthetic, deterministic)", () => {
    // Drive the migration walk with an injected current version + a synthetic
    // chain so the forward-migration code path is proven before a real
    // migration ships. We simulate v0 -> current by temporarily registering a
    // migration via the exported array shape.
    const synthetic: SchemaMigration = {
      from: "rich_document_v0",
      to: EDITOR_CURRENT_SCHEMA_VERSION,
      describe: "test: tag every paragraph as migrated",
      migrate: (content) => ({
        ...content,
        attrs: { ...(content.attrs ?? {}), migratedFrom: "rich_document_v0" },
      }),
    };
    // SCHEMA_MIGRATIONS is readonly by type; mutate the underlying array for the
    // duration of this test to exercise the walk, then restore.
    const arr = SCHEMA_MIGRATIONS as unknown as SchemaMigration[];
    arr.push(synthetic);
    try {
      const result = assertEditorSchema("rich_document_v0", DOC);
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.compatibility).toBe("migrated");
        expect(result.applied).toEqual([`rich_document_v0->${EDITOR_CURRENT_SCHEMA_VERSION}`]);
        expect(result.content.attrs?.migratedFrom).toBe("rich_document_v0");
      }
      expect(knownSchemaVersions()).toContain("rich_document_v0");
    } finally {
      arr.pop();
    }
  });

  it("reports the current schema version among known versions", () => {
    expect(knownSchemaVersions()).toContain(EDITOR_CURRENT_SCHEMA_VERSION);
  });
});
