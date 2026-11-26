//! The watches every test with a window runs under: one on the Operator's desktop, and one that ends a test that hangs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
};

/// Ends the test process when it hangs. A modal loop that is not ended would hang forever.
pub(super) struct Watchdog {
    alive: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Watchdog {
    pub(super) fn start(limit: Duration) -> Self {
        let (alive, watched) = channel::<()>();
        let thread = std::thread::spawn(move || {
            if watched.recv_timeout(limit) == Err(RecvTimeoutError::Timeout) {
                eprintln!("the test did not end within {limit:?}");
                std::process::exit(101);
            }
        });
        Self {
            alive: Some(alive),
            thread: Some(thread),
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.alive.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FocusSeen {
    pub samples: u64,
    /// Samples in which a window of this process was in the foreground, active or focused on the Operator's desktop.
    pub ours: u64,
}

pub(super) struct FocusWatch {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<FocusSeen>>,
}

impl FocusWatch {
    /// The sampling thread stays on the process's own desktop, the Operator's.
    pub(super) fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut seen = FocusSeen::default();
            while !stopped.load(Ordering::Acquire) {
                seen.samples += 1;
                if foreground_windows().into_iter().any(is_ours) {
                    seen.ours += 1;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            seen
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub(super) fn finish(mut self) -> FocusSeen {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for FocusWatch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn foreground_windows() -> [HWND; 3] {
    // SAFETY: integers and rectangles; the structure says its own size.
    unsafe {
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        GetGUIThreadInfo(0, &mut info);
        [GetForegroundWindow(), info.hwndActive, info.hwndFocus]
    }
}

pub(super) fn is_ours(hwnd: HWND) -> bool {
    let mut process = 0;
    // SAFETY: the number outlives the call.
    unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
    hwnd != 0 && process == std::process::id()
}
