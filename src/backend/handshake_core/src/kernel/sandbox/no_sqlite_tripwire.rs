//! MT-015 No-SQLite authority tripwire.
//!
//! Acceptance (MT-015.json): "Kernel003 authority fails closed without
//! Postgres/EventLedger authority." The tripwire is a deterministic guard
//! every KB003 write must call before persisting a `SandboxRunV1`,
//! `SandboxPolicyV1`, `ValidationRunV1`, `PromotionDecisionV1`, or
//! `PromotionReceiptV1`. If the active control-plane mode is anything but
//! `PostgresPrimary`, the guard returns an error and the caller MUST refuse
//! to write.
//!
//! Why a separate module rather than re-using `assert_kernel_authority_storage_mode`:
//!
//! - This guard is KB003-specific and surfaces the policy ID
//!   `KB003_NO_SQLITE_AUTHORITY_V1` so denial receipts and DCC projections can
//!   cite it without grepping the broader kernel module.
//! - The standalone module gives MT-015 a focused unit-test surface.

use thiserror::Error;

pub const KB003_NO_SQLITE_AUTHORITY_POLICY_ID: &str = "KB003_NO_SQLITE_AUTHORITY_V1";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NoSqliteTripwireError {
    #[error(
        "KB003 authority write refused: control-plane mode `{mode}` is not PostgresPrimary (policy {policy})"
    )]
    NonPostgresAuthority {
        mode: String,
        policy: &'static str,
    },
}

/// Modes the tripwire recognises. Mirrors `ControlPlaneStorageMode` so this
/// module can be tested without pulling the full storage stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityMode {
    PostgresPrimary,
    SqliteCache,
    SqliteOffline,
    Test,
}

impl AuthorityMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PostgresPrimary => "postgres_primary",
            Self::SqliteCache => "sqlite_cache",
            Self::SqliteOffline => "sqlite_offline",
            Self::Test => "test",
        }
    }
    pub fn is_authority(&self) -> bool {
        matches!(self, Self::PostgresPrimary)
    }
}

/// Returns `Ok(())` only if `mode` is `PostgresPrimary`. Every KB003 authority
/// write site must call this before persisting.
pub fn guard_authority_write(mode: AuthorityMode) -> Result<(), NoSqliteTripwireError> {
    if mode.is_authority() {
        Ok(())
    } else {
        Err(NoSqliteTripwireError::NonPostgresAuthority {
            mode: mode.as_str().to_string(),
            policy: KB003_NO_SQLITE_AUTHORITY_POLICY_ID,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_primary_allows_write() {
        assert!(guard_authority_write(AuthorityMode::PostgresPrimary).is_ok());
    }

    #[test]
    fn sqlite_modes_fail_closed() {
        for mode_under_test in [
            AuthorityMode::SqliteCache,
            AuthorityMode::SqliteOffline,
            AuthorityMode::Test,
        ] {
            let err = guard_authority_write(mode_under_test).expect_err("KB003 must refuse non-Postgres authority");
            match err {
                NoSqliteTripwireError::NonPostgresAuthority { mode: m, policy } => {
                    assert_eq!(policy, KB003_NO_SQLITE_AUTHORITY_POLICY_ID);
                    assert_eq!(m, mode_str_round_trip(mode_under_test));
                }
            }
        }
    }

    fn mode_str_round_trip(m: AuthorityMode) -> String {
        m.as_str().to_string()
    }
}
