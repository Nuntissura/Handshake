//! WP-KERNEL-012 MT-073 (E12) — built-in User Manual editors content + agent-tool reference proofs.
//!
//! Proves, with REAL runtime evidence (no tautologies):
//! - AC-001 / PT-001: the manual pane loads the editors section and ALL eight GLOBAL-BUILD-MANUAL
//!   headings are present as individual topics.
//! - AC-002 / PT-002: the agent-tool reference lists every editor/knowledge/FEMS/interop action with a
//!   NON-EMPTY author_id + a NON-EMPTY MCP tool that is one of the four REAL mcp/tools.rs methods.
//! - AC-003 / PT-003: the WP-011-style manual SEARCH box (driven via egui_kittest) finds an editor topic
//!   by keyword — a live interaction, not an in-memory assertion.
//! - AC-004 / PT-004: NO documented author_id is missing from the LIVE AccessKit registry — the id-audit
//!   cross-checks every agent-tool-reference author_id against the live registries (catalogs +
//!   DECLARED_IDENTITIES + the fixed interop/FEMS/Stage/Calendar/Locus constants) and fails on any orphan.
//! - AC-005 / PT-002: the four interop edges (FEMS, Stage, Calendar, Locus) are each documented with an
//!   author_id + mcp_tool.
//! - MC-006: the manual content contains NO 'SQLite' token and no direct-DB-write language.
//!
//! ARTIFACT HYGIENE (CX-212E / the SCREENSHOT/TEST-ARTIFACT rule): the HBR-VIS screenshot is written ONLY
//! to the EXTERNAL Handshake_Artifacts/handshake-test/wp-kernel-012-mt-073/ root via
//! [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if any repo-local
//! `test_output/` or `tests/screenshots/` dir exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

use handshake_native::accessibility::editor_action_registry::{rich_action_catalog, CODE_ACTION_CATALOG};
use handshake_native::accessibility::{
    DECLARED_IDENTITIES, CANVAS_CONTROL_CATALOG, COLLECTION_CONTROL_CATALOG, GRAPH_CONTROL_CATALOG,
    PALETTE_AUTHOR_IDS,
};
use handshake_native::manual_content_editors::{
    agent_tool_rows, editors_manual_section, INTEROP_EDGES, REQUIRED_HEADINGS,
};
use handshake_native::manual_pane::{
    ManualPane, ManualPaneState, ManualRegistry, ManualSurface, MANUAL_SEARCH_AUTHOR_ID,
};
use handshake_native::theme::HsPalette;

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the artifact-hygiene guard the
/// SCREENSHOT/TEST-ARTIFACT rule mandates). Checks BOTH `test_output/` and `tests/screenshots/`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The FOUR real MCP tool names from mcp/tools.rs (the only legal `mcp_tool` values).
const REAL_MCP_TOOLS: &[&str] = &["list_widgets", "click_widget", "set_value", "screenshot"];

/// Build the LIVE author_id set — the union of every real registered/static author_id across the surfaces
/// the manual documents. This is the id-audit's source of truth; a documented author_id absent from this
/// set is an ORPHAN (AC-004).
///
/// Sources (all LIVE registry-owning resources, none hand-typed): every entry is read from a real
/// registry/catalog/const so a documented id that drifts from the live id is caught as an orphan — the
/// set is NEVER seeded with a literal copy of a documented id (that would make the audit tautological).
/// - shell chrome: [`DECLARED_IDENTITIES`] + [`PALETTE_AUTHOR_IDS`] (the dot-form command-palette
///   container ids the live shell emits; quick-switcher / settings container ids live in
///   DECLARED_IDENTITIES);
/// - code editor: `editor.code.<action>` for every [`CODE_ACTION_CATALOG`] entry;
/// - rich editor: `editor.rich.<action>` for every `rich_action_catalog()` entry;
/// - graph/canvas/collection: the three control catalogs;
/// - FEMS / Stage / Calendar / Locus / manual: the fixed `&'static str` constants from their modules.
fn live_author_id_set() -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();

    // Shell chrome declared identities (this is where the command-palette + quick-switcher + settings
    // container ids actually live — the DOT-form ids the live shell emits).
    for ident in DECLARED_IDENTITIES {
        set.insert(ident.author_id.to_owned());
    }
    // The command-palette dialog/search/list container ids, sourced from the REAL registry const
    // (PALETTE_AUTHOR_IDS = command-palette.dialog/.search/.list) — NOT hand-typed literals. These are
    // already covered by DECLARED_IDENTITIES above; pulling them from the same const the registry exports
    // keeps the audit reading the live resource instead of an implementer-authored mirror, so any
    // documented palette id that drifts from the live id is correctly flagged as an orphan (AC-004/MC-001).
    for id in PALETTE_AUTHOR_IDS {
        set.insert((*id).to_owned());
    }

    // Code editor canonical action ids.
    for entry in CODE_ACTION_CATALOG {
        set.insert(format!("editor.code.{}", entry.action_id));
    }
    // Rich editor canonical action ids.
    for entry in rich_action_catalog() {
        set.insert(format!("editor.rich.{}", entry.action_id));
    }
    // Graph / canvas / collection control catalogs.
    for entry in GRAPH_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    for entry in CANVAS_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    for entry in COLLECTION_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }

    // FEMS fixed ids.
    set.insert(handshake_native::fems::RELEVANT_MEMORY_PANEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::RELEVANT_MEMORY_LIST_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_CONFIRM_AUTHOR_ID.to_owned());

    // Stage fixed ids.
    set.insert(handshake_native::stage_pane::STAGE_PANE_AUTHOR_ID.to_owned());
    set.insert(handshake_native::stage_pane::STAGE_ROUTED_CONTENT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::stage_pane::STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID.to_owned());

    // Calendar (daily-journal) fixed ids.
    set.insert(handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_PANEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID.to_owned());
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID.to_owned(),
    );

    // Locus (outgoing-links) fixed ids.
    set.insert(handshake_native::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID.to_owned());
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::RESOLVED_SECTION_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::UNRESOLVED_SECTION_AUTHOR_ID
            .to_owned(),
    );

    // Manual pane's own search box id (documented as a Knowledge surface row).
    set.insert(MANUAL_SEARCH_AUTHOR_ID.to_owned());

    set
}

// ── AC-001 / PT-001: all eight GLOBAL-BUILD-MANUAL headings present as topics ─────────────────────────
#[test]
fn manual_loads_section_with_all_eight_required_headings() {
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    assert_eq!(reg.len(), 1, "the editors section registered into the pane");

    let section = reg.section("native-editors").expect("editors section is registered");
    for heading in REQUIRED_HEADINGS {
        assert!(
            section.topic(heading).is_some(),
            "AC-001: GLOBAL-BUILD-MANUAL heading '{heading}' must be present as an individual topic"
        );
        // Each topic body must be a real no-context body (not an empty stub).
        let body = &section.topic(heading).unwrap().body;
        assert!(
            body.len() > 60,
            "AC-001: heading '{heading}' must carry a substantive no-context body (got {} chars)",
            body.len()
        );
    }
    assert_eq!(REQUIRED_HEADINGS.len(), 8, "exactly the eight GLOBAL-BUILD-MANUAL headings");
}

// ── AC-002 / PT-002: every agent-tool row has a non-empty author_id + a REAL mcp_tool ─────────────────
#[test]
fn agent_tool_reference_rows_are_complete_and_use_real_tools() {
    let rows = agent_tool_rows();
    assert!(rows.len() >= 30, "the reference covers every editor/knowledge/FEMS/interop action (got {})", rows.len());
    for row in &rows {
        assert!(!row.author_id.is_empty(), "AC-002: a row has an empty author_id");
        assert!(!row.mcp_tool.is_empty(), "AC-002: row '{}' has an empty mcp_tool", row.author_id);
        assert!(
            REAL_MCP_TOOLS.contains(&row.mcp_tool),
            "AC-002/RISK-002: row '{}' uses non-real MCP tool '{}' (must be one of {:?})",
            row.author_id,
            row.mcp_tool,
            REAL_MCP_TOOLS
        );
    }
    // The reference must cover EACH editor + knowledge + FEMS + interop surface (no surface omitted).
    let surfaces: HashSet<ManualSurface> = rows.iter().map(|r| r.surface).collect();
    for required in [
        ManualSurface::Code,
        ManualSurface::RichText,
        ManualSurface::Graph,
        ManualSurface::Canvas,
        ManualSurface::Knowledge,
        ManualSurface::Fems,
        ManualSurface::Interop,
    ] {
        assert!(surfaces.contains(&required), "AC-002: surface {required:?} has no agent-tool rows");
    }
}

// ── AC-004 / PT-004: id-audit — no documented author_id is an orphan ──────────────────────────────────
#[test]
fn id_audit_no_documented_author_id_missing_from_live_registry() {
    let live = live_author_id_set();
    // Sanity: the live set is non-trivial (guards against a false-green empty-registry pass).
    assert!(live.len() > 40, "live author_id set is suspiciously small ({})", live.len());

    let rows = agent_tool_rows();
    let mut orphans: Vec<&str> = Vec::new();
    for row in &rows {
        if !live.contains(row.author_id) {
            orphans.push(row.author_id);
        }
    }
    assert!(
        orphans.is_empty(),
        "AC-004/MC-001: documented author_id(s) absent from the live AccessKit registry (ORPHANS): {orphans:?}"
    );
}

// ── AC-005 / PT-002: the four interop edges are each documented with an author_id + mcp_tool ───────────
#[test]
fn interop_edges_all_documented_with_author_id_and_tool() {
    let section = editors_manual_section();
    let interop_topic = section
        .topic("Interop Edges")
        .expect("the interop topic exists");
    // Each of FEMS / Stage / Calendar / Locus is named in the interop topic body (AC-005).
    for edge in INTEROP_EDGES {
        assert!(
            interop_topic.body.contains(edge),
            "AC-005/MC-007: interop edge '{edge}' is not named in the interop topic"
        );
    }
    assert_eq!(INTEROP_EDGES.len(), 4, "exactly FEMS/Stage/Calendar/Locus");

    // Each edge has at least one agent-tool row carrying a non-empty author_id + mcp_tool.
    let rows = agent_tool_rows();
    let interop_rows: Vec<_> = rows.iter().filter(|r| r.surface == ManualSurface::Interop).collect();
    assert!(
        interop_rows.len() >= 4,
        "AC-005: at least one interop row per edge (got {})",
        interop_rows.len()
    );
    // FEMS rows are the dedicated Fems surface (the FEMS edge); assert it too.
    let fems_rows: Vec<_> = rows.iter().filter(|r| r.surface == ManualSurface::Fems).collect();
    assert!(!fems_rows.is_empty(), "AC-005: the FEMS edge has agent-tool rows");

    // Concretely assert each edge's signature author_id appears among the rows (Stage/Calendar/Locus on
    // the Interop surface; FEMS on the Fems surface).
    let row_ids: HashSet<&str> = rows.iter().map(|r| r.author_id).collect();
    assert!(row_ids.contains("stage-pane"), "Stage edge author_id present");
    assert!(row_ids.contains("daily-journal-panel"), "Calendar edge author_id present");
    assert!(row_ids.contains("outgoing.panel"), "Locus edge author_id present");
    assert!(row_ids.contains("relevant-memory-panel"), "FEMS edge author_id present");
}

// ── MC-006: the manual content names NO SQLite and no direct-DB-write language ────────────────────────
#[test]
fn manual_content_has_no_sqlite_and_no_direct_db_writes() {
    let section = editors_manual_section();
    let all_text: String = section
        .topics
        .iter()
        .map(|t| format!("{}\n{}", t.heading, t.body))
        .collect::<Vec<_>>()
        .join("\n");
    let lower = all_text.to_lowercase();
    assert!(!lower.contains("sqlite"), "MC-006: the manual must not mention SQLite");
    // Persistence must be described as PostgreSQL/EventLedger via handshake_core.
    assert!(
        lower.contains("postgresql") || lower.contains("eventledger"),
        "MC-006: persistence must be described as PostgreSQL/EventLedger"
    );
    assert!(lower.contains("handshake_core"), "MC-006: persistence routes through handshake_core");
    // No "direct DB write" affirmation (the manual states persistence is NOT direct).
    assert!(
        !lower.contains("write directly to the database") && !lower.contains("direct db write"),
        "MC-006: the manual must not describe direct DB writes as a path"
    );
}

// ── AC-003 / PT-003: the manual SEARCH box (live egui_kittest) finds an editor topic by keyword ───────
#[test]
fn manual_search_box_finds_editor_topic_by_keyword() {
    // The standalone manual-pane widget driven headlessly via egui_kittest (AccessKit enabled). Typing a
    // keyword into the search box filters the nav list to the matching topic — a LIVE interaction.
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let palette = HsPalette::dark();
    let mut state = ManualPaneState::default();

    // Drive the pane in a kittest harness. State (reg/palette) lives outside the closure via 'static
    // leaks so the harness app closure can borrow them for 'static (the test owns process lifetime).
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(palette));

    let mut harness = Harness::builder().build_ui(move |ui| {
        ManualPane::new(reg, &mut state, palette).show(ui);
    });
    harness.run();

    // The search box carries the stable accessible label "Search Manual" (and the author_id
    // 'manual-search'). Type a keyword that lives in the "Core Workflows" topic body ("command palette").
    let search = harness.get_by_label("Search Manual");
    search.focus();
    harness.run();
    harness.get_by_label("Search Manual").type_text("command palette");
    harness.run();
    harness.run();

    // After filtering, the matching topic surfaces in the live tree (the nav list + body show only
    // matching topics). "Core Workflows" mentions the command palette. The heading appears as BOTH a nav
    // Button AND a body Label, so count matches with query_all (query_by_label panics on >1).
    let match_count = harness.query_all_by_label("Core Workflows").count();
    assert!(
        match_count > 0,
        "AC-003: typing 'command palette' into manual-search surfaces the matching editor topic"
    );

    // A non-matching keyword filters it OUT (proves the search actually filters, not always-passes).
    // Appending more text makes the query no longer a substring of the topic, so the row disappears.
    harness.get_by_label("Search Manual").type_text(" zzznotarealtopiczzz");
    harness.run();
    harness.run();
    let after_count = harness.query_all_by_label("Core Workflows").count();
    assert_eq!(
        after_count, 0,
        "AC-003: a non-matching keyword removes the topic (the search really filters)"
    );
}

// ── HBR-VIS: render the manual pane + save a screenshot to the EXTERNAL artifact root ─────────────────
#[test]
fn manual_pane_renders_and_screenshots() {
    let _guard = wgpu_guard();
    assert_no_local_artifact_dir();

    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(HsPalette::dark()));
    // Pre-select the agent-tool reference so the screenshot shows the steering table.
    let mut state = ManualPaneState {
        selected: Some(("native-editors".to_owned(), "Agent Tool Reference".to_owned())),
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .wgpu()
        .build_ui(move |ui| {
            ManualPane::new(reg, &mut state, palette).show(ui);
        });
    harness.run();
    harness.run();

    // The container + search box render without panic/overlap.
    assert!(
        harness.query_by_label("Search Manual").is_some(),
        "HBR-VIS: the manual search box renders"
    );

    let out_dir = external_artifact_dir("wp-kernel-012-mt-073");
    let _ = std::fs::create_dir_all(&out_dir);
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let out_path = out_dir.join("manual_pane_editors.png");
            let saved = image.save(&out_path).is_ok();
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!("PT-005 manual-pane screenshot: {w}x{h}, saved={saved} ({})", abs.display());
            assert!(saved, "HBR-VIS: the manual pane screenshot PNG saved to the external root");
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-073 manual-pane screenshot render unavailable (no wgpu \
                 adapter): {e}. The content + search + id-audit proofs stand; the PNG is a GPU-host item."
            );
        }
    }

    assert_no_local_artifact_dir();
}
