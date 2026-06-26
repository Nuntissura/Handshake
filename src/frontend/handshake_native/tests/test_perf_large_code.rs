//! WP-KERNEL-012 MT-045 — E8 Large-Document Performance Proof, code-editor scenarios (LC-01..LC-08).
//!
//! ## Reality split (KERNEL_BUILDER gate 2026-06-26, same honest split as MT-044)
//!
//! LC-01..LC-05, LC-07, LC-08 are FRONTEND-ONLY: they exercise the REAL native
//! `handshake_native::code_editor::*` impls (ropey `TextBuffer`, the tree-sitter `Highlighter`, the
//! virtualized `CodeEditorPanel`, `CursorSet::insert_at_all`, `FindEngine`, `Minimap`, the gutter
//! diagnostic store) with NO PostgreSQL and REAL measured timings on this host. They RUN + PASS NOW and
//! WRITE-BACK their `measured_value` + `PASS` into `tests/perf_proof/perf_manifest.json`.
//!
//! LC-06 (large-codebase index, 500 files) BINDS the handshake_core code-nav indexer. In THIS crate the
//! code-nav surface is a backend CLIENT (`code_editor::code_nav::CodeNavClient`) — there is NO in-process
//! workspace indexer; symbols are produced by handshake_core behind PostgreSQL. So LC-06 is honestly
//! `#[ignore]`d `requires_pg` (manifest status REQUIRES_PG): it hits the REAL code-nav route against a
//! live managed PostgreSQL, never a mock, and is NOT a permanent ignore (it PASSES with `--ignored` once
//! a seeded backend is up). This is the contract's `NEEDS_MANAGED_RESOURCE_PROOF` honest gate.
//!
//! ## No mock smuggling (RISK-2 / CTRL-2)
//!
//! Every frontend proof calls a real native impl by its fully-qualified Rust path. There is NO sqlite,
//! NO in-memory backend stub, and NO hard-coded result substituted for a real impl call. The
//! `Instant::now()` that brackets each MEASURED operation is placed AFTER all fixture setup, so synthetic
//! file/buffer construction is never counted in a code-editor budget.
//!
//! ## Budgets are overridable (RISK-1 / CTRL-1)
//!
//! Every gate reads `PERF_BUDGET_LCxx_MS` (or `_MB`) and records the MEASURED value, not just PASS, so a
//! slow host widens the ceiling without a code change and a reviewer sees the real cost. Run with
//! `--nocapture` to see the printed `measured=…ms … PASS` lines the proof_targets grep for.

mod perf_proof_support;
mod pg_proof_support;

use perf_proof_support::{
    assert_no_local_artifact_dir, measure_rss_delta_median, record, skip_all, Budget,
};

use std::time::Instant;

use handshake_native::code_editor::buffer::TextBuffer;
use handshake_native::code_editor::cursor::{Cursor, CursorSet};
use handshake_native::code_editor::find_replace::{FindEngine, FindQuery};
use handshake_native::code_editor::gutter::{DiagnosticSeverity, GutterMarker};
use handshake_native::code_editor::highlight::{HighlightSpan, LanguageRegistry};
use handshake_native::code_editor::minimap::Minimap;
use handshake_native::code_editor::panel::CodeEditorPanel;

/// The number of trivial flat functions in the synthetic 10k-line Rust file. Each line is
/// `fn fN() -> u32 { N }` — valid Rust the tree-sitter Rust grammar parses (impl note 1).
const FLAT_FN_LINES: usize = 10_000;

/// Build the LC-01 synthetic 10k-line Rust source. In ADDITION to the 10k flat functions it embeds ONE
/// deeply-nested function (10 levels of nested `{ … }` blocks) so the AST-DEPTH path is stressed, not
/// only the line-count path (RISK-3 / CTRL-3). The generation is a deterministic counter loop (no RNG)
/// so a failure is reproducible (impl note 4).
fn synth_10k_rust() -> String {
    let mut src = String::with_capacity(FLAT_FN_LINES * 24 + 512);
    // 10k flat fns.
    for i in 0..FLAT_FN_LINES {
        src.push_str(&format!("fn f{i}() -> u32 {{ {i} }}\n"));
    }
    // One deeply-nested fn: 10 levels of nested blocks each binding a value. This is the AST-depth
    // stressor — a real Rust file with deep nesting is harder for the parser than flat lines.
    src.push_str("fn deeply_nested() -> u32 {\n");
    for lvl in 0..10 {
        src.push_str(&"    ".repeat(lvl + 1));
        src.push_str(&format!("{{ let v{lvl} = {lvl};\n"));
    }
    src.push_str(&"    ".repeat(11));
    src.push_str("let total = 0;\n");
    for lvl in (0..10).rev() {
        src.push_str(&"    ".repeat(lvl + 1));
        src.push_str("}\n");
    }
    src.push_str("    total\n}\n");
    src
}

// ── LC-01: initial render of a 10k-line file — load + tree-sitter highlight pass <= 200 ms ────────

#[test]
fn perf_lc01_initial_render() {
    if skip_all() {
        return;
    }
    assert_no_local_artifact_dir();
    let budget = Budget::resolve("LC-01", "PERF_BUDGET_LC01_MS", 200);

    // FIXTURE (NOT timed): synthesize the 10k-line source + the deeply-nested fn, and build the
    // highlighter. Construction of the source string and grammar loading is setup, excluded from the
    // budget (RISK-2 / CTRL-2 — Instant::now() is placed AFTER this).
    let src = synth_10k_rust();
    assert!(
        src.lines().count() >= FLAT_FN_LINES,
        "LC-01: the synthetic source must be >= {FLAT_FN_LINES} lines (got {})",
        src.lines().count()
    );
    let registry = LanguageRegistry::with_bundled_languages();
    let mut highlighter = registry
        .highlighter_for_extension("rs")
        .expect("LC-01: the bundled registry must provide a Rust highlighter");
    let src_bytes = src.into_bytes();

    // MEASURED: load into the rope buffer AND run the tree-sitter parse + first highlighted-span
    // emission. The contract requires the parse to COMPLETE (not be deferred), so we assert spans came
    // back. Timer starts here, after all setup.
    let t0 = Instant::now();
    let buffer = TextBuffer::new(std::str::from_utf8(&src_bytes).unwrap());
    let spans: Vec<HighlightSpan> = highlighter.highlight(&src_bytes);
    let elapsed_ms = t0.elapsed().as_millis();

    assert!(
        buffer.len_lines() >= FLAT_FN_LINES,
        "LC-01: the 10k-line buffer must load ({} lines)",
        buffer.len_lines()
    );
    assert!(
        !spans.is_empty(),
        "LC-01: the tree-sitter parse must COMPLETE and emit >= 1 highlighted span (got 0 — the parse \
         was deferred or failed)"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LC-01: 10k-line load + highlight {elapsed_ms} ms must be <= {} ms (override {}). Spans: {}",
        budget.ceiling,
        budget.env_var,
        spans.len()
    );

    println!(
        "LC-01 measured={elapsed_ms}ms (<= {}ms) PASS — 10k-line load + tree-sitter highlight ({} \
         spans, AST-depth nested fn included) [{}]",
        budget.ceiling,
        spans.len(),
        budget.provenance()
    );
    record("LC-01", elapsed_ms as f64, "PASS");
}

// ── LC-02: scroll a 10k-line file to the last line — virtualized paint <= 16 ms ───────────────────

#[test]
fn perf_lc02_scroll_to_bottom() {
    if skip_all() {
        return;
    }
    let budget = Budget::resolve("LC-02", "PERF_BUDGET_LC02_MS", 16);

    // FIXTURE (NOT timed): build a 10k-line panel and a headless egui harness; run one frame to lay out
    // the top (measure line height). Virtualization (MT-002) means only the visible window paints.
    use egui_kittest::Harness;
    use std::sync::Arc;

    let big = "let x = 1; // a line of code\n".repeat(FLAT_FN_LINES);
    let panel = Arc::new(CodeEditorPanel::new(&big, "rs"));
    assert!(
        panel.buffer().len_lines() >= FLAT_FN_LINES,
        "LC-02: the 10k-line buffer must load ({} lines)",
        panel.buffer().len_lines()
    );
    let panel_for_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui(move |ui| {
            panel_for_ui.show(ui);
        });
    harness.run(); // top-of-file layout (setup)

    // MEASURED: move the viewport to the last line and paint the frame that lands it. Timer starts after
    // the viewport-move call so we measure the paint-to-correct-window cost, the virtualized frame.
    let last_line = FLAT_FN_LINES - 1;
    let t0 = Instant::now();
    panel.scroll_to_line(last_line);
    harness.run();
    harness.run(); // settle the forced offset
    let elapsed_ms = t0.elapsed().as_millis();

    let visible = panel.last_visible_range();
    assert!(
        visible.contains(&last_line),
        "LC-02: the last line {last_line} must be inside the painted window {visible:?} after scroll"
    );
    // Virtualization invariant: a bounded window was painted, NOT 10k lines.
    let stats = panel.perf_stats();
    assert!(
        stats.frame_lines_rendered > 0 && stats.frame_lines_rendered < 1_000,
        "LC-02: virtualized — a bounded window painted, not 10k lines (got {})",
        stats.frame_lines_rendered
    );
    assert!(
        budget.passes(elapsed_ms),
        "LC-02: scroll-to-bottom paint {elapsed_ms} ms must be <= {} ms (override {}); painted {} of \
         {} lines",
        budget.ceiling,
        budget.env_var,
        stats.frame_lines_rendered,
        stats.buffer_len_lines
    );

    println!(
        "LC-02 measured={elapsed_ms}ms (<= {}ms) PASS — scroll 10k-line to last line, virtualized \
         window {visible:?} (painted {} of {} lines)",
        budget.ceiling, stats.frame_lines_rendered, stats.buffer_len_lines
    );
    record("LC-02", elapsed_ms as f64, "PASS");
}

// ── LC-03: find/replace across a 10k-line file — 200 matches, search <= 100 ms, replace <= 100 ms ─

#[test]
fn perf_lc03_find_replace() {
    if skip_all() {
        return;
    }
    let search_budget = Budget::resolve("LC-03", "PERF_BUDGET_LC03_MS", 100);
    let replace_budget = Budget::resolve("LC-03", "PERF_BUDGET_LC03_MS", 100);

    // FIXTURE (NOT timed): a 10k-line buffer where exactly 200 lines (every 50th) contain the token
    // "NEEDLE". The other 9800 lines are filler with no occurrence — so search must scan the whole 10k
    // buffer but find exactly 200.
    let mut src = String::with_capacity(FLAT_FN_LINES * 16);
    for i in 0..FLAT_FN_LINES {
        if i % 50 == 0 {
            src.push_str("let NEEDLE = 1;\n");
        } else {
            src.push_str("let filler = 0;\n");
        }
    }
    let mut buffer = TextBuffer::new(&src);
    assert!(buffer.len_lines() >= FLAT_FN_LINES, "LC-03: 10k-line buffer loaded");
    let query = FindQuery::literal("NEEDLE");

    // MEASURED (search): collect all 200 match spans over the whole 10k buffer.
    let t_search = Instant::now();
    let matches = FindEngine::search(&query, &buffer);
    let search_ms = t_search.elapsed().as_millis();
    assert_eq!(
        matches.len(),
        200,
        "LC-03: search must collect exactly 200 match spans across the 10k buffer (got {})",
        matches.len()
    );
    assert!(
        search_budget.passes(search_ms),
        "LC-03: 200-match search {search_ms} ms must be <= {} ms (override {})",
        search_budget.ceiling,
        search_budget.env_var
    );

    // MEASURED (replace): replace-all, then assert exactly 200 replacements landed.
    let t_replace = Instant::now();
    let replaced = FindEngine::replace_all(&mut buffer, &matches, "REPLACED");
    let replace_ms = t_replace.elapsed().as_millis();
    assert_eq!(replaced, 200, "LC-03: replace-all must rewrite exactly 200 matches (got {replaced})");
    let final_text = buffer.to_string();
    assert!(!final_text.contains("NEEDLE"), "LC-03: no 'NEEDLE' remains after replace-all");
    assert_eq!(
        final_text.matches("REPLACED").count(),
        200,
        "LC-03: exactly 200 'REPLACED' tokens after replace-all"
    );
    assert!(
        replace_budget.passes(replace_ms),
        "LC-03: 200-match replace-all {replace_ms} ms must be <= {} ms (override {})",
        replace_budget.ceiling,
        replace_budget.env_var
    );

    // Record the worse of the two phases as the scenario's measured value (the binding gate).
    let worst = search_ms.max(replace_ms);
    println!(
        "LC-03 measured={worst}ms (search {search_ms}ms + replace {replace_ms}ms, both <= {}ms) PASS — \
         200 matches found + replaced across 10k lines",
        search_budget.ceiling
    );
    record("LC-03", worst as f64, "PASS");
}

// ── LC-04: multi-cursor insert at 1000 positions simultaneously <= 500 ms ─────────────────────────

#[test]
fn perf_lc04_multi_cursor() {
    if skip_all() {
        return;
    }
    let budget = Budget::resolve("LC-04", "PERF_BUDGET_LC04_MS", 500);

    // FIXTURE (NOT timed): a 1000-line buffer; place one caret at the start of each line (1000 cursors).
    let mut buffer = TextBuffer::new(&"original\n".repeat(1000));
    let line_starts: Vec<usize> = (0..1000)
        .map(|n| buffer.line_to_byte(n).expect("LC-04: line start byte offset"))
        .collect();
    let mut cursors = CursorSet::new();
    cursors.set_cursors(line_starts.iter().map(|b| Cursor::caret(*b)).collect(), &buffer);
    assert_eq!(cursors.len(), 1000, "LC-04: 1000 simultaneous cursors set");

    // MEASURED: insert "X-" at all 1000 cursor positions at once via the REAL CursorSet::insert_at_all
    // (applies high->low so earlier edits never shift later offsets).
    let t0 = Instant::now();
    let applied = cursors.insert_at_all("X-", &mut buffer);
    let elapsed_ms = t0.elapsed().as_millis();

    assert_eq!(applied, 1000, "LC-04: insert applied at all 1000 cursor positions (got {applied})");
    let final_text = buffer.to_string();
    assert_eq!(
        final_text.matches("X-original").count(),
        1000,
        "LC-04: all 1000 lines must carry the inserted prefix (got {})",
        final_text.matches("X-original").count()
    );
    assert!(
        budget.passes(elapsed_ms),
        "LC-04: 1000-cursor insert {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LC-04 measured={elapsed_ms}ms (<= {}ms) PASS — multi-cursor insert at 1000 positions",
        budget.ceiling
    );
    record("LC-04", elapsed_ms as f64, "PASS");
}

// ── LC-05: memory budget for a 10k-line file — RSS delta <= 50 MB (median of 3) ───────────────────

#[test]
fn perf_lc05_memory() {
    if skip_all() {
        return;
    }
    let budget = Budget::resolve("LC-05", "PERF_BUDGET_LC05_MB", 50);

    // MEASURED (memory, median of 3 — RISK-5 / CTRL-5): each run loads the 10k-line buffer AND runs a
    // highlight pass, holding both alive across the "after" RSS reading so the allocation is counted.
    // The median of 3 deltas (MB) absorbs allocator page-reservation noise. The synthetic source is
    // rebuilt per run inside the closure but its construction RSS is part of the workload by design
    // (the contract budgets the load+highlight memory; we keep it consistent across runs).
    let registry = LanguageRegistry::with_bundled_languages();
    let median_mb = measure_rss_delta_median(|| {
        let src = synth_10k_rust();
        let buffer = TextBuffer::new(&src);
        let mut hl = registry
            .highlighter_for_extension("rs")
            .expect("LC-05: rust highlighter");
        let spans = hl.highlight(src.as_bytes());
        // Return the heavy allocations so they stay alive until AFTER the post-reading.
        (buffer, spans, src)
    });

    assert!(
        median_mb <= budget.ceiling as f64,
        "LC-05: 10k-line RSS delta median {median_mb:.2} MB must be <= {} MB (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LC-05 measured={median_mb:.2}mb (<= {}mb) PASS — 10k-line load+highlight RSS delta (median of \
         3 via sysinfo)",
        budget.ceiling
    );
    record("LC-05", median_mb, "PASS");
}

// ── LC-06: large-codebase index, 500 files — REQUIRES_PG (code-nav is a backend client) ───────────

#[test]
#[ignore = "requires_pg: the code-nav workspace indexer is a handshake_core route (CodeNavClient); set HSK_TEST_BASE + HSK_TEST_WORKSPACE_ID and run with --ignored against live managed PostgreSQL"]
fn perf_lc06_codebase_index() {
    // HONEST GATE: in this crate the code-nav surface is a CLIENT to handshake_core
    // (`code_editor::code_nav::CodeNavClient`); the actual symbol indexer lives in the backend behind
    // PostgreSQL. There is NO in-process workspace indexer to time frontend-only. So this scenario binds
    // the live backend: it writes 500 synthetic ~200-line Rust files to a temp dir, drives the backend
    // code-nav index route, and asserts symbol_count >= 500 with the index completing <= 10 s. With no
    // backend env it panics with a descriptive requires_pg message (the no-silent-no-op rule), never a
    // mock. NOT a permanent ignore — it PASSES with --ignored once a seeded backend + PostgreSQL is up.
    let be = pg_proof_support::require_live_backend();
    let budget = Budget::resolve("LC-06", "PERF_BUDGET_LC06_MS", 10_000);

    // FIXTURE (NOT timed): write 500 synthetic ~200-line Rust files to a UUID-named temp subdir, removed
    // in a Drop guard so the run is idempotent (impl note 2).
    let dir = std::env::temp_dir().join(format!("mt045-lc06-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("LC-06: create temp codebase dir");
    let _cleanup = TempDirGuard(dir.clone());
    for f in 0..500usize {
        let mut body = String::with_capacity(200 * 24);
        for i in 0..200usize {
            body.push_str(&format!("fn file{f}_sym{i}() -> u32 {{ {i} }}\n"));
        }
        std::fs::write(dir.join(format!("file_{f}.rs")), body).expect("LC-06: write synthetic file");
    }

    // MEASURED: drive the backend code-nav index route for this workspace and read the symbol count.
    // Block creation/file write above is NOT in the budget (RISK-2 / CTRL-2 — Instant after setup).
    let t0 = Instant::now();
    let resp = be.post_json(
        &format!("/workspaces/{}/code-nav/index", be.workspace_id),
        &serde_json::json!({ "root_path": dir.to_string_lossy() }),
    );
    let elapsed_ms = t0.elapsed().as_millis();

    let symbol_count = resp
        .get("symbol_count")
        .and_then(|v| v.as_u64())
        .or_else(|| resp.get("symbols").and_then(|v| v.as_array()).map(|a| a.len() as u64))
        .unwrap_or(0);
    assert!(
        symbol_count >= 500,
        "LC-06: the codebase index must yield >= 500 symbols (got {symbol_count})"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LC-06: 500-file index {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LC-06 measured={elapsed_ms}ms (<= {}ms) PASS — 500-file codebase index, symbol_count={symbol_count} (live PG)",
        budget.ceiling
    );
    record("LC-06", elapsed_ms as f64, "PASS");
}

/// Removes the LC-06 temp codebase dir on drop so the run is idempotent.
struct TempDirGuard(std::path::PathBuf);
impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ── LC-07: minimap at 10k lines — glyph/row layout <= 50 ms, covers all 10000 lines ───────────────

#[test]
fn perf_lc07_minimap() {
    if skip_all() {
        return;
    }
    let budget = Budget::resolve("LC-07", "PERF_BUDGET_LC07_MS", 50);

    // FIXTURE (NOT timed): a 10k-line buffer + its highlight spans (the minimap colors each row by the
    // dominant scope on that buffer line). The minimap row layout is the native glyph-layout equivalent
    // (Minimap::compute_row_colors builds one color per minimap row, O(spans) over the whole file).
    let src = "let x = 1; // line\n".repeat(FLAT_FN_LINES);
    let buffer = TextBuffer::new(&src);
    assert!(buffer.len_lines() >= FLAT_FN_LINES, "LC-07: 10k-line buffer loaded");
    let registry = LanguageRegistry::with_bundled_languages();
    let mut hl = registry.highlighter_for_extension("rs").expect("LC-07: rust highlighter");
    let spans = hl.highlight(src.as_bytes());

    // The minimap scales the whole file into a panel-height column. ratio = lines per minimap row.
    let panel_height_px = 800.0f32;
    let ratio = Minimap::compression_ratio(FLAT_FN_LINES, panel_height_px);
    let painted_rows = FLAT_FN_LINES.div_ceil(ratio);

    // MEASURED: compute the per-row colors for the whole 10k-line file (the glyph-layout pass).
    let t0 = Instant::now();
    let row_colors = Minimap::compute_row_colors(&buffer, &spans, painted_rows, ratio, true);
    let elapsed_ms = t0.elapsed().as_millis();

    // The minimap must COVER all 10000 buffer lines: the last buffer line maps to the last row, and the
    // number of rows tracks the line count via the ratio (no lines lost). This is the contract's "glyph
    // count == buffer line count" expressed for a scaled minimap (a 1:ratio mapping, no gaps).
    let last_row = Minimap::row_for_line(FLAT_FN_LINES - 1, ratio);
    assert_eq!(
        last_row + 1,
        painted_rows,
        "LC-07: the minimap row count ({}) must cover all {FLAT_FN_LINES} buffer lines (ratio {ratio})",
        last_row + 1
    );
    assert_eq!(
        row_colors.len(),
        painted_rows,
        "LC-07: compute_row_colors must produce one color per minimap row (got {} for {painted_rows})",
        row_colors.len()
    );
    assert!(
        budget.passes(elapsed_ms),
        "LC-07: minimap glyph layout {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LC-07 measured={elapsed_ms}ms (<= {}ms) PASS — minimap layout covers {FLAT_FN_LINES} lines in \
         {painted_rows} rows (ratio {ratio})",
        budget.ceiling
    );
    record("LC-07", elapsed_ms as f64, "PASS");
}

// ── LC-08: LSP diagnostics, 500 items — overlay/store pass <= 16 ms, none dropped ─────────────────

#[test]
fn perf_lc08_diagnostics_overlay() {
    if skip_all() {
        return;
    }
    let budget = Budget::resolve("LC-08", "PERF_BUDGET_LC08_MS", 16);

    // FIXTURE (NOT timed): a panel + 500 diagnostic markers spread across 500 lines. The contract asks
    // that all 500 are RECORDED in the diagnostic map (none dropped), not that all 500 are visually
    // painted — so we measure the store pass and read them back.
    let mut doc = String::with_capacity(500 * 16);
    for i in 0..500usize {
        doc.push_str(&format!("let l{i} = {i};\n"));
    }
    let panel = CodeEditorPanel::new(&doc, "rs");
    let markers: Vec<GutterMarker> = (0..500usize)
        .map(|i| GutterMarker::diagnostic(i, DiagnosticSeverity::Warning, format!("diag {i}")))
        .collect();

    // MEASURED: push all 500 diagnostics into the store (the overlay-data pass).
    let t0 = Instant::now();
    panel.push_diagnostics(markers);
    let stored = panel.diagnostic_markers();
    let elapsed_ms = t0.elapsed().as_millis();

    assert_eq!(
        stored.len(),
        500,
        "LC-08: all 500 diagnostics must be recorded in the map (none dropped) — got {}",
        stored.len()
    );
    assert!(
        budget.passes(elapsed_ms),
        "LC-08: 500-diagnostic overlay pass {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    println!(
        "LC-08 measured={elapsed_ms}ms (<= {}ms) PASS — 500 diagnostics recorded (none dropped)",
        budget.ceiling
    );
    record("LC-08", elapsed_ms as f64, "PASS");
}
