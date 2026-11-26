//! The operating system's clipboard under KEEP_PLAIN_TEXT_OR_NOTHING (MT-214 E3-F2, E3-F3, E4-F3), on a window station
//! made fresh for the test, never the logon session's shared one (E5-F1-S), taken in turn with every other clipboard
//! test of the session and emptied at the end (E4-F1); nothing is written before its clipboard opened, and otherwise the
//! test stops BLOCKED (E5-F1, tests/support). The Operator's clipboard is never opened, and the test fails unless its
//! sequence number stayed the same. Ignored by default: the proof runs at the MT-213 x64 gate with the command in the
//! README. Another thread stands for another program: it puts text
//! there, or promises formats and renders one only when the clipboard asks it to, and counts each time it is asked. A
//! write must make it render nothing but promised plain text, and only when the clipboard holds nothing else; a
//! program that hangs instead makes the write busy within a quarter of a second, never makes the loop wait. A
//! withdrawal takes back only this process's own write. A process has one loop, so this is a target of its own.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handshake_shell_window::{
    ClipboardError, CommandSource, Control, Event, InputStamp, LogicalSize, LoopConfig,
    PhysicalPosition, Placement, PressState, WindowClass, WindowCommand, WindowEvent, WindowKey,
    WindowSpec,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW, PostMessageW,
    RegisterClassW, SetWindowPos, HWND_TOP, MSG, PM_NOREMOVE, PM_REMOVE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, WM_KEYDOWN, WM_RENDERFORMAT, WNDCLASSW, WS_POPUP,
};

use support::{
    assert_clipboard_is_private, clipboard_is_private, on_private_window_station, run_script,
    spawn_on_this_desktop, wide,
};

const TEXT: u32 = CF_UNICODETEXT as u32;

/// How long the other program hangs in the E4-F3 step. The write may wait a quarter of a second at most.
const HANG: Duration = Duration::from_secs(3);

/// What the other program puts on the clipboard.
#[derive(Clone, Copy, Debug)]
enum Put {
    /// Plain text, rendered at once.
    Text(&'static str),
    /// Plain text promised, with a promised format of its own or alone.
    Promised { rich: bool },
}

enum Ask {
    Put(Put),
    /// Look at no message for this long, as a hung program, from right after the answer.
    Hang(Duration),
    /// How often the clipboard asked it to render a promised format so far.
    Renders,
    Quit,
}

thread_local! {
    static RENDERS: Cell<u32> = const { Cell::new(0) };
}

fn registered(name: &str) -> u32 {
    let name = wide(name);
    // SAFETY: the name outlives the call. Registered in the test's window station.
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    assert_ne!(format, 0);
    format
}

/// Global memory with `text` and its ending 0.
fn memory(text: &str) -> isize {
    let units: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let size = std::mem::size_of_val(&units[..]);
    // SAFETY: plain values; then as many bytes as were allocated, into memory that is ours alone.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, size);
        assert!(!memory.is_null());
        let start = GlobalLock(memory).cast::<u16>();
        assert!(!start.is_null());
        ptr::copy_nonoverlapping(units.as_ptr(), start, units.len());
        GlobalUnlock(memory);
        memory as isize
    }
}

unsafe extern "system" fn other_program(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Only on the test's fresh window station; a window procedure must not panic.
    if message == WM_RENDERFORMAT && clipboard_is_private() {
        RENDERS.set(RENDERS.get() + 1);
        // Rendered as Delayed Rendering says: no OpenClipboard, the clipboard is held open by who asked.
        let data = if wparam as u32 == TEXT { memory("promised") } else { memory("rich") };
        // SAFETY: the format asked for, and memory the clipboard takes.
        unsafe { SetClipboardData(wparam as u32, data) };
        return 0;
    }
    // SAFETY: the message as it came.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// The other program: a thread with a window of its own on the test's desktop.
struct Other {
    ask: Sender<Ask>,
    answer: Receiver<u32>,
    thread: Option<JoinHandle<()>>,
}

impl Other {
    fn start() -> Self {
        let (ask, asked) = channel::<Ask>();
        let (answered, answer) = channel();
        let thread = spawn_on_this_desktop(move || {
            let class = wide("hsk_test_other_program");
            // SAFETY: the class name outlives the calls; a hidden window of this thread.
            let hwnd = unsafe {
                let mut description: WNDCLASSW = std::mem::zeroed();
                description.lpfnWndProc = Some(other_program);
                description.hInstance = GetModuleHandleW(ptr::null());
                description.lpszClassName = class.as_ptr();
                RegisterClassW(&description);
                CreateWindowExW(0, class.as_ptr(), ptr::null(), WS_POPUP, 0, 0, 10, 10, 0, 0, 0, ptr::null())
            };
            assert_ne!(hwnd, 0);
            let rich = registered("HSK test rich");
            loop {
                match asked.try_recv() {
                    Ok(Ask::Put(put)) => {
                        Self::put(hwnd, put, rich);
                        let _ = answered.send(0);
                    }
                    Ok(Ask::Hang(hang)) => {
                        let _ = answered.send(0);
                        // What the clipboard sends meanwhile waits; it is handled below, before the next question.
                        std::thread::sleep(hang);
                    }
                    Ok(Ask::Renders) => {
                        let _ = answered.send(RENDERS.get());
                    }
                    Ok(Ask::Quit) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {}
                }
                // SAFETY: a structure of this function; what the clipboard sends is handled.
                unsafe {
                    let mut message: MSG = std::mem::zeroed();
                    while PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) != 0 {
                        DispatchMessageW(&message);
                    }
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            // SAFETY: created above, on this thread.
            unsafe { DestroyWindow(hwnd) };
        });
        Self {
            ask,
            answer,
            thread: Some(thread),
        }
    }

    fn put(hwnd: HWND, put: Put, rich: u32) {
        assert_clipboard_is_private();
        // SAFETY: the clipboard of the test's window station, opened by this thread's window and closed below.
        unsafe {
            let until = Instant::now() + Duration::from_secs(5);
            while OpenClipboard(hwnd) == 0 {
                assert!(Instant::now() < until, "the clipboard stays open");
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_ne!(EmptyClipboard(), 0);
            match put {
                Put::Text(text) => assert_ne!(SetClipboardData(TEXT, memory(text)), 0),
                Put::Promised { rich: with_rich } => {
                    SetClipboardData(TEXT, 0);
                    if with_rich {
                        SetClipboardData(rich, 0);
                    }
                }
            }
            CloseClipboard();
        }
    }

    /// Ask, and wait for the answer while this thread handles what the clipboard sends it.
    fn ask(&self, ask: Ask) -> u32 {
        self.ask.send(ask).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            match self.answer.try_recv() {
                Ok(answer) => return answer,
                Err(TryRecvError::Empty) => assert!(Instant::now() < until, "the other program answers"),
                Err(TryRecvError::Disconnected) => panic!("the other program ended"),
            }
            // SAFETY: a structure of this function; nothing is taken out, sent messages are handled.
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                PeekMessageW(&mut message, 0, 0, 0, PM_NOREMOVE);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn renders(&self) -> u32 {
        self.ask(Ask::Renders)
    }
}

impl Drop for Other {
    fn drop(&mut self) {
        let _ = self.ask.send(Ask::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Default)]
struct State {
    other: RefCell<Option<Other>>,
    window: Cell<Option<WindowKey>>,
    hwnd: Cell<HWND>,
    step: Cell<u64>,
}

impl State {
    fn other(&self) -> std::cell::Ref<'_, Other> {
        std::cell::Ref::map(self.other.borrow(), |other| other.as_ref().unwrap())
    }

    /// A key down, as another program posts it: its stamp lets the host write while a window of ours is in front.
    fn post_key(&self) {
        // SAFETY: plain values, to a window of this thread.
        assert_ne!(unsafe { PostMessageW(self.hwnd.get(), WM_KEYDOWN, 0x41, 0x001E_0001) }, 0);
    }

    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                assert_clipboard_is_private();
                *self.other.borrow_mut() = Some(Other::start());
                let spec = WindowSpec {
                    title: "test".to_owned(),
                    inner: LogicalSize::new(200.0, 100.0),
                    min_inner: None,
                    placement: Placement::At(PhysicalPosition::new(40, 40)),
                    class: WindowClass::Operator,
                };
                let key = control.create_window(&CommandSource::host(1), spec).unwrap();
                self.window.set(Some(key));
            }
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => {
                self.hwnd.set(control.native_window(window).unwrap().win32().hwnd.get());
                let show = WindowCommand::Show { activate: false };
                control.command(&CommandSource::host(2), window, show).unwrap();
                let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;
                // SAFETY: plain values, for a window of this thread on the test's desktop; activates it, as the
                // Operator's click would.
                unsafe { SetWindowPos(self.hwnd.get(), HWND_TOP, 0, 0, 0, 0, flags) };
                // SAFETY: no arguments.
                assert_eq!(unsafe { GetActiveWindow() }, self.hwnd.get());
                self.post_key();
            }
            Event::Window {
                event: WindowEvent::Key(key),
                ..
            } if key.state == PressState::Pressed => self.step(control, &key.stamp),
            Event::Window {
                event: WindowEvent::Destroyed,
                ..
            } => {
                self.other.borrow_mut().take();
                control.begin_shutdown().unwrap();
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            _ => {}
        }
    }

    fn write(&self, control: &Control, stamp: &InputStamp, text: &str) {
        assert_clipboard_is_private();
        let source = CommandSource::from_input(stamp, 100 + self.step.get());
        assert_eq!(control.write_clipboard(&source, text), Ok(()), "step {}", self.step.get());
        assert_eq!(control.read_clipboard(), Ok(Some(text.to_owned())));
    }

    fn step(&self, control: &Control, stamp: &InputStamp) {
        self.step.set(self.step.get() + 1);
        assert_clipboard_is_private();
        let other = self.other();
        let before = other.renders();
        let put = |put| other.ask(Ask::Put(put));
        match self.step.get() {
            1 => {
                // Plain text, rendered: kept for the swap, and nothing is asked of the other program.
                put(Put::Text("old"));
                self.write(control, stamp, "one");
                assert_eq!(other.renders(), before);
            }
            2 => {
                // A promised format of its own beside promised text: the write asks for neither. A marker of the
                // clipboard history is such a format too (MT-214 E4-F4): the unit tests list it.
                put(Put::Promised { rich: true });
                self.write(control, stamp, "two");
                assert_eq!(other.renders(), before, "no promised format rendered for a write");
            }
            3 => {
                // The one exception: promised plain text, alone, is asked for once, to be kept.
                put(Put::Promised { rich: false });
                self.write(control, stamp, "three");
                assert_eq!(other.renders(), before + 1);
            }
            4 => {
                // MT-214 E4-F3: promised plain text alone, and its program hangs. Its text is asked for on a thread of
                // the write's own, which waits for it a quarter of a second at most: the write is busy, and so is the
                // next while that thread still holds the clipboard open; the loop never waits for the program.
                put(Put::Promised { rich: false });
                other.ask(Ask::Hang(HANG));
                let source = CommandSource::from_input(stamp, 100 + self.step.get());
                let started = Instant::now();
                assert_eq!(control.write_clipboard(&source, "four"), Err(ClipboardError::Busy));
                assert_eq!(control.write_clipboard(&source, "four"), Err(ClipboardError::Busy));
                let waited = started.elapsed();
                assert!(waited < HANG / 3, "the write waited for the hung program: {waited:?}");
                // Once the program answers, it renders once and the copy ends; then a write goes through.
                assert_eq!(other.renders(), before + 1);
                let until = Instant::now() + Duration::from_secs(5);
                loop {
                    match control.write_clipboard(&source, "four") {
                        Ok(()) => break,
                        Err(ClipboardError::Busy) if Instant::now() < until => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        other => panic!("after the program answered: {other:?}"),
                    }
                }
                assert_eq!(control.read_clipboard(), Ok(Some("four".to_owned())));
                assert_eq!(other.renders(), before + 1, "rendered once");
            }
            5 => {
                // MT-214 E3-F3: this process's own write is withdrawn; another program's is not.
                assert_eq!(control.withdraw_clipboard(), Ok(true));
                assert_eq!(control.read_clipboard(), Ok(None));
                assert_eq!(control.withdraw_clipboard(), Ok(false), "once");
                put(Put::Text("the operator's"));
                assert_eq!(control.withdraw_clipboard(), Ok(false));
                assert_eq!(control.read_clipboard(), Ok(Some("the operator's".to_owned())));
                let window = self.window.get().unwrap();
                control.command(&CommandSource::host(3), window, WindowCommand::Destroy).unwrap();
                return;
            }
            other => panic!("step {other}"),
        }
        self.post_key();
    }
}

#[test]
#[ignore = "real-OS clipboard proof runs at the MT-213 x64 gate; private-station preflight blocked on the ARM64 dev machine"]
fn a_write_asks_no_program_to_render_anything_but_plain_text_and_withdraws_only_its_own() {
    on_private_window_station(Duration::from_secs(60), || {
        let state = Rc::new(State::default());
        let events = Rc::clone(&state);
        let report = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(state.step.get(), 5, "every step ran");
        assert_eq!((report.windows_created, report.windows_destroyed), (1, 1));
    });
}
