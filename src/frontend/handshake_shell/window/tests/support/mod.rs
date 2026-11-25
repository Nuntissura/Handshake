//! A desktop of the test's own, and watches on it and on the Operator's.
//!
//! Every test that creates a window runs its loop on a desktop that receives no input. A window there can be neither
//! seen nor brought to the foreground, whatever the code under test does. One watch runs on the Operator's desktop for
//! the whole test and fails it if a window of this process ever is in the foreground or holds the focus there; it
//! cannot see a window on the private desktop, so it guards the Operator and proves nothing about the code. The other
//! runs on the private desktop and fails the test if a window the test named with [`must_stay_inactive`] is ever the
//! active or focused window of the test's thread; [`control_the_watch`] proves that it sees an activation.

#![allow(dead_code)] // Each test target uses a part.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::fmt::Debug;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handshake_shell_window::{
    Control, DispatchedWake, Event, LoopConfig, LoopError, RunReport, ShellHost,
};
use windows_sys::Win32::Foundation::{GetLastError, BOOL, HWND};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, CreateDesktopW, GetThreadDesktop, GetUserObjectInformationW, SetThreadDesktop,
    HDESK, UOI_IO,
};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetForegroundWindow, GetGUIThreadInfo, GetWindow,
    GetWindowThreadProcessId, SetWindowPos, GUITHREADINFO, GW_HWNDNEXT, HWND_TOP, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};

const GENERIC_ALL: u32 = 0x1000_0000;

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

struct PrivateDesktop(HDESK);

impl PrivateDesktop {
    fn create() -> Self {
        static NUMBER: AtomicU32 = AtomicU32::new(0);
        let name = wide(&format!(
            "hsk-shell-test-{}-{}",
            std::process::id(),
            NUMBER.fetch_add(1, Ordering::Relaxed)
        ));
        // SAFETY: the name outlives the call.
        let desktop = unsafe {
            CreateDesktopW(
                name.as_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                GENERIC_ALL,
                ptr::null(),
            )
        };
        // SAFETY: no arguments.
        assert_ne!(desktop, 0, "no desktop: error {}", unsafe {
            GetLastError()
        });
        Self(desktop)
    }
}

impl Drop for PrivateDesktop {
    fn drop(&mut self) {
        // SAFETY: created here, and no thread uses it any more.
        unsafe { CloseDesktop(self.0) };
    }
}

/// Run `work` on a thread of its own whose windows are on a private desktop, under the watches.
pub fn on_private_desktop<T: Send + 'static>(
    limit: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let watchdog = Watchdog::start(limit);
    let watch = FocusWatch::start();
    let desktop = PrivateDesktop::create();
    let handle = desktop.0;
    let quiet = Arc::new(QuietWatch::default());
    let for_work = Arc::clone(&quiet);
    let (thread, thread_is) = channel();
    let done = std::thread::spawn(move || {
        // SAFETY: the thread is new and has no window, and the desktop outlives it.
        let bound = unsafe { SetThreadDesktop(handle) };
        // SAFETY: no arguments.
        assert_ne!(bound, 0, "not bound to the desktop: error {}", unsafe {
            GetLastError()
        });
        assert!(
            !receives_input(),
            "the private desktop must not be the Operator's"
        );
        QUIET.with(|watch| *watch.borrow_mut() = Some(for_work));
        // SAFETY: no arguments.
        let _ = thread.send(unsafe { GetCurrentThreadId() });
        work()
    });
    let watcher = thread_is
        .recv()
        .ok()
        .map(|thread| QuietWatch::start(Arc::clone(&quiet), handle, thread));
    let done = done.join();
    quiet.stop.store(true, Ordering::Release);
    if let Some(watcher) = watcher {
        let _ = watcher.join();
    }
    drop(desktop);
    let seen = watch.finish();
    drop(watchdog);
    assert!(seen.samples > 0, "the watch did not run");
    assert_eq!(
        seen.ours, 0,
        "a window of this process was in the foreground or held the focus"
    );
    let value = match done {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    assert!(quiet.samples.load(Ordering::Acquire) > 0, "the private watch did not run");
    assert_eq!(
        quiet.hits.load(Ordering::Acquire),
        0,
        "a window that must stay inactive was active or focused on the private desktop"
    );
    value
}

/// Windows of the test's thread that must never be its active or focused window, and how often one was.
#[derive(Default)]
struct QuietWatch {
    windows: Mutex<Vec<HWND>>,
    hits: AtomicU64,
    samples: AtomicU64,
    stop: AtomicBool,
}

thread_local! {
    static QUIET: RefCell<Option<Arc<QuietWatch>>> = const { RefCell::new(None) };
}

impl QuietWatch {
    /// Samples, from a thread on the same desktop, the active and focused window of the test's thread.
    fn start(watch: Arc<Self>, desktop: HDESK, thread: u32) -> JoinHandle<()> {
        std::thread::spawn(move || {
            // SAFETY: the thread is new and has no window, and the desktop outlives it.
            assert_ne!(unsafe { SetThreadDesktop(desktop) }, 0);
            while !watch.stop.load(Ordering::Acquire) {
                // SAFETY: integers; the structure says its own size. Fails until the thread has a queue.
                let (known, info) = unsafe {
                    let mut info: GUITHREADINFO = std::mem::zeroed();
                    info.cbSize = size_of::<GUITHREADINFO>() as u32;
                    (GetGUIThreadInfo(thread, &mut info) != 0, info)
                };
                if known {
                    let windows = watch.windows.lock().unwrap();
                    let hit = [info.hwndActive, info.hwndFocus]
                        .iter()
                        .any(|hwnd| *hwnd != 0 && windows.contains(hwnd));
                    if hit {
                        watch.hits.fetch_add(1, Ordering::AcqRel);
                    }
                }
                watch.samples.fetch_add(1, Ordering::AcqRel);
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    }
}

fn with_quiet_watch(work: impl FnOnce(&QuietWatch)) {
    QUIET.with(|watch| {
        let watch = watch.borrow();
        work(watch.as_ref().expect("call on the thread of `on_private_desktop`"));
    });
}

/// `hwnd` must never be the active or focused window of this thread; the test fails if it ever is.
pub fn must_stay_inactive(hwnd: HWND) {
    with_quiet_watch(|watch| watch.windows.lock().unwrap().push(hwnd));
}

/// `hwnd` is gone; its handle may come back for another window.
pub fn stop_watching(hwnd: HWND) {
    with_quiet_watch(|watch| watch.windows.lock().unwrap().retain(|kept| *kept != hwnd));
}

/// The control of the private watch: a plain window it watches is activated (SetWindowPos without SWP_NOACTIVATE),
/// and the watch counts it. Call on the test's thread before the loop runs; afterwards the count is 0 again.
pub fn control_the_watch() {
    let class = wide("STATIC");
    // SAFETY: a class of the system's; every pointer outlives the call.
    let hwnd = unsafe {
        CreateWindowExW(0, class.as_ptr(), ptr::null(), WS_POPUP, 5, 5, 40, 40, 0, 0, 0, ptr::null())
    };
    assert_ne!(hwnd, 0);
    must_stay_inactive(hwnd);
    // SAFETY: plain values, for a window of this thread on the private desktop.
    unsafe { SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW) };
    let until = Instant::now() + Duration::from_secs(5);
    let mut seen = false;
    while !seen && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
        with_quiet_watch(|watch| seen = watch.hits.load(Ordering::Acquire) > 0);
    }
    stop_watching(hwnd);
    // SAFETY: created above, on this thread.
    unsafe { DestroyWindow(hwnd) };
    with_quiet_watch(|watch| watch.hits.store(0, Ordering::Release));
    assert!(seen, "the private watch sees an activation");
}

fn receives_input() -> bool {
    let mut receives: BOOL = 1;
    // SAFETY: the desktop is this thread's, and the result outlives the call.
    let asked = unsafe {
        GetUserObjectInformationW(
            GetThreadDesktop(GetCurrentThreadId()),
            UOI_IO,
            (&mut receives as *mut BOOL).cast(),
            size_of::<BOOL>() as u32,
            ptr::null_mut(),
        )
    };
    assert_ne!(
        asked, 0,
        "the desktop did not say whether it receives input"
    );
    receives != 0
}

/// Ends the test process when it hangs. A modal loop that is not ended would hang forever.
struct Watchdog {
    alive: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Watchdog {
    fn start(limit: Duration) -> Self {
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

struct FocusWatch {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<FocusSeen>>,
}

impl FocusWatch {
    /// The sampling thread stays on the process's own desktop, the Operator's.
    fn start() -> Self {
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

    fn finish(mut self) -> FocusSeen {
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

fn is_ours(hwnd: HWND) -> bool {
    let mut process = 0;
    // SAFETY: the number outlives the call.
    unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
    hwnd != 0 && process == std::process::id()
}

/// A plain visible window on the private desktop, in the place of one of the Operator's.
pub struct Decoy(pub HWND);

impl Decoy {
    pub fn create() -> Self {
        let class = wide("STATIC");
        // SAFETY: a class of the system's; every pointer outlives the call.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class.as_ptr(),
                ptr::null(),
                WS_POPUP | WS_VISIBLE,
                10,
                10,
                60,
                60,
                0,
                0,
                0,
                ptr::null(),
            )
        };
        assert_ne!(hwnd, 0);
        Self(hwnd)
    }

    /// Whether `hwnd` is somewhere below this window.
    pub fn is_above(&self, hwnd: HWND) -> bool {
        let mut below = self.0;
        for _ in 0..10_000 {
            // SAFETY: plain values, read only.
            below = unsafe { GetWindow(below, GW_HWNDNEXT) };
            if below == hwnd {
                return true;
            }
            if below == 0 {
                return false;
            }
        }
        panic!("the Z order has no end");
    }
}

impl Drop for Decoy {
    fn drop(&mut self) {
        // SAFETY: created on this thread.
        unsafe { DestroyWindow(self.0) };
    }
}

/// A host made of two closures.
pub struct Script<E, W> {
    pub on_event: E,
    pub on_wake: W,
}

impl<E, W> ShellHost for Script<E, W>
where
    E: Fn(&Control, Event),
    W: Fn(&Control, &DispatchedWake<'_>),
{
    fn on_event(&self, control: &Control, event: Event) {
        (self.on_event)(control, event);
    }

    fn on_wake(&self, control: &Control, wake: &DispatchedWake<'_>) {
        (self.on_wake)(control, wake);
    }
}

/// Run the loop with a host made of two closures.
pub fn run_script(
    config: LoopConfig,
    on_event: impl Fn(&Control, Event) + 'static,
    on_wake: impl Fn(&Control, &DispatchedWake<'_>) + 'static,
) -> Result<RunReport, LoopError> {
    handshake_shell_window::run(config, Rc::new(Script { on_event, on_wake }))
}

/// A list that a test's closures share.
#[derive(Clone)]
pub struct Log<T>(Rc<RefCell<Vec<T>>>);

impl<T: Clone + Debug + PartialEq> Log<T> {
    pub fn new() -> Self {
        Self(Rc::default())
    }

    pub fn add(&self, entry: T) {
        self.0.borrow_mut().push(entry);
    }

    pub fn all(&self) -> Vec<T> {
        self.0.borrow().clone()
    }

    pub fn count(&self, entry: &T) -> usize {
        self.0.borrow().iter().filter(|one| *one == entry).count()
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}
