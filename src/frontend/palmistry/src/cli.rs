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
    /// OS process id of the Handshake parent this watcher guards.
    pub parent_pid: u32,
    /// Diagnostic session id (used in logs + to correlate with the ring's session-scoped name).
    pub session_id: String,
    /// Absolute path to the MT-081 ring backing file (read passively for liveness in MT-090).
    pub ring_path: String,
    /// Local-socket name for the control channel (Shutdown/Ping/...).
    pub control_socket: String,
}

/// Env var names (the primary input channel for a spawned sibling).
pub const ENV_PARENT_PID: &str = "HANDSHAKE_PARENT_PID";
pub const ENV_SESSION_ID: &str = "HANDSHAKE_SESSION_ID";
pub const ENV_RING_PATH: &str = "HANDSHAKE_RING_PATH";
pub const ENV_CONTROL_SOCK: &str = "HANDSHAKE_CONTROL_SOCK";

/// A typed reason a configuration could not be assembled. Carries the FIELD name so the refusal message
/// names exactly what was missing/invalid (AC-009-2: a clear error, not a silent half-start).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// A required input was absent from both the CLI args and the environment.
    Missing {
        field: &'static str,
        env: &'static str,
    },
    /// A required input was present but empty / whitespace-only.
    Empty { field: &'static str },
    /// The parent PID was present but did not parse as a non-zero u32.
    BadPid { value: String },
    /// A CLI flag was given without a following value.
    DanglingFlag { flag: String },
    /// An unrecognized CLI argument was supplied.
    UnknownArg { arg: String },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

        Ok(Self {
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
