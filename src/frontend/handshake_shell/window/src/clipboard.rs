//! Text on the clipboard.

/// The most text one clipboard read or write carries, in bytes of UTF-8.
pub const MAX_CLIPBOARD_BYTES: usize = 16 * 1024 * 1024;

/// Why the clipboard did not give or take text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClipboardError {
    /// Another program has it open. Try again later; nothing here waits for it.
    Busy,
    /// More than [`MAX_CLIPBOARD_BYTES`].
    TooLarge,
    /// Only the Operator replaces the clipboard.
    NeedsOperator,
    /// The operating system refused, with its error number.
    System(u32),
}

impl ClipboardError {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Busy => "clipboard.busy",
            Self::TooLarge => "clipboard.too_large",
            Self::NeedsOperator => "clipboard.needs_operator",
            Self::System(_) => "clipboard.system",
        }
    }
}

/// Where clipboard text is kept. The default is the operating system's clipboard; tests give their own.
pub trait TextClipboard {
    /// The text, if there is any. A program that promised text and did not give it yet is asked for it, and that takes
    /// as long as that program takes.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    fn read(&mut self) -> Result<Option<String>, ClipboardError>;

    /// Replace the clipboard with `text`.
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
