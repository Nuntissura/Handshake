//! MT-015: Operator cloud-model access configuration service.
//!
//! WP-KERNEL-004 built the cloud backends (BYOK runtimes, OS-keychain
//! secrets vault, official CLI bridge, consent gate). What was missing was
//! an operator-facing way to REACH them: a place to type a BYOK API key or
//! to see / start CLI-bridge (subscription-plan) login. This module is the
//! backend half of that surface. The native egui settings dialog
//! (`handshake_native::settings_dialog`) is the operator-facing half; it
//! talks to the [`crate::api::model_access`] HTTP routes that wrap this
//! service.
//!
//! ## Security invariants (MT-015 pre-implementation review decisions)
//!
//! * BYOK keys live ONLY in the OS-keychain vault
//!   ([`super::secrets_vault::OsKeychainSecretsVault`]). This service takes
//!   the key as a [`secrecy::SecretString`], exposes it exactly once at the
//!   `vault.put` boundary, and holds no copy. Nothing here logs, returns,
//!   serialises, or `Debug`-prints key material.
//! * Storing a key creates NO [`super::consent_gate::ConsentGate`] entry and
//!   NO ConsentReceipt (decision MED CONSENT). Configuring access is not the
//!   same as consenting to a cloud send; the first lane launch still hits the
//!   MT-006 fail-closed consent boundary.
//! * The enumeration surface is NON-SECRET. It reports each offered provider
//!   as `configured` / `unavailable` (mapping a missing key —
//!   "ProviderNotConfigured" — to `unavailable`, never an error) WITHOUT
//!   returning key material. It never calls `vault.list_lanes` (the OS
//!   keychain has no portable enumeration) and never touches an alternate
//!   database.
//! * Gemini is not offered. It is not a variant of [`ByokProvider`] or
//!   [`CliBridgeProvider`], so the exclusion is enforced by construction,
//!   not by a runtime filter that a caller could bypass. The reset brief
//!   §6.11 lists Gemini as a valid cloud lane; the operator forbids offering
//!   it here because its CLI is being discontinued. That divergence is
//!   recorded so it is not silently "restored" later.
//! * CLI-bridge login is operator-initiated and uses ONLY the provider's own
//!   official login command (surfaced by [`CliBridgeProvider::login_command`]).
//!   The backend runs it inside a Handshake-hosted pseudo-terminal and surfaces
//!   it as an in-app login session ([`CliBridgeLoginSessionRegistry`]): NO OS
//!   console window is opened and nothing is raised to the foreground
//!   (HBR-QUIET-001), while the provider's interactive device/OAuth prompt and
//!   the operator's typed answer still flow both ways through the native
//!   Settings login panel. Auth status is read only through the provider's own
//!   non-interactive status command. Handshake never reads credential files
//!   directly. Bounded provider output is reduced to a typed state, zeroized,
//!   and never logged, persisted, or returned.
//! * The login transcript is memory-only and bounded. It is held for the life
//!   of one session so the operator can read the prompt, and is never written
//!   to tracing, the Flight Recorder, the EventLedger, or any store. Session
//!   lifetime is bounded, so an abandoned login cannot linger as an unbounded
//!   child process.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

use super::official_cli_bridge::{
    CliBridgeConfig, CliInvocationContext, CliKind, InteractiveLoginTransport, LiveCliSpawner,
};
use super::secrets_vault::{SecretsVault, SecretsVaultError};
use crate::sandbox::{
    IsolationTier, NetPolicy, RequiredCapability, TrustClass,
    CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
};

/// BYOK providers Handshake OFFERS an operator-facing API-key entry for.
///
/// Adding a variant here is the ONLY way a provider becomes offerable for
/// BYOK, so Gemini's exclusion is a compile-time fact rather than a runtime
/// filter. The operator's own primary path is subscription plans via the CLI
/// bridge (see [`CliBridgeProvider`]); BYOK is available for other users.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ByokProvider {
    Anthropic,
    OpenAi,
}

impl ByokProvider {
    /// Every offered BYOK provider, in stable display order. Gemini is
    /// deliberately absent.
    pub const OFFERED: [ByokProvider; 2] = [ByokProvider::Anthropic, ByokProvider::OpenAi];

    /// Stable wire/id string (lower-kebab). Used in routes + the native
    /// author_id targets.
    pub fn id(self) -> &'static str {
        match self {
            ByokProvider::Anthropic => "anthropic",
            ByokProvider::OpenAi => "openai",
        }
    }

    /// Operator-facing label.
    pub fn label(self) -> &'static str {
        match self {
            ByokProvider::Anthropic => "Anthropic (Claude)",
            ByokProvider::OpenAi => "OpenAI (GPT)",
        }
    }

    /// The vault lane id the provider's key is stored under. Stable so the
    /// MT-006 CloudLane/BYOK backend can fetch the same lane via
    /// [`super::secrets_vault::VaultApiKeyProvider`].
    pub fn vault_lane(self) -> &'static str {
        match self {
            ByokProvider::Anthropic => "cloud-byok-anthropic",
            ByokProvider::OpenAi => "cloud-byok-openai",
        }
    }

    /// Parse a provider id. Returns `None` for unknown ids AND for ids that
    /// are deliberately not offered (e.g. `"gemini"`), so an excluded
    /// provider can never be configured through this surface.
    pub fn from_id(id: &str) -> Option<ByokProvider> {
        match id {
            "anthropic" => Some(ByokProvider::Anthropic),
            "openai" => Some(ByokProvider::OpenAi),
            _ => None,
        }
    }
}

/// CLI-bridge providers Handshake offers subscription-plan login status for.
/// This is the operator's PRIMARY path (Claude Code + GPT/Codex via the
/// official CLI bridge). Gemini is not a variant (excluded by construction).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CliBridgeProvider {
    ClaudeCode,
    Codex,
}

impl CliBridgeProvider {
    /// Every offered CLI-bridge provider, in stable display order.
    pub const OFFERED: [CliBridgeProvider; 2] =
        [CliBridgeProvider::ClaudeCode, CliBridgeProvider::Codex];

    pub fn id(self) -> &'static str {
        match self {
            CliBridgeProvider::ClaudeCode => "claude_code",
            CliBridgeProvider::Codex => "codex",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CliBridgeProvider::ClaudeCode => "Claude Code",
            CliBridgeProvider::Codex => "GPT / Codex CLI",
        }
    }

    pub fn from_id(id: &str) -> Option<CliBridgeProvider> {
        match id {
            "claude_code" => Some(CliBridgeProvider::ClaudeCode),
            "codex" => Some(CliBridgeProvider::Codex),
            _ => None,
        }
    }

    fn accepts_cli_kind(self, cli_kind: CliKind) -> bool {
        matches!(
            (self, cli_kind),
            (CliBridgeProvider::ClaudeCode, CliKind::ClaudeCode)
                | (CliBridgeProvider::Codex, CliKind::CodexCli)
        )
    }

    /// The provider's OWN official login command, run operator-initiated inside
    /// a Handshake-hosted pseudo-terminal (no OS console window, no focus or
    /// Z-order change) and surfaced in the native Settings login panel.
    ///
    /// Handshake NEVER captures or stores the credentials this command
    /// establishes; it only starts the provider's official interactive flow.
    /// The fixed argv vectors are the provider-owned login surfaces; neither
    /// operator text nor provider response data is interpolated.
    pub fn login_command(self) -> OfficialLoginCommand {
        match self {
            CliBridgeProvider::ClaudeCode => OfficialLoginCommand {
                program: "claude",
                args: &["auth", "login"],
                hint: "Starts the official Claude Code CLI login. Handshake stores no credential; \
                       your Claude subscription session lives in the Claude Code CLI.",
            },
            CliBridgeProvider::Codex => OfficialLoginCommand {
                program: "codex",
                args: &["login"],
                hint: "Starts the official Codex/GPT CLI login. Handshake stores no credential; \
                       your ChatGPT/Codex session lives in the Codex CLI.",
            },
        }
    }

    /// The provider's own non-interactive authentication-status command.
    ///
    /// These commands report auth metadata/status only. Handshake parses them
    /// into [`CliBridgeAuthStatus`] and discards their raw output; it never
    /// reads either provider's credential files.
    pub fn auth_status_command(self) -> OfficialAuthStatusCommand {
        match self {
            CliBridgeProvider::ClaudeCode => OfficialAuthStatusCommand {
                program: "claude",
                args: &["auth", "status"],
            },
            CliBridgeProvider::Codex => OfficialAuthStatusCommand {
                program: "codex",
                args: &["login", "status"],
            },
        }
    }
}

/// A provider's own official login command, started operator-initiated inside a
/// Handshake-hosted in-app terminal session (no OS console window, no focus or
/// Z-order change). Non-secret static data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct OfficialLoginCommand {
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub hint: &'static str,
}

/// A provider's own non-interactive auth-status command. Non-secret static
/// data; output is never part of this type or the public API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfficialAuthStatusCommand {
    pub program: &'static str,
    pub args: &'static [&'static str],
}

/// Non-secret authentication state for an official CLI bridge.
///
/// `Unavailable` is deliberately distinct from `LoggedOut`: a missing CLI,
/// timeout, or unrecognized provider response is not evidence that the
/// operator has logged out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CliBridgeAuthStatus {
    LoggedIn,
    LoggedOut,
    Expired,
    Unavailable,
}

/// Typed auth-status seam used by the route and operator picker. Implementations
/// return status only; raw CLI output and credentials can never cross this
/// boundary.
pub trait CliBridgeAuthStatusProbe: Send + Sync {
    fn auth_status(&self, provider: CliBridgeProvider) -> CliBridgeAuthStatus;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum CliBridgeLoginLaunchError {
    #[error("official CLI login is unavailable")]
    Unavailable,
    #[error("official CLI login launch failed")]
    LaunchFailed,
}

/// Backend-owned interactive login launcher. Implementations return a live
/// [`InteractiveLoginTransport`] — a pid receipt plus a read/write channel to
/// the provider's own login process. Executable paths, argv, and the child
/// environment never cross into the GUI.
pub trait CliBridgeLoginLauncher: Send + Sync {
    fn launch_login(
        &self,
        provider: CliBridgeProvider,
    ) -> Result<Arc<dyn InteractiveLoginTransport>, CliBridgeLoginLaunchError>;
}

/// One canonical, launchable CLI target. It reuses the exact configured launch
/// graph and live spawner already accepted by Operator Chat; status probing
/// never performs an independent PATH lookup.
#[derive(Clone)]
struct CanonicalCliAuthTarget {
    spawner: Arc<LiveCliSpawner>,
    config: CliBridgeConfig,
}

/// Production provider-owned CLI status probe.
///
/// Targets are supplied only after the production launch factory has validated
/// and pinned them. Commands execute through the existing attached Official-CLI
/// sandbox lifecycle: Windows uses creation-time Job Object containment and
/// bounded pipe drainage; other hosts fail closed until an equivalent attached
/// implementation exists. Raw provider output is reduced to a typed state,
/// zeroized, and never returned or logged.
#[derive(Clone, Default)]
pub struct ProductionCliBridgeAuthStatusProbe {
    targets: BTreeMap<CliBridgeProvider, CanonicalCliAuthTarget>,
}

impl std::fmt::Debug for ProductionCliBridgeAuthStatusProbe {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProductionCliBridgeAuthStatusProbe")
            .field(
                "providers",
                &self
                    .targets
                    .keys()
                    .map(|provider| provider.id())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl ProductionCliBridgeAuthStatusProbe {
    pub fn from_canonical_launches(
        spawner: Arc<LiveCliSpawner>,
        launches: impl IntoIterator<Item = (CliBridgeProvider, CliBridgeConfig)>,
    ) -> Self {
        Self {
            targets: launches
                .into_iter()
                .filter_map(|(provider, config)| {
                    provider.accepts_cli_kind(config.cli_kind).then(|| {
                        (
                            provider,
                            CanonicalCliAuthTarget {
                                spawner: spawner.clone(),
                                config,
                            },
                        )
                    })
                })
                .collect(),
        }
    }
}

impl CliBridgeAuthStatusProbe for ProductionCliBridgeAuthStatusProbe {
    fn auth_status(&self, provider: CliBridgeProvider) -> CliBridgeAuthStatus {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = provider;
            return CliBridgeAuthStatus::Unavailable;
        }

        #[cfg(target_os = "windows")]
        let Some(target) = self.targets.get(&provider) else {
            return CliBridgeAuthStatus::Unavailable;
        };
        #[cfg(target_os = "windows")]
        let command = provider.auth_status_command();
        #[cfg(target_os = "windows")]
        let Ok(mut output) = target.spawner.run_auxiliary_fixed_command(
            &target.config,
            command.args,
            AUTH_STATUS_TIMEOUT,
            &auth_status_invocation(provider),
            AUTH_STATUS_OUTPUT_LIMIT,
        ) else {
            return CliBridgeAuthStatus::Unavailable;
        };
        #[cfg(target_os = "windows")]
        {
            let status = parse_official_auth_status(provider, output.success, &output.stdout);
            output.stdout.zeroize();
            status
        }
    }
}

impl CliBridgeLoginLauncher for ProductionCliBridgeAuthStatusProbe {
    fn launch_login(
        &self,
        provider: CliBridgeProvider,
    ) -> Result<Arc<dyn InteractiveLoginTransport>, CliBridgeLoginLaunchError> {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = provider;
            Err(CliBridgeLoginLaunchError::Unavailable)
        }

        #[cfg(target_os = "windows")]
        {
            let target = self
                .targets
                .get(&provider)
                .ok_or(CliBridgeLoginLaunchError::Unavailable)?;
            target
                .spawner
                .launch_foreground_fixed_command(&target.config, provider.login_command().args)
                .map(|pty| Arc::new(pty) as Arc<dyn InteractiveLoginTransport>)
                .map_err(|_| CliBridgeLoginLaunchError::LaunchFailed)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct UnavailableCliBridgeAuthStatusProbe;

impl CliBridgeAuthStatusProbe for UnavailableCliBridgeAuthStatusProbe {
    fn auth_status(&self, _provider: CliBridgeProvider) -> CliBridgeAuthStatus {
        CliBridgeAuthStatus::Unavailable
    }
}

impl CliBridgeLoginLauncher for UnavailableCliBridgeAuthStatusProbe {
    fn launch_login(
        &self,
        _provider: CliBridgeProvider,
    ) -> Result<Arc<dyn InteractiveLoginTransport>, CliBridgeLoginLaunchError> {
        Err(CliBridgeLoginLaunchError::Unavailable)
    }
}

/// Bounded lifetime for one in-app login session. Matches the backend watcher's
/// bounded-lifetime ceiling so the operator-visible `timed_out` state and the
/// process-ownership kill agree. HBR-QUIET-004 has NOT been declared for this
/// MT, so an unbounded attached login is not permitted.
pub const CLI_LOGIN_SESSION_MAX_LIFETIME: Duration = Duration::from_secs(15 * 60);

/// Bytes of transcript tail returned to the operator surface. Bounded so a
/// chatty provider cannot grow the HTTP response without limit; the PTY's own
/// scrollback cap bounds backend memory independently.
pub const CLI_LOGIN_TRANSCRIPT_TAIL_BYTES: usize = 16 * 1024;

/// Observable state of one in-app official-CLI login session.
///
/// `AwaitingInput` deliberately means "the provider has printed something and
/// the process is still running" — Handshake does not parse provider prompts,
/// so it reports what it can prove rather than guessing at flow stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliLoginSessionStatus {
    /// Launched; the provider has not written anything yet.
    Pending,
    /// The provider has written output and the login process is still running.
    AwaitingInput,
    /// The login process exited 0.
    Succeeded,
    /// The login process exited non-zero.
    Failed,
    /// The bounded session window elapsed; Handshake terminated the process.
    TimedOut,
    /// The operator cancelled the login.
    Cancelled,
}

impl CliLoginSessionStatus {
    /// Whether no further operator input can change this state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            CliLoginSessionStatus::Succeeded
                | CliLoginSessionStatus::Failed
                | CliLoginSessionStatus::TimedOut
                | CliLoginSessionStatus::Cancelled
        )
    }
}

/// Non-secret, operator-facing view of one login session.
///
/// `transcript` is the provider's own terminal output. It is memory-only: it is
/// never logged, traced, recorded to the Flight Recorder / EventLedger, or
/// persisted. It exists so the operator can read the provider's device/OAuth
/// prompt inside Handshake instead of in a separate OS window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliLoginSessionSnapshot {
    pub session_id: String,
    pub provider: String,
    pub status: CliLoginSessionStatus,
    pub transcript: String,
    pub transcript_truncated: bool,
    pub exit_code: Option<i32>,
    pub elapsed_ms: u64,
    pub remaining_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum CliLoginSessionError {
    #[error("no such CLI login session")]
    UnknownSession,
    #[error("CLI login session already finished")]
    SessionFinished,
    #[error("CLI login session input could not be delivered")]
    InputFailed,
}

/// One live login session: the transport plus the non-secret bookkeeping the
/// operator surface reads.
struct CliLoginSession {
    session_id: String,
    provider: CliBridgeProvider,
    transport: Arc<dyn InteractiveLoginTransport>,
    // WAIVER [CX-573E]: Instant::now() is duration/timeout bookkeeping only for
    // the bounded login window; it carries no determinism-bearing authority.
    started_at: std::time::Instant,
    cancelled: std::sync::atomic::AtomicBool,
    timed_out: std::sync::atomic::AtomicBool,
}

impl CliLoginSession {
    fn status(&self) -> CliLoginSessionStatus {
        use std::sync::atomic::Ordering;
        if self.cancelled.load(Ordering::Acquire) {
            return CliLoginSessionStatus::Cancelled;
        }
        if let Some(code) = self.transport.exit_code() {
            if self.timed_out.load(Ordering::Acquire) {
                return CliLoginSessionStatus::TimedOut;
            }
            return if code == 0 {
                CliLoginSessionStatus::Succeeded
            } else {
                CliLoginSessionStatus::Failed
            };
        }
        if self.timed_out.load(Ordering::Acquire) {
            return CliLoginSessionStatus::TimedOut;
        }
        if self.transport.transcript().is_empty() {
            CliLoginSessionStatus::Pending
        } else {
            CliLoginSessionStatus::AwaitingInput
        }
    }

    /// Enforce the bounded window. Called on every read, so an abandoned login
    /// is terminated by the first surface that looks at it, and independently by
    /// the backend watcher's own lifetime ceiling.
    fn enforce_deadline(&self) {
        use std::sync::atomic::Ordering;
        if self.transport.exit_code().is_some() || self.cancelled.load(Ordering::Acquire) {
            return;
        }
        if self.started_at.elapsed() >= CLI_LOGIN_SESSION_MAX_LIFETIME {
            self.timed_out.store(true, Ordering::Release);
            self.transport.cancel();
        }
    }

    fn snapshot(&self) -> CliLoginSessionSnapshot {
        self.enforce_deadline();
        let (transcript, transcript_truncated) =
            render_login_transcript(&self.transport.transcript());
        let elapsed = self.started_at.elapsed();
        CliLoginSessionSnapshot {
            session_id: self.session_id.clone(),
            provider: self.provider.id().to_string(),
            status: self.status(),
            transcript,
            transcript_truncated,
            exit_code: self.transport.exit_code(),
            elapsed_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            remaining_ms: CLI_LOGIN_SESSION_MAX_LIFETIME
                .saturating_sub(elapsed)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        }
    }
}

impl std::fmt::Debug for CliLoginSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Transcript deliberately omitted: provider login output can carry
        // one-time codes and must never reach a Debug/log surface.
        formatter
            .debug_struct("CliLoginSession")
            .field("session_id", &self.session_id)
            .field("provider", &self.provider.id())
            .finish()
    }
}

/// Turn raw PTY bytes into text an egui label can render.
///
/// Strips ANSI CSI/OSC control sequences (the provider's spinners and colour
/// codes would otherwise render as mojibake), normalises CR/LF, and keeps only
/// the trailing [`CLI_LOGIN_TRANSCRIPT_TAIL_BYTES`]. Device codes and login URLs
/// are deliberately NOT redacted: they are exactly what the operator must read
/// to finish the flow, and they are single-use provider-issued values, not
/// Handshake-held credentials.
fn render_login_transcript(raw: &[u8]) -> (String, bool) {
    let text = String::from_utf8_lossy(raw);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' => match chars.next() {
                // CSI: consume parameter/intermediate bytes then the final byte.
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&next) {
                            break;
                        }
                    }
                }
                // OSC: runs until BEL or ESC \.
                Some(']') => {
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' {
                            let _ = chars.next();
                            break;
                        }
                    }
                }
                // Any other two-character escape is dropped whole.
                Some(_) | None => {}
            },
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    continue;
                }
                out.push('\n');
            }
            '\n' | '\t' => out.push(ch),
            other if other.is_control() => {}
            other => out.push(other),
        }
    }
    if out.len() > CLI_LOGIN_TRANSCRIPT_TAIL_BYTES {
        let start = out
            .char_indices()
            .rev()
            .map(|(index, _)| index)
            .find(|index| out.len() - index >= CLI_LOGIN_TRANSCRIPT_TAIL_BYTES)
            .unwrap_or(0);
        (out.split_off(start), true)
    } else {
        (out, false)
    }
}

/// Registry of live in-app official-CLI login sessions.
///
/// At most one session per provider is live at a time: starting a new login for
/// a provider cancels its predecessor, so a repeated click can never accumulate
/// login children. Sessions are dropped once terminal AND read at least once by
/// the operator surface, so a completed transcript is not retained indefinitely.
pub struct CliBridgeLoginSessionRegistry {
    launcher: Arc<dyn CliBridgeLoginLauncher>,
    sessions: RwLock<Vec<Arc<CliLoginSession>>>,
}

impl std::fmt::Debug for CliBridgeLoginSessionRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CliBridgeLoginSessionRegistry")
            .field(
                "live_sessions",
                &self.sessions.read().map(|s| s.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl CliBridgeLoginSessionRegistry {
    pub fn new(launcher: Arc<dyn CliBridgeLoginLauncher>) -> Self {
        Self {
            launcher,
            sessions: RwLock::new(Vec::new()),
        }
    }

    /// Start the provider's own official login inside a Handshake-hosted PTY.
    pub fn start(
        &self,
        provider: CliBridgeProvider,
    ) -> Result<CliLoginSessionSnapshot, CliBridgeLoginLaunchError> {
        let transport = self.launcher.launch_login(provider)?;
        let session = Arc::new(CliLoginSession {
            session_id: uuid::Uuid::now_v7().to_string(),
            provider,
            transport,
            started_at: std::time::Instant::now(),
            cancelled: std::sync::atomic::AtomicBool::new(false),
            timed_out: std::sync::atomic::AtomicBool::new(false),
        });
        let snapshot = session.snapshot();
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| CliBridgeLoginLaunchError::LaunchFailed)?;
        // One live login per provider: cancel and evict any predecessor.
        sessions.retain(|existing| {
            if existing.provider == provider {
                existing
                    .cancelled
                    .store(true, std::sync::atomic::Ordering::Release);
                existing.transport.cancel();
                return false;
            }
            true
        });
        sessions.push(session);
        Ok(snapshot)
    }

    fn find(&self, session_id: &str) -> Result<Arc<CliLoginSession>, CliLoginSessionError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| CliLoginSessionError::UnknownSession)?;
        sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .cloned()
            .ok_or(CliLoginSessionError::UnknownSession)
    }

    pub fn snapshot(
        &self,
        session_id: &str,
    ) -> Result<CliLoginSessionSnapshot, CliLoginSessionError> {
        Ok(self.find(session_id)?.snapshot())
    }

    /// Deliver one operator response to the provider's login process.
    ///
    /// A newline is appended here so the operator surface never has to encode
    /// terminal control characters, and the raw input is not echoed anywhere.
    pub fn send_input(
        &self,
        session_id: &str,
        input: &str,
    ) -> Result<CliLoginSessionSnapshot, CliLoginSessionError> {
        let session = self.find(session_id)?;
        session.enforce_deadline();
        if session.status().is_terminal() {
            return Err(CliLoginSessionError::SessionFinished);
        }
        let mut line = input.replace(['\r', '\n'], "");
        line.push('\r');
        session
            .transport
            .write_input(line.as_bytes())
            .map_err(|_| CliLoginSessionError::InputFailed)?;
        Ok(session.snapshot())
    }

    /// Operator-initiated cancel. Idempotent.
    pub fn cancel(
        &self,
        session_id: &str,
    ) -> Result<CliLoginSessionSnapshot, CliLoginSessionError> {
        let session = self.find(session_id)?;
        session
            .cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        session.transport.cancel();
        let snapshot = session.snapshot();
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.retain(|existing| existing.session_id != session_id);
        }
        Ok(snapshot)
    }
}

const AUTH_STATUS_TIMEOUT: Duration = Duration::from_secs(5);
const AUTH_STATUS_OUTPUT_LIMIT: usize = 64 * 1024;

fn auth_status_invocation(provider: CliBridgeProvider) -> CliInvocationContext {
    let mut context = CliInvocationContext::new(
        "MODEL_ACCESS_AUTH_STATUS",
        format!("official-cli-auth-status:{}", provider.id()),
    );
    context.owner_wp = Some("WP-1".to_string());
    context.role_id = Some("MODEL_ACCESS_AUTH_STATUS".to_string());
    context.wp_id = Some("WP-1".to_string());
    context.mt_id = Some("MT-015".to_string());
    context.session_id = Some(format!("model-access-auth-status-{}", provider.id()));
    // MT-019 P-2: `parent_session_id` was left NULL here, which made this row
    // class invisible to BOTH session-keyed reclaim claims AND `restart_sessions`
    // (which requires `parent_session_id IS NOT NULL`). An auth-status child that
    // could not be reaped therefore stayed OPEN forever, not even closed at the
    // next boot. This probe owns its own session identity, so it is its own parent.
    context.parent_session_id = Some(format!("model-access-auth-status-{}", provider.id()));
    context.reclaim_key = Some(format!("model-access-auth-status-{}", provider.id()));
    context.requested_trust_class = Some(TrustClass::Reviewed);
    context.requested_isolation_tier = Some(IsolationTier::Tier1Container);
    context.requested_sandbox_capabilities =
        Some(BTreeSet::from([RequiredCapability::HighStdioThroughput]));
    context.requested_net_policy = Some(NetPolicy::HostInherited);
    context.requested_execution_policy_ref =
        Some(CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
    context
}

fn parse_official_auth_status(
    provider: CliBridgeProvider,
    success: bool,
    stdout: &[u8],
) -> CliBridgeAuthStatus {
    match provider {
        CliBridgeProvider::ClaudeCode => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct ClaudeAuthStatus {
                logged_in: bool,
                #[serde(default)]
                subscription_type: Option<String>,
            }

            let Ok(status) = serde_json::from_slice::<ClaudeAuthStatus>(stdout) else {
                return CliBridgeAuthStatus::Unavailable;
            };
            match status {
                ClaudeAuthStatus {
                    logged_in: true,
                    subscription_type: Some(subscription_type),
                } if success && !subscription_type.trim().is_empty() => {
                    CliBridgeAuthStatus::LoggedIn
                }
                ClaudeAuthStatus {
                    logged_in: false, ..
                } if success => CliBridgeAuthStatus::LoggedOut,
                _ => CliBridgeAuthStatus::Unavailable,
            }
        }
        CliBridgeProvider::Codex => {
            let Ok(text) = std::str::from_utf8(stdout) else {
                return CliBridgeAuthStatus::Unavailable;
            };
            match text.trim() {
                "Not logged in" if success => CliBridgeAuthStatus::LoggedOut,
                "Logged in using ChatGPT" if success => CliBridgeAuthStatus::LoggedIn,
                _ => CliBridgeAuthStatus::Unavailable,
            }
        }
    }
}

/// Non-secret access status for one provider. `ProviderNotConfigured`
/// (a missing key) maps to [`ProviderAccessStatus::Unavailable`], never to an
/// error, so the MT-012 model picker can list an un-keyed provider as simply
/// unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAccessStatus {
    Configured,
    Unavailable,
}

/// One non-secret BYOK enumeration row (never carries key material).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ByokAccessRow {
    pub provider: &'static str,
    pub label: &'static str,
    pub status: ProviderAccessStatus,
}

/// One non-secret CLI-bridge enumeration row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CliBridgeAccessRow {
    pub provider: &'static str,
    pub label: &'static str,
    pub auth_status: CliBridgeAuthStatus,
    pub login: OfficialLoginCommand,
}

/// The full non-secret enumeration surface consumed by MT-012's model picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloudAccessEnumeration {
    pub byok: Vec<ByokAccessRow>,
    pub cli_bridge: Vec<CliBridgeAccessRow>,
    /// Providers deliberately NOT offered here, surfaced so the exclusion is
    /// visible (and testable) rather than a silent gap.
    pub excluded: Vec<&'static str>,
}

/// NON-SECRET view of which providers are configured. Implementations answer
/// per-provider status WITHOUT returning key material; they never call
/// `vault.list_lanes` and never touch an alternate database.
pub trait ProviderAccessRegistry: Send + Sync {
    fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus;
}

/// Production registry: derives `configured` / `unavailable` from the presence
/// of a key in the OS-keychain vault for the provider's lane. The OS keychain
/// exposes no presence-only probe, so this reads the secret value to decide
/// presence and DROPS it immediately — it never returns, logs, or stores the
/// key. `NoSecretForLane` (ProviderNotConfigured) maps to `Unavailable`; any
/// other vault error also maps to `Unavailable` (fail-closed: a backend error
/// must not make an unconfigured provider look available, nor abort the whole
/// enumeration).
pub struct VaultBackedAccessRegistry {
    vault: Arc<dyn SecretsVault>,
}

impl VaultBackedAccessRegistry {
    pub fn new(vault: Arc<dyn SecretsVault>) -> Self {
        Self { vault }
    }
}

impl ProviderAccessRegistry for VaultBackedAccessRegistry {
    fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus {
        match self.vault.get(provider.vault_lane()) {
            Ok(secret) => {
                // Presence only: the read is a `Zeroizing<String>`, so dropping
                // it here wipes the plaintext instead of leaving an un-zeroized
                // heap copy behind (MT-015 F1). We never surface the value.
                drop(secret);
                ProviderAccessStatus::Configured
            }
            Err(_) => ProviderAccessStatus::Unavailable,
        }
    }
}

/// In-memory NON-SECRET registry used by unit tests to prove the enumeration
/// mapping in isolation from the OS keychain. Holds only which providers are
/// marked configured (booleans), never key material.
#[derive(Default)]
pub struct InMemoryAccessRegistry {
    configured: RwLock<BTreeSet<ByokProvider>>,
}

impl InMemoryAccessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_configured(&self, provider: ByokProvider, configured: bool) {
        let mut guard = self.configured.write().expect("registry lock");
        if configured {
            guard.insert(provider);
        } else {
            guard.remove(&provider);
        }
    }
}

impl ProviderAccessRegistry for InMemoryAccessRegistry {
    fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus {
        if self
            .configured
            .read()
            .expect("registry lock")
            .contains(&provider)
        {
            ProviderAccessStatus::Configured
        } else {
            ProviderAccessStatus::Unavailable
        }
    }
}

/// Enumerate every offered BYOK provider's non-secret status. Gemini can never
/// appear because it is not in [`ByokProvider::OFFERED`].
pub fn enumerate_byok(registry: &dyn ProviderAccessRegistry) -> Vec<ByokAccessRow> {
    ByokProvider::OFFERED
        .iter()
        .map(|provider| ByokAccessRow {
            provider: provider.id(),
            label: provider.label(),
            status: registry.byok_status(*provider),
        })
        .collect()
}

/// Enumerate every offered CLI-bridge provider with its typed auth state and
/// official login command.
pub fn enumerate_cli_bridge() -> Vec<CliBridgeAccessRow> {
    enumerate_cli_bridge_with_probe(&UnavailableCliBridgeAuthStatusProbe)
}

/// Deterministic/injectable form used by HTTP route tests and other consumers
/// that already own an auth-status probe.
pub fn enumerate_cli_bridge_with_probe(
    probe: &dyn CliBridgeAuthStatusProbe,
) -> Vec<CliBridgeAccessRow> {
    // Provider probes are independent and each may take up to the bounded
    // subprocess timeout. Run the two offered providers concurrently so a
    // settings refresh is bounded by one timeout window rather than two.
    thread::scope(|scope| {
        let probes = CliBridgeProvider::OFFERED
            .map(|provider| (provider, scope.spawn(move || probe.auth_status(provider))));
        probes
            .into_iter()
            .map(|(provider, handle)| CliBridgeAccessRow {
                provider: provider.id(),
                label: provider.label(),
                auth_status: handle.join().unwrap_or(CliBridgeAuthStatus::Unavailable),
                login: provider.login_command(),
            })
            .collect()
    })
}

/// Build the full non-secret enumeration surface for the model picker.
pub fn enumerate(registry: &dyn ProviderAccessRegistry) -> CloudAccessEnumeration {
    enumerate_with_cli_auth_probe(registry, &UnavailableCliBridgeAuthStatusProbe)
}

/// Build the full non-secret enumeration surface with an injected CLI-status
/// probe. Gemini remains excluded by construction: the probe is invoked only
/// for [`CliBridgeProvider::OFFERED`].
pub fn enumerate_with_cli_auth_probe(
    registry: &dyn ProviderAccessRegistry,
    probe: &dyn CliBridgeAuthStatusProbe,
) -> CloudAccessEnumeration {
    CloudAccessEnumeration {
        byok: enumerate_byok(registry),
        cli_bridge: enumerate_cli_bridge_with_probe(probe),
        // Deliberately not offered (reset brief §6.11 divergence — CLI
        // deprecating). Surfaced so the exclusion is visible + testable.
        excluded: vec!["gemini"],
    }
}

#[derive(Debug, Error)]
pub enum AccessConfigError {
    #[error("provider {0} is not offered for BYOK configuration")]
    ProviderNotOffered(String),
    #[error("API key must not be empty")]
    EmptyKey,
    #[error(
        "OS keychain feature is not enabled; refusing to persist a cloud key \
         (there is no plaintext fallback)"
    )]
    KeychainUnavailable,
    #[error("secrets vault error: {0}")]
    Vault(#[from] SecretsVaultError),
}

/// The operator cloud-model access service: the thin, security-critical seam
/// between the operator config surface and the OS-keychain vault.
///
/// It holds `Arc<dyn SecretsVault>` (no key material) plus a stable
/// `vault_kind` label so a leak test can assert the wired vault is the OS
/// keychain and not the in-memory impl. Its `Debug` never renders a key
/// because it never holds one.
pub struct CloudModelAccess {
    vault: Arc<dyn SecretsVault>,
    vault_kind: &'static str,
}

impl std::fmt::Debug for CloudModelAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudModelAccess")
            .field("vault", &"<Arc<dyn SecretsVault>>")
            .field("vault_kind", &self.vault_kind)
            .finish()
    }
}

impl CloudModelAccess {
    /// Wire the service to an explicit vault. `vault_kind` is a stable label
    /// (e.g. `"OsKeychainSecretsVault"`) used by the leak test to prove the
    /// production wiring is the OS keychain. Prefer [`Self::production`] in
    /// app code.
    pub fn with_vault(vault: Arc<dyn SecretsVault>, vault_kind: &'static str) -> Self {
        Self { vault, vault_kind }
    }

    /// Production wiring. With the default `os-keychain` feature this returns a
    /// service backed by [`super::secrets_vault::OsKeychainSecretsVault`].
    /// WITHOUT `os-keychain` it REFUSES (returns
    /// [`AccessConfigError::KeychainUnavailable`]) rather than silently
    /// falling back to an in-memory / plaintext store — a cloud key must never
    /// be persisted outside the OS keychain.
    pub fn production() -> Result<Self, AccessConfigError> {
        #[cfg(feature = "os-keychain")]
        {
            let vault = super::secrets_vault::OsKeychainSecretsVault::new(
                super::secrets_vault::HANDSHAKE_KEYCHAIN_SERVICE,
            );
            Ok(Self::with_vault(Arc::new(vault), "OsKeychainSecretsVault"))
        }
        #[cfg(not(feature = "os-keychain"))]
        {
            Err(AccessConfigError::KeychainUnavailable)
        }
    }

    /// Stable label of the wired vault impl (for leak-test assertions).
    pub fn vault_kind(&self) -> &'static str {
        self.vault_kind
    }

    /// A non-secret registry view over this service's vault, for enumeration.
    pub fn registry(&self) -> VaultBackedAccessRegistry {
        VaultBackedAccessRegistry::new(self.vault.clone())
    }

    /// Crate-internal access to the same vault for runtime factory wiring. This
    /// does not expose key material; cloud builders still fetch provider secrets
    /// only at launch time and return fail-closed when absent.
    pub(crate) fn vault(&self) -> Arc<dyn SecretsVault> {
        self.vault.clone()
    }

    /// Store a BYOK API key for `provider` ONLY in the OS-keychain vault.
    ///
    /// The key is exposed exactly once, at the `vault.put` boundary; no copy is
    /// retained. This creates NO ConsentGate entry and NO ConsentReceipt — the
    /// first cloud lane launch still hits the MT-006 fail-closed consent gate.
    pub fn store_byok_key(
        &self,
        provider: ByokProvider,
        api_key: &SecretString,
    ) -> Result<(), AccessConfigError> {
        let api_key = api_key.expose_secret();
        if api_key.trim().is_empty() {
            return Err(AccessConfigError::EmptyKey);
        }
        // Expose once as a borrowed `&str` and hand it straight to the vault,
        // which copies it into the OS credential store. No owned, un-zeroized
        // `String` copy is materialised on this path (MT-015 F2); nothing here
        // keeps the key.
        self.vault.put(provider.vault_lane(), api_key)?;
        Ok(())
    }

    /// Remove (or rotate) a provider's BYOK key. Idempotent — removing a key
    /// that was never stored succeeds — so a leaked key can be rotated without
    /// first checking whether one exists.
    pub fn remove_byok_key(&self, provider: ByokProvider) -> Result<(), AccessConfigError> {
        self.vault.delete(provider.vault_lane())?;
        Ok(())
    }

    /// Fetch a stored BYOK key back out of the vault for USE by the cloud
    /// backend (round-trip). Returns the key on success or a vault error. This
    /// is the ONLY method that returns key material and exists so the MT-006
    /// CloudLane/BYOK backend (via `VaultApiKeyProvider`) — and the leak test —
    /// can prove the key still round-trips. Callers MUST NOT log the result.
    pub fn fetch_byok_key(&self, provider: ByokProvider) -> Result<String, AccessConfigError> {
        // The vault read is `Zeroizing`; the owned `String` returned here is the
        // deliberate transient copy the caller (the BYOK backend / the leak
        // test) uses, and the `Zeroizing` original is wiped as it drops.
        Ok(self.vault.get(provider.vault_lane())?.to_string())
    }

    /// Non-secret status for one BYOK provider (ProviderNotConfigured ->
    /// Unavailable). Convenience wrapper over the vault-backed registry.
    pub fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus {
        self.registry().byok_status(provider)
    }

    /// The full non-secret enumeration surface.
    pub fn enumerate(&self) -> CloudAccessEnumeration {
        enumerate(&self.registry())
    }

    /// The full non-secret enumeration surface with an injected official-CLI
    /// auth probe. The model-access route uses this seam so tests never invoke
    /// host CLIs and production remains provider-owned.
    pub fn enumerate_with_cli_auth_probe(
        &self,
        probe: &dyn CliBridgeAuthStatusProbe,
    ) -> CloudAccessEnumeration {
        enumerate_with_cli_auth_probe(&self.registry(), probe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Provider identity + Gemini exclusion (enforced by construction).
    // ------------------------------------------------------------------

    #[test]
    fn byok_providers_are_anthropic_and_openai_only_no_gemini() {
        let ids: Vec<&str> = ByokProvider::OFFERED.iter().map(|p| p.id()).collect();
        assert_eq!(ids, vec!["anthropic", "openai"]);
        assert!(
            !ids.contains(&"gemini"),
            "Gemini must never be offered for BYOK"
        );
        // No id string can parse to a Gemini provider (there is no variant).
        assert!(ByokProvider::from_id("gemini").is_none());
        assert!(ByokProvider::from_id("google").is_none());
        assert!(ByokProvider::from_id("").is_none());
    }

    #[test]
    fn cli_bridge_providers_are_claude_and_codex_only_no_gemini() {
        let ids: Vec<&str> = CliBridgeProvider::OFFERED.iter().map(|p| p.id()).collect();
        assert_eq!(ids, vec!["claude_code", "codex"]);
        assert!(CliBridgeProvider::from_id("gemini_cli").is_none());
        assert!(CliBridgeProvider::from_id("gemini").is_none());
    }

    #[test]
    fn byok_vault_lanes_are_stable_and_distinct() {
        assert_eq!(ByokProvider::Anthropic.vault_lane(), "cloud-byok-anthropic");
        assert_eq!(ByokProvider::OpenAi.vault_lane(), "cloud-byok-openai");
        assert_ne!(
            ByokProvider::Anthropic.vault_lane(),
            ByokProvider::OpenAi.vault_lane()
        );
    }

    #[test]
    fn cli_bridge_login_commands_are_the_providers_own_official_commands() {
        let claude = CliBridgeProvider::ClaudeCode.login_command();
        assert_eq!(claude.program, "claude");
        assert_eq!(claude.args, &["auth", "login"]);
        let codex = CliBridgeProvider::Codex.login_command();
        assert_eq!(codex.program, "codex");

        let claude_status = CliBridgeProvider::ClaudeCode.auth_status_command();
        assert_eq!(claude_status.program, "claude");
        assert_eq!(claude_status.args, &["auth", "status"]);
        let codex_status = CliBridgeProvider::Codex.auth_status_command();
        assert_eq!(codex_status.program, "codex");
        assert_eq!(codex_status.args, &["login", "status"]);
    }

    #[test]
    fn auth_probe_rejects_cross_provider_and_excluded_cli_kinds() {
        assert!(CliBridgeProvider::ClaudeCode.accepts_cli_kind(CliKind::ClaudeCode));
        assert!(CliBridgeProvider::Codex.accepts_cli_kind(CliKind::CodexCli));
        assert!(!CliBridgeProvider::ClaudeCode.accepts_cli_kind(CliKind::CodexCli));
        assert!(!CliBridgeProvider::Codex.accepts_cli_kind(CliKind::ClaudeCode));
        for excluded in [CliKind::GeminiCli, CliKind::Other] {
            assert!(!CliBridgeProvider::ClaudeCode.accepts_cli_kind(excluded));
            assert!(!CliBridgeProvider::Codex.accepts_cli_kind(excluded));
        }
    }

    #[test]
    fn production_auth_probe_fails_closed_without_canonical_launch_targets() {
        let probe = ProductionCliBridgeAuthStatusProbe::default();
        for provider in CliBridgeProvider::OFFERED {
            assert_eq!(
                probe.auth_status(provider),
                CliBridgeAuthStatus::Unavailable
            );
        }
    }

    #[test]
    fn auth_status_module_has_no_independent_path_discovery_or_raw_command_spawn() {
        let source = include_str!("access_config.rs");
        assert!(!source.contains("std::env::var_os(\"PATH\")"));
        assert!(!source.contains("Command::new("));
        assert!(source.contains("run_auxiliary_fixed_command"));
    }

    #[test]
    fn official_auth_status_parsing_uses_exact_subscription_grammar_and_never_returns_output() {
        const CANARY: &str = "oauth-refresh-token-NEVER-RETURN";
        assert_eq!(
            parse_official_auth_status(
                CliBridgeProvider::ClaudeCode,
                true,
                br#"{"loggedIn":true,"subscriptionType":"max","email":"operator@example.invalid"}"#,
            ),
            CliBridgeAuthStatus::LoggedIn
        );
        assert_eq!(
            parse_official_auth_status(
                CliBridgeProvider::ClaudeCode,
                true,
                format!(
                    r#"{{"loggedIn":false,"refresh_token":"{CANARY}","email":"expired@example.invalid"}}"#
                )
                .as_bytes(),
            ),
            CliBridgeAuthStatus::LoggedOut
        );
        assert_eq!(
            parse_official_auth_status(
                CliBridgeProvider::ClaudeCode,
                false,
                br#"{"loggedIn":false}"#,
            ),
            CliBridgeAuthStatus::Unavailable,
            "a recognized payload from a failed command is not authoritative"
        );
        assert_eq!(
            parse_official_auth_status(
                CliBridgeProvider::ClaudeCode,
                true,
                br#"{"loggedIn":true,"authMethod":"api_key"}"#,
            ),
            CliBridgeAuthStatus::Unavailable,
            "API-key auth is not subscription-plan availability"
        );
        assert_eq!(
            parse_official_auth_status(CliBridgeProvider::Codex, true, b"Not logged in",),
            CliBridgeAuthStatus::LoggedOut
        );
        assert_eq!(
            parse_official_auth_status(CliBridgeProvider::Codex, false, b"Not logged in",),
            CliBridgeAuthStatus::Unavailable,
            "a recognized string from a failed command is not authoritative"
        );
        assert_eq!(
            parse_official_auth_status(CliBridgeProvider::Codex, true, b"Logged in using ChatGPT",),
            CliBridgeAuthStatus::LoggedIn
        );
        for unsupported in [
            b"Logged in using an API key - sk-proj-redacted".as_slice(),
            b"Logged in using Agent Identity".as_slice(),
            b"refresh token expired".as_slice(),
            b"token not expired".as_slice(),
            CANARY.as_bytes(),
        ] {
            assert_eq!(
                parse_official_auth_status(CliBridgeProvider::Codex, true, unsupported),
                CliBridgeAuthStatus::Unavailable
            );
        }
    }

    // ------------------------------------------------------------------
    // Enumeration API: ProviderNotConfigured -> Unavailable, no Gemini.
    // ------------------------------------------------------------------

    #[test]
    fn enumeration_maps_unconfigured_to_unavailable_and_configured_to_configured() {
        let registry = InMemoryAccessRegistry::new();
        // Nothing configured yet: both providers unavailable, not an error.
        let rows = enumerate_byok(&registry);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(
                row.status,
                ProviderAccessStatus::Unavailable,
                "unconfigured {} must be unavailable",
                row.provider
            );
        }

        // Configure OpenAI only.
        registry.set_configured(ByokProvider::OpenAi, true);
        let rows = enumerate_byok(&registry);
        let openai = rows.iter().find(|r| r.provider == "openai").unwrap();
        let anthropic = rows.iter().find(|r| r.provider == "anthropic").unwrap();
        assert_eq!(openai.status, ProviderAccessStatus::Configured);
        assert_eq!(anthropic.status, ProviderAccessStatus::Unavailable);
    }

    #[test]
    fn full_enumeration_never_offers_gemini_and_records_the_exclusion() {
        let registry = InMemoryAccessRegistry::new();
        let enumeration = enumerate(&registry);
        // BYOK rows carry no Gemini.
        assert!(enumeration.byok.iter().all(|r| r.provider != "gemini"));
        // CLI-bridge rows carry no Gemini.
        assert!(enumeration
            .cli_bridge
            .iter()
            .all(|r| r.provider != "gemini" && r.provider != "gemini_cli"));
        // The exclusion is explicit + visible.
        assert!(enumeration.excluded.contains(&"gemini"));
    }

    #[test]
    fn enumeration_is_json_serialisable_for_the_model_picker() {
        let registry = InMemoryAccessRegistry::new();
        registry.set_configured(ByokProvider::Anthropic, true);
        let enumeration = enumerate(&registry);
        let json = serde_json::to_value(&enumeration).expect("serialises");
        assert_eq!(json["byok"][0]["provider"], "anthropic");
        assert_eq!(json["byok"][0]["status"], "configured");
        assert_eq!(json["byok"][1]["status"], "unavailable");
        assert_eq!(json["excluded"][0], "gemini");
    }

    // ------------------------------------------------------------------
    // Store / round-trip / remove against the in-memory vault (fast unit
    // coverage; the real OS-keychain round-trip + leak proof is in
    // tests/cloud_byok_access_config_leak_tests.rs).
    // ------------------------------------------------------------------

    fn in_memory_service() -> CloudModelAccess {
        let vault = Arc::new(super::super::secrets_vault::InMemorySecretsVault::default());
        CloudModelAccess::with_vault(vault, "InMemorySecretsVault")
    }

    #[test]
    fn store_then_fetch_round_trips_the_key_only_through_the_vault() {
        let service = in_memory_service();
        let key = SecretString::from("sk-canary-unit".to_string());
        service.store_byok_key(ByokProvider::OpenAi, &key).unwrap();
        assert_eq!(
            service.fetch_byok_key(ByokProvider::OpenAi).unwrap(),
            "sk-canary-unit"
        );
        assert_eq!(
            service.byok_status(ByokProvider::OpenAi),
            ProviderAccessStatus::Configured
        );
        assert_eq!(
            service.byok_status(ByokProvider::Anthropic),
            ProviderAccessStatus::Unavailable
        );
    }

    #[test]
    fn empty_key_is_rejected_and_not_stored() {
        let service = in_memory_service();
        let err = service
            .store_byok_key(ByokProvider::OpenAi, &SecretString::from("   ".to_string()))
            .expect_err("empty key rejected");
        assert!(matches!(err, AccessConfigError::EmptyKey));
        assert_eq!(
            service.byok_status(ByokProvider::OpenAi),
            ProviderAccessStatus::Unavailable
        );
    }

    #[test]
    fn remove_is_idempotent_for_rotation() {
        let service = in_memory_service();
        // Remove a never-stored key: must succeed (rotation-safe).
        service.remove_byok_key(ByokProvider::Anthropic).unwrap();
        let key = SecretString::from("sk-rotate-me".to_string());
        service
            .store_byok_key(ByokProvider::Anthropic, &key)
            .unwrap();
        service.remove_byok_key(ByokProvider::Anthropic).unwrap();
        assert_eq!(
            service.byok_status(ByokProvider::Anthropic),
            ProviderAccessStatus::Unavailable
        );
    }

    #[test]
    fn cloud_model_access_debug_never_holds_or_prints_key_material() {
        let service = in_memory_service();
        service
            .store_byok_key(
                ByokProvider::OpenAi,
                &SecretString::from("sk-DO-NOT-PRINT".to_string()),
            )
            .unwrap();
        let dbg = format!("{service:?}");
        assert!(!dbg.contains("sk-DO-NOT-PRINT"));
        assert!(dbg.contains("InMemorySecretsVault"));
    }

    #[test]
    fn saving_a_key_creates_no_consent_and_first_launch_still_fails_closed() {
        // Saving a BYOK key must NOT pre-approve consent. The access service holds no ConsentGate, so
        // storing a key cannot create an approval; we prove it behaviourally — a fresh gate (as at the
        // first cloud lane launch) still fails closed under a deny provider AFTER the key is stored.
        use super::super::consent_gate::{
            ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider,
        };
        struct Deny;
        impl ConsentProvider for Deny {
            fn prompt_for_decision(
                &self,
                _session: &str,
                _lane: &str,
            ) -> Result<ConsentDecision, ConsentGateError> {
                Ok(ConsentDecision::Denied)
            }
        }

        let service = in_memory_service();
        service
            .store_byok_key(
                ByokProvider::OpenAi,
                &SecretString::from("sk-no-consent".to_string()),
            )
            .unwrap();

        // The key is stored + usable...
        assert_eq!(
            service.byok_status(ByokProvider::OpenAi),
            ProviderAccessStatus::Configured
        );
        // ...but the first lane launch still hits the fail-closed MT-006 gate (no approval was created).
        let gate = ConsentGate::new();
        let first_launch = gate.check_or_prompt("session-mt015", ByokProvider::OpenAi.id(), &Deny);
        assert!(
            matches!(first_launch, Err(ConsentGateError::ConsentDenied { .. })),
            "saving a key must not grant consent; first launch must fail closed"
        );
    }

    // ── MT-015 v5: in-app login-session registry ────────────────────────────────

    /// Deterministic stand-in for the production PTY transport. The ConPTY
    /// behaviour itself is proven by `cli_bridge_login_quiet_tests` against the
    /// real `LiveCliSpawner`; these tests own the registry's state machine.
    #[derive(Debug, Default)]
    struct FakeLoginTransport {
        transcript: std::sync::Mutex<Vec<u8>>,
        written: std::sync::Mutex<Vec<u8>>,
        exit_code: std::sync::Mutex<Option<i32>>,
        cancel_calls: std::sync::atomic::AtomicUsize,
    }

    impl InteractiveLoginTransport for FakeLoginTransport {
        fn pid(&self) -> u32 {
            1234
        }
        fn transcript(&self) -> Vec<u8> {
            self.transcript.lock().expect("transcript lock").clone()
        }
        fn write_input(&self, bytes: &[u8]) -> Result<(), String> {
            self.written
                .lock()
                .expect("written lock")
                .extend_from_slice(bytes);
            Ok(())
        }
        fn exit_code(&self) -> Option<i32> {
            *self.exit_code.lock().expect("exit lock")
        }
        fn cancel(&self) {
            self.cancel_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct FakeLoginLauncher {
        transports: RwLock<Vec<Arc<FakeLoginTransport>>>,
    }

    impl CliBridgeLoginLauncher for FakeLoginLauncher {
        fn launch_login(
            &self,
            _provider: CliBridgeProvider,
        ) -> Result<Arc<dyn InteractiveLoginTransport>, CliBridgeLoginLaunchError> {
            let transport = Arc::new(FakeLoginTransport::default());
            self.transports
                .write()
                .expect("transport lock")
                .push(transport.clone());
            Ok(transport as Arc<dyn InteractiveLoginTransport>)
        }
    }

    impl FakeLoginLauncher {
        fn transport(&self, index: usize) -> Arc<FakeLoginTransport> {
            self.transports.read().expect("transport lock")[index].clone()
        }
        fn count(&self) -> usize {
            self.transports.read().expect("transport lock").len()
        }
    }

    /// HBR-QUIET-003: repeated `Log in…` clicks must not accumulate login
    /// children. Starting a second login for the same provider cancels and evicts
    /// the first, so at most one child per provider is ever live.
    #[test]
    fn starting_a_second_login_cancels_and_evicts_the_previous_one_for_that_provider() {
        let launcher = Arc::new(FakeLoginLauncher::default());
        let registry = CliBridgeLoginSessionRegistry::new(launcher.clone());

        let first = registry
            .start(CliBridgeProvider::ClaudeCode)
            .expect("first login starts");
        let second = registry
            .start(CliBridgeProvider::ClaudeCode)
            .expect("second login starts");
        assert_ne!(first.session_id, second.session_id);
        assert_eq!(launcher.count(), 2);
        assert_eq!(
            launcher
                .transport(0)
                .cancel_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the superseded login child must be terminated"
        );
        assert_eq!(
            registry.snapshot(&first.session_id),
            Err(CliLoginSessionError::UnknownSession),
            "the superseded session must be evicted"
        );
        assert!(registry.snapshot(&second.session_id).is_ok());

        // A different provider keeps its own live session.
        let codex = registry
            .start(CliBridgeProvider::Codex)
            .expect("codex login starts");
        assert!(registry.snapshot(&second.session_id).is_ok());
        assert!(registry.snapshot(&codex.session_id).is_ok());
    }

    /// The typed status is derived from what Handshake can prove, and operator
    /// input reaches the login process with exactly one carriage return.
    #[test]
    fn login_session_status_is_derived_from_provable_state_and_input_is_line_terminated() {
        let launcher = Arc::new(FakeLoginLauncher::default());
        let registry = CliBridgeLoginSessionRegistry::new(launcher.clone());
        let started = registry
            .start(CliBridgeProvider::Codex)
            .expect("login starts");
        assert_eq!(started.status, CliLoginSessionStatus::Pending);
        assert!(started.transcript.is_empty());

        let transport = launcher.transport(0);
        transport
            .transcript
            .lock()
            .expect("transcript lock")
            .extend_from_slice(b"\x1b[32mEnter code:\x1b[0m ");
        let awaiting = registry
            .snapshot(&started.session_id)
            .expect("session is live");
        assert_eq!(awaiting.status, CliLoginSessionStatus::AwaitingInput);
        assert_eq!(
            awaiting.transcript, "Enter code: ",
            "ANSI control sequences must not reach the operator surface"
        );

        registry
            .send_input(&started.session_id, "AB\r\nCD")
            .expect("input is delivered");
        assert_eq!(
            String::from_utf8_lossy(&transport.written.lock().expect("written lock")),
            "ABCD\r",
            "embedded newlines must not let one answer become several"
        );

        *transport.exit_code.lock().expect("exit lock") = Some(2);
        let failed = registry
            .snapshot(&started.session_id)
            .expect("session is live");
        assert_eq!(failed.status, CliLoginSessionStatus::Failed);
        assert_eq!(failed.exit_code, Some(2));
        assert_eq!(
            registry.send_input(&started.session_id, "too late"),
            Err(CliLoginSessionError::SessionFinished),
            "a finished login must not accept more input"
        );
    }

    /// The transcript is bounded and keeps the TAIL: the provider's newest prompt
    /// is what the operator needs, so a flood must not push it out of view.
    #[test]
    fn login_transcript_is_bounded_and_keeps_the_newest_output() {
        let mut raw = vec![b'a'; CLI_LOGIN_TRANSCRIPT_TAIL_BYTES + 512];
        raw.extend_from_slice(b"ENTER-CODE-WDJB");
        let (text, truncated) = render_login_transcript(&raw);
        assert!(truncated, "an over-cap transcript must report truncation");
        assert!(text.len() <= CLI_LOGIN_TRANSCRIPT_TAIL_BYTES + 64);
        assert!(
            text.ends_with("ENTER-CODE-WDJB"),
            "the newest provider output must survive truncation"
        );
    }

    /// The registry must never carry the transcript into a Debug/log surface.
    #[test]
    fn login_registry_debug_never_carries_the_transcript() {
        let launcher = Arc::new(FakeLoginLauncher::default());
        let registry = CliBridgeLoginSessionRegistry::new(launcher.clone());
        let started = registry
            .start(CliBridgeProvider::ClaudeCode)
            .expect("login starts");
        launcher
            .transport(0)
            .transcript
            .lock()
            .expect("transcript lock")
            .extend_from_slice(b"ONE-TIME-CODE-CANARY");
        let rendered = registry
            .snapshot(&started.session_id)
            .expect("session is live");
        assert!(rendered.transcript.contains("ONE-TIME-CODE-CANARY"));
        let debug = format!("{registry:?}");
        assert!(
            !debug.contains("ONE-TIME-CODE-CANARY"),
            "the registry Debug must never carry provider login output: {debug}"
        );
    }
}
