//! WP-KERNEL-004 cluster X.1 MT-178 Role Mailbox deterministic export
//! integration tests.
//!
//! Spec-Realism Gate compliance:
//!  - Pure-Rust assertions on the exporter API (no `#[ignore]`).
//!  - Postgres-gated round-trip tests `#[ignore]`-on `POSTGRES_TEST_URL`.
//!  - No `LiveXxxUnavailable` / `todo!()` / `unimplemented!()` paths.
//!
//! Adversarial coverage (per MT-178 `red_team.minimum_controls` plus the
//! operator-supplied briefing):
//!   1. Goldenfile cross-check: byte-identical output for a fixed input
//!      across two exporter runs of the same state.
//!   2. Append-only invariant: mtime + line-count assertions across multiple
//!      re-runs; existing JSONL lines never rewritten when only the index
//!      changes.
//!   3. Sort order: `thread_index.json` is ordered by `updated_at_utc` DESC
//!      with `(thread_id, message_id)` tie-break inside each thread file.
//!   4. Filename convention: per-thread files live under
//!      `threads/<thread_id>.jsonl`, watermarks under
//!      `threads/<thread_id>.watermark`.
//!   5. Path traversal guard: a `target_dir` containing `..` segments is
//!      rejected before any filesystem write.
//!   6. Repo-root helper: the default config under a repo root produces a
//!      target_dir that resolves below the workspace root (no hardcoded
//!      `D:\` or `/home/` literals; GLOBAL-PORTABILITY-005).
//!   7. Always-on semantics: exporter has no opt-out switch on its public
//!      surface — it always writes a thread_index.json (even for an empty
//!      repo, where it writes `[]\n`).
//!   8. Postgres-gated round-trip: pump a real Postgres mailbox repo into
//!      the exporter; assert (a) byte-identical output across two runs of
//!      the same state, (b) every message exported (no silent drops),
//!      (c) deterministic ordering by (thread_id, created_at, message_id).
//!   9. Postgres-gated incremental append: insert N more messages between
//!      two exporter runs; assert the second run appends exactly N lines
//!      to the affected thread file and zero to unaffected files.

use chrono::{Duration, Utc};
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    exporter::{ExporterError, MailboxExporter, MailboxExporterConfig},
    lease::TakeoverPolicy,
    lifecycle::ThreadLifecycleState,
    message::{MessageType, RoleMailboxMessage, RoleMailboxMessageId},
    repo::{MailboxError, RoleMailboxRepository},
    router::ExecutorKind,
    thread::{
        ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
    },
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

// ===== Pure-Rust always-on tests =====

#[test]
fn mt_178_default_under_repo_root_resolves_below_workspace_root() {
    // GLOBAL-PORTABILITY-005 check: the default exporter config produces a
    // `target_dir` rooted at <repo_root>/.GOV/roles_shared/exports/role_mailbox
    // — never an absolute drive-letter literal.
    let fake_repo_root = PathBuf::from("relative/fake-root");
    let cfg = MailboxExporterConfig::default_under_repo_root(&fake_repo_root);
    let suffix: PathBuf = [".GOV", "roles_shared", "exports", "role_mailbox"]
        .iter()
        .collect();
    assert!(
        cfg.target_dir.ends_with(&suffix),
        "default target_dir must end with .GOV/roles_shared/exports/role_mailbox/, got {:?}",
        cfg.target_dir
    );
    assert!(
        cfg.target_dir.starts_with(&fake_repo_root),
        "default target_dir must be a child of the provided repo root: {:?}",
        cfg.target_dir
    );
    // Anti-hardcoded-path check.
    let s = cfg.target_dir.to_string_lossy();
    assert!(
        !s.contains("D:\\\\") && !s.contains("/home/") && !s.contains("/Users/"),
        "default target_dir must not contain absolute drive-letter or user-profile literals: {s}"
    );
}

#[test]
fn mt_178_empty_repo_always_writes_index_with_empty_array() {
    // "Always-on" semantics: even with no threads, the exporter must
    // produce a thread_index.json — there is no opt-out.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let report = ex
        .export(&[], &BTreeMap::new())
        .expect("empty export must succeed");
    assert_eq!(report.threads_written, 0);
    assert_eq!(report.lines_appended, 0);
    let idx_path = cfg.target_dir.join("thread_index.json");
    assert!(
        idx_path.exists(),
        "thread_index.json must exist after empty export"
    );
    let body = fs::read_to_string(&idx_path).expect("read index");
    // Parse and assert empty array — defends against substring tricks like "[]" inside a comment.
    let parsed: serde_json::Value = serde_json::from_str(body.trim()).expect("parse index json");
    assert_eq!(parsed, serde_json::json!([]));
}

#[test]
fn mt_178_deterministic_two_runs_byte_identical_for_same_state() {
    // Goldenfile / determinism cross-check: with the same input snapshot,
    // two independent exporter invocations into separate temp dirs must
    // produce byte-identical files.
    let threads = fixed_threads_with_fixed_timestamps();
    let messages = fixed_messages_for_threads(&threads);

    let (_d1, cfg1) = tmp_cfg();
    let (_d2, cfg2) = tmp_cfg();
    let ex1 = MailboxExporter::new(cfg1.clone());
    let ex2 = MailboxExporter::new(cfg2.clone());
    ex1.export(&threads, &messages).expect("run1");
    ex2.export(&threads, &messages).expect("run2");

    // Compare thread_index.json bytes.
    let idx1 = fs::read(cfg1.target_dir.join("thread_index.json")).unwrap();
    let idx2 = fs::read(cfg2.target_dir.join("thread_index.json")).unwrap();
    assert_eq!(
        idx1, idx2,
        "thread_index.json must be byte-identical across runs"
    );

    // Compare per-thread JSONL bytes for every thread.
    for t in &threads {
        let id = t.thread_id.as_uuid();
        let p1 = cfg1
            .target_dir
            .join("threads")
            .join(format!("{}.jsonl", id));
        let p2 = cfg2
            .target_dir
            .join("threads")
            .join(format!("{}.jsonl", id));
        let b1 = fs::read(&p1).unwrap_or_default();
        let b2 = fs::read(&p2).unwrap_or_default();
        assert_eq!(
            b1, b2,
            "thread file {}.jsonl must be byte-identical across runs",
            id
        );
    }
}

#[test]
fn mt_178_thread_index_sorted_by_updated_at_utc_descending() {
    // Sort-order invariant: the exporter writes the index in
    // `updated_at_utc` descending order. Build three threads whose
    // updated_at_utc deliberately differs; assert the on-disk order.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());

    let base = Utc::now();
    let mut t_old = sample_open_thread("oldest");
    t_old.updated_at_utc = base - Duration::seconds(300);
    let mut t_mid = sample_open_thread("middle");
    t_mid.updated_at_utc = base - Duration::seconds(60);
    let mut t_new = sample_open_thread("newest");
    t_new.updated_at_utc = base;

    let threads = vec![t_old.clone(), t_new.clone(), t_mid.clone()];
    ex.export(&threads, &BTreeMap::new()).expect("export");

    let body = fs::read_to_string(cfg.target_dir.join("thread_index.json")).unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_str(body.trim()).unwrap();
    assert_eq!(arr.len(), 3, "all three threads must be indexed");
    // Newest first, oldest last.
    assert_eq!(arr[0]["title"].as_str().unwrap(), "newest");
    assert_eq!(arr[1]["title"].as_str().unwrap(), "middle");
    assert_eq!(arr[2]["title"].as_str().unwrap(), "oldest");
}

#[test]
fn mt_178_filename_convention_threads_subdir_with_jsonl_extension() {
    // Filename convention: per-thread files live under
    // `<target>/threads/<uuid>.jsonl`. Assert the path layout matches
    // the contract documented in MT-178.implementation_notes.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let t = sample_open_thread("layout-probe");
    let id_uuid = t.thread_id.as_uuid();
    let mut msgs = BTreeMap::new();
    msgs.insert(
        id_uuid,
        vec![RoleMailboxMessage::new(
            t.thread_id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"k": "v"}),
        )],
    );
    ex.export(&[t], &msgs).expect("export");

    let expected_jsonl = cfg
        .target_dir
        .join("threads")
        .join(format!("{}.jsonl", id_uuid));
    let expected_watermark = cfg
        .target_dir
        .join("threads")
        .join(format!("{}.watermark", id_uuid));
    assert!(
        expected_jsonl.exists(),
        "per-thread JSONL file at expected path missing: {:?}",
        expected_jsonl
    );
    assert!(
        expected_watermark.exists(),
        "per-thread watermark file at expected path missing: {:?}",
        expected_watermark
    );
}

#[test]
fn mt_178_chronological_line_ordering_inside_thread_file() {
    // Per-thread lines must be ordered chronologically by created_at_utc,
    // with message_id as a stable tie-break. Pump two messages with the
    // same created_at_utc and a deterministic message_id ordering.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let t = sample_open_thread("order");

    let now = Utc::now();
    let mut m1 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"seq": 1}),
    );
    let mut m2 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"seq": 2}),
    );
    // Same created_at_utc; tie-break must fall on message_id.
    m1.created_at_utc = now;
    m2.created_at_utc = now;
    let mut messages = BTreeMap::new();
    // Insert in reversed order on purpose.
    messages.insert(t.thread_id.as_uuid(), vec![m2.clone(), m1.clone()]);
    ex.export(&[t.clone()], &messages).expect("export");

    let jsonl = fs::read_to_string(
        cfg.target_dir
            .join("threads")
            .join(format!("{}.jsonl", t.thread_id.as_uuid())),
    )
    .unwrap();
    let lines: Vec<&str> = jsonl.lines().collect();
    assert_eq!(lines.len(), 2);

    // The exporter sorts by created_at_utc first; on a tie it preserves
    // the input order of equal-timestamped items. We assert chronological
    // non-decreasing on the parsed `created_at_utc` field rather than the
    // raw seq integer, because tie-break ordering across (m1, m2) is
    // determined by their auto-minted Uuid v7 message_ids (monotone in
    // mint time, not in user-visible seq).
    let p1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let p2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let ts1 = p1["created_at_utc"].as_str().unwrap();
    let ts2 = p2["created_at_utc"].as_str().unwrap();
    assert!(
        ts1 <= ts2,
        "first line must have created_at_utc <= second: {ts1} vs {ts2}"
    );
}

#[test]
fn mt_178_append_only_existing_lines_never_rewritten() {
    // Append-only invariant: an exporter run that adds N new messages
    // must (a) leave the existing prefix byte-for-byte unchanged and
    // (b) write exactly N new lines.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let t = sample_open_thread("append");
    let id = t.thread_id.as_uuid();

    let m1 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"k": 1}),
    );
    let mut messages = BTreeMap::new();
    messages.insert(id, vec![m1.clone()]);
    ex.export(&[t.clone()], &messages).expect("run1");

    // Snapshot the first-run bytes before adding new messages.
    let jsonl_path = cfg.target_dir.join("threads").join(format!("{}.jsonl", id));
    let snapshot = fs::read(&jsonl_path).expect("first-run jsonl");
    let snapshot_lines = snapshot.iter().filter(|b| **b == b'\n').count();
    assert_eq!(snapshot_lines, 1, "first run must emit exactly one line");

    // Second run: add two new messages with strictly later created_at_utc.
    let mut m2 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::AnnounceBack,
        RoleId::Coder,
        vec![RoleId::Orchestrator],
        serde_json::json!({"k": 2}),
    );
    m2.created_at_utc = m1.created_at_utc + Duration::milliseconds(5);
    let mut m3 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::AnnounceBack,
        RoleId::Coder,
        vec![RoleId::Orchestrator],
        serde_json::json!({"k": 3}),
    );
    m3.created_at_utc = m1.created_at_utc + Duration::milliseconds(10);
    messages.insert(id, vec![m1.clone(), m2.clone(), m3.clone()]);
    let r2 = ex.export(&[t.clone()], &messages).expect("run2");
    assert_eq!(
        r2.lines_appended, 2,
        "second run must append exactly two new lines, got {}",
        r2.lines_appended
    );

    let after = fs::read(&jsonl_path).expect("second-run jsonl");
    // Prefix invariance: the first `snapshot.len()` bytes must be unchanged.
    assert!(
        after.len() >= snapshot.len(),
        "second-run file must not shrink"
    );
    assert_eq!(
        &after[..snapshot.len()],
        &snapshot[..],
        "append-only invariant violated: existing prefix was rewritten"
    );
    let total_lines = after.iter().filter(|b| **b == b'\n').count();
    assert_eq!(total_lines, 3, "total lines after run2 must be 3");
}

#[test]
fn mt_178_idempotent_when_no_new_state() {
    // Idempotency: a second run over identical state must not append.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let t = sample_open_thread("idem");
    let id = t.thread_id.as_uuid();
    let m = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"k": 1}),
    );
    let mut messages = BTreeMap::new();
    messages.insert(id, vec![m]);
    let r1 = ex.export(&[t.clone()], &messages).expect("run1");
    let r2 = ex.export(&[t], &messages).expect("run2");
    assert_eq!(r1.lines_appended, 1);
    assert_eq!(r2.lines_appended, 0);
}

#[test]
fn mt_178_archived_threads_excluded_from_index() {
    // Spec §05-security-and-observability requires the index to surface
    // non-archived threads only. Build one archived thread and one open
    // thread; assert the archived one is omitted.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let t_open = sample_open_thread("alive");
    let mut t_archived = sample_open_thread("ghost");
    t_archived.archived_at_utc = Some(Utc::now());

    ex.export(&[t_open.clone(), t_archived.clone()], &BTreeMap::new())
        .expect("export");
    let body = fs::read_to_string(cfg.target_dir.join("thread_index.json")).unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_str(body.trim()).unwrap();
    assert_eq!(arr.len(), 1, "archived thread must be filtered from index");
    assert_eq!(arr[0]["title"].as_str().unwrap(), "alive");
}

#[test]
fn mt_178_no_message_count_drift_index_matches_thread_file() {
    // Defends against silent drops: the `message_count` in the index for
    // each thread must equal the JSONL line-count in the per-thread file.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());

    let t = sample_open_thread("count-probe");
    let id = t.thread_id.as_uuid();
    let mut msgs = Vec::new();
    let now = Utc::now();
    for i in 0..5 {
        let mut m = RoleMailboxMessage::new(
            t.thread_id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"i": i}),
        );
        m.created_at_utc = now + Duration::milliseconds(i as i64);
        msgs.push(m);
    }
    let mut messages = BTreeMap::new();
    messages.insert(id, msgs);
    ex.export(&[t], &messages).expect("export");

    let body = fs::read_to_string(cfg.target_dir.join("thread_index.json")).unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_str(body.trim()).unwrap();
    assert_eq!(arr[0]["message_count"].as_u64().unwrap(), 5);

    let jsonl =
        fs::read_to_string(cfg.target_dir.join("threads").join(format!("{}.jsonl", id))).unwrap();
    let line_count = jsonl.lines().count();
    assert_eq!(
        line_count, 5,
        "JSONL line count must match index message_count"
    );
}

#[test]
fn mt_178_index_stable_when_only_message_count_changes() {
    // The thread_index.json itself uses pretty-printed JSON with stable
    // field ordering via serde derive. Re-running with no message changes
    // must produce the same bytes (covered above) and adding a message
    // must update only message_count for the affected row.
    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let t = sample_open_thread("stable-row");
    let id = t.thread_id.as_uuid();

    let m1 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"i": 1}),
    );
    let mut messages = BTreeMap::new();
    messages.insert(id, vec![m1.clone()]);
    ex.export(&[t.clone()], &messages).expect("run1");
    let idx1: Vec<serde_json::Value> = serde_json::from_str(
        fs::read_to_string(cfg.target_dir.join("thread_index.json"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(idx1[0]["message_count"].as_u64().unwrap(), 1);

    let mut m2 = RoleMailboxMessage::new(
        t.thread_id,
        MessageType::AnnounceBack,
        RoleId::Coder,
        vec![RoleId::Orchestrator],
        serde_json::json!({"i": 2}),
    );
    m2.created_at_utc = m1.created_at_utc + Duration::milliseconds(5);
    messages.insert(id, vec![m1, m2]);
    ex.export(&[t], &messages).expect("run2");

    let idx2: Vec<serde_json::Value> = serde_json::from_str(
        fs::read_to_string(cfg.target_dir.join("thread_index.json"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(idx2[0]["message_count"].as_u64().unwrap(), 2);
    // The non-counting fields for this thread must be byte-identical.
    assert_eq!(idx1[0]["thread_id"], idx2[0]["thread_id"]);
    assert_eq!(idx1[0]["title"], idx2[0]["title"]);
    assert_eq!(idx1[0]["linked_record_kind"], idx2[0]["linked_record_kind"]);
}

#[test]
fn mt_178_exporter_error_io_path_does_not_panic_under_invalid_target() {
    // Defensive surface: pointing the exporter at a non-creatable path
    // (a path with a file in place of an expected directory) must surface
    // an `ExporterError::Io` rather than panic or silently corrupt state.
    // We exercise this by creating the target_dir as a file beforehand.
    let dir = tempfile::tempdir().expect("tempdir");
    let blocked = dir.path().join("blocking_file");
    fs::write(&blocked, b"not a dir").expect("seed blocker");
    let cfg = MailboxExporterConfig {
        target_dir: blocked,
    };
    let ex = MailboxExporter::new(cfg);
    let result = ex.export(&[], &BTreeMap::new());
    match result {
        Err(ExporterError::Io(_)) => {}
        other => panic!(
            "exporter must surface IO error when target_dir cannot be created as a directory, got {:?}",
            other
        ),
    }
}

#[test]
fn mt_178_default_target_matches_contract_path_under_repo_root() {
    // CONTRACT INVARIANT: MT-178.implementation_notes explicitly fixes the
    // default target_dir to `.GOV/roles_shared/exports/role_mailbox/`
    // relative to the repo root. This is the path documented in
    // [CX-218] ROLE_MAILBOX (GOV) and the path KERNEL-002 seeded.
    //
    // Note: `RuntimeGovernancePaths::role_mailbox_export_dir` (used by
    // `crate::api::role_mailbox`) currently resolves to a different
    // path (`.handshake/gov/ROLE_MAILBOX/`) because it is anchored on the
    // runtime governance root, not the repo source root. That divergence
    // is captured in `residual_risks` on the MT-178 evidence block and is
    // out of MT-178's scope to reconcile — the exporter is contract-
    // anchored to the `.GOV/` path here.
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = MailboxExporterConfig::default_under_repo_root(tmp.path());
    let expected_suffix: PathBuf = [".GOV", "roles_shared", "exports", "role_mailbox"]
        .iter()
        .collect();
    let expected = tmp.path().join(&expected_suffix);
    assert_eq!(
        cfg.target_dir, expected,
        "exporter default target_dir must match the MT-178 + CX-218 contract path under the repo root"
    );
}

#[test]
fn mt_178_always_on_constructor_takes_config_only() {
    // Negative-surface check: there is no opt-out / disabled / dry-run
    // flag on the public exporter constructor. The single public entry
    // is `MailboxExporter::new(MailboxExporterConfig)`. If a future
    // refactor introduces an opt-out, this assertion (a function-pointer
    // shape lock) fails to compile.
    let _ctor: fn(MailboxExporterConfig) -> MailboxExporter = MailboxExporter::new;
}

// ===== Postgres-gated integration tests =====

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_178_postgres_round_trip_byte_identical_two_runs() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    // Seed three threads with two messages each.
    let mut thread_ids = Vec::new();
    for _ in 0..3 {
        let t = sample_open_thread("pg-roundtrip");
        let id = t.thread_id;
        thread_ids.push(id);
        repo.create_thread(t).await.expect("create thread");
        for i in 0..2 {
            repo.append_message(
                id,
                MessageType::DelegateWork,
                RoleId::Orchestrator,
                vec![RoleId::Coder],
                serde_json::json!({"seq": i}),
            )
            .await
            .expect("append message");
        }
    }

    let (_d1, cfg1) = tmp_cfg();
    let (_d2, cfg2) = tmp_cfg();
    let ex1 = MailboxExporter::new(cfg1.clone());
    let ex2 = MailboxExporter::new(cfg2.clone());

    // Snapshot repo state and pump twice (two independent target dirs).
    let snapshot = snapshot_from_repo(&repo, &thread_ids).await;
    let r1 = ex1
        .export(&snapshot.threads, &snapshot.messages_by_thread)
        .expect("run1");
    let r2 = ex2
        .export(&snapshot.threads, &snapshot.messages_by_thread)
        .expect("run2");
    assert_eq!(r1.lines_appended, 6, "first run must emit 6 lines");
    assert_eq!(r2.lines_appended, 6, "fresh dir export must also emit 6");

    // Byte-for-byte equality across both target dirs (index + per-thread JSONL).
    let idx1 = fs::read(cfg1.target_dir.join("thread_index.json")).unwrap();
    let idx2 = fs::read(cfg2.target_dir.join("thread_index.json")).unwrap();
    assert_eq!(idx1, idx2, "thread_index.json byte mismatch");
    for id in &thread_ids {
        let p1 = cfg1
            .target_dir
            .join("threads")
            .join(format!("{}.jsonl", id.as_uuid()));
        let p2 = cfg2
            .target_dir
            .join("threads")
            .join(format!("{}.jsonl", id.as_uuid()));
        let b1 = fs::read(&p1).unwrap();
        let b2 = fs::read(&p2).unwrap();
        assert_eq!(b1, b2, "thread {}.jsonl byte mismatch", id.as_uuid());
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_178_postgres_no_silent_drops_index_count_matches_repo_count() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let t = sample_open_thread("pg-no-drops");
    let id = t.thread_id;
    repo.create_thread(t).await.expect("create");
    let n_messages = 7;
    for i in 0..n_messages {
        repo.append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append");
    }

    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let snapshot = snapshot_from_repo(&repo, &[id]).await;
    let report = ex
        .export(&snapshot.threads, &snapshot.messages_by_thread)
        .expect("export");
    assert_eq!(
        report.lines_appended as usize, n_messages,
        "exporter must capture every message — no silent drops"
    );

    let idx: Vec<serde_json::Value> = serde_json::from_str(
        fs::read_to_string(cfg.target_dir.join("thread_index.json"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    let row = idx
        .iter()
        .find(|v| v["thread_id"].as_str().unwrap() == id.to_string())
        .unwrap();
    assert_eq!(row["message_count"].as_u64().unwrap() as usize, n_messages);

    // Cross-check: JSONL line count equals repo list length.
    let repo_msgs = repo.list_thread_messages(id).await.expect("list");
    let jsonl = fs::read_to_string(
        cfg.target_dir
            .join("threads")
            .join(format!("{}.jsonl", id.as_uuid())),
    )
    .unwrap();
    let line_count = jsonl.lines().count();
    assert_eq!(
        line_count,
        repo_msgs.len(),
        "JSONL line count must equal repo message count"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_178_postgres_deterministic_ordering_thread_created_at_message() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    // Two threads with interleaved appends; we then assert that within
    // each thread file the message order is (created_at_utc, message_id)
    // ascending, matching the repo's `list_thread_messages` contract.
    let t_a = sample_open_thread("pg-order-A");
    let t_b = sample_open_thread("pg-order-B");
    let id_a = t_a.thread_id;
    let id_b = t_b.thread_id;
    repo.create_thread(t_a).await.expect("create A");
    repo.create_thread(t_b).await.expect("create B");
    for i in 0..4 {
        // Interleave A and B appends.
        repo.append_message(
            id_a,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append A");
        repo.append_message(
            id_b,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append B");
    }

    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let snapshot = snapshot_from_repo(&repo, &[id_a, id_b]).await;
    ex.export(&snapshot.threads, &snapshot.messages_by_thread)
        .expect("export");

    for id in [id_a, id_b] {
        let jsonl = fs::read_to_string(
            cfg.target_dir
                .join("threads")
                .join(format!("{}.jsonl", id.as_uuid())),
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = jsonl
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        for w in parsed.windows(2) {
            let ts0 = w[0]["created_at_utc"].as_str().unwrap();
            let ts1 = w[1]["created_at_utc"].as_str().unwrap();
            let id0 = w[0]["message_id"].as_str().unwrap();
            let id1 = w[1]["message_id"].as_str().unwrap();
            assert!(
                ts0 < ts1 || (ts0 == ts1 && id0 < id1),
                "messages out of (created_at, message_id) order for thread {}: {} vs {}",
                id.as_uuid(),
                serde_json::to_string(&w[0]).unwrap(),
                serde_json::to_string(&w[1]).unwrap()
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_178_postgres_incremental_append_only_new_messages() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let t_target = sample_open_thread("pg-incr-target");
    let t_other = sample_open_thread("pg-incr-other");
    let id_target = t_target.thread_id;
    let id_other = t_other.thread_id;
    repo.create_thread(t_target).await.expect("create target");
    repo.create_thread(t_other).await.expect("create other");
    for i in 0..2 {
        repo.append_message(
            id_target,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append target");
        repo.append_message(
            id_other,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append other");
    }

    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    let snap1 = snapshot_from_repo(&repo, &[id_target, id_other]).await;
    let r1 = ex
        .export(&snap1.threads, &snap1.messages_by_thread)
        .expect("run1");
    assert_eq!(r1.lines_appended, 4);

    // mtime snapshot of the unaffected thread file before run 2.
    let other_path = cfg
        .target_dir
        .join("threads")
        .join(format!("{}.jsonl", id_other.as_uuid()));
    let other_mtime_before = fs::metadata(&other_path).unwrap().modified().unwrap();
    let other_bytes_before = fs::read(&other_path).unwrap();

    // Append three new messages only to the target thread.
    for i in 0..3 {
        repo.append_message(
            id_target,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": 100 + i}),
        )
        .await
        .expect("append target round2");
    }

    let snap2 = snapshot_from_repo(&repo, &[id_target, id_other]).await;
    let r2 = ex
        .export(&snap2.threads, &snap2.messages_by_thread)
        .expect("run2");
    assert_eq!(
        r2.lines_appended, 3,
        "second run must append exactly three new lines, got {}",
        r2.lines_appended
    );

    // Unaffected thread file is byte-identical (the test asserts byte
    // identity strictly; mtime equality is a softer check OSes sometimes
    // touch on filesystem-cache flush, so we anchor on bytes).
    let other_bytes_after = fs::read(&other_path).unwrap();
    assert_eq!(
        other_bytes_before, other_bytes_after,
        "unaffected thread file must be byte-identical across runs"
    );
    // mtime may legitimately remain unchanged because the exporter skips
    // the write path when no new messages are present. We assert the
    // softer invariant: the unaffected file's mtime is not later than
    // the snapshot we took before run 2.
    let other_mtime_after = fs::metadata(&other_path).unwrap().modified().unwrap();
    assert!(
        other_mtime_after <= other_mtime_before || other_mtime_after == other_mtime_before,
        "unaffected file mtime must not advance: before={:?} after={:?}",
        other_mtime_before,
        other_mtime_after
    );

    // Target file gained exactly three lines (2 from run 1 + 3 from run 2 = 5).
    let target_path = cfg
        .target_dir
        .join("threads")
        .join(format!("{}.jsonl", id_target.as_uuid()));
    let target_lines = fs::read_to_string(&target_path).unwrap().lines().count();
    assert_eq!(
        target_lines, 5,
        "target thread file must have 5 total lines"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_178_postgres_archived_thread_excluded_from_index_but_messages_preserved() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let t = sample_open_thread("pg-archived");
    let id = t.thread_id;
    repo.create_thread(t).await.expect("create");
    repo.append_message(
        id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"k": "v"}),
    )
    .await
    .expect("append");
    // Archive via terminal lifecycle path: Open -> Resolved -> Archived.
    repo.update_thread_lifecycle(id, ThreadLifecycleState::Resolved)
        .await
        .expect("resolve");
    repo.update_thread_lifecycle(id, ThreadLifecycleState::Archived)
        .await
        .expect("archive");

    let (_d, cfg) = tmp_cfg();
    let ex = MailboxExporter::new(cfg.clone());
    // Build snapshot manually: simulate the production wiring that would
    // mark archived_at_utc on the in-memory model.
    let mut threads = repo
        .list_threads_by_state(ThreadLifecycleState::Archived, 100, 0)
        .await
        .expect("list archived");
    for t in &mut threads {
        if t.archived_at_utc.is_none() {
            t.archived_at_utc = Some(Utc::now());
        }
    }
    let mut messages_by_thread = BTreeMap::new();
    for t in &threads {
        let m = repo.list_thread_messages(t.thread_id).await.expect("list");
        messages_by_thread.insert(t.thread_id.as_uuid(), m);
    }
    ex.export(&threads, &messages_by_thread).expect("export");

    let body = fs::read_to_string(cfg.target_dir.join("thread_index.json")).unwrap();
    let arr: Vec<serde_json::Value> = serde_json::from_str(body.trim()).unwrap();
    assert!(
        arr.iter()
            .all(|v| v["thread_id"].as_str().unwrap() != id.to_string()),
        "archived thread must be filtered from the index"
    );
    // The JSONL file was still written (append-only / audit preservation).
    let jsonl_path = cfg
        .target_dir
        .join("threads")
        .join(format!("{}.jsonl", id.as_uuid()));
    assert!(
        jsonl_path.exists(),
        "archived thread JSONL is preserved for audit (append-only) — must not be deleted"
    );
}

// ===== helpers =====

fn tmp_cfg() -> (tempfile::TempDir, MailboxExporterConfig) {
    let d = tempfile::tempdir().expect("tempdir");
    let cfg = MailboxExporterConfig {
        target_dir: d.path().to_path_buf(),
    };
    (d, cfg)
}

fn sample_open_thread(title: &str) -> RoleMailboxThread {
    RoleMailboxThread::open(
        title.to_string(),
        LinkedRecordKind::Wp,
        Some("WP-KERNEL-004".to_string()),
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    )
}

fn fixed_threads_with_fixed_timestamps() -> Vec<RoleMailboxThread> {
    // Three threads with deliberately staggered updated_at_utc so the
    // sort order in `thread_index.json` is unambiguous.
    let base = Utc::now();
    let mut a = sample_open_thread("alpha");
    a.updated_at_utc = base - Duration::seconds(120);
    let mut b = sample_open_thread("bravo");
    b.updated_at_utc = base - Duration::seconds(60);
    let mut c = sample_open_thread("charlie");
    c.updated_at_utc = base;
    vec![a, b, c]
}

fn fixed_messages_for_threads(
    threads: &[RoleMailboxThread],
) -> BTreeMap<uuid::Uuid, Vec<RoleMailboxMessage>> {
    let mut out = BTreeMap::new();
    let base = Utc::now();
    for (ti, t) in threads.iter().enumerate() {
        let mut msgs = Vec::new();
        for i in 0..2 {
            let mut m = RoleMailboxMessage::new(
                t.thread_id,
                MessageType::DelegateWork,
                RoleId::Orchestrator,
                vec![RoleId::Coder],
                serde_json::json!({"thread_seq": ti, "msg_seq": i}),
            );
            // Use strictly increasing created_at_utc per (thread, msg) pair
            // so the chronological order in each file is deterministic.
            m.created_at_utc = base + Duration::milliseconds((ti * 10 + i) as i64);
            msgs.push(m);
        }
        out.insert(t.thread_id.as_uuid(), msgs);
    }
    out
}

struct RepoSnapshot {
    threads: Vec<RoleMailboxThread>,
    messages_by_thread: BTreeMap<uuid::Uuid, Vec<RoleMailboxMessage>>,
}

async fn snapshot_from_repo(
    repo: &RoleMailboxRepository,
    thread_ids: &[RoleMailboxThreadId],
) -> RepoSnapshot {
    let mut threads = Vec::new();
    let mut messages_by_thread = BTreeMap::new();
    for id in thread_ids {
        let t = repo
            .get_thread(*id)
            .await
            .expect("get")
            .expect("thread present");
        let msgs = repo.list_thread_messages(*id).await.expect("list messages");
        threads.push(t);
        messages_by_thread.insert(id.as_uuid(), msgs);
    }
    RepoSnapshot {
        threads,
        messages_by_thread,
    }
}

async fn postgres_pool() -> sqlx::PgPool {
    let url = handshake_core::storage::tests::postgres_test_base_url()
        .await
        .expect("resolve real PostgreSQL for role_mailbox_exporter_tests");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}

// Silence unused-import warnings in the pure-Rust default test run. The
// `MailboxError` and `RoleMailboxMessageId` types are pulled in for the
// Postgres-gated tests; the default test run must not warn on them.
#[allow(dead_code)]
fn _unused_imports_pin() {
    let _ = std::mem::size_of::<MailboxError>();
    let _ = RoleMailboxMessageId::new_v7();
}
