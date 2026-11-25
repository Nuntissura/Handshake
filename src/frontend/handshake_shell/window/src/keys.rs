//! Names of keys. A key's position and its meaning are two lists, because layouts differ.

/// Calls `$apply!` with every key position this crate names.
macro_rules! with_key_codes {
    ($apply:ident) => {
        $apply! {
            Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2, Digit3, Digit4, Digit5,
            Digit6, Digit7, Digit8, Digit9, Equal, IntlBackslash, IntlRo, IntlYen, KeyA, KeyB, KeyC, KeyD, KeyE, KeyF,
            KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX,
            KeyY, KeyZ, Minus, Period, Quote, Semicolon, Slash, AltLeft, AltRight, Backspace, CapsLock, ContextMenu,
            ControlLeft, ControlRight, Enter, SuperLeft, SuperRight, ShiftLeft, ShiftRight, Space, Tab, Convert,
            KanaMode, Lang1, Lang2, Lang3, Lang4, Lang5, NonConvert, Delete, End, Help, Home, Insert, PageDown, PageUp,
            ArrowDown, ArrowLeft, ArrowRight, ArrowUp, NumLock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5,
            Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadComma, NumpadDecimal, NumpadDivide, NumpadEnter,
            NumpadEqual, NumpadMultiply, NumpadSubtract, Escape, Fn, FnLock, PrintScreen, ScrollLock, Pause,
            BrowserBack, BrowserFavorites, BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop,
            Eject, LaunchApp1, LaunchApp2, LaunchMail, MediaPlayPause, MediaSelect, MediaStop, MediaTrackNext,
            MediaTrackPrevious, Power, Sleep, AudioVolumeDown, AudioVolumeMute, AudioVolumeUp, WakeUp, Copy, Cut, Paste,
            Undo, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22,
            F23, F24,
        }
    };
}

/// Calls `$apply!` with every key meaning this crate names.
macro_rules! with_named_keys {
    ($apply:ident) => {
        $apply! {
            Alt, AltGraph, CapsLock, Control, Fn, FnLock, NumLock, ScrollLock, Shift, Super, Enter, Tab, Space,
            ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Home, PageDown, PageUp, Backspace, Clear, Copy, CrSel, Cut,
            Delete, EraseEof, ExSel, Insert, Paste, Redo, Undo, Accept, Attn, Cancel, ContextMenu, Escape, Execute,
            Find, Help, Pause, Play, Select, ZoomIn, ZoomOut, Eject, Power, PrintScreen, Standby, WakeUp, Convert,
            NonConvert, ModeChange, Process, FinalMode, HangulMode, HanjaMode, JunjaMode, Hiragana, KanaMode, KanjiMode,
            Katakana, Romaji, Zenkaku, Hankaku, Alphanumeric, MediaPlayPause, MediaStop, MediaTrackNext,
            MediaTrackPrevious, AudioVolumeDown, AudioVolumeUp, AudioVolumeMute, LaunchMail, LaunchMediaPlayer,
            LaunchApplication1, LaunchApplication2, BrowserBack, BrowserFavorites, BrowserForward, BrowserHome,
            BrowserRefresh, BrowserSearch, BrowserStop, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14,
            F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
        }
    };
}

macro_rules! define_key_codes {
    ($($name:ident),* $(,)?) => {
        /// A key by its position, named after the key that a US keyboard has there.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        #[allow(missing_docs)]
        pub enum KeyCode {
            /// A position without a name here; [`crate::PhysicalKey::scancode`] still identifies it.
            Unidentified,
            $($name,)*
        }

        impl KeyCode {
            /// Every named position.
            pub const NAMED: &'static [KeyCode] = &[$(KeyCode::$name,)*];

            /// Its name.
            pub fn name(self) -> &'static str {
                match self {
                    Self::Unidentified => "Unidentified",
                    $(Self::$name => stringify!($name),)*
                }
            }
        }
    };
}

macro_rules! define_named_keys {
    ($($name:ident),* $(,)?) => {
        /// A key by its meaning under the active layout, where that meaning is not text.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        #[allow(missing_docs)]
        pub enum NamedKey {
            $($name,)*
        }

        impl NamedKey {
            /// Every name.
            pub const ALL: &'static [NamedKey] = &[$(NamedKey::$name,)*];

            /// Its name.
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$name => stringify!($name),)*
                }
            }
        }
    };
}

with_key_codes!(define_key_codes);
with_named_keys!(define_named_keys);

#[cfg(windows)]
pub(crate) use {with_key_codes, with_named_keys};
