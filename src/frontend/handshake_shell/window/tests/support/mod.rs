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
use std::io::Write as _;
use std::marker::PhantomData;
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
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, BOOL, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, WAIT_ABANDONED,
    WAIT_OBJECT_0, WPARAM,
};
use windows_sys::Win32::Security::{
    AddAccessAllowedAce, CreateWellKnownSid, GetTokenInformation, ImpersonateAnonymousToken,
    InitializeAcl, InitializeSecurityDescriptor, RevertToSelf, SetSecurityDescriptorDacl,
    TokenStatistics, TokenUser, WinCreatorOwnerRightsSid, WinLocalSystemSid, ACL, ACL_REVISION,
    PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, TOKEN_INFORMATION_CLASS,
    TOKEN_QUERY, TOKEN_STATISTICS, TOKEN_USER, WELL_KNOWN_SID_TYPE,
};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard,
};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, CloseWindowStation, CreateDesktopW, CreateWindowStationW,
    GetProcessWindowStation, GetThreadDesktop, GetUserObjectInformationW, SetProcessWindowStation,
    SetThreadDesktop, HDESK, HWINSTA, UOI_FLAGS, UOI_IO, UOI_NAME, USEROBJECTFLAGS,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, GetCurrentThread, GetCurrentThreadId, OpenProcessToken,
    ReleaseMutex, WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetDesktopWindow, GetGUIThreadInfo, GetWindow, PeekMessageW,
    SendMessageW, SetWindowPos, CWF_CREATE_ONLY, GUITHREADINFO, GW_HWNDNEXT, HWND_TOP, MSG,
    PM_NOREMOVE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WM_NULL, WSF_VISIBLE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};

use watches::{is_ours, FocusWatch, Watchdog};

const GENERIC_ALL: u32 = 0x1000_0000;
/// Every right to a window station (WINSTA_ALL_ACCESS, winuser.h).
const WINSTA_ALL_ACCESS: u32 = 0x37f;
/// READ_CONTROL (winnt.h): all the fresh window station's owner may do.
const READ_CONTROL: u32 = 0x0002_0000;
/// SECURITY_DESCRIPTOR_REVISION (winnt.h).
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;

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

/// The fresh window station [`on_private_window_station`] switched the process to, its name in lower case and its
/// handle, from the moment its clipboard opened ([`preflight`]) until the process leaves it. Every clipboard write of
/// the tests checks it first ([`assert_clipboard_is_private`], [`clipboard_is_private`]).
static STATION: Mutex<Option<(String, HWINSTA)>> = Mutex::new(None);

/// The window station of the clipboard tests, the process's for as long as this lives (MT-214 E4-F1, E5-F1, E5-F1-S).
/// "A window station contains a clipboard, an atom table, and one or more desktop objects"; one that is not WinSta0
/// "cannot display a user interface or receive user input" (learn.microsoft.com/windows/win32/winstation/window-stations).
/// SetProcessWindowStation "enables the process to access objects in the window station such as desktops, the
/// clipboard, and global atoms", and a desktop CreateDesktop makes then belongs to it, as SetThreadDesktop requires.
///
/// Made here and never opened: CreateWindowStationW with CWF_CREATE_ONLY fails when the station exists, so the logon
/// session's unnamed station, which the sandbox desktops of browsers and other programs of the session share, is never
/// the tests'. Naming a station needs elevation, and without a name "the system forms a window station name using the
/// logon session identifier", so the thread makes it as the anonymous user ([`created_as_anonymous`]) and is itself
/// again at once ([`Anonymous`]). Its access list gives this user and SYSTEM every right and its owner, the anonymous
/// user, READ_CONTROL only: an OWNER RIGHTS entry takes away the owner's implicit WRITE_DAC. That name is fixed
/// ("Service-0x0-3e6$" on the ARM64 development machine), so only one such station exists at a time: [`ClipboardTurn`]
/// keeps the clipboard tests of the session in turn, and a station someone else holds under it stops the test
/// BLOCKED(busy). Then a thread on a desktop of the station must open its clipboard and close it again ([`preflight`])
/// before anything is put on any clipboard; where it cannot, as on the ARM64 development machine (error 5, MT-214
/// clipboard_specialist_20261126), the test stops BLOCKED(environment). The station and its clipboard disappear when
/// this is dropped.
struct PrivateStation {
    station: HWINSTA,
    before: HWINSTA,
}

impl PrivateStation {
    fn enter() -> Self {
        let shared = shared_station_name();
        let security = StationSecurity::new();
        let station = match created_as_anonymous(&security.attributes) {
            Ok(station) => station,
            Err(Unmade::NotAnonymous(error)) => {
                panic!("BLOCKED(environment): the thread cannot act as the anonymous user: error {error}")
            }
            Err(Unmade::Exists) => panic!(
                "BLOCKED(busy): a window station of the anonymous logon session exists (error 183); no clipboard was \
                 opened"
            ),
            Err(Unmade::Refused(error)) => {
                panic!("BLOCKED(environment): no fresh window station: error {error}")
            }
        };
        drop(security);
        let name = object_name(station).unwrap_or_default().to_lowercase();
        if name.is_empty() || name == "winsta0" || name == shared {
            // SAFETY: made above and not in use.
            unsafe { CloseWindowStation(station) };
            panic!("refused: the window station {name:?} is not a fresh one of the tests' own");
        }
        // SAFETY: no arguments, then the station made above, in this process's session.
        let (before, switched) = unsafe { (GetProcessWindowStation(), SetProcessWindowStation(station)) };
        if switched == 0 {
            // SAFETY: no arguments; then the station made above, not in use.
            let error = unsafe { GetLastError() };
            // SAFETY: as above.
            unsafe { CloseWindowStation(station) };
            panic!("BLOCKED(environment): the process was not switched to the fresh window station: error {error}");
        }
        // From here a panic switches the process back and closes the station.
        let entered = Self { station, before };
        if let Err(failed) = preflight(station) {
            panic!(
                "BLOCKED(environment): the clipboard of the fresh window station {name} does not open ({failed}); \
                 nothing was put on any clipboard (MT-214 E5-F1)"
            );
        }
        *STATION.lock().unwrap_or_else(PoisonError::into_inner) = Some((name, station));
        entered
    }
}

/// Why [`created_as_anonymous`] made no window station.
enum Unmade {
    /// ImpersonateAnonymousToken failed, with this error; the thread stayed itself.
    NotAnonymous(u32),
    /// One of that name exists (ERROR_ALREADY_EXISTS).
    Exists,
    /// CreateWindowStationW failed otherwise, with this error.
    Refused(u32),
}

/// A window station the calling thread makes as the anonymous user, with no name and CWF_CREATE_ONLY: a new one, or
/// none. Nothing between the impersonation and its end can panic.
fn created_as_anonymous(attributes: &SECURITY_ATTRIBUTES) -> Result<HWINSTA, Unmade> {
    let anonymous = Anonymous::impersonate().map_err(Unmade::NotAnonymous)?;
    // SAFETY: no name, so the system forms one from the anonymous logon session; the attributes, their descriptor and
    // its access list outlive the call, which copies them.
    let station = unsafe { CreateWindowStationW(ptr::null(), CWF_CREATE_ONLY, WINSTA_ALL_ACCESS, attributes) };
    // SAFETY: no arguments; read before the revert.
    let error = unsafe { GetLastError() };
    drop(anonymous);
    match (station, error) {
        (0, ERROR_ALREADY_EXISTS) => Err(Unmade::Exists),
        (0, error) => Err(Unmade::Refused(error)),
        (station, _) => Ok(station),
    }
}

/// The calling thread acts as the anonymous user while this lives (ImpersonateAnonymousToken). Not `Send`: the
/// impersonation is the thread's.
struct Anonymous(PhantomData<*const ()>);

impl Anonymous {
    fn impersonate() -> Result<Self, u32> {
        // SAFETY: the pseudo-handle of the calling thread.
        if unsafe { ImpersonateAnonymousToken(GetCurrentThread()) } == 0 {
            // SAFETY: no arguments.
            return Err(unsafe { GetLastError() });
        }
        Ok(Self(PhantomData))
    }
}

impl Drop for Anonymous {
    fn drop(&mut self) {
        // SAFETY: no arguments; ends the calling thread's impersonation.
        let reverted = unsafe { RevertToSelf() };
        if reverted == 0 {
            // The thread, and every test after it, would go on as the anonymous user: nothing more may run.
            // SAFETY: no arguments.
            let error = unsafe { GetLastError() };
            let _ = writeln!(std::io::stderr(), "RevertToSelf failed: error {error}; aborting");
            std::process::abort();
        }
    }
}

/// The security the fresh window station is made with ([`PrivateStation`]): an access list of this user and SYSTEM
/// with every right, and OWNER RIGHTS with READ_CONTROL. The system copies it.
struct StationSecurity {
    attributes: SECURITY_ATTRIBUTES,
    _descriptor: Box<SECURITY_DESCRIPTOR>,
    _list: Box<[u64; 32]>,
}

impl StationSecurity {
    fn new() -> Self {
        let user = token_information(TokenUser);
        // SAFETY: the buffer holds the TOKEN_USER asked for, aligned for it, and the SID it points to.
        let user_sid = unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid };
        let mut system = well_known_sid(WinLocalSystemSid);
        let mut owner_rights = well_known_sid(WinCreatorOwnerRightsSid);
        let mut list = Box::new([0u64; 32]);
        let acl = list.as_mut_ptr().cast::<ACL>();
        // SAFETY: the list's buffer, of the size given; each entry copies its SID, and each SID is valid here.
        let built = unsafe {
            InitializeAcl(acl, size_of::<[u64; 32]>() as u32, ACL_REVISION) != 0
                && AddAccessAllowedAce(acl, ACL_REVISION, GENERIC_ALL, user_sid) != 0
                && AddAccessAllowedAce(acl, ACL_REVISION, GENERIC_ALL, system.as_mut_ptr().cast()) != 0
                && AddAccessAllowedAce(acl, ACL_REVISION, READ_CONTROL, owner_rights.as_mut_ptr().cast()) != 0
        };
        // SAFETY: no arguments.
        assert!(built, "no access list: error {}", unsafe { GetLastError() });
        // SAFETY: plain data, initialised below.
        let mut descriptor: Box<SECURITY_DESCRIPTOR> = Box::new(unsafe { std::mem::zeroed() });
        let at: PSECURITY_DESCRIPTOR = (&mut *descriptor as *mut SECURITY_DESCRIPTOR).cast();
        // SAFETY: the descriptor and the list are boxed and live as long as the attributes that point at them.
        let made = unsafe {
            InitializeSecurityDescriptor(at, SECURITY_DESCRIPTOR_REVISION) != 0
                && SetSecurityDescriptorDacl(at, 1, acl, 0) != 0
        };
        // SAFETY: no arguments.
        assert!(made, "no security descriptor: error {}", unsafe { GetLastError() });
        Self {
            attributes: SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: at,
                bInheritHandle: 0,
            },
            _descriptor: descriptor,
            _list: list,
        }
    }
}

/// One class of information about this process's own token, in a buffer of 8-byte units that holds what it points to.
fn token_information(class: TOKEN_INFORMATION_CLASS) -> Box<[u64; 16]> {
    let mut token: HANDLE = 0;
    // SAFETY: the pseudo-handle of this process, and a handle of this function.
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    // SAFETY: no arguments.
    assert_ne!(opened, 0, "no token: error {}", unsafe { GetLastError() });
    let mut information = Box::new([0u64; 16]);
    let mut needed = 0;
    // SAFETY: the buffer and the number outlive the call, which writes at most the buffer's size; then the handle
    // opened above.
    let asked = unsafe {
        let asked = GetTokenInformation(
            token,
            class,
            information.as_mut_ptr().cast(),
            size_of::<[u64; 16]>() as u32,
            &mut needed,
        );
        CloseHandle(token);
        asked
    };
    assert_ne!(asked, 0, "the token did not answer {class}: {needed} bytes needed");
    information
}

/// A well-known SID, in a buffer as large as any SID (SECURITY_MAX_SID_SIZE, 68 bytes).
fn well_known_sid(kind: WELL_KNOWN_SID_TYPE) -> Box<[u64; 9]> {
    let mut sid = Box::new([0u64; 9]);
    let mut size = size_of::<[u64; 9]>() as u32;
    // SAFETY: no domain; the buffer and its size outlive the call, which writes at most that size.
    let made = unsafe { CreateWellKnownSid(kind, ptr::null_mut(), sid.as_mut_ptr().cast(), &mut size) };
    // SAFETY: no arguments.
    assert_ne!(made, 0, "no SID {kind}: error {}", unsafe { GetLastError() });
    sid
}

/// In lower case, the name of the station the system forms for this process's logon session when CreateWindowStationW
/// is given none: "Service-0x" and the session's identifier, as the services' own "Service-0x0-3e7$". The shared one,
/// which [`PrivateStation`] refuses.
fn shared_station_name() -> String {
    let information = token_information(TokenStatistics);
    // SAFETY: the buffer holds the TOKEN_STATISTICS asked for, aligned for it.
    let session = unsafe { information.as_ptr().cast::<TOKEN_STATISTICS>().read() }.AuthenticationId;
    format!("service-0x{:x}-{:x}$", session.HighPart, session.LowPart)
}

/// Whether a thread on a desktop of `station`, the process's, can open its clipboard; it closes it again at once and
/// puts nothing there. Microsoft lists WINSTA_ACCESSCLIPBOARD among the rights of a station that is not interactive,
/// yet on the ARM64 development machine OpenClipboard returns error 5 on every such station.
fn preflight(station: HWINSTA) -> Result<(), String> {
    let desktop = PrivateDesktop::create();
    let handle = desktop.0;
    let opened = std::thread::spawn(move || {
        // SAFETY: no arguments.
        let process = unsafe { GetProcessWindowStation() };
        assert_eq!(process, station, "the process is on the fresh window station");
        // SAFETY: the thread is new and has no window, and the desktop outlives it.
        if unsafe { SetThreadDesktop(handle) } == 0 {
            // SAFETY: no arguments.
            return Err(format!("SetThreadDesktop: error {}", unsafe { GetLastError() }));
        }
        // SAFETY: no window owns the clipboard opened here, and it is closed at once with nothing put on it.
        unsafe {
            if OpenClipboard(0) == 0 {
                return Err(format!("OpenClipboard: error {}", GetLastError()));
            }
            CloseClipboard();
        }
        Ok(())
    })
    .join();
    drop(desktop);
    opened.unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

impl Drop for PrivateStation {
    fn drop(&mut self) {
        *STATION.lock().unwrap_or_else(PoisonError::into_inner) = None;
        // SAFETY: the station the process had, then the handle opened here, which nothing uses any more.
        unsafe {
            SetProcessWindowStation(self.before);
            CloseWindowStation(self.station);
        }
    }
}

/// The turn of one clipboard test among the processes of this session: the fresh window station's name is fixed, so
/// one exists at a time ([`PrivateStation`]). A named mutex, which an owner's ending process gives up too (then the wait
/// says abandoned).
struct ClipboardTurn(HANDLE);

impl ClipboardTurn {
    fn take(limit: Duration) -> Self {
        let name = wide("Local\\hsk-shell-test-clipboard-turn");
        // SAFETY: the name outlives the call; default security, not owned at creation.
        let mutex = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
        // SAFETY: no arguments.
        assert_ne!(mutex, 0, "no turn: error {}", unsafe { GetLastError() });
        let millis = u32::try_from(limit.as_millis()).unwrap_or(u32::MAX - 1);
        // SAFETY: a handle opened above.
        let waited = unsafe { WaitForSingleObject(mutex, millis) };
        if !matches!(waited, WAIT_OBJECT_0 | WAIT_ABANDONED) {
            // SAFETY: opened above, not owned.
            unsafe { CloseHandle(mutex) };
            panic!("another clipboard test kept its turn for {limit:?}: {waited:#x}");
        }
        Self(mutex)
    }
}

impl Drop for ClipboardTurn {
    fn drop(&mut self) {
        // SAFETY: owned by this thread, which took it; then the handle opened by `take`.
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

/// Empties the clipboard of the test's window station when the test's work ends, also when it panics (CX-GIT-003), and
/// only while the process is on that fresh station: never another's clipboard (E5-F1-S).
struct EmptiedAtEnd;

impl Drop for EmptiedAtEnd {
    fn drop(&mut self) {
        if !clipboard_is_private() {
            return;
        }
        // SAFETY: on the test's thread, whose desktop is on the fresh station; no window owns what is emptied. Fails
        // at once while another thread has it open, and then nothing is emptied.
        unsafe {
            if OpenClipboard(0) != 0 {
                EmptyClipboard();
                CloseClipboard();
            }
        }
    }
}

/// Run `work` as [`on_private_desktop`] does, with the process on a window station made fresh for the test
/// ([`PrivateStation`]), taken in turn with every other clipboard test of the session: its clipboard is nobody else's.
/// `work` runs only once that station's clipboard opened ([`preflight`]); otherwise the test stops BLOCKED before
/// anything is put on any clipboard. The only place a test may use the operating system's clipboard; see
/// [`assert_clipboard_is_private`]. Fails unless the clipboard of the window station the process had, the Operator's
/// WinSta0 when run from the Operator's session, kept its sequence number ("incremented whenever the contents of the
/// clipboard change or the clipboard is emptied", GetClipboardSequenceNumber).
pub fn on_private_window_station<T: Send + 'static>(
    limit: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let turn = ClipboardTurn::take(limit);
    // SAFETY: no arguments; the window station the process has before the switch.
    let operators_before = unsafe { GetClipboardSequenceNumber() };
    let station = PrivateStation::enter();
    let value = on_private_desktop(limit, move || {
        assert_clipboard_is_private();
        let _emptied = EmptiedAtEnd;
        work()
    });
    drop(station);
    // SAFETY: no arguments; the process is back on its own window station.
    let operators_after = unsafe { GetClipboardSequenceNumber() };
    drop(turn);
    assert_eq!(
        operators_after, operators_before,
        "the clipboard of the process's own window station changed"
    );
    value
}

/// Panics unless the process is on the fresh window station of [`on_private_window_station`], whose clipboard opened,
/// and which is not interactive. Call before the operating system's clipboard is written, so nothing reaches another's.
pub fn assert_clipboard_is_private() {
    if let Some(why) = not_private() {
        panic!("{why}");
    }
}

/// [`assert_clipboard_is_private`] for code that must not panic: a destructor, a window procedure.
pub fn clipboard_is_private() -> bool {
    not_private().is_none()
}

/// Why the process is not on the fresh window station of [`on_private_window_station`] after its preflight, if not.
fn not_private() -> Option<String> {
    let Some((expected, fresh)) = STATION.lock().unwrap_or_else(PoisonError::into_inner).clone() else {
        return Some("only inside on_private_window_station, once its preflight passed".to_owned());
    };
    // SAFETY: no arguments.
    let station = unsafe { GetProcessWindowStation() };
    let name = object_name(station).unwrap_or_default().to_lowercase();
    if station != fresh || name != expected || name == "winsta0" {
        return Some(format!("the process is on the window station {name:?}, not the test's {expected:?}"));
    }
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
    if asked == 0 || (flags.dwFlags & WSF_VISIBLE as u32) != 0 {
        return Some("the test's window station may be interactive".to_owned());
    }
    None
}

fn object_name(object: HANDLE) -> Option<String> {
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
    if asked == 0 {
        return None;
    }
    let end = name.iter().position(|unit| *unit == 0).unwrap_or(name.len());
    Some(String::from_utf16_lossy(&name[..end]))
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
