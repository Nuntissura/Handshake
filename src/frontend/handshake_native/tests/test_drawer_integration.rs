//! WP-KERNEL-011 MT-023 (C6) — PROOF-023-3 live integration against a REAL PostgreSQL-backed
//! handshake_core server. `#[ignore]` by default; run with a live backend on 127.0.0.1:37501:
//!
//! ```text
//! cargo test -p handshake-native --test test_drawer_integration -- --ignored
//! ```
//!
//! Requires env `HSK_TEST_WORKSPACE_ID` pointing at a workspace that has at least one Loom block with
//! `content_type = "note"` (the contract's example uses a `list`, but `list` is not a real
//! `LoomBlockContentType` — the verified note view is the honest equivalent; see the MT-023 deviation
//! notes). Asserts the Notes card badge count is >= 1 within 3 seconds, proving the REAL off-thread
//! fetch path resolves against the real PostgreSQL/EventLedger backend.
//!
//! HBR-INT: this is the integration test the MT-023 contract mandates, marked `#[ignore]` so the default
//! `cargo test` run (no backend) does not depend on a running server.

use handshake_native::backend_client::{DrawerDataCell, DrawerDataClient, DrawerDataKind};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BACKEND_BASE_URL: &str = "http://127.0.0.1:37501";

#[test]
#[ignore = "needs a live handshake_core + PostgreSQL on 127.0.0.1:37501 and HSK_TEST_WORKSPACE_ID"]
fn notes_card_badge_count_is_at_least_one_from_real_pg() {
    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID")
        .expect("set HSK_TEST_WORKSPACE_ID to a workspace with at least one note block");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");

    let client = DrawerDataClient::new(BACKEND_BASE_URL, rt.handle().clone());
    let cell: DrawerDataCell = Arc::new(Mutex::new(None));

    // The SAME off-thread fetch the open drawer fires.
    client.fetch_count(&workspace_id, DrawerDataKind::Notes, cell.clone());

    // Poll the delivery cell for up to 3 seconds (the contract's bound).
    let deadline = Instant::now() + Duration::from_secs(3);
    let delivered = loop {
        if let Some(v) = cell.lock().unwrap().take() {
            break v;
        }
        if Instant::now() > deadline {
            panic!("Notes card fetch did not deliver within 3s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let (kind, result) = delivered;
    assert_eq!(kind, DrawerDataKind::Notes);
    let data = result.expect("Notes count fetch succeeded against the real backend");
    assert!(
        data.badge_count >= 1,
        "expected >= 1 note block in workspace {workspace_id}, got badge_count {}",
        data.badge_count
    );
    println!(
        "PASS: Notes card badge_count = {} from real PG-backed /loom/views/all?content_type=note",
        data.badge_count
    );
}
