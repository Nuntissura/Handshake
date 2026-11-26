//! Text on the clipboard.
//!
//! # KEEP_PLAIN_TEXT_OR_NOTHING (MT-214 E3-F2)
//!
//! What a write of the operating system's clipboard keeps of the old content, to put back should the system refuse
//! the new text after the clipboard was emptied (R2-N6):
//!
//! - Only plain text, and only when the clipboard holds nothing else: every format it lists is a standard text format
//!   (`CF_UNICODETEXT`, `CF_TEXT`, `CF_OEMTEXT`, `CF_LOCALE`), and the text is at most [`KEEP_MAX_BYTES`]. One copy,
//!   in `CF_UNICODETEXT`; the system makes the other text formats from it again (Synthesized Clipboard Formats).
//! - So nothing from a clipboard that carries a format of the clipboard history and the cloud clipboard,
//!   `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` or `CanUploadToCloudClipboard`
//!   (learn.microsoft.com/windows/win32/dataxchg/clipboard-formats, "Cloud Clipboard and Clipboard History Formats"),
//!   whatever its value: these are registered formats, not plain text (MT-214 E4-F4). Password managers mark their
//!   entries so.
//! - No other format is asked for, so no program is made to render a format it promised and did not render yet
//!   (Clipboard Operations, "Delayed Rendering"), with one exception: promised plain text, when the clipboard holds
//!   nothing but plain text. That request is made on a thread of its own, and the write waits for it at most a quarter
//!   of a second: a program that does not render in time makes the write [`ClipboardError::Busy`] and never makes the
//!   loop wait (MT-214 E4-F3). A read asks for promised text the same way, on the same thread, within the same bound
//!   (R4-D3).
//! - When nothing is kept, the write replaces the clipboard all the same. Should the system then refuse the new text,
//!   the clipboard stays empty and the write says so ([`ClipboardError::NotRestored`]). Refusing such writes instead
//!   ([`ClipboardError::CannotKeep`], clipboard untouched) would make most copies fail: after a copy in a browser, an
//!   office program, an image editor or a password manager.
//! - A kept copy is wiped before it is freed. The clipboard is open only for the swap.
//!
//! A withdrawal empties the clipboard only while it holds this process's own last write: not after a refused write
//! put the old text back, which made this process the owner of the Operator's text again (E3-F3).

/// The most text one clipboard read or write carries, in bytes of UTF-8.
pub const MAX_CLIPBOARD_BYTES: usize = 16 * 1024 * 1024;

/// The most plain text a write keeps, to put back should its own text be refused, in bytes of UTF-16 with the ending
/// 0. Larger text is not kept.
pub const KEEP_MAX_BYTES: usize = 1024 * 1024;

/// Why the clipboard did not give or take text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClipboardError {
    /// Another program has it open; or a program that promised text did not render it within a quarter of a second, for
    /// a read or for what a write keeps, and until it did or gave up every read and write is busy (MT-214 E4-F3, R4-D3).
    /// Try again later; nothing here waits for it longer.
    Busy,
    /// More than [`MAX_CLIPBOARD_BYTES`].
    TooLarge,
    /// Only the Operator replaces the clipboard.
    NeedsOperator,
    /// What is on the clipboard could not be kept, so it was left as it is. Only under a policy that refuses such writes;
    /// KEEP_PLAIN_TEXT_OR_NOTHING writes all the same.
    CannotKeep,
    /// The system refused the text after the clipboard was emptied, and what was there had not been kept: the
    /// clipboard is empty now. With the operating system's error number.
    NotRestored(u32),
    /// The operating system refused, with its error number. A write that failed left the clipboard as it was, or put
    /// back the plain text that was there.
    System(u32),
}

impl ClipboardError {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Busy => "clipboard.busy",
            Self::TooLarge => "clipboard.too_large",
            Self::NeedsOperator => "clipboard.needs_operator",
            Self::CannotKeep => "clipboard.cannot_keep",
            Self::NotRestored(_) => "clipboard.not_restored",
            Self::System(_) => "clipboard.system",
        }
    }
}

/// Where clipboard text is kept. The default is the operating system's clipboard; tests give their own.
pub trait TextClipboard {
    /// The text, if there is any. A program that promised text and did not give it yet is asked for it; the operating
    /// system's clipboard asks on a thread of its own and waits a quarter of a second at most, then is
    /// [`ClipboardError::Busy`] (MT-214 R4-D3). Another implementation must not block for longer either: it runs on the
    /// loop's thread.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    fn read(&mut self) -> Result<Option<String>, ClipboardError>;

    /// Replace the clipboard with `text`. The operating system's clipboard keeps what KEEP_PLAIN_TEXT_OR_NOTHING says
    /// (module documentation) and puts it back should its own put be refused.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    fn write(&mut self, text: &str) -> Result<(), ClipboardError>;

    /// Take back what this process last wrote, if it is still there. Returns whether it was.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    fn withdraw(&mut self) -> Result<bool, ClipboardError>;
}
