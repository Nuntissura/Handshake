//! `palmistry` library crate — the Tier 3 EXTERNAL out-of-process watcher's reusable modules
//! (WP-KERNEL-012 MT-089..MT-093, Master Spec v02.196 §6.13).
//!
//! The watcher BINARY (`src/main.rs`) is a thin entrypoint over these modules; exposing them as a library
//! lets the integration tests under `tests/` (e.g. `test_survivor_store.rs`, `test_fr_forward.rs`) drive
//! the REAL production types directly (a `[bin]`-only crate's items are not importable by `tests/`). The
//! binary and the tests therefore exercise the SAME code — no test-only reimplementation of the store /
//! forwarder logic.
//!
//! Module map:
//! - [`cli`] — argument/env intake (MT-089).
//! - [`control`] — the local-socket control channel (MT-089).
//! - [`lifecycle`] — the inverted lifecycle (MT-089).
//! - [`ring_reader`] — the zero-cooperation MT-081 ring reader (MT-090).
//! - [`hung_window_probe`] — the OS hung-window probe (MT-091).
//! - [`freeze_detect`] — the double-signal freeze detector (MT-091).
//! - [`child_stall`] / [`child_registry`] — child-process no-progress stall detection (MT-106).
//! - [`crash_capture`] — the out-of-process minidump + typed crash record (MT-092).
//! - [`survivor_store`] — the DURABLE survivor store (MT-093, §6.13.7).
//! - [`fr_forward`] — the recovery-time Flight Recorder forwarder (MT-093, §6.13.7).

pub mod child_registry;
pub mod child_stall;
pub mod cli;
pub mod control;
pub mod crash_capture;
pub mod fr_forward;
pub mod freeze_detect;
pub mod hung_window_probe;
pub mod lifecycle;
pub mod ring_reader;
pub mod survivor_store;
