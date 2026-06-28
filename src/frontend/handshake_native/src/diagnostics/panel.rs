//! WP-KERNEL-012 MT-087 (D3 — internal_diagnostics, Tier 2 §5.8.4 in-app Diagnostics Panel +
//! §10.12.5 three-tier model): the egui widget that PROJECTS the live `internal_diagnostics` state.
//!
//! # What this is — a pure projection (§5.8.4), NOT a state owner
//!
//! Master Spec v02.196 §5.8.4 calls the in-app Diagnostics Panel a PROJECTION over
//! `internal_diagnostics`: it holds NO state authority. This widget renders, every frame, from the
//! live producers built by the earlier diagnostic MTs:
//!
//! - **Heartbeat** (MT-084) — the UI-thread frame counter + monotonic clock. A non-zero, advancing
//!   counter says "the app is alive". Sourced from [`DiagnosticsView::heartbeat_counter`] /
//!   `heartbeat_elapsed_nanos` (the app's `frame_counter` + heartbeat clock, read each frame).
//! - **Frame-time** (MT-085) — last / p50 / p95 from [`crate::diagnostics::FrameStats`], formatted ms.
//! - **Resource** (MT-086) — CPU% + RSS from the LAST `ResourceSample` event in the process-global
//!   ring, plus the static `GpuInfo` hardware line.
//! - **Last-N events** (MT-082) — read DIRECTLY from the process-global [`crate::diagnostics::
//!   snapshot_last_n`] each frame (the panel allocates NO copy of its own — RISK-007-2).
//! - **Tier-3 Palmistry** — an honest empty-state placeholder. Live freeze/crash forwarding is MT-093;
//!   the three-tier §10.12.5 layout is PRESENT here, honestly empty, not faked (AC-007-4).
//!
//! # No own authority (RISK-007-2 / §5.8.4)
//!
//! The panel caches NOTHING. Events come straight from [`crate::diagnostics::snapshot_last_n`]; the
//! frame/heartbeat/resource/GPU values come from a read-only [`DiagnosticsView`] the SHELL rebuilds
//! from the live `HandshakeApp` producers each frame (`frame_stats()`, `frame_counter()`, `gpu_info()`,
//! and the last ring `ResourceSample`). So the panel can never drift from the producer.
//!
//! # Typed-allowlist preserved by construction (§5.8.3)
//!
//! A [`DiagEvent`] is typed integers only — there is no free text to render. Each event row maps its
//! `event_code` (a closed `u16`) to a STATIC human label via [`event_code_label`]; the privacy
//! invariant holds because the panel never receives or renders a string from a producer.
//!
//! # Theme tokens only (CONTROL-4 / AC-007-5)
//!
//! Every colour comes from the live [`HsPalette`] (`palette.text`, `palette.text_subtle`,
//! `palette.surface`, `palette.diagnostics.*`, ...). There is NO opaque colour literal in this file
//! (the grep guard in `tests/test_theme.rs` flags the opaque-RGB construction form outside
//! `theme/palette.rs` / `theme/syntax.rs`); severity colours come from `palette.diagnostics`.

use egui::accesskit;

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagSeverity};

use crate::diagnostics::{FrameStats, GpuInfo, ResourceSample};
use crate::theme::HsPalette;

// ── Stable AccessKit author_ids (HBR-VIS / HBR-SWARM — out-of-process steering) ────────────────────
//
// The panel container is a Role::Region with a stable author_id so a no-context model + swarm agents
// address the whole diagnostics surface by `diagnostics_panel`; each child section is a Role::Group
// with its own stable author_id. These are addressed by author_id STRING in egui's hashed id space
// (the same convention settings_editor_section.rs uses for its controls), NOT a fixed NodeId band, so
// they are NOT enumerated in accessibility::registry::DECLARED_IDENTITIES.

/// AccessKit author_id for the whole Diagnostics panel container (Role::Region). The MT names this
/// EXACTLY `diagnostics_panel` (AC-007-1 / AC-007-5).
pub const DIAGNOSTICS_PANEL_AUTHOR_ID: &str = "diagnostics_panel";
/// AccessKit author_id for the heartbeat section (Role::Group). The MT names this `diagnostics_heartbeat`.
pub const DIAGNOSTICS_HEARTBEAT_AUTHOR_ID: &str = "diagnostics_heartbeat";
/// AccessKit author_id for the frame-time section (Role::Group).
pub const DIAGNOSTICS_FRAME_AUTHOR_ID: &str = "diagnostics_frame";
/// AccessKit author_id for the resource (CPU/RSS/GPU) section (Role::Group).
pub const DIAGNOSTICS_RESOURCE_AUTHOR_ID: &str = "diagnostics_resource";
/// AccessKit author_id for the last-N events section (Role::Group). The MT names this `diagnostics_events`.
pub const DIAGNOSTICS_EVENTS_AUTHOR_ID: &str = "diagnostics_events";
/// AccessKit author_id for the Tier-3 Palmistry section (Role::Group).
pub const DIAGNOSTICS_PALMISTRY_AUTHOR_ID: &str = "diagnostics_palmistry";

/// How many recent events the panel surfaces (§5.8.4 "last-N"). The recorder buffer is larger
/// ([`crate::diagnostics::BUFFER_CAP`] = 512); the panel shows the most-recent window so a long session
/// never renders an unbounded list. ~50 is the MT-named budget.
pub const PANEL_EVENT_WINDOW: usize = 50;

/// A read-only snapshot of the live `internal_diagnostics` state the SHELL builds each frame from the
/// `HandshakeApp` producers and hands to [`DiagnosticsPanel::show`]. The panel holds NO own authority
/// (§5.8.4 / RISK-007-2): this view is rebuilt every frame from `frame_stats()` / `frame_counter()` /
/// `gpu_info()` / the last ring `ResourceSample`, so it is always a faithful projection of the
/// producers, never a cached divergent copy. The last-N events are NOT in this view — the panel reads
/// them straight from the process-global [`crate::diagnostics::snapshot_last_n`] each frame.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticsView {
    /// The UI-thread heartbeat frame counter (MT-084). 0 before the first frame; advances by 1 each
    /// `update`. A non-zero, advancing value is the liveness signal (AC-007-2).
    pub heartbeat_counter: u64,
    /// Monotonic nanoseconds elapsed since process start at the last heartbeat (MT-084). Used to render
    /// a human "time since process start" / "last beat" line. Monotonic (never goes backward).
    pub heartbeat_elapsed_nanos: u64,
    /// The live frame-time stats (MT-085): last/min/max/p50/p95 + counts, all integer micros.
    pub frame_stats: FrameStats,
    /// The most-recent CPU%/RSS resource sample (MT-086), if one has been emitted yet. `None` before
    /// the first sample (the ~1s cadence gate means the very first frames may have none).
    pub last_resource_sample: Option<ResourceSample>,
    /// How many resource samples have been emitted so far (MT-086). Lets the panel show "no sample yet"
    /// honestly vs a real zero sample.
    pub resource_sample_count: u64,
    /// The static GPU/driver identity captured once at startup (MT-086), if a wgpu adapter was present.
    /// `None` in a headless shell. The human strings here are field-standard HARDWARE identity (panel
    /// only), never pushed into the typed ring.
    pub gpu_info: Option<GpuInfo>,
    /// Count of events shed because the in-process buffer hit cap (MT-082). Surfaced so a dropped event
    /// is visible, not silently lost.
    pub dropped_count: u64,
    /// Whether the MT-081 ring writer is installed (Palmistry can see events out-of-process). When
    /// false, diagnostics are in-process-only this session (graceful degradation) — surfaced honestly.
    pub ring_writer_installed: bool,
    /// WP-KERNEL-012 MT-093 (§6.13.7 / §10.12.5 Tier-3): the freeze/crash survivor records the external
    /// Palmistry watcher persisted + (on recovery) forwarded, read by the shell from the durable survivor
    /// store via [`crate::diagnostics::read_default_survivor_records`]. Empty before any freeze/crash (the
    /// honest empty-state MT-087 renders); POPULATED post-recovery (AC-013-6). Typed-allowlist only — no
    /// project content.
    pub palmistry_records: Vec<crate::diagnostics::PalmistrySurvivorView>,
}

/// The in-app Diagnostics Panel widget (§5.8.4). STATELESS beyond the read-only [`DiagnosticsView`]
/// passed to [`show`](DiagnosticsPanel::show): it owns no diagnostics data of its own (RISK-007-2). A
/// unit struct so the caller can construct it freely; all live data flows in through `show`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiagnosticsPanel;

impl DiagnosticsPanel {
    /// Render the whole diagnostics surface into `ui`, projecting the live `internal_diagnostics` state.
    ///
    /// `view` carries the per-frame heartbeat/frame/resource/GPU snapshot the shell built from the live
    /// producers; the last-N events are read DIRECTLY from the process-global recorder here (the true
    /// projection — no copy held by the panel). `palette` supplies every colour (theme tokens only —
    /// CONTROL-4). The whole container is a `Role::Region` with author_id `diagnostics_panel`; each
    /// section is a `Role::Group`.
    pub fn show(&self, ui: &mut egui::Ui, view: &DiagnosticsView, palette: &HsPalette) {
        // The panel container: a Role::Region the whole surface lives under. We allocate a real group
        // scope so the child sections attach beneath it in the accessibility tree, then tag that
        // group's id with the Region role + the stable author_id.
        let region = ui.scope(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Diagnostics")
                        .heading()
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(
                        "Live internal_diagnostics (Tier 2). Projection only — no state stored here.",
                    )
                    .small()
                    .color(palette.text_subtle),
                );
                ui.add_space(6.0);

                self.heartbeat_section(ui, view, palette);
                ui.add_space(8.0);
                self.frame_section(ui, view, palette);
                ui.add_space(8.0);
                self.resource_section(ui, view, palette);
                ui.add_space(8.0);
                self.events_section(ui, palette);
                ui.add_space(8.0);
                self.palmistry_section(ui, view, palette);
            });
        });
        set_region(ui, region.response.id, DIAGNOSTICS_PANEL_AUTHOR_ID, "Diagnostics panel");
    }

    /// Heartbeat section (MT-084): the live counter + time since process start. A non-zero, advancing
    /// counter is the liveness signal (AC-007-2). `Role::Group`, author_id `diagnostics_heartbeat`.
    fn heartbeat_section(&self, ui: &mut egui::Ui, view: &DiagnosticsView, palette: &HsPalette) {
        let group = ui.scope(|ui| {
            section_heading(ui, "Heartbeat", palette);
            // A non-zero counter is the "alive" signal; colour it with the success/error severity token
            // so the operator + a model can read liveness at a glance (theme tokens only).
            let alive = view.heartbeat_counter > 0;
            let beat_color = if alive { palette.accent } else { palette.diagnostics.error };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("UI-thread beats").color(palette.text_subtle));
                ui.label(
                    egui::RichText::new(view.heartbeat_counter.to_string())
                        .monospace()
                        .color(beat_color),
                );
                ui.label(
                    egui::RichText::new(if alive { "(alive)" } else { "(no frames yet)" })
                        .small()
                        .color(palette.text_subtle),
                );
            });
            kv_row(
                ui,
                "Uptime",
                &format_nanos_human(view.heartbeat_elapsed_nanos),
                palette,
            );
        });
        set_group(ui, group.response.id, DIAGNOSTICS_HEARTBEAT_AUTHOR_ID, "Heartbeat");
    }

    /// Frame-time section (MT-085): last / p50 / p95 (+ min/max + slow-emit count), formatted ms.
    /// `Role::Group`, author_id `diagnostics_frame`.
    fn frame_section(&self, ui: &mut egui::Ui, view: &DiagnosticsView, palette: &HsPalette) {
        let s = view.frame_stats;
        let group = ui.scope(|ui| {
            section_heading(ui, "Frame time", palette);
            if s.frame_count == 0 {
                muted_empty(ui, "No frames recorded yet.", palette);
            } else {
                kv_row(ui, "Last", &format_micros_ms(s.last_micros), palette);
                kv_row(ui, "p50", &format_micros_ms(s.p50_micros), palette);
                kv_row(ui, "p95", &format_micros_ms(s.p95_micros), palette);
                kv_row(
                    ui,
                    "min / max",
                    &format!("{} / {}", format_micros_ms(s.min_micros), format_micros_ms(s.max_micros)),
                    palette,
                );
                kv_row(ui, "Frames", &s.frame_count.to_string(), palette);
                kv_row(ui, "Slow frames flagged", &s.slow_emit_count.to_string(), palette);
            }
        });
        set_group(ui, group.response.id, DIAGNOSTICS_FRAME_AUTHOR_ID, "Frame time");
    }

    /// Resource section (MT-086): CPU% + RSS from the last `ResourceSample` + the GPU hardware line.
    /// `Role::Group`, author_id `diagnostics_resource`.
    fn resource_section(&self, ui: &mut egui::Ui, view: &DiagnosticsView, palette: &HsPalette) {
        let group = ui.scope(|ui| {
            section_heading(ui, "Resources", palette);
            match view.last_resource_sample {
                Some(sample) => {
                    kv_row(ui, "CPU", &format_cpu_milli(sample.cpu_milli), palette);
                    kv_row(ui, "RSS", &format_rss_kb(sample.rss_kb), palette);
                }
                None => muted_empty(ui, "No resource sample yet (samples every ~1s).", palette),
            }
            kv_row(ui, "Samples taken", &view.resource_sample_count.to_string(), palette);

            ui.add_space(2.0);
            match &view.gpu_info {
                Some(gpu) if gpu.is_captured() => {
                    kv_row(ui, "GPU", &gpu.name, palette);
                    let driver = if gpu.driver_info.is_empty() {
                        gpu.driver.clone()
                    } else {
                        format!("{} {}", gpu.driver, gpu.driver_info)
                    };
                    if !driver.trim().is_empty() {
                        kv_row(ui, "Driver", driver.trim(), palette);
                    }
                    kv_row(ui, "Backend", backend_label(gpu.backend_code), palette);
                }
                _ => muted_empty(ui, "GPU identity unavailable (headless render).", palette),
            }
        });
        set_group(ui, group.response.id, DIAGNOSTICS_RESOURCE_AUTHOR_ID, "Resources");
    }

    /// Last-N events section (MT-082): a scrolling list of the most-recent typed events, read DIRECTLY
    /// from the process-global recorder each frame (no copy held — the projection invariant). Each row
    /// renders the typed fields (event label + phase + severity + counters + timestamp); there is NO
    /// free text (DiagEvent is typed integers — event_code maps to a static label).
    /// `Role::Group`, author_id `diagnostics_events`.
    fn events_section(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        // True projection: read the live ring every frame (RISK-007-2). The panel keeps no copy.
        let events = crate::diagnostics::snapshot_last_n(PANEL_EVENT_WINDOW);
        let dropped = crate::diagnostics::dropped_count();

        let group = ui.scope(|ui| {
            ui.horizontal(|ui| {
                section_heading(ui, "Recent events", palette);
                ui.label(
                    egui::RichText::new(format!("({})", events.len()))
                        .small()
                        .color(palette.text_subtle),
                );
                if dropped > 0 {
                    ui.label(
                        egui::RichText::new(format!("· {dropped} dropped"))
                            .small()
                            .color(palette.diagnostics.warning),
                    );
                }
            });

            if events.is_empty() {
                muted_empty(ui, "No events recorded yet.", palette);
                return;
            }

            // Newest-first so the most recent event is at the top of the scroll list.
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for event in events.iter().rev() {
                        event_row(ui, event, palette);
                    }
                });
        });
        set_group(ui, group.response.id, DIAGNOSTICS_EVENTS_AUTHOR_ID, "Recent events");
    }

    /// Tier-3 Palmistry section (§10.12.5): projects the freeze/crash survivor records the external
    /// Palmistry watcher persisted + forwarded (MT-093 §6.13.7). When no records exist it renders the
    /// honest empty-state MT-087 established (NOT faked, NOT a spinner — AC-007-4/5); when records exist
    /// (POST-RECOVERY) it lists each typed record (AC-013-6). `Role::Group`, author_id
    /// `diagnostics_palmistry`.
    fn palmistry_section(&self, ui: &mut egui::Ui, view: &DiagnosticsView, palette: &HsPalette) {
        let group = ui.scope(|ui| {
            section_heading(ui, "Palmistry (Tier 3 — external watcher)", palette);
            if view.palmistry_records.is_empty() {
                // Honest empty-state (no freeze/crash this session, or none forwarded yet).
                muted_empty(ui, "No freeze/crash records.", palette);
            } else {
                // POPULATED post-recovery (AC-013-6): one row per forwarded/known survivor record. Every
                // value is typed (kind, codes, durations, exit code, LOCAL minidump path, timestamp) — no
                // project content. The forwarded flag shows whether it rejoined the Flight Recorder ledger.
                for rec in &view.palmistry_records {
                    palmistry_record_row(ui, rec, palette);
                }
            }
            // Honest status of the out-of-process visibility path so the operator knows whether the
            // external watcher could even see events this session (graceful-degradation transparency).
            let ring_status = if view.ring_writer_installed {
                "Shared-memory ring active (out-of-process visible)."
            } else {
                "In-process only this session (no ring writer)."
            };
            ui.label(
                egui::RichText::new(ring_status)
                    .small()
                    .color(palette.text_subtle),
            );
        });
        set_group(ui, group.response.id, DIAGNOSTICS_PALMISTRY_AUTHOR_ID, "Palmistry");
    }
}

/// One Tier-3 survivor-record row (MT-093 §10.12.5): the typed kind + the typed evidence (stale duration
/// for a freeze, exit code + LOCAL minidump path for a crash) + whether it has been forwarded to the
/// Flight Recorder ledger. NO free text (the record carries none). Severity colour from `palette` tokens.
fn palmistry_record_row(
    ui: &mut egui::Ui,
    rec: &crate::diagnostics::PalmistrySurvivorView,
    palette: &HsPalette,
) {
    use crate::diagnostics::PalmistrySurvivorKind;
    ui.horizontal(|ui| {
        // The kind, coloured by severity (a crash is an error tone, a freeze a warn tone).
        let kind_color = match rec.kind {
            PalmistrySurvivorKind::Crash => palette.diagnostics.error,
            PalmistrySurvivorKind::Freeze => palette.diagnostics.warning,
            PalmistrySurvivorKind::Other => palette.text_subtle,
        };
        ui.label(egui::RichText::new(rec.kind.label()).strong().color(kind_color));
        // The typed evidence specific to the kind.
        match rec.kind {
            PalmistrySurvivorKind::Freeze => {
                ui.label(
                    egui::RichText::new(format!("stale {}ms", rec.stale_ms))
                        .monospace()
                        .color(palette.text),
                );
            }
            _ => {
                if let Some(code) = rec.exit_code {
                    ui.label(
                        egui::RichText::new(format!("exit 0x{code:X}"))
                            .monospace()
                            .color(palette.text),
                    );
                }
            }
        }
        // The forwarded-to-ledger flag (the §6.13.7 recovery-rejoin signal).
        let (fwd_text, fwd_color) = if rec.forwarded {
            ("forwarded", palette.accent)
        } else {
            ("pending", palette.text_subtle)
        };
        ui.label(egui::RichText::new(fwd_text).small().color(fwd_color));
    });
    // A crash's LOCAL minidump path (a local reference only — never the bytes).
    if let Some(path) = &rec.minidump_path {
        ui.label(
            egui::RichText::new(format!("minidump: {path}"))
                .small()
                .monospace()
                .color(palette.text_subtle),
        );
    }
}

// ── Row + formatting helpers (theme tokens only) ──────────────────────────────────────────────────

/// A section heading line in the subtle-strong token so the sections read as distinct groups.
fn section_heading(ui: &mut egui::Ui, text: &str, palette: &HsPalette) {
    ui.label(egui::RichText::new(text).strong().color(palette.text));
}

/// A `label : value` row — the label in the subtle token, the value monospace in the primary text
/// token so numbers line up and read clearly.
fn kv_row(ui: &mut egui::Ui, key: &str, value: &str, palette: &HsPalette) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{key}:")).color(palette.text_subtle));
        ui.label(egui::RichText::new(value).monospace().color(palette.text));
    });
}

/// A muted empty-state line (an honest "nothing here yet", not a spinner — AC-007-5 forbids a
/// perpetual spinner).
fn muted_empty(ui: &mut egui::Ui, text: &str, palette: &HsPalette) {
    ui.label(egui::RichText::new(text).color(palette.text_subtle));
}

/// One typed event row: a severity-coloured dot label + the static event-code label + phase + the
/// numeric counters + the monotonic timestamp. NO free text (the event has none). The severity colour
/// comes from `palette.diagnostics` (theme tokens — CONTROL-4).
fn event_row(ui: &mut egui::Ui, event: &DiagEvent, palette: &HsPalette) {
    let severity_color = severity_color(event.severity, palette);
    ui.horizontal(|ui| {
        // Severity glyph in the severity token (a filled marker, not a spinner).
        ui.label(egui::RichText::new("●").monospace().color(severity_color));
        ui.label(
            egui::RichText::new(event_code_label(event.event_code))
                .monospace()
                .color(palette.text),
        );
        ui.label(
            egui::RichText::new(phase_label(event.phase_marker))
                .small()
                .color(palette.text_subtle),
        );
        // The numeric payload (typed integers only) so the row carries the real event data.
        ui.label(
            egui::RichText::new(format!(
                "a={} b={} {}",
                event.counter_a,
                event.counter_b,
                format_micros_ms(event.metric_micros),
            ))
            .small()
            .monospace()
            .color(palette.text_subtle),
        );
        ui.label(
            egui::RichText::new(format!("@{}", format_nanos_human(event.timestamp_nanos)))
                .small()
                .monospace()
                .color(palette.text_subtle),
        );
    });
}

/// Map a `DiagEvent.severity` (`u8`) to its `palette.diagnostics` token. Unknown values fall back to
/// the info token (never panics, never a literal).
fn severity_color(severity: u8, palette: &HsPalette) -> egui::Color32 {
    if severity == DiagSeverity::Error.as_u8() {
        palette.diagnostics.error
    } else if severity == DiagSeverity::Warn.as_u8() {
        palette.diagnostics.warning
    } else {
        palette.diagnostics.info
    }
}

/// Map a `DiagEvent.event_code` (`u16`, a closed [`DiagEventCode`] discriminant) to a STATIC human
/// label for display (§5.8.3 — the typed-allowlist; there is no free text to render). An unknown code
/// (a forward-compat event from a newer producer) renders as `event(<code>)` rather than panicking.
pub fn event_code_label(code: u16) -> String {
    let named = |c: DiagEventCode| c.as_u16() == code;
    if named(DiagEventCode::Heartbeat) {
        "Heartbeat".to_owned()
    } else if named(DiagEventCode::PanicCaught) {
        "PanicCaught".to_owned()
    } else if named(DiagEventCode::SlowFrame) {
        "SlowFrame".to_owned()
    } else if named(DiagEventCode::ResourceSample) {
        "ResourceSample".to_owned()
    } else if named(DiagEventCode::BackendUnreachable) {
        "BackendUnreachable".to_owned()
    } else if named(DiagEventCode::BackendRecovered) {
        "BackendRecovered".to_owned()
    } else if named(DiagEventCode::PaneMounted) {
        "PaneMounted".to_owned()
    } else if named(DiagEventCode::FreezeSuspected) {
        "FreezeSuspected".to_owned()
    } else if named(DiagEventCode::CrashDetected) {
        "CrashDetected".to_owned()
    } else if named(DiagEventCode::PalmistryHandshake) {
        "PalmistryHandshake".to_owned()
    } else if named(DiagEventCode::Shutdown) {
        "Shutdown".to_owned()
    } else if named(DiagEventCode::Other) {
        "Other".to_owned()
    } else {
        format!("event({code})")
    }
}

/// Map a `DiagEvent.phase_marker` (`u8`) to a short static label.
fn phase_label(phase: u8) -> &'static str {
    use handshake_diag_ring::DiagPhase;
    if phase == DiagPhase::Start.as_u8() {
        "start"
    } else if phase == DiagPhase::Tick.as_u8() {
        "tick"
    } else if phase == DiagPhase::End.as_u8() {
        "end"
    } else if phase == DiagPhase::Recovered.as_u8() {
        "recovered"
    } else if phase == DiagPhase::Degraded.as_u8() {
        "degraded"
    } else {
        "?"
    }
}

/// Map a `GpuInfo.backend_code` (the wgpu `Backend` `repr(u8)` discriminant) to a short static label.
fn backend_label(code: u8) -> &'static str {
    match code {
        0 => "Noop",
        1 => "Vulkan",
        2 => "Metal",
        3 => "Dx12",
        4 => "OpenGL",
        5 => "WebGPU",
        _ => "Other",
    }
}

/// Format a micros duration as a human ms string (e.g. `16.7 ms`). 0 renders `0.0 ms`.
fn format_micros_ms(micros: u64) -> String {
    format!("{:.1} ms", micros as f64 / 1000.0)
}

/// Format CPU milli-percent (percent * 1000) as a human percent string (e.g. `12.5%`).
fn format_cpu_milli(cpu_milli: u64) -> String {
    format!("{:.1}%", cpu_milli as f64 / 1000.0)
}

/// Format an RSS in KiB as a human MiB string for readability (e.g. `184.2 MiB`).
fn format_rss_kb(rss_kb: u64) -> String {
    format!("{:.1} MiB", rss_kb as f64 / 1024.0)
}

/// Format a monotonic nanosecond value as a human uptime/elapsed string (`12.345 s`). The values are
/// process-relative monotonic nanos, so this is "seconds since process start".
fn format_nanos_human(nanos: u64) -> String {
    format!("{:.3} s", nanos as f64 / 1_000_000_000.0)
}

// ── AccessKit helpers (own the container node: role + author_id + label) ──────────────────────────

/// Tag an already-allocated group scope's id as a `Role::Region` with a stable author_id + label (the
/// whole panel container). Mirrors `accessibility::live::emit_pane_node` (own the container node).
fn set_region(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Region);
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

/// Tag an already-allocated group scope's id as a `Role::Group` with a stable author_id + label (a
/// child section). Non-interactive role, so it never trips the unnamed-interactive gate.
fn set_group(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

#[cfg(test)]
mod tests {
    //! Pure-unit tests for the label/format mapping helpers (no app, no egui). The live-render +
    //! AccessKit-subtree + screenshot proofs live in `tests/test_diagnostics_panel.rs` because they
    //! drive the real `HandshakeApp` through egui_kittest.

    use super::*;

    #[test]
    fn event_code_label_covers_every_named_code() {
        // Each named DiagEventCode maps to its own non-fallback label.
        for (code, expected) in [
            (DiagEventCode::Heartbeat, "Heartbeat"),
            (DiagEventCode::PanicCaught, "PanicCaught"),
            (DiagEventCode::SlowFrame, "SlowFrame"),
            (DiagEventCode::ResourceSample, "ResourceSample"),
            (DiagEventCode::BackendUnreachable, "BackendUnreachable"),
            (DiagEventCode::BackendRecovered, "BackendRecovered"),
            (DiagEventCode::PaneMounted, "PaneMounted"),
            (DiagEventCode::FreezeSuspected, "FreezeSuspected"),
            (DiagEventCode::CrashDetected, "CrashDetected"),
            (DiagEventCode::PalmistryHandshake, "PalmistryHandshake"),
            (DiagEventCode::Shutdown, "Shutdown"),
            (DiagEventCode::Other, "Other"),
        ] {
            assert_eq!(event_code_label(code.as_u16()), expected);
        }
    }

    #[test]
    fn unknown_event_code_falls_back_without_panic() {
        // A code not in the closed enum (a forward-compat event) renders a generic label, never panics.
        // 4242 is not a current DiagEventCode discriminant.
        assert_eq!(event_code_label(4242), "event(4242)");
    }

    #[test]
    fn phase_label_covers_every_phase() {
        use handshake_diag_ring::DiagPhase;
        assert_eq!(phase_label(DiagPhase::Start.as_u8()), "start");
        assert_eq!(phase_label(DiagPhase::Tick.as_u8()), "tick");
        assert_eq!(phase_label(DiagPhase::End.as_u8()), "end");
        assert_eq!(phase_label(DiagPhase::Recovered.as_u8()), "recovered");
        assert_eq!(phase_label(DiagPhase::Degraded.as_u8()), "degraded");
        assert_eq!(phase_label(250), "?");
    }

    #[test]
    fn backend_label_maps_known_codes() {
        assert_eq!(backend_label(1), "Vulkan");
        assert_eq!(backend_label(3), "Dx12");
        assert_eq!(backend_label(4), "OpenGL");
        assert_eq!(backend_label(99), "Other");
    }

    #[test]
    fn formatters_produce_human_strings() {
        assert_eq!(format_micros_ms(16_700), "16.7 ms");
        assert_eq!(format_micros_ms(0), "0.0 ms");
        assert_eq!(format_cpu_milli(12_500), "12.5%");
        assert_eq!(format_rss_kb(188_620), "184.2 MiB");
        assert_eq!(format_nanos_human(12_345_000_000), "12.345 s");
    }

    #[test]
    fn author_ids_are_the_mt_named_stable_ids() {
        // AC-007-1 / AC-007-5 name these EXACTLY.
        assert_eq!(DIAGNOSTICS_PANEL_AUTHOR_ID, "diagnostics_panel");
        assert_eq!(DIAGNOSTICS_HEARTBEAT_AUTHOR_ID, "diagnostics_heartbeat");
        assert_eq!(DIAGNOSTICS_EVENTS_AUTHOR_ID, "diagnostics_events");
    }
}
