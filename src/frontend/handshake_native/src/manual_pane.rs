//! WP-KERNEL-012 MT-073 (E12) — the built-in **User Manual pane** for the native editors, plus the
//! in-Rust manual data model the content modules ([`crate::manual_content_editors`]) populate.
//!
//! ## Why this module exists (KERNEL_BUILDER gate, VERIFIED 2026-06-26)
//!
//! The MT-073 contract assumed a manual pane already existed from MT-072 — it does NOT (MT-072 built the
//! editor *Settings* sections, [`crate::settings_editor_section`], not a manual pane). So MT-073 CREATES
//! the manual pane here on top of the WP-011 shell primitives it reuses:
//!
//! - the WP-011 theme tokens ([`crate::theme::HsPalette`]) for all colors (no `Color32` literals),
//! - the WP-011 AccessKit live-emission helpers ([`crate::accessibility::emit_interactive_node`] +
//!   `ctx.accesskit_node_builder`) so every interactive control carries a stable `author_id`.
//!
//! The LIVE DOCK of this pane into the running `HandshakeApp` (a `pane_registry`/`app.rs` host-mount) is
//! an E11 carry to MT-080. THIS MT proves the pane + content + search + navigation + AccessKit ids +
//! the id-audit at the WIDGET level (egui_kittest), which is everything provable now.
//!
//! ## Data model
//!
//! A [`ManualSection`] is one top-level manual section (e.g. "Native Editors"). It holds a Vec of
//! [`ManualTopic`] (each an addressable heading + body) and an optional [`AgentToolReference`] — the
//! `author_id -> MCP tool` steering index (HBR-VIS / HBR-SWARM). The pane holds a [`ManualRegistry`]
//! (a Vec of sections); content modules call [`ManualRegistry::register_section`] to add their section.
//!
//! ## AccessKit identities (this pane owns)
//!
//! - [`MANUAL_PANE_AUTHOR_ID`] (`manual-pane`, container),
//! - [`MANUAL_SEARCH_AUTHOR_ID`] (`manual-search`, the keyword filter text input),
//! - one per-topic nav node `manual-topic-{section}.{topic}` ([`manual_topic_author_id`]).
//!
//! These are pane chrome ids, distinct from the editor/knowledge/interop `author_id`s the manual
//! *documents* — the manual's `author_id -> tool` rows reference the LIVE registered editor ids, which
//! the id-audit test cross-checks against the live registries.

use egui::accesskit;

use crate::theme::HsPalette;

/// AccessKit author_id for the manual pane container.
pub const MANUAL_PANE_AUTHOR_ID: &str = "manual-pane";
/// AccessKit author_id for the manual search box (the keyword topic filter).
pub const MANUAL_SEARCH_AUTHOR_ID: &str = "manual-search";
/// AccessKit author_id PREFIX for one navigation-tree topic row (`manual-topic-{section}.{topic}`).
pub const MANUAL_TOPIC_AUTHOR_ID_PREFIX: &str = "manual-topic-";

/// Which native editor / pillar surface an [`AgentToolRow`] addresses. The `surface` taxonomy the
/// agent-vision/steering reference groups rows by (HBR-VIS / HBR-SWARM).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManualSurface {
    /// The E1 VS-Code-class code editor.
    Code,
    /// The E2 Obsidian/Notion-class rich-text editor.
    RichText,
    /// The E3 Loom graph view.
    Graph,
    /// The E3 Loom canvas board.
    Canvas,
    /// The Stage pane (Pillar 17) interop surface.
    Fems,
    /// The Obsidian-class knowledge surface (folder tree / backlinks / outgoing / collections / FEMS).
    Knowledge,
    /// A cross-pillar interop edge (FEMS / Stage / Calendar / Locus).
    Interop,
}

impl ManualSurface {
    /// The stable lowercase surface key (used in tests + dumps).
    pub fn as_str(self) -> &'static str {
        match self {
            ManualSurface::Code => "code",
            ManualSurface::RichText => "rich-text",
            ManualSurface::Graph => "graph",
            ManualSurface::Canvas => "canvas",
            ManualSurface::Fems => "fems",
            ManualSurface::Knowledge => "knowledge",
            ManualSurface::Interop => "interop",
        }
    }
}

/// One agent-vision / steering row: pairs a stable AccessKit `author_id` with the REAL MCP swarm tool
/// that drives it. `author_id` and `mcp_tool` are `&'static str` literals so the id-audit test can
/// statically cross-check `author_id` against the live AccessKit registry (RISK-001 / MC-001) and assert
/// `mcp_tool` is a real `mcp/tools.rs` method name (RISK-002 / MC-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentToolRow {
    /// The stable AccessKit author_id a swarm agent addresses the control by. MUST be a LIVE registered
    /// id (the id-audit test fails on any orphan).
    pub author_id: &'static str,
    /// The surface this control belongs to (for grouping in the steering reference).
    pub surface: ManualSurface,
    /// A short human/model-readable action label.
    pub action_label: &'static str,
    /// The real Argus/MCP swarm tool. Prefer canonical `argus.inspect` / `argus.click` /
    /// `argus.set_value` / `argus.screenshot`; the primitive names remain compatibility aliases.
    /// Never an invented `gui.*` name.
    pub mcp_tool: &'static str,
    /// A one-line description of how an agent drives this control via `mcp_tool`.
    pub description: &'static str,
}

/// The agent-vision / steering reference topic body: one [`AgentToolRow`] per addressable
/// editor/knowledge/FEMS/interop action (HBR-VIS / HBR-SWARM). Carried by a [`ManualSection`] alongside
/// its prose topics.
#[derive(Debug, Clone)]
pub struct AgentToolReference {
    /// The reference's heading (an addressable topic, e.g. "Agent Tool Reference").
    pub heading: &'static str,
    /// The steering rows.
    pub rows: Vec<AgentToolRow>,
}

/// One addressable manual topic: a heading + a no-context body. The heading is the stable key the
/// heading-presence test asserts by name (the eight GLOBAL-BUILD-MANUAL headings each map to one topic).
#[derive(Debug, Clone)]
pub struct ManualTopic {
    /// The topic heading (stable, individually addressable — e.g. "Purpose", "Recovery Steps").
    pub heading: &'static str,
    /// The no-context body: concrete commands, pane names, AccessKit ids, keybinds — no hand-waving.
    pub body: String,
}

impl ManualTopic {
    /// True when `query` (lowercased) appears in this topic's heading or body. The same substring match
    /// the manual search box uses, so the search test and the data model agree on what "matches".
    pub fn matches(&self, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }
        self.heading.to_lowercase().contains(query_lower)
            || self.body.to_lowercase().contains(query_lower)
    }
}

/// One top-level manual section (e.g. the native-editors manual). Holds its prose topics and an optional
/// agent-tool steering reference. Content modules ([`crate::manual_content_editors`]) build a section and
/// the pane registers it via [`ManualRegistry::register_section`].
#[derive(Debug, Clone)]
pub struct ManualSection {
    /// The stable section key (kebab-case, e.g. `native-editors`), used in the per-topic author_id.
    pub id: &'static str,
    /// The section's display title.
    pub title: &'static str,
    /// The prose topics (the eight GLOBAL-BUILD-MANUAL headings live here).
    pub topics: Vec<ManualTopic>,
    /// The agent-vision / steering reference, if this section carries one.
    pub agent_tools: Option<AgentToolReference>,
}

impl ManualSection {
    /// Look up a topic by exact heading (used by the heading-presence test).
    pub fn topic(&self, heading: &str) -> Option<&ManualTopic> {
        self.topics.iter().find(|t| t.heading == heading)
    }

    /// True when EVERY heading in `required` is present as an individual topic (the GLOBAL-BUILD-MANUAL
    /// heading-presence invariant, AC-001 / MC-003).
    pub fn has_all_headings(&self, required: &[&str]) -> bool {
        required.iter().all(|h| self.topic(h).is_some())
    }

    /// All [`AgentToolRow`]s, or an empty slice when the section carries no steering reference.
    pub fn agent_rows(&self) -> &[AgentToolRow] {
        self.agent_tools
            .as_ref()
            .map(|r| r.rows.as_slice())
            .unwrap_or(&[])
    }
}

/// The stable AccessKit author_id for a navigation-tree topic row (`manual-topic-{section}.{topic_slug}`).
/// The topic slug is the heading lowercased with non-alphanumerics collapsed to `-`, so the id is stable
/// and kebab-cased.
pub fn manual_topic_author_id(section_id: &str, topic_heading: &str) -> String {
    let slug: String = topic_heading
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    // Collapse runs of '-' and trim, so "Startup and Run" -> "startup-and-run".
    let mut collapsed = String::with_capacity(slug.len());
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                collapsed.push(c);
            }
            prev_dash = true;
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }
    let collapsed = collapsed.trim_matches('-');
    format!("{MANUAL_TOPIC_AUTHOR_ID_PREFIX}{section_id}.{collapsed}")
}

/// The manual content registry: the ordered set of registered [`ManualSection`]s the pane renders. The
/// pane owns one; content modules push their section into it via [`Self::register_section`] (the single
/// registration call MT-073 wires from [`crate::manual_content_editors`]).
#[derive(Debug, Default, Clone)]
pub struct ManualRegistry {
    sections: Vec<ManualSection>,
}

impl ManualRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one section into the manual. The single registration mechanism MT-073 uses to add the
    /// editors content; duplicate section ids are rejected (kept first) so a double-register is a no-op
    /// rather than a duplicated nav entry.
    pub fn register_section(&mut self, section: ManualSection) {
        if self.sections.iter().any(|s| s.id == section.id) {
            return;
        }
        self.sections.push(section);
    }

    /// All registered sections in registration order.
    pub fn sections(&self) -> &[ManualSection] {
        &self.sections
    }

    /// Look up a section by id.
    pub fn section(&self, id: &str) -> Option<&ManualSection> {
        self.sections.iter().find(|s| s.id == id)
    }

    /// The number of registered sections.
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    /// True when no section is registered.
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Every topic across all sections whose heading or body matches `query` (substring, case-insensitive).
    /// The search index the search box and the search test both consume: registering a section makes its
    /// topic bodies searchable automatically (the contract's "registered sections are indexed" guarantee).
    pub fn search_topics<'a>(&'a self, query: &str) -> Vec<(&'a ManualSection, &'a ManualTopic)> {
        let q = query.to_lowercase();
        let mut hits = Vec::new();
        for section in &self.sections {
            for topic in &section.topics {
                if topic.matches(&q) {
                    hits.push((section, topic));
                }
            }
        }
        hits
    }
}

/// The transient per-frame UI state for the manual pane: the search query and the selected topic. Held by
/// the host (or a test) across frames; the registry (content) is immutable once registered.
#[derive(Debug, Default, Clone)]
pub struct ManualPaneState {
    /// The current search-box text (filters the visible topics).
    pub query: String,
    /// The currently selected `(section_id, topic_heading)`, if any.
    pub selected: Option<(String, String)>,
}

/// The manual pane widget: renders the search box + a filtered navigation list of topics + the selected
/// topic body, reading colors from the WP-011 [`HsPalette`]. Borrows the immutable [`ManualRegistry`]
/// (content) and the mutable [`ManualPaneState`] (search/selection).
pub struct ManualPane<'a> {
    registry: &'a ManualRegistry,
    state: &'a mut ManualPaneState,
    palette: &'a HsPalette,
}

impl<'a> ManualPane<'a> {
    /// Build a manual pane over the given registry + state + palette.
    pub fn new(
        registry: &'a ManualRegistry,
        state: &'a mut ManualPaneState,
        palette: &'a HsPalette,
    ) -> Self {
        Self {
            registry,
            state,
            palette,
        }
    }

    /// Render the pane. Emits the container AccessKit node, the search box (with the stable
    /// `manual-search` author_id), the filtered topic navigation list (each row carrying its stable
    /// `manual-topic-*` author_id), and the selected topic body. Quiet: it never grabs focus on its own
    /// and pops no window (HBR-QUIET) — it is a plain in-pane widget.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // The container AccessKit node (Role::Region, label so a no-context model finds the manual).
        let container_id = egui::Id::new(MANUAL_PANE_AUTHOR_ID);
        ui.ctx().accesskit_node_builder(container_id, |node| {
            node.set_role(accesskit::Role::Region);
            node.set_author_id(MANUAL_PANE_AUTHOR_ID.to_owned());
            node.set_label("User Manual".to_owned());
        });

        ui.vertical(|ui| {
            // ── Search box ──────────────────────────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                // The visible caption is deliberately DIFFERENT from the input's accessible label
                // ("Search Manual") so a label query resolves to exactly one node (the TextInput).
                ui.colored_label(self.palette.text_subtle, "Find in Manual");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.state.query)
                        .hint_text("filter topics by keyword")
                        .desired_width(220.0),
                );
                // The search box is an interactive widget egui already gave a Role::TextInput; attach the
                // stable author_id AND a stable accessible label (so a no-context model / kittest can find
                // it by name) WITHOUT overwriting egui's role/actions (the MT-072 emit_interactive pattern,
                // extended with a label the way the WP-011 settings search labels its box).
                let author = MANUAL_SEARCH_AUTHOR_ID.to_owned();
                ui.ctx().accesskit_node_builder(resp.id, move |node| {
                    node.set_author_id(author.clone());
                    node.set_label("Search Manual".to_owned());
                });
            });

            ui.separator();

            let q = self.state.query.to_lowercase();

            // ── Navigation list of matching topics ────────────────────────────────────────────────────
            ui.horizontal_top(|ui| {
                // Left: the nav list.
                ui.vertical(|ui| {
                    ui.set_min_width(240.0);
                    egui::ScrollArea::vertical()
                        .id_salt("manual-nav-scroll")
                        .max_height(360.0)
                        .show(ui, |ui| {
                            for section in self.registry.sections() {
                                let any_match = section.topics.iter().any(|t| t.matches(&q));
                                if !any_match {
                                    continue;
                                }
                                ui.colored_label(self.palette.accent, section.title);
                                for topic in &section.topics {
                                    if !topic.matches(&q) {
                                        continue;
                                    }
                                    let selected =
                                        self.state.selected.as_ref().is_some_and(|(s, t)| {
                                            s == section.id && t == topic.heading
                                        });
                                    let resp = ui.selectable_label(selected, topic.heading);
                                    let author = manual_topic_author_id(section.id, topic.heading);
                                    crate::accessibility::emit_interactive_node(
                                        ui.ctx(),
                                        resp.id,
                                        &author,
                                    );
                                    if resp.clicked() {
                                        self.state.selected =
                                            Some((section.id.to_owned(), topic.heading.to_owned()));
                                    }
                                }
                            }
                        });
                });

                ui.separator();

                // Right: the selected topic body (or the first matching topic when nothing is selected).
                ui.vertical(|ui| {
                    let body_topic = self.resolve_body_topic(&q);
                    egui::ScrollArea::vertical()
                        .id_salt("manual-body-scroll")
                        .max_height(360.0)
                        .show(ui, |ui| {
                            if let Some((section, topic)) = body_topic {
                                ui.colored_label(self.palette.text, topic.heading);
                                ui.add_space(4.0);
                                ui.colored_label(self.palette.text, topic.body.clone());
                                ui.add_space(8.0);
                                self.render_agent_tools(ui, section);
                            } else {
                                ui.colored_label(
                                    self.palette.text_subtle,
                                    "No manual topic matches the search.",
                                );
                            }
                        });
                });
            });
        });
    }

    /// The topic whose body to render: the explicitly selected one if it still matches, else the first
    /// topic matching the current query.
    fn resolve_body_topic(
        &self,
        query_lower: &str,
    ) -> Option<(&'a ManualSection, &'a ManualTopic)> {
        if let Some((sid, heading)) = &self.state.selected {
            if let Some(section) = self.registry.section(sid) {
                if let Some(topic) = section.topic(heading) {
                    if topic.matches(query_lower) {
                        return Some((section, topic));
                    }
                }
            }
        }
        self.registry.search_topics(query_lower).into_iter().next()
    }

    /// Render the section's agent-tool steering rows (read-only), if the selected topic is the agent-tool
    /// reference heading. Each row's author_id is shown so a model reading the manual sees the real id.
    fn render_agent_tools(&self, ui: &mut egui::Ui, section: &ManualSection) {
        let Some(reference) = section.agent_tools.as_ref() else {
            return;
        };
        // Only show the table when the agent-tool reference topic is the one in view (its heading is a
        // topic too, so it surfaces in search / selection like the prose topics).
        let showing_reference = self
            .state
            .selected
            .as_ref()
            .is_some_and(|(_, t)| t == reference.heading);
        if !showing_reference {
            return;
        }
        ui.colored_label(
            self.palette.text_subtle,
            "Agent Tool Reference (author_id -> MCP tool)",
        );
        for row in &reference.rows {
            ui.colored_label(
                self.palette.text,
                format!(
                    "[{}] {} -> {}  ({})",
                    row.surface.as_str(),
                    row.author_id,
                    row.mcp_tool,
                    row.action_label
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manual_content_editors::editors_manual_section;

    #[test]
    fn topic_author_id_is_stable_and_kebab() {
        assert_eq!(
            manual_topic_author_id("native-editors", "Startup and Run"),
            "manual-topic-native-editors.startup-and-run"
        );
        assert_eq!(
            manual_topic_author_id("native-editors", "Inputs and Outputs"),
            "manual-topic-native-editors.inputs-and-outputs"
        );
    }

    #[test]
    fn register_section_is_idempotent() {
        let mut reg = ManualRegistry::new();
        reg.register_section(editors_manual_section());
        reg.register_section(editors_manual_section());
        assert_eq!(
            reg.len(),
            1,
            "double-register of the same section id is a no-op"
        );
    }

    #[test]
    fn search_finds_topic_body_after_registration() {
        let mut reg = ManualRegistry::new();
        reg.register_section(editors_manual_section());
        // A keyword that lives in a topic BODY (not the heading) proves the body is indexed.
        let hits = reg.search_topics("command palette");
        assert!(
            !hits.is_empty(),
            "a body keyword surfaces a topic via the registered search index"
        );
    }

    #[test]
    fn topic_matches_is_case_insensitive_substring() {
        let t = ManualTopic {
            heading: "Purpose",
            body: "The Code editor mounts.".to_owned(),
        };
        assert!(t.matches("code"));
        assert!(t.matches("purpose"));
        assert!(!t.matches("excalidraw"));
        assert!(t.matches(""), "empty query matches everything");
    }
}
