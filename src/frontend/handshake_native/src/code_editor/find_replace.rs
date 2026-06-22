//! In-file find/replace search engine for the native code editor (WP-KERNEL-012 MT-004).
//!
//! [`FindEngine`] searches the [`TextBuffer`](super::buffer::TextBuffer) and computes the byte ranges
//! of every match for a [`FindQuery`], then replaces one or all matches. It is the native analog of the
//! React editor's find panel (`app/src/lib/editor/editor_find_request.ts` `EditorFindOptions`, ported
//! verbatim as [`FindQuery`]) reimplemented over our own buffer because we own the text (no
//! `egui::TextEdit`, no Monaco find widget).
//!
//! ## Regex engine choice (MC-003 — documented in code)
//!
//! Regex search uses the `regex` crate (Rust's standard RE2-style engine), NOT `fancy-regex`. The
//! `regex` crate has LINEAR-TIME guarantees: it compiles to a finite automaton and does NOT backtrack,
//! so a pathological pattern like `(a+)+` or `.*` can never cause catastrophic backtracking / ReDoS
//! (RISK-001). The MT contract's original RISK-001 mitigation named `fancy-regex with a backtrack
//! limit`, but `fancy-regex` IS a backtracking engine and is itself ReDoS-prone — so it is the WRONG
//! choice and is deliberately avoided (KERNEL_BUILDER gate correction). Defense-in-depth here is a
//! [`MAX_PATTERN_LEN`] cap (a 512-char pattern is rejected with an error string) plus
//! empty-match-list-on-compile-error so an invalid pattern never panics (RISK-001 / MC-001 / AC-003).
//!
//! ## Match-offset invalidation after a replace (RISK-003)
//!
//! [`FindEngine::replace_one`] / [`replace_all`](FindEngine::replace_all) mutate the buffer, which
//! shifts every byte offset after the edit. The stored [`Match`] list is therefore STALE the instant a
//! replace runs. `replace_all` processes matches in REVERSE byte order so an earlier replace never
//! invalidates a later match's offset within the SAME batch, but the caller (the panel) MUST re-run
//! [`search`](FindEngine::search) immediately after any replace before using the match list again — the
//! panel's `replace_current` / `replace_all` do exactly that (RISK-003 / MC-002).

use std::ops::Range;

use super::buffer::TextBuffer;
use super::cursor::byte_to_line_col;

/// The maximum find-pattern length (chars). A longer pattern is rejected with an error string rather
/// than compiled, as defense-in-depth against a pathological regex (RISK-001). The `regex` crate is
/// already linear-time, so this is belt-and-suspenders, not the primary ReDoS control.
pub const MAX_PATTERN_LEN: usize = 512;

/// A find query — the Rust port of the React editor's `EditorFindOptions`
/// (`app/src/lib/editor/editor_find_request.ts`: `{ query, caseSensitive, wholeWord, isRegex }`),
/// with field names in snake_case. Drives [`FindEngine::search`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindQuery {
    /// The search text (a literal substring, or a regex source when `is_regex`).
    pub pattern: String,
    /// Match case exactly when `true`; case-insensitive otherwise.
    pub case_sensitive: bool,
    /// Only match when the surrounding chars are not word chars (alphanumeric or `_`).
    pub whole_word: bool,
    /// Interpret `pattern` as a regular expression (`regex` crate syntax) rather than a literal.
    pub is_regex: bool,
}

impl FindQuery {
    /// A plain literal query (the common case: case-insensitive, not whole-word, not regex).
    pub fn literal(pattern: impl Into<String>) -> Self {
        Self { pattern: pattern.into(), ..Default::default() }
    }
}

/// One match: the half-open BYTE range it occupies in the buffer, plus the `(line, col)` of its start
/// (col is a CHAR column from the line start, matching the editor's monospace columns — RISK-002 for
/// non-ASCII). The panel uses `byte_range` to highlight/replace and `line`/`col` to scroll to a match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    /// The half-open `start..end` byte range of the match in the buffer.
    pub byte_range: Range<usize>,
    /// The line index of the match start (0-based).
    pub line: usize,
    /// The CHAR column of the match start from the line start (0-based; aligns with monospace glyph
    /// columns, not bytes).
    pub col: usize,
}

/// The stateless search/replace engine. All methods are associated functions: the engine holds no
/// state of its own (the query + match list live in the panel's `FindState`), so it is a pure
/// transform over a [`FindQuery`] + [`TextBuffer`].
pub struct FindEngine;

impl FindEngine {
    /// Find every occurrence of `query` in `buffer`, in ascending byte order. Returns an empty vec
    /// when the pattern is empty, too long (> [`MAX_PATTERN_LEN`]), or — for a regex — fails to
    /// compile (the caller surfaces the compile error separately via [`compile_error`]). Never panics
    /// (AC-003 / MC-001): a bad regex degrades to an empty list, not an abort.
    pub fn search(query: &FindQuery, buffer: &TextBuffer) -> Vec<Match> {
        if query.pattern.is_empty() || query.pattern.chars().count() > MAX_PATTERN_LEN {
            return Vec::new();
        }
        let text = buffer.to_string();
        let raw: Vec<Range<usize>> = if query.is_regex {
            Self::regex_ranges(query, &text)
        } else {
            Self::plain_ranges(query, &text)
        };
        // Apply the whole-word filter (for both plain and regex matches) then attach line/col.
        raw.into_iter()
            .filter(|r| !query.whole_word || is_whole_word(&text, r))
            .map(|byte_range| {
                let (line, col) = byte_to_line_col(byte_range.start, buffer);
                Match { byte_range, line, col }
            })
            .collect()
    }

    /// If `query.is_regex`, return the regex compile error string (or `None` when it compiles or is
    /// too long/empty). For a non-regex query this is always `None`. The panel stores this in
    /// `FindState.error` so an invalid pattern shows a message instead of silently finding nothing
    /// (AC-003).
    pub fn compile_error(query: &FindQuery) -> Option<String> {
        if !query.is_regex || query.pattern.is_empty() {
            return None;
        }
        if query.pattern.chars().count() > MAX_PATTERN_LEN {
            return Some(format!(
                "pattern too long ({} chars; max {MAX_PATTERN_LEN})",
                query.pattern.chars().count()
            ));
        }
        match Self::build_regex(query) {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        }
    }

    /// Replace a single match `m` in `buffer` with `replacement`. Returns `true` on success. The
    /// caller MUST re-run [`search`](Self::search) afterward — the remaining match offsets are now
    /// stale (RISK-003). A no-op (and `false`) if the range is invalid for the current buffer.
    pub fn replace_one(buffer: &mut TextBuffer, m: &Match, replacement: &str) -> bool {
        if m.byte_range.end > buffer.len_bytes() || m.byte_range.start > m.byte_range.end {
            return false;
        }
        // Delete the matched bytes then insert the replacement at the (now-vacated) start.
        if buffer.delete(m.byte_range.clone()).is_err() {
            return false;
        }
        buffer.insert(m.byte_range.start, replacement).is_ok()
    }

    /// Replace ALL of `matches` in `buffer` with `replacement`, processing in REVERSE byte order so an
    /// earlier replacement never shifts a later match's stored offset out from under it (RISK-003 /
    /// MC-002). Returns the number of replacements actually applied. The caller MUST re-run
    /// [`search`](Self::search) afterward; the input `matches` are fully consumed/invalidated here.
    pub fn replace_all(buffer: &mut TextBuffer, matches: &[Match], replacement: &str) -> usize {
        // Sort a copy by descending start so we mutate high offsets first; the caller's list is already
        // ascending, but never assume — a defensive sort keeps the reverse-order invariant true even if
        // a caller passes an unsorted list.
        let mut ordered: Vec<&Match> = matches.iter().collect();
        ordered.sort_by(|a, b| b.byte_range.start.cmp(&a.byte_range.start));
        let mut applied = 0usize;
        let mut last_start = usize::MAX;
        for m in ordered {
            // Skip a match that overlaps one we just replaced (defensive: overlapping matches in the
            // same batch would corrupt offsets). Ascending-sorted non-overlapping input never hits this.
            if m.byte_range.end > last_start {
                continue;
            }
            if Self::replace_one(buffer, m, replacement) {
                applied += 1;
                last_start = m.byte_range.start;
            }
        }
        applied
    }

    // ── internals ────────────────────────────────────────────────────────────────────────────────────

    /// Literal substring search. Case-insensitive search lowercases BOTH needle and haystack into
    /// parallel strings of equal byte length per ASCII char; for non-ASCII case-insensitivity we fall
    /// back to a regex with the `(?i)` flag on the escaped literal so byte offsets stay correct
    /// (`str::to_lowercase` can change byte length for some Unicode chars, which would desync offsets).
    fn plain_ranges(query: &FindQuery, text: &str) -> Vec<Range<usize>> {
        let needle = &query.pattern;
        if query.case_sensitive {
            return all_literal_occurrences(text, needle);
        }
        // Case-insensitive: only ASCII can be lowercased without changing byte length. If the needle is
        // pure ASCII, do a fast ascii-lowercased scan that preserves byte offsets exactly. Otherwise use
        // the regex engine with a case-insensitive escaped-literal pattern (still linear-time).
        if needle.is_ascii() && text.is_ascii() {
            let hay = text.to_ascii_lowercase();
            let need = needle.to_ascii_lowercase();
            // hay has identical byte length to text (ASCII lowercase is 1:1), so offsets match.
            all_literal_occurrences(&hay, &need)
        } else {
            let escaped = regex::escape(needle);
            match regex::RegexBuilder::new(&escaped).case_insensitive(true).build() {
                Ok(re) => re.find_iter(text).map(|mat| mat.start()..mat.end()).collect(),
                Err(_) => Vec::new(),
            }
        }
    }

    /// Regex search: compile (respecting `case_sensitive`) and collect every non-overlapping match
    /// range. A compile failure yields an empty list (AC-003: no panic); the error string is surfaced
    /// separately by [`compile_error`](Self::compile_error).
    fn regex_ranges(query: &FindQuery, text: &str) -> Vec<Range<usize>> {
        match Self::build_regex(query) {
            Ok(re) => re.find_iter(text).map(|m| m.start()..m.end()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Compile the query's regex with the case flag applied. Returns the `regex::Error` on a bad
    /// pattern (used by both [`regex_ranges`](Self::regex_ranges) and
    /// [`compile_error`](Self::compile_error)).
    fn build_regex(query: &FindQuery) -> Result<regex::Regex, regex::Error> {
        regex::RegexBuilder::new(&query.pattern)
            .case_insensitive(!query.case_sensitive)
            .build()
    }
}

/// Every non-overlapping occurrence of `needle` in `hay`, as byte ranges, scanning left to right and
/// advancing past each match so overlapping matches are not double-counted. `needle` is assumed
/// non-empty (the caller guards that).
fn all_literal_occurrences(hay: &str, needle: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let mut from = 0usize;
    while from <= hay.len() {
        match hay[from..].find(needle) {
            Some(rel) => {
                let start = from + rel;
                let end = start + needle.len();
                out.push(start..end);
                // Advance past this match (non-overlapping). Step at least one byte to avoid a stall on
                // a zero-length situation (impossible for a non-empty needle, but defensive).
                from = end.max(start + 1);
            }
            None => break,
        }
    }
    out
}

/// True when the byte range `r` in `text` is a whole word: the char immediately before `r.start` and
/// the char immediately after `r.end` are NOT word chars (alphanumeric or `_`). Matches the React
/// editor's `wholeWord` semantics and the `\b`-anchor intent in the MT contract. Char-boundary safe.
fn is_whole_word(text: &str, r: &Range<usize>) -> bool {
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    // For non-ASCII boundary chars, `is_word` is false (a CJK char is a word boundary against an ASCII
    // identifier needle); that matches the ASCII-identifier definition the editor uses.
    let before_ok = r.start == 0 || !is_word(bytes[r.start - 1]);
    let after_ok = r.end >= bytes.len() || !is_word(bytes[r.end]);
    before_ok && after_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranges(matches: &[Match]) -> Vec<Range<usize>> {
        matches.iter().map(|m| m.byte_range.clone()).collect()
    }

    #[test]
    fn plain_case_sensitive_finds_exact_case() {
        let buf = TextBuffer::new("Foo foo FOO foo");
        let q = FindQuery { pattern: "foo".into(), case_sensitive: true, ..Default::default() };
        // Only the two lowercase "foo" at bytes 4..7 and 12..15.
        assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![4..7, 12..15]);
    }

    #[test]
    fn plain_case_insensitive_finds_all_cases() {
        let buf = TextBuffer::new("Foo foo FOO");
        let q = FindQuery { pattern: "foo".into(), case_sensitive: false, ..Default::default() };
        assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![0..3, 4..7, 8..11]);
    }

    #[test]
    fn whole_word_excludes_substrings() {
        let buf = TextBuffer::new("foo foobar foo_baz foo");
        let q = FindQuery {
            pattern: "foo".into(),
            case_sensitive: true,
            whole_word: true,
            ..Default::default()
        };
        // "foo" (0..3) is whole; "foobar" and "foo_baz" are not; the trailing "foo" (19..22) is whole.
        assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![0..3, 19..22]);
    }

    #[test]
    fn regex_finds_pattern() {
        let buf = TextBuffer::new("fn a() {}\nfn bb() {}\nlet c = 1;");
        let q = FindQuery { pattern: r"fn \w+".into(), is_regex: true, case_sensitive: true, ..Default::default() };
        // "fn a" (0..4) and "fn bb" (10..15).
        assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![0..4, 10..15]);
    }

    #[test]
    fn no_match_returns_empty_vec() {
        let buf = TextBuffer::new("hello world");
        let q = FindQuery::literal("zzz");
        assert!(FindEngine::search(&q, &buf).is_empty());
    }

    #[test]
    fn empty_pattern_returns_empty_vec() {
        let buf = TextBuffer::new("hello");
        let q = FindQuery::literal("");
        assert!(FindEngine::search(&q, &buf).is_empty());
    }

    #[test]
    fn match_carries_correct_line_and_col() {
        let buf = TextBuffer::new("alpha\nbeta target\ngamma");
        let q = FindQuery::literal("target");
        let m = FindEngine::search(&q, &buf);
        assert_eq!(m.len(), 1);
        // "target" starts on line 1 at char column 5 (after "beta ").
        assert_eq!(m[0].line, 1);
        assert_eq!(m[0].col, 5);
    }

    #[test]
    fn search_wraps_naturally_over_whole_buffer() {
        // All occurrences are returned in order regardless of any cursor position (the panel handles
        // "current match" wrapping by indexing into this full list, so the engine returns every match).
        let buf = TextBuffer::new("x x x x");
        let q = FindQuery::literal("x");
        assert_eq!(ranges(&FindEngine::search(&q, &buf)), vec![0..1, 2..3, 4..5, 6..7]);
    }

    #[test]
    fn replace_one_replaces_correct_match_and_adjusts_buffer() {
        let mut buf = TextBuffer::new("foo bar foo");
        let q = FindQuery::literal("foo");
        let matches = FindEngine::search(&q, &buf);
        assert_eq!(matches.len(), 2);
        // Replace the SECOND match (8..11) with "X"; the first is untouched.
        assert!(FindEngine::replace_one(&mut buf, &matches[1], "X"));
        assert_eq!(buf.to_string(), "foo bar X");
    }

    #[test]
    fn replace_all_replaces_all_occurrences_offset_correct() {
        // MC-002: replace_all must replace every occurrence with correct offsets (reverse order so a
        // length-changing replacement does not corrupt later offsets).
        let mut buf = TextBuffer::new("foo foo foo");
        let q = FindQuery::literal("foo");
        let matches = FindEngine::search(&q, &buf);
        assert_eq!(matches.len(), 3);
        // Replace with a LONGER string so a forward (wrong-order) walk would corrupt offsets.
        let applied = FindEngine::replace_all(&mut buf, &matches, "LONGER");
        assert_eq!(applied, 3);
        assert_eq!(buf.to_string(), "LONGER LONGER LONGER");
    }

    #[test]
    fn replace_all_with_shorter_replacement_offset_correct() {
        let mut buf = TextBuffer::new("aaaa aaaa aaaa");
        let q = FindQuery::literal("aaaa");
        let matches = FindEngine::search(&q, &buf);
        let applied = FindEngine::replace_all(&mut buf, &matches, "b");
        assert_eq!(applied, 3);
        assert_eq!(buf.to_string(), "b b b");
    }

    #[test]
    fn invalid_regex_returns_empty_and_error_string_no_panic() {
        // AC-003 / RISK-001: an unbalanced group is a compile error -> empty match list + a non-empty
        // error string, and absolutely no panic.
        let buf = TextBuffer::new("some (text) here");
        let q = FindQuery { pattern: "(".into(), is_regex: true, ..Default::default() };
        assert!(FindEngine::search(&q, &buf).is_empty(), "bad regex -> no matches");
        let err = FindEngine::compile_error(&q);
        assert!(err.is_some(), "bad regex -> a compile error string");
        assert!(!err.unwrap().is_empty(), "the error string is non-empty");
    }

    #[test]
    fn valid_regex_has_no_compile_error() {
        let q = FindQuery { pattern: r"\d+".into(), is_regex: true, ..Default::default() };
        assert!(FindEngine::compile_error(&q).is_none());
    }

    #[test]
    fn catastrophic_pattern_does_not_hang() {
        // RISK-001: the `regex` crate is linear-time, so a classic catastrophic-backtracking pattern on
        // a long input completes immediately (a backtracking engine would hang here for seconds).
        let buf = TextBuffer::new(&"a".repeat(2000));
        let q = FindQuery { pattern: "(a+)+$".into(), is_regex: true, ..Default::default() };
        // Just assert it RETURNS (no hang, no panic). One match spanning the whole run is expected.
        let m = FindEngine::search(&q, &buf);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn over_long_pattern_is_rejected() {
        let buf = TextBuffer::new("data");
        let long = "a".repeat(MAX_PATTERN_LEN + 1);
        let q = FindQuery { pattern: long, ..Default::default() };
        assert!(FindEngine::search(&q, &buf).is_empty(), "over-long pattern finds nothing");
        let q_re = FindQuery { pattern: "a".repeat(MAX_PATTERN_LEN + 1), is_regex: true, ..Default::default() };
        assert!(FindEngine::compile_error(&q_re).is_some(), "over-long regex reports the cap error");
    }

    #[test]
    fn non_ascii_case_insensitive_keeps_byte_offsets() {
        // "café" + "CAFÉ": case-insensitive must find both with byte-correct ranges (the regex
        // fallback path). "café" = 5 bytes (é=2), "CAFÉ" = 5 bytes (É=2). Separated by a space.
        let buf = TextBuffer::new("café CAFÉ");
        let q = FindQuery { pattern: "café".into(), case_sensitive: false, ..Default::default() };
        let m = FindEngine::search(&q, &buf);
        assert_eq!(m.len(), 2, "both cases found");
        assert_eq!(m[0].byte_range, 0..5);
        assert_eq!(m[1].byte_range, 6..11);
    }
}
