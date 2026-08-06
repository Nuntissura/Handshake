//! Palmistry RECOVERY-TIME FLIGHT-RECORDER FORWARDER (MT-093, Master Spec v02.196 §6.13.7).
//!
//! When Handshake RECOVERS (unfreezes after a freeze, or a new instance reconnects after a crash-
//! restart), Palmistry drains the unforwarded [`crate::survivor_store::SurvivorRecord`]s and FORWARDS the
//! captured out-of-process freeze/crash evidence into the Tier-1 Flight Recorder via the EXISTING FR HTTP
//! route (reuse-via-API), so the evidence rejoins the governed business-event ledger and the §10.12.5
//! Diagnostics Panel Tier-3 section becomes populated post-recovery.
//!
//! # WP-KERNEL-012 MT-115 — THE ROUTE THIS MODULE TARGETED NO LONGER EXISTS
//!
//! MT-109 removed the unscoped `POST /api/flight_recorder/runtime_chat_event` and replaced the whole
//! flight-recorder route group with workspace-scoped, capability-gated routes that require a live
//! native-MCP session credential. Palmistry has no workspace id and, by design, runs when the app is
//! frozen or dead — exactly when no live binding can exist. It therefore satisfies neither half of
//! the new contract. The route constant is REMOVED (see the note above `FrForwarder`), the forwarder
//! now takes its target path explicitly, and production binds NONE. Nothing regressed into silence:
//! the typed [`FrForwardBlocker::SchemaIncompatible`] deferral to [`FR_INGESTION_FOLLOW_ON_WP`] still
//! fires before any network call, and each survivor record stays local and flagged pending.
//!
//! # The formerly-existing route (historical, kept for the closed-schema evidence below)
//!
//! `POST /api/flight_recorder/runtime_chat_event` WAS the FR ingestion endpoint. The in-repo
//! reference for its EXACT accepted body shape + requirements is
//! `handshake_native/src/event_emitter.rs` (the MT-036 `RuntimeChatLedgerTransport`), which posts to
//! exactly this route and documents (verified against `src/backend/handshake_core`):
//!   1. the body is `RuntimeChatEventV0_1` with `#[serde(deny_unknown_fields)]` — ONLY the known keys
//!      are accepted (a nested freeze/crash payload field is REJECTED 400),
//!   2. its `type` is a CLOSED 3-value enum (`runtime_chat_message_appended` / `_ans001_validation` /
//!      `_session_closed`) — there is NO `system`/`freeze`/`crash` variant,
//!   3. the handler HARDCODES `actor_id = "runtime_chat"` + `actor = System` — a custom actor cannot be
//!      set through this endpoint,
//!   4. `session_id` MUST parse as a non-nil UUID, else 400.
//!
//! We REUSE this route and do NOT invent a new one and do NOT edit the FR module (it is `forbidden_paths`).
//!
//! # The HONESTY GATE (HARD, §6.13.7 + the FR-kept-as-is constraint — AC-013-4 / RISK-013-1/2)
//!
//! Because the verified `runtime_chat_event` schema is a CHAT-event shape, it CANNOT FAITHFULLY carry a
//! freeze/crash survivor record: the survivor's defining typed fields — `event_code`
//! (FreezeSuspected/CrashDetected), `stale_ms`, `exit_code`, `faulting_thread_id`, `minidump_path`, the
//! last-heartbeat counter/timestamp — have NO field in the closed `RuntimeChatEventV0_1` schema, and the
//! closed `type` enum has no freeze/crash variant. Therefore the correct, HONEST outcome of forwarding a
//! survivor record into the EXISTING route is a TYPED BLOCKER ([`FrForwardBlocker::SchemaIncompatible`]),
//! NOT a faked success: the record stays LOCAL flagged `forward pending` and a follow-on (WP-KERNEL-016)
//! is noted to add a proper FR ingestion shape WITHOUT touching the FR here. This mirrors
//! `event_emitter.rs`'s existing typed-backend-blocker for the absent native-editor ingestion shape.
//!
//! A forward MUST NOT fabricate a success, MUST NOT edit the FR, and MUST NOT invent a new route. We post
//! the CLOSEST-valid `RuntimeChatEventV0_1` body only as the SWAPPABLE wire seam (so the forwarder is
//! correct + live the instant the backend gap closes), and the honesty assessment decides whether a 2xx
//! on that closest body counts as a faithful forward (it does NOT for the chat-event route — that is the
//! blocker). The same code FAITHFULLY forwards + marks-forwarded against a route that CAN carry the
//! survivor shape (the WP-016 endpoint, or the stub server the AC-013-3 test asserts the body against).

use std::time::Duration;

use serde_json::{json, Value as JsonValue};

use crate::survivor_store::SurvivorRecord;

// ── WP-KERNEL-012 MT-115: the removed route constant ──────────────────────────────────────────────
//
// This module used to export `FR_ROUTE_PATH = "/api/flight_recorder/runtime_chat_event"`, described as
// "the EXISTING verified FR ingestion endpoint". WP-KERNEL-012 MT-109 DELETED that route. The
// flight-recorder route group is now workspace-scoped and capability-gated:
// `POST /api/workspaces/{workspace_id}/flight_recorder/runtime_chat_event`, behind fail-closed
// middleware that requires an `x-hsk-session-token` matching the on-disk native-MCP binding whose
// recorded owner process is still ALIVE with the same OS birth identity
// (`handshake_core::api::stage::capture_context`).
//
// Palmistry can satisfy NEITHER half, and not by accident:
//   * it has no workspace id, so it cannot form the path, and
//   * it is an out-of-process watcher whose entire reason to exist is surviving a Handshake freeze or
//     crash — precisely when the app process is gone and `process_birth_identity` therefore fails, so
//     even reading the app's binding file yields `401 HSK-401-FR-SESSION` (`StaleBinding`).
//
// So the constant was not merely stale, it named a route Palmistry could never have used. It is
// REMOVED rather than renamed or re-pointed, so no future reader (or WP-KERNEL-016 implementer)
// believes an ingestion route already exists. The forwarder now takes the target path EXPLICITLY;
// production binds none, which is the honest state of the world. The typed
// [`FrForwardBlocker::SchemaIncompatible`] deferral to [`FR_INGESTION_FOLLOW_ON_WP`] is unchanged and
// still short-circuits every production forward BEFORE any network call.

/// The verified default FR base URL (the loopback the Handshake backend serves on — the in-repo
/// reference 127.0.0.1:37501). A caller may override it (tests point at a local stub).
pub const FR_DEFAULT_BASE_URL: &str = "http://127.0.0.1:37501";

/// The verified RuntimeChatEvent wire schema version (the body the route accepts —
/// `event_emitter.rs::FR_RUNTIME_CHAT_SCHEMA_VERSION`).
pub const FR_RUNTIME_CHAT_SCHEMA_VERSION: &str = "hsk.fr.runtime_chat@0.1";

/// The follow-on WP that must add a PROPER FR ingestion shape for survivor records (WITHOUT touching the
/// FR here). Named in the typed blocker so the honest gap is tracked, not papered over.
pub const FR_INGESTION_FOLLOW_ON_WP: &str = "WP-KERNEL-016";

/// Whether the EXISTING `runtime_chat_event` route can FAITHFULLY carry a survivor record. Verified
/// against the route's known closed schema (via `event_emitter.rs`): it CANNOT (a chat-event shape has no
/// field for the survivor's typed freeze/crash data, and the closed `type` enum has no freeze/crash
/// variant). This is the honesty input AC-013-4 keys on — see the module docs.
///
/// Returns the typed [`FrSchemaCompat`] so the forwarder, a reviewer, and the test reason over a typed
/// verdict rather than a bool — and so a future route that CAN carry the shape is representable
/// ([`FrSchemaCompat::Compatible`]) without changing the forwarder's contract.
pub fn runtime_chat_event_compatibility() -> FrSchemaCompat {
    // VERIFIED INCOMPATIBLE: the closed RuntimeChatEventV0_1 schema (deny_unknown_fields, closed `type`
    // enum, hardcoded actor) has no representation for the survivor's typed fields (event_code, stale_ms,
    // exit_code, faulting_thread_id, minidump_path, last heartbeat). A faithful forward needs the WP-016
    // ingestion shape. (event_emitter.rs reached the same verified verdict for the native-editor event.)
    FrSchemaCompat::Incompatible {
        reason:
            "runtime_chat_event is a closed chat-event schema (deny_unknown_fields, closed `type` enum, \
             hardcoded actor_id) with no field for the survivor's typed freeze/crash data \
             (event_code/stale_ms/exit_code/faulting_thread_id/minidump_path/last_heartbeat)"
                .to_owned(),
        follow_on_wp: FR_INGESTION_FOLLOW_ON_WP,
    }
}

/// Whether an FR route can FAITHFULLY carry a survivor record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrSchemaCompat {
    /// The route CAN faithfully carry the typed survivor record (the WP-016 ingestion shape, or a stub
    /// that accepts the survivor-faithful body). A forward against this route really forwards the record.
    Compatible,
    /// The route CANNOT faithfully carry the survivor record (the existing chat-event route). Forwarding
    /// into it is a TYPED BLOCKER, not a fake success — the record stays local flagged pending.
    Incompatible {
        /// Why it cannot carry the record (verified against the closed schema).
        reason: String,
        /// The follow-on WP that must add a proper ingestion shape WITHOUT editing the FR here.
        follow_on_wp: &'static str,
    },
}

/// The typed blocker a forward returns when it cannot HONESTLY complete (AC-013-4 / RISK-013-1/2). NEVER
/// a faked success: each variant leaves the survivor record LOCAL + flagged pending so the next recovery
/// re-drains it, and surfaces the honest reason. The FR module is NEVER edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrForwardBlocker {
    /// The EXISTING route's schema cannot faithfully represent the survivor record (the chat-event route
    /// has no freeze/crash field). The honest outcome of forwarding into the kept-as-is FR — a follow-on
    /// WP must add a proper ingestion shape. Carries the verified reason + the follow-on WP.
    SchemaIncompatible {
        /// Why the schema cannot carry the record.
        reason: String,
        /// The follow-on WP (WP-KERNEL-016) that adds the proper ingestion shape.
        follow_on_wp: &'static str,
    },
    /// The route is ABSENT / the FR is not reachable (connection refused, DNS, timeout). The record stays
    /// pending for the next recovery. Carries the transport reason.
    RouteAbsent {
        /// The transport-level reason (connection refused / timeout / etc.).
        reason: String,
    },
    /// The route was reached but REJECTED the body (a non-2xx HTTP status — e.g. 400 from
    /// deny_unknown_fields / a closed-enum violation). The record stays pending. Carries the status code.
    Rejected {
        /// The HTTP status code the route returned.
        status: u16,
        /// The response body text (truncated), for honest diagnosis — never used as a success signal.
        body: String,
    },
}

impl std::fmt::Display for FrForwardBlocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrForwardBlocker::SchemaIncompatible { reason, follow_on_wp } => write!(
                f,
                "FR forward blocked (schema incompatible): {reason}; the existing \
                 runtime_chat_event route is kept as-is — {follow_on_wp} must add a survivor ingestion \
                 shape (record kept local, forward pending — NOT faked)"
            ),
            FrForwardBlocker::RouteAbsent { reason } => {
                write!(f, "FR forward blocked (route absent/unreachable): {reason} (record kept local, pending)")
            }
            FrForwardBlocker::Rejected { status, body } => write!(
                f,
                "FR forward blocked (route rejected the body): HTTP {status}: {body} (record kept local, pending)"
            ),
        }
    }
}

impl std::error::Error for FrForwardBlocker {}

/// Build the survivor record's FR-forward marker `message_id`: `palmistry_survivor:<kind>:<session>`. A
/// fixed-vocabulary kind tag + the opaque session token — NO project content / free text (the typed-
/// allowlist of the whole substrate, RISK-013-3). This is the identity the closed `runtime_chat_event`
/// body CAN carry (in its `message_id` field), mirroring `event_emitter.rs`'s `native_editor:<action>`.
pub fn survivor_marker(record: &SurvivorRecord) -> String {
    format!(
        "palmistry_survivor:{}:{}",
        record.kind.wire_tag(),
        crate::survivor_store::safe_session_token(&record.session_id)
    )
}

/// WP-KERNEL-016 SEAM — PRODUCTION-DEAD BY DESIGN (MT-093 remediation label). NO production path calls
/// this: `FrForwarder::forward` posts the survivor-faithful body ([`build_survivor_forward_body`]) on a
/// Compatible route and returns the typed [`FrForwardBlocker::SchemaIncompatible`] BEFORE any post on
/// the existing (Incompatible) chat-event route — so a degraded chat-event post is never sent and this
/// builder is deliberately unreachable in the shipped composition. It is KEPT (not deleted) as the
/// documented seam pinning the EXACT verified `RuntimeChatEventV0_1` key set the closed route accepts
/// (`runtime_chat_event_body_has_only_verified_keys` asserts it), so when WP-KERNEL-016 designs the
/// survivor ingestion shape the closed-schema evidence stays executable in-repo. If WP-016 chooses a
/// different bridge, DELETE this fn + its test in that WP.
///
/// Build the EXACT verified `RuntimeChatEventV0_1` body for forwarding `record`, carrying ONLY the typed
/// survivor fields the CLOSED schema can hold (the identity marker in `message_id`). Kept SEPARATE from
/// the network post so a unit test can assert every required key + the deny_unknown_fields constraint
/// with no IO.
///
/// What the closed schema CAN carry of the survivor record:
/// - `message_id` = the typed [`survivor_marker`] (kind + opaque session) — the survivor identity.
/// - `session_id` = a non-nil UUID the route requires (the caller supplies a stable per-session UUID).
/// - `type` = the only general-purpose closed variant (`runtime_chat_message_appended`).
///
/// What it CANNOT carry (the honesty gap — these typed survivor fields have NO destination key, which is
/// exactly why a faithful forward needs the WP-016 ingestion shape): `event_code`, `stale_ms`,
/// `exit_code`, `faulting_thread_id`, `minidump_path`, `last_heartbeat_*`, `probe_result`. The forward
/// does NOT smuggle them into an unknown field (deny_unknown_fields would 400, and it would be content
/// drift); they are simply not representable here, which the [`FrForwardBlocker::SchemaIncompatible`]
/// honesty gate reports.
pub fn build_runtime_chat_event_body(record: &SurvivorRecord, session_uuid: &str) -> JsonValue {
    json!({
        "schema_version": FR_RUNTIME_CHAT_SCHEMA_VERSION,
        "event_id": new_uuid_string(),
        "ts_utc": now_rfc3339(),
        "session_id": session_uuid,
        "type": "runtime_chat_message_appended",
        "message_id": survivor_marker(record),
        "role": "system",
    })
}

/// Build the SURVIVOR-FAITHFUL forward body — the shape a PROPER FR ingestion endpoint (WP-016) or the
/// AC-013-3 stub accepts, carrying the FULL typed survivor record (all typed fields, NO free text). This
/// is what a faithful forward really sends; the closed `runtime_chat_event` route cannot accept it (which
/// is the [`FrForwardBlocker::SchemaIncompatible`] gate). Every value is a number / typed enum tag /
/// opaque token / LOCAL path string — the typed-allowlist (RISK-013-3); a test value-scans it.
pub fn build_survivor_forward_body(record: &SurvivorRecord) -> JsonValue {
    json!({
        "schema_version": "hsk.palmistry.survivor_forward@0.1",
        "kind": record.kind.wire_tag(),
        "session_id": record.session_id,
        "process_id": record.process_id,
        "event_code": record.event_code,
        "stale_ms": record.stale_ms,
        "last_heartbeat_counter": record.last_heartbeat_counter,
        "last_heartbeat_ts_nanos": record.last_heartbeat_ts_nanos,
        "last_event_count": record.last_event_count,
        "exit_code": record.exit_code,
        "faulting_thread_id": record.faulting_thread_id,
        "child_process_id": record.child_process_id,
        "child_session_id": record.child_session_id,
        "last_progress_counter": record.last_progress_counter,
        "last_progress_ts_nanos": record.last_progress_ts_nanos,
        "child_stall_reason_code": record.child_stall_reason_code,
        // The minidump path is a LOCAL path string reference only (never the bytes) — §6.13.8 local-only.
        "minidump_path": record.minidump_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
        "captured_at_unix_ms": record.captured_at_unix_ms,
    })
}

/// The recovery-time FR forwarder. Holds a blocking reqwest client, the FR base URL, the EXPLICIT
/// ingestion path (MT-115: `None` in production, because MT-109 removed the route this module used to
/// name and nothing has replaced it), and a stable non-nil session UUID. `compat` is the honesty
/// verdict on whether the target route can FAITHFULLY carry a survivor record: for the real FR it is
/// [`FrSchemaCompat::Incompatible`] (the typed blocker), for a future WP-016 ingestion shape or the
/// AC-013-3 stub it is [`FrSchemaCompat::Compatible`] (a real forward).
pub struct FrForwarder {
    client: reqwest::blocking::Client,
    base_url: String,
    /// MT-115: the EXPLICIT ingestion path this forwarder posts to, or `None` when no survivor
    /// ingestion route is bound. Production binds `None` — MT-109 removed the route this module used
    /// to hardcode, and nothing has replaced it (see the module-level note above).
    route_path: Option<String>,
    session_uuid: String,
    compat: FrSchemaCompat,
}

impl FrForwarder {
    /// Build a forwarder against the EXISTING FR route at `base_url`, with the verified honesty verdict
    /// ([`runtime_chat_event_compatibility`] — Incompatible) baked in. This is the PRODUCTION forwarder:
    /// a `forward` of any survivor record returns [`FrForwardBlocker::SchemaIncompatible`] (HONEST, not
    /// faked), because the kept-as-is chat-event route cannot carry the survivor shape (AC-013-4). The
    /// record stays local + pending; WP-016 adds the proper ingestion shape.
    /// MT-115: NO ingestion path is bound, because none exists. The honesty gate returns
    /// [`FrForwardBlocker::SchemaIncompatible`] before any URL is needed, so this is not a hidden
    /// default — it is the accurate statement that the survivor record has nowhere faithful to go.
    pub fn for_existing_fr(base_url: impl Into<String>) -> Self {
        Self::with_compat(base_url, None, runtime_chat_event_compatibility())
    }

    /// Build a forwarder with an EXPLICIT ingestion path + compatibility verdict + base URL.
    /// `FrSchemaCompat::Compatible` is used for a route that CAN carry the survivor shape (a future
    /// WP-016 ingestion endpoint, or the AC-013-3 stub) — a forward then really posts + marks
    /// forwarded. `Incompatible` returns the typed blocker. `route_path` is `Option` and REQUIRED at
    /// the call site rather than defaulted, so no caller can inherit a route that does not exist. The
    /// blocking reqwest client uses a bounded connect+read timeout so a forward can NEVER hang the
    /// watcher (a dead/absent FR returns a RouteAbsent blocker promptly).
    pub fn with_compat(
        base_url: impl Into<String>,
        route_path: Option<String>,
        compat: FrSchemaCompat,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            route_path,
            // A stable non-nil UUID for the route's required session_id (the route 400s a nil/non-UUID).
            session_uuid: new_uuid_string(),
            compat,
        }
    }

    /// The full forward URL, or `None` when no survivor ingestion path is bound (production).
    pub fn url(&self) -> Option<String> {
        self.route_path
            .as_ref()
            .map(|path| format!("{}{}", self.base_url.trim_end_matches('/'), path))
    }

    /// The non-nil session UUID this forwarder stamps on the route's required `session_id`.
    pub fn session_uuid(&self) -> &str {
        &self.session_uuid
    }

    /// The compatibility verdict (the honesty input).
    pub fn compat(&self) -> &FrSchemaCompat {
        &self.compat
    }

    /// Forward ONE survivor `record` to the FR (reuse-via-API). Returns:
    /// - `Ok(())` ONLY when the route really accepted a FAITHFUL forward (compat = Compatible AND a 2xx).
    ///   The caller then marks the record forwarded (idempotent).
    /// - `Err(FrForwardBlocker::SchemaIncompatible)` when the target route cannot faithfully carry the
    ///   survivor shape (the EXISTING chat-event route) — HONEST, NOT a faked success; the record stays
    ///   local pending + WP-016 is named (AC-013-4 / RISK-013-1/2).
    /// - `Err(FrForwardBlocker::RouteAbsent)` when the route is unreachable; `Rejected{status}` when it
    ///   returned a non-2xx. The record stays pending for the next recovery.
    ///
    /// It NEVER edits the FR and NEVER invents a route. The honesty gate is checked BEFORE the post for an
    /// incompatible route (we do not even pretend a degraded chat-event post is a survivor forward).
    pub fn forward(&self, record: &SurvivorRecord) -> Result<(), FrForwardBlocker> {
        // HONESTY GATE (AC-013-4): if the target route cannot faithfully carry the survivor record, the
        // correct outcome is a typed blocker — NOT a degraded post that 2xx's and looks forwarded.
        if let FrSchemaCompat::Incompatible {
            reason,
            follow_on_wp,
        } = &self.compat
        {
            return Err(FrForwardBlocker::SchemaIncompatible {
                reason: reason.clone(),
                follow_on_wp,
            });
        }

        // Compatible route (WP-016 ingestion / the AC-013-3 stub): post the survivor-faithful body and
        // honor the real HTTP outcome. A bounded-timeout blocking POST so a dead FR cannot hang.
        // MT-115: a Compatible verdict with NO bound ingestion path is reported as an absent route,
        // never as a silent success. This is unreachable in the shipped composition, where
        // `for_existing_fr` always short-circuits above.
        let url = self.url().ok_or_else(|| FrForwardBlocker::RouteAbsent {
            reason: "no survivor ingestion route is bound (MT-109 removed the unscoped Flight \
                     Recorder ingestion path and nothing has replaced it)"
                .to_owned(),
        })?;
        let body = build_survivor_forward_body(record);
        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .map_err(|e| FrForwardBlocker::RouteAbsent {
                reason: format!("{e}"),
            })?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let code = status.as_u16();
            let text = resp.text().unwrap_or_default();
            let body = text.chars().take(256).collect();
            Err(FrForwardBlocker::Rejected { status: code, body })
        }
    }
}

/// A fresh time-ordered UUID v7 string (the route's required non-nil session id + each event id).
/// HBR-INT-008 mandates `now_v7()` (time-ordered) over `new_v4()`.
fn new_uuid_string() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// An RFC3339 timestamp string for the route's required `ts_utc`.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::child_stall::{ChildStallReasonCode, ChildStallReport};
    use crate::freeze_detect::FreezeReport;
    use crate::survivor_store::{SurvivorProbeResult, SurvivorRecord};

    fn freeze_record() -> SurvivorRecord {
        SurvivorRecord::from_freeze(
            "sess-fr",
            4242,
            &FreezeReport {
                stale_ms: 6000,
                last_heartbeat_counter: 42,
                last_heartbeat_ts_nanos: 123,
            },
            3,
            SurvivorProbeResult::NotResponding,
        )
    }

    fn child_stall_record() -> SurvivorRecord {
        SurvivorRecord::from_child_stall(
            "sess-child-fr",
            4242,
            &ChildStallReport {
                child_pid: 333,
                child_session_id: 90,
                stale_ms: 6100,
                last_progress_counter: 5,
                last_progress_ts_nanos: 777,
                reason_code: ChildStallReasonCode::ProgressStaleWhileAlive,
            },
            4,
        )
    }

    #[test]
    fn existing_fr_is_verified_incompatible() {
        // The verified honesty verdict: the kept-as-is chat-event route cannot carry a survivor record.
        match runtime_chat_event_compatibility() {
            FrSchemaCompat::Incompatible { follow_on_wp, .. } => {
                assert_eq!(follow_on_wp, FR_INGESTION_FOLLOW_ON_WP);
            }
            FrSchemaCompat::Compatible => {
                panic!("the existing chat-event route must be Incompatible")
            }
        }
    }

    #[test]
    fn runtime_chat_event_body_has_only_verified_keys() {
        // The closest-valid RuntimeChatEventV0_1 body carries ONLY the verified keys (deny_unknown_fields
        // on the backend means an extra key would 400). Matches event_emitter.rs's accepted shape.
        let body = build_runtime_chat_event_body(&freeze_record(), &new_uuid_string());
        let obj = body.as_object().unwrap();
        let allowed: std::collections::HashSet<&str> = [
            "schema_version",
            "event_id",
            "ts_utc",
            "session_id",
            "type",
            "message_id",
            "role",
        ]
        .into_iter()
        .collect();
        for k in obj.keys() {
            assert!(
                allowed.contains(k.as_str()),
                "unexpected key {k} (would trip deny_unknown_fields)"
            );
        }
        // session_id is a non-nil UUID (the route 400s otherwise).
        let sid = obj["session_id"].as_str().unwrap();
        let parsed = uuid::Uuid::parse_str(sid).expect("session_id is a UUID");
        assert_ne!(parsed, uuid::Uuid::nil());
        // type is the closed general-purpose variant.
        assert_eq!(obj["type"], "runtime_chat_message_appended");
        // message_id carries the typed survivor marker (kind + opaque session) — no free text.
        assert_eq!(obj["message_id"], "palmistry_survivor:freeze:sess-fr");
    }

    #[test]
    fn survivor_forward_body_is_typed_allowlist_only() {
        // The survivor-faithful body (the WP-016 / stub shape) carries ONLY numbers, typed enum tags,
        // opaque tokens, and a LOCAL path string — NO free text (RISK-013-3).
        let body = build_survivor_forward_body(&freeze_record());
        let obj = body.as_object().unwrap();
        // String-valued keys are limited to the fixed schema vocabulary + the opaque session + the kind
        // tag (+ a local path when a crash carries one).
        assert_eq!(obj["kind"], "freeze");
        assert_eq!(obj["session_id"], "sess-fr");
        assert_eq!(
            obj["event_code"],
            handshake_diag_ring::DiagEventCode::FreezeSuspected.as_u16()
        );
        assert_eq!(obj["stale_ms"], 6000);
        // No free-text field present.
        assert!(obj.get("message").is_none());
        assert!(obj.get("text").is_none());
    }

    #[test]
    fn survivor_forward_body_carries_child_stall_fields() {
        let body = build_survivor_forward_body(&child_stall_record());
        assert_eq!(body["kind"], "child-stall");
        assert_eq!(body["session_id"], "sess-child-fr");
        assert_eq!(body["process_id"], 4242);
        assert_eq!(body["child_process_id"], 333);
        assert_eq!(body["child_session_id"], 90);
        assert_eq!(body["stale_ms"], 6100);
        assert_eq!(body["last_progress_counter"], 5);
        assert_eq!(body["last_progress_ts_nanos"], 777);
        assert_eq!(body["child_stall_reason_code"], 1);
    }

    #[test]
    fn survivor_marker_is_typed_no_free_text() {
        let m = survivor_marker(&freeze_record());
        assert_eq!(m, "palmistry_survivor:freeze:sess-fr");
    }

    #[test]
    fn forward_into_existing_fr_returns_typed_blocker_not_fake_success() {
        // AC-013-4 (the honesty gate): forwarding into the EXISTING (incompatible) route returns the
        // SchemaIncompatible typed blocker — NOT a faked success. The base URL is unreachable on purpose
        // (port 1) to PROVE the blocker is decided WITHOUT a network call (we never even post a degraded
        // chat-event body and call it forwarded).
        let fwd = FrForwarder::for_existing_fr("http://127.0.0.1:1");
        let err = fwd
            .forward(&freeze_record())
            .expect_err("must be a typed blocker, not Ok");
        match err {
            FrForwardBlocker::SchemaIncompatible { follow_on_wp, .. } => {
                assert_eq!(
                    follow_on_wp, FR_INGESTION_FOLLOW_ON_WP,
                    "names the WP-016 follow-on"
                );
            }
            other => panic!("expected SchemaIncompatible, got {other:?}"),
        }
    }

    #[test]
    fn forward_against_absent_compatible_route_is_route_absent_blocker() {
        // A COMPATIBLE route that is unreachable yields RouteAbsent (the record stays pending) — bounded,
        // never hangs. Port 1 is reliably refused.
        let fwd = FrForwarder::with_compat(
            "http://127.0.0.1:1",
            Some("/api/wp016/survivor_forward".to_owned()),
            FrSchemaCompat::Compatible,
        );
        let err = fwd
            .forward(&freeze_record())
            .expect_err("absent route must block");
        assert!(
            matches!(err, FrForwardBlocker::RouteAbsent { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn production_forwarder_binds_no_ingestion_route() {
        // MT-115: the shipped composition (`main.rs` -> `for_existing_fr`) binds NO ingestion path,
        // because MT-109 removed the one this module used to hardcode. A future reader must not be
        // able to conclude from this file that a survivor-ingestion route exists.
        let fwd = FrForwarder::for_existing_fr("http://127.0.0.1:37501");
        assert!(
            fwd.url().is_none(),
            "production must not name a Flight Recorder survivor-ingestion route"
        );
    }

    #[test]
    fn compatible_forwarder_without_a_route_reports_absent_never_success() {
        // Fail-closed: a Compatible verdict with no bound path is an honest RouteAbsent blocker, not a
        // silent Ok. The record stays local and pending.
        let fwd = FrForwarder::with_compat(
            "http://127.0.0.1:1",
            None,
            FrSchemaCompat::Compatible,
        );
        let err = fwd
            .forward(&freeze_record())
            .expect_err("an unbound route must block, never succeed");
        match err {
            FrForwardBlocker::RouteAbsent { reason } => assert!(
                reason.contains("no survivor ingestion route is bound"),
                "got {reason}"
            ),
            other => panic!("expected RouteAbsent, got {other:?}"),
        }
    }
}
