//! KEEP_PLAIN_TEXT_OR_NOTHING against a clipboard of the test's own. It lists formats as the system does: the one a
//! program placed, then those the system makes from it ("the system first enumerates the format that is on the
//! clipboard, followed by the formats to which it can be converted", Synthesized Clipboard Formats).

use std::cell::Cell;

use windows_sys::Win32::System::Ole::{
    CF_BITMAP, CF_DIB, CF_DIBV5, CF_ENHMETAFILE, CF_HDROP, CF_LOCALE, CF_METAFILEPICT, CF_OEMTEXT,
    CF_PRIVATEFIRST, CF_TEXT,
};

use super::*;

/// A registered format, as "HTML Format" is.
const HTML: u32 = 0xC100;

/// A registered format named as one of the markers of the clipboard history.
const MARKER: u32 = 0xC200;

fn text() -> Vec<u32> {
    vec![FORMAT, u32::from(CF_LOCALE), u32::from(CF_TEXT), u32::from(CF_OEMTEXT)]
}

fn ansi_text() -> Vec<u32> {
    vec![u32::from(CF_TEXT), u32::from(CF_LOCALE), u32::from(CF_OEMTEXT), FORMAT]
}

fn not_plain_text() -> Vec<(&'static str, Vec<u32>)> {
    let mut rich = vec![HTML];
    rich.extend(text());
    vec![
        ("text with a registered format", rich),
        ("a bitmap", [CF_BITMAP, CF_DIB, CF_DIBV5].map(u32::from).to_vec()),
        ("a metafile", [CF_ENHMETAFILE, CF_METAFILEPICT].map(u32::from).to_vec()),
        ("files", vec![u32::from(CF_HDROP)]),
        ("a private format", vec![u32::from(CF_PRIVATEFIRST)]),
        ("a registered format alone", vec![HTML]),
    ]
}

#[derive(Default)]
struct Pad {
    formats: Vec<u32>,
    /// Its text: a value, and its size in bytes. The system makes it from any text format.
    text: Option<(u32, usize)>,
    /// What was put on it since it was emptied.
    on: Vec<(u32, u32)>,
    /// How often its text was asked for.
    asked: Cell<u32>,
    refuses_first_put: bool,
    refuses_empty: bool,
    emptied: bool,
    puts: u32,
}

impl Pad {
    fn holding(formats: Vec<u32>) -> Self {
        let text = formats.iter().any(|format| PLAIN_TEXT.contains(format)).then_some((1, 64));
        Self {
            formats,
            text,
            ..Self::default()
        }
    }

    fn refusing(formats: Vec<u32>) -> Self {
        Self {
            refuses_first_put: true,
            ..Self::holding(formats)
        }
    }
}

impl Board for Pad {
    type Data = u32;

    fn formats(&self, most: usize) -> Vec<u32> {
        self.formats.iter().copied().take(most).collect()
    }

    fn is_marker(&self, format: u32) -> bool {
        format == MARKER
    }

    fn copy_text(&self, limit: usize) -> Copied<u32> {
        self.asked.set(self.asked.get() + 1);
        match self.text {
            Some((_, size)) if size > limit => Copied::TooLarge,
            Some((value, _)) => Copied::Data(value),
            None => Copied::Unreadable,
        }
    }

    fn empty(&mut self) -> Result<(), u32> {
        if self.refuses_empty {
            return Err(5);
        }
        self.formats.clear();
        self.text = None;
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

fn write(pad: &mut Pad) -> Result<(), ClipboardError> {
    replace(pad, FORMAT, 9, Unkept::ReplaceAnyway)
}

#[test]
fn a_write_replaces_whatever_is_there() {
    let mut all: Vec<Vec<u32>> = not_plain_text().into_iter().map(|(_, formats)| formats).collect();
    all.extend([text(), ansi_text(), vec![MARKER, FORMAT], Vec::new()]);
    for formats in all {
        let mut pad = Pad::holding(formats.clone());
        assert_eq!(write(&mut pad), Ok(()), "{formats:x?}");
        assert_eq!(pad.on, [(FORMAT, 9)]);
    }
}

/// R2-N6, still: a refused put does not leave plain text lost.
#[test]
fn plain_text_is_kept_and_put_back_when_the_put_is_refused() {
    for formats in [text(), ansi_text()] {
        let mut pad = Pad::refusing(formats);
        assert_eq!(write(&mut pad), Err(ClipboardError::System(8)));
        assert!(pad.emptied);
        assert_eq!(pad.on, [(FORMAT, 1)], "the system makes the other text formats again");
        assert_eq!(pad.asked.get(), 1);
    }
}

/// MT-214 E3-F2: a write asks for no format but plain text, so no program renders a promised format for it, and it
/// keeps nothing else; a refused put then leaves the clipboard empty and says so.
#[test]
fn nothing_but_plain_text_is_asked_for_or_kept() {
    for (what, formats) in not_plain_text() {
        let mut pad = Pad::holding(formats.clone());
        assert_eq!(keep(&pad), Err(Skipped::NotPlainText), "{what}");
        assert_eq!(write(&mut pad), Ok(()), "{what}");
        assert_eq!(pad.asked.get(), 0, "{what}: asked for nothing");
        let mut pad = Pad::refusing(formats);
        assert_eq!(write(&mut pad), Err(ClipboardError::NotRestored(8)), "{what}");
        assert!(pad.emptied && pad.on.is_empty(), "{what}: nothing put back");
        assert_eq!(pad.asked.get(), 0, "{what}");
    }
}

/// MT-214 E3-F2: what a password manager marks for the clipboard history is never copied, wherever the marker is.
#[test]
fn a_clipboard_with_a_marker_of_the_clipboard_history_keeps_nothing() {
    let mut first = vec![MARKER];
    first.extend(text());
    let mut last = text();
    last.push(MARKER);
    for formats in [first, last, vec![MARKER]] {
        let pad = Pad::holding(formats.clone());
        assert_eq!(keep(&pad), Err(Skipped::Marked), "{formats:x?}");
        assert_eq!(pad.asked.get(), 0);
        let mut pad = Pad::refusing(formats);
        assert_eq!(write(&mut pad), Err(ClipboardError::NotRestored(8)));
        assert!(pad.on.is_empty());
    }
}

#[test]
fn text_beyond_the_limit_or_unreadable_text_is_not_kept() {
    let mut pad = Pad::holding(text());
    pad.text = Some((1, KEEP_MAX_BYTES));
    assert_eq!(keep(&pad), Ok(vec![(FORMAT, 1)]), "at the limit");
    pad.text = Some((1, KEEP_MAX_BYTES + 1));
    assert_eq!(keep(&pad), Err(Skipped::TooLarge));
    pad.refuses_first_put = true;
    assert_eq!(write(&mut pad), Err(ClipboardError::NotRestored(8)));
    let mut pad = Pad::holding(text());
    pad.text = None;
    assert_eq!(keep(&pad), Err(Skipped::Unreadable), "a promise its program did not keep");
    let pad = Pad::holding(vec![u32::from(CF_TEXT); MAX_FORMATS]);
    assert_eq!(keep(&pad), Err(Skipped::NotPlainText), "more formats than are looked at");
}

/// The other choice the policy names: what cannot be kept is left as it is, and the write refused.
#[test]
fn leave_as_is_refuses_a_write_that_keeps_nothing_and_touches_nothing() {
    let leave = |pad: &mut Pad| replace(pad, FORMAT, 9, Unkept::LeaveAsIs);
    for (what, formats) in not_plain_text() {
        let mut pad = Pad::holding(formats.clone());
        assert_eq!(leave(&mut pad), Err(ClipboardError::CannotKeep), "{what}");
        assert!(!pad.emptied && pad.puts == 0, "{what}");
        assert_eq!((pad.formats, pad.asked.get()), (formats, 0), "{what}");
    }
    let mut pad = Pad::holding(text());
    assert_eq!(leave(&mut pad), Ok(()));
    assert_eq!(UNKEPT, Unkept::ReplaceAnyway, "KEEP_PLAIN_TEXT_OR_NOTHING writes all the same");
}

#[test]
fn an_empty_clipboard_is_written_and_a_refused_empty_changes_nothing() {
    let mut pad = Pad::default();
    assert_eq!(write(&mut pad), Ok(()));
    assert_eq!(pad.on, [(FORMAT, 9)]);
    let mut pad = Pad::refusing(Vec::new());
    assert_eq!(write(&mut pad), Err(ClipboardError::System(8)), "nothing was lost");
    let mut pad = Pad {
        refuses_empty: true,
        ..Pad::holding(text())
    };
    assert_eq!(write(&mut pad), Err(ClipboardError::System(5)));
    assert_eq!((pad.formats, pad.puts), (text(), 0));
}

/// MT-214 E3-F3: after a refused write put the Operator's text back, this window owns the clipboard again, and a
/// withdrawal must not empty it.
#[test]
fn only_this_windows_own_last_write_is_withdrawn() {
    let mut last = LastWrite::default();
    assert!(!last.may_withdraw(true), "nothing written yet");
    last.wrote(&Err(ClipboardError::System(8)));
    assert!(!last.may_withdraw(true), "the restored text");
    last.wrote(&Err(ClipboardError::NotRestored(8)));
    assert!(!last.may_withdraw(true));
    last.wrote(&Ok(()));
    assert!(last.may_withdraw(true));
    last.withdrawn();
    assert!(!last.may_withdraw(true), "withdrawn once");
    last.wrote(&Ok(()));
    assert!(!last.may_withdraw(false), "another program wrote since");
    assert!(!last.may_withdraw(true), "and it stays so");
}
