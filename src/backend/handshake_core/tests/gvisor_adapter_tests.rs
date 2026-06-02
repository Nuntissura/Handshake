//! Real (non-mock) integration tests for the Tier-2 gVisor (`runsc`)
//! syscall-isolation sandbox adapter.
//!
//! These tests actually start a gVisor sandbox inside WSL2 via `runsc do` and
//! run a command inside it. On a host without WSL2 + a runsc that can start a
//! sandbox, `GvisorAdapter::try_new` returns `AdapterUnavailable`; in that case
//! the test prints a clear skip message and returns so non-WSL CI does not
//! fail. On a host where the adapter IS available it MUST exercise a real
//! sandboxed exec.

use std::collections::BTreeMap;

use handshake_core::sandbox::{
    AdapterId, Command, GvisorAdapter, GvisorConfig, ImageRef, IsolationTier, NetPolicy,
    ProcessSpec, ProcessStatus, ResourceLimits, SandboxAdapter, SandboxAdapterError, TrustClass,
    GVISOR_ADAPTER_ID,
};

fn skip_message(error: &SandboxAdapterError) -> String {
    format!(
        "SKIP gvisor adapter test: runtime unavailable on this host ({error}). \
         This is expected on hosts without WSL2 or where runsc cannot start a sandbox."
    )
}

fn sample_spec() -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("gvisor-test-spec"),
        image_or_root: ImageRef::new("runsc-do"),
        cmd: vec!["true".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        idle_timeout_ms: None,
        required_capabilities: Default::default(),
        trust_class: TrustClass::UntrustedAgent,
        metadata: BTreeMap::new(),
    }
}

/// Real gVisor sandbox: spawn a handle, exec a command inside a freshly started
/// `runsc do` sandbox, and assert on the captured guest stdout + exit code.
/// Also asserts the Tier-2 capability shape.
#[tokio::test]
async fn gvisor_runs_real_sandbox_and_captures_stdout() {
    let adapter = match GvisorAdapter::try_new(GvisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("{}", skip_message(&error));
            return;
        }
    };

    // Tier-2 capability shape is asserted only when the adapter is available
    // (it is only constructed when the runtime is available).
    let caps = adapter.capabilities();
    assert_eq!(caps.adapter_id, AdapterId::new(GVISOR_ADAPTER_ID));
    assert_eq!(caps.isolation_tier, IsolationTier::Tier2Syscall);
    assert!(
        !caps.requires_nested_virt,
        "Tier-2 gVisor must not require nested virtualization"
    );
    assert!(caps.runtime_available);

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn gvisor handle");

    let cmd = Command {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo gvisor-tier2-ok; uname -s".to_string(),
        ],
        env_overlay: BTreeMap::new(),
        stdin: None,
        timeout_ms: None,
    };

    let result = adapter
        .exec(&handle, cmd)
        .await
        .expect("exec inside real gvisor sandbox");

    let stdout = String::from_utf8_lossy(&result.stdout);
    eprintln!("--- REAL GVISOR SANDBOX STDOUT BEGIN ---");
    eprintln!("{stdout}");
    eprintln!(
        "--- REAL GVISOR SANDBOX STDOUT END (exit_code={}, {} ms) ---",
        result.exit_code, result.duration_ms
    );

    assert!(
        stdout.contains("gvisor-tier2-ok"),
        "captured stdout must contain the echoed marker; got: {stdout:?}"
    );
    assert!(
        stdout.contains("Linux"),
        "captured stdout must contain `Linux` from `uname -s` (gVisor sentry reports Linux); got: {stdout:?}"
    );
    assert_eq!(result.exit_code, 0, "successful command must report exit 0");

    // The ephemeral model reports Exited after a completed exec.
    match adapter.status(&handle).await.expect("status after exec") {
        ProcessStatus::Exited { code } => assert_eq!(code, 0),
        other => panic!("expected Exited after completed exec, got {other:?}"),
    }
    assert_eq!(
        adapter.exit_code(&handle).await.expect("exit_code after exec"),
        Some(0)
    );
}

/// Negative path: a command that exits non-zero must surface the real guest
/// exit code (not 0, not a host-side error).
#[tokio::test]
async fn gvisor_propagates_nonzero_exit_code() {
    let adapter = match GvisorAdapter::try_new(GvisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("{}", skip_message(&error));
            return;
        }
    };

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn gvisor handle");

    let cmd = Command {
        argv: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo before-failure; exit 7".to_string(),
        ],
        env_overlay: BTreeMap::new(),
        stdin: None,
        timeout_ms: None,
    };

    let result = adapter
        .exec(&handle, cmd)
        .await
        .expect("exec non-zero command inside real gvisor sandbox");

    let stdout = String::from_utf8_lossy(&result.stdout);
    eprintln!(
        "--- REAL GVISOR SANDBOX (nonzero) STDOUT ---\n{stdout}\n--- exit_code={} ---",
        result.exit_code
    );

    assert!(stdout.contains("before-failure"));
    assert_eq!(
        result.exit_code, 7,
        "non-zero guest exit code must be propagated verbatim"
    );
}

/// `fs_bind` is not yet supported by the `runsc do` exec model and must fail
/// closed with a typed error rather than silently dropping or unsafely
/// exposing the host filesystem.
#[tokio::test]
async fn gvisor_fs_bind_is_unsupported_for_now() {
    let adapter = match GvisorAdapter::try_new(GvisorConfig::default()).await {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("{}", skip_message(&error));
            return;
        }
    };

    let handle = adapter
        .spawn(sample_spec())
        .await
        .expect("spawn gvisor handle");

    let err = adapter
        .fs_bind(
            &handle,
            std::path::PathBuf::from("D:/host"),
            std::path::PathBuf::from("/work"),
            handshake_core::sandbox::BindMode::ReadOnly,
        )
        .await
        .expect_err("fs_bind must fail closed while unsupported");
    assert!(
        matches!(err, SandboxAdapterError::BindGuestPathInvalid { .. }),
        "fs_bind must return a typed error (not fake success), got {err:?}"
    );
}
