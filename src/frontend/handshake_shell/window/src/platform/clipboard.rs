//! The operating system's clipboard, as text.

use std::ptr;
use std::slice;

use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardOwner,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

use super::last_error;
use crate::clipboard::{ClipboardError, TextClipboard, MAX_CLIPBOARD_BYTES};

const FORMAT: u32 = CF_UNICODETEXT as u32;

/// One UTF-16 unit is at most three bytes of UTF-8, and at least one.
const MAX_UNITS: usize = MAX_CLIPBOARD_BYTES;

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
        // SAFETY: plain values.
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, units.len() * size_of::<u16>()) };
        if memory.is_null() {
            return Err(ClipboardError::System(last_error()));
        }
        let written = lock(memory, &opened).and_then(|locked| {
            // SAFETY: as many units as were allocated, and the memory is ours alone.
            unsafe { ptr::copy_nonoverlapping(units.as_ptr(), locked.start, units.len()) };
            drop(locked);
            // SAFETY: the clipboard is open. It owns the memory only when the second call succeeds.
            let given =
                unsafe { EmptyClipboard() != 0 && SetClipboardData(FORMAT, memory as isize) != 0 };
            if given {
                Ok(())
            } else {
                Err(ClipboardError::System(last_error()))
            }
        });
        if written.is_err() {
            // SAFETY: still ours.
            unsafe { GlobalFree(memory) };
        }
        written
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
