//! Argument / environment intake for the Palmistry watcher (MT-089, §6.13.2 + §6.13.3).
//!
//! Palmistry is a SPAWNED SIBLING of Handshake, so its configuration arrives at launch as a fixed
//! set of inputs (MT-094 is the Handshake-side spawn that supplies them). This module parses those
//! inputs and VALIDATES that ALL required ones are present — it REFUSES to run partially configured
//! (AC-009-2 / RISK-009-4): a half-started watcher that silently no-ops is worse than a clear refusal,
//! because the operator would believe a watcher is guarding the process when none is.
//!
//! # Inputs (env-first; CLI overrides)
//!
//! Env vars are the primary channel (the MT note: env is cleaner for a spawned sibling — no quoting /
//! arg-splitting issues). CLI flags are also accepted and OVERRIDE the corresponding env var so a
//! manual launch / a test can pin a value without mutating the process environment:
//!
//! | input          | env var                  | CLI flag           | meaning                                  |
//! |----------------|--------------------------|--------------------|------------------------------------------|
//! | parent PID     | `HANDSHAKE_PARENT_PID`   | `--parent-pid`     | OS pid of the Handshake process to watch |
//! | session id     | `HANDSHAKE_SESSION_ID`   | `--session-id`     | the diagnostic session id (ring naming)  |
//! | ring path      | `HANDSHAKE_RING_PATH`    | `--ring-path`      | absolute path to the MT-081 ring file    |
//! | control socket | `HANDSHAKE_CONTROL_SOCK` | `--control-socket` | local-socket name for the control channel|
//!
//! All four are REQUIRED. A missing or malformed value yields a typed [`CliError`] and a non-zero exit
//! (the binary entrypoint maps it to a clear stderr message + exit code 2).

use std::fmt;

/// The fully-validated configuration the watcher runs with. Constructed only by [`PalmistryConfig::parse`]
/// after every required input is present and well-formed — so once you hold one, the watcher is fully
/// configured (the "refuse a partial start" invariant is enforced at construction, not later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalmistryConfig {
    /// Optional private-seed bootstrap for the current owned child; never a native HMAC key.
    pub signing_process_instance: Option<uuid::Uuid>,
    pub signing_runtime_identity: Option<SigningRuntimeIdentity>,
    /// Explicit wire choice; ordinary invocations retain v1. This is not authenticated admission.
    pub ring_format: RingFormat,
    /// OS process id of the Handshake parent this watcher guards.
    pub parent_pid: u32,
    /// Diagnostic session id (used in logs + to correlate with the ring's session-scoped name).
    pub session_id: String,
    /// Absolute path to the MT-081 ring backing file (read passively for liveness in MT-090).
    pub ring_path: String,
    /// Local-socket name for the control channel (Shutdown/Ping/...).
    pub control_socket: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningRuntimeIdentity {
    pub runtime_instance_schema_id: String,
    pub runtime_instance_id: uuid::Uuid,
    pub runtime_host_scope_id: String,
    pub runtime_lease_protocol: String,
    pub runtime_lease_address: std::net::IpAddr,
    pub runtime_lease_port: u16,
    pub parent_os_creation_time_100ns: u64,
}

pub const RUNTIME_INSTANCE_SCHEMA_ID: &str = "hsk.embedded_runtime.instance@2";
pub const RUNTIME_LEASE_PROTOCOL: &str = "hsk.embedded_runtime.loopback_udp_exclusive@1";

impl SigningRuntimeIdentity {
    pub fn is_valid_for_session(&self, session: uuid::Uuid) -> bool {
        self.runtime_instance_schema_id == RUNTIME_INSTANCE_SCHEMA_ID
            && !session.is_nil()
            && self.runtime_instance_id == session
            && !self.runtime_host_scope_id.is_empty()
            && self.runtime_host_scope_id.trim() == self.runtime_host_scope_id
            && self.runtime_host_scope_id.len() <= 256
            && self.runtime_lease_protocol == RUNTIME_LEASE_PROTOCOL
            && self.runtime_lease_address.is_loopback()
            && self.runtime_lease_port != 0
            && self.parent_os_creation_time_100ns != 0
    }
}

/// Explicit wire choice. V2 always carries the caller's immutable expected nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingFormat {
    V1,
    V2 { expected_nonce: [u8; 16] },
}

pub const ENV_RING_VERSION: &str = "HANDSHAKE_RING_VERSION";
pub const ENV_RING_NONCE: &str = "HANDSHAKE_RING_NONCE";

/// Env var names (the primary input channel for a spawned sibling).
pub const ENV_PARENT_PID: &str = "HANDSHAKE_PARENT_PID";
pub const ENV_SESSION_ID: &str = "HANDSHAKE_SESSION_ID";
pub const ENV_RING_PATH: &str = "HANDSHAKE_RING_PATH";
pub const ENV_CONTROL_SOCK: &str = "HANDSHAKE_CONTROL_SOCK";

/// A typed reason a configuration could not be assembled. Carries the FIELD name so the refusal message
/// names exactly what was missing/invalid (AC-009-2: a clear error, not a silent half-start).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    BadSigningIdentity,
    BadRingVersion,
    BadRingNonce,
    NonceWithV1,
    /// A required input was absent from both the CLI args and the environment.
    Missing {
        field: &'static str,
        env: &'static str,
    },
    /// A required input was present but empty / whitespace-only.
    Empty {
        field: &'static str,
    },
    /// The parent PID was present but did not parse as a non-zero u32.
    BadPid {
        value: String,
    },
    /// A CLI flag was given without a following value.
    DanglingFlag {
        flag: String,
    },
    /// An unrecognized CLI argument was supplied.
    UnknownArg {
        arg: String,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::BadSigningIdentity => write!(
                f,
                "signing bootstrap requires v2, original session/process UUIDs and the complete canonical native runtime identity"
            ),
            CliError::BadRingVersion => write!(f, "ring version must be explicitly 1 or 2"),
            CliError::BadRingNonce => {
                write!(f, "v2 requires an exact nonzero 16-byte hexadecimal nonce")
            }
            CliError::NonceWithV1 => write!(f, "ring nonce is invalid with v1"),
            CliError::Missing { field, env } => write!(
                f,
                "missing required input '{field}' (set env {env} or pass the matching --flag); \
                 Palmistry refuses to run partially configured"
            ),
            CliError::Empty { field } => {
                write!(f, "required input '{field}' is present but empty")
            }
            CliError::BadPid { value } => {
                write!(f, "parent PID '{value}' is not a valid non-zero process id")
            }
            CliError::DanglingFlag { flag } => {
                write!(f, "CLI flag '{flag}' was given without a value")
            }
            CliError::UnknownArg { arg } => write!(f, "unrecognized argument '{arg}'"),
        }
    }
}

impl std::error::Error for CliError {}

/// Raw, possibly-partial inputs gathered from CLI flags before the environment fallback + validation.
#[derive(Default)]
struct RawInputs {
    signing_process_instance: Option<String>,
    runtime_instance_schema_id: Option<String>,
    runtime_instance_id: Option<String>,
    runtime_host_scope_id: Option<String>,
    runtime_lease_protocol: Option<String>,
    runtime_lease_address: Option<String>,
    runtime_lease_port: Option<String>,
    parent_os_creation_time_100ns: Option<String>,
    ring_version: Option<String>,
    ring_nonce: Option<String>,
    parent_pid: Option<String>,
    session_id: Option<String>,
    ring_path: Option<String>,
    control_socket: Option<String>,
}

impl PalmistryConfig {
    /// Parse from the real process inputs: `std::env::args()` (skipping argv[0]) for CLI flags and
    /// `std::env::var` for the env fallback. The convenience entrypoint used by `main`.
    pub fn parse_from_process() -> Result<Self, CliError> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::parse(&args, |key| std::env::var(key).ok())
    }

    /// Parse from an explicit arg list + an env lookup closure. Pure + dependency-injected so tests can
    /// drive every branch (missing/empty/bad-pid/unknown-flag) WITHOUT mutating the real process
    /// environment — which is essential because the test binary is multi-threaded and a global env
    /// mutation would race other tests (RISK: flaky env-dependent tests).
    pub fn parse(args: &[String], env: impl Fn(&str) -> Option<String>) -> Result<Self, CliError> {
        let mut raw = RawInputs::default();

        // --- CLI flags (override env) ---
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            let next = |i: usize| -> Result<String, CliError> {
                args.get(i + 1)
                    .cloned()
                    .ok_or_else(|| CliError::DanglingFlag { flag: arg.clone() })
            };
            match arg.as_str() {
                "--signing-process-instance" => {
                    raw.signing_process_instance = Some(next(i)?);
                    i += 2;
                }
                "--runtime-instance-schema-id" => {
                    raw.runtime_instance_schema_id = Some(next(i)?);
                    i += 2;
                }
                "--runtime-instance-id" => {
                    raw.runtime_instance_id = Some(next(i)?);
                    i += 2;
                }
                "--runtime-host-scope-id" => {
                    raw.runtime_host_scope_id = Some(next(i)?);
                    i += 2;
                }
                "--runtime-lease-protocol" => {
                    raw.runtime_lease_protocol = Some(next(i)?);
                    i += 2;
                }
                "--runtime-lease-address" => {
                    raw.runtime_lease_address = Some(next(i)?);
                    i += 2;
                }
                "--runtime-lease-port" => {
                    raw.runtime_lease_port = Some(next(i)?);
                    i += 2;
                }
                "--parent-os-creation-time-100ns" => {
                    raw.parent_os_creation_time_100ns = Some(next(i)?);
                    i += 2;
                }
                "--ring-version" => {
                    raw.ring_version = Some(next(i)?);
                    i += 2;
                }
                "--ring-nonce" => {
                    raw.ring_nonce = Some(next(i)?);
                    i += 2;
                }
                "--parent-pid" => {
                    raw.parent_pid = Some(next(i)?);
                    i += 2;
                }
                "--session-id" => {
                    raw.session_id = Some(next(i)?);
                    i += 2;
                }
                "--ring-path" => {
                    raw.ring_path = Some(next(i)?);
                    i += 2;
                }
                "--control-socket" => {
                    raw.control_socket = Some(next(i)?);
                    i += 2;
                }
                other => {
                    return Err(CliError::UnknownArg {
                        arg: other.to_string(),
                    })
                }
            }
        }

        // --- env fallback for any flag not supplied on the CLI ---
        let resolve = |cli: Option<String>, env_key: &'static str| -> Option<String> {
            cli.or_else(|| env(env_key))
        };
        let version = resolve(raw.ring_version, ENV_RING_VERSION).unwrap_or_else(|| "1".into());
        let nonce = resolve(raw.ring_nonce, ENV_RING_NONCE);
        let ring_format = match version.as_str() {
            "1" if nonce.is_none() => RingFormat::V1,
            "1" => return Err(CliError::NonceWithV1),
            "2" => {
                let text = nonce.ok_or(CliError::BadRingNonce)?;
                if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(CliError::BadRingNonce);
                }
                let mut expected_nonce = [0; 16];
                for (i, value) in expected_nonce.iter_mut().enumerate() {
                    *value = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                        .map_err(|_| CliError::BadRingNonce)?;
                }
                if expected_nonce == [0; 16] {
                    return Err(CliError::BadRingNonce);
                }
                RingFormat::V2 { expected_nonce }
            }
            _ => return Err(CliError::BadRingVersion),
        };
        let parent_pid_raw = resolve(raw.parent_pid, ENV_PARENT_PID);
        let session_id_raw = resolve(raw.session_id, ENV_SESSION_ID);
        let ring_path_raw = resolve(raw.ring_path, ENV_RING_PATH);
        let control_socket_raw = resolve(raw.control_socket, ENV_CONTROL_SOCK);

        // --- required-presence validation (refuse a partial start) ---
        let parent_pid_str = require(parent_pid_raw, "parent_pid", ENV_PARENT_PID)?;
        let session_id = require(session_id_raw, "session_id", ENV_SESSION_ID)?;
        let ring_path = require(ring_path_raw, "ring_path", ENV_RING_PATH)?;
        let control_socket = require(control_socket_raw, "control_socket", ENV_CONTROL_SOCK)?;

        // --- PID well-formedness: a non-zero u32 (pid 0 is never a real watched process) ---
        let parent_pid: u32 = parent_pid_str
            .parse()
            .ok()
            .filter(|&p| p != 0)
            .ok_or_else(|| CliError::BadPid {
                value: parent_pid_str.clone(),
            })?;

        let signing_process_instance = raw
            .signing_process_instance
            .map(|text| {
                let process =
                    uuid::Uuid::parse_str(&text).map_err(|_| CliError::BadSigningIdentity)?;
                let session =
                    uuid::Uuid::parse_str(&session_id).map_err(|_| CliError::BadSigningIdentity)?;
                if process.is_nil()
                    || session.is_nil()
                    || process == session
                    || !matches!(ring_format, RingFormat::V2 { .. })
                {
                    return Err(CliError::BadSigningIdentity);
                }
                Ok(process)
            })
            .transpose()?;
        let has_runtime_fields = raw.runtime_instance_schema_id.is_some()
            || raw.runtime_instance_id.is_some()
            || raw.runtime_host_scope_id.is_some()
            || raw.runtime_lease_protocol.is_some()
            || raw.runtime_lease_address.is_some()
            || raw.runtime_lease_port.is_some()
            || raw.parent_os_creation_time_100ns.is_some();
        let signing_runtime_identity = if signing_process_instance.is_some() {
            let identity = SigningRuntimeIdentity {
                runtime_instance_schema_id: raw
                    .runtime_instance_schema_id
                    .ok_or(CliError::BadSigningIdentity)?,
                runtime_instance_id: raw
                    .runtime_instance_id
                    .ok_or(CliError::BadSigningIdentity)?
                    .parse()
                    .map_err(|_| CliError::BadSigningIdentity)?,
                runtime_host_scope_id: raw
                    .runtime_host_scope_id
                    .ok_or(CliError::BadSigningIdentity)?,
                runtime_lease_protocol: raw
                    .runtime_lease_protocol
                    .ok_or(CliError::BadSigningIdentity)?,
                runtime_lease_address: raw
                    .runtime_lease_address
                    .ok_or(CliError::BadSigningIdentity)?
                    .parse()
                    .map_err(|_| CliError::BadSigningIdentity)?,
                runtime_lease_port: raw
                    .runtime_lease_port
                    .ok_or(CliError::BadSigningIdentity)?
                    .parse()
                    .map_err(|_| CliError::BadSigningIdentity)?,
                parent_os_creation_time_100ns: raw
                    .parent_os_creation_time_100ns
                    .ok_or(CliError::BadSigningIdentity)?
                    .parse()
                    .map_err(|_| CliError::BadSigningIdentity)?,
            };
            let session = session_id
                .parse()
                .map_err(|_| CliError::BadSigningIdentity)?;
            if !identity.is_valid_for_session(session) {
                return Err(CliError::BadSigningIdentity);
            }
            Some(identity)
        } else {
            if has_runtime_fields {
                return Err(CliError::BadSigningIdentity);
            }
            None
        };
        Ok(Self {
            signing_process_instance,
            signing_runtime_identity,
            ring_format,
            parent_pid,
            session_id,
            ring_path,
            control_socket,
        })
    }
}

/// Require a present, non-empty value or yield the precise typed error naming the field + its env var.
fn require(
    value: Option<String>,
    field: &'static str,
    env: &'static str,
) -> Result<String, CliError> {
    match value {
        None => Err(CliError::Missing { field, env }),
        Some(v) if v.trim().is_empty() => Err(CliError::Empty { field }),
        Some(v) => Ok(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An env lookup that always returns None (forces the CLI-only / missing paths).
    #[test]
    fn signed_mode_preserves_default_and_requires_original_v2_identity() {
        assert_eq!(
            PalmistryConfig::parse(&full_args(), |_| None)
                .unwrap()
                .signing_process_instance,
            None
        );
        let mut args = full_args();
        args.extend([
            "--signing-process-instance".into(),
            uuid::Uuid::from_u128(2).to_string(),
        ]);
        assert_eq!(
            PalmistryConfig::parse(&args, |_| None),
            Err(CliError::BadSigningIdentity)
        );
        args[3] = uuid::Uuid::from_u128(1).to_string();
        args.extend([
            "--ring-version".into(),
            "2".into(),
            "--ring-nonce".into(),
            "55555555555555555555555555555555".into(),
        ]);
        assert_eq!(
            PalmistryConfig::parse(&args, |_| None),
            Err(CliError::BadSigningIdentity)
        );
        args.extend([
            "--runtime-instance-schema-id".into(),
            RUNTIME_INSTANCE_SCHEMA_ID.into(),
            "--runtime-instance-id".into(),
            uuid::Uuid::from_u128(1).to_string(),
            "--runtime-host-scope-id".into(),
            "component-host".into(),
            "--runtime-lease-protocol".into(),
            RUNTIME_LEASE_PROTOCOL.into(),
            "--runtime-lease-address".into(),
            "127.0.0.1".into(),
            "--runtime-lease-port".into(),
            "12345".into(),
            "--parent-os-creation-time-100ns".into(),
            "42".into(),
        ]);
        assert_eq!(
            PalmistryConfig::parse(&args, |_| None)
                .unwrap()
                .signing_process_instance,
            Some(uuid::Uuid::from_u128(2))
        );
        for (flag, invalid) in [
            ("--runtime-instance-schema-id", "unknown"),
            (
                "--runtime-instance-id",
                "00000000-0000-0000-0000-000000000000",
            ),
            ("--runtime-host-scope-id", " "),
            ("--runtime-lease-protocol", "unknown"),
            ("--runtime-lease-address", "192.0.2.1"),
            ("--runtime-lease-port", "0"),
            ("--parent-os-creation-time-100ns", "0"),
        ] {
            let index = args.iter().position(|arg| arg == flag).unwrap();
            let mut invalid_args = args.clone();
            invalid_args[index + 1] = invalid.into();
            assert_eq!(
                PalmistryConfig::parse(&invalid_args, |_| None),
                Err(CliError::BadSigningIdentity)
            );
            let mut missing_args = args.clone();
            missing_args.drain(index..index + 2);
            assert_eq!(
                PalmistryConfig::parse(&missing_args, |_| None),
                Err(CliError::BadSigningIdentity)
            );
        }
        let mut unsigned_args = args.clone();
        let index = unsigned_args
            .iter()
            .position(|arg| arg == "--signing-process-instance")
            .unwrap();
        unsigned_args.drain(index..index + 2);
        assert_eq!(
            PalmistryConfig::parse(&unsigned_args, |_| None),
            Err(CliError::BadSigningIdentity)
        );
        let mut same_instance_args = args.clone();
        let process_index = same_instance_args
            .iter()
            .position(|arg| arg == "--signing-process-instance")
            .unwrap()
            + 1;
        same_instance_args[process_index] = uuid::Uuid::from_u128(1).to_string();
        assert_eq!(
            PalmistryConfig::parse(&same_instance_args, |_| None),
            Err(CliError::BadSigningIdentity)
        );
        let index = args
            .iter()
            .position(|arg| arg == "--signing-process-instance")
            .unwrap()
            + 1;
        args[index] = uuid::Uuid::nil().to_string();
        assert_eq!(
            PalmistryConfig::parse(&args, |_| None),
            Err(CliError::BadSigningIdentity)
        );
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn full_args() -> Vec<String> {
        vec![
            "--parent-pid".into(),
            "4242".into(),
            "--session-id".into(),
            "sess-abc".into(),
            "--ring-path".into(),
            "C:/tmp/handshake-diag-sess-abc.ring".into(),
            "--control-socket".into(),
            "handshake-palmistry-sess-abc".into(),
        ]
    }

    #[test]
    fn parses_full_cli() {
        let cfg = PalmistryConfig::parse(&full_args(), no_env).expect("full cli parses");
        assert_eq!(cfg.parent_pid, 4242);
        assert_eq!(cfg.session_id, "sess-abc");
        assert_eq!(cfg.ring_path, "C:/tmp/handshake-diag-sess-abc.ring");
        assert_eq!(cfg.control_socket, "handshake-palmistry-sess-abc");
    }

    #[test]
    fn env_fallback_fills_missing_flags() {
        // Only the PID on the CLI; the rest come from env.
        let args = vec!["--parent-pid".into(), "7".into()];
        let env = |k: &str| match k {
            ENV_SESSION_ID => Some("envsess".to_string()),
            ENV_RING_PATH => Some("/tmp/r.ring".to_string()),
            ENV_CONTROL_SOCK => Some("envsock".to_string()),
            _ => None,
        };
        let cfg = PalmistryConfig::parse(&args, env).expect("env fallback parses");
        assert_eq!(cfg.parent_pid, 7);
        assert_eq!(cfg.session_id, "envsess");
        assert_eq!(cfg.ring_path, "/tmp/r.ring");
        assert_eq!(cfg.control_socket, "envsock");
    }

    #[test]
    fn cli_overrides_env() {
        let args = vec!["--session-id".into(), "cli-wins".into()];
        let env = |k: &str| match k {
            ENV_PARENT_PID => Some("9".to_string()),
            ENV_SESSION_ID => Some("env-loses".to_string()),
            ENV_RING_PATH => Some("/tmp/r.ring".to_string()),
            ENV_CONTROL_SOCK => Some("sock".to_string()),
            _ => None,
        };
        let cfg = PalmistryConfig::parse(&args, env).expect("parses");
        assert_eq!(cfg.session_id, "cli-wins");
        assert_eq!(cfg.parent_pid, 9);
    }

    #[test]
    fn refuses_when_pid_missing() {
        let args = vec![
            "--session-id".into(),
            "s".into(),
            "--ring-path".into(),
            "/r".into(),
            "--control-socket".into(),
            "sock".into(),
        ];
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::Missing {
                field: "parent_pid",
                env: ENV_PARENT_PID
            }
        );
    }

    #[test]
    fn refuses_when_ring_path_missing() {
        let args = vec![
            "--parent-pid".into(),
            "1".into(),
            "--session-id".into(),
            "s".into(),
            "--control-socket".into(),
            "sock".into(),
        ];
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::Missing {
                field: "ring_path",
                env: ENV_RING_PATH
            }
        );
    }

    #[test]
    fn refuses_when_socket_missing() {
        let args = vec![
            "--parent-pid".into(),
            "1".into(),
            "--session-id".into(),
            "s".into(),
            "--ring-path".into(),
            "/r".into(),
        ];
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::Missing {
                field: "control_socket",
                env: ENV_CONTROL_SOCK
            }
        );
    }

    #[test]
    fn refuses_zero_pid() {
        let mut args = full_args();
        args[1] = "0".into();
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(err, CliError::BadPid { value: "0".into() });
    }

    #[test]
    fn refuses_nonnumeric_pid() {
        let mut args = full_args();
        args[1] = "notapid".into();
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::BadPid {
                value: "notapid".into()
            }
        );
    }

    #[test]
    fn refuses_empty_value() {
        let mut args = full_args();
        args[3] = "   ".into(); // whitespace session id
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::Empty {
                field: "session_id"
            }
        );
    }

    #[test]
    fn refuses_dangling_flag() {
        let args = vec!["--parent-pid".into()];
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::DanglingFlag {
                flag: "--parent-pid".into()
            }
        );
    }

    #[test]
    fn refuses_unknown_arg() {
        let args = vec!["--bogus".into()];
        let err = PalmistryConfig::parse(&args, no_env).unwrap_err();
        assert_eq!(
            err,
            CliError::UnknownArg {
                arg: "--bogus".into()
            }
        );
    }
}

#[cfg(test)]
mod wire_format_tests {
    use super::*;
    fn args(extra: &[&str]) -> Vec<String> {
        [
            "--parent-pid",
            "42",
            "--session-id",
            "original",
            "--ring-path",
            "/tmp/original.ring",
            "--control-socket",
            "original-control",
        ]
        .into_iter()
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
    }
    #[test]
    fn explicit_v2_nonce_is_required_without_downgrade() {
        assert_eq!(
            PalmistryConfig::parse(&args(&[]), |_| None)
                .unwrap()
                .ring_format,
            RingFormat::V1
        );
        let config = PalmistryConfig::parse(
            &args(&[
                "--ring-version",
                "2",
                "--ring-nonce",
                "55555555555555555555555555555555",
            ]),
            |_| None,
        )
        .unwrap();
        assert_eq!(
            config.ring_format,
            RingFormat::V2 {
                expected_nonce: [0x55; 16]
            }
        );
        for extra in [
            vec!["--ring-version", "3"],
            vec!["--ring-version", "2"],
            vec![
                "--ring-version",
                "2",
                "--ring-nonce",
                "00000000000000000000000000000000",
            ],
            vec![
                "--ring-version",
                "2",
                "--ring-nonce",
                "zz555555555555555555555555555555",
            ],
            vec!["--ring-version", "2", "--ring-nonce", "55"],
            vec!["--ring-nonce", "55555555555555555555555555555555"],
        ] {
            assert!(PalmistryConfig::parse(&args(&extra), |_| None).is_err());
        }
        let config = PalmistryConfig::parse(
            &args(&[
                "--ring-version",
                "2",
                "--ring-nonce",
                "55555555555555555555555555555555",
            ]),
            |key| match key {
                ENV_RING_VERSION => Some("9".into()),
                ENV_RING_NONCE => Some("bad".into()),
                _ => None,
            },
        )
        .unwrap();
        assert_eq!(
            config.ring_format,
            RingFormat::V2 {
                expected_nonce: [0x55; 16]
            }
        );
    }
}
