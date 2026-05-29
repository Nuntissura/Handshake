//! Placeholder escape hatch for operator foreground exceptions.
//!
//! HBR-QUIET-001 treats this module as the only documented operator foreground exception
//! namespace. MT-015 adds passive foreground-audit evidence here without authorizing foreground
//! windows. MT-019 owns the runtime declaration, warning, and approval surface before any
//! intentional foreground window or focus behavior can be added here.

pub mod cdp_client;
pub mod focus_audit;
pub mod foreground_exception;
pub mod keyboard_inject_test;
