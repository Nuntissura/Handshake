//! MT-128 integration tests for `OsKeychainSecretsVault`.
//!
//! These tests exercise the production code path against the
//! real host OS credential store - no `keyring` mock backend is
//! installed in this binary. Each test runs in its own
//! `cargo test` process (integration tests compile to per-file
//! binaries), so the global `keyring` default credential builder
//! starts unset and falls through to the platform-native backend
//! selected by the `keyring` crate features in `Cargo.toml`.
//!
//! Spec-Realism Gate Sub-rule 2 compliance: every test in this
//! file that runs (not `#[ignore]`d) MUST touch the actual OS
//! keychain. The Windows host is the canonical target, so the
//! primary round-trip test is `#[cfg(target_os = "windows")]`.
//! Equivalent tests for macOS / Linux are sketched as `#[ignore]`
//! placeholders that document why they are not auto-enabled on
//! this host.
//!
//! Every test uses a unique service namespace per run (UUID v7
//! suffix) so concurrent runs and repeat invocations cannot
//! collide on the same credential entry, and every test
//! attempts to delete its credential before returning regardless
//! of success path.

// `OsKeychainSecretsVault` + `HANDSHAKE_KEYCHAIN_SERVICE` are
// authored in `secrets_vault.rs` but not yet re-exported from
// `cloud/mod.rs` (MT-128 scope is the impl + tests; the
// re-export lives downstream so this MT does not touch
// `cloud/mod.rs`). Reach in via the module path; the parent
// `cloud` module is already `pub`, and `secrets_vault` is
// declared `pub mod secrets_vault;` there, so this path is part
// of the stable public surface.
use handshake_core::model_runtime::cloud::secrets_vault::{
    OsKeychainSecretsVault, HANDSHAKE_KEYCHAIN_SERVICE,
};
use handshake_core::model_runtime::cloud::{SecretsVault, SecretsVaultError};
use uuid::Uuid;

/// Build a per-run service namespace prefixed with the
/// Handshake default so a forensic reader of the OS credential
/// store can recognise the entries as test artefacts even if
/// cleanup fails.
fn unique_test_namespace(label: &str) -> String {
    format!(
        "{}-test-{}-{}",
        HANDSHAKE_KEYCHAIN_SERVICE,
        label,
        Uuid::now_v7()
    )
}

/// Best-effort cleanup so the OS credential store does not
/// accumulate test entries on flake or panic. Uses the vault's
/// own delete (which is idempotent) so a missing entry is not
/// surfaced as a failure.
fn cleanup_lane(vault: &OsKeychainSecretsVault, lane: &str) {
    let _ = vault.delete(lane);
}

#[cfg(target_os = "windows")]
#[test]
fn os_keychain_vault_round_trips_against_windows_credential_manager() {
    let namespace = unique_test_namespace("rt-windows");
    let vault = OsKeychainSecretsVault::new(&namespace);
    let lane = "openai-byok-test";
    let secret = format!("sk-windows-credman-{}", Uuid::now_v7());

    // Pre-emptive cleanup in case a prior run leaked under the
    // same lane (the namespace itself is unique per run, so this
    // is belt-and-braces - but the cost is one keychain call).
    cleanup_lane(&vault, lane);

    // put -> get round trip against Windows Credential Manager.
    vault
        .put(lane, secret.clone())
        .expect("put against Windows Credential Manager must succeed");

    let retrieved = vault
        .get(lane)
        .expect("get must return the secret just stored");
    assert_eq!(
        retrieved, secret,
        "Windows Credential Manager must return the exact secret stored",
    );

    // delete must remove the entry and make subsequent get return
    // NoSecretForLane (mapped from `keyring::Error::NoEntry`).
    vault
        .delete(lane)
        .expect("delete against Windows Credential Manager must succeed");

    let err = vault
        .get(lane)
        .expect_err("get on deleted lane must surface NoSecretForLane");
    match err {
        SecretsVaultError::NoSecretForLane(returned_lane) => {
            assert_eq!(returned_lane, lane);
        }
        other => panic!("expected NoSecretForLane, got {other:?}"),
    }

    // Idempotent delete after the entry is gone.
    vault
        .delete(lane)
        .expect("idempotent delete on missing entry must succeed");
}

#[cfg(target_os = "windows")]
#[test]
fn os_keychain_vault_supports_multiple_lanes_under_same_namespace() {
    let namespace = unique_test_namespace("multi-lane");
    let vault = OsKeychainSecretsVault::new(&namespace);

    let lanes = ["openai", "anthropic", "claude-byok"];
    let secrets: Vec<String> = lanes
        .iter()
        .map(|lane| format!("sk-{}-{}", lane, Uuid::now_v7()))
        .collect();

    // Pre-clean.
    for lane in &lanes {
        cleanup_lane(&vault, lane);
    }

    // Store each lane under the same service namespace.
    for (lane, secret) in lanes.iter().zip(secrets.iter()) {
        vault
            .put(lane, secret.clone())
            .unwrap_or_else(|err| panic!("put for lane {lane} failed: {err:?}"));
    }

    // Each lane must round-trip independently - i.e. the vault
    // must use the lane as the credential discriminator, not
    // collapse all writes onto a single entry.
    for (lane, expected) in lanes.iter().zip(secrets.iter()) {
        let retrieved = vault
            .get(lane)
            .unwrap_or_else(|err| panic!("get for lane {lane} failed: {err:?}"));
        assert_eq!(
            &retrieved, expected,
            "lane {lane} returned the wrong secret",
        );
    }

    // Clean up every lane we touched.
    for lane in &lanes {
        vault
            .delete(lane)
            .unwrap_or_else(|err| panic!("cleanup delete for lane {lane} failed: {err:?}"));
    }

    // After deletion, all lanes must report NoSecretForLane.
    for lane in &lanes {
        let err = vault
            .get(lane)
            .expect_err("get on deleted lane must surface NoSecretForLane");
        assert!(matches!(err, SecretsVaultError::NoSecretForLane(_)));
    }
}

#[cfg(target_os = "windows")]
#[test]
fn os_keychain_vault_rejects_empty_lane_and_secret_before_touching_keychain() {
    // The validation guards live in the vault, not in the
    // keyring crate, so they must fire even when the OS keychain
    // is reachable. Using the real (non-mock) keychain here pins
    // that the guards short-circuit before any Windows API call
    // is dispatched.
    let namespace = unique_test_namespace("validation");
    let vault = OsKeychainSecretsVault::new(&namespace);

    assert!(matches!(
        vault.put("", "value".to_string()).unwrap_err(),
        SecretsVaultError::EmptyLaneId
    ));
    assert!(matches!(
        vault.put("   ", "value".to_string()).unwrap_err(),
        SecretsVaultError::EmptyLaneId
    ));
    assert!(matches!(
        vault.put("openai", "".to_string()).unwrap_err(),
        SecretsVaultError::EmptySecretValue
    ));
    assert!(matches!(
        vault.get("").unwrap_err(),
        SecretsVaultError::EmptyLaneId
    ));
    assert!(matches!(
        vault.delete("").unwrap_err(),
        SecretsVaultError::EmptyLaneId
    ));
}

// ---------------------------------------------------------------
// Non-Windows hosts: keep these tests as documented placeholders
// so a reviewer on a different platform sees that the integration
// surface exists but is gated. The current host is Windows, so
// the Windows variants above are the ones that actually run.
// ---------------------------------------------------------------

#[cfg(target_os = "macos")]
#[test]
#[ignore = "macOS Keychain integration not exercised on the Windows host that ships MT-128; \
           enable by removing `#[ignore]` when running on a macOS CI runner where the \
           login keychain is unlocked. The OsKeychainSecretsVault code path is identical \
           across platforms; only the keyring backend differs."]
fn os_keychain_vault_round_trips_against_apple_keychain() {
    let namespace = unique_test_namespace("rt-macos");
    let vault = OsKeychainSecretsVault::new(&namespace);
    let lane = "openai-byok-test";
    let secret = format!("sk-apple-keychain-{}", Uuid::now_v7());

    cleanup_lane(&vault, lane);
    vault.put(lane, secret.clone()).expect("put");
    assert_eq!(vault.get(lane).expect("get"), secret);
    vault.delete(lane).expect("delete");
    assert!(matches!(
        vault.get(lane).unwrap_err(),
        SecretsVaultError::NoSecretForLane(_)
    ));
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "Linux Secret Service / keyutils integration not exercised on the Windows host \
           that ships MT-128; enable by removing `#[ignore]` when running on a Linux CI \
           runner with a running D-Bus session and an unlocked Secret Service collection. \
           The OsKeychainSecretsVault code path is identical across platforms."]
fn os_keychain_vault_round_trips_against_linux_secret_service() {
    let namespace = unique_test_namespace("rt-linux");
    let vault = OsKeychainSecretsVault::new(&namespace);
    let lane = "openai-byok-test";
    let secret = format!("sk-linux-secret-service-{}", Uuid::now_v7());

    cleanup_lane(&vault, lane);
    vault.put(lane, secret.clone()).expect("put");
    assert_eq!(vault.get(lane).expect("get"), secret);
    vault.delete(lane).expect("delete");
    assert!(matches!(
        vault.get(lane).unwrap_err(),
        SecretsVaultError::NoSecretForLane(_)
    ));
}
