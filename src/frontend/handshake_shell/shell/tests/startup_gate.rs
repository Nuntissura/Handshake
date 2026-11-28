//! The shell program's pre-account startup gate and its typed endings (MT-214 S5, AC214-10, AC214-11). No device, no
//! window, no desktop and no thread: everything here is decided by plain functions of numbers and types, so it runs
//! on any machine and cannot touch the Operator's desktop.

use std::cell::Cell;
use std::collections::BTreeSet;

use handshake_shell::{
    conclude, may_acknowledge, resolve_product, run_product, shutdown_settled, DeviceAuthority, ExitKind,
    NoWatcher, ShutdownWatcher, StartupRefusal, TestBinary, WatcherAck, CAPABILITY, MISSING,
};

#[test]
fn the_product_without_the_issuer_refuses_and_opens_nothing() {
    let opened = Cell::new(0_u32);
    let started = run_product(|_authority| {
        opened.set(opened.get() + 1);
        7
    });
    assert_eq!(started, Err(StartupRefusal::NoDeviceAuthority));
    assert_eq!(opened.get(), 0, "the device is opened only after the gate passed");
    assert!(resolve_product().is_err(), "no issuer exists, so no authority is issued");
}

#[test]
fn the_refusal_is_typed_and_names_exactly_what_is_missing() {
    let refusal = StartupRefusal::NoDeviceAuthority;
    assert_eq!(refusal.code(), "startup.no_device_authority");
    let line = refusal.line();
    assert!(line.starts_with("HSK_SHELL_REFUSED code=startup.no_device_authority"), "{line}");
    assert!(line.contains(&format!("capability={CAPABILITY}")), "{line}");
    assert_eq!(CAPABILITY, "device.gpu.shell");
    assert_eq!(MISSING.len(), 2);
    assert!(line.contains("MT-219") && line.contains("NATIVE-RENDER-BOOT-007"), "{line}");
    assert_eq!(ExitKind::NoDeviceAuthority.code(), 7);
}

#[test]
fn a_test_binary_says_in_its_receipt_that_it_is_not_the_product() {
    let authority = DeviceAuthority::Test(TestBinary::declare("a_test_binary"));
    assert!(!authority.is_product());
    let receipt = authority.receipt();
    assert!(receipt.starts_with("HSK_SHELL_RECEIPT "), "{receipt}");
    assert!(receipt.contains("product=false"), "{receipt}");
    assert!(receipt.contains("device_authority=none"), "{receipt}");
    assert!(receipt.contains("name=a_test_binary"), "{receipt}");
    assert!(receipt.contains(&format!("capability_required={CAPABILITY}")), "{receipt}");
    assert!(!receipt.contains("product=true"), "{receipt}");
}

#[test]
fn every_ending_has_its_own_exit_code_and_name() {
    let codes: BTreeSet<u8> = ExitKind::ALL.iter().map(|kind| kind.code()).collect();
    let names: BTreeSet<&str> = ExitKind::ALL.iter().map(|kind| kind.name()).collect();
    assert_eq!(codes.len(), ExitKind::ALL.len(), "two endings share an exit code");
    assert_eq!(names.len(), ExitKind::ALL.len(), "two endings share a name");
    assert_eq!(codes, BTreeSet::from([0, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
    assert_eq!(ExitKind::Clean.code(), 0, "only a clean ending exits with 0");
    assert!(ExitKind::ALL.iter().filter(|kind| kind.code() == 0).count() == 1);
    // The codes the examples of the surface crate already print keep their numbers.
    assert_eq!(ExitKind::Usage.code(), 2);
    assert_eq!(ExitKind::DeviceUnavailable.code(), 4);
    assert_eq!(ExitKind::NoWindow.code(), 5);
    assert_eq!(ExitKind::LoopFailed.code(), 6);
}

#[test]
fn a_shutdown_is_settled_only_when_everything_is() {
    assert!(shutdown_settled(Some(true), 0, 3, 3));
    assert!(!shutdown_settled(Some(false), 0, 3, 3), "the owner still held something");
    assert!(!shutdown_settled(None, 0, 3, 3), "the owner reported nothing");
    assert!(!shutdown_settled(Some(true), 1, 3, 3), "a window's claim was let go unsettled");
    assert!(!shutdown_settled(Some(true), 0, 3, 2), "a window is still there");
    assert!(!shutdown_settled(Some(true), 0, 2, 3), "more destroyed than created: something else interfered");
}

#[test]
fn only_a_settled_shutdown_is_acknowledged_and_an_unsettled_one_is_never_reported_clean() {
    assert!(may_acknowledge(true, None));
    assert!(!may_acknowledge(false, None));
    assert_eq!(conclude(false, None, None), ExitKind::ShutdownUnsettled);
    assert_eq!(
        conclude(false, None, Some(WatcherAck::Acknowledged)),
        ExitKind::ShutdownUnsettled,
        "an acknowledged shutdown that was not settled is still not clean"
    );
    assert_eq!(
        conclude(false, Some(ExitKind::NoWindow), None),
        ExitKind::ShutdownUnsettled,
        "work still held outranks the failure that ended the run"
    );
    assert_eq!(conclude(true, None, Some(WatcherAck::Acknowledged)), ExitKind::Clean);
    assert_eq!(conclude(true, None, Some(WatcherAck::NotConnected)), ExitKind::Clean);
    assert_eq!(conclude(true, None, None), ExitKind::Clean);
    // The realistic pair: a failed run is never asked, so it has no answer, and ends with its own failure.
    assert_eq!(
        conclude(true, Some(ExitKind::NoWindow), None),
        ExitKind::NoWindow,
        "a settled shutdown after a refused window ends with the refusal"
    );
    // Defensive ordering of the pure function: `ended` can no longer produce an answer beside a failure (the watcher is
    // not asked after one), but if one came the run's own failure would still be named before the watcher's silence.
    assert_eq!(
        conclude(true, Some(ExitKind::NoWindow), Some(WatcherAck::NotAcknowledged)),
        ExitKind::NoWindow,
        "the run's own failure is named before the watcher's silence"
    );
    assert_eq!(
        conclude(true, None, Some(WatcherAck::NotAcknowledged)),
        ExitKind::WatcherNotAcknowledged
    );
}

#[test]
fn without_a_watcher_nothing_is_acknowledged_and_it_says_so() {
    assert_eq!(NoWatcher.acknowledge_shutdown(), WatcherAck::NotConnected);
    let names: BTreeSet<&str> = [
        WatcherAck::Acknowledged,
        WatcherAck::NotConnected,
        WatcherAck::NotAcknowledged,
    ]
    .iter()
    .map(|ack| ack.name())
    .collect();
    assert_eq!(names.len(), 3);
}

/// MT-214 E9-F1 Q6: a run that ended with a typed failure (a refused window, a failed loop, a refused heartbeat) leaves the
/// watcher untold whatever its shutdown reported; only a settled shutdown with no failure is told.
#[test]
fn may_acknowledge_needs_a_settled_shutdown_and_no_failure() {
    assert!(may_acknowledge(true, None));
    assert!(!may_acknowledge(false, None));
    for kind in ExitKind::ALL.into_iter().filter(|kind| *kind != ExitKind::Clean) {
        assert!(!may_acknowledge(true, Some(kind)), "{kind:?}: a settled shutdown after a failure is not told");
        assert!(!may_acknowledge(false, Some(kind)), "{kind:?}: an unsettled one is never told");
    }
    // Every ending the program can fail with is covered by the table above.
    assert_eq!(ExitKind::ALL.len(), 10);
}
