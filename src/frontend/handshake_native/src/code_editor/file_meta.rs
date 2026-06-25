//! File-metadata model: EOL / indent / encoding / render-whitespace (WP-KERNEL-012 MT-071, E11).
//!
//! The VS-Code-parity file-metadata controls the status-bar segments (MT-071) drive. Pure, egui-free
//! logic over the document text; the per-document STATE (which EOL, which encoding, the indent prefs,
//! the whitespace toggle) hangs off the MT-010 document model
//! ([`super::panel::CodeEditorPanel`]) — NOT a parallel store (RISK-004/MC-004).
//!
//! ## EOL convert is ONE undo step (RISK-002/MC-002)
//!
//! [`Eol::rewrite`] returns the WHOLE document with its line endings rewritten as a single new string.
//! The panel applies it through [`super::panel::CodeEditorPanel::set_text`] — the same whole-buffer
//! replace the MT-035/050 single-undo path uses — so ONE Ctrl+Z reverts the entire conversion. It is
//! NEVER emitted as per-line edits (which would make a single undo revert only the last line and
//! corrupt the undo history).
//!
//! ## Defaults when ambiguous (MC-007)
//!
//! `Eol::Lf`, `IndentKind::Spaces` size 4, `Encoding::Utf8` — to avoid a surprising mid-edit Tab-key
//! flip on a file the heuristics cannot read confidently.

/// A line-ending style. `Lf` is `\n` (Unix), `Crlf` is `\r\n` (Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    Crlf,
}

impl Eol {
    /// The default when detection is ambiguous (an empty / single-line buffer): LF (MC-007).
    pub const DEFAULT: Eol = Eol::Lf;

    /// The literal line-ending bytes this EOL writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::Crlf => "\r\n",
        }
    }

    /// The compact status-bar label (`LF` / `CRLF`), matching VS Code.
    pub fn label(self) -> &'static str {
        match self {
            Eol::Lf => "LF",
            Eol::Crlf => "CRLF",
        }
    }

    /// Detect the dominant EOL of `text` by counting `\r\n` vs LONE `\n` — majority wins. A buffer with
    /// no line breaks (or a tie) yields the [`DEFAULT`](Eol::DEFAULT) (LF, MC-007).
    pub fn detect(text: &str) -> Eol {
        let crlf = text.matches("\r\n").count();
        // Lone `\n` = total `\n` minus the ones that are part of a `\r\n`.
        let total_lf = text.matches('\n').count();
        let lone_lf = total_lf.saturating_sub(crlf);
        if crlf > lone_lf {
            Eol::Crlf
        } else {
            // Ties + no-line-break buffers default to LF.
            Eol::Lf
        }
    }

    /// Rewrite EVERY line ending in `text` to this EOL and return the WHOLE new document as one string
    /// (the single-undo transaction unit — RISK-002/MC-002). Normalizes `\r\n` and lone `\r` and lone
    /// `\n` all to a canonical `\n` first, then re-emits with the target ending, so a mixed-EOL file
    /// converts cleanly and idempotently (re-running the same target is a no-op).
    pub fn rewrite(self, text: &str) -> String {
        // Normalize to LF: collapse CRLF first, then any stray lone CR (old-Mac) to LF.
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        match self {
            Eol::Lf => normalized,
            Eol::Crlf => normalized.replace('\n', "\r\n"),
        }
    }
}

impl Default for Eol {
    fn default() -> Self {
        Eol::DEFAULT
    }
}

/// Whether one indent unit is tabs or spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentKind {
    Tabs,
    Spaces,
}

/// An indent style: tabs-or-spaces + the display width of one indent unit. For `Tabs` the `size` is the
/// tab DISPLAY width; for `Spaces` it is the number of spaces per indent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentStyle {
    pub kind: IndentKind,
    pub size: usize,
}

impl IndentStyle {
    /// The default when detection is ambiguous: 4 spaces (MC-007 — VS Code's default, avoids a
    /// surprising mid-edit Tab-key flip).
    pub const DEFAULT: IndentStyle = IndentStyle {
        kind: IndentKind::Spaces,
        size: 4,
    };

    /// The compact status-bar label (`Spaces: 4` / `Tab Size: 4`), matching VS Code's status segment.
    pub fn label(self) -> String {
        match self.kind {
            IndentKind::Spaces => format!("Spaces: {}", self.size),
            IndentKind::Tabs => format!("Tab Size: {}", self.size),
        }
    }
}

impl Default for IndentStyle {
    fn default() -> Self {
        IndentStyle::DEFAULT
    }
}

/// Detect the indent style of `text`. If any line's LEADING indentation contains a tab, the file is
/// tab-indented ([`IndentKind::Tabs`], size kept at the default display width 4). Otherwise infer the
/// space size from the MOST COMMON positive leading-space delta between consecutive DISTINCT indent
/// levels, defaulting to 4 when nothing conclusive is found (MC-007). Mixed/empty files fall back to
/// the [`DEFAULT`](IndentStyle::DEFAULT) (Spaces 4), so the Tab key never flips to a surprising mode
/// (RISK-007).
///
/// Conservative single-level rule (RISK-007 hardening, adversarial review must-fix #1): when there is
/// only ONE distinct space-indent level (no consecutive-level delta to measure), the function does NOT
/// infer the unit from that lone level — a single `        deep` line (8 spaces) is NOT evidence that
/// one indent unit is 8 spaces. With no delta evidence the result is the [`DEFAULT`] (Spaces 4). Any
/// inferred unit is clamped to the sane VS-Code-style set {2, 4, 8}; an out-of-set or ambiguous
/// inference collapses to 4. This keeps `Tab`/`Dedent` on the conventional 4-space unit instead of
/// silently flipping the document's indent unit on a file that happens to have one deep indent.
pub fn detect_indent(text: &str) -> IndentStyle {
    let mut saw_leading_tab = false;
    // Histogram of leading-space counts on space-indented lines.
    let mut space_indents: Vec<usize> = Vec::new();
    for line in text.lines() {
        let leading: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        if leading.is_empty() {
            continue;
        }
        if leading.contains('\t') {
            saw_leading_tab = true;
            // A tab in the leading indent is decisive for Tabs; keep scanning only to confirm.
            continue;
        }
        // Pure-space leading indent.
        space_indents.push(leading.len());
    }
    if saw_leading_tab {
        return IndentStyle { kind: IndentKind::Tabs, size: 4 };
    }
    if space_indents.is_empty() {
        return IndentStyle::DEFAULT;
    }
    // Infer the unit ONLY from the most common positive delta between consecutive DISTINCT indent
    // levels. The MT names exactly this ("the most common positive leading-space delta"). A single lone
    // indent level produces NO delta, and we deliberately do NOT fall back to that lone level as the
    // unit (that is the RISK-007 mis-inference the review caught: one 8-space line -> size 8 -> Dedent
    // removes 8). With no delta evidence we keep the safe default unit (4).
    let mut deltas: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut sorted = space_indents.clone();
    sorted.sort_unstable();
    sorted.dedup();
    for w in sorted.windows(2) {
        let delta = w[1] - w[0];
        if delta > 0 {
            *deltas.entry(delta).or_insert(0) += 1;
        }
    }
    let inferred = deltas
        .iter()
        .max_by_key(|(delta, count)| (*count, std::cmp::Reverse(**delta)))
        .map(|(delta, _)| *delta)
        // No delta evidence (single distinct indent level, or no indented lines after dedup):
        // keep the safe default unit rather than guessing from a lone level (RISK-007).
        .unwrap_or(IndentStyle::DEFAULT.size);
    // Clamp to the sane VS-Code-style unit set; anything else collapses to the default (4). This stops
    // an odd one-off delta (e.g. 3, 5, 7) from becoming the live Tab/Dedent unit mid-edit.
    let size = match inferred {
        2 | 4 | 8 => inferred,
        _ => IndentStyle::DEFAULT.size,
    };
    IndentStyle { kind: IndentKind::Spaces, size }
}

/// A text encoding the editor can read on load + display in the status bar. The default is UTF-8.
/// Reopening a document under a different encoding re-decodes the on-disk bytes through the MT-010
/// document load path (no backend call — RISK-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    /// The default when no BOM is present (MC-007).
    pub const DEFAULT: Encoding = Encoding::Utf8;

    /// All encodings, for the status-bar picker rows.
    pub const ALL: [Encoding; 4] = [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
    ];

    /// The compact status-bar label, matching VS Code's encoding segment.
    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 with BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
        }
    }

    /// A stable kebab id for the encoding (the status-bar picker item author_id suffix + a settings key).
    pub fn id(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf8",
            Encoding::Utf8Bom => "utf8-bom",
            Encoding::Utf16Le => "utf16-le",
            Encoding::Utf16Be => "utf16-be",
        }
    }

    /// Detect the encoding of `bytes` from a BOM if present, else assume UTF-8 (the MT-071 load rule —
    /// "read a BOM if present, else assume Utf8"). Pure byte inspection; no allocation.
    pub fn detect_bom(bytes: &[u8]) -> Encoding {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            Encoding::Utf8Bom
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            Encoding::Utf16Le
        } else if bytes.starts_with(&[0xFE, 0xFF]) {
            Encoding::Utf16Be
        } else {
            Encoding::DEFAULT
        }
    }

    /// Decode `bytes` under this encoding into a `String` (lossily for invalid sequences, never a
    /// panic — a mis-detected encoding degrades to replacement chars rather than aborting). This is the
    /// in-process re-decode the MT-010 "reopen with encoding" action routes through; NO backend call.
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Encoding::Utf8Bom => {
                let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
                String::from_utf8_lossy(body).into_owned()
            }
            Encoding::Utf16Le => decode_utf16(bytes, &[0xFF, 0xFE], true),
            Encoding::Utf16Be => decode_utf16(bytes, &[0xFE, 0xFF], false),
        }
    }
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::DEFAULT
    }
}

/// Decode UTF-16 bytes (stripping a matching BOM if present). `little_endian` selects byte order. A
/// trailing odd byte is dropped (best-effort, never a panic). Invalid code units become the Unicode
/// replacement char.
fn decode_utf16(bytes: &[u8], bom: &[u8; 2], little_endian: bool) -> String {
    let body = bytes.strip_prefix(bom).unwrap_or(bytes);
    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| {
            if little_endian {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            }
        })
        .collect();
    String::from_utf16_lossy(&units)
}

/// The render-whitespace toggle state — `true` renders middots for spaces + arrows for tabs in the
/// MT-001 editor draw path (which READS this flag). This module only owns the state shape; the panel
/// holds the live value and the draw path reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderWhitespace(pub bool);

impl RenderWhitespace {
    /// The compact status-bar label (a check-mark hint), matching VS Code's whitespace toggle.
    pub fn label(self) -> &'static str {
        if self.0 {
            "Whitespace: On"
        } else {
            "Whitespace: Off"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eol_detect_majority() {
        assert_eq!(Eol::detect("a\nb\nc\n"), Eol::Lf);
        assert_eq!(Eol::detect("a\r\nb\r\nc\r\n"), Eol::Crlf);
        // No line breaks -> default LF (MC-007).
        assert_eq!(Eol::detect("single line"), Eol::Lf);
        // Mixed, CRLF majority -> CRLF.
        assert_eq!(Eol::detect("a\r\nb\r\nc\n"), Eol::Crlf);
        // Tie -> LF default.
        assert_eq!(Eol::detect("a\r\nb\n"), Eol::Lf);
    }

    #[test]
    fn eol_rewrite_round_trips_byte_for_byte() {
        let lf = "line1\nline2\nline3\n";
        // LF -> CRLF -> LF restores the original exactly (the single-undo round trip — RISK-002).
        let crlf = Eol::Crlf.rewrite(lf);
        assert_eq!(crlf, "line1\r\nline2\r\nline3\r\n");
        let back = Eol::Lf.rewrite(&crlf);
        assert_eq!(back, lf, "LF->CRLF->LF restores byte-for-byte");
        // Idempotent: rewriting to the same EOL is a no-op.
        assert_eq!(Eol::Lf.rewrite(lf), lf);
        // Mixed input normalizes cleanly.
        assert_eq!(Eol::Lf.rewrite("a\r\nb\rc\nd"), "a\nb\nc\nd");
    }

    #[test]
    fn indent_detect_tabs_vs_spaces() {
        let tabs = "fn f() {\n\tlet x = 1;\n\treturn x;\n}\n";
        assert_eq!(detect_indent(tabs).kind, IndentKind::Tabs);

        let spaces4 = "def f():\n    x = 1\n    if x:\n        return x\n";
        let s = detect_indent(spaces4);
        assert_eq!(s.kind, IndentKind::Spaces);
        assert_eq!(s.size, 4);

        let spaces2 = "function f() {\n  let x = 1;\n  if (x) {\n    return x;\n  }\n}\n";
        let s2 = detect_indent(spaces2);
        assert_eq!(s2.kind, IndentKind::Spaces);
        assert_eq!(s2.size, 2);

        // Empty / no indent -> default Spaces 4 (MC-007).
        assert_eq!(detect_indent(""), IndentStyle::DEFAULT);
        assert_eq!(detect_indent("a\nb\nc\n"), IndentStyle::DEFAULT);
    }

    #[test]
    fn indent_single_lone_level_stays_default_not_eight() {
        // RISK-007 / adversarial-review must-fix #1: a SINGLE lone indent level is NOT evidence of the
        // indent unit. One 8-space line must NOT make detect_indent infer size=8 (which previously
        // regressed DedentLine to remove 8 spaces). With no consecutive-level delta to measure, the unit
        // stays the safe default (Spaces 4) so Tab/Dedent keeps the conventional 4-space step.
        assert_eq!(detect_indent("        deep"), IndentStyle::DEFAULT, "lone 8-space line -> default 4");
        assert_eq!(detect_indent("    one\n    two\n    three"), IndentStyle::DEFAULT, "one repeated 4-space level (no delta) -> default 4");
        assert_eq!(detect_indent("      six"), IndentStyle::DEFAULT, "lone 6-space line -> default 4");
        // An odd inferred delta (3) is out of the sane set {2,4,8} and collapses to the default.
        assert_eq!(detect_indent("x\n   a\n      b").size, 4, "out-of-set delta (3) clamps to default 4");
        // 8-space unit is honored only with real delta evidence (two distinct levels 8 apart).
        let eight_unit = detect_indent("a\n        b\n                c");
        assert_eq!(eight_unit.kind, IndentKind::Spaces);
        assert_eq!(eight_unit.size, 8, "real 8-space delta evidence -> size 8");
    }

    #[test]
    fn encoding_bom_detection() {
        assert_eq!(Encoding::detect_bom(b"plain"), Encoding::Utf8);
        assert_eq!(Encoding::detect_bom(&[0xEF, 0xBB, 0xBF, b'h', b'i']), Encoding::Utf8Bom);
        assert_eq!(Encoding::detect_bom(&[0xFF, 0xFE, b'h', 0]), Encoding::Utf16Le);
        assert_eq!(Encoding::detect_bom(&[0xFE, 0xFF, 0, b'h']), Encoding::Utf16Be);
    }

    #[test]
    fn encoding_decode_round_trip() {
        // UTF-8 with BOM strips the BOM.
        let mut bom_bytes = vec![0xEF, 0xBB, 0xBF];
        bom_bytes.extend_from_slice("héllo".as_bytes());
        assert_eq!(Encoding::Utf8Bom.decode(&bom_bytes), "héllo");

        // UTF-16 LE round trip.
        let le: Vec<u8> = "hi".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(Encoding::Utf16Le.decode(&le), "hi");

        // UTF-16 BE round trip.
        let be: Vec<u8> = "hi".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        assert_eq!(Encoding::Utf16Be.decode(&be), "hi");
    }

    #[test]
    fn labels_match_vscode_convention() {
        assert_eq!(Eol::Lf.label(), "LF");
        assert_eq!(Eol::Crlf.label(), "CRLF");
        assert_eq!(IndentStyle { kind: IndentKind::Spaces, size: 2 }.label(), "Spaces: 2");
        assert_eq!(IndentStyle { kind: IndentKind::Tabs, size: 4 }.label(), "Tab Size: 4");
        assert_eq!(Encoding::Utf8.label(), "UTF-8");
        assert_eq!(RenderWhitespace(true).label(), "Whitespace: On");
    }
}
