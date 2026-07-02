//! MT-106 child-stall integration proofs over the public Palmistry APIs.

use std::fs;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use palmistry::child_registry::{ChildProcessProbe, ChildRegistry, read_file_counter};
use palmistry::child_stall::{
    ChildProcessState, ChildProgress, ChildStallDetector, ChildStallReasonCode, ChildStallState,
};

struct FakeProbe(Arc<AtomicU8>);

impl ChildProcessProbe for FakeProbe {
    fn state(&self) -> ChildProcessState {
        match self.0.load(Ordering::SeqCst) {
            1 => ChildProcessState::Alive,
            2 => ChildProcessState::Exited,
            _ => ChildProcessState::Unknown,
        }
    }
}

struct FileGuard(PathBuf);

impl Drop for FileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn temp_file(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hsk-mt106-child-stall-{label}-{}-{nanos}.txt",
        std::process::id()
    ))
}

fn registry_with_state(state: Arc<AtomicU8>) -> ChildRegistry {
    ChildRegistry::new(Arc::new(move |_| Box::new(FakeProbe(Arc::clone(&state)))))
}

fn progress(counter: u64) -> ChildProgress {
    ChildProgress {
        counter,
        timestamp_nanos: counter * 1_000,
    }
}

#[test]
fn alive_stale_child_emits_one_typed_child_stall_report() {
    let state = Arc::new(AtomicU8::new(1));
    let registry = registry_with_state(state);
    let path = temp_file("alive-stale");
    let _guard = FileGuard(path.clone());
    fs::write(&path, "1\n").expect("write liveness baseline");

    let start = Instant::now();
    registry
        .register_file_child(55, 106_001, &path, Duration::from_millis(100), start)
        .expect("register child");

    let first = registry.poll(start + Duration::from_millis(150));
    assert_eq!(first.len(), 1, "alive + stale progress emits one report");
    assert_eq!(first[0].child_pid, 55);
    assert_eq!(first[0].child_session_id, 106_001);
    assert_eq!(
        first[0].reason_code,
        ChildStallReasonCode::ProgressStaleWhileAlive
    );
    assert_eq!(first[0].last_progress_counter, 1);

    assert!(
        registry.poll(start + Duration::from_millis(250)).is_empty(),
        "same stale edge is debounced"
    );
}

#[test]
fn progressing_child_never_false_positives_past_threshold() {
    let state = Arc::new(AtomicU8::new(1));
    let registry = registry_with_state(state);
    let path = temp_file("progressing");
    let _guard = FileGuard(path.clone());
    fs::write(&path, "1\n").expect("write liveness baseline");

    let start = Instant::now();
    registry
        .register_file_child(56, 106_002, &path, Duration::from_millis(100), start)
        .expect("register child");

    for i in 2..10 {
        fs::write(&path, format!("{i}\n")).expect("advance progress");
        let reports = registry.poll(start + Duration::from_millis(i * 80));
        assert!(
            reports.is_empty(),
            "advancing child must not report a stall at counter {i}: {reports:?}"
        );
    }
}

#[test]
fn clean_exit_is_not_a_child_stall() {
    let state = Arc::new(AtomicU8::new(2));
    let registry = registry_with_state(state);
    let path = temp_file("exited");
    let _guard = FileGuard(path.clone());
    fs::write(&path, "1\n").expect("write liveness baseline");

    let start = Instant::now();
    registry
        .register_file_child(57, 106_003, &path, Duration::from_millis(100), start)
        .expect("register child");

    assert!(
        registry.poll(start + Duration::from_millis(500)).is_empty(),
        "cleanly exited child is removed, not recorded as stalled"
    );
}

#[test]
fn recovery_clears_stall_and_allows_a_future_edge() {
    let start = Instant::now();
    let mut detector = ChildStallDetector::new(58, 106_004, Duration::from_millis(100));
    detector.poll(start, ChildProcessState::Alive, Some(progress(1)));

    let first = detector.poll(
        start + Duration::from_millis(150),
        ChildProcessState::Alive,
        Some(progress(1)),
    );
    assert!(first.report.is_some(), "first stale edge reports");

    let recovered = detector.poll(
        start + Duration::from_millis(175),
        ChildProcessState::Alive,
        Some(progress(2)),
    );
    assert_eq!(recovered.state, ChildStallState::Healthy);
    assert!(recovered.report.is_none());

    let second = detector.poll(
        start + Duration::from_millis(300),
        ChildProcessState::Alive,
        Some(progress(2)),
    );
    assert!(
        second.report.is_some(),
        "a fresh no-progress interval after recovery reports again"
    );
}

#[test]
fn missing_or_malformed_liveness_is_suspected_only() {
    let state = Arc::new(AtomicU8::new(1));
    let registry = registry_with_state(state);
    let malformed = temp_file("malformed");
    let _guard = FileGuard(malformed.clone());
    fs::write(&malformed, "not-a-counter").expect("write malformed source");
    assert!(
        read_file_counter(&malformed).is_none(),
        "malformed liveness source never creates a progress baseline"
    );

    let start = Instant::now();
    registry
        .register_file_child(59, 106_005, &malformed, Duration::from_millis(100), start)
        .expect("register child");
    assert!(
        registry.poll(start + Duration::from_millis(500)).is_empty(),
        "without a valid baseline, stale/malformed liveness is not a confirmed stall"
    );

    fs::write(&malformed, "1\n").expect("establish baseline");
    assert!(registry.poll(start + Duration::from_millis(550)).is_empty());
    fs::remove_file(&malformed).expect("remove liveness source after baseline");
    assert!(
        registry.poll(start + Duration::from_millis(800)).is_empty(),
        "missing source after baseline is suspected-only, not durable ChildStall"
    );
}

#[test]
fn deregistered_child_is_not_polled_into_a_stall() {
    let state = Arc::new(AtomicU8::new(1));
    let registry = registry_with_state(state);
    let path = temp_file("deregister");
    let _guard = FileGuard(path.clone());
    fs::write(&path, "1\n").expect("write liveness baseline");

    let start = Instant::now();
    registry
        .register_file_child(60, 106_006, &path, Duration::from_millis(100), start)
        .expect("register child");
    assert!(registry.deregister(60, 106_006));
    assert!(
        registry.poll(start + Duration::from_millis(500)).is_empty(),
        "deregistered child does not emit a stale report"
    );
}
