//! Top-right MODULE switcher for the native work surface (WP-KERNEL-011 MT-012).
//!
//! ## What this provides
//!
//! A horizontal rail of six pill/tab buttons — `MAIN`, `Atelier`, `INGEST`, `STAGE`, `LAB`, `STUDIO` —
//! rendered right-aligned in the shell header row (next to the theme toggle). Clicking a module
//! button switches the active MODULE on the currently active pane: it rewrites that pane's tab list to
//! the module's canonical tab set (with the module's default tab first, then the module's tabs, then
//! the pane's already-open tabs, deduped preserving order) and activates the module's default tab.
//!
//! This is the C3 navigation MODULE layer. It is DISTINCT from:
//! - the project (workspace) tabs ([`crate::project_tabs`], MT-011), which switch whole projects, and
//! - the per-pane document tab bar ([`crate::tab_bar`], MT-007), which switches documents inside a pane.
//!
//! ## Faithful port of the React module model
//!
//! [`ModuleId`] mirrors the React `ModuleId` union (`app/src/App.tsx` line 77) and serializes to the
//! exact uppercase strings (`"MAIN"`, `"CKC"`, …) used by the workbench layout persistence schema.
//! [`MODULE_DEFINITIONS`] is a compile-time const array that ports the React `MODULE_DEFINITIONS` const
//! (`app/src/App.tsx` lines 197-267) verbatim — same six modules, same tab lists, same default tabs —
//! so a snapshot test can prove the native definitions never drift from the React source. The tab ids
//! map onto the existing [`PaneType`] enum (itself the Rust port of the React `PaneTabId` union), so the
//! switcher reuses the one tab-identity vocabulary the rest of the shell already shares.
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! The module count is FIXED at six, so — unlike the dynamic project tabs — each module button gets a
//! fixed `NodeId` in a dedicated fresh band ([`MODULE_NODE_ID_BASE`] = 51..56), disjoint from every
//! other declared identity (theme toggle 10, chrome 20/21, dividers 30/31, scrollbar rails 40..43,
//! project-tab strip 50, tab-bar containers 60..63, merge-back 64..67, panes >= 100). The collision
//! test in [`crate::accessibility::registry`] proves the disjointness across the whole declared set.
//! Each button is a `Role::Button` node with `Action::Click`/`Action::Focus` and an `author_id` equal
//! to its `data_id` (e.g. `module-main`) — the same stable key the React `data-stable-id` attribute
//! used — so an out-of-process model addresses a module button by a stable id, not a frame counter.

use egui::accesskit;

use crate::pane_registry::PaneType;

/// Fixed AccessKit/egui `NodeId` of the FIRST module button (`MAIN`). The six module buttons occupy
/// the FRESH band 51..=56 — slot 50 is the project-tab strip container ([`crate::project_tabs`]), and
/// the per-pane tab-bar containers start at 60, so 51..56 sits cleanly between them and strictly below
/// the pane id base (100). Each button's id is `MODULE_NODE_ID_BASE + index_in_MODULE_DEFINITIONS`.
/// A fixed-value `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames + process
/// restarts — the same convention the theme toggle, chrome, and dividers use.
pub const MODULE_NODE_ID_BASE: u64 = 51;

/// The work-surface MODULE a pane is showing. Ported from the React `ModuleId` union
/// (`app/src/App.tsx` line 77). Serializes to the uppercase string the workbench layout persistence
/// schema uses (`"MAIN"`, `"CKC"`, `"INGEST"`, `"STAGE"`, `"LAB"`, `"STUDIO"`) via [`ModuleId::as_str`]
/// / [`ModuleId::parse`] so the persisted `active_module` field matches the React blob exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleId {
    Main,
    Ckc,
    Ingest,
    Stage,
    Lab,
    Studio,
}

impl ModuleId {
    /// The uppercase serialization string for this module (the React `ModuleId` value + the
    /// `active_module` layout-persistence string).
    pub fn as_str(self) -> &'static str {
        match self {
            ModuleId::Main => "MAIN",
            ModuleId::Ckc => "CKC",
            ModuleId::Ingest => "INGEST",
            ModuleId::Stage => "STAGE",
            ModuleId::Lab => "LAB",
            ModuleId::Studio => "STUDIO",
        }
    }

    /// Parse a serialized module string back into a [`ModuleId`]. Returns `None` for an unknown string
    /// so a corrupt/foreign layout blob falls back to the default module rather than panicking.
    ///
    /// Named `parse` (not `from_str`) deliberately: this returns `Option<Self>` (a missing-value
    /// signal, not a typed error), so it is NOT the `std::str::FromStr` contract. Using the inherent
    /// name `from_str` would shadow the std trait method and trips `clippy::should_implement_trait`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "MAIN" => Some(ModuleId::Main),
            "CKC" => Some(ModuleId::Ckc),
            "INGEST" => Some(ModuleId::Ingest),
            "STAGE" => Some(ModuleId::Stage),
            "LAB" => Some(ModuleId::Lab),
            "STUDIO" => Some(ModuleId::Studio),
            _ => None,
        }
    }

    /// This module's definition (label, data_id, tab list, default tab) from [`MODULE_DEFINITIONS`].
    pub fn definition(self) -> &'static ModuleDefinition {
        MODULE_DEFINITIONS
            .iter()
            .find(|def| def.id == self)
            // Safe: MODULE_DEFINITIONS is a const that contains every ModuleId variant, proven by the
            // `every_module_id_has_a_definition` unit test.
            .expect("MODULE_DEFINITIONS contains every ModuleId variant")
    }
}

// ModuleId serializes/deserializes as its uppercase string so the persisted `active_module` field is
// the exact React value (`"MAIN"`, …), not a Rust-enum JSON object. Hand-written rather than derived so
// the wire form is the string, matching the React workbench layout schema.
impl serde::Serialize for ModuleId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ModuleId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ModuleId::parse(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown module id '{s}'")))
    }
}

/// One module's compile-time definition: its id, header label, stable `data_id` (the AccessKit
/// author_id), the canonical tab set it opens, and the tab it activates by default. Ported verbatim
/// from the React `MODULE_DEFINITIONS` const objects (`app/src/App.tsx` lines 197-267).
///
/// Not `Copy`: [`PaneType`] carries a `Placeholder(String)` variant so it is not `Copy`, and
/// `default_tab` is a `PaneType`. Definitions are only ever borrowed (`MODULE_DEFINITIONS` is a const
/// and [`ModuleId::definition`] returns a `&'static`), so `Copy` is unnecessary.
#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    /// The module identity.
    pub id: ModuleId,
    /// The button label shown in the header (same visible label as the React `label`).
    pub label: &'static str,
    /// The stable kebab-case id (`module-main`, …) used as the AccessKit author_id — the React
    /// `data-stable-id` value adopted verbatim.
    pub data_id: &'static str,
    /// The canonical tab set this module opens on the active pane, in display order. Each entry is a
    /// [`PaneType`] (the Rust port of the React `PaneTabId`).
    pub tabs: &'static [PaneType],
    /// The tab activated when this module is selected (always the first effective tab after dedup).
    pub default_tab: PaneType,
}

/// Every work-surface module, in header display order. A const array (NOT config-loaded) so the
/// snapshot test can prove completeness + non-drift at compile time, mirroring the React
/// `MODULE_DEFINITIONS` const (`app/src/App.tsx` lines 197-267) exactly.
pub const MODULE_DEFINITIONS: [ModuleDefinition; 6] = [
    ModuleDefinition {
        id: ModuleId::Main,
        label: "MAIN",
        data_id: "module-main",
        tabs: &[
            PaneType::Workspace,
            PaneType::LoomDailyJournal,
            PaneType::LoomBlock,
            PaneType::LoomWikiPage,
            PaneType::UserManual,
            PaneType::Problems,
            PaneType::Jobs,
            PaneType::Timeline,
        ],
        default_tab: PaneType::Workspace,
    },
    ModuleDefinition {
        id: ModuleId::Ckc,
        label: "Atelier",
        data_id: "module-ckc",
        tabs: &[
            PaneType::AtelierEditor,
            PaneType::KernelDcc,
            PaneType::CodeSymbol,
            PaneType::SourceControl,
            PaneType::LoomDailyJournal,
            PaneType::LoomBlock,
            PaneType::LoomWikiPage,
            PaneType::UserManual,
            PaneType::Problems,
            PaneType::Jobs,
            PaneType::Timeline,
        ],
        default_tab: PaneType::AtelierEditor,
    },
    ModuleDefinition {
        id: ModuleId::Ingest,
        label: "INGEST",
        data_id: "module-ingest",
        tabs: &[
            PaneType::MediaDownloader,
            PaneType::FontManager,
            PaneType::FlightRecorder,
            PaneType::VisualDebugger,
            PaneType::Problems,
        ],
        default_tab: PaneType::MediaDownloader,
    },
    ModuleDefinition {
        id: ModuleId::Stage,
        label: "STAGE",
        data_id: "module-stage",
        tabs: &[
            PaneType::FontManager,
            PaneType::InferenceLab,
            PaneType::VisualDebugger,
            PaneType::FlightRecorder,
            PaneType::Problems,
        ],
        default_tab: PaneType::FontManager,
    },
    ModuleDefinition {
        id: ModuleId::Lab,
        label: "LAB",
        data_id: "module-lab",
        tabs: &[
            PaneType::InferenceLab,
            PaneType::ModelRuntime,
            PaneType::Swarm,
            PaneType::FontManager,
            PaneType::KernelDcc,
            PaneType::UserManual,
        ],
        default_tab: PaneType::InferenceLab,
    },
    ModuleDefinition {
        id: ModuleId::Studio,
        label: "STUDIO",
        data_id: "module-studio",
        tabs: &[
            PaneType::ModelRuntime,
            PaneType::Swarm,
            PaneType::InferenceLab,
            PaneType::FontManager,
            PaneType::KernelDcc,
            PaneType::UserManual,
        ],
        default_tab: PaneType::ModelRuntime,
    },
];

/// Compute the tab list a pane should have after switching to `module`, mirroring the React
/// `setModule` body (`app/src/App.tsx` lines 1463-1483): `uniqueTabs([defaultTab, ...module.tabs,
/// ...existing_pane_tabs])` — the default tab first, then the module's tabs, then the pane's currently
/// open tabs, with duplicates removed while preserving first-seen order (React `uniqueTabs` =
/// `[...new Set(tabs)]`). The returned list always starts with the module's default tab, so the active
/// tab (the default tab) is at index 0.
pub fn module_tab_list(module: ModuleId, existing_pane_tabs: &[PaneType]) -> Vec<PaneType> {
    let def = module.definition();
    let mut ordered: Vec<PaneType> =
        Vec::with_capacity(1 + def.tabs.len() + existing_pane_tabs.len());
    ordered.push(def.default_tab.clone());
    ordered.extend(def.tabs.iter().cloned());
    ordered.extend(existing_pane_tabs.iter().cloned());

    // Dedup preserving first-seen order (PaneType is not Hash-stable-cheap to rely on a HashSet across
    // the Placeholder(String) variant, and the list is tiny, so a linear `seen` check is both correct
    // and clear — same effect as React's `[...new Set(tabs)]`).
    let mut seen: Vec<PaneType> = Vec::with_capacity(ordered.len());
    for tab in ordered {
        if !seen.contains(&tab) {
            seen.push(tab);
        }
    }
    seen
}

/// Colors the module switcher paints with, sourced from the active theme tokens by the caller so the
/// switcher never reads egui's generic visuals (mirrors [`crate::project_tabs::ProjectTabColors`]).
#[derive(Debug, Clone, Copy)]
pub struct ModuleSwitcherColors {
    /// Background fill of the active module button.
    pub active_bg: egui::Color32,
    /// Background fill of an inactive module button.
    pub inactive_bg: egui::Color32,
    /// Background fill of a hovered (inactive) module button.
    pub hover_bg: egui::Color32,
    /// Label text color of an inactive button.
    pub text: egui::Color32,
    /// Label text color of the active button.
    pub active_text: egui::Color32,
}

/// The top-right MODULE switcher widget + its state.
///
/// Owns only the active module id; it does NOT own pane/tab state. Switching a module is reported to
/// the caller ([`crate::app::HandshakeApp`]) via the [`Self::show`] return value, and the app performs
/// the active-pane tab-list mutation (single source of truth for pane state).
#[derive(Debug, Clone)]
pub struct ModuleSwitcher {
    active: ModuleId,
}

impl ModuleSwitcher {
    /// Build a switcher with the given active module.
    pub fn new(active: ModuleId) -> Self {
        Self { active }
    }

    /// The active module id.
    pub fn active(&self) -> ModuleId {
        self.active
    }

    /// Set the active module id (called by the app after a switch / a layout restore so the highlight
    /// tracks the shell's `active_module`).
    pub fn set_active(&mut self, module: ModuleId) {
        self.active = module;
    }

    /// Render the six module buttons into `ui` and return `Some(module_id)` when the operator clicked a
    /// module button that is NOT the active one (a switch request). Returns `None` when nothing changed
    /// or the already-active module was re-clicked (a no-op — the contract's no-op acceptance criterion).
    ///
    /// AccessKit: each button is a `Role::Button` node with author_id = its `data_id` (`module-main`,
    /// …) and `Action::Click`; the active button is marked `selected`.
    pub fn show(&mut self, ui: &mut egui::Ui, colors: ModuleSwitcherColors) -> Option<ModuleId> {
        let mut switch_to: Option<ModuleId> = None;

        ui.horizontal(|ui| {
            for (index, def) in MODULE_DEFINITIONS.iter().enumerate() {
                let is_active = self.active == def.id;
                if Self::render_button(ui, def, index, is_active, colors) && !is_active {
                    switch_to = Some(def.id);
                }
            }
        });

        switch_to
    }

    /// Render a single module button as a real interactive egui widget at its FIXED AccessKit id and
    /// emit its `Role::Button` node enriched with the stable author_id + selected state. Returns `true`
    /// if it was clicked this frame.
    fn render_button(
        ui: &mut egui::Ui,
        def: &ModuleDefinition,
        index: usize,
        is_active: bool,
        colors: ModuleSwitcherColors,
    ) -> bool {
        // Fixed-value Id -> fixed AccessKit NodeId in the 51..56 band. Each button's id is the band
        // base plus its index in MODULE_DEFINITIONS, so the ids are stable and disjoint by construction
        // (the registry collision test proves they do not overlap any other declared identity).
        let node_id = MODULE_NODE_ID_BASE + index as u64;
        let button_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };

        let text_color = if is_active {
            colors.active_text
        } else {
            colors.text
        };
        let font = egui::FontId::proportional(13.0);
        let galley = ui
            .painter()
            .layout_no_wrap(def.label.to_owned(), font, text_color);

        let pad_x = 10.0;
        let pad_y = 4.0;
        let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        // Interact at the FIXED button_id so the Response, its widget_info (Role::Button / label /
        // Action::Click), the AccessKit bounding box, and the author_id all land on the SAME node
        // (mirrors the theme-toggle + project-tab id discipline).
        let response = ui.interact(rect, button_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let bg = if is_active {
                colors.active_bg
            } else if response.hovered() {
                colors.hover_bg
            } else {
                colors.inactive_bg
            };
            ui.painter().rect_filled(rect, 4.0, bg);
            let text_pos = egui::pos2(rect.left() + pad_x, rect.center().y - galley.size().y * 0.5);
            ui.painter().galley(text_pos, galley, text_color);
        }

        // AccessKit: egui derived Action::Click/Action::Focus from Sense::click(); add the Button role +
        // label + selected state via widget_info, then attach the stable author_id to the SAME node.
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                ui.is_enabled(),
                is_active,
                def.label,
            )
        });
        ui.ctx().accesskit_node_builder(button_id, |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(def.data_id.to_owned());
            node.set_label(def.label.to_owned());
            if is_active {
                node.set_selected(true);
            }
        });

        response.clicked()
    }
}

/// The React kebab-case `PaneTabId` string for a [`PaneType`] — the inverse of the React
/// `TAB_LABEL_BY_ID` keys (`app/src/App.tsx` lines 175-195). Used by the `definitions_match_react`
/// snapshot test and by [`crate::app`] to serialize `active_module`-adjacent tab ids if needed. Kept a
/// free function (not a `PaneType` method) so it stays local to the module-switcher port and does not
/// widen the `PaneType` public surface owned by MT-005.
pub fn pane_type_tab_id(pane_type: &PaneType) -> &'static str {
    match pane_type {
        PaneType::Workspace => "workspace",
        PaneType::MediaDownloader => "media-downloader",
        PaneType::FontManager => "fonts",
        PaneType::FlightRecorder => "flight-recorder",
        PaneType::KernelDcc => "kernel-dcc",
        PaneType::InferenceLab => "inference-lab",
        PaneType::ModelRuntime => "model-runtime",
        PaneType::Swarm => "swarm",
        PaneType::Problems => "problems",
        PaneType::Jobs => "jobs",
        PaneType::Timeline => "timeline",
        PaneType::UserManual => "user-manual",
        PaneType::CodeSymbol => "code-symbol",
        PaneType::SourceControl => "source-control",
        PaneType::LoomDailyJournal => "loom-daily-journal",
        PaneType::LoomBlock => "loom-block",
        PaneType::LoomWikiPage => "loom-wiki-page",
        PaneType::AtelierEditor => "atelier",
        PaneType::VisualDebugger => "visual-debugger",
        // WP-KERNEL-012 MT-028: the native LoomSearchV2 surface (no React PaneTabId — it is a
        // KERNEL-012 native addition, not part of the React MODULE_DEFINITIONS tab list).
        PaneType::LoomSearchV2 => "loom-search-v2",
        // WP-KERNEL-012 MT-029: the native Find-in-Files surface (a KERNEL-012 native addition, not
        // part of the React MODULE_DEFINITIONS tab list).
        PaneType::FindInFiles => "find-in-files",
        // Placeholder has no React PaneTabId; it is not part of any MODULE_DEFINITIONS tab list.
        PaneType::Placeholder(_) => "placeholder",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn module_id_round_trips_through_string() {
        for module in [
            ModuleId::Main,
            ModuleId::Ckc,
            ModuleId::Ingest,
            ModuleId::Stage,
            ModuleId::Lab,
            ModuleId::Studio,
        ] {
            assert_eq!(ModuleId::parse(module.as_str()), Some(module));
        }
        assert_eq!(ModuleId::parse("NOPE"), None);
    }

    #[test]
    fn module_id_serializes_as_uppercase_string() {
        assert_eq!(serde_json::to_value(ModuleId::Lab).unwrap(), json!("LAB"));
        let back: ModuleId = serde_json::from_value(json!("STUDIO")).unwrap();
        assert_eq!(back, ModuleId::Studio);
    }

    #[test]
    fn every_module_id_has_a_definition() {
        // definition() must not panic for any variant — proves MODULE_DEFINITIONS is exhaustive.
        for module in [
            ModuleId::Main,
            ModuleId::Ckc,
            ModuleId::Ingest,
            ModuleId::Stage,
            ModuleId::Lab,
            ModuleId::Studio,
        ] {
            assert_eq!(module.definition().id, module);
        }
        assert_eq!(MODULE_DEFINITIONS.len(), 6);
    }

    /// MODULE_DEFINITIONS serializes to the SAME JSON object as the React `MODULE_DEFINITIONS` const
    /// (`app/src/App.tsx` lines 197-267): each module's id, label, data_id, tab list (kebab-case ids),
    /// and default tab. This is the contract's drift gate — if a tab list or default tab diverges from
    /// React, this test fails. Tab ids are serialized to the React kebab-case `PaneTabId` strings via
    /// `pane_type_tab_id`.
    #[test]
    fn definitions_match_react() {
        let actual: Vec<serde_json::Value> = MODULE_DEFINITIONS
            .iter()
            .map(|def| {
                json!({
                    "id": def.id.as_str(),
                    "label": def.label,
                    "dataId": def.data_id,
                    "tabs": def.tabs.iter().map(|t| pane_type_tab_id(t)).collect::<Vec<_>>(),
                    "defaultTab": pane_type_tab_id(&def.default_tab),
                })
            })
            .collect();

        let expected = json!([
            {
                "id": "MAIN", "label": "MAIN", "dataId": "module-main",
                "tabs": ["workspace", "loom-daily-journal", "loom-block", "loom-wiki-page", "user-manual", "problems", "jobs", "timeline"],
                "defaultTab": "workspace"
            },
            {
                "id": "CKC", "label": "Atelier", "dataId": "module-ckc",
                "tabs": ["atelier", "kernel-dcc", "code-symbol", "source-control", "loom-daily-journal", "loom-block", "loom-wiki-page", "user-manual", "problems", "jobs", "timeline"],
                "defaultTab": "atelier"
            },
            {
                "id": "INGEST", "label": "INGEST", "dataId": "module-ingest",
                "tabs": ["media-downloader", "fonts", "flight-recorder", "visual-debugger", "problems"],
                "defaultTab": "media-downloader"
            },
            {
                "id": "STAGE", "label": "STAGE", "dataId": "module-stage",
                "tabs": ["fonts", "inference-lab", "visual-debugger", "flight-recorder", "problems"],
                "defaultTab": "fonts"
            },
            {
                "id": "LAB", "label": "LAB", "dataId": "module-lab",
                "tabs": ["inference-lab", "model-runtime", "swarm", "fonts", "kernel-dcc", "user-manual"],
                "defaultTab": "inference-lab"
            },
            {
                "id": "STUDIO", "label": "STUDIO", "dataId": "module-studio",
                "tabs": ["model-runtime", "swarm", "inference-lab", "fonts", "kernel-dcc", "user-manual"],
                "defaultTab": "model-runtime"
            }
        ]);

        assert_eq!(
            serde_json::Value::Array(actual),
            expected,
            "MODULE_DEFINITIONS must match the React MODULE_DEFINITIONS (App.tsx 197-267) exactly"
        );
    }

    #[test]
    fn module_tab_list_puts_default_first_and_dedups() {
        // LAB default = inference-lab; existing pane already has workspace + inference-lab open.
        let existing = vec![PaneType::Workspace, PaneType::InferenceLab];
        let list = module_tab_list(ModuleId::Lab, &existing);
        assert_eq!(
            list.first(),
            Some(&PaneType::InferenceLab),
            "default tab is first"
        );
        // inference-lab appears exactly once (dedup), and the existing-only Workspace is preserved at end.
        assert_eq!(
            list.iter()
                .filter(|t| **t == PaneType::InferenceLab)
                .count(),
            1,
            "module default tab is not duplicated"
        );
        assert!(
            list.contains(&PaneType::Workspace),
            "existing open tab preserved"
        );
        // Full expected order: [inference-lab] + LAB tabs + existing (deduped).
        assert_eq!(
            list,
            vec![
                PaneType::InferenceLab,
                PaneType::ModelRuntime,
                PaneType::Swarm,
                PaneType::FontManager,
                PaneType::KernelDcc,
                PaneType::UserManual,
                PaneType::Workspace,
            ]
        );
    }

    #[test]
    fn module_node_ids_sit_in_a_disjoint_fresh_band() {
        // The six button ids are 51..=56: above the project-tab strip container (50), below the
        // per-pane tab-bar containers (60..63), strictly below the pane id base (100), and disjoint
        // from every other fixed id. The full collision proof is in accessibility::registry.
        for index in 0..MODULE_DEFINITIONS.len() as u64 {
            let id = MODULE_NODE_ID_BASE + index;
            assert!((51..=56).contains(&id), "module id {id} in the 51..56 band");
            assert!(id < crate::accessibility::PANE_NODE_ID_BASE);
            for fixed in [10_u64, 20, 21, 30, 31, 40, 41, 42, 43, 50, 60, 61, 62, 63] {
                assert_ne!(id, fixed, "module id collides with fixed id {fixed}");
            }
        }
    }
}
