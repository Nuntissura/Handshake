//! Files dropped on a window.
//!
//! The drop message and not a drop target: it carries all paths and the position at once, and needs no COM apartment
//! on the loop's thread. It reports nothing while files are dragged over the window.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::Foundation::{POINT, WPARAM};
use windows_sys::Win32::UI::Shell::{DragFinish, DragQueryFileW, DragQueryPoint, HDROP};

use crate::geometry::PhysicalPosition;
use crate::input::{FilesDropped, MAX_DROPPED_FILES};
use crate::origin::InputStamp;

/// The longest path Windows uses.
const MAX_PATH_UNITS: u32 = 32_768;

const COUNT: u32 = u32::MAX;

pub(super) struct Dropped {
    paths: Vec<PathBuf>,
    left_out: u32,
}

impl Dropped {
    pub(super) fn into_event(
        self,
        position: Option<PhysicalPosition>,
        stamp: InputStamp,
    ) -> FilesDropped {
        FilesDropped {
            paths: self.paths,
            left_out: self.left_out,
            position,
            stamp,
        }
    }
}

/// Take what a drop message carries, and release it.
pub(super) fn take(wparam: WPARAM) -> (Dropped, Option<PhysicalPosition>) {
    let drop = wparam as HDROP;
    // SAFETY: the message's own drop, released once, below. Every pointer outlives its call.
    unsafe {
        let count = DragQueryFileW(drop, COUNT, ptr::null_mut(), 0);
        let paths = (0..count.min(MAX_DROPPED_FILES))
            .filter_map(|at| path(drop, at))
            .collect::<Vec<_>>();
        let mut point = POINT { x: 0, y: 0 };
        let in_client = DragQueryPoint(drop, &mut point) != 0;
        DragFinish(drop);
        let taken = u32::try_from(paths.len()).unwrap_or(u32::MAX);
        let dropped = Dropped {
            paths,
            left_out: count.saturating_sub(taken),
        };
        (
            dropped,
            in_client.then(|| PhysicalPosition::new(point.x, point.y)),
        )
    }
}

/// # Safety
///
/// `drop` is a drop that was not released.
unsafe fn path(drop: HDROP, at: u32) -> Option<PathBuf> {
    // SAFETY: asks for the length only.
    let units = unsafe { DragQueryFileW(drop, at, ptr::null_mut(), 0) };
    if units == 0 || units > MAX_PATH_UNITS {
        return None;
    }
    let mut buffer = vec![0u16; units as usize + 1];
    // SAFETY: room for the path and its end, as the call is told.
    let written = unsafe { DragQueryFileW(drop, at, buffer.as_mut_ptr(), units + 1) };
    buffer.truncate(written.min(units) as usize);
    (written != 0).then(|| PathBuf::from(OsString::from_wide(&buffer)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::UI::Shell::DROPFILES;

    /// What the system puts into a drop message. `take` releases it.
    fn dropped(paths: &[&str], at: POINT, on_frame: bool) -> WPARAM {
        let names: Vec<u16> = paths
            .iter()
            .flat_map(|path| path.encode_utf16().chain(Some(0)))
            .chain(Some(0))
            .collect();
        let start = size_of::<DROPFILES>();
        // SAFETY: room for the header and the names, written while the memory is locked.
        unsafe {
            let memory = GlobalAlloc(GMEM_MOVEABLE, start + names.len() * size_of::<u16>());
            assert!(!memory.is_null());
            let bytes = GlobalLock(memory).cast::<u8>();
            let header = DROPFILES {
                pFiles: start as u32,
                pt: at,
                fNC: i32::from(on_frame),
                fWide: 1,
            };
            bytes.cast::<DROPFILES>().write_unaligned(header);
            let text = bytes.add(start).cast::<u16>();
            for (offset, unit) in names.iter().enumerate() {
                text.add(offset).write_unaligned(*unit);
            }
            GlobalUnlock(memory);
            memory as WPARAM
        }
    }

    #[test]
    fn a_drop_keeps_every_path_in_order_and_the_position() {
        let paths = [r"C:\a\b.txt", r"C:\日本\ê 😀.png", r"\\server\share\c"];
        let (files, position) = take(dropped(&paths, POINT { x: 12, y: -3 }, false));
        let expected: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        assert_eq!(files.paths, expected);
        assert_eq!(files.left_out, 0);
        assert_eq!(position, Some(PhysicalPosition::new(12, -3)));
    }

    #[test]
    fn a_drop_on_the_frame_has_no_position_in_the_client_area() {
        let (files, position) = take(dropped(&["C:/x"], POINT { x: 1, y: 2 }, true));
        assert_eq!(files.paths.len(), 1);
        assert_eq!(position, None);
    }

    #[test]
    fn a_huge_drop_is_bounded_and_says_so() {
        let names: Vec<String> = (0..MAX_DROPPED_FILES + 7)
            .map(|n| format!("C:/{n}"))
            .collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let (files, _) = take(dropped(&names, POINT { x: 0, y: 0 }, false));
        assert_eq!(files.paths.len(), MAX_DROPPED_FILES as usize);
        assert_eq!(files.left_out, 7);
        assert_eq!(files.paths[0], PathBuf::from("C:/0"));
    }
}
