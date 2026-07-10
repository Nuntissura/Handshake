//! Child-process watch registry for MT-106.
//!
//! The registry owns no child work and never waits for a child to answer. It reads a bounded passive
//! progress source, checks process liveness with a nonblocking probe, and emits `ChildStallReport`s from
//! [`crate::child_stall::ChildStallDetector`].

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::child_stall::{
    ChildProcessState, ChildProgress, ChildStallDetector, ChildStallReport, ChildStallState,
    CHILD_STALL_THRESHOLD,
};

/// Passive child-progress source kind. Only file-counter is implemented for MT-106 because it is simple,
/// portable, inspectable, and does not require a responsive child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChildLivenessKind {
    FileCounter = 1,
}

impl ChildLivenessKind {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone)]
enum ChildLivenessSource {
    FileCounter(PathBuf),
}

/// Nonblocking process-state probe.
pub trait ChildProcessProbe: Send + Sync {
    fn state(&self) -> ChildProcessState;
}

pub type ChildProbeFactory =
    Arc<dyn Fn(u32) -> Box<dyn ChildProcessProbe + Send + Sync> + Send + Sync + 'static>;

/// A thread-safe collection of watched children.
pub struct ChildRegistry {
    probe_factory: ChildProbeFactory,
    children: Mutex<HashMap<ChildKey, WatchedChild>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ChildKey {
    pid: u32,
    child_session_id: u64,
}

struct WatchedChild {
    detector: ChildStallDetector,
    liveness: ChildLivenessSource,
    probe: Box<dyn ChildProcessProbe + Send + Sync>,
    /// MT-108 (MT-106 residual): the last non-healthy observation discriminant emitted for this child,
    /// so `poll_observations` fires a Suspected/NoBaseline diagnostic ONCE on entry (edge-triggered)
    /// rather than every poll while the condition persists.
    last_observation: Option<ObservationDiscriminant>,
}

/// The variant of a non-healthy child observation, used only to edge-debounce repeated Suspected /
/// NoBaseline observations (their `*_ms` payload grows every poll, so debounce keys on the variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservationDiscriminant {
    NoBaseline,
    Suspected,
    Stalled,
}

/// A non-healthy observation for one watched child on a poll. MT-108 (MT-106 residual): the diagnostics
/// surface emits an event per observation so Suspected + NoBaseline child stalls are visible, not only
/// confirmed `Stalled` edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildObservation {
    pub child_pid: u32,
    pub child_session_id: u64,
    pub kind: ChildObservationKind,
}

/// The kind of a [`ChildObservation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildObservationKind {
    /// The child never established a liveness baseline and has been watched past the threshold.
    NoBaseline { waited_ms: u64 },
    /// A baselined child's progress went stale, but the process is not proven alive / the source is
    /// unavailable — a suspicion, not a confirmed durable stall.
    Suspected { stale_ms: u64 },
    /// A confirmed durable child stall (process alive + progress stale). Carries the durable report.
    Stalled(ChildStallReport),
}

impl ChildRegistry {
    pub fn new(probe_factory: ChildProbeFactory) -> Self {
        Self {
            probe_factory,
            children: Mutex::new(HashMap::new()),
        }
    }

    /// Register or replace one child-process watch. Missing/malformed initial progress is accepted as
    /// "no baseline yet"; the detector will not confirm a stall until a valid progress counter has been
    /// observed at least once.
    pub fn register_file_child(
        &self,
        pid: u32,
        child_session_id: u64,
        liveness_path: impl Into<PathBuf>,
        threshold: Duration,
        now: Instant,
    ) -> io::Result<()> {
        let liveness_path = liveness_path.into();
        let threshold = if threshold.is_zero() {
            CHILD_STALL_THRESHOLD
        } else {
            threshold
        };
        let mut detector = ChildStallDetector::new(pid, child_session_id, threshold);
        if let Some(progress) = read_file_counter(&liveness_path) {
            let _ = detector.poll(now, ChildProcessState::Alive, Some(progress));
        }
        let watched = WatchedChild {
            detector,
            liveness: ChildLivenessSource::FileCounter(liveness_path),
            probe: (self.probe_factory)(pid),
            last_observation: None,
        };
        let mut guard = self
            .children
            .lock()
            .map_err(|_| io::Error::other("child registry mutex poisoned"))?;
        guard.insert(
            ChildKey {
                pid,
                child_session_id,
            },
            watched,
        );
        Ok(())
    }

    pub fn deregister(&self, pid: u32, child_session_id: u64) -> bool {
        let Ok(mut guard) = self.children.lock() else {
            return false;
        };
        guard
            .remove(&ChildKey {
                pid,
                child_session_id,
            })
            .is_some()
    }

    /// Poll every watched child and return only the CONFIRMED durable stall reports (the healthy ->
    /// stalled edges). This is the survivor-record stream. For the full diagnostics stream — which also
    /// surfaces Suspected and NoBaseline child stalls — use [`poll_observations`](Self::poll_observations).
    pub fn poll(&self, now: Instant) -> Vec<ChildStallReport> {
        self.poll_observations(now)
            .into_iter()
            .filter_map(|obs| match obs.kind {
                ChildObservationKind::Stalled(report) => Some(report),
                _ => None,
            })
            .collect()
    }

    /// Poll every watched child and return ALL non-healthy observations — confirmed `Stalled` edges,
    /// plus (MT-108 / MT-106 residual) `Suspected` and `NoBaseline` observations that were previously
    /// computed by the detector and then dropped. Suspected/NoBaseline are edge-triggered (emitted once
    /// on entry) so the diagnostics stream is not spammed every poll; confirmed `Stalled` uses the
    /// detector's own report edge. Exited children are removed. This is the diagnostics-facing surface:
    /// the run loop emits one diagnostics event per observation.
    pub fn poll_observations(&self, now: Instant) -> Vec<ChildObservation> {
        let sources = {
            let Ok(guard) = self.children.lock() else {
                return Vec::new();
            };
            guard
                .iter()
                .map(|(key, watched)| (*key, watched.liveness.clone()))
                .collect::<Vec<_>>()
        };
        let readings = sources
            .into_iter()
            .map(|(key, liveness)| {
                let progress = match liveness {
                    ChildLivenessSource::FileCounter(path) => read_file_counter(&path),
                };
                (key, progress)
            })
            .collect::<Vec<_>>();

        let Ok(mut guard) = self.children.lock() else {
            return Vec::new();
        };
        let mut observations = Vec::new();
        let mut exited = Vec::new();
        for (key, progress) in readings {
            let Some(watched) = guard.get_mut(&key) else {
                continue;
            };
            let poll = watched.detector.poll(now, watched.probe.state(), progress);
            match &poll.state {
                ChildStallState::Exited => {
                    watched.last_observation = None;
                    exited.push(key);
                }
                ChildStallState::Healthy => {
                    watched.last_observation = None;
                }
                ChildStallState::Stalled(report) => {
                    watched.last_observation = Some(ObservationDiscriminant::Stalled);
                    // Confirmed stalls use the detector's own edge (report is Some only on the edge).
                    if poll.report.is_some() {
                        observations.push(ChildObservation {
                            child_pid: key.pid,
                            child_session_id: key.child_session_id,
                            kind: ChildObservationKind::Stalled(report.clone()),
                        });
                    }
                }
                ChildStallState::Suspected { stale_ms } => {
                    let edge = watched.last_observation != Some(ObservationDiscriminant::Suspected);
                    watched.last_observation = Some(ObservationDiscriminant::Suspected);
                    if edge {
                        observations.push(ChildObservation {
                            child_pid: key.pid,
                            child_session_id: key.child_session_id,
                            kind: ChildObservationKind::Suspected {
                                stale_ms: *stale_ms,
                            },
                        });
                    }
                }
                ChildStallState::NoBaseline { waited_ms } => {
                    let edge = watched.last_observation != Some(ObservationDiscriminant::NoBaseline);
                    watched.last_observation = Some(ObservationDiscriminant::NoBaseline);
                    if edge {
                        observations.push(ChildObservation {
                            child_pid: key.pid,
                            child_session_id: key.child_session_id,
                            kind: ChildObservationKind::NoBaseline {
                                waited_ms: *waited_ms,
                            },
                        });
                    }
                }
            }
        }
        for key in exited {
            guard.remove(&key);
        }
        observations
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.children.lock().map(|g| g.len()).unwrap_or(0)
    }
}

/// Read a bounded unsigned counter from `path`. Returns `None` for missing, unreadable, oversized,
/// non-UTF8, or malformed content.
pub fn read_file_counter(path: &Path) -> Option<ChildProgress> {
    let mut file = File::open(path).ok()?;
    let mut limited = String::new();
    file.by_ref().take(64).read_to_string(&mut limited).ok()?;
    let counter = limited.trim().parse::<u64>().ok()?;
    let timestamp_nanos = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    Some(ChildProgress {
        counter,
        timestamp_nanos,
    })
}

pub fn default_child_watch_factory() -> ChildProbeFactory {
    Arc::new(|pid| default_child_probe(pid))
}

fn default_child_probe(pid: u32) -> Box<dyn ChildProcessProbe + Send + Sync> {
    platform::open_process_probe(pid)
}

struct UnknownProcessProbe;

impl ChildProcessProbe for UnknownProcessProbe {
    fn state(&self) -> ChildProcessState {
        ChildProcessState::Unknown
    }
}

#[cfg(windows)]
mod platform {
    use super::{ChildProcessProbe, ChildProcessState, UnknownProcessProbe};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;

    pub fn open_process_probe(pid: u32) -> Box<dyn ChildProcessProbe + Send + Sync> {
        #[allow(unsafe_code)]
        let handle = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        };
        if handle.is_null() {
            Box::new(UnknownProcessProbe)
        } else {
            Box::new(WindowsChildProcessProbe { handle })
        }
    }

    struct WindowsChildProcessProbe {
        handle: HANDLE,
    }

    unsafe impl Send for WindowsChildProcessProbe {}
    unsafe impl Sync for WindowsChildProcessProbe {}

    impl ChildProcessProbe for WindowsChildProcessProbe {
        fn state(&self) -> ChildProcessState {
            #[allow(unsafe_code)]
            match unsafe { WaitForSingleObject(self.handle, 0) } {
                WAIT_TIMEOUT => ChildProcessState::Alive,
                WAIT_OBJECT_0 => ChildProcessState::Exited,
                _ => ChildProcessState::Unknown,
            }
        }
    }

    impl Drop for WindowsChildProcessProbe {
        fn drop(&mut self) {
            #[allow(unsafe_code)]
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{ChildProcessProbe, UnknownProcessProbe};

    pub fn open_process_probe(_pid: u32) -> Box<dyn ChildProcessProbe + Send + Sync> {
        Box::new(UnknownProcessProbe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn temp_file(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "hsk-mt106-child-{label}-{}-{nanos}.txt",
            std::process::id()
        ))
    }

    struct FileGuard(PathBuf);
    impl Drop for FileGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn malformed_liveness_file_does_not_create_a_baseline() {
        let path = temp_file("malformed");
        let _g = FileGuard(path.clone());
        fs::write(&path, "not-a-counter").unwrap();
        assert!(read_file_counter(&path).is_none());
    }

    #[test]
    fn registry_reports_stalled_alive_child_once() {
        let state = Arc::new(AtomicU8::new(1));
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("stalled");
        let _g = FileGuard(path.clone());
        fs::write(&path, "1\n").unwrap();

        let start = Instant::now();
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), start)
            .unwrap();
        let reports = registry.poll(start + Duration::from_millis(150));
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].child_pid, 55);
        assert_eq!(reports[0].child_session_id, 99);
        assert!(registry.poll(start + Duration::from_millis(250)).is_empty());
    }

    #[test]
    fn missing_liveness_after_baseline_does_not_confirm_stall() {
        let state = Arc::new(AtomicU8::new(1));
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("missing-after-baseline");
        fs::write(&path, "1\n").unwrap();
        let start = Instant::now();
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), start)
            .unwrap();
        fs::remove_file(&path).unwrap();
        assert!(
            registry.poll(start + Duration::from_millis(500)).is_empty(),
            "source-unavailable after a baseline is suspected, not durable ChildStall"
        );
    }

    #[test]
    fn missing_baseline_and_unknown_process_do_not_confirm() {
        let state = Arc::new(AtomicU8::new(0));
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("missing");
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), Instant::now())
            .unwrap();
        assert!(registry
            .poll(Instant::now() + Duration::from_millis(500))
            .is_empty());
    }

    #[test]
    fn exited_child_is_removed() {
        let state = Arc::new(AtomicU8::new(2));
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("exited");
        let _g = FileGuard(path.clone());
        fs::write(&path, "1\n").unwrap();
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), Instant::now())
            .unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.poll(Instant::now()).is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn poll_observations_surfaces_nobaseline_once_for_a_never_baselined_child() {
        // MT-108 (MT-106 residual): a registered child that never establishes a baseline is surfaced as
        // a NoBaseline observation past the threshold (mandatory-baseline observability), edge-triggered.
        let state = Arc::new(AtomicU8::new(1)); // Alive
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("nobaseline-obs"); // never written -> no baseline is ever established
        let start = Instant::now();
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), start)
            .unwrap();

        // Before the threshold, a fresh registration is not an observation.
        assert!(registry
            .poll_observations(start + Duration::from_millis(10))
            .is_empty());

        // Past the threshold with still no baseline -> exactly one NoBaseline observation.
        let obs = registry.poll_observations(start + Duration::from_millis(200));
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].child_pid, 55);
        assert_eq!(obs[0].child_session_id, 99);
        assert!(
            matches!(obs[0].kind, ChildObservationKind::NoBaseline { waited_ms } if waited_ms >= 100),
            "got {:?}",
            obs[0].kind
        );

        // Edge-triggered: while still no-baseline, a subsequent poll does not re-emit.
        assert!(registry
            .poll_observations(start + Duration::from_millis(300))
            .is_empty());

        // The confirmed-only stream never surfaces NoBaseline.
        assert!(registry
            .poll(start + Duration::from_millis(400))
            .is_empty());
    }

    #[test]
    fn poll_observations_surfaces_suspected_when_source_unavailable_after_baseline() {
        // MT-108 (MT-106 residual): a baselined child whose progress source disappears is surfaced as a
        // Suspected observation (process not proven alive / source gone), NOT a confirmed durable stall.
        let state = Arc::new(AtomicU8::new(1)); // Alive
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("suspected-obs");
        fs::write(&path, "1\n").unwrap();
        let start = Instant::now();
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), start)
            .unwrap();
        fs::remove_file(&path).unwrap(); // source unavailable after a baseline existed

        let obs = registry.poll_observations(start + Duration::from_millis(200));
        assert_eq!(obs.len(), 1);
        assert!(
            matches!(obs[0].kind, ChildObservationKind::Suspected { .. }),
            "source-unavailable after a baseline is Suspected; got {:?}",
            obs[0].kind
        );

        // Confirmed-only stream stays empty for the same condition (not a durable stall).
        // (poll() re-polls the same detector; use a later instant so the edge-debounce does not hide it —
        // Suspected never enters the confirmed stream regardless.)
        assert!(registry
            .poll(start + Duration::from_millis(400))
            .is_empty());
    }

    #[test]
    fn poll_observations_surfaces_confirmed_stall() {
        // The confirmed-stall edge also appears in the observation stream (as a Stalled kind).
        let state = Arc::new(AtomicU8::new(1)); // Alive
        let factory_state = Arc::clone(&state);
        let registry = ChildRegistry::new(Arc::new(move |_| {
            Box::new(FakeProbe(Arc::clone(&factory_state)))
        }));
        let path = temp_file("confirmed-obs");
        let _g = FileGuard(path.clone());
        fs::write(&path, "1\n").unwrap();
        let start = Instant::now();
        registry
            .register_file_child(55, 99, &path, Duration::from_millis(100), start)
            .unwrap();
        let obs = registry.poll_observations(start + Duration::from_millis(150));
        assert_eq!(obs.len(), 1);
        assert!(
            matches!(&obs[0].kind, ChildObservationKind::Stalled(r) if r.child_pid == 55),
            "confirmed stall surfaces as a Stalled observation; got {:?}",
            obs[0].kind
        );
    }
}
