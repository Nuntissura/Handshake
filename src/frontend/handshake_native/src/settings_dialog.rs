//! Settings / Options dialog for the native Handshake shell (WP-KERNEL-011 MT-018).
//!
//! ## What this provides (no-context model navigation — HBR-VIS / HBR-SWARM)
//!
//! A modal settings dialog (a centred, always-on-top floating panel with a hidden title bar) opened
//! from HELP > Open Settings… (MT-015 menu), the command palette action `settings.open` (MT-016), or a
//! test/agent setting `app_state.settings_open = true`. It is a port of
//! `app/src/components/SettingsMenu.tsx` over the [`crate::workspace_settings`] schema + helpers, with
//! these NINE sections in order: Appearance (theme + view mode — both WIRED), Keybindings (editable,
//! with live conflict detection), Swarm (wired default-open checkboxes, two backend-fixed interval rows
//! that state WHY they cannot be configured, and the WIRED per-frame swarm admission budget),
//! Terminal (not-yet-wired rows), Layout (a wired Reset panes & drawers button), Cloud Models (BYOK +
//! CLI-bridge login, plus the per-lane cloud consent / export posture — currently an EXPLICIT not-wired
//! state, never a fabricated posture), Model Runtime (production-pane navigation), Diagnostics (the
//! wired resource-sampling toggle + real Palmistry/internal-diagnostics status), and About (app name +
//! the real Cargo version).
//!
//! ## Truthfulness rule for un-wired state (WP-1 MT-021)
//!
//! A value Handshake cannot actually read renders as an EXPLICIT unavailable state that also says WHY.
//! It never renders as a plausible default. This applies to the cloud consent / export posture (no
//! backend route exists) and to the two swarm interval rows (backend-owned cadence, no route). A
//! settings control also never grants or widens authority: the admission budget can only TIGHTEN the
//! compiled-in flood ceiling, and nothing in the consent block is interactive at all.
//!
//! ## Swarm interaction contract (HBR-SWARM)
//!
//! A swarm agent drives this dialog out-of-process without screen scraping:
//! 1. set `app_state.settings_open = true` (HELP menu / palette `settings.open` / direct flag) to open;
//! 2. read the current theme from `app_state.workspace_settings().theme` (or the `Theme / appearance`
//!    ComboBox node, author_id [`THEME_COMBO_AUTHOR_ID`]);
//! 3. toggle it via the ComboBox (the dialog returns [`SettingsOutcome::ThemeChanged`], which the shell
//!    applies to `current_theme` + persists via `PUT /workspaces/{id}/settings`);
//! 4. verify the change propagated by re-reading `app_state.current_theme()` /
//!    `app_state.workspace_settings().theme`.
//!
//! Every interactive control carries a stable AccessKit role + author_id (see the consts below and the
//! per-control `author_id` strings) so the agent addresses them deterministically.
//!
//! ## Ownership split (mirrors the MT-016 command palette + MT-015 menu bar)
//!
//! The dialog owns ONLY its transient UI state (the search query + the in-progress *draft* keybinding
//! text per action), stored in egui persistent memory keyed to the dialog id and RESET when the dialog
//! re-opens (keyed by a monotonic `open_count`, like the palette). It NEVER mutates app state: [`show`]
//! takes a read-only [`SettingsView`] (the live settings + open generation) and returns a
//! [`SettingsOutcome`] the shell ([`crate::app`]) matches on to mutate `workspace_settings` /
//! `current_theme` / `view_mode` and persist. The shell owns the `settings_open` flag; the dialog only
//! REQUESTS close via [`SettingsOutcome::Close`].
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! Three FIXED container nodes in a fresh disjoint band (17..=19, directly above the quick-switcher
//! band 14..=16, below the chrome title bar 20 and the pane id base 100):
//! - the dialog root ([`SETTINGS_DIALOG_NODE_ID`] = 17, Role::Dialog, modal),
//! - the search box ([`SETTINGS_SEARCH_NODE_ID`] = 18, Role::TextInput),
//! - the body/list region ([`SETTINGS_LIST_NODE_ID`] = 19, Role::Group).
//!
//! Every interactive CONTROL (theme combo, view-mode combo, per-action keybinding inputs + reset
//! buttons, the swarm-board checkbox, the reset-layout button, the close button) is rendered with a
//! stable author_id STRING (in egui's hashed id space, the same convention as the palette command rows
//! and the per-tab nodes), so the count can vary with the search filter without bloating the fixed
//! band, while every control stays discoverable/clickable out-of-process and never trips the MT-025
//! interactive-naming gate. The three fixed container ids ARE enumerated in `DECLARED_IDENTITIES`.
//!
//! The dialog renders ONLY while `settings_open` is true (closed by default), so the default-seed live
//! tree never contains any of these nodes — exactly like the palette / switcher overlays.
//!
//! ## Two hosts: root-viewport modal and detached OS window (MT-015)
//!
//! The SAME surface renders in one of two mutually exclusive hosts, never both:
//!
//! | Host | Entry | Root node | Argus window |
//! |------|-------|-----------|--------------|
//! | modal (default) | `settings_open = true` | `settings.dialog` (`Role::Dialog`, modal) | `main` |
//! | detached window | `settings.popout` -> [`SettingsOutcome::PopOut`] | `popout-window-settings` (`Role::Window`) | `popout-settings` |
//!
//! [`show`] draws the modal; [`show_detached`] draws the detached window's body. Both call the SAME
//! [`render_search_and_sections`] -> [`render_sections`] path over the same shell-owned state, so every
//! section, control, and author_id is identical in both hosts — only the surrounding chrome differs.
//! `settings.redock` returns the surface to the modal; the detached window's Close control and its OS
//! close button close settings outright (a later re-open comes back as the modal).

use egui::accesskit;

use crate::workspace_settings::{
    find_keybinding_conflicts, keybinding_label_for_conflict, normalize_chord_input,
    setting_matches_query, Keybinding, NotYetWiredSetting, SettingsViewMode,
    WorkspaceSettingsState, WorkspaceTheme, ABOUT_APP_NAME, ABOUT_VERSION, APP_KEYBINDING_ACTIONS,
    SWARM_RECONCILE_INTERVAL_SETTING, SWARM_RESOURCE_POLL_INTERVAL_SETTING,
    TERMINAL_DEFAULT_SHELL_SETTING, TERMINAL_MAX_SCROLLBACK_SETTING,
    TERMINAL_OUTPUT_LOGGING_SETTING,
};

/// Fixed AccessKit/egui `NodeId` of the settings DIALOG root (Role::Dialog, modal). Fresh band slot 17:
/// directly above the quick-switcher band (14..=16), below the chrome title bar (20) and the pane id
/// base (100). A fixed-value `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames
/// + restarts — the same convention every other fixed-band node in this crate uses.
pub const SETTINGS_DIALOG_NODE_ID: u64 = 17;
/// Fixed AccessKit/egui `NodeId` of the settings SEARCH box (Role::TextInput). Fresh band slot 18.
pub const SETTINGS_SEARCH_NODE_ID: u64 = 18;
/// Fixed AccessKit/egui `NodeId` of the settings BODY/list region (Role::Group). Fresh band slot 19.
pub const SETTINGS_LIST_NODE_ID: u64 = 19;

/// Stable out-of-process author_id for the settings dialog root.
pub const SETTINGS_DIALOG_AUTHOR_ID: &str = "settings.dialog";
/// Stable out-of-process author_id for the settings search box.
pub const SETTINGS_SEARCH_AUTHOR_ID: &str = "settings.search";
/// Stable out-of-process author_id for the settings body/list region.
pub const SETTINGS_LIST_AUTHOR_ID: &str = "settings.list";

/// Stable author_id for the Theme / appearance ComboBox.
pub const THEME_COMBO_AUTHOR_ID: &str = "settings.theme";
/// Stable author_id for the View Mode ComboBox.
pub const VIEW_MODE_COMBO_AUTHOR_ID: &str = "settings.view-mode";
/// Stable author_id for the Swarm board default-open checkbox.
pub const SWARM_BOARD_CHECKBOX_AUTHOR_ID: &str = "settings.swarm-board-default-open";
/// Stable author_id for the Swarm lane diagnostics default-open checkbox.
pub const SWARM_LANE_DIAGNOSTICS_CHECKBOX_AUTHOR_ID: &str =
    "settings.swarm-lane-diagnostics-default-open";
/// Stable author_id for the Operator Chat default-open checkbox.
pub const SWARM_OPERATOR_CHAT_CHECKBOX_AUTHOR_ID: &str =
    "settings.swarm-operator-chat-default-open";
/// WP-1 MT-021 (AC-3): stable author_id for the per-frame SWARM ADMISSION BUDGET ComboBox — the number
/// of queued swarm/Argus actions the shell admits into one egui frame when N agents drive it
/// concurrently. This is a REAL control: the selected value is clamped, persisted through workspace
/// settings, and pushed into the live [`crate::mcp::ActionChannel`] the running MCP/Argus transport
/// drains, so the very next frame honours it.
pub const SWARM_MAX_ACTIONS_COMBO_AUTHOR_ID: &str = "settings.swarm-max-actions-per-frame";
/// WP-1 MT-021: stable author_id for the distinct SwarmCoordinator model-session cap. Unlike the
/// per-frame action budget above, this control calls the coordinator's GET/PUT authority and displays
/// both requested and currently-in-force values during cooperative lowering.
pub const SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID: &str =
    "settings.swarm-model-sessions-max-concurrent";
/// Stable status node for the coordinator values returned by the backend.
pub const SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID: &str =
    "settings.swarm-model-sessions-max-concurrent.status";
/// Stable author_id for the Reset panes & drawers button.
pub const RESET_LAYOUT_AUTHOR_ID: &str = "settings.reset-layout";
/// Stable author_id for opening the production Model Runtime registry pane.
pub const OPEN_MODEL_RUNTIME_AUTHOR_ID: &str = "settings.model-runtime.open";
/// Stable author_id for opening the canonical Problems/internal-diagnostics pane.
pub const OPEN_PROBLEMS_AUTHOR_ID: &str = "settings.model-runtime.open-problems";
/// WP-1 (a): stable author_id for the deep-link that opens the Operator Chat / Launch pane — the real
/// surface where the operator selects the completion model + lane before launch. Settings does not
/// duplicate that selection logic; it deep-links to the single source of truth.
pub const OPEN_OPERATOR_CHAT_AUTHOR_ID: &str = "settings.model-runtime.open-operator-chat";
/// WP-1 (b): stable author_id for the background resource-sampling enable checkbox. Toggling it drives
/// the real internal_diagnostics resource sampler (pause/resume), not a cosmetic flag.
pub const RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID: &str =
    "settings.diagnostics.resource-sampling-enabled";
/// WP-1 (c): stable author_id for the Palmistry watcher status label (display backed by real state).
pub const PALMISTRY_STATUS_AUTHOR_ID: &str = "settings.diagnostics.palmistry-status";
/// WP-1 (c): stable author_id for the internal-diagnostics subsystem status label (real state).
pub const DIAGNOSTICS_SUBSYSTEM_STATUS_AUTHOR_ID: &str = "settings.diagnostics.subsystem-status";
/// Stable author_id for the Close button. Rendered in BOTH the root-viewport modal header and the
/// detached-window header (the same control in two hosts; Argus scopes targets per window).
pub const CLOSE_AUTHOR_ID: &str = "settings.close";
/// Stable author_id for the modal header's "Pop out" control: moves the whole settings surface into its
/// OWN OS window ([`show_detached`]). The shell flips its detached flag on the returned
/// [`SettingsOutcome::PopOut`], and from the next frame the modal no longer renders — the two hosts are
/// mutually exclusive, so there is never a double settings UI.
pub const SETTINGS_POPOUT_AUTHOR_ID: &str = "settings.popout";
/// Stable author_id for the detached window's "Re-dock" control: returns the surface to the
/// root-viewport modal WITHOUT closing settings ([`SettingsOutcome::Redock`]).
pub const SETTINGS_REDOCK_AUTHOR_ID: &str = "settings.redock";
/// The pop-out KEY the detached Settings window feeds to the generic pane pop-out id scheme in
/// [`crate::popout_window`], so the detached Settings window is addressed by exactly the same scheme as
/// every pane pop-out:
/// - Argus window id `popout-settings` ([`crate::popout_window::argus_window_id`]) — what
///   `argus.list_windows` enumerates and what `screenshot` targets by recorded OS window handle;
/// - root AccessKit node `popout-window-settings`
///   ([`crate::popout_window::popout_window_author_id`], `Role::Window`).
///
/// Settings is NOT a pane; this key only feeds those two pure id formatters (and the stable
/// `ViewportId`), it never enters the pane registry.
pub const SETTINGS_POPOUT_KEY: &str = "settings";
/// The surface label the detached window title is built from:
/// `"Handshake – Settings"` via [`crate::popout_window::popout_title_for`].
pub const SETTINGS_WINDOW_LABEL: &str = "Settings";
/// Author_id prefix for a per-action keybinding text input (`{prefix}{action_id}`).
pub const KEYBINDING_INPUT_AUTHOR_ID_PREFIX: &str = "settings.keybinding.";
/// Author_id prefix for a per-action keybinding Reset button (`{prefix}{action_id}`).
pub const KEYBINDING_RESET_AUTHOR_ID_PREFIX: &str = "settings.keybinding-reset.";
/// Author_id prefix for a not-yet-wired row's disabled control (`{prefix}{setting_id}`).
pub const NOT_WIRED_AUTHOR_ID_PREFIX: &str = "settings.not-wired.";
/// Author_id prefix for a settings SECTION collapsing-header button (`{prefix}{section_key}`). Each
/// section header (Appearance / Keybindings / Swarm / Terminal / Layout / Cloud Models / Model
/// Runtime / About) renders as an
/// interactive `Role::Button` in egui's hashed id space; tagging it gives an out-of-process model a
/// stable handle to expand/collapse each section. Without it the header is an interactive control with
/// no stable address — the gap the MT-029 overlay accessibility-invariant proof surfaces.
pub const SECTION_HEADER_AUTHOR_ID_PREFIX: &str = "settings.section.";

// ── Cloud Models section (MT-015: operator cloud-model access config) ──────────────────────────────
//
// Per-provider BYOK API-key entry (Anthropic, OpenAI) + subscription-plan CLI-bridge login status
// (Claude Code, GPT/Codex). Gemini is never rendered (the backend enumeration never lists it). Every
// control carries a stable per-provider author_id so an out-of-process model (Argus) addresses each
// provider row/field/button deterministically. author_ids are BUILT from the provider id so the set
// stays consistent as providers are added; the stability test pins the exact strings.

/// Author_id for a BYOK provider's password-masked API-key input: `settings.cloud.byok.{provider}.key`.
pub fn cloud_byok_key_author_id(provider: &str) -> String {
    format!("settings.cloud.byok.{provider}.key")
}
/// The STABLE egui widget id for a BYOK provider's key `TextEdit`. Fixed (not
/// auto-derived from the ui id-stack) so the dialog can RESET that widget's egui
/// state (cursor + undo history) on close/open, wiping any typed-but-unsaved key
/// from egui memory (MT-015 F3). The undo history would otherwise retain a
/// plaintext copy of the key in process memory across a reopen.
pub fn cloud_byok_key_egui_id(provider: &str) -> egui::Id {
    egui::Id::new(("settings.cloud.byok.key", provider))
}

/// The static BYOK providers Handshake offers, used to render key-entry rows
/// even before (or entirely without) a backend enumeration response (MT-015
/// F10). Mirrors the backend `ByokProvider::OFFERED` ids/labels; Gemini is never
/// present. When these seed rows are used, `configured` is unknown (the backend
/// has not answered), so they render as not-configured with a "status unknown"
/// note — but the key field + Save ALWAYS render so BYOK entry never depends on
/// the backend being reachable.
pub const STATIC_BYOK_PROVIDERS: [(&str, &str); 2] = [
    ("anthropic", "Anthropic (Claude)"),
    ("openai", "OpenAI (GPT)"),
];
/// Author_id for a BYOK provider's Save button: `settings.cloud.byok.{provider}.save`.
pub fn cloud_byok_save_author_id(provider: &str) -> String {
    format!("settings.cloud.byok.{provider}.save")
}
/// Author_id for a BYOK provider's Remove/Rotate button: `settings.cloud.byok.{provider}.remove`.
pub fn cloud_byok_remove_author_id(provider: &str) -> String {
    format!("settings.cloud.byok.{provider}.remove")
}
/// Author_id for a BYOK provider's status label: `settings.cloud.byok.{provider}.status`.
pub fn cloud_byok_status_author_id(provider: &str) -> String {
    format!("settings.cloud.byok.{provider}.status")
}
/// Author_id for a CLI-bridge provider's Log-in button: `settings.cloud.cli.{provider}.login`.
pub fn cloud_cli_login_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login")
}
pub fn cloud_cli_login_confirm_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.confirm")
}
pub fn cloud_cli_login_cancel_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.cancel")
}
/// Author_id for a CLI-bridge provider's status label: `settings.cloud.cli.{provider}.status`.
pub fn cloud_cli_status_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.status")
}

// ── In-app official-CLI login panel (MT-015 v5, HBR-QUIET-001) ─────────────────────────────────────
//
// The login used to be launched into a NEW OS console window that could take focus. It now runs in a
// Handshake-hosted pseudo-terminal in the backend and is driven from THIS panel: the provider's own
// prompt is rendered here and the operator types the device code / answer here. Every control carries
// a stable per-provider author_id so Argus can read the prompt and type the answer out-of-process.

/// Author_id for the live login transcript: `settings.cloud.cli.{provider}.login.transcript`.
pub fn cloud_cli_login_transcript_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.transcript")
}
/// Author_id for the login session status line: `settings.cloud.cli.{provider}.login.state`.
pub fn cloud_cli_login_state_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.state")
}
/// Author_id for the operator's answer field: `settings.cloud.cli.{provider}.login.input`.
pub fn cloud_cli_login_input_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.input")
}
/// Author_id for the Send button: `settings.cloud.cli.{provider}.login.send`.
pub fn cloud_cli_login_send_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.send")
}
/// Author_id for the Stop-login button: `settings.cloud.cli.{provider}.login.stop`.
pub fn cloud_cli_login_stop_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.stop")
}
/// Author_id for the Close-panel button: `settings.cloud.cli.{provider}.login.close`.
pub fn cloud_cli_login_close_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.close")
}
/// The STABLE egui widget id for the login answer field, so its egui state (cursor + undo history)
/// can be reset when the panel closes. The answer is a provider-issued one-time prompt response, not
/// a Handshake credential, but it is still cleared rather than left in widget memory.
pub fn cloud_cli_login_input_egui_id(provider: &str) -> egui::Id {
    egui::Id::new(("settings.cloud.cli.login.input", provider))
}

/// Typed state of the in-app official-CLI login session, mirroring the backend
/// `CliLoginSessionStatus` wire enum. Handshake never guesses a state it cannot prove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudCliLoginState {
    /// The session was started but the provider has not printed anything yet.
    Pending,
    /// The provider has printed output and the login process is still running.
    AwaitingInput,
    Succeeded,
    Failed,
    /// The bounded login window elapsed and the backend terminated the process.
    TimedOut,
    Cancelled,
}

impl CloudCliLoginState {
    pub fn from_wire(value: &str) -> Self {
        match value {
            "pending" => Self::Pending,
            "awaiting_input" => Self::AwaitingInput,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "timed_out" => Self::TimedOut,
            _ => Self::Cancelled,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Starting the provider's login…",
            Self::AwaitingInput => "Waiting for you — read the prompt below and answer it here.",
            Self::Succeeded => "Login finished successfully.",
            Self::Failed => "The provider's login exited with an error.",
            Self::TimedOut => "Login timed out and was stopped.",
            Self::Cancelled => "Login stopped.",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::TimedOut | Self::Cancelled
        )
    }
}

/// Shell-owned state for ONE live in-app login session (at most one at a time).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloudCliLoginPanel {
    pub provider: String,
    pub label: String,
    /// Backend-issued session id; empty until the start request answers.
    pub session_id: String,
    pub state: CloudCliLoginState,
    /// The provider's own terminal output (ANSI already stripped by the backend).
    pub transcript: String,
    /// The operator's in-progress answer to the provider's prompt.
    pub input: String,
    /// Transient transport error, if the last request failed.
    pub error: Option<String>,
}

impl Default for CloudCliLoginState {
    fn default() -> Self {
        Self::Pending
    }
}

// ── Cloud consent / export posture (WP-1 MT-021 AC-2) ──────────────────────────────────────────────
//
// The operator needs to see, per configured provider lane, whether cloud escalation is CONSENTED and
// what leaves the machine — and, when access is refused, why. Handshake cannot show that today:
//
// - the consent artifacts exist as backend TYPES only (`llm/guard.rs` ProjectionPlanV0_4 /
//   ConsentReceiptV0_4 / CloudEscalationBundleV0_4 + migration 0353);
// - `api/mod.rs` builds `CloudLaneObservability { flight_recorder, consent: None }` — the live wiring
//   is a literal `None`;
// - `api/model_access.rs::routes` exposes ONLY `/model-access/providers`,
//   `/model-access/byok/:provider/key`, and `/model-access/cli-bridge/:provider/login`. There is no
//   consent or export-posture route to call.
//
// DECISION (recorded here because this is the code that would consume it): wiring the backend consent
// route is NOT part of this MT. This MT is frontend-only by contract scope, a consent/export-posture
// route is a backend trust-boundary surface that must be designed with its own privacy review
// (HBR-PRIV-008: a denial reason must not leak restricted resource metadata), and inventing a shape
// here would fix a contract the backend has not agreed to. It belongs in a FOLLOW-UP backend MT that
// owns `api/model_access.rs` + `CloudLaneObservability.consent`.
//
// Until then the UI renders an explicit NOT-WIRED posture per lane and MUST NOT fabricate one. A
// plausible-looking default ("Consented", "No export") would be worse than nothing: it would tell the
// operator cloud data flow is under control when Handshake cannot see it at all.

/// Author_id for a provider lane's consent / export posture row:
/// `settings.cloud.consent.{provider}.posture`.
pub fn cloud_consent_posture_author_id(provider: &str) -> String {
    format!("settings.cloud.consent.{provider}.posture")
}

/// Author_id for the Cloud Models section's consent-posture summary row (the one node that states the
/// whole surface's wiring state): `settings.cloud.consent.status`.
pub const CLOUD_CONSENT_STATUS_AUTHOR_ID: &str = "settings.cloud.consent.status";

/// The literal token every un-wired consent row carries, so a test (and an out-of-process model) can
/// assert the surface is showing an explicit unavailable state rather than a fabricated posture.
pub const CLOUD_CONSENT_NOT_WIRED_TOKEN: &str = "NOT WIRED";

/// The exact per-lane posture line rendered while no backend consent route exists. It names ONLY the
/// provider id already visible in the section above it — no project, workspace, account, artifact,
/// resource id, or any other restricted metadata (HBR-PRIV-008). There is deliberately no
/// "posture: allowed/denied" wording anywhere in it: Handshake does not know, and must not imply it.
pub fn cloud_consent_posture_line(provider_label: &str) -> String {
    format!(
        "{CLOUD_CONSENT_NOT_WIRED_TOKEN} — Handshake cannot read {provider_label}'s cloud consent or \
         export posture. No consent/export-posture route exists on the backend yet, so no posture is \
         shown and none is assumed. Cloud escalation stays fail-closed at launch."
    )
}

/// One non-secret BYOK provider row from the backend enumeration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudByokRow {
    pub provider: String,
    pub label: String,
    pub configured: bool,
}

/// One non-secret CLI-bridge provider row from the backend enumeration, carrying the provider's OWN
/// official login command (started operator-initiated in a Handshake-hosted in-app terminal session;
/// no OS console window and no focus change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudCliAuthStatus {
    LoggedIn,
    LoggedOut,
    Expired,
    Unavailable,
}

impl CloudCliAuthStatus {
    pub fn from_wire(value: &str) -> Self {
        match value {
            "logged_in" => Self::LoggedIn,
            "logged_out" => Self::LoggedOut,
            "expired" => Self::Expired,
            _ => Self::Unavailable,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LoggedIn => "Logged in",
            Self::LoggedOut => "Logged out",
            Self::Expired => "Session expired",
            Self::Unavailable => "Status unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudCliRow {
    pub provider: String,
    pub label: String,
    pub auth_status: CloudCliAuthStatus,
    pub login_program: String,
    pub login_args: Vec<String>,
    pub hint: String,
}

/// The non-secret cloud-access enumeration snapshot the dialog renders from. Fetched by the shell from
/// `GET /model-access/providers`; never contains key material.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloudAccessSnapshot {
    pub byok: Vec<CloudByokRow>,
    pub cli_bridge: Vec<CloudCliRow>,
}

/// Mutable, shell-owned UI state for the Cloud Models section.
///
/// SECURITY (MT-015): the per-provider API-key edit buffer is a
/// [`zeroize::Zeroizing<String>`] so it is wiped on clear/drop. This state lives in the shell
/// (`HandshakeApp`), NOT in the dialog's `DialogState` and NOT in the persisted egui snapshot, so a key
/// never reaches the persisted snapshot or the workspace-settings PUT. The buffer is cleared immediately
/// after a Save is dispatched AND on every dialog close (Escape / Close button / backdrop) and on
/// (re-)open; the same close/open paths also reset each key `TextEdit`'s egui state so the widget's undo
/// history never retains a typed-but-unsaved key across a reopen (F3).
#[derive(Default)]
pub struct CloudModelsSettingsState {
    snapshot: CloudAccessSnapshot,
    /// Per-provider editable API-key buffer (zeroized on clear/drop).
    key_drafts: Vec<(String, zeroize::Zeroizing<String>)>,
    /// Per-provider transient status message (e.g. "Saved", "Removed", an error).
    messages: Vec<(String, String)>,
    pending_cli_login_confirmation: Option<String>,
    /// MT-015 v5: the live in-app official-CLI login session, when one is open.
    /// At most one at a time, matching the backend's one-session-per-provider rule.
    login_panel: Option<CloudCliLoginPanel>,
}

impl CloudModelsSettingsState {
    /// Replace the non-secret enumeration snapshot (does not touch key buffers).
    pub fn set_snapshot(&mut self, snapshot: CloudAccessSnapshot) {
        self.snapshot = snapshot;
    }

    pub fn snapshot(&self) -> &CloudAccessSnapshot {
        &self.snapshot
    }

    /// Mutable handle to a provider's key buffer, created empty on first use.
    fn key_draft_mut(&mut self, provider: &str) -> &mut zeroize::Zeroizing<String> {
        if let Some(idx) = self.key_drafts.iter().position(|(p, _)| p == provider) {
            &mut self.key_drafts[idx].1
        } else {
            self.key_drafts
                .push((provider.to_owned(), zeroize::Zeroizing::new(String::new())));
            &mut self.key_drafts.last_mut().unwrap().1
        }
    }

    /// Whether a provider's key buffer currently holds any non-whitespace text.
    pub fn key_draft_is_empty(&self, provider: &str) -> bool {
        self.key_drafts
            .iter()
            .find(|(p, _)| p == provider)
            .map(|(_, b)| b.trim().is_empty())
            .unwrap_or(true)
    }

    /// Take the provider's key buffer OUT (moving it to the caller, e.g. an async save task) and leave
    /// the UI slot empty. The moved buffer zeroizes when the caller drops it; the UI buffer is cleared.
    pub fn take_key_draft(&mut self, provider: &str) -> zeroize::Zeroizing<String> {
        if let Some(idx) = self.key_drafts.iter().position(|(p, _)| p == provider) {
            std::mem::replace(
                &mut self.key_drafts[idx].1,
                zeroize::Zeroizing::new(String::new()),
            )
        } else {
            zeroize::Zeroizing::new(String::new())
        }
    }

    /// Drop ALL per-provider key buffers. Each buffer is a [`zeroize::Zeroizing<String>`], so clearing
    /// the vector zeroizes every held key. Called on dialog close and (re-)open so a typed-but-unsaved
    /// key never lingers in the shell across a reopen (MT-015 F3).
    pub fn clear_key_drafts(&mut self) {
        self.key_drafts.clear();
        self.pending_cli_login_confirmation = None;
    }

    /// The BYOK rows to render: the backend enumeration snapshot when present, else the static seed rows
    /// (MT-015 F10) so a key field + Save ALWAYS render even when the backend is unreachable. Only the
    /// `configured` badge needs the backend; key entry does not.
    pub fn byok_rows_for_render(&self) -> Vec<CloudByokRow> {
        if !self.snapshot.byok.is_empty() {
            self.snapshot.byok.clone()
        } else {
            STATIC_BYOK_PROVIDERS
                .iter()
                .map(|(id, label)| CloudByokRow {
                    provider: (*id).to_owned(),
                    label: (*label).to_owned(),
                    configured: false,
                })
                .collect()
        }
    }

    /// True when the rendered BYOK rows are the static seed (no backend enumeration yet), so the section
    /// can show a "status unknown" note rather than a misleading "not configured".
    pub fn byok_rows_are_static_seed(&self) -> bool {
        self.snapshot.byok.is_empty()
    }

    /// Every BYOK provider id that could currently own a key `TextEdit` — the union of the rendered rows
    /// (snapshot or static) and any provider that already has a draft buffer. Used to reset each key
    /// widget's egui state on close/open (MT-015 F3).
    pub fn byok_provider_ids_for_clear(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .byok_rows_for_render()
            .into_iter()
            .map(|r| r.provider)
            .collect();
        for (provider, _) in &self.key_drafts {
            if !ids.iter().any(|id| id == provider) {
                ids.push(provider.clone());
            }
        }
        ids
    }

    /// Set a provider's transient status message.
    pub fn set_message(&mut self, provider: &str, message: impl Into<String>) {
        let message = message.into();
        if let Some(slot) = self.messages.iter_mut().find(|(p, _)| p == provider) {
            slot.1 = message;
        } else {
            self.messages.push((provider.to_owned(), message));
        }
    }

    fn message_for(&self, provider: &str) -> Option<&str> {
        self.messages
            .iter()
            .find(|(p, _)| p == provider)
            .map(|(_, m)| m.as_str())
    }

    /// The transient status message for a provider, if any (for tests + drivers).
    pub fn message(&self, provider: &str) -> Option<&str> {
        self.message_for(provider)
    }

    /// Open (or replace) the in-app login panel for a provider.
    pub fn open_login_panel(&mut self, provider: &str, label: &str) {
        self.login_panel = Some(CloudCliLoginPanel {
            provider: provider.to_owned(),
            label: label.to_owned(),
            ..CloudCliLoginPanel::default()
        });
    }

    /// The live in-app login panel, if one is open (for the shell + tests).
    pub fn login_panel(&self) -> Option<&CloudCliLoginPanel> {
        self.login_panel.as_ref()
    }

    /// Mutable access for the shell's async delivery pump.
    pub fn login_panel_mut(&mut self) -> Option<&mut CloudCliLoginPanel> {
        self.login_panel.as_mut()
    }

    /// Close the in-app login panel. The shell separately resets the answer
    /// field's egui state so no typed answer survives in widget memory.
    pub fn close_login_panel(&mut self) -> Option<CloudCliLoginPanel> {
        self.login_panel.take()
    }

    /// Wipe a half-typed answer without dismissing the panel or disturbing the
    /// live backend session.
    pub fn clear_login_input(&mut self) {
        if let Some(panel) = self.login_panel.as_mut() {
            panel.input.clear();
        }
    }

    /// Apply one backend login-session snapshot to the open panel.
    pub fn apply_login_snapshot(
        &mut self,
        session_id: &str,
        state: CloudCliLoginState,
        transcript: String,
    ) {
        if let Some(panel) = self.login_panel.as_mut() {
            panel.session_id = session_id.to_owned();
            panel.state = state;
            panel.transcript = transcript;
            panel.error = None;
        }
    }

    /// Record a transport failure against the open login panel.
    pub fn set_login_error(&mut self, error: impl Into<String>) {
        if let Some(panel) = self.login_panel.as_mut() {
            panel.error = Some(error.into());
        }
    }

    /// The CLI-bridge login command for a provider, if present in the snapshot.
    pub fn cli_login_command(&self, provider: &str) -> Option<(String, Vec<String>)> {
        self.snapshot
            .cli_bridge
            .iter()
            .find(|r| r.provider == provider)
            .map(|r| (r.login_program.clone(), r.login_args.clone()))
    }
}

/// What the dialog wants the shell to do after a frame.
///
/// Returned by [`show`]. The shell matches on it: a wired change variant updates
/// `workspace_settings` / `current_theme` / `view_mode` and persists via `PUT /workspaces/{id}/settings`;
/// `Close` clears the open flag; `None` leaves the dialog open. At most one outcome per frame (a single
/// control interaction), so the shell never has to reconcile two simultaneous changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    /// Nothing happened this frame; keep the dialog open.
    None,
    /// The Theme / appearance ComboBox selected a (different) theme. WIRED.
    ThemeChanged(WorkspaceTheme),
    /// The View Mode ComboBox selected a (different) mode. WIRED.
    ViewModeChanged(SettingsViewMode),
    /// A keybinding's chord changed to a NON-conflicting value (already normalized). WIRED — the shell
    /// persists it. A conflicting draft does NOT emit this (it only shows the banner), so a conflicting
    /// binding is never saved (AC6).
    KeybindingChanged { action_id: String, chord: String },
    /// A keybinding Reset button was clicked; restore the action's default chord. WIRED.
    KeybindingReset { action_id: String },
    /// The Swarm board default-open checkbox was toggled. WIRED.
    SwarmBoardDefaultOpenChanged(bool),
    /// The Swarm lane diagnostics default-open checkbox was toggled. WIRED.
    SwarmLaneDiagnosticsDefaultOpenChanged(bool),
    /// The Operator Chat default-open checkbox was toggled. WIRED.
    OperatorChatDefaultOpenChanged(bool),
    /// WP-1 MT-021: the per-frame swarm ADMISSION BUDGET ComboBox selected a different value. WIRED —
    /// the shell clamps it, persists it, and pushes it into the live [`crate::mcp::ActionChannel`] the
    /// running MCP/Argus transport drains, so concurrent-agent admission changes on the next frame.
    SwarmMaxActionsPerFrameChanged(usize),
    /// WP-1 MT-021: request a new cap for concurrently running model sessions. The shell persists the
    /// desired value and sends it through `PUT /operator-chat/swarm/max-concurrent`; it does not claim
    /// the value is in force until the coordinator returns its snapshot.
    SwarmModelSessionsMaxConcurrentChanged(usize),
    /// The Reset panes & drawers button was clicked (same action as VIEW > Reset Layout). WIRED.
    ResetLayout,
    /// Open the production Model Runtime registry pane through the same route as RUN > Open Model
    /// Runtime. This is navigation, not a second registry authority or a mutable rebind surface.
    OpenModelRuntime,
    /// Open the canonical Problems pane from the existing Model Runtime settings section.
    OpenProblems,
    /// WP-1 (a): open the Operator Chat / Launch pane — the completion-model + lane selection surface.
    /// Navigation only; the selection authority lives in that pane, not in Settings.
    OpenOperatorChat,
    /// WP-1 (b): the background resource-sampling enable checkbox was toggled. The shell forwards the
    /// value to `InternalDiagnostics::set_resource_sampling_enabled`, which pauses/resumes the real
    /// producer thread, and persists it via the workspace-settings path. WIRED.
    ResourceSamplingEnabledChanged(bool),
    /// MT-015: Save clicked for a BYOK provider. The shell reads the key buffer, sends it to the vault
    /// via `PUT /model-access/byok/{provider}/key`, then zeroizes + clears the buffer.
    CloudByokKeySaveRequested { provider: String },
    /// MT-015: Remove/Rotate clicked for a BYOK provider. The shell calls
    /// `DELETE /model-access/byok/{provider}/key` (idempotent).
    CloudByokKeyRemoveRequested { provider: String },
    /// MT-015: Log-in clicked for a CLI-bridge provider. The shell asks the backend to start the
    /// provider's OWN official login inside a Handshake-hosted pseudo-terminal (operator-initiated)
    /// and opens the in-app login panel. No OS console window is opened and focus does not move.
    /// Handshake stores no credential.
    CliBridgeLoginRequested { provider: String },
    /// MT-015 v5: the operator answered the provider's prompt in the in-app login panel. The shell
    /// POSTs the answer to the running login process's stdin.
    CliBridgeLoginInputSubmitted { provider: String, input: String },
    /// MT-015 v5: Stop clicked in the in-app login panel. The shell cancels the running login process.
    CliBridgeLoginStopRequested { provider: String },
    /// MT-015 v5: Close clicked on a finished in-app login panel. Dismisses the panel only.
    CliBridgeLoginPanelClosed { provider: String },
    /// MT-015: the modal header's "Pop out" control was clicked. The shell detaches the settings surface
    /// into its own OS window (Argus `popout-settings`) and STOPS rendering the modal, so the surface has
    /// exactly one host at a time.
    PopOut,
    /// MT-015: the detached window's "Re-dock" control was clicked. The shell returns the surface to the
    /// root-viewport modal without closing settings.
    Redock,
    /// The user dismissed the dialog (Escape, the Close button, or a backdrop click). The shell clears
    /// the open flag.
    Close,
}

/// WP-1 (c): real internal-diagnostics status the shell computes ONCE per settings-render frame and
/// hands to the dialog for display. Every field is derived from live state (`InternalDiagnostics`
/// presence, the Palmistry-provisioned Argus signing secret, and the recovered-crash count) — nothing
/// here is fabricated. `Default` is the "diagnostics subsystem unavailable" posture used by the
/// headless/test shell.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiagnosticsSettingsView {
    /// True when the internal_diagnostics subsystem started successfully (panic hook, frame-time tick,
    /// resource sampler, and Palmistry maintenance are live).
    pub subsystem_live: bool,
    /// True when Palmistry has successfully launched and provisioned the Argus MCP signing secret — a
    /// real, observable effect of a healthy out-of-process watcher (the secret only exists after a
    /// durable Palmistry launch + secret rotation).
    pub palmistry_signing_provisioned: bool,
    /// Count of prior-run crash survivors the diagnostics subsystem recovered at startup.
    pub recovered_survivor_count: usize,
}

/// Live coordinator truth rendered beside the model-session concurrency control. `snapshot == None`
/// is an explicit loading/unavailable state, never a fabricated default.
#[derive(Debug, Clone, Default)]
pub struct SwarmModelSessionsSettingsState {
    pub snapshot: Option<crate::backend_client::SwarmConcurrencySnapshot>,
    pub updating: bool,
    pub error: Option<String>,
}

/// Read-only inputs the dialog renders from (the live settings + the open generation). The dialog never
/// borrows `&mut HandshakeApp`; the shell applies the returned [`SettingsOutcome`].
pub struct SettingsView<'a> {
    /// Monotonic open generation; a new value resets the dialog's transient state.
    pub open_count: u64,
    /// The live workspace settings (theme, keybindings, view mode, swarm board flag).
    pub settings: &'a WorkspaceSettingsState,
    /// The last transient persistence error, if any, surfaced on the status row.
    pub persist_error: Option<&'a str>,
    /// MT-015: mutable Cloud Models UI state (enumeration snapshot + per-provider key buffers). Held by
    /// the shell — NOT in `DialogState` or the persisted snapshot — so a BYOK key never persists.
    pub cloud: &'a mut CloudModelsSettingsState,
    /// WP-1 (c): live internal-diagnostics status for the Diagnostics section (display-only, real state).
    pub diagnostics: DiagnosticsSettingsView,
    /// WP-1 MT-021: live SwarmCoordinator cap state, separate from the per-frame action budget.
    pub swarm_model_sessions: &'a SwarmModelSessionsSettingsState,
}

/// Transient per-open dialog UI state: the search query + the in-progress draft keybinding text per
/// action. Stored in egui persistent memory keyed to the dialog id, and RESET when [`open_count`]
/// changes so a re-open never shows the previous session's text/drafts.
///
/// [`open_count`]: DialogState::open_count
#[derive(Debug, Clone, Default)]
struct DialogState {
    /// The open generation this state was initialized for.
    open_count: u64,
    /// The current settings-search query.
    query: String,
    /// In-progress draft chord text per action (`(action_id, draft)`). The draft is what the text input
    /// shows; it is normalized for conflict detection + only persisted (via the outcome) when conflict-
    /// free (red-team R3/MC3: normalize the draft before comparing). Seeded from the live settings on
    /// (re-)open.
    drafts: Vec<(String, String)>,
    /// Set once after a (re-)open so the search box is focused on the first frame only.
    focus_requested: bool,
}

impl DialogState {
    /// The draft chord for `action_id`, if a draft has been seeded/edited.
    fn draft_for(&self, action_id: &str) -> Option<&str> {
        self.drafts
            .iter()
            .find(|(id, _)| id == action_id)
            .map(|(_, d)| d.as_str())
    }

    /// Set the draft chord for `action_id`.
    fn set_draft(&mut self, action_id: &str, draft: String) {
        if let Some(slot) = self.drafts.iter_mut().find(|(id, _)| id == action_id) {
            slot.1 = draft;
        } else {
            self.drafts.push((action_id.to_owned(), draft));
        }
    }

    /// A settings snapshot used to compute conflicts from the CURRENT drafts (red-team R3/MC3): each
    /// draft is normalized; an action with no draft uses the live chord.
    fn draft_settings(&self, live: &WorkspaceSettingsState) -> WorkspaceSettingsState {
        let keybindings: Vec<Keybinding> = APP_KEYBINDING_ACTIONS
            .iter()
            .map(|action| {
                let chord = self
                    .draft_for(action.id)
                    .map(normalize_chord_input)
                    .unwrap_or_else(|| {
                        normalize_chord_input(
                            live.chord_for(action.id).unwrap_or(action.default_chord),
                        )
                    });
                Keybinding {
                    action_id: action.id.to_owned(),
                    chord,
                }
            })
            .collect();
        WorkspaceSettingsState {
            theme: live.theme,
            keybindings,
            view_mode: live.view_mode,
            swarm_board_default_open: live.swarm_board_default_open,
            swarm_lane_diagnostics_default_open: live.swarm_lane_diagnostics_default_open,
            operator_chat_default_open: live.operator_chat_default_open,
            resource_sampling_enabled: live.resource_sampling_enabled,
            swarm_max_actions_per_frame: live.swarm_max_actions_per_frame,
            swarm_model_sessions_max_concurrent: live.swarm_model_sessions_max_concurrent,
        }
    }
}

/// Render the settings dialog overlay and return the [`SettingsOutcome`] for this frame.
///
/// `view.open_count` is a monotonic counter the shell increments each time `settings_open` flips to
/// `true`; the dialog resets its transient state whenever it sees a new value. The dialog is rendered as
/// a backdrop [`egui::Area`] (full-screen, behind the panel, catches click-to-dismiss) plus a centred
/// [`egui::Window`] with the title bar hidden — both on the `Foreground` order so the dialog sits above
/// the whole workspace (and above the palette/switcher overlays the shell renders earlier).
///
/// Layout choice (contract note): a CENTRED modal with a scroll body is used rather than a right-edge
/// side-drawer. egui's `Window` right-edge anchoring with a full-height fixed panel is awkward (it
/// fights egui's auto-sizing + the existing top/bottom panels), and the contract explicitly allows the
/// centred-modal fallback; the centred modal matches the palette/switcher overlay convention already in
/// this crate, so the three overlays are visually + structurally consistent.
pub fn show(ctx: &egui::Context, view: SettingsView<'_>) -> SettingsOutcome {
    let state_id = settings_state_id();
    let mut state = load_or_reset_state(ctx, state_id, view.open_count, view.settings, view.cloud);

    // ── Escape (AC12) — popup-aware (FIX-C). ────────────────────────────────────────────────────────
    // Escape has two jobs in this dialog: close an OPEN ComboBox popup (Theme / View Mode), or — when
    // nothing else is open — close the whole dialog. egui's ComboBox popup closes itself on Escape by
    // PEEKING `i.key_pressed(Key::Escape)` (it does not consume the event), and so does this handler
    // (`i.events.iter()` is a peek too). So on a single Escape both fire in the same frame: the popup
    // would close AND the dialog would close. That is the bug — one Escape collapsing the popup should
    // NOT also tear down the dialog.
    //
    // Fix: read whether ANY egui popup/combo is open coming into this frame (`Popup::is_any_open` reads
    // the memory the popup wrote LAST frame). If a popup is open, this Escape is "owned" by the popup —
    // egui closes it when the combo renders below this frame — so we suppress the dialog-close and keep
    // the dialog open. Only when no popup is open does Escape request a dialog close. (We do not call
    // `Popup::close_all` ourselves: egui's own Escape handling closes the combo, and double-closing
    // could swallow a second nested popup the same frame.)
    let escape = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                }
            )
        })
    });
    if escape {
        let popup_open = egui::Popup::is_any_open(ctx);
        if !popup_open {
            clear_cloud_key_state(ctx, view.cloud);
            persist(ctx, state_id, &state);
            return SettingsOutcome::Close;
        }
        // A popup/combo is open: let egui's own Escape handling close just the popup this frame; the
        // dialog stays open. Fall through to render so the combo gets its frame to close.
    }

    // ── Backdrop: a full-screen interactable Area BEHIND the window; a click on it dismisses. ──
    let screen = ctx.content_rect();
    let backdrop = egui::Area::new(egui::Id::new("settings.backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(96));
            response
        });
    if backdrop.inner.clicked() {
        clear_cloud_key_state(ctx, view.cloud);
        persist(ctx, state_id, &state);
        return SettingsOutcome::Close;
    }

    let search_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_SEARCH_NODE_ID) };
    let dialog_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_DIALOG_NODE_ID) };
    let list_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_LIST_NODE_ID) };

    let mut outcome = SettingsOutcome::None;

    egui::Window::new("settings")
        .id(egui::Id::new("settings.window"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([480.0, 560.0])
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // Header: eyebrow + title on the left, Close button on the right.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("GLOBAL").small().weak());
                    ui.label(egui::RichText::new("Settings").heading());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.button("Close");
                    set_author_id(ui, close.id, CLOSE_AUTHOR_ID);
                    if close.clicked() {
                        // Ack the applied effect (dialog dismissal is requested) so an
                        // out-of-process argus.click on `settings.close` resolves Applied.
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), CLOSE_AUTHOR_ID);
                        outcome = SettingsOutcome::Close;
                    }
                    // MT-015: detach the whole surface into its own OS window. Outcome-gated (like every
                    // other button here), so the ack stays truthful when a same-frame Close already won.
                    let popout = ui.button("Pop out");
                    set_author_id(ui, popout.id, SETTINGS_POPOUT_AUTHOR_ID);
                    if popout.clicked() && outcome == SettingsOutcome::None {
                        crate::mcp::argus::acknowledge_action_effect(
                            ui.ctx(),
                            SETTINGS_POPOUT_AUTHOR_ID,
                        );
                        outcome = SettingsOutcome::PopOut;
                    }
                });
            });
            ui.add_space(6.0);

            outcome = render_search_and_sections(
                ui,
                &mut state,
                view.settings,
                view.persist_error,
                &mut *view.cloud,
                view.diagnostics,
                view.swarm_model_sessions,
                search_egui_id,
                list_egui_id,
                440.0,
                outcome.clone(),
            );
        });

    // The dialog root container node (Role::Dialog, modal) attached to the fixed dialog id so an
    // out-of-process model finds the modal by `settings.dialog`. Emitted each open frame.
    emit_dialog_node(ctx, dialog_egui_id);

    // MT-015 F3: the Close BUTTON (handled inside the window) also dismisses the dialog. Wipe the BYOK
    // key buffers + reset each key TextEdit's egui state here so this path clears exactly like Escape /
    // backdrop before the shell tears the dialog down. Pop-out is treated identically: a
    // typed-but-unsaved key never travels across a window transition into the detached host.
    if matches!(outcome, SettingsOutcome::Close | SettingsOutcome::PopOut) {
        clear_cloud_key_state(ctx, view.cloud);
    }

    persist(ctx, state_id, &state);
    outcome
}

/// Render the settings surface as the body of its OWN detached OS window (MT-015) and return this
/// frame's [`SettingsOutcome`].
///
/// This is the SECOND host of the same surface — it does NOT fork the section rendering. Both hosts call
/// the same [`render_search_and_sections`] -> [`render_sections`] path over the same shell-owned state
/// (`SettingsView`) and the same transient [`DialogState`] in egui memory (keyed by the SAME
/// `settings.state` id and open generation), so a search query / keybinding draft survives a pop-out or
/// re-dock and every section behaves identically in both windows.
///
/// Differences from the modal, all host chrome only:
/// - it opens the SINGLE [`egui::CentralPanel`] of the detached viewport instead of a backdrop + centred
///   `Window`, so the surface fills the OS window (the caller's window-root `Role::Window` node is
///   emitted from a zero-interaction `Area`, never a second CentralPanel — same rule as
///   [`crate::popout_window::PopOutManager::show_all`]);
/// - the header carries a "Re-dock" control ([`SETTINGS_REDOCK_AUTHOR_ID`]) next to the shared Close;
/// - it emits NO `settings.dialog` node: the modal's `Role::Dialog` root is modal-only, and the detached
///   host's root is `popout-window-settings` (`Role::Window`, emitted by the shell). That difference is
///   what makes "which host is live" observable out-of-process instead of ambiguous;
/// - Escape does not close it (the OS close button and the Close control own that), so a stray Escape in
///   a background window cannot tear down the operator's detached settings.
///
/// The caller ([`crate::app::HandshakeApp`]) renders this INSIDE `show_viewport_immediate` and owns the
/// detached flag, the Argus window registration, and the OS-close seam.
pub fn show_detached(ctx: &egui::Context, view: SettingsView<'_>) -> SettingsOutcome {
    let state_id = settings_state_id();
    let mut state = load_or_reset_state(ctx, state_id, view.open_count, view.settings, view.cloud);

    let search_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_SEARCH_NODE_ID) };
    let list_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_LIST_NODE_ID) };

    let mut outcome = SettingsOutcome::None;

    egui::CentralPanel::default().show(ctx, |ui| {
        // Header: the same eyebrow + title as the modal, with Close + Re-dock on the right.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("GLOBAL").small().weak());
                ui.label(egui::RichText::new("Settings").heading());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close = ui.button("Close");
                set_author_id(ui, close.id, CLOSE_AUTHOR_ID);
                if close.clicked() {
                    crate::mcp::argus::acknowledge_action_effect(ui.ctx(), CLOSE_AUTHOR_ID);
                    outcome = SettingsOutcome::Close;
                }
                let redock = ui.button("Re-dock");
                set_author_id(ui, redock.id, SETTINGS_REDOCK_AUTHOR_ID);
                if redock.clicked() && outcome == SettingsOutcome::None {
                    crate::mcp::argus::acknowledge_action_effect(
                        ui.ctx(),
                        SETTINGS_REDOCK_AUTHOR_ID,
                    );
                    outcome = SettingsOutcome::Redock;
                }
            });
        });
        ui.add_space(6.0);

        // The detached window is resizable by the operator, so the scroll body takes whatever height is
        // left instead of the modal's fixed 440pt. A floor keeps the body usable on a tiny window.
        let body_height = (ui.available_height() - 8.0).max(120.0);
        outcome = render_search_and_sections(
            ui,
            &mut state,
            view.settings,
            view.persist_error,
            &mut *view.cloud,
            view.diagnostics,
            view.swarm_model_sessions,
            search_egui_id,
            list_egui_id,
            body_height,
            outcome.clone(),
        );
    });

    // Both window-leaving paths clear exactly like the modal's close (MT-015 F3).
    if matches!(outcome, SettingsOutcome::Close | SettingsOutcome::Redock) {
        clear_cloud_key_state(ctx, view.cloud);
    }

    persist(ctx, state_id, &state);
    outcome
}

/// The egui-memory id of the transient [`DialogState`]. Shared by BOTH hosts (modal + detached window) so
/// the search query and keybinding drafts survive a pop-out / re-dock instead of resetting.
fn settings_state_id() -> egui::Id {
    egui::Id::new("settings.state")
}

/// Load the transient dialog state, resetting it when the shell reports a new open generation.
///
/// A new `open_count` clears the query + reseeds every keybinding draft from the live settings, so a
/// re-open never shows the previous session's text/drafts, and (MT-015 F3) wipes any BYOK key buffer +
/// resets each key `TextEdit`'s egui state so a key typed but not saved in a PRIOR open never lingers.
/// Popping out / re-docking does NOT bump `open_count`, so the surface keeps its live state across the
/// host change.
fn load_or_reset_state(
    ctx: &egui::Context,
    state_id: egui::Id,
    open_count: u64,
    settings: &WorkspaceSettingsState,
    cloud: &mut CloudModelsSettingsState,
) -> DialogState {
    let state: DialogState = ctx
        .data_mut(|d| d.get_temp::<DialogState>(state_id))
        .unwrap_or_default();
    if state.open_count == open_count {
        return state;
    }
    clear_cloud_key_state(ctx, cloud);
    DialogState {
        open_count,
        query: String::new(),
        drafts: APP_KEYBINDING_ACTIONS
            .iter()
            .map(|action| {
                (
                    action.id.to_owned(),
                    settings
                        .chord_for(action.id)
                        .unwrap_or(action.default_chord)
                        .to_owned(),
                )
            })
            .collect(),
        focus_requested: false,
    }
}

/// The shared settings BODY both hosts render: the search box (pinned to the fixed search id so its
/// AccessKit NodeId is stable), the persistence-error row, and the scrollable section list
/// ([`render_sections`]) inside the fixed `Role::Group` list container.
///
/// `max_body_height` is the only host-dependent input (the modal's fixed 440pt vs the detached window's
/// remaining height). Everything an operator or Argus can address — every section, control, author_id —
/// comes from this one path, so the modal and the detached window can never drift apart.
#[allow(clippy::too_many_arguments)]
fn render_search_and_sections(
    ui: &mut egui::Ui,
    state: &mut DialogState,
    settings: &WorkspaceSettingsState,
    persist_error: Option<&str>,
    cloud: &mut CloudModelsSettingsState,
    diagnostics: DiagnosticsSettingsView,
    swarm_model_sessions: &SwarmModelSessionsSettingsState,
    search_egui_id: egui::Id,
    list_egui_id: egui::Id,
    max_body_height: f32,
    mut outcome: SettingsOutcome,
) -> SettingsOutcome {
    // Search input, pinned to the fixed search id so its AccessKit NodeId is stable.
    ui.label(egui::RichText::new("Search settings").small().weak());
    let edit = egui::TextEdit::singleline(&mut state.query)
        .id(search_egui_id)
        .hint_text("Theme, quick switcher, terminal...")
        .desired_width(f32::INFINITY);
    let edit_response = ui.add(edit);
    if !state.focus_requested {
        edit_response.request_focus();
        state.focus_requested = true;
    }
    emit_search_node(ui.ctx(), search_egui_id);

    // Persistence error row (HBR: important state visible; surfaces a save/load failure).
    if let Some(err) = persist_error {
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!("Settings sync error: {err}"))
                    .color(ui.visuals().error_fg_color),
            )
            .wrap(),
        );
    }

    ui.add_space(6.0);

    let query = state.query.trim().to_lowercase();

    // The scrollable body region (Role::Group container at the fixed list id).
    ui.push_id(list_egui_id, |ui| {
        egui::ScrollArea::vertical()
            .max_height(max_body_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                outcome = render_sections(
                    ui,
                    &query,
                    state,
                    settings,
                    cloud,
                    diagnostics,
                    swarm_model_sessions,
                    outcome.clone(),
                );
            });
    });
    emit_list_node(ui.ctx(), list_egui_id);
    outcome
}

/// Wipe every BYOK key edit buffer AND reset each key `TextEdit`'s egui state (cursor + undo history) so
/// a typed-but-unsaved key never lingers in the shell buffer or in egui memory across a dialog close or
/// reopen (MT-015 F3). Called on (re-)open and on every close path (Escape / Close button / backdrop).
fn clear_cloud_key_state(ctx: &egui::Context, cloud: &mut CloudModelsSettingsState) {
    for provider in cloud.byok_provider_ids_for_clear() {
        // Overwrite the widget's persisted egui state with a default (empty cursor + empty undoer) so the
        // undo history no longer holds a plaintext snapshot of the typed key.
        egui::TextEdit::store_state(
            ctx,
            cloud_byok_key_egui_id(&provider),
            egui::text_edit::TextEditState::default(),
        );
    }
    cloud.clear_key_drafts();
    // MT-015 v5: wipe the login ANSWER field (buffer + egui cursor/undo history) for
    // the same reason the key fields are wiped — a half-typed provider prompt answer
    // should not survive in widget memory across a close/reopen or a window
    // transition.
    //
    // The PANEL itself is deliberately NOT dropped here. It is shell-owned state
    // over a LIVE backend login session, and this helper also runs on `PopOut` /
    // `Redock`, where settings stays open. Dropping it would strand a running
    // device/OAuth login: the child would keep running under its bounded window
    // with no surface left to answer its prompt. The panel is dismissed only by an
    // explicit operator action — `Stop login` (which cancels the session) or
    // `Close` on a finished one.
    if let Some(provider) = cloud.login_panel().map(|panel| panel.provider.clone()) {
        cloud.clear_login_input();
        egui::TextEdit::store_state(
            ctx,
            cloud_cli_login_input_egui_id(&provider),
            egui::text_edit::TextEditState::default(),
        );
    }
}

/// Render every settings section in order, threading `outcome` so the first interaction this frame wins
/// (we never overwrite a Close already chosen in the header). Returns the (possibly updated) outcome.
fn render_sections(
    ui: &mut egui::Ui,
    query: &str,
    state: &mut DialogState,
    settings: &WorkspaceSettingsState,
    cloud: &mut CloudModelsSettingsState,
    diagnostics: DiagnosticsSettingsView,
    swarm_model_sessions: &SwarmModelSessionsSettingsState,
    mut outcome: SettingsOutcome,
) -> SettingsOutcome {
    // ── [1] Appearance (theme + view mode — both WIRED) ────────────────────────────────────────────
    let show_appearance = setting_matches_query(
        query,
        &[
            "appearance",
            "theme",
            "light",
            "dark",
            "view",
            "mode",
            "sfw",
            "nsfw",
        ],
    );
    let show_theme_row = setting_matches_query(query, &["appearance", "theme", "light", "dark"]);
    let show_view_mode_row =
        setting_matches_query(query, &["appearance", "view", "mode", "sfw", "nsfw"]);
    if show_appearance {
        let appearance_header = egui::CollapsingHeader::new("Appearance")
            .default_open(true)
            .show(ui, |ui| {
                if show_view_mode_row {
                    ui.horizontal(|ui| {
                        ui.label("View Mode");
                        // UPGRADED: NSFW/SFW content visibility is wired + persisted in the native shell.
                        let current = settings.view_mode;
                        let mut selected = current;
                        let combo = egui::ComboBox::from_id_salt("settings.view-mode.combo")
                            .selected_text(match current {
                                SettingsViewMode::Nsfw => "NSFW",
                                SettingsViewMode::Sfw => "SFW",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut selected, SettingsViewMode::Nsfw, "NSFW");
                                ui.selectable_value(&mut selected, SettingsViewMode::Sfw, "SFW");
                            });
                        // The visible "View Mode" row label (above) provides the accessible name; the
                        // combo carries only the stable author_id so there is exactly ONE node labeled
                        // "View Mode" in the tree (unambiguous for out-of-process lookup-by-label).
                        // Ack the open/close click (always an applied effect) so argus.click on the
                        // combo resolves Applied even when it only opens the popup.
                        set_author_id_ack_click(ui, &combo.response, VIEW_MODE_COMBO_AUTHOR_ID);
                        if selected != current && outcome == SettingsOutcome::None {
                            outcome = SettingsOutcome::ViewModeChanged(selected);
                        }
                    });
                }
                if show_theme_row {
                    ui.horizontal(|ui| {
                        ui.label("Theme / appearance");
                        // UPGRADED from NotYetWiredRow: theme is now wired in the native shell.
                        let current = settings.theme;
                        let mut selected = current;
                        let combo = egui::ComboBox::from_id_salt("settings.theme.combo")
                            .selected_text(match current {
                                WorkspaceTheme::Light => "Light",
                                WorkspaceTheme::Dark => "Dark",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut selected, WorkspaceTheme::Light, "Light");
                                ui.selectable_value(&mut selected, WorkspaceTheme::Dark, "Dark");
                            });
                        // The visible "Theme / appearance" row label (above) provides the accessible
                        // name; the combo carries only the stable author_id so there is exactly ONE node
                        // labeled "Theme / appearance" in the tree (unambiguous lookup-by-label).
                        set_author_id_ack_click(ui, &combo.response, THEME_COMBO_AUTHOR_ID);
                        if selected != current && outcome == SettingsOutcome::None {
                            outcome = SettingsOutcome::ThemeChanged(selected);
                        }
                    });
                }
            });
        set_author_id_ack_click(
            ui,
            &appearance_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}appearance"),
        );
    }

    // ── [2] Keybindings (editable + live conflict detection) ───────────────────────────────────────
    let visible_actions: Vec<&'static crate::workspace_settings::AppKeybindingAction> =
        APP_KEYBINDING_ACTIONS
            .iter()
            .filter(|action| {
                let mut terms: Vec<&str> =
                    vec!["keybinding", "shortcut", action.label, action.description];
                terms.extend_from_slice(action.keywords);
                setting_matches_query(query, &terms)
            })
            .collect();
    let show_keybindings = !visible_actions.is_empty()
        || setting_matches_query(
            query,
            &["keybinding", "keybindings", "shortcut", "shortcuts"],
        );
    if show_keybindings {
        let keybindings_header = egui::CollapsingHeader::new("Keybindings")
            .default_open(true)
            .show(ui, |ui| {
                // Conflict banner computed from the CURRENT drafts (normalized — red-team R3/MC3).
                let draft_settings = state.draft_settings(settings);
                let conflicts = find_keybinding_conflicts(&draft_settings);
                if !conflicts.is_empty() {
                    let text = conflicts
                        .iter()
                        .map(|c| {
                            format!(
                                "{} both use {}.",
                                keybinding_label_for_conflict(&c.action_labels),
                                c.chord
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let banner = ui.colored_label(ui.visuals().error_fg_color, &text);
                    // An addressable alert node so a swarm agent reads the conflict out-of-process.
                    ui.ctx().accesskit_node_builder(banner.id, |node| {
                        node.set_role(accesskit::Role::Alert);
                        node.set_author_id("settings.keybinding-conflict".to_owned());
                        node.set_label(text.clone());
                    });
                }
                for action in &visible_actions {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(action.label);
                            ui.label(egui::RichText::new(action.description).small().weak());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let reset = ui.button("Reset");
                            let reset_author =
                                format!("{KEYBINDING_RESET_AUTHOR_ID_PREFIX}{}", action.id);
                            set_author_id(ui, reset.id, &reset_author);
                            if reset.clicked() && outcome == SettingsOutcome::None {
                                // Reflect the default in the draft immediately, then emit the reset.
                                crate::mcp::argus::acknowledge_action_effect(
                                    ui.ctx(),
                                    &reset_author,
                                );
                                state.set_draft(action.id, action.default_chord.to_owned());
                                outcome = SettingsOutcome::KeybindingReset {
                                    action_id: action.id.to_owned(),
                                };
                            }

                            // Editable chord input bound to the draft.
                            let mut draft = state
                                .draft_for(action.id)
                                .map(str::to_owned)
                                .unwrap_or_else(|| action.default_chord.to_owned());
                            let input = ui.add(
                                egui::TextEdit::singleline(&mut draft)
                                    .desired_width(140.0)
                                    .hint_text(action.default_chord),
                            );
                            set_author_id_and_label(
                                ui,
                                input.id,
                                &format!("{KEYBINDING_INPUT_AUTHOR_ID_PREFIX}{}", action.id),
                                &format!("{} keybinding", action.label),
                            );
                            if input.changed() {
                                state.set_draft(action.id, draft.clone());
                                // Persist ONLY when the new draft is conflict-free (AC6): build the
                                // would-be settings, normalize, check conflicts; emit on clean.
                                //
                                // FIX-D — conflict basis (DELIBERATE): the conflict check runs over
                                // `draft_settings` (every action's CURRENT draft text, normalized),
                                // NOT over the persisted `settings.keybindings`. This is intentional:
                                // an editor must see a conflict against what the user is TYPING right
                                // now across all rows, not against the last-saved chords (which may
                                // already be stale relative to two in-progress edits). The persisted
                                // keybindings only seed the drafts on (re-)open; from then on the
                                // visible drafts are authoritative for conflict detection, so a
                                // `KeybindingChanged` is emitted (and later persisted) only when the
                                // DRAFT set — the user's visible intent — is conflict-free.
                                let normalized = normalize_chord_input(&draft);
                                if !normalized.is_empty() && outcome == SettingsOutcome::None {
                                    let mut probe = state.draft_settings(settings);
                                    probe.set_chord(action.id, normalized.clone());
                                    if find_keybinding_conflicts(&probe).is_empty() {
                                        outcome = SettingsOutcome::KeybindingChanged {
                                            action_id: action.id.to_owned(),
                                            chord: normalized,
                                        };
                                    }
                                }
                            }
                        });
                    });
                }
            });
        set_author_id_ack_click(
            ui,
            &keybindings_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}keybindings"),
        );
    }

    // ── [3] Swarm (wired default checkboxes + not-yet-wired interval rows) ─────────────────────────
    let show_swarm = setting_matches_query(
        query,
        &[
            "swarm",
            "board",
            "diagnostics",
            "operator",
            "chat",
            "reconcile",
            "resource",
            "poll",
            "concurrency",
            "concurrent",
            "admission",
            "budget",
            "agents",
            "throttle",
            "model",
            "session",
        ],
    );
    if show_swarm {
        let concurrency_query = !query.is_empty()
            && [
                "concurrency",
                "concurrent",
                "admission",
                "budget",
                "agents",
                "throttle",
                "model",
                "session",
            ]
            .iter()
            .any(|term| term.contains(query) || query.contains(term));
        let model_session_query = query.contains("model") || query.contains("session");
        let swarm_header = egui::CollapsingHeader::new("Swarm")
            .default_open(true)
            .show(ui, |ui| {
                if !concurrency_query {
                    not_yet_wired_row(ui, &SWARM_RECONCILE_INTERVAL_SETTING);
                    not_yet_wired_row(ui, &SWARM_RESOURCE_POLL_INTERVAL_SETTING);
                    ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Open Swarm Board on launch");
                        ui.label(
                            egui::RichText::new(
                                "Persisted. Board stays collapsed by default; enable to open it at startup.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                    let mut checked = settings.swarm_board_default_open;
                    let cb_label = if checked { "Open" } else { "Collapsed" };
                    let cb = ui.checkbox(&mut checked, cb_label);
                    set_author_id(ui, cb.id, SWARM_BOARD_CHECKBOX_AUTHOR_ID);
                    if cb.changed() && outcome == SettingsOutcome::None {
                        crate::mcp::argus::acknowledge_action_effect(
                            ui.ctx(),
                            SWARM_BOARD_CHECKBOX_AUTHOR_ID,
                        );
                        outcome = SettingsOutcome::SwarmBoardDefaultOpenChanged(checked);
                    }
                    });
                    ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Open Lane Diagnostics from Swarm defaults");
                        ui.label(
                            egui::RichText::new(
                                "Persisted. Keeps Dexterity lane/message diagnostics in the Swarm operator toolset.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                    let mut checked = settings.swarm_lane_diagnostics_default_open;
                    let cb_label = if checked { "Included" } else { "Hidden" };
                    let cb = ui.checkbox(&mut checked, cb_label);
                    set_author_id(ui, cb.id, SWARM_LANE_DIAGNOSTICS_CHECKBOX_AUTHOR_ID);
                    if cb.changed() && outcome == SettingsOutcome::None {
                        crate::mcp::argus::acknowledge_action_effect(
                            ui.ctx(),
                            SWARM_LANE_DIAGNOSTICS_CHECKBOX_AUTHOR_ID,
                        );
                        outcome =
                            SettingsOutcome::SwarmLaneDiagnosticsDefaultOpenChanged(checked);
                    }
                    });
                    ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Open Operator Chat from Swarm defaults");
                        ui.label(
                            egui::RichText::new(
                                "Persisted. Keeps the Operator Chat / Launch work-surface in the Swarm operator toolset.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                    let mut checked = settings.operator_chat_default_open;
                    let cb_label = if checked { "Included" } else { "Hidden" };
                    let cb = ui.checkbox(&mut checked, cb_label);
                    set_author_id(ui, cb.id, SWARM_OPERATOR_CHAT_CHECKBOX_AUTHOR_ID);
                    if cb.changed() && outcome == SettingsOutcome::None {
                        crate::mcp::argus::acknowledge_action_effect(
                            ui.ctx(),
                            SWARM_OPERATOR_CHAT_CHECKBOX_AUTHOR_ID,
                        );
                        outcome = SettingsOutcome::OperatorChatDefaultOpenChanged(checked);
                    }
                    });
                }
                // ── WP-1 MT-021 (AC-3): distinct live concurrency controls. ────────────────────────
                // Before this, the Swarm section had NO concurrency/lease control at all — only two
                // read-only backend-owned interval literals. This row is bound to live runtime
                // behaviour: the selected value is clamped, persisted, and pushed into the
                // `ActionChannel` the running MCP/Argus transport drains, so it bounds how many queued
                // actions N concurrent agents are admitted per frame from the next frame on.
                //
                // The coordinator's own `max_concurrent` SPAWN budget is a DIFFERENT quantity and is NOT
                // what this control drives. This row bounds queued shell actions per frame; that one
                // bounds live model sessions. They are deliberately not merged: one is a UI-responsiveness
                // knob, the other admits real processes, and a single "concurrency" slider driving both
                // would be the misleading control AC-3 exists to remove.
                //
                // The coordinator's model-session cap is rendered immediately below with its own stable
                // ID and backend truth, so neither quantity can be mistaken for the other.
                //
                // APPENDED at the END of the section body deliberately: inserting it above the existing
                // rows would push every later widget (and the Terminal / Layout sections) down in the
                // scroll body, the same coordinate-viewport regression the Layout-section rule at the
                // Cloud Models block guards against.
                if !model_session_query {
                    ui.separator();
                    ui.vertical(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Concurrent swarm action budget");
                        ui.add(
                            egui::Label::new(egui::RichText::new(
                                "Persisted + live. Max queued swarm/Argus actions admitted per frame when several agents drive the shell at once. Lower it to serialize concurrent agents; it can only tighten the built-in flood ceiling, never raise it.",
                            )
                            .small()
                            .weak()).wrap(),
                        );
                    });
                    let current =
                        crate::mcp::clamp_admission_budget(settings.swarm_max_actions_per_frame);
                    let mut selected = current;
                    let combo = egui::ComboBox::from_id_salt("settings.swarm-max-actions.combo")
                        .selected_text(admission_budget_label(current))
                        .show_ui(ui, |ui| {
                            for option in crate::mcp::SWARM_ADMISSION_BUDGET_OPTIONS {
                                let response = ui.selectable_value(
                                    &mut selected,
                                    option,
                                    admission_budget_label(option),
                                );
                                let option_id = swarm_action_budget_option_author_id(option);
                                set_author_id_ack_click(ui, &response, &option_id);
                            }
                        });
                    // The visible row label above is the accessible name; the combo carries only the
                    // stable author_id (one node per name, like the Theme / View Mode combos). Ack the
                    // open/close click so an out-of-process `argus.click` resolves Applied.
                    set_author_id_ack_click(ui, &combo.response, SWARM_MAX_ACTIONS_COMBO_AUTHOR_ID);
                    if selected != current && outcome == SettingsOutcome::None {
                        outcome = SettingsOutcome::SwarmMaxActionsPerFrameChanged(selected);
                    }
                    });
                }

                ui.separator();
                ui.vertical(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Concurrent model sessions");
                        ui.add(
                            egui::Label::new(egui::RichText::new(
                                "Project-persisted desired cap, applied through the live SwarmCoordinator. Lowering is cooperative: running model sessions are never killed to make the number converge.",
                            )
                            .small()
                            .weak()).wrap(),
                        );
                        let status_text = match (&swarm_model_sessions.snapshot, &swarm_model_sessions.error) {
                            (Some(snapshot), error) => format!(
                                "Requested: {} · In force: {} · Fully applied: {} · Live sessions: {}{}{}",
                                snapshot.requested,
                                snapshot.max_concurrent,
                                snapshot.fully_applied,
                                snapshot.live_sessions,
                                if swarm_model_sessions.updating { " · Applying…" } else { "" },
                                error
                                    .as_ref()
                                    .map(|message| format!(" · Update failed: {message}"))
                                    .unwrap_or_default(),
                            ),
                            (None, Some(error)) => format!("Coordinator status unavailable: {error}"),
                            (None, None) if swarm_model_sessions.updating => {
                                "Coordinator status: applying request…".to_owned()
                            }
                            (None, None) => "Coordinator status: loading…".to_owned(),
                        };
                        let status = ui.label(egui::RichText::new(&status_text).small().weak());
                        set_author_id_and_label(
                            ui,
                            status.id,
                            SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID,
                            &status_text,
                        );
                    });

                    let current = settings
                        .swarm_model_sessions_max_concurrent
                        .or_else(|| swarm_model_sessions.snapshot.as_ref().map(|s| s.requested))
                        .unwrap_or(1)
                        .max(1);
                    let mut selected = current;
                    let mut options = vec![1usize, 2, 4, 8, 16, 32, 64, current];
                    options.sort_unstable();
                    options.dedup();
                    ui.add_enabled_ui(!swarm_model_sessions.updating, |ui| {
                        let combo = egui::ComboBox::from_id_salt(
                            "settings.swarm-model-sessions-max-concurrent.combo",
                        )
                        .selected_text(current.to_string())
                        .show_ui(ui, |ui| {
                            for option in options {
                                let response = ui.selectable_value(
                                    &mut selected,
                                    option,
                                    option.to_string(),
                                );
                                let option_id = swarm_model_session_option_author_id(option);
                                set_author_id_ack_click(ui, &response, &option_id);
                            }
                        });
                        set_author_id_ack_click(
                            ui,
                            &combo.response,
                            SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID,
                        );
                    });
                    if selected != current && outcome == SettingsOutcome::None {
                        outcome = SettingsOutcome::SwarmModelSessionsMaxConcurrentChanged(selected);
                    }
                });
            });
        set_author_id_ack_click(
            ui,
            &swarm_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}swarm"),
        );
    }

    // ── [4] Terminal (not-yet-wired rows) ──────────────────────────────────────────────────────────
    let show_terminal =
        setting_matches_query(query, &["terminal", "shell", "scrollback", "logging"]);
    if show_terminal {
        let terminal_header = egui::CollapsingHeader::new("Terminal")
            .default_open(true)
            .show(ui, |ui| {
                not_yet_wired_row(ui, &TERMINAL_DEFAULT_SHELL_SETTING);
                not_yet_wired_row(ui, &TERMINAL_MAX_SCROLLBACK_SETTING);
                not_yet_wired_row(ui, &TERMINAL_OUTPUT_LOGGING_SETTING);
            });
        set_author_id_ack_click(
            ui,
            &terminal_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}terminal"),
        );
    }

    // ── [5] Layout (wired Reset panes & drawers button) ────────────────────────────────────────────
    let show_layout = setting_matches_query(query, &["layout", "reset", "panes", "drawers"]);
    if show_layout {
        let layout_header = egui::CollapsingHeader::new("Layout")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Reset layout");
                        ui.label(
                            egui::RichText::new("Restore panes & drawers to their defaults.")
                                .small()
                                .weak(),
                        );
                    });
                    let btn = ui.button("Reset panes & drawers");
                    set_author_id(ui, btn.id, RESET_LAYOUT_AUTHOR_ID);
                    if btn.clicked() && outcome == SettingsOutcome::None {
                        crate::mcp::argus::acknowledge_action_effect(
                            ui.ctx(),
                            RESET_LAYOUT_AUTHOR_ID,
                        );
                        outcome = SettingsOutcome::ResetLayout;
                    }
                });
            });
        set_author_id_ack_click(
            ui,
            &layout_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}layout"),
        );
    }

    // ── [5b] Cloud Models (MT-015: BYOK key entry + CLI-bridge subscription-plan login) ─────────────
    // Placed after Layout so it never pushes the earlier interactive sections (Keybindings / Layout)
    // below the scroll fold — a section added above them would move their live widgets out of the
    // coordinate-clickable viewport and regress those sections' kittest interaction proofs.
    let show_cloud = setting_matches_query(
        query,
        &[
            "cloud",
            "model",
            "models",
            "byok",
            "api",
            "key",
            "anthropic",
            "openai",
            "claude",
            "gpt",
            "codex",
            "login",
            "plan",
            "subscription",
            "consent",
            "export",
            "posture",
            "escalation",
        ],
    );
    if show_cloud {
        let cloud_header = egui::CollapsingHeader::new("Cloud Models")
            .default_open(true)
            .show(ui, |ui| {
                outcome = render_cloud_models_body(ui, cloud, outcome.clone());
            });
        set_author_id_ack_click(
            ui,
            &cloud_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}cloud-models"),
        );
    }

    // ── [5c] Model Runtime (MT-014 durable registry + production-pane navigation) ──────────────────
    let show_model_runtime = setting_matches_query(
        query,
        &[
            "model runtime",
            "local model",
            "registry",
            "adapter",
            "candle",
            "llama cpp",
            "embedding",
            "artifact",
            "diagnostics",
            "problems",
            "crash",
        ],
    );
    if show_model_runtime {
        let model_runtime_header = egui::CollapsingHeader::new("Model Runtime")
            .default_open(true)
            .show(ui, |ui| {
                ui.label("Durable local-model registry");
                ui.label(
                    egui::RichText::new(
                        "Inspect selected adapters, primary and embedding artifacts, live versus dormant state, revisions, portable locators, and audit references.",
                    )
                    .small()
                    .weak(),
                );
                let open = ui.button("Open Model Runtime");
                set_author_id(ui, open.id, OPEN_MODEL_RUNTIME_AUTHOR_ID);
                if open.clicked() && outcome == SettingsOutcome::None {
                    crate::mcp::argus::acknowledge_action_effect(
                        ui.ctx(),
                        OPEN_MODEL_RUNTIME_AUTHOR_ID,
                    );
                    outcome = SettingsOutcome::OpenModelRuntime;
                }
                ui.separator();
                // WP-1 (a): completion model + lane selection entry point. Settings does NOT own the
                // model registry or the selection logic — it deep-links to the Operator Chat / Launch
                // pane, the single surface where the operator picks the completion model + lane before
                // launch. Displaying a "current default" here would fabricate state the shell does not
                // persist; the honest surface is the deep-link.
                ui.label("Default completion model & lane");
                ui.label(
                    egui::RichText::new(
                        "Pick the completion model + lane (LOCAL / CLOUD / CLI / SUBAGENT) in the Operator Chat / Launch pane. Local adapters are managed in Model Runtime above.",
                    )
                    .small()
                    .weak(),
                );
                let open_operator_chat = ui.button("Open Operator Chat");
                set_author_id(ui, open_operator_chat.id, OPEN_OPERATOR_CHAT_AUTHOR_ID);
                if open_operator_chat.clicked() && outcome == SettingsOutcome::None {
                    crate::mcp::argus::acknowledge_action_effect(
                        ui.ctx(),
                        OPEN_OPERATOR_CHAT_AUTHOR_ID,
                    );
                    outcome = SettingsOutcome::OpenOperatorChat;
                }
                ui.separator();
                ui.label("Runtime diagnostics");
                ui.label(
                    egui::RichText::new(
                        "Inspect native problems, crash evidence, GPU state, Palmistry probes, and Flight Recorder import posture.",
                    )
                    .small()
                    .weak(),
                );
                let open_problems = ui.button("Open Problems");
                set_author_id(ui, open_problems.id, OPEN_PROBLEMS_AUTHOR_ID);
                if open_problems.clicked() && outcome == SettingsOutcome::None {
                    crate::mcp::argus::acknowledge_action_effect(ui.ctx(), OPEN_PROBLEMS_AUTHOR_ID);
                    outcome = SettingsOutcome::OpenProblems;
                }
            });
        set_author_id_ack_click(
            ui,
            &model_runtime_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}model-runtime"),
        );
    }

    // ── [5d] Diagnostics (WP-1: real internal-diagnostics controls + Palmistry status) ──────────────
    let show_diagnostics = setting_matches_query(
        query,
        &[
            "diagnostics",
            "internal",
            "resource",
            "sampling",
            "cpu",
            "memory",
            "palmistry",
            "watcher",
            "flight recorder",
        ],
    );
    if show_diagnostics {
        let diagnostics_header = egui::CollapsingHeader::new("Diagnostics")
            .default_open(true)
            .show(ui, |ui| {
                // WP-1 (b): a REAL producer toggle. Toggling this pauses/resumes the internal
                // diagnostics resource sampler thread (CPU/RSS counters) via
                // `InternalDiagnostics::set_resource_sampling_enabled`. It never touches the panic hook,
                // the frame-time tick, or Palmistry, so disabling background sampling cannot blind the
                // crash path or starve the Argus signing-secret rotation.
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Background resource sampling");
                        ui.label(
                            egui::RichText::new(
                                "Persisted. Controls the internal_diagnostics CPU/RSS/GPU sampler thread. Disabling it stops periodic resource counters only; the panic hook and Palmistry watcher stay live.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                    let mut checked = settings.resource_sampling_enabled;
                    let cb_label = if checked { "Sampling" } else { "Paused" };
                    let cb = ui.checkbox(&mut checked, cb_label);
                    set_author_id(ui, cb.id, RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID);
                    if cb.changed() && outcome == SettingsOutcome::None {
                        crate::mcp::argus::acknowledge_action_effect(
                            ui.ctx(),
                            RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID,
                        );
                        outcome = SettingsOutcome::ResourceSamplingEnabledChanged(checked);
                    }
                });
                ui.separator();
                // WP-1 (c): Palmistry watcher STATUS display backed by real state (no disable toggle —
                // see the module note / handoff: pausing Palmistry would starve the Argus MCP signing
                // secret rotation and disable the Argus transport this WP delivers).
                let subsystem_text = if diagnostics.subsystem_live {
                    "Internal diagnostics: live (panic hook, frame-time, resource sampler, Palmistry maintenance running)"
                } else {
                    "Internal diagnostics: unavailable in this shell (headless/test or startup failure)"
                };
                let subsystem = ui.label(egui::RichText::new(subsystem_text).small());
                set_author_id(ui, subsystem.id, DIAGNOSTICS_SUBSYSTEM_STATUS_AUTHOR_ID);
                let palmistry_text = if !diagnostics.subsystem_live {
                    "Palmistry watcher: not started".to_owned()
                } else if diagnostics.palmistry_signing_provisioned {
                    "Palmistry watcher: active — Argus signing secret provisioned by a durable watcher launch".to_owned()
                } else {
                    "Palmistry watcher: starting — awaiting durable launch + signing-secret provisioning".to_owned()
                };
                let palmistry = ui.label(egui::RichText::new(&palmistry_text).small().strong());
                set_author_id(ui, palmistry.id, PALMISTRY_STATUS_AUTHOR_ID);
                ui.label(
                    egui::RichText::new(format!(
                        "Recovered prior-run crash survivors: {}",
                        diagnostics.recovered_survivor_count
                    ))
                    .small()
                    .weak(),
                );
            });
        set_author_id_ack_click(
            ui,
            &diagnostics_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}diagnostics"),
        );
    }

    // ── [6] About (app name + REAL Cargo version) ──────────────────────────────────────────────────
    let show_about = setting_matches_query(query, &["about", "app", "version"]);
    if show_about {
        let about_header = egui::CollapsingHeader::new("About")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("App").small().weak());
                    ui.label(ABOUT_APP_NAME);
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Version").small().weak());
                    ui.label(ABOUT_VERSION);
                });
            });
        set_author_id_ack_click(
            ui,
            &about_header.header_response,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}about"),
        );
        // TODO MT-0XX: CLI Bridge config panel - see app/src/components/CliBridgeConfigPanel.tsx
    }

    outcome
}

/// Render the Cloud Models section body (MT-015). BYOK provider rows (password key entry + Save +
/// Remove/Rotate) followed by CLI-bridge provider rows (subscription-plan login launch). Every control
/// carries a stable per-provider author_id. Gemini never appears — the snapshot never lists it.
///
/// SECURITY: the key input edits a shell-owned [`zeroize::Zeroizing<String>`] buffer directly; the key
/// never enters `DialogState` or the persisted egui snapshot. The dialog only REQUESTS a save (the
/// shell reads the buffer, sends it to the vault, and clears it) — it never persists the key itself.
fn render_cloud_models_body(
    ui: &mut egui::Ui,
    cloud: &mut CloudModelsSettingsState,
    mut outcome: SettingsOutcome,
) -> SettingsOutcome {
    ui.add(
        egui::Label::new(egui::RichText::new(
            "Configure cloud model access. Your subscription PLAN via the official CLI (Claude Code, \
             GPT/Codex) is the primary path; a BYOK API key is available if you bring your own. \
             Gemini is not offered.",
        )
        .small()
        .weak()).wrap(),
    );

    // ---- BYOK: per-provider password key entry stored only in the OS keychain. ----
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Bring your own API key")
            .small()
            .strong(),
    );
    // F10: render the STATIC provider rows client-side when the backend enumeration has not arrived, so
    // the key field + Save ALWAYS render (key entry never depends on the backend being reachable). Only
    // the `configured` badge needs the backend; a seed row shows "status unknown" instead.
    let byok_rows = cloud.byok_rows_for_render();
    let byok_static_seed = cloud.byok_rows_are_static_seed();
    if byok_static_seed {
        ui.label(
            egui::RichText::new(
                "Provider status unknown until the backend responds — you can still enter a key.",
            )
            .small()
            .weak(),
        );
    }
    for row in &byok_rows {
        ui.vertical(|ui| {
            ui.vertical(|ui| {
                ui.label(&row.label);
                let status_text = if row.configured {
                    "Configured — key stored in the OS keychain"
                } else if byok_static_seed {
                    "Status unknown — backend not reachable"
                } else {
                    "Not configured"
                };
                let status = ui.label(egui::RichText::new(status_text).small().weak());
                // Attach the non-secret BYOK status TEXT as the AccessKit label EXPLICITLY (MT-015 v4).
                // The detached Settings window renders into an embedded egui viewport whose plain
                // `ui.label()` text is NOT auto-emitted into the AccessKit tree (only interactive
                // widgets are), so relying on the auto-emitted name left the detached window's
                // configured/unavailable state un-readable out-of-process. Setting the label directly
                // makes the status readable in BOTH hosts. `status_text` is a fixed non-secret string —
                // never key material.
                set_author_id_and_label(
                    ui,
                    status.id,
                    &cloud_byok_status_author_id(&row.provider),
                    status_text,
                );
            });
        });
        ui.horizontal(|ui| {
            {
                let buf = cloud.key_draft_mut(&row.provider);
                let input = ui.add(
                    // Pin a STABLE egui id so the dialog can reset this widget's egui state (undo
                    // history) on close/open, wiping any typed-but-unsaved key from egui memory (F3).
                    egui::TextEdit::singleline(&mut **buf)
                        .id(cloud_byok_key_egui_id(&row.provider))
                        .password(true)
                        .hint_text("Paste API key")
                        .desired_width(220.0),
                );
                set_author_id_and_label(
                    ui,
                    input.id,
                    &cloud_byok_key_author_id(&row.provider),
                    &format!("{} API key", row.label),
                );
            }
            let save = ui.button("Save");
            let save_author = cloud_byok_save_author_id(&row.provider);
            set_author_id(ui, save.id, &save_author);
            if save.clicked() && outcome == SettingsOutcome::None {
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &save_author);
                outcome = SettingsOutcome::CloudByokKeySaveRequested {
                    provider: row.provider.clone(),
                };
            }
            let remove = ui.add_enabled(row.configured, egui::Button::new("Remove"));
            let remove_author = cloud_byok_remove_author_id(&row.provider);
            set_author_id(ui, remove.id, &remove_author);
            if remove.clicked() && outcome == SettingsOutcome::None {
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &remove_author);
                outcome = SettingsOutcome::CloudByokKeyRemoveRequested {
                    provider: row.provider.clone(),
                };
            }
        });
        if let Some(msg) = cloud.message_for(&row.provider) {
            ui.label(egui::RichText::new(msg).small().weak());
        }
    }

    // ---- CLI bridge: subscription-plan login status + operator-initiated official login launch. ----
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Subscription plan (official CLI login)")
            .small()
            .strong(),
    );
    let cli_rows = cloud.snapshot().cli_bridge.clone();
    for row in &cli_rows {
        ui.vertical(|ui| {
            ui.vertical(|ui| {
                ui.label(&row.label);
                let status_label = row.auth_status.label();
                let status = ui.label(egui::RichText::new(status_label).small().strong());
                // Explicit AccessKit label so the CLI-bridge auth status (logged-in / logged-out /
                // expired / unavailable) is out-of-process readable in the DETACHED window too (embedded
                // egui viewports do not auto-emit plain-label text). Non-secret text only. MT-015 v4.
                set_author_id_and_label(
                    ui,
                    status.id,
                    &cloud_cli_status_author_id(&row.provider),
                    status_label,
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(if row.hint.is_empty() {
                            "Provider-owned CLI auth; Handshake stores no credential."
                        } else {
                            row.hint.as_str()
                        })
                        .small()
                        .weak(),
                    )
                    .wrap(),
                );
            });
            let login = ui.button("Log in…");
            let login_author = cloud_cli_login_author_id(&row.provider);
            set_author_id(ui, login.id, &login_author);
            if login.clicked() && outcome == SettingsOutcome::None {
                // Applied effect: the login-confirmation prompt is armed for this provider.
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &login_author);
                cloud.pending_cli_login_confirmation = Some(row.provider.clone());
            }
        });
        if let Some(msg) = cloud.message_for(&row.provider) {
            ui.label(egui::RichText::new(msg).small().weak());
        }
    }

    // ---- WP-1 MT-021 (AC-2): per-lane cloud consent / export posture, explicitly NOT WIRED. ----
    // "Each configured provider lane" = every lane rendered above: the BYOK rows (snapshot or static
    // seed) plus the CLI-bridge rows. De-duplicated by provider id so a provider offering both a BYOK
    // key and a CLI login shows one posture row, not two.
    let mut consent_lanes: Vec<(String, String)> = Vec::new();
    for row in byok_rows
        .iter()
        .map(|r| (&r.provider, &r.label))
        .chain(cli_rows.iter().map(|r| (&r.provider, &r.label)))
    {
        if !consent_lanes.iter().any(|(p, _)| p == row.0) {
            consent_lanes.push((row.0.clone(), row.1.clone()));
        }
    }
    render_cloud_consent_posture(ui, &consent_lanes);

    if let Some(provider) = cloud.pending_cli_login_confirmation.clone() {
        let label = cli_rows
            .iter()
            .find(|row| row.provider == provider)
            .map(|row| row.label.as_str())
            .unwrap_or(provider.as_str());
        ui.group(|ui| {
            let prompt = cloud_cli_login_confirmation_line(label);
            let prompt_row = ui.label(&prompt);
            set_author_id_and_label(
                ui,
                prompt_row.id,
                &cloud_cli_login_confirm_prompt_author_id(&provider),
                &prompt,
            );
            ui.horizontal(|ui| {
                let confirm = ui.button("Start login");
                let confirm_author = cloud_cli_login_confirm_author_id(&provider);
                set_author_id(ui, confirm.id, &confirm_author);
                if confirm.clicked() && outcome == SettingsOutcome::None {
                    crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &confirm_author);
                    cloud.pending_cli_login_confirmation = None;
                    outcome = SettingsOutcome::CliBridgeLoginRequested {
                        provider: provider.clone(),
                    };
                }
                let cancel = ui.button("Cancel");
                let cancel_author = cloud_cli_login_cancel_author_id(&provider);
                set_author_id(ui, cancel.id, &cancel_author);
                if cancel.clicked() {
                    crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &cancel_author);
                    cloud.pending_cli_login_confirmation = None;
                }
            });
        });
    }

    if outcome == SettingsOutcome::None {
        if let Some(panel_outcome) = render_cli_login_panel(ui, cloud) {
            outcome = panel_outcome;
        }
    } else {
        render_cli_login_panel(ui, cloud);
    }

    outcome
}

/// The operator-facing confirmation line for starting an official-CLI login.
///
/// HONESTY CONTRACT (HBR-QUIET-001): this text used to warn that a new terminal window would open
/// and might take focus. That is no longer what happens — the backend runs the provider's login in a
/// Handshake-hosted pseudo-terminal and this dialog renders it — so the text says what the product
/// actually does. If the launch mechanism ever regresses to an OS console, this line and
/// `render_cli_login_panel` are the two places that must change back.
pub fn cloud_cli_login_confirmation_line(provider_label: &str) -> String {
    format!(
        "Start the official {provider_label} login inside Handshake? It runs in an in-app terminal \
         session here in Settings — no new window opens, nothing takes focus, and you answer the \
         provider's prompt in this panel. Handshake stores no credential."
    )
}

/// Author_id for the confirmation prompt line: `settings.cloud.cli.{provider}.login.prompt`.
pub fn cloud_cli_login_confirm_prompt_author_id(provider: &str) -> String {
    format!("settings.cloud.cli.{provider}.login.prompt")
}

/// Render the live in-app official-CLI login panel.
///
/// This is the surface that makes the quiet launch USABLE: hiding the console without it would turn
/// "steals focus" into "hangs invisibly with no prompt". The provider's own output is rendered
/// read-only, the operator's answer goes back to the login process's stdin, and every node carries a
/// stable author_id so Argus can read the prompt and type the answer out-of-process.
fn render_cli_login_panel(
    ui: &mut egui::Ui,
    cloud: &mut CloudModelsSettingsState,
) -> Option<SettingsOutcome> {
    let panel = cloud.login_panel.as_mut()?;
    let provider = panel.provider.clone();
    let mut outcome: Option<SettingsOutcome> = None;

    ui.add_space(8.0);
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(format!("{} login (in Handshake)", panel.label))
                .small()
                .strong(),
        );

        let state_line = panel.state.label();
        let state_row = ui.label(egui::RichText::new(state_line).small());
        set_author_id_and_label(
            ui,
            state_row.id,
            &cloud_cli_login_state_author_id(&provider),
            state_line,
        );

        // The provider's own terminal output. Read-only and monospaced so a device code or URL is
        // copyable and unambiguous. Explicitly labelled for AccessKit: plain labels are not
        // auto-emitted into an embedded (detached-window) viewport's tree.
        let transcript_text = if panel.transcript.trim().is_empty() {
            "(no output from the provider yet)".to_string()
        } else {
            panel.transcript.clone()
        };
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .id_salt(("settings.cloud.cli.login.transcript", provider.as_str()))
            .show(ui, |ui| {
                let transcript =
                    ui.label(egui::RichText::new(&transcript_text).monospace().small());
                set_author_id_and_label(
                    ui,
                    transcript.id,
                    &cloud_cli_login_transcript_author_id(&provider),
                    &transcript_text,
                );
            });

        if !panel.state.is_terminal() {
            ui.vertical(|ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut panel.input)
                        .id(cloud_cli_login_input_egui_id(&provider))
                        .desired_width(f32::INFINITY)
                        .hint_text("Answer the prompt above"),
                );
                set_author_id(ui, field.id, &cloud_cli_login_input_author_id(&provider));

                let submitted_with_enter =
                    field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.vertical(|ui| {
                    let send = ui.button("Send");
                    let send_author = cloud_cli_login_send_author_id(&provider);
                    set_author_id(ui, send.id, &send_author);
                    if (send.clicked() || submitted_with_enter) && outcome.is_none() {
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &send_author);
                        let input = std::mem::take(&mut panel.input);
                        outcome = Some(SettingsOutcome::CliBridgeLoginInputSubmitted {
                            provider: provider.clone(),
                            input,
                        });
                    }

                    let stop = ui.button("Stop login");
                    let stop_author = cloud_cli_login_stop_author_id(&provider);
                    set_author_id(ui, stop.id, &stop_author);
                    if stop.clicked() && outcome.is_none() {
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &stop_author);
                        outcome = Some(SettingsOutcome::CliBridgeLoginStopRequested {
                            provider: provider.clone(),
                        });
                    }
                });
            });
        } else {
            let close = ui.button("Close");
            let close_author = cloud_cli_login_close_author_id(&provider);
            set_author_id(ui, close.id, &close_author);
            if close.clicked() && outcome.is_none() {
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &close_author);
                outcome = Some(SettingsOutcome::CliBridgeLoginPanelClosed {
                    provider: provider.clone(),
                });
            }
        }

        if let Some(error) = panel.error.as_deref() {
            ui.label(egui::RichText::new(error).small().weak());
        }
    });

    outcome
}

/// The ComboBox text for one per-frame swarm admission budget. The default (the compiled-in flood
/// ceiling) is marked so the operator can tell "no extra throttle" from a deliberate throttle, and the
/// fully-serialized minimum is named rather than shown as a bare `1`.
fn admission_budget_label(budget: usize) -> String {
    if budget == crate::mcp::MIN_ACTIONS_PER_BURST {
        format!("{budget} (serialized)")
    } else if budget == crate::mcp::MAX_ACTIONS_PER_BURST {
        format!("{budget} (default — no extra throttle)")
    } else {
        budget.to_string()
    }
}

/// Stable child id for a per-frame action-budget option, enabling a real AccessKit popup selection.
pub fn swarm_action_budget_option_author_id(value: usize) -> String {
    format!("{SWARM_MAX_ACTIONS_COMBO_AUTHOR_ID}.option.{value}")
}

/// Stable child id for a model-session cap option, so Argus can open the ComboBox and choose a value
/// through the real AccessKit action path instead of injecting a SettingsOutcome.
pub fn swarm_model_session_option_author_id(value: usize) -> String {
    format!("{SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID}.option.{value}")
}

/// Render the per-lane cloud consent / export posture rows (WP-1 MT-021 AC-2).
///
/// HONESTY CONTRACT: there is no backend consent/export-posture route (see the module-level decision
/// note next to [`cloud_consent_posture_author_id`]), so every row renders the SAME explicit not-wired
/// line — never an inferred, defaulted, or plausible-looking posture. If a route later lands, this is
/// the single place that switches from the not-wired line to the real posture + denial reason.
///
/// PRIVACY (HBR-PRIV-008): the rendered text is built ONLY from the provider label already displayed in
/// the rows above. It carries no project, workspace, account, artifact, or resource identifier, and no
/// denial reason is synthesized — Handshake has none to show, and inventing one is exactly the leak
/// vector this control exists to avoid.
fn render_cloud_consent_posture(ui: &mut egui::Ui, lanes: &[(String, String)]) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Cloud consent & export posture")
            .small()
            .strong(),
    );
    let summary_text = format!(
        "{CLOUD_CONSENT_NOT_WIRED_TOKEN} — no consent or export posture is available for any lane. \
         The backend exposes no consent/export-posture route yet, so Handshake shows no posture and \
         assumes none. Nothing here grants, widens, or records consent.",
    );
    // `selectable_labels` is enabled in Handshake's egui style, which upgrades a plain Label to
    // click-and-drag sense and therefore advertises AccessKit `Click`.  This posture is deliberately
    // display-only: pin `selectable(false)` so the accessibility contract cannot imply an authority-
    // widening interaction that the product does not implement.
    let summary = ui.add(
        egui::Label::new(egui::RichText::new(&summary_text).small().weak())
            .selectable(false)
            .wrap(),
    );
    set_author_id_and_label(
        ui,
        summary.id,
        CLOUD_CONSENT_STATUS_AUTHOR_ID,
        &summary_text,
    );

    for (provider, label) in lanes {
        let line = cloud_consent_posture_line(label);
        // Plain `ui.label` text is NOT auto-emitted into the AccessKit tree of the DETACHED settings
        // viewport (embedded viewports only auto-emit interactive widgets), so — exactly like the BYOK
        // and CLI status rows — the posture text is attached as an explicit label. Non-secret text only.
        let row = ui.add(
            egui::Label::new(egui::RichText::new(&line).small())
                .selectable(false)
                .wrap(),
        );
        set_author_id_and_label(
            ui,
            row.id,
            &cloud_consent_posture_author_id(provider),
            &line,
        );
    }
}

/// Render one not-yet-wired row: label + note on the left, a DISABLED read-only text input pinned to the
/// fixed value on the right. The control uses `add_enabled(false, ..)` so it is visually grayed AND
/// cannot receive Tab focus (red-team R5/MC5), and carries a stable author_id derived from the setting
/// id so it is addressable out-of-process while clearly non-editable.
fn not_yet_wired_row(ui: &mut egui::Ui, setting: &NotYetWiredSetting) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(setting.label);
            ui.label(egui::RichText::new(setting.note).small().weak());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut value = setting.fixed_value.to_owned();
            let resp = ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut value).desired_width(180.0),
            );
            set_author_id(
                ui,
                resp.id,
                &format!("{NOT_WIRED_AUTHOR_ID_PREFIX}{}", setting.id),
            );
        });
    });
}

/// Persist the transient dialog state back into egui memory.
fn persist(ctx: &egui::Context, state_id: egui::Id, state: &DialogState) {
    ctx.data_mut(|d| d.insert_temp(state_id, state.clone()));
}

/// Attach a stable author_id to an already-interactive live node (egui derived its role + actions from
/// the widget's `Sense`/`widget_info`). Mirrors `accessibility::emit_interactive_node`, but takes a
/// `&Ui` for the egui-version ergonomics inside the closures.
fn set_author_id(ui: &egui::Ui, widget_id: egui::Id, author_id: &str) {
    let author_id = author_id.to_owned();
    ui.ctx()
        .accesskit_node_builder(widget_id, move |node| node.set_author_id(author_id));
}

/// Attach a stable author_id to a live node AND, if the node was clicked THIS frame, acknowledge the
/// Argus action effect for that author_id. Use ONLY for controls whose click ALWAYS applies an
/// in-dialog effect that is independent of the shared `outcome` accumulator — a ComboBox open/close
/// and a CollapsingHeader expand/collapse both mutate egui state on every click regardless of what
/// else happened this frame. Without this, an out-of-process `argus.click` on those controls times out
/// on the handler-acknowledgement gate (argus.rs `observe_postcondition`) even though the control did
/// react. Outcome-gated controls (buttons/checkboxes that only apply when `outcome == None`) MUST ack
/// inside their guarded block instead, so the ack stays truthful when a same-frame arbitration drops
/// their effect.
fn set_author_id_ack_click(ui: &egui::Ui, response: &egui::Response, author_id: &str) {
    set_author_id(ui, response.id, author_id);
    if response.clicked() {
        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), author_id);
    }
}

/// Attach a stable author_id AND an accessible label to an already-interactive live node. Used for
/// controls whose accessible name is not derivable from rendered text (e.g. a `TextEdit`, or a ComboBox
/// whose visible label is a sibling), so an out-of-process model + kittest can resolve them by label.
fn set_author_id_and_label(ui: &egui::Ui, widget_id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(widget_id, move |node| {
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

/// Emit the dialog ROOT node (Role::Dialog, modal=true, label="Settings").
fn emit_dialog_node(ctx: &egui::Context, dialog_id: egui::Id) {
    ctx.accesskit_node_builder(dialog_id, |node| {
        node.set_role(accesskit::Role::Dialog);
        node.set_author_id(SETTINGS_DIALOG_AUTHOR_ID.to_owned());
        node.set_label("Settings".to_owned());
        node.set_modal();
    });
}

/// Emit the search box address. egui already derived `Role::TextInput`; this adds the stable author_id.
fn emit_search_node(ctx: &egui::Context, search_id: egui::Id) {
    crate::accessibility::emit_interactive_node(ctx, search_id, SETTINGS_SEARCH_AUTHOR_ID);
}

/// Emit the body/list region node (Role::Group, label="Settings sections").
fn emit_list_node(ctx: &egui::Context, list_id: egui::Id) {
    ctx.accesskit_node_builder(list_id, |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(SETTINGS_LIST_AUTHOR_ID.to_owned());
        node.set_label("Settings sections".to_owned());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three fixed container ids sit in the fresh 17..=19 band, strictly below the pane id base, and
    /// are distinct.
    #[test]
    fn settings_container_ids_in_disjoint_fresh_band() {
        for id in [
            SETTINGS_DIALOG_NODE_ID,
            SETTINGS_SEARCH_NODE_ID,
            SETTINGS_LIST_NODE_ID,
        ] {
            assert!((17..=19).contains(&id), "settings id {id} in band 17..=19");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "settings id {id} below pane base {}",
                crate::accessibility::PANE_NODE_ID_BASE
            );
        }
        assert_ne!(SETTINGS_DIALOG_NODE_ID, SETTINGS_SEARCH_NODE_ID);
        assert_ne!(SETTINGS_SEARCH_NODE_ID, SETTINGS_LIST_NODE_ID);
        assert_ne!(SETTINGS_DIALOG_NODE_ID, SETTINGS_LIST_NODE_ID);
    }

    /// The author_ids are stable kebab-case keys.
    #[test]
    fn settings_author_ids_are_stable() {
        assert_eq!(SETTINGS_DIALOG_AUTHOR_ID, "settings.dialog");
        assert_eq!(SETTINGS_SEARCH_AUTHOR_ID, "settings.search");
        assert_eq!(SETTINGS_LIST_AUTHOR_ID, "settings.list");
        assert_eq!(THEME_COMBO_AUTHOR_ID, "settings.theme");
        assert_eq!(VIEW_MODE_COMBO_AUTHOR_ID, "settings.view-mode");
        assert_eq!(
            SWARM_BOARD_CHECKBOX_AUTHOR_ID,
            "settings.swarm-board-default-open"
        );
        assert_eq!(RESET_LAYOUT_AUTHOR_ID, "settings.reset-layout");
        assert_eq!(CLOSE_AUTHOR_ID, "settings.close");
        assert_eq!(SETTINGS_POPOUT_AUTHOR_ID, "settings.popout");
        assert_eq!(SETTINGS_REDOCK_AUTHOR_ID, "settings.redock");

        // ── WP-1 MT-021 (AC-5): pre-existing coverage hole. ────────────────────────────────────────
        // These ids shipped with the WP-1 Settings work but were pinned by NOTHING — no test in the
        // crate referenced them, so a rename would have silently broken every out-of-process model and
        // Argus script addressing them without a single failing test. They are pinned here now.
        assert_eq!(
            SWARM_LANE_DIAGNOSTICS_CHECKBOX_AUTHOR_ID,
            "settings.swarm-lane-diagnostics-default-open"
        );
        assert_eq!(
            SWARM_OPERATOR_CHAT_CHECKBOX_AUTHOR_ID,
            "settings.swarm-operator-chat-default-open"
        );
        assert_eq!(OPEN_MODEL_RUNTIME_AUTHOR_ID, "settings.model-runtime.open");
        assert_eq!(
            OPEN_PROBLEMS_AUTHOR_ID,
            "settings.model-runtime.open-problems"
        );
        assert_eq!(
            OPEN_OPERATOR_CHAT_AUTHOR_ID,
            "settings.model-runtime.open-operator-chat"
        );
        assert_eq!(
            RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID,
            "settings.diagnostics.resource-sampling-enabled"
        );
        assert_eq!(
            PALMISTRY_STATUS_AUTHOR_ID,
            "settings.diagnostics.palmistry-status"
        );
        assert_eq!(
            DIAGNOSTICS_SUBSYSTEM_STATUS_AUTHOR_ID,
            "settings.diagnostics.subsystem-status"
        );

        // ── WP-1 MT-021 (AC-4): the ids this MT adds. ──────────────────────────────────────────────
        assert_eq!(
            SWARM_MAX_ACTIONS_COMBO_AUTHOR_ID,
            "settings.swarm-max-actions-per-frame"
        );
        assert_eq!(
            CLOUD_CONSENT_STATUS_AUTHOR_ID,
            "settings.cloud.consent.status"
        );
        assert_eq!(
            cloud_consent_posture_author_id("anthropic"),
            "settings.cloud.consent.anthropic.posture"
        );
        assert_eq!(
            cloud_consent_posture_author_id("claude_code"),
            "settings.cloud.consent.claude_code.posture"
        );

        // Every pinned id is unique — a copy/paste collision would make two controls indistinguishable
        // out-of-process.
        let ids = [
            SETTINGS_DIALOG_AUTHOR_ID,
            SETTINGS_SEARCH_AUTHOR_ID,
            SETTINGS_LIST_AUTHOR_ID,
            THEME_COMBO_AUTHOR_ID,
            VIEW_MODE_COMBO_AUTHOR_ID,
            SWARM_BOARD_CHECKBOX_AUTHOR_ID,
            SWARM_LANE_DIAGNOSTICS_CHECKBOX_AUTHOR_ID,
            SWARM_OPERATOR_CHAT_CHECKBOX_AUTHOR_ID,
            SWARM_MAX_ACTIONS_COMBO_AUTHOR_ID,
            RESET_LAYOUT_AUTHOR_ID,
            OPEN_MODEL_RUNTIME_AUTHOR_ID,
            OPEN_PROBLEMS_AUTHOR_ID,
            OPEN_OPERATOR_CHAT_AUTHOR_ID,
            RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID,
            PALMISTRY_STATUS_AUTHOR_ID,
            DIAGNOSTICS_SUBSYSTEM_STATUS_AUTHOR_ID,
            CLOUD_CONSENT_STATUS_AUTHOR_ID,
            CLOSE_AUTHOR_ID,
            SETTINGS_POPOUT_AUTHOR_ID,
            SETTINGS_REDOCK_AUTHOR_ID,
        ];
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "settings author_ids are unique");
    }

    /// WP-1 MT-021 (AC-2, red-team): the consent posture line is an EXPLICIT unavailable state and
    /// never a fabricated posture, and it leaks no restricted resource metadata (HBR-PRIV-008).
    #[test]
    fn cloud_consent_posture_line_is_explicitly_unwired_and_leaks_no_metadata() {
        let line = cloud_consent_posture_line("Anthropic (Claude)");
        assert!(
            line.contains(CLOUD_CONSENT_NOT_WIRED_TOKEN),
            "the line carries the explicit not-wired token: {line}"
        );
        assert!(
            line.contains("no posture is shown and none is assumed"),
            "the line refuses to assume a posture: {line}"
        );
        // No fabricated verdict wording. "consent" appears only as the NAME of the missing surface.
        let lowered = line.to_lowercase();
        for forbidden in [
            "consented",
            "approved",
            "granted",
            "allowed",
            "denied",
            "refused",
            "export allowed",
            "no export",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "the not-wired line must not imply a verdict ('{forbidden}'): {line}"
            );
        }
        // Only the provider label the section already displays; no scoped identifiers.
        for leaked in [
            "workspace",
            "project",
            "account",
            "artifact",
            "resource id",
            "user_id",
            "receipt",
            "sha256",
        ] {
            assert!(
                !lowered.contains(leaked),
                "the not-wired line must not carry restricted metadata ('{leaked}'): {line}"
            );
        }
    }

    /// WP-1 MT-021 (AC-3): the admission-budget ComboBox text distinguishes "no extra throttle" from a
    /// deliberate throttle, so the default is never mistaken for a configured limit.
    #[test]
    fn admission_budget_labels_name_the_default_and_the_serialized_floor() {
        assert!(admission_budget_label(crate::mcp::MIN_ACTIONS_PER_BURST).contains("serialized"));
        assert!(admission_budget_label(crate::mcp::MAX_ACTIONS_PER_BURST).contains("default"));
        assert_eq!(admission_budget_label(4), "4");
    }

    /// MT-015 detached window: the pop-out key feeds the SHARED pane pop-out id scheme, so the detached
    /// Settings window is addressed exactly like a pane pop-out — Argus window `popout-settings`, root
    /// node `popout-window-settings`, OS title "Handshake – Settings" (en dash).
    #[test]
    fn detached_settings_window_reuses_the_shared_popout_id_scheme() {
        assert_eq!(SETTINGS_POPOUT_KEY, "settings");
        assert_eq!(
            crate::popout_window::argus_window_id(SETTINGS_POPOUT_KEY),
            "popout-settings"
        );
        assert_eq!(
            crate::popout_window::popout_window_author_id(SETTINGS_POPOUT_KEY),
            "popout-window-settings"
        );
        assert_eq!(
            crate::popout_window::popout_title_for(SETTINGS_WINDOW_LABEL),
            "Handshake \u{2013} Settings"
        );
        // The detached window id can never collide with one of the four fixed grid panes.
        for (pane, _) in crate::popout_window::MERGE_BACK_SLOTS {
            assert_ne!(
                crate::popout_window::argus_window_id(pane),
                crate::popout_window::argus_window_id(SETTINGS_POPOUT_KEY)
            );
        }
    }

    /// MT-015: the Cloud Models per-provider author_ids are stable, kebab/dotted, and distinct per
    /// provider so Argus addresses each provider's key field, Save, Remove, status, and CLI login
    /// deterministically. Gemini is never an offered provider id here.
    #[test]
    fn cloud_models_author_ids_are_stable_per_provider() {
        assert_eq!(
            cloud_byok_key_author_id("openai"),
            "settings.cloud.byok.openai.key"
        );
        assert_eq!(
            cloud_byok_key_author_id("anthropic"),
            "settings.cloud.byok.anthropic.key"
        );
        assert_eq!(
            cloud_byok_save_author_id("openai"),
            "settings.cloud.byok.openai.save"
        );
        assert_eq!(
            cloud_byok_remove_author_id("anthropic"),
            "settings.cloud.byok.anthropic.remove"
        );
        assert_eq!(
            cloud_byok_status_author_id("openai"),
            "settings.cloud.byok.openai.status"
        );
        assert_eq!(
            cloud_cli_login_author_id("claude_code"),
            "settings.cloud.cli.claude_code.login"
        );
        assert_eq!(
            cloud_cli_status_author_id("codex"),
            "settings.cloud.cli.codex.status"
        );
        assert_eq!(
            cloud_cli_login_confirm_author_id("claude_code"),
            "settings.cloud.cli.claude_code.login.confirm"
        );
        assert_eq!(
            cloud_cli_login_cancel_author_id("codex"),
            "settings.cloud.cli.codex.login.cancel"
        );
        // Distinct per provider.
        assert_ne!(
            cloud_byok_key_author_id("openai"),
            cloud_byok_key_author_id("anthropic")
        );
    }

    /// The zeroizing key buffer clears on take (proves the shell can wipe the buffer after submit) and
    /// starts empty.
    #[test]
    fn cloud_key_buffer_takes_and_clears() {
        let mut state = CloudModelsSettingsState::default();
        assert!(state.key_draft_is_empty("openai"));
        state.key_draft_mut("openai").push_str("sk-secret-draft");
        assert!(!state.key_draft_is_empty("openai"));
        let taken = state.take_key_draft("openai");
        assert_eq!(&*taken, "sk-secret-draft");
        // After take, the UI buffer is empty again.
        assert!(state.key_draft_is_empty("openai"));
    }
}
