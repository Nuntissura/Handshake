//! Settings / Options dialog for the native Handshake shell (WP-KERNEL-011 MT-018).
//!
//! ## What this provides (no-context model navigation — HBR-VIS / HBR-SWARM)
//!
//! A modal settings dialog (a centred, always-on-top floating panel with a hidden title bar) opened
//! from HELP > Open Settings… (MT-015 menu), the command palette action `settings.open` (MT-016), or a
//! test/agent setting `app_state.settings_open = true`. It is a port of
//! `app/src/components/SettingsMenu.tsx` over the [`crate::workspace_settings`] schema + helpers, with
//! these sections in order: Appearance (theme + view mode — both WIRED), Keybindings (editable, with
//! live conflict detection), Swarm (a wired board-default-open checkbox + not-yet-wired interval rows),
//! Terminal (not-yet-wired rows), Layout (a wired Reset panes & drawers button), and About (app name +
//! the real Cargo version).
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

use egui::accesskit;

use crate::module_switcher::{ModuleId, MODULE_DEFINITIONS, pane_type_tab_id};
use crate::pane_registry::PaneType;

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
/// Stable author_id for the Calendar view timezone input.
pub const CALENDAR_TIMEZONE_AUTHOR_ID: &str = "settings.calendar-timezone";
/// Stable author_id for applying a valid Calendar view timezone.
pub const CALENDAR_TIMEZONE_APPLY_AUTHOR_ID: &str = "settings.calendar-timezone.apply";
/// Stable author_id for the Swarm board default-open checkbox.
pub const SWARM_BOARD_CHECKBOX_AUTHOR_ID: &str = "settings.swarm-board-default-open";
pub const SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID: &str =
    "settings.swarm-model-sessions-max-concurrent";
pub const SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID: &str =
    "settings.swarm-model-sessions-max-concurrent.status";

pub fn swarm_model_session_option_author_id(value: usize) -> String {
    format!("{SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID}.option.{value}")
}
/// Stable author_id for the Reset panes & drawers button.
pub const RESET_LAYOUT_AUTHOR_ID: &str = "settings.reset-layout";
/// Stable author_id for the Settings -> Model Session action that opens the real launch dialog.
pub const MODEL_SESSION_OPEN_LAUNCH_AUTHOR_ID: &str = "settings.model-session.open-launch";
pub const ENGINE_PROFILE_CREATE_AUTHOR_ID: &str = "settings.engine-profiles.create";
/// Stable author_id for the Close button.
pub const CLOSE_AUTHOR_ID: &str = "settings.close";
/// Author_id prefix for a per-action keybinding text input (`{prefix}{action_id}`).
pub const KEYBINDING_INPUT_AUTHOR_ID_PREFIX: &str = "settings.keybinding.";
/// Author_id prefix for a per-action keybinding Reset button (`{prefix}{action_id}`).
pub const KEYBINDING_RESET_AUTHOR_ID_PREFIX: &str = "settings.keybinding-reset.";
/// Author_id prefix for a not-yet-wired row's disabled control (`{prefix}{setting_id}`).
pub const NOT_WIRED_AUTHOR_ID_PREFIX: &str = "settings.not-wired.";
/// Author_id prefix for a settings SECTION collapsing-header button (`{prefix}{section_key}`). Each
/// section header (Appearance / Keybindings / Swarm / Terminal / Layout / About) renders as an
/// interactive `Role::Button` in egui's hashed id space; tagging it gives an out-of-process model a
/// stable handle to expand/collapse each section. Without it the header is an interactive control with
/// no stable address — the gap the MT-029 overlay accessibility-invariant proof surfaces.
pub const SECTION_HEADER_AUTHOR_ID_PREFIX: &str = "settings.section.";
/// Stable AccessKit address for retrying the exact failed settings GET or PUT.
pub const SETTINGS_PERSIST_RETRY_AUTHOR_ID: &str = "settings.persist.retry";
/// Stable AccessKit status address for the visible settings persistence failure.
pub const SETTINGS_PERSIST_ERROR_AUTHOR_ID: &str = "settings.persist.error";

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
    pub fn from_wire(value: &str) -> Option<Self> {
        Some(match value {
            "pending" => Self::Pending,
            "awaiting_input" => Self::AwaitingInput,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "timed_out" => Self::TimedOut,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
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

    pub(crate) fn set_unavailable(&mut self) {
        self.snapshot.byok.clear();
        for row in &mut self.snapshot.cli_bridge {
            row.auth_status = CloudCliAuthStatus::Unavailable;
        }
    }

    pub(crate) fn snapshot_projection(&self) -> Self {
        Self {
            snapshot: self.snapshot.clone(),
            key_drafts: Vec::new(),
            messages: self.messages.clone(),
            pending_cli_login_confirmation: self.pending_cli_login_confirmation.clone(),
            login_panel: self.login_panel.as_ref().map(|panel| CloudCliLoginPanel {
                provider: panel.provider.clone(),
                label: panel.label.clone(),
                session_id: panel.session_id.clone(),
                state: panel.state,
                transcript: panel.transcript.clone(),
                input: String::new(),
                error: panel.error.clone(),
            }),
        }
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
            zeroize::Zeroize::zeroize(&mut panel.input);
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

/// The production settings operation whose last attempt failed. The app retains this typed identity so
/// Retry never guesses between reloading durable state and re-sending the current in-memory edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPersistenceOperation {
    Load,
    Save,
    /// MT-072 (FAIL_V2): a failed editor-preference write on the canonical PreferenceRecord route. Retry
    /// re-dispatches the exact retained write so a transient backend failure does not drop the edit.
    Preference,
}

/// What the dialog wants the shell to do after a frame.
///
/// Returned by [`show`]. The shell matches on it: a wired change variant updates
/// `workspace_settings` / `current_theme` / `view_mode` and persists via `PUT /workspaces/{id}/settings`;
/// `Close` clears the open flag; `None` leaves the dialog open. At most one outcome per frame (a single
/// control interaction), so the shell never has to reconcile two simultaneous changes.
///
/// `PartialEq` only (not `Eq`): the MT-072 [`SettingsOutcome::EditorPrefsChanged`] carries an
/// [`crate::workspace_settings::EditorPrefs`] whose `editor_font_size` is an `f32`. Every call site
/// compares with `==` (which `PartialEq` satisfies); no `Eq`-requiring use exists.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsOutcome {
    EngineProfilesRefreshRequested,
    EngineProfileMutationRequested {
        token: EngineProfileUiToken,
        request: EngineProfileMutationRequestV1,
    },
    /// Nothing happened this frame; keep the dialog open.
    None,
    CloudByokKeySaveRequested {
        provider: String,
    },
    CloudByokKeyRemoveRequested {
        provider: String,
    },
    CliBridgeLoginRequested {
        provider: String,
    },
    CliBridgeLoginInputSubmitted {
        provider: String,
        input: String,
    },
    CliBridgeLoginStopRequested {
        provider: String,
    },
    CliBridgeLoginPanelClosed {
        provider: String,
    },
    OpenModuleFeature(PaneType),
    PopOut,
    Redock,
    /// The Theme / appearance ComboBox selected a (different) theme. WIRED.
    ThemeChanged(WorkspaceTheme),
    /// The View Mode ComboBox selected a (different) mode. WIRED.
    ViewModeChanged(SettingsViewMode),
    /// A valid IANA timezone was selected for Calendar local-date projection.
    CalendarTimezoneChanged(String),
    /// A keybinding's chord changed to a NON-conflicting value (already normalized). WIRED — the shell
    /// persists it. A conflicting draft does NOT emit this (it only shows the banner), so a conflicting
    /// binding is never saved (AC6).
    KeybindingChanged { action_id: String, chord: String },
    /// A keybinding Reset button was clicked; restore the action's default chord. WIRED.
    KeybindingReset { action_id: String },
    /// The Swarm board default-open checkbox was toggled. WIRED.
    SwarmBoardDefaultOpenChanged(bool),
    SwarmModelSessionsMaxConcurrentChanged(usize),
    /// The Reset panes & drawers button was clicked (same action as VIEW > Reset Layout). WIRED.
    ResetLayout,
    /// Settings -> Model Session requested the same launch dialog as Run / command palette. WIRED.
    OpenModelSessionLaunch,
    /// MT-072: the Editor prefs group changed (font/tab/insert-spaces/wrap/whitespace). WIRED — the
    /// shell stores the new `editor_prefs` and persists via the existing debounced `PUT`.
    EditorPrefsChanged(crate::workspace_settings::EditorPrefs),
    /// MT-072: the Syntax palette changed (mode and/or a Custom swatch). WIRED.
    SyntaxPaletteChanged(crate::workspace_settings::SyntaxPalette),
    /// MT-072: an editor keybinding override changed to a non-empty chord. WIRED — stored in the
    /// SEPARATE `editor_keybindings` list (not the WP-011 `keybindings` map — RISK-001).
    EditorKeybindingChanged { action_id: String, chord: String },
    /// MT-072: an editor keybinding override was reset to its built-in default (override removed). WIRED.
    EditorKeybindingReset { action_id: String },
    /// MT-072 (FAIL_V2 / SET-UI-002): reset every editor scalar preference to its registry default via
    /// the canonical `.../reset` route. WIRED.
    EditorPrefsReset,
    /// MT-072 (FAIL_V2 / SET-UI-002): reset the syntax palette (mode + custom colors) to defaults. WIRED.
    SyntaxPaletteReset,
    /// MT-102: Settings -> Diagnostics requested a Visual Debugger worksurface JSON dump.
    WorksurfaceInspectorDumpRequested,
    /// Retry the exact failed GET or PUT while retaining the current in-memory settings.
    RetryPersistence,
    /// The user dismissed the dialog (Escape, the Close button, or a backdrop click). The shell clears
    /// the open flag.
    Close,
}

/// Result of one Settings frame. The shell retains `query` so its fresh, side-effect-free canonical
/// MCP capture context reproduces the same filtered dialog the operator and model just changed.
pub struct SettingsFrame {
    pub orphaned_engine_profile_operations: Vec<EngineProfileUiToken>,
    pub outcome: SettingsOutcome,
    pub query: String,
}

#[derive(Debug, Clone, Default)]
pub struct SwarmModelSessionsSettingsState {
    pub snapshot: Option<crate::backend_client::SwarmConcurrencySnapshot>,
    pub updating: bool,
    pub error: Option<String>,
}

/// Read-only inputs the dialog renders from (the live settings + the open generation). The dialog never
/// borrows `&mut HandshakeApp`; the shell applies the returned [`SettingsOutcome`].
pub struct SettingsView<'a> {
    pub engine_profiles: &'a mut EngineProfilesSettingsState,
    pub registered_features: &'a [PaneType],
    pub cloud: &'a mut CloudModelsSettingsState,
    pub swarm_model_sessions: &'a SwarmModelSessionsSettingsState,
    /// Monotonic open generation; a new value resets the dialog's transient state.
    pub open_count: u64,
    /// The live workspace settings (theme, keybindings, view mode, swarm board flag, MT-072 editor
    /// prefs / syntax palette / editor keybindings).
    pub settings: &'a WorkspaceSettingsState,
    /// The last transient persistence error, if any, surfaced on the status row.
    pub persist_error: Option<&'a str>,
    /// Typed failed operation paired with `persist_error`; drives the exact Retry action.
    pub persist_retry_operation: Option<SettingsPersistenceOperation>,
    /// WP-KERNEL-012 MT-087: the live `internal_diagnostics` projection the shell rebuilt this frame
    /// (heartbeat / frame-stats / resource / GPU / dropped / ring-status). Rendered by the Settings ->
    /// Diagnostics section (§5.8.4). The panel reads the last-N events directly from the process-global
    /// recorder; this carries the app-owned producer state the recorder global does not hold. The
    /// section is render-only (a pure projection — no setting to mutate), so this is the ONLY new input.
    pub diagnostics: &'a crate::diagnostics::DiagnosticsView,
    /// Keep Diagnostics expanded for the shell-latched backend-loss Settings session. The live frame and
    /// the side-effect-free model-navigation snapshot use distinct egui contexts, so down/recovered
    /// observability cannot depend on either context's private CollapsingHeader memory.
    pub diagnostics_force_open: bool,
    /// WP-KERNEL-012 MT-087: the active resolved palette (theme tokens) the Diagnostics panel paints
    /// with, so the panel reads a theme token for every colour and never a literal (CONTROL-4).
    pub palette: &'a crate::theme::HsPalette,
    /// MT-102 Visual Debugger: transient status for the last Settings -> Diagnostics worksurface dump.
    pub worksurface_inspector_last_dump: Option<&'a str>,
    /// WP-KERNEL-012 MT-072 (validation V4 item 8): the resolved SET-REC-001 `source` + `revision` for
    /// every editor preference id, as last read from the canonical SurrealDB PreferenceRecord surface.
    /// Rendered as a per-control chip in the Editor / Syntax / Keybindings sections so default-vs-custom
    /// provenance is visible next to the value and the reset affordance.
    pub preference_provenance: &'a crate::settings_editor_section::EditorPreferenceProvenance,
}

/// MT-072: the read-only auto-save interval string surfaced (NOT owned) inside the Editor section so the
/// editor prefs read as a complete group. WP-011 does NOT persist an auto-save-interval field today (the
/// MT contract assumed one exists; it does not — see the MT handoff note), so this honest fixed label
/// states the current build's behavior rather than inventing a new persisted field. The dialog passes it
/// to the Editor section.
pub const AUTO_SAVE_INTERVAL_LABEL: &str = "On focus loss / close (not yet interval-configurable)";

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
    /// In-progress IANA timezone text, committed only through the adjacent Apply action.
    calendar_tzid_draft: String,
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
            swarm_model_sessions_max_concurrent: live.swarm_model_sessions_max_concurrent,
            calendar_view_tzid: live.calendar_view_tzid.clone(),
            // MT-072: this snapshot is used ONLY for APP-level keybinding conflict detection; the
            // editor settings ride along unchanged from the live state so the snapshot stays a faithful
            // copy of everything the conflict check does not touch.
            editor_prefs: live.editor_prefs,
            syntax_palette: live.syntax_palette.clone(),
            editor_keybindings: live.editor_keybindings.clone(),
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
pub fn show(ctx: &egui::Context, view: SettingsView<'_>) -> SettingsFrame {
    show_host(ctx, view, false, None)
}

pub fn show_detached(ctx: &egui::Context, view: SettingsView<'_>) -> SettingsFrame {
    show_host(ctx, view, true, None)
}

/// Render the existing Settings content inside a caller-owned window or panel.
/// The caller owns geometry and backdrop interaction; this path creates no independent container.
/// Detached mode retains Re-dock and non-modal accessibility; embedded mode retains Pop out.
/// All Settings state, private-field cleanup and typed outcomes use the same content owner.
pub fn show_in_ui(ui: &mut egui::Ui, view: SettingsView<'_>, detached: bool) -> SettingsFrame {
    let ctx = ui.ctx().clone();
    show_host(&ctx, view, detached, Some(ui))
}

fn show_host(
    ctx: &egui::Context,
    mut view: SettingsView<'_>,
    detached: bool,
    content_ui: Option<&mut egui::Ui>,
) -> SettingsFrame {
    let mounted = content_ui.is_some();
    view.engine_profiles.begin_open(view.open_count);
    let state_id = egui::Id::new("settings.state");
    let mut state: DialogState = ctx
        .data_mut(|d| d.get_temp::<DialogState>(state_id))
        .unwrap_or_default();

    // MT-072: the editor sections widget (catalog + per-open keybinding-row drafts), persisted in egui
    // temp memory across frames keyed to the dialog so the row drafts survive between frames and reset on
    // (re-)open (the `open_count`-keyed reseed lives inside the widget).
    let editor_section_id = egui::Id::new("settings.editor-section");
    let mut editor_section: crate::settings_editor_section::EditorSettingsSection = ctx
        .data_mut(|d| {
            d.get_temp::<crate::settings_editor_section::EditorSettingsSection>(editor_section_id)
        })
        .unwrap_or_default();

    // Reset transient state on (re-)open: a new open generation clears the query + reseeds drafts from
    // the live settings so a re-open never shows the previous session's text/drafts.
    if state.open_count != view.open_count {
        clear_cloud_key_state(ctx, view.cloud);
        state = DialogState {
            open_count: view.open_count,
            query: String::new(),
            drafts: APP_KEYBINDING_ACTIONS
                .iter()
                .map(|action| {
                    (
                        action.id.to_owned(),
                        view.settings
                            .chord_for(action.id)
                            .unwrap_or(action.default_chord)
                            .to_owned(),
                    )
                })
                .collect(),
            focus_requested: false,
            calendar_tzid_draft: view.settings.calendar_view_tzid.clone(),
        };
    }

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
            view.engine_profiles.exit();
            return SettingsFrame {
                orphaned_engine_profile_operations: view.engine_profiles.take_orphaned(),
                outcome: SettingsOutcome::Close,
                query: state.query,
            };
        }
        // A popup/combo is open: let egui's own Escape handling close just the popup this frame; the
        // dialog stays open. Fall through to render so the combo gets its frame to close.
    }

    // ── Backdrop: a full-screen interactable Area BEHIND the window; a click on it dismisses. ──
    let screen = ctx.content_rect();
    let backdrop = (!detached && !mounted).then(|| egui::Area::new(egui::Id::new("settings.backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(96));
            response
        }));
    if backdrop.is_some_and(|backdrop| backdrop.inner.clicked()) {
        persist(ctx, state_id, &state);
        clear_cloud_key_state(ctx, view.cloud);
        view.engine_profiles.exit();
        return SettingsFrame {
            orphaned_engine_profile_operations: view.engine_profiles.take_orphaned(),
            outcome: SettingsOutcome::Close,
            query: state.query,
        };
    }

    let search_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_SEARCH_NODE_ID) };
    let dialog_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_DIALOG_NODE_ID) };
    let list_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_LIST_NODE_ID) };

    let mut outcome = SettingsOutcome::None;

    let mut render_body = |ui: &mut egui::Ui| {
            // Header: eyebrow + title on the left, Close button on the right.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("GLOBAL").small().weak());
                    ui.label(egui::RichText::new("Settings").heading());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let host = ui.button(if detached { "Re-dock" } else { "Pop out" });
                    set_author_id(ui, host.id, if detached { SETTINGS_REDOCK_AUTHOR_ID } else { SETTINGS_POPOUT_AUTHOR_ID });
                    if host.clicked() { outcome = if detached { SettingsOutcome::Redock } else { SettingsOutcome::PopOut }; }
                    let close = ui.button("Close");
                    set_author_id(ui, close.id, CLOSE_AUTHOR_ID);
                    if close.clicked() {
                        outcome = SettingsOutcome::Close;
                    }
                });
            });
            ui.add_space(6.0);

            ui.add_enabled_ui(outcome == SettingsOutcome::None, |ui| {
                render_settings_tabs(ui, view.engine_profiles);
            });

            // Search input, pinned to the fixed search id so its AccessKit NodeId is stable.
            ui.label(egui::RichText::new("Search settings").small().weak());
            // Drain a target-specific model replacement before TextEdit builds its value/tree for
            // this frame. Reading it after `ui.add(edit)` leaves the current widget snapshot on the
            // pre-action value and can cause the action channel's post-render receipt to reject an
            // otherwise valid canonical Argus SetValue.
            let previous_query = state.query.clone();
            if let Some(value) = crate::mcp::accesskit_string_set_value(ui, search_egui_id) {
                state.query = value;
                ui.ctx().request_repaint();
            }
            let edit = egui::TextEdit::singleline(&mut state.query)
                .id(search_egui_id)
                .hint_text("Theme, quick switcher, terminal...")
                .desired_width(f32::INFINITY);
            let _edit_response = ui.add(edit);
            if !state.focus_requested {
                _edit_response.request_focus();
                state.focus_requested = true;
            }
            if previous_query != state.query {
                let was_general = view.engine_profiles.is_general_tab();
                let navigated = view.engine_profiles.tab != EngineProfilesTab::General
                    || view.engine_profiles.section.is_some();
                view.engine_profiles.tab = EngineProfilesTab::General;
                view.engine_profiles.section = None;
                view.engine_profiles.navigation_error = None;
                if navigated {
                    record_settings_navigation(ui, was_general);
                }
            }
            emit_search_node(ui.ctx(), search_egui_id);
            if !state.query.trim().is_empty()
                && (view.engine_profiles.tab != EngineProfilesTab::General
                    || view.engine_profiles.section.is_some())
            {
                let label = "Search paused. Edit Search settings or select General to search all settings.";
                let response = ui.add(egui::Label::new(label).wrap());
                set_author_id_and_label(ui, response.id, "settings.search.scope", label);
            }

            // Persistence error row (HBR: important state visible; surfaces a save/load failure).
            if let Some(err) = view.persist_error {
                ui.add_space(4.0);
                let status = ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("Settings sync error: {err}"),
                );
                set_author_id(ui, status.id, SETTINGS_PERSIST_ERROR_AUTHOR_ID);
                if view.persist_retry_operation.is_some() {
                    let retry_label = match view.persist_retry_operation {
                        Some(SettingsPersistenceOperation::Load) => "Retry settings load",
                        Some(SettingsPersistenceOperation::Save) => "Retry settings save",
                        Some(SettingsPersistenceOperation::Preference) => "Retry saving preference",
                        None => unreachable!(),
                    };
                    let retry = ui.button(retry_label);
                    emit_enabled_retry_node(ui, retry.id);
                    if retry.clicked() && outcome == SettingsOutcome::None {
                        outcome = SettingsOutcome::RetryPersistence;
                    }
                }
            }

            ui.add_space(6.0);

            let query = state.query.trim().to_lowercase();

            // The scrollable body region (Role::Group container at the fixed list id).
            ui.push_id(list_egui_id, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(if detached || mounted { ui.available_height() } else { ui.available_height().min(440.0) }.max(1.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if matches!(view.engine_profiles.tab, EngineProfilesTab::General | EngineProfilesTab::SharedFeature)
                            && view.engine_profiles.section.is_none()
                        {
                            render_registered_features(ui, view.registered_features, &query, view.engine_profiles);
                        }
                        match view.engine_profiles.tab {
                            EngineProfilesTab::General => {
                                if view.engine_profiles.section.is_some() {
                                    ui.label("Showing the selected section.");
                                }
                                let section_query = if view.engine_profiles.section.is_some() { "" } else { query.as_str() };
                                outcome = render_sections(
                                    ui, section_query, &mut state, &mut view,
                                    &mut editor_section, outcome.clone(),
                                );
                            }
                            EngineProfilesTab::EngineProfiles if outcome == SettingsOutcome::None => {
                                outcome = render_engine_profiles_tab(ui, view.engine_profiles);
                            }
                            EngineProfilesTab::Module(module) if outcome == SettingsOutcome::None => {
                                outcome = render_module_settings(ui, module, view.engine_profiles);
                            }
                            EngineProfilesTab::SharedFeature if outcome == SettingsOutcome::None => {
                                if let Some(feature) = view.engine_profiles.feature.clone() {
                                    outcome = render_feature_settings(ui, feature, view.engine_profiles, false);
                                }
                            }
                            _ => {}
                        }
                    });
            });
            emit_list_node(ui.ctx(), list_egui_id);
    };
    if let Some(ui) = content_ui {
        render_body(ui);
    } else if detached {
        egui::CentralPanel::default().show(ctx, &mut render_body);
    } else {
        egui::Window::new("settings")
            .id(egui::Id::new("settings.window"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([480.0, 560.0])
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .show(ctx, &mut render_body);
    }

    // The dialog root container node (Role::Dialog, modal) attached to the fixed dialog id so an
    // out-of-process model finds the modal by `settings.dialog`. Emitted each open frame.
    emit_dialog_node(ctx, dialog_egui_id, !detached);

    let query = state.query.clone();
    persist(ctx, state_id, &state);
    // MT-072: store the editor section (with its updated keybinding-row drafts) back to temp memory.
    ctx.data_mut(|d| d.insert_temp(editor_section_id, editor_section));
    if matches!(
        outcome,
        SettingsOutcome::Close | SettingsOutcome::PopOut | SettingsOutcome::Redock
    ) {
        clear_cloud_key_state(ctx, view.cloud);
    }
    if outcome == SettingsOutcome::Close {
        view.engine_profiles.exit();
    }
    SettingsFrame {
        orphaned_engine_profile_operations: view.engine_profiles.take_orphaned(),
        outcome, query,
    }
}


pub fn module_settings_author_id(module: ModuleId) -> String {
    format!("settings.module.{}", module.definition().data_id)
}

pub fn module_feature_author_id(module: ModuleId, feature: &PaneType) -> String {
    format!("settings.feature.{}.{}", module.definition().data_id, pane_type_tab_id(feature))
}

pub fn feature_open_author_id(feature: &PaneType) -> String {
    format!("settings.feature.open.{}", pane_type_tab_id(feature))
}

pub fn feature_from_open_author_id(author_id: &str) -> Option<PaneType> {
    let tab_id = author_id.strip_prefix("settings.feature.open.")?;
    MODULE_DEFINITIONS.iter().flat_map(|module| module.tabs.iter())
        .find(|feature| pane_type_tab_id(feature) == tab_id && feature_has_context_free_route(feature)).cloned()
}

fn feature_has_context_free_route(feature: &PaneType) -> bool {
    !matches!(feature, PaneType::CalendarEvent | PaneType::LoomBlock | PaneType::KernelDcc
        | PaneType::Wp1OrchestrationConsole | PaneType::Placeholder(_))
}

fn section_visible(selected: Option<&str>, section: &str) -> bool {
    selected.is_none() || selected == Some(section)
}

fn record_settings_navigation(ui: &egui::Ui, unchanged_general_posture: bool) {
    if unchanged_general_posture {
        crate::diagnostics::gui_events::record_gui_observation(
            crate::diagnostics::gui_events::GuiDiagnosticOperation::SettingsTab,
            crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted,
        );
    }
    ui.ctx().request_repaint();
}

fn render_settings_tabs(ui: &mut egui::Ui, state: &mut EngineProfilesSettingsState) {
    ui.horizontal_wrapped(|ui| {
        for (tab, label, author) in [
            (EngineProfilesTab::General, "General", "settings.tab.general"),
            (EngineProfilesTab::EngineProfiles, "Engine Profiles", "settings.tab.engine-profiles"),
        ] {
            let selected = state.tab == tab && (tab != EngineProfilesTab::General || state.section.is_none());
            let response = ui.selectable_label(selected, label);
            set_author_id(ui, response.id, author);
            if response.clicked() && !selected {
                let was_general = state.is_general_tab();
                state.tab = tab;
                state.section = None;
                state.navigation_error = None;
                record_settings_navigation(ui, was_general == state.is_general_tab());
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), author);
            }
        }
        for module in &MODULE_DEFINITIONS {
            let selected = state.tab == EngineProfilesTab::Module(module.id);
            let response = ui.selectable_label(selected, module.label);
            let author = module_settings_author_id(module.id);
            set_author_id(ui, response.id, &author);
            if response.clicked() && !selected {
                let was_general = state.is_general_tab();
                state.tab = EngineProfilesTab::Module(module.id);
                state.feature = Some(module.default_tab.clone());
                state.section = None;
                state.navigation_error = None;
                record_settings_navigation(ui, was_general == state.is_general_tab());
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
            }
        }
    });
}

fn shared_section_link(ui: &mut egui::Ui, state: &mut EngineProfilesSettingsState, key: &'static str, label: &str) {
    let response = ui.button(label);
    let author = format!("settings.feature.section.{key}");
    set_author_id(ui, response.id, &author);
    if response.clicked() {
        let was_general = state.is_general_tab();
        state.tab = EngineProfilesTab::General;
        state.section = Some(key);
        state.navigation_error = None;
        record_settings_navigation(ui, was_general == state.is_general_tab());
        crate::mcp::argus::acknowledge_action_effect_allow_target_transition(ui.ctx(), &author);
    }
}

pub fn registered_feature_author_id(feature: &PaneType) -> String {
    match feature {
        PaneType::Placeholder(label) => format!("settings.feature.shared.placeholder.{label}"),
        _ => format!("settings.feature.shared.{}", pane_type_tab_id(feature)),
    }
}

fn render_registered_features(ui: &mut egui::Ui, registered: &[PaneType], query: &str, state: &mut EngineProfilesSettingsState) {
    let matches: Vec<_> = registered.iter().filter(|feature| {
        let assigned = MODULE_DEFINITIONS.iter().any(|module| module.tabs.contains(feature));
        if query.is_empty() { !assigned } else { feature.label().to_lowercase().contains(query) }
    }).collect();
    if matches.is_empty() { return; }
    let header = egui::CollapsingHeader::new("Features")
        .default_open(false)
        .open((!query.is_empty() || state.tab == EngineProfilesTab::SharedFeature).then_some(true))
        .show(ui, |ui| {
            ui.label("Select a feature to view its settings.");
            ui.horizontal_wrapped(|ui| {
                for feature in matches {
                    let selected = state.tab == EngineProfilesTab::SharedFeature
                        && state.feature.as_ref() == Some(feature);
                    let response = ui.selectable_label(selected, feature.label());
                    let author = registered_feature_author_id(feature);
                    set_author_id(ui, response.id, &author);
                    if response.clicked() && !selected {
                        let was_general = state.is_general_tab();
                        state.tab = EngineProfilesTab::SharedFeature;
                        state.feature = Some(feature.clone());
                        state.section = None;
                        state.navigation_error = None;
                        record_settings_navigation(ui, was_general == state.is_general_tab());
                        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
                    }
                }
            });
        });
    set_author_id(ui, header.header_response.id, "settings.features.shared");
    if header.header_response.clicked() && query.is_empty() && state.tab != EngineProfilesTab::SharedFeature {
        record_settings_navigation(ui, true);
        crate::mcp::argus::acknowledge_action_effect(ui.ctx(), "settings.features.shared");
    }
}

fn render_module_settings(ui: &mut egui::Ui, module: ModuleId, state: &mut EngineProfilesSettingsState) -> SettingsOutcome {
    let definition = module.definition();
    ui.horizontal_wrapped(|ui| {
        for feature in definition.tabs {
            let selected = state.feature.as_ref() == Some(feature);
            let response = ui.selectable_label(selected, feature.label());
            let author = module_feature_author_id(module, feature);
            set_author_id(ui, response.id, &author);
            if response.clicked() && !selected {
                state.feature = Some(feature.clone());
                state.navigation_error = None;
                record_settings_navigation(ui, true);
                crate::mcp::argus::acknowledge_action_effect(ui.ctx(), &author);
            }
        }
    });
    let feature = state.feature.clone().unwrap_or_else(|| definition.default_tab.clone());
    let can_open = feature_has_context_free_route(&feature);
    render_feature_settings(ui, feature, state, can_open)
}

fn render_feature_settings(ui: &mut egui::Ui, feature: PaneType, state: &mut EngineProfilesSettingsState, can_open: bool) -> SettingsOutcome {
    ui.heading(feature.label());
    let note = match &feature {
        PaneType::CodeSymbol => "Font size is shared with rich documents. Other code settings remain in Editor, Syntax and Keybindings.",
        PaneType::LoomWikiPage => "Shared Editor and Keybindings settings. Generated Wiki Projection is read-only; annotations remain additive.",
        PaneType::LoomDailyJournal => "Calendar timezone is in Appearance. Edit journal content in its pane.",
        PaneType::CalendarEvent => "Select an event to open its details. Calendar timezone is in Appearance.",
        PaneType::LoomSearchV2 => "Query and facet options are in Notes Search.",
        PaneType::FindInFiles => "Search and replace-preview options are in Find in Files.",
        PaneType::RuntimeChat => "Chat is unavailable. No dedicated preference.",
        PaneType::OperatorChatLaunch => "Choose models, profiles and folders in Operator Chat / Launch. Provider access and concurrency use shared settings.",
        PaneType::Wp1OrchestrationConsole => "Opening from Settings is unavailable. Filters remain in the console.",
        PaneType::AtelierEditor | PaneType::MediaDownloader => "Open Atelier and expand Settings / Defaults for Atelier, CKC, PoseKit and Ingest defaults.",
        PaneType::ModelRuntime => "Model selection and lifecycle controls are in Model Runtime. Engine Profiles and Cloud Models are shared settings.",
        PaneType::Swarm => "Concurrency is in Swarm settings. This native board view is unavailable; the startup option is saved but not applied.",
        PaneType::SwarmLaneDiagnostics => "Run, lane and message filters are in Swarm Lane Diagnostics.",
        PaneType::FlightRecorder => "No dedicated preference. Flight Recorder follows the active workspace and actor.",
        PaneType::VisualDebugger => "Use Visual Debugger or shared Diagnostics. Diagnostics is read-only.",
        PaneType::KernelDcc | PaneType::InferenceLab | PaneType::FontManager => "Configuration is unavailable in this native view.",
        _ => "Settings controls are unavailable here.",
    };
    let response = ui.add(egui::Label::new(note).wrap());
    set_author_id_and_label(ui, response.id, "settings.feature.disposition", note);
    match &feature {
        PaneType::CodeSymbol => {
            ui.horizontal_wrapped(|ui| {
                shared_section_link(ui, state, "editor", "Editor");
                shared_section_link(ui, state, "syntax", "Syntax");
                shared_section_link(ui, state, "keybindings", "Keybindings");
            });
        }
        PaneType::LoomWikiPage => {
            ui.horizontal_wrapped(|ui| {
                shared_section_link(ui, state, "editor", "Editor");
                shared_section_link(ui, state, "keybindings", "Keybindings");
            });
        }
        PaneType::LoomDailyJournal | PaneType::CalendarEvent => shared_section_link(ui, state, "appearance", "Appearance"),
        PaneType::Swarm => shared_section_link(ui, state, "swarm", "Swarm settings"),
        PaneType::OperatorChatLaunch => {
            ui.horizontal_wrapped(|ui| {
                shared_section_link(ui, state, "cloud-models", "Cloud Models");
                shared_section_link(ui, state, "swarm", "Swarm settings");
            });
        }
        PaneType::ModelRuntime => {
            ui.horizontal_wrapped(|ui| {
                shared_section_link(ui, state, "cloud-models", "Cloud Models");
                shared_section_link(ui, state, "model-session", "Model Session");
            });
        }
        PaneType::SwarmLaneDiagnostics | PaneType::FlightRecorder | PaneType::VisualDebugger => {
            shared_section_link(ui, state, "diagnostics", "Diagnostics");
        }
        PaneType::AtelierEditor | PaneType::MediaDownloader => {
            ui.label("Defaults: landing tab, book mode, PoseKit framing and markers, and Ingest tags and policy. Facial review stays in Ingest.");
        }
        _ => {}
    }
    if let Some(error) = state.navigation_error {
        let response = ui.colored_label(ui.visuals().error_fg_color, error);
        set_author_id_and_label(ui, response.id, "settings.navigation.error", error);
    }
    if can_open {
        let response = ui.button(format!("Open {}", feature.label()));
        set_author_id(ui, response.id, &feature_open_author_id(&feature));
        if response.clicked() {
            return SettingsOutcome::OpenModuleFeature(feature);
        }
    } else {
        ui.label("Opening is unavailable here.");
    }
    SettingsOutcome::None
}

/// Seed the fresh canonical MCP capture context with the query last rendered by the live Settings
/// context. The capture pass is intentionally a different egui context, so context-local transient
/// memory must be copied explicitly or canonical Argus sees an empty query after a successful SetValue.
pub fn prime_query(ctx: &egui::Context, open_count: u64, query: impl Into<String>) {
    let state_id = egui::Id::new("settings.state");
    let mut state: DialogState = ctx
        .data_mut(|d| d.get_temp::<DialogState>(state_id))
        .unwrap_or_default();
    state.open_count = open_count;
    state.query = query.into();
    state.focus_requested = false;
    persist(ctx, state_id, &state);
}

/// Render every settings section in order, threading `outcome` so the first interaction this frame wins
/// (we never overwrite a Close already chosen in the header). Returns the (possibly updated) outcome.
fn render_sections(
    ui: &mut egui::Ui,
    query: &str,
    state: &mut DialogState,
    view: &mut SettingsView<'_>,
    editor_section: &mut crate::settings_editor_section::EditorSettingsSection,
    mut outcome: SettingsOutcome,
) -> SettingsOutcome {
    let settings = view.settings;
    let selected_section = view.engine_profiles.section;

    // MT-072: the read-only editor view the three new sections render from (the live settings + the
    // surfaced auto-save interval label).
    let editor_view = crate::settings_editor_section::EditorSettingsView {
        settings,
        auto_save_interval_label: AUTO_SAVE_INTERVAL_LABEL,
        provenance: view.preference_provenance,
    };

    if section_visible(selected_section, "cloud-models") && setting_matches_query(
        query,
        &[
            "cloud",
            "models",
            "byok",
            "anthropic",
            "openai",
            "claude",
            "codex",
            "login",
            "consent",
        ],
    ) {
        let header = egui::CollapsingHeader::new("Cloud Models")
            .default_open(true)
            .open((!query.is_empty() || selected_section.is_some()).then_some(true))
            .show(ui, |ui| {
                outcome = render_cloud_models_body(ui, view.cloud, outcome.clone());
            });
        set_author_id(
            ui,
            header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}cloud-models"),
        );
    }

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
            "calendar",
            "timezone",
            "iana",
        ],
    );
    let show_theme_row = setting_matches_query(query, &["appearance", "theme", "light", "dark"]);
    let show_view_mode_row =
        setting_matches_query(query, &["appearance", "view", "mode", "sfw", "nsfw"]);
    let show_calendar_timezone_row = setting_matches_query(
        query,
        &["appearance", "calendar", "timezone", "iana", "tzid"],
    );
    if section_visible(selected_section, "appearance") && show_appearance {
        let appearance_header = egui::CollapsingHeader::new("Appearance")
            .default_open(true)
            .open(selected_section.map(|_| true))
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
                        set_author_id(ui, combo.response.id, VIEW_MODE_COMBO_AUTHOR_ID);
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
                        set_author_id(ui, combo.response.id, THEME_COMBO_AUTHOR_ID);
                        if selected != current && outcome == SettingsOutcome::None {
                            outcome = SettingsOutcome::ThemeChanged(selected);
                        }
                    });
                }
                if show_calendar_timezone_row {
                    ui.horizontal(|ui| {
                        ui.label("Calendar timezone");
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut state.calendar_tzid_draft)
                                .desired_width(180.0),
                        );
                        set_author_id_and_label(
                            ui,
                            input.id,
                            CALENDAR_TIMEZONE_AUTHOR_ID,
                            "Calendar timezone (IANA)",
                        );
                        let valid = state.calendar_tzid_draft.parse::<chrono_tz::Tz>().is_ok();
                        let apply = ui.add_enabled(valid, egui::Button::new("Apply"));
                        set_author_id(ui, apply.id, CALENDAR_TIMEZONE_APPLY_AUTHOR_ID);
                        if apply.clicked()
                            && state.calendar_tzid_draft != settings.calendar_view_tzid
                            && outcome == SettingsOutcome::None
                        {
                            outcome = SettingsOutcome::CalendarTimezoneChanged(
                                state.calendar_tzid_draft.clone(),
                            );
                        }
                        if !valid {
                            ui.colored_label(ui.visuals().error_fg_color, "Invalid IANA timezone");
                        }
                    });
                }
            });
        set_author_id(
            ui,
            appearance_header.header_response.id,
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
    if section_visible(selected_section, "keybindings") && show_keybindings {
        let keybindings_header = egui::CollapsingHeader::new("Keybindings")
            .default_open(true)
            .open(selected_section.map(|_| true))
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
                            set_author_id(
                                ui,
                                reset.id,
                                &format!("{KEYBINDING_RESET_AUTHOR_ID_PREFIX}{}", action.id),
                            );
                            if reset.clicked() && outcome == SettingsOutcome::None {
                                // Reflect the default in the draft immediately, then emit the reset.
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

                // ── MT-072: EXTEND this SAME Keybindings section in place (NOT a 2nd section —
                //    RISK-005) with the editor-specific actions (MT-010 code chords + rich-editor
                //    commands). A custom binding overrides the default and persists into the SEPARATE
                //    editor_keybindings list (the backend deny-unknown-validates the app keybindings
                //    map — RISK-001). The conflict banner above is for the APP keybindings; editor
                //    bindings are namespaced (code./rich.) and live in their own list.
                //
                //    The ~85-row editor-action table is wrapped in a NESTED collapsing sub-header that is
                //    CLOSED BY DEFAULT so it does not push the Swarm/Terminal/Layout/About sections below
                //    the scroll viewport (which would make them unreachable by a single pointer click —
                //    the WP-011 reset-layout interaction test relies on Layout staying reachable). It is
                //    still the SAME one Keybindings section (RISK-005); the editor rows render + become
                //    AccessKit-addressable once the sub-header is expanded.
                ui.separator();
                let editor_kb_header = egui::CollapsingHeader::new("Editor actions")
                    .default_open(false)
                    .show(ui, |ui| {
                        let o = editor_section.render_editor_keybindings(
                            ui,
                            &editor_view,
                            query,
                            view.open_count,
                        );
                        if outcome == SettingsOutcome::None {
                            outcome = map_editor_outcome(o);
                        }
                    });
                set_author_id(
                    ui,
                    editor_kb_header.header_response.id,
                    &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}keybindings-editor"),
                );
            });
        set_author_id(
            ui,
            keybindings_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}keybindings"),
        );
    }

    // ── [3] Swarm (wired checkbox + not-yet-wired interval rows) ───────────────────────────────────
    let show_swarm = setting_matches_query(
        query,
        &[
            "swarm", "board", "reconcile", "resource", "poll", "model", "sessions",
            "concurrency", "concurrent",
        ],
    );
    if section_visible(selected_section, "swarm") && show_swarm {
        let swarm_header = egui::CollapsingHeader::new("Swarm")
            .default_open(true)
            .open(selected_section.map(|_| true))
            .show(ui, |ui| {
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
                        outcome = SettingsOutcome::SwarmBoardDefaultOpenChanged(checked);
                    }
                });
                ui.horizontal(|ui| {
                    let swarm = view.swarm_model_sessions;
                    ui.vertical(|ui| {
                        ui.label("Model sessions max concurrent");
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(
                                    "Coordinator budget for running model sessions. Lowering applies cooperatively as live sessions finish.",
                                )
                                .small()
                                .weak(),
                            )
                            .wrap(),
                        );
                        let status_text = match (&swarm.snapshot, &swarm.error) {
                            (Some(snapshot), error) => format!(
                                "Requested: {} · In force: {} · Fully applied: {} · Live sessions: {}{}{}",
                                snapshot.requested,
                                snapshot.max_concurrent,
                                snapshot.fully_applied,
                                snapshot.live_sessions,
                                if swarm.updating { " · Applying…" } else { "" },
                                error
                                    .as_ref()
                                    .map(|message| format!(" · Update failed: {message}"))
                                    .unwrap_or_default(),
                            ),
                            (None, Some(error)) => format!("Coordinator status unavailable: {error}"),
                            (None, None) if swarm.updating => {
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
                        .or_else(|| swarm.snapshot.as_ref().map(|snapshot| snapshot.requested))
                        .unwrap_or(1)
                        .max(1);
                    let mut selected = current;
                    let mut options = vec![1usize, 2, 4, 8, 16, 32, 64, current];
                    options.sort_unstable();
                    options.dedup();
                    ui.add_enabled_ui(!swarm.updating, |ui| {
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
                                let author_id = swarm_model_session_option_author_id(option);
                                set_author_id(ui, response.id, &author_id);
                                if response.clicked() {
                                    crate::mcp::argus::acknowledge_action_effect(
                                        ui.ctx(),
                                        &author_id,
                                    );
                                }
                            }
                        });
                        set_author_id(
                            ui,
                            combo.response.id,
                            SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID,
                        );
                        if combo.response.clicked() {
                            crate::mcp::argus::acknowledge_action_effect(
                                ui.ctx(),
                                SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID,
                            );
                        }
                    });
                    if selected != current && outcome == SettingsOutcome::None {
                        outcome = SettingsOutcome::SwarmModelSessionsMaxConcurrentChanged(selected);
                    }
                });
            });
        set_author_id(
            ui,
            swarm_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}swarm"),
        );
    }

    // ── [4] Terminal (not-yet-wired rows) ──────────────────────────────────────────────────────────
    let show_terminal =
        setting_matches_query(query, &["terminal", "shell", "scrollback", "logging"]);
    if section_visible(selected_section, "terminal") && show_terminal {
        let terminal_header = egui::CollapsingHeader::new("Terminal")
            .default_open(true)
            .open(selected_section.map(|_| true))
            .show(ui, |ui| {
                not_yet_wired_row(ui, &TERMINAL_DEFAULT_SHELL_SETTING);
                not_yet_wired_row(ui, &TERMINAL_MAX_SCROLLBACK_SETTING);
                not_yet_wired_row(ui, &TERMINAL_OUTPUT_LOGGING_SETTING);
            });
        set_author_id(
            ui,
            terminal_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}terminal"),
        );
    }

    // ── [5] Layout (wired Reset panes & drawers button) ────────────────────────────────────────────
    let show_layout = setting_matches_query(query, &["layout", "reset", "panes", "drawers"]);
    if section_visible(selected_section, "layout") && show_layout {
        let layout_header = egui::CollapsingHeader::new("Layout")
            .default_open(true)
            .open(selected_section.map(|_| true))
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
                        outcome = SettingsOutcome::ResetLayout;
                    }
                });
            });
        set_author_id(
            ui,
            layout_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}layout"),
        );
    }

    // ── [5b] Model Session (wired launcher entrypoint; no placeholder settings) ───────────────────
    let show_model_session = setting_matches_query(
        query,
        &["model", "session", "launch", "local", "cloud", "wrapper"],
    );
    if section_visible(selected_section, "model-session") && show_model_session {
        let model_header = egui::CollapsingHeader::new("Model Session")
            .default_open(true)
            .open(selected_section.map(|_| true))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Launch surface");
                        ui.label(
                            egui::RichText::new(
                                "Provider, model id, folder, and wrapper are chosen per launch.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = ui.button("Open launch dialog");
                        set_author_id(ui, btn.id, MODEL_SESSION_OPEN_LAUNCH_AUTHOR_ID);
                        if btn.clicked() && outcome == SettingsOutcome::None {
                            outcome = SettingsOutcome::OpenModelSessionLaunch;
                        }
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Initial provider");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("Local (editable per launch)");
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Initial wrapper");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("repo-folder-wrapper-v1 (editable per launch)");
                    });
                });
            });
        set_author_id(
            ui,
            model_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}model-session"),
        );
    }

    // ── [MT-072 / Editor] Editor preferences (font/tab/insert-spaces/wrap/whitespace) ──────────────
    //
    // MT-072 Fix 2: the editor-related sections render as a CONTIGUOUS group in the contract's structured
    // order — Editor -> Syntax -> About — BEFORE About (they were previously appended AFTER About, which
    // split them out of order). They stay OPEN BY DEFAULT so the operator and AccessKit clients can
    // discover and operate the declared editor options without a hidden prerequisite click; a search
    // query still force-expands a matching section. They render AFTER the WP-011 control
    // sections (Appearance..Model Session), so the reset-layout single-pointer-click test on a higher
    // WP-011 control is unaffected. The editor Keybindings EXTENSION stays IN PLACE inside the one
    // Keybindings section above (RISK-005 — not a 2nd keybindings store), wrapped in its own collapsed
    // sub-header for the same length reason.
    let show_editor = setting_matches_query(
        query,
        &[
            "editor",
            "font",
            "size",
            "tab",
            "spaces",
            "indent",
            "wrap",
            "word",
            "whitespace",
            "render",
            "auto",
            "save",
            "wiki",
            "projection",
            "overlay",
            "runtime",
            "chat",
            "endpointmissing",
            "/chat",
            "transport",
            "flight",
            "recorder",
            "atelier",
            "ckc",
            "stage",
            "route",
        ],
    );
    if section_visible(selected_section, "editor") && show_editor {
        // FORCE open while a search query is active (`.open(Some(true))` overrides egui's persisted
        // collapsed state — `default_open` would be ignored after the user manually closed it). With
        // no query, leave egui's own open/close state alone (`.open(None)`) so that choice sticks.
        let force_open = (!query.is_empty() || selected_section.is_some()).then_some(true);
        let editor_header = egui::CollapsingHeader::new("Editor")
            .default_open(true)
            .open(force_open)
            .show(ui, |ui| {
                let o = editor_section.render_editor_prefs(ui, &editor_view);
                if outcome == SettingsOutcome::None {
                    outcome = map_editor_outcome(o);
                }
            });
        set_author_id(
            ui,
            editor_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}editor"),
        );
    }

    // ── [MT-072 / Syntax] Syntax color-scheme palette editor (Muted | Standard | Custom) ───────────
    let show_syntax = setting_matches_query(
        query,
        &[
            "syntax",
            "palette",
            "color",
            "colour",
            "scheme",
            "muted",
            "standard",
            "custom",
            "keyword",
            "string",
            "comment",
            "highlight",
        ],
    );
    if section_visible(selected_section, "syntax") && show_syntax {
        let force_open = (!query.is_empty() || selected_section.is_some()).then_some(true);
        let syntax_header = egui::CollapsingHeader::new("Syntax")
            .default_open(true)
            .open(force_open)
            .show(ui, |ui| {
                let o = editor_section.render_syntax_palette(ui, &editor_view);
                if outcome == SettingsOutcome::None {
                    outcome = map_editor_outcome(o);
                }
            });
        set_author_id(
            ui,
            syntax_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}syntax"),
        );
    }

    // ── [6] About (app name + REAL Cargo version) ──────────────────────────────────────────────────
    // MT-072 Fix 2: About now renders AFTER the contiguous Editor + Syntax group (it previously rendered
    // BEFORE them). Section content + author_id are unchanged — this is an ordering-only move.
    let show_about = setting_matches_query(query, &["about", "app", "version"]);
    if section_visible(selected_section, "about") && show_about {
        let about_header = egui::CollapsingHeader::new("About")
            .default_open(true)
            .open(selected_section.map(|_| true))
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
        set_author_id(
            ui,
            about_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}about"),
        );
        // TODO MT-0XX: CLI Bridge config panel - see app/src/components/CliBridgeConfigPanel.tsx
    }

    // ── [MT-087 / Diagnostics] In-app Diagnostics panel (§5.8.4 + §10.12.5 three-tier) ─────────────
    //
    // Operator steer 2026-06-27: the in-app Diagnostics Panel lives HERE in the Options/Settings window
    // (Settings -> Diagnostics), NOT as a worksurface feature pane (the worksurface is stripped to
    // notes-only in MT-097). It is a READ-ONLY observability surface: it projects the live
    // internal_diagnostics state (heartbeat MT-084 / frame-time MT-085 / resource+GPU MT-086 / last-N
    // events MT-082 / an honest Tier-3 Palmistry empty-state until MT-093) and mutates NO setting, so
    // unlike the Editor/Syntax sections it returns no outcome the shell applies. Appended LAST (after the
    // editor sections) and collapsed by default so the EXISTING sections' positions stay byte-stable (a
    // single-pointer-click test on a lower control is unaffected) — the same length discipline the
    // MT-072 sections use. A search query auto-expands it.
    let show_diagnostics = setting_matches_query(
        query,
        crate::settings_diagnostics_section::DIAGNOSTICS_SEARCH_KEYWORDS,
    );
    if section_visible(selected_section, "diagnostics") && show_diagnostics {
        let force_open = (!query.is_empty() || selected_section.is_some() || view.diagnostics_force_open).then_some(true);
        let diagnostics_header = egui::CollapsingHeader::new("Diagnostics")
            .default_open(false)
            .open(force_open)
            .show(ui, |ui| {
                let diag_view = crate::settings_diagnostics_section::DiagnosticsSettingsView {
                    diagnostics: view.diagnostics,
                    palette: view.palette,
                    worksurface_inspector_last_dump: view.worksurface_inspector_last_dump,
                };
                let o = crate::settings_diagnostics_section::render(ui, &diag_view);
                if outcome == SettingsOutcome::None
                    && o == crate::settings_diagnostics_section::DiagnosticsSectionOutcome::WorksurfaceInspectorDumpRequested
                {
                    outcome = SettingsOutcome::WorksurfaceInspectorDumpRequested;
                }
            });
        set_author_id(
            ui,
            diagnostics_header.header_response.id,
            &format!("{SECTION_HEADER_AUTHOR_ID_PREFIX}diagnostics"),
        );
    }

    outcome
}

/// MT-072: map an [`crate::settings_editor_section::EditorSectionOutcome`] from the editor sections onto
/// the dialog's [`SettingsOutcome`] the shell already applies + persists. `None` maps to `None`.
fn map_editor_outcome(o: crate::settings_editor_section::EditorSectionOutcome) -> SettingsOutcome {
    use crate::settings_editor_section::EditorSectionOutcome as E;
    match o {
        E::None => SettingsOutcome::None,
        E::EditorPrefsChanged(prefs) => SettingsOutcome::EditorPrefsChanged(prefs),
        E::SyntaxPaletteChanged(p) => SettingsOutcome::SyntaxPaletteChanged(p),
        E::EditorKeybindingChanged { action_id, chord } => {
            SettingsOutcome::EditorKeybindingChanged { action_id, chord }
        }
        E::EditorKeybindingReset { action_id } => {
            SettingsOutcome::EditorKeybindingReset { action_id }
        }
        E::EditorPrefsReset => SettingsOutcome::EditorPrefsReset,
        E::SyntaxPaletteReset => SettingsOutcome::SyntaxPaletteReset,
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
pub const SETTINGS_POPOUT_AUTHOR_ID: &str = "settings.popout";
pub const SETTINGS_REDOCK_AUTHOR_ID: &str = "settings.redock";
pub const SETTINGS_POPOUT_KEY: &str = "settings";
pub const SETTINGS_WINDOW_LABEL: &str = "Settings";

pub(crate) fn clear_cloud_key_state(ctx: &egui::Context, cloud: &mut CloudModelsSettingsState) {
    for provider in cloud.byok_provider_ids_for_clear() {
        // Overwrite the widget's persisted egui state with a default (empty cursor + empty undoer) so the
        // undo history no longer holds a plaintext snapshot of the typed key.
        egui::TextEdit::store_state(
            ctx,
            cloud_byok_key_egui_id(&provider),
            egui::text_edit::TextEditState::default(),
        );
        ctx.accesskit_node_builder(cloud_byok_key_egui_id(&provider), |node| node.clear_value());
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
        ctx.accesskit_node_builder(cloud_cli_login_input_egui_id(&provider), |node| {
            node.clear_value()
        });
    }
}

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
                if let Some(value) = crate::mcp::accesskit_string_set_value(
                    ui,
                    cloud_byok_key_egui_id(&row.provider),
                ) {
                    zeroize::Zeroize::zeroize(&mut **buf);
                    **buf = value;
                    crate::mcp::argus::acknowledge_secret_set_value(ui.ctx(),
                        &cloud_byok_key_author_id(&row.provider), cloud_byok_key_egui_id(&row.provider), buf.as_str());
                }
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
                outcome = SettingsOutcome::CloudByokKeySaveRequested {
                    provider: row.provider.clone(),
                };
            }
            let remove = ui.add_enabled(row.configured, egui::Button::new("Remove"));
            let remove_author = cloud_byok_remove_author_id(&row.provider);
            set_author_id(ui, remove.id, &remove_author);
            if remove.clicked() && outcome == SettingsOutcome::None {
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

        if !panel.state.is_terminal() && !(panel.session_id.is_empty() && panel.error.is_some()) {
            ui.vertical(|ui| {
                if let Some(value) = crate::mcp::accesskit_string_set_value(
                    ui,
                    cloud_cli_login_input_egui_id(&provider),
                ) {
                    zeroize::Zeroize::zeroize(&mut panel.input);
                    panel.input = value;
                    crate::mcp::argus::acknowledge_action_effect(
                        ui.ctx(),
                        &cloud_cli_login_input_author_id(&provider),
                    );
                }
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

/// Publish the visible persistence retry as the enabled button it is on screen. As with the fixed-id
/// search field, egui can retain a disabled bit while a node builder augments the widget for AccessKit;
/// leaving that stale bit makes canonical Argus reject the operator's documented recovery action.
fn emit_enabled_retry_node(ui: &egui::Ui, widget_id: egui::Id) {
    set_author_id(ui, widget_id, SETTINGS_PERSIST_RETRY_AUTHOR_ID);
    ui.ctx().accesskit_node_builder(widget_id, |node| {
        node.clear_disabled();
        node.add_action(accesskit::Action::Click);
    });
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
fn emit_dialog_node(ctx: &egui::Context, dialog_id: egui::Id, modal: bool) {
    ctx.accesskit_node_builder(dialog_id, |node| {
        node.set_role(accesskit::Role::Dialog);
        node.set_author_id(SETTINGS_DIALOG_AUTHOR_ID.to_owned());
        node.set_label("Settings".to_owned());
        if modal {
            node.set_modal();
        }
    });
}

/// Emit the search box address. egui already derived `Role::TextInput`; this adds the stable author_id.
fn emit_search_node(ctx: &egui::Context, search_id: egui::Id) {
    crate::accessibility::emit_interactive_node(ctx, search_id, SETTINGS_SEARCH_AUTHOR_ID);
    ctx.accesskit_node_builder(search_id, |node| {
        // This node is the real editable TextEdit above. egui can retain a disabled bit on the
        // augmented fixed-id node even though the widget is enabled, which makes canonical Argus
        // reject the documented SetValue route. Keep the accessibility state aligned with the live
        // control instead of exposing an honestly editable field as disabled out-of-process.
        node.clear_disabled();
        node.add_action(accesskit::Action::SetValue);
    });
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
        assert_eq!(ENGINE_PROFILE_CREATE_AUTHOR_ID, "settings.engine-profiles.create");
    }
}


use handshake_dexterity::{Device, Engine, EngineProfile, Model, ModelFormat, Preset, Sampling};
use crate::backend_client::{
    EngineProfileSnapshotV1, EngineProfileIdentityV1, EngineProfileRegistryV1,
    EngineProfileMutationV1, EngineProfileMutationRequestV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineProfilesScope {
    pub account_id: String,
    pub principal_id: String,
    pub session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineProfileUiToken {
    pub scope: EngineProfilesScope,
    pub open_count: u64,
    pub editor_generation: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum EngineProfilesTab {
    #[default]
    General,
    EngineProfiles,
    SharedFeature,
    Module(ModuleId),
}

#[derive(Debug, Clone)]
struct EngineProfileEditor {
    expected: Option<EngineProfileSnapshotV1>,
    expected_head_generation: u64,
    registry_generation: u64,
    active_generation: u64,
    profile: EngineProfile,
    inspect_only: bool,
}

impl EngineProfileEditor {
    fn create(registry: &EngineProfileRegistryV1) -> Self {
        Self {
            expected: None,
            expected_head_generation: 0,
            registry_generation: registry.registry_generation,
            active_generation: registry.active_generation,
            inspect_only: false,
            profile: EngineProfile {
                id: String::new(), revision: 1,
                model: Model {
                    id: String::new(), format: ModelFormat::QairtW4a16,
                    content_sha256: String::new(), compiled_context: None,
                },
                engine: Engine::GenieXQairt, device: Device::Npu,
                preset: Preset {
                    name: String::new(), context_limit: 0, max_tokens: 0,
                    sampling: Sampling::Qairt { temperature: 0.0, seed: 0 },
                },
            },
        }
    }

    fn request(&self) -> Option<EngineProfileMutationRequestV1> {
        if self.inspect_only { return None; }
        let mut profile = self.profile.clone();
        if let Some(expected) = &self.expected {
            if profile.id != expected.profile.id { return None; }
            profile.id = expected.profile.id.clone();
            profile.revision = expected.profile.revision.checked_add(1)?;
        } else {
            profile.revision = 1;
        }
        let resolved = handshake_dexterity::Registry::new(vec![profile.clone()])
            .and_then(|registry| registry.resolve(&profile.id)).ok()?;
        let snapshot = EngineProfileSnapshotV1 {
            profile: resolved.profile().clone(),
            fingerprint: resolved.fingerprint().to_owned(),
        };
        let mutation = match &self.expected {
            Some(expected) => EngineProfileMutationV1::NewRevision {
                expected_head: engine_profile_identity(expected),
                expected_head_generation: self.expected_head_generation,
                snapshot,
            },
            None => EngineProfileMutationV1::Create { snapshot },
        };
        Some(EngineProfileMutationRequestV1 {
            request_id: uuid::Uuid::new_v4(),
            reason: "operator_engine_profile_save".to_owned(),
            expected_registry_generation: self.registry_generation,
            expected_active_generation: self.active_generation,
            mutation,
        })
    }

    fn head_request(&self, activate: bool) -> Option<EngineProfileMutationRequestV1> {
        let expected = self.expected.as_ref()?;
        let identity = engine_profile_identity(expected);
        let mutation = if activate {
            EngineProfileMutationV1::Activate {
                expected_head: identity,
                expected_head_generation: self.expected_head_generation,
            }
        } else {
            EngineProfileMutationV1::Tombstone {
                expected_head: identity,
                expected_head_generation: self.expected_head_generation,
            }
        };
        Some(EngineProfileMutationRequestV1 {
            request_id: uuid::Uuid::new_v4(),
            reason: if activate { "operator_engine_profile_activate" }
                else { "operator_engine_profile_delete" }.to_owned(),
            expected_registry_generation: self.registry_generation,
            expected_active_generation: self.active_generation,
            mutation,
        })
    }
}

fn engine_profile_identity(snapshot: &EngineProfileSnapshotV1) -> EngineProfileIdentityV1 {
    EngineProfileIdentityV1 {
        profile_id: snapshot.profile.id.clone(),
        revision: snapshot.profile.revision,
        fingerprint: snapshot.fingerprint.clone(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct EngineProfilesSettingsState {
    scope: Option<EngineProfilesScope>,
    open_count: u64,
    editor_generation: u64,
    sequence: u64,
    exhausted: bool,
    tab: EngineProfilesTab,
    feature: Option<PaneType>,
    section: Option<&'static str>,
    navigation_error: Option<&'static str>,
    pub registry: Option<EngineProfileRegistryV1>,
    refresh_attempted: bool,
    editor: Option<EngineProfileEditor>,
    delete_candidate: Option<EngineProfileEditor>,
    pending: Option<(EngineProfileUiToken, bool)>,
    orphaned: Vec<EngineProfileUiToken>,
    error: Option<String>,
}

impl EngineProfilesSettingsState {
    pub fn snapshot_projection(&self) -> Self { self.clone() }

    pub fn is_general_tab(&self) -> bool { self.tab == EngineProfilesTab::General }

    pub fn navigation_failed(&mut self) {
        crate::diagnostics::gui_events::record_gui_observation(
            crate::diagnostics::gui_events::GuiDiagnosticOperation::SettingsTab,
            crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected,
        );
        self.navigation_error = Some("The feature could not be opened. Finish any pending navigation, then retry.");
    }

    pub fn bind_scope(&mut self, scope: Option<EngineProfilesScope>) {
        if self.scope != scope {
            self.exit();
            self.scope = scope;
            self.registry = None;
            self.refresh_attempted = false;
            self.error = None;
        }
    }

    pub fn begin_open(&mut self, open_count: u64) {
        if self.open_count != open_count {
            self.exit();
            self.open_count = open_count;
            self.editor_generation = 0;
            self.sequence = 0;
            self.exhausted = false;
            self.registry = None;
            self.refresh_attempted = false;
            self.tab = EngineProfilesTab::General;
            self.feature = None;
            self.section = None;
            self.navigation_error = None;
        }
    }

    pub fn exit(&mut self) {
        if let Some((token, _)) = self.pending.take() { self.orphaned.push(token); }
        self.editor = None;
        self.delete_candidate = None;
    }

    fn replace_editor(&mut self, editor: Option<EngineProfileEditor>) {
        if let Some((token, _)) = self.pending.take() { self.orphaned.push(token); }
        let Some(generation) = self.editor_generation.checked_add(1) else {
            self.exhausted = true;
            self.error = Some("Close and reopen Settings to continue editing profiles.".to_owned());
            return;
        };
        self.editor_generation = generation;
        self.editor = editor;
        self.delete_candidate = None;
        self.error = None;
    }

    pub fn take_orphaned(&mut self) -> Vec<EngineProfileUiToken> {
        std::mem::take(&mut self.orphaned)
    }

    pub fn needs_registry_refresh(&self) -> bool {
        self.registry.is_none() && self.pending.is_none()
            && self.scope.is_some() && !self.exhausted && !self.refresh_attempted
    }

    fn begin_pending(&mut self, clear_editor: bool) -> Option<EngineProfileUiToken> {
        if self.pending.is_some() || self.exhausted { return None; }
        let scope = self.scope.clone()?;
        let Some(sequence) = self.sequence.checked_add(1) else {
            self.exhausted = true;
            self.error = Some("Close and reopen Settings to continue editing profiles.".to_owned());
            return None;
        };
        self.sequence = sequence;
        let token = EngineProfileUiToken {
            scope, open_count: self.open_count,
            editor_generation: self.editor_generation, sequence,
        };
        self.pending = Some((token.clone(), clear_editor));
        self.error = None;
        Some(token)
    }

    pub fn begin_refresh(&mut self) -> Option<EngineProfileUiToken> {
        let token = self.begin_pending(false)?;
        self.refresh_attempted = true;
        Some(token)
    }

    fn emit(
        &mut self,
        request: EngineProfileMutationRequestV1,
        clear_editor: bool,
    ) -> SettingsOutcome {
        let Some(token) = self.begin_pending(clear_editor) else {
            return SettingsOutcome::None;
        };
        SettingsOutcome::EngineProfileMutationRequested { token, request }
    }

    pub fn complete(
        &mut self,
        token: &EngineProfileUiToken,
        result: Result<EngineProfileRegistryV1, String>,
    ) {
        use crate::diagnostics::gui_events::{record_gui_observation, GuiDiagnosticOperation as GuiOp, GuiDiagnosticPhase as GuiPhase};
        let Some((expected, clear_editor)) = self.pending.as_ref() else {
            record_gui_observation(GuiOp::SettingsOutcome, GuiPhase::Stale);
            return;
        };
        if expected != token
            || self.scope.as_ref() != Some(&token.scope)
            || self.open_count != token.open_count
            || self.editor_generation != token.editor_generation
        {
            record_gui_observation(GuiOp::SettingsOutcome, GuiPhase::Stale);
            return;
        }
        let clear_editor = *clear_editor;
        self.pending = None;
        match result {
            Ok(registry) => {
                self.registry = Some(registry);
                if clear_editor { self.editor = None; }
                self.delete_candidate = None;
                self.error = None;
                record_gui_observation(GuiOp::SettingsOutcome, GuiPhase::Completed);
            }
            Err(_) => {
                self.error = Some(
                    "Engine profile request failed. Refresh and choose a current profile; failed edits remain available."
                        .to_owned(),
                );
                record_gui_observation(GuiOp::SettingsOutcome, GuiPhase::Failed);
            }
        }
    }
}

fn render_engine_profiles_tab(
    ui: &mut egui::Ui,
    state: &mut EngineProfilesSettingsState,
) -> SettingsOutcome {
    ui.heading("Engine Profiles");
    if state.scope.is_none() {
        ui.label("Sign in and open a workspace to manage Engine Profiles.");
        return SettingsOutcome::None;
    }
    let mut outcome = SettingsOutcome::None;
    if let Some(error) = &state.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    if state.pending.is_some() { ui.label("Saving Engine Profiles..."); }

    let mut refresh = false;
    let mut create = false;
    ui.add_enabled_ui(state.pending.is_none() && !state.exhausted, |ui| {
        refresh = engine_profile_button(ui, "settings.engine-profiles.refresh", "Refresh");
        create = engine_profile_button(ui, ENGINE_PROFILE_CREATE_AUTHOR_ID, "Create profile")
            && state.registry.is_some();
    });
    if refresh { return SettingsOutcome::EngineProfilesRefreshRequested; }
    if create {
        if let Some(registry) = state.registry.as_ref() {
            let previous_generation = state.editor_generation;
            let editor = EngineProfileEditor::create(registry);
            state.replace_editor(Some(editor));
            if state.editor_generation != previous_generation {
                crate::mcp::argus::acknowledge_action_effect(
                    ui.ctx(),
                    "settings.engine-profiles.create",
                );
            }
        }
    }

    let Some(registry) = state.registry.clone() else {
        ui.label("Refresh to load the workspace profile registry.");
        return outcome;
    };
    for definition in &registry.profiles {
        if definition.tombstoned { continue; }
        let Some(snapshot) = definition.revisions.last() else { continue; };
        let captured = EngineProfileEditor {
            expected: Some(snapshot.clone()),
            expected_head_generation: definition.head_generation,
            registry_generation: registry.registry_generation,
            active_generation: registry.active_generation,
            profile: snapshot.profile.clone(),
            inspect_only: false,
        };
        let profile = &snapshot.profile;
        let prefix = format!("settings.engine-profiles.{}", snapshot.fingerprint);
        ui.separator();
        ui.label(format!(
            "{} · revision {} · {} · {:?} · {:?} · {}",
            profile.id, profile.revision, profile.model.id,
            profile.engine, profile.device, profile.preset.name,
        ));
        let active = registry.active.as_ref() == Some(&engine_profile_identity(snapshot));
        if active { ui.label("Active"); }
        let mut inspect = false;
        let mut edit = false;
        let mut delete = false;
        let mut activate = false;
        ui.add_enabled_ui(state.pending.is_none() && !state.exhausted, |ui| {
            ui.horizontal_wrapped(|ui| {
                inspect = engine_profile_button(ui, &format!("{prefix}.inspect"), "Inspect");
                edit = engine_profile_button(ui, &format!("{prefix}.edit"), "Edit");
                delete = engine_profile_button(ui, &format!("{prefix}.delete"), "Delete");
                if !active {
                    activate = engine_profile_button(
                        ui, &format!("{prefix}.activate"), "Make active",
                    );
                }
            });
        });
        if inspect || edit {
            let mut editor = captured.clone();
            editor.inspect_only = inspect;
            state.replace_editor(Some(editor));
        } else if delete {
            state.delete_candidate = Some(captured.clone());
        } else if activate {
            if let Some(request) = captured.head_request(true) {
                return state.emit(request, false);
            }
        }
    }

    if let Some(candidate) = state.delete_candidate.clone() {
        ui.separator();
        ui.label(format!("Delete {} revision {}?",
            candidate.profile.id, candidate.profile.revision));
        ui.label("Started runs retain their original profile snapshot.");
        let mut confirm = false;
        let mut cancel = false;
        ui.add_enabled_ui(state.pending.is_none(), |ui| {
            confirm = engine_profile_button(
                ui, "settings.engine-profiles.delete.confirm", "Confirm delete",
            );
            cancel = engine_profile_button(
                ui, "settings.engine-profiles.delete.cancel", "Cancel delete",
            );
        });
        if cancel { state.delete_candidate = None; }
        if confirm {
            if let Some(request) = candidate.head_request(false) {
                return state.emit(request, false);
            }
        }
    }

    let mut close_editor = false;
    let mut request = None;
    if let Some(editor) = state.editor.as_mut() {
        ui.separator();
        ui.add_enabled_ui(state.pending.is_none() && !state.exhausted, |ui| {
            render_engine_profile_editor(ui, editor);
            let can_save = !editor.inspect_only && !candidate_profile_invalid(editor);
            ui.add_enabled_ui(can_save, |ui| {
                if engine_profile_button(
                    ui, "settings.engine-profiles.draft.save", "Save profile",
                ) { request = editor.request(); }
            });
            if !editor.inspect_only && candidate_profile_invalid(editor) {
                ui.label("Complete the identity, artifact digest and compatible settings.");
            }
        });
        close_editor = engine_profile_button(
            ui, "settings.engine-profiles.draft.close", "Close editor",
        );
    }
    if close_editor {
        state.replace_editor(None);
    } else if let Some(request) = request {
        outcome = state.emit(request, true);
    }
    outcome
}

fn candidate_profile_invalid(editor: &EngineProfileEditor) -> bool {
    if editor.profile.validate().is_err() { return true; }
    editor.expected.as_ref().is_some_and(|expected|
        editor.profile.id != expected.profile.id
        || expected.profile.revision.checked_add(1).is_none())
}

fn engine_profile_button(
    ui: &mut egui::Ui,
    author_id: &str,
    label: &str,
) -> bool {
    let response = ui.button(label);
    set_author_id(ui, response.id, author_id);
    response.clicked()
}

fn engine_profile_text(
    ui: &mut egui::Ui,
    author_id: &str,
    label: &str,
    value: &mut String,
) {
    ui.label(label);
    let id = ui.make_persistent_id(author_id);
    if let Some(replacement) = crate::mcp::accesskit_string_set_value(ui, id) {
        *value = replacement;
    }
    let response = ui.add(egui::TextEdit::singleline(value).id(id));
    set_author_id(ui, response.id, author_id);
}

fn engine_profile_u32(
    ui: &mut egui::Ui,
    author_id: &str,
    label: &str,
    value: &mut u32,
) {
    ui.label(label);
    let response = ui.add(egui::DragValue::new(value));
    set_author_id(ui, response.id, author_id);
}

fn engine_profile_f64(
    ui: &mut egui::Ui,
    author_id: &str,
    label: &str,
    value: &mut f64,
) {
    ui.label(label);
    let response = ui.add(egui::DragValue::new(value).speed(0.01));
    set_author_id(ui, response.id, author_id);
}

fn render_engine_profile_editor(ui: &mut egui::Ui, editor: &mut EngineProfileEditor) {
    ui.add_enabled_ui(!editor.inspect_only, |ui| {
        ui.add_enabled_ui(editor.expected.is_none(), |ui| {
            engine_profile_text(
                ui, "settings.engine-profiles.draft.id", "Profile ID",
                &mut editor.profile.id,
            );
        });
        engine_profile_text(
            ui, "settings.engine-profiles.draft.model", "Registered model ID",
            &mut editor.profile.model.id,
        );
        engine_profile_text(
            ui, "settings.engine-profiles.draft.digest", "Model artifact SHA-256",
            &mut editor.profile.model.content_sha256,
        );
        engine_profile_text(
            ui, "settings.engine-profiles.draft.preset", "Preset name",
            &mut editor.profile.preset.name,
        );

        ui.label(format!("Revision: {}", editor.profile.revision));

        ui.horizontal(|ui| {
            ui.label("Engine");
            let qairt = ui.selectable_value(
                &mut editor.profile.engine, Engine::GenieXQairt, "GenieX QAIRT",
            );
            set_author_id(ui, qairt.id, "settings.engine-profiles.draft.engine.qairt");
            let llama = ui.selectable_value(
                &mut editor.profile.engine, Engine::GenieXLlamaCpp, "GenieX llama_cpp",
            );
            set_author_id(ui, llama.id, "settings.engine-profiles.draft.engine.llama-cpp");
        });

        ui.horizontal(|ui| {
            ui.label("Device");
            let cpu = ui.selectable_value(
                &mut editor.profile.device, Device::Cpu, "CPU",
            );
            set_author_id(ui, cpu.id, "settings.engine-profiles.draft.device.cpu");
            let npu = ui.selectable_value(
                &mut editor.profile.device, Device::Npu, "NPU",
            );
            set_author_id(ui, npu.id, "settings.engine-profiles.draft.device.npu");
        });

        ui.horizontal(|ui| {
            ui.label("Artifact format");
            let qairt = ui.selectable_value(
                &mut editor.profile.model.format,
                ModelFormat::QairtW4a16,
                "QAIRT W4A16",
            );
            set_author_id(ui, qairt.id, "settings.engine-profiles.draft.format.qairt");
            let gguf = ui.selectable_value(
                &mut editor.profile.model.format,
                ModelFormat::GgufQ4_0,
                "GGUF Q4_0",
            );
            set_author_id(ui, gguf.id, "settings.engine-profiles.draft.format.gguf");
        });

        let mut compiled = editor.profile.model.compiled_context.is_some();
        let response = ui.checkbox(&mut compiled, "Compiled context limit");
        set_author_id(ui, response.id, "settings.engine-profiles.draft.compiled-context.enabled");
        if response.changed() {
            editor.profile.model.compiled_context = compiled.then_some(0);
        }
        if let Some(limit) = &mut editor.profile.model.compiled_context {
            engine_profile_u32(
                ui, "settings.engine-profiles.draft.compiled-context",
                "Compiled context", limit,
            );
        }

        engine_profile_u32(
            ui, "settings.engine-profiles.draft.context",
            "Context limit", &mut editor.profile.preset.context_limit,
        );
        engine_profile_u32(
            ui, "settings.engine-profiles.draft.max-tokens",
            "Maximum generated tokens", &mut editor.profile.preset.max_tokens,
        );

        ui.horizontal(|ui| {
            let is_qairt = matches!(
                editor.profile.preset.sampling, Sampling::Qairt { .. }
            );
            if engine_profile_button(
                ui, "settings.engine-profiles.draft.sampling.qairt", "QAIRT sampling",
            ) && !is_qairt {
                editor.profile.preset.sampling =
                    Sampling::Qairt { temperature: 0.0, seed: 0 };
            }
            if engine_profile_button(
                ui, "settings.engine-profiles.draft.sampling.llama-cpp",
                "llama_cpp sampling",
            ) && is_qairt {
                editor.profile.preset.sampling = Sampling::LlamaCpp {
                    temperature: 0.0, top_k: 0, top_p: 0.0, min_p: 0.0,
                };
            }
        });

        match &mut editor.profile.preset.sampling {
            Sampling::Qairt { temperature, seed } => {
                engine_profile_f64(
                    ui, "settings.engine-profiles.draft.temperature",
                    "Temperature", temperature,
                );
                ui.label("Seed");
                let response = ui.add(egui::DragValue::new(seed));
                set_author_id(ui, response.id, "settings.engine-profiles.draft.seed");
            }
            Sampling::LlamaCpp { temperature, top_k, top_p, min_p } => {
                engine_profile_f64(
                    ui, "settings.engine-profiles.draft.temperature",
                    "Temperature", temperature,
                );
                engine_profile_u32(
                    ui, "settings.engine-profiles.draft.top-k", "Top K", top_k,
                );
                engine_profile_f64(
                    ui, "settings.engine-profiles.draft.top-p", "Top P", top_p,
                );
                engine_profile_f64(
                    ui, "settings.engine-profiles.draft.min-p", "Min P", min_p,
                );
            }
        }
    });
}
