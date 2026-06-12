// WP-KERNEL-009 / MT-162 — TiptapSchemaVersioning.
//
// Schema-version assertion + forward migration for persisted Tiptap/ProseMirror
// JSON. Master Spec §2.3.13.11 says a RichDocument carries a schema version and
// §7.1.1.8 requires deterministic load/round-trip across "schema migration".
// PostgreSQL/EventLedger is the authority: the backend stamps
// `RichDocument.schema_version` (RichDocument.schema_version in
// app/src/lib/api.ts) on every save. This module is the EDITOR-side guard that
// runs on load:
//   1. reads the persisted schema_version,
//   2. if it matches the editor's current version → pass through,
//   3. if it is an older known version → run the registered migration(s) to
//      bring the content_json up to the current node/mark schema,
//   4. if it is unknown/newer → return a typed mismatch the UI surfaces (MT-174)
//      instead of silently feeding incompatible JSON to ProseMirror (which would
//      throw and blank the editor).
//
// No network, no authority writes here — the backend remains canonical. This is
// the deterministic, restartable transform a no-context model can read to know
// exactly how the editor reconciles an on-disk schema version with the running
// editor schema. Migrations are pure functions over JSONContentLike.

import type { JSONContentLike } from "../api";
import { WP009_RICH_DOCUMENT_SCHEMA_VERSION } from "../tiptap/extension_set";

/** The schema version the running editor produces and asserts against. */
export const EDITOR_CURRENT_SCHEMA_VERSION = WP009_RICH_DOCUMENT_SCHEMA_VERSION;

/** A pure forward migration from one schema version to the next. */
export interface SchemaMigration {
  readonly from: string;
  readonly to: string;
  readonly describe: string;
  /** Transforms a document's content_json from `from` to `to`. Must be pure. */
  migrate(content: JSONContentLike): JSONContentLike;
}

/**
 * Ordered registry of forward migrations. Empty today (v1 is the first schema),
 * but the chain is exercised by tests with a synthetic older version so the
 * migration walk is proven BEFORE a real migration is ever needed (§7.1.1.8
 * "schema migration" round-trip). Add a migration here (and bump
 * WP009_RICH_DOCUMENT_SCHEMA_VERSION in extension_set.ts) whenever the node/mark
 * schema changes incompatibly.
 */
export const SCHEMA_MIGRATIONS: readonly SchemaMigration[] = [];

export type SchemaCompatibility =
  | "current"
  | "migrated"
  | "unknown_version"
  | "newer_than_editor";

/**
 * Result of reconciling a persisted document's schema version with the editor.
 * `ok` content is safe to load into ProseMirror; a non-ok result must be
 * surfaced (MT-174) rather than loaded.
 */
export type SchemaAssertion =
  | {
      ok: true;
      compatibility: "current" | "migrated";
      fromVersion: string;
      toVersion: string;
      /** Migrations applied, in order (empty when already current). */
      applied: readonly string[];
      content: JSONContentLike;
    }
  | {
      ok: false;
      compatibility: "unknown_version" | "newer_than_editor";
      fromVersion: string;
      toVersion: string;
      reason: string;
    };

/** All known schema versions, oldest→newest, derived from the migration chain. */
export function knownSchemaVersions(): string[] {
  const versions = new Set<string>([EDITOR_CURRENT_SCHEMA_VERSION]);
  for (const m of SCHEMA_MIGRATIONS) {
    versions.add(m.from);
    versions.add(m.to);
  }
  return [...versions];
}

/** Numeric rank of the editor's current version within `v1, v2, ...` naming. */
function versionRank(version: string): number | null {
  const match = /_v(\d+)$/.exec(version);
  return match ? Number(match[1]) : null;
}

/**
 * Reconciles a persisted RichDocument's schema version against the editor.
 *
 * - matches current → pass through (compatibility "current").
 * - older known version → walk SCHEMA_MIGRATIONS to current ("migrated").
 * - unknown version with a parseable rank GREATER than current → "newer_than_editor"
 *   (the running build is too old; do not down-convert, surface it).
 * - otherwise unknown → "unknown_version".
 *
 * The transform is deterministic and side-effect-free, so a restart/projection
 * rebuild reproduces the same result (§7.1.1.8 deterministic round-trip).
 */
export function assertEditorSchema(
  persistedVersion: string,
  content: JSONContentLike,
  currentVersion: string = EDITOR_CURRENT_SCHEMA_VERSION,
): SchemaAssertion {
  if (persistedVersion === currentVersion) {
    return {
      ok: true,
      compatibility: "current",
      fromVersion: persistedVersion,
      toVersion: currentVersion,
      applied: [],
      content,
    };
  }

  // Try to migrate forward from the persisted version to current.
  const applied: string[] = [];
  let cursor = persistedVersion;
  let working = content;
  let guard = 0;
  while (cursor !== currentVersion && guard < SCHEMA_MIGRATIONS.length + 1) {
    const step = SCHEMA_MIGRATIONS.find((m) => m.from === cursor);
    if (!step) break;
    working = step.migrate(working);
    applied.push(`${step.from}->${step.to}`);
    cursor = step.to;
    guard += 1;
  }

  if (cursor === currentVersion) {
    return {
      ok: true,
      compatibility: "migrated",
      fromVersion: persistedVersion,
      toVersion: currentVersion,
      applied,
      content: working,
    };
  }

  // No migration path. Decide whether it's a future schema or just unknown.
  const persistedRank = versionRank(persistedVersion);
  const currentRank = versionRank(currentVersion);
  if (persistedRank !== null && currentRank !== null && persistedRank > currentRank) {
    return {
      ok: false,
      compatibility: "newer_than_editor",
      fromVersion: persistedVersion,
      toVersion: currentVersion,
      reason:
        `Document schema ${persistedVersion} is newer than this editor (${currentVersion}). ` +
        `Update Handshake to open it; the editor will not down-convert a newer schema.`,
    };
  }

  return {
    ok: false,
    compatibility: "unknown_version",
    fromVersion: persistedVersion,
    toVersion: currentVersion,
    reason:
      `Document schema ${persistedVersion} is not known to this editor (${currentVersion}) ` +
      `and no migration path is registered.`,
  };
}
