//! Inline `#tag` detection + normalization (WP-KERNEL-012 MT-058) — the native port of the
//! Obsidian/Notion inline-tag authoring grammar, adapted from the React wikilink word-boundary
//! trigger in `app/src/lib/editor/wikilink.ts` (which uses `[[`; this uses `#`).
//!
//! ## What an inline tag is (NODE-SHAPE RECONCILIATION — the KERNEL_BUILDER gate)
//!
//! An inline `#tag` is NOT a new invented mark. Following the MT-014 / MT-034 "everything is an
//! `hsLink` atom by `ref_kind`" lesson (and the way wikilinks already model `[[...]]` as
//! [`crate::rich_editor::document_model::node::HsLinkNode`] / `Child::HsLink`), a committed inline tag
//! is the EXISTING `hsLink` inline atom with `ref_kind = "tag"`, `ref_value = the normalized tag
//! identity`, and `label = "#" + display_name`. This is what makes it (a) round-trip the backend
//! `content_json` (no new node type the backend would strip on save), and (b) converge with the
//! property-panel tag set on the SAME normalized identity (one tag, one hub — RISK-001 / MC-001). This
//! module produces the SYNTAX tokens; the commit path inserts the atom via the EXISTING
//! [`crate::rich_editor::wikilinks::confirm::confirm_wikilink`] insert (which, per the MT-020 undo
//! rewire, applies a transactional `Step::InsertInlineChild` and pushes the receipt on the undo
//! manager — no tag-specific model step is introduced).
//!
//! ## normalize_tag is DEFINED HERE (typed-blocker realism gate — RISK-001 / MC-007)
//!
//! The MT contract directed reusing an `MT-017 normalize_tag(&str) -> String` export so inline tags and
//! property tags collapse to ONE identity. That export does NOT exist: MT-017's property tags
//! ([`crate::rich_editor::properties::PropertiesState::tags`]) are a LOCAL-ONLY `Vec<String>` of RAW
//! strings with a VISIBLE "Tags not persisted (backend gap: coming soon)" banner — there is no
//! normalization function and no backend persistence. So this module DEFINES the canonical inline-tag
//! normalization ([`normalize_tag`]) as the SINGLE shared identity function the convergence builder
//! ([`crate::rich_editor::inline_tags::inline_chip::build_tag_edge_payload`]) keys on. The "one tag, one
//! hub" invariant therefore holds for INLINE tags today, and the property-tag union becomes live only
//! when MT-017 persists tags through THIS same function — that gap is the typed blocker the coder
//! handoff records (TB-058-NORMALIZE). It is NOT worked around with divergent normalization (which would
//! split the hub) and NOT with a backend rewrite.

use std::ops::Range;

use crate::rich_editor::document_model::node::HsLinkNode;

/// A normalized inline tag identity. `name` is the tag string WITHOUT the leading `#` and AS THE AUTHOR
/// TYPED IT (original case preserved for display); the CANONICAL identity used for edge identity +
/// convergence is [`Tag::canonical`] (the [`normalize_tag`] of `name`). The display label is
/// `"#" + name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The tag body the author typed, WITHOUT the leading `#` (e.g. `"#Rust"` -> `name = "Rust"`).
    /// Original case is preserved here for the chip label; identity uses [`Tag::canonical`].
    pub name: String,
}

impl Tag {
    /// Build a tag from a body WITHOUT the leading `#` (trimmed of surrounding whitespace). The body is
    /// stored verbatim (case preserved); the canonical identity is derived via [`Tag::canonical`].
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into().trim().to_owned(),
        }
    }

    /// The canonical identity key for edge identity + property/inline convergence (RISK-001 / MC-001):
    /// the [`normalize_tag`] of the display name. Two tags with the same canonical key are the SAME tag
    /// and MUST resolve to the same hub (`#Rust`, `#rust`, `# rust ` all -> `"rust"`).
    pub fn canonical(&self) -> String {
        normalize_tag(&self.name)
    }

    /// The display label for the inline chip: `"#" + name` (the author-cased body with the leading `#`).
    pub fn display_label(&self) -> String {
        format!("#{}", self.name)
    }
}

/// Materialize a [`Tag`] as the inline-atom [`HsLinkNode`] the document model + backend `content_json`
/// round-trip. The atom is `ref_kind = "tag"`, `ref_value = the canonical identity` (so the backend
/// tag-edge indexer keys it on the same normalized identity property tags converge on), and
/// `label = "#" + display_name` (so the EXISTING chip renderer paints `#name` directly from the
/// non-empty label). `resolved = true` (a committed tag is a live link to its hub). This is the node the
/// commit path inserts via the EXISTING [`crate::rich_editor::wikilinks::confirm::confirm_wikilink`]
/// insert (transactional `Step::InsertInlineChild` + undo receipt per the MT-020 rewire) — NOT a
/// tag-specific model step and NOT a new mark variant (the KERNEL_BUILDER "everything is an hsLink
/// by ref_kind" gate). A tag whose canonical identity is empty (a bare `#`) is never committed, so this
/// always produces a valid atom.
pub fn tag_to_hs_link(tag: &Tag) -> HsLinkNode {
    HsLinkNode {
        ref_kind: "tag".to_owned(),
        ref_value: tag.canonical(),
        label: tag.display_label(),
        resolved: true,
    }
}

/// The canonical normalization for an inline tag identity (RISK-001 / MC-001). This is the SINGLE shared
/// identity function inline tags + the convergence edge builder key on, defined here because MT-017's
/// property tags expose NO such function (they are a local-only raw `Vec<String>` with a backend gap —
/// see the module docs / TB-058-NORMALIZE). The rule mirrors the wikilink resolver's whitespace+case
/// collapse and the Obsidian tag-charset normalization:
///   1. trim surrounding whitespace, drop a leading `#` if present,
///   2. lower-case (so `#Rust` and `#rust` are ONE tag — the one-tag-one-hub invariant),
///   3. keep only the Obsidian tag charset `[A-Za-z0-9_/-]` (collapsing any other char to nothing),
///      preserving `/` so nested tags (`#area/sub` -> `area/sub`) keep their hierarchy.
///
/// Pure + total: any input yields a (possibly empty) canonical string, never a panic. An empty result
/// means the input had no valid tag chars (the caller treats an empty canonical as "not a tag").
pub fn normalize_tag(raw: &str) -> String {
    let trimmed = raw.trim();
    let body = trimmed.strip_prefix('#').unwrap_or(trimmed);
    body.chars()
        .filter(|c| is_tag_body_char(*c))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// True when `c` is a valid char in an Obsidian inline-tag BODY (the charset after the `#`): ASCII
/// letters, ASCII digits, `_`, `-`, and `/` (for nested tags like `#area/sub`). NOTHING else — a tag
/// ends at the first char outside this set (so `see #wip,` ends the tag at the comma).
fn is_tag_body_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '/'
}

/// True when the byte at `idx` in `text` begins a tag word boundary for a `#` — i.e. the `#` is at the
/// very start of the text OR the char immediately before it is whitespace or a non-word punctuation
/// boundary (anything that is NOT a tag-body char). This is the rule that REJECTS a mid-word `#`:
/// `C#` (the `#` is preceded by the word char `C`) and `a#b` (preceded by `a`) do NOT begin a tag.
fn is_word_boundary_before(text: &str, hash_byte_idx: usize) -> bool {
    if hash_byte_idx == 0 {
        return true;
    }
    // The char immediately before the `#` (decode the previous UTF-8 char, byte-safe — RISK-003).
    match text[..hash_byte_idx].chars().next_back() {
        // A tag-body char before the `#` means it is mid-word -> NOT a boundary (e.g. `C#`, `a#b`).
        Some(prev) => !is_tag_body_char(prev),
        None => true,
    }
}

/// A detected inline-tag token in a paragraph's source text: the byte range covered by the WHOLE
/// `#tag` literal (INCLUDING the leading `#`) and the parsed [`Tag`]. The `byte_range` is over the
/// source `text` passed to [`parse_inline_tags`] (UTF-8 byte offsets, safe to slice on a char
/// boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagToken {
    /// The byte span in the source text covered by the `#tag` literal, including the leading `#`.
    pub byte_range: Range<usize>,
    /// The parsed tag (display name without the `#`; canonical identity via [`Tag::canonical`]).
    pub tag: Tag,
}

/// Scan free text for every inline `#tag` occurrence, in document order (the bulk/paste detector and
/// the convergence-set source). A tag begins ONLY at a WORD BOUNDARY (the `#` is at offset 0 or
/// preceded by whitespace / non-word punctuation — [`is_word_boundary_before`]); a mid-word `#` (`C#`,
/// `a#b`) is rejected. The body after the `#` matches the Obsidian charset `[A-Za-z0-9_/-]` and ends at
/// the first char outside it. A bare `#` with no valid body char, and an empty body (`# foo`), are NOT
/// tags. Byte-safe for multi-byte UTF-8 (emoji before a tag does not mis-slice — RISK-003 / MC-003): it
/// walks `char_indices`, never raw byte arithmetic.
pub fn parse_inline_tags(text: &str) -> Vec<TagToken> {
    let mut tokens = Vec::new();
    let bytes_len = text.len();
    let mut chars = text.char_indices().peekable();

    while let Some((hash_idx, ch)) = chars.next() {
        if ch != '#' {
            continue;
        }
        // The `#` must begin at a word boundary (offset 0 or preceded by whitespace/punctuation).
        if !is_word_boundary_before(text, hash_idx) {
            continue;
        }
        // Consume the contiguous body chars right after the `#`.
        let body_start = hash_idx + ch.len_utf8(); // `#` is ASCII (1 byte), but stay byte-correct.
        let mut body_end = body_start;
        while let Some(&(next_idx, next_ch)) = chars.peek() {
            if is_tag_body_char(next_ch) {
                body_end = next_idx + next_ch.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
        // A bare `#` (no body) or an all-invalid body is NOT a tag.
        if body_end == body_start {
            continue;
        }
        let body = &text[body_start..body_end];
        let tag = Tag::new(body);
        // Defensive: a body that normalizes to empty (only invalid chars survived the charset, which
        // cannot happen here since body_start..body_end already passed is_tag_body_char) is skipped.
        if tag.canonical().is_empty() {
            continue;
        }
        tokens.push(TagToken {
            byte_range: hash_idx..body_end.min(bytes_len),
            tag,
        });
    }
    tokens
}

/// Detect an OPEN inline-tag trigger at the END of `text_before_caret`: the caret is inside an
/// unterminated `#tag` token the author is currently typing. Returns `(start_char, query)` where
/// `start_char` is the CHAR offset of the opening `#` within `text_before_caret` (so the caller can
/// compute the char span of `#query` to replace on commit — the doc is CHAR-indexed, RISK-003) and
/// `query` is the tag body typed so far (WITHOUT the `#`).
///
/// Returns `None` when there is no open `#` trigger before the caret. The trigger is open when:
///   - a `#` appears in the trailing run, AND it is at a word boundary (offset 0 or preceded by
///     whitespace/punctuation — so `C#` never opens the menu), AND
///   - every char between that `#` and the caret is a valid tag-body char (so typing a space / comma
///     after the body closes the trigger — the body ended).
///
/// This is the LIVE input-trigger detector the input handler calls each keystroke (mirrors
/// [`crate::rich_editor::wikilinks::parser::open_wikilink_query`] for `[[`). An EMPTY query (`#` just
/// typed) IS an open trigger (the menu opens immediately on `#`, AC-001).
pub fn open_tag_query(text_before_caret: &str) -> Option<(usize, String)> {
    // Find the LAST `#` before the caret (byte offset).
    let hash_byte = text_before_caret.rfind('#')?;
    // It must begin at a word boundary, or it is a mid-word `#` (e.g. `C#`) that never opens the menu.
    if !is_word_boundary_before(text_before_caret, hash_byte) {
        return None;
    }
    // Everything between the `#` and the caret must be valid tag-body chars; otherwise the body ended
    // (a space / comma / newline closed the token) and there is no OPEN trigger.
    let after = &text_before_caret[hash_byte + 1..];
    if !after.chars().all(is_tag_body_char) {
        return None;
    }
    // Convert the byte offset of the `#` to a CHAR offset (the doc addresses text by char).
    let start_char = text_before_caret[..hash_byte].chars().count();
    Some((start_char, after.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_tag_collapses_case_and_strips_hash_and_invalid_chars() {
        // RISK-001 / MC-001: `#Rust`, `#rust`, `# rust ` all collapse to the SAME canonical key.
        assert_eq!(normalize_tag("#Rust"), "rust");
        assert_eq!(normalize_tag("rust"), "rust");
        assert_eq!(normalize_tag("  #RUST  "), "rust");
        assert_eq!(normalize_tag("# rust "), "rust");
        // Nested tags keep the slash hierarchy.
        assert_eq!(normalize_tag("#area/Sub"), "area/sub");
        // Out-of-charset chars are dropped (not a panic).
        assert_eq!(normalize_tag("#wip!!!"), "wip");
        assert_eq!(normalize_tag("#"), "", "a bare # normalizes to empty");
        assert_eq!(normalize_tag("###"), "", "only-# normalizes to empty");
    }

    #[test]
    fn tag_identity_and_display_label() {
        let t = Tag::new("Rust");
        assert_eq!(t.name, "Rust", "display name keeps the author's case");
        assert_eq!(
            t.canonical(),
            "rust",
            "the canonical identity is lower-cased"
        );
        assert_eq!(
            t.display_label(),
            "#Rust",
            "the chip label is #Name with the author's case"
        );
        // A leading/trailing-space body is trimmed.
        assert_eq!(Tag::new("  wip  ").name, "wip");
    }

    #[test]
    fn parse_extracts_two_tags_in_order_with_full_byte_ranges_ac002() {
        // AC-002: parse_inline_tags('learning #rust and #wip today') -> exactly two tokens [rust, wip],
        // each byte_range covering the FULL `#tag` literal INCLUDING the leading `#`.
        let text = "learning #rust and #wip today";
        let toks = parse_inline_tags(text);
        assert_eq!(toks.len(), 2, "exactly two tags (got {toks:?})");
        assert_eq!(toks[0].tag.name, "rust");
        assert_eq!(toks[1].tag.name, "wip");
        // The byte range covers `#rust` (including the `#`).
        assert_eq!(&text[toks[0].byte_range.clone()], "#rust");
        assert_eq!(&text[toks[1].byte_range.clone()], "#wip");
    }

    #[test]
    fn word_boundary_rejects_mid_word_hash_ac001() {
        // AC-001 / RISK-002 / MC-002 adversarial corpus: a `#` preceded by a word char is NOT a tag.
        assert!(
            parse_inline_tags("C#").is_empty(),
            "C# is not a tag (# preceded by 'C')"
        );
        assert!(
            parse_inline_tags("a#b").is_empty(),
            "a#b is not a tag (# preceded by 'a')"
        );
        assert!(
            parse_inline_tags("foo#bar baz").is_empty(),
            "foo#bar is mid-word"
        );
        // A URL fragment: `http://x#y` — the `#` is preceded by 'x' (a word char) -> NOT a tag.
        assert!(
            parse_inline_tags("see http://x#y here").is_empty(),
            "a URL fragment is not a tag"
        );
    }

    #[test]
    fn bare_and_empty_body_hash_are_not_tags_mc002() {
        assert!(parse_inline_tags("#").is_empty(), "a bare # is not a tag");
        assert!(
            parse_inline_tags("# foo").is_empty(),
            "# followed by a space (empty body) is not a tag"
        );
        assert!(
            parse_inline_tags("end. # ").is_empty(),
            "# with no body char after it is not a tag"
        );
    }

    #[test]
    fn boundary_after_punctuation_and_trailing_punct_mc002() {
        // A `#` after punctuation IS a boundary: `(#rust)` -> tag `rust`; trailing punct ends the tag.
        let toks = parse_inline_tags("see #wip, today");
        assert_eq!(toks.len(), 1, "trailing comma ends the tag (got {toks:?})");
        assert_eq!(toks[0].tag.name, "wip");
        assert_eq!(
            &"see #wip, today"[toks[0].byte_range.clone()],
            "#wip",
            "the comma is NOT in the range"
        );

        let nested = parse_inline_tags("#area/sub end");
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].tag.name, "area/sub", "nested tag keeps the slash");

        // After an opening paren the `#` is at a boundary.
        let paren = parse_inline_tags("(#rust)");
        assert_eq!(paren.len(), 1);
        assert_eq!(paren[0].tag.name, "rust");
    }

    #[test]
    fn utf8_emoji_before_tag_is_byte_safe_risk003() {
        // RISK-003 / MC-003: a multi-byte emoji before the tag must not mis-slice the byte range.
        let text = "🚀 #rust"; // the rocket is 4 bytes; the `#` is at byte 5 (space at 4).
        let toks = parse_inline_tags(text);
        assert_eq!(
            toks.len(),
            1,
            "the tag after an emoji is found (got {toks:?})"
        );
        assert_eq!(toks[0].tag.name, "rust");
        // The byte range slices cleanly on a char boundary to `#rust`.
        assert_eq!(&text[toks[0].byte_range.clone()], "#rust");

        // An emoji INSIDE would end the body (emoji is not a tag-body char).
        let mid = parse_inline_tags("#ru🚀st");
        assert_eq!(mid.len(), 1);
        assert_eq!(mid[0].tag.name, "ru", "the emoji ends the tag body");
    }

    #[test]
    fn tag_to_hs_link_is_a_tag_ref_kind_atom() {
        // The Tag -> hsLink atom round-trips content_json: ref_kind=tag, ref_value=canonical, label=#name.
        let link = tag_to_hs_link(&Tag::new("Rust"));
        assert_eq!(link.ref_kind, "tag");
        assert_eq!(
            link.ref_value, "rust",
            "ref_value is the canonical identity (property-tag convergence key)"
        );
        assert_eq!(
            link.label, "#Rust",
            "the chip label is #Name with the author's case"
        );
        assert!(
            link.resolved,
            "a committed tag is a resolved live link to its hub"
        );
    }

    #[test]
    fn open_tag_query_detects_open_trigger() {
        // The `#` just typed opens the menu with an empty query (AC-001).
        assert_eq!(open_tag_query("hello #"), Some((6, String::new())));
        // A partial body refines the query.
        assert_eq!(open_tag_query("hello #ru"), Some((6, "ru".to_owned())));
        // At the very start of the line.
        assert_eq!(open_tag_query("#wip"), Some((0, "wip".to_owned())));
        // Nested tag in progress.
        assert_eq!(
            open_tag_query("see #area/su"),
            Some((4, "area/su".to_owned()))
        );
    }

    #[test]
    fn open_tag_query_rejects_mid_word_and_closed_triggers() {
        // A mid-word `#` (C#) never opens the menu.
        assert_eq!(open_tag_query("C#"), None, "C# is not an open tag trigger");
        assert_eq!(open_tag_query("a#b"), None);
        // A space after the body closes the trigger (the body ended).
        assert_eq!(
            open_tag_query("#wip "),
            None,
            "a trailing space closes the trigger"
        );
        assert_eq!(open_tag_query("#wip done"), None);
        // No `#` at all.
        assert_eq!(open_tag_query("plain text"), None);
        assert_eq!(open_tag_query(""), None);
        // A later open trigger after a finished one re-opens for the new token.
        assert_eq!(open_tag_query("#done and #wi"), Some((10, "wi".to_owned())));
    }
}
