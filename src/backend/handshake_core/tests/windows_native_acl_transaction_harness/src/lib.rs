#![cfg(target_os = "windows")]
#![allow(dead_code)]

// Compile the exact private production module as a focused test crate. The
// parent crate's unit-test harness currently contains unrelated WIP failures.
#[path = "../../../src/sandbox/windows_native_jail/acl_transaction.rs"]
mod acl_transaction;
