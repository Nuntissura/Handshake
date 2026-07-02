//! The OS HUNG-WINDOW PROBE (MT-091, §6.13.5 — the second of the double-signal gate).
//!
//! Master Spec v02.196 §6.13.5 requires that staleness of the heartbeat *alone* is not enough to
//! declare a freeze: *"Staleness alone is corroborated by an OS hung-window probe ... before Palmistry
//! declares a freeze."* The hung-window probe is the corroborating signal. A legitimate long frame
//! (the UI thread is busy for a few seconds but its message pump is still alive) must NOT be declared a
//! hard freeze; only a window whose message pump has actually stopped pumping is a real freeze.
//!
//! # The technique (the field-standard Win32 hung-window check)
//!
//! This is exactly what Task Manager uses to show "Not responding": send the window a benign
//! [`WM_NULL`](windows_sys::Win32::UI::WindowsAndMessaging) message with
//! [`SendMessageTimeoutW`](windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW) and the
//! `SMTO_ABORTIFHUNG | SMTO_BLOCK` flags and a bounded timeout. The call is delivered to the window's
//! message queue and waits for the window procedure to process it:
//!
//! - if the window procedure processes `WM_NULL` within the timeout, the pump is alive => **responding**;
//! - if the call TIMES OUT, the message pump is stuck (the UI thread is not pumping) => **not responding**.
//!
//! `SMTO_ABORTIFHUNG` makes the call return immediately if the system already considers the window hung
//! (so the probe never waits the full timeout on an already-flagged window), and `SMTO_BLOCK` prevents
//! our probe thread from processing incoming messages while it waits. `WM_NULL` is a no-op message: it
//! changes nothing in the target, it only measures whether the pump is alive.
//!
//! # HWND resolution from the parent PID
//!
//! Palmistry knows Handshake's PID (MT-089 input) but the Win32 probe needs an HWND. We resolve the top-
//! level window owned by that PID by enumerating all top-level windows
//! ([`EnumWindows`](windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows)) and matching each one's
//! owning process id ([`GetWindowThreadProcessId`](windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId))
//! against the parent PID. The first visible top-level match is used. If no window is found (Handshake
//! has no window yet, or is headless), the probe reports `WindowNotFound` and the detector treats that
//! as "cannot corroborate" — it does NOT invent a freeze from a missing window (RISK-011-5).
//!
//! # The trait seam (headless / CI testability)
//!
//! Driving a real Win32 message pump from a headless CI test is awkward, so the probe is behind the
//! [`HungWindowProbe`] trait. Tests inject a [`FakeHungWindowProbe`] returning a canned result so the
//! double-signal gate is provable without a real window; the real Win32 implementation
//! ([`Win32HungWindowProbe`]) sits behind the same seam and is what `main` uses on Windows.

/// The result of one hung-window probe. A small closed enum so the freeze detector reasons over a typed
/// signal, never a bare bool that conflates "responding" with "could not check".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    /// The window processed the probe message within the timeout — its message pump is alive. A stale
    /// heartbeat with a RESPONDING window is a legitimate long frame, NOT a hard freeze (§6.13.5).
    Responding,
    /// The probe TIMED OUT (or the system already flagged the window hung) — the message pump is stuck.
    /// This is the corroborating signal that, together with heartbeat staleness, confirms a freeze.
    NotResponding,
    /// No top-level window owned by the watched PID could be resolved (Handshake has no window yet, is
    /// headless, or already gone). The detector treats this as "cannot corroborate" — it does NOT
    /// confirm a freeze from a missing window (RISK-011-5).
    WindowNotFound,
}

/// The probe seam (§6.13.5 corroboration). `probe()` performs ONE bounded hung-window check and returns
/// a typed [`ProbeResult`]. Implemented for real on Windows by [`Win32HungWindowProbe`]; faked in tests
/// by [`FakeHungWindowProbe`]. The detector depends only on this trait so the double-signal gate is the
/// same code on Windows and in a headless test.
pub trait HungWindowProbe: Send {
    /// Run one bounded hung-window probe of the watched process's top-level window.
    fn probe(&self) -> ProbeResult;
}

/// A test/headless fake: returns a fixed [`ProbeResult`] every call. Lets the double-signal gate be
/// proven deterministically (stale + NotResponding => Frozen; stale + Responding => Suspected-only)
/// without a real Win32 message pump — the MT's required injection seam ("a `HungWindowProbe` trait so a
/// headless/non-Windows test can inject a fake probe result"). It is the live probe on the
/// `#[cfg(not(windows))]` build (where no real Win32 window exists) and is constructed by the in-crate
/// `#[cfg(test)]` freeze-detector unit tests on every platform.
///
/// `#[allow(dead_code)]`: on a Windows NON-test build both consumers are `cfg`-disabled (the real
/// `Win32HungWindowProbe` is used instead, and the unit tests are `cfg(test)`-only), so the bin-only
/// Windows compile sees no caller. The allow is scoped + documented rather than deleting a contractually-
/// required seam; the type IS exercised under `cargo test` (the gate clippy `--all-targets` runs).
#[allow(dead_code)]
pub struct FakeHungWindowProbe {
    result: ProbeResult,
}

#[allow(dead_code)]
impl FakeHungWindowProbe {
    /// A fake that always reports `result`.
    pub fn new(result: ProbeResult) -> Self {
        Self { result }
    }
}

impl HungWindowProbe for FakeHungWindowProbe {
    fn probe(&self) -> ProbeResult {
        self.result
    }
}

// ----------------------------------------------------------------------------------------------------
// The REAL Win32 hung-window probe (behind the HungWindowProbe seam).
// ----------------------------------------------------------------------------------------------------

/// Default bounded timeout for the `SendMessageTimeoutW` probe. Must be SHORT and BOUNDED so the probe
/// never blocks the Palmistry poll thread (RISK-011-4): a hung window would otherwise stall the probe
/// for the full timeout. 250ms is long enough that a momentarily-busy-but-alive pump still answers, and
/// short enough that a probe of a genuinely hung window returns quickly (further shortened by
/// `SMTO_ABORTIFHUNG`, which returns immediately if the system already flags the window hung).
#[cfg(windows)]
pub const PROBE_TIMEOUT_MS: u32 = 250;

/// The real Windows hung-window probe. Resolves the watched PID's top-level HWND once at construction
/// (lazily re-resolved if it was not found yet), then on each `probe()` sends `WM_NULL` with
/// `SendMessageTimeoutW(SMTO_ABORTIFHUNG | SMTO_BLOCK, timeout)`. A timeout means the pump is stuck.
#[cfg(windows)]
pub struct Win32HungWindowProbe {
    /// The watched process id; retained so the HWND can be (re)resolved lazily if it was not present at
    /// construction (Handshake may not have created its window yet when Palmistry starts).
    pid: u32,
    /// The resolved top-level HWND, cached. `RefCell` because `probe(&self)` may need to (re)resolve it,
    /// and the probe is owned by a single poll thread (no cross-thread sharing of the cell).
    hwnd: std::cell::RefCell<Option<isize>>,
    /// The bounded probe timeout in milliseconds.
    timeout_ms: u32,
}

// SAFETY: the probe is moved onto the single dedicated freeze-poll thread and used only there; the
// cached HWND (`isize`) is a plain handle value, and the only OS calls (`SendMessageTimeoutW`,
// `EnumWindows`, `GetWindowThreadProcessId`) are thread-safe to call. The `RefCell` is never shared
// across threads (the type is not `Sync`), so moving it to the poll thread is sound.
#[cfg(windows)]
unsafe impl Send for Win32HungWindowProbe {}

#[cfg(windows)]
impl Win32HungWindowProbe {
    /// Build a probe for `pid` with the default [`PROBE_TIMEOUT_MS`]. The HWND is resolved eagerly here
    /// (so a window that already exists is cached) and lazily re-resolved on a later `probe()` if it was
    /// not present yet.
    pub fn new(pid: u32) -> Self {
        Self::with_timeout(pid, PROBE_TIMEOUT_MS)
    }

    /// Build a probe for `pid` with an explicit bounded `timeout_ms`.
    pub fn with_timeout(pid: u32, timeout_ms: u32) -> Self {
        let hwnd = resolve_top_level_hwnd(pid);
        Self {
            pid,
            hwnd: std::cell::RefCell::new(hwnd),
            timeout_ms,
        }
    }

    /// Resolve (or re-resolve) the cached HWND, returning it if found. Re-resolution covers the startup
    /// race where Handshake had no window when the probe was built.
    fn ensure_hwnd(&self) -> Option<isize> {
        if let Some(h) = *self.hwnd.borrow() {
            return Some(h);
        }
        let resolved = resolve_top_level_hwnd(self.pid);
        *self.hwnd.borrow_mut() = resolved;
        resolved
    }
}

#[cfg(windows)]
impl HungWindowProbe for Win32HungWindowProbe {
    fn probe(&self) -> ProbeResult {
        use windows_sys::Win32::Foundation::{
            GetLastError, SetLastError, ERROR_INVALID_WINDOW_HANDLE,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_NULL,
        };

        let hwnd = match self.ensure_hwnd() {
            Some(h) => h as windows_sys::Win32::Foundation::HWND,
            None => return ProbeResult::WindowNotFound,
        };

        let mut result: usize = 0;
        // Clear the thread's last-error first: SendMessageTimeoutW is documented NOT to set a last
        // error on a plain timeout on every Windows version, so a stale error code from an earlier
        // call must not masquerade as this call's failure reason.
        unsafe { SetLastError(0) };
        // SAFETY: `hwnd` was resolved from a live EnumWindows match (or is re-resolved each call). WM_NULL
        // is a no-op message that mutates nothing. The timeout is bounded so the call cannot block the
        // poll thread indefinitely (RISK-011-4); SMTO_ABORTIFHUNG returns immediately if the system
        // already flags the window hung. `&mut result` is a valid out pointer for the lpdwResult arg.
        let ret = unsafe {
            SendMessageTimeoutW(
                hwnd,
                WM_NULL,
                0,
                0,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                self.timeout_ms,
                &mut result,
            )
        };
        // SendMessageTimeoutW returns nonzero on success (the window processed the message in time) and
        // 0 on failure/timeout. MT-091 remediation: a zero return CONFLATES two very different cases —
        //
        //   (a) the call TIMED OUT / the system flags the window hung => the pump is stuck
        //       (the standard Task-Manager "Not responding" signal), and
        //   (b) the call itself FAILED because the cached HWND no longer names a live window
        //       (Handshake's window was DESTROYED — e.g. the process died or recreated its window).
        //
        // Case (b) must NOT corroborate a freeze: a destroyed window is "cannot corroborate"
        // (RISK-011-5), exactly like never having found a window. Distinguish via GetLastError:
        // ERROR_INVALID_WINDOW_HANDLE => drop the stale cached HWND, re-resolve immediately (so a
        // recreated window is picked up by the NEXT probe), and report WindowNotFound for THIS probe.
        if ret == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_INVALID_WINDOW_HANDLE {
                *self.hwnd.borrow_mut() = resolve_top_level_hwnd(self.pid);
                return ProbeResult::WindowNotFound;
            }
            ProbeResult::NotResponding
        } else {
            ProbeResult::Responding
        }
    }
}

/// Enumerate top-level windows and return the HWND (as `isize`) of the first VISIBLE top-level window
/// owned by `pid`, or `None` if none is found. Field-standard EnumWindows + GetWindowThreadProcessId
/// match. Returns the raw handle as `isize` so the caller can cache it without holding a Win32 type in a
/// `RefCell` across the FFI boundary.
#[cfg(windows)]
fn resolve_top_level_hwnd(pid: u32) -> Option<isize> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    // The state EnumWindows threads through its LPARAM: the target pid + the matched HWND (as isize).
    struct Search {
        pid: u32,
        found: Option<isize>,
    }

    // The EnumWindows callback: for each top-level window, read its owning pid and, on a VISIBLE match,
    // record the HWND and stop the enumeration (return FALSE).
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        // SAFETY: `lparam` is the `&mut Search` we passed to EnumWindows; EnumWindows invokes this
        // synchronously on the calling thread, so the reference is valid for the duration of the call.
        let search = &mut *(lparam as *mut Search);
        let mut owner_pid: u32 = 0;
        // SAFETY: `hwnd` is a live window handle supplied by EnumWindows; `&mut owner_pid` is a valid out
        // pointer.
        GetWindowThreadProcessId(hwnd, &mut owner_pid);
        if owner_pid == search.pid && IsWindowVisible(hwnd) != 0 {
            search.found = Some(hwnd as isize);
            return 0; // FALSE: stop enumerating, we have our window.
        }
        1 // TRUE: keep enumerating.
    }

    let mut search = Search { pid, found: None };
    // SAFETY: `enum_proc` matches the EnumWindows callback ABI; we pass a valid pointer to `search` as
    // the LPARAM, which the callback reconstitutes. EnumWindows runs synchronously and does not retain
    // the pointer after returning.
    unsafe {
        EnumWindows(Some(enum_proc), &mut search as *mut Search as LPARAM);
    }
    search.found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_probe_returns_canned_result() {
        assert_eq!(
            FakeHungWindowProbe::new(ProbeResult::NotResponding).probe(),
            ProbeResult::NotResponding
        );
        assert_eq!(
            FakeHungWindowProbe::new(ProbeResult::Responding).probe(),
            ProbeResult::Responding
        );
        assert_eq!(
            FakeHungWindowProbe::new(ProbeResult::WindowNotFound).probe(),
            ProbeResult::WindowNotFound
        );
    }

    /// On Windows, resolving the HWND of a PID that owns NO top-level window yields `None`, and a probe
    /// of such a process reports `WindowNotFound` rather than fabricating a freeze (RISK-011-5). The
    /// current test process is a console binary with no visible top-level window, so it is a natural
    /// no-window subject.
    #[cfg(windows)]
    #[test]
    fn no_window_pid_resolves_to_window_not_found() {
        let probe = Win32HungWindowProbe::new(std::process::id());
        // The test harness process has no visible top-level window of its own.
        assert_eq!(
            probe.probe(),
            ProbeResult::WindowNotFound,
            "a process with no visible top-level window must report WindowNotFound, not a freeze"
        );
    }
}
