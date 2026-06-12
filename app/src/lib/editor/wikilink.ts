// WP-KERNEL-009 / MT-163 — wikilink parsing (pure logic).
//
// Parses the [[kind:value]] / [[kind:value|label]] wikilink syntax (the link
// family from the hs_rich_text_editor stub field of the WP-KERNEL-009 contract)
// into a TYPED link descriptor that maps to a backend ref kind. Split out of the
// Tiptap node so the parse/resolve logic is unit-testable in jsdom without the
// editor runtime.
//
// A typed link node persists into the backend as a `link` block with a
// link_kind/ref (RichDocBacklink.link_kind, RichDocEmbed.ref_kind in
// app/src/lib/api.ts) — the backend block-tree is the authority for which
// targets actually resolve; this layer only classifies the SYNTAX into the
// correct typed kind. An unrecognized prefix is preserved as a typed
// "unknown" link (never silently dropped) so the operator/validator can see it.

import {
  WP009_WIKILINK_KIND_BY_PREFIX,
  type WikilinkKindDescriptor,
} from "./extension_inventory";

export interface ParsedWikilink {
  /** Backend ref kind (e.g. "wp", "file") or "unknown" for an unrecognized prefix. */
  refKind: string;
  /** The target value after the prefix (e.g. "WP-KERNEL-009", "src/app.ts"). */
  refValue: string;
  /** Display label: explicit `|label`, else a readable default. */
  label: string;
  /** True when the prefix matched a known wikilink kind. */
  resolved: boolean;
  /** The raw prefix as typed (lower-cased), for diagnostics. */
  rawPrefix: string;
}

/** Single-match wikilink regex: [[prefix:value]] or [[prefix:value|label]]. */
export const WIKILINK_REGEX = /\[\[([a-zA-Z_][\w]*):([^\]|]+)(?:\|([^\]]+))?\]\]/;

/** Global variant for scanning a whole string / paste rule. */
export const WIKILINK_REGEX_GLOBAL = new RegExp(WIKILINK_REGEX.source, "g");

/**
 * Parses a single wikilink token's captured groups into a typed descriptor.
 * `prefix`, `value`, and optional `label` come straight from the regex groups.
 */
export function classifyWikilink(
  prefix: string,
  value: string,
  label?: string,
): ParsedWikilink {
  const rawPrefix = prefix.trim().toLowerCase();
  const refValue = value.trim();
  const descriptor: WikilinkKindDescriptor | undefined =
    WP009_WIKILINK_KIND_BY_PREFIX.get(rawPrefix);
  const explicitLabel = label?.trim();
  if (descriptor) {
    return {
      refKind: descriptor.backendRefKind,
      refValue,
      label: explicitLabel && explicitLabel.length > 0 ? explicitLabel : refValue,
      resolved: true,
      rawPrefix,
    };
  }
  return {
    refKind: "unknown",
    refValue,
    label: explicitLabel && explicitLabel.length > 0 ? explicitLabel : `${rawPrefix}:${refValue}`,
    resolved: false,
    rawPrefix,
  };
}

/**
 * Parses a single wikilink string (must be exactly one token, e.g.
 * "[[wp:WP-KERNEL-009]]"). Returns null when the string is not a wikilink.
 */
export function parseWikilink(token: string): ParsedWikilink | null {
  const match = WIKILINK_REGEX.exec(token.trim());
  if (!match) return null;
  return classifyWikilink(match[1], match[2], match[3]);
}

/**
 * Extracts every wikilink occurrence from a free-text string (for paste / bulk
 * parse). Returns the parsed links in document order.
 */
export function extractWikilinks(text: string): ParsedWikilink[] {
  const results: ParsedWikilink[] = [];
  for (const match of text.matchAll(WIKILINK_REGEX_GLOBAL)) {
    results.push(classifyWikilink(match[1], match[2], match[3]));
  }
  return results;
}
