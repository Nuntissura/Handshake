//! Canonical Argus method names and their compatibility aliases.
//!
//! Product-facing clients use the four `argus.*` names. The pre-Argus MCP spellings remain accepted
//! at the wire boundary so older clients keep working, but callers inside the product can normalize a
//! request once and use the canonical method throughout leasing, dispatch, attribution, and manuals.

/// Canonical read-the-live-AccessKit-tree method.
pub const ARGUS_INSPECT_METHOD: &str = "argus.inspect";
/// Canonical activate-an-addressed-control method.
pub const ARGUS_CLICK_METHOD: &str = "argus.click";
/// Canonical replace-an-addressed-control's whole value method.
pub const ARGUS_SET_VALUE_METHOD: &str = "argus.set_value";
/// Canonical focus-safe pixel capture method.
pub const ARGUS_SCREENSHOT_METHOD: &str = "argus.screenshot";

/// Legacy aliases retained only at the compatibility boundary.
pub const LEGACY_INSPECT_METHOD: &str = "list_widgets";
pub const LEGACY_CLICK_METHOD: &str = "click_widget";
pub const LEGACY_SET_VALUE_METHOD: &str = "set_value";
pub const LEGACY_SCREENSHOT_METHOD: &str = "screenshot";

/// The closed set of Argus operations after wire-name normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgusMethod {
    Inspect,
    Click,
    SetValue,
    Screenshot,
}

impl ArgusMethod {
    /// Accept a canonical method or its legacy alias.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        match name {
            ARGUS_INSPECT_METHOD | LEGACY_INSPECT_METHOD => Some(Self::Inspect),
            ARGUS_CLICK_METHOD | LEGACY_CLICK_METHOD => Some(Self::Click),
            ARGUS_SET_VALUE_METHOD | LEGACY_SET_VALUE_METHOD => Some(Self::SetValue),
            ARGUS_SCREENSHOT_METHOD | LEGACY_SCREENSHOT_METHOD => Some(Self::Screenshot),
            _ => None,
        }
    }

    /// Stable product-facing name used in manuals, receipts, and attributed action logs.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Inspect => ARGUS_INSPECT_METHOD,
            Self::Click => ARGUS_CLICK_METHOD,
            Self::SetValue => ARGUS_SET_VALUE_METHOD,
            Self::Screenshot => ARGUS_SCREENSHOT_METHOD,
        }
    }

    /// Whether this operation mutates an addressed live control.
    pub const fn is_mutating(self) -> bool {
        matches!(self, Self::Click | Self::SetValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_and_legacy_names_normalize_to_the_same_operations() {
        for (canonical, legacy, expected) in [
            (
                ARGUS_INSPECT_METHOD,
                LEGACY_INSPECT_METHOD,
                ArgusMethod::Inspect,
            ),
            (ARGUS_CLICK_METHOD, LEGACY_CLICK_METHOD, ArgusMethod::Click),
            (
                ARGUS_SET_VALUE_METHOD,
                LEGACY_SET_VALUE_METHOD,
                ArgusMethod::SetValue,
            ),
            (
                ARGUS_SCREENSHOT_METHOD,
                LEGACY_SCREENSHOT_METHOD,
                ArgusMethod::Screenshot,
            ),
        ] {
            assert_eq!(ArgusMethod::from_wire_name(canonical), Some(expected));
            assert_eq!(ArgusMethod::from_wire_name(legacy), Some(expected));
            assert_eq!(expected.canonical_name(), canonical);
        }
        assert_eq!(ArgusMethod::from_wire_name("argus.unknown"), None);
    }
}
