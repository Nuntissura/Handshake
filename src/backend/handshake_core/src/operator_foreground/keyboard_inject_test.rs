use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use thiserror::Error;

pub const LLKHF_INJECTED_FLAG: u32 = 0x0000_0010;
pub const LIVE_PROBE_ENV: &str = "HANDSHAKE_RUN_KEYBOARD_INJECT_LIVE";

static INJECTED_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInjectionCounters {
    pub injected_event_count: u64,
    pub total_event_count: u64,
}

pub fn record_keyboard_hook_flags(flags: u32) -> KeyboardInjectionCounters {
    TOTAL_EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
    if flags & LLKHF_INJECTED_FLAG != 0 {
        INJECTED_EVENT_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    keyboard_injection_counters()
}

pub fn keyboard_injection_counters() -> KeyboardInjectionCounters {
    KeyboardInjectionCounters {
        injected_event_count: INJECTED_EVENT_COUNT.load(Ordering::SeqCst),
        total_event_count: TOTAL_EVENT_COUNT.load(Ordering::SeqCst),
    }
}

pub fn reset_keyboard_injection_counters() {
    INJECTED_EVENT_COUNT.store(0, Ordering::SeqCst);
    TOTAL_EVENT_COUNT.store(0, Ordering::SeqCst);
}

#[derive(Debug, Clone, Default)]
pub struct CmdTracker {
    invocation_count: Arc<AtomicU64>,
}

impl CmdTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_invocation(&self) {
        self.invocation_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn invocation_count(&self) -> u64 {
        self.invocation_count.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MutationSentinel {
    mutated: Arc<AtomicBool>,
}

impl MutationSentinel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_mutated(&self) {
        self.mutated.store(true, Ordering::SeqCst);
    }

    pub fn state_mutated(&self) -> bool {
        self.mutated.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInjectionProbeReport {
    pub injected_event_count: u64,
    pub command_invocation_count: u64,
    pub state_mutated: bool,
}

pub fn assert_keyboard_injection_negative(
    report: &KeyboardInjectionProbeReport,
) -> Result<(), KeyboardInjectionError> {
    if report.injected_event_count == 0 {
        return Err(KeyboardInjectionError::MissingInjectedEvent);
    }
    if report.command_invocation_count != 0 {
        return Err(KeyboardInjectionError::CommandFired {
            count: report.command_invocation_count,
        });
    }
    if report.state_mutated {
        return Err(KeyboardInjectionError::StateMutated);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VkStroke {
    pub vk: u16,
    pub key_up: bool,
}

impl VkStroke {
    pub const fn down(vk: u16) -> Self {
        Self { vk, key_up: false }
    }

    pub const fn up(vk: u16) -> Self {
        Self { vk, key_up: true }
    }
}

pub fn ctrl_shift_f_sequence() -> [VkStroke; 6] {
    [
        VkStroke::down(0x11),
        VkStroke::down(0x10),
        VkStroke::down(0x46),
        VkStroke::up(0x46),
        VkStroke::up(0x10),
        VkStroke::up(0x11),
    ]
}

pub fn keyboard_injection_live_probe_requires_explicit_env(
    tracker: &CmdTracker,
    sentinel: &MutationSentinel,
) -> Result<KeyboardInjectionProbeReport, KeyboardInjectionError> {
    if std::env::var(LIVE_PROBE_ENV).as_deref() != Ok("1") {
        return Err(KeyboardInjectionError::LiveProbeDisabled);
    }
    reset_keyboard_injection_counters();
    let _hook = platform::install_ll_hook()?;
    platform::simulate_shortcut_injection(&ctrl_shift_f_sequence())?;
    platform::pump_hook_messages(Duration::from_millis(100));
    let counters = keyboard_injection_counters();
    let report = KeyboardInjectionProbeReport {
        injected_event_count: counters.injected_event_count,
        command_invocation_count: tracker.invocation_count(),
        state_mutated: sentinel.state_mutated(),
    };
    assert_keyboard_injection_negative(&report)?;
    Ok(report)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum KeyboardInjectionError {
    #[error("LL hook did not observe injected input")]
    MissingInjectedEvent,
    #[error("Tauri command handler fired during injected-keyboard probe: {count}")]
    CommandFired { count: u64 },
    #[error("state mutation occurred during injected-keyboard probe")]
    StateMutated,
    #[error("live keyboard injection probe disabled; set HANDSHAKE_RUN_KEYBOARD_INJECT_LIVE=1 on Windows desktop")]
    LiveProbeDisabled,
    #[error("keyboard injection probe unsupported on this platform")]
    UnsupportedPlatform,
    #[error("Windows hook install failed: {0}")]
    HookInstall(String),
    #[error("SendInput inserted {sent} of {expected} input events")]
    SendInputPartial { sent: u32, expected: u32 },
}

#[cfg(windows)]
mod platform {
    use std::{mem::size_of, time::Instant};

    use windows::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
                VIRTUAL_KEY,
            },
            WindowsAndMessaging::{
                CallNextHookEx, DispatchMessageW, PeekMessageW, SetWindowsHookExW,
                TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
                PM_REMOVE, WH_KEYBOARD_LL,
            },
        },
    };

    use super::{record_keyboard_hook_flags, KeyboardInjectionError, VkStroke};

    pub struct LlHookHandle {
        hook: windows::Win32::UI::WindowsAndMessaging::HHOOK,
    }

    impl Drop for LlHookHandle {
        fn drop(&mut self) {
            let _ = unsafe { UnhookWindowsHookEx(self.hook) };
        }
    }

    pub fn install_ll_hook() -> Result<LlHookHandle, KeyboardInjectionError> {
        let hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0) }
                .map_err(|error| KeyboardInjectionError::HookInstall(error.to_string()))?;
        Ok(LlHookHandle { hook })
    }

    pub fn simulate_shortcut_injection(seq: &[VkStroke]) -> Result<u32, KeyboardInjectionError> {
        let inputs: Vec<INPUT> = seq.iter().map(input_from_stroke).collect();
        let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            return Err(KeyboardInjectionError::SendInputPartial {
                sent,
                expected: inputs.len() as u32,
            });
        }
        Ok(sent)
    }

    pub fn pump_hook_messages(duration: std::time::Duration) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
                let _ = unsafe { TranslateMessage(&message) };
                unsafe {
                    DispatchMessageW(&message);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn input_from_stroke(stroke: &VkStroke) -> INPUT {
        let mut input = INPUT::default();
        input.r#type = INPUT_KEYBOARD;
        let flags = if stroke.key_up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        };
        input.Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(stroke.vk),
            wScan: 0,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: 0,
        };
        input
    }

    unsafe extern "system" fn low_level_keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code >= 0 {
            let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if event.flags.contains(LLKHF_INJECTED) {
                record_keyboard_hook_flags(LLKHF_INJECTED.0);
            } else {
                record_keyboard_hook_flags(event.flags.0);
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{KeyboardInjectionError, VkStroke};

    pub struct LlHookHandle;

    pub fn install_ll_hook() -> Result<LlHookHandle, KeyboardInjectionError> {
        Err(KeyboardInjectionError::UnsupportedPlatform)
    }

    pub fn simulate_shortcut_injection(_seq: &[VkStroke]) -> Result<u32, KeyboardInjectionError> {
        Err(KeyboardInjectionError::UnsupportedPlatform)
    }

    pub fn pump_hook_messages(_duration: std::time::Duration) {}
}

pub use platform::{install_ll_hook, simulate_shortcut_injection, LlHookHandle};
