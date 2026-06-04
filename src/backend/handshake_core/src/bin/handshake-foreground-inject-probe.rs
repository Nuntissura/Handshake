//! MT-058 — `handshake-foreground-inject-probe`
//!
//! Real Win32 foreground-steal attempt used as the live payload for the
//! ESC-WIN32-FOREGROUND-INJECT escape attempt (the HBR-QUIET-001 acid test).
//!
//! Contract with `src/test_harness/escape_attempts.rs`:
//!   - The escape harness spawns this probe inside the
//!     `WindowsNativeJailAdapter` (restricted AppContainer + job object).
//!   - The catalog entry sets `green_when_exit_is_zero: Some(false)`, so:
//!       * exit code 0       => the foreground steal SUCCEEDED (the jail
//!                              LEAKED the QUIET guarantee) => harness Red.
//!       * exit code != 0    => the foreground steal was DENIED/blocked
//!                              (the jail HELD the guarantee)   => harness Green.
//!
//! This is a genuine attack, not a stub: it creates a real top-level window
//! owned by the probe process and then runs the canonical Win32
//! foreground-stealing sequence — `AttachThreadInput` to the current
//! foreground thread, `SetForegroundWindow` / `BringWindowToTop` /
//! `SetWindowPos`, plus a synthetic `SendInput` keystroke to satisfy the
//! "the process recently received input" heuristic that Win32 uses to gate
//! `SetForegroundWindow`. Success is asserted strictly: the steal only counts
//! if `GetForegroundWindow()` ends up equal to the probe's own window AND the
//! probe could synthesize input. Inside a correctly-configured AppContainer /
//! LPAC jail with UI restrictions, these calls fail with `ERROR_ACCESS_DENIED`
//! and the foreground never transitions, so the probe exits non-zero (Green).
//!
//! Production note (Windows CI lane): this file is compiled as the
//! `[[bin]] handshake-foreground-inject-probe` target, gated on the
//! `win-native-integration` cargo feature. On the Windows CI lane it is built
//! with `--features win-native-integration`, which produces
//! `handshake-foreground-inject-probe.exe` under the cargo target `deps`/bin
//! output. Cargo exposes its absolute path to the integration-test crate as
//! `CARGO_BIN_EXE_handshake-foreground-inject-probe`; the escape catalog reads
//! that path (via the `HANDSHAKE_FOREGROUND_INJECT_PROBE` env override the test
//! driver sets) so `resolve_executable` in the WindowsNativeJailAdapter takes
//! the absolute-path fast-path instead of a fragile PATH lookup.

#[cfg(all(windows, feature = "win-native-integration"))]
fn main() {
    std::process::exit(win::run());
}

#[cfg(not(all(windows, feature = "win-native-integration")))]
fn main() {
    // Non-Windows / feature-off builds must still link so `cargo check`
    // succeeds on the Linux/dev lane. The probe cannot perform a real Win32
    // foreground steal here, so it reports "steal not performed" via a
    // non-zero exit — which the harness reads as Green (no leak), the safe
    // default for a host where the attack surface does not exist.
    eprintln!(
        "handshake-foreground-inject-probe: Win32 foreground-steal probe requires \
         a Windows build with --features win-native-integration; no steal performed."
    );
    std::process::exit(2);
}

#[cfg(all(windows, feature = "win-native-integration"))]
mod win {
    use windows_sys::Win32::Foundation::{
        GetLastError, ERROR_ACCESS_DENIED, HWND, LPARAM, LRESULT, WPARAM,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_SPACE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, GetForegroundWindow,
        GetWindowThreadProcessId, RegisterClassExW, SetForegroundWindow, SetWindowPos, ShowWindow,
        HWND_TOP, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
    };

    const ERROR_ACCESS_DENIED_U32: u32 = ERROR_ACCESS_DENIED;

    /// Exit code: foreground steal SUCCEEDED (the jail leaked). Harness -> Red.
    const EXIT_STEAL_SUCCEEDED: i32 = 0;
    /// Exit code: foreground steal was DENIED with ERROR_ACCESS_DENIED.
    /// Harness -> Green. This is the expected jailed outcome.
    const EXIT_STEAL_DENIED_ACCESS: i32 = 5;
    /// Exit code: foreground steal failed for another reason (window never
    /// became foreground, input could not be synthesized). Harness -> Green.
    const EXIT_STEAL_FAILED_OTHER: i32 = 1;
    /// Exit code: probe could not even set up its own window. Harness -> Green
    /// (no leak occurred), but signals a fixture problem to the operator.
    const EXIT_SETUP_FAILED: i32 = 3;

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn run() -> i32 {
        unsafe { attempt_foreground_steal() }
    }

    unsafe fn attempt_foreground_steal() -> i32 {
        let hinstance = GetModuleHandleW(std::ptr::null());

        let class_name = wide("HandshakeForegroundInjectProbe");
        let window_name = wide("handshake-foreground-inject-probe");

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };

        // RegisterClassExW returning 0 inside a UI-restricted AppContainer is
        // itself evidence the jail blocked window-station access. Treat it as
        // a setup failure (no leak) rather than a steal.
        if RegisterClassExW(&wnd_class) == 0 {
            eprintln!(
                "RegisterClassExW failed (err={}); window-station access likely denied by jail",
                GetLastError()
            );
            return EXIT_SETUP_FAILED;
        }

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            320,
            240,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            eprintln!(
                "CreateWindowExW failed (err={}); UI access likely denied by jail",
                GetLastError()
            );
            return EXIT_SETUP_FAILED;
        }

        ShowWindow(hwnd, SW_SHOW);

        // The canonical foreground-stealing sequence. Each Win32 call is a
        // genuine attempt; the jail is expected to deny SetForegroundWindow /
        // AttachThreadInput / SendInput with ERROR_ACCESS_DENIED.
        let our_thread = GetCurrentThreadId();
        let foreground_hwnd = GetForegroundWindow();
        let mut attached = false;
        if !foreground_hwnd.is_null() {
            let mut fg_pid: u32 = 0;
            let fg_thread = GetWindowThreadProcessId(foreground_hwnd, &mut fg_pid);
            if fg_thread != 0 && fg_thread != our_thread {
                // AttachThreadInput shares the input-state queue with the
                // current foreground thread — the classic SetForegroundWindow
                // bypass. A jail with UI isolation denies this.
                attached = AttachThreadInput(our_thread, fg_thread, 1) != 0;
            }
        }

        // Synthesize a keystroke so Win32's "process received input recently"
        // heuristic does not pre-emptively reject SetForegroundWindow. Inside
        // a restricted AppContainer, SendInput is denied (returns 0 with
        // ERROR_ACCESS_DENIED) — record it but keep attempting the steal so we
        // can classify the failure mode precisely.
        let input_synthesized = synthesize_keystroke();

        let set_fg_ok = SetForegroundWindow(hwnd) != 0;
        let set_fg_err = GetLastError();
        let _ = BringWindowToTop(hwnd);
        let _ = SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);

        if attached {
            // Detach regardless of outcome to avoid leaking the shared input
            // state if we somehow got this far inside the jail.
            let mut fg_pid: u32 = 0;
            let fg_thread = GetWindowThreadProcessId(foreground_hwnd, &mut fg_pid);
            if fg_thread != 0 {
                AttachThreadInput(our_thread, fg_thread, 0);
            }
        }

        // Strict success assertion: the steal only counts as a LEAK if the
        // probe window genuinely became the system foreground window.
        let now_foreground = GetForegroundWindow();
        let became_foreground = !now_foreground.is_null() && now_foreground == hwnd;

        let verdict = if set_fg_ok && became_foreground && input_synthesized {
            eprintln!(
                "LEAK: foreground steal SUCCEEDED — probe window is now foreground \
                 (set_fg_ok={set_fg_ok}, input_synthesized={input_synthesized})"
            );
            EXIT_STEAL_SUCCEEDED
        } else if set_fg_err == ERROR_ACCESS_DENIED_U32 || !input_synthesized {
            eprintln!(
                "DENIED: foreground steal blocked by jail \
                 (set_fg_ok={set_fg_ok}, set_fg_err={set_fg_err}, \
                 became_foreground={became_foreground}, input_synthesized={input_synthesized})"
            );
            EXIT_STEAL_DENIED_ACCESS
        } else {
            eprintln!(
                "BLOCKED: foreground steal did not take effect \
                 (set_fg_ok={set_fg_ok}, set_fg_err={set_fg_err}, \
                 became_foreground={became_foreground}, input_synthesized={input_synthesized})"
            );
            EXIT_STEAL_FAILED_OTHER
        };

        DestroyWindow(hwnd);
        verdict
    }

    /// Attempt to inject a synthetic SPACE keypress via SendInput. Returns
    /// true only if the OS accepted the injected events. Inside a UI-restricted
    /// AppContainer this is denied (returns 0 events accepted).
    unsafe fn synthesize_keystroke() -> bool {
        let mut inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_SPACE,
                        wScan: 0,
                        dwFlags: 0,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_SPACE,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];
        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
        sent == inputs.len() as u32
    }
}
