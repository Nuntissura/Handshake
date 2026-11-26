//! Limits a test keeps by itself, under a plain `cargo test` with no runner configuration (MT-215 E5, E7; CX-EXEC-005,
//! CX-GIT-003): a time limit, and ring files that are removed however the test ends.
//!
//! - Passes: [`RingFile`] is dropped and removes its file.
//! - Fails: the panic unwinds the test's frame and drops the [`RingFile`] there.
//! - Overruns its [`TimeLimit`]: the limit's own thread writes a `TIMEOUT` line to the standard error (past the test
//!   harness's capture), removes every ring file of the process that still exists and ends the process with
//!   [`TIMED_OUT`], which is neither a pass (0) nor a failed assertion (101).
//!
//! Shared by the port's tests and, through `#[path]`, the shell's. The standard library only.

#![allow(dead_code)] // Each test target uses a part.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The exit code of a test process that a [`TimeLimit`] ended.
pub const TIMED_OUT: i32 = 124;

/// The ring files of this process that still exist.
static LIVE_RINGS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn live_rings() -> std::sync::MutexGuard<'static, Vec<PathBuf>> {
    LIVE_RINGS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A ring backing file in Cargo's temporary folder of the test target, which lies inside the artifact root, removed
/// when this is dropped or when a [`TimeLimit`] ends the process. Creates nothing itself.
///
/// Declare it before everything that maps the file, so it is dropped after them.
pub struct RingFile(PathBuf);

impl RingFile {
    pub fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after 1970")
            .as_nanos();
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "{}-{label}-{}-{unique}.ring",
            env!("CARGO_PKG_NAME"),
            std::process::id()
        ));
        live_rings().push(path.clone());
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for RingFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        live_rings().retain(|live| *live != self.0);
    }
}

/// Ends the process if the test has not ended within its limit. Create it first in the test; it ends with it.
pub struct TimeLimit {
    alive: Option<Sender<()>>,
    watchdog: Option<JoinHandle<()>>,
}

impl TimeLimit {
    /// Names the test by its thread, which the test harness names after the test.
    pub fn new(limit: Duration) -> Self {
        let test = std::thread::current()
            .name()
            .unwrap_or("a test")
            .to_owned();
        let (alive, watched) = channel::<()>();
        let watchdog = std::thread::Builder::new()
            .name(format!("time limit of {test}"))
            .spawn(move || {
                if watched.recv_timeout(limit) == Err(RecvTimeoutError::Timeout) {
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "TIMEOUT {test}: no result within {limit:?}"
                    );
                    for ring in live_rings().iter() {
                        let _ = std::fs::remove_file(ring);
                    }
                    std::process::exit(TIMED_OUT);
                }
            })
            .expect("the time limit's thread");
        Self {
            alive: Some(alive),
            watchdog: Some(watchdog),
        }
    }
}

impl Drop for TimeLimit {
    fn drop(&mut self) {
        self.alive.take();
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
    }
}
