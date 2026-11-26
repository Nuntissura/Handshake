//! A desktop of the test's own, and watches on it and on the Operator's.
//!
//! Every test that creates a window runs its loop on a desktop that receives no input. A window there can be neither
//! seen nor brought to the foreground, whatever the code under test does. One watch runs on the Operator's desktop for
//! the whole test and fails it if a window of this process ever is in the foreground or holds the focus there; it
//! cannot see a window on the private desktop, so it guards the Operator and proves nothing about the code. The other
//! runs on the private desktop and fails the test if a window the test named with [`must_stay_inactive`] is ever the
//! active or focused window of the test's thread; [`control_the_watch`] proves that it sees an activation. The first
//! watch's own control can only be that it recognises a window of this process, since none may ever be in front of
//! the Operator. Whatever runs a loop or makes a window here fails at once on a desktop that receives input.

#![allow(dead_code)] // Each test target uses a part.
#![allow(unsafe_code)]

mod watches;

use std::cell::RefCell;
use std::fmt::Debug;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handshake_shell_window::{
    Control, DispatchedWake, Event, LoopConfig, LoopError, RunReport, ShellHost,
};
use windows_sys::Win32::Foundation::{GetLastError, BOOL, HANDLE, HWND, LPARAM, WPARAM};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, CloseWindowStation, CreateDesktopW, CreateWindowStationW,
    GetProcessWindowStation, GetThreadDesktop, GetUserObjectInformationW, SetProcessWindowStation,
    SetThreadDesktop, HDESK, HWINSTA, UOI_FLAGS, UOI_IO, UOI_NAME, USEROBJECTFLAGS,
};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetDesktopWindow, GetGUIThreadInfo, GetWindow, PeekMessageW,
    SendMessageW, SetWindowPos, GUITHREADINFO, GW_HWNDNEXT, HWND_TOP, MSG, PM_NOREMOVE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, WM_NULL, WSF_VISIBLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
    WS_VISIBLE,
};

use watches::{is_ours, FocusWatch, Watchdog};

const GENERIC_ALL: u32 = 0x1000_0000;

/// CreateWindowStation fails when the name exists.
const CWF_CREATE_ONLY: u32 = 1;

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
        control_the_operators_watch();
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

/// The name of the window station [`on_private_window_station`] switched the process to, in lower case, while it is.
static STATION: Mutex<Option<String>> = Mutex::new(None);

/// A window station of the test's own, the process's for as long as this lives. "A window station contains a
/// clipboard, an atom table, and one or more desktop objects"; one that is not WinSta0 "cannot display a user interface
/// or receive user input" (learn.microsoft.com/windows/win32/winstation/window-stations). SetProcessWindowStation
/// "enables the process to access objects in the window station such as desktops, the clipboard, and global atoms",
/// and a desktop CreateDesktop makes then belongs to it, as SetThreadDesktop requires.
struct PrivateStation {
    station: HWINSTA,
    before: HWINSTA,
}

impl PrivateStation {
    fn enter() -> Self {
        let name = format!("hsk-shell-test-station-{}", std::process::id());
        let wide_name = wide(&name);
        // SAFETY: the name outlives the call; the station gets the default security.
        let station = unsafe {
            CreateWindowStationW(wide_name.as_ptr(), CWF_CREATE_ONLY, GENERIC_ALL, ptr::null())
        };
        // SAFETY: no arguments.
        assert_ne!(station, 0, "no window station: error {}", unsafe { GetLastError() });
        // SAFETY: no arguments, then a station of this process's session.
        let (before, switched) = unsafe { (GetProcessWindowStation(), SetProcessWindowStation(station)) };
        if switched == 0 {
            // SAFETY: created above and not in use.
            unsafe { CloseWindowStation(station) };
            panic!("the process was not switched to the test's window station");
        }
        *STATION.lock().unwrap_or_else(PoisonError::into_inner) = Some(name.to_lowercase());
        Self { station, before }
    }
}

impl Drop for PrivateStation {
    fn drop(&mut self) {
        *STATION.lock().unwrap_or_else(PoisonError::into_inner) = None;
        // SAFETY: the station the process had, then the one made here, which nothing uses any more.
        unsafe {
            SetProcessWindowStation(self.before);
            CloseWindowStation(self.station);
        }
    }
}

/// Run `work` as [`on_private_desktop`] does, with the process on a window station of the test's own: its clipboard is
/// no other program's. The only place a test may use the operating system's clipboard; see
/// [`assert_clipboard_is_private`].
pub fn on_private_window_station<T: Send + 'static>(
    limit: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let station = PrivateStation::enter();
    let value = on_private_desktop(limit, work);
    drop(station);
    value
}

/// Panics unless the process is on the window station of [`on_private_window_station`], which is not interactive.
/// Call before the operating system's clipboard is used, so the Operator's is never opened.
pub fn assert_clipboard_is_private() {
    let expected = STATION
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .expect("only inside on_private_window_station");
    // SAFETY: no arguments.
    let station = unsafe { GetProcessWindowStation() };
    let name = object_name(station).to_lowercase();
    assert_eq!(name, expected, "the process's window station is the test's");
    assert_ne!(name, "winsta0");
    // SAFETY: integers; the result outlives the call and says its size.
    let (asked, flags) = unsafe {
        let mut flags: USEROBJECTFLAGS = std::mem::zeroed();
        let asked = GetUserObjectInformationW(
            station,
            UOI_FLAGS,
            (&mut flags as *mut USEROBJECTFLAGS).cast(),
            size_of::<USEROBJECTFLAGS>() as u32,
            ptr::null_mut(),
        );
        (asked, flags)
    };
    assert_ne!(asked, 0, "the station did not say whether it is interactive");
    assert_eq!(flags.dwFlags & WSF_VISIBLE as u32, 0, "not interactive");
}

fn object_name(object: HANDLE) -> String {
    let mut name = [0u16; 256];
    let mut needed = 0;
    // SAFETY: the buffer and the number outlive the call, which writes at most the buffer's size in bytes.
    let asked = unsafe {
        GetUserObjectInformationW(
            object,
            UOI_NAME,
            name.as_mut_ptr().cast(),
            (name.len() * size_of::<u16>()) as u32,
            &mut needed,
        )
    };
    assert_ne!(asked, 0, "the object has no name");
    let end = name.iter().position(|unit| *unit == 0).unwrap_or(name.len());
    String::from_utf16_lossy(&name[..end])
}

/// Run `work` on a new thread on the calling thread's desktop: another program, as far as the loop can tell.
pub fn spawn_on_this_desktop<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> JoinHandle<T> {
    // SAFETY: no arguments.
    let desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    std::thread::spawn(move || {
        // SAFETY: the thread is new and has no window; the desktop outlives it, its caller's thread being on it.
        assert_ne!(unsafe { SetThreadDesktop(desktop) }, 0);
        work()
    })
}

/// Send a message to a window of a thread that does not look at its queue for `hang`, as a hung program's. The calling
/// thread waits in SendMessage meanwhile and handles only the messages other threads send it (SendMessage, Remarks).
pub fn send_to_a_thread_that_hangs(hang: Duration) {
    let (window, window_is) = channel();
    let done = Arc::new(AtomicBool::new(false));
    let finished = Arc::clone(&done);
    let hung = spawn_on_this_desktop(move || {
        let class = wide("STATIC");
        // SAFETY: a class of the system's, hidden; every pointer outlives the call.
        let hwnd = unsafe {
            CreateWindowExW(0, class.as_ptr(), ptr::null(), WS_POPUP, 0, 0, 10, 10, 0, 0, 0, ptr::null())
        };
        assert_ne!(hwnd, 0);
        let _ = window.send(hwnd);
        std::thread::sleep(hang);
        while !finished.load(Ordering::Acquire) {
            // SAFETY: a structure of this function; nothing is taken out, the sent message is handled.
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                PeekMessageW(&mut message, 0, 0, 0, PM_NOREMOVE);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // SAFETY: created above, on this thread.
        unsafe { DestroyWindow(hwnd) };
    });
    let hwnd = window_is.recv().expect("the hung thread made its window");
    // SAFETY: plain values, to the window of the hung thread.
    unsafe { SendMessageW(hwnd, WM_NULL, 0, 0) };
    done.store(true, Ordering::Release);
    if let Err(panic) = hung.join() {
        std::panic::resume_unwind(panic);
    }
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

/// The control of the watch on the Operator's desktop, on the private one: what it counts, a window of this process,
/// it recognises, and the desktop's own window it does not.
fn control_the_operators_watch() {
    let class = wide("STATIC");
    // SAFETY: a class of the system's; hidden, on the private desktop; every pointer outlives the call.
    let hwnd = unsafe {
        CreateWindowExW(0, class.as_ptr(), ptr::null(), WS_POPUP, 0, 0, 10, 10, 0, 0, 0, ptr::null())
    };
    assert_ne!(hwnd, 0);
    let recognised = is_ours(hwnd);
    // SAFETY: created above, on this thread; the other call has no arguments.
    let (_, desktop) = unsafe { (DestroyWindow(hwnd), GetDesktopWindow()) };
    assert!(recognised, "the Operator's watch sees a window of this process");
    assert!(!is_ours(desktop) && !is_ours(0));
}

/// The control of the private watch: a plain window it watches is activated (SetWindowPos without SWP_NOACTIVATE),
/// and the watch counts it. Call on the test's thread before the loop runs; afterwards the count is 0 again.
pub fn control_the_watch() {
    assert!(!receives_input(), "only on a private desktop");
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

/// A plain visible window on the private desktop, in the place of one of the Operator's.
pub struct Decoy(pub HWND);

impl Decoy {
    pub fn create() -> Self {
        assert!(!receives_input(), "only on a private desktop");
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

/// Run the loop with a host made of two closures. Only on a private desktop: the source scan finds a call outside
/// [`on_private_desktop`], and this finds one it missed.
pub fn run_script(
    config: LoopConfig,
    on_event: impl Fn(&Control, Event) + 'static,
    on_wake: impl Fn(&Control, &DispatchedWake<'_>) + 'static,
) -> Result<RunReport, LoopError> {
    assert!(!receives_input(), "a loop runs only on a private desktop");
    handshake_shell_window::run(config, Rc::new(Script { on_event, on_wake }))
}

/// Send `messages` to `hwnd` from another thread on this thread's desktop, one after the other, while this thread
/// takes them in: the system hands a sent message over inside the receiving thread's next look at its queue.
pub fn send_from_another_thread(hwnd: HWND, messages: Vec<(u32, WPARAM, LPARAM)>) {
    // SAFETY: no arguments.
    let desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    let sender = std::thread::spawn(move || {
        // SAFETY: the thread is new and has no window; the desktop outlives it, its caller's thread being on it.
        assert_ne!(unsafe { SetThreadDesktop(desktop) }, 0);
        for (message, wparam, lparam) in messages {
            // SAFETY: plain values, to the caller's window.
            unsafe { SendMessageW(hwnd, message, wparam, lparam) };
        }
    });
    while !sender.is_finished() {
        // SAFETY: a structure of this function; nothing is taken out of the queue, sent messages are handled.
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            PeekMessageW(&mut message, 0, 0, 0, PM_NOREMOVE);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if let Err(panic) = sender.join() {
        std::panic::resume_unwind(panic);
    }
}

/// A list that a test's closures share.
#[derive(Clone)]
pub struct Log<T>(Rc<RefCell<Vec<T>>>);

impl<T> Default for Log<T> {
    fn default() -> Self {
        Self(Rc::default())
    }
}

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
