//! The operating system's clipboard, as text: KEEP_PLAIN_TEXT_OR_NOTHING (`crate::clipboard`, MT-214 E3-F2, E3-F3).
//!
//! A write has to empty the clipboard before it puts its text there: `EmptyClipboard` "assigns clipboard ownership to
//! the window that has the clipboard open", and only then may that window put data on it (Clipboard Operations, "Cut
//! and Copy Operations"). Which of the old content it keeps, to put back should the put be refused (R2-N6), is the
//! policy: [`keep`] lists the formats (`EnumClipboardFormats` renders nothing) and asks for data only when everything
//! there is plain text, and then for `CF_UNICODETEXT` alone.
//!
//! That one request may make another program render text it promised: the system sends it `WM_RENDERFORMAT` and the
//! request waits for it (Clipboard Operations, "Delayed Rendering"), and Microsoft's documentation of
//! `GetClipboardData`, `WM_RENDERFORMAT` and Delayed Rendering names no time after which it gives up. So the copy is
//! made on a thread of its own, on the loop thread's desktop, and the write waits for it at most [`KEEP_WAIT`]: a
//! program that does not render in time makes the write [`ClipboardError::Busy`], and never makes the loop wait
//! (MT-214 E4-F3). One copy runs at a time; while one still runs it holds the clipboard open, and writes are busy.
//!
//! A withdrawal empties the clipboard only while it holds this window's own last write: its owner is this window
//! ("A window becomes the clipboard owner when it ... calls the EmptyClipboard function. The window remains the
//! clipboard owner until it is closed or another window empties the clipboard"), and the last write was not refused,
//! since a refused one put the Operator's text back as this window's.

use std::marker::PhantomData;
use std::ptr;
use std::slice;
use std::sync::mpsc::{sync_channel, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::Duration;

use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardOwner,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows_sys::Win32::System::Ole::{CF_LOCALE, CF_OEMTEXT, CF_TEXT, CF_UNICODETEXT};
use windows_sys::Win32::System::StationsAndDesktops::{GetThreadDesktop, SetThreadDesktop, HDESK};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use super::last_error;
use crate::clipboard::{ClipboardError, TextClipboard, KEEP_MAX_BYTES, MAX_CLIPBOARD_BYTES};

const FORMAT: u32 = CF_UNICODETEXT as u32;

/// One UTF-16 unit is at most three bytes of UTF-8, and at least one.
const MAX_UNITS: usize = MAX_CLIPBOARD_BYTES;

/// The most formats looked at on one clipboard. One that lists more keeps nothing.
const MAX_FORMATS: usize = 1_024;

/// The formats of plain text, as the clipboard lists them: the one placed, and those the system makes from it.
const PLAIN_TEXT: [u32; 4] = [FORMAT, CF_TEXT as u32, CF_OEMTEXT as u32, CF_LOCALE as u32];

/// How long a write waits for the copy of what it keeps (E4-F3). Text already on the clipboard is copied in far less;
/// the rest is the program that promised it, rendering on request (Microsoft's estimate of the overhead of a render on
/// request is 10 to 100 microseconds, Clipboard Operations, "Delayed Rendering Guidance").
pub(super) const KEEP_WAIT: Duration = Duration::from_millis(250);

/// What a write does when it keeps nothing of a clipboard that holds something (`crate::clipboard`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unkept {
    /// Write all the same; a refused put then leaves the clipboard empty ([`ClipboardError::NotRestored`]).
    ReplaceAnyway,
    /// Do not write ([`ClipboardError::CannotKeep`]).
    LeaveAsIs,
}

/// KEEP_PLAIN_TEXT_OR_NOTHING.
const UNKEPT: Unkept = Unkept::ReplaceAnyway;

pub(super) struct SystemClipboard {
    /// Owns what is put on the clipboard. Never shown.
    owner: HWND,
    last: LastWrite,
    /// The thread of a copy of what a write keeps that did not end within [`KEEP_WAIT`]: it holds the clipboard open.
    keeper: Option<JoinHandle<()>>,
}

/// A copy of what a write keeps, made on a thread of its own (E4-F3), and the clipboard's sequence number then.
struct Keeping {
    kept: Result<Vec<(u32, Held)>, Skipped>,
    sequence: u32,
}

impl SystemClipboard {
    pub(super) fn new(owner: HWND) -> Self {
        Self {
            owner,
            last: LastWrite::default(),
            keeper: None,
        }
    }

    /// What a write keeps, copied on a thread of its own on this thread's desktop; waits at most [`KEEP_WAIT`]. A copy
    /// that runs on keeps the clipboard open, so the write is [`ClipboardError::Busy`], and so is every write until it
    /// ended. Nobody waits for its answer any more: its thread wipes and frees the copy when it ends.
    fn keep_within_bound(&mut self) -> Result<Keeping, ClipboardError> {
        if let Some(keeper) = self.keeper.take() {
            if !keeper.is_finished() {
                self.keeper = Some(keeper);
                return Err(ClipboardError::Busy);
            }
            let _ = keeper.join();
        }
        // SAFETY: no arguments, then this thread's own id. The handle need not be closed (GetThreadDesktop).
        let desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
        let (answered, answer) = sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("hsk-clipboard-keep".to_owned())
            .spawn(move || {
                let _ = answered.send(copy_to_keep(desktop));
            })
            .map_err(|_| ClipboardError::Busy)?;
        match answer.recv_timeout(KEEP_WAIT) {
            Ok(Some(keeping)) => {
                let _ = thread.join();
                Ok(keeping)
            }
            Ok(None) | Err(RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
                Err(ClipboardError::Busy)
            }
            Err(RecvTimeoutError::Timeout) => {
                self.keeper = Some(thread);
                Err(ClipboardError::Busy)
            }
        }
    }

    fn open(&self) -> Result<OpenedClipboard, ClipboardError> {
        // SAFETY: a plain value. Fails at once while another program has it open.
        if unsafe { OpenClipboard(self.owner) } == 0 {
            Err(ClipboardError::Busy)
        } else {
            Ok(OpenedClipboard)
        }
    }

    fn owns(&self) -> bool {
        // SAFETY: no arguments.
        unsafe { GetClipboardOwner() == self.owner }
    }
}

/// Whether what is on the clipboard is still this window's own last write (E3-F3).
#[derive(Debug, Default)]
struct LastWrite {
    ours: bool,
}

impl LastWrite {
    fn wrote(&mut self, written: &Result<(), ClipboardError>) {
        self.ours = written.is_ok();
    }

    /// `owns`: the clipboard's owner is this window. Asked once more after it opened the clipboard.
    fn may_withdraw(&mut self, owns: bool) -> bool {
        self.ours &= owns;
        self.ours
    }

    fn withdrawn(&mut self) {
        self.ours = false;
    }
}

/// Closes the clipboard.
struct OpenedClipboard;

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        // SAFETY: this thread opened it.
        unsafe { CloseClipboard() };
    }
}

/// Unlocks global memory.
struct Locked {
    memory: HGLOBAL,
    start: *mut u8,
}

impl Drop for Locked {
    fn drop(&mut self) {
        // SAFETY: locked by whoever made this.
        unsafe { GlobalUnlock(self.memory) };
    }
}

fn lock(memory: HGLOBAL) -> Result<Locked, ClipboardError> {
    // SAFETY: a plain value; null when it is no movable memory.
    let start = unsafe { GlobalLock(memory) }.cast::<u8>();
    if start.is_null() {
        return Err(ClipboardError::System(last_error()));
    }
    Ok(Locked { memory, start })
}

impl TextClipboard for SystemClipboard {
    fn read(&mut self) -> Result<Option<String>, ClipboardError> {
        // SAFETY: a plain value, read only.
        if unsafe { IsClipboardFormatAvailable(FORMAT) } == 0 {
            return Ok(None);
        }
        let _opened = self.open()?;
        // SAFETY: the clipboard is open. The memory stays the clipboard's, and valid until it is closed.
        let memory = unsafe { GetClipboardData(FORMAT) };
        if memory == 0 {
            return Ok(None);
        }
        let memory = memory as HGLOBAL;
        let locked = lock(memory)?;
        // SAFETY: a plain value.
        let room = unsafe { GlobalSize(memory) } / size_of::<u16>();
        // SAFETY: `room` units are readable while the memory is locked, and a lock is aligned for any value. Another
        // program wrote them; every bit pattern is a u16.
        let units = unsafe { slice::from_raw_parts(locked.start.cast::<u16>(), room) };
        let text = units.split(|unit| *unit == 0).next().unwrap_or(&[]);
        if text.len() > MAX_UNITS {
            return Err(ClipboardError::TooLarge);
        }
        let text = String::from_utf16_lossy(text);
        if text.len() > MAX_CLIPBOARD_BYTES {
            return Err(ClipboardError::TooLarge);
        }
        Ok(Some(text))
    }

    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        if text.len() > MAX_CLIPBOARD_BYTES {
            return Err(ClipboardError::TooLarge);
        }
        let units: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        // Before the clipboard is opened, so it is open for the swap only.
        let data = text_memory(&units)?;
        let keeping = self.keep_within_bound()?;
        let opened = self.open()?;
        // The copy is of what is there only if nothing changed the clipboard since it was made.
        // SAFETY: no arguments.
        let unchanged = unsafe { GetClipboardSequenceNumber() } == keeping.sequence;
        let kept = if unchanged {
            keeping.kept
        } else {
            Err(Skipped::Changed)
        };
        let written = replace(&mut SystemBoard::of(&opened), FORMAT, data, UNKEPT, kept);
        drop(opened);
        self.last.wrote(&written);
        written
    }

    fn withdraw(&mut self) -> Result<bool, ClipboardError> {
        if !self.last.may_withdraw(self.owns()) {
            return Ok(false);
        }
        let _opened = self.open()?;
        // Opening it does not change who owns what is on it.
        if !self.last.may_withdraw(self.owns()) {
            return Ok(false);
        }
        // SAFETY: the clipboard is open, by this window.
        if unsafe { EmptyClipboard() } == 0 {
            return Err(ClipboardError::System(last_error()));
        }
        self.last.withdrawn();
        Ok(true)
    }
}

fn text_memory(units: &[u16]) -> Result<Held, ClipboardError> {
    let size = std::mem::size_of_val(units);
    // SAFETY: plain values.
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) };
    if memory.is_null() {
        return Err(ClipboardError::System(last_error()));
    }
    let held = Held(Some(memory));
    let locked = lock(memory)?;
    // SAFETY: as many bytes as were allocated, and the memory is ours alone.
    unsafe { ptr::copy_nonoverlapping(units.as_ptr().cast::<u8>(), locked.start, size) };
    drop(locked);
    Ok(held)
}

/// A copy of one format.
#[derive(Debug, PartialEq, Eq)]
enum Copied<D> {
    Data(D),
    /// More than the limit; not copied.
    TooLarge,
    /// It holds nothing that could be read, or the copy failed.
    Unreadable,
}

/// Why a write keeps nothing of what is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Skipped {
    /// A format that is not plain text, or more formats than are looked at. The registered formats with which a
    /// program keeps what it copies from the clipboard history and the cloud clipboard are among them (Clipboard Formats,
    /// "Cloud Clipboard and Clipboard History Formats"), so a password manager's entry is never kept (E4-F4).
    NotPlainText,
    TooLarge,
    Unreadable,
    /// The clipboard changed after the copy was made.
    Changed,
}

/// What a write needs of the open clipboard. Tests give their own.
trait Board {
    type Data;

    /// The formats on it, in their order, at most `most` of them.
    fn formats(&self, most: usize) -> Vec<u32>;

    /// A copy of its `CF_UNICODETEXT`, when that is at most `limit` bytes.
    fn copy_text(&self, limit: usize) -> Copied<Self::Data>;

    /// Returns the error number when it failed.
    fn empty(&mut self) -> Result<(), u32>;

    /// Gives the data back, with the error number, when the system did not take it.
    fn put(&mut self, format: u32, data: Self::Data) -> Result<(), (Self::Data, u32)>;
}

/// Put `data` on the board in place of what is there, of which `kept` is what [`keep`] copied.
fn replace<B: Board>(
    board: &mut B,
    format: u32,
    data: B::Data,
    unkept: Unkept,
    kept: Result<Vec<(u32, B::Data)>, Skipped>,
) -> Result<(), ClipboardError> {
    if kept.is_err() && unkept == Unkept::LeaveAsIs {
        return Err(ClipboardError::CannotKeep);
    }
    board.empty().map_err(ClipboardError::System)?;
    let Err((refused, error)) = board.put(format, data) else {
        return Ok(());
    };
    drop(refused);
    let Ok(kept) = kept else {
        return Err(ClipboardError::NotRestored(error));
    };
    for (format, data) in kept {
        // What the system does not take back is freed here.
        let _ = board.put(format, data);
    }
    Err(ClipboardError::System(error))
}

/// What of the board a write keeps: nothing of an empty one, plain text when there is nothing else (the module's
/// policy), and otherwise why it keeps nothing. Asks for no data but plain text.
fn keep<B: Board>(board: &B) -> Result<Vec<(u32, B::Data)>, Skipped> {
    let formats = board.formats(MAX_FORMATS);
    if formats.is_empty() {
        return Ok(Vec::new());
    }
    let plain = formats.iter().all(|format| PLAIN_TEXT.contains(format));
    if !plain || formats.len() >= MAX_FORMATS {
        return Err(Skipped::NotPlainText);
    }
    match board.copy_text(KEEP_MAX_BYTES) {
        Copied::Data(data) => Ok(vec![(FORMAT, data)]),
        Copied::TooLarge => Err(Skipped::TooLarge),
        Copied::Unreadable => Err(Skipped::Unreadable),
    }
}

/// The copy of what a write keeps, made on the thread of [`SystemClipboard::keep_within_bound`], which runs it on the
/// desktop of the loop's thread (`desktop`): the program that promised text is sent `WM_RENDERFORMAT`, and window
/// messages pass only between threads of one desktop (Desktops). `None` when another program has the clipboard open.
fn copy_to_keep(desktop: HDESK) -> Option<Keeping> {
    // SAFETY: a thread of its own that has no window or hook yet; the desktop is the loop thread's, of this process's
    // window station, and stays open while that thread runs.
    if unsafe { SetThreadDesktop(desktop) } == 0 {
        return Some(Keeping {
            kept: Err(Skipped::Unreadable),
            sequence: 0,
        });
    }
    // SAFETY: no window: the clipboard is read here, never emptied or written. Fails at once while it is open.
    if unsafe { OpenClipboard(0) } == 0 {
        return None;
    }
    let opened = OpenedClipboard;
    let kept = keep(&SystemBoard::of(&opened));
    // SAFETY: no arguments. Read while the clipboard is still open, so no other program changed it since the copy.
    let sequence = unsafe { GetClipboardSequenceNumber() };
    drop(opened);
    Some(Keeping { kept, sequence })
}

/// Clipboard data this process holds. Wiped and freed unless the clipboard takes it.
struct Held(Option<HGLOBAL>);

// SAFETY: global memory is the process's, valid on every thread; a `Held` is its one owner, and moves between threads
// only as a whole.
unsafe impl Send for Held {}

impl Drop for Held {
    fn drop(&mut self) {
        let Some(memory) = self.0.take() else { return };
        // SAFETY: made here, and the clipboard did not take it: no other reference to it exists.
        unsafe {
            let size = GlobalSize(memory);
            if let Ok(locked) = lock(memory) {
                ptr::write_bytes(locked.start, 0, size);
            }
            GlobalFree(memory);
        }
    }
}

/// The open clipboard of the operating system, for as long as it is open.
struct SystemBoard<'a>(PhantomData<&'a OpenedClipboard>);

impl<'a> SystemBoard<'a> {
    fn of(_: &'a OpenedClipboard) -> Self {
        Self(PhantomData)
    }
}

impl Board for SystemBoard<'_> {
    type Data = Held;

    fn formats(&self, most: usize) -> Vec<u32> {
        let mut formats = Vec::new();
        let mut format = 0;
        while formats.len() < most {
            // SAFETY: the clipboard is open; 0 after the last. Lists, renders nothing.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            formats.push(format);
        }
        formats
    }

    fn copy_text(&self, limit: usize) -> Copied<Held> {
        // SAFETY: the clipboard is open, by this thread; the handle stays the clipboard's while it is. Promised text is
        // rendered on request, which the caller bounds ([`KEEP_WAIT`]).
        let source = unsafe { GetClipboardData(FORMAT) } as HGLOBAL;
        if source.is_null() {
            return Copied::Unreadable;
        }
        // SAFETY: a plain value; 0 for a handle that is no global memory.
        let size = unsafe { GlobalSize(source) };
        if size == 0 {
            return Copied::Unreadable;
        }
        if size > limit {
            return Copied::TooLarge;
        }
        // SAFETY: plain values.
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) };
        if memory.is_null() {
            return Copied::Unreadable;
        }
        let held = Held(Some(memory));
        let (Ok(from), Ok(to)) = (lock(source), lock(memory)) else {
            return Copied::Unreadable;
        };
        // SAFETY: both blocks are `size` bytes long and locked, and they are not the same.
        unsafe { ptr::copy_nonoverlapping(from.start, to.start, size) };
        drop((from, to));
        Copied::Data(held)
    }

    fn empty(&mut self) -> Result<(), u32> {
        // SAFETY: the clipboard is open, by the owner window.
        if unsafe { EmptyClipboard() } == 0 {
            Err(last_error())
        } else {
            Ok(())
        }
    }

    fn put(&mut self, format: u32, mut data: Held) -> Result<(), (Held, u32)> {
        let handle = data.0.map_or(0, |memory| memory as isize);
        // SAFETY: the clipboard is open and owned by this window. It owns the data only when the call succeeds.
        if unsafe { SetClipboardData(format, handle) } == 0 {
            return Err((data, last_error()));
        }
        data.0 = None;
        Ok(())
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
