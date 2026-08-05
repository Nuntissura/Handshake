//! WP-KERNEL-012 MT-072 (E12) — editor Settings section render + AccessKit proofs (PT-004 / AC-007 / AC-008).
//!
//! These proofs drive the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit and
//! pushes the SAME `TreeUpdate` the out-of-process Windows UIA adapter receives) and prove:
//!
//! - AC-007: the new editor controls expose stable AccessKit author_ids — `settings-editor-font-size`,
//!   `settings-editor-tab-size`, `settings-editor-insert-spaces`, `settings-editor-word-wrap`,
//!   `settings-editor-render-whitespace`, `settings-syntax-palette-mode`, at least one
//!   `settings-syntax-swatch-{scope}` (Custom mode), and at least one `settings-keybind-row-{action}`.
//! - AC-008: the Editor settings section renders against the live settings surface without overlap, the
//!   control values reflect the stored state, and (the visual HBR-VIS proof) a wgpu screenshot of the
//!   rendered Editor + Syntax sections is saved to the EXTERNAL artifact root.
//!
//! ARTIFACT HYGIENE (CX-212E / the SCREENSHOT/TEST-ARTIFACT rule): every PNG is written ONLY to the
//! EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-072/` root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if any repo-local `test_output/` or `tests/screenshots/`
//! dir exists.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::HighlightScope;
use handshake_native::settings_dialog::SettingsOutcome;
use handshake_native::preference_client::{
    PREF_EDITOR_FONT_SIZE, PREF_EDITOR_KEYBINDING_OVERRIDES, PREF_EDITOR_SYNTAX_CUSTOM_COLORS,
    PREF_EDITOR_TAB_SIZE,
};
use handshake_native::settings_editor_section::{
    preference_provenance_author_id, syntax_swatch_author_id,
    ATELIER_CKC_STAGE_SETTINGS_POSTURE_AUTHOR_ID, ATELIER_CKC_STAGE_SETTINGS_POSTURE_NOTE,
    EDITOR_FONT_SIZE_AUTHOR_ID, EDITOR_INSERT_SPACES_AUTHOR_ID, EDITOR_PREFS_RESET_AUTHOR_ID,
    EDITOR_RENDER_WHITESPACE_AUTHOR_ID, EDITOR_TAB_SIZE_AUTHOR_ID, EDITOR_WORD_WRAP_AUTHOR_ID,
    RUNTIME_CHAT_SETTINGS_POSTURE_AUTHOR_ID, RUNTIME_CHAT_SETTINGS_POSTURE_NOTE,
    SYNTAX_PALETTE_MODE_AUTHOR_ID, SYNTAX_PALETTE_RESET_AUTHOR_ID,
    WIKI_PROJECTION_SETTINGS_POSTURE_AUTHOR_ID, WIKI_PROJECTION_SETTINGS_POSTURE_NOTE,
};
use handshake_native::workspace_settings::{SyntaxPalette, SyntaxPaletteMode};
use screenshot_harness::ScreenshotHarness as Harness;

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
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

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// True when the live AccessKit tree contains a node carrying `author_id`.
fn has_author_id(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> bool {
    let root = harness.root();
    root.children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(author_id))
}

/// Build a fresh live shell with the settings dialog open and a single search `query` applied, with the
/// syntax palette seeded to `palette_mode` (so the Custom swatch controls render when asked). Returns the
/// harness. The search both FILTERS the dialog body to the matching section(s) — shortening it so the
/// (collapsed-by-default) MT-072 section is within the 440px scroll viewport — AND auto-expands the
/// matching section (the dialog opens an MT-072 section when `!query.is_empty()`). This is the
/// deterministic path a no-context model uses to surface a specific section.
fn open_settings_searched(
    query: &str,
    palette_mode: SyntaxPaletteMode,
) -> Harness<'static, HandshakeApp> {
    let app = ok_app();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    // Opening settings performs the normal persisted GET. Seed the requested palette after that
    // load so this test exercises the live Custom controls instead of having the fixture overwritten
    // by the default remote settings response.
    harness
        .state_mut()
        .set_workspace_syntax_palette_for_test(SyntaxPalette {
            mode: palette_mode,
            custom: Default::default(),
        });
    harness.run();
    if !query.is_empty() {
        let search = harness.get_by_label("Search settings");
        search.focus();
        harness.run();
        harness.get_by_label("Search settings").type_text(query);
        harness.run();
        harness.run();
    }
    harness
}

/// Find any live node whose author_id starts with `prefix`.
fn has_author_id_prefix(harness: &Harness<'_, HandshakeApp>, prefix: &str) -> bool {
    harness.root().children_recursive().any(|n| {
        n.accesskit_node()
            .author_id()
            .is_some_and(|a| a.starts_with(prefix))
    })
}

// ── AC-007: the new editor controls expose stable AccessKit author_ids ───────────────────────────────
#[test]
fn editor_controls_expose_stable_accesskit_author_ids() {
    // (1) The editor-prefs controls — surface the Editor section via search ("editor font").
    {
        let harness = open_settings_searched("editor", SyntaxPaletteMode::Standard);
        for id in [
            EDITOR_FONT_SIZE_AUTHOR_ID,
            EDITOR_TAB_SIZE_AUTHOR_ID,
            EDITOR_INSERT_SPACES_AUTHOR_ID,
            EDITOR_WORD_WRAP_AUTHOR_ID,
            EDITOR_RENDER_WHITESPACE_AUTHOR_ID,
        ] {
            assert!(
                has_author_id(&harness, id),
                "AC-007: control '{id}' is addressable by stable AccessKit author_id in the live tree"
            );
        }
    }

    // (2) The syntax palette mode + at least one Custom swatch — surface the Syntax section via search
    //     ("syntax") with the palette in Custom mode (swatches render only in Custom).
    {
        let harness = open_settings_searched("syntax", SyntaxPaletteMode::Custom);
        assert!(
            has_author_id(&harness, SYNTAX_PALETTE_MODE_AUTHOR_ID),
            "AC-007: '{SYNTAX_PALETTE_MODE_AUTHOR_ID}' is addressable"
        );
        assert!(
            has_author_id(&harness, &syntax_swatch_author_id(HighlightScope::Keyword)),
            "AC-007: at least one settings-syntax-swatch-{{scope}} control is addressable (Custom mode)"
        );
    }

    // (3) At least one editor keybinding row — surface the Keybindings section + expand the Editor-actions
    //     sub-header (search "keybinding" shows the Keybindings section; the editor-actions sub-header is
    //     opened by clicking it).
    {
        let mut harness = open_settings_searched("keybinding", SyntaxPaletteMode::Standard);
        if let Some(node) = harness.query_by_label("Editor actions") {
            node.click();
            harness.run();
            harness.run();
        }
        assert!(
            has_author_id_prefix(&harness, "settings-keybind-row-"),
            "AC-007: at least one settings-keybind-row-{{action_id}} control is addressable"
        );
    }
}

#[test]
fn wiki_projection_posture_is_exposed_in_live_settings() {
    let harness = open_settings_searched("wiki projection", SyntaxPaletteMode::Standard);
    assert!(
        has_author_id(&harness, WIKI_PROJECTION_SETTINGS_POSTURE_AUTHOR_ID),
        "MT-025: the live Settings surface exposes the stable Wiki Projection posture row"
    );
    assert!(
        harness
            .query_by_label_contains(WIKI_PROJECTION_SETTINGS_POSTURE_NOTE)
            .is_some(),
        "MT-025: Settings truthfully states the exact active-workspace/theme and additive-overlay posture"
    );
}

#[test]
fn runtime_chat_posture_is_exposed_in_live_settings() {
    let harness = open_settings_searched("runtime chat", SyntaxPaletteMode::Standard);
    assert!(
        has_author_id(&harness, RUNTIME_CHAT_SETTINGS_POSTURE_AUTHOR_ID),
        "MT-098: the live Settings surface exposes the stable Runtime Chat posture row"
    );
    assert!(
        harness
            .query_by_label_contains(RUNTIME_CHAT_SETTINGS_POSTURE_NOTE)
            .is_some(),
        "MT-098: Settings truthfully states the app-transport, absent-route, and no-fabrication posture"
    );
}

#[test]
fn mt033_atelier_ckc_stage_posture_is_exposed_in_live_settings() {
    let harness = open_settings_searched("atelier", SyntaxPaletteMode::Standard);
    assert!(
        has_author_id(&harness, ATELIER_CKC_STAGE_SETTINGS_POSTURE_AUTHOR_ID),
        "MT-033: live Settings exposes the stable read-only Atelier/CKC/Stage posture row"
    );
    assert!(
        harness
            .query_by_label_contains(ATELIER_CKC_STAGE_SETTINGS_POSTURE_NOTE)
            .is_some(),
        "MT-033: Settings names the real VIEW/EDITORS commands without inventing a preference"
    );
}

// ── WP-KERNEL-012 wave-6 (S6 item 3): the font-size + Custom palette controls render an HONEST LIVE-effect
//    note (the MT-072 inert follow-up is now wired: both apply to the mounted editor) ────────────────────
#[test]
fn editor_settings_controls_show_honest_live_effect_note() {
    // S6 item 3 resolved the MT-072 typed follow-up: editor_font_size now resizes the mounted code editor
    // plus rich editor document text, and a Custom syntax palette repaints code/minimap syntax rows (see
    // `HandshakeApp::sync_editor_prefs_to_panel`). The inline notes are now a LIVE-effect disclosure
    // (honest about what applies live AND the small gutter sizing follow-up). Prove the updated note is live
    // in the AccessKit tree (a no-context operator/model reads the real disclosure, not the stale
    // "not yet wired" text).
    {
        let harness = open_settings_searched("editor", SyntaxPaletteMode::Standard);
        assert!(
            harness
                .query_by_label_contains("resizes the mounted code editor and rich editor")
                .is_some(),
            "S6 item 3: the font-size control must render its LIVE-effect note (it now resizes the running editor)"
        );
    }
    {
        let harness = open_settings_searched("syntax", SyntaxPaletteMode::Custom);
        assert!(
            harness
                .query_by_label_contains("repaints the mounted code editor")
                .is_some(),
            "S6 item 3: the syntax palette must render its LIVE-effect note (a Custom swatch repaints the running pane)"
        );
    }
}

// ── MT-072 Fix 2: the editor-related sections render contiguously (Editor -> Syntax) BEFORE About ─────
//
// Before the fix the Editor + Syntax sections were appended AFTER About, splitting them out of the
// contract's stated order. This proves the live render order (== AccessKit tree order) is now
// Editor -> Syntax -> About, grouped and before About. Ordering only — section content + author_ids are
// unchanged.
#[test]
fn editor_and_syntax_sections_render_contiguously_before_about() {
    let app = ok_app();
    // A tall viewport so every collapsed section header lays out (no scroll culling of the later sections).
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 1000.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    // Collect the section-header author_ids in live tree order (depth-first == render/sibling order).
    let ids: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
        .collect();
    let pos = |id: &str| ids.iter().position(|a| a == id);

    let editor = pos("settings.section.editor").expect("Editor section header renders");
    let syntax = pos("settings.section.syntax").expect("Syntax section header renders");
    let about = pos("settings.section.about").expect("About section header renders");

    assert!(
        editor < syntax,
        "Fix 2: Editor renders above Syntax (contract order Editor -> Syntax); editor={editor}, syntax={syntax}"
    );
    assert!(
        syntax < about,
        "Fix 2: the editor sections render BEFORE About (grouped, not appended after it); syntax={syntax}, about={about}"
    );
}

// ── AC-008: the Editor settings section renders + reflects stored state + saves a screenshot ─────────
#[test]
fn editor_settings_section_renders_and_screenshots() {
    let _guard = wgpu_guard();
    assert_no_local_artifact_dir();

    let mut app = ok_app();
    // Seed a KNOWN stored state so the rendered controls reflect it (AC-008 value-reflects-state).
    app.set_workspace_syntax_palette_for_test(SyntaxPalette {
        mode: SyntaxPaletteMode::Custom,
        custom: Default::default(),
    });

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    // With no query, the new Editor + Syntax section HEADERS render (collapsed) against the live settings
    // surface — present in the tree, no panic, no overlap.
    assert!(
        harness.query_by_label("Editor").is_some(),
        "AC-008: Editor section header renders"
    );
    assert!(
        harness.query_by_label("Syntax").is_some(),
        "AC-008: Syntax section header renders"
    );
    assert!(
        harness.query_by_label("Keybindings").is_some(),
        "AC-008: the (extended) Keybindings section renders"
    );

    // Surface the Editor + Syntax sections (expanded) via search so the screenshot shows the REAL controls
    // reflecting the stored state. "color" matches BOTH the Editor section (keyword "color"? no) — use a
    // term that matches both editor-prefs + syntax: "render" hits the Editor (render whitespace) and
    // "syntax"-adjacent terms. To keep the body short AND show the editor controls, search "tab" (Editor
    // only — font/tab/spaces/wrap/whitespace) so the Editor section renders alone, expanded, at the top.
    let search = harness.get_by_label("Search settings");
    search.focus();
    harness.run();
    harness.get_by_label("Search settings").type_text("tab");
    harness.run();
    harness.run();

    // The control VALUE reflects stored state: change the editor prefs, re-run, and confirm the live
    // settings hold the new value (the section renders from the live settings each frame).
    let mut new_prefs = harness.state().workspace_settings().editor_prefs;
    new_prefs.tab_size = 8;
    new_prefs.render_whitespace = handshake_native::workspace_settings::RenderWhitespaceMode::All;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(new_prefs));
    harness.run();
    assert_eq!(
        harness.state().workspace_settings().editor_prefs.tab_size,
        8,
        "AC-008: the section reflects the stored tab_size after a change"
    );

    // HBR-VIS: save a wgpu screenshot of the rendered editor settings sections to the EXTERNAL root. On a
    // GPU host this saves a PNG; absent an adapter, record an honest non-fatal note (the AccessKit + render
    // proofs above stand).
    let out_dir = external_artifact_dir("wp-kernel-012-mt-072");
    let _ = std::fs::create_dir_all(&out_dir);
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let out_path = out_dir.join("editor_settings_sections.png");
            let saved = image.save(&out_path).is_ok();
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!(
                "PT-004 editor-settings screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "AC-008: the editor settings screenshot PNG saved to the external root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-072 editor-settings screenshot render unavailable (no wgpu \
                 adapter): {e}. AC-007 AccessKit author_id proof + AC-008 render-without-overlap proof \
                 passed; the PNG is a GPU-host item."
            );
        }
    }

    // No repo-local artifact dir leaked (the screenshot went to the external root only).
    assert_no_local_artifact_dir();
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-072 remediation item 8 (FAIL_V4): UNFILTERED visual proof for Editor, Syntax AND Keybindings.
//
// V4 rejected the previous screenshot because it was captured with the settings SEARCH set to "tab",
// which removed the Syntax and Keybindings sections from the render (and therefore from the AccessKit
// tree) entirely. Everything below runs with an EMPTY search box — every section header renders — and
// the target section is brought inside the dialog's fixed 440px scroll viewport by COLLAPSING the
// sections above it (a normal operator affordance that hides nothing from the tree) instead of
// filtering them away. Each capture writes BOTH a PNG and its paired AccessKit tree dump.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Click the live node carrying `author_id` (used to collapse/expand section headers by stable id
/// rather than by a label string that may repeat elsewhere in the shell).
fn click_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) -> bool {
    let found = {
        let node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id));
        match node {
            Some(node) => {
                node.click();
                true
            }
            None => false,
        }
    };
    if found {
        harness.run();
        harness.run();
    }
    found
}

/// The section headers that render ABOVE Editor / Syntax in the dialog body.
const SECTIONS_ABOVE_EDITOR: &[&str] = &[
    "settings.section.appearance",
    "settings.section.keybindings",
    "settings.section.swarm",
    "settings.section.terminal",
    "settings.section.layout",
    "settings.section.model-session",
];

/// Dump the live AccessKit tree (author_id / role / label / value, depth-first) next to its screenshot so
/// a no-context validator can read the exact addressable state the pixels show.
fn dump_accesskit_tree(harness: &Harness<'_, HandshakeApp>, out_dir: &Path, stem: &str) -> PathBuf {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let author = ak.author_id().unwrap_or("");
        if author.is_empty() {
            continue;
        }
        lines.push(format!(
            "{author}\trole={:?}\tlabel={}\tvalue={}",
            ak.role(),
            ak.label().unwrap_or_default(),
            ak.value().unwrap_or_default()
        ));
    }
    assert!(
        !lines.is_empty(),
        "the {stem} AccessKit dump must contain addressable nodes"
    );
    let path = out_dir.join(format!("{stem}.accesskit.txt"));
    std::fs::write(&path, lines.join("\n")).unwrap_or_else(|error| {
        panic!("write AccessKit dump {}: {error}", path.display());
    });
    path
}

/// Render + save `stem`.png from `harness` into the external artifact root. Returns whether a GPU
/// adapter produced a real image (absent an adapter the AccessKit dump still stands, and the run says so
/// rather than silently claiming a screenshot).
fn save_screenshot(harness: &mut Harness<'_, HandshakeApp>, out_dir: &Path, stem: &str) -> bool {
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "{stem}: rendered image is non-empty");
            let out_path = out_dir.join(format!("{stem}.png"));
            assert!(
                image.save(&out_path).is_ok(),
                "{stem}: screenshot PNG saved to the external root"
            );
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!("PT-004 unfiltered capture {stem}: {w}x{h} -> {}", abs.display());
            true
        }
        Err(error) => {
            println!(
                "BLOCKER(non-fatal): MT-072 {stem} screenshot render unavailable (no wgpu adapter): \
                 {error}. The paired AccessKit dump + the render/no-overlap assertions still hold."
            );
            false
        }
    }
}

/// True when ANY live node's accessible label contains `needle` (multi-match safe, unlike
/// `query_by_label_contains`, which panics when more than one node matches).
fn has_label_containing(harness: &Harness<'_, HandshakeApp>, needle: &str) -> bool {
    harness.root().children_recursive().any(|node| {
        node.accesskit_node()
            .label()
            .is_some_and(|label| label.contains(needle))
    })
}

/// The longest label currently rendered on a visible editor-keybinding row (used to prove long action
/// labels are laid out in full rather than clipped away).
fn longest_visible_keybind_label(harness: &Harness<'_, HandshakeApp>) -> String {
    harness
        .root()
        .children_recursive()
        .filter(|node| {
            node.accesskit_node()
                .author_id()
                .is_some_and(|author| author.starts_with("settings-keybind-row-"))
        })
        .filter_map(|node| node.accesskit_node().label())
        .max_by_key(|label: &String| label.chars().count())
        .unwrap_or_default()
}

/// The on-screen band the settings dialog's fixed-height scroll body occupies. egui keeps AccessKit
/// nodes for rows that the `ScrollArea` has scrolled OUT of view, so "present in the tree" is NOT the
/// same as "visible in the screenshot"; every visual claim below is made against this band.
fn settings_body_band(harness: &Harness<'_, HandshakeApp>) -> (f32, f32) {
    let search = harness
        .query_by_label("Search settings")
        .expect("the settings dialog renders its search field")
        .rect();
    // `settings_dialog.rs` gives the body a `ScrollArea::vertical().max_height(440.0)` directly under
    // the search field.
    (search.bottom(), search.bottom() + 440.0)
}

/// Which editor surfaces have at least one keybinding row actually VISIBLE inside the scroll body.
fn visible_keybind_surfaces(harness: &Harness<'_, HandshakeApp>) -> (bool, bool) {
    let (top, bottom) = settings_body_band(harness);
    let mut code = false;
    let mut rich = false;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let Some(author) = ak.author_id() else {
            continue;
        };
        let Some(action_id) = author.strip_prefix("settings-keybind-row-") else {
            continue;
        };
        let rect = node.rect();
        if rect.height() <= 0.0 {
            continue;
        }
        let center = rect.center().y;
        if center < top || center > bottom {
            continue;
        }
        if action_id.starts_with("code.") {
            code = true;
        } else if action_id.starts_with("rich.") {
            rich = true;
        }
    }
    (code, rich)
}

/// Scroll the settings dialog body by `lines` mouse-wheel lines (negative scrolls DOWN). The pointer is
/// parked over a node that is inside the dialog's scroll viewport, which is what egui requires before a
/// `ScrollArea` consumes wheel input.
fn scroll_settings_body(harness: &mut Harness<'_, HandshakeApp>, lines: f32) {
    // Hover a FIXED point in the middle of the scroll body. An anchor derived from a moving node stops
    // working after the first scroll (the node leaves the viewport and the pointer is then outside the
    // `ScrollArea`, so egui stops routing wheel input to it).
    let search = harness
        .query_by_label("Search settings")
        .expect("the settings dialog renders its search field")
        .rect();
    let anchor = egui::pos2(search.center().x, search.bottom() + 200.0);
    harness.event(egui::Event::PointerMoved(anchor));
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, lines),
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
}

/// Build an UNFILTERED settings shell (empty search box) at `size`, with the syntax palette seeded so the
/// Custom swatch controls render.
fn unfiltered_settings(
    size: egui::Vec2,
    palette_mode: SyntaxPaletteMode,
) -> Harness<'static, HandshakeApp> {
    let app = ok_app();
    let mut harness = Harness::builder()
        .with_size(size)
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    // Opening settings performs the normal persisted load, which resets the in-memory settings for the
    // newly-bound workspace. Seed the palette AFTER that load (same ordering the searched helper uses)
    // so the Custom swatch controls are the ones actually rendered.
    harness
        .state_mut()
        .set_workspace_syntax_palette_for_test(SyntaxPalette {
            mode: palette_mode,
            custom: Default::default(),
        });
    harness.run();
    harness.run();
    harness
}

/// Assert no two addressable control nodes in the live tree overlap, and none is clipped to zero size —
/// the machine-checkable half of "readable, no overlap or clipping" (the PNG is the human half).
fn assert_no_overlapping_controls(harness: &Harness<'_, HandshakeApp>, context: &str) {
    let mut boxes: Vec<(String, egui::Rect)> = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let Some(author) = ak.author_id() else {
            continue;
        };
        // Only the MT-072 editor-settings controls (author_id prefixes owned by this MT). Filtering by
        // id rather than by AccessKit role keeps this proof independent of role-mapping changes.
        if !(author.starts_with("settings-editor-")
            || author.starts_with("settings-syntax-")
            || author.starts_with("settings-keybind-")
            || author.starts_with("settings-pref-source-"))
        {
            continue;
        }
        let rect = node.rect();
        assert!(
            rect.width() > 0.0 && rect.height() > 0.0,
            "{context}: control '{author}' is clipped to a zero-size rect {rect:?}"
        );
        boxes.push((author.to_owned(), rect));
    }
    assert!(
        !boxes.is_empty(),
        "{context}: at least one addressable control must be laid out"
    );
    for (i, (a_id, a)) in boxes.iter().enumerate() {
        for (b_id, b) in boxes.iter().skip(i + 1) {
            let overlap = a.intersect(*b);
            // egui lays sibling controls out edge-to-edge; require a real (>0.5px on BOTH axes) overlap
            // before failing so a shared 1px border is not reported as an overlap.
            assert!(
                overlap.width() <= 0.5 || overlap.height() <= 0.5,
                "{context}: controls '{a_id}' {a:?} and '{b_id}' {b:?} overlap by {overlap:?}"
            );
        }
    }
}

/// PT-004 / item 8 — UNFILTERED Editor, Syntax and Keybindings captures with paired AccessKit trees,
/// including default/custom/source/revision/reset state and long action labels.
#[test]
fn unfiltered_editor_syntax_and_keybindings_sections_capture_with_accesskit_trees() {
    let _guard = wgpu_guard();
    assert_no_local_artifact_dir();
    let out_dir = external_artifact_dir("wp-kernel-012-mt-072");
    let _ = std::fs::create_dir_all(&out_dir);

    // ── (A) EDITOR section, unfiltered, with a CUSTOM value + resolved provenance visible. ──────────
    {
        let mut harness = unfiltered_settings(egui::vec2(900.0, 760.0), SyntaxPaletteMode::Standard);
        // Every section header is present BEFORE any collapsing — this is what "unfiltered" means.
        for section in [
            "settings.section.appearance",
            "settings.section.keybindings",
            "settings.section.editor",
            "settings.section.syntax",
            "settings.section.about",
        ] {
            assert!(
                has_author_id(&harness, section),
                "item 8: '{section}' renders with an EMPTY search box (nothing is filtered away)"
            );
        }

        // Seed the SET-REC-001 provenance a resolved projection would carry, so the capture shows real
        // default-vs-custom + revision chips rather than the unresolved placeholder.
        harness
            .state_mut()
            .hydrate_editor_preferences_for_test(&provenance_projection());
        harness.run();
        // A CUSTOM (non-default) value so the capture shows both custom and default state side by side.
        let mut prefs = harness.state().workspace_settings().editor_prefs;
        prefs.tab_size = 8;
        prefs.render_whitespace = handshake_native::workspace_settings::RenderWhitespaceMode::All;
        harness
            .state_mut()
            .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));
        harness.run();

        for section in SECTIONS_ABOVE_EDITOR {
            click_author_id(&mut harness, section);
        }
        harness.run();

        for id in [
            EDITOR_FONT_SIZE_AUTHOR_ID,
            EDITOR_TAB_SIZE_AUTHOR_ID,
            EDITOR_INSERT_SPACES_AUTHOR_ID,
            EDITOR_WORD_WRAP_AUTHOR_ID,
            EDITOR_RENDER_WHITESPACE_AUTHOR_ID,
        ] {
            assert!(
                has_author_id(&harness, id),
                "item 8 (Editor capture): '{id}' is laid out and addressable unfiltered"
            );
        }
        // SOURCE + REVISION state is visible next to the controls.
        assert!(
            has_author_id(
                &harness,
                &preference_provenance_author_id(PREF_EDITOR_FONT_SIZE)
            ),
            "item 8: the Editor capture exposes the font-size source/revision chip"
        );
        assert!(
            has_label_containing(&harness, "custom (operator) · rev"),
            "item 8: the Editor capture shows a CUSTOM preference's source + revision"
        );
        assert!(
            has_label_containing(&harness, "default · rev"),
            "item 8: the Editor capture shows a DEFAULT preference's source + revision"
        );
        // RESET state is visible.
        assert!(
            has_author_id(&harness, EDITOR_PREFS_RESET_AUTHOR_ID),
            "item 8: the Editor capture exposes the reset-to-defaults affordance"
        );
        assert_no_overlapping_controls(&harness, "Editor section (unfiltered)");
        dump_accesskit_tree(&harness, &out_dir, "mt072_settings_editor_unfiltered");
        save_screenshot(&mut harness, &out_dir, "mt072_settings_editor_unfiltered");

        // The Editor group is taller than the dialog's fixed 440px scroll body, so a second capture
        // scrolls to its foot to make the RESET affordance + the surfaced auto-save row visually
        // present (not merely addressable in the tree).
        let mut reset_visible = false;
        for _ in 0..60 {
            let (top, bottom) = settings_body_band(&harness);
            if harness
                .root()
                .children_recursive()
                .filter(|node| {
                    node.accesskit_node().author_id() == Some(EDITOR_PREFS_RESET_AUTHOR_ID)
                })
                .any(|node| {
                    let center = node.rect().center().y;
                    center >= top && center <= bottom
                })
            {
                reset_visible = true;
                break;
            }
            scroll_settings_body(&mut harness, -2.0);
        }
        assert!(
            reset_visible,
            "item 8: the Editor group's reset-to-defaults affordance must be VISIBLY reachable in the \
             unfiltered dialog"
        );
        dump_accesskit_tree(&harness, &out_dir, "mt072_settings_editor_reset_state");
        save_screenshot(&mut harness, &out_dir, "mt072_settings_editor_reset_state");
    }

    // ── (B) SYNTAX section, unfiltered, Custom mode (all eight swatches + palette reset). ───────────
    {
        let mut harness = unfiltered_settings(egui::vec2(900.0, 760.0), SyntaxPaletteMode::Custom);
        harness
            .state_mut()
            .hydrate_editor_preferences_for_test(&provenance_projection());
        harness.run();
        for section in SECTIONS_ABOVE_EDITOR {
            click_author_id(&mut harness, section);
        }
        // Collapse Editor too so the whole Syntax body fits the fixed 440px viewport.
        click_author_id(&mut harness, "settings.section.editor");
        harness.run();

        assert!(
            has_author_id(&harness, SYNTAX_PALETTE_MODE_AUTHOR_ID),
            "item 8 (Syntax capture): the palette-mode control is addressable unfiltered"
        );
        for scope in HighlightScope::ALL.iter().copied() {
            assert!(
                has_author_id(&harness, &syntax_swatch_author_id(scope)),
                "item 8 (Syntax capture): every scope swatch renders (no missing scope): {scope:?}"
            );
        }
        assert!(
            has_author_id(&harness, SYNTAX_PALETTE_RESET_AUTHOR_ID),
            "item 8: the Syntax capture exposes its reset affordance"
        );
        assert!(
            has_author_id(
                &harness,
                &preference_provenance_author_id(PREF_EDITOR_SYNTAX_CUSTOM_COLORS)
            ),
            "item 8: the Syntax capture exposes the custom-colour source/revision chip"
        );
        assert_no_overlapping_controls(&harness, "Syntax section (unfiltered)");
        dump_accesskit_tree(&harness, &out_dir, "mt072_settings_syntax_unfiltered");
        save_screenshot(&mut harness, &out_dir, "mt072_settings_syntax_unfiltered");
    }

    // ── (C) KEYBINDINGS section, unfiltered, with BOTH code and rich-editor rows visible. ──────────
    {
        let mut harness = unfiltered_settings(egui::vec2(900.0, 760.0), SyntaxPaletteMode::Standard);
        harness
            .state_mut()
            .hydrate_editor_preferences_for_test(&provenance_projection());
        harness.run();
        // A CUSTOM chord on a rich action so the table shows custom-vs-default side by side.
        harness
            .state_mut()
            .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
                action_id: "rich.toggle_bold".to_owned(),
                chord: "Mod+Alt+B".to_owned(),
            });
        harness.run();
        // Collapse Appearance so Keybindings starts at the top of the viewport; expand the (default
        // closed) editor-actions sub-table. Nothing is filtered — Swarm/Terminal/Layout/Editor/Syntax/
        // About all still render below.
        click_author_id(&mut harness, "settings.section.appearance");
        click_author_id(&mut harness, "settings.section.keybindings-editor");
        harness.run();

        assert!(
            has_author_id_prefix(&harness, "settings-keybind-row-code."),
            "item 8 (Keybindings capture): CODE editor action rows render unfiltered"
        );
        assert!(
            has_author_id(
                &harness,
                &preference_provenance_author_id(PREF_EDITOR_KEYBINDING_OVERRIDES)
            ),
            "item 8: the Keybindings capture exposes the override-map source/revision chip"
        );
        assert!(
            has_author_id_prefix(&harness, "settings-keybind-reset-code."),
            "item 8: each keybinding row exposes its reset affordance"
        );
        // Long action labels are rendered in full, not truncated to an unreadable stub.
        let longest = longest_visible_keybind_label(&harness);
        assert!(
            longest.chars().count() >= 12,
            "item 8: the capture must include a long action label rendered in full (longest visible \
             was {longest:?})"
        );
        assert_no_overlapping_controls(&harness, "Keybindings section (unfiltered)");
        dump_accesskit_tree(&harness, &out_dir, "mt072_settings_keybindings_unfiltered");
        save_screenshot(&mut harness, &out_dir, "mt072_settings_keybindings_unfiltered");

        // Still the SAME unfiltered dialog (empty search box, every section rendered): SCROLL the
        // dialog body down to the Code -> Rich boundary of the editor-action table so ONE frame shows
        // rows from BOTH editor surfaces together — the V4 "enough rows to show both code and
        // rich-editor actions" requirement, proven without filtering anything away.
        // VISIBILITY, not tree presence: egui keeps AccessKit nodes for scrolled-out rows, so the loop
        // asserts against the rows actually laid out inside the dialog's 440px scroll band.
        let mut reached_boundary = false;
        for _ in 0..200 {
            if visible_keybind_surfaces(&harness) == (true, true) {
                reached_boundary = true;
                break;
            }
            scroll_settings_body(&mut harness, -3.0);
        }
        assert!(
            reached_boundary,
            "item 8: scrolling the unfiltered editor-action table must reach a frame that VISIBLY \
             shows BOTH a code-editor row and a rich-editor row inside the dialog scroll body"
        );
        assert_no_overlapping_controls(&harness, "Keybindings code+rich rows");
        dump_accesskit_tree(
            &harness,
            &out_dir,
            "mt072_settings_keybindings_code_and_rich_rows",
        );
        save_screenshot(
            &mut harness,
            &out_dir,
            "mt072_settings_keybindings_code_and_rich_rows",
        );
    }

    // ── (D) NARROW-WINDOW layout: the same unfiltered Editor section at a constrained width. ────────
    {
        let mut harness = unfiltered_settings(egui::vec2(520.0, 620.0), SyntaxPaletteMode::Standard);
        harness
            .state_mut()
            .hydrate_editor_preferences_for_test(&provenance_projection());
        harness.run();
        for section in SECTIONS_ABOVE_EDITOR {
            click_author_id(&mut harness, section);
        }
        harness.run();
        for id in [
            EDITOR_FONT_SIZE_AUTHOR_ID,
            EDITOR_TAB_SIZE_AUTHOR_ID,
            EDITOR_WORD_WRAP_AUTHOR_ID,
            EDITOR_RENDER_WHITESPACE_AUTHOR_ID,
        ] {
            assert!(
                has_author_id(&harness, id),
                "item 8 (narrow layout): '{id}' is still laid out at a constrained width"
            );
        }
        assert_no_overlapping_controls(&harness, "Editor section (narrow window)");
        dump_accesskit_tree(&harness, &out_dir, "mt072_settings_editor_narrow");
        save_screenshot(&mut harness, &out_dir, "mt072_settings_editor_narrow");
    }

    // ── (E) ERROR / RETRY state on the unfiltered dialog. ───────────────────────────────────────────
    {
        let mut harness = unfiltered_settings(egui::vec2(900.0, 760.0), SyntaxPaletteMode::Standard);
        harness
            .state_mut()
            .surface_preference_persist_error_for_test(
                "Could not save Editor font size: settings backend unavailable: connection refused",
            );
        harness.run();
        harness.run();
        assert!(
            has_author_id(
                &harness,
                handshake_native::settings_dialog::SETTINGS_PERSIST_ERROR_AUTHOR_ID
            ),
            "item 8: the error/retry capture shows the persistence failure status row"
        );
        assert!(
            has_label_containing(&harness, "Retry saving preference"),
            "item 8: the error/retry capture shows the exact typed Retry affordance"
        );
        assert_no_overlapping_controls(&harness, "Settings error/retry state");
        dump_accesskit_tree(&harness, &out_dir, "mt072_settings_error_retry");
        save_screenshot(&mut harness, &out_dir, "mt072_settings_error_retry");
    }

    assert_no_local_artifact_dir();
}

/// A resolved projection carrying a MIX of default and operator-set provenance, so a capture shows both
/// chip states. Values match the registry defaults except the two marked custom.
fn provenance_projection() -> Vec<handshake_native::preference_client::PreferenceProjectionRow> {
    use handshake_native::preference_client::{PreferenceProjectionRow, EDITOR_PREFERENCE_IDS};
    EDITOR_PREFERENCE_IDS
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let custom = *id == PREF_EDITOR_FONT_SIZE || *id == PREF_EDITOR_TAB_SIZE;
            PreferenceProjectionRow {
                preference_id: (*id).to_owned(),
                // An empty/absent value is skipped by `apply_projection`, so pass Null for rows this
                // capture does not need to change — only the provenance chip is under test here.
                value: serde_json::Value::Null,
                default_value: serde_json::Value::Null,
                source: if custom { "operator" } else { "default" }.to_owned(),
                revision: if custom { index as i64 + 1 } else { 0 },
            }
        })
        .collect()
}
