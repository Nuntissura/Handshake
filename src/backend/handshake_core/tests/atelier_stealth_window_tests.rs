//! WP-KERNEL-005 atelier Stealth Reference Window: real PostgreSQL round-trip
//! proofs for the stealth_window submodule (MT-205). Run with a live
//! DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_stealth_window_tests -- --nocapture
//!
//! No mocks: each test connects the actual `AtelierStore` to a real Postgres,
//! ensures the schema, exercises the stealth-window registry with REAL data,
//! and asserts the load-bearing invariants: idempotent window create on
//! (owner_actor, title), append-only seq monotonicity + UNIQUE(window, seq),
//! two-phase reorder permutation guard, idempotent capture receipt on
//! (window, manifest_id), governed-resolver LAW rejection, redaction assertion,
//! soft-close audit retention, and EventLedger emission via count_events.
//! Tables persist between runs, so all titles / resolvers / manifest ids are
//! made unique per run via `Uuid::new_v4()` to avoid cross-run collisions. Only
//! `handshake_core` + `tokio` + `uuid` (+ serde_json + std) are used; sqlx is
//! never imported directly.

use handshake_core::atelier::stealth_window::stealth_ref_event_family::{
    STEALTH_REF_ADDED, STEALTH_REF_CAPTURED, STEALTH_REF_REMOVED, STEALTH_REF_REORDERED,
    STEALTH_REF_WINDOW_CLOSED, STEALTH_REF_WINDOW_CREATED,
};
use handshake_core::atelier::stealth_window::{
    ContentRefKind, NewContentRef, NewStealthWindow, QuietFlags, StealthRefStatus, VisibilityFlag,
};
use handshake_core::atelier::AtelierStore;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Build a run-unique `NewStealthWindow` with default (all-ON) quiet flags and
/// the non-intrusive off-screen-only visibility.
fn fresh_window_input() -> NewStealthWindow {
    NewStealthWindow {
        owner_actor: format!("operator-{}", Uuid::new_v4()),
        title: format!("stealth-window-{}", Uuid::new_v4()),
        visibility: VisibilityFlag::OffScreenOnly,
        quiet: QuietFlags::default(),
        layout: None,
    }
}

/// A run-unique governed resolver id (never a localhost / network / file
/// authority, never a machine-local path), accepted by `validate_resolver`.
fn governed_resolver() -> String {
    format!("artifact-manifest-{}", Uuid::new_v4())
}

#[tokio::test]
async fn stealth_window_create_idempotent_and_quiet_default() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_create_idempotent_and_quiet_default: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let created0 = store
        .count_events(STEALTH_REF_WINDOW_CREATED)
        .await
        .expect("count window_created events (before)");

    // --- create a window; round-trips with quiet-default + open status ---
    let input = fresh_window_input();
    let window = store
        .create_stealth_window(&input)
        .await
        .expect("create stealth window");
    assert_eq!(window.owner_actor, input.owner_actor, "owner round-trips");
    assert_eq!(window.title, input.title, "title round-trips");
    assert_eq!(
        window.visibility,
        VisibilityFlag::OffScreenOnly,
        "visibility round-trips as off-screen-only"
    );
    assert!(
        window.quiet.all_quiet(),
        "default quiet flags are all ON (non-intrusive by construction, HBR-QUIET)"
    );
    assert_eq!(window.status, StealthRefStatus::Open, "new window is Open");
    assert_eq!(window.revision, 1, "fresh window starts at revision 1");

    // --- IDEMPOTENCY: re-creating the same (owner, title) wins, no duplicate ---
    let window_again = store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: input.owner_actor.clone(),
            title: input.title.clone(),
            // Even with different visibility input, the existing entry wins.
            visibility: VisibilityFlag::DiagnosticEmbed,
            quiet: QuietFlags::default(),
            layout: Some(serde_json::json!({ "panels": ["a", "b"] })),
        })
        .await
        .expect("re-create same (owner, title)");
    assert_eq!(
        window.window_ref_id, window_again.window_ref_id,
        "re-creating the same (owner, title) returns the existing window id"
    );
    assert_eq!(
        window_again.visibility,
        VisibilityFlag::OffScreenOnly,
        "the existing entry wins; the second call does not overwrite visibility"
    );

    // get-by-title resolves to the same single registry row.
    let by_title = store
        .get_stealth_window_by_title(&input.owner_actor, &input.title)
        .await
        .expect("get by title")
        .expect("window present by title");
    assert_eq!(by_title.window_ref_id, window.window_ref_id);

    // Only ONE window for this owner exists (idempotent create did not duplicate).
    let listed = store
        .list_stealth_windows(&input.owner_actor, None, 100)
        .await
        .expect("list windows for owner");
    assert_eq!(
        listed.len(),
        1,
        "idempotent create yields exactly one registry row for the owner"
    );

    // --- INVARIANT: non-quiet off-screen window is rejected (HBR-QUIET) ---
    let mut loud = fresh_window_input();
    loud.quiet = QuietFlags {
        no_focus_steal: true,
        no_foreground: false, // inverted outside a foreground-exception window
        no_taskbar: true,
        no_global_shortcut: true,
        no_synthetic_input: true,
    };
    let loud_err = store.create_stealth_window(&loud).await;
    assert!(
        loud_err.is_err(),
        "a non-quiet off-screen-only window must be rejected (quiet flags must stay ON)"
    );

    // --- EVENT EMISSION: exactly one new window_created (idempotent re-create + ---
    // --- the rejected loud window emit nothing) ---
    let created1 = store
        .count_events(STEALTH_REF_WINDOW_CREATED)
        .await
        .expect("count window_created events (after)");
    assert_eq!(
        created1,
        created0 + 1,
        "exactly one window_created event for the single materialized window"
    );
}

#[tokio::test]
async fn stealth_window_add_refs_seq_monotonic_and_resolver_law() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP stealth_window_add_refs_seq_monotonic_and_resolver_law: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");

    let added0 = store
        .count_events(STEALTH_REF_ADDED)
        .await
        .expect("count added events (before)");

    // --- append two refs; seq is append-only monotonic 0, 1 ---
    let ref0 = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add first content ref");
    assert_eq!(ref0.seq, 0, "first appended ref is seq 0");
    assert!(ref0.redaction_state, "redaction assertion round-trips");
    assert_eq!(
        ref0.window_ref_id, window.window_ref_id,
        "ref is bound to its window"
    );

    let ref1 = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::SpecAnchor,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add second content ref");
    assert_eq!(ref1.seq, 1, "second appended ref is seq 1 (monotonic increment)");

    // round-trip via the read-only projection, ascending by seq.
    let refs = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs");
    assert_eq!(refs.len(), 2, "both refs present");
    assert_eq!(refs[0].ref_id, ref0.ref_id, "ordered by seq ascending");
    assert_eq!(refs[1].ref_id, ref1.ref_id);
    assert_eq!(refs[0].ref_kind, ContentRefKind::Artifact, "kind round-trips");
    assert_eq!(refs[1].ref_kind, ContentRefKind::SpecAnchor);
    assert!(
        refs[1].seq > refs[0].seq,
        "append-only sequence is strictly monotonic"
    );

    // --- INVARIANT: a non-governed resolver (machine-local path) is rejected ---
    let path_err = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Screenshot,
                resolver: "C:\\Users\\op\\capture.png".to_string(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await;
    assert!(
        path_err.is_err(),
        "a machine-local filesystem path resolver must be rejected (governed-id LAW)"
    );

    // --- INVARIANT: a non-redacted ref is rejected (secret hygiene) ---
    let redact_err = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: false,
            },
        )
        .await;
    assert!(
        redact_err.is_err(),
        "a ref that does not assert redaction_state must be rejected"
    );

    // The two rejected adds did not append anything.
    let refs_after = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after rejected adds");
    assert_eq!(refs_after.len(), 2, "rejected adds appended no rows");

    // --- EVENT EMISSION: exactly two ref-added events (rejected adds emit none) ---
    let added1 = store
        .count_events(STEALTH_REF_ADDED)
        .await
        .expect("count added events (after)");
    assert_eq!(
        added1,
        added0 + 2,
        "exactly two ref-added events for the two successful appends"
    );
}

#[tokio::test]
async fn stealth_window_reorder_permutation_guard_and_remove() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP stealth_window_reorder_permutation_guard_and_remove: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");

    // Append three refs at seq 0, 1, 2.
    let mut ref_ids = Vec::new();
    for _ in 0..3 {
        let r = store
            .add_stealth_ref(
                window.window_ref_id,
                &NewContentRef {
                    ref_kind: ContentRefKind::Artifact,
                    resolver: governed_resolver(),
                    content_sha256: format!("sha256-{}", Uuid::new_v4()),
                    redaction_state: true,
                },
            )
            .await
            .expect("add ref");
        ref_ids.push(r.ref_id);
    }

    let reordered0 = store
        .count_events(STEALTH_REF_REORDERED)
        .await
        .expect("count reordered events (before)");

    // --- INVARIANT: reorder must be an exact permutation (no missing ids) ---
    let partial_err = store
        .reorder_stealth_refs(window.window_ref_id, &ref_ids[..2], None)
        .await;
    assert!(
        partial_err.is_err(),
        "a reorder list missing a current ref must be rejected (no silent drop)"
    );

    // --- INVARIANT: reorder rejects duplicate ids in the supplied order ---
    let dup_err = store
        .reorder_stealth_refs(
            window.window_ref_id,
            &[ref_ids[0], ref_ids[0], ref_ids[1]],
            None,
        )
        .await;
    assert!(
        dup_err.is_err(),
        "a reorder list with a duplicate ref id must be rejected"
    );

    // --- valid reorder (two-phase) reverses the order and repins layout ---
    let new_order = vec![ref_ids[2], ref_ids[1], ref_ids[0]];
    let new_layout = serde_json::json!({ "panels": ["c", "b", "a"] });
    let window_after = store
        .reorder_stealth_refs(window.window_ref_id, &new_order, Some(&new_layout))
        .await
        .expect("reorder refs");
    assert_eq!(
        window_after.layout, new_layout,
        "the optional new layout is repinned in the same mutation"
    );
    assert!(
        window_after.revision > window.revision,
        "reorder bumps the window revision monotonically"
    );

    let refs_reordered = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after reorder");
    assert_eq!(
        refs_reordered.iter().map(|r| r.ref_id).collect::<Vec<_>>(),
        new_order,
        "refs now list in the requested new order"
    );
    // Seqs are re-assigned 0..N (no gaps, no UNIQUE(window, seq) collision).
    assert_eq!(refs_reordered[0].seq, 0);
    assert_eq!(refs_reordered[1].seq, 1);
    assert_eq!(refs_reordered[2].seq, 2);

    // --- remove one ref; remaining rows are preserved (no silent cascade) ---
    let removed = store
        .remove_stealth_ref(window.window_ref_id, new_order[0])
        .await
        .expect("remove ref");
    assert!(removed, "removing an existing ref returns true");
    let removed_again = store
        .remove_stealth_ref(window.window_ref_id, new_order[0])
        .await
        .expect("remove already-removed ref");
    assert!(
        !removed_again,
        "removing an already-removed ref returns false (no spurious event)"
    );
    let refs_post_remove = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after remove");
    assert_eq!(refs_post_remove.len(), 2, "exactly one ref removed");

    // --- EVENT EMISSION: exactly one reorder event (rejected reorders emit none) ---
    let reordered1 = store
        .count_events(STEALTH_REF_REORDERED)
        .await
        .expect("count reordered events (after)");
    assert_eq!(
        reordered1,
        reordered0 + 1,
        "exactly one reorder event for the single valid reorder"
    );
    // And the remove path emitted a removed event for the single real removal.
    let removed_events = store
        .count_events(STEALTH_REF_REMOVED)
        .await
        .expect("count removed events");
    assert!(
        removed_events >= 1,
        "at least one ref-removed event was emitted"
    );
}

#[tokio::test]
async fn stealth_window_capture_idempotent_and_close_audit() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_capture_idempotent_and_close_audit: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");

    let captured0 = store
        .count_events(STEALTH_REF_CAPTURED)
        .await
        .expect("count captured events (before)");
    let closed0 = store
        .count_events(STEALTH_REF_WINDOW_CLOSED)
        .await
        .expect("count closed events (before)");

    // --- record a capture receipt; round-trips manifest id + hash ---
    let manifest_id = governed_resolver();
    let sha = format!("sha256-{}", Uuid::new_v4());
    let receipt = store
        .record_stealth_capture(window.window_ref_id, &manifest_id, &sha)
        .await
        .expect("record capture receipt");
    assert_eq!(receipt.window_ref_id, window.window_ref_id, "receipt bound to window");
    assert_eq!(receipt.artifact_manifest_id, manifest_id, "manifest id round-trips");
    assert_eq!(receipt.content_sha256, sha, "content hash round-trips");

    // --- IDEMPOTENCY: re-recording the same (window, manifest_id) is stable ---
    let receipt_again = store
        .record_stealth_capture(window.window_ref_id, &manifest_id, &sha)
        .await
        .expect("re-record same capture");
    assert_eq!(
        receipt.capture_id, receipt_again.capture_id,
        "re-recording the same (window, manifest_id) returns the same receipt id (ON CONFLICT)"
    );
    let captures = store
        .list_stealth_captures(window.window_ref_id)
        .await
        .expect("list captures");
    assert_eq!(
        captures.len(),
        1,
        "idempotent capture did not duplicate the receipt row"
    );

    // --- INVARIANT: a non-governed manifest id (URL authority) is rejected ---
    let bad_manifest = store
        .record_stealth_capture(
            window.window_ref_id,
            "http://localhost:9222/screenshot",
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await;
    assert!(
        bad_manifest.is_err(),
        "a localhost / network manifest id must be rejected (governed-id LAW)"
    );

    // --- soft-close retains the row for audit; status flips to Closed ---
    let closed = store
        .close_stealth_window(window.window_ref_id)
        .await
        .expect("close window");
    assert_eq!(closed.status, StealthRefStatus::Closed, "status flips to Closed");
    assert!(
        closed.revision > window.revision,
        "close bumps the window revision"
    );
    // Audit retention: the closed row is still fetchable, not deleted.
    let fetched = store
        .get_stealth_window(window.window_ref_id)
        .await
        .expect("get closed window (retained for audit)");
    assert_eq!(fetched.status, StealthRefStatus::Closed, "closed row retained");
    // The capture receipt survives the close (no silent cascade delete).
    let captures_after_close = store
        .list_stealth_captures(window.window_ref_id)
        .await
        .expect("list captures after close");
    assert_eq!(captures_after_close.len(), 1, "capture receipt retained after close");

    // --- INVARIANT: a closed window refuses new refs ---
    let add_on_closed = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await;
    assert!(
        add_on_closed.is_err(),
        "a closed window must refuse new content refs"
    );

    // --- EVENT EMISSION: capture + close each emitted (idempotent re-record ---
    // --- and rejected capture do not inflate the capture count beyond +2) ---
    let captured1 = store
        .count_events(STEALTH_REF_CAPTURED)
        .await
        .expect("count captured events (after)");
    // Two successful captures (initial + idempotent re-record both emit), the
    // rejected bad-manifest capture emits nothing.
    assert_eq!(
        captured1,
        captured0 + 2,
        "two capture events (initial + idempotent re-record); rejected capture emits none"
    );
    let closed1 = store
        .count_events(STEALTH_REF_WINDOW_CLOSED)
        .await
        .expect("count closed events (after)");
    assert_eq!(
        closed1,
        closed0 + 1,
        "exactly one window_closed event for the single close"
    );
}
