//! The operating system's clipboard, as text.
//!
//! A write has to empty the clipboard before it puts text there: only `EmptyClipboard` makes the window the owner, and
//! only the owner may put data on it (EmptyClipboard, SetClipboardData). So that a failed put does not leave the
//! Operator's clipboard empty (MT-214 R2-N6), the write first copies what is there, one format of each group the
//! system converts into each other, and puts it back when the put fails. What cannot be copied (owner-display, the
//! display forms of a private format, the private and GDI-object ranges) is lost only on such a failure; a clipboard
//! that holds nothing else is not touched at all ([`ClipboardError::CannotKeep`]). Copying asks a program that promised
//! a format and did not render it yet to render it now, as reading does.

use std::ptr;
use std::slice;

use windows_sys::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows_sys::Win32::Graphics::Gdi::{CopyEnhMetaFileW, DeleteEnhMetaFile, HENHMETAFILE};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardOwner,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows_sys::Win32::System::Ole::{
    CF_BITMAP, CF_DIB, CF_DIBV5, CF_DSPBITMAP, CF_DSPENHMETAFILE, CF_DSPMETAFILEPICT,
    CF_ENHMETAFILE, CF_GDIOBJFIRST, CF_GDIOBJLAST, CF_METAFILEPICT, CF_OEMTEXT, CF_OWNERDISPLAY,
    CF_PALETTE, CF_PRIVATEFIRST, CF_PRIVATELAST, CF_TEXT, CF_UNICODETEXT,
};

use super::last_error;
use crate::clipboard::{ClipboardError, TextClipboard, MAX_CLIPBOARD_BYTES};

const FORMAT: u32 = CF_UNICODETEXT as u32;

/// One UTF-16 unit is at most three bytes of UTF-8, and at least one.
const MAX_UNITS: usize = MAX_CLIPBOARD_BYTES;

/// The most formats looked at on one clipboard.
const MAX_FORMATS: usize = 1_024;

pub(super) struct SystemClipboard {
    /// Owns what is put on the clipboard. Never shown.
    owner: HWND,
}

impl SystemClipboard {
    pub(super) fn new(owner: HWND) -> Self {
        Self { owner }
    }

    fn open(&self) -> Result<OpenedClipboard, ClipboardError> {
        // SAFETY: a plain value. Fails at once while another program has it open.
        if unsafe { OpenClipboard(self.owner) } == 0 {
            Err(ClipboardError::Busy)
        } else {
            Ok(OpenedClipboard)
        }
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
struct Locked<'a> {
    memory: HGLOBAL,
    start: *mut u16,
    owner: std::marker::PhantomData<&'a OpenedClipboard>,
}

impl Drop for Locked<'_> {
    fn drop(&mut self) {
        // SAFETY: locked by whoever made this.
        unsafe { GlobalUnlock(self.memory) };
    }
}

fn lock(memory: HGLOBAL, _: &OpenedClipboard) -> Result<Locked<'_>, ClipboardError> {
    // SAFETY: a plain value; null when it is no movable memory.
    let start = unsafe { GlobalLock(memory) }.cast::<u16>();
    if start.is_null() {
        return Err(ClipboardError::System(last_error()));
    }
    Ok(Locked {
        memory,
        start,
        owner: std::marker::PhantomData,
    })
}

impl TextClipboard for SystemClipboard {
    fn read(&mut self) -> Result<Option<String>, ClipboardError> {
        // SAFETY: a plain value, read only.
        if unsafe { IsClipboardFormatAvailable(FORMAT) } == 0 {
            return Ok(None);
        }
        let opened = self.open()?;
        // SAFETY: the clipboard is open. The memory stays the clipboard's, and valid until it is closed.
        let memory = unsafe { GetClipboardData(FORMAT) };
        if memory == 0 {
            return Ok(None);
        }
        let memory = memory as HGLOBAL;
        let locked = lock(memory, &opened)?;
        // SAFETY: a plain value.
        let room = unsafe { GlobalSize(memory) } / size_of::<u16>();
        // SAFETY: `room` units are readable while the memory is locked. Another program wrote them; every bit pattern
        // is a u16.
        let units = unsafe { slice::from_raw_parts(locked.start, room) };
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
        let opened = self.open()?;
        let data = text_memory(&units, &opened)?;
        replace(&mut SystemBoard(&opened), FORMAT, data)
    }

    fn withdraw(&mut self) -> Result<bool, ClipboardError> {
        // SAFETY: no arguments.
        if unsafe { GetClipboardOwner() } != self.owner {
            return Ok(false);
        }
        let _opened = self.open()?;
        // SAFETY: as above; opening it does not change who owns what is on it.
        unsafe {
            if GetClipboardOwner() != self.owner {
                return Ok(false);
            }
            if EmptyClipboard() == 0 {
                return Err(ClipboardError::System(last_error()));
            }
        }
        Ok(true)
    }
}

fn text_memory(units: &[u16], opened: &OpenedClipboard) -> Result<Held, ClipboardError> {
    // SAFETY: plain values.
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of_val(units)) };
    if memory.is_null() {
        return Err(ClipboardError::System(last_error()));
    }
    let held = Held(Some(Handle::Memory(memory)));
    let locked = lock(memory, opened)?;
    // SAFETY: as many units as were allocated, and the memory is ours alone.
    unsafe { ptr::copy_nonoverlapping(units.as_ptr(), locked.start, units.len()) };
    drop(locked);
    Ok(held)
}

/// How a format can be kept (Standard Clipboard Formats).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Global memory, which the system frees when the clipboard is emptied: copied byte for byte.
    Memory,
    /// An enhanced metafile: copied with `CopyEnhMetaFileW`.
    Metafile,
    /// A handle that is no memory, which the system makes again from another format of its group.
    Twin,
    /// Neither: lost when a failed write puts back what was there.
    Lost,
}

fn kind(format: u32) -> Kind {
    let Ok(format) = u16::try_from(format) else {
        return Kind::Lost;
    };
    match format {
        CF_ENHMETAFILE => Kind::Metafile,
        CF_BITMAP | CF_METAFILEPICT | CF_PALETTE => Kind::Twin,
        CF_OWNERDISPLAY | CF_DSPBITMAP | CF_DSPMETAFILEPICT | CF_DSPENHMETAFILE => Kind::Lost,
        CF_PRIVATEFIRST..=CF_PRIVATELAST | CF_GDIOBJFIRST..=CF_GDIOBJLAST => Kind::Lost,
        _ => Kind::Memory,
    }
}

/// The groups of formats the system converts into each other (Synthesized Clipboard Formats); one of each is enough.
fn group(format: u32) -> Option<usize> {
    match u16::try_from(format).ok()? {
        CF_TEXT | CF_OEMTEXT | CF_UNICODETEXT => Some(0),
        CF_BITMAP | CF_DIB | CF_DIBV5 | CF_PALETTE => Some(1),
        CF_ENHMETAFILE | CF_METAFILEPICT => Some(2),
        _ => None,
    }
}

/// A copy of one format.
enum Copied<D> {
    Data(D),
    /// The format holds nothing.
    Nothing,
    /// It holds something that could not be copied.
    Failed,
}

/// What a write needs of the open clipboard. Tests give their own.
trait Board {
    type Data;

    /// The formats on it, in their order.
    fn formats(&self) -> Vec<u32>;

    fn copy(&self, format: u32, kind: Kind) -> Copied<Self::Data>;

    /// Returns the error number when it failed.
    fn empty(&mut self) -> Result<(), u32>;

    /// Gives the data back, with the error number, when the system did not take it.
    fn put(&mut self, format: u32, data: Self::Data) -> Result<(), (Self::Data, u32)>;
}

/// Put `data` on the board in place of what is there, or leave that there (MT-214 R2-N6).
fn replace<B: Board>(board: &mut B, format: u32, data: B::Data) -> Result<(), ClipboardError> {
    let kept = keep(board)?;
    board.empty().map_err(ClipboardError::System)?;
    let Err((refused, error)) = board.put(format, data) else {
        return Ok(());
    };
    drop(refused);
    for (format, data) in kept {
        // What the system does not take back is freed here.
        let _ = board.put(format, data);
    }
    Err(ClipboardError::System(error))
}

/// Everything on the board that can be put back. [`ClipboardError::CannotKeep`] when something could not be copied,
/// or when it holds something and none of it can be kept.
fn keep<B: Board>(board: &B) -> Result<Vec<(u32, B::Data)>, ClipboardError> {
    let mut kept = Vec::new();
    let mut kept_group = [false; 3];
    let mut twins = Vec::new();
    let mut lost = false;
    for format in board.formats() {
        let group = group(format);
        if group.is_some_and(|group| kept_group[group]) {
            continue;
        }
        match kind(format) {
            Kind::Lost => lost = true,
            Kind::Twin => twins.extend(group),
            kind => match board.copy(format, kind) {
                Copied::Data(data) => {
                    if let Some(group) = group {
                        kept_group[group] = true;
                    }
                    kept.push((format, data));
                }
                Copied::Nothing => {}
                Copied::Failed => return Err(ClipboardError::CannotKeep),
            },
        }
    }
    let twin_lost = twins.iter().any(|group| !kept_group[*group]);
    if kept.is_empty() && (lost || twin_lost) {
        return Err(ClipboardError::CannotKeep);
    }
    Ok(kept)
}

#[derive(Clone, Copy)]
enum Handle {
    Memory(HGLOBAL),
    Metafile(HENHMETAFILE),
}

/// Clipboard data this process holds. Freed unless the clipboard takes it.
struct Held(Option<Handle>);

impl Held {
    fn handle(&self) -> HANDLE {
        match self.0 {
            Some(Handle::Memory(memory)) => memory as HANDLE,
            Some(Handle::Metafile(metafile)) => metafile,
            None => 0,
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // SAFETY: made here, and the clipboard did not take it.
        unsafe {
            match self.0.take() {
                Some(Handle::Memory(memory)) => {
                    GlobalFree(memory);
                }
                Some(Handle::Metafile(metafile)) => {
                    DeleteEnhMetaFile(metafile);
                }
                None => {}
            }
        }
    }
}

/// The open clipboard of the operating system.
struct SystemBoard<'a>(&'a OpenedClipboard);

impl Board for SystemBoard<'_> {
    type Data = Held;

    fn formats(&self) -> Vec<u32> {
        let mut formats = Vec::new();
        let mut format = 0;
        while formats.len() < MAX_FORMATS {
            // SAFETY: the clipboard is open; 0 after the last.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            formats.push(format);
        }
        formats
    }

    fn copy(&self, format: u32, kind: Kind) -> Copied<Held> {
        // SAFETY: the clipboard is open; the handle stays the clipboard's while it is.
        let handle = unsafe { GetClipboardData(format) };
        if handle == 0 {
            return Copied::Nothing;
        }
        if kind == Kind::Metafile {
            // SAFETY: an enhanced metafile handle, copied into memory (no file name).
            let copy = unsafe { CopyEnhMetaFileW(handle, ptr::null()) };
            return if copy == 0 {
                Copied::Failed
            } else {
                Copied::Data(Held(Some(Handle::Metafile(copy))))
            };
        }
        copy_memory(handle as HGLOBAL, self.0)
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
        // SAFETY: the clipboard is open and owned by this window. It owns the data only when the call succeeds.
        if unsafe { SetClipboardData(format, data.handle()) } == 0 {
            return Err((data, last_error()));
        }
        data.0 = None;
        Ok(())
    }
}

/// A copy of global memory of the clipboard's. Memory that says it has no size is nothing.
fn copy_memory(source: HGLOBAL, opened: &OpenedClipboard) -> Copied<Held> {
    // SAFETY: a plain value; 0 for a handle that is no global memory.
    let size = unsafe { GlobalSize(source) };
    if size == 0 {
        return Copied::Nothing;
    }
    // SAFETY: plain values.
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) };
    if memory.is_null() {
        return Copied::Failed;
    }
    let held = Held(Some(Handle::Memory(memory)));
    let (Ok(from), Ok(to)) = (lock(source, opened), lock(memory, opened)) else {
        return Copied::Failed;
    };
    // SAFETY: both blocks are `size` bytes long and locked, and they are not the same.
    unsafe { ptr::copy_nonoverlapping(from.start.cast::<u8>(), to.start.cast::<u8>(), size) };
    drop((from, to));
    Copied::Data(held)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: u32 = 0xC100;

    /// A clipboard of the test's: formats with a value each, in order.
    #[derive(Default)]
    struct Pad {
        on: Vec<(u32, u32)>,
        uncopyable: Vec<u32>,
        refuses_first_put: bool,
        refuses_empty: bool,
        emptied: bool,
        puts: u32,
    }

    impl Board for Pad {
        type Data = u32;

        fn formats(&self) -> Vec<u32> {
            self.on.iter().map(|(format, _)| *format).collect()
        }

        fn copy(&self, format: u32, _: Kind) -> Copied<u32> {
            if self.uncopyable.contains(&format) {
                return Copied::Failed;
            }
            let found = self.on.iter().find(|(on, _)| *on == format);
            found.map_or(Copied::Nothing, |(_, value)| Copied::Data(*value))
        }

        fn empty(&mut self) -> Result<(), u32> {
            if self.refuses_empty {
                return Err(5);
            }
            self.on.clear();
            self.emptied = true;
            Ok(())
        }

        fn put(&mut self, format: u32, data: u32) -> Result<(), (u32, u32)> {
            self.puts += 1;
            if self.refuses_first_put && self.puts == 1 {
                return Err((data, 8));
            }
            self.on.push((format, data));
            Ok(())
        }
    }

    fn text_and_more() -> Vec<(u32, u32)> {
        let text = u32::from(CF_TEXT);
        let oem = u32::from(CF_OEMTEXT);
        vec![(HTML, 1), (FORMAT, 2), (16, 3), (text, 4), (oem, 5)]
    }

    #[test]
    fn a_write_replaces_what_is_there() {
        let mut pad = Pad {
            on: text_and_more(),
            ..Pad::default()
        };
        assert_eq!(replace(&mut pad, FORMAT, 9), Ok(()));
        assert_eq!(pad.on, [(FORMAT, 9)]);
    }

    /// MT-214 R2-N6.
    #[test]
    fn a_failed_put_puts_back_what_was_there_one_format_of_each_group() {
        let mut pad = Pad {
            on: text_and_more(),
            refuses_first_put: true,
            ..Pad::default()
        };
        assert_eq!(replace(&mut pad, FORMAT, 9), Err(ClipboardError::System(8)));
        assert!(pad.emptied);
        assert_eq!(pad.on, [(HTML, 1), (FORMAT, 2), (16, 3)], "the system makes the other texts again");
        // A bitmap: kept as the device-independent form the system makes it from.
        let bitmap = [CF_BITMAP, CF_DIB, CF_DIBV5, CF_PALETTE].map(u32::from);
        let mut pad = Pad {
            on: bitmap.iter().copied().zip(10..).collect(),
            refuses_first_put: true,
            ..Pad::default()
        };
        assert!(replace(&mut pad, FORMAT, 9).is_err());
        assert_eq!(pad.on, [(u32::from(CF_DIB), 11)]);
    }

    #[test]
    fn a_clipboard_that_cannot_be_kept_is_not_touched() {
        let private = u32::from(CF_PRIVATEFIRST);
        for on in [vec![(private, 1)], vec![(u32::from(CF_BITMAP), 1)]] {
            let mut pad = Pad {
                on: on.clone(),
                ..Pad::default()
            };
            assert_eq!(replace(&mut pad, FORMAT, 9), Err(ClipboardError::CannotKeep));
            assert!(!pad.emptied && pad.puts == 0);
            assert_eq!(pad.on, on);
        }
        // One format that holds something but cannot be copied (no memory) is enough.
        let mut pad = Pad {
            on: text_and_more(),
            uncopyable: vec![HTML],
            ..Pad::default()
        };
        assert_eq!(replace(&mut pad, FORMAT, 9), Err(ClipboardError::CannotKeep));
        assert_eq!(pad.on, text_and_more());
        // What cannot be kept beside what can does not stop a write.
        let mut pad = Pad {
            on: vec![(private, 1), (FORMAT, 2)],
            ..Pad::default()
        };
        assert_eq!(replace(&mut pad, FORMAT, 9), Ok(()));
    }

    #[test]
    fn an_empty_clipboard_is_written_and_a_refused_empty_changes_nothing() {
        let mut pad = Pad::default();
        assert_eq!(replace(&mut pad, FORMAT, 9), Ok(()));
        assert_eq!(pad.on, [(FORMAT, 9)]);
        let mut pad = Pad {
            on: text_and_more(),
            refuses_empty: true,
            ..Pad::default()
        };
        assert_eq!(replace(&mut pad, FORMAT, 9), Err(ClipboardError::System(5)));
        assert_eq!((pad.on, pad.puts), (text_and_more(), 0));
    }

    #[test]
    fn formats_are_kept_by_what_their_handle_is() {
        assert_eq!(kind(FORMAT), Kind::Memory);
        assert_eq!(kind(HTML), Kind::Memory);
        assert_eq!(kind(u32::from(CF_ENHMETAFILE)), Kind::Metafile);
        for twin in [CF_BITMAP, CF_METAFILEPICT, CF_PALETTE] {
            assert_eq!(kind(u32::from(twin)), Kind::Twin);
            assert!(group(u32::from(twin)).is_some(), "it has a group to be made from");
        }
        for lost in [CF_OWNERDISPLAY, CF_DSPBITMAP, CF_PRIVATELAST, CF_GDIOBJFIRST] {
            assert_eq!(kind(u32::from(lost)), Kind::Lost);
        }
        assert_eq!(kind(0x1_0000), Kind::Lost);
    }
}
